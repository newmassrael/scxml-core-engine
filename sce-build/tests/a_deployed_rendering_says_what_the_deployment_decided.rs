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
