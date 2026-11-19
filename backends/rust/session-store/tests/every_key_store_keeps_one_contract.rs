// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both key stores of the crate run the key conformance kit (ADR 0109, D7), function by function,
//! each on a store that holds no key when it starts: the keys in memory a host keeps when it is
//! given no durable store, and the SQLite store's file, whose every handle is a connection of its
//! own, as a second server process's would be.

use sce_rust_session_store::key_conformance::{self, KeyHarness};
use sce_rust_session_store::MemoryKeys;

/// Keys in memory; every handle is a clone of them.
struct InMemory(MemoryKeys);

impl KeyHarness for InMemory {
    type Store = MemoryKeys;

    fn handle(&self) -> MemoryKeys {
        self.0.clone()
    }
}

fn in_memory() -> InMemory {
    InMemory(MemoryKeys::new())
}

#[test]
fn in_memory_a_key_is_kept_once_under_its_id() {
    key_conformance::a_key_is_kept_once_under_its_id(&in_memory());
}

#[test]
fn in_memory_a_generation_makes_one_key() {
    key_conformance::a_generation_makes_one_key(&in_memory());
}

#[test]
fn in_memory_a_retired_key_stays_retired() {
    key_conformance::a_retired_key_stays_retired(&in_memory());
}

#[test]
fn in_memory_a_key_is_rewrapped_in_place() {
    key_conformance::a_key_is_rewrapped_in_place(&in_memory());
}

#[test]
fn in_memory_the_first_naming_key_is_kept() {
    key_conformance::the_first_naming_key_is_kept(&in_memory());
}

#[test]
fn in_memory_handles_share_the_keys() {
    key_conformance::handles_share_the_keys(&in_memory());
}

#[cfg(feature = "sqlite")]
mod common;

#[cfg(feature = "sqlite")]
mod on_sqlite {
    use sce_rust_session_store::key_conformance::{self, KeyHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl KeyHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(name))
    }

    #[test]
    fn a_key_is_kept_once_under_its_id() {
        key_conformance::a_key_is_kept_once_under_its_id(&on_sqlite("keys-once"));
    }

    #[test]
    fn a_generation_makes_one_key() {
        key_conformance::a_generation_makes_one_key(&on_sqlite("keys-generation"));
    }

    #[test]
    fn a_retired_key_stays_retired() {
        key_conformance::a_retired_key_stays_retired(&on_sqlite("keys-retired"));
    }

    #[test]
    fn a_key_is_rewrapped_in_place() {
        key_conformance::a_key_is_rewrapped_in_place(&on_sqlite("keys-rewrap"));
    }

    #[test]
    fn the_first_naming_key_is_kept() {
        key_conformance::the_first_naming_key_is_kept(&on_sqlite("keys-naming"));
    }

    #[test]
    fn handles_share_the_keys() {
        key_conformance::handles_share_the_keys(&on_sqlite("keys-handles"));
    }
}
