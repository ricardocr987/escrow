use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount, Transfer, Mint };

declare_id!("D7ko992PKYLDKFy3fWCQsePvWF3Z7CmvoDHnViGf8bfm");

#[program]
pub mod escrow {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        amount_a: u64,
        amount_b: u64,
        escrow_bump: u8,
        vault_bump: u8,
        id: u64,
    ) -> Result<()> {

        let escrow = &mut ctx.accounts.escrow_account;

        escrow.maker = ctx.accounts.authority.to_account_info().key();
        escrow.mint_b = ctx.accounts.mint_b.to_account_info().key();
        escrow.amount_b = amount_b;
        escrow.id = id;
        escrow.escrow_bump = escrow_bump;
        escrow.vault_bump = vault_bump;

        // Transfer instruction will incoke the function process transfer to tranfer a certain amount of tokens
        // from a source account to a destination account, important checks:
        // - Neither from and to accounts is frozen
        // - From and to accounts mints are the same
        // - Transferred amount <= source_account token amount
        // - authority is signed
        // If the transfer instruction is succesful, the source account token amount is decremented by amount and the
        // destination token amount is incremented by amount
        let cpi_accounts = Transfer {
            from: ctx.accounts.token_account_a.to_account_info(),
            to: ctx.accounts.vault_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts
        );

        token::transfer(
            cpi_ctx, 
            amount_a
        )?;

        Ok(())
    }

    pub fn cancel(
        ctx: Context<Cancel>,
    ) -> Result<()> {

        let escrow_id = ctx.accounts.escrow_account.id.to_le_bytes();

        let seeds = &[
            b"escrow",
            escrow_id.as_ref(),
            &[ctx.accounts.escrow_account.escrow_bump]
        ];

        let signer = &[&seeds[..]];

        let cpi_accounts_tx = Transfer {
            from: ctx.accounts.vault_account.to_account_info(),
            to: ctx.accounts.token_account_a.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };

        let cpi_ctx_tx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts_tx, 
            signer,
        );

        token::transfer(cpi_ctx_tx, ctx.accounts.escrow_account.amount_b)?;
        
        // Para cerrar la vault Account y devolver los lamports al maker
        let cpi_accounts_close = CloseAccount {
            account: ctx.accounts.vault_account.to_account_info(),
            destination: ctx.accounts.authority.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };

        let cpi_ctx_cancel = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts_close, 
            signer,
        );

        token::close_account(cpi_ctx_cancel)?;

        Ok(())
    }
    // Un usuario (taker) acepta la oferta del la crea (maker), introduciendo la cantidad demandada del token seleccionado por el maker
    // Esto desbloquea los tokens del maker en el vault al taker
    pub fn exchange(ctx: Context<Exchange>) -> Result<()> {

        let escrow_id = ctx.accounts.escrow_account.id.to_le_bytes();

        let seeds = &[
            b"escrow",
            escrow_id.as_ref(),
            &[ctx.accounts.escrow_account.escrow_bump]
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
            authority: ctx.accounts.escrow_account.to_account_info(),
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
        let cpi_accounts_close = CloseAccount {
            account: ctx.accounts.vault_account.to_account_info(),
            destination: ctx.accounts.authority.to_account_info(),
            authority: ctx.accounts.escrow_account.to_account_info(),
        };

        let cpi_ctx_cancel = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(), 
            cpi_accounts_close, 
            signer,
        );

        token::close_account(cpi_ctx_cancel)?;
        
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(amount_a: u64, amount_b: u64, escrow_bump: u8, vault_bump: u8, id: u64)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 32 + 8 + 8 + 1 + 1,
        seeds = [
            b"escrow",
            id.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub escrow_account: Account<'info, EscrowAccount>,
    #[account(
        init,
        payer = authority,
        seeds = [
            b"vault",
            escrow_account.key().as_ref(),
        ],
        bump,
        token::mint = mint_a,
        token::authority = escrow_account,
    )]
    pub vault_account: Account<'info, TokenAccount>, // Aquí es donde almacenamos los tokens del maker
    #[account(mut)]
    pub authority: Signer<'info>, // El maker es el que firma esta instrucción
    #[account(
        mut,
        constraint = token_account_a.mint == mint_a.key()
    )]
    pub token_account_a: Account<'info, TokenAccount>, // Token Account del token A del maker
    pub mint_a: Account<'info, Mint>, // Necesario para crear el vault
    pub mint_b: Account<'info, Mint>, // Lo queremos en el context para almacenarlo en el Escrow Account, para hacer comprobaciones después
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(
        mut,
        close = authority, // al final de la instrucción, cerramos la Account y devuelve los lamports al usuario que la creó
        constraint = escrow_account.maker == *authority.key, // comprobamos si el que ha inicializado la oferta es el que realmente la ha hecho
        seeds = [
            b"escrow",
            escrow_account.id.to_le_bytes().as_ref(),
        ],
        bump = escrow_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, EscrowAccount>,
    #[account(
        mut,
        seeds = [
            b"vault",
            escrow_account.key().as_ref(),
        ],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub authority: Signer<'info>, // El que ha hecho la oferta con la instrucción anterior, tiene que firmar si quiere cancelarla
    #[account(
        mut,
        constraint = token_account_a.mint == vault_account.mint,
    )]
    pub token_account_a: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct Exchange<'info> {
    #[account(
        mut,
        constraint = escrow_account.maker == *maker.key,
        close = authority,
        seeds = [
            b"escrow",
            escrow_account.id.to_le_bytes().as_ref(),
        ],
        bump = escrow_account.escrow_bump,
    )]
    pub escrow_account: Account<'info, EscrowAccount>,

    #[account(
        mut,
        seeds = [
            b"vault",
            escrow_account.key().as_ref(),
        ],
        bump = escrow_account.vault_bump,
    )]
    pub vault_account: Account<'info, TokenAccount>,

    /// CHECK:
    #[account(
        mut, 
        constraint = escrow_account.maker == maker.key()
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
        constraint = taker_token_account_b.mint == escrow_account.mint_b
        // comprueba si el que acepta la oferta esta introduciendo los tokens correctos
    )]
    pub taker_token_account_b: Account<'info, TokenAccount>, 

    #[account(mut)]  
    pub taker_token_account_a: Account<'info, TokenAccount>,

    #[account(address = escrow_account.mint_b)]
    pub taker_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
}

#[account]
pub struct EscrowAccount {
    pub maker: Pubkey, // Almacenamos la pubkey del usuario que pone la oferta
    pub mint_b: Pubkey,// Almacenamos el mint del TokenB que introduce el maker para poder comprobar que el taker realmente
    // va intercambiar esos tokens, el mint es el identificador del token
    pub amount_b: u64, // Almacenamos la cantidad que quiere el maker del tokenB para hacer comprobaciones
    pub id: u64, // Utilizo este número para las seeds del escrow
    pub escrow_bump: u8, 
    pub vault_bump: u8, // Cuando el maker hace su oferta, almacenamos sus token en un vault account su dirección será un PDA, 
    // con las la key del EscrowAccount + vault_bump + ProgramID, podemos obtener el PDA almacenando aqui el bump 
    // nos permite que el client no tenga que pasarlo como argumento a la instrucción cancel o exchange
}
