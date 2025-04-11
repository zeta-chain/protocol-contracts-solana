use anchor_lang::prelude::*;

use super::recover_and_verify_eth_address::recover_and_verify_eth_address;
use super::validate_message_hash::validate_message_hash;
use super::verify_and_update_nonce::verify_and_update_nonce;
use crate::state::InstructionId;
use crate::state::Pda;

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
