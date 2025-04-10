use crate::{
    contexts::{Execute, ExecuteSPLToken, IncrementNonce},
    errors::InstructionId,
    state::CallableInstruction,
    utils::{prepare_account_metas, validate_message, verify_ata_match},
};
use anchor_lang::prelude::*;
use solana_program::{instruction::Instruction, program::invoke};

// Increments nonce, used by TSS in case outbound fails.
pub fn increment_nonce(
    ctx: Context<IncrementNonce>,
    amount: u64,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;

    // 1. Validate message
    validate_message(
        pda,
        InstructionId::IncrementNonce,
        nonce,
        amount,
        &[], // No additional data for this instruction
        &message_hash,
        &signature,
        recovery_id,
    )?;

    Ok(())
}

// Withdraws amount to destination program pda, and calls on_call on destination program
pub fn handle_sol(
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

    // 1. Validate message
    validate_message(
        pda,
        InstructionId::Execute,
        nonce,
        amount,
        &[&ctx.accounts.destination_program.key().to_bytes(), &data],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Prepare on_call instruction
    let instruction_data = CallableInstruction::ConnectedCall {
        amount,
        sender,
        data,
    }
    .pack();

    let account_metas = prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

    let ix = Instruction {
        program_id: ctx.accounts.destination_program.key(),
        accounts: account_metas,
        data: instruction_data,
    };

    // 3. Transfer SOL to destination program PDA
    pda.sub_lamports(amount)?;
    ctx.accounts.destination_program_pda.add_lamports(amount)?;

    // 4. Invoke destination program's on_call function
    invoke(&ix, ctx.remaining_accounts)?;

    // 5. Log success
    msg!(
        "Execute done: destination contract = {}, amount = {}, sender = {:?}",
        ctx.accounts.destination_program.key(),
        amount,
        sender,
    );

    Ok(())
}

// Withdraws amount of SPL tokens to destination program pda, and calls on_call on destination program
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

    // 1. Validate message
    validate_message(
        pda,
        InstructionId::ExecuteSplToken,
        nonce,
        amount,
        &[
            &ctx.accounts.mint_account.key().to_bytes(),
            &ctx.accounts.destination_program_pda_ata.key().to_bytes(),
            &data,
        ],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Prepare on_call instruction
    let instruction_data = CallableInstruction::ConnectedCall {
        amount,
        sender,
        data,
    }
    .pack();

    let account_metas = prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

    let ix = Instruction {
        program_id: ctx.accounts.destination_program.key(),
        accounts: account_metas,
        data: instruction_data,
    };

    // 3. Verify token accounts
    verify_ata_match(
        &pda.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.pda_ata.key(),
    )?;

    verify_ata_match(
        &ctx.accounts.destination_program_pda.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.destination_program_pda_ata.key(),
    )?;

    // 4. Transfer tokens
    let token = &ctx.accounts.token_program;
    let signer_seeds: &[&[&[u8]]] = &[&[b"meta", &[ctx.bumps.pda]]];

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

    anchor_spl::token::transfer_checked(xfer_ctx, amount, decimals)?;

    // 5. Invoke destination program's on_call function
    invoke(&ix, ctx.remaining_accounts)?;

    // 6. Log success
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

// Withdraws amount to destination program pda, and calls on_revert on destination program
pub fn handle_sol_revert(
    ctx: Context<Execute>,
    amount: u64,
    sender: Pubkey,
    data: Vec<u8>,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;

    // 1. Validate message
    validate_message(
        pda,
        InstructionId::ExecuteRevert,
        nonce,
        amount,
        &[&ctx.accounts.destination_program.key().to_bytes(), &data],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Prepare on_call instruction
    let instruction_data = CallableInstruction::ConnectedRevert {
        amount,
        sender,
        data,
    }
    .pack();

    let account_metas = prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

    let ix = Instruction {
        program_id: ctx.accounts.destination_program.key(),
        accounts: account_metas,
        data: instruction_data,
    };

    // 3. Transfer SOL to destination program PDA
    pda.sub_lamports(amount)?;
    ctx.accounts.destination_program_pda.add_lamports(amount)?;

    // 4. Invoke destination program's on_call function
    invoke(&ix, ctx.remaining_accounts)?;

    // 5. Log success
    msg!(
        "Execute done: destination contract = {}, amount = {}, sender = {:?}",
        ctx.accounts.destination_program.key(),
        amount,
        sender,
    );

    Ok(())
}

// Withdraws amount of SPL tokens to destination program pda, and calls on_revert on destination program
pub fn handle_spl_token_revert(
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
    let pda = &mut ctx.accounts.pda;

    // 1. Validate message
    validate_message(
        pda,
        InstructionId::ExecuteSplTokenRevert,
        nonce,
        amount,
        &[
            &ctx.accounts.mint_account.key().to_bytes(),
            &ctx.accounts.destination_program_pda_ata.key().to_bytes(),
            &data,
        ],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Prepare on_call instruction
    let instruction_data = CallableInstruction::ConnectedRevert {
        amount,
        sender,
        data,
    }
    .pack();

    let account_metas = prepare_account_metas(ctx.remaining_accounts, &ctx.accounts.signer, pda)?;

    let ix = Instruction {
        program_id: ctx.accounts.destination_program.key(),
        accounts: account_metas,
        data: instruction_data,
    };

    // 3. Verify token accounts
    verify_ata_match(
        &pda.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.pda_ata.key(),
    )?;

    verify_ata_match(
        &ctx.accounts.destination_program_pda.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.destination_program_pda_ata.key(),
    )?;

    // 4. Transfer tokens
    let token = &ctx.accounts.token_program;
    let signer_seeds: &[&[&[u8]]] = &[&[b"meta", &[ctx.bumps.pda]]];

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

    anchor_spl::token::transfer_checked(xfer_ctx, amount, decimals)?;

    // 5. Invoke destination program's on_call function
    invoke(&ix, ctx.remaining_accounts)?;

    // 6. Log success
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