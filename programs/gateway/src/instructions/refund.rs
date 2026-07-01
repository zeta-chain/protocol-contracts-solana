use crate::{
    contexts::{RefundSol, RefundSplToken},
    errors::Errors,
    state::Pda,
    utils::{verify_ata_match, verify_authority},
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token::transfer_checked;
use spl_associated_token_account::instruction::create_associated_token_account;

fn validate_refund_request(authority: &Pubkey, pda: &Account<Pda>, amount: u64) -> Result<()> {
    verify_authority(authority, pda)?;
    require!(amount > 0, Errors::InvalidAmount);
    Ok(())
}

// Refunds native SOL from gateway custody to a user. Caller is authority stored in PDA.
pub fn handle_sol(ctx: Context<RefundSol>, amount: u64) -> Result<()> {
    let pda = &ctx.accounts.pda;

    validate_refund_request(&ctx.accounts.signer.key(), pda, amount)?;

    let pda_info = pda.to_account_info();
    let rent = Rent::get()?;
    let min_balance = rent.minimum_balance(pda_info.data_len());
    let available = pda_info.lamports().saturating_sub(min_balance);

    require!(available >= amount, Errors::InsufficientBalance);

    pda_info.sub_lamports(amount)?;
    ctx.accounts.recipient.add_lamports(amount)?;

    msg!(
        "Refund SOL executed: amount = {}, recipient = {}, authority = {}",
        amount,
        ctx.accounts.recipient.key(),
        ctx.accounts.signer.key()
    );

    Ok(())
}

// Refunds SPL tokens from gateway custody to a user. Caller is authority stored in PDA.
pub fn handle_spl(ctx: Context<RefundSplToken>, amount: u64, decimals: u8) -> Result<()> {
    let pda = &ctx.accounts.pda;

    validate_refund_request(&ctx.accounts.signer.key(), pda, amount)?;

    verify_ata_match(
        &pda.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.pda_ata.key(),
    )?;

    verify_ata_match(
        &ctx.accounts.recipient.key(),
        &ctx.accounts.mint_account.key(),
        &ctx.accounts.recipient_ata.key(),
    )?;

    require!(
        ctx.accounts.pda_ata.amount >= amount,
        Errors::InsufficientBalance
    );

    let recipient_ata_account = ctx.accounts.recipient_ata.to_account_info();
    require!(
        *recipient_ata_account.owner == anchor_spl::token::ID
            || (*recipient_ata_account.owner == anchor_lang::system_program::ID
                && recipient_ata_account.lamports() == 0),
        Errors::InvalidAtaOwner
    );

    if recipient_ata_account.lamports() == 0 {
        msg!(
            "Creating associated token account {:?} for recipient {:?}...",
            recipient_ata_account.key(),
            ctx.accounts.recipient.key(),
        );

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

        msg!("Associated token account created!");
    }

    let token = &ctx.accounts.token_program;
    let signer_seeds: &[&[&[u8]]] = &[&[b"meta", &[ctx.bumps.pda]]];

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

    msg!(
        "Refund SPL executed: amount = {}, decimals = {}, recipient = {}, mint = {}, authority = {}",
        amount,
        decimals,
        ctx.accounts.recipient.key(),
        ctx.accounts.mint_account.key(),
        ctx.accounts.signer.key()
    );

    Ok(())
}
