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
//! * `a_durable_outbox_survives_a_killed_server` (docs/adr/0109, D10): a client whose binding is
//!   durable reads its outbox by polls, goes away for a while and loses responses, while the
//!   server is killed at points of the commit that appends to the outbox, of an acknowledgement
//!   and of an expiry; every reply is delivered at least once and taken in once.
//! * `a_stopped_server_leaves_only_clean_checkpoints` (docs/adr/0109, D10): a server stopped in
//!   order under load saves every session as a clean checkpoint, and a start after it answers.
//! * `group_commit_throughput` (docs/adr/0109, D15): ignored, a measurement: the inputs a pool
//!   takes in a second on the reference store, and the inputs a transaction holds, as the
//!   clients that send at once grow; its numbers are recorded in docs/SCE_SERVER_DEPLOY.md.
//! * `the_https_binding_reaches_a_pool`: a pool reached over the HTTPS binding through an explicit
//!   proxy, through a proxy that inspects TLS under an authority the client trusts, with a
//!   response lost on purpose, and with the store refusing a checkpoint, which no `200`
//!   acknowledges before a later one is durable.
//!
//! * `a_server_upgraded_in_place_under_load_loses_nothing` (docs/adr/0109, D11, and
//!   "Conformance"): a server of the first version of a generated document, stopped in order under
//!   load, and the next build started on its store: every session migrates or is refused as D11
//!   says, nothing accepted or owed is lost, a repeated request is answered with its first answer,
//!   and the notes the client signed and sealed under the first build open under the next; then a
//!   kill in the middle of the upgrade, and a rollback the old build refuses.
//! * `the_session_contract_holds` (docs/adr/0112): the ledger and the generated machines, served
//!   by the library's adapter, each held to the kit of the session machine's contract
//!   (`sce_rust_mesh::session_conformance`).
//! * `an_input_killed_at_every_write_makes_one_entry_or_none` (docs/adr/0114, D1): a ledger that
//!   says each tag it takes into the event journal, killed at points of load, checkpoint and
//!   delivery; each session's stream holds exactly the tags its saved state took, so a request
//!   made one entry with the state that took it, or none.
//! * `a_follower_of_many_sessions_killed_repeats_and_misses_nothing` (docs/adr/0114, D4): a view
//!   of many ledgers' sessions, killed again and again while it follows their journal; each view
//!   ends holding its key's tags in the journal's order, each once.
//!
//! This library is what they share:
//!
//! * [`ledger`]: the machine most scenarios' sessions run, written by hand in the shape a
//!   generated machine has to the pool, as the pool's own tests write theirs.
//! * [`journal`]: the view that follows the journaling ledger, and what reads both back.
//! * [`upgrade`] and [`machines`]: the document a server is upgraded between, its two versions
//!   generated (the tree `rust-server-suite`) and served by `sce_rust_mesh::served`.
//! * [`protocol`]: the lines a server process (this crate's binary, `sce-suite-server`) reads and
//!   writes, and [`process`], the parent's handle on one: start it, feed it, read it, kill it.
//! * [`campaign`]: the driver of a server killed again and again while its client keeps asking.
//! * [`wire`], [`clock`], [`draw`] and [`scratch`]: envelopes, time, seeded randomness and a
//!   directory for a store.

#![forbid(unsafe_code)]

pub mod campaign;
pub mod clock;
pub mod draw;
pub mod journal;
pub mod ledger;
pub mod machines;
pub mod process;
pub mod protocol;
pub mod scratch;
pub mod upgrade;
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
