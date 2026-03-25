use std::path::Path;

use ecdsa::signature::Signer as _;
use p256::ecdsa::{DerSignature, SigningKey, VerifyingKey};
use rand::rngs::OsRng;

use crate::error::{Result, SignerError};
use crate::signer::Signer;

pub struct FileSigner {
    signing_key: SigningKey,
    public_key_hex: String,
}

impl FileSigner {
    /// Generate a new P256 keypair. Writes private key hex to `path` (chmod 600).
    pub fn generate(path: &Path) -> Result<Self> {
        let signing_key = SigningKey::random(&mut OsRng);
        let private_hex = hex::encode(signing_key.to_bytes());

        std::fs::write(path, &private_hex)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }

        let public_key_hex = compress_verifying_key(signing_key.verifying_key());
        Ok(Self {
            signing_key,
            public_key_hex,
        })
    }

    /// Load a P256 private key from a hex file.
    pub fn load(path: &Path) -> Result<Self> {
        let hex_str = std::fs::read_to_string(path)?.trim().to_string();
        let key_bytes = hex::decode(&hex_str)?;
        let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())
            .map_err(|e| SignerError::InvalidKey(format!("Invalid P256 key: {e}")))?;

        let public_key_hex = compress_verifying_key(signing_key.verifying_key());
        Ok(Self {
            signing_key,
            public_key_hex,
        })
    }

    /// Create a FileSigner directly from a private key byte slice (for testing).
    pub fn from_bytes(key_bytes: &[u8]) -> Result<Self> {
        let signing_key = SigningKey::from_bytes(key_bytes.into())
            .map_err(|e| SignerError::InvalidKey(format!("Invalid P256 key: {e}")))?;

        let public_key_hex = compress_verifying_key(signing_key.verifying_key());
        Ok(Self {
            signing_key,
            public_key_hex,
        })
    }
}

impl Signer for FileSigner {
    fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        let sig: DerSignature = self.signing_key.sign(message);
        Ok(sig.to_bytes().to_vec())
    }
}

fn compress_verifying_key(vk: &VerifyingKey) -> String {
    let point = vk.to_encoded_point(true);
    format!("0x{}", hex::encode(point.as_bytes()))
}
