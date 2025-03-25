use crate::{
    contexts::{Initialize, Unwhitelist, UpdateAuthority, UpdatePaused, UpdateTss, Whitelist},
    errors::{Errors, InstructionId},
    utils::{recover_and_verify_eth_address, validate_message_hash, verify_and_update_nonce},
};
use anchor_lang::prelude::*;

/// Initializes the gateway PDA.
pub fn initialize(ctx: Context<Initialize>, tss_address: [u8; 20], chain_id: u64) -> Result<()> {
    let initialized_pda = &mut ctx.accounts.pda;

    initialized_pda.nonce = 0;
    initialized_pda.tss_address = tss_address;
    initialized_pda.authority = ctx.accounts.signer.key();
    initialized_pda.chain_id = chain_id;
    initialized_pda.deposit_paused = false;

    msg!(
        "Gateway initialized: PDA authority = {}, chain_id = {}, TSS address = {:?}",
        ctx.accounts.signer.key(),
        chain_id,
        tss_address
    );

    Ok(())
}

/// Updates the TSS address. Caller is authority stored in PDA.
//     /// # Arguments
//     /// * `ctx` - The instruction context.
//     /// * `tss_address` - The new Ethereum TSS address (20 bytes).
pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    require!(
        ctx.accounts.signer.key() == pda.authority,
        Errors::SignerIsNotAuthority
    );
    pda.tss_address = tss_address;

    msg!(
        "TSS address updated: new TSS address = {:?}, PDA authority = {}",
        tss_address,
        ctx.accounts.signer.key()
    );

    Ok(())
}

/// Updates the PDA authority. Caller is authority stored in PDA.
//     /// # Arguments
//     /// * `ctx` - The instruction context.
//     /// * `new_authority_address` - The new authority's public key.
pub fn update_authority(
    ctx: Context<UpdateAuthority>,
    new_authority_address: Pubkey,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    require!(
        ctx.accounts.signer.key() == pda.authority,
        Errors::SignerIsNotAuthority
    );
    pda.authority = new_authority_address;

    msg!(
        "PDA authority updated: new authority = {}, previous authority = {}",
        new_authority_address,
        ctx.accounts.signer.key()
    );

    Ok(())
}

/// Pauses or unpauses deposits. Caller is authority stored in PDA.
/// # Arguments
/// * `ctx` - The instruction context.
/// * `deposit_paused` - Boolean flag to pause or unpause deposits.
pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    require!(
        ctx.accounts.signer.key() == pda.authority,
        Errors::SignerIsNotAuthority
    );
    pda.deposit_paused = deposit_paused;

    msg!("Set deposit paused: {:?}", deposit_paused);
    Ok(())
}

/// Whitelists a new SPL token. Caller is TSS.
/// # Arguments
/// * `ctx` - The instruction context.
/// * `signature` - The TSS signature.
/// * `recovery_id` - The recovery ID for signature verification.
/// * `message_hash` - Message hash for signature verification.
/// * `nonce` - The current nonce value.
pub fn whitelist_spl_mint(
    ctx: Context<Whitelist>,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    let whitelist_candidate = &mut ctx.accounts.whitelist_candidate;
    let authority = &ctx.accounts.authority;

    if signature != [0u8; 64] {
        // Verify and update nonce
        verify_and_update_nonce(pda, nonce)?;

        // Validate message hash - pass None for amount to match original whitelist hash structure
        validate_message_hash(
            InstructionId::WhitelistSplToken,
            pda.chain_id,
            nonce,
            None, // Skip amount in hash calculation
            &[&whitelist_candidate.key().to_bytes()],
            &message_hash,
        )?;

        // Verify TSS signature
        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;
    } else {
        require!(
            authority.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
    }

    msg!(
        "SPL token whitelisted: mint = {}, whitelist_entry = {}, authority = {}",
        whitelist_candidate.key(),
        ctx.accounts.whitelist_entry.key(),
        ctx.accounts.authority.key()
    );

    Ok(())
}

/// Unwhitelists an SPL token. Caller is TSS.
/// # Arguments
/// * `ctx` - The instruction context.
/// * `signature` - The TSS signature.
/// * `recovery_id` - The recovery ID for signature verification.
/// * `message_hash` - Message hash for signature verification.
/// * `nonce` - The current nonce value.
pub fn unwhitelist_spl_mint(
    ctx: Context<Unwhitelist>,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    let whitelist_candidate = &mut ctx.accounts.whitelist_candidate;
    let authority = &ctx.accounts.authority;

    if signature != [0u8; 64] {
        // Verify and update nonce
        verify_and_update_nonce(pda, nonce)?;

        // Validate message hash
        validate_message_hash(
            InstructionId::UnwhitelistSplToken,
            pda.chain_id,
            nonce,
            None, // Skip amount in hash calculation
            &[&whitelist_candidate.key().to_bytes()],
            &message_hash,
        )?;

        // Verify TSS signature
        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;
    } else {
        require!(
            authority.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
    }

    msg!(
        "SPL token unwhitelisted: mint = {}, whitelist_entry = {}, authority = {}",
        whitelist_candidate.key(),
        ctx.accounts.whitelist_entry.key(),
        ctx.accounts.authority.key()
    );

    Ok(())
}
