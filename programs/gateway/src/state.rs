use anchor_lang::prelude::*;

/// PDA account storing program state and settings.
#[account]
pub struct Pda {
    /// The nonce to ensure each signature can only be used once.
    pub nonce: u64,
    /// The Ethereum TSS address (20 bytes).
    pub tss_address: [u8; 20],
    /// The authority controlling the PDA.
    pub authority: Pubkey,
    /// The chain ID associated with the PDA.
    pub chain_id: u64,
    /// Flag to indicate whether deposits are paused.
    pub deposit_paused: bool,
}

/// Whitelist entry account for whitelisted SPL tokens.
#[account]
pub struct WhitelistEntry {}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum CallableInstruction {
    OnCall {
        amount: u64,
        sender: [u8; 20],
        data: Vec<u8>,
    },
    OnRevert {
        amount: u64,
        sender: Pubkey,
        data: Vec<u8>,
    },
}

impl CallableInstruction {
    pub fn pack(&self) -> Vec<u8> {
        match self {
            CallableInstruction::OnCall {
                amount,
                sender,
                data,
            } => {
                let discriminator = [16, 136, 66, 32, 254, 40, 181, 8]; // on_call
                let data_len = data.len() as u32;

                let mut buf = Vec::with_capacity(8 + 8 + 20 + 4 + data.len());
                buf.extend_from_slice(&discriminator);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(sender);
                buf.extend_from_slice(&data_len.to_le_bytes());
                buf.extend_from_slice(data);
                buf
            }
            CallableInstruction::OnRevert {
                amount,
                sender,
                data,
            } => {
                let discriminator = [226, 44, 101, 52, 224, 214, 41, 9]; // on_revert
                let data_len = data.len() as u32;

                let mut buf = Vec::with_capacity(8 + 8 + 32 + 4 + data.len());
                buf.extend_from_slice(&discriminator);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(sender.as_ref());
                buf.extend_from_slice(&data_len.to_le_bytes());
                buf.extend_from_slice(data);
                buf
            }
        }
    }
}

/// Struct containing revert options
/// # Arguments
/// * `revert_address` Address to receive revert.
/// * `abort_address` Address to receive funds if aborted.
/// * `call_on_revert` Flag if on_revert hook should be called.
/// * `revert_message` Arbitrary data sent back in on_revert.
/// * `on_revert_gas_limit` Gas limit for revert tx.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq)]
pub struct RevertOptions {
    pub revert_address: Pubkey,
    pub abort_address: [u8; 20],
    pub call_on_revert: bool,
    pub revert_message: Vec<u8>,
    pub on_revert_gas_limit: u64,
}

/// Enumeration for instruction identifiers in message hashes.
#[repr(u8)]
pub enum InstructionId {
    Withdraw = 1,
    WithdrawSplToken = 2,
    WhitelistSplToken = 3,
    UnwhitelistSplToken = 4,
    ExecuteSol = 5,
    ExecuteSplToken = 6,
    IncrementNonce = 7,
    ExecuteSolRevert = 8,
    ExecuteSplTokenRevert = 9,
}
