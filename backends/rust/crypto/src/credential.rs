// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Bearer tokens a host issues and a server checks (ADR 0109, D3; SCE_MESH.md §mesh-22.5;
//! docs/SCE_HOST_FEATURES.md section 14).
//!
//! - **The text.** A token is `sce1.` followed by the base64url (RFC 4648 section 5), without
//!   padding, of 32 bytes: 48 bytes of text in all ([`token_text`], [`is_token`]).
//! - **What is kept.** A store keeps the SHA-256 of the text ([`token_digest`]), the name it proves,
//!   when it was issued, when it expires and whether it is revoked
//!   (`sce_rust_session_store::TokenRow`), never the text: a store read by someone else proves no
//!   name.
//! - **What is issued.** The 32 bytes are the HMAC-SHA-256, under the host's issuing key
//!   (`crate::keyring::Keyring::issuing_tag`), of the deterministic CBOR array [`issue_message`]
//!   writes of the invocation's identity and the name. A repeat under one identity (a restore that
//!   starts the invocation again) gives the same token, and the host keeps no token in the clear to
//!   give it.
//! - **What is checked.** [`StoredTokens`] is the reference check of a bearer server: a presented
//!   token is digested, looked up by its digest, its name compared with the claimed one in constant
//!   time ([`crate::equal::same_secret`]), and refused when it is revoked or expired.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64ct::{Base64UrlUnpadded, Encoding as _};
use sce_forge_runtime::cbor::{write_array_head, write_text, write_uint};
use sce_forge_runtime::codec::VecSink;
use sce_rust_session_store::{Digest, TokenStore};
use sha2::{Digest as _, Sha256};

use crate::cose::put;
use crate::equal::same_secret;

/// What every token's text begins with: the version of the token's form.
pub const TOKEN_PREFIX: &str = "sce1.";
/// The bytes a token's text spells after its prefix.
pub const TOKEN_SECRET_BYTES: usize = 32;
/// The base64url digits, without padding, of [`TOKEN_SECRET_BYTES`] bytes.
const TOKEN_DIGITS: usize = 43;
/// The bytes of a token's text: its prefix and its digits.
pub const TOKEN_TEXT_BYTES: usize = TOKEN_PREFIX.len() + TOKEN_DIGITS;

/// The text of the token whose bytes are `secret`. A token is a secret, so its digits are written
/// in time that does not depend on its bytes (`base64ct`, as libsodium's `sodium_bin2base64` is in
/// the C++ host).
pub fn token_text(secret: &[u8; TOKEN_SECRET_BYTES]) -> String {
    format!("{TOKEN_PREFIX}{}", Base64UrlUnpadded::encode_string(secret))
}

/// Whether `text` is a token's text: the prefix, then the base64url without padding of exactly
/// [`TOKEN_SECRET_BYTES`] bytes, in its one canonical spelling, read in time that does not depend
/// on its bytes.
pub fn is_token(text: &str) -> bool {
    let Some(digits) = text.strip_prefix(TOKEN_PREFIX) else {
        return false;
    };
    let mut secret = [0u8; TOKEN_SECRET_BYTES];
    digits.len() == TOKEN_DIGITS
        && Base64UrlUnpadded::decode(digits, &mut secret)
            .is_ok_and(|decoded| decoded.len() == TOKEN_SECRET_BYTES)
}

/// The digest a store keeps a token as, and finds a presented one by: the SHA-256 of its text.
pub fn token_digest(text: &str) -> Digest {
    Sha256::digest(text.as_bytes()).into()
}

/// The message whose HMAC-SHA-256 under the issuing key is the token an issue gives: the
/// deterministic CBOR array `[session_id, invoke_id, start, name]`, the invocation's identity
/// (docs/SCE_HOST_FEATURES.md section 1) and the name the token proves.
pub fn issue_message(session_id: &str, invoke_id: &str, start: u64, name: &str) -> Vec<u8> {
    let mut message = Vec::new();
    let mut w = VecSink::new(&mut message);
    put(write_array_head(&mut w, 4));
    put(write_text(&mut w, session_id));
    put(write_text(&mut w, invoke_id));
    put(write_uint(&mut w, start));
    put(write_text(&mut w, name));
    message
}

/// What a check reads the time from: milliseconds since the Unix epoch.
pub type NowMs = Box<dyn Fn() -> u64 + Send + Sync>;

/// The system's clock in milliseconds since the Unix epoch. A clock set before the epoch reads as
/// the latest moment there is, at which every token has expired: a check that cannot tell the time
/// proves no name.
fn system_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(u64::MAX, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// The reference check of the `bearer` authenticator (SCE_MESH.md §mesh-22.5): a token proves a
/// name when the store holds its digest for that name, not revoked and not expired.
pub struct StoredTokens {
    store: Arc<dyn TokenStore>,
    now_ms: NowMs,
}

impl StoredTokens {
    /// Tokens kept in `store`, checked against the system's clock.
    pub fn new(store: Arc<dyn TokenStore>) -> Self {
        Self::with_clock(store, Box::new(system_now_ms))
    }

    /// Tokens kept in `store`, checked against the time `now_ms` reads: for a test that moves
    /// time past an expiry.
    pub fn with_clock(store: Arc<dyn TokenStore>, now_ms: NowMs) -> Self {
        Self { store, now_ms }
    }

    /// Whether `token` proves `name` now: it is a token's text, the store holds its digest, the
    /// name it was issued for is `name` (compared in constant time), and it is neither revoked nor
    /// expired. A store that cannot be read proves nothing.
    pub fn check(&self, name: &str, token: &str) -> bool {
        if !is_token(token) {
            return false;
        }
        let Ok(Some(row)) = self.store.token(&token_digest(token)) else {
            return false;
        };
        same_secret(row.name.as_bytes(), name.as_bytes()) && row.live_at((self.now_ms)())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sce_rust_session_store::{CredentialStore, MemoryCredentials, TokenRow};

    #[test]
    fn a_token_is_its_prefix_and_the_base64url_of_its_bytes() {
        let text = token_text(&[0xFB; TOKEN_SECRET_BYTES]);
        assert_eq!(text.len(), TOKEN_TEXT_BYTES);
        assert_eq!(
            TOKEN_TEXT_BYTES, 48,
            "fits the 64 bytes of the issue's result"
        );
        assert_eq!(
            text, "sce1.-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_s",
            "the URL-safe alphabet, no padding"
        );
        assert!(is_token(&text));
        assert!(!is_token(&text[1..]), "without its prefix");
        assert!(!is_token(&format!("{text}=")), "padded");
        assert!(!is_token(&text[..TOKEN_TEXT_BYTES - 1]), "a digit short");
        assert!(
            !is_token(&format!("sce2.{}", &text[TOKEN_PREFIX.len()..])),
            "of another version"
        );
        assert!(
            !is_token("sce1.-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_t"),
            "a last digit whose unused bits are set is not the canonical spelling"
        );
    }

    #[test]
    fn a_tokens_digest_is_the_sha_256_of_its_text() {
        // FIPS 180-4's example "abc".
        assert_eq!(
            crate::hex(&token_digest("abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_issue_message_is_the_cbor_array_of_the_identity_and_the_name() {
        assert_eq!(
            crate::hex(&issue_message("s", "op", 24, "phone")),
            // [ "s", "op", 24, "phone" ]
            "84617362 6f701818 6570686f6e65".replace(' ', "")
        );
    }

    fn tokens_at(store: &MemoryCredentials, now: u64) -> StoredTokens {
        StoredTokens::with_clock(Arc::new(store.clone()), Box::new(move || now))
    }

    #[test]
    fn a_stored_token_proves_its_name_until_it_expires_or_is_revoked() {
        let store = MemoryCredentials::new();
        let token = token_text(&[7; TOKEN_SECRET_BYTES]);
        store
            .issue(&TokenRow {
                digest: token_digest(&token),
                name: "phone".to_string(),
                issued_ms: 1_000,
                expires_ms: 2_000,
                revoked: false,
            })
            .expect("issuing a token");
        assert!(tokens_at(&store, 1_000).check("phone", &token));
        assert!(
            !tokens_at(&store, 1_000).check("tablet", &token),
            "another name"
        );
        assert!(
            !tokens_at(&store, 1_000).check("phon", &token),
            "a name that is a prefix of its own"
        );
        assert!(
            !tokens_at(&store, 1_000).check("phone", &token_text(&[8; TOKEN_SECRET_BYTES])),
            "a token the store does not hold"
        );
        assert!(
            !tokens_at(&store, 1_000).check("phone", "Bearer"),
            "a text that is no token"
        );
        assert!(!tokens_at(&store, 2_000).check("phone", &token), "expired");
        store.revoke("phone").expect("revoking phone");
        assert!(!tokens_at(&store, 1_000).check("phone", &token), "revoked");
    }
}
