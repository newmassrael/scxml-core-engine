// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
    let (judged, violations): (usize, Vec<String>) = std::thread::scope(|scope| {
        let handles: Vec<_> = documents
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    let mut judged = 0usize;
                    let mut found = Vec::new();
                    for document in slice {
                        if check(document, false).exit != Some(0) {
                            continue;
                        }
                        judged += 1;
                        let linted = check(document, true);
                        if linted.exit != Some(0) {
                            found.push(format!(
                                "{document}: --lint refuses it\n{}",
                                linted.stderr.trim()
                            ));
                        } else if !linted.stdout.contains("\"status\":\"ok\"") {
                            found.push(format!(
                                "{document}: no backend accepts it\n{}",
                                linted.stdout.trim()
                            ));
                        }
                    }
                    (judged, found)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("sweep thread"))
            .fold((0usize, Vec::new()), |(n, mut acc), (judged, found)| {
                acc.extend(found);
                (n + judged, acc)
            })
    });

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
