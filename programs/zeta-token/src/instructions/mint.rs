use crate::{contexts::Mint, errors::ZetaTokenErrors, state::ZetaTokenPda};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, MintTo};

/// Mint ZETA tokens to a recipient.
pub fn mint_tokens(ctx: Context<Mint>, amount: u64, internal_send_hash: [u8; 32]) -> Result<()> {
    let zeta_token_pda = &mut ctx.accounts.zeta_token_pda;

    // Check if minting would exceed max supply.
    require!(
        zeta_token_pda.total_supply + amount <= zeta_token_pda.max_supply,
        ZetaTokenErrors::MaxSupplyExceeded
    );

    // Check amount is valid.
    require!(amount > 0, ZetaTokenErrors::InvalidAmount);

    // Mint tokens to recipient.
    let cpi_accounts = MintTo {
        mint: ctx.accounts.zeta_mint.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    mint_to(cpi_ctx, amount)?;

    // Update total supply.
    zeta_token_pda.total_supply += amount;

    msg!(
        "Minted {} ZETA tokens to {}",
        amount,
        ctx.accounts.recipient_token_account.key()
    );
    msg!("Internal send hash: {:?}", internal_send_hash);
    msg!("New total supply: {}", zeta_token_pda.total_supply);

    Ok(())
}
