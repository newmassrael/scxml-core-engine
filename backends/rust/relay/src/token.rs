// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The token a device presents to a relay, bound to one list and one device.
//!
//! Who a person is (their login: OIDC, a password, a passkey) is the application's, and it is not the
//! key of anything. After a person logs in, the application asks the relay's [`Authority`] for a
//! token that names the list and the node of the device that person is using, with an expiry; the
//! device presents it when it connects. A relay therefore knows a device by the node number the
//! membership log gave it for that list and by nothing that names an account, and a token for one
//! list is no use for another, nor for another relay (the audience).
//!
//! A token is `payload.tag`, both in the URL alphabet: the payload is the audience, the list, the
//! node and the expiry, each written with its length, and the tag is HMAC-SHA256 of it under the
//! relay's secret. Short expiries are the revocation a token has; what a member taken off may still
//! do is decided at each request by the log, not by the token.

use sha2::{Digest, Sha256};

use sce_rust_trust::encoding;

/// A device of one list, as a token names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub list: String,
    pub node: u64,
}

/// Why a token was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenError {
    /// It is not what [`Authority::issue`] writes.
    Malformed,
    /// Its tag is not the one the relay's secret makes: made up, changed, or another relay's.
    BadTag,
    /// It was issued for another relay.
    WrongAudience,
    /// Its time has passed.
    Expired,
}

/// What issues and checks tokens for one relay.
#[derive(Clone)]
pub struct Authority {
    secret: [u8; 32],
    audience: String,
}

impl Authority {
    pub fn new(secret: [u8; 32], audience: &str) -> Self {
        Self {
            secret,
            audience: audience.to_string(),
        }
    }

    /// An authority with a secret drawn from the operating system, which is the only source a
    /// secret is drawn from here.
    pub fn generate(audience: &str) -> Result<Self, getrandom::Error> {
        let mut secret = [0u8; 32];
        getrandom::getrandom(&mut secret)?;
        Ok(Self::new(secret, audience))
    }

    /// A token for the device of `node` of `list`, good until `expires_at_ms` (the clock of the
    /// relay's host, in milliseconds).
    pub fn issue(&self, list: &str, node: u64, expires_at_ms: u64) -> String {
        let mut payload = Vec::new();
        for part in [
            self.audience.as_str(),
            list,
            &node.to_string(),
            &expires_at_ms.to_string(),
        ] {
            payload.extend_from_slice(&(part.len() as u32).to_be_bytes());
            payload.extend_from_slice(part.as_bytes());
        }
        let tag = hmac_sha256(&self.secret, &payload);
        format!("{}.{}", encoding::encode(&payload), encoding::encode(&tag))
    }

    /// The device a token names, when the relay issued it for itself and it has not expired.
    pub fn verify(&self, token: &str, now_ms: u64) -> Result<Session, TokenError> {
        let (payload, tag) = token.split_once('.').ok_or(TokenError::Malformed)?;
        let payload = encoding::decode(payload).ok_or(TokenError::Malformed)?;
        let tag = encoding::decode(tag).ok_or(TokenError::Malformed)?;
        if !equal(&hmac_sha256(&self.secret, &payload), &tag) {
            return Err(TokenError::BadTag);
        }
        let mut parts = Vec::new();
        let mut rest = payload.as_slice();
        while !rest.is_empty() {
            let length = u32::from_be_bytes(
                rest.get(..4)
                    .ok_or(TokenError::Malformed)?
                    .try_into()
                    .map_err(|_| TokenError::Malformed)?,
            ) as usize;
            let part = rest.get(4..4 + length).ok_or(TokenError::Malformed)?;
            parts.push(std::str::from_utf8(part).map_err(|_| TokenError::Malformed)?);
            rest = &rest[4 + length..];
        }
        let [audience, list, node, expires] = parts[..] else {
            return Err(TokenError::Malformed);
        };
        if audience != self.audience {
            return Err(TokenError::WrongAudience);
        }
        let expires: u64 = expires.parse().map_err(|_| TokenError::Malformed)?;
        if now_ms >= expires {
            return Err(TokenError::Expired);
        }
        Ok(Session {
            list: list.to_string(),
            node: node.parse().map_err(|_| TokenError::Malformed)?,
        })
    }
}

/// Whether two tags are the same, in time that does not depend on where they first differ.
fn equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// HMAC-SHA256 (RFC 2104) of `message` under `key`, for a key of at most 64 bytes.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    assert!(
        key.len() <= 64,
        "a key longer than a block is not used here"
    );
    let mut inner_pad = [0x36u8; 64];
    let mut outer_pad = [0x5cu8; 64];
    for (index, byte) in key.iter().enumerate() {
        inner_pad[index] ^= byte;
        outer_pad[index] ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn hmac_is_the_one_of_rfc_4231() {
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn a_token_names_the_device_of_one_list_until_it_expires() {
        let authority = Authority::new([5; 32], "relay-a");
        let token = authority.issue("home", 7, 10_000);
        assert_eq!(
            authority.verify(&token, 9_999),
            Ok(Session {
                list: "home".to_string(),
                node: 7
            })
        );
        assert_eq!(authority.verify(&token, 10_000), Err(TokenError::Expired));
    }

    #[test]
    fn a_token_that_is_made_up_changed_or_for_another_relay_is_not_taken() {
        let authority = Authority::new([5; 32], "relay-a");
        let token = authority.issue("home", 7, 10_000);
        assert_eq!(
            Authority::new([6; 32], "relay-a").verify(&token, 0),
            Err(TokenError::BadTag),
            "another secret"
        );
        assert_eq!(
            Authority::new([5; 32], "relay-b").verify(&token, 0),
            Err(TokenError::WrongAudience),
            "another relay with the same secret"
        );
        let (payload, tag) = token.split_once('.').unwrap();
        let other = authority.issue("home", 8, 10_000);
        let (other_payload, _) = other.split_once('.').unwrap();
        assert_eq!(
            authority.verify(&format!("{other_payload}.{tag}"), 0),
            Err(TokenError::BadTag),
            "node 8 under the tag of node 7"
        );
        for bad in ["", "x", "a.b.c", &format!("{payload}."), ".AAAA"] {
            assert!(authority.verify(bad, 0).is_err(), "{bad}");
        }
    }
}
