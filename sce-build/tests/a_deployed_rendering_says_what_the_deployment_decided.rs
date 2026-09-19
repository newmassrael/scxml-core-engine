// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The facts a reviewer is shown are the ones codegen was given.
//!
//! `a_deployment_annotation_is_not_the_document` holds the SYNTAX — a
//! derived line is told apart from the document's by its first token,
//! and stripping those lines gives the document back. That says
//! nothing about whether the facts are true, and a review surface
//! whose annotations are plausible and wrong is worse than one with no
//! annotations at all, because it converts an unreviewed binding into
//! an approved one.
//!
//! So this file sweeps the mesh fixtures — every `deploy.yaml` in
//! `tests/mesh` against the machines it names — and asks two things of
//! each rendering:
//!
//! 1. **Every target the deployment resolved is annotated**, and with
//!    the transport `deploy.yaml` actually names for it. The expected
//!    value is read out of the yaml here rather than out of the
//!    pipeline, so the two sides of the comparison do not come from
//!    one place.
//! 2. **Every field of `ResolvedTarget` reaches the page.** The
//!    builder destructures without `..`, so the compiler catches a new
//!    field; what the compiler cannot catch is a field taken apart and
//!    then dropped, or one whose serde shape skips it. This checks the
//!    names that must appear whatever the deployment settled.
//!
//! ⚠ The second check is why the fixtures matter more than a
//! hand-built `ResolvedTarget` would: the first shape of the builder
//! serialised the struct and flattened it, and five fields carrying
//! `skip_serializing_if` vanished whenever they were empty. A
//! synthetic target with every field populated would have passed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sce_build::forge::model::ForgeDocument;
use sce_build::forge::pseudo::{self, DEPLOYMENT_SIGIL};
use sce_build::generator::Language;
use sce_build::mesh::review;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every field of `ResolvedTarget` owes the page a line whose name
/// starts with one of these, whatever the deployment settled.
///
/// `target` is deliberately absent: it is the key the annotation is
/// printed under. Everything else must be visible — an empty list says
/// `(none)` rather than saying nothing.
const OWED: &[&str] = &[
    "transport",
    "events",
    "event-pattern",
    "subscribes",
    "invoke",
    "ordering",
    "responders",
    "retry",
    "auth",
    "pool",
];

/// Each `deploy.yaml` under `tests/mesh`, paired with a machine it
/// deploys and that machine's document.
fn deployed_machines() -> Vec<(String, PathBuf, PathBuf)> {
    let dir = repo_root().join("tests/mesh");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let p = e.path();
        if !p.file_name().is_some_and(|n| {
            n.to_str()
                .is_some_and(|n| n.starts_with("deploy") && n.ends_with(".yaml"))
        }) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        // `source: <file>.scxml` is how a machine names its document.
        // Read straight out of the yaml rather than through the deploy
        // parser: this side of the comparison has to be independent of
        // the pipeline the other side runs.
        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("source:") else {
                continue;
            };
            let src = rest.trim().trim_matches(['"', '\''].as_slice());
            let doc = dir.join(src);
            if doc.is_file() {
                out.push((src.to_string(), doc, p.clone()));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The transports `deploy.yaml` names, read from its text.
///
/// Deliberately naive — `transport: <name>` wherever it appears — so
/// the expectation comes from the file a human wrote and not from the
/// resolver whose answer is under test.
fn transports_named_in(yaml: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(yaml) else {
        return BTreeSet::new();
    };
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("transport:"))
        .map(|v| v.trim().trim_matches(['"', '\''].as_slice()).to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

fn parse_statechart(path: &Path) -> Option<ForgeDocument> {
    let text = std::fs::read_to_string(path).ok()?;
    let stem = path.file_stem()?.to_str()?;
    let (expanded, _, _) =
        sce_build::parser::expand_preprocessors(&text, stem, path.parent(), &[]).ok()?;
    let model = sce_build::parser::SCXMLParser::new()
        .parse_string(&expanded, stem)
        .ok()?;
    Some(ForgeDocument::Statechart(Box::new(model)))
}

#[test]
fn a_deployed_rendering_carries_the_resolution_codegen_was_given() {
    let cases = deployed_machines();
    assert!(
        !cases.is_empty(),
        "no deploy.yaml named a machine — the sweep is vacuous"
    );

    let mut rendered = 0usize;
    let mut with_targets = 0usize;
    let mut transports_seen: BTreeSet<String> = BTreeSet::new();
    let mut broken: Vec<String> = Vec::new();

    for (src, doc_path, yaml) in &cases {
        let Some(doc) = parse_statechart(doc_path) else {
            continue;
        };
        let ForgeDocument::Statechart(model) = &doc else {
            continue;
        };
        // A fixture whose deployment this tree rejects is not this
        // file's subject: the pipeline's own suites judge those. Only a
        // resolution that succeeded can be checked against the page.
        let Ok(deployment) = review::deployment_for(model, yaml, Language::Cpp) else {
            continue;
        };
        let Ok(text) = pseudo::render_with_deployment(&doc, &deployment) else {
            continue;
        };
        rendered += 1;
        let case = format!("{src} + {}", yaml.file_name().unwrap().to_string_lossy());

        if deployment.targets.is_empty() {
            continue;
        }
        with_targets += 1;

        // Property 1: every fact the builder produced is on the page.
        //
        // ⚠ Not "under a `to` clause". That was this check's first
        // shape and it was wrong in both directions: a deployment can
        // bind a target the document never sends to (a `deploy.yaml`
        // `subscriptions:` entry does exactly that in eleven fixtures
        // here), and a target the document sends to from three places
        // is annotated three times. What is owed is that each fact
        // appears at least once, wherever the renderer decided it
        // belongs.
        for (target, facts) in &deployment.targets {
            for f in facts {
                let line = format!("{DEPLOYMENT_SIGIL} {} {}", f.name, f.value);
                if !text.lines().any(|l| l.trim() == line.trim()) {
                    broken.push(format!(
                        "{case}: the deployment settled `{}` = `{}` for `{target}` \
                         and no line on the page says so",
                        f.name, f.value
                    ));
                }
            }
        }

        // The transport on the page must be one the yaml names. Not
        // "the" transport: one deploy.yaml can bind several targets
        // differently, and asserting the set is what this file can
        // check without becoming a second resolver.
        let named = transports_named_in(yaml);
        for facts in deployment.targets.values() {
            // The fact named exactly `transport` is the one the builder
            // takes from `TransportState::transport_name`; the
            // `transport.*` facts beneath it are that transport's
            // parameters and are not transport names.
            for f in facts.iter().filter(|f| f.name == "transport") {
                transports_seen.insert(f.value.clone());
                if !named.contains(&f.value) {
                    broken.push(format!(
                        "{case}: the page says transport `{}` and the yaml names {named:?}",
                        f.value
                    ));
                }
            }
            // Property 2: every field of `ResolvedTarget` owes a line.
            for owed in OWED {
                if !facts
                    .iter()
                    .any(|f| f.name == *owed || f.name.starts_with(&format!("{owed}.")))
                {
                    broken.push(format!(
                        "{case}: nothing on the page answers `{owed}`, so a reviewer \
                         cannot tell an empty setting from a missing one"
                    ));
                }
            }
        }

        // And the other direction: no derived line the builder did not
        // produce. Without this, a renderer that invented a plausible
        // binding would satisfy every check above.
        //
        // `bound-without-a-send` is the renderer's own head line for a
        // target no clause claimed, so it is expected and named rather
        // than excluded by a pattern that would also hide a mistake.
        let produced: BTreeSet<String> = deployment
            .machine
            .iter()
            .chain(deployment.targets.values().flatten())
            .map(|f| format!("{DEPLOYMENT_SIGIL} {} {}", f.name, f.value))
            .chain(
                deployment
                    .targets
                    .keys()
                    .map(|t| format!("{DEPLOYMENT_SIGIL} bound-without-a-send {t}")),
            )
            .collect();
        for l in text
            .lines()
            .map(str::trim)
            .filter(|l| l.split_whitespace().next() == Some(DEPLOYMENT_SIGIL))
        {
            if !produced.contains(l) {
                broken.push(format!(
                    "{case}: the page carries a derived line the deployment never \
                     settled: `{l}`"
                ));
            }
        }
    }

    println!("machine + deploy.yaml pairs rendered: {rendered}");
    println!("of those, with a resolved target    : {with_targets}");
    println!("transports exercised                : {transports_seen:?}");

    assert!(
        rendered > 0,
        "no fixture rendered under a deployment — the sweep is vacuous, not clean"
    );
    assert!(
        with_targets > 0,
        "no deployment resolved a target, so the per-target annotation was \
         never checked against a real resolution"
    );
    // A floor on the transports, not just on the count: a sweep that
    // only ever saw `local` would say nothing about the variants that
    // carry keys, topics and service ids — which are the facts a
    // reviewer most needs and the ones a renderer is most likely to
    // drop.
    assert!(
        transports_seen.len() >= 2,
        "only {transports_seen:?} was exercised; one transport cannot show \
         that the facts follow the binding"
    );
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}

/// Every type that can hide a field from the page is taken apart by
/// hand.
///
/// `mesh::review` destructures three types — `ResolvedTarget`,
/// `TransportState`, `EventPatternInfo` — because each carries a
/// `skip_serializing_if`, and a skipped field prints nothing at all on
/// a surface whose contract is that it prints everything. Everything
/// below those goes through serde, which is sound exactly while nothing
/// below them skips.
///
/// That was a sentence in a comment, and a sentence cannot notice a
/// fourth type acquiring a skip. This reads the declarations instead.
///
/// ⚠ Comments are stripped first. Measuring this by hand, an occurrence
/// inside the prose `No `skip_serializing_if`: the field always
/// serializes` was counted as a real one and made `MeshRpcInvokeSite`
/// look like a hole it is not. A scanner that reads comments is
/// measuring the wrong text — the tree's own standing note.
#[test]
fn nothing_the_builder_leaves_to_serde_can_skip_a_field() {
    // Both files, because reachability crosses them:
    // `ResolvedTarget::retry` is a `deploy::RetryPolicyConfig`.
    let sources = [
        repo_root().join("sce-build/src/mesh/topology.rs"),
        repo_root().join("sce-build/src/mesh/deploy.rs"),
    ];
    let mut declared: std::collections::BTreeMap<String, TypeDecl> = Default::default();
    for src in &sources {
        let text = std::fs::read_to_string(src)
            .unwrap_or_else(|e| panic!("{} is unreadable: {e}", src.display()));
        read_declarations(&text, &mut declared);
    }
    assert!(
        declared.contains_key("ResolvedTarget"),
        "the reader found no `ResolvedTarget` declaration, so the walk below \
         starts nowhere and would pass on an empty set"
    );

    // Taken apart by name in `mesh::review`, so a skip inside one of
    // these is already answered — and their FIELDS are still walked,
    // because destructuring a type says nothing about what its fields
    // hand to serde.
    const DESTRUCTURED: &[&str] = &[
        "ResolvedTarget",
        "TransportState",
        "EventPatternInfo",
        "AuthPolicyConfig",
    ];

    // Reachable from `ResolvedTarget` by field position. Derived
    // rather than listed: a list of "the types the review surface
    // touches" is a list that stops being true, and `BindingDefaultIds`
    // is the other half of the same point — it carries five skips and
    // is an INPUT to resolution, so a scan of the whole file reports it
    // and a scan of what the surface actually serialises does not.
    let mut reached: BTreeSet<String> = BTreeSet::new();
    let mut queue = vec!["ResolvedTarget".to_string()];
    while let Some(name) = queue.pop() {
        if !reached.insert(name.clone()) {
            continue;
        }
        if let Some(decl) = declared.get(&name) {
            for f in &decl.field_types {
                if declared.contains_key(f) {
                    queue.push(f.clone());
                }
            }
        }
    }

    let offenders: Vec<String> = reached
        .iter()
        .filter(|n| !DESTRUCTURED.contains(&n.as_str()))
        .filter_map(|n| {
            let decl = declared.get(n)?;
            (decl.skips > 0).then(|| format!("{n}: {} field(s) skip", decl.skips))
        })
        .collect();

    let total_skips: usize = declared.values().map(|d| d.skips).sum();
    println!("types declared         : {}", declared.len());
    println!("reachable from ResolvedTarget: {}", reached.len());
    println!("skip_serializing_if in those files: {total_skips}");

    // Three floors. Without the first a reader that matched nothing
    // would pass loudest; without the second a walk that reached only
    // its own root would too; without the third a reader that stopped
    // recognising the attribute would look like a tree that has none.
    assert!(
        declared.len() > 20,
        "the declaration reader found almost nothing"
    );
    assert!(
        reached.len() > 5,
        "the walk reached {} types from ResolvedTarget, which is fewer than it \
         has fields — the field-type reader stopped working",
        reached.len()
    );
    assert!(
        total_skips > 0,
        "no `skip_serializing_if` was read at all, so this scan is measuring \
         nothing"
    );
    assert!(
        offenders.is_empty(),
        "these types are reachable from `ResolvedTarget`, are not taken apart \
         in `mesh::review`, and skip a field when it is empty — so that field \
         vanishes from the review surface with nothing said:\n{}",
        offenders.join("\n")
    );
}

/// What one `struct`/`enum` declaration says that this check needs.
#[derive(Default)]
struct TypeDecl {
    /// Every identifier appearing in a field's type, so
    /// `Option<RetryPolicyConfig>` and `BTreeMap<String, SomeipEventIds>`
    /// both yield their payload. Over-inclusive on purpose: a name that
    /// is not a declared type is dropped by the walk.
    field_types: Vec<String>,
    /// How many of its fields carry `skip_serializing_if`.
    skips: usize,
}

/// Read the `struct`/`enum` declarations out of one Rust source.
///
/// ⚠ Comments are stripped before anything is counted. Measuring this
/// by hand, the prose "No `skip_serializing_if`: the field always
/// serializes" was counted as a real attribute and made
/// `MeshRpcInvokeSite` look like a hole it is not — the tree's standing
/// note that a scanner must clear comments first, met again.
fn read_declarations(text: &str, out: &mut std::collections::BTreeMap<String, TypeDecl>) {
    let mut current: Option<String> = None;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("//") {
            continue;
        }
        // A declaration at column zero opens a type and `}` at column
        // zero closes it, which is the shape of both files. The floors
        // in the caller are what turn a file that stops looking like
        // that into a red.
        let opened = ["pub struct ", "pub enum ", "struct ", "enum "]
            .iter()
            .find_map(|kw| line.strip_prefix(*kw));
        if let Some(rest) = opened {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                current = Some(name.clone());
                out.entry(name).or_default();
            }
            continue;
        }
        if line.starts_with('}') {
            current = None;
            continue;
        }
        let Some(name) = &current else { continue };
        let entry = out.entry(name.clone()).or_default();
        if t.contains("skip_serializing_if") {
            entry.skips += 1;
        }
        // A field line, or an enum variant's named field. Both are
        // `<name>: <type>`, which is the only form these two files use.
        if let Some((_, ty)) = t.split_once(": ") {
            entry.field_types.extend(
                ty.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            );
        }
    }
}

/// A deployment leaves the caller's model as it found it.
///
/// The pipeline injects sends — SCE_MESH.md §13 auto-symmetry and the
/// Session E response legs — and the rendering must be of what the
/// author wrote. If the builder ran the pipeline on the caller's model
/// instead of a copy, those injected sends would appear as ordinary
/// lines with nothing marking them derived, which is the one thing
/// this whole surface is built to prevent.
#[test]
fn resolving_a_deployment_does_not_touch_the_model() {
    let cases = deployed_machines();
    let mut checked = 0usize;
    let mut injected_anywhere = 0usize;
    let mut broken: Vec<String> = Vec::new();

    for (src, doc_path, yaml) in &cases {
        let Some(doc) = parse_statechart(doc_path) else {
            continue;
        };
        let ForgeDocument::Statechart(model) = &doc else {
            continue;
        };
        let Ok(before) = pseudo::render(&doc) else {
            continue;
        };
        if review::deployment_for(model, yaml, Language::Cpp).is_err() {
            continue;
        }
        checked += 1;
        let Ok(after) = pseudo::render(&doc) else {
            broken.push(format!(
                "{src}: the model stopped rendering after resolution"
            ));
            continue;
        };
        if before != after {
            broken.push(format!(
                "{src}: resolving a deployment changed the model — the rendering \
                 moved from\n{before}\nto\n{after}"
            ));
        }

        // And the pipeline really does inject, somewhere in this
        // corpus. Without this the test above would pass just as well
        // against a pipeline that mutates nothing, and would be
        // guarding a hazard that does not exist.
        let mut scratch = (**model).clone();
        if sce_build::compile_mesh_transport(&mut scratch, yaml, Language::Cpp).is_ok()
            && review::injected_send_count(model, &scratch) > 0
        {
            injected_anywhere += 1;
        }
    }

    println!("models checked for mutation: {checked}");
    println!("fixtures where the pipeline injects a send: {injected_anywhere}");
    assert!(checked > 0, "nothing was resolved — the sweep is vacuous");
    assert!(
        injected_anywhere > 0,
        "no fixture in this corpus makes the pipeline inject a send, so the \
         clone this test defends is guarding nothing — either the corpus lost \
         its auto-symmetry and server fixtures, or the injection stages stopped \
         running"
    );
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}
