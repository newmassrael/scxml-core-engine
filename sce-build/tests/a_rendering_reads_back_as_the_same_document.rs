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

fn fixture_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(&repo_root().join("tests/forge/resources"), &mut out);
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
        let Ok(Some(parsed)) =
            sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label)
        else {
            continue;
        };
        if !unpseudo::covers(&parsed.document) {
            continue;
        }

        let rendered = match sce_build::forge::pseudo::render(&parsed.document) {
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

        let before = unpseudo::ir_for_comparison(&parsed.document).expect("a model serialises");
        let after = unpseudo::ir_for_comparison(&read_back).expect("a model serialises");
        checked += 1;
        // The reader's own namer, not a copy of it here: two spellings
        // of "which kind is this" would let this gate report a kind the
        // reader does not take.
        kinds_seen.extend(unpseudo::covered_kind(&parsed.document));
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
