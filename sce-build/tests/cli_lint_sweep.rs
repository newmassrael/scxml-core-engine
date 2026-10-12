// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every machine this repository authors is lint-clean, and something can
// generate it.
//
// The design-time lints are off by default because the W3C corpus
// declares unreachable states on purpose — a fixture proving `initial`
// is respected has states no event can enter — so a lint that refused
// them would be wrong about conformance. A document this repository
// writes carries no such excuse, which is what `--lint` is for.
//
// ⚠ WHY THIS MOVED. `scripts/gates/example-codegen.sh` held this sweep
// over a hand-written pair of directories (`examples/*.scxml` and
// `integration_resources/*/*.scxml`), and it is `ci_only`, so the claim
// was judged in one workflow over 56 of the tracked statecharts.
// Measured 2026-09-22 over the derived population instead: `check`
// accepts 228 of them and the lints refused 12 — ten `<xi:include>`
// fragments, whose unreachable states are what being a fragment means;
// `tests/mesh/srcexpr_miss.scxml`, whose unreachable state worked around
// the generator emitting no transport for a srcexpr-only document; and
// the no_std probe, whose deliberate event-handling gaps were
// undeclared. The first is a property of the document (so `corpus`
// excludes fragments), and the other two were repaired rather than
// exempted.
//
// ⚠ THE ONE KIND OF DOCUMENT THAT CANNOT BE REPAIRED is a probe whose
// SUBJECT is what a lint flags: `a_delay_is_a_css2_time` writes a bare-number
// `delay` because proving that every backend raises the argument error at
// such a send takes a document that writes one (2026-10-12, when
// `scxml/send-delay-invalid` was added and this sweep refused it). That is the
// excuse the W3C corpus has for its illegal expressions, so it is named, in
// `PROBES`, with the exact codes it must be refused by and the reason. The
// entry is checked both ways: a probe refused by anything else, or by nothing,
// fails here, so an exemption cannot outlive the thing it excuses.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

struct Run {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Authored machines the lints refuse on purpose: the document, the exact
/// codes it must be refused by, and why that is its point.
const PROBES: &[(&str, &[&str], &str)] = &[(
    "integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml",
    &["scxml/send-delay-invalid"],
    "its subject is the bare-number delay: every backend must raise the argument \
     error at that send, and only a document that writes one can show it",
)];

/// The `code` of every record on a `--error-format=json` stderr, in order.
fn wire_codes(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| line.starts_with('{'))
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|record| {
            record
                .get("code")
                .and_then(|c| c.as_str())
                .map(str::to_string)
        })
        .collect()
}

fn check(document: &str, lint: bool) -> Run {
    let mut args = vec!["check", document, "--error-format=json"];
    if lint {
        args.push("--lint");
    }
    let out = Command::new(sce_codegen_bin())
        .args(&args)
        .current_dir(common::corpus::repo_root())
        .output()
        .expect("spawn sce-codegen");
    Run {
        exit: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// A machine this repository authors passes its own lints, and at least
/// one backend accepts it.
///
/// ⚠ THE SECOND HALF IS NOT IMPLIED BY THE FIRST. Without `--language`
/// the per-backend verdict rides the manifest and the exit stays 0
/// (SCE_ERROR_CONTRACT.md §10.3.1), so a document every backend rejects
/// would read as lint-clean. The flag is deliberately not passed: the
/// lints are a document-axis question, and naming a backend answers a
/// different one — `examples/smart_light` writes a `cpp:` guard only C++
/// lowers, so pinning Rust would turn "this document is lint-clean" into
/// "Rust can lower this document".
///
/// A document plain `check` refuses is out of scope here and reported by
/// whatever refuses it: a negative fixture exists to be refused, and the
/// lints have nothing to add to a document that never parsed. The floor
/// below is what keeps that filter from quietly emptying the sweep.
#[test]
fn every_authored_machine_is_lint_clean() {
    let documents = common::corpus::authored_machines();
    // 297 authored machines when this bound was set, of which `check`
    // accepted 228.
    assert!(
        documents.len() >= 250,
        "swept only {} authored machine(s)",
        documents.len()
    );

    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    let chunk = documents.len().div_ceil(threads).max(1);
    let (judged, mut violations, probed): (usize, Vec<String>, Vec<String>) =
        std::thread::scope(|scope| {
            let handles: Vec<_> = documents
                .chunks(chunk)
                .map(|slice| {
                    scope.spawn(move || {
                        let mut judged = 0usize;
                        let mut found = Vec::new();
                        let mut probed = Vec::new();
                        for document in slice {
                            if check(document, false).exit != Some(0) {
                                continue;
                            }
                            judged += 1;
                            let linted = check(document, true);
                            if linted.exit != Some(0) {
                                let codes = wire_codes(&linted.stderr);
                                match PROBES.iter().find(|(path, _, _)| *path == document) {
                                    Some((_, expected, _)) => {
                                        probed.push(document.clone());
                                        if !codes
                                            .iter()
                                            .map(String::as_str)
                                            .eq(expected.iter().copied())
                                        {
                                            found.push(format!(
                                                "{document}: a probe the lints must refuse by \
                                                 {expected:?}, refused by {codes:?}\n{}",
                                                linted.stderr.trim()
                                            ));
                                        }
                                    }
                                    None => found.push(format!(
                                        "{document}: --lint refuses it\n{}",
                                        linted.stderr.trim()
                                    )),
                                }
                            } else if !linted.stdout.contains("\"status\":\"ok\"") {
                                found.push(format!(
                                    "{document}: no backend accepts it\n{}",
                                    linted.stdout.trim()
                                ));
                            }
                        }
                        (judged, found, probed)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("sweep thread"))
                .fold(
                    (0usize, Vec::new(), Vec::new()),
                    |(n, mut acc, mut seen), (judged, found, probed)| {
                        acc.extend(found);
                        seen.extend(probed);
                        (n + judged, acc, seen)
                    },
                )
        });

    // An exemption is held to the thing it excuses: a probe the lints no longer
    // refuse (the fixture changed, or the lint stopped firing) is a stale entry.
    for (path, expected, reason) in PROBES {
        if !probed.iter().any(|seen| seen == path) {
            violations.push(format!(
                "{path}: listed as a probe ({reason}) but --lint no longer refuses it by \
                 {expected:?}; remove the entry or restore the probe"
            ));
        }
    }

    assert!(
        judged >= 200,
        "only {judged} of {} authored machine(s) reached the lints; a filter \
         that stops accepting documents empties this sweep without failing it",
        documents.len()
    );
    assert!(
        violations.is_empty(),
        "{} authored machine(s) are not lint-clean:\n{}",
        violations.len(),
        violations.join("\n")
    );
}
