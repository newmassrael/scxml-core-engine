// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A push must not depend on a service being reachable.
//!
//! Four suites here compile a generated probe crate by writing a throwaway
//! Cargo project and running cargo inside it. That project carries no
//! `Cargo.lock`, so cargo resolves from scratch and reaches for the registry
//! index — even when every version it needs is already in the local cache.
//!
//! That is not a hypothetical cost. On 2026-08-19 a push carrying two commits
//! was refused after ten minutes of gates because `index.crates.io` was
//! briefly unreachable inside `regen-reproduces`; the same test passed in 26
//! seconds under `CARGO_NET_OFFLINE=true`, from the cache it already had. Two
//! further gates never ran, so nothing was claimed about them either.
//!
//! `common::run_cargo_offline_first` is the repair, and these tests are what
//! hold it: the offline attempt comes FIRST, a real compile error is NOT
//! retried online (which would double every failure's wall-clock and hide
//! which attempt produced the diagnostic), and every call site goes through
//! the shared helper rather than spelling `Command::new("cargo")` again.

mod common;

use std::path::Path;

/// A cargo invocation that succeeds offline must never reach the network.
///
/// The probe is `cargo --version`: it needs no registry at all, so a run that
/// records an online attempt could only have made one because the helper asks
/// for it unconditionally.
#[test]
fn a_run_the_cache_can_answer_stays_offline() {
    let run = common::run_cargo_offline_first(|| {
        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("--version");
        cmd
    });

    assert!(
        run.output.status.success(),
        "`cargo --version` must succeed; the harness cannot measure anything otherwise"
    );
    assert_eq!(
        run.attempts,
        vec!["offline"],
        "a command that succeeded offline was retried online anyway. Every probe \
         crate build in this suite would then depend on a registry being \
         reachable, which is how a push was refused on 2026-08-19"
    );
    // The label and the command it describes are two facts. Asserting only on
    // the label would let a helper name an attempt "offline" while running it
    // against the network, which is the state this repair exists to end.
    assert!(
        run.first_attempt_args.iter().any(|a| a == "--offline"),
        "the first attempt was labelled offline but did not carry `--offline`; \
         it ran: {:?}",
        run.first_attempt_args
    );
}

/// A genuine failure is reported from the offline attempt, not retried.
///
/// Retrying a real compile error online would double its wall-clock and leave
/// the reader unsure which attempt produced the diagnostic they are reading.
#[test]
fn a_real_failure_is_not_retried_online() {
    let run = common::run_cargo_offline_first(|| {
        let mut cmd = std::process::Command::new("cargo");
        // A subcommand that does not exist: cargo fails, and not for want of
        // the network.
        cmd.arg("this-subcommand-does-not-exist");
        cmd
    });

    assert!(
        !run.output.status.success(),
        "the probe is supposed to fail; nothing below tests what it claims otherwise"
    );
    assert_eq!(
        run.attempts,
        vec!["offline"],
        "a failure that had nothing to do with the network was retried online. \
         The retry is for a cache that cannot answer, not for every failure"
    );
}

/// The fallback arm exists and is labelled for what it is.
///
/// No command a test can cheaply run both fails offline AND succeeds online,
/// so the retry arm is unreachable from the two probes above — which is why
/// the decision is a pure function rather than a branch buried in the run.
/// Without this, a helper that quietly dropped the retry, or recorded it as
/// having stayed offline, would pass everything.
#[test]
fn a_cache_that_cannot_answer_is_retried_online_and_says_so() {
    assert_eq!(
        common::attempts_for(true),
        vec!["offline", "online"],
        "an offline attempt that failed for want of the network must be retried \
         online, and the retry must be recorded — a run that reaches the network \
         while reporting it stayed offline makes every other assertion here vacuous"
    );
    assert_eq!(
        common::attempts_for(false),
        vec!["offline"],
        "an offline attempt that answered must not be followed by an online one"
    );
}

/// Every suite that invokes cargo routes it through the shared helper.
///
/// The four call sites were identical and independent, and the one that
/// refused the push was indistinguishable from the other three. A fifth
/// spelled by hand would be the same defect again — and it would not announce
/// itself, because a probe crate that reaches the network still passes on
/// every day the network is up.
///
/// The check is per FILE rather than per line: after the repair the command
/// is still built with `Command::new("cargo")`, only inside the closure the
/// helper drives. What distinguishes a routed call from a bare one is whether
/// the file names the helper at all.
///
/// The lower bound is the half that keeps this from passing vacuously — a
/// walk that found no files, or a rename that left the marker matching
/// nothing, would otherwise read as "all clear".
#[test]
fn every_suite_that_invokes_cargo_routes_it_through_the_helper() {
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut routed: Vec<String> = Vec::new();
    let mut offenders: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&tests_dir).expect("read tests/") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if name == "probe_crates_build_without_the_network.rs" {
            // This file drives the helper directly to measure its route; its
            // own invocations are the subject, not a call site.
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read test source");
        if !source.contains(r#"Command::new("cargo")"#) {
            continue;
        }
        if source.contains("run_cargo_offline_first") {
            routed.push(name);
        } else {
            offenders.push(name);
        }
    }

    assert!(
        offenders.is_empty(),
        "these suites invoke cargo without going through \
         `common::run_cargo_offline_first`, so each one makes a push depend on \
         a registry being reachable:\n  {}",
        offenders.join("\n  ")
    );
    assert!(
        routed.len() >= 4,
        "only {} suite(s) were found invoking cargo, and there were four when \
         this was written. A walk that finds nothing reports the same clean \
         result as a repository with no defect, so the count is asserted rather \
         than assumed. Found: {:?}",
        routed.len(),
        routed
    );
}
