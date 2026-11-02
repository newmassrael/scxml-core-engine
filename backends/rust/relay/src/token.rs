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
//!
//! Whoever holds the secret issues tokens the relay takes: the relay, and the service that logs a
//! person in and asks for their device's token, which holds the same key. The key is kept as a
//! short text ([`Authority::to_key_text`]): its audience and its secret, which is never printed.

use std::fmt;

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

/// Why the text of an authority's key was not taken: what is wrong with its form, never what it
/// holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyTextError(pub &'static str);

impl fmt::Display for KeyTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for KeyTextError {}

/// The first line of the text a key is kept in, which names the form and its version.
const KEY_TEXT_HEADER: &str = "sce-relay-authority 1";

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

    /// The relay this authority's tokens are for: a name, not a secret.
    pub fn audience(&self) -> &str {
        &self.audience
    }

    /// Whether `name` can be an audience: not empty, no space at either end, and no control
    /// character, so that it stays one line of the key's text.
    pub fn is_audience(name: &str) -> bool {
        !name.is_empty() && name.trim() == name && !name.chars().any(char::is_control)
    }

    /// The text the key is kept in: three lines, the form (`sce-relay-authority 1`), `audience
    /// <name>` and `secret <32 bytes in the URL alphabet>`. Whoever holds it issues tokens this
    /// relay takes, so it is kept where only the relay and the service that issues tokens read it.
    pub fn to_key_text(&self) -> String {
        format!(
            "{KEY_TEXT_HEADER}\naudience {}\nsecret {}\n",
            self.audience,
            encoding::encode(&self.secret)
        )
    }

    /// The authority whose key `text` is, as [`Authority::to_key_text`] writes it, and nothing
    /// else: no other line, no other form.
    pub fn from_key_text(text: &str) -> Result<Self, KeyTextError> {
        let mut lines = text.strip_suffix('\n').unwrap_or(text).split('\n');
        if lines.next() != Some(KEY_TEXT_HEADER) {
            return Err(KeyTextError(
                "its first line is not `sce-relay-authority 1`",
            ));
        }
        let audience = lines
            .next()
            .and_then(|line| line.strip_prefix("audience "))
            .filter(|name| Self::is_audience(name))
            .ok_or(KeyTextError(
                "its second line is not `audience <name>` with a name of one line",
            ))?;
        let secret: [u8; 32] = lines
            .next()
            .and_then(|line| line.strip_prefix("secret "))
            .and_then(encoding::decode)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(KeyTextError(
                "its third line is not `secret <32 bytes in the URL alphabet>`",
            ))?;
        if lines.next().is_some() {
            return Err(KeyTextError("it has more than three lines"));
        }
        Ok(Self::new(secret, audience))
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
        let (payload, tag) = halves(token)?;
        if !equal(&hmac_sha256(&self.secret, &payload), &tag) {
            return Err(TokenError::BadTag);
        }
        let Claims {
            audience,
            list,
            node,
            expires_at_ms: expires,
        } = Claims::read(&payload)?;
        if audience != self.audience {
            return Err(TokenError::WrongAudience);
        }
        if now_ms >= expires {
            return Err(TokenError::Expired);
        }
        Ok(Session { list, node })
    }
}

/// What a token says: the relay it is for, the device of one list it names, and when it expires.
/// Anyone who holds a token reads it ([`Claims::of`]), since the payload is not sealed; only the
/// relay's [`Authority::verify`] says that it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claims {
    pub audience: String,
    pub list: String,
    pub node: u64,
    pub expires_at_ms: u64,
}

impl Claims {
    /// What `token` says, read without the secret and so not checked: for a holder that needs to
    /// know which list a token it was handed is for (a probe, which writes only to a list of its
    /// own). A text that is not a token's form is [`TokenError::Malformed`].
    pub fn of(token: &str) -> Result<Self, TokenError> {
        let (payload, _) = halves(token)?;
        Self::read(&payload)
    }

    /// The claims a payload writes: four parts, each its length as four bytes big-endian and then
    /// its UTF-8, and no more.
    fn read(payload: &[u8]) -> Result<Self, TokenError> {
        let mut parts = Vec::new();
        let mut rest = payload;
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
        // A list has a name: the empty one names no list a store could keep.
        if list.is_empty() {
            return Err(TokenError::Malformed);
        }
        Ok(Self {
            audience: audience.to_string(),
            list: list.to_string(),
            node: node.parse().map_err(|_| TokenError::Malformed)?,
            expires_at_ms: expires.parse().map_err(|_| TokenError::Malformed)?,
        })
    }
}

/// The payload and the tag of a token, each decoded from the URL alphabet.
fn halves(token: &str) -> Result<(Vec<u8>, Vec<u8>), TokenError> {
    let (payload, tag) = token.split_once('.').ok_or(TokenError::Malformed)?;
    let payload = encoding::decode(payload).ok_or(TokenError::Malformed)?;
    let tag = encoding::decode(tag).ok_or(TokenError::Malformed)?;
    Ok((payload, tag))
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

    /// The bytes a token is, pinned: a service in another language that issues tokens for a relay
    /// writes these for the same key and the same device.
    #[test]
    fn a_token_is_the_bytes_the_documentation_gives() {
        let authority = Authority::new([1; 32], "relay.example");
        let token = authority.issue("home", 7, 1_800_000_000_000);
        assert_eq!(
            token,
            "AAAADXJlbGF5LmV4YW1wbGUAAAAEaG9tZQAAAAE3AAAADTE4MDAwMDAwMDAwMDA\
             .z-lS-hdIHQ8_6aqSt6shwplWq3VOPIcCCbj0bLifhwo"
        );
    }

    #[test]
    fn a_holder_reads_what_a_token_names_and_only_the_relay_says_it_holds() {
        let authority = Authority::new([5; 32], "relay-a");
        let token = authority.issue("home", 7, 10_000);
        let named = Claims {
            audience: "relay-a".to_string(),
            list: "home".to_string(),
            node: 7,
            expires_at_ms: 10_000,
        };
        assert_eq!(Claims::of(&token), Ok(named.clone()));
        // Read whatever its tag, and not taken for it: reading is not checking.
        let (payload, _) = token.split_once('.').unwrap();
        let forged = format!("{payload}.AAAA");
        assert_eq!(Claims::of(&forged), Ok(named));
        assert_eq!(authority.verify(&forged, 0), Err(TokenError::BadTag));
        // An expired token still says what it named.
        assert_eq!(
            Claims::of(&authority.issue("home", 7, 1)).map(|claims| claims.expires_at_ms),
            Ok(1)
        );
        for bad in ["", "x", payload, "AAAA.AAAA"] {
            assert_eq!(Claims::of(bad), Err(TokenError::Malformed), "{bad}");
        }
        assert_eq!(
            Claims::of(&authority.issue("", 7, 10_000)),
            Err(TokenError::Malformed),
            "a list with no name"
        );
    }

    #[test]
    fn a_token_for_a_list_with_no_name_is_not_taken() {
        let authority = Authority::new([5; 32], "relay-a");
        let token = authority.issue("", 7, 10_000);
        assert_eq!(authority.verify(&token, 0), Err(TokenError::Malformed));
    }

    #[test]
    fn a_key_is_kept_as_three_lines_and_read_back_as_the_same_authority() {
        let authority = Authority::new([1; 32], "relay.example");
        let text = authority.to_key_text();
        assert_eq!(
            text,
            "sce-relay-authority 1\naudience relay.example\n\
             secret AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE\n"
        );
        let again = Authority::from_key_text(&text).unwrap();
        assert_eq!(again.audience(), "relay.example");
        let token = authority.issue("home", 7, 10_000);
        assert_eq!(again.issue("home", 7, 10_000), token, "the same secret");
        assert!(again.verify(&token, 0).is_ok());
        assert!(
            Authority::from_key_text(text.trim_end()).is_ok(),
            "the last line end is optional"
        );
    }

    #[test]
    fn a_key_text_that_is_not_the_form_is_refused_and_says_where() {
        let good = Authority::new([1; 32], "relay.example").to_key_text();
        let refused = |text: &str| Authority::from_key_text(text).err().map(|error| error.0);
        let wrong: Vec<(String, &str)> = vec![
            (String::new(), "first line"),
            (good.replace("authority 1", "authority 2"), "first line"),
            (good.replace('\n', "\r\n"), "first line"),
            (
                good.replace("audience relay.example", "audience "),
                "second line",
            ),
            (
                good.replace("audience relay.example", "audience  x"),
                "second line",
            ),
            (good.replace("audience ", "audience:"), "second line"),
            (good.replace("secret AQ", "secret A"), "third line"),
            (good.replace("secret AQ", "secret AQEB"), "third line"),
            (good.replace("secret ", "secret\t"), "third line"),
            (format!("{good}extra\n"), "more than three"),
            (format!("{good}\n"), "more than three"),
        ];
        for (text, said) in &wrong {
            let why = refused(text.as_str()).unwrap_or_else(|| panic!("taken: {text:?}"));
            assert!(why.contains(*said), "{text:?}: {why}");
            assert!(!why.contains("AQEB"), "the secret is never said: {why}");
        }
    }

    #[test]
    fn an_audience_is_one_line_with_no_space_at_either_end() {
        for good in ["relay.example", "wss://relay.example/sce-relay/1", "a b"] {
            assert!(Authority::is_audience(good), "{good}");
        }
        for bad in ["", " a", "a ", "a\nb", "a\tb", "a\u{7f}"] {
            assert!(!Authority::is_audience(bad), "{bad:?}");
        }
    }
}
