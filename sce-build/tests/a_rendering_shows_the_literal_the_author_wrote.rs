// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A number on the review surface is spelled the way the author wrote
//! it.
//!
//! `claudedocs/rfc-pseudocode-review-surface.md` §14 records two losses
//! the round trip could not see: `0x10` renders as `16` and `7.0` as
//! `7`. Neither costs the round trip, because the comparison is at model
//! level and the model never held the text — which is exactly why
//! nothing in the tree noticed. What they cost is the reviewer, who is
//! holding the page against a UDS table or a calibration sheet written
//! in the author's spelling. A surface that exists to be checked against
//! a specification must not quietly restate it.
//!
//! # ⚠ Which token on the page belongs to which attribute
//!
//! The first shape of this file asked whether a token of equal VALUE
//! appeared anywhere in the rendering. That reported `scxml/@version`
//! as respelled: the renderer drops `version="1.0"` as a constant, and
//! an unrelated `1` elsewhere on the page happened to have the same
//! value. Equal value is not the same fact as same origin.
//!
//! So the attribute is MUTATED in the source and the document rendered
//! again. The numeric tokens that disappear are that attribute's, by
//! construction and not by coincidence. Three answers follow:
//!
//! - nothing disappeared → the renderer does not print this attribute,
//!   which is a different question and one the totality gates ask;
//! - the token that disappeared is spelled as the author wrote it →
//!   nothing to report;
//! - it is spelled otherwise → a respelling, named with both spellings.
//!
//! ⚠ The lossy fields are not listed here. Asking the corpus is what
//! makes this catch the loss nobody has met yet; a list of the ones that
//! are known would go green the moment a new carrier was added.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use common::xml_literal::{number, second_value_of_same_shape, Num};
use sce_build::forge::model::ForgeDocument;
use sce_build::forge::pseudo;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

fn fixture_files() -> Vec<PathBuf> {
    let root = repo_root();
    let mut out = Vec::new();
    for sub in ["tests/forge/resources", "integration_resources", "examples"] {
        collect(&root.join(sub), &mut out);
    }
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
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "scxml") {
            out.push(p);
        }
    }
}

/// The document this text parses to, whichever pipeline takes it.
fn document_of(text: &str, stem: &str, dir: Option<&Path>) -> Option<ForgeDocument> {
    let (expanded, _, _) = sce_build::parser::expand_preprocessors(text, stem, dir, &[]).ok()?;
    let label = DocumentLabel {
        identifier: stem,
        diagnostic_label: stem,
    };
    match sce_build::forge::parser::parse_forge_with_imports(&expanded, label) {
        Ok(Some(p)) => Some(p.document),
        Ok(None) => sce_build::parser::SCXMLParser::new()
            .parse_string(&expanded, stem)
            .ok()
            .map(|m| ForgeDocument::Statechart(Box::new(m))),
        Err(_) => None,
    }
}

/// Every numeric token of a rendering, counted.
///
/// A multiset, because one value can legitimately be printed twice and
/// a set would then hide the disappearance of one of them.
fn numeric_tokens(rendered: &str) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for w in rendered.split_whitespace() {
        let w = w.trim_matches(|c: char| matches!(c, ':' | ',' | '(' | ')' | '<' | '>'));
        if number(w).is_some() {
            *out.entry(w.to_string()).or_insert(0) += 1;
        }
    }
    out
}

/// The tokens `before` has that `after` does not, with multiplicity.
fn vanished(before: &BTreeMap<String, usize>, after: &BTreeMap<String, usize>) -> Vec<String> {
    before
        .iter()
        .filter(|(t, n)| **n > after.get(*t).copied().unwrap_or(0))
        .map(|(t, _)| t.clone())
        .collect()
}

/// Of the tokens a mutation removed, the ones that denote the value the
/// author wrote.
///
/// ⚠ A mutation moves more than one token. Changing a `<sce:data
/// bit-offset>` from `5` to `6` also moved a derived value that renders
/// as `0.5`, and the first shape of this check reported *"author wrote
/// `5`, the page says `0.5`"* — a sentence that cannot be true. Equal
/// value is what makes a vanished token a spelling OF this attribute
/// rather than a consequence of changing it.
fn spellings_of(gone: &[String], authored: Num) -> Vec<String> {
    gone.iter()
        .filter(|t| number(t).is_some_and(|n| n.same(authored)))
        .cloned()
        .collect()
}

#[test]
fn a_number_is_rendered_as_the_author_spelled_it() {
    let files = fixture_files();
    assert!(!files.is_empty(), "no fixture found — the sweep is vacuous");

    let mut rendered_count = 0usize;
    let mut numeric_attributes = 0usize;
    let mut probed = 0usize;
    let mut unprintable = 0usize;
    let mut undecided: BTreeSet<String> = BTreeSet::new();
    let mut respelled: BTreeSet<String> = BTreeSet::new();

    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let Some(doc) = document_of(&text, stem, path.parent()) else {
            continue;
        };
        let Ok(rendered) = pseudo::render(&doc) else {
            continue;
        };
        rendered_count += 1;
        let before = numeric_tokens(&rendered);

        // The source as the author wrote it, not the expanded text: a
        // preprocessor's output is not what a reviewer holds against
        // their specification.
        let Ok(src) = roxmltree::Document::parse(&text) else {
            continue;
        };
        for node in src.descendants() {
            for attr in node.attributes() {
                let Some(authored) = number(attr.value()) else {
                    continue;
                };
                numeric_attributes += 1;
                let as_written = attr.value().trim().to_string();
                // Already on the page as written — nothing to ask.
                if before.contains_key(&as_written) {
                    continue;
                }

                let where_ = format!("{}/@{}", node.tag_name().name(), attr.name());
                let mut mutated = text.clone();
                mutated.replace_range(attr.range_value(), &second_value_of_same_shape(&as_written));
                let Some(other_doc) = document_of(&mutated, stem, path.parent()) else {
                    // The mutant does not parse, so this attribute
                    // cannot be probed here. Counted and printed, never
                    // treated as clean.
                    undecided.insert(format!("{where_} — the mutant did not parse"));
                    continue;
                };
                let Ok(other) = pseudo::render(&other_doc) else {
                    undecided.insert(format!("{where_} — the mutant did not render"));
                    continue;
                };
                probed += 1;

                let gone = vanished(&before, &numeric_tokens(&other));
                let spellings = spellings_of(&gone, authored);
                if spellings.is_empty() {
                    // Either nothing moved — the renderer does not print
                    // this attribute, which is where `scxml/@version`
                    // lands and is why this file mutates rather than
                    // matching by value — or what moved was a derived
                    // value of a different number, which is not a
                    // spelling of this one.
                    unprintable += 1;
                    continue;
                }
                for shown in spellings {
                    respelled.insert(format!(
                        "{where_} — author wrote `{as_written}`, the page says `{shown}`"
                    ));
                }
            }
        }
    }

    println!("documents rendered        : {rendered_count}");
    println!("numeric attributes read   : {numeric_attributes}");
    println!("probed by mutation        : {probed}");
    println!("of those, never printed   : {unprintable}");
    println!("undecided (mutant refused): {}", undecided.len());
    for u in &undecided {
        println!("   {u}");
    }
    println!("respelled on the page     : {}", respelled.len());
    for r in &respelled {
        println!("   {r}");
    }

    // Three floors. Nothing rendered, no number found, and no attribute
    // actually probed each turn this file green by doing less rather
    // than by finding less.
    assert!(
        rendered_count > 0,
        "nothing rendered — the sweep is vacuous, not clean"
    );
    assert!(
        numeric_attributes > 20,
        "only {numeric_attributes} numeric attribute(s) across the corpus — the \
         reader stopped recognising numbers"
    );
    assert!(
        probed > 0,
        "no attribute was probed by mutation, so every claim below rests on \
         nothing having been tried"
    );

    // ⚠⚠ A DEBT CEILING, not the contract. The contract is
    // `respelled.is_empty()`, and it does not hold yet. Measured
    // 2026-09-20 it was 43 across six carriers, each a model holding a
    // number where the author wrote a spelling; `VariantArm` now keeps
    // the spelling (`crate::source_literal`) and the rest follow one at
    // a time, each lowering this number. Recorded in
    // `claudedocs/rfc-pseudocode-review-surface.md` §14.
    //
    // What IS enforced meanwhile is the thing that matters most: a
    // carrier NOT on this list cannot join them quietly, and the ones
    // on it cannot get worse. A new carrier that loses the author's
    // spelling reddens this on the first run.
    const KNOWN_LOSSY: &[&str] = &[
        // `arm/@value`, `flag/@value`, `decoded/@value` and
        // `test-vector/@hex` were here and are not: `VariantArm`,
        // `FlagDef`, `DecodedField` and `CodecTestVector` carry the
        // author's spelling now. Each carrier leaves this list as it is
        // fixed, and the ceiling below comes down with it.
        "test-vector/@value",
        "data/@byte",
        "data/@bit-offset",
    ];
    let newcomers: Vec<&String> = respelled
        .iter()
        .filter(|r| !KNOWN_LOSSY.iter().any(|k| r.starts_with(k)))
        .collect();
    assert!(
        newcomers.is_empty(),
        "these values reach the page in a spelling the author did not use, and \
         they are not among the losses already recorded — a reviewer comparing \
         the page against the document they wrote will not find them:\n   {}",
        newcomers
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n   ")
    );
    /// Where the debt stands. Lowered with each carrier that starts
    /// keeping the author's spelling; at zero this whole block goes and
    /// `assert!(respelled.is_empty())` takes its place.
    const CEILING: usize = 3;
    assert!(
        respelled.len() <= CEILING,
        "the recorded losses grew from {CEILING} to {} — a carrier that already \
         respells the author's number is now doing it in more places",
        respelled.len()
    );

    // A drop is not a failure; it is the signal to lower the ceiling.
    // Printed rather than asserted, because a round that fixes a
    // carrier should not have to fix this file before it can see that
    // it worked.
    if respelled.len() < CEILING {
        println!(
            "NOTE: only {} respelling(s) remain of the {CEILING} recorded — lower \
             the ceiling in this file, and delete it at zero",
            respelled.len()
        );
    }
}
