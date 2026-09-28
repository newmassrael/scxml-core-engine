// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An `sce:` attribute on a W3C element that nothing reads is refused, in
//! both pipelines, rather than dropped.
//!
//! # What was wrong
//!
//! `schemas/sce-forge.xsd` takes the attributes of a W3C element as
//! `<xs:anyAttribute namespace="##other" processContents="lax"/>`, and `lax`
//! validates only an attribute some global declaration names. So an `sce:`
//! attribute nobody reads — an invented name, or a real one on an element
//! that does not take it — built with exit 0. Measured 2026-09-28:
//! `sce:unresolved` on a `<data>` and on an `<onentry>`, `sce:req` on a
//! forge root, and `sce:totally-invented` on a `<data>`.
//!
//! # What is held
//!
//! * Every document this repository commits that parses still parses: the
//!   rule refuses nothing the tree writes, so every attribute the tree
//!   writes has a reader (or a reader that leaves it unread on purpose,
//!   `sce_attr::acknowledge`).
//! * For every W3C element name the committed documents use, an invented
//!   `sce:` attribute written on such an element, in documents that parse,
//!   is refused as `validation/sce-attribute-unread` on the row that holds
//!   it. The documents are the population, so an element is covered by a
//!   document using it, not by a list here.
//!
//! ⚠ What the second half leaves out, and why: an element inside a value
//! the parser carries as written — `<content>`, `<data>` and `<assign>`
//! children (§scxml-5.4, §scxml-5.5, §scxml-5.6) — is the value's, judged by
//! whoever reads the value; and an element inside an SCE element is that
//! element's reader's. Neither is this parse's claim.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::parser::SCXMLParser;
use sce_build::{DocumentLabel, Pipeline};

const SCXML_NAMESPACE: &str = "http://www.w3.org/2005/07/scxml";
const SCE_NAMESPACE: &str = "http://sce.dev/ext";
const UNREAD: &str = "\"validation/sce-attribute-unread\"";
const UNKNOWN_NAME: &str = "\"validation/unknown-sce-attribute\"";

/// How many documents carry each element name into the second half —
/// enough that one document's peculiarity does not decide an element, few
/// enough that the sweep stays a sweep.
const CARRIERS_PER_ELEMENT: usize = 3;

type Refusal = (String, Option<u32>, String);

/// Parse `text` through the pipeline it routes to; the first record's code,
/// line and message on a refusal.
fn parse(text: &str, label: &str) -> Result<(), Refusal> {
    let outcome = match sce_build::classify_document(text) {
        Pipeline::Forge => sce_build::forge::parser::parse_forge_with_imports(
            text,
            DocumentLabel::symmetric(label),
        )
        .map(|_| ()),
        _ => SCXMLParser::new().parse_string(text, label).map(|_| ()),
    };
    outcome.map_err(|refusal| {
        let record = refusal.error.to_diagnostics().remove(0);
        (
            serde_json::to_string(&record.code).expect("a code serialises"),
            refusal.location.line,
            record.message,
        )
    })
}

/// Parse the committed file at `path` the way the CLI does: expanded first.
fn parse_committed(path: &Path) -> Result<(), Refusal> {
    let text = std::fs::read_to_string(path).expect("a committed document reads");
    let label = path.display().to_string();
    let expanded = sce_build::parser::expand_preprocessors(&text, &label, path.parent(), &[])
        .map(|(expanded, _, _)| expanded)
        .map_err(|refusal| {
            let record = refusal.error.to_diagnostics().remove(0);
            (
                serde_json::to_string(&record.code).expect("a code serialises"),
                refusal.location.line,
                record.message,
            )
        })?;
    parse(&expanded, &label)
}

fn documents() -> Vec<PathBuf> {
    common::repository::files_git_tracks(&["*.scxml"])
}

#[test]
fn no_document_this_tree_commits_carries_an_sce_attribute_nothing_reads() {
    let documents = documents();
    assert!(
        documents.len() > 600,
        "found only {} committed SCXML documents; an empty sweep passes for the wrong reason",
        documents.len()
    );
    let mut parsed = 0usize;
    let mut refused = Vec::new();
    for path in &documents {
        match parse_committed(path) {
            Ok(()) => parsed += 1,
            Err((code, line, message)) if code == UNREAD => refused.push(format!(
                "  {}:{}: {message}",
                path.display(),
                line.unwrap_or(0)
            )),
            // Refused for another reason — a refusal fixture, or a document
            // that needs what only its build supplies. Not this rule's.
            Err(_) => {}
        }
    }
    // A floor, so a parse that stopped working everywhere cannot pass as
    // "nothing refused by this rule".
    assert!(
        parsed > 400,
        "only {parsed} committed document(s) parse at all"
    );
    assert!(
        refused.is_empty(),
        "the tree commits {} document(s) carrying an sce: attribute nothing reads:\n{}",
        refused.len(),
        refused.join("\n")
    );
}

/// A name the tree knows, on an element no reader asks it of, is refused by
/// the placement rule in a forge document too: `sce:window` means a
/// monitor's window on a monitor's `<data>`, and nothing on a codec's
/// `<datamodel>`.
#[test]
fn a_known_name_on_an_element_that_does_not_take_it_is_refused_in_a_forge_document() {
    let path = common::repository::root().join("tests/forge/resources/codec_cbor_map.scxml");
    let text = std::fs::read_to_string(&path).expect("the fixture reads");
    assert_eq!(
        parse(&text, "codec_cbor_map"),
        Ok(()),
        "the fixture parses as written"
    );
    let at = text
        .find("<datamodel>")
        .expect("the fixture has a datamodel");
    let placed = format!(
        "{}<datamodel sce:window=\"4\">{}",
        &text[..at],
        &text[at + "<datamodel>".len()..]
    );
    let row = text[..at].matches('\n').count() as u32 + 1;
    let (code, line, message) =
        parse(&placed, "codec_cbor_map").expect_err("a misplaced sce:window is refused");
    assert_eq!((code.as_str(), line), (UNREAD, Some(row)), "{message}");
    assert!(message.contains("sce:window on <datamodel>"), "{message}");
}

/// Whether an element under `node` is this parse's to judge — outside every
/// carried value and every SCE element (the module header says why).
fn judged_by_this_parse(node: &roxmltree::Node) -> bool {
    node.ancestors()
        .skip(1)
        .filter(|a| a.is_element())
        .all(|a| {
            a.tag_name().namespace() != Some(SCE_NAMESPACE)
                && !matches!(a.tag_name().name(), "content" | "data" | "assign")
        })
}

/// `text` with an invented `sce:` attribute on `element`, and the row that
/// holds it. Spelled with the prefix the document already binds to the SCE
/// namespace; where it binds none, with a binding of its own on the element
/// under a prefix the document does not use.
fn with_invented_attribute(text: &str, element: &roxmltree::Node) -> (String, u32) {
    let start = element.range().start;
    let name_end = start
        + 1
        + text[start + 1..]
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .expect("a start tag ends");
    let invented = match element.lookup_prefix(SCE_NAMESPACE) {
        Some(prefix) => format!(r#" {prefix}:totally-invented="x""#),
        None => format!(r#" xmlns:invented="{SCE_NAMESPACE}" invented:totally-invented="x""#),
    };
    let mut out = String::with_capacity(text.len() + invented.len());
    out.push_str(&text[..name_end]);
    out.push_str(&invented);
    out.push_str(&text[name_end..]);
    let row = text[..start].matches('\n').count() as u32 + 1;
    (out, row)
}

#[test]
fn an_invented_sce_attribute_on_any_w3c_element_is_refused_where_it_is_written() {
    // Carriers: documents that parse as written, with no expansion to shift
    // their rows, for each W3C element name they use.
    let mut carriers: BTreeMap<String, Vec<(PathBuf, usize)>> = BTreeMap::new();
    for path in documents() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(document) = roxmltree::Document::parse(&text) else {
            continue;
        };
        let label = path.display().to_string();
        let mut offered: BTreeSet<&str> = BTreeSet::new();
        for node in document.descendants().filter(|n| n.is_element()) {
            let name = node.tag_name().name();
            if node.tag_name().namespace() != Some(SCXML_NAMESPACE)
                || !judged_by_this_parse(&node)
                || offered.contains(name)
                || carriers
                    .get(name)
                    .is_some_and(|c| c.len() >= CARRIERS_PER_ELEMENT)
            {
                continue;
            }
            if offered.is_empty() && parse(&text, &label).is_err() {
                break;
            }
            offered.insert(name);
            carriers
                .entry(name.to_string())
                .or_default()
                .push((path.clone(), node.range().start));
        }
    }

    let mut wrong = Vec::new();
    for (name, documents) in &carriers {
        for (path, start) in documents {
            let text = std::fs::read_to_string(path).expect("a carrier reads");
            let document = roxmltree::Document::parse(&text).expect("a carrier parses");
            let element = document
                .descendants()
                .find(|n| n.is_element() && n.range().start == *start)
                .expect("the carrier's element");
            let (invented, row) = with_invented_attribute(&text, &element);
            match parse(&invented, &path.display().to_string()) {
                Err((code, Some(line), _)) if code == UNREAD && line == row => {}
                // A forge document is first held to the names the tree
                // knows at all (`KNOWN_SCE_ATTRS`), which refuses an
                // invented one on the same row before the placement rule is
                // reached. Either refusal is the one this case asks for;
                // the case above holds the placement rule itself on forge
                // content, with a name the tree does know.
                Err((code, Some(line), _))
                    if code == UNKNOWN_NAME
                        && line == row
                        && sce_build::classify_document(&text) == Pipeline::Forge => {}
                other => wrong.push(format!("  <{name}> in {}:{row}: {other:?}", path.display())),
            }
        }
    }

    // Floors: the committed documents use far more W3C elements than this,
    // and a sweep that found few is one that stopped looking.
    assert!(
        carriers.len() >= 20,
        "only {} W3C element name(s) found a carrier: {:?}",
        carriers.len(),
        carriers.keys().collect::<Vec<_>>()
    );
    assert!(
        wrong.is_empty(),
        "an invented sce: attribute was not refused where it was written:\n{}",
        wrong.join("\n")
    );
}
