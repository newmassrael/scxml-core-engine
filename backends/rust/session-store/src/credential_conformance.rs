// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`CredentialStore`] is held to (ADR 0109, D3).
//!
//! The contract is the documentation of [`PinStore`], [`TokenStore`] and [`CredentialStore`];
//! these functions are that documentation as histories with the answers a store must give. The two
//! credential stores of this crate run them (`tests/every_credential_store_keeps_one_contract.rs`),
//! and a store written elsewhere runs the same functions from its own `#[test]`s, as with
//! [`crate::conformance`]. That a durable store keeps them across a restart is the SQLite store's
//! own test (`tests/a_sqlite_session_store_survives_its_process.rs`), since a store in memory keeps
//! nothing past its process by what it is.
//!
//! * [`a_name_is_pinned_to_its_first_secret`]: the first digest a name brings is its pin, and a
//!   second is answered with the first and kept nowhere.
//! * [`an_unpinned_name_is_pinned_again`]: a removed pin takes the next digest the name brings.
//! * [`a_token_is_kept_once_under_its_digest`]: a token is read back as it was issued, and an issue
//!   of a digest held already answers the row held.
//! * [`a_token_expires_at_its_moment`]: the moments are kept as given, and a token proves its name
//!   before its expiry and not at it.
//! * [`a_revoked_name_keeps_no_live_token_and_no_pin`]: revocation revokes the name's tokens, removes
//!   its pin, touches no other name, and is not undone by a repeated issue.
//! * [`handles_share_the_credentials`]: what one handle kept, another reads.
//!
//! Each function needs a harness whose store holds no credential when it starts.

use crate::credentials::{CredentialStore, Digest, PinStore, Revoked, TokenRow, TokenStore};
use crate::LATEST_MOMENT;

/// What gives the functions of this module their store.
pub trait CredentialHarness {
    /// A credential store, named with the two contracts it is made of, whose methods the functions
    /// of this module call.
    type Store: CredentialStore + PinStore + TokenStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// A digest whose every byte is `byte`.
fn digest(byte: u8) -> Digest {
    [byte; 32]
}

/// A token of `digest` for `name`, issued at 1000 and expiring at 2000.
fn token(digest: Digest, name: &str) -> TokenRow {
    TokenRow {
        digest,
        name: name.to_string(),
        issued_ms: 1_000,
        expires_ms: 2_000,
        revoked: false,
    }
}

/// The first digest a name brings is its pin; a second is answered with the first.
pub fn a_name_is_pinned_to_its_first_secret(harness: &impl CredentialHarness) {
    let store = harness.handle();
    assert_eq!(store.pinned("phone").expect("reading no pin"), None);
    assert_eq!(
        store.pin("phone", &digest(1)).expect("the first pin"),
        digest(1),
        "the first digest a name brings is its pin"
    );
    assert_eq!(
        store
            .pin("phone", &digest(1))
            .expect("the same secret again"),
        digest(1)
    );
    assert_eq!(
        store.pin("phone", &digest(2)).expect("a second secret"),
        digest(1),
        "a second secret is answered with the pin it does not match"
    );
    assert_eq!(
        store.pinned("phone").expect("the pin"),
        Some(digest(1)),
        "a second secret replaced nothing"
    );
    assert_eq!(
        store.pin("tablet", &digest(2)).expect("another name"),
        digest(2),
        "another name pins its own"
    );
}

/// A removed pin takes the next digest the name brings.
pub fn an_unpinned_name_is_pinned_again(harness: &impl CredentialHarness) {
    let store = harness.handle();
    assert!(
        !store.unpin("phone").expect("unpinning a name never pinned"),
        "a name never pinned is not unpinned"
    );
    store.pin("phone", &digest(1)).expect("the first pin");
    store.pin("tablet", &digest(3)).expect("another name's pin");
    assert!(store.unpin("phone").expect("unpinning it"));
    assert_eq!(store.pinned("phone").expect("no pin"), None);
    assert!(
        !store.unpin("phone").expect("unpinning it again"),
        "a pin removed already is not removed again"
    );
    assert_eq!(
        store.pin("phone", &digest(2)).expect("the next secret"),
        digest(2),
        "the next secret after an unpin is the pin"
    );
    assert_eq!(
        store.pinned("tablet").expect("the other pin"),
        Some(digest(3)),
        "an unpin removed no other name's pin"
    );
}

/// A token is read back as it was issued, and an issue of a digest held answers the row held.
pub fn a_token_is_kept_once_under_its_digest(harness: &impl CredentialHarness) {
    let store = harness.handle();
    assert_eq!(store.token(&digest(9)).expect("reading no token"), None);
    assert!(store.tokens().expect("no tokens").is_empty());
    let first = token(digest(9), "phone");
    assert_eq!(store.issue(&first).expect("the first issue"), first);
    assert_eq!(
        store.token(&digest(9)).expect("reading it back"),
        Some(first.clone())
    );
    let again = TokenRow {
        name: "tablet".to_string(),
        issued_ms: 5_000,
        expires_ms: 9_000,
        ..first.clone()
    };
    assert_eq!(
        store.issue(&again).expect("an issue of the same digest"),
        first,
        "a token is never replaced: an issue of its digest answers the row kept first"
    );
    let other = token(digest(4), "tablet");
    store.issue(&other).expect("another token");
    assert_eq!(
        store.tokens().expect("every token"),
        vec![other, first],
        "every token, in the order of their digests"
    );
}

/// The moments are kept as given, and a token proves its name before its expiry and not at it.
pub fn a_token_expires_at_its_moment(harness: &impl CredentialHarness) {
    let store = harness.handle();
    let issued = store
        .issue(&token(digest(1), "phone"))
        .expect("a token expiring at 2000");
    assert_eq!((issued.issued_ms, issued.expires_ms), (1_000, 2_000));
    let held = store
        .token(&digest(1))
        .expect("reading it")
        .expect("the token is held");
    assert!(held.live_at(1_000), "live from its issue");
    assert!(held.live_at(1_999), "live until the moment it expires");
    assert!(!held.live_at(2_000), "not live at its expiry");
    assert!(!held.live_at(u64::MAX), "nor after it");
    let lasting = TokenRow {
        expires_ms: u64::MAX,
        ..token(digest(2), "tablet")
    };
    assert_eq!(
        store
            .issue(&lasting)
            .expect("a token that expires past what a store holds")
            .expires_ms,
        LATEST_MOMENT,
        "an expiry later than a store holds is kept as the latest moment it holds"
    );
}

/// Revocation revokes the name's tokens, removes its pin, touches no other name, and is not undone
/// by a repeated issue.
pub fn a_revoked_name_keeps_no_live_token_and_no_pin(harness: &impl CredentialHarness) {
    let store = harness.handle();
    assert_eq!(
        store.revoke("phone").expect("revoking a name with nothing"),
        Revoked::default()
    );
    let phone = [token(digest(1), "phone"), token(digest(2), "phone")];
    for row in &phone {
        store.issue(row).expect("a token of phone");
    }
    store.pin("phone", &digest(7)).expect("phone's pin");
    let tablet = token(digest(3), "tablet");
    store.issue(&tablet).expect("a token of tablet");
    store.pin("tablet", &digest(8)).expect("tablet's pin");

    let revoked = store.revoke("phone").expect("revoking phone");
    assert_eq!(
        revoked,
        Revoked {
            tokens: 2,
            pin: true
        }
    );
    assert_eq!(revoked.count(), 3, "two tokens and a pin");
    for row in &phone {
        let held = store
            .token(&row.digest)
            .expect("reading a revoked token")
            .expect("a revoked token is kept");
        assert!(held.revoked && !held.live_at(1_500), "{held:?}");
    }
    assert_eq!(store.pinned("phone").expect("phone's pin"), None);
    assert_eq!(
        store.token(&digest(3)).expect("tablet's token"),
        Some(tablet),
        "another name's token is untouched"
    );
    assert_eq!(
        store.pinned("tablet").expect("tablet's pin"),
        Some(digest(8)),
        "another name's pin is untouched"
    );
    assert!(
        store
            .issue(&phone[0])
            .expect("the revoked token issued again")
            .revoked,
        "a repeated issue does not make a revoked token live"
    );
    assert_eq!(
        store.revoke("phone").expect("revoking phone again"),
        Revoked::default(),
        "a name revoked already has nothing more to revoke"
    );
}

/// What one handle kept, another reads, its revocations included.
pub fn handles_share_the_credentials(harness: &impl CredentialHarness) {
    let writer = harness.handle();
    let reader = harness.handle();
    writer.pin("phone", &digest(1)).expect("a pin");
    writer.issue(&token(digest(2), "phone")).expect("a token");
    assert_eq!(
        reader.pinned("phone").expect("the pin from another handle"),
        Some(digest(1))
    );
    assert_eq!(
        reader
            .pin("phone", &digest(3))
            .expect("a second secret from another handle"),
        digest(1),
        "the pin one handle kept refuses a second secret through another"
    );
    writer.revoke("phone").expect("revoking phone");
    assert!(
        reader
            .token(&digest(2))
            .expect("the token from another handle")
            .is_some_and(|row| row.revoked),
        "a revocation through one handle is read through another"
    );
    assert_eq!(reader.pinned("phone").expect("no pin"), None);
}
