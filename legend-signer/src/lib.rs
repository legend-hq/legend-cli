pub mod error;
pub mod file_signer;
pub mod keychain;
pub mod signer;
pub mod turnkey;

// Secure Enclave signer exists but is not currently exported.
// It works for ephemeral keys but requires a provisioning profile with
// keychain-access-groups entitlement for persistent keys (DataProtectionKeychain).
// TODO: Re-enable once we have a provisioning profile set up.
#[allow(dead_code)]
pub mod secure_enclave;

pub use error::{Result, SignerError};
pub use file_signer::FileSigner;
pub use signer::Signer;
pub use turnkey::{TurnkeyClient, TurnkeyConfig};

#[cfg(target_os = "macos")]
pub use keychain::KeychainSigner;

#[cfg(target_os = "macos")]
pub use secure_enclave::SecureEnclaveSigner;
