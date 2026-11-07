//! Public-only platform-token identity checks. No key generation or signing.
use crate::{validate_solana_address, Error, Result};

pub const PLATFORM_TOKEN_NAME: &str = "Supa Agency";
pub const PLATFORM_TOKEN_SYMBOL: &str = "SUPA";
pub const PLATFORM_MINT_PREFIX: &str = "SUPA";

/// Validate canonical 32-byte Base58 syntax and the exact uppercase prefix.
///
/// Success proves neither signer possession, an on-curve Ed25519 key, platform
/// ownership, onchain mint creation nor launch approval. Those are separate gates.
/// This rule applies only to the official platform token, never to agent tokens.
pub fn validate_official_platform_mint_address(address: &str) -> Result<()> {
    validate_solana_address(address)?;
    if !address.starts_with(PLATFORM_MINT_PREFIX) {
        return Err(Error::Invalid);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Public address syntax fixture only. No keypair or private seed exists here.
    fn syntax_fixture() -> String {
        format!("SUPA{}", "1".repeat(39))
    }

    #[test]
    fn platform_identity_is_fixed_and_address_round_trips_exactly() {
        assert_eq!(PLATFORM_TOKEN_NAME, "Supa Agency");
        assert_eq!(PLATFORM_TOKEN_SYMBOL, "SUPA");
        assert_eq!(PLATFORM_MINT_PREFIX, "SUPA");
        let address = syntax_fixture();
        validate_official_platform_mint_address(&address).unwrap();
        let public_bytes = bs58::decode(&address).into_vec().unwrap();
        assert_eq!(public_bytes.len(), 32);
        assert_eq!(bs58::encode(public_bytes).into_string(), address);
    }

    #[test]
    fn case_suffix_whitespace_and_unicode_are_never_normalized() {
        let address = syntax_fixture();
        for rejected in [
            address.replacen("SUPA", "Supa", 1),
            address.replacen("SUPA", "supa", 1),
            format!("{}SUPA", "A".repeat(39)),
            format!(" {address}"),
            format!("{address}\n"),
            address.replacen('S', "Ｓ", 1),
        ] {
            assert!(validate_official_platform_mint_address(&rejected).is_err());
        }
    }

    #[test]
    fn prefix_alone_invalid_alphabet_and_wrong_decoded_width_are_rejected() {
        for rejected in ["", "SUPA", "SUPA0", "SUPAO", "SUPAI", "SUPAl"] {
            assert!(validate_official_platform_mint_address(rejected).is_err());
        }
        for size in [31, 33, 64] {
            let address = bs58::encode(vec![7; size]).into_string();
            assert!(validate_official_platform_mint_address(&address).is_err());
        }
    }

    #[test]
    fn agent_address_validation_does_not_inherit_the_platform_prefix() {
        let address = bs58::encode([9; 32]).into_string();
        validate_solana_address(&address).unwrap();
        assert!(validate_official_platform_mint_address(&address).is_err());
    }
}
