// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A person's key. The seed is what a device keeps secret and the public key is what the
//! others are told; a signature is Ed25519's, which is deterministic, so signing the same
//! bytes again gives the same signature and a host that has lost what it signed can sign it
//! again.

use core::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::encoding;

/// A key that signs. It does not print its seed.
pub struct Identity {
    key: SigningKey,
}

impl Identity {
    /// The key a 32-byte secret seed makes. Where the seed comes from (the device's secure
    /// storage, a derivation from a passphrase) is the host's.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn public(&self) -> PublicKey {
        PublicKey(self.key.verifying_key().to_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.key.sign(message).to_bytes()
    }

    /// The secret that this key and `peer`'s agree on, the same one from either side: the
    /// signing key and the public key are taken as the points of the curve that a key
    /// agreement works on, so that a person has one key for both. `None` when `peer` is not
    /// a point of the curve.
    pub fn agree(&self, peer: &PublicKey) -> Option<[u8; 32]> {
        let peer = VerifyingKey::from_bytes(&peer.0).ok()?;
        let secret = x25519_dalek::StaticSecret::from(self.key.to_scalar_bytes());
        let public = x25519_dalek::PublicKey::from(peer.to_montgomery().to_bytes());
        let shared = secret.diffie_hellman(&public);
        // A peer that is a point of small order makes the secret all zeros, which anyone can
        // compute.
        (!shared.as_bytes().iter().all(|b| *b == 0)).then(|| *shared.as_bytes())
    }
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identity({})", self.public())
    }
}

/// A key that checks what a person signed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PublicKey([u8; 32]);

impl PublicKey {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// The key written as the text a certificate and a host's configuration carry.
    pub fn to_text(self) -> String {
        encoding::encode(&self.0)
    }

    pub fn from_text(text: &str) -> Option<Self> {
        let bytes: [u8; 32] = encoding::decode(text)?.try_into().ok()?;
        Some(Self(bytes))
    }

    /// Whether `signature` is what the holder of this key signed `message` as. A key that is
    /// not a point of the curve, and a signature that is not canonical, verify nothing.
    pub fn verifies(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let Ok(key) = VerifyingKey::from_bytes(&self.0) else {
            return false;
        };
        key.verify_strict(message, &Signature::from_bytes(signature))
            .is_ok()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_a_key_signs_its_public_key_verifies_and_nothing_else_does() {
        let alice = Identity::from_seed([1; 32]);
        let bob = Identity::from_seed([2; 32]);
        let signature = alice.sign(b"a write");
        assert!(alice.public().verifies(b"a write", &signature));
        assert!(!alice.public().verifies(b"another write", &signature));
        assert!(!bob.public().verifies(b"a write", &signature));
        assert_eq!(
            alice.sign(b"a write"),
            signature,
            "a signature is deterministic"
        );
    }

    #[test]
    fn a_key_reads_back_from_its_text_and_prints_no_seed() {
        let key = Identity::from_seed([7; 32]);
        let text = key.public().to_text();
        assert_eq!(PublicKey::from_text(&text), Some(key.public()));
        assert_eq!(PublicKey::from_text("short"), None);
        let shown = format!("{key:?}");
        assert!(shown.contains(&text) && !shown.contains("07"), "{shown}");
    }
}
