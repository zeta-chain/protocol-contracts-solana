use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::mem::size_of;

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

/// Instruction context for executing a call on program.
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
    #[account(mut, constraint = from.mint == mint_account.key())]
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

/// Instruction context for resetting the PDA nonce.
#[derive(Accounts)]
pub struct ResetNonce<'info> {
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

/// Instruction context for checking upgrade status
#[derive(Accounts)]
pub struct Upgrade<'info> {
    /// The account of the signer checking the upgrade
    pub signer: Signer<'info>,
}
