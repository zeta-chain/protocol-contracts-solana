use anchor_lang::prelude::*;

mod contexts;
mod errors;
mod instructions;
mod state;

pub use contexts::*;
pub use errors::*;
pub use instructions::*;
pub use state::*;

// Define the program ID
declare_id!("EMNgcw2sH5wRKMqf9St4Rz1LEqvpZYCsTcE3hdgGsPD6");

#[program]
pub mod zeta_token {
    use super::*;

    /// Initialize the ZETA token program.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `tss_address` - The Ethereum TSS address (20 bytes).
    /// * `tss_address_updater` - The address that can update TSS address.
    /// * `max_supply` - Maximum supply of ZETA tokens.
    pub fn initialize(
        ctx: Context<Initialize>,
        tss_address: [u8; 20],
        tss_address_updater: Pubkey,
        max_supply: u64,
    ) -> Result<()> {
        instructions::admin::initialize(ctx, tss_address, tss_address_updater, max_supply)
    }

    /// Update TSS address.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `new_tss_address` - New TSS address (20 bytes).
    pub fn update_tss_address(
        ctx: Context<UpdateTssAddress>,
        new_tss_address: [u8; 20],
    ) -> Result<()> {
        instructions::admin::update_tss_address(ctx, new_tss_address)
    }

    /// Mint ZETA tokens to a recipient.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - Amount of tokens to mint.
    /// * `internal_send_hash` - Hash for internal tracking.
    pub fn mint(ctx: Context<MintZeta>, amount: u64, internal_send_hash: [u8; 32]) -> Result<()> {
        instructions::mint::mint_tokens(ctx, amount, internal_send_hash)
    }

    /// Burn ZETA tokens from an account.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - Amount of tokens to burn.
    pub fn burn(ctx: Context<BurnZeta>, amount: u64) -> Result<()> {
        instructions::burn::burn_tokens(ctx, amount)
    }
}
