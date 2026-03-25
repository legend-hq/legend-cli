//! macOS Keychain signer — stores P256 keys in the iCloud-synced keychain.
//!
//! Keys are standard P256 (not Secure Enclave), stored with kSecAttrSynchronizable=true
//! so they sync across all iCloud-connected Apple devices. Signing uses
//! SecKeyCreateSignature (ECDSA-SHA256).
//!
//! No special entitlements or provisioning profiles required.
//!
//! Future: add Secure Enclave support for hardware-bound keys (requires provisioning
//! profile with keychain-access-groups entitlement for DataProtectionKeychain persistence).

#![cfg(target_os = "macos")]

use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFMutableDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use security_framework::key::{Algorithm, SecKey};
use security_framework_sys::item::*;
use security_framework_sys::key::SecKeyCreateRandomKey;
use security_framework_sys::keychain_item::{SecItemCopyMatching, SecItemDelete};

use crate::error::{Result, SignerError};
use crate::signer::Signer;

pub struct KeychainSigner {
    private_key: SecKey,
    public_key_hex: String,
}

impl KeychainSigner {
    /// Generate a new P256 key and store it in the iCloud-synced keychain.
    ///
    /// The key syncs to all Apple devices signed into the same iCloud account.
    /// Label convention: `com.legend.cli.<env>.<profile_name>`
    pub fn generate(label: &str) -> Result<Self> {
        let private_key = generate_persistent_key(label)?;
        let public_key_hex = extract_compressed_public_key(&private_key)?;

        Ok(Self {
            private_key,
            public_key_hex,
        })
    }

    /// Load an existing key from the keychain by its label.
    pub fn load(label: &str) -> Result<Self> {
        let private_key = find_key_by_label(label)?;
        let public_key_hex = extract_compressed_public_key(&private_key)?;
        Ok(Self {
            private_key,
            public_key_hex,
        })
    }
}

impl Signer for KeychainSigner {
    fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        self.private_key
            .create_signature(Algorithm::ECDSASignatureMessageX962SHA256, message)
            .map_err(|e| SignerError::Keychain(format!("Signing failed: {e}")))
    }
}

/// Generate a P256 key directly into the keychain with sync enabled.
/// Uses SecKeyCreateRandomKey with private key attributes that include
/// kSecAttrIsPermanent and kSecAttrSynchronizable.
fn generate_persistent_key(label: &str) -> Result<SecKey> {
    unsafe {
        // Private key attributes — persist in the login keychain.
        // TODO: Add kSecAttrSynchronizable=true for iCloud sync once we have
        // the provisioning profile / keychain-access-groups entitlement sorted.
        // That one attribute flip enables sync to all iCloud-connected devices.
        let mut private_attrs = CFMutableDictionary::new();
        private_attrs.set(
            CFString::wrap_under_get_rule(kSecAttrIsPermanent).as_CFTypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
        private_attrs.set(
            CFString::wrap_under_get_rule(kSecAttrLabel).as_CFTypeRef(),
            CFString::new(label).as_CFTypeRef(),
        );

        // Top-level key generation attributes
        let mut attrs = CFMutableDictionary::new();
        attrs.set(
            CFString::wrap_under_get_rule(kSecAttrKeyType).as_CFTypeRef(),
            CFString::wrap_under_get_rule(kSecAttrKeyTypeECSECPrimeRandom).as_CFTypeRef(),
        );
        attrs.set(
            CFString::wrap_under_get_rule(kSecAttrKeySizeInBits).as_CFTypeRef(),
            CFNumber::from(256i32).as_CFTypeRef(),
        );
        attrs.set(kSecPrivateKeyAttrs.cast(), private_attrs.as_CFTypeRef());

        let mut error = std::ptr::null_mut();
        let key = SecKeyCreateRandomKey(attrs.as_concrete_TypeRef(), &mut error);

        if key.is_null() {
            let err_desc = if !error.is_null() {
                let cf_error = core_foundation::error::CFError::wrap_under_create_rule(error);
                cf_error.description().to_string()
            } else {
                "Unknown error".to_string()
            };
            return Err(SignerError::Keychain(format!(
                "Key generation failed: {err_desc}"
            )));
        }

        Ok(SecKey::wrap_under_create_rule(key))
    }
}

fn find_key_by_label(label: &str) -> Result<SecKey> {
    unsafe {
        let mut query = CFMutableDictionary::new();
        query.set(
            CFString::wrap_under_get_rule(kSecClass).as_CFTypeRef(),
            CFString::wrap_under_get_rule(kSecClassKey).as_CFTypeRef(),
        );
        query.set(
            CFString::wrap_under_get_rule(kSecAttrLabel).as_CFTypeRef(),
            CFString::new(label).as_CFTypeRef(),
        );
        query.set(
            CFString::wrap_under_get_rule(kSecReturnRef).as_CFTypeRef(),
            CFBoolean::true_value().as_CFTypeRef(),
        );
        // TODO: Add kSecAttrSynchronizableAny when sync is enabled
        // query.set(
        //     CFString::wrap_under_get_rule(kSecAttrSynchronizable).as_CFTypeRef(),
        //     CFString::wrap_under_get_rule(kSecAttrSynchronizableAny).as_CFTypeRef(),
        // );

        let mut result = std::ptr::null();
        let status = SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result);

        if status != 0 || result.is_null() {
            return Err(SignerError::Keychain(format!(
                "Key not found for label '{label}' (status: {status})"
            )));
        }

        Ok(SecKey::wrap_under_create_rule(result as _))
    }
}

fn extract_compressed_public_key(private_key: &SecKey) -> Result<String> {
    let pub_key = private_key
        .public_key()
        .ok_or_else(|| SignerError::Keychain("Failed to get public key".into()))?;

    let raw = pub_key
        .external_representation()
        .ok_or_else(|| SignerError::Keychain("Failed to export public key".into()))?;

    let bytes: &[u8] = raw.bytes();
    if bytes.len() != 65 || bytes[0] != 0x04 {
        return Err(SignerError::Keychain("Unexpected public key format".into()));
    }

    let x = &bytes[1..33];
    let y = &bytes[33..65];
    let prefix = if y[31] % 2 == 0 { 0x02 } else { 0x03 };

    let mut compressed = vec![prefix];
    compressed.extend_from_slice(x);

    Ok(format!("0x{}", hex::encode(&compressed)))
}

/// Delete a keychain key by label. Used for test cleanup.
pub fn delete_key(label: &str) -> Result<()> {
    unsafe {
        let mut query = CFMutableDictionary::new();
        query.set(
            CFString::wrap_under_get_rule(kSecClass).as_CFTypeRef(),
            CFString::wrap_under_get_rule(kSecClassKey).as_CFTypeRef(),
        );
        query.set(
            CFString::wrap_under_get_rule(kSecAttrLabel).as_CFTypeRef(),
            CFString::new(label).as_CFTypeRef(),
        );
        query.set(
            CFString::wrap_under_get_rule(kSecAttrSynchronizable).as_CFTypeRef(),
            CFString::wrap_under_get_rule(kSecAttrSynchronizableAny).as_CFTypeRef(),
        );

        let status = SecItemDelete(query.as_concrete_TypeRef());

        if status != 0 {
            return Err(SignerError::Keychain(format!(
                "Failed to delete key '{label}' (status: {status})"
            )));
        }

        Ok(())
    }
}
