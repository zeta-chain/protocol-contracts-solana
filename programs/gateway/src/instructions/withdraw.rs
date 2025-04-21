use crate::{
    contexts::{Withdraw, WithdrawSPLToken},
    state::InstructionId,
    utils::{validate_message, verify_ata_match, DEFAULT_GAS_COST},
};
use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::token::transfer_checked;
use spl_associated_token_account::instruction::create_associated_token_account;

// Withdraws SOL. Caller is TSS.
pub fn handle_sol(
    ctx: Context<Withdraw>,
    amount: u64,
    signature: [u8; 64],
    recovery_id: u8,
    message_hash: [u8; 32],
    nonce: u64,
) -> Result<()> {
    let pda = &mut ctx.accounts.pda;

    // 1. Verify cross-chain message
    validate_message(
        pda,
        InstructionId::Withdraw,
        nonce,
        amount,
        &[&ctx.accounts.recipient.key().to_bytes()],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Transfer SOL
    pda.sub_lamports(amount)?;
    ctx.accounts.recipient.add_lamports(amount)?;

    // 3. Log success
    msg!(
        "Withdraw executed: amount = {}, recipient = {}, pda = {}",
        amount,
        ctx.accounts.recipient.key(),
        ctx.accounts.pda.key()
    );

    Ok(())
}

// Withdraws SPL tokens. Caller is TSS
pub fn handle_spl(
    ctx: Context<WithdrawSPLToken>,
    decimals: u8,
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
        InstructionId::WithdrawSplToken,
        nonce,
        amount,
        &[
            &ctx.accounts.mint_account.key().to_bytes(),
            &ctx.accounts.recipient_ata.key().to_bytes(),
        ],
        &message_hash,
        &signature,
        recovery_id,
    )?;

    // 2. Verify token accounts
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

    // 3. Create recipient ATA if needed and calculate costs
    let mut cost_ata_create: u64 = 0;
    let recipient_ata_account = ctx.accounts.recipient_ata.to_account_info();

    if recipient_ata_account.lamports() == 0
        || *recipient_ata_account.owner == ctx.accounts.system_program.key()
    {
        // ATA needs to be created
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
        cost_ata_create = bal_before - bal_after;

        msg!("Associated token account created!");
        msg!(
            "Refunding the rent ({:?} lamports) paid by the signer {:?}",
            cost_ata_create,
            ctx.accounts.signer.to_account_info().key
        );
    }

    // 4. Transfer tokens
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

    // 5. Reimburse gas costs
    let reimbursement = DEFAULT_GAS_COST + cost_ata_create;
    pda.sub_lamports(reimbursement)?;
    ctx.accounts.signer.add_lamports(reimbursement)?;

    // 6. Log success
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
