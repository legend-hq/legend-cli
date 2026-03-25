#![cfg(target_os = "macos")]

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use security_framework::item::Location;
use security_framework::key::{Algorithm, GenerateKeyOptions, KeyType, SecKey, Token};
use security_framework_sys::item::*;
use security_framework_sys::keychain_item::SecItemCopyMatching;

use crate::error::{Result, SignerError};
use crate::signer::Signer;

pub struct SecureEnclaveSigner {
    private_key: SecKey,
    public_key_hex: String,
}

impl SecureEnclaveSigner {
    /// Generate a new P256 key in the Secure Enclave.
    ///
    /// The key is non-exportable and tagged with `label` for later retrieval.
    /// Label convention: `com.legend.prime.<profile_name>`
    ///
    /// The binary must be code-signed with keychain-access-groups entitlement.
    pub fn generate(label: &str) -> Result<Self> {
        let mut opts = GenerateKeyOptions::default();
        opts.set_key_type(KeyType::ec())
            .set_size_in_bits(256)
            .set_label(label.to_string())
            .set_token(Token::SecureEnclave)
            .set_location(Location::DefaultFileKeychain);

        let private_key = SecKey::new(&opts)
            .map_err(|e| SignerError::SecureEnclave(format!("Key generation failed: {e}")))?;

        let public_key_hex = extract_compressed_public_key(&private_key)?;

        Ok(Self {
            private_key,
            public_key_hex,
        })
    }

    /// Load an existing Secure Enclave key by its label.
    pub fn load(label: &str) -> Result<Self> {
        let private_key = find_key_by_label(label)?;
        let public_key_hex = extract_compressed_public_key(&private_key)?;
        Ok(Self {
            private_key,
            public_key_hex,
        })
    }
}

impl Signer for SecureEnclaveSigner {
    fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        self.private_key
            .create_signature(Algorithm::ECDSASignatureMessageX962SHA256, message)
            .map_err(|e| SignerError::SecureEnclave(format!("Signing failed: {e}")))
    }
}

fn extract_compressed_public_key(private_key: &SecKey) -> Result<String> {
    let pub_key = private_key
        .public_key()
        .ok_or_else(|| SignerError::SecureEnclave("Failed to get public key".into()))?;

    let raw = pub_key
        .external_representation()
        .ok_or_else(|| SignerError::SecureEnclave("Failed to export public key".into()))?;

    let bytes: &[u8] = raw.bytes();
    // Expect 65 bytes: 0x04 || x (32) || y (32)
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(SignerError::SecureEnclave(
            "Unexpected public key format".into(),
        ));
    }

    let x = &bytes[1..33];
    let y = &bytes[33..65];
    let prefix = if y[31] % 2 == 0 { 0x02 } else { 0x03 };

    let mut compressed = vec![prefix];
    compressed.extend_from_slice(x);

    Ok(format!("0x{}", hex::encode(&compressed)))
}

fn find_key_by_label(label: &str) -> Result<SecKey> {
    unsafe {
        let query = CFDictionary::from_CFType_pairs(&[
            (
                CFString::wrap_under_get_rule(kSecClass).as_CFType(),
                CFString::wrap_under_get_rule(kSecClassKey).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecAttrLabel).as_CFType(),
                CFString::new(label).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecReturnRef).as_CFType(),
                CFBoolean::true_value().as_CFType(),
            ),
        ]);

        let mut result = std::ptr::null();
        let status = SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result);

        if status != 0 || result.is_null() {
            return Err(SignerError::SecureEnclave(format!(
                "Key not found for label '{label}' (status: {status})"
            )));
        }

        Ok(SecKey::wrap_under_create_rule(result as _))
    }
}

/// Delete a Secure Enclave key by label. Used for test cleanup.
pub fn delete_key(label: &str) -> Result<()> {
    unsafe {
        let query = CFDictionary::from_CFType_pairs(&[
            (
                CFString::wrap_under_get_rule(kSecClass).as_CFType(),
                CFString::wrap_under_get_rule(kSecClassKey).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecAttrLabel).as_CFType(),
                CFString::new(label).as_CFType(),
            ),
        ]);

        let status =
            security_framework_sys::keychain_item::SecItemDelete(query.as_concrete_TypeRef());

        if status != 0 {
            return Err(SignerError::SecureEnclave(format!(
                "Failed to delete key '{label}' (status: {status})"
            )));
        }

        Ok(())
    }
}
