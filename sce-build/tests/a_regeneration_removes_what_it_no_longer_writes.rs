// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A regeneration removes each generated file the generator no longer writes,
//! names it, and touches nothing written by hand.
//!
//! Regenerating used to write what the generator produces now and remove
//! nothing. Measured 2026-10-21: when the escaping of a synthesized invoke id
//! changed (`invokedChild` became `0zinvokedzuchild`), the regeneration wrote
//! `test192__sce_synth_invoke__0zinvokedzuchild_sm.rs` and its Kotlin twin
//! beside the old `..._invokedChild` files, and the old ones stayed committed
//! for days. `regen-reproduces` compared the files a run writes and could not
//! see one it no longer writes.
//!
//! What is held here is `scripts/lib/sce_generated_tree.sh`, sourced the way
//! the regen scripts and `scripts/gates/regen-reproduces.sh` source it, so what
//! runs is the code a regeneration runs and not a copy of it:
//!
//! * a planted generated file the run does not write again leaves the tree, is
//!   named by the settle, and is named again by `sce_generated_tree_unproduced`,
//!   the reading `regen-reproduces` fails on;
//! * a failed run puts every file back;
//! * clearing a directory takes each kind of generated file and leaves a
//!   hand-written one;
//! * every file of the committed trees the regeneration sets aside is either
//!   recognised as generated or carries a licence header, so a new kind of
//!   generated file the rules miss is found here rather than left behind;
//! * every step of the regeneration is the bx action `regen-<step>`, every
//!   generated file of a step's trees is an output of that step's action, so a
//!   removal made on a build machine comes back instead of refusing the run,
//!   and no action reads or writes what another writes, so each runs, and can
//!   be kept, on its own.
//!
//! The generated files planted here carry the header the generator renders
//! (`render_header`) and the stub it writes (`stub_document`), so a rule of the
//! helper that drifts from the generator's own spelling fails here.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, SystemTime};

use sce_build::forge::drift::{render_header, DriftHeader};
use sce_build::parser::SCXMLParser;
use tempfile::tempdir;

const HELPER: &str = "scripts/lib/sce_generated_tree.sh";

/// An mtime no run of the test writes: 2000-01-01T00:00:00Z.
const OLD_MTIME_SECS: u64 = 946_684_800;

/// Run `script` under `bash -c` in `dir`, with the helper sourced first.
fn with_helper(dir: &Path, script: &str) -> Output {
    let helper = common::repository::root().join(HELPER);
    Command::new("bash")
        .arg("-c")
        .arg(format!(
            "set -euo pipefail\nsource '{}'\n{script}",
            helper.display()
        ))
        .current_dir(dir)
        .output()
        .expect("run bash with the generated-tree helper sourced")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `body` behind the drift header the generator puts on a file of this
/// comment prefix.
fn generated(prefix: &str, body: &str) -> String {
    let header = render_header(
        &DriftHeader {
            source_hash: [0; 32],
        },
        prefix,
    );
    format!("{header}{body}")
}

/// A file a person wrote: what every hand-written file in the swept trees
/// opens with.
fn hand_written(body: &str) -> String {
    format!("// SPDX-License-Identifier: AGPL-3.0-only\n{body}")
}

/// The module index a Rust regen script writes for its directory.
fn module_index(body: &str) -> String {
    format!("// GENERATED -- DO NOT EDIT (scripts/regen_example.sh)\n{body}")
}

/// The sidecar the generator writes beside a machine directory.
fn sourcemap() -> String {
    "{\n  \"version\": 1,\n  \"source_hash\": \"00\",\n  \"symbols\": {}\n}\n".to_string()
}

/// The stub the generator writes for a hybrid invoke's child, and the name it
/// writes it under, asked of the generator for a document of the W3C corpus.
fn hybrid_stub() -> (String, String) {
    let path = common::repository::root().join("resources/216/test216.scxml");
    let path_str = path.to_str().expect("the resource path is UTF-8");
    let Ok(model) = SCXMLParser::new().parse_file(path_str) else {
        panic!("{path_str} does not parse");
    };
    let invoke = model
        .iter_hybrid_invokes()
        .next()
        .expect("test216 holds a hybrid invoke");
    let stub = invoke
        .stub_document()
        .expect("test216's hybrid invoke has a stub child");
    (format!("{}.scxml", invoke.child_name), stub)
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("a planted file sits in a directory"))
        .unwrap_or_else(|e| panic!("create the directory of {rel}: {e}"));
    fs::write(&path, contents).unwrap_or_else(|e| panic!("write {rel}: {e}"));
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed:\n{}",
        text(&out.stderr)
    );
}

/// A repository holding one generated tree, `gen`, with a file of every kind
/// the helper recognises, a hand-written file inside it, and a generated file
/// outside it; everything staged, as a checkout's tracked files are. `next`
/// holds what the stand-in generation writes into `gen`.
fn planted_repository(dir: &Path) {
    let (stub_name, stub) = hybrid_stub();
    git(dir, &["init", "-q"]);
    write(
        dir,
        "gen/kept_sm.rs",
        &generated("//", "pub struct Kept;\n"),
    );
    write(
        dir,
        "gen/rewritten_sm.rs",
        &generated("//", "pub struct Old;\n"),
    );
    write(
        dir,
        "gen/stale_sm.rs",
        &generated("//", "pub struct Stale;\n"),
    );
    write(dir, "gen/mod.rs", &module_index("mod kept_sm;\n"));
    write(dir, &format!("gen/{stub_name}"), &stub);
    write(dir, "gen/sub/sce_sourcemap.json", &sourcemap());
    write(
        dir,
        "gen/hand_test.rs",
        &hand_written("#[test]\nfn t() {}\n"),
    );
    write(
        dir,
        "other/stale_sm.rs",
        &generated("//", "pub struct Outside;\n"),
    );

    write(
        dir,
        "next/kept_sm.rs",
        &generated("//", "pub struct Kept;\n"),
    );
    write(
        dir,
        "next/rewritten_sm.rs",
        &generated("//", "pub struct New;\n"),
    );
    write(
        dir,
        "next/new_sm.rs",
        &generated("//", "pub struct Added;\n"),
    );
    write(dir, "next/mod.rs", &module_index("mod kept_sm;\n"));
    write(dir, &format!("next/{stub_name}"), &stub);
    git(dir, &["add", "-A"]);
}

/// A generated file the run does not write again leaves the tree, and is
/// named twice: by the settle, and by the reading `regen-reproduces` fails on.
///
/// Turns red if `sce_generated_tree_settle` put back a file the run did not
/// write again (the `else` arm that collects `unproduced`), or if
/// `sce_generated_tree_unproduced` stopped asking git for deleted files.
#[test]
fn a_file_the_run_does_not_write_again_leaves_the_tree_and_is_named() {
    let dir = tempdir().expect("temp repository");
    let repo = dir.path();
    planted_repository(repo);

    let out = with_helper(
        repo,
        &format!(
            "touch -d @{OLD_MTIME_SECS} gen/kept_sm.rs\n\
             sce_generated_tree_stash gen\n\
             if [[ -e gen/kept_sm.rs || -e gen/stale_sm.rs ]]; then\n\
             \x20   echo 'the stash left a generated file in place' >&2; exit 3\n\
             fi\n\
             if [[ ! -e gen/hand_test.rs ]]; then\n\
             \x20   echo 'the stash took a hand-written file' >&2; exit 3\n\
             fi\n\
             cp -R next/. gen/\n\
             sce_generated_tree_settle 0\n\
             sce_generated_tree_unproduced\n"
        ),
    );
    let stderr = text(&out.stderr);
    assert!(out.status.success(), "the run failed:\n{stderr}");

    for gone in ["gen/stale_sm.rs", "gen/sub/sce_sourcemap.json"] {
        assert!(
            !repo.join(gone).exists(),
            "{gone} was generated before and not written again, and it is still there"
        );
        assert!(
            stderr.contains(gone),
            "the settle removed {gone} without naming it:\n{stderr}"
        );
    }
    assert!(
        !repo.join("gen/sub").exists(),
        "the directory the removal emptied is still there"
    );

    let unproduced: BTreeSet<String> = text(&out.stdout).lines().map(str::to_string).collect();
    let expected: BTreeSet<String> = ["gen/stale_sm.rs", "gen/sub/sce_sourcemap.json"]
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        unproduced, expected,
        "the files regen-reproduces is told are no longer produced are not \
         the ones the run stopped writing"
    );

    let kept_mtime = fs::metadata(repo.join("gen/kept_sm.rs"))
        .and_then(|m| m.modified())
        .expect("gen/kept_sm.rs has an mtime");
    assert_eq!(
        kept_mtime,
        SystemTime::UNIX_EPOCH + Duration::from_secs(OLD_MTIME_SECS),
        "a file the run wrote again with the same bytes did not get its old \
         copy back, so its mtime no longer says when its bytes changed"
    );
    assert_eq!(
        read(repo, "gen/rewritten_sm.rs"),
        generated("//", "pub struct New;\n"),
        "a file the run rewrote lost its new bytes to the copy set aside"
    );
    assert!(
        repo.join("gen/new_sm.rs").is_file(),
        "a file the run wrote for the first time is gone"
    );
    assert_eq!(
        read(repo, "gen/hand_test.rs"),
        hand_written("#[test]\nfn t() {}\n"),
        "a hand-written file in a generated tree changed"
    );
    assert!(
        repo.join("other/stale_sm.rs").is_file(),
        "a generated file outside the trees the run set aside was removed"
    );
}

/// A run that fails says nothing about what the generator no longer writes, so
/// every file goes back as it was — through the EXIT trap, the shape
/// `regen_all_committed_trees.sh` uses.
///
/// Turns red if the settle judged a failed run like a finished one (the
/// `status != 0` arm), or if `sce_generated_tree_on_exit` swallowed the
/// script's exit status.
#[test]
fn a_failed_run_puts_every_file_back() {
    let dir = tempdir().expect("temp repository");
    let repo = dir.path();
    planted_repository(repo);

    let out = with_helper(
        repo,
        "trap sce_generated_tree_on_exit EXIT\n\
         sce_generated_tree_stash gen\n\
         printf 'half written\\n' > gen/rewritten_sm.rs\n\
         false\n",
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "the script's own failure did not reach its exit status:\n{}",
        text(&out.stderr)
    );
    assert_eq!(
        read(repo, "gen/stale_sm.rs"),
        generated("//", "pub struct Stale;\n"),
        "a failed run removed a file it never got to write"
    );
    assert_eq!(
        read(repo, "gen/rewritten_sm.rs"),
        generated("//", "pub struct Old;\n"),
        "a failed run left a half-written file in place of the one set aside"
    );
    assert_eq!(
        read(repo, "gen/sub/sce_sourcemap.json"),
        sourcemap(),
        "a failed run removed a sidecar it never got to write"
    );
}

/// Clearing a directory takes every kind of file the generator writes and
/// leaves what a person wrote, whatever its name; `--recursive` reaches below.
///
/// Turns red if any rule of `sce_generated_file` stops recognising what the
/// generator writes (the drift banner, the module index banner, the sourcemap,
/// the hybrid stub), or starts taking a hand-written file.
#[test]
fn clearing_a_directory_takes_only_what_the_generator_wrote() {
    let dir = tempdir().expect("temp directory");
    let root = dir.path();
    let (stub_name, stub) = hybrid_stub();
    write(root, "d/machine_sm.rs", &generated("//", "pub struct M;\n"));
    write(
        root,
        "d/machine_sm.py",
        &generated("#", "class M:\n    pass\n"),
    );
    write(root, "d/mod.rs", &module_index("mod machine_sm;\n"));
    write(root, "d/sce_sourcemap.json", &sourcemap());
    write(root, &format!("d/{stub_name}"), &stub);
    // Hand-written, under names a pattern would have taken or that resemble
    // generated output.
    write(
        root,
        "d/machine_test_sm.rs",
        &hand_written("#[test]\nfn t() {}\n"),
    );
    write(
        root,
        "d/test_machine.py",
        "# SPDX-License-Identifier: AGPL-3.0-only\nimport unittest\n",
    );
    write(root, "d/notes.json", &sourcemap());
    write(
        root,
        "d/other_hybrid0.scxml",
        "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\"/>\n",
    );
    write(
        root,
        "d/pkg/nested_sm.go",
        &generated("//", "package pkg\n"),
    );

    let shallow = with_helper(root, "sce_generated_tree_clear d\n");
    assert!(
        shallow.status.success(),
        "clearing failed:\n{}",
        text(&shallow.stderr)
    );
    let generated_names = [
        "d/machine_sm.rs".to_string(),
        "d/machine_sm.py".to_string(),
        "d/mod.rs".to_string(),
        "d/sce_sourcemap.json".to_string(),
        format!("d/{stub_name}"),
    ];
    for gone in &generated_names {
        assert!(
            !root.join(gone).exists(),
            "{gone} is generator output and clearing left it"
        );
    }
    for kept in [
        "d/machine_test_sm.rs",
        "d/test_machine.py",
        "d/notes.json",
        "d/other_hybrid0.scxml",
    ] {
        assert!(
            root.join(kept).is_file(),
            "{kept} was written by hand and clearing removed it"
        );
    }
    assert!(
        root.join("d/pkg/nested_sm.go").is_file(),
        "clearing without --recursive reached below the directory"
    );

    let deep = with_helper(root, "sce_generated_tree_clear --recursive d\n");
    assert!(
        deep.status.success(),
        "clearing --recursive failed:\n{}",
        text(&deep.stderr)
    );
    assert!(
        !root.join("d/pkg").exists(),
        "clearing --recursive left a generated file below, or the directory it emptied"
    );
    assert!(
        root.join("d/machine_test_sm.rs").is_file(),
        "clearing --recursive removed a hand-written file"
    );
}

/// The trees `regen_all_committed_trees.sh` sets aside, as the helper names
/// them, relative to the repository root.
fn swept_trees() -> Vec<String> {
    let out = with_helper(&common::repository::root(), "sce_generated_trees\n");
    assert!(
        out.status.success(),
        "sce_generated_trees failed:\n{}",
        text(&out.stderr)
    );
    let trees: Vec<String> = text(&out.stdout).lines().map(str::to_string).collect();
    assert!(
        trees.len() >= 10,
        "only {} tree(s) are set aside — the list was emptied, not the tree",
        trees.len()
    );
    trees
}

/// Every committed file of the swept trees is recognised as generated or was
/// written by hand, and the W3C trees, which hold nothing written by hand, are
/// recognised whole.
///
/// A file the rules do not recognise is one the regeneration neither sets aside
/// nor removes: the stale-file shape this suite exists for, coming back
/// through a kind of file nobody taught the helper. Every hand-written file in
/// these trees opens with its licence header, which is what the second half of
/// the split reads.
#[test]
fn every_file_of_a_swept_tree_is_generated_or_written_by_hand() {
    let root = common::repository::root();
    let trees = swept_trees();
    let tree_refs: Vec<&str> = trees.iter().map(String::as_str).collect();
    let tracked: Vec<String> = common::repository::paths_git_tracks(&tree_refs)
        .into_iter()
        .filter(|rel| root.join(rel).is_file())
        .collect();

    let listing = with_helper(
        &root,
        &format!("sce_generated_tree_files {}\n", tree_refs.join(" ")),
    );
    assert!(
        listing.status.success(),
        "sce_generated_tree_files failed:\n{}",
        text(&listing.stderr)
    );
    let recognised: BTreeSet<String> = text(&listing.stdout).lines().map(str::to_string).collect();

    let mut unaccounted = Vec::new();
    let mut by_hand = 0usize;
    for rel in &tracked {
        let first = read(&root, rel)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        let hand = first.contains("SPDX-License-Identifier");
        if recognised.contains(rel) {
            assert!(
                !hand,
                "{rel} opens with a licence header and is taken for generated"
            );
        } else if hand {
            by_hand += 1;
        } else {
            unaccounted.push(rel.clone());
        }
    }
    assert!(
        unaccounted.is_empty(),
        "{} committed file(s) in the trees a regeneration sets aside are neither \
         recognised as generated nor written by hand, so a regeneration that \
         stopped writing one would leave it behind. Teach `sce_generated_file` in \
         {HELPER} the header they carry:\n  {}",
        unaccounted.len(),
        unaccounted.join("\n  ")
    );

    // The W3C trees hold generator output only, so every committed file there
    // must be recognised, and each kind must be seen at least once: an arm of
    // the rules that matched nothing would otherwise pass unseen.
    let w3c: Vec<&String> = tracked
        .iter()
        .filter(|rel| {
            rel.starts_with("backends/rust/tests/src/generated/")
                || rel.starts_with("backends/kotlin/tests/src/main/kotlin/com/sce/generated/test")
        })
        .collect();
    assert!(
        w3c.len() >= 1000,
        "only {} committed W3C file(s) were read — the scan is broken, not the tree",
        w3c.len()
    );
    let missed: Vec<&&String> = w3c
        .iter()
        .filter(|rel| !recognised.contains(**rel))
        .collect();
    assert!(
        missed.is_empty(),
        "committed W3C file(s) the helper does not recognise as generated: {missed:#?}"
    );
    for (kind, suffix) in [
        ("hybrid stub", ".scxml"),
        ("sourcemap", "/sce_sourcemap.json"),
        ("module index", "/mod.rs"),
    ] {
        assert!(
            w3c.iter().any(|rel| rel.ends_with(suffix)),
            "no committed W3C {kind} was read, so its rule went unexercised"
        );
    }
    assert!(
        by_hand >= 1,
        "no hand-written file was found in the swept trees — the split read nothing"
    );
}

/// The steps of `regen_all_committed_trees.sh`, as the helper names them.
fn regen_steps() -> Vec<String> {
    let out = with_helper(&common::repository::root(), "sce_regen_steps\n");
    assert!(
        out.status.success(),
        "sce_regen_steps failed:\n{}",
        text(&out.stderr)
    );
    let steps: Vec<String> = text(&out.stdout).lines().map(str::to_string).collect();
    assert!(
        steps.len() >= 10,
        "only {} step(s) are named — the list was emptied, not the script",
        steps.len()
    );
    steps
}

/// The trees step `step` sets aside, as the helper names them.
fn step_trees(step: &str) -> Vec<String> {
    let out = with_helper(
        &common::repository::root(),
        &format!("sce_generated_trees {step}\n"),
    );
    assert!(
        out.status.success(),
        "sce_generated_trees {step} failed:\n{}",
        text(&out.stderr)
    );
    text(&out.stdout).lines().map(str::to_string).collect()
}

/// One `[actions.<name>]` table of bx.toml: its command and its patterns.
#[derive(Debug, Default)]
struct BxAction {
    run: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
}

/// The double-quoted strings of one line of bx.toml.
fn quoted(line: &str) -> Vec<String> {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

fn push_patterns(action: &mut BxAction, key: &str, patterns: Vec<String>) {
    if key == "inputs" {
        action.inputs.extend(patterns);
    } else {
        action.outputs.extend(patterns);
    }
}

/// Every `[actions.<name>]` table of bx.toml, by name.
///
/// Read in the shape bx.toml writes them: `run = '...'` on one line, and
/// `inputs` and `outputs` each either on one line or as an array of one
/// double-quoted pattern a line. Comment lines are skipped.
fn bx_actions() -> BTreeMap<String, BxAction> {
    let manifest = read(&common::repository::root(), "bx.toml");
    let mut actions: BTreeMap<String, BxAction> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut open_list: Option<&'static str> = None;
    for raw in manifest.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(key) = open_list {
            let name = current.as_ref().expect("an open list is inside an action");
            let action = actions.get_mut(name).expect("the action was recorded");
            push_patterns(action, key, quoted(line));
            if line.ends_with(']') {
                open_list = None;
            }
            continue;
        }
        if line.starts_with('[') {
            current = line
                .strip_prefix("[actions.")
                .and_then(|rest| rest.strip_suffix(']'))
                .map(str::to_string);
            if let Some(name) = &current {
                actions.entry(name.clone()).or_default();
            }
            continue;
        }
        let Some(name) = current.as_ref() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        let action = actions.get_mut(name).expect("the action was recorded");
        match key {
            "run" => action.run = value.trim_matches(['\'', '"']).to_string(),
            "inputs" | "outputs" => {
                push_patterns(action, key, quoted(value));
                if !value.ends_with(']') {
                    open_list = Some(if key == "inputs" { "inputs" } else { "outputs" });
                }
            }
            _ => {}
        }
    }
    assert!(
        !actions.is_empty(),
        "no [actions.<name>] table was read out of bx.toml"
    );
    actions
}

/// Whether one path component matches one pattern component, where `*` is any
/// run of characters and `?` one character, as bx reads them.
fn component_matches(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some((b'*', rest)) => (0..=text.len()).any(|n| component_matches(rest, &text[n..])),
        Some((b'?', rest)) => !text.is_empty() && component_matches(rest, &text[1..]),
        Some((c, rest)) => text.first() == Some(c) && component_matches(rest, &text[1..]),
    }
}

fn components_match(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => (0..=path.len()).any(|n| components_match(rest, &path[n..])),
        Some((p, rest)) => {
            path.first()
                .is_some_and(|c| component_matches(p.as_bytes(), c.as_bytes()))
                && components_match(rest, &path[1..])
        }
    }
}

/// Whether `pattern` covers `path`: it matches the path or a directory above
/// it, which is how bx judges an action's declared outputs.
fn covers(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.split('/').collect();
    (1..=path.len()).any(|n| components_match(&pattern, &path[..n]))
}

fn is_wild(component: &str) -> bool {
    component.contains(['*', '?'])
}

/// Whether some path could be covered by both patterns, as bx decides it
/// (bx RFC 0003, section 6): component by component, two literals meet when
/// they are the same, a literal and a wildcard component when the wildcard
/// matches the literal, two wildcard components always, and `**` meets
/// anything; a pattern that ends while every component so far met covers what
/// is under it.
fn may_overlap(a: &str, b: &str) -> bool {
    for (x, y) in a.split('/').zip(b.split('/')) {
        if x == "**" || y == "**" {
            return true;
        }
        let meet = match (is_wild(x), is_wild(y)) {
            (false, false) => x == y,
            (true, false) => component_matches(x.as_bytes(), y.as_bytes()),
            (false, true) => component_matches(y.as_bytes(), x.as_bytes()),
            (true, true) => true,
        };
        if !meet {
            return false;
        }
    }
    true
}

/// The overlap rule read here is the one bx applies, on the examples its RFC
/// gives and on the two shapes bx.toml leans on.
///
/// Turns red if `may_overlap` stopped treating a pattern that ends as covering
/// what is under it (the `sce-specs/src/*` case), or matched a wildcard against
/// a literal it does not match (the `*/*.go` case).
#[test]
fn the_overlap_rule_is_the_one_bx_applies() {
    assert!(!may_overlap(
        "crates/*/Cargo.toml",
        "crates/tests/src/gen/**"
    ));
    assert!(may_overlap("gen/*.rs", "gen/*.go"));
    assert!(may_overlap(
        "sce-specs/src/*",
        "sce-specs/src/*/generated/**"
    ));
    assert!(!may_overlap(
        "backends/go/forge-runtime/*/*.go",
        "backends/go/forge-runtime/round_trip/generated/**"
    ));
    assert!(!may_overlap(
        "backends/kotlin/tests/src/test/kotlin/com/sce/w3c/W3CTestBase.kt",
        "backends/kotlin/tests/src/test/kotlin/com/sce/w3c/Test*.kt"
    ));
}

/// Every step of the regeneration is the bx action `regen-<step>`, whose
/// command is the script given that step, and bx.toml declares no other
/// action.
///
/// The second half is what lets the bare `bx --actions`, which runs every
/// declared action, stand for "regenerate everything". Turns red if a step is
/// added to `sce_regen_steps` without its action (or the other way round), if
/// an action's `run` names another step, or if an action that is no step is
/// declared without the callers of `bx --actions` being given a list.
#[test]
fn every_step_is_the_bx_action_that_runs_it() {
    let actions = bx_actions();
    let steps = regen_steps();
    let expected: BTreeSet<String> = steps.iter().map(|s| format!("regen-{s}")).collect();
    let declared: BTreeSet<String> = actions.keys().cloned().collect();
    assert_eq!(
        declared, expected,
        "the actions bx.toml declares are not the steps of \
         scripts/regen_all_committed_trees.sh, one `regen-<step>` each"
    );
    for step in &steps {
        let action = &actions[&format!("regen-{step}")];
        assert_eq!(
            action.run,
            format!("scripts/regen_all_committed_trees.sh {step}"),
            "[actions.regen-{step}] does not run the step it is named for"
        );
        assert!(
            !action.inputs.is_empty() && !action.outputs.is_empty(),
            "[actions.regen-{step}] declares no inputs or no outputs"
        );
    }
}

/// Every committed generated file of a step's trees is covered by an output of
/// that step's bx action.
///
/// The step sets those files aside and settles them when it ends, so the
/// removal of one it no longer writes is a change of that step alone, which bx
/// carries back from a build machine like a write; a change outside the
/// action's declared outputs refuses its whole result. Turns red if a tree is
/// added to a step in `sce_generated_trees` without an output of its action,
/// or if an output is narrowed past a generated file (`test_*.rs`,
/// `Test*.kt`).
#[test]
fn every_generated_file_of_a_step_is_an_output_of_its_action() {
    let root = common::repository::root();
    let actions = bx_actions();
    let mut checked = 0usize;
    let mut uncovered = Vec::new();
    for step in regen_steps() {
        let Some(action) = actions.get(&format!("regen-{step}")) else {
            panic!("bx.toml declares no [actions.regen-{step}]");
        };
        let trees = step_trees(&step);
        if trees.is_empty() {
            continue;
        }
        let tree_refs: Vec<&str> = trees.iter().map(String::as_str).collect();
        let tracked: BTreeSet<String> = common::repository::paths_git_tracks(&tree_refs)
            .into_iter()
            .collect();
        let listing = with_helper(
            &root,
            &format!("sce_generated_tree_files {}\n", tree_refs.join(" ")),
        );
        assert!(
            listing.status.success(),
            "sce_generated_tree_files failed for step {step}:\n{}",
            text(&listing.stderr)
        );
        for file in text(&listing.stdout)
            .lines()
            .filter(|file| tracked.contains(*file))
        {
            checked += 1;
            if !action.outputs.iter().any(|o| covers(o, file)) {
                uncovered.push(format!("{file} (step {step})"));
            }
        }
    }
    assert!(
        checked >= 1000,
        "only {checked} committed generated file(s) were found in the steps' \
         trees — the scan is broken, not the tree"
    );
    assert!(
        uncovered.is_empty(),
        "{} committed generated file(s) a step may remove are outside every \
         output of that step's action in bx.toml, so removing one would refuse \
         the action's whole result:\n  {}",
        uncovered.len(),
        uncovered.join("\n  ")
    );
}

/// No regen action reads or writes a path another regen action writes.
///
/// bx refuses two actions that write the same path, and runs an action that
/// reads what another writes after it, in one chain: on one machine, with no
/// result kept (bx RFC 0029, section 4.8). The steps are declared apart so
/// that each is cached and spread on its own, so a meeting here is either a
/// read-after-write that belongs in one step, or a pattern drawn wider than
/// what it reads. Turns red if an input over a whole directory reaches into
/// another step's outputs (`tests/forge/**` would reach the goldens), or if a
/// language step's output is widened over the hand-written files the generator
/// compiles in.
#[test]
fn no_regen_action_reads_or_writes_what_another_writes() {
    let actions = bx_actions();
    let mut meetings = Vec::new();
    for (writer, w) in &actions {
        for (other, o) in &actions {
            if writer == other {
                continue;
            }
            for out in &w.outputs {
                for input in o.inputs.iter().filter(|i| may_overlap(i, out)) {
                    meetings.push(format!(
                        "{other} reads `{input}`, which can be `{out}`, written by {writer}"
                    ));
                }
                if writer < other {
                    for theirs in o.outputs.iter().filter(|t| may_overlap(t, out)) {
                        meetings.push(format!(
                            "{writer} writes `{out}` and {other} writes `{theirs}`, \
                             which can be the same path"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        meetings.is_empty(),
        "{} meeting(s) between the regen actions of bx.toml:\n  {}",
        meetings.len(),
        meetings.join("\n  ")
    );
}
