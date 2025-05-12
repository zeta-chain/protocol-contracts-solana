use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount};
use std::mem::size_of;

declare_id!("8iUjRRhUCn8BjrvsWPfj8mguTe9L81ES4oAUApiF8JFC");

// NOTE: this is just example contract that can be called from gateway in execute_spl_token function for testing withdraw and call spl
#[program]
pub mod connected_spl {
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
        let pda = &mut ctx.accounts.pda;

        // Store the sender's public key
        pda.last_sender = sender;

        // Convert data to a string and store it
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_message = message;

        // Transfer some portion of tokens transferred from gateway to another account
        let token = &ctx.accounts.token_program;
        let signer_seeds: &[&[&[u8]]] = &[&[b"connected", &[ctx.bumps.pda]]];

        let xfer_ctx = CpiContext::new_with_signer(
            token.to_account_info(),
            anchor_spl::token::TransferChecked {
                from: ctx.accounts.pda_ata.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.random_wallet_ata.to_account_info(),
                authority: pda.to_account_info(),
            },
            signer_seeds,
        );

        transfer_checked(xfer_ctx, amount / 2, 6)?;

        // Check if the message contains "revert" and return an error if so
        if pda.last_message.contains("revert") {
            msg!(
                "Reverting transaction due to message: '{}'",
                pda.last_message
            );
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

    pub fn on_revert(
        ctx: Context<OnRevert>,
        amount: u64,
        sender: Pubkey,
        data: Vec<u8>,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        // Store the sender's public key
        pda.last_revert_sender = sender;

        // Convert data to a string and store it
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_revert_message = message;

        // Transfer some portion of lamports transferred from gateway to another account
        // Check if the message contains "revert" and return an error if so
        if pda.last_revert_message.contains("revert") {
            msg!(
                "Reverting transaction due to message: '{}'",
                pda.last_revert_message
            );
            return Err(ErrorCode::RevertMessage.into());
        }

        msg!(
            "On revert executed with amount {}, sender {:?} and message {}",
            amount,
            pda.last_revert_sender,
            pda.last_revert_message
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

    #[account(mut)]
    pub pda_ata: Account<'info, TokenAccount>,

    pub mint_account: Account<'info, Mint>,

    /// CHECK: This is test program.
    pub gateway_pda: UncheckedAccount<'info>,

    /// CHECK: This is test program.
    pub random_wallet: UncheckedAccount<'info>,

    /// CHECK: This is test program.
    #[account(mut)]
    pub random_wallet_ata: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnRevert<'info> {
    #[account(mut, seeds = [b"connected"], bump)]
    pub pda: Account<'info, Pda>,

    #[account(mut)]
    pub pda_ata: Account<'info, TokenAccount>,

    pub mint_account: Account<'info, Mint>,

    /// CHECK: This is test program.
    pub gateway_pda: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Pda {
    pub last_sender: [u8; 20],
    pub last_message: String,
    pub last_revert_sender: Pubkey,
    pub last_revert_message: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The data provided could not be converted to a valid UTF-8 string.")]
    InvalidDataFormat,

    #[msg("Revert message detected. Transaction execution halted.")]
    RevertMessage,
}
