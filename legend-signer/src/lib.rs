pub mod error;
pub mod file_signer;
pub mod keychain;
pub mod signer;
pub mod turnkey;

// Secure Enclave signer — generates non-exportable P256 keys in hardware.
// Same .app bundle + provisioning profile + code-signing setup as the Keychain
// signer. SE keys are device-local (cannot sync via iCloud).
pub mod secure_enclave;

pub use error::{Result, SignerError};
pub use file_signer::FileSigner;
pub use signer::Signer;
pub use turnkey::{TurnkeyClient, TurnkeyConfig};

#[cfg(target_os = "macos")]
pub use keychain::KeychainSigner;

#[cfg(target_os = "macos")]
pub use secure_enclave::SecureEnclaveSigner;
