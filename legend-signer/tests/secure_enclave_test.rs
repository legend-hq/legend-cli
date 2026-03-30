#![cfg(feature = "keychain")]
//! Secure Enclave e2e tests. macOS only, requires keychain feature.
//!
//! These tests generate real keys in the Secure Enclave, sign data, and verify.
//! The test binary must be code-signed for SE access:
//!
//!   cargo test --package legend-signer --test secure_enclave_test --no-run
//!   codesign --force --sign - --entitlements ../entitlements.plist target/debug/deps/secure_enclave_test-*
//!   cargo test --package legend-signer --test secure_enclave_test -- --nocapture

#[cfg(target_os = "macos")]
mod tests {
    use legend_signer::*;
    use p256::ecdsa::{VerifyingKey, signature::Verifier};

    const TEST_LABEL: &str = "com.legend.test.secure_enclave_e2e";

    fn cleanup() {
        #[cfg(target_os = "macos")]
        let _ = legend_signer::secure_enclave::delete_key(TEST_LABEL);
    }

    #[test]
    #[ignore] // requires code-signed binary + Secure Enclave access
    fn generate_sign_load_verify() {
        cleanup();

        // 1. Generate a key in Secure Enclave
        let signer = match SecureEnclaveSigner::generate(TEST_LABEL) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("SE generate failed (may need code signing): {e}");
                eprintln!("To code-sign the test binary:");
                eprintln!(
                    "  cargo test --package legend-signer --test secure_enclave_test --no-run"
                );
                eprintln!(
                    "  codesign --force --sign - --entitlements ../entitlements.plist target/debug/deps/secure_enclave_test-*"
                );
                eprintln!(
                    "  cargo test --package legend-signer --test secure_enclave_test -- --nocapture"
                );
                panic!("SE key generation failed: {e}");
            }
        };

        let pubkey_hex = signer.public_key_hex();
        eprintln!("Generated SE key: {pubkey_hex}");

        // Verify public key format: 0x + 02/03 prefix + 32 bytes
        assert!(
            pubkey_hex.starts_with("0x02") || pubkey_hex.starts_with("0x03"),
            "Expected compressed P256 key, got: {pubkey_hex}"
        );
        assert_eq!(pubkey_hex.len(), 68); // "0x" + 66 hex chars = 33 bytes

        // 2. Sign a message
        let message = b"hello from secure enclave";
        let sig_der = signer.sign(message).unwrap();
        assert!(!sig_der.is_empty());
        eprintln!(
            "Signature ({} bytes DER): {}",
            sig_der.len(),
            hex::encode(&sig_der)
        );

        // 3. Verify the signature with the public key
        let pk_bytes = hex::decode(pubkey_hex.strip_prefix("0x").unwrap()).unwrap();
        let vk = VerifyingKey::from_sec1_bytes(&pk_bytes).unwrap();
        let der_sig = p256::ecdsa::DerSignature::from_bytes(&sig_der).unwrap();
        vk.verify(message, &der_sig)
            .expect("Signature verification failed");
        eprintln!("Signature verified OK");

        // 4. Load the key by label and verify it's the same
        let signer2 = SecureEnclaveSigner::load(TEST_LABEL).unwrap();
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
