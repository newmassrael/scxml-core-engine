// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The suite that holds a server's durability and scale (docs/adr/0107, decision D10), run
//! against the Rust host core's keyed session pool (`sce_rust_mesh::pool`) on the reference
//! session store (`sce_rust_session_store::SqliteStore`).
//!
//! Each scenario is a test target of its own under `tests/`, so that one can be run alone:
//!
//! * `delayed_sends_survive_a_killed_server`: many keyed sessions, each with delayed sends at
//!   spread due times, served by a process killed with SIGKILL at points of load, checkpoint and
//!   delivery; every send is delivered once, at or after its due.
//! * `an_answered_request_survives_a_killed_server`: every request a client saw answered has its
//!   effect in the restored state, and a request sent again is answered with the first reply.
//! * `concurrent_clients_never_pass_a_limit`: clients that race on one key, and sessions that race
//!   on one value under `sce-host-kv-put-if`, never pass the limit the document states.
//! * `two_servers_never_both_write_a_session`: two server processes on one store, one paused past
//!   its lease, never both write a session.
//! * `the_https_binding_reaches_a_pool`: a pool reached over the HTTPS binding through an explicit
//!   proxy, through a proxy that inspects TLS under an authority the client trusts, with a
//!   response lost on purpose, and with the store refusing a checkpoint, which no `200`
//!   acknowledges before a later one is durable.
//!
//! This library is what they share:
//!
//! * [`ledger`]: the machine every session runs, written by hand in the shape a generated machine
//!   has to the pool, as the pool's own tests write theirs.
//! * [`protocol`]: the lines a server process (this crate's binary, `sce-suite-server`) reads and
//!   writes, and [`process`], the parent's handle on one: start it, feed it, read it, kill it.
//! * [`campaign`]: the driver of a server killed again and again while its client keeps asking.
//! * [`wire`], [`clock`], [`draw`] and [`scratch`]: envelopes, time, seeded randomness and a
//!   directory for a store.

#![forbid(unsafe_code)]

pub mod campaign;
pub mod clock;
pub mod draw;
pub mod ledger;
pub mod process;
pub mod protocol;
pub mod scratch;
pub mod wire;

/// The environment variable a scenario reads its count of sessions from, when it is set: the
/// ADR's ten thousand is `SCE_SERVER_SUITE_SESSIONS=10000`.
pub const SESSIONS_VARIABLE: &str = "SCE_SERVER_SUITE_SESSIONS";

/// The environment variable a killing scenario reads how many times it kills its server from.
pub const KILLS_VARIABLE: &str = "SCE_SERVER_SUITE_KILLS";

/// The number the environment variable `variable` holds, or `default` when it holds none. A
/// value that is not a number is refused by name rather than read as the default.
pub fn count_from(variable: &str, default: u64) -> u64 {
    match std::env::var(variable) {
        Ok(text) => text
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("{variable}={text} is not a count")),
        Err(_) => default,
    }
}
