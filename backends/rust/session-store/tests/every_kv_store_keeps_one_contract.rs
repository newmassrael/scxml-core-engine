// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both stores of the crate run the key-value conformance kit (ADR 0109, D13), function by
//! function, each on a store that holds nothing when it starts: the store in memory a host keeps
//! when it is given no durable one, and the SQLite store's file, whose every handle is a
//! connection of its own, as a second process's would be.

/// A test per function of the kit, each on a store of its own the function `$made` makes.
macro_rules! kit {
    ($made:ident) => {
        kit!(
            $made,
            [
                a_value_is_read_at_the_version_its_write_gave_it,
                a_put_if_writes_only_over_the_version_it_names,
                the_quota_is_judged_only_for_a_write_that_would_be_made,
                a_put_if_repeated_under_its_identity_answers_its_first_result,
                the_keys_are_walked_in_the_order_of_their_bytes,
                the_values_are_listed_a_page_at_a_time,
                a_value_is_replaced_only_from_the_bytes_it_holds,
                handles_share_the_values,
                a_removal_of_a_session_deletes_its_identities_first_results,
            ]
        );
    };
    ($made:ident, [$($case:ident),* $(,)?]) => {
        $(
            #[test]
            fn $case() {
                kv_conformance::$case(&$made(stringify!($case)));
            }
        )*
    };
}

mod memory_kit {
    use sce_rust_session_store::kv_conformance::{self, KvHarness};
    use sce_rust_session_store::MemoryStore;

    /// A store in memory; every handle is a clone of it.
    struct InMemory(MemoryStore);

    impl KvHarness for InMemory {
        type Store = MemoryStore;

        fn handle(&self) -> MemoryStore {
            self.0.clone()
        }
    }

    fn in_memory(_: &str) -> InMemory {
        InMemory(MemoryStore::new())
    }

    kit!(in_memory);
}

#[cfg(feature = "sqlite")]
mod common;

#[cfg(feature = "sqlite")]
mod sqlite_kit {
    use sce_rust_session_store::kv_conformance::{self, KvHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl KvHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(&format!("kv-{name}")))
    }

    kit!(on_sqlite);
}
