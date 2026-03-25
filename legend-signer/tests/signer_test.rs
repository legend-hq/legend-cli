use legend_signer::*;
use p256::ecdsa::{VerifyingKey, signature::Verifier};

// --- FileSigner ---

#[test]
fn file_signer_generate_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.key");

    let signer1 = FileSigner::generate(&path).unwrap();
    let signer2 = FileSigner::load(&path).unwrap();

    assert_eq!(signer1.public_key_hex(), signer2.public_key_hex());

    // Both should produce valid signatures over the same message
    let msg = b"test message";
    let sig1 = signer1.sign(msg).unwrap();
    let sig2 = signer2.sign(msg).unwrap();
    assert!(!sig1.is_empty());
    assert!(!sig2.is_empty());

    // Verify both against the public key
    let vk = verifying_key_from_hex(signer1.public_key_hex());
    verify_der_signature(&vk, msg, &sig1);
    verify_der_signature(&vk, msg, &sig2);
}

#[test]
fn file_signer_public_key_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.key");

    let signer = FileSigner::generate(&path).unwrap();
    let pk = signer.public_key_hex();

    // 0x + 66 hex chars = 33 bytes compressed P256
    assert!(pk.starts_with("0x02") || pk.starts_with("0x03"));
    assert_eq!(pk.len(), 68);
}

#[cfg(unix)]
#[test]
fn file_signer_key_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.key");

    FileSigner::generate(&path).unwrap();

    let perms = std::fs::metadata(&path).unwrap().permissions();
    assert_eq!(perms.mode() & 0o777, 0o600);
}

#[test]
fn file_signer_from_known_key() {
    // Known key from the Elixir stamper.ex doctest
    let private_key_hex = "f2c9fa33bb68809f0c716542aa4fe2b5ee536f0472fbfce482f4a7b931d42fe0";
    let expected_public_key =
        "0x0371dd5ab2de3ba282ec0d5ee32a5425acd9280a1d70c5bb0e04d5c26d8ce04c41";

    let key_bytes = hex::decode(private_key_hex).unwrap();
    let signer = FileSigner::from_bytes(&key_bytes).unwrap();

    assert_eq!(signer.public_key_hex(), expected_public_key);
}

// --- Stamp verification (cross-language with Elixir) ---

#[test]
fn stamp_matches_elixir_reference() {
    // Test vector from turnkey/lib/turnkey/api_client/stamper.ex doctest
    let private_key_hex = "f2c9fa33bb68809f0c716542aa4fe2b5ee536f0472fbfce482f4a7b931d42fe0";
    let public_key_hex = "0x0371dd5ab2de3ba282ec0d5ee32a5425acd9280a1d70c5bb0e04d5c26d8ce04c41";
    let message = r#"{"organizationId": "105d7217-3600-42b5-a818-226efdb25019"}"#;

    let key_bytes = hex::decode(private_key_hex).unwrap();
    let signer = FileSigner::from_bytes(&key_bytes).unwrap();

    // Create a TurnkeyClient just to test stamping
    let client = TurnkeyClient::new(TurnkeyConfig {
        signer: Box::new(signer),
        sub_org_id: "unused".into(),
        ethereum_signer_address: "unused".into(),
        api_base_url: None,
        verbose: false,
    });

    let stamp_b64 = client.stamp(message).unwrap();

    // Decode the stamp
    use base64::Engine;
    let stamp_json = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&stamp_b64)
        .unwrap();
    let stamp: serde_json::Value = serde_json::from_slice(&stamp_json).unwrap();

    // Verify structure
    assert_eq!(
        stamp["publicKey"].as_str().unwrap(),
        public_key_hex.strip_prefix("0x").unwrap()
    );
    assert_eq!(
        stamp["scheme"].as_str().unwrap(),
        "SIGNATURE_SCHEME_TK_API_P256"
    );

    // Verify the signature is valid
    let sig_hex = stamp["signature"].as_str().unwrap();
    let sig_der = hex::decode(sig_hex).unwrap();

    let vk = verifying_key_from_hex(public_key_hex);
    verify_der_signature(&vk, message.as_bytes(), &sig_der);
}

// --- Helpers ---

fn verifying_key_from_hex(hex_str: &str) -> VerifyingKey {
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str).unwrap();
    VerifyingKey::from_sec1_bytes(&bytes).unwrap()
}

fn verify_der_signature(vk: &VerifyingKey, msg: &[u8], der_sig: &[u8]) {
    let sig = p256::ecdsa::DerSignature::from_bytes(der_sig).unwrap();
    vk.verify(msg, &sig).expect("Signature verification failed");
}
