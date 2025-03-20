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

/// Errors that can occur during execution.
#[error_code]
pub enum Errors {
    #[msg("SignerIsNotAuthority")]
    SignerIsNotAuthority,
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
    #[msg("DepositPaused")]
    DepositPaused,
    #[msg("SPLAtaAndMintAddressMismatch")]
    SPLAtaAndMintAddressMismatch,
    #[msg("EmptyReceiver")]
    EmptyReceiver,
    #[msg("InvalidInstructionData")]
    InvalidInstructionData,
}

/// Enumeration for instruction identifiers in message hashes.
#[repr(u8)]
enum InstructionId {
    Withdraw = 1,
    WithdrawSplToken = 2,
    WhitelistSplToken = 3,
    UnwhitelistSplToken = 4,
    Execute = 5,
    ExecuteSplToken = 6,
    IncrementNonce = 7,
}

#[cfg(feature = "dev")]
declare_id!("94U5AHQMKkV5txNJ17QPXWoh474PheGou6cNP2FEuL1d");
#[cfg(not(feature = "dev"))]
declare_id!("ZETAjseVjuFsxdRxo6MmTCvqFwb3ZHUx56Co3vCmGis");

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum CallableInstruction {
    OnCall {
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
    },
}

impl CallableInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf;
        match self {
            CallableInstruction::OnCall {
                amount,
                sender,
                data,
            } => {
                let data_len = data.len() as u32;

                //8 (discriminator) + 8 (u64 amount) + 20 (sender) + 4 (data length)
                buf = Vec::with_capacity(40 + data_len as usize);

                // Discriminator for instruction (example)
                // This ensures the program knows how to handle this instruction.
                // Example discriminator: anchor typically uses `hash("global:on_call")`
                buf.extend_from_slice(&[16, 136, 66, 32, 254, 40, 181, 8]);

                // Encode amount (u64) in little-endian format
                buf.extend_from_slice(&amount.to_le_bytes());

                // Encode sender ([u8; 20])
                buf.extend_from_slice(sender);

                // Encode the length of the data array (u32)
                buf.extend_from_slice(&data_len.to_le_bytes());

                // Encode the data itself
                buf.extend_from_slice(data);
            }
        }
        buf
    }
}

#[program]
pub mod gateway {
    use super::*;

    /// Deposit fee used when depositing SOL or SPL tokens.
    const DEPOSIT_FEE: u64 = 2_000_000;
    /// Prefix used for outbounds message hashes.
    pub const ZETACHAIN_PREFIX: &[u8] = b"ZETACHAIN";
    /// Max deposit payload size
    const MAX_DEPOSIT_PAYLOAD_SIZE: usize = 750;

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
        let initialized_pda = &mut ctx.accounts.pda;

        **initialized_pda = Pda {
            nonce: 0,
            tss_address,
            authority: ctx.accounts.signer.key(),
            chain_id,
            deposit_paused: false,
        };

        msg!(
            "Gateway initialized: PDA authority = {}, chain_id = {}, TSS address = {:?}",
            ctx.accounts.signer.key(),
            chain_id,
            tss_address
        );

        Ok(())
    }

    /// Increments nonce, used by TSS in case outbound fails.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount in original outbound.
    /// * `signature` - The TSS signature.
    /// * `recovery_id` - The recovery ID for signature verification.
    /// * `message_hash` - Message hash for signature verification.
    /// * `nonce` - The current nonce value.
    pub fn increment_nonce(
        ctx: Context<IncrementNonce>,
        amount: u64,
        signature: [u8; 64],
        recovery_id: u8,
        message_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        verify_and_update_nonce(pda, nonce)?;

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
        concatenated_buffer.push(InstructionId::IncrementNonce as u8);
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        msg!("Computed message hash: {:?}", message_hash);

        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;

        Ok(())
    }

    /// Withdraws amount to destination program pda, and calls on_call on destination program
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
        let pda = &mut ctx.accounts.pda;

        verify_and_update_nonce(pda, nonce)?;

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
        concatenated_buffer.push(InstructionId::Execute as u8);
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.destination_program.key().to_bytes());
        concatenated_buffer.extend_from_slice(&data);
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        msg!("Computed message hash: {:?}", message_hash);

        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;

        // NOTE: have to manually create Instruction, pack it and invoke since there is no crate for contract
        // since any contract with on_call instruction can be called
        let instruction_data = CallableInstruction::OnCall {
            amount,
            sender,
            data,
        }
        .pack();

        // account metas for remaining accounts
        let account_metas =
            prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

        let ix = Instruction {
            program_id: ctx.accounts.destination_program.key(),
            accounts: account_metas,
            data: instruction_data,
        };

        // withdraw to destination program pda
        pda.sub_lamports(amount)?;
        ctx.accounts.destination_program_pda.add_lamports(amount)?;

        // invoke destination program on_call function
        invoke(&ix, ctx.remaining_accounts)?;

        msg!(
            "Execute done: destination contract = {}, amount = {}, sender = {:?}",
            amount,
            ctx.accounts.destination_program.key(),
            sender,
        );

        Ok(())
    }

    /// Execute with SPL tokens. Caller is TSS.
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
        let pda = &mut ctx.accounts.pda;
        verify_and_update_nonce(pda, nonce)?;

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
        concatenated_buffer.push(InstructionId::ExecuteSplToken as u8);
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.mint_account.key().to_bytes());
        concatenated_buffer
            .extend_from_slice(&ctx.accounts.destination_program_pda_ata.key().to_bytes());
        concatenated_buffer.extend_from_slice(&data);
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        msg!("Computed message hash: {:?}", message_hash);

        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?; // ethereum address is the last 20 Bytes of the hashed pubkey

        // NOTE: have to manually create Instruction, pack it and invoke since there is no crate for contract
        // since any contract with on_call instruction can be called
        let instruction_data = CallableInstruction::OnCall {
            amount,
            sender,
            data,
        }
        .pack();

        // account metas for remaining accounts
        let account_metas =
            prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

        let ix = Instruction {
            program_id: ctx.accounts.destination_program.key(),
            accounts: account_metas,
            data: instruction_data,
        };

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

        // make sure that ctx.accounts.destination_program_pda_ata is ATA of destination_program
        let recipient_ata = get_associated_token_address(
            &ctx.accounts.destination_program_pda.key(),
            &ctx.accounts.mint_account.key(),
        );
        require!(
            recipient_ata
                == ctx
                    .accounts
                    .destination_program_pda_ata
                    .to_account_info()
                    .key(),
            Errors::SPLAtaAndMintAddressMismatch,
        );
        // withdraw to destination program pda
        let xfer_ctx = CpiContext::new_with_signer(
            token.to_account_info(),
            anchor_spl::token::TransferChecked {
                from: ctx.accounts.pda_ata.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                to: ctx.accounts.destination_program_pda_ata.to_account_info(),
                authority: pda.to_account_info(),
            },
            signer_seeds,
        );

        transfer_checked(xfer_ctx, amount, decimals)?;

        // invoke destination program on_call function
        invoke(&ix, ctx.remaining_accounts)?;

        msg!(
            "Execute SPL done: amount = {}, decimals = {}, recipient = {}, mint = {}, pda = {}",
            amount,
            decimals,
            ctx.accounts.destination_program_pda.key(),
            ctx.accounts.mint_account.key(),
            ctx.accounts.pda.key()
        );

        Ok(())
    }

    /// Pauses or unpauses deposits. Caller is authority stored in PDA.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `deposit_paused` - Boolean flag to pause or unpause deposits.
    pub fn set_deposit_paused(ctx: Context<UpdatePaused>, deposit_paused: bool) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(
            ctx.accounts.signer.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
        pda.deposit_paused = deposit_paused;

        msg!("Set deposit paused: {:?}", deposit_paused);
        Ok(())
    }

    /// Updates the TSS address. Caller is authority stored in PDA.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `tss_address` - The new Ethereum TSS address (20 bytes).
    pub fn update_tss(ctx: Context<UpdateTss>, tss_address: [u8; 20]) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(
            ctx.accounts.signer.key() == pda.authority,
            Errors::SignerIsNotAuthority
        );
        pda.tss_address = tss_address;

        msg!(
            "TSS address updated: new TSS address = {:?}, PDA authority = {}",
            tss_address,
            ctx.accounts.signer.key()
        );

        Ok(())
    }

    /// Updates the PDA authority. Caller is authority stored in PDA.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `new_authority_address` - The new authority's public key.
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

        msg!(
            "PDA authority updated: new authority = {}, previous authority = {}",
            new_authority_address,
            ctx.accounts.signer.key()
        );

        Ok(())
    }

    /// Whitelists a new SPL token. Caller is TSS.
    ///
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
        let pda = &mut ctx.accounts.pda;
        let whitelist_candidate = &mut ctx.accounts.whitelist_candidate;
        let authority = &ctx.accounts.authority;

        if signature != [0u8; 64] {
            validate_whitelist_tss_signature(
                pda,
                whitelist_candidate,
                signature,
                recovery_id,
                message_hash,
                nonce,
                InstructionId::WhitelistSplToken as u8,
            )?;
        } else {
            require!(
                authority.key() == pda.authority,
                Errors::SignerIsNotAuthority
            );
        }

        msg!(
            "SPL token whitelisted: mint = {}, whitelist_entry = {}, authority = {}",
            whitelist_candidate.key(),
            ctx.accounts.whitelist_entry.key(),
            ctx.accounts.authority.key()
        );

        Ok(())
    }

    /// Unwhitelists an SPL token. Caller is TSS.
    ///
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
        let pda = &mut ctx.accounts.pda;
        let whitelist_candidate: &mut Account<'_, Mint> = &mut ctx.accounts.whitelist_candidate;
        let authority = &ctx.accounts.authority;

        if signature != [0u8; 64] {
            validate_whitelist_tss_signature(
                pda,
                whitelist_candidate,
                signature,
                recovery_id,
                message_hash,
                nonce,
                InstructionId::UnwhitelistSplToken as u8,
            )?;
        } else {
            require!(
                authority.key() == pda.authority,
                Errors::SignerIsNotAuthority
            );
        }

        msg!(
            "SPL token unwhitelisted: mint = {}, whitelist_entry = {}, authority = {}",
            whitelist_candidate.key(),
            ctx.accounts.whitelist_entry.key(),
            ctx.accounts.authority.key()
        );

        Ok(())
    }

    /// Deposits SOL into the program and credits the `receiver` on ZetaChain zEVM.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of lamports to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    pub fn deposit(ctx: Context<Deposit>, amount: u64, receiver: [u8; 20]) -> Result<()> {
        let pda = &mut ctx.accounts.pda;
        require!(!pda.deposit_paused, Errors::DepositPaused);
        require!(receiver != [0u8; 20], Errors::EmptyReceiver);

        let amount_with_fees = amount + DEPOSIT_FEE;
        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.signer.to_account_info().clone(),
                to: ctx.accounts.pda.to_account_info().clone(),
            },
        );
        system_program::transfer(cpi_context, amount_with_fees)?;

        msg!(
            "Deposit executed: amount = {}, fee = {}, receiver = {:?}, pda = {}",
            amount,
            DEPOSIT_FEE,
            receiver,
            ctx.accounts.pda.key()
        );

        Ok(())
    }

    /// Deposits SOL and calls a contract on ZetaChain zEVM.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of lamports to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    pub fn deposit_and_call(
        ctx: Context<Deposit>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        require!(
            message.len() <= MAX_DEPOSIT_PAYLOAD_SIZE,
            Errors::MemoLengthExceeded
        );
        deposit(ctx, amount, receiver)?;

        msg!("Deposit and call executed with message = {:?}", message);

        Ok(())
    }

    /// Deposits SPL tokens and credits the `receiver` on ZetaChain zEVM.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SPL tokens to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    pub fn deposit_spl_token(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
    ) -> Result<()> {
        let token = &ctx.accounts.token_program;
        let from = &ctx.accounts.from;

        let pda = &mut ctx.accounts.pda;
        require!(!pda.deposit_paused, Errors::DepositPaused);
        require!(receiver != [0u8; 20], Errors::EmptyReceiver);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.signer.to_account_info().clone(),
                to: pda.to_account_info().clone(),
            },
        );
        system_program::transfer(cpi_context, DEPOSIT_FEE)?;

        let pda_ata = get_associated_token_address(&ctx.accounts.pda.key(), &from.mint);
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

        msg!(
            "Deposit SPL executed: amount = {}, fee = {}, receiver = {:?}, pda = {}, mint = {}",
            amount,
            DEPOSIT_FEE,
            receiver,
            ctx.accounts.pda.key(),
            ctx.accounts.mint_account.key()
        );

        Ok(())
    }

    /// Deposits SPL tokens and calls a contract on ZetaChain zEVM.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `amount` - The amount of SPL tokens to deposit.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    pub fn deposit_spl_token_and_call(
        ctx: Context<DepositSplToken>,
        amount: u64,
        receiver: [u8; 20],
        message: Vec<u8>,
    ) -> Result<()> {
        require!(
            message.len() <= MAX_DEPOSIT_PAYLOAD_SIZE,
            Errors::MemoLengthExceeded
        );
        deposit_spl_token(ctx, amount, receiver)?;

        msg!("Deposit SPL and call executed with message = {:?}", message);

        Ok(())
    }

    /// Calls a contract on ZetaChain zEVM.
    ///
    /// # Arguments
    /// * `ctx` - The instruction context.
    /// * `receiver` - The Ethereum address of the receiver on ZetaChain zEVM.
    /// * `message` - The message passed to the contract.
    pub fn call(_ctx: Context<Call>, receiver: [u8; 20], message: Vec<u8>) -> Result<()> {
        require!(receiver != [0u8; 20], Errors::EmptyReceiver);
        require!(
            message.len() <= MAX_DEPOSIT_PAYLOAD_SIZE,
            Errors::MemoLengthExceeded
        );

        msg!(
            "Call executed: receiver = {:?}, message = {:?}",
            receiver,
            message
        );

        Ok(())
    }

    /// Withdraws SOL. Caller is TSS.
    ///
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
        let pda = &mut ctx.accounts.pda;

        verify_and_update_nonce(pda, nonce)?;

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
        concatenated_buffer.push(InstructionId::Withdraw as u8);
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.recipient.key().to_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        msg!("Computed message hash: {:?}", message_hash);

        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;

        pda.sub_lamports(amount)?;
        ctx.accounts.recipient.add_lamports(amount)?;

        msg!(
            "Withdraw executed: amount = {}, recipient = {}, pda = {}",
            amount,
            ctx.accounts.recipient.key(),
            ctx.accounts.pda.key()
        );

        Ok(())
    }

    /// Withdraws SPL tokens. Caller is TSS.
    ///
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
        let pda = &mut ctx.accounts.pda;

        verify_and_update_nonce(pda, nonce)?;

        let mut concatenated_buffer = Vec::new();
        concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
        concatenated_buffer.push(InstructionId::WithdrawSplToken as u8);
        concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
        concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
        concatenated_buffer.extend_from_slice(&amount.to_be_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.mint_account.key().to_bytes());
        concatenated_buffer.extend_from_slice(&ctx.accounts.recipient_ata.key().to_bytes());
        require!(
            message_hash == hash(&concatenated_buffer[..]).to_bytes(),
            Errors::MessageHashMismatch
        );

        msg!("Computed message hash: {:?}", message_hash);

        recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;

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

        let cost_gas = 5000; // default gas cost in lamports
        let cost_ata_create = &mut 0; // will be updated if ATA creation is needed

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
            let bal_before = ctx.accounts.signer.lamports();
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
            let bal_after = ctx.accounts.signer.lamports();
            *cost_ata_create = bal_before - bal_after;

            msg!("Associated token account for recipient created!");
            msg!(
                "Refunding the rent ({:?} lamports) paid by the signer {:?}",
                cost_ata_create,
                ctx.accounts.signer.to_account_info().key
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

        transfer_checked(xfer_ctx, amount, decimals)?;
        // Note: this pda.sub_lamports() must be done here due to this issue https://github.com/solana-labs/solana/issues/9711
        // otherwise the previous CPI calls might fail with error:
        // "sum of account balances before and after instruction do not match"
        // Note2: to keep PDA from deficit, all SPL ZRC20 contracts needs to charge withdraw fee of
        // at least 5000(gas)+2039280(rent) lamports.
        let reimbursement = cost_gas + *cost_ata_create;
        pda.sub_lamports(reimbursement)?;
        ctx.accounts.signer.add_lamports(reimbursement)?;

        msg!(
            "Withdraw SPL executed: amount = {}, decimals = {}, recipient = {}, mint = {}, pda = {}",
            amount,
            decimals,
            ctx.accounts.recipient.key(),
            ctx.accounts.mint_account.key(),
            ctx.accounts.pda.key()
        );

        Ok(())
    }
}

// Verifies provided nonce is correct and updates pda nonce.
fn verify_and_update_nonce(pda: &mut Account<Pda>, nonce: u64) -> Result<()> {
    if nonce != pda.nonce {
        msg!(
            "Mismatch nonce: provided nonce = {}, expected nonce = {}",
            nonce,
            pda.nonce,
        );
        return err!(Errors::NonceMismatch);
    }
    pda.nonce += 1;
    Ok(())
}

/// Recovers and verifies eth address from signature.
fn recover_and_verify_eth_address(
    pda: &mut Account<Pda>,
    message_hash: &[u8; 32],
    recovery_id: u8,
    signature: &[u8; 64],
) -> Result<()> {
    let pubkey = secp256k1_recover(message_hash, recovery_id, signature)
        .map_err(|_| ProgramError::InvalidArgument)?;

    // pubkey is 64 Bytes, uncompressed public secp256k1 public key
    let h = hash(pubkey.to_bytes().as_slice()).to_bytes();
    let address = &h.as_slice()[12..32]; // ethereum address is the last 20 Bytes of the hashed pubkey
    msg!("Recovered address {:?}", address);

    let mut eth_address = [0u8; 20];
    eth_address.copy_from_slice(address);

    if eth_address != pda.tss_address {
        msg!("ECDSA signature error");
        return err!(Errors::TSSAuthenticationFailed);
    }

    Ok(())
}

/// Recovers and verifies tss signature for whitelist and unwhitelist instructions.
fn validate_whitelist_tss_signature(
    pda: &mut Account<Pda>,
    whitelist_candidate: &mut Account<Mint>,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
    instruction: u8,
) -> Result<()> {
    verify_and_update_nonce(pda, nonce)?;

    let mut concatenated_buffer = Vec::new();
    concatenated_buffer.extend_from_slice(ZETACHAIN_PREFIX);
    concatenated_buffer.push(instruction);
    concatenated_buffer.extend_from_slice(&pda.chain_id.to_be_bytes());
    concatenated_buffer.extend_from_slice(&nonce.to_be_bytes());
    concatenated_buffer.extend_from_slice(&whitelist_candidate.key().to_bytes());
    require!(
        message_hash == hash(&concatenated_buffer[..]).to_bytes(),
        Errors::MessageHashMismatch
    );

    msg!("Computed message hash: {:?}", message_hash);

    recover_and_verify_eth_address(pda, &message_hash, recovery_id, &signature)?;

    Ok(())
}

// Prepares account metas for withdraw and call, revert if unallowed account is passed
// TODO: this might be extended
fn prepare_account_metas(
    remaining_accounts: &[AccountInfo],
    signer: &Signer,
    pda: &Account<Pda>,
) -> Result<Vec<AccountMeta>> {
    let mut account_metas = Vec::new();

    for account_info in remaining_accounts.iter() {
        let account_key = account_info.key;

        // Prevent signer from being included
        require!(account_key != signer.key, Errors::InvalidInstructionData);

        // Gateway pda can be added as not writable
        if *account_key == pda.key() {
            account_metas.push(AccountMeta::new_readonly(*account_key, false));
        } else {
            if account_info.is_writable {
                account_metas.push(AccountMeta::new(*account_key, false));
            } else {
                account_metas.push(AccountMeta::new_readonly(*account_key, false));
            }
        }
    }

    Ok(account_metas)
}

/// Instruction context for initializing the program.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The account of the signer initializing the program.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(init, payer = signer, space = size_of::<Pda>() + 8, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Execute<'info> {
    /// The account of the signer making the deposit.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The destination program.
    /// CHECK: This is arbitrary program.
    pub destination_program: AccountInfo<'info>,

    // Pda for destination program
    /// CHECK: Validation will occur during instruction processing.
    #[account(
        mut,
        seeds = [b"connected"],
        bump,
        seeds::program = destination_program.key()
    )]
    pub destination_program_pda: UncheckedAccount<'info>,
}

/// Instruction context for increment nonce.
#[derive(Accounts)]
pub struct IncrementNonce<'info> {
    /// The account of the signer incrementing nonce.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
}

/// Instruction context for SOL deposit operations.
#[derive(Accounts)]
pub struct Deposit<'info> {
    /// The account of the signer making the deposit.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

/// Instruction context for depositing SPL tokens.
#[derive(Accounts)]
pub struct DepositSplToken<'info> {
    /// The account of the signer making the deposit.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The whitelist entry account for the SPL token.
    #[account(seeds = [b"whitelist", mint_account.key().as_ref()], bump)]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    /// The mint account of the SPL token being deposited.
    pub mint_account: Account<'info, Mint>,

    /// The token program.
    pub token_program: Program<'info, Token>,

    /// The source token account owned by the signer.
    #[account(mut)]
    pub from: Account<'info, TokenAccount>,

    /// The destination token account owned by the PDA.
    #[account(mut)]
    pub to: Account<'info, TokenAccount>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

/// Instruction context for call operation.
#[derive(Accounts)]
pub struct Call<'info> {
    /// The account of the signer making the call.
    #[account(mut)]
    pub signer: Signer<'info>,
}

/// Instruction context for SOL withdrawal operations.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The account of the signer making the withdrawal.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The recipient account for the withdrawn SOL.
    /// CHECK: Recipient account is not read; ownership validation is unnecessary.
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
}

/// Instruction context for SPL token withdrawal operations.
#[derive(Accounts)]
pub struct WithdrawSPLToken<'info> {
    /// The account of the signer making the withdrawal.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The associated token account for the Gateway PDA.
    #[account(mut, associated_token::mint = mint_account, associated_token::authority = pda)]
    pub pda_ata: Account<'info, TokenAccount>,

    /// The mint account of the SPL token being withdrawn.
    pub mint_account: Account<'info, Mint>,

    /// The recipient account for the withdrawn tokens.
    /// CHECK: Recipient account is not read; ownership validation is unnecessary.
    pub recipient: UncheckedAccount<'info>,

    /// The recipient's associated token account.
    /// CHECK: Validation will occur during instruction processing.
    #[account(mut)]
    pub recipient_ata: AccountInfo<'info>,

    /// The token program.
    pub token_program: Program<'info, Token>,

    /// The associated token program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteSPLToken<'info> {
    /// The account of the signer making the withdrawal.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The associated token account for the Gateway PDA.
    #[account(mut, associated_token::mint = mint_account, associated_token::authority = pda)]
    pub pda_ata: Account<'info, TokenAccount>,

    /// The mint account of the SPL token being withdrawn.
    pub mint_account: Account<'info, Mint>,

    /// The destination program.
    /// CHECK: This is arbitrary program.
    pub destination_program: AccountInfo<'info>,

    // Pda for destination program
    /// CHECK: Validation will occur during instruction processing.
    #[account(
        mut,
        seeds = [b"connected"],
        bump,
        seeds::program = destination_program.key()
    )]
    pub destination_program_pda: UncheckedAccount<'info>,

    /// The destination program associated token account.
    /// CHECK: Validation will occur during instruction processing.
    #[account(mut)]
    pub destination_program_pda_ata: AccountInfo<'info>,

    /// The token program.
    pub token_program: Program<'info, Token>,

    /// The associated token program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

/// Instruction context for updating the TSS address.
#[derive(Accounts)]
pub struct UpdateTss<'info> {
    /// The account of the signer performing the update.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
}

/// Instruction context for updating the PDA authority.
#[derive(Accounts)]
pub struct UpdateAuthority<'info> {
    /// The account of the signer performing the update.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
}

/// Instruction context for pausing or unpausing deposits.
#[derive(Accounts)]
pub struct UpdatePaused<'info> {
    /// The account of the signer performing the update.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,
}

/// Instruction context for whitelisting SPL tokens.
#[derive(Accounts)]
pub struct Whitelist<'info> {
    /// The account of the authority performing the operation.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The whitelist entry account being initialized.
    #[account(
        init,
        space = 8,
        payer = authority,
        seeds = [b"whitelist", whitelist_candidate.key().as_ref()],
        bump
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    /// The mint account of the SPL token being whitelisted.
    pub whitelist_candidate: Account<'info, Mint>,

    /// The system program.
    pub system_program: Program<'info, System>,
}

/// Instruction context for unwhitelisting SPL tokens.
#[derive(Accounts)]
pub struct Unwhitelist<'info> {
    /// The account of the authority performing the operation.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Gateway PDA.
    #[account(mut, seeds = [b"meta"], bump)]
    pub pda: Account<'info, Pda>,

    /// The whitelist entry account being closed.
    #[account(
        mut,
        seeds = [b"whitelist", whitelist_candidate.key().as_ref()],
        bump,
        close = authority,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    /// The mint account of the SPL token being unwhitelisted.
    pub whitelist_candidate: Account<'info, Mint>,
}

/// PDA account storing program state and settings.
#[account]
pub struct Pda {
    /// The nonce to ensure each signature can only be used once.
    nonce: u64,
    /// The Ethereum TSS address (20 bytes).
    tss_address: [u8; 20],
    /// The authority controlling the PDA.
    authority: Pubkey,
    /// The chain ID associated with the PDA.
    chain_id: u64,
    /// Flag to indicate whether deposits are paused.
    deposit_paused: bool,
}

/// Whitelist entry account for whitelisted SPL tokens.
#[account]
pub struct WhitelistEntry {}
