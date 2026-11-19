// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What every implementation of [`KeyStore`] is held to (ADR 0109, D7).
//!
//! The contract is the documentation of [`KeyStore`]; these functions are that documentation as
//! histories with the answers a store must give. The two key stores of this crate run them
//! (`tests/every_key_store_keeps_one_contract.rs`), and a store written elsewhere runs the same
//! functions from its own `#[test]`s, as with [`crate::conformance`].
//!
//! * [`a_key_is_kept_once_under_its_id`]: a key is read back as it was kept, and a second key of
//!   the same id is not kept: the first is answered.
//! * [`a_generation_makes_one_key`]: a generation repeated under its identity answers the key it
//!   made the first time and keeps nothing more.
//! * [`a_retired_key_stays_retired`]: retiring answers whether the key was active, keeps every other
//!   column, and a key the store does not hold is answered as none.
//! * [`a_key_is_rewrapped_in_place`]: the rotation of the key-encryption key replaces the wrapped
//!   material and nothing else, of the keys and of the naming key.
//! * [`the_first_naming_key_is_kept`]: of two naming keys offered, the store keeps the first.
//! * [`handles_share_the_keys`]: what one handle kept, another reads.
//!
//! Each function needs a harness whose store holds no key when it starts.

use crate::keys::{KeyRow, KeyState, KeyStore, KeyUse};

/// What gives the functions of this module their store.
pub trait KeyHarness {
    type Store: KeyStore;

    /// Another handle on the one store this harness holds: it sees everything every other handle
    /// wrote. For a store in a database, a connection of its own, as a second process would have.
    fn handle(&self) -> Self::Store;
}

/// A row of `id` and `key_use`, wrapped as `wrapped`.
fn row(id: &str, key_use: KeyUse, wrapped: &[u8]) -> KeyRow {
    KeyRow {
        id: id.to_string(),
        key_use,
        algorithm: format!("algorithm of {}", key_use.name()),
        wrapped: wrapped.to_vec(),
        label: format!("label of {id}"),
        created_ms: 1_000,
        state: KeyState::Active,
    }
}

/// A key is read back as it was kept, and a second key of the same id is not kept.
pub fn a_key_is_kept_once_under_its_id(harness: &impl KeyHarness) {
    let store = harness.handle();
    assert_eq!(store.key("a").expect("reading a key never kept"), None);
    for key_use in KeyUse::ALL {
        let kept = row(&format!("k-{}", key_use.name()), key_use, &[1, 2, 3]);
        assert_eq!(store.insert(&kept, None).expect("keeping a key"), kept);
        assert_eq!(
            store.key(&kept.id).expect("reading it back"),
            Some(kept.clone()),
            "every column of a key of use {} is read back as kept",
            key_use.name()
        );
    }
    let first = row("a", KeyUse::Seal, &[9; 40]);
    store.insert(&first, None).expect("the first key of `a`");
    let second = row("a", KeyUse::Mac, &[7; 40]);
    assert_eq!(
        store.insert(&second, None).expect("a second key of `a`"),
        first,
        "a second key of one id answers the first and replaces nothing"
    );
    assert_eq!(store.key("a").expect("reading `a`"), Some(first));
    let mut ids = vec!["a".to_string()];
    ids.extend(
        KeyUse::ALL
            .into_iter()
            .map(|key_use| format!("k-{}", key_use.name())),
    );
    ids.sort();
    assert_eq!(store.ids().expect("the ids"), ids);
}

/// A generation repeated under its identity answers the key it made the first time.
pub fn a_generation_makes_one_key(harness: &impl KeyHarness) {
    let store = harness.handle();
    assert_eq!(
        store
            .generated_by("gen-1")
            .expect("a generation never made"),
        None
    );
    let made = row("generated:1", KeyUse::Sign, &[1; 48]);
    assert_eq!(
        store.insert(&made, Some("gen-1")).expect("the generation"),
        made
    );
    assert_eq!(
        store.generated_by("gen-1").expect("the generation's key"),
        Some("generated:1".to_string())
    );
    let again = row("generated:2", KeyUse::Sign, &[2; 48]);
    assert_eq!(
        store
            .insert(&again, Some("gen-1"))
            .expect("the generation repeated"),
        made,
        "a repeated generation answers its first key"
    );
    assert_eq!(
        store.key("generated:2").expect("reading the second id"),
        None,
        "a repeated generation keeps nothing"
    );
    let other = row("generated:3", KeyUse::Mac, &[3; 48]);
    assert_eq!(
        store
            .insert(&other, Some("gen-2"))
            .expect("another generation"),
        other
    );
    assert_eq!(
        store.ids().expect("the ids"),
        ["generated:1".to_string(), "generated:3".to_string()]
    );
}

/// Retiring answers whether the key was active and keeps every other column.
pub fn a_retired_key_stays_retired(harness: &impl KeyHarness) {
    let store = harness.handle();
    assert_eq!(
        store.retire("nobody").expect("retiring a key never kept"),
        None
    );
    let kept = row("k", KeyUse::Mac, &[5; 48]);
    store.insert(&kept, None).expect("keeping a key");
    assert_eq!(store.retire("k").expect("retiring it"), Some(true));
    let retired = KeyRow {
        state: KeyState::Retired,
        ..kept
    };
    assert_eq!(store.key("k").expect("reading it"), Some(retired.clone()));
    assert_eq!(
        store.retire("k").expect("retiring it again"),
        Some(false),
        "a key retired already answers false"
    );
    assert_eq!(store.key("k").expect("reading it again"), Some(retired));
}

/// The rotation of the key-encryption key replaces the wrapped material and nothing else.
pub fn a_key_is_rewrapped_in_place(harness: &impl KeyHarness) {
    let store = harness.handle();
    assert!(
        !store
            .rewrap("nobody", &[1])
            .expect("rewrapping a key never kept"),
        "a key the store does not hold is not rewrapped"
    );
    assert!(
        !store
            .rewrap_naming_key(&[1])
            .expect("rewrapping no naming key"),
        "no naming key is rewrapped before one is kept"
    );
    let kept = row("k", KeyUse::Derived, &[5; 48]);
    store.insert(&kept, None).expect("keeping a key");
    store.retire("k").expect("retiring it");
    assert!(store.rewrap("k", &[6; 48]).expect("rewrapping it"));
    assert_eq!(
        store.key("k").expect("reading it"),
        Some(KeyRow {
            wrapped: vec![6; 48],
            state: KeyState::Retired,
            ..kept
        }),
        "only the wrapped material changed"
    );
    store.keep_naming_key(&[7; 48]).expect("a naming key");
    assert!(store
        .rewrap_naming_key(&[8; 48])
        .expect("rewrapping the naming key"));
    assert_eq!(
        store.naming_key().expect("the naming key"),
        Some(vec![8; 48])
    );
}

/// Of two naming keys offered, the store keeps the first.
pub fn the_first_naming_key_is_kept(harness: &impl KeyHarness) {
    let store = harness.handle();
    assert_eq!(store.naming_key().expect("no naming key yet"), None);
    assert_eq!(
        store
            .keep_naming_key(&[1; 48])
            .expect("the first naming key"),
        vec![1; 48]
    );
    assert_eq!(
        harness
            .handle()
            .keep_naming_key(&[2; 48])
            .expect("a second, from another handle"),
        vec![1; 48],
        "the naming key a store holds is the one every handle names keys under"
    );
    assert_eq!(
        store.naming_key().expect("the naming key"),
        Some(vec![1; 48])
    );
}

/// What one handle kept, another reads, its generations and its retirements included.
pub fn handles_share_the_keys(harness: &impl KeyHarness) {
    let writer = harness.handle();
    let reader = harness.handle();
    let kept = row("generated:shared", KeyUse::Seal, &[4; 48]);
    writer
        .insert(&kept, Some("gen-shared"))
        .expect("keeping a key");
    assert_eq!(
        reader.key("generated:shared").expect("reading it"),
        Some(kept)
    );
    assert_eq!(
        reader
            .generated_by("gen-shared")
            .expect("its generation from another handle"),
        Some("generated:shared".to_string())
    );
    writer.retire("generated:shared").expect("retiring it");
    assert_eq!(
        reader
            .key("generated:shared")
            .expect("reading it again")
            .map(|row| row.state),
        Some(KeyState::Retired)
    );
}
