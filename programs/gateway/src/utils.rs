use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address;
use solana_program::keccak::hash;
use solana_program::program_error::ProgramError;
use solana_program::secp256k1_recover::secp256k1_recover;

use crate::errors::Errors;
use crate::state::Pda;
use crate::errors::InstructionId;

/// Prefix used for outbounds message hashes.
pub const ZETACHAIN_PREFIX: &[u8] = b"ZETACHAIN";
/// Default gas cost in lamports for SPL token operations
pub const DEFAULT_GAS_COST: u64 = 5000;

// Verifies provided nonce is correct and updates pda nonce.
pub fn verify_and_update_nonce(pda: &mut Account<Pda>, nonce: u64) -> Result<()> {
    if nonce != pda.nonce {
        msg!(
            "Mismatch nonce: provided nonce = {}, expected nonce = {}",
            nonce,
            pda.nonce,
        );
        return err!(Errors::NonceMismatch);
    }
    pda.nonce += 1;
    Ok(())
}

/// Recovers and verifies eth address from signature.
pub fn recover_and_verify_eth_address(
    pda: &mut Account<Pda>,
    message_hash: &[u8; 32],
    recovery_id: u8,
    signature: &[u8; 64],
) -> Result<()> {
    let pubkey = secp256k1_recover(message_hash, recovery_id, signature)
        .map_err(|_| ProgramError::InvalidArgument)?;

    // pubkey is 64 Bytes, uncompressed public secp256k1 public key
    let h = hash(pubkey.to_bytes().as_slice()).to_bytes();
    let address = &h.as_slice()[12..32]; // ethereum address is the last 20 Bytes of the hashed pubkey
    msg!("Recovered address {:?}", address);

    let mut eth_address = [0u8; 20];
    eth_address.copy_from_slice(address);

    if eth_address != pda.tss_address {
        msg!("ECDSA signature error");
        return err!(Errors::TSSAuthenticationFailed);
    }

    Ok(())
}

/// Creates and validates a message hash for cross-chain instruction verification
/// Creates and validates a message hash for cross-chain instruction verification
/// with optional amount inclusion
pub fn validate_message_hash(
    instruction_id: InstructionId,
    chain_id: u64,
    nonce: u64,
    amount: Option<u64>, // Make amount optional
    additional_data: &[&[u8]],
    message_hash: &[u8; 32],
) -> Result<()> {
    let mut concatenated_buffer = Vec::new();

    concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
    concatenated_buffer.push(instruction_id as u8);
    concatenated_buffer.extend_from_slice(&chain_id.to_be_bytes());
    concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());

    // Only include amount in the hash if it's provided
    if let Some(amount_value) = amount {
        concatenated_buffer.extend_from_slice(&amount_value.to_be_bytes());
    }

    for data in additional_data {
        concatenated_buffer.extend_from_slice(data);
    }

    let computed_hash = hash(&concatenated_buffer[..]).to_bytes();
    require!(
        *message_hash == computed_hash,
        Errors::MessageHashMismatch
    );

    msg!("Computed message hash: {:?}", message_hash);

    Ok(())
}

/// Perform common cross-chain verification steps
pub fn validate_message(
    pda: &mut Account<Pda>,
    instruction_id: InstructionId,
    nonce: u64,
    amount: u64,
    additional_data: &[&[u8]],
    message_hash: &[u8; 32],
    signature: &[u8; 64],
    recovery_id: u8,
) -> Result<()> {
    verify_and_update_nonce(pda, nonce)?;

    validate_message_hash(
        instruction_id,
        pda.chain_id,
        nonce,
        Some(amount),
        additional_data,
        message_hash,
    )?;

    recover_and_verify_eth_address(pda, message_hash, recovery_id, signature)?;

    Ok(())
}

/// Verify ATA address matches expected value
pub fn verify_ata_match(owner: &Pubkey, mint: &Pubkey, actual_ata: &Pubkey) -> Result<()> {
    let expected_ata = get_associated_token_address(owner, mint);
    require!(
        expected_ata == *actual_ata,
        Errors::SPLAtaAndMintAddressMismatch
    );
    Ok(())
}

// Prepares account metas for withdraw and call, revert if unallowed account is passed
pub fn prepare_account_metas(
    remaining_accounts: &[AccountInfo],
    signer: &Signer,
    pda: &Account<Pda>,
) -> Result<Vec<solana_program::instruction::AccountMeta>> {
    use solana_program::instruction::AccountMeta;

    let mut account_metas = Vec::new();

    for account_info in remaining_accounts.iter() {
        let account_key = account_info.key;

        // Prevent signer from being included
        require!(account_key != signer.key, Errors::InvalidInstructionData);

        // Gateway pda can be added as not writable
        if *account_key == pda.key() {
            account_metas.push(AccountMeta::new_readonly(*account_key, false));
        } else {
            if account_info.is_writable {
                account_metas.push(AccountMeta::new(*account_key, false));
            } else {
                account_metas.push(AccountMeta::new_readonly(*account_key, false));
            }
        }
    }

    Ok(account_metas)
}
