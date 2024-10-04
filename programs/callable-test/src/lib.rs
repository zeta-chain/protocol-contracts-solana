use anchor_lang::prelude::*;

declare_id!("HhLWiKkriQSSZmu1Pfa2tkQD87HosDSFUqeuZKeEc88m");

#[program]
pub mod callable_test {
    use super::*;

    pub fn on_call(ctx: Context<OnCall>, sender: Pubkey, data: Vec<u8>) -> Result<()> {
        // Perform custom logic here based on the received data

        Ok(())
    }
}

#[derive(Accounts)]
pub struct OnCall {}


#[account]
pub struct StorageAccount {
    pub last_sender: Pubkey,
    pub last_data: Vec<u8>, // Store the last used data
}