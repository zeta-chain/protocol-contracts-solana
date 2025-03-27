/// Prefix used for outbounds message hashes.
pub const ZETACHAIN_PREFIX: &[u8] = b"ZETACHAIN";

/// Default gas cost in lamports for SPL token operations
pub const DEFAULT_GAS_COST: u64 = 5000;

// Maximum size of a message payload in bytes
pub const MAX_DEPOSIT_PAYLOAD_SIZE: usize = 745;

/// Deposit fee used when depositing SOL or SPL tokens.
pub const DEPOSIT_FEE: u64 = 2_000_000;
