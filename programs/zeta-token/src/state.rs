use anchor_lang::prelude::*;

/// PDA account storing program ZETA token state.
#[account]
pub struct ZetaTokenPda {
    /// The mint authority (ZetaConnector address)
    pub mint_authority: Pubkey,
    /// The burn authority (ZetaConnector address)
    pub burn_authority: Pubkey,
    /// The TSS address (20 bytes from EVM)
    pub tss_address: [u8; 20],
    /// The TSS address updater
    pub tss_address_updater: Pubkey,
    /// Maximum suppy of ZETA tokens
    pub max_supply: u64,
    /// Current total supply
    pub total_supply: u64,
    /// Token decimals (18 for ZETA)
    pub decimals: u8,
}

/// ZETA token mint account
#[account]
pub struct ZetaMint {
    /// The mint account for the ZETA token
    pub mint: Pubkey,
    /// The mint authority (ZetaConnector)
    pub mint_authority: Pubkey,
    /// The freeze authority (ZetaConnector)
    pub freeze_authority: Pubkey,
    /// Token decimals
    pub decimals: u8,
    /// Whether the mint is initialized
    pub is_initialized: bool,
}
