use anchor_lang::prelude::*;
use anchor_spl::associated_token::get_associated_token_address;

use crate::errors::Errors;

/// Verify ATA address matches expected value
///
/// # Arguments
///
/// * `owner` - The owner of the associated token account
/// * `mint` - The mint (token) address
/// * `actual_ata` - The provided associated token account address to verify
///
/// # Returns
///
/// * `Result<()>` - Ok if the addresses match, Error otherwise
///
/// # Errors
///
/// Returns `Errors::SPLAtaAndMintAddressMismatch` if the provided ATA doesn't match the expected one
pub fn verify_ata_match(owner: &Pubkey, mint: &Pubkey, actual_ata: &Pubkey) -> Result<()> {
    let expected_ata = get_associated_token_address(owner, mint);
    require!(
        expected_ata == *actual_ata,
        Errors::SPLAtaAndMintAddressMismatch
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_ata_match_success() {
        // Arrange
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let expected_ata = get_associated_token_address(&owner, &mint);

        // Act
        let result = verify_ata_match(&owner, &mint, &expected_ata);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_ata_match_failure() {
        // Arrange
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();

        // Act
        let wrong_ata = Pubkey::new_unique();
        let result = verify_ata_match(&owner, &mint, &wrong_ata);

        // Assert
        assert!(result.is_err());
    }
}
