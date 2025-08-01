use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint as TokenMint, Token, TokenAccount};
use std::mem::size_of;

/// Context for initializing the ZETA token program.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The signer initializing the program.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// ZETA token PDA.
    #[account(
        init,
        payer = signer,
        space = size_of::<ZetaTokenPda>() + 8,
        seeds = [b"zeta-token-pda"],
        bump
    )]
    pub zeta_token_pda: Account<'info, ZetaTokenPda>,

    /// ZETA token mint account.
    #[account(
        init,
        payer = signer,
        mint::decimals = 18,
        mint::authority = zeta_token_pda,
        mint::freeze_authority = zeta_token_pda,
        seeds = [b"zeta-mint"],
        bump
    )]
    pub zeta_mint: Account<'info, TokenMint>,

    /// The system program.
    pub system_program: Program<'info, System>,

    /// The token program.
    pub token_program: Program<'info, Token>,

    /// The rent sysvar.
    pub rent: Sysvar<'info, Rent>,
}

/// Context for minting ZETA tokens.
#[derive(Accounts)]
pub struct MintZeta<'info> {
    /// ZETA token PDA.
    #[account(
        mut,
        seeds = [b"zeta-token-pda"],
        bump
    )]
    pub zeta_token_pda: Account<'info, ZetaTokenPda>,

    /// The ZETA token mint.
    #[account(
        mut,
        seeds = [b"zeta-mint"],
        bump,
        constraint = zeta_mint.key() == zeta_token_pda.connector_authority
    )]
    pub zeta_mint: Account<'info, TokenMint>,

    /// The recipient's token account.
    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    /// The token program
    pub token_program: Program<'info, Token>,

    /// The mint authority (ZetaConnector).
    #[account(
        constraint = mint_authority.key() == zeta_token_pda.connector_authority 
        @ crate::errors::ZetaTokenErrors::CallerIsNotConnector
    )]
    pub mint_authority: Signer<'info>,
}

/// Context for burning ZETA tokens.
#[derive(Accounts)]
pub struct BurnZeta<'info> {
    /// ZETA token PDA.
    #[account(
        mut,
        seeds = [b"zeta-token-pda"],
        bump
    )]
    pub zeta_token_pda: Account<'info, ZetaTokenPda>,

    /// The ZETA token mint.
    #[account(
        mut,
        seeds = [b"zeta-mint"],
        bump,
        constraint = zeta_mint.key() == zeta_token_pda.connector_authority
    )]
    pub zeta_mint: Account<'info, TokenMint>,

    /// The token account to burn from.
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,

    /// The token program.
    pub token_program: Program<'info, Token>,

    /// The burn authority (ZetaConnector).
    #[account(
        constraint = burn_authority.key() == zeta_token_pda.connector_authority 
        @ crate::errors::ZetaTokenErrors::CallerIsNotConnector
    )]
    pub burn_authority: Signer<'info>,
}

/// Context for updating TSS address.
#[derive(Accounts)]
pub struct UpdateTssAddress<'info> {
    /// ZETA token PDA.
    #[account(
        mut,
        seeds = [b"zeta-token-pda"],
        bump
    )]
    pub zeta_token_pda: Account<'info, ZetaTokenPda>,

    /// The updater (must be TSS updater or current TSS).
    #[account(
        constraint = updater.key() == zeta_token_pda.tss_address_updater
    )]
    pub updater: Signer<'info>,
}
