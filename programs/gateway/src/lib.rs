use anchor_lang::prelude::*;

// Import your modules
mod contexts;
mod errors;
mod instructions;
mod state;
mod utils;

// Re-export needed items
pub use contexts::*;
pub use errors::*;
pub use state::*;

// Define the program ID
#[cfg(feature = "dev")]
declare_id!("94U5AHQMKkV5txNJ17QPXWoh474PheGou6cNP2FEuL1d");
#[cfg(not(feature = "dev"))]
declare_id!("ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis");

#[program]
pub mod gateway {
    use super::*;

    /// Deposit fee used when depositing SOL or SPL tokens.
    const DEPOSIT_FEE: u64 = 2_000_000;
    /// Max deposit payload size
    const MAX_DEPOSIT_PAYLOAD_SIZE: usize = 750;

    // Initialize instruction
    pub fn initialize(
        ctx: Context<Initialize>,
        tss_address: [u8; 20],
        chain_id: u64,
    ) -> Result<()> {
        instructions::admin::initialize(ctx, tss_address, chain_id)
    }

    // Increment nonce instruction
    pub fn increment_nonce(
        ctx: Context<IncrementNonce>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::execute::increment_nonce(
            ctx,
            amount,
            signature,
            recovery_id,
            message_hash,
            nonce,
        )
    }

    // Execute instruction
    pub fn execute(
        ctx: Context<Execute>,
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::execute::handle_sol(
            ctx,
            amount,
            sender,
            data,
            signature,
            recovery_id,
            message_hash,
            nonce,
        )
    }

    // Execute SPL Token instruction
    pub fn execute_spl_token(
        ctx: Context<ExecuteSPLToken>,
        decimals: u8,
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::execute::handle_spl_token(
            ctx,
            decimals,
            amount,
            sender,
            data,
            signature,
            recovery_id,
            message_hash,
            nonce,
        )
    }

    // Set deposit paused instruction
    pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
        instructions::admin::set_deposit_paused(ctx, deposit_paused)
    }

    // Update TSS address instruction
    pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
        instructions::admin::update_tss(ctx, tss_address)
    }

    // Update authority instruction
    pub fn update_authority(
        ctx: Context<UpdateAuthority>,
        new_authority_address: Pubkey,
    ) -> Result<()> {
        instructions::admin::update_authority(ctx, new_authority_address)
    }

    // Whitelist SPL mint instruction
    pub fn whitelist_spl_mint(
        ctx: Context<Whitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::admin::whitelist_spl_mint(ctx, signature, recovery_id, message_hash, nonce)
    }

    // Unwhitelist SPL mint instruction
    pub fn unwhitelist_spl_mint(
        ctx: Context<Unwhitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::admin::unwhitelist_spl_mint(ctx, signature, recovery_id, message_hash, nonce)
    }

    // Deposit instruction
    pub fn deposit(ctx: Context<Deposit>, amount: u64, receiver: [u8; 20]) -> Result<()> {
        instructions::deposit::handle_sol(ctx, amount, receiver, DEPOSIT_FEE)
    }

    // Deposit and call instruction
    pub fn deposit_and_call(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        instructions::deposit::handle_sol_with_call(
            ctx,
            amount,
            receiver,
            message,
            DEPOSIT_FEE,
            MAX_DEPOSIT_PAYLOAD_SIZE,
        )
    }

    // Deposit SPL token instruction
    pub fn deposit_spl_token(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
    ) -> Result<()> {
        instructions::deposit::handle_spl(ctx, amount, receiver, DEPOSIT_FEE)
    }

    // Deposit SPL token and call instruction
    pub fn deposit_spl_token_and_call(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        instructions::deposit::handle_spl_with_call(
            ctx,
            amount,
            receiver,
            message,
            DEPOSIT_FEE,
            MAX_DEPOSIT_PAYLOAD_SIZE,
        )
    }

    // Call instruction
    pub fn call(_ctx: Context<Call>, receiver: [u8; 20], message: Vec<u8>) -> Result<()> {
        instructions::deposit::handle_call(receiver, message, MAX_DEPOSIT_PAYLOAD_SIZE)
    }

    // Withdraw instruction
    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::withdraw::handle_sol(ctx, amount, signature, recovery_id, message_hash, nonce)
    }

    // Withdraw SPL token instruction
    pub fn withdraw_spl_token(
        ctx: Context<WithdrawSPLToken>,
        decimals: u8,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::withdraw::handle_spl(
            ctx,
            decimals,
            amount,
            signature,
            recovery_id,
            message_hash,
            nonce,
        )
    }

    /// Returns true to indicate program has been upgraded
    pub fn upgraded(_ctx: Context<Upgrade>) -> Result<bool> {
        msg!("Program has been upgraded!");
        Ok(true)
    }
}
