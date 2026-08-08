// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! Fixture prose must not contradict the attribute it restates.
//!
//! Codec fixtures explain themselves in a leading comment, and those
//! comments quote the numbers the document declares below — "max-depth=4
//! covers PUT's upstream ext slots". When the attribute is later
//! retuned, the sentence keeps the old number and starts describing a
//! document that no longer exists. Nothing reads prose, so the drift
//! survives every other gate: codegen, goldens, and round-trip all pass
//! on the attribute and never look at the sentence.
//!
//! This is not hypothetical. `codec_zenoh_msg_put.scxml` shipped with a
//! comment reading `max-depth=4` above an attribute that this same
//! commit moves to `8`, and the identical stale sentence had already
//! propagated into a downstream consumer that vendors these fixtures —
//! the same wrong sentence in two repositories, because it was copied
//! while it was already wrong.
//!
//! The check is deliberately narrow. It compares only the *prose* form
//! (`attr=N`, no quotes) against the *declared* forms (`attr="N"`) in
//! the same file, so a comment may still discuss any value the document
//! actually declares — a fixture that declares two chain depths can
//! describe either. It does not attempt to parse English.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Attributes whose values fixtures restate in prose.
///
/// Table-driven so covering another attribute is one line. Keep to
/// attributes that carry a bare integer: a value with units or a
/// symbolic form would need its own comparison and would make a
/// mismatch here mean something other than drift.
const PROSE_PINNED_ATTRS: &[&str] = &["max-depth", "max-size"];

/// Number of prose mentions the corpus is known to contain.
///
/// A scanner that silently stops matching reads exactly like a clean
/// corpus: zero violations over zero inputs. This lower bound is the
/// measured count at the time of writing, so a regex that decays, a
/// resource directory that moves, or a glob that stops resolving fails
/// here instead of reporting success over nothing. Raise it when
/// fixtures add mentions; never lower it to make a run pass.
const MIN_PROSE_MENTIONS: usize = 6;

fn resources_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build manifest dir has a parent (workspace root)")
        .join("tests/forge/resources")
}

/// Values the document declares for `attr`, as `attr="N"`.
fn declared_values(body: &str, attr: &str) -> BTreeSet<String> {
    let needle = format!("{attr}=\"");
    let mut out = BTreeSet::new();
    for (idx, _) in body.match_indices(&needle) {
        let rest = &body[idx + needle.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        // `attr="8"` — digits then the closing quote. Anything else
        // (an expression, a symbolic value) is not a bare integer and
        // is not this check's business.
        if !digits.is_empty() && rest[digits.len()..].starts_with('"') {
            out.insert(digits);
        }
    }
    out
}

/// Prose mentions of `attr` — `attr=N` with no quote — as
/// `(1-based line, value)`.
fn prose_mentions(body: &str, attr: &str) -> Vec<(usize, String)> {
    let needle = format!("{attr}=");
    let mut out = Vec::new();
    for (idx, _) in body.match_indices(&needle) {
        let rest = &body[idx + needle.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            // `attr="…"` — the declaration form, not prose.
            continue;
        }
        out.push((body[..idx].matches('\n').count() + 1, digits));
    }
    out
}

fn fixture_paths(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("resources dir must be readable at {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "scxml"))
        .collect();
    // readdir order is not stable across filesystems; sort so a failure
    // list reads the same on every machine.
    paths.sort();
    paths
}

#[test]
fn fixture_prose_never_contradicts_the_attribute_it_restates() {
    let dir = resources_dir();
    let fixtures = fixture_paths(&dir);
    assert!(
        !fixtures.is_empty(),
        "no .scxml fixtures found under {} — the scan resolved nothing",
        dir.display(),
    );

    let mut violations: Vec<String> = Vec::new();
    let mut mentions = 0usize;

    for path in &fixtures {
        let body = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("fixture must be readable at {}: {e}", path.display()));
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unnamed>");

        for attr in PROSE_PINNED_ATTRS {
            let declared = declared_values(&body, attr);
            for (line, value) in prose_mentions(&body, attr) {
                mentions += 1;
                if !declared.contains(&value) {
                    // Collect rather than stop: halting on the first
                    // leaves every later fixture unproven, and the whole
                    // point is to know the corpus is clean.
                    violations.push(format!(
                        "{name}:{line}  prose says {attr}={value}, but the document declares \
                         {}",
                        if declared.is_empty() {
                            "no such attribute at all".to_string()
                        } else {
                            format!("{declared:?}")
                        },
                    ));
                }
            }
        }
    }

    assert!(
        mentions >= MIN_PROSE_MENTIONS,
        "only {mentions} prose mentions scanned across {} fixtures, expected at least \
         {MIN_PROSE_MENTIONS} — the scan is matching less than it used to, so a clean \
         result here proves nothing",
        fixtures.len(),
    );

    assert!(
        violations.is_empty(),
        "fixture prose contradicts the document it describes ({} site(s)):\n  {}",
        violations.len(),
        violations.join("\n  "),
    );
}

/// The comparison must be able to tell the two forms apart. If
/// `prose_mentions` ever started matching the declaration form, every
/// fixture would trivially agree with itself and the gate above would
/// pass on any corpus.
#[test]
fn the_declaration_form_is_not_read_as_prose() {
    let body = r#"<sce:tlv-chain max-depth="8"/> <!-- max-depth=4 in the comment -->"#;
    assert_eq!(
        declared_values(body, "max-depth"),
        BTreeSet::from(["8".to_string()]),
    );
    assert_eq!(
        prose_mentions(body, "max-depth"),
        vec![(1, "4".to_string())],
        "only the unquoted mention is prose",
    );
}

/// The drift this gate exists for, reduced to one line: a comment
/// quoting a value the document does not declare.
#[test]
fn a_prose_value_absent_from_the_document_is_a_violation() {
    let drifted = r#"<!-- max-depth=4 covers it --> <sce:tlv-chain max-depth="8"/>"#;
    let declared = declared_values(drifted, "max-depth");
    let stale: Vec<_> = prose_mentions(drifted, "max-depth")
        .into_iter()
        .filter(|(_, v)| !declared.contains(v))
        .collect();
    assert_eq!(stale, vec![(1, "4".to_string())]);

    let agreeing = r#"<!-- max-depth=8 covers it --> <sce:tlv-chain max-depth="8"/>"#;
    let declared = declared_values(agreeing, "max-depth");
    assert!(prose_mentions(agreeing, "max-depth")
        .into_iter()
        .all(|(_, v)| declared.contains(&v)));
}
