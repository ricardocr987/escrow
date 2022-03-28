use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer, Mint};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>, 
        amount_a: u64, 
        amount_b: u64
    ) -> Result<()> {
        
        let escrow = &mut ctx.accounts.escrow_account;

        escrow.is_initilized = true; 
        escrow.authority = *ctx.accounts.authority.to_account_info().key;
        escrow.escrow_token_account_a = *ctx.accounts.maker_token_account_a.to_account_info().key;
        escrow.escrow_token_account_b = *ctx.accounts.taker_token_account_b.to_account_info().key;
        escrow.amount_a = amount_a;
        escrow.amount_b = amount_b;
        escrow.vault_bump = *ctx.bumps.get("vault").unwrap();

        // Transfer instruction will incoke the function process_ transfer to tranfer a certain amount od token
        // from a source account to a destination account, important checks:
        // - Neither from and to accounts is frozen
        // - From and to accounts mints are the same
        // - Transferred amount <= source_account token amount
        // - authority is signed
        // If the transfer instruction is succesful, the source account token amount is decremented by amount and the
        // destination token amount is incremented by amount
        let cpi_accounts = Transfer {
            from: ctx.accounts.maker_token_account_a.to_account_info().clone(),
            to: ctx.accounts.vault_account.to_account_info().clone(),
            authority: ctx.accounts.authority.to_account_info().clone(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_accounts
        );

        token::transfer(
            cpi_ctx,
            escrow.amount_a,
        )?;

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        
        let seeds = &[
            b"vault".as_ref(),
            ctx.accounts.escrow_account.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_account.vault_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_return_accounts = Transfer {
            from: ctx.accounts.vault_account.to_account_info().clone(),
            to: ctx.accounts.token_account_a.to_account_info().clone(),
            authority: ctx.accounts.token_account_a.to_account_info().clone(),
            // Las token Account son su propia autoridad porque son PDAs Accounts,
            // por eso nuestro program pueden firmar
        };

        let cpi_return_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_return_accounts,
            signer,
        );

        token::transfer(
            cpi_return_ctx,
            ctx.accounts.escrow_account.amount_a,
        )?;

        let cpi_accounts = CloseAccount {
            account: ctx.accounts.vault_account.to_account_info().clone(),
            destination: ctx.accounts.authority.to_account_info().clone(),
            authority: ctx.accounts.vault_account.to_account_info().clone(),
        };

        // Cerrar la vault Account y devolver los lamports al maker porque pagó por
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_accounts,
            signer,
        );

        token::close_account(
            cpi_ctx
        )?;

        Ok(())
    }

    // Un usuario (taker) acepta la oferta del la crea (maker), introduciendo la cantidad demandada del token seleccionado por el maker
    // Esto desbloquea los tokens del maker en el vault al taker
    pub fn exchange(ctx: Context<Exchange>) -> Result<()> {

        let seeds = &[
            b"vault".as_ref(),
            ctx.accounts.escrow_account.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_account.vault_bump],
        ];

        let signer = &[&seeds[..]];
        // ------------------------------------------------------------------------------------------------------------
        // Transfiere los tokens del taker al maker
        let cpi_initializer_accounts = Transfer {
            // ya hemos comprobado en el context de la instrucción si quiere enviar los tokens correctos
            from: ctx.accounts.taker_token_account_b.to_account_info().clone(),
            to: ctx.accounts.offer_makers_taker_tokens.to_account_info().clone(),
            // el que acepta la oferta tiene que firmar desde el client
            authority: ctx.accounts.authority.to_account_info().clone(),
        };

        let cpi_to_initializer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_initializer_accounts
        );

        token::transfer(
            cpi_to_initializer_ctx,
            ctx.accounts.escrow_account.amount_b,
        )?;
        // ------------------------------------------------------------------------------------------------------------
        // Transfiere los tokens del maker que están en el vault al taker
        let cpi_taker_accounts = Transfer {
            from: ctx.accounts.vault_account.to_account_info().clone(),
            to: ctx.accounts.maker_token_account_a.to_account_info().clone(),
            authority: ctx.accounts.vault_account.to_account_info().clone(),
        };

        let cpi_to_taker_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_taker_accounts,
            signer,
        );

        token::transfer(
            cpi_to_taker_ctx,
            ctx.accounts.escrow_account.amount_a,
        )?;

        // ------------------------------------------------------------------------------------------------------------
        let cpi_close_accounts = CloseAccount {
            account: ctx.accounts.vault_account.to_account_info().clone(),
            destination: ctx.accounts.authority.to_account_info().clone(),
            authority: ctx.accounts.vault_account.to_account_info().clone(),
        };

        let cpi_close_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info().clone(), 
            cpi_close_accounts,
            signer,
        );

        token::close_account(
            cpi_close_ctx,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_account_a_amount: u64)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 1 + 32 + 32 + 32 + 8 + 8 + 1,
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init, 
        payer = authority,
        seeds = [
            b"vault".as_ref(),
            escrow_account.key().as_ref(),
        ],
        bump,
        token::mint = maker_token_account_a,
        token::authority = vault_account, // Queremos que el mismo program tenga la autoridad sobre el token de la vault
        // Account, por eso necesitamos un PDA aquí y establecer como autiridad su propia dirección
    )]
    pub vault_account: Account<'info, TokenAccount>, // Aquí es donde almacenamos los tokens del maker

    #[account(
        mut,
        constraint = maker_token_account_a.amount >= token_account_a_amount,
    )]
    pub maker_token_account_a: Account<'info, TokenAccount>,
    pub taker_token_account_b: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(
        mut,
        constraint = escrow_account.authority == *authority.key, // comprobamos si el que ha inicializado la oferta es el que realmente la ha hecho
        constraint = escrow_account.escrow_token_account_a == *token_account_a.to_account_info().key,
        // otra comprobación, comprobamos si ecoincide la token account
        close = authority // al final de la instrucción, cerramos la Account y devuelve los lamports al usuario que la creó
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [
            b"vault".as_ref(),
            escrow_account.key().as_ref(),
        ],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>, // El que ha hecho la oferta con la instrucción anterior, tiene que firmar si quiere cancelarla
    #[account(mut)]
    pub token_account_a: Account<'info, TokenAccount>, // Aquí es donde tenemos que devolver los tokens
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    #[account(
        mut,
        constraint = escrow_account.amount_b <= taker_token_account_b.amount,
        constraint = escrow_account.authority == *authority.key,
        constraint = escrow_account.escrow_token_account_a == *maker_token_account_a.to_account_info().key,
        constraint = escrow_account.escrow_token_account_b == *taker_token_account_b.to_account_info().key,
        close = authority
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [
            b"vault".as_ref(),
            escrow_account.key().as_ref(),
        ],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub authority: Signer<'info>, // offer_taker, el que acepta la oferta
    pub offer_maker: AccountInfo<'info>, // el que crea el escrow

    #[account(
        mut,
        associated_token::mint = taker_mint,
        associated_token::authority = offer_maker,
    )]
    pub offer_makers_taker_tokens: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = taker_token_account_b.mint == escrow_account.escrow_token_account_b
        // comprueba si el que acepta la oferta esta introduciendo los tokens correctos
    )]
    pub taker_token_account_b: Account<'info, TokenAccount>, 

    #[account(mut)]  
    pub maker_token_account_a: Account<'info, TokenAccount>,

    #[account(address = escrow_account.escrow_token_account_b)]
    pub taker_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Escrow {
    pub is_initilized: bool,
    pub authority: Pubkey, // Almacenamos la pubkey del usuario que pone la oferta
    pub escrow_token_account_a: Pubkey, // Token Accounts que almacenarán los tokens en el escrow
    pub escrow_token_account_b: Pubkey,
    pub amount_a: u64, // Para hacer comprobaciones con los constraint del context
    pub amount_b: u64, 
    pub vault_bump: u8, // Cuando el maker hace su pferta, almacenamos sus token en un vault account
        // que estrá en un PDA Account, con las seeds de este Escrow Account, almacenarlo aqui nos permite
        // que el client no tenga que pasarlo como argumento a la instrucción
}