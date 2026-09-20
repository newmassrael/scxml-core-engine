// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every line that opens a block names the word that opened it.
//!
//! This is the condition a second shape rests on. `indent` writes
//! nesting as leading whitespace and never needs to know what a line
//! says; a shape that closes a block — `endmark`, braces, anything with
//! an end marker — has to write the opener's word again at the close,
//! and it can only do that if the node carries it.
//!
//! [`Node::raw`] exists because the renderer was decomposed one kind at
//! a time, and a raw node is a line whose words this module has not
//! been taught. A raw node in a LEAF is harmless: no shape needs to say
//! anything about it. A raw node that OPENS a block is the thing this
//! case refuses.
//!
//! # ⚠ Derived from the corpus, not from a list
//!
//! The population is every document the parser takes, and whether a
//! node opens a block is read off the page's own shape — the next node
//! is deeper — rather than from a marker the renderer sets. Both halves
//! are deliberate. A hand-listed set of "the openers" would be a second
//! copy of the renderer's structure, and this session has already
//! measured what a hand-listed scope does: five gates carried one, and
//! each was silent about exactly the documents nobody had thought of.
//!
//! So a renderer that grows a new block tomorrow is measured by this
//! case on the day it is written, without anybody remembering to come
//! back here.

use std::path::{Path, PathBuf};

use sce_build::forge::page::{Node, Part};
use sce_build::forge::pseudo;
use sce_build::DocumentLabel;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every `.scxml` in the checkout — the scope the gates in this family
/// converged on, for the reason the module note gives.
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

/// Whether this node's block is opened by it — read off the page.
fn opens_a_block(nodes: &[Node], i: usize) -> bool {
    nodes
        .get(i + 1)
        .is_some_and(|next| next.depth > nodes[i].depth)
}

/// A node whose first part is a word can be closed by name.
fn carries_a_word(node: &Node) -> bool {
    matches!(node.parts.first(), Some(Part::Word(_)))
}

#[test]
fn every_line_that_opens_a_block_carries_its_word() {
    let files = fixture_files();
    assert!(
        !files.is_empty(),
        "no document found — the sweep is vacuous"
    );

    let mut documents = 0usize;
    let mut openers = 0usize;
    let mut lines = 0usize;
    let mut lines_with_a_word = 0usize;
    let mut wordless: Vec<String> = Vec::new();

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
        // ⚠ BOTH pipelines. The forge entry point answers `Ok(None)` for
        // a statechart, and a statechart is the kind with the most
        // blocks in it, so routing through one alone would leave the
        // largest half of this question unasked.
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
        for (i, node) in nodes.iter().enumerate() {
            lines += 1;
            if node.parts.iter().any(|p| matches!(p, Part::Word(_))) {
                lines_with_a_word += 1;
            }
            if !opens_a_block(&nodes, i) {
                continue;
            }
            openers += 1;
            if !carries_a_word(node) {
                wordless.push(format!(
                    "{stem}: `{}` opens a block and names no word",
                    render_for_report(node)
                ));
            }
        }
    }

    println!("documents rendered   : {documents}");
    println!("lines opening a block: {openers}");
    println!("of those, wordless   : {}", wordless.len());
    println!("lines on those pages : {lines}");
    println!("of those, with a word: {lines_with_a_word}");

    // Two floors. The first catches a sweep that rendered nothing; the
    // second catches a page shape that stopped having blocks at all,
    // which would make the case pass by measuring nothing.
    assert!(
        documents > 0,
        "no document rendered — the sweep is vacuous, not clean"
    );
    assert!(
        openers > 0,
        "no line opens a block anywhere in the corpus, so this case is \
         green about nothing"
    );

    // ⚠ A FIGURE, not a ceiling — and the difference is the point. A
    // wordless line is not a defect on its own: `red` under an enum is
    // one value and nothing else, and a shape has nothing to say about
    // it. What the figure answers is how far a LEXICON reaches, which
    // no other case asks: a page whose lines are mostly text is a page
    // that reads almost the same in every language while every test
    // here stays green. The floor below is the only part that can be
    // asserted without inventing an allowlist of legitimately wordless
    // lines — which is the hand-listed scope this family keeps finding.
    assert!(
        lines_with_a_word > 0,
        "not one line on any page carries a word, so no lexicon reaches \
         the corpus at all"
    );

    assert!(
        wordless.is_empty(),
        "these lines open a block without naming the word that opened \
         it, so a shape that closes blocks cannot write the close:\n  {}",
        wordless.join("\n  ")
    );
}

/// The line as text, for a failure message only.
fn render_for_report(node: &Node) -> String {
    node.parts
        .iter()
        .map(|p| match p {
            Part::Word(w) => format!("{w:?}"),
            Part::Text(t) | Part::Glued(t) => t.clone(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
