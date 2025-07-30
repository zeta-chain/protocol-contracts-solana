use crate::{contexts::Burn, errors::ZetaTokenErrors, state::ZetaTokenPda};
use anchor_lang::prelude::*;
use anchor_spl::token::{burn, Burn};

/// Burn ZETA tokens from an account.
pub fn burn_tokens(ctx: Context<Burn>, amount: u64) -> Result<()> {
    let zeta_token_pda = &mut ctx.accounts.zeta_token_pda;

    // Check amount is valid.
    require!(amount > 0, ZetaTokenErrors::InvalidAmount);

    // Burn tokens from account
    let cpi_accounts = Burn {
        mint: ctx.accounts.zeta_mint.to_account_info(),
        from: ctx.accounts.token_account.to_account_info(),
        authority: ctx.accounts.burn_authority.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    burn(cpi_ctx, amount)?;

    // Update total supply.
    zeta_token_pda.total_supply -= amount;

    msg!(
        "Burned {} ZETA tokens from {}",
        amount,
        ctx.accounts.token_account.key()
    );
    msg!("New total supply: {}", zeta_token_pda.total_supply);

    Ok(())
}
