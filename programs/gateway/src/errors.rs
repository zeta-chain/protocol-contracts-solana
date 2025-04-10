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

/// Enumeration for instruction identifiers in message hashes.
#[repr(u8)]
pub enum InstructionId {
    Withdraw = 1,
    WithdrawSplToken = 2,
    WhitelistSplToken = 3,
    UnwhitelistSplToken = 4,
    Execute = 5,
    ExecuteSplToken = 6,
    IncrementNonce = 7,
    ExecuteRevert = 8,
    ExecuteSplTokenRevert = 9,
}
