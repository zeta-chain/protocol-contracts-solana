use anchor_lang::prelude::*;
use std::mem::size_of;

declare_id!("4xEw862A2SEwMjofPkUyd4NEekmVJKJsdHkK3UkAtDrc");

// NOTE: this is just example contract that can be called from gateway in execute function for testing withdraw and call
#[program]
pub mod connected {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn on_call(
        ctx: Context<OnCall>,
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        // Store the sender's public key
        pda.last_sender = sender;

        // Convert data to a string and store it
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_message = message;

        // Transfer some portion of lamports transfered from gateway to another account
        pda.sub_lamports(amount / 2)?;
        ctx.accounts.random_wallet.add_lamports(amount / 2)?;

        // Check if the message is "revert" and return an error if so
        if pda.last_message == "revert" {
            msg!("Reverting transaction due to 'revert' message.");
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

    /// CHECK: this is just test contract
    pub gateway_pda: UncheckedAccount<'info>,

    /// CHECK: this is just test contract
    pub random_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Pda {
    pub last_sender: [u8; 20],
    pub last_message: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The data provided could not be converted to a valid UTF-8 string.")]
    InvalidDataFormat,

    #[msg("Revert message detected. Transaction execution halted.")]
    RevertMessage,
}