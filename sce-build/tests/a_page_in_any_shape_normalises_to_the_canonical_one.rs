// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A page written in any shape and any lexicon comes back as the
//! canonical one.
//!
//! This is what makes a shape SELECTABLE rather than merely writable.
//! A reviewer approves the page they were shown; for that approval to
//! reach the document, the page has to be readable — and the reader
//! reads one grammar, `indent` with `EN`. So each shape owes a way
//! back to that, and this is where the debt is collected:
//!
//! ```text
//! normalise_S,L( write_S,L(nodes) )  ==  canonical(nodes)
//! ```
//!
//! byte for byte, over every document the parser takes.
//!
//! # ⚠ Why the population is derived twice over
//!
//! The documents come from the checkout, and the pairs come from
//! `SHAPES` × `LEXICONS` — neither is a list written here. A shape or
//! a lexicon added to the registry is measured by this case on the day
//! it is registered, without anybody remembering to come back. That is
//! the correction this session made five times in one round: a
//! hand-listed scope does not stay silent about what it omits, it
//! reports that the omitted part is fine.
//!
//! # ⚠ What a failure means
//!
//! Not that the document is wrong. It means the pair is: either the
//! shape wrote something its own normaliser cannot undo, or the
//! lexicon spells two things the same way once they are on a page.
//! Both are answers about the pair, and neither is answered by
//! measuring fewer documents.

use std::path::{Path, PathBuf};

use sce_build::forge::page::{canonical, LEXICONS, SHAPES};
use sce_build::forge::pseudo;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

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
fn a_page_in_any_shape_normalises_to_the_canonical_one() {
    let files = fixture_files();
    assert!(
        !files.is_empty(),
        "no document found — the sweep is vacuous"
    );

    let mut documents = 0usize;
    let mut pages = 0usize;
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
        // ⚠ BOTH pipelines; the forge entry point answers `Ok(None)`
        // for a statechart, the kind with the most nesting in it.
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
        let Ok(nodes) = pseudo::render_nodes(&document, &pseudo::Deployment::default()) else {
            continue;
        };

        documents += 1;
        let want = canonical(&nodes);
        for shape in SHAPES {
            for lexicon in LEXICONS {
                let page = match shape.write(&nodes, lexicon) {
                    Ok(p) => p,
                    Err(e) => {
                        broken.push(format!(
                            "{stem}: {} refused to write it — {e}",
                            shape.name()
                        ));
                        continue;
                    }
                };
                let back = match shape.normalise(&page, lexicon) {
                    Ok(b) => b,
                    Err(e) => {
                        broken.push(format!(
                            "{stem}: {} × {} wrote a page it could not read back — {e}",
                            shape.name(),
                            lexicon.name
                        ));
                        continue;
                    }
                };
                pages += 1;
                if back != want {
                    broken.push(format!(
                        "{stem}: {} × {} does not normalise to the canonical page",
                        shape.name(),
                        lexicon.name
                    ));
                }
            }
        }
    }

    println!("documents rendered: {documents}");
    println!("shapes             : {}", SHAPES.len());
    println!("lexicons           : {}", LEXICONS.len());
    println!("pages normalised   : {pages}");
    println!("of those, broken   : {}", broken.len());

    // Three floors. The first two catch a sweep that measured nothing;
    // the third catches a registry that lost its second pair, which
    // would leave this case green while proving only that the default
    // is the default.
    assert!(
        documents > 0,
        "no document rendered — the sweep is vacuous, not clean"
    );
    assert!(
        SHAPES.len() > 1 && LEXICONS.len() > 1,
        "one shape and one lexicon prove nothing about selecting either"
    );
    assert!(pages > 0, "no page was normalised, so nothing was measured");

    assert!(
        broken.is_empty(),
        "these pages cannot be read back into the canonical one, so the \
         approval they carry does not reach the document:\n  {}",
        broken.join("\n  ")
    );
}
