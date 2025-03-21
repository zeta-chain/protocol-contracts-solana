use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::transfer;
use anchor_spl::associated_token::get_associated_token_address;
use crate::{
    contexts::{Deposit, DepositSplToken, Call},
    errors::Errors,
};

/// Deposits SOL into the program and credits the `receiver` on ZetaChain zEVM.
pub fn handle_sol(
    ctx: Context<Deposit>,
    amount: u64,
    receiver: [u8; 20],
    deposit_fee: u64
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;
    require!(!pda.deposit_paused, Errors::DepositPaused);
    require!(receiver != [0u8; 20], Errors::EmptyReceiver);

    let amount_with_fees = amount + deposit_fee;
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
        deposit_fee,
        receiver,
        ctx.accounts.pda.key()
    );

    Ok(())
}

/// Deposits SOL and calls a contract on ZetaChain zEVM.
pub fn handle_sol_with_call(
    ctx: Context<Deposit>,
    amount: u64,
    receiver: [u8; 20],
    message: Vec<u8>,
    deposit_fee: u64,
    max_message_size: usize
) -> Result<()> {
    require!(
        message.len() <= max_message_size,
        Errors::MemoLengthExceeded
    );
    handle_sol(ctx, amount, receiver, deposit_fee)?;

    msg!("Deposit and call executed with message = {:?}", message);

    Ok(())
}

/// Deposits SPL tokens and credits the `receiver` on ZetaChain zEVM.
pub fn handle_spl(
    ctx: Context<DepositSplToken>,
    amount: u64,
    receiver: [u8; 20],
    deposit_fee: u64
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
    system_program::transfer(cpi_context, deposit_fee)?;

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
        deposit_fee,
        receiver,
        ctx.accounts.pda.key(),
        ctx.accounts.mint_account.key()
    );

    Ok(())
}

/// Deposits SPL tokens and calls a contract on ZetaChain zEVM.
pub fn handle_spl_with_call(
    ctx: Context<DepositSplToken>,
    amount: u64,
    receiver: [u8; 20],
    message: Vec<u8>,
    deposit_fee: u64,
    max_message_size: usize
) -> Result<()> {
    require!(
        message.len() <= max_message_size,
        Errors::MemoLengthExceeded
    );
    handle_spl(ctx, amount, receiver, deposit_fee)?;

    msg!("Deposit SPL and call executed with message = {:?}", message);

    Ok(())
}

/// Calls a contract on ZetaChain zEVM.
pub fn handle_call(
    receiver: [u8; 20],
    message: Vec<u8>,
    max_message_size: usize
) -> Result<()> {
    require!(receiver != [0u8; 20], Errors::EmptyReceiver);
    require!(
        message.len() <= max_message_size,
        Errors::MemoLengthExceeded
    );

    msg!(
        "Call executed: receiver = {:?}, message = {:?}",
        receiver,
        message
    );

    Ok(())
}