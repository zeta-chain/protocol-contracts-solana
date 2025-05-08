use crate::{
    contexts::{
        Initialize, ResetNonce, Unwhitelist, UpdateAuthority, UpdatePaused, UpdateTss, Whitelist,
    },
    state::InstructionId,
    utils::{
        recover_and_verify_eth_address, validate_message_hash, verify_and_update_nonce,
        verify_authority,
    },
    Pda,
};
use anchor_lang::prelude::*;

// Initializes the gateway PDA.
pub fn initialize(ctx: Context<Initialize>, tss_address: [u8; 20], chain_id: u64) -> Result<()> {
    let initialized_pda = &mut ctx.accounts.pda;

    **initialized_pda = Pda {
        nonce: 0,
        tss_address,
        authority: ctx.accounts.signer.key(),
        chain_id,
        deposit_paused: false,
    };

    msg!(
        "Gateway initialized: PDA authority = {}, chain_id = {}, TSS address = {:?}",
        ctx.accounts.signer.key(),
        chain_id,
        tss_address
    );

    Ok(())
}

// Updates the TSS address. Caller is authority stored in PDA.
pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;
    pda.tss_address = tss_address;
    pda.nonce = 0;

    msg!(
        "TSS address updated: new TSS address = {:?}, new nonce = {}, PDA authority = {}",
        tss_address,
        pda.nonce,
        ctx.accounts.signer.key()
    );

    Ok(())
}

// Updates the PDA authority. Caller is authority stored in PDA.
pub fn update_authority(
    ctx: Context<UpdateAuthority>,
    new_authority_address: Pubkey,
) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;
    pda.authority = new_authority_address;

    msg!(
        "PDA authority updated: new authority = {}, previous authority = {}",
        new_authority_address,
        ctx.accounts.signer.key()
    );

    Ok(())
}

// Pauses or unpauses deposits. Caller is authority stored in PDA.
pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;

    pda.deposit_paused = deposit_paused;

    msg!("Set deposit paused: {:?}", deposit_paused);
    Ok(())
}

// Whitelists a new SPL token. Caller is TSS
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

    // If signature is not zero, verify the signature is valid and signed by TSS
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
        // If signature is zero, authority must sign the transaction
        verify_authority(&authority.key(), &ctx.accounts.pda)?;
    }

    msg!(
        "SPL token whitelisted: mint = {}, whitelist_entry = {}, authority = {}",
        whitelist_candidate.key(),
        ctx.accounts.whitelist_entry.key(),
        ctx.accounts.authority.key()
    );

    Ok(())
}

// Unwhitelists an SPL token. Caller is TSS.
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

    // If signature is not zero, verify the signature is valid and signed by TSS
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
        // If signature is zero, authority must sign the transaction
        verify_authority(&authority.key(), &ctx.accounts.pda)?;
    }

    msg!(
        "SPL token unwhitelisted: mint = {}, whitelist_entry = {}, authority = {}",
        whitelist_candidate.key(),
        ctx.accounts.whitelist_entry.key(),
        ctx.accounts.authority.key()
    );

    Ok(())
}

// Resets the PDA authority. Caller is authority stored in PDA.
pub fn reset_nonce(ctx: Context<ResetNonce>, new_nonce: u64) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;
    pda.nonce = new_nonce;

    msg!("PDA nonce reset: new nonce = {}", new_nonce);

    Ok(())
}
