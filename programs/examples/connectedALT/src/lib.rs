use anchor_lang::prelude::*;
use anchor_lang::solana_program::{sysvar, sysvar::instructions::get_instruction_relative};
use std::mem::size_of;

declare_id!("CKUn75XW5LQFRPZLScBuzdp2JaActxZQX4kZY1B9NryL");

// NOTE: this is just example contract that can be called from gateway in execute function for testing withdraw and call
// difference between `connected` and `connected_alt` is that this one requires 85 accounts in `on_call`
#[program]
pub mod connected_alt {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn on_call(
        ctx: Context<OnCall>,
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
    ) -> Result<()> {
        // Verify that the caller is the gateway program
        let current_ix = get_instruction_relative(
            0,
            &ctx.accounts.instruction_sysvar_account.to_account_info(),
        )
        .unwrap();

        msg!(
            "on_call invoked by: {}, gateway is {}",
            current_ix.program_id,
            gateway::ID
        );

        require!(
            current_ix.program_id == gateway::ID,
            ErrorCode::InvalidCaller
        );

        let pda = &mut ctx.accounts.pda;

        // Store sender and message
        pda.last_sender = sender;
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_message = message;

        // 1st used to get funds and 80 more for ALT demonstration
        require!(
            ctx.remaining_accounts.len() == 81,
            ErrorCode::InvalidRemainingAccounts
        );

        // Transfer half the amount to the first remaining account
        let target = &ctx.remaining_accounts[0];
        pda.sub_lamports(amount / 2)?;
        target.add_lamports(amount / 2)?;

        // Optional: revert on "revert" message
        if pda.last_message.contains("revert") {
            msg!("Reverting due to message: {}", pda.last_message);
            return Err(ErrorCode::RevertMessage.into());
        }

        msg!(
            "On call executed with amount {}, sender {:?} and message {}",
            amount,
            pda.last_sender,
            pda.last_message
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(init, payer = signer, space = size_of::<Pda>() + 32, seeds = [b"connected"], bump)]
    pub pda: Account<'info, Pda>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnCall<'info> {
    #[account(mut, seeds = [b"connected"], bump)]
    pub pda: Account<'info, Pda>,

    /// CHECK: gateway PDA
    pub gateway_pda: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: sysvar instructions
    #[account(address = sysvar::instructions::id())]
    pub instruction_sysvar_account: UncheckedAccount<'info>,
}

#[account]
pub struct Pda {
    pub last_sender: [u8; 20],
    pub last_message: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid UTF-8 data format.")]
    InvalidDataFormat,

    #[msg("Revert message detected.")]
    RevertMessage,

    #[msg("Caller is not the gateway program.")]
    InvalidCaller,

    #[msg("Missing required remaining accounts.")]
    InvalidRemainingAccounts,
}
