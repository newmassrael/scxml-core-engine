// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both stores of the crate run the conformance kit, function by function, each on a store that
//! holds nothing when it starts: the store in memory that other crates' tests are given, and the
//! SQLite store a server keeps its sessions in, whose every handle is a connection of its own, as
//! a second server process's would be.

use sce_rust_session_store::conformance::{self, Harness};
use sce_rust_session_store::outbox_conformance;
use sce_rust_session_store::MemoryStore;

/// A store in memory; every handle is a clone of it.
struct InMemory(MemoryStore);

impl Harness for InMemory {
    type Store = MemoryStore;

    fn handle(&self) -> MemoryStore {
        self.0.clone()
    }
}

fn in_memory() -> InMemory {
    InMemory(MemoryStore::new())
}

#[test]
fn in_memory_versions_are_compared_and_set() {
    conformance::versions_are_compared_and_set(&in_memory());
}

#[test]
fn in_memory_a_write_needs_the_current_fence() {
    conformance::a_write_needs_the_current_fence(&in_memory());
}

#[test]
fn in_memory_a_lease_runs_out_and_is_taken_over() {
    conformance::a_lease_runs_out_and_is_taken_over(&in_memory());
}

#[test]
fn in_memory_due_sessions_are_listed_earliest_first() {
    conformance::due_sessions_are_listed_earliest_first(&in_memory());
}

#[test]
fn in_memory_state_and_due_are_written_together() {
    conformance::state_and_due_are_written_together(&in_memory());
}

#[test]
fn in_memory_concurrent_writers_never_both_win() {
    conformance::concurrent_writers_never_both_win(&in_memory());
}

#[test]
fn in_memory_no_update_is_lost() {
    conformance::no_update_is_lost(&in_memory());
}

#[test]
fn in_memory_handles_share_one_store() {
    conformance::handles_share_one_store(&in_memory());
}

#[test]
fn in_memory_every_session_is_listed_by_its_entry_and_named_by_its_key() {
    conformance::every_session_is_listed_by_its_entry_and_named_by_its_key(&in_memory());
}

#[test]
fn in_memory_an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all() {
    outbox_conformance::an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all(&in_memory());
}

#[test]
fn in_memory_an_outbox_is_opened_once_and_numbers_on() {
    outbox_conformance::an_outbox_is_opened_once_and_numbers_on(&in_memory());
}

#[test]
fn in_memory_an_acknowledgement_deletes_what_it_covers_in_its_incarnation() {
    outbox_conformance::an_acknowledgement_deletes_what_it_covers_in_its_incarnation(&in_memory());
}

#[test]
fn in_memory_an_old_entry_expires_and_its_sender_is_noted() {
    outbox_conformance::an_old_entry_expires_and_its_sender_is_noted(&in_memory());
}

#[test]
fn in_memory_an_entry_and_a_note_are_replaced_only_from_what_they_hold() {
    outbox_conformance::an_entry_and_a_note_are_replaced_only_from_what_they_hold(&in_memory());
}

#[test]
fn in_memory_a_batch_is_decided_item_by_item() {
    conformance::a_batch_is_decided_item_by_item(&in_memory());
}

#[test]
fn in_memory_a_batch_sees_its_earlier_items_and_is_seen_whole_on_return() {
    conformance::a_batch_sees_its_earlier_items_and_is_seen_whole_on_return(&in_memory());
}

#[test]
fn in_memory_a_batch_appends_and_acknowledges_item_by_item() {
    outbox_conformance::a_batch_appends_and_acknowledges_item_by_item(&in_memory());
}

#[test]
fn in_memory_a_session_is_its_machine_and_its_key() {
    let store = MemoryStore::new();
    conformance::a_session_is_its_machine_and_its_key(
        &store.of_machine("rooms"),
        &store.of_machine("view"),
    );
}

#[cfg(feature = "sqlite")]
mod common;

#[cfg(feature = "sqlite")]
mod on_sqlite {
    use sce_rust_session_store::conformance::{self, Harness};
    use sce_rust_session_store::outbox_conformance;
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl Harness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(name))
    }

    #[test]
    fn versions_are_compared_and_set() {
        conformance::versions_are_compared_and_set(&on_sqlite("versions"));
    }

    #[test]
    fn a_write_needs_the_current_fence() {
        conformance::a_write_needs_the_current_fence(&on_sqlite("fence"));
    }

    #[test]
    fn a_lease_runs_out_and_is_taken_over() {
        conformance::a_lease_runs_out_and_is_taken_over(&on_sqlite("lease"));
    }

    #[test]
    fn due_sessions_are_listed_earliest_first() {
        conformance::due_sessions_are_listed_earliest_first(&on_sqlite("due"));
    }

    #[test]
    fn state_and_due_are_written_together() {
        conformance::state_and_due_are_written_together(&on_sqlite("together"));
    }

    #[test]
    fn concurrent_writers_never_both_win() {
        conformance::concurrent_writers_never_both_win(&on_sqlite("concurrent"));
    }

    #[test]
    fn no_update_is_lost() {
        conformance::no_update_is_lost(&on_sqlite("lost"));
    }

    #[test]
    fn handles_share_one_store() {
        conformance::handles_share_one_store(&on_sqlite("handles"));
    }

    #[test]
    fn every_session_is_listed_by_its_entry_and_named_by_its_key() {
        conformance::every_session_is_listed_by_its_entry_and_named_by_its_key(&on_sqlite(
            "entries",
        ));
    }

    #[test]
    fn an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all() {
        outbox_conformance::an_outbox_is_appended_to_by_its_sessions_write_or_not_at_all(
            &on_sqlite("outbox-write"),
        );
    }

    #[test]
    fn an_outbox_is_opened_once_and_numbers_on() {
        outbox_conformance::an_outbox_is_opened_once_and_numbers_on(&on_sqlite("outbox-open"));
    }

    #[test]
    fn an_acknowledgement_deletes_what_it_covers_in_its_incarnation() {
        outbox_conformance::an_acknowledgement_deletes_what_it_covers_in_its_incarnation(
            &on_sqlite("outbox-ack"),
        );
    }

    #[test]
    fn an_old_entry_expires_and_its_sender_is_noted() {
        outbox_conformance::an_old_entry_expires_and_its_sender_is_noted(&on_sqlite(
            "outbox-expiry",
        ));
    }

    #[test]
    fn an_entry_and_a_note_are_replaced_only_from_what_they_hold() {
        outbox_conformance::an_entry_and_a_note_are_replaced_only_from_what_they_hold(&on_sqlite(
            "outbox-reseal",
        ));
    }

    #[test]
    fn a_batch_is_decided_item_by_item() {
        conformance::a_batch_is_decided_item_by_item(&on_sqlite("batch-items"));
    }

    #[test]
    fn a_batch_sees_its_earlier_items_and_is_seen_whole_on_return() {
        conformance::a_batch_sees_its_earlier_items_and_is_seen_whole_on_return(&on_sqlite(
            "batch-whole",
        ));
    }

    #[test]
    fn a_batch_appends_and_acknowledges_item_by_item() {
        outbox_conformance::a_batch_appends_and_acknowledges_item_by_item(&on_sqlite(
            "batch-outbox",
        ));
    }

    /// Each machine through a connection of its own, as two processes serving one machine each
    /// would hold them.
    #[test]
    fn a_session_is_its_machine_and_its_key() {
        let store = on_sqlite("machines");
        conformance::a_session_is_its_machine_and_its_key(
            &store.handle().of_machine("rooms"),
            &store.handle().of_machine("view"),
        );
    }
}
