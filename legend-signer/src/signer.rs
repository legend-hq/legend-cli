use crate::error::Result;

/// A P256 key that can sign messages.
///
/// Implementations handle key storage (Secure Enclave, file, etc.).
/// The `sign` method performs ECDSA-SHA256 over the raw message bytes
/// and returns a DER-encoded signature.
pub trait Signer: Send + Sync {
    /// Compressed P256 public key (33 bytes), hex-encoded with 0x prefix.
    fn public_key_hex(&self) -> &str;

    /// Sign a message with ECDSA-P256-SHA256.
    /// The implementation hashes with SHA-256 internally.
    /// Returns a DER-encoded ECDSA signature.
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;
}
