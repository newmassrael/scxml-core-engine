// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `cli`-feature gating: which integration targets need the binary, and
// which commands must therefore turn the feature on.
//
// `sce-codegen` is `required-features = ["cli"]`, so cargo builds it —
// and sets `CARGO_BIN_EXE_sce-codegen` to a path that exists — only when
// that feature is on. A target reaches it by expanding
// env!("CARGO_BIN_EXE_sce-codegen"), and one that does so without
// declaring the feature still compiles: the env var is set to where the
// binary WOULD go, and the test dies at run time on `NotFound`. Or it
// passes, because a `target/debug/sce-codegen` left by some earlier build
// is still sitting there. That second outcome is what kept this
// invisible.
//
// Measured 2026-09-02, before this gate existed: 25 of the 53 targets
// that spawn the binary carried no declaration, and a run in a clean
// target directory failed 14 of `rust_derive_ssot`'s 16 cases on
// `spawn sce-codegen: NotFound` while the same command in the working
// tree passed on a stale binary. The manifest and the sources had drifted
// apart with nothing reading both.
//
// Two questions live here, and they are not the same one:
//
//   * Does every target that needs the binary say so? That is the
//     manifest, and getting it wrong costs a run-time death or a false
//     pass.
//   * Does every command that reaches such a target turn the feature on?
//     That is the lanes, and it is the cost of fixing the first: cargo
//     drops a target whose `required-features` are unmet WITHOUT
//     building it and without reporting a skip, so a declaration turns a
//     loud death into a silent absence. `hook_ci_parity` pins that the
//     hook and CI agree with each other; nothing asked whether what they
//     agree on is enough to run the targets that exist.
//
// The second question is not about `cargo test` alone, and reading it
// that way cost a regression on the commit that first declared these
// features: `clippy` runs `--workspace --all-targets`, which asks for
// every target the FEATURE SET allows, so the declaration took 25
// integration targets out of lint coverage without a word. Measured
// 2026-09-02 on that commit — clippy saw 135 of this package's
// integration targets and none of the 53 gated ones.
//
// It reads every command-carrying file under `scripts/` and
// `.github/workflows/`, which puts it among the gates whose inputs no
// `paths:` filter can enumerate: the next lane to name one of these
// targets will be a file that does not exist today. It is registered in
// `workflow_trigger_coverage`'s `UNFILTERABLE_GATES` and runs by name in
// `scripts/gates/tree-hygiene.sh`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::gate_selectors::repo_root;

// `code_only` is called by its full path rather than imported. The mutation
// that matters here is the one that stops stripping prose, and with a `use`
// line the deletion takes the import out of service too, so the tree fails
// to compile under `-D warnings` and the case reports INCONCLUSIVE instead
// of the red it exists to produce (measured 2026-09-02, on this file).

/// The feature the binary is gated behind.
const FEATURE: &str = "cli";

/// The env var cargo sets for a test target that can reach the binary.
const BIN_ENV: &str = "CARGO_BIN_EXE_sce-codegen";

/// The `env!` call that reads it, assembled rather than written out.
///
/// A target reaches the binary by expanding that macro, not by naming the
/// variable: this file names the variable in a constant and in its
/// diagnostics, and spawns nothing. Writing the whole pattern as a literal
/// here would therefore put this gate into its own population, and the
/// honest fix is a pattern no source can carry by talking about it rather
/// than an exemption for the one file that does.
fn binary_macro_call() -> String {
    format!("env!(\"{BIN_ENV}\")")
}

/// Source with every space removed, so a call split across lines by
/// rustfmt reads the same as one that fits.
fn without_spaces(src: &str) -> String {
    src.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Trees whose files may carry a command that names a test target.
///
/// Both are read whole rather than by a list of file names: a gate script
/// and a workflow are added by writing a new file, which is precisely the
/// arrival a list written today cannot name.
const COMMAND_TREES: &[&str] = &["scripts", ".github/workflows"];

/// Command-carrying files that are not inside those trees.
const COMMAND_FILES: &[&str] = &["tools/git-hooks/pre-push"];

/// Integration-test sources of this package, as `(target name, text)`.
///
/// The target name of an auto-discovered test is its file stem, and the
/// explicit `[[test]]` entries in the manifest use the same spelling, so
/// one name joins the two sides.
fn test_sources() -> Vec<(String, String)> {
    let dir = repo_root().join("sce-build/tests");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .map(|p| {
            let stem = p
                .file_stem()
                .expect("a source file has a stem")
                .to_string_lossy()
                .into_owned();
            let text =
                fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
            (stem, text)
        })
        .collect();
    out.sort();
    assert!(
        out.len() > 100,
        "found {} integration source(s); the directory read is broken, not the tree",
        out.len()
    );
    out
}

/// Targets whose code — not whose prose — reaches the binary.
///
/// Comments are stripped first. Several of these files explain the
/// feature gate in a comment that names the env var, and a scan that
/// counted those would be reading the explanation instead of the code,
/// which is the mistake `common::rust_source` exists to prevent.
fn binary_driven_targets() -> BTreeSet<String> {
    let call = without_spaces(&binary_macro_call());
    test_sources()
        .into_iter()
        .filter(|(_, text)| without_spaces(&common::rust_source::code_only(text)).contains(&call))
        .map(|(name, _)| name)
        .collect()
}

/// Targets the manifest declares as needing the feature.
///
/// Parsed rather than pattern-matched over the whole file so that a
/// `required-features` belonging to a `[[bin]]` or `[[bench]]` cannot be
/// read as a test's.
fn declared_targets() -> BTreeSet<String> {
    let path = repo_root().join("sce-build/Cargo.toml");
    let manifest =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    let mut out = BTreeSet::new();
    let mut in_test = false;
    let mut name: Option<String> = None;
    let mut gated = false;

    let close = |name: &mut Option<String>, gated: &mut bool, out: &mut BTreeSet<String>| {
        if let Some(n) = name.take() {
            if *gated {
                out.insert(n);
            }
        }
        *gated = false;
    };

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            close(&mut name, &mut gated, &mut out);
            in_test = trimmed == "[[test]]";
            continue;
        }
        if !in_test {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("name") {
            if let Some((_, value)) = rest.split_once('=') {
                name = Some(value.trim().trim_matches('"').to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("required-features") {
            if let Some((_, value)) = rest.split_once('=') {
                gated = value.split(',').any(|f| {
                    f.trim().trim_matches(|c: char| {
                        c == '[' || c == ']' || c == '"' || c.is_whitespace()
                    }) == FEATURE
                });
            }
        }
    }
    close(&mut name, &mut gated, &mut out);
    out
}

/// Every file under the command trees, plus the named ones.
fn command_files() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => panic!("read {}: {}", dir.display(), e),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }

    let root = repo_root();
    let mut paths = Vec::new();
    for tree in COMMAND_TREES {
        walk(&root.join(tree), &mut paths);
    }
    for file in COMMAND_FILES {
        paths.push(root.join(file));
    }
    paths.sort();

    let mut out = Vec::new();
    for path in paths {
        // A binary artifact under one of these trees is not a command.
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        out.push((name, text));
    }
    assert!(
        out.len() > 50,
        "found {} command-carrying file(s); the tree read is broken, not the tree",
        out.len()
    );
    out
}

/// Cargo subcommands that compile a target and so can drop a gated one.
///
/// `test` is not the only way to lose these targets, and reading it as
/// the only way cost this repository a regression on the very commit that
/// declared the features: `clippy-check.yml` runs `--workspace
/// --all-targets` with no feature flag, so the declaration took 25
/// integration targets out of lint coverage silently. `check` and `build`
/// are here for the same reason rather than because a lane uses them
/// today — the arm that only ever matches one spelling is the one that
/// stops matching.
const TARGET_SUBCOMMANDS: &[&str] = &["cargo test", "cargo clippy", "cargo check", "cargo build"];

/// The target-compiling cargo invocations a file carries, one string per
/// logical command.
///
/// Whole-line comments are dropped before anything else. A workflow
/// header explaining that "a bare `cargo test --workspace` runs them" is
/// prose about a command that no longer exists, and counting it would
/// fail the lane for a sentence — the same reading that
/// `common::rust_source` prevents on the Rust side. A YAML `name:` label
/// is NOT prose and stays in: a label naming a command the step does not
/// run is its own defect, and `hook_ci_parity` scans labels for the same
/// reason.
///
/// Continuation backslashes join lines, so a command split for width is
/// one string here.
fn cargo_test_commands(text: &str) -> Vec<String> {
    let mut joined: Vec<String> = Vec::new();
    let mut pending = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(head) = trimmed.strip_suffix('\\') {
            pending.push_str(head);
            pending.push(' ');
            continue;
        }
        pending.push_str(trimmed);
        joined.push(std::mem::take(&mut pending));
    }
    if !pending.is_empty() {
        joined.push(pending);
    }
    joined
        .into_iter()
        .flat_map(|cmd| shell_segments(&cmd))
        .filter(|cmd| TARGET_SUBCOMMANDS.iter().any(|sub| cmd.contains(sub)))
        .collect()
}

/// One joined line split at the shell operators that end a command.
///
/// Without this the failure message a gate script hands `sce_gate_fail`
/// answers for the command it describes: `scripts/gates/clippy.sh` reads
///
/// ```sh
/// cargo clippy --workspace --all-targets --features cli -- -D warnings \
///     || sce_gate_fail "cargo clippy --workspace --all-targets --features cli"
/// ```
///
/// and the continuation backslash makes those one string, so deleting the
/// flag from the command left the flag standing in the label three words
/// later and the mutation SURVIVED. Measured 2026-09-02; it is the same
/// reading the comment strip prevents, arriving through a label instead of
/// a comment, and the third time this repository has paid for it.
///
/// A label is still scanned — it is a separate segment now, and a label
/// naming a feature its command does not pass is its own defect, which is
/// why `hook_ci_parity` reads labels too.
fn shell_segments(cmd: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let bytes: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let two: String = bytes[i..(i + 2).min(bytes.len())].iter().collect();
        if two == "||" || two == "&&" {
            out.push(String::new());
            i += 2;
            continue;
        }
        if bytes[i] == ';' || bytes[i] == '|' {
            out.push(String::new());
            i += 1;
            continue;
        }
        out.last_mut().expect("out is never empty").push(bytes[i]);
        i += 1;
    }
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Feature names a command enables.
fn features_of(cmd: &str) -> BTreeSet<String> {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    let mut out = BTreeSet::new();
    for (i, token) in tokens.iter().enumerate() {
        let list = if let Some(rest) = token.strip_prefix("--features=") {
            rest
        } else if *token == "--features" {
            match tokens.get(i + 1) {
                Some(next) => next,
                None => continue,
            }
        } else {
            continue;
        };
        out.extend(
            list.split(',')
                .map(|f| f.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                .filter(|f| !f.is_empty()),
        );
    }
    if cmd.contains("--all-features") {
        out.insert(FEATURE.to_string());
    }
    out
}

/// Test targets a command names with `--test`.
fn named_targets(cmd: &str) -> BTreeSet<String> {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    let mut out = BTreeSet::new();
    for (i, token) in tokens.iter().enumerate() {
        if *token != "--test" {
            continue;
        }
        if let Some(next) = tokens.get(i + 1) {
            out.insert(next.trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    out
}

/// Whether a command builds this package's integration tests without
/// naming which ones.
///
/// Naming any target with `--test` takes a command out of this arm even
/// when the named target is not gated: cargo then builds that target and
/// no other, so the gated ones are not in the run to be dropped from it.
/// `scripts/measure-scope-obligation.sh` is the case — `-p sce-build
/// --test scope_obligation`, a target that needs no binary — and reading
/// its `-p` alone reported it as a lane running a smaller suite than it
/// claims, which it is not.
///
/// A command restricted to non-test targets is out for the same reason:
/// `--lib` compiles no integration test at all.
///
/// The subcommands differ in what they build by default, and reading them
/// alike would be wrong in both directions. `cargo test` builds every test
/// target unless told otherwise; `clippy`, `check` and `build` build the
/// library and binaries and reach an integration test only through
/// `--all-targets` or `--tests`. So `cargo build --bin sce-codegen -p
/// sce-build` is not in the population, and `cargo clippy --workspace
/// --all-targets` is.
fn sweeps_the_package(cmd: &str) -> bool {
    if !named_targets(cmd).is_empty() {
        return false;
    }
    let reaches_package = cmd.contains("--workspace")
        || cmd.contains("-p sce-build")
        || cmd.contains("--package sce-build");
    if !reaches_package {
        return false;
    }
    let asks_for_tests = cmd.contains("--all-targets") || cmd.contains("--tests");
    let restricted = [
        "--lib",
        "--bins",
        "--bin ",
        "--doc",
        "--benches",
        "--examples",
    ]
    .iter()
    .any(|flag| cmd.contains(flag));
    if restricted && !asks_for_tests {
        return false;
    }
    cmd.contains("cargo test") || asks_for_tests
}

#[test]
fn every_target_that_spawns_the_binary_declares_the_feature() {
    let driven = binary_driven_targets();
    let declared = declared_targets();
    assert!(
        !driven.is_empty(),
        "no target was found to spawn the binary — the scan is broken, not the tree"
    );

    let undeclared: Vec<&String> = driven.difference(&declared).collect();
    assert!(
        undeclared.is_empty(),
        "{} of {} target(s) that spawn `sce-codegen` do not declare \
         `required-features = [\"{FEATURE}\"]` in sce-build/Cargo.toml.\n\
         Without it cargo builds the target with the binary absent, so the \
         target dies on `NotFound` — or passes on a stale binary left by an \
         earlier build, which is how this stayed unseen.\n{}",
        undeclared.len(),
        driven.len(),
        undeclared
            .iter()
            .map(|n| format!("  {n}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn no_target_declares_the_feature_it_does_not_need() {
    let driven = binary_driven_targets();
    let declared = declared_targets();

    let spurious: Vec<&String> = declared.difference(&driven).collect();
    assert!(
        spurious.is_empty(),
        "{} declared target(s) require `{FEATURE}` without reaching \
         `{BIN_ENV}`.\n\
         A declaration cargo cannot justify is not free: an unmet feature \
         drops the target silently, so a target gated for no reason is one \
         that stops running the moment a lane spells its features \
         differently.\n\
         If a target genuinely needs the feature for something other than \
         the binary — a `cfg(feature)` item in the library, say — then this \
         gate has a second kind of member and must learn to name it, rather \
         than being given an exemption list.\n{}",
        spurious.len(),
        spurious
            .iter()
            .map(|n| format!("  {n}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_command_that_reaches_a_gated_target_enables_the_feature() {
    let declared = declared_targets();
    assert!(
        !declared.is_empty(),
        "the manifest declares no gated target — the parse is broken, not the tree"
    );

    let mut offenders: Vec<String> = Vec::new();
    let mut examined = 0usize;
    let mut sweeping = 0usize;
    let mut linting = 0usize;
    for (file, text) in command_files() {
        for cmd in cargo_test_commands(&text) {
            let named: BTreeSet<String> = named_targets(&cmd)
                .intersection(&declared)
                .cloned()
                .collect();
            let sweeps = sweeps_the_package(&cmd);
            if !named.is_empty() || sweeps {
                examined += 1;
            } else {
                continue;
            }
            if sweeps {
                sweeping += 1;
                if cmd.contains("cargo clippy") {
                    linting += 1;
                }
            }
            if features_of(&cmd).contains(FEATURE) {
                continue;
            }
            let reason = if named.is_empty() {
                "sweeps sce-build".to_string()
            } else {
                format!(
                    "names {}",
                    named.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            };
            offenders.push(format!("  {file}: {reason}\n    {cmd}"));
        }
    }

    assert!(
        examined > 0,
        "no command was found to reach a gated target — the scan is broken, \
         not the tree"
    );
    // The two arms fail differently, so they need separate floors. A reader
    // blinded to the sweeping arm still finds the targets `tree-hygiene.sh`
    // names, keeps `examined` above zero, and reports no offender — the
    // silence of a scan that stopped looking, wearing the shape of a pass.
    // The workspace suite is not a lane that may come and go: both the hook
    // and CI run it, `hook_ci_parity` pins that they agree, and a tree with
    // no command sweeping this package is a change to report rather than one
    // to absorb.
    //
    // The naming arm gets no floor of its own on purpose. Which targets a
    // lane runs by name is a list that shrinks legitimately — a gate retired,
    // a target folded into a sweep — and a floor there would make a good
    // change red.
    assert!(
        sweeping > 0,
        "no command sweeps sce-build's test targets, so the arm that judges \
         `--workspace` runs examined nothing. Every offender this test could \
         report would have come from the naming arm alone"
    );
    // And `cargo test` alone would keep that floor satisfied while clippy
    // went unread, which is exactly the regression this arm was widened
    // for: `cargo test --workspace` is a sweep, so narrowing
    // `TARGET_SUBCOMMANDS` back to it changes no verdict and produces no
    // red. The lint lane earns a floor of its own — it is not a lane that
    // may quietly come and go either, since a tree whose sources are never
    // linted is a change to report.
    //
    // `check` and `build` get no floor: no lane compiles this package's
    // integration tests through them today, so a floor there would be red
    // from the moment it was written.
    assert!(
        linting > 0,
        "no `cargo clippy` command sweeps this package's targets, so the \
         subcommand that lost 25 targets to a feature declaration is no \
         longer being read at all"
    );
    assert!(
        offenders.is_empty(),
        "{} command(s) reach a target requiring `{FEATURE}` without enabling \
         it. cargo drops an unmet-features target without building it and \
         without reporting a skip, so the command runs a smaller suite than \
         its name claims and reports success for what it never ran:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
