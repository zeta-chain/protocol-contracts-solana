use anchor_lang::prelude::*;
use anchor_spl::{
    token::transfer_checked,
    associated_token::get_associated_token_address
};
use solana_program::{
    program::invoke,
    instruction::Instruction,
    keccak::hash,
};
use crate::{
    contexts::{Execute, ExecuteSPLToken, IncrementNonce},
    errors::{Errors, InstructionId},
    state::{CallableInstruction},
    utils::{verify_and_update_nonce, recover_and_verify_eth_address, prepare_account_metas},
};

/// Prefix used for outbounds message hashes.
pub const ZETACHAIN_PREFIX: &[u8] = b"ZETACHAIN";

/// Increments nonce, used by TSS in case outbound fails.
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
pub fn handle(
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
pub fn handle_spl_token(
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