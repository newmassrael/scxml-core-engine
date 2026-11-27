// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both stores of the crate run the membership conformance kit (ADR 0113, D4 to D6; ADR 0116, D1,
//! D6 and D7), function by
//! function, each on a store that holds nothing when it starts: the store in memory, and the
//! SQLite store's file, whose every handle is a connection of its own, as a second process's
//! would be.

/// A test per function of the kit, each on a store of its own the function `$made` makes.
macro_rules! kit {
    ($made:ident) => {
        kit!(
            $made,
            [
                a_join_offers_and_the_exchange_that_names_it_confirms,
                a_repeated_join_is_offered_the_same_incarnation,
                a_late_join_puts_no_live_member_out,
                what_the_store_does_not_hold_is_fenced,
                a_mark_only_rises_and_every_handle_reads_it,
                a_write_keeps_a_run_only_under_an_incarnation_the_store_holds,
                no_member_is_taken_past_the_bound,
                what_ran_out_is_swept_and_told,
                a_batch_judges_each_write_by_its_own_incarnations,
                a_confirmation_binds_its_person_to_its_incarnation_alone,
                a_delegation_ends_its_incarnation_at_until,
                a_revocation_ends_what_it_names_in_one_write,
                a_renewal_moves_until_alone_and_only_for_its_person,
                a_delegations_bytes_are_replaced_only_from_what_they_hold,
            ]
        );
    };
    ($made:ident, [$($case:ident),* $(,)?]) => {
        $(
            #[test]
            fn $case() {
                membership_conformance::$case(&$made(stringify!($case)));
            }
        )*
    };
}

mod memory_kit {
    use sce_rust_session_store::membership_conformance::{self, MembershipHarness};
    use sce_rust_session_store::MemoryStore;

    /// A store in memory; every handle is a clone of it.
    struct InMemory(MemoryStore);

    impl MembershipHarness for InMemory {
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
    use sce_rust_session_store::membership_conformance::{self, MembershipHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl MembershipHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(&format!("membership-{name}")))
    }

    kit!(on_sqlite);
}
