// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Shared support for the tests that compile a generated probe crate.
//!
//! Four of them build a throwaway Cargo project against the runtime crates
//! and run `cargo build` / `cargo test` inside it. That project has no
//! `Cargo.lock`, so cargo resolves its dependency graph from scratch — and
//! resolution reaches for the registry index even when every version it needs
//! is already in the local cache.
//!
//! The cost of that is not hypothetical. On 2026-08-19 a push carrying two
//! commits was refused after ten minutes of gates because
//! `index.crates.io` was briefly unreachable, and the same test passed in
//! 26 seconds with `CARGO_NET_OFFLINE=true` — the cache had the package all
//! along. Two further gates never ran, so nothing was claimed about them.
//!
//! So these runs try offline first and reach the network only when the cache
//! genuinely cannot answer.
//!
//! The probe-crate helpers are this file's own subject; `workflow` is a
//! sibling with a different one, published here because two suites parse the
//! same workflow structure and a second parser would be a second answer.

use std::process::{Command, Output};

pub mod gate_selectors;
pub mod ledger;
pub mod workflow;

/// What one probe run did, so a caller can assert on the route rather than
/// only on the result.
///
/// `dead_code` is allowed because this module is compiled into every test
/// binary that declares it, and each one reads the parts it needs: the four
/// probe-crate suites want `output` alone, while the suite that measures the
/// route reads the rest. Without this, adding a field to serve one of them
/// breaks the build of the other four.
#[allow(dead_code)]
pub struct ProbeRun {
    pub output: Output,
    /// The attempts made, in order: `"offline"` always first, `"online"` only
    /// when the offline attempt failed for want of the network.
    pub attempts: Vec<&'static str>,
    /// The arguments the FIRST attempt actually carried.
    ///
    /// Recorded separately from `attempts` on purpose: a label and the command
    /// it describes are two different facts, and a helper that names an
    /// attempt `"offline"` while running it online would satisfy every
    /// assertion about the label alone. This is what the flag can be checked
    /// against.
    pub first_attempt_args: Vec<String>,
}

impl ProbeRun {
    /// Whether the run had to reach the network.
    #[allow(dead_code)]
    pub fn went_online(&self) -> bool {
        self.attempts.contains(&"online")
    }
}

/// The arguments a command carries, as strings, for the record above.
fn args_of(cmd: &Command) -> Vec<String> {
    cmd.get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

/// Which attempts an offline outcome calls for.
///
/// Split out as a pure function because the fallback arm is otherwise
/// unreachable from a test: every command a test can cheaply run either
/// succeeds offline or fails for a reason that is not the network, so the
/// online retry — and the label it is recorded under — would never execute
/// under any assertion. Deciding here and executing below makes both arms
/// observable.
#[allow(dead_code)]
pub fn attempts_for(offline_failed_for_want_of_network: bool) -> Vec<&'static str> {
    if offline_failed_for_want_of_network {
        vec!["offline", "online"]
    } else {
        vec!["offline"]
    }
}

/// Whether a cargo failure is cargo saying "I would have to use the network".
///
/// Matched on cargo's own wording rather than on the exit code, because a
/// genuine compile error exits the same way and must NOT be retried — that
/// would double every real failure's wall-clock and hide which attempt
/// produced the diagnostic.
fn failed_for_want_of_network(output: &Output) -> bool {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.contains("--offline")
        || stderr.contains("offline mode")
        || stderr.contains("net.offline")
}

/// Run a cargo command against a probe crate, offline first.
///
/// `build` is called once per attempt and must produce the same command each
/// time; this adds `--offline` to the first one. A cache that can answer
/// means the run never touches the network, which is what keeps a push from
/// depending on a service being up.
///
/// `dead_code` for the same reason `ProbeRun` carries it: the suites that
/// declare this module for `workflow` alone compile this function and never
/// call it.
#[allow(dead_code)]
pub fn run_cargo_offline_first(build: impl Fn() -> Command) -> ProbeRun {
    let mut offline_cmd = build();
    offline_cmd.arg("--offline");
    let first_attempt_args = args_of(&offline_cmd);
    let offline = offline_cmd
        .output()
        .expect("cargo invocation (offline attempt)");
    let needs_network = !offline.status.success() && failed_for_want_of_network(&offline);
    let attempts = attempts_for(needs_network);
    if !needs_network {
        return ProbeRun {
            output: offline,
            attempts,
            first_attempt_args,
        };
    }

    // The cache could not answer — a dependency this tree has never fetched,
    // or a cold CI runner. Reaching the network here is the fallback, not the
    // default.
    let online = build().output().expect("cargo invocation (online retry)");
    ProbeRun {
        output: online,
        attempts,
        first_attempt_args,
    }
}
