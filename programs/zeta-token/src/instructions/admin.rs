use crate::{
    contexts::{Initialize, UpdateTssAddress }
    errors::ZetaTokenErrors,
    state::ZetaTokenPda
};
use anchor_lang::prelude::*;
use anchor_spl::token::{initialize_mint, InitializeMint};

/// Initialize the ZETA token program.
pub fn initialize(
    ctx: Context<Initialize>,
    tss_address: [u8; 20],
    tss_address_updater: Pubkey,
    max_supply: u64,
) -> Result<()> {
    let zeta_token_pda = &mut ctx.accounts.zeta_token_pda;
    

    zeta_token_pda.mint_authority = ctx.accounts.zeta_token_pda.key();
    zeta_token_pda.burn_authority = ctx.accounts.zeta_token_pda.key();
    zeta_token_pda.tss_address = tss_address;
    zeta_token_pda.tss_address_updater = tss_address_updater;
    zeta_token_pda.max_supply = max_supply;
    zeta_token_pda.total_supply = 0;
    zeta_token_pda.decimals = 18;

    msg!("ZETA token program initialized");
    msg!("TSS Address: {:?}", tss_address);
    msg!("Max Supply: {}", max_supply);

    Ok(())
}

/// Update TSS address.
pub fn update_tss_address(
    ctx: Context<UpdateTssAddress>,
    new_tss_address: [u8; 20],
) -> Result<()> {
    let zeta_token_pda = &mut ctx.accounts.zeta_token_pda;
    
    let old_tss_address = zeta_token_pda.tss_address;
    zeta_token_pda.tss_address = new_tss_address;

    msg!("TSS address updated from {:?} to {:?}", old_tss_address, new_tss_address);

    Ok(())
}