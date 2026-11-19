// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Both credential stores of the crate run the credential conformance kit (ADR 0109, D3), function
//! by function, each on a store that holds no credential when it starts: the credentials in memory
//! a server keeps when it is given no durable store, and the SQLite store's file, whose every handle
//! is a connection of its own, as a second server process's would be.

use sce_rust_session_store::credential_conformance::{self, CredentialHarness};
use sce_rust_session_store::MemoryCredentials;

/// Credentials in memory; every handle is a clone of them.
struct InMemory(MemoryCredentials);

impl CredentialHarness for InMemory {
    type Store = MemoryCredentials;

    fn handle(&self) -> MemoryCredentials {
        self.0.clone()
    }
}

fn in_memory() -> InMemory {
    InMemory(MemoryCredentials::new())
}

#[test]
fn in_memory_a_name_is_pinned_to_its_first_secret() {
    credential_conformance::a_name_is_pinned_to_its_first_secret(&in_memory());
}

#[test]
fn in_memory_an_unpinned_name_is_pinned_again() {
    credential_conformance::an_unpinned_name_is_pinned_again(&in_memory());
}

#[test]
fn in_memory_a_token_is_kept_once_under_its_digest() {
    credential_conformance::a_token_is_kept_once_under_its_digest(&in_memory());
}

#[test]
fn in_memory_a_token_expires_at_its_moment() {
    credential_conformance::a_token_expires_at_its_moment(&in_memory());
}

#[test]
fn in_memory_a_revoked_name_keeps_no_live_token_and_no_pin() {
    credential_conformance::a_revoked_name_keeps_no_live_token_and_no_pin(&in_memory());
}

#[test]
fn in_memory_handles_share_the_credentials() {
    credential_conformance::handles_share_the_credentials(&in_memory());
}

#[test]
fn in_memory_a_pin_is_replaced_only_from_the_bytes_it_holds() {
    credential_conformance::a_pin_is_replaced_only_from_the_bytes_it_holds(&in_memory());
}

#[test]
fn in_memory_a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds() {
    credential_conformance::a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds(&in_memory());
}

#[cfg(feature = "sqlite")]
mod common;

#[cfg(feature = "sqlite")]
mod on_sqlite {
    use sce_rust_session_store::credential_conformance::{self, CredentialHarness};
    use sce_rust_session_store::SqliteStore;

    use crate::common::Scratch;

    /// A database file of its own; every handle opens a connection to it.
    struct OnSqlite(Scratch);

    impl CredentialHarness for OnSqlite {
        type Store = SqliteStore;

        fn handle(&self) -> SqliteStore {
            SqliteStore::open(self.0.database()).expect("opening the store")
        }
    }

    fn on_sqlite(name: &str) -> OnSqlite {
        OnSqlite(Scratch::new(name))
    }

    #[test]
    fn a_name_is_pinned_to_its_first_secret() {
        credential_conformance::a_name_is_pinned_to_its_first_secret(&on_sqlite("pins-first"));
    }

    #[test]
    fn an_unpinned_name_is_pinned_again() {
        credential_conformance::an_unpinned_name_is_pinned_again(&on_sqlite("pins-unpin"));
    }

    #[test]
    fn a_token_is_kept_once_under_its_digest() {
        credential_conformance::a_token_is_kept_once_under_its_digest(&on_sqlite("tokens-once"));
    }

    #[test]
    fn a_token_expires_at_its_moment() {
        credential_conformance::a_token_expires_at_its_moment(&on_sqlite("tokens-expiry"));
    }

    #[test]
    fn a_revoked_name_keeps_no_live_token_and_no_pin() {
        credential_conformance::a_revoked_name_keeps_no_live_token_and_no_pin(&on_sqlite(
            "credentials-revoke",
        ));
    }

    #[test]
    fn handles_share_the_credentials() {
        credential_conformance::handles_share_the_credentials(&on_sqlite("credentials-handles"));
    }

    #[test]
    fn a_pin_is_replaced_only_from_the_bytes_it_holds() {
        credential_conformance::a_pin_is_replaced_only_from_the_bytes_it_holds(&on_sqlite(
            "pins-replaced",
        ));
    }

    #[test]
    fn a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds() {
        credential_conformance::a_tokens_sealed_bytes_are_replaced_only_from_what_it_holds(
            &on_sqlite("tokens-resealed"),
        );
    }
}
