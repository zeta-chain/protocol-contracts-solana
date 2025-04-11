use anchor_lang::prelude::*;

/// Errors that can occur during execution.
#[error_code]
pub enum Errors {
    #[msg("SignerIsNotAuthority")]
    SignerIsNotAuthority,
    #[msg("NonceMismatch")]
    NonceMismatch,
    #[msg("TSSAuthenticationFailed")]
    TSSAuthenticationFailed,
    #[msg("DepositToAddressMismatch")]
    DepositToAddressMismatch,
    #[msg("MessageHashMismatch")]
    MessageHashMismatch,
    #[msg("MemoLengthExceeded")]
    MemoLengthExceeded,
    #[msg("DepositPaused")]
    DepositPaused,
    #[msg("SPLAtaAndMintAddressMismatch")]
    SPLAtaAndMintAddressMismatch,
    #[msg("EmptyReceiver")]
    EmptyReceiver,
    #[msg("InvalidInstructionData")]
    InvalidInstructionData,
}
