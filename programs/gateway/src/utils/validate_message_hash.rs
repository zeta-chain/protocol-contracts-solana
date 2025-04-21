use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak::hash;

use super::constants::ZETACHAIN_PREFIX;
use crate::errors::Errors;
use crate::state::InstructionId;

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
    require!(*message_hash == computed_hash, Errors::MessageHashMismatch);

    msg!("Computed message hash: {:?}", message_hash);

    Ok(())
}
