// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The main build's ctest cases are split between two gates, and the split has
// to stay a partition.
//
// Measured 2026-08-12 with a logging shim in place of `ctest` for a full
// `scripts/gate --all`: 28 gates passed and the only runs against the main
// build were `w3c-c11`'s two. 159 of 382 registered cases — every `mesh_*`
// case including all 19 zenoh ones, plus twenty-four C++ unit suites — were
// executed by no gate, and by no workflow either, since none configures the
// main tree. `cpp-suite` now runs exactly the complement.
//
// Selecting by label rather than by name is what keeps a third list from
// existing, and it is only safe while the two selectors are exact complements
// of each other over one build directory. `cpp-suite` re-checks that at run
// time, where it can count. This checks the two properties that decide it
// before either gate is run at all, because the run-time check needs a
// configured tree and this does not: the same label on both sides, and one
// resolver for the directory.
//
// Only code is read. An earlier parity test in this suite accepted the word
// `pipefail` from a COMMENT as the declaration it was looking for and passed a
// mutation that removed the real setting, so comment lines are stripped before
// anything is matched.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// The script with comment lines and trailing comments removed, so a word in
/// prose can never satisfy a check about what the script does.
fn code_of(gate: &str) -> String {
    let path = repo_root().join("scripts/gates").join(format!("{gate}.sh"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    text.lines()
        .map(|line| match line.find('#') {
            // A `#` inside quotes is not a comment; none of these scripts has
            // one before their ctest invocations, and treating the whole line
            // as code when quotes are open keeps this from silently dropping a
            // selector it should have read.
            Some(i) if line[..i].matches('"').count() % 2 == 0 => &line[..i],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Logical commands: backslash continuations joined, so a `ctest` invocation
/// split across lines is read as the one command it is.
fn logical_lines(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending = String::new();
    for line in code.lines() {
        let trimmed = line.trim_end();
        if let Some(head) = trimmed.strip_suffix('\\') {
            pending.push_str(head);
            pending.push(' ');
        } else {
            pending.push_str(trimmed);
            out.push(std::mem::take(&mut pending));
        }
    }
    if !pending.is_empty() {
        out.push(pending);
    }
    out
}

/// The selector on the invocation that RUNS the suite, as opposed to the ones
/// that only count what is registered. Both gates spell a run with
/// `--output-on-failure`; a counting call passes `-N` and no such flag.
fn run_selector(gate: &str) -> (String, String) {
    let code = code_of(gate);
    let runs: Vec<String> = logical_lines(&code)
        .into_iter()
        .filter(|l| l.contains("ctest ") && l.contains("--output-on-failure"))
        .collect();
    assert_eq!(
        runs.len(),
        1,
        "{gate} has {} ctest invocation(s) that run a suite; this reads the \
         one that decides the verdict and cannot pick between several",
        runs.len()
    );
    let selectors = label_selectors(&runs[0]);
    assert_eq!(
        selectors.len(),
        1,
        "{gate}'s run selects on {} label(s) ({selectors:?}); one label per \
         side is what makes the two halves complements",
        selectors.len()
    );
    selectors.into_iter().next().expect("checked above")
}

/// Every `-L <label>` / `-LE <label>` selector in the given code.
fn label_selectors(code: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in code.lines() {
        let mut words = line.split_whitespace().peekable();
        while let Some(word) = words.next() {
            if word == "-L" || word == "-LE" {
                if let Some(label) = words.peek() {
                    // The selector appears inside command substitutions too
                    // (`$(count_registered -LE c11)`), so the label carries the
                    // shell's punctuation. Keep the label's own characters.
                    let label: String = label
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                        .collect();
                    if !label.is_empty() {
                        out.push((word.to_string(), label));
                    }
                }
            }
        }
    }
    out
}

/// The two halves select on the same label, one including and one excluding.
///
/// A renamed label, or a second one added to either side, stops the two from
/// covering the registered set — and the cases that fall out are run by
/// nothing, which is the state `cpp-suite` was added to end.
#[test]
fn the_two_ctest_gates_select_complementary_halves_of_one_label() {
    let (c11_flag, c11_label) = run_selector("w3c-c11");
    let (rest_flag, rest_label) = run_selector("cpp-suite");

    assert_eq!(
        c11_label, rest_label,
        "the two ctest gates run on different labels ({c11_label} vs \
         {rest_label}), so they are not complements: cases carrying neither \
         are run by nothing"
    );
    assert_eq!(
        c11_flag, "-L",
        "w3c-c11 must INCLUDE its label; its run passes {c11_flag}"
    );
    assert_eq!(
        rest_flag, "-LE",
        "cpp-suite must EXCLUDE the C11 label — including it would make this \
         gate a second spelling of a run w3c-c11 already makes; its run passes \
         {rest_flag}"
    );
}

/// Both halves judge one directory, resolved in one place.
///
/// The resolution used to be copied into each gate, and the copies had already
/// diverged on the question of whether a restored CMake cache means the tree is
/// buildable. Two gates that disagree about which tree they judge cannot be
/// complements of each other on it.
#[test]
fn both_ctest_gates_resolve_the_build_directory_through_one_helper() {
    for gate in ["w3c-c11", "cpp-suite", "w3c-cpp"] {
        let code = code_of(gate);
        assert!(
            code.contains("sce_main_build_dir"),
            "{gate} does not resolve its build directory through \
             `sce_main_build_dir`; a private copy is a second answer to which \
             tree is under judgement"
        );
        assert!(
            !code.contains("CMAKE_BUILD_TYPE:STRING="),
            "{gate} still carries its own build-type check — the helper owns \
             that, and a copy is what let the readiness test drift between \
             these gates once already"
        );
    }
}

/// Both halves clear the log temporaries their own runs leak.
///
/// ctest renames `Testing/Temporary/LastTest.log.tmpNNNNN` to `LastTest.log`
/// when a suite finishes, so a `.tmp` file that outlives its run belongs to a
/// run that was killed. Nothing removed them: measured 2026-08-12, 40 orphans
/// totalling 774 MB, one of them 652 MB, all from a single day in April. A
/// completed run's log is 1 MB, so this is not a size problem — it is a count
/// that only ever goes up, in a directory nobody looks at.
#[test]
fn both_ctest_gates_clear_the_temporaries_their_runs_leak() {
    for gate in ["w3c-c11", "cpp-suite"] {
        assert!(
            code_of(gate).contains("sce_prune_ctest_temporaries"),
            "{gate} runs ctest without clearing the log temporaries an \
             interrupted run leaves behind. The gate that creates them is the \
             one that has to clear them; otherwise the count only grows, and \
             the directory it grows in is one nobody opens."
        );
    }
}

/// `cpp-suite` re-derives the partition from the registered counts at run time.
///
/// The static check above cannot see a case that carries neither label, or
/// carries both; only counting can. This asserts the count comparison is still
/// there, since it is the only reader of that failure.
#[test]
fn the_complement_gate_checks_the_partition_against_the_registered_counts() {
    let code = code_of("cpp-suite");
    assert!(
        code.contains("registered + c11 != total"),
        "cpp-suite no longer compares its half plus the C11 half against the \
         total registered count. Without it, a case labelled neither `c11` nor \
         anything else this gate selects is registered, run by no gate, and \
         reported by none — the exact silence that hid 159 cases"
    );
}
