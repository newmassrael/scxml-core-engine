// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Driving the mutation ledger from a test: seed a round, then ask about it.
//!
//! Two suites need exactly this pair and they are split for a reason that has
//! nothing to do with what they measure. `mutation_ledger` covers the record
//! format and the corpus questions; `ledger_holes_are_still_declared` covers
//! one question — whether a hole's case still exists — and is separate ONLY so
//! a mutation round can name it as an oracle.
//!
//! Measured 2026-08-30: a round over `mutation_ledger` cannot start. One of
//! its tests drives `scripts/mutate --check`, which calls `cargo metadata`,
//! and inside a cargo round that comes back `cargo metadata failed — the
//! selector cannot be checked`, so the baseline is red before a case is
//! applied. A suite that cannot be a baseline cannot be an oracle, and the
//! cases that hold this contract have to live somewhere a round can reach.
//!
//! Both helpers execute the shipped scripts rather than reimplementing them,
//! which is the whole point: a second copy of the writer or the reader would
//! be a second answer to what the ledger holds.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Run a snippet with the ledger library sourced, and return its stdout.
///
/// `HOME` is pointed at a directory of the caller's own so nothing here writes
/// into the real ledger, and `XDG_DATA_HOME` is deliberately pointed somewhere
/// else entirely — a caller is entitled to assume the record lands under
/// `HOME`, and the suite that checks that says so.
pub fn with_library(home: &Path, snippet: &str) -> String {
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -euo pipefail; source scripts/lib/mutation_ledger.sh; {snippet}"
        ))
        .current_dir(repo_root())
        .env("HOME", home)
        .env("XDG_DATA_HOME", home.join("decoy"))
        .output()
        .expect("run bash with the ledger library sourced");
    assert!(
        out.status.success(),
        "the snippet failed:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Ask `scripts/mutation-ledger` a question about a given ledger directory.
pub fn ask(ledger: &Path, args: &[&str]) -> String {
    ask_with_corpus(ledger, None, args)
}

/// The same, over a corpus directory of the caller's choosing.
///
/// A test that needs a casefile of a particular shape — one whose label the
/// reader cannot take, say — hands one over rather than editing the real
/// corpus, and the tool reads it through the same parser it reads the real one
/// with. A parser reimplemented in the test would answer a question about
/// itself.
pub fn ask_with_corpus(ledger: &Path, corpus: Option<&Path>, args: &[&str]) -> String {
    let mut cmd = Command::new(repo_root().join("scripts/mutation-ledger"));
    cmd.args(args)
        .current_dir(repo_root())
        .env("SCE_MUTATION_LEDGER_DIR", ledger);
    if let Some(dir) = corpus {
        cmd.env("SCE_MUTATION_CORPUS_DIR", dir);
    }
    let out = cmd.output().expect("run scripts/mutation-ledger");
    assert!(
        out.status.success(),
        "mutation-ledger {args:?} failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The case labels a casefile declares, read from the casefile.
///
/// The same text `scripts/mutation-ledger` reads, so a test that seeds a
/// record about "the first case in that file" names whatever that file
/// actually calls it today. A list written into a test instead would go stale
/// silently and take the assertion with it.
pub fn labels_of(casefile: &str) -> Vec<String> {
    let body = fs::read_to_string(repo_root().join(casefile))
        .unwrap_or_else(|e| panic!("read {casefile}: {e}"));
    body.lines()
        .filter_map(|line| line.strip_prefix("mutation_case "))
        .filter_map(|rest| {
            let rest = rest.trim_start();
            let quote = rest.chars().next()?;
            if quote != '"' && quote != '\'' {
                return None;
            }
            rest[1..].split(quote).next().map(str::to_string)
        })
        .collect()
}
