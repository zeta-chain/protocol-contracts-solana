use anchor_lang::prelude::*;

/// Errors that can occur during execution.
#[error_code]
pub enum ZetaTokenErrors {
    #[msg("Invalid address provided.")]
    InvalidAddress,

    #[msg("Caller is not authorized.")]
    Unauthorized,

    #[msg("Caller is not the connector.")]
    CallerIsNotConnector,

    #[msg("Max supply would be exceeded.")]
    MaxSupplyExceeded,

    #[msg("Amount must be greater than 0.")]
    InvalidAmount,
}
