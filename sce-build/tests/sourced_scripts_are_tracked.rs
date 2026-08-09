// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// A script this repo sources must exist in a fresh checkout.
//
// `.gitignore` carried an unanchored `lib/` rule aimed at build output.
// It also matched `scripts/lib/`, which is source: the shell locator
// that 54 `regen_*.sh` scripts and `backends/go/forge-runtime/
// generate.sh` source. The file was present on every developer machine
// and absent from every CI checkout, so the failure mode was "green
// locally, red in CI" across four jobs at once —
// `codegen_binary_resolution`, `generate-integration`, the Go
// conformance generate, and every regen script.
//
// Nothing could see it: the scripts run fine locally, `git status` says
// clean because the file is ignored rather than untracked, and no test
// asked whether a sourced path is something git would hand a fresh
// clone. This gate asks exactly that.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn tracked_paths() -> BTreeSet<String> {
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Lower bound on the `source`/`.` directives this gate must find. A
/// scan that matches nothing would pass while proving nothing.
const MIN_SOURCE_DIRECTIVES: usize = 20;

/// Every in-repo path a tracked shell script sources is itself tracked.
///
/// Only paths expressed relative to the repo (via `$REPO_ROOT`,
/// `$(dirname …)`, or a literal `scripts/…`) are checked — a directive
/// naming a system path like `/etc/profile` is not this repo's to
/// guarantee.
#[test]
fn every_sourced_repo_path_is_tracked() {
    let tracked = tracked_paths();
    let root = repo_root();

    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();

    for rel in tracked.iter().filter(|p| p.ends_with(".sh")) {
        let Ok(body) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for line in body.lines() {
            let line = line.trim();
            let Some(rest) = line
                .strip_prefix("source ")
                .or_else(|| line.strip_prefix(". "))
            else {
                continue;
            };
            // Take the argument and reduce it to a repo-relative path.
            // Interpolations that resolve to the repo root all end at
            // the same place, so the tail after the last `}` or `)` is
            // what identifies the file.
            let arg = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches('"');
            // Strip the leading interpolation that names the repo root
            // — `$REPO_ROOT/…`, `${ROOT}/…`, `$(dirname "$0")/…` all
            // resolve there, and what identifies the file is the tail.
            // An earlier revision skipped any argument containing `$`,
            // which is every real call site in this repo: the scan
            // found one directive instead of dozens.
            let tail = match arg.split_once('/') {
                Some((head, t)) if head.contains('$') => t,
                _ => arg,
            };
            if !tail.contains('/') || tail.starts_with('/') || tail.contains('$') {
                continue;
            }
            // Resolve `../` segments a `$(dirname "$0")/..` form leaves.
            let normalised: String = tail
                .split('/')
                .fold(Vec::new(), |mut acc, seg| {
                    match seg {
                        "." | "" => {}
                        ".." => {
                            acc.pop();
                        }
                        s => acc.push(s),
                    }
                    acc
                })
                .join("/");
            if normalised.is_empty() {
                continue;
            }
            checked += 1;
            if !tracked.contains(&normalised) {
                missing.push(format!(
                    "{rel} sources {normalised:?}, which git does not track"
                ));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "{} sourced path(s) are absent from a fresh checkout — present \
         locally only, so this passes on a developer machine and fails \
         in CI:\n  {}",
        missing.len(),
        missing.join("\n  "),
    );
    assert!(
        checked >= MIN_SOURCE_DIRECTIVES,
        "found only {checked} in-repo source directives; expected at \
         least {MIN_SOURCE_DIRECTIVES}. A scan that matches nothing \
         certifies nothing.",
    );
}
