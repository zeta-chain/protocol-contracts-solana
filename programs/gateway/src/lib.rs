use anchor_lang::prelude::*;

mod contexts;
mod errors;
mod instructions;
mod state;
mod utils;

pub use contexts::*;
pub use errors::*;
pub use state::*;
pub use utils::DEPOSIT_FEE;

// Define the program ID
#[cfg(feature = "dev")]
declare_id!("94U5AHQMKkV5txNJ17QPXWoh474PheGou6cNP2FEuL1d");
#[cfg(not(feature = "dev"))]
declare_id!("ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis");

#[program]
pub mod gateway {
    use super::*;
    /// Initializes the gateway PDA.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `tss_address` - The Ethereum TSS address (20 bytes).
    /// * `chain_id` - The chain ID associated with the PDA.
    pub fn initialize(
        ctx: Context<Initialize>,
        tss_address: [u8; 20],
        chain_id: u64,
    ) -> Result<()> {
        instructions::admin::initialize(ctx, tss_address, chain_id)
    }

    /// Increments nonce, used by TSS in case outbound fails.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount in original outbound.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    /// * `failure_reason` - The reason for the failure of original outbound.
    pub fn increment_nonce(
        ctx: Context<IncrementNonce>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
        failure_reason: String,
    ) -> Result<()> {
        instructions::execute::increment_nonce(
            ctx,
            amount,
            signature,
            recovery_id,
            message_hash,
            nonce,
            failure_reason,
        )
    }

    /// Withdraws amount to destination program pda, and calls on_call on destination program
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - Amount of SOL to transfer.
    /// * `sender` - Sender's address.
    /// * `data` - Arbitrary data to pass to the destination program.
    /// * `signature` - Signature of the message.
    /// * `recovery_id` - Recovery ID of the signature.
    /// * `message_hash` - Hash of the message.
    /// * `nonce` - Nonce of the message.
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

    /// Withdraws amount to destination program pda, and calls on_revert on destination program
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SOL to withdraw.
    /// * `sender` - Sender from ZEVM.
    /// * `data` - Data to pass to destination program.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    pub fn execute_revert(
        ctx: Context<Execute>,
        amount: u64,
        sender: Pubkey,
        data: Vec<u8>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::execute::handle_sol_revert(
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

    /// Withdraws amount of SPL tokens to destination program pda, and calls on_call on destination program
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `decimals` - Token decimals for precision.
    /// * `amount` - The amount of tokens to withdraw.
    /// * `sender` - Sender from ZEVM.
    /// * `data` - Data to pass to destination program.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
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

    /// Withdraws SPL token amount to destination program pda, and calls on_revert on destination program
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `decimals` - Token decimals for precision.
    /// * `amount` - The amount of tokens to withdraw.
    /// * `sender` - Sender from ZEVM.
    /// * `data` - Data to pass to destination program.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    pub fn execute_spl_token_revert(
        ctx: Context<ExecuteSPLToken>,
        decimals: u8,
        amount: u64,
        sender: Pubkey,
        data: Vec<u8>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::execute::handle_spl_token_revert(
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

    /// Pauses or unpauses deposits. Caller is authority stored in PDA.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `deposit_paused` - Boolean flag to pause or unpause deposits.
    pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
        instructions::admin::set_deposit_paused(ctx, deposit_paused)
    }

    /// Updates the TSS address. Caller is authority stored in PDA.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `tss_address` - The new Ethereum TSS address (20 bytes).
    pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
        instructions::admin::update_tss(ctx, tss_address)
    }

    /// Updates the PDA authority. Caller is authority stored in PDA.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `new_authority_address` - The new authority's public key.
    pub fn update_authority(
        ctx: Context<UpdateAuthority>,
        new_authority_address: Pubkey,
    ) -> Result<()> {
        instructions::admin::update_authority(ctx, new_authority_address)
    }

    /// Resets the PDA nonce. Caller is authority stored in PDA.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `new_nonce` - The new nonce.
    pub fn reset_nonce(ctx: Context<ResetNonce>, new_nonce: u64) -> Result<()> {
        instructions::admin::reset_nonce(ctx, new_nonce)
    }

    /// Whitelists a new SPL token. Caller is TSS.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    pub fn whitelist_spl_mint(
        ctx: Context<Whitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::admin::whitelist_spl_mint(ctx, signature, recovery_id, message_hash, nonce)
    }

    /// Unwhitelists an SPL token. Caller is TSS.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    pub fn unwhitelist_spl_mint(
        ctx: Context<Unwhitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        instructions::admin::unwhitelist_spl_mint(ctx, signature, recovery_id, message_hash, nonce)
    }

    /// Deposits SOL into the program and credits the `receiver` on ZetaChain zEVM.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of lamports to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `revert_options` - The revert options created by the caller.
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20],
        revert_options: Option<RevertOptions>,
    ) -> Result<()> {
        instructions::deposit::handle_sol(ctx, amount, receiver, revert_options, DEPOSIT_FEE)
    }

    /// Deposits SOL and calls a contract on ZetaChain zEVM.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of lamports to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    /// * `revert_options` - The revert options created by the caller.
    pub fn deposit_and_call(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
        revert_options: Option<RevertOptions>,
    ) -> Result<()> {
        instructions::deposit::handle_sol_with_call(
            ctx,
            amount,
            receiver,
            message,
            revert_options,
            DEPOSIT_FEE,
        )
    }

    /// Deposits SPL tokens and credits the `receiver` on ZetaChain zEVM.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SPL tokens to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `revert_options` - The revert options created by the caller.
    pub fn deposit_spl_token(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
        revert_options: Option<RevertOptions>,
    ) -> Result<()> {
        instructions::deposit::handle_spl(ctx, amount, receiver, revert_options, DEPOSIT_FEE)
    }

    /// Deposits SPL tokens and calls a contract on ZetaChain zEVM.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SPL tokens to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    /// * `revert_options` - The revert options created by the caller.
    pub fn deposit_spl_token_and_call(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
        revert_options: Option<RevertOptions>,
    ) -> Result<()> {
        instructions::deposit::handle_spl_with_call(
            ctx,
            amount,
            receiver,
            message,
            revert_options,
            DEPOSIT_FEE,
        )
    }

    /// Calls a contract on ZetaChain zEVM.
    /// # Arguments
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    /// * `revert_options` - The revert options created by the caller.
    pub fn call(
        ctx: Context<Call>,
        receiver: [u8; 20],
        message: Vec<u8>,
        revert_options: Option<RevertOptions>,
    ) -> Result<()> {
        instructions::deposit::handle_call(ctx, receiver, message, revert_options)
    }

    /// Withdraws SOL. Caller is TSS.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SOL to withdraw.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
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

    /// Withdraws SPL tokens. Caller is TSS.
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `decimals` - Token decimals for precision.
    /// * `amount` - The amount of tokens to withdraw.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
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

    // Use the feature flag to conditionally compile the upgrade test function
    // This is used for localnet testing only and should not be included in the production build
    #[cfg(feature = "upgrade-test")]
    /// Returns true to indicate program has been upgraded
    pub fn upgraded(_ctx: Context<Upgrade>) -> Result<bool> {
        msg!("Program has been upgraded!");
        Ok(true)
    }
}
