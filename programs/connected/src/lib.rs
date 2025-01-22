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

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum FooInstruction {
    Foo {
        // amount: u64,
        // receiver: [u8; 20],
    },
}

impl FooInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf;
        match self {
            FooInstruction::Foo {} => {
                buf = Vec::with_capacity(8);

                // Add the discriminator for "global:foo"
                buf.extend_from_slice(&[167, 117, 211, 79, 251, 254, 47, 135]);

                // // Add the `amount` as a little-endian 8-byte array
                // buf.extend_from_slice(&amount.to_le_bytes());

                // // Add the `receiver` (20-byte Ethereum address)
                // buf.extend_from_slice(receiver);
            }
        }
        buf
    }
}

#[program]
pub mod connected {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn on_call(ctx: Context<OnCall>, sender: Pubkey, data: Vec<u8>) -> Result<()> {
        let instruction_data = FooInstruction::Foo {}.pack();
        msg!("On call executed {:?}", instruction_data);

        let account_metas = vec![
            // AccountMeta::new(ctx.accounts.signer.to_account_info().key(), true),
            // AccountMeta::new(ctx.accounts.gpda.to_account_info().key(), false),
            // AccountMeta::new_readonly(ctx.accounts.system_program.to_account_info().key(), false),
        ];

        let ix: Instruction = Instruction {
            program_id: ctx.accounts.gateway_program.key(),
            accounts: account_metas,
            data: instruction_data,
        };

        invoke(
            &ix,
            // &[ctx.accounts.system_program.to_account_info().clone()],
            &[],
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(init, payer = signer, space = size_of::<Pda>() + 8, seeds = [b"connected"], bump)]
    pub pda: Account<'info, Pda>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnCall<'info> {
    pub system_program: Program<'info, System>,

    pub gateway_program: AccountInfo<'info>,
}

#[account]
pub struct Pda {}
