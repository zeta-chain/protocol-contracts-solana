use super::constants::MAX_DEPOSIT_PAYLOAD_SIZE;
use crate::errors::Errors;
use crate::state::RevertOptions;
use anchor_lang::prelude::*;

/// Verify the size of the payload for deposit transactions
/// ## Arguments
/// * `message` - The message payload to be verified.
/// * `revert_options` - The revert options containing the revert message.
/// ## Returns
/// * `Result<()>` - Ok if the payload size is within limits, Error otherwise.
pub fn verify_payload_size(
    message: Option<&Vec<u8>>,
    revert_options: &Option<RevertOptions>,
) -> Result<()> {
    let msg_len = message.map(|m| m.len()).unwrap_or(0);
    let revert_len = revert_options
        .as_ref()
        .map(|opts| opts.revert_message.len())
        .unwrap_or(0);

    require!(
        msg_len + revert_len <= MAX_DEPOSIT_PAYLOAD_SIZE,
        Errors::MemoLengthExceeded
    );

    Ok(())
}
