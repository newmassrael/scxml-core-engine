// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both stores of the crate run the journal conformance kit (ADR 0114), function by function, each
//! on a store that holds nothing when it starts: the store in memory, and the SQLite store's file,
//! whose every handle is a connection of its own, as a second process's would be. Each writes one
//! transaction at a time, so the kit's two transactions committed out of the order of their numbers
//! are not applicable to either, which each says; the kit's own tests hold it to a store they are.

/// A test per function of the kit, each on a store of its own the function `$made` makes.
macro_rules! kit {
    ($made:ident) => {
        kit!(
            $made,
            [
                an_entry_is_made_by_its_sessions_write_or_not_at_all,
                a_batch_appends_item_by_item,
                a_removed_session_keeps_its_journal,
                an_input_makes_one_entry_and_a_reader_misses_none,
                a_page_is_resumed_where_it_ended,
                a_moment_is_never_below_the_one_before_it,
                an_answer_as_of_a_position_does_not_change,
                an_observed_entry_is_kept_once_for_its_origin,
                a_session_and_an_observed_stream_share_no_name,
                a_write_stopped_partway_leaves_its_state_and_its_entries_or_neither,
                a_fact_is_kept_with_the_bytes_made_for_its_position,
                data_is_replaced_only_from_what_it_holds,
                handles_share_the_journal,
            ]
        );

        #[test]
        fn a_second_transaction_does_not_begin_while_one_is_open() {
            assert_eq!(
                journal_conformance::a_reader_waits_for_a_lower_number_still_open(&$made(
                    "out-of-order"
                )),
                Forced::NotApplicable
            );
        }
    };
    ($made:ident, [$($case:ident),* $(,)?]) => {
        $(
            #[test]
            fn $case() {
                journal_conformance::$case(&$made(stringify!($case)));
            }
        )*
    };
}

mod memory_kit {
    use sce_rust_session_store::journal_conformance::{self, Forced, JournalHarness};
    use sce_rust_session_store::MemoryStore;

    /// A store in memory; every handle is a clone of it.
    struct InMemory(MemoryStore);

    impl JournalHarness for InMemory {
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
    use rusqlite::Connection;
    use sce_rust_session_store::journal_conformance::{self, Fault, Forced, JournalHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl OnSqlite {
        /// Run `statements` on the file, as another program's connection does.
        fn beside(&self, statements: &str) {
            Connection::open(self.0.database())
                .expect("a connection of another program")
                .execute_batch(statements)
                .expect("its statements");
        }

        /// The fault that refuses what `trigger`, the rest of a trigger's head, names.
        fn refuses(&self, at: &'static str, trigger: &'static str) -> Fault<'_> {
            Fault {
                at,
                set: Box::new(move || {
                    self.beside(&format!(
                        "CREATE TRIGGER refused_by_the_test {trigger} \
                         BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;"
                    ));
                }),
                clear: Box::new(move || self.beside("DROP TRIGGER refused_by_the_test;")),
            }
        }
    }

    impl JournalHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }

        /// Every row a write that appends writes, in the order it writes them.
        fn faults(&self) -> Vec<Fault<'_>> {
            vec![
                self.refuses("the number of its version", "AFTER UPDATE ON numbers"),
                self.refuses("its state", "AFTER INSERT ON sessions"),
                self.refuses("its outbox's entry", "AFTER INSERT ON outbox_entries"),
                self.refuses(
                    "the position of its first fact",
                    "AFTER UPDATE ON journal_head",
                ),
                self.refuses(
                    "the entry of its first fact",
                    "AFTER INSERT ON journal_entries",
                ),
                self.refuses(
                    "the entry of its second fact",
                    "AFTER INSERT ON journal_entries WHEN EXISTS (SELECT 1 FROM journal_entries \
                     WHERE position = NEW.position - 1 AND recorded_ms = NEW.recorded_ms \
                     AND name = NEW.name AND data = X'6669727374')",
                ),
            ]
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(&format!("journal-{name}")))
    }

    kit!(on_sqlite);
}
