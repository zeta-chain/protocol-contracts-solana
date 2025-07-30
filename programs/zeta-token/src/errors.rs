use anchor_lang::prelude::*;

/// Errors that can occur during execution.
#[error_code]
pub enum Errors {
    #[msg("InvalidAddress")]
    InvalidAddress,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("CallerIsNotConnector")]
    CallerIsNotConnector,
    #[msg("CallerIsNotConnector")]
    CallerIsNotConnector,
    #[msg("CallerIsNotTssUpdater")]
    CallerIsNotTssUpdater,
    #[msg("MaxSupplyExceeded")]
    MaxSupplyExceeded,
    #[msg("InvalidAmount")]
    InvalidAmount,
}
