use anchor_lang::prelude::*;

declare_id!("B7apRShjWeCk2j64MFurBzjpnh5YYuNieMVkMZA7joVv");

// NOTE: will be removed, wanted to check if discriminator for on_call will be the same
#[program]
pub mod callable_test_2 {
    use super::*;

    pub fn on_call(ctx: Context<OnCall>, sender: Pubkey, data: Vec<u8>) -> Result<()> {

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct OnCall {}