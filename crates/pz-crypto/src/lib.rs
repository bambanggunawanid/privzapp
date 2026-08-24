//! PrivZapp crypto primitives.
//!
//! Security posture:
//! - AES-256-GCM (authenticated encryption) for anything that could contain
//!   PII before it is persisted or transmitted.
//! - SHA-256 for one-way identifiers (never reversible hashes of raw PII with
//!   low entropy — callers must salt/scope appropriately).
//! - OS/browser CSPRNG (`getrandom`) for all randomness.
//!
//! Nothing in this crate phones home; it is pure computation.

#![forbid(unsafe_code)]

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use sha2::{Digest, Sha256};

pub const KEY_LEN: usize = 32;
pub const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

/// PBKDF2-HMAC-SHA256 rounds for password vaults (OWASP 2023 guidance).
const PBKDF2_ROUNDS: u32 = 600_000;

/// Magic prefix of a PrivZapp password vault (.pzv) file.
pub const VAULT_MAGIC: &[u8; 4] = b"PZV1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Ciphertext failed authentication or was malformed.
    Opening,
    /// Key material had the wrong length.
    BadKey,
    /// Input is not a PrivZapp vault (missing/unknown magic).
    NotAVault,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::Opening => write!(f, "could not decrypt: data corrupt or wrong key"),
            CryptoError::BadKey => write!(f, "key must be exactly {KEY_LEN} bytes"),
            CryptoError::NotAVault => write!(f, "this is not a PrivZapp vault (.pzv) file"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Cryptographically secure random bytes from the OS/browser CSPRNG.
pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    OsRng.fill_bytes(&mut buf);
    buf
}

/// Fresh AES-256 key.
pub fn generate_key() -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    OsRng.fill_bytes(&mut key);
    key
}

/// Encrypt with AES-256-GCM. Output layout: `nonce (12) || ciphertext+tag`.
/// A fresh random nonce is drawn per call — never reuse output slices as keys.
pub fn seal(key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != KEY_LEN {
        return Err(CryptoError::BadKey);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext)
        .map_err(|_| CryptoError::Opening)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt data produced by [`seal`].
pub fn open(key: &[u8], sealed: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if key.len() != KEY_LEN {
        return Err(CryptoError::BadKey);
    }
    if sealed.len() < NONCE_LEN {
        return Err(CryptoError::Opening);
    }
    let (nonce, ct) = sealed.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(Nonce::from_slice(nonce), ct)
        .map_err(|_| CryptoError::Opening)
}

/// Derive an AES-256 key from a password with PBKDF2-HMAC-SHA256.
/// Deliberately slow (600k rounds) to resist brute force; runs on-device.
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ROUNDS, &mut key);
    key
}

/// Password-encrypt into the PrivZapp vault format:
/// `"PZV1" || salt (16) || nonce (12) || ciphertext+tag`.
pub fn seal_with_password(password: &str, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let salt = random_bytes(SALT_LEN);
    let key = derive_key(password, &salt);
    let sealed = seal(&key, plaintext)?;
    let mut out = Vec::with_capacity(4 + SALT_LEN + sealed.len());
    out.extend_from_slice(VAULT_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Decrypt a vault produced by [`seal_with_password`].
pub fn open_with_password(password: &str, vault: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if vault.len() < 4 + SALT_LEN || &vault[..4] != VAULT_MAGIC {
        return Err(CryptoError::NotAVault);
    }
    let salt = &vault[4..4 + SALT_LEN];
    let key = derive_key(password, salt);
    open(&key, &vault[4 + SALT_LEN..])
}

/// Hex-encoded SHA-256 digest.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut s = String::with_capacity(64);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = generate_key();
        let sealed = seal(&key, b"secret document name").unwrap();
        assert_eq!(open(&key, &sealed).unwrap(), b"secret document name");
    }

    #[test]
    fn tamper_detected() {
        let key = generate_key();
        let mut sealed = seal(&key, b"hello").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert_eq!(open(&key, &sealed), Err(CryptoError::Opening));
    }

    #[test]
    fn wrong_key_fails() {
        let sealed = seal(&generate_key(), b"hello").unwrap();
        assert!(open(&generate_key(), &sealed).is_err());
    }

    #[test]
    fn password_roundtrip() {
        let vault = seal_with_password("hunter2", b"tax-return.pdf bytes").unwrap();
        assert_eq!(&vault[..4], b"PZV1");
        assert_eq!(
            open_with_password("hunter2", &vault).unwrap(),
            b"tax-return.pdf bytes"
        );
    }

    #[test]
    fn wrong_password_fails() {
        let vault = seal_with_password("hunter2", b"secret").unwrap();
        assert_eq!(
            open_with_password("hunter3", &vault),
            Err(CryptoError::Opening)
        );
    }

    #[test]
    fn not_a_vault_detected() {
        assert_eq!(
            open_with_password("pw", b"just a plain file"),
            Err(CryptoError::NotAVault)
        );
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
