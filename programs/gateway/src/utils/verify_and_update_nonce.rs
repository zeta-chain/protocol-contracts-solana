use crate::errors::Errors;
use crate::state::Pda;
use anchor_lang::prelude::*;

/// Verifies provided nonce is correct and updates pda nonce.
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
