// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! An administrator's invitation of a device that is not in the list's log yet, the proof the
//! device joins with, the text a host shows as a QR code, and the fingerprint two people compare
//! (ADR 0080 section 4).
//!
//! **The invitation.** An administrator's device draws a secret `s` of 32 bytes and makes from it an
//! Ed25519 key pair, the seed being HKDF-SHA256 of `s` with no salt and the info
//! `sce-relay/invitation-key/1` ([`key_of`]). It signs with an administrator's key of the log the
//! parts `sce-relay/invitation/1`, the audience of the relay, the list, the hash of the genesis in
//! the URL alphabet, the node in decimal, the instant the invitation expires at in decimal
//! milliseconds and the public key `P_s` in the URL alphabet, each in the trust layer's framing
//! ([`Invitation::signing_bytes`]). The node is one the log never granted; the relay holds the log
//! and checks the signature, the administrator, the genesis, the expiry and that the node was never
//! granted.
//!
//! **The proof.** The joining device signs, with the key made from `s`, the parts
//! `sce-relay/join/1`, the audience it was given in the invitation, the list, the node, the relay's
//! nonce in the URL alphabet, the instant the nonce expires at, its identity key in the URL alphabet
//! and the SHA-256 of its key package in the URL alphabet (of no bytes for a device that leaves
//! none, a list its administrator admits by a grant) ([`Proof`]). The relay checks it under `P_s`,
//! which is all it is given: it cannot make a proof of its own. The administrator's device takes only
//! a key package whose proof holds under the `P_s` it made for that node, and grants exactly the
//! identity key the proof names.
//!
//! **The text.** What the person being invited scans is the relay's address, the signed invitation
//! and `s` ([`Ticket::to_text`]): `SCEI1.` and, in the URL alphabet, each part its length as two
//! bytes big-endian and then its bytes, in the order the relay's address, the audience, the list (each
//! UTF-8), the genesis (32 bytes), the node and the expiry (each 8 bytes big-endian), the
//! administrator's key (32 bytes), the signature (64 bytes) and `s` (32 bytes). `P_s` is not carried:
//! it is made from `s`. Whoever holds the text can race the invited device to the relay, which is
//! why the invitation is used once and lives at most a day, and why the two people compare the
//! fingerprint before the administrator's device admits a key.
//!
//! **The fingerprint.** SHA-256 of the parts `sce-relay/fingerprint/1`, the list, the node, `P_s`
//! and the identity key (the keys in the URL alphabet), in the trust layer's framing; of the first
//! twenty bytes, each four as a big-endian number modulo 10 000, written as four decimal digits, the
//! five groups joined by a space ([`fingerprint`]): twenty digits, about 66 bits, the same on the
//! administrator's device and the invited one, in every lane, since each reads it from the core.

use core::fmt;

use hkdf::Hkdf;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::encoding;
use crate::identity::{Identity, PublicKey};
use crate::membership::Hash;
use crate::message::part;

/// The domain an administrator signs an invitation under.
pub const INVITATION_DOMAIN: &str = "sce-relay/invitation/1";
/// The domain the joining device signs its proof under.
pub const JOIN_DOMAIN: &str = "sce-relay/join/1";
/// The info the key of an invitation is made from its secret under.
pub const KEY_INFO: &str = "sce-relay/invitation-key/1";
/// The domain of the fingerprint two people compare.
pub const FINGERPRINT_DOMAIN: &str = "sce-relay/fingerprint/1";
/// The prefix of the text a host shows as a QR code, and its version.
pub const TEXT_PREFIX: &str = "SCEI1.";
/// The longest an invitation lives: a day.
pub const MOST_MS: u64 = 86_400_000;

/// A secret of an invitation, drawn from the operating system's source of randomness, the only
/// source it is drawn from.
pub fn draw_secret() -> Result<[u8; 32], getrandom::Error> {
    let mut secret = [0u8; 32];
    getrandom::getrandom(&mut secret)?;
    Ok(secret)
}

/// The key pair an invitation's secret makes: the seed is HKDF-SHA256 of `secret`, with no salt and
/// the info [`KEY_INFO`].
pub fn key_of(secret: &[u8; 32]) -> Identity {
    let mut seed = [0u8; 32];
    Hkdf::<Sha256>::new(None, secret)
        .expand(KEY_INFO.as_bytes(), &mut seed)
        .expect("32 bytes is a length HKDF-SHA256 gives");
    let identity = Identity::from_seed(seed);
    seed.zeroize();
    identity
}

/// The digits two people compare before the administrator's device admits the device of `node` of
/// `list` that signs with `key`, under the invitation whose key is `invitation_key`.
pub fn fingerprint(list: &str, node: u64, invitation_key: &PublicKey, key: &PublicKey) -> String {
    let mut bytes = Vec::new();
    part(&mut bytes, FINGERPRINT_DOMAIN);
    part(&mut bytes, list);
    part(&mut bytes, &node.to_string());
    part(&mut bytes, &invitation_key.to_text());
    part(&mut bytes, &key.to_text());
    let hash = Sha256::digest(&bytes);
    hash[..20]
        .chunks(4)
        .map(|four| {
            let number = u32::from_be_bytes([four[0], four[1], four[2], four[3]]);
            format!("{:04}", number % 10_000)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// An administrator's invitation of the device of `node` of `list`, at the relay named `audience`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invitation {
    pub audience: String,
    pub list: String,
    /// The hash of the list's genesis, which the invited device's log takes and no other.
    pub genesis: Hash,
    pub node: u64,
    /// The instant it expires at, in milliseconds since the Unix epoch.
    pub expires_at_ms: u64,
    /// `P_s`: the public key the invitation's secret makes.
    pub key: PublicKey,
    /// The administrator's key that signed it.
    pub administrator: PublicKey,
    pub signature: [u8; 64],
}

impl Invitation {
    /// The invitation `administrator` signs, of the device of `node` of `list` whose genesis is
    /// `genesis`, at the relay named `audience`, until `expires_at_ms`, under the key `key` an
    /// invitation's secret makes.
    pub fn sign(
        administrator: &Identity,
        audience: &str,
        list: &str,
        genesis: Hash,
        node: u64,
        expires_at_ms: u64,
        key: PublicKey,
    ) -> Self {
        let mut invitation = Self {
            audience: audience.to_string(),
            list: list.to_string(),
            genesis,
            node,
            expires_at_ms,
            key,
            administrator: administrator.public(),
            signature: [0; 64],
        };
        invitation.signature = administrator.sign(&invitation.signing_bytes());
        invitation
    }

    /// The bytes the administrator signs.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        part(&mut bytes, INVITATION_DOMAIN);
        part(&mut bytes, &self.audience);
        part(&mut bytes, &self.list);
        part(&mut bytes, &encoding::encode(&self.genesis));
        part(&mut bytes, &self.node.to_string());
        part(&mut bytes, &self.expires_at_ms.to_string());
        part(&mut bytes, &self.key.to_text());
        bytes
    }

    /// Whether the administrator it names signed it. Whether that key is an administrator's of the
    /// list is the log's to say.
    pub fn is_signed(&self) -> bool {
        self.administrator
            .verifies(&self.signing_bytes(), &self.signature)
    }

    /// The invitation as the JSON a join carries and a relay keeps: every hash, key and signature in
    /// the URL alphabet, and every 64-bit number as text.
    pub fn to_value(&self) -> Value {
        json!({
            "audience": self.audience,
            "list": self.list,
            "genesis": encoding::encode(&self.genesis),
            "node": self.node.to_string(),
            "expires_at_ms": self.expires_at_ms.to_string(),
            "key": self.key.to_text(),
            "administrator": self.administrator.to_text(),
            "signature": encoding::encode(&self.signature),
        })
    }

    /// Read back what [`Self::to_value`] wrote; `None` when the value is not that.
    pub fn from_value(value: &Value) -> Option<Self> {
        let text = |name: &str| value.get(name)?.as_str();
        Some(Self {
            audience: text("audience")?.to_string(),
            list: text("list")?.to_string(),
            genesis: encoding::decode(text("genesis")?)?.try_into().ok()?,
            node: text("node")?.parse().ok()?,
            expires_at_ms: text("expires_at_ms")?.parse().ok()?,
            key: PublicKey::from_text(text("key")?)?,
            administrator: PublicKey::from_text(text("administrator")?)?,
            signature: encoding::decode(text("signature")?)?.try_into().ok()?,
        })
    }
}

/// The SHA-256 of a key package, in the URL alphabet, as the proof signs it: of no bytes for a
/// device that leaves none.
fn package_digest(key_package: &[u8]) -> String {
    encoding::encode(&Sha256::digest(key_package))
}

/// What a device joins with: its identity key, the relay's nonce and when it expires, and the
/// signature by the key of the invitation's secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    /// The device's identity key, which the administrator's device grants.
    pub key: PublicKey,
    pub nonce: Vec<u8>,
    pub expires_at_ms: u64,
    pub signature: [u8; 64],
}

impl Proof {
    /// The bytes the key of the invitation's secret signs, at the relay named `audience` (the one the
    /// invitation carried), against the relay's `nonce` that expires at `expires_at_ms`.
    pub fn signing_bytes(
        audience: &str,
        invitation: &Invitation,
        nonce: &[u8],
        expires_at_ms: u64,
        key: &PublicKey,
        key_package: &[u8],
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        part(&mut bytes, JOIN_DOMAIN);
        part(&mut bytes, audience);
        part(&mut bytes, &invitation.list);
        part(&mut bytes, &invitation.node.to_string());
        part(&mut bytes, &encoding::encode(nonce));
        part(&mut bytes, &expires_at_ms.to_string());
        part(&mut bytes, &key.to_text());
        part(&mut bytes, &package_digest(key_package));
        bytes
    }

    /// The proof of the device that signs with `key` and leaves `key_package` (empty for none),
    /// made with the invitation's `secret` against the relay's `nonce` that expires at
    /// `expires_at_ms`, for the audience the invitation carried.
    pub fn make(
        secret: &[u8; 32],
        invitation: &Invitation,
        nonce: &[u8],
        expires_at_ms: u64,
        key: PublicKey,
        key_package: &[u8],
    ) -> Self {
        let bytes = Self::signing_bytes(
            &invitation.audience,
            invitation,
            nonce,
            expires_at_ms,
            &key,
            key_package,
        );
        Self {
            key,
            nonce: nonce.to_vec(),
            expires_at_ms,
            signature: key_of(secret).sign(&bytes),
        }
    }

    /// Whether the proof holds under the invitation's key `P_s`, for the audience the invitation
    /// carried and the key package `key_package` (empty for none).
    pub fn holds(&self, invitation: &Invitation, key_package: &[u8]) -> bool {
        let bytes = Self::signing_bytes(
            &invitation.audience,
            invitation,
            &self.nonce,
            self.expires_at_ms,
            &self.key,
            key_package,
        );
        invitation.key.verifies(&bytes, &self.signature)
    }
}

/// Why the text of an invitation was not read or not written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketError {
    /// It does not begin `SCEI1.`: another form, or a later version.
    NotThisForm,
    /// It is not what [`Ticket::to_text`] writes: not the URL alphabet, a part cut short or of
    /// another length, or bytes past the last part.
    Malformed,
    /// A text part is longer than two bytes of length say (65 535 bytes), and cannot be written.
    TooLong,
}

/// What the person being invited scans: the relay's address, the signed invitation and its secret.
#[derive(Clone, PartialEq, Eq)]
pub struct Ticket {
    /// The relay's address (`wss://host:port`), which the device dials.
    pub relay: String,
    pub invitation: Invitation,
    pub secret: [u8; 32],
}

impl Ticket {
    /// The text a host shows as a QR code (the module's note).
    pub fn to_text(&self) -> Result<String, TicketError> {
        let invitation = &self.invitation;
        let mut bytes = Vec::new();
        for given in [
            self.relay.as_bytes(),
            invitation.audience.as_bytes(),
            invitation.list.as_bytes(),
            &invitation.genesis,
            &invitation.node.to_be_bytes(),
            &invitation.expires_at_ms.to_be_bytes(),
            &invitation.administrator.to_bytes(),
            &invitation.signature,
            &self.secret,
        ] {
            let len = u16::try_from(given.len()).map_err(|_| TicketError::TooLong)?;
            bytes.extend_from_slice(&len.to_be_bytes());
            bytes.extend_from_slice(given);
        }
        Ok(format!("{TEXT_PREFIX}{}", encoding::encode(&bytes)))
    }

    /// Read back what [`Self::to_text`] wrote. `P_s` is made again from the secret.
    pub fn from_text(text: &str) -> Result<Self, TicketError> {
        let body = text
            .trim()
            .strip_prefix(TEXT_PREFIX)
            .ok_or(TicketError::NotThisForm)?;
        let bytes = encoding::decode(body).ok_or(TicketError::Malformed)?;
        let mut rest = bytes.as_slice();
        let mut take = || take_part(&mut rest);
        let utf8 =
            |given: &[u8]| String::from_utf8(given.to_vec()).map_err(|_| TicketError::Malformed);
        let relay = utf8(take()?)?;
        let audience = utf8(take()?)?;
        let list = utf8(take()?)?;
        let genesis: Hash = take()?.try_into().map_err(|_| TicketError::Malformed)?;
        let node = u64::from_be_bytes(take()?.try_into().map_err(|_| TicketError::Malformed)?);
        let expires_at_ms =
            u64::from_be_bytes(take()?.try_into().map_err(|_| TicketError::Malformed)?);
        let administrator: [u8; 32] = take()?.try_into().map_err(|_| TicketError::Malformed)?;
        let signature: [u8; 64] = take()?.try_into().map_err(|_| TicketError::Malformed)?;
        let secret: [u8; 32] = take()?.try_into().map_err(|_| TicketError::Malformed)?;
        if !rest.is_empty() {
            return Err(TicketError::Malformed);
        }
        Ok(Self {
            relay,
            invitation: Invitation {
                audience,
                list,
                genesis,
                node,
                expires_at_ms,
                key: key_of(&secret).public(),
                administrator: PublicKey::from_bytes(administrator),
                signature,
            },
            secret,
        })
    }
}

/// The next part of `rest` (its length as two bytes big-endian, then it), and `rest` past it.
fn take_part<'a>(rest: &mut &'a [u8]) -> Result<&'a [u8], TicketError> {
    let (len, after) = rest.split_at_checked(2).ok_or(TicketError::Malformed)?;
    let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
    let (given, after) = after.split_at_checked(len).ok_or(TicketError::Malformed)?;
    *rest = after;
    Ok(given)
}

impl fmt::Debug for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ticket")
            .field("relay", &self.relay)
            .field("invitation", &self.invitation)
            .finish_non_exhaustive()
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: [u8; 32] = [4; 32];

    fn invitation() -> Invitation {
        Invitation::sign(
            &Identity::from_seed([9; 32]),
            "relay.example",
            "home",
            [1; 32],
            2,
            86_400_000,
            key_of(&SECRET).public(),
        )
    }

    /// Each part is its length as eight bytes big-endian and then its bytes, in the order the ADR
    /// gives: a relay in another language checks these bytes.
    #[test]
    fn an_invitation_signs_its_parts_in_the_trust_layers_framing() {
        let invitation = invitation();
        let mut expected = Vec::new();
        for given in [
            INVITATION_DOMAIN.to_string(),
            "relay.example".to_string(),
            "home".to_string(),
            encoding::encode(&[1; 32]),
            "2".to_string(),
            "86400000".to_string(),
            key_of(&SECRET).public().to_text(),
        ] {
            expected.extend_from_slice(&(given.len() as u64).to_be_bytes());
            expected.extend_from_slice(given.as_bytes());
        }
        assert_eq!(invitation.signing_bytes(), expected);
        assert!(invitation.is_signed());
        let mut moved = invitation.clone();
        moved.node = 3;
        assert!(
            !moved.is_signed(),
            "a signature for one node is none for another"
        );
        assert_eq!(
            Invitation::from_value(&invitation.to_value()),
            Some(invitation)
        );
    }

    #[test]
    fn the_key_of_a_secret_is_the_same_for_the_same_secret_and_another_for_another() {
        assert_eq!(key_of(&SECRET).public(), key_of(&SECRET).public());
        assert_ne!(key_of(&SECRET).public(), key_of(&[5; 32]).public());
        assert_ne!(
            key_of(&SECRET).public(),
            Identity::from_seed(SECRET).public(),
            "the secret is not itself the seed"
        );
    }

    #[test]
    fn a_proof_holds_under_the_invitations_key_for_its_key_and_its_key_package_alone() {
        let invitation = invitation();
        let key = Identity::from_seed([2; 32]).public();
        let proof = Proof::make(&SECRET, &invitation, &[6; 32], 60_000, key, b"package");
        assert!(proof.holds(&invitation, b"package"));
        assert!(!proof.holds(&invitation, b"another package"));
        assert!(!proof.holds(&invitation, b""), "a key package left out");
        let other = Proof::make(&[5; 32], &invitation, &[6; 32], 60_000, key, b"package");
        assert!(
            !other.holds(&invitation, b"package"),
            "made under another secret"
        );
        let mut another_key = proof.clone();
        another_key.key = Identity::from_seed([3; 32]).public();
        assert!(!another_key.holds(&invitation, b"package"));
        let bare = Proof::make(&SECRET, &invitation, &[6; 32], 60_000, key, &[]);
        assert!(bare.holds(&invitation, &[]), "a device that leaves none");
    }

    #[test]
    fn the_text_reads_back_as_it_was_written_and_nothing_else_reads() {
        let ticket = Ticket {
            relay: "wss://relay.example".to_string(),
            invitation: invitation(),
            secret: SECRET,
        };
        let text = ticket.to_text().unwrap();
        assert!(text.starts_with(TEXT_PREFIX), "{text}");
        assert_eq!(Ticket::from_text(&text), Ok(ticket.clone()));
        assert_eq!(
            Ticket::from_text(&text.replacen("SCEI1.", "SCEI2.", 1)),
            Err(TicketError::NotThisForm)
        );
        assert_eq!(
            Ticket::from_text(&text[..text.len() - 4]),
            Err(TicketError::Malformed)
        );
        assert_eq!(
            Ticket::from_text(&format!("{text}AAA")),
            Err(TicketError::Malformed)
        );
        assert!(!format!("{ticket:?}").contains(&encoding::encode(&SECRET)));
    }

    /// The digits of `docs/SCE_SHARED_LIST_PEER_ABI.md` section 12, which a lane in another language
    /// that computes them itself checks against: the list `home`, node 2, the invitation's key of 32
    /// bytes each `0x11` and the device's key of 32 bytes each `0x22`.
    #[test]
    fn a_fingerprint_is_the_digits_the_documentation_gives() {
        assert_eq!(
            fingerprint(
                "home",
                2,
                &PublicKey::from_bytes([0x11; 32]),
                &PublicKey::from_bytes([0x22; 32])
            ),
            "2074 5706 5351 1674 5217"
        );
    }

    /// The digits are the construction's: SHA-256 of the framed parts, five groups of four digits.
    #[test]
    fn a_fingerprint_is_twenty_digits_the_same_from_either_side() {
        let p_s = key_of(&SECRET).public();
        let key = Identity::from_seed([2; 32]).public();
        let digits = fingerprint("home", 2, &p_s, &key);
        assert_eq!(digits.len(), 24, "{digits}");
        assert!(digits
            .split(' ')
            .all(|group| group.len() == 4 && group.bytes().all(|b| b.is_ascii_digit())));
        assert_eq!(fingerprint("home", 2, &p_s, &key), digits);
        assert_ne!(
            fingerprint("home", 2, &p_s, &Identity::from_seed([3; 32]).public()),
            digits
        );
        assert_ne!(fingerprint("home", 3, &p_s, &key), digits);
    }
}
