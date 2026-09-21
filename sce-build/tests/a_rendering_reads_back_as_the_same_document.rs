// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The round trip: a rendering reads back as the document it came from.
//!
//! This is the law `rfc-pseudocode-review-surface.md` §4 states, and it
//! is what turns the renderer's totality from a claim into evidence.
//! Until it runs, "every field reaches the output" is something the
//! whole-output tests assert about the cases somebody thought to write;
//! here the corpus decides.
//!
//! # The comparison
//!
//! Models, not bytes, and the WHOLE model — `crate::forge::unpseudo::
//! ir_for_comparison` serialises the document and strips exactly one
//! key, `source_location`, wherever it occurs. One key by name, not a
//! list of paths: a hand-kept exclusion list is how a real difference
//! gets filed as an expected one.
//!
//! # ⚠ Two layers, and why the statechart gets the weaker one
//!
//! For the seventeen forge kinds the model IS the authored document, so
//! comparing models proves both halves at once: the renderer wrote
//! everything and the reader recovered it.
//!
//! The statechart model is not the document. Of the 376 property names
//! its schema declares, 43 are written by the analyzer — measured, and
//! printed, by `the_analyzer_declares_which_fields_it_writes`; it was
//! 39 until that gate stopped sweeping three directories — and more are
//! computed while parsing, and the
//! renderer deliberately writes only the authored core — so a model
//! read back from a rendering is missing everything derived, and a
//! model comparison would fail on every document for a reason that is
//! not a defect. Excluding those fields by name would be the hand-kept
//! list this file exists to avoid.
//!
//! So the statechart is compared as TEXT:
//! `render(parse(render(m))) == render(m)`. That proves the reader
//! recovers everything the renderer wrote, and nothing about what the
//! renderer left out. The other half is carried by two gates that
//! already stand: `an_action_uses_only_the_fields_its_tag_declares`
//! pins which fields each action tag may use, and
//! `the_analyzer_declares_which_fields_it_writes` pins which fields are
//! SCE's arithmetic rather than the author's. Neither is a substitute
//! for the model comparison; together they are what makes the weaker
//! layer honest rather than convenient.
//!
//! # ⚠ What a failure means, and what it does not
//!
//! A difference is a defect in the PAIR, not a reason to widen the
//! exclusion. Either the renderer dropped something the reader then
//! could not restore, or the reader misread something the renderer
//! wrote correctly. Both are answers about the two halves; neither is
//! answered by comparing less.

use std::path::{Path, PathBuf};

use sce_build::forge::unpseudo;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every `.scxml` in the checkout.
///
/// ⚠ The whole checkout, not a list of fixture roots — the same scope
/// `a_declared_attribute_must_reach_the_ir` had to widen to, and for the
/// same reason: **a hand-listed scope reports about the directories it
/// was pointed at and says nothing about the rest, in a voice that
/// sounds like it covered everything.**
///
/// This list used to read `tests/forge/resources`,
/// `integration_resources` and `examples`, and explained the omission by
/// saying the W3C corpus "is a build artefact of another tree, which a
/// test may not reach into". Measured 2026-09-20: `resources/` holds
/// **253 tracked `.scxml` documents**, committed by
/// `6322123e69 feat: Add TXML to SCXML conversion script and commit
/// converted files`. They are source, not output, and they are where the
/// statecharts live in bulk — so the law was proven over 233 documents
/// while the largest body of machines in the tree went unasked.
///
/// `target/`, `.git/` and `node_modules/` are excluded because they are
/// not documents anybody wrote. Symlinked directories are followed:
/// `resources/403a`, `403b` and `403c` are one W3C fixture reached by
/// three names, and a sweep that does not follow them is a sweep that
/// silently drops it.
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

#[test]
fn every_covered_kind_survives_the_round_trip() {
    let files = fixture_files();
    assert!(!files.is_empty(), "no fixture found — the sweep is vacuous");

    let mut checked = 0usize;
    let mut kinds_seen: std::collections::BTreeSet<&'static str> = Default::default();
    let mut broken: Vec<String> = Vec::new();

    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let Ok(expanded) = sce_build::parser::expand_preprocessors(&text, stem, path.parent(), &[])
        else {
            continue;
        };
        let label = DocumentLabel {
            identifier: stem,
            diagnostic_label: stem,
        };
        // ⚠ BOTH pipelines. The forge entry point answers `Ok(None)`
        // for a statechart, so routing through it alone skips exactly
        // the kind with the largest grammar. The same omission was made
        // once already, in the action-table gate.
        let document = match sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label)
        {
            Ok(Some(p)) => p.document,
            Ok(None) => match sce_build::parser::SCXMLParser::new().parse_string(&expanded.0, stem)
            {
                Ok(model) => sce_build::forge::model::ForgeDocument::Statechart(Box::new(model)),
                Err(_) => continue,
            },
            Err(_) => continue,
        };
        if !unpseudo::covers(&document) {
            continue;
        }
        let is_statechart = matches!(
            document,
            sce_build::forge::model::ForgeDocument::Statechart(_)
        );

        let rendered = match sce_build::forge::pseudo::render(&document) {
            Ok(r) => r,
            Err(e) => {
                broken.push(format!("{stem}: the renderer refused it — {e}"));
                continue;
            }
        };
        let read_back = match unpseudo::parse(&rendered) {
            Ok(d) => d,
            Err(e) => {
                broken.push(format!("{stem}: the reader could not read it — {e}"));
                continue;
            }
        };

        // The statechart is compared as text; see the note at the top
        // of this file for why its model cannot be.
        let (before, after) = if is_statechart {
            let again = match sce_build::forge::pseudo::render(&read_back) {
                Ok(r) => r,
                Err(e) => {
                    broken.push(format!("{stem}: the re-render refused it — {e}"));
                    continue;
                }
            };
            (
                serde_json::Value::String(rendered.clone()),
                serde_json::Value::String(again),
            )
        } else {
            (
                unpseudo::ir_for_comparison(&document).expect("a model serialises"),
                unpseudo::ir_for_comparison(&read_back).expect("a model serialises"),
            )
        };
        checked += 1;
        // The reader's own namer, not a copy of it here: two spellings
        // of "which kind is this" would let this gate report a kind the
        // reader does not take.
        kinds_seen.extend(unpseudo::covered_kind(&document));
        if before != after {
            broken.push(format!(
                "{stem}: the IR moved across the round trip\n  before: {before}\n  after : {after}"
            ));
        }
    }

    println!("documents round-tripped: {checked}");
    println!("kinds covered          : {kinds_seen:?}");
    println!("kinds the reader claims: {:?}", unpseudo::COVERED_KINDS);

    // Two floors, because one is not enough. The first catches a sweep
    // that found no document; the second catches a reader that claims a
    // kind no fixture exercises, which would let that arm rot while the
    // count still looked healthy.
    assert!(
        checked > 0,
        "no document was round-tripped — the sweep is vacuous, not clean"
    );
    let unexercised: Vec<&&str> = unpseudo::COVERED_KINDS
        .iter()
        .filter(|k| !kinds_seen.contains(**k))
        .collect();
    assert!(
        unexercised.is_empty(),
        "the reader claims these kinds and no fixture exercises them, so \
         the claim rests on nothing: {unexercised:?}"
    );

    assert!(
        broken.is_empty(),
        "the rendering and the reader disagree about these documents:\n{}",
        broken.join("\n")
    );
}

/// Every field clause the model can carry survives the round trip —
/// including the values a corpus does not happen to contain.
///
/// # Why the sweep above does not cover this
///
/// The sweep judges the documents that exist. The reader covered
/// `max-size` and `quantity` and silently refused `retain` and
/// `default-covers`, and the sweep said nothing, because no document in
/// the tree used them; the day two documents did, the gate went red for
/// a gap that had been there all along. A corpus answers what authors
/// have written so far, which is not the question of what the model can
/// hold.
///
/// # How this stays honest as the model grows
///
/// `ForgeField` is built here as a struct literal naming EVERY field.
/// Adding one to the model does not silently skip this test — it stops
/// compiling until somebody fills the new field in, and the assertion
/// then makes the renderer and the reader agree about it. That is the
/// difference between a guard and a list: a list of clauses would go
/// stale exactly the way the reader did.
///
/// # The spaces are the point
///
/// Each free-text clause value carries an interior space. A `string`
/// field's `sce:initial` may hold one — `forge::retention::check`
/// refuses nothing a string can hold — and a clause value is not the
/// last thing on its line, so without the escape `pseudo::word` writes,
/// the line reads back as a different field or as none.
#[test]
fn every_field_clause_survives_the_round_trip() {
    use sce_build::forge::model::{
        Direction, ForgeDocument, ForgeField, Retention, SceType, TransformModel,
    };
    use sce_build::forge::quantity::{Quantity, Rational, UnitTag};

    let loaded = ForgeField {
        id: "every_clause".to_string(),
        sce_type: SceType::String,
        direction: Direction::In,
        expr: Some("a + b  /* two spaces stay */".to_string()),
        quantity: Some(Quantity {
            scale: Rational::new(1, 10).expect("a legal rational"),
            offset: Rational::new(-5, 1).expect("a legal rational"),
            unit: UnitTag::intern("degrees per second"),
        }),
        max_size: Some(64),
        // ⚠ Invented, and deliberately so. A `default-covers` list in
        // the wild names the variants of somebody's enum, and copying a
        // real one in would put that vocabulary in this tree. Two
        // members is what the clause needs to prove it is read as a
        // list rather than a single word; what they spell is nothing.
        default_covers: vec!["FIRST_VARIANT".to_string(), "SECOND_VARIANT".to_string()],
        retain: Some(Retention {
            scope: "battery backed store".to_string(),
            initial: "hello world".to_string(),
        }),
    };

    let document = ForgeDocument::Transform(TransformModel {
        name: "EveryClause".to_string(),
        inputs: vec![loaded],
        outputs: Vec::new(),
        source_location: None,
    });

    let rendered =
        sce_build::forge::pseudo::render(&document).expect("the renderer covers a transform");
    let read_back = unpseudo::parse(&rendered).unwrap_or_else(|e| {
        panic!("the reader refused a rendering of its own renderer:\n{rendered}\n{e:?}")
    });

    assert_eq!(
        unpseudo::ir_for_comparison(&document).expect("a model serialises"),
        unpseudo::ir_for_comparison(&read_back).expect("a model serialises"),
        "a clause was lost between the renderer and the reader:\n{rendered}"
    );
}
