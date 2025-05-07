use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak::hash;
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::secp256k1_recover::secp256k1_recover;

use crate::errors::Errors;
use crate::state::Pda;

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
