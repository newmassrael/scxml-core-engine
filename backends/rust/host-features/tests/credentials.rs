// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Credentials through the reference host (docs/SCE_HOST_FEATURES.md, section
// 14; docs/adr/0109, D3): a token issued for a name is the one a server's
// stored-token check takes, a token issued twice under one identity is one
// token, and a revocation takes the name's tokens and its pin away, each
// answer, failure and refusal of the two operations.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sce_rust_crypto::credential::{
    issue_message, token_digest, token_text, StoredTokens, TOKEN_PREFIX, TOKEN_TEXT_BYTES,
};
use sce_rust_crypto::keyring::{Keyring, MemoryKeys, ISSUING_KEY_ID};
use sce_rust_crypto::mac::hmac_sha256;
use sce_rust_crypto::wrap::KeyEncryptionKey;
use sce_rust_host_features::{
    HttpRequest, HttpResponse, Invocation, MemoryCredentials, Outcome, Params, RandomSource,
    Reading, ReferenceHost, ScriptedClock, ScriptedRandom, Transport, TransportFailure,
};
use sce_rust_session_store::{
    CredentialStore, Digest, Failure, KeyStore, PinRow, PinStore, Revoked, TokenRow, TokenStore,
};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure::Network)
    }
}

/// A random source that gives nothing.
struct Exhausted;

impl RandomSource for Exhausted {
    fn fill(&mut self, _: &mut [u8]) -> bool {
        false
    }
}

/// The issuing key the scripted random source gives the host's first issue.
const ISSUING_KEY: [u8; 32] = [0x5D; 32];

/// The moment the host's clock reads, in milliseconds since the Unix epoch.
const NOW_MS: i64 = 1_700_000_000_000;

fn params(pairs: &[(&str, &str)]) -> Params {
    pairs
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

/// A host whose clock reads [`NOW_MS`], whose random source gives the issuing
/// key first, and which keeps its credentials in `store`; the clock is shared,
/// so a test moves it.
fn host(store: &MemoryCredentials) -> (ReferenceHost<Unreachable>, Arc<Mutex<ScriptedClock>>) {
    let clock = Arc::new(Mutex::new(ScriptedClock::new()));
    clock.lock().expect("the clock").reading = Some(Reading {
        utc_ms: NOW_MS,
        monotonic_ms: 0,
    });
    let mut reference = ReferenceHost::new(Box::new(Arc::clone(&clock)), Unreachable);
    let mut random = ScriptedRandom::new();
    random.set(&ISSUING_KEY);
    reference.set_random_source(Box::new(random));
    reference.set_credentials(Arc::new(store.clone()));
    (reference, clock)
}

fn start(
    host: &mut ReferenceHost<Unreachable>,
    operation: &str,
    invocation: &Invocation,
    pairs: &[(&str, &str)],
) -> Option<Outcome> {
    host.start(operation, invocation, &params(pairs))
}

fn done(outcome: Option<Outcome>) -> serde_json::Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

fn refused(outcome: Option<Outcome>) -> bool {
    matches!(outcome, Some(Outcome::Refuse(_)))
}

/// The token an issue of `name` for `ttl_s` seconds answers as `invocation`.
fn issue(
    host: &mut ReferenceHost<Unreachable>,
    invocation: &Invocation,
    name: &str,
    ttl_s: &str,
) -> String {
    let fields = done(start(
        host,
        "sce-host-credential-issue",
        invocation,
        &[("name", name), ("ttl_s", ttl_s)],
    ));
    assert_eq!(fields["error"], "", "{fields}");
    fields["token"].as_str().expect("a token").to_string()
}

fn first() -> Invocation {
    Invocation::new("session", "issue", 1)
}

/// The token of an issue is the HMAC-SHA-256 under the issuing key of the
/// invocation's identity and the name; the store keeps its digest with the
/// name, the moment of the issue and its expiry, and the server's check takes
/// it for that name until it expires.
#[test]
fn an_issued_token_proves_its_name_until_its_expiry() {
    let store = MemoryCredentials::new();
    let (mut reference, _clock) = host(&store);
    let token = issue(&mut reference, &first(), "phone", "3600");
    let tag = hmac_sha256(&ISSUING_KEY, &issue_message("session", "issue", 1, "phone"));
    assert_eq!(token, token_text(&tag));
    assert_eq!(
        token, "sce1.KJG0XTyXotgk5bba3j0fmD5ZMWB6R6-oxTnxxKT8I8g",
        "the token AStandardHostCredentials holds the C++ host to for the same key and identity"
    );
    assert!(token.starts_with(TOKEN_PREFIX) && token.len() == TOKEN_TEXT_BYTES);
    assert_eq!(
        store.tokens().expect("the tokens"),
        vec![TokenRow {
            digest: token_digest(&token),
            name: "phone".to_string(),
            issued_ms: NOW_MS as u64,
            expires_ms: NOW_MS as u64 + 3_600_000,
            revoked: false,
            sealed: Vec::new(),
        }],
        "the store keeps the token's digest, never the token"
    );
    let at = |now: u64| StoredTokens::with_clock(Arc::new(store.clone()), Box::new(move || now));
    assert!(at(NOW_MS as u64).check("phone", &token));
    assert!(!at(NOW_MS as u64).check("tablet", &token), "another name");
    assert!(
        !at(NOW_MS as u64 + 3_600_000).check("phone", &token),
        "expired at its expiry"
    );
}

/// A token issued twice under one identity, as a restore starts the
/// invocation again, is one token and one row, its expiry the first issue's;
/// another start, or another name, is another token.
#[test]
fn a_token_issued_twice_under_one_identity_is_one_token() {
    let store = MemoryCredentials::new();
    let (mut reference, clock) = host(&store);
    let token = issue(&mut reference, &first(), "phone", "60");
    clock.lock().expect("the clock").reading = Some(Reading {
        utc_ms: NOW_MS + 30_000,
        monotonic_ms: 0,
    });
    assert_eq!(
        issue(&mut reference, &first(), "phone", "60"),
        token,
        "the same identity is the same token"
    );
    assert_eq!(store.tokens().expect("the tokens").len(), 1, "and one row");
    assert_eq!(
        store
            .token(&token_digest(&token))
            .expect("reading it")
            .expect("it is kept")
            .expires_ms,
        NOW_MS as u64 + 60_000,
        "the repeat keeps the first issue's expiry"
    );
    let next = issue(
        &mut reference,
        &Invocation::new("session", "issue", 2),
        "phone",
        "60",
    );
    assert_ne!(next, token, "the next start of the invoke is another token");
    let other = issue(&mut reference, &first(), "tablet", "60");
    assert_ne!(other, token, "another name under the same identity");
    assert_eq!(store.tokens().expect("the tokens").len(), 3);
}

/// The issuing key is drawn once and kept in the key ring, so every issue
/// after the first draws nothing.
#[test]
fn the_issuing_key_is_drawn_at_the_first_issue_and_kept() {
    let store = MemoryCredentials::new();
    let (mut reference, _clock) = host(&store);
    let token = issue(&mut reference, &first(), "phone", "60");
    reference.set_random_source(Box::new(Exhausted));
    assert_ne!(
        issue(
            &mut reference,
            &Invocation::new("session", "issue", 2),
            "phone",
            "60"
        ),
        token,
        "an issue after the first needs no random bytes"
    );
    let (mut dry, _clock) = host(&MemoryCredentials::new());
    dry.set_random_source(Box::new(Exhausted));
    assert_eq!(
        done(start(
            &mut dry,
            "sce-host-credential-issue",
            &first(),
            &[("name", "phone"), ("ttl_s", "60")]
        ))["error"],
        "unavailable",
        "a host whose key ring holds no issuing key draws one at its first issue"
    );
}

/// A revocation revokes the name's tokens and removes its pin, and answers how
/// many it took away; a token issued again under its first identity stays
/// revoked; another name is untouched.
#[test]
fn a_revocation_takes_the_names_tokens_and_its_pin() {
    let store = MemoryCredentials::new();
    let (mut reference, _clock) = host(&store);
    let token = issue(&mut reference, &first(), "phone", "60");
    let second = issue(
        &mut reference,
        &Invocation::new("session", "issue", 2),
        "phone",
        "60",
    );
    let tablet = issue(
        &mut reference,
        &Invocation::new("session", "issue", 3),
        "tablet",
        "60",
    );
    store.pin("phone", &[0xA1; 32]).expect("phone's pin");
    let revoke = Invocation::new("session", "revoke", 1);
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-revoke",
            &revoke,
            &[("name", "phone")]
        )),
        serde_json::json!({ "error": "", "revoked": 3 }),
        "two tokens and a pin"
    );
    let check = StoredTokens::with_clock(Arc::new(store.clone()), Box::new(|| NOW_MS as u64));
    assert!(!check.check("phone", &token) && !check.check("phone", &second));
    assert!(check.check("tablet", &tablet), "another name's token");
    assert_eq!(store.pinned("phone").expect("phone's pin"), None);
    assert_eq!(
        issue(&mut reference, &first(), "phone", "60"),
        token,
        "the first issue repeated answers its token"
    );
    assert!(
        !check.check("phone", &token),
        "which a repeat does not make live again"
    );
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-revoke",
            &Invocation::new("session", "revoke", 2),
            &[("name", "phone")]
        )),
        serde_json::json!({ "error": "", "revoked": 0 }),
        "nothing more to revoke"
    );
}

/// A name that is empty or past its bound, and a `ttl_s` that is no count of
/// seconds from 1 to the largest a uint32 holds, are refused, and nothing is
/// issued or revoked.
#[test]
fn a_request_out_of_its_bounds_is_refused() {
    let store = MemoryCredentials::new();
    let (mut reference, _clock) = host(&store);
    let long = "n".repeat(257);
    for pairs in [
        vec![("name", ""), ("ttl_s", "60")],
        vec![("name", long.as_str()), ("ttl_s", "60")],
        vec![("name", "phone"), ("ttl_s", "0")],
        vec![("name", "phone"), ("ttl_s", "4294967296")],
        vec![("name", "phone"), ("ttl_s", "-1")],
        vec![("name", "phone")],
    ] {
        assert!(
            refused(start(
                &mut reference,
                "sce-host-credential-issue",
                &first(),
                &pairs
            )),
            "{pairs:?}"
        );
    }
    for name in ["", long.as_str()] {
        assert!(
            refused(start(
                &mut reference,
                "sce-host-credential-revoke",
                &first(),
                &[("name", name)]
            )),
            "{name:?}"
        );
    }
    assert!(store.tokens().expect("the tokens").is_empty());
    let longest = "n".repeat(256);
    issue(&mut reference, &first(), &longest, "4294967295");
}

/// An issue the host cannot carry out fails `unavailable` and keeps nothing: a
/// clock that gives no time, a random source that gives no issuing key, and a
/// failure the embedding injected.
#[test]
fn an_issue_the_host_cannot_carry_out_fails_unavailable() {
    let store = MemoryCredentials::new();
    let (mut reference, clock) = host(&store);
    let failed = serde_json::json!({ "error": "unavailable", "token": "" });
    clock.lock().expect("the clock").reading = None;
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-issue",
            &first(),
            &[("name", "phone"), ("ttl_s", "60")]
        )),
        failed,
        "no time to issue from"
    );
    clock.lock().expect("the clock").reading = Some(Reading {
        utc_ms: NOW_MS,
        monotonic_ms: 0,
    });
    reference.set_random_source(Box::new(Exhausted));
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-issue",
            &first(),
            &[("name", "phone"), ("ttl_s", "60")]
        )),
        failed,
        "no issuing key to draw"
    );
    reference.set_faults(BTreeMap::from([(
        "sce-host-credential-revoke".to_string(),
        "unavailable".to_string(),
    )]));
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-revoke",
            &first(),
            &[("name", "phone")]
        )),
        serde_json::json!({ "error": "unavailable", "revoked": 0 })
    );
    assert!(store.tokens().expect("the tokens").is_empty());
}

/// An issue under an issuing key the operator retired fails `key_retired` and
/// keeps nothing more: a retired key makes nothing new (section 13).
#[test]
fn an_issue_under_a_retired_issuing_key_fails_key_retired() {
    let store = MemoryCredentials::new();
    let (mut reference, _clock) = host(&store);
    let keys = MemoryKeys::new();
    reference.set_keyring(Keyring::new(
        Box::new(keys.clone()),
        Box::new(KeyEncryptionKey::new([7; 32])),
    ));
    issue(&mut reference, &first(), "phone", "60");
    assert_eq!(
        keys.retire(ISSUING_KEY_ID)
            .expect("the operator retires the issuing key"),
        Some(true)
    );
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-issue",
            &Invocation::new("session", "issue", 2),
            &[("name", "phone"), ("ttl_s", "60")]
        )),
        serde_json::json!({ "error": "key_retired", "token": "" })
    );
    assert_eq!(store.tokens().expect("the tokens").len(), 1);
}

/// A credential store whose medium is gone.
struct Broken;

fn gone() -> Failure {
    Failure::new("the medium is gone")
}

impl PinStore for Broken {
    fn pinned(&self, _: &str) -> Result<Option<Vec<u8>>, Failure> {
        Err(gone())
    }

    fn pin(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Failure> {
        Err(gone())
    }

    fn unpin(&self, _: &str) -> Result<bool, Failure> {
        Err(gone())
    }

    fn pins(&self) -> Result<Vec<PinRow>, Failure> {
        Err(gone())
    }

    fn repin(&self, _: &str, _: &[u8], _: &[u8]) -> Result<bool, Failure> {
        Err(gone())
    }
}

impl TokenStore for Broken {
    fn token(&self, _: &Digest) -> Result<Option<TokenRow>, Failure> {
        Err(gone())
    }

    fn issue(&self, _: &TokenRow) -> Result<TokenRow, Failure> {
        Err(gone())
    }

    fn tokens(&self) -> Result<Vec<TokenRow>, Failure> {
        Err(gone())
    }

    fn reseal(&self, _: &Digest, _: &[u8], _: &[u8]) -> Result<bool, Failure> {
        Err(gone())
    }
}

impl CredentialStore for Broken {
    fn revoke(&self, _: &str) -> Result<Revoked, Failure> {
        Err(gone())
    }
}

/// A credential store that cannot keep or revoke fails both operations
/// `unavailable`.
#[test]
fn a_store_that_fails_fails_both_operations_unavailable() {
    let (mut reference, _clock) = host(&MemoryCredentials::new());
    reference.set_credentials(Arc::new(Broken));
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-issue",
            &first(),
            &[("name", "phone"), ("ttl_s", "60")]
        )),
        serde_json::json!({ "error": "unavailable", "token": "" })
    );
    assert_eq!(
        done(start(
            &mut reference,
            "sce-host-credential-revoke",
            &first(),
            &[("name", "phone")]
        )),
        serde_json::json!({ "error": "unavailable", "revoked": 0 })
    );
}
