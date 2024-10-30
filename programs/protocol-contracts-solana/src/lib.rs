use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::{get_associated_token_address, AssociatedToken};
use anchor_spl::token::{transfer, transfer_checked, Mint, Token, TokenAccount};
use solana_program::keccak::hash;
use solana_program::program::invoke;
use solana_program::secp256k1_recover::secp256k1_recover;
use spl_associated_token_account::instruction::create_associated_token_account;
use std::mem::size_of;

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
    #[msg("DepositPaused")]
    DepositPaused,
    #[msg("SPLAtaAndMintAddressMismatch")]
    SPLAtaAndMintAddressMismatch,
}

declare_id!("ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis");

#[program]
pub mod gateway {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        tss_address: [u8; 20],
        chain_id: u64,
    ) -> Result<()> {
        let initialized_pda = &mut ctx.accounts.pda;
        initialized_pda.nonce = 0;
        initialized_pda.tss_address = tss_address;
        initialized_pda.authority = ctx.accounts.signer.key();
        initialized_pda.chain_id = chain_id;
        initialized_pda.deposit_paused = false;

        Ok(())
    }

    // admin function to pause or unpause deposit
    pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(
            ctx.accounts.signer.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
        pda.deposit_paused = deposit_paused;
        msg!("set_deposit_paused: {:?}", deposit_paused);
        Ok(())
    }

    // the authority stored in PDA can call this instruction to update tss address
    pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(
            ctx.accounts.signer.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
        pda.tss_address = tss_address;
        Ok(())
    }

    // the authority stored in PDA can call this instruction to update the authority address
    pub fn update_authority(
        ctx: Context<UpdateAuthority>,
        new_authority_address: Pubkey,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(
            ctx.accounts.signer.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
        pda.authority = new_authority_address;
        Ok(())
    }

    pub fn whitelist_spl_mint(
        ctx: Context<Whitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        validate_signature_or_authority(
            pda,
            &ctx.accounts.authority,
            signature,
            recovery_id,
            message_hash,
            nonce,
            "whitelist_spl_mint",
        )?;

        Ok(())
    }

    pub fn unwhitelist_spl_mint(
        ctx: Context<Unwhitelist>,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        validate_signature_or_authority(
            pda,
            &ctx.accounts.authority,
            signature,
            recovery_id,
            message_hash,
            nonce,
            "unwhitelist_spl_mint",
        )?;

        Ok(())
    }

    pub fn initialize_rent_payer(_ctx: Context<InitializeRentPayer>) -> Result<()> {
        Ok(())
    }

    // deposit SOL into this program and the `receiver` on ZetaChain zEVM
    // will get corresponding ZRC20 credit.
    // amount: amount of lamports (10^-9 SOL) to deposit
    // receiver: ethereum address (20Bytes) of the receiver on ZetaChain zEVM
    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20], // not used in this program; for directing zetachain protocol only
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(!pda.deposit_paused, Errors::DepositPaused);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.signer.to_account_info().clone(),
                to: ctx.accounts.pda.to_account_info().clone(),
            },
        );
        system_program::transfer(cpi_context, amount)?;
        msg!(
            "{:?} deposits {:?} lamports to PDA; receiver {:?}",
            ctx.accounts.signer.key(),
            amount,
            receiver,
        );

        Ok(())
    }

    // deposit SOL into this program and the `receiver` on ZetaChain zEVM
    // will get corresponding ZRC20 credit. The `receiver` should be a contract
    // on zEVM and the `message` will be used as input data for the contract call.
    // The `receiver` contract on zEVM will get the SOL ZRC20 credit and receive the `message`.
    pub fn deposit_and_call(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        require!(message.len() <= 512, Errors::MemoLengthExceeded);
        deposit(ctx, amount, receiver)?;
        Ok(())
    }

    // deposit SPL token into this program and the `receiver` on ZetaChain zEVM
    // will get corresponding ZRC20 credit.
    // amount: amount of SPL token to deposit
    // receiver: ethereum address (20Bytes) of the receiver on ZetaChain zEVM
    #[allow(unused)]
    pub fn deposit_spl_token(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20], // unused in this program; for directing zetachain protocol only
    ) -> Result<()> {
        let token = &ctx.accounts.token_program;
        let from = &ctx.accounts.from;

        let pda = &mut ctx.accounts.pda;
        require!(!pda.deposit_paused, Errors::DepositPaused);

        let pda_ata = get_associated_token_address(&ctx.accounts.pda.key(), &from.mint);
        // must deposit to the ATA from PDA in order to receive credit
        require!(
            pda_ata == ctx.accounts.to.to_account_info().key(),
            Errors::DepositToAddressMismatch
        );

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

    // like `deposit_spl_token` instruction,
    // deposit SPL token into this program and the `receiver` on ZetaChain zEVM
    // will get corresponding ZRC20 credit. The `receiver` should be a contract
    // on zEVM and the `message` will be used as input data for the contract call.
    // The `receiver` contract on zEVM will get the SPL token ZRC20 credit and receive the `message`.
    #[allow(unused)]
    pub fn deposit_spl_token_and_call(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        require!(message.len() <= 512, Errors::MemoLengthExceeded);
        deposit_spl_token(ctx, amount, receiver)?;
        Ok(())
    }

    // require tss address signature on the internal message defined in the following
    // concatenated_buffer vec.
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
        concatenated_buffer.extend_from_slice("withdraw".as_bytes());
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.to.key().to_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        let address = recover_eth_address(&message_hash, recovery_id, &signature)?; // ethereum address is the last 20 Bytes of the hashed pubkey
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

    // require tss address signature on the internal message defined in the following
    // concatenated_buffer vec.
    pub fn withdraw_spl_token(
        ctx: Context<WithdrawSPLToken>,
        decimals: u8,
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
        concatenated_buffer.extend_from_slice("withdraw_spl_token".as_bytes());
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.mint_account.key().to_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.recipient_ata.key().to_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        let address = recover_eth_address(&message_hash, recovery_id, &signature)?; // ethereum address is the last 20 Bytes of the hashed pubkey
        msg!("recovered address {:?}", address);
        if address != pda.tss_address {
            msg!("ECDSA signature error");
            return err!(Errors::TSSAuthenticationFailed);
        }

        // associated token address (ATA) of the program PDA
        // the PDA is the "wallet" (owner) of the token account
        // the token is stored in ATA account owned by the PDA
        let pda_ata = get_associated_token_address(&pda.key(), &ctx.accounts.mint_account.key());
        require!(
            pda_ata == ctx.accounts.pda_ata.to_account_info().key(),
            Errors::SPLAtaAndMintAddressMismatch,
        );

        let token = &ctx.accounts.token_program;
        let signer_seeds: &[&[&[u8]]] = &[&[b"meta", &[ctx.bumps.pda]]];

        // make sure that ctx.accounts.recipient_ata is ATA (PDA account of token program)
        let recipient_ata = get_associated_token_address(
            &ctx.accounts.recipient.key(),
            &ctx.accounts.mint_account.key(),
        );
        require!(
            recipient_ata == ctx.accounts.recipient_ata.to_account_info().key(),
            Errors::SPLAtaAndMintAddressMismatch,
        );

        // test whether the recipient_ata is created or not; if not, create it
        let recipient_ata_account = ctx.accounts.recipient_ata.to_account_info();
        if recipient_ata_account.lamports() == 0
            || *recipient_ata_account.owner == ctx.accounts.system_program.key()
        {
            // if lamports of recipient_ata_account is 0 or its owner being system program then it's not created
            msg!(
                "Creating associated token account {:?} for recipient {:?}...",
                recipient_ata_account.key(),
                ctx.accounts.recipient.key(),
            );
            let signer_info = &ctx.accounts.signer.to_account_info();
            let bal_before = signer_info.lamports();
            invoke(
                &create_associated_token_account(
                    ctx.accounts.signer.to_account_info().key,
                    ctx.accounts.recipient.to_account_info().key,
                    ctx.accounts.mint_account.to_account_info().key,
                    ctx.accounts.token_program.key,
                ),
                &[
                    ctx.accounts.mint_account.to_account_info().clone(),
                    ctx.accounts.recipient_ata.clone(),
                    ctx.accounts.recipient.to_account_info().clone(),
                    ctx.accounts.signer.to_account_info().clone(),
                    ctx.accounts.system_program.to_account_info().clone(),
                    ctx.accounts.token_program.to_account_info().clone(),
                    ctx.accounts
                        .associated_token_program
                        .to_account_info()
                        .clone(),
                ],
            )?;
            let bal_after = signer_info.lamports();

            msg!("Associated token account for recipient created!");
            msg!(
                "Refunding the rent paid by the signer {:?}",
                ctx.accounts.signer.to_account_info().key
            );

            let rent_payer_info = ctx.accounts.rent_payer_pda.to_account_info();
            let cost = bal_before - bal_after;
            rent_payer_info.sub_lamports(cost)?;
            signer_info.add_lamports(cost)?;
            msg!(
                "Signer refunded the ATA account creation rent amount {:?} lamports",
                cost,
            );
        }

        let xfer_ctx = CpiContext::new_with_signer(
            token.to_account_info(),
            anchor_spl::token::TransferChecked {
                from: ctx.accounts.pda_ata.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.recipient_ata.to_account_info(),
                authority: pda.to_account_info(),
            },
            signer_seeds,
        );

        pda.nonce += 1;

        transfer_checked(xfer_ctx, amount, decimals)?;
        msg!("withdraw spl token successfully");

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

fn validate_signature_or_authority(
    pda: &mut Account<Pda>,
    authority: &Signer,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
    purpose: &str,
) -> Result<()> {
    // signature provided, recover and verify that tss is the signer
    if signature != [0u8; 64] {
        if nonce != pda.nonce {
            msg!("mismatch nonce");
            return err!(Errors::NonceMismatch);
        }

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(purpose.as_bytes());
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        let address = recover_eth_address(&message_hash, recovery_id, &signature)?;
        if address != pda.tss_address {
            msg!("ECDSA signature error");
            return err!(Errors::TSSAuthenticationFailed);
        }

        pda.nonce += 1;
    } else {
        // no signature provided, fallback to authority check
        require!(
            authority.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
    }

    Ok(())
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

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositSplToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    #[account(seeds=[b"whitelist", mint_account.key().as_ref()], bump)]
    pub whitelist_entry: Account<'info, WhitelistEntry>, // attach whitelist entry to show the mint_account is whitelisted

    pub mint_account: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,

    #[account(mut)]
    pub from: Account<'info, TokenAccount>, // this must be owned by signer; normally the ATA of signer
    #[account(mut)]
    pub to: Account<'info, TokenAccount>, // this must be ATA of PDA
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    /// CHECK: to account is not read so no need to check its owners; the program neither knows nor cares who the owner is.
    #[account(mut)]
    pub to: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct WithdrawSPLToken<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    #[account(mut, associated_token::mint = mint_account, associated_token::authority = pda)]
    pub pda_ata: Account<'info, TokenAccount>, // associated token address of PDA

    pub mint_account: Account<'info, Mint>,

    pub recipient: SystemAccount<'info>,
    /// CHECK: recipient_ata might not have been created; avoid checking its content.
    /// the validation will be done in the instruction processor.
    #[account(mut)]
    pub recipient_ata: AccountInfo<'info>,

    #[account(mut, seeds = [b"rent-payer"], bump)]
    pub rent_payer_pda: Account<'info, RentPayerPda>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateTss<'info> {
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdatePaused<'info> {
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct Whitelist<'info> {
    #[account(
        init,
        space=8,
        payer=authority,
        seeds=[
            b"whitelist",
            whitelist_candidate.key().as_ref()
        ],
        bump
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub whitelist_candidate: Account<'info, Mint>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Unwhitelist<'info> {
    #[account(
        mut,
        seeds=[
            b"whitelist",
            whitelist_candidate.key().as_ref()
        ],
        bump,
        close = authority,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,
    pub whitelist_candidate: Account<'info, Mint>,

    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeRentPayer<'info> {
    #[account(init, payer = authority, space = 8, seeds = [b"rent-payer"], bump)]
    pub rent_payer_pda: Account<'info, RentPayerPda>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Pda {
    nonce: u64,            // ensure that each signature can only be used once
    tss_address: [u8; 20], // 20 bytes address format of ethereum
    authority: Pubkey,
    chain_id: u64,
    deposit_paused: bool,
}

#[account]
pub struct WhitelistEntry {}

#[account]
pub struct RentPayerPda {}

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
