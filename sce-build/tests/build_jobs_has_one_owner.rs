// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How many parallel jobs a build may ask this machine for is decided in one
//! file.
//!
//! `scripts/lib/sce_build_jobs.sh` owns the rule and carries the measurement
//! behind it. The rule is four lines of arithmetic over `nproc` and
//! `/proc/loadavg`, which is exactly the size at which a second spelling looks
//! harmless — and the repository has already had one. `scripts/mutate` carried
//! its own copy under a comment that said, in so many words, "duplicated on
//! purpose, because the only library that has this drags a `cd` and a gate
//! slug with it". That reason expired when the rule was given a side-effect-
//! free home of its own, and nothing noticed: the copy sat there through the
//! commit that created the owner and every commit after it.
//!
//! So this is the check that would have noticed. A script that computes a job
//! count from the load average has to be reading it from the owner, or it is a
//! second answer to a question with one.
//!
//! What it does NOT forbid is a bare `nproc`. Plenty of tooling legitimately
//! wants the core count for something other than a build cap, and a check that
//! banned the syscall would be a check people route around. The signal is the
//! ARITHMETIC — `/proc/loadavg` subtracted from `nproc` — which is the rule
//! itself and nothing else.

use std::path::{Path, PathBuf};

/// The file that owns the rule. Exempt from its own check.
const OWNER: &str = "scripts/lib/sce_build_jobs.sh";

/// What the rule looks like when it has been written out by hand.
const RULE_MARKER: &str = "/proc/loadavg";

/// Scripts that source the owner directly, COUNTED rather than guessed: the
/// first draft of this file put 5 here from memory and the check failed on a
/// tree that was correct. Four is what the tree holds — the gate library and
/// three standalone scripts. Gates reach the rule through `gates/lib.sh` and
/// name it only in prose, which is why this counts sourcers and not mentions.
///
/// The floor exists because a scanner that stops finding files reads zero
/// re-spellings and passes: without it, a broken walk is indistinguishable
/// from a clean tree.
const READER_FLOOR: usize = 4;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Tracked files under `scripts/`, as git sees them.
fn scripts() -> Vec<String> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "scripts"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files scripts");
    assert!(out.status.success(), "git ls-files failed: {out:?}");
    String::from_utf8(out.stdout)
        .expect("utf-8")
        .lines()
        .map(str::to_string)
        .collect()
}

/// Source with whole-line comments removed.
///
/// The owner's header explains the rule in prose and names `/proc/loadavg`
/// while doing it, and so does the paragraph `scripts/mutate` now carries in
/// place of its copy. A scanner that read comments would report both as
/// re-spellings — describing a rule is not restating it.
fn without_comments(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn nothing_spells_the_parallel_jobs_rule_a_second_time() {
    let root = repo_root();
    let mut respellers: Vec<String> = Vec::new();
    let mut readers = 0usize;
    let mut scanned = 0usize;

    for rel in scripts() {
        if rel == OWNER {
            continue;
        }
        let path = root.join(&rel);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue; // not text; nothing to spell a shell rule in
        };
        scanned += 1;
        let code = without_comments(&raw);
        if code.contains("sce_build_jobs.sh") {
            readers += 1;
        }
        if code.contains(RULE_MARKER) {
            respellers.push(rel);
        }
    }

    assert!(
        scanned > 20,
        "read only {scanned} tracked file(s) under scripts/ — the walk is \
         broken, not the tree"
    );
    assert!(
        readers >= READER_FLOOR,
        "only {readers} script(s) source {OWNER}, expected at least \
         {READER_FLOOR}. Either the readers stopped sourcing it — which is the \
         defect this file exists for — or the marker this counts by moved"
    );
    assert!(
        respellers.is_empty(),
        "{respellers:?} compute a job count from /proc/loadavg instead of \
         sourcing {OWNER}. That rule has one home and a measurement behind it; \
         a second spelling is a second answer, and the one that was there \
         before survived the commit that created the owner because nothing \
         looked. Source the owner — it has no side effects, which is why it is \
         a file of its own."
    );
}
