// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A key's material wrapped under a key-encryption key (ADR 0109, D7): envelope encryption, as
//! every key management service structures it (NIST SP 800-57 Part 1).
//!
//! The key store keeps what [`KeyWrap::wrap`] gives and never the key-encryption key, which the
//! program supplies through this port: [`KeyEncryptionKey`] is the reference adapter, a 32-byte key
//! read from a file or a variable the operator names; a key management service implements the
//! same two calls.

use std::fmt;
use std::path::Path;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::bytes_of_hex;
use crate::random::{RandomSource, SystemRandom};

/// Why a key could not be wrapped or unwrapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapError(pub String);

impl fmt::Display for WrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the key-encryption key could not be used: {}", self.0)
    }
}

impl std::error::Error for WrapError {}

/// What wraps a key's material for a key store and unwraps it again.
pub trait KeyWrap: Send + Sync {
    /// `material` wrapped, bound to `context` (the key's id and use), so that wrapped material
    /// moved to another key's row does not unwrap there.
    fn wrap(&self, context: &[u8], material: &[u8]) -> Result<Vec<u8>, WrapError>;

    /// The material `wrapped` holds, which [`KeyWrap::wrap`] gave for `context`; refused when it
    /// was wrapped for another context, under another key-encryption key, or changed since.
    fn unwrap(&self, context: &[u8], wrapped: &[u8]) -> Result<Zeroizing<Vec<u8>>, WrapError>;
}

/// The bytes of a key-encryption key.
pub const KEY_ENCRYPTION_KEY_BYTES: usize = 32;

/// What a wrapping begins with: the version of its form.
const WRAPPED_VERSION: u8 = 1;
/// The bytes of an XChaCha20-Poly1305 nonce: 192 bits, which no count of wrappings under one key
/// exhausts when drawn at random.
const WRAP_NONCE_BYTES: usize = 24;
/// What every wrapping authenticates before its context, so that a wrapping is never taken for
/// another kind of sealed bytes.
const WRAP_DOMAIN: &[u8] = b"sce-key-wrap-1\0";

/// The reference key-encryption key: 32 bytes the program holds and the store never does. A
/// wrapping is the version byte `1`, a 192-bit nonce drawn from the operating system, and the
/// XChaCha20-Poly1305 ciphertext of the material with its tag, authenticating the context.
pub struct KeyEncryptionKey {
    key: Zeroizing<[u8; KEY_ENCRYPTION_KEY_BYTES]>,
}

/// The key is never shown.
impl fmt::Debug for KeyEncryptionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("KeyEncryptionKey(..)")
    }
}

impl KeyEncryptionKey {
    /// The key-encryption key of the bytes `key`.
    pub fn new(key: [u8; KEY_ENCRYPTION_KEY_BYTES]) -> Self {
        Self {
            key: Zeroizing::new(key),
        }
    }

    /// A key-encryption key drawn from the operating system, for a store that lives no longer
    /// than the process (a host's keys in memory); `None` when the system gave no bytes.
    pub fn drawn() -> Option<Self> {
        let mut key = Zeroizing::new([0u8; KEY_ENCRYPTION_KEY_BYTES]);
        if SystemRandom.fill(&mut key[..]) {
            Some(Self { key })
        } else {
            None
        }
    }

    /// The key-encryption key a file holds: its 32 bytes, or their 64 hexadecimal digits with
    /// surrounding white space.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, WrapError> {
        let path = path.as_ref();
        let read = Zeroizing::new(std::fs::read(path).map_err(|error| {
            WrapError(format!(
                "the key-encryption key file {} could not be read: {error}",
                path.display()
            ))
        })?);
        if read.len() == KEY_ENCRYPTION_KEY_BYTES {
            let mut key = [0u8; KEY_ENCRYPTION_KEY_BYTES];
            key.copy_from_slice(&read);
            let wrapped = Self::new(key);
            zeroize::Zeroize::zeroize(&mut key);
            return Ok(wrapped);
        }
        let text = std::str::from_utf8(&read).map_err(|_| {
            WrapError(format!(
                "the key-encryption key file {} holds neither 32 bytes nor their hexadecimal digits",
                path.display()
            ))
        })?;
        Self::from_hex(text.trim())
    }

    /// The key-encryption key the environment variable `name` holds as 64 hexadecimal digits.
    pub fn from_variable(name: &str) -> Result<Self, WrapError> {
        let text =
            Zeroizing::new(std::env::var(name).map_err(|_| {
                WrapError(format!("the variable {name} holds no key-encryption key"))
            })?);
        Self::from_hex(text.trim())
    }

    fn from_hex(text: &str) -> Result<Self, WrapError> {
        let bytes = Zeroizing::new(bytes_of_hex(text).unwrap_or_default());
        if bytes.len() != KEY_ENCRYPTION_KEY_BYTES {
            return Err(WrapError(
                "a key-encryption key is 64 hexadecimal digits".to_string(),
            ));
        }
        let mut key = [0u8; KEY_ENCRYPTION_KEY_BYTES];
        key.copy_from_slice(&bytes);
        let wrapped = Self::new(key);
        zeroize::Zeroize::zeroize(&mut key);
        Ok(wrapped)
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(Key::from_slice(&self.key[..]))
    }
}

/// What a wrapping authenticates: the domain, then the context.
fn associated(context: &[u8]) -> Vec<u8> {
    [WRAP_DOMAIN, context].concat()
}

impl KeyWrap for KeyEncryptionKey {
    fn wrap(&self, context: &[u8], material: &[u8]) -> Result<Vec<u8>, WrapError> {
        let mut nonce = [0u8; WRAP_NONCE_BYTES];
        if !SystemRandom.fill(&mut nonce) {
            return Err(WrapError("the operating system gave no nonce".to_string()));
        }
        let sealed = self
            .cipher()
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: material,
                    aad: &associated(context),
                },
            )
            .map_err(|_| WrapError("the material could not be sealed".to_string()))?;
        let mut wrapped = Vec::with_capacity(1 + WRAP_NONCE_BYTES + sealed.len());
        wrapped.push(WRAPPED_VERSION);
        wrapped.extend_from_slice(&nonce);
        wrapped.extend_from_slice(&sealed);
        Ok(wrapped)
    }

    fn unwrap(&self, context: &[u8], wrapped: &[u8]) -> Result<Zeroizing<Vec<u8>>, WrapError> {
        let refused = || {
            WrapError(
                "the wrapped material does not open under this key-encryption key for this key"
                    .to_string(),
            )
        };
        let (version, rest) = wrapped.split_first().ok_or_else(refused)?;
        if *version != WRAPPED_VERSION || rest.len() < WRAP_NONCE_BYTES {
            return Err(refused());
        }
        let (nonce, sealed) = rest.split_at(WRAP_NONCE_BYTES);
        self.cipher()
            .decrypt(
                XNonce::from_slice(nonce),
                Payload {
                    msg: sealed,
                    aad: &associated(context),
                },
            )
            .map(Zeroizing::new)
            .map_err(|_| refused())
    }
}

/// The wrap of a host whose operating system gave no key-encryption key: every wrapping and
/// unwrapping fails, which the host answers as `unavailable`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoKeyEncryptionKey;

impl KeyWrap for NoKeyEncryptionKey {
    fn wrap(&self, _: &[u8], _: &[u8]) -> Result<Vec<u8>, WrapError> {
        Err(WrapError(
            "the operating system gave no key-encryption key".to_string(),
        ))
    }

    fn unwrap(&self, _: &[u8], _: &[u8]) -> Result<Zeroizing<Vec<u8>>, WrapError> {
        Err(WrapError(
            "the operating system gave no key-encryption key".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_unwraps_under_its_key_and_context_alone() {
        let kek = KeyEncryptionKey::new([7; KEY_ENCRYPTION_KEY_BYTES]);
        let material = [0x42u8; 32];
        let wrapped = kek.wrap(b"generated:01/seal", &material).expect("wrapping");
        assert_eq!(wrapped.len(), 1 + WRAP_NONCE_BYTES + material.len() + 16);
        assert!(
            !wrapped.windows(material.len()).any(|w| w == material),
            "the wrapping does not hold the material in the clear"
        );
        assert_eq!(
            kek.unwrap(b"generated:01/seal", &wrapped)
                .expect("unwrapping")
                .as_slice(),
            material
        );
        assert!(
            kek.unwrap(b"generated:02/seal", &wrapped).is_err(),
            "another key's context"
        );
        let other = KeyEncryptionKey::new([8; KEY_ENCRYPTION_KEY_BYTES]);
        assert!(
            other.unwrap(b"generated:01/seal", &wrapped).is_err(),
            "another key-encryption key"
        );
        let mut changed = wrapped.clone();
        let last = changed.len() - 1;
        changed[last] ^= 0x01;
        assert!(
            kek.unwrap(b"generated:01/seal", &changed).is_err(),
            "a changed byte"
        );
        assert_ne!(
            kek.wrap(b"generated:01/seal", &material).expect("again"),
            wrapped,
            "each wrapping draws its own nonce"
        );
    }

    #[test]
    fn a_key_encryption_key_is_read_from_hexadecimal_digits_and_never_shown() {
        let key = KeyEncryptionKey::from_hex(&"0a".repeat(32)).expect("64 digits");
        assert_eq!(format!("{key:?}"), "KeyEncryptionKey(..)");
        assert!(
            KeyEncryptionKey::from_hex("0a0b").is_err(),
            "too few digits"
        );
        assert!(
            KeyEncryptionKey::from_hex(&"zz".repeat(32)).is_err(),
            "no digits"
        );
        let wrapped = KeyEncryptionKey::new([0x0a; KEY_ENCRYPTION_KEY_BYTES])
            .wrap(b"c", b"m")
            .expect("wrapping");
        assert_eq!(
            key.unwrap(b"c", &wrapped).expect("the same key").as_slice(),
            b"m"
        );
        assert!(NoKeyEncryptionKey.wrap(b"c", b"m").is_err());
    }
}
