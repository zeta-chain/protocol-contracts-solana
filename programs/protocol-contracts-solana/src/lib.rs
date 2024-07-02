use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{transfer, Token, TokenAccount};
use solana_program::keccak::hash;
use solana_program::secp256k1_recover::secp256k1_recover;
use std::mem::size_of;
use spl_associated_token_account;

#[error_code]
pub enum Errors {
    #[msg("SignerIsNotAuthority")]
    SignerIsNotAuthority,
    #[msg("InsufficientPoints")]
    InsufficientPoints,
    #[msg("NonceMismatch")]
    NonceMismatch,
    #[msg("TSSAuthenticationFailed")]
    TSSAuthenticationFailed,
    #[msg("DepositToAddressMismatch")]
    DepositToAddressMismatch,
    #[msg("MessageHashMismatch")]
    MessageHashMismatch,
    #[msg("MemoLengthExceeded")]
    MemoLengthExceeded,
    #[msg("MemoLengthTooShort")]
    MemoLengthTooShort,
}

declare_id!("94U5AHQMKkV5txNJ17QPXWoh474PheGou6cNP2FEuL1d");


#[program]
pub mod gateway {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, tss_address:[u8; 20], chain_id: u64) -> Result<()> {
        let initialized_pda = &mut ctx.accounts.pda;
        initialized_pda.nonce = 0;
        initialized_pda.tss_address = tss_address;
        initialized_pda.authority = ctx.accounts.signer.key();
        initialized_pda.chain_id = chain_id;

        Ok(())
    }

    pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(ctx.accounts.signer.key() == pda.authority, Errors::SignerIsNotAuthority);
        pda.tss_address = tss_address;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, memo: Vec<u8>) -> Result<()> {
        require!(memo.len() >= 20, Errors::MemoLengthTooShort);
        require!(memo.len() <= 512, Errors::MemoLengthExceeded);
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.signer.to_account_info().clone(),
                to: ctx.accounts.pda.to_account_info().clone(),
            },
        );
        system_program::transfer(cpi_context, amount)?;
        msg!("{:?} deposits {:?} lamports to PDA", ctx.accounts.signer.key(), amount);

        Ok(())
    }

    pub fn deposit_spl_token(ctx: Context<DepositSplToken>, amount: u64, memo: Vec<u8>) -> Result<()> {
        require!(memo.len() >= 20, Errors::MemoLengthTooShort);
        require!(memo.len() <= 512, Errors::MemoLengthExceeded);
        let token = &ctx.accounts.token_program;
        let from = &ctx.accounts.from;

        let pda_ata = spl_associated_token_account::get_associated_token_address(
            &ctx.accounts.pda.key(),
            &from.mint,
        );
        // must deposit to the ATA from PDA in order to receive credit
        require!(pda_ata == ctx.accounts.to.to_account_info().key(), Errors::DepositToAddressMismatch);

        let xfer_ctx = CpiContext::new(
            token.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: ctx.accounts.signer.to_account_info(),
            },
        );
        transfer(xfer_ctx, amount)?;

        msg!("deposit spl token successfully");

        Ok(())
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        if nonce != pda.nonce {
            msg!("mismatch nonce");
            return err!(Errors::NonceMismatch);
        }
        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.to.key().to_bytes());
        require!(message_hash == hash(&concatenated_buffer[..]).to_bytes(), Errors::MessageHashMismatch);

        let address = recover_eth_address(&message_hash, recovery_id, &signature)?; // ethereum address is the last 20 Bytes of the hashed pubkey
        msg!("recovered address {:?}", address);
        if address != pda.tss_address {
            msg!("ECDSA signature error");
            return err!(Errors::TSSAuthenticationFailed);
        }

        // transfer amount of SOL from PDA to the payer
        pda.sub_lamports(amount)?;
        ctx.accounts.to.add_lamports(amount)?;

        pda.nonce += 1;

        Ok(())
    }

    pub fn withdraw_spl_token(
        ctx: Context<WithdrawSPLToken>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        // let program_id = &mut ctx.accounts
        if nonce != pda.nonce {
            msg!("mismatch nonce");
            return err!(Errors::NonceMismatch);
        }
        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.to.key().to_bytes());
        require!(message_hash == hash(&concatenated_buffer[..]).to_bytes(), Errors::MessageHashMismatch);

        let address = recover_eth_address(&message_hash, recovery_id, &signature)?; // ethereum address is the last 20 Bytes of the hashed pubkey
        msg!("recovered address {:?}", address);
        if address != pda.tss_address {
            msg!("ECDSA signature error");
            return err!(Errors::TSSAuthenticationFailed);
        }

        let token = &ctx.accounts.token_program;
        let signer_seeds: &[&[&[u8]]] = &[&[b"meta", &[ctx.bumps.pda]]];

        let xfer_ctx = CpiContext::new_with_signer(
            token.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.from.to_account_info(),
                to: ctx.accounts.to.to_account_info(),
                authority: pda.to_account_info(),
            },
            signer_seeds,
        );
        transfer(xfer_ctx, amount)?;
        msg!("withdraw spl token successfully");

        pda.nonce += 1;

        Ok(())
    }
}

fn recover_eth_address(
    message_hash: &[u8; 32],
    recovery_id: u8,
    signature: &[u8; 64],
) -> Result<[u8; 20]> {
    let pubkey = secp256k1_recover(message_hash, recovery_id, signature)
        .map_err(|_| ProgramError::InvalidArgument)?;

    // pubkey is 64 Bytes, uncompressed public secp256k1 public key
    let h = hash(pubkey.to_bytes().as_slice()).to_bytes();
    let address = &h.as_slice()[12..32]; // ethereum address is the last 20 Bytes of the hashed pubkey
    msg!("recovered address {:?}", address);

    let mut eth_address = [0u8; 20];
    eth_address.copy_from_slice(address);
    Ok(eth_address)
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(init, payer = signer, space = size_of::< Pda > () + 8, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub pda: Account<'info, Pda>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSplToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub from: Account<'info, TokenAccount>, // this must be owned by signer; normally the ATA of signer
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,   // this must be ATA of PDA
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub to: SystemAccount<'info>,
}

#[derive(Accounts)]
pub struct WithdrawSPLToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    #[account(mut)]
    pub from: Account<'info, TokenAccount>,

    #[account(mut)]
    pub to: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateTss<'info> {
    #[account(mut)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[account]
pub struct Pda {
    nonce: u64,            // ensure that each signature can only be used once
    tss_address: [u8; 20], // 20 bytes address format of ethereum
    authority: Pubkey,
    chain_id: u64,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let nonce: u64 = 0;
        let amount: u64 = 500_000;
        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        println!("concatenated_buffer: {:?}", concatenated_buffer);

        let message_hash = hash(&concatenated_buffer[..]).to_bytes();
        println!("message_hash: {:?}", message_hash);
    }
}