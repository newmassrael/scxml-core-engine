// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both stores of the crate run the audit conformance kit (ADR 0109, D14), function by function,
//! each on a store that holds nothing when it starts: the store in memory a host keeps when it is
//! given no durable one, and the SQLite store's file, whose every handle is a connection of its
//! own, as a second process's would be.

/// A test per function of the kit, each on a store of its own the function `$made` makes.
macro_rules! kit {
    ($made:ident) => {
        kit!(
            $made,
            [
                rows_are_appended_one_past_the_head,
                a_checkpoint_is_kept_with_its_row_or_on_its_own,
                an_append_repeated_under_its_identity_answers_its_first_sequence,
                a_segment_is_deleted_whole_up_to_a_checkpoint,
                streams_are_kept_apart,
                handles_share_the_log,
                an_entry_and_a_signature_are_replaced_only_from_what_they_hold,
                a_removal_of_a_session_deletes_its_identities_and_no_row,
            ]
        );
    };
    ($made:ident, [$($case:ident),* $(,)?]) => {
        $(
            #[test]
            fn $case() {
                audit_conformance::$case(&$made(stringify!($case)));
            }
        )*
    };
}

mod memory_kit {
    use sce_rust_session_store::audit_conformance::{self, AuditHarness};
    use sce_rust_session_store::MemoryStore;

    /// A store in memory; every handle is a clone of it.
    struct InMemory(MemoryStore);

    impl AuditHarness for InMemory {
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
    use sce_rust_session_store::audit_conformance::{self, AuditHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl AuditHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(&format!("audit-{name}")))
    }

    kit!(on_sqlite);
}
