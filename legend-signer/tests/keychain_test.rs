//! iCloud Keychain signer e2e tests. macOS only.
//!
//! These tests generate real P256 keys in the iCloud-synced keychain,
//! sign data, verify, reload by label, and clean up.
//!
//! No code signing or entitlements required.

#[cfg(target_os = "macos")]
mod tests {
    use legend_signer::*;
    use p256::ecdsa::{VerifyingKey, signature::Verifier};

    const TEST_LABEL: &str = "com.legend.test.keychain_e2e";

    fn cleanup() {
        let _ = legend_signer::keychain::delete_key(TEST_LABEL);
    }

    #[test]
    #[ignore] // requires code-signed .app bundle with provisioning profile
    fn generate_sign_load_verify() {
        cleanup();

        // 1. Generate a key in iCloud Keychain
        let signer = KeychainSigner::generate(TEST_LABEL).expect("Keychain key generation failed");

        let pubkey_hex = signer.public_key_hex();
        eprintln!("Generated keychain key: {pubkey_hex}");

        // Verify public key format: 0x + 02/03 prefix + 32 bytes
        assert!(
            pubkey_hex.starts_with("0x02") || pubkey_hex.starts_with("0x03"),
            "Expected compressed P256 key, got: {pubkey_hex}"
        );
        assert_eq!(pubkey_hex.len(), 68);

        // 2. Sign a message
        let message = b"hello from icloud keychain";
        let sig_der = signer.sign(message).unwrap();
        assert!(!sig_der.is_empty());
        eprintln!(
            "Signature ({} bytes DER): {}",
            sig_der.len(),
            hex::encode(&sig_der)
        );

        // 3. Verify the signature
        let pk_bytes = hex::decode(pubkey_hex.strip_prefix("0x").unwrap()).unwrap();
        let vk = VerifyingKey::from_sec1_bytes(&pk_bytes).unwrap();
        let der_sig = p256::ecdsa::DerSignature::from_bytes(&sig_der).unwrap();
        vk.verify(message, &der_sig)
            .expect("Signature verification failed");
        eprintln!("Signature verified OK");

        // 4. Load the key by label
        let signer2 = KeychainSigner::load(TEST_LABEL).unwrap();
        assert_eq!(signer.public_key_hex(), signer2.public_key_hex());
        eprintln!("Loaded key matches");

        // 5. Sign again with the loaded key and verify
        let sig2_der = signer2.sign(message).unwrap();
        let der_sig2 = p256::ecdsa::DerSignature::from_bytes(&sig2_der).unwrap();
        vk.verify(message, &der_sig2)
            .expect("Loaded key signature verification failed");
        eprintln!("Loaded key signature verified OK");

        // 6. Clean up
        cleanup();
        eprintln!("Key deleted, test complete");
    }
}
