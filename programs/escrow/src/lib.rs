use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer, Mint };

declare_id!("D7ko992PKYLDKFy3fWCQsePvWF3Z7CmvoDHnViGf8bfm");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        vault_bump: u8,
        amount_a: u64, 
        amount_b: u64
    ) -> Result<()> {
        
        let escrow = &mut ctx.accounts.escrow_account;

        escrow.authority = ctx.accounts.authority.key();
        escrow.taker_mint = ctx.accounts.taker_mint.key();
        escrow.amount_b = amount_b;
        escrow.vault_bump = vault_bump;

        // Transfer instruction will incoke the function process_ transfer to tranfer a certain amount od token
        // from a source account to a destination account, important checks:
        // - Neither from and to accounts is frozen
        // - From and to accounts mints are the same
        // - Transferred amount <= source_account token amount
        // - authority is signed
        // If the transfer instruction is succesful, the source account token amount is decremented by amount and the
        // destination token amount is incremented by amount
        let cpi_accounts = Transfer {
            from: ctx.accounts.maker_token_account_a.to_account_info(),
            to: ctx.accounts.vault_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts
        );

        token::transfer(
            cpi_ctx,
            amount_a,
        )?;

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
        
        let seeds = &[
            ctx.accounts.escrow_account.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_account.vault_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_return_accounts = Transfer {
            from: ctx.accounts.vault_account.to_account_info(),
            to: ctx.accounts.token_account_a.to_account_info(),
            authority: ctx.accounts.vault_account.to_account_info(),
            // Las token Account son su propia autoridad porque son PDAs Accounts,
            // por eso nuestro program pueden firmar los CPIs
        };

        let cpi_return_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_return_accounts,
            signer,
        );

        token::transfer(
            cpi_return_ctx,
            ctx.accounts.vault_account.amount,
        )?;
        
        if ctx.accounts.vault_account.amount == 0 {
            // Cerrar la vault Account y devolver los lamports al maker
            let cpi_accounts = CloseAccount {
                account: ctx.accounts.vault_account.to_account_info(),
                destination: ctx.accounts.authority.to_account_info(),
                authority: ctx.accounts.vault_account.to_account_info(),
            };
            
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), 
                cpi_accounts,
                signer,
            );

            token::close_account(
                cpi_ctx
            )?;
        }

        Ok(())
    }

    // Un usuario (taker) acepta la oferta del la crea (maker), introduciendo la cantidad demandada del token seleccionado por el maker
    // Esto desbloquea los tokens del maker en el vault al taker
    pub fn exchange(ctx: Context<Exchange>) -> Result<()> {

        let seeds = &[
            ctx.accounts.escrow_account.to_account_info().key.as_ref(),
            &[ctx.accounts.escrow_account.vault_bump],
        ];
        let signer = &[&seeds[..]];
        // ------------------------------------------------------------------------------------------------------------
        // Transfiere los tokens del taker al maker
        let cpi_initializer_accounts = Transfer {
            // ya hemos comprobado en el context de la instrucción si quiere enviar los tokens correctos
            from: ctx.accounts.taker_token_account_b.to_account_info(),
            to: ctx.accounts.maker_token_account_b.to_account_info(),
            // el que acepta la oferta tiene que firmar desde el client
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_to_initializer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            cpi_initializer_accounts
        );

        token::transfer(
            cpi_to_initializer_ctx,
            ctx.accounts.escrow_account.amount_b,
        )?;
        // ------------------------------------------------------------------------------------------------------------
        // Transfiere los tokens del maker que están en el vault al taker
        let cpi_taker_accounts = Transfer {
            from: ctx.accounts.vault_account.to_account_info(),
            to: ctx.accounts.taker_token_account_a.to_account_info(),
            authority: ctx.accounts.vault_account.to_account_info(),
        };

        let cpi_to_taker_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_taker_accounts,
            signer,
        );

        token::transfer(
            cpi_to_taker_ctx,
            ctx.accounts.vault_account.amount,
        )?;

        // ------------------------------------------------------------------------------------------------------------
        let cpi_close_accounts = CloseAccount {
            account: ctx.accounts.vault_account.to_account_info(),
            destination: ctx.accounts.maker.to_account_info(),
            authority: ctx.accounts.vault_account.to_account_info(),
        };

        let cpi_close_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
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
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 90,
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(mut)]
    pub authority: Signer<'info>, // El maker es el que firma esta transacción

    #[account(
        mut, 
        constraint = maker_token_account_a.mint == maker_mint.key()
    )]
    pub maker_token_account_a: Account<'info, TokenAccount>, // Token Account del token A del maker

    #[account(
        init, 
        payer = authority,
        seeds = [
            escrow_account.key().as_ref(),
        ],
        bump,
        token::mint = maker_mint,
        token::authority = vault_account, // Queremos que el mismo program tenga la autoridad sobre el token de la vault
        // Account, por eso necesitamos un PDA aquí y establecer como autiridad su propia dirección
    )]
    pub vault_account: Account<'info, TokenAccount>, // Aquí es donde almacenamos los tokens del maker
    

    pub maker_mint: Account<'info, Mint>, // Necesario para crear el vault
    pub taker_mint: Account<'info, Mint>, // Lo queremos en el context para almacenarlo en el Escrow Account
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(
        mut,
        constraint = escrow_account.authority == *authority.key, // comprobamos si el que ha inicializado la oferta es el que realmente la ha hecho
        close = authority // al final de la instrucción, cerramos la Account y devuelve los lamports al usuario que la creó
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(mut)]
    pub authority: Signer<'info>, // El que ha hecho la oferta con la instrucción anterior, tiene que firmar si quiere cancelarla

    #[account(
        mut,
        seeds = [escrow_account.key().as_ref()],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub token_account_a: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    #[account(
        mut,
        constraint = escrow_account.authority == *maker.key,
        close = authority
    )]
    pub escrow_account: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [escrow_account.key().as_ref()],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,
    /// CHECK:
    #[account(
        mut, 
        constraint = escrow_account.authority == maker.key()
    )]/// CHECK:
    pub maker: AccountInfo<'info>, // el que crea el escrow

    #[account(mut)]
    pub authority: Signer<'info>, // taker, el que acepta la oferta
    
    #[account(
        mut,
        associated_token::mint = taker_mint,
        associated_token::authority = maker,
    )]
    pub maker_token_account_b: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = taker_token_account_b.mint == escrow_account.taker_mint
        // comprueba si el que acepta la oferta esta introduciendo los tokens correctos
    )]
    pub taker_token_account_b: Account<'info, TokenAccount>, 

    #[account(mut)]  
    pub taker_token_account_a: Account<'info, TokenAccount>,

    #[account(address = escrow_account.taker_mint)]
    pub taker_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Escrow {
    pub authority: Pubkey, // Almacenamos la pubkey del usuario que pone la oferta
    pub taker_mint: Pubkey, // Almacenamos el mint del TokenB que introduce el maker para poder comprobar que el taker realmente
                            // va intercambiar esos tokens, el mint es el identificador del token
    pub amount_b: u64, // Almacenamos la cantidad que quiere el maker del tokenB para hacer comprobaciones
    pub vault_bump: u8, // Cuando el maker hace su oferta, almacenamos sus token en un vault account
        // que estrá en un PDA Account, con las seeds de este Escrow Account, almacenarlo aqui nos permite
        // que el client no tenga que pasarlo como argumento a la instrucción
}
