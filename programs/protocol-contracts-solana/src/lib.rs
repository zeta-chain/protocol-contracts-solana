use anchor_lang::prelude::*;

declare_id!("9WSwbVLthCsJXABeDJcVcw4UQMYuoNLTJTLqueFXU5Q2");

#[program]
pub mod protocol_contracts_solana {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
