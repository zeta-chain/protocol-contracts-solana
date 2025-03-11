use anchor_lang::prelude::*;
use std::mem::size_of;
use raydium_cpmm_cpi::{
    cpi,
    program::RaydiumCpmm,
    states::{AmmConfig, ObservationState, PoolState},
};

pub mod instructions;
use instructions::*;


declare_id!("8iUjRRhUCn8BjrvsWPfj8mguTe9L81ES4oAUApiF8JFC");

// NOTE: this is just example contract that can be called from gateway in execute function for testing withdraw and call spl
#[program]
pub mod connected_spl {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn proxy_initialize(
        ctx: Context<ProxyInitialize>,
        init_amount_0: u64,
        init_amount_1: u64,
        open_time: u64,
    ) -> Result<()> {
        instructions::proxy_initialize(ctx, init_amount_0, init_amount_1, open_time)
    }

    pub fn proxy_deposit(
        ctx: Context<ProxyDeposit>,
        lp_token_amount: u64,
        maximum_token_0_amount: u64,
        maximum_token_1_amount: u64,
    ) -> Result<()> {
        instructions::proxy_deposit(
            ctx,
            lp_token_amount,
            maximum_token_0_amount,
            maximum_token_1_amount,
        )
    }

    pub fn proxy_swap_base_input(
        ctx: Context<ProxySwapBaseInput>,
        amount_in: u64,
        minimum_amount_out: u64,
    ) -> Result<()> {
        instructions::proxy_swap_base_input(ctx, amount_in, minimum_amount_out)
    }

    // NOTE: this will swap 2 provided tokens using raydium
    // half amount is swaped using pda as signer to provided output account
    pub fn on_call(
        ctx: Context<OnCall>,
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
    ) -> Result<()> {
        let pda = &mut ctx.accounts.pda;

        // Store the sender's public key
        pda.last_sender = sender;

        // Convert data to a string and store it
        let message = String::from_utf8(data).map_err(|_| ErrorCode::InvalidDataFormat)?;
        pda.last_message = message;

        let signer_seeds: &[&[&[u8]]] = &[&[b"connectedSPL", &[ctx.bumps.pda]]];

        let cpi_accounts = cpi::accounts::Swap {
            payer: pda.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
            amm_config: ctx.accounts.amm_config.to_account_info(),
            pool_state: ctx.accounts.pool_state.to_account_info(),
            input_token_account: ctx.accounts.input_token_account.to_account_info(),
            output_token_account: ctx.accounts.output_token_account.to_account_info(),
            input_vault: ctx.accounts.input_vault.to_account_info(),
            output_vault: ctx.accounts.output_vault.to_account_info(),
            input_token_program: ctx.accounts.input_token_program.to_account_info(),
            output_token_program: ctx.accounts.output_token_program.to_account_info(),
            input_token_mint: ctx.accounts.input_token_mint.to_account_info(),
            output_token_mint: ctx.accounts.output_token_mint.to_account_info(),
            observation_state: ctx.accounts.observation_state.to_account_info(),
        };
        let cpi_context = CpiContext::new(ctx.accounts.cp_swap_program.to_account_info(), cpi_accounts).with_signer(signer_seeds);
        cpi::swap_base_input(cpi_context, amount / 2, 0);

        msg!(
            "On call executed with amount {}, sender {:?} and message {}",
            amount,
            pda.last_sender,
            pda.last_message
        );

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(init, payer = signer, space = size_of::<Pda>() + 32, seeds = [b"connectedSPL"], bump)]
    pub pda: Account<'info, Pda>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnCall<'info> {
    #[account(mut, seeds = [b"connectedSPL"], bump)]
    pub pda: Account<'info, Pda>,

    /// CHECK: test
    pub cp_swap_program: UncheckedAccount<'info>,
    // /// The user performing the swap
    // pub payer: Signer<'info>,

    /// CHECK: pool vault and lp mint authority
    pub authority: UncheckedAccount<'info>,

    /// The factory state to read protocol fees
    /// CHECK: test
    pub amm_config: UncheckedAccount<'info>,

    /// The program account of the pool in which the swap will be performed
    /// CHECK: test
    #[account(mut)]
    pub pool_state: UncheckedAccount<'info>,

    /// The user token account for input token
    /// CHECK: test
    #[account(mut)]
    pub input_token_account: UncheckedAccount<'info>,

    /// The user token account for output token
    /// CHECK: test
    #[account(mut)]
    pub output_token_account: UncheckedAccount<'info>,

    /// The vault token account for input token
    /// CHECK: test
    pub input_vault: UncheckedAccount<'info>,

    /// The vault token account for output token
    /// CHECK: test
    pub output_vault: UncheckedAccount<'info>,

    /// SPL program for input token transfers
    /// CHECK: test
    pub input_token_program:  UncheckedAccount<'info>,

    /// SPL program for output token transfers
    /// CHECK: test
    pub output_token_program:  UncheckedAccount<'info>,

    /// The mint of input token
    /// CHECK: test
    pub input_token_mint:  UncheckedAccount<'info>,

    /// The mint of output token
    /// CHECK: test
    pub output_token_mint:  UncheckedAccount<'info>,
    /// The program account for the most recent oracle observation
    /// CHECK: test
    pub observation_state:  UncheckedAccount<'info>,
}

#[account]
pub struct Pda {
    pub last_sender: [u8; 20],
    pub last_message: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The data provided could not be converted to a valid UTF-8 string.")]
    InvalidDataFormat,
}

