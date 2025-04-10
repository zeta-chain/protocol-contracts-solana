use crate::{errors::Errors, state::Pda};
use anchor_lang::prelude::*;
/// Verifies that the signer is the authority of the PDA
/// Returns an error if the signer is not authorized
pub fn verify_authority(signer: &Pubkey, pda: &Account<Pda>) -> Result<()> {
    require!(*signer == pda.authority, Errors::SignerIsNotAuthority);
    Ok(())
}
