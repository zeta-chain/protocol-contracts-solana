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
}

impl CallableInstruction {
    pub fn pack(&self) -> Vec<u8> {
        let mut buf;
        match self {
            CallableInstruction::OnCall {
                amount,
                sender,
                data,
            } => {
                let data_len = data.len() as u32;
                //8 (discriminator) + 8 (u64 amount) + 20 (sender) + 4 (data length)
                buf = Vec::with_capacity(40 + data_len as usize);

                // Discriminator for instruction (example)
                // This ensures the program knows how to handle this instruction.
                // Example discriminator: anchor typically uses `hash("global:on_call")`
                buf.extend_from_slice(&[16, 136, 66, 32, 254, 40, 181, 8]);

                // Encode amount (u64) in little-endian format
                buf.extend_from_slice(&amount.to_le_bytes());

                // Encode sender ([u8; 20])
                buf.extend_from_slice(sender);

                // Encode the length of the data array (u32)
                buf.extend_from_slice(&data_len.to_le_bytes());

                // Encode the data itself
                buf.extend_from_slice(data);
            }
        }
        buf
    }
}