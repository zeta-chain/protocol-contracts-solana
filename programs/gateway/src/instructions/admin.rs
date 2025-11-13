use crate::{
    contexts::{
        AcceptAuthority, Initialize, MigratePda, ResetNonce, Unwhitelist, UpdateAuthority,
        UpdatePaused, UpdateTss, Whitelist,
    },
    errors::Errors,
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
    let bump = ctx.bumps.pda;

    **initialized_pda = Pda {
        nonce: 0,
        tss_address,
        authority: ctx.accounts.signer.key(),
        chain_id,
        deposit_paused: false,
        bump,
        nominated_authority: None,
    };

    msg!(
        "Gateway initialized: PDA authority = {}, chain_id = {}, TSS address = {:?}, bump = {}",
        ctx.accounts.signer.key(),
        chain_id,
        tss_address,
        bump
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

// Migrates existing PDA to new format by reallocating account space.
// This is a one-time migration that can be called after program upgrade.
pub fn migrate_pda(ctx: Context<MigratePda>) -> Result<()> {
    let pda_info = &ctx.accounts.pda.to_account_info();

    // Check if already migrated by comparing account size
    let current_size = pda_info.data_len();
    let new_size = 8 + std::mem::size_of::<Pda>();

    // If already migrated (size matches new size), do nothing
    if current_size >= new_size {
        msg!("PDA already migrated, skipping");
        return Ok(());
    }

    // The realloc constraint in the context has already resized the account
    // Now we need to properly initialize the new fields
    // We'll read the existing data, then write it back with new fields

    // Find the bump seed from the seeds
    let seeds = &[b"meta".as_ref()];
    let (_, bump) = Pubkey::find_program_address(seeds, ctx.program_id);

    // Now we can safely deserialize the PDA (account is already resized)
    // The new space will be uninitialized, but Anchor will handle it
    let pda = &mut ctx.accounts.pda;

    // Initialize new fields - they should be zero/uninitialized after realloc
    // Set bump if it's not already set (should be 0 for old PDAs)
    if pda.bump == 0 {
        pda.bump = bump;
    }
    // Initialize nominated_authority to None if not set
    if pda.nominated_authority.is_none() {
        pda.nominated_authority = None;
    }

    msg!("PDA migrated: bump = {}, new_size = {}", bump, new_size);

    Ok(())
}

// Nominates a new authority (step 1 of 2-step transfer).
// Caller is current authority stored in PDA.
pub fn nominate_authority(
    ctx: Context<UpdateAuthority>,
    new_authority_address: Pubkey,
) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;

    // Prevent self-nomination
    require!(
        new_authority_address != pda.authority,
        Errors::InvalidAuthority
    );

    pda.nominated_authority = Some(new_authority_address);

    msg!(
        "Authority nominated: nominated authority = {}, current authority = {}",
        new_authority_address,
        ctx.accounts.signer.key()
    );

    Ok(())
}

// Accepts authority nomination (step 2 of 2-step transfer).
// Caller is the nominated authority.
pub fn accept_authority(ctx: Context<AcceptAuthority>) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    let nominated = pda
        .nominated_authority
        .ok_or(Errors::NoNominatedAuthority)?;

    // Verify caller is the nominated authority
    require!(
        ctx.accounts.new_authority.key() == nominated,
        Errors::InvalidAuthority
    );

    let old_authority = pda.authority;
    pda.authority = nominated;
    pda.nominated_authority = None;

    msg!(
        "Authority accepted: new authority = {}, previous authority = {}",
        nominated,
        old_authority
    );

    Ok(())
}

// Cancels authority nomination (optional safety feature).
// Caller is current authority stored in PDA.
pub fn cancel_authority_nomination(ctx: Context<UpdateAuthority>) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;
    pda.nominated_authority = None;

    msg!("Authority nomination cancelled by current authority");
    Ok(())
}

// Legacy function: Updates the PDA authority in one step (deprecated).
// Caller is authority stored in PDA.
// This is kept for backward compatibility but should use 2-step process.
pub fn update_authority(
    ctx: Context<UpdateAuthority>,
    new_authority_address: Pubkey,
) -> Result<()> {
    verify_authority(&ctx.accounts.signer.key(), &ctx.accounts.pda)?;
    let pda = &mut ctx.accounts.pda;
    pda.authority = new_authority_address;
    // Clear any existing nomination
    pda.nominated_authority = None;

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
            &[
                &whitelist_candidate.key().to_bytes(),
                &ctx.accounts.whitelist_entry.key().to_bytes(),
            ],
            &message_hash,
            None, // No remaining accounts for admin operations
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
            &[
                &whitelist_candidate.key().to_bytes(),
                &ctx.accounts.whitelist_entry.key().to_bytes(),
            ],
            &message_hash,
            None, // No remaining accounts for admin operations
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
