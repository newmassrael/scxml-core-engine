// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The environment the `mutation-rounds` gate chooses from, cleared before a
//! test drives it.
//!
//! A test that runs the gate as a subprocess inherits the environment it was
//! itself started in, and the gate reads that environment to decide WHAT TO
//! RUN. So a variable left in the caller's environment silently rewrites the
//! test's scenario — and the caller here is a CI job that sets exactly those
//! variables on purpose. `.github/workflows/mutation-rounds.yml` hands its
//! round job `SCE_MUTATION_ROUNDS` and `SCE_MUTATION_SHARD`; anything that job
//! runs which drives the gate again gets both, whether or not it meant to.
//!
//! Measured 2026-08-30: with `SCE_MUTATION_SHARD=1/2` in the environment,
//! `the_gate_starts_the_declared_service_for_that_round_and_no_other` fails
//! with `names a slice, but 3 casefile(s) are selected` — a verdict about the
//! caller's environment wearing the costume of a verdict about the tree. The
//! other five call sites survived that measurement, and NOT because they were
//! protected: the gate checks `SCE_MUTATION_ROUNDS_DRY_RUN` before it checks
//! the shard, so a dry run returns above the shard's guard. They are not
//! closed, they are merely upstream of the door — and that is exactly the
//! shape that comes back the day the gate reorders its own checks.
//!
//! ⚠ The repair is therefore NOT another `env_remove` per site. Before this,
//! each of the six sites carried a hand-written subtraction list, and between
//! them they cleared `SCE_MUTATION_ROUNDS` six times, `SCE_GATE_CHANGED_FILE`
//! five, `SCE_MUTATION_ROUNDS_DRY_RUN` six — and `SCE_MUTATION_SHARD` ZERO.
//! Six copies of a list is six chances to miss the next entry, and the list
//! had already missed one.
//!
//! So the list is DERIVED from the gate script and there is one command
//! builder. A fifth selector added to the gate tomorrow is cleared at every
//! site the day it lands, with nothing here to update.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The gate whose selectors these are, relative to the repository root.
pub const GATE_SCRIPT: &str = "scripts/gates/mutation-rounds.sh";

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every `SCE_*` name that appears in a line of the token's own text.
///
/// Anchored on a name boundary so a longer identifier that merely contains
/// `SCE_` is not split into a false name.
fn sce_names(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 4] != b"SCE_" {
            i += 1;
            continue;
        }
        let starts_a_name =
            i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
        let mut end = i + 4;
        while end < bytes.len()
            && (bytes[end].is_ascii_uppercase()
                || bytes[end].is_ascii_digit()
                || bytes[end] == b'_')
        {
            end += 1;
        }
        if starts_a_name && end > i + 4 {
            found.push(line[i..end].to_string());
        }
        i = end.max(i + 1);
    }
    found
}

/// The `SCE_*` name a line ASSIGNS, if it assigns one.
///
/// A script that sets a variable itself is not reading it from the caller, and
/// clearing such a name would be clearing a local. None today — the check
/// exists so a future `SCE_ROUND_TMP=...` inside the gate does not become a
/// variable every test is forced to remove.
fn assigned_name(line: &str) -> Option<String> {
    let mut text = line.trim_start();
    for prefix in ["export ", "local ", "declare ", "readonly "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            text = rest.trim_start();
        }
    }
    if !text.starts_with("SCE_") {
        return None;
    }
    let name: String = text
        .chars()
        .take_while(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    text[name.len()..].starts_with('=').then_some(name)
}

/// The selector variables `scripts/gates/mutation-rounds.sh` reads from the
/// environment.
///
/// Derived from the script rather than listed, because a list here is the
/// sixth copy of the thing that already went stale. Comment lines are stripped
/// first — not to narrow the answer but so the answer is about the CODE: a
/// name that survives only in prose is a name the gate no longer reads, and
/// clearing it would be a rule kept alive by its own explanation.
pub fn mutation_rounds_selectors() -> BTreeSet<String> {
    let path = repo_root().join(GATE_SCRIPT);
    let body = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    selectors_in(&body)
}

/// The same derivation over a script's TEXT.
///
/// Split from the reader above so both of its branches can be measured. The
/// gate assigns no `SCE_*` name today, which makes the "assigned names are
/// locals, not selectors" rule unreachable from the real file — and a rule
/// nothing can reach is a rule nothing is keeping true. A synthetic script
/// exercises it; the real one is what the helper actually uses.
pub fn selectors_in(script: &str) -> BTreeSet<String> {
    let code: Vec<&str> = script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();

    let assigned: BTreeSet<String> = code.iter().filter_map(|l| assigned_name(l)).collect();
    code.iter()
        .flat_map(|line| sce_names(line))
        .filter(|name| !assigned.contains(name))
        .collect()
}

/// A `bash` command with every one of those selectors removed.
///
/// Use this instead of `Command::new("bash")` anywhere a test drives the
/// `mutation-rounds` gate — directly, or through a workflow step that calls
/// it. The caller then sets the ones its own scenario means; those `.env()`
/// calls run after this and win, which is why the clearing lives in the
/// constructor rather than in a method a call site could forget to chain.
pub fn gate_shell() -> Command {
    let mut cmd = Command::new("bash");
    for name in mutation_rounds_selectors() {
        cmd.env_remove(name);
    }
    cmd
}
