use anchor_lang::prelude::*;
use anchor_spl::{
    token::transfer_checked,
    associated_token::get_associated_token_address
};
use solana_program::{
    program::invoke,
    keccak::hash,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use crate::{
    contexts::{Withdraw, WithdrawSPLToken},
    errors::{Errors, InstructionId},
    utils::{verify_and_update_nonce, recover_and_verify_eth_address,ZETACHAIN_PREFIX},
};

/// Withdraws SOL. Caller is TSS.
/// # Arguments
/// * `ctx` - The instruction context.
/// * `amount` - The amount of SOL to withdraw.
/// * `signature` - The TSS signature.
/// * `recovery_id` - The recovery ID for signature verification.
/// * `message_hash` - Message hash for signature verification.
/// * `nonce` - The current nonce value.
pub fn handle_sol(
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
/// # Arguments
/// * `ctx` - The instruction context.
/// * `decimals` - Token decimals for precision.
/// * `amount` - The amount of tokens to withdraw.
/// * `signature` - The TSS signature.
/// * `recovery_id` - The recovery ID for signature verification.
/// * `message_hash` - Message hash for signature verification.
/// * `nonce` - The current nonce value.
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