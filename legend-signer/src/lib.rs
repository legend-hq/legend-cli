pub mod error;
pub mod file_signer;
pub mod signer;
pub mod turnkey;

#[cfg(feature = "keychain")]
pub mod keychain;

#[cfg(feature = "keychain")]
pub mod secure_enclave;

pub use error::{Result, SignerError};
pub use file_signer::FileSigner;
pub use signer::Signer;
pub use turnkey::{TurnkeyClient, TurnkeyConfig};

#[cfg(feature = "keychain")]
pub use keychain::KeychainSigner;

#[cfg(feature = "keychain")]
pub use secure_enclave::SecureEnclaveSigner;
