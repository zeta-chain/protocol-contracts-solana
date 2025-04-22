use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;

use crate::errors::Errors;
use crate::state::Pda;

/// Prepares account metas for withdraw and call, revert if unallowed account is passed
pub fn prepare_account_metas(
    remaining_accounts: &[AccountInfo],
    signer: &Signer,
    pda: &Account<Pda>,
) -> Result<Vec<AccountMeta>> {
    let mut account_metas = Vec::new();

    for account_info in remaining_accounts.iter() {
        let account_key = account_info.key;

        // Prevent signer from being included
        require!(account_key != signer.key, Errors::InvalidInstructionData);

        // Gateway pda can be added as not writable
        if *account_key == pda.key() {
            account_metas.push(AccountMeta::new_readonly(*account_key, false));
        } else if account_info.is_writable {
            account_metas.push(AccountMeta::new(*account_key, false));
        } else {
            account_metas.push(AccountMeta::new_readonly(*account_key, false));
        }
    }
    Ok(account_metas)
}
