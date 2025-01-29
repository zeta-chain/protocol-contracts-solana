use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::{get_associated_token_address, AssociatedToken};
use anchor_spl::token::{transfer, transfer_checked, Mint, Token, TokenAccount};
use solana_program::instruction::Instruction;
use solana_program::keccak::hash;
use solana_program::program::invoke;
use solana_program::secp256k1_recover::secp256k1_recover;
use spl_associated_token_account::instruction::create_associated_token_account;
use std::mem::size_of;
use std::str::FromStr;

declare_id!("4xEw862A2SEwMjofPkUyd4NEekmVJKJsdHkK3UkAtDrc");


#[program]
pub mod connected {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn on_call(ctx: Context<OnCall>, amount: u64, sender: Pubkey, data: Vec<u8>) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        // Store the sender's public key
        pda.last_sender = sender;

        // Convert data to a string and store it
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_message = message;

        // Transfer some portion of lamports transfered from gateway to another account
        pda.sub_lamports(amount/2)?;
        ctx.accounts.random_wallet.add_lamports(amount/2)?;

        msg!("On call executed with amount {}, sender {} and message {}", amount, pda.last_sender, pda.last_message);

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

    pub gateway_pda: UncheckedAccount<'info>,

    pub random_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct Pda {
    pub last_sender: Pubkey,
    pub last_message: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The data provided could not be converted to a valid UTF-8 string.")]
    InvalidDataFormat,
}