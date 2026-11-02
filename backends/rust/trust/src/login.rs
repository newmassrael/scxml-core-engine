// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The bytes a device signs to log in at a relay with the key its list's log gave its node (ADR
//! 0080 section 2), and that the relay checks against the log it holds.
//!
//! They are written in the trust layer's framing (each part its length as eight bytes big-endian,
//! then its bytes) under a domain of their own, `sce-relay/login/1`, which no other signature of
//! this crate is made under: a login signature is no event, operation, entry of the log, checkpoint
//! or link of a chain, and none of those is a login. The parts are the domain, the audience the
//! device was configured with (never the one a challenge said), the list, the node in decimal, the
//! relay's nonce in the URL alphabet (as every byte string of the trust layer's framing is written),
//! and the instant the challenge expires at in decimal milliseconds. So a signature is good at one
//! relay, for one node of one list, against one nonce, until one instant.
//!
//! What signs them is the device's person ([`crate::Trust::sign_login`]); what checks them is the
//! relay, under the key the latest grant of the node gives ([`crate::PublicKey::verifies`]).

use crate::encoding;
use crate::message::part;

/// The domain of a login signature.
pub const DOMAIN: &str = "sce-relay/login/1";

/// The bytes a device of `node` of `list` signs to log in at the relay named `audience`, against
/// the relay's `nonce` that expires at `expires_at_ms`.
pub fn signing_bytes(
    audience: &str,
    list: &str,
    node: u64,
    nonce: &[u8],
    expires_at_ms: u64,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    part(&mut bytes, DOMAIN);
    part(&mut bytes, audience);
    part(&mut bytes, list);
    part(&mut bytes, &node.to_string());
    part(&mut bytes, &encoding::encode(nonce));
    part(&mut bytes, &expires_at_ms.to_string());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Identity, Trust};

    /// Each part is its length as eight bytes big-endian and then its bytes, in the order the ADR
    /// gives: a relay in another language checks these bytes.
    #[test]
    fn the_bytes_are_the_parts_in_the_trust_layers_framing() {
        let bytes = signing_bytes("relay.example", "home", 7, &[1, 2, 3], 1_800_000_000_000);
        let mut expected = Vec::new();
        for part in [
            "sce-relay/login/1",
            "relay.example",
            "home",
            "7",
            "AQID",
            "1800000000000",
        ] {
            expected.extend_from_slice(&(part.len() as u64).to_be_bytes());
            expected.extend_from_slice(part.as_bytes());
        }
        assert_eq!(bytes, expected);
    }

    #[test]
    fn a_login_is_signed_under_a_domain_no_other_signature_of_the_trust_layer_uses() {
        for other in [
            "sce-trust/event/2",
            "sce-trust/operation/1",
            "sce-trust/membership/1",
            "sce-trust/checkpoint/1",
            "sce-trust/link/1",
        ] {
            assert_ne!(DOMAIN, other);
        }
    }

    #[test]
    fn a_person_signs_a_login_that_holds_for_one_relay_one_node_one_nonce_and_one_instant() {
        let trust = Trust::open("home");
        assert_eq!(
            trust.sign_login("relay-a", &[5; 32], 60_000),
            None,
            "no person"
        );
        let identity = Identity::from_seed([7; 32]);
        let key = identity.public();
        trust.set_identity(3, identity);
        let (node, signature) = trust.sign_login("relay-a", &[5; 32], 60_000).unwrap();
        assert_eq!(node, 3);
        let good = signing_bytes("relay-a", "home", 3, &[5; 32], 60_000);
        assert!(key.verifies(&good, &signature));
        for other in [
            signing_bytes("relay-b", "home", 3, &[5; 32], 60_000),
            signing_bytes("relay-a", "work", 3, &[5; 32], 60_000),
            signing_bytes("relay-a", "home", 4, &[5; 32], 60_000),
            signing_bytes("relay-a", "home", 3, &[6; 32], 60_000),
            signing_bytes("relay-a", "home", 3, &[5; 32], 60_001),
        ] {
            assert!(!key.verifies(&other, &signature));
        }
    }
}
