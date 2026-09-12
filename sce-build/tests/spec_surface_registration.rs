// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Spec-bearing surface registration.
//
// `SCE_WIRE_CONTRACTS.md` is the single registry of this repository's
// surfaces. Its wire half is guarded by `wire_surface_stability.rs`,
// which walks `schemas/` and `apis/` so a schema cannot land unregistered.
// Nothing walked the other half: the artifacts that carry the
// SPECIFICATION model — which sections exist, which code and which tests
// are bound to them, which fixtures the conformance suites run, and what
// the vendored upstream said.
//
// What that cost is measured rather than imagined. On 2026-09-13 this
// repository spent a day designing a requirement-trace table that
// `docs/spec/scxml/.atomic/` had been running for months. The search
// vocabulary did not overlap — `requirement`, `provenance`, `trace` and
// `coverage` occur zero times in a catalogue that says `section_ids` 199
// times — and no entry document named the stores, so a complete third
// column was invisible to the session rebuilding it.
//
// The repair is registration with a gate, because a rule would be
// forgotten the same way. This file derives the population FROM THE TREE
// and fails on anything the registry does not name.
//
// Two derivations, deliberately independent:
//
//   1. A shape walk over `git ls-files`. Four rules, each asserted
//      non-vacuous, so a rule that stops matching fails rather than
//      quietly narrowing the population.
//   2. A walk of every tracked `mnemosyne.toml`. Every artifact path a
//      workspace declares must be both derived by rule 1 and registered.
//      This is what stops rule 1 certifying itself: a workspace that
//      moves its store out of a `.atomic/` directory would slip past the
//      shape rule and is caught here.
//
// ⚠ The population is DERIVED, never listed. A hand-written list is how
// the defect this file exists for is produced: measured the same day in a
// sibling gate, a hand-listed set covered 6 of 16 trees and stayed green
// while misclassifying 2000+ files as production.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Repo root = the crate manifest dir's parent (`sce-build/..`).
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// The registry every surface must appear in. One document, by its own
/// declaration — a second register would be the defect, not the repair.
const REGISTRY: &str = "SCE_WIRE_CONTRACTS.md";

/// Tracked paths, as the tree itself answers.
///
/// `git ls-files` rather than a filesystem walk: the question is which
/// files are IN this repository, and a walk would have to be told about
/// `build/`, `target/` and every other untracked tree by a list — the
/// exact thing this file refuses to write.
fn tracked_paths() -> Vec<String> {
    let root = repo_root();
    let out = std::process::Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files"])
        .output()
        .expect("git ls-files runs");
    assert!(
        out.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&out.stderr),
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A candidate under a `fixtures/` DIRECTORY is one member of a corpus
/// some test owns; a surface names a population.
///
/// The discriminator is the directory segment, not the filename:
/// `tests/w3c/conformance/fixtures.json` is a registry and is registered,
/// while `sce-build/tests/fixtures/**/*.manifest.json` is an input to one
/// test and is not.
///
/// ⚠ What this costs is stated in `SCE_WIRE_CONTRACTS.md` rather than
/// hidden: a genuine surface parked under a `fixtures/` directory would be
/// excluded and nothing would notice. `the_exclusion_never_swallows_more_
/// than_it_leaves` is the fence, and the repair if it ever fires is to
/// move the surface, not to widen this.
fn is_test_fixture(path: &str) -> bool {
    dir_segments(path).any(|s| s == "fixtures")
}

/// Path segments excluding the file name.
fn dir_segments(path: &str) -> impl Iterator<Item = &str> {
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts.into_iter()
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The name families a registry-shaped JSON belongs to.
///
/// Not an enumeration of today's files — a vocabulary. An artifact named
/// for one of these is claiming to be a catalogue of something, and that
/// claim is what makes it a surface.
const NAME_FAMILIES: &[&str] = &[
    "catalog",
    "reference",
    "manifest",
    "provenance",
    "verification",
];

/// Which shape rule admitted a path, or `None` if it is not a surface.
///
/// Each arm is a STRUCTURAL claim about the artifact, which is why the
/// rule generalises to a file that does not exist yet:
///
/// - `.atomic/` — a store owned by mnemosyne, whatever its extension; the
///   `epub` content SSOT lives in one too.
/// - `conformance/` — a directory whose name declares it holds the registry
///   of what a conformance suite runs. Narrowed to JSON because the same
///   segment also names generated runner trees and template directories in
///   six languages, which are code.
/// - `spec-snapshot/` — a vendored upstream specification and its provenance.
/// - a registry name family — a JSON claiming to catalogue something.
fn surface_rule(path: &str) -> Option<&'static str> {
    if is_test_fixture(path) {
        return None;
    }
    let name = file_name(path);
    let is_json = name.ends_with(".json");
    if dir_segments(path).any(|s| s == ".atomic") {
        return Some("atomic-store");
    }
    if is_json && dir_segments(path).any(|s| s == "conformance") {
        return Some("conformance-registry");
    }
    if dir_segments(path).any(|s| s == "spec-snapshot") {
        return Some("vendored-spec-snapshot");
    }
    if is_json {
        let lower = name.to_ascii_lowercase();
        if NAME_FAMILIES.iter().any(|fam| lower.contains(fam)) {
            return Some("registry-name-family");
        }
    }
    None
}

/// Every rule must be shown selecting something. A rule that matches
/// nothing is either stale or wrong, and either way it is not doing the
/// work the population claims it does.
const RULES: &[&str] = &[
    "atomic-store",
    "conformance-registry",
    "vendored-spec-snapshot",
    "registry-name-family",
];

/// Lower bound on surfaces the walk must observe.
///
/// Measured 2026-09-13: 19 tracked paths are derived, of which 18 are
/// spec-bearing and one (`schemas/sce-manifest.v1.schema.json`) is a wire
/// surface the name-family rule also reaches — it is registered either
/// way, so no arm needs to special-case it.
///
/// The floor exists because every per-path assertion below passes
/// vacuously over an empty list. A walk that finds nothing certifies
/// nothing, and `git ls-files` returning empty is a real failure mode on a
/// tree checked out without an index.
const MIN_DERIVED_SURFACES: usize = 15;

/// Format owners a row may declare. A closed vocabulary, checked as a
/// whole table cell so a stray `SCE` inside a prose cell cannot satisfy it.
const FORMAT_OWNERS: &[&str] = &["| SCE |", "| mnemosyne |", "| W3C |"];

fn derive(tracked: &[String]) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = tracked
        .iter()
        .filter_map(|p| surface_rule(p).map(|r| (p.clone(), r)))
        .collect();
    out.sort();
    out
}

/// The registry names this artifact.
///
/// Substring against the whole document rather than a table parse: the
/// question is whether a reader searching for the path finds it here,
/// which is the failure this gate exists for.
fn registry_names(registry: &str, path: &str) -> bool {
    registry.contains(path)
}

fn unregistered(registry: &str, surfaces: &[(String, &'static str)]) -> Vec<String> {
    surfaces
        .iter()
        .filter(|(p, _)| !registry_names(registry, p))
        .map(|(p, _)| p.clone())
        .collect()
}

#[test]
fn every_spec_bearing_surface_in_the_tree_is_registered() {
    let tracked = tracked_paths();
    let surfaces = derive(&tracked);
    let registry = read(REGISTRY);

    // Printed, not merely counted: a reviewer reading this gate's output
    // should be able to see WHICH population it judged, because the number
    // alone cannot distinguish a narrowed rule from a shrunken tree.
    println!(
        "spec-surface walk: {} tracked path(s) -> {} surface(s)",
        tracked.len(),
        surfaces.len(),
    );
    for (path, rule) in &surfaces {
        println!("  [{rule}] {path}");
    }

    let missing = unregistered(&registry, &surfaces);
    assert!(
        missing.is_empty(),
        "{} surface(s) in the tree that {REGISTRY} does not name:\n  {}\n\
         Each carries part of this repository's specification model and is \
         invisible to anyone who does not already know the path. Add a row \
         to the table in {REGISTRY} — do not start a second register, which \
         is the defect this gate exists to prevent.",
        missing.len(),
        missing.join("\n  "),
    );

    assert!(
        surfaces.len() >= MIN_DERIVED_SURFACES,
        "the walk derived only {} surface(s), below the {MIN_DERIVED_SURFACES} \
         on record. Either the shape rules stopped selecting or the tree \
         stopped being read — both leave every assertion here passing over an \
         empty list.",
        surfaces.len(),
    );
}

#[test]
fn every_shape_rule_selects_something() {
    let surfaces = derive(&tracked_paths());
    for rule in RULES {
        let n = surfaces.iter().filter(|(_, r)| r == rule).count();
        assert!(
            n > 0,
            "shape rule `{rule}` selected nothing. A rule that matches no \
             path is either stale or wrong, and either way the population is \
             narrower than this file claims. Fix the rule or delete it — \
             leaving it is how a derivation decays into a hand list that \
             happens to be spelled as globs.",
        );
    }
    let fired: BTreeSet<&str> = surfaces.iter().map(|(_, r)| *r).collect();
    assert_eq!(
        fired.len(),
        RULES.len(),
        "rules that fired: {fired:?}; rules declared: {RULES:?}",
    );
}

#[test]
fn every_registered_surface_row_names_its_format_owner() {
    let surfaces = derive(&tracked_paths());
    let registry = read(REGISTRY);
    let mut checked = 0usize;

    for (path, _) in &surfaces {
        // Any line naming the path, not the first: a path may also be named
        // in prose, and the row is the line that also carries an owner cell.
        let mentions: Vec<&str> = registry
            .lines()
            .filter(|l| l.contains(path.as_str()))
            .collect();
        assert!(
            !mentions.is_empty(),
            "{REGISTRY} names no row for {path} — \
             every_spec_bearing_surface_in_the_tree_is_registered reports \
             this first",
        );
        assert!(
            mentions
                .iter()
                .any(|row| FORMAT_OWNERS.iter().any(|owner| row.contains(owner))),
            "no {REGISTRY} row for {path} declares a format owner. One of \
             {FORMAT_OWNERS:?} must be a cell of the row. The distinction is \
             the point of registering these at all: a mnemosyne-owned store \
             is recorded as existing and as holding what it holds, NOT as a \
             format SCE has taken over.\n  lines naming it: {mentions:#?}",
        );
        checked += 1;
    }

    assert!(
        checked >= MIN_DERIVED_SURFACES,
        "only {checked} row(s) checked for a format owner, below the \
         {MIN_DERIVED_SURFACES} on record",
    );
}

/// Every workspace config's declared artifacts are derived AND registered.
///
/// The second derivation, and the reason there are two. The shape walk's
/// `.atomic/` rule is a claim about where mnemosyne puts a store; this
/// walk asks the configs themselves, so the day a workspace declares a
/// store somewhere else, the shape rule's silence becomes a failure here
/// instead of a surface nobody sees.
#[test]
fn every_workspace_config_declares_a_registered_store() {
    let tracked = tracked_paths();
    let derived: BTreeSet<String> = derive(&tracked).into_iter().map(|(p, _)| p).collect();
    let registry = read(REGISTRY);

    let configs: Vec<&String> = tracked
        .iter()
        .filter(|p| file_name(p) == "mnemosyne.toml")
        .collect();

    let mut declared = 0usize;
    for config in &configs {
        for artifact in declared_artifacts(&read(config)) {
            declared += 1;
            assert!(
                derived.contains(&artifact),
                "{config} declares the artifact {artifact}, which the shape \
                 walk in this file does not select. The two derivations exist \
                 to disagree exactly here: a store the configs know about and \
                 the shape rules cannot see is a surface that would land \
                 unregistered and silent. Widen the rule — do not add the path \
                 to a list.",
            );
            assert!(
                registry_names(&registry, &artifact),
                "{config} declares the artifact {artifact} and {REGISTRY} does \
                 not name it.",
            );
        }
    }

    assert!(
        configs.len() >= 5,
        "found only {} mnemosyne.toml config(s); five workspaces are on \
         record (scxml, synth, mesh, wire, bytesguard), so a smaller number \
         means this walk stopped reading the tree",
        configs.len(),
    );
    assert!(
        declared >= configs.len(),
        "{declared} artifact declaration(s) across {} config(s) — every \
         workspace declares at least an `[atomic].sidecar_path`, so a \
         shortfall means the extractor stopped matching and this walk is \
         certifying nothing",
        configs.len(),
    );
}

/// Repo-relative artifact paths a `mnemosyne.toml` declares.
///
/// Read as assignments rather than parsed as TOML: the three keys that
/// name a file are unambiguous across the workspace configs, and taking a
/// TOML dependency into a test that exists to read three strings would buy
/// nothing the floor assertions above do not already buy.
fn declared_artifacts(config: &str) -> Vec<String> {
    const KEYS: &[&str] = &["sidecar_path", "path", "epub_path"];
    let mut out = Vec::new();
    for line in config.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((lhs, rhs)) = line.split_once('=') else {
            continue;
        };
        if !KEYS.contains(&lhs.trim()) {
            continue;
        }
        let value = rhs.trim().trim_matches('"');
        // `path` is also spelled by keys that name a directory or a
        // non-artifact; an artifact declaration is a file that exists.
        if value.is_empty() || !repo_root().join(value).is_file() {
            continue;
        }
        out.push(value.to_string());
    }
    out
}

/// The refusal fires on a surface that is not there.
///
/// A guard whose failure path is never exercised is a guard nobody has
/// seen work. This one fabricates the case the gate exists for — a store
/// added under a new workspace, which is exactly how the eight existing
/// ones arrived — and asserts both halves: that the shape walk SELECTS it,
/// and that the registry check REFUSES it.
///
/// Both halves matter. A rule that stopped selecting would leave the
/// refusal untested while the test still passed on the registered
/// population, which is the shape of a premise that is true while its
/// conclusion is false.
///
/// ⚠ Judged DIFFERENTIALLY — what the fabricated path adds to the refusal
/// set, not what that set contains. The first draft asserted the whole
/// set, and a live demonstration on 2026-09-13 showed what that buys: with
/// a real unregistered surface in the tree, this case failed for the other
/// file's reason and said nothing about its own subject. A synthetic case
/// that cannot fail alone is not a control.
#[test]
fn an_unregistered_surface_is_refused() {
    let invented = "docs/spec/_fabricated_by_this_test/.atomic/workspace.atomic.json".to_string();
    let registry = read(REGISTRY);
    assert!(
        !registry_names(&registry, &invented),
        "the fabricated path is registered, so this case proves nothing about \
         the refusal — pick one the registry does not name",
    );

    let tree = tracked_paths();
    let before = unregistered(&registry, &derive(&tree));

    let mut with_invented = tree;
    with_invented.push(invented.clone());
    let surfaces = derive(&with_invented);
    assert!(
        surfaces
            .iter()
            .any(|(p, r)| *p == invented && *r == "atomic-store"),
        "the shape walk did not select the fabricated store, so the refusal \
         below would never have been reached",
    );

    let after = unregistered(&registry, &surfaces);
    let added: Vec<&String> = after.iter().filter(|p| !before.contains(p)).collect();
    assert_eq!(
        added,
        vec![&invented],
        "adding one unregistered surface must add exactly it to the refusal \
         set. Nothing added means the refusal does not fire at all.",
    );
}

/// A second fabricated case, on the rule that does not depend on a
/// directory name.
///
/// The `.atomic/` case above shares a rule with eight registered paths, so
/// it could keep passing on a tree where the name-family rule had rotted.
/// This one exercises that arm on its own.
#[test]
fn an_unregistered_catalog_is_refused() {
    let invented = "tools/_fabricated_by_this_test_section_catalog.json".to_string();
    let registry = read(REGISTRY);
    assert!(!registry_names(&registry, &invented));

    let surfaces = derive(std::slice::from_ref(&invented));
    assert_eq!(
        surfaces,
        vec![(invented.clone(), "registry-name-family")],
        "a JSON claiming to catalogue something must be selected by the \
         name-family rule",
    );
    assert_eq!(unregistered(&registry, &surfaces), vec![invented]);
}

/// The fixture exclusion never swallows more than it leaves.
///
/// The exclusion is the one hand-shaped judgement in this file, and a
/// hand-shaped exclusion is what produced the defect measured on
/// 2026-09-13 in a sibling gate: a set that covered 6 of 16 trees, green
/// the whole time. A ratio is a weaker fence than a derivation, and it is
/// the honest one available — it would have caught 10 of 16.
#[test]
fn the_exclusion_never_swallows_more_than_it_leaves() {
    let tracked = tracked_paths();
    let kept = derive(&tracked).len();

    // What the rules would have admitted without the exclusion.
    let swallowed: Vec<&String> = tracked
        .iter()
        .filter(|p| is_test_fixture(p))
        .filter(|p| {
            let stripped: String = p
                .split('/')
                .filter(|s| *s != "fixtures")
                .collect::<Vec<_>>()
                .join("/");
            surface_rule(&stripped).is_some()
        })
        .collect();

    println!(
        "fixture exclusion: {} candidate(s) excluded, {kept} surface(s) kept",
        swallowed.len(),
    );
    for p in &swallowed {
        println!("  excluded {p}");
    }
    assert!(
        swallowed.len() < kept,
        "the fixture exclusion removed {} candidate(s) and left {kept}. An \
         exclusion that takes more than it leaves is not a carve-out, it is \
         the population — re-derive it before trusting anything this file \
         reports.",
        swallowed.len(),
    );
}

/// The registry still says of itself that it is the only one.
///
/// Every refusal message above tells the reader to add a row rather than
/// start a second register, and that instruction is only correct while the
/// document makes the claim. If the sentence goes, these messages are
/// sending people somewhere that no longer exists.
#[test]
fn the_registry_still_claims_to_be_the_single_one() {
    let registry = read(REGISTRY);
    assert!(
        registry.contains("the single registry"),
        "{REGISTRY} no longer calls itself the single registry, which is the \
         claim every refusal in this file rests on",
    );
    assert!(
        registry.contains("spec_surface_registration.rs"),
        "{REGISTRY} must name the gate that holds it to the tree, so a reader \
         deciding how much to trust the table can find what enforces it",
    );
}
