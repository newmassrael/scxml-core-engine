// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A deployment's facts are told apart from the document's by syntax.
//!
//! A mesh deployment
//! decides what a `<send>` actually does, the pseudocode rendered from
//! the document alone cannot say which, and so a reviewer approves
//! behaviour the deployment then changes. The resolution is to render
//! the deployment's facts and mark them derived — and the mark must be
//! **syntactic**, because if "this part is derived" is something the
//! reader is expected to notice, then the reverse converter's refusal
//! and the reviewer's understanding can disagree and nothing catches
//! it.
//!
//! Three properties make the mark syntactic rather than typographic,
//! and this file is all three:
//!
//! 1. **No authored rendering can produce a derived line.** Held to the
//!    whole corpus, not to an argument about the grammar.
//! 2. **Removing the derived lines yields the authored rendering, byte
//!    for byte.** A deployment adds and never alters, so a reviewer's
//!    approval of the authored half does not depend on whether the
//!    deployment was shown.
//! 3. **The reader refuses a deployed rendering**, naming the line,
//!    rather than dropping what it does not understand.
//!
//! ⚠ Property 2 is nearly true by construction — the renderer writes
//! derived lines and never edits others — and it is here anyway,
//! because the two ways it fails are both real and neither is visible
//! at the call site: a fact whose value carries a newline splits into
//! lines that no longer begin with the sigil, and a fact emitted at the
//! wrong depth re-parents the line that follows it. Both are caught by
//! comparing the stripped text against the authored one and by nothing
//! else.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::forge::pseudo::{self, Deployment, Fact, DEPLOYMENT_SIGIL};
use sce_build::forge::unpseudo;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every `.scxml` in the checkout — the same corpus the round-trip
/// gate sweeps.
///
/// ⚠ This list used to name four directories and, like the two gates
/// beside it, said nothing about the rest while reading as though it
/// had covered everything. The W3C corpus alone is 253 tracked
/// documents that no list here named.
fn fixture_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&repo_root(), &mut out);
    out.sort();
    out
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name()
                .is_some_and(|n| n == "target" || n == ".git" || n == "node_modules")
            {
                continue;
            }
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "scxml") {
            out.push(p);
        }
    }
}

/// Every document the renderer takes, as a model.
fn documents() -> Vec<(String, sce_build::forge::model::ForgeDocument)> {
    let mut out = Vec::new();
    for path in fixture_files() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture")
            .to_string();
        let Ok(expanded) =
            sce_build::parser::expand_preprocessors(&text, &stem, path.parent(), &[])
        else {
            continue;
        };
        let label = DocumentLabel {
            identifier: &stem,
            diagnostic_label: &stem,
        };
        // Both pipelines: the forge entry point answers `Ok(None)` for a
        // statechart, and a statechart is the only kind a deployment
        // applies to, so routing through it alone would sweep every
        // document except the ones this file is about.
        let document = match sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label)
        {
            Ok(Some(p)) => p.document,
            Ok(None) => {
                match sce_build::parser::SCXMLParser::new().parse_string(&expanded.0, &stem) {
                    Ok(model) => {
                        sce_build::forge::model::ForgeDocument::Statechart(Box::new(model))
                    }
                    Err(_) => continue,
                }
            }
            Err(_) => continue,
        };
        out.push((stem, document));
    }
    out
}

/// Every line of `rendered` that the deployment wrote.
fn derived_lines(rendered: &str) -> Vec<&str> {
    rendered
        .lines()
        .filter(|l| l.split_whitespace().next() == Some(DEPLOYMENT_SIGIL))
        .collect()
}

/// `rendered` with every derived line removed.
fn strip_derived(rendered: &str) -> String {
    let mut out = String::new();
    for l in rendered
        .lines()
        .filter(|l| l.split_whitespace().next() != Some(DEPLOYMENT_SIGIL))
    {
        out.push_str(l);
        out.push('\n');
    }
    out
}

/// Property 1: the sigil cannot come out of a document.
///
/// ⚠ The check is on the FIRST word, which is the same test the reader
/// and the stripper apply. A weaker `contains` would pass while a line
/// merely mentioning the sigil somewhere in an expression existed, and
/// a stronger `starts_with` on the untrimmed line would miss a derived
/// line at depth. All three sites ask the one question.
#[test]
fn no_authored_rendering_writes_a_derived_line() {
    let docs = documents();
    assert!(!docs.is_empty(), "no fixture found — the sweep is vacuous");

    let mut rendered_count = 0usize;
    let mut offenders: Vec<String> = Vec::new();
    for (stem, doc) in &docs {
        let Ok(rendered) = pseudo::render(doc) else {
            continue;
        };
        rendered_count += 1;
        for l in derived_lines(&rendered) {
            offenders.push(format!("{stem}: {l}"));
        }
    }

    println!("documents rendered: {rendered_count}");
    assert!(
        rendered_count > 0,
        "nothing rendered — the sweep is vacuous, not clean"
    );
    assert!(
        offenders.is_empty(),
        "these authored lines begin with `{DEPLOYMENT_SIGIL}`, so the sigil no \
         longer tells the document's text from a deployment's:\n{}",
        offenders.join("\n")
    );
}

/// A deployment that names every target the document sends to, with a
/// value chosen to be hostile to the line grammar.
///
/// Built from the rendering rather than from a hand-written list, so
/// the sweep covers whatever targets the corpus actually holds.
fn hostile_deployment_for(rendered: &str) -> Deployment {
    let mut targets: BTreeMap<String, Vec<Fact>> = BTreeMap::new();
    for line in rendered.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("to ") {
            // The rendered value is encoded; the key is the raw one, so
            // it has to be decoded back to what the document wrote.
            let raw = sce_build::comment_text::decode(rest.trim())
                .expect("the renderer wrote it, so it decodes");
            targets.entry(raw).or_default().extend([
                Fact::new("transport", "zenoh"),
                // A value carrying a line terminator is the failure
                // property 2 exists for: unencoded, it would split into
                // a second line that does not begin with the sigil and
                // would survive the strip.
                Fact::new("key", "sce/brake\nmotor/cmd"),
            ]);
        }
    }
    Deployment {
        machine: vec![Fact::new("device", "ecu1"), Fact::new("mode", "peer")],
        targets,
        // An injected send is the hardest case property 2 has: it is
        // the only derived block that spans several lines, so it is the
        // only one where a line can lose the sigil in the middle and
        // survive the strip as if the author had written it. Given
        // enough clauses to be a block rather than one line, and a
        // value carrying a newline for the same reason as `key` above.
        injected: vec![sce_build::forge::pseudo::InjectedSend {
            state: "s0".to_string(),
            site: "on exit".to_string(),
            action: sce_build::model::Action {
                action_type: "send".to_string(),
                event: "event.unsubscribe.brake\nstatus".to_string(),
                target: "#motor".to_string(),
                delay: "50ms".to_string(),
                ..Default::default()
            },
        }],
    }
}

/// Property 2, and property 3 on the same renderings.
#[test]
fn a_deployment_adds_lines_and_alters_none() {
    let docs = documents();
    assert!(!docs.is_empty(), "no fixture found — the sweep is vacuous");

    let mut with_facts = 0usize;
    let mut with_targets = 0usize;
    let mut broken: Vec<String> = Vec::new();

    for (stem, doc) in &docs {
        if !matches!(doc, sce_build::forge::model::ForgeDocument::Statechart(_)) {
            continue;
        }
        let Ok(authored) = pseudo::render(doc) else {
            continue;
        };
        let deployment = hostile_deployment_for(&authored);
        let annotated_targets = !deployment.targets.is_empty();
        let deployed = match pseudo::render_with_deployment(doc, &deployment) {
            Ok(r) => r,
            Err(e) => {
                broken.push(format!("{stem}: the deployed rendering was refused — {e}"));
                continue;
            }
        };

        with_facts += 1;
        if annotated_targets {
            with_targets += 1;
        }

        if strip_derived(&deployed) != authored {
            broken.push(format!(
                "{stem}: the deployment altered the document's own lines\n  \
                 authored:\n{authored}\n  stripped:\n{}",
                strip_derived(&deployed)
            ));
        }

        // Property 3. The reader takes the authored text and refuses the
        // deployed one — both halves, because a reader that refused
        // everything would satisfy the second on its own.
        if unpseudo::covers(doc) && unpseudo::parse(&authored).is_err() {
            broken.push(format!("{stem}: the reader refused the AUTHORED rendering"));
        }
        match unpseudo::parse(&deployed) {
            Ok(_) => broken.push(format!(
                "{stem}: the reader read a deployed rendering back as a document, \
                 so it silently dropped lines no document could produce"
            )),
            Err(e) => {
                let expected = deployed
                    .lines()
                    .position(|l| l.split_whitespace().next() == Some(DEPLOYMENT_SIGIL))
                    .map(|i| i + 1);
                if Some(e.line) != expected {
                    broken.push(format!(
                        "{stem}: the refusal names line {} and the first derived \
                         line is {expected:?}",
                        e.line
                    ));
                }
            }
        }
    }

    println!("statecharts given a deployment: {with_facts}");
    println!("of those, with a send target   : {with_targets}");

    // Two floors. The first catches a sweep that annotated nothing; the
    // second catches one where every deployment was machine-level, which
    // would leave `annotate_target` — the half this whole section is
    // about — unexercised while the count still looked healthy.
    assert!(
        with_facts > 0,
        "no statechart was given a deployment — the sweep is vacuous"
    );
    assert!(
        with_targets > 0,
        "no fixture in the corpus sends to a static target, so the \
         per-target annotation was never written"
    );
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}

/// A deployment handed to a kind that cannot carry one is refused.
///
/// The quiet alternative is what this pins: rendering the document and
/// dropping the facts would hand back a text that looks complete and is
/// missing exactly what the caller asked to see.
#[test]
fn a_non_statechart_refuses_a_deployment() {
    let docs = documents();
    let deployment = Deployment {
        machine: vec![Fact::new("device", "ecu1")],
        targets: BTreeMap::new(),
        injected: Vec::new(),
    };

    let mut checked = 0usize;
    let mut leaked: Vec<String> = Vec::new();
    for (stem, doc) in &docs {
        if matches!(doc, sce_build::forge::model::ForgeDocument::Statechart(_)) {
            continue;
        }
        if pseudo::render(doc).is_err() {
            continue;
        }
        checked += 1;
        match pseudo::render_with_deployment(doc, &deployment) {
            Ok(_) => leaked.push(stem.clone()),
            Err(e) => {
                assert_eq!(
                    e.kind,
                    doc.kind().as_attr(),
                    "{stem}: the refusal names a kind the document is not"
                );
            }
        }
        // An EMPTY deployment is not a caller error, and this is the
        // half that keeps the refusal from being "never takes one".
        assert_eq!(
            pseudo::render_with_deployment(doc, &Deployment::default()),
            pseudo::render(doc),
            "{stem}: an empty deployment changed the rendering"
        );
    }

    println!("non-statechart documents checked: {checked}");
    assert!(checked > 0, "no non-statechart rendered — vacuous");
    assert!(
        leaked.is_empty(),
        "these kinds accepted a deployment they cannot carry, so the facts \
         were dropped without a word: {leaked:?}"
    );
}
