// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Ed25519 (RFC 8032): the one signature every Rust host of SCE makes and checks.
//!
//! A `COSE_Sign1` of a statechart (docs/SCE_HOST_FEATURES.md section 10) and an HTTP message
//! signature of the Mesh `https` binding (SCE_MESH.md section 22.9, ADR 0109 D9) sign with the
//! same primitive, so it is here once: a key pair made from its 32-byte seed, a signature over a
//! message, and the check of one, which refuses a signature that is not canonical and a public
//! key of small order (`verify_strict`), as libsodium's `crypto_sign_verify_detached` does on the
//! C++ side.

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use zeroize::Zeroizing;

use crate::random::RandomSource;

/// The bytes of a seed, the secret a key pair is made from (RFC 8032 section 5.1.5).
pub const SEED_BYTES: usize = 32;
/// The bytes of a public key.
pub const PUBLIC_KEY_BYTES: usize = 32;
/// The bytes of a signature.
pub const SIGNATURE_BYTES: usize = 64;

/// An Ed25519 key pair: its seed, held in memory that is wiped when the pair is dropped, and its
/// public key. Two pairs are equal when their public keys are, which a seed decides. `Debug`
/// prints the public key alone.
#[derive(Clone)]
pub struct KeyPair {
    seed: Zeroizing<[u8; SEED_BYTES]>,
    public: [u8; PUBLIC_KEY_BYTES],
}

impl KeyPair {
    /// The pair of `seed`.
    pub fn from_seed(seed: &[u8; SEED_BYTES]) -> Self {
        KeyPair {
            seed: Zeroizing::new(*seed),
            public: public_key_of(seed),
        }
    }

    /// A pair whose seed is drawn from `random`; `None` when the source gave no bytes.
    pub fn generate(random: &mut dyn RandomSource) -> Option<Self> {
        let mut seed = Zeroizing::new([0u8; SEED_BYTES]);
        random
            .fill(&mut seed[..])
            .then(|| KeyPair::from_seed(&seed))
    }

    /// The public key.
    pub fn public_key(&self) -> [u8; PUBLIC_KEY_BYTES] {
        self.public
    }

    /// The signature of `message` by this pair.
    pub fn sign(&self, message: &[u8]) -> [u8; SIGNATURE_BYTES] {
        sign(&self.seed, message)
    }
}

impl PartialEq for KeyPair {
    fn eq(&self, other: &Self) -> bool {
        self.public == other.public
    }
}

impl Eq for KeyPair {}

impl core::fmt::Debug for KeyPair {
    // A seed is a secret; a log that prints one hands the key out.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KeyPair")
            .field("public", &crate::hex(&self.public))
            .finish_non_exhaustive()
    }
}

/// The public key of the seed `seed`.
pub fn public_key_of(seed: &[u8; SEED_BYTES]) -> [u8; PUBLIC_KEY_BYTES] {
    SigningKey::from_bytes(seed).verifying_key().to_bytes()
}

/// The signature of `message` by the key of `seed` (RFC 8032 section 5.1.6).
pub fn sign(seed: &[u8; SEED_BYTES], message: &[u8]) -> [u8; SIGNATURE_BYTES] {
    SigningKey::from_bytes(seed).sign(message).to_bytes()
}

/// Whether `signature` is the signature of `message` by `public_key`. A signature of another
/// length, one that is not canonical, and a public key that is no point of the curve or of small
/// order are not.
pub fn verify(public_key: &[u8; PUBLIC_KEY_BYTES], message: &[u8], signature: &[u8]) -> bool {
    let Ok(signature) = <[u8; SIGNATURE_BYTES]>::try_from(signature) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature);
    VerifyingKey::from_bytes(public_key)
        .is_ok_and(|key| key.verify_strict(message, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes_of_hex;
    use crate::random::ScriptedRandom;

    fn bytes<const N: usize>(text: &str) -> [u8; N] {
        bytes_of_hex(text)
            .expect("hex")
            .try_into()
            .expect("the length")
    }

    /// RFC 8032 section 7.1, test 1: the seed, its public key and its signature of the empty
    /// message. Red if `sign` or `public_key_of` signs with another key.
    #[test]
    fn rfc_8032_test_1_signs_and_verifies() {
        let seed: [u8; 32] =
            bytes("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
        let public: [u8; 32] =
            bytes("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
        let signature: [u8; 64] = bytes(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        );
        let pair = KeyPair::from_seed(&seed);
        assert_eq!(pair.public_key(), public);
        assert_eq!(pair.sign(b""), signature);
        assert!(verify(&public, b"", &signature));
        assert!(!verify(&public, b"x", &signature), "another message");
        let mut flipped = signature;
        flipped[10] ^= 0x01;
        assert!(!verify(&public, b"", &flipped), "a changed signature");
        assert!(!verify(&public, b"", &signature[..63]), "a short signature");
    }

    #[test]
    fn a_generated_pair_is_drawn_from_its_source_and_prints_no_seed() {
        let mut source = ScriptedRandom::new();
        source.set(&[7u8; 32]);
        let pair = KeyPair::generate(&mut source).expect("a pair");
        assert_eq!(pair, KeyPair::from_seed(&[7u8; 32]));
        let printed = format!("{pair:?}");
        assert!(printed.contains(&crate::hex(&pair.public_key())));
        assert!(!printed.contains(&crate::hex(&[7u8; 32])), "{printed}");
    }
}
