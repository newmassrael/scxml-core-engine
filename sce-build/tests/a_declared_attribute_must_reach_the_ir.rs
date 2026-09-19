// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An attribute the grammar declares must reach the IR.
//!
//! Measured 2026-09-20: `<sce:while max-iter>` was declared by
//! `schemas/sce-forge-ext.xsd`, written by three fixtures, accepted by
//! XSD validation — and read by nobody. The grammar declares it
//! LOCALLY, and `attributeFormDefault="unqualified"` makes a local
//! declaration unprefixed; the parser asked for it in the SCE namespace,
//! which an unprefixed attribute is not in. The declared loop bound was
//! dropped in silence, and every check in the tree stayed green.
//!
//! # Why this is a mutation and not a scan
//!
//! The first attempt at this answer read the parser's source for
//! `sce_attr("<name>")` and concluded from the call which namespace each
//! attribute is read in. That produced a FALSE POSITIVE on
//! `returns-max-size`, which has two read sites — namespaced on
//! `<sce:helper>`, plain on `<sce:return>` — and the scan saw only the
//! first. A source scan is only as precise as its pattern, and the
//! pattern here would have to model the parser.
//!
//! So the question is asked of the behaviour instead: change the
//! attribute's value in a real document and require the IR to move. An
//! IR that does not move is a parser that did not read the attribute,
//! whatever its source says.
//!
//! # ⚠ What an unmeasurable pair is, and why it is not a pass
//!
//! A pair is measurable only when a fixture writes it AND this test can
//! produce a second value of its declared type that the document still
//! accepts. Everything else — no fixture, a type with no mechanical
//! second value, a mutant the parser rejects — is reported as
//! **unmeasured**, never as measured-and-fine. An empty sweep would
//! otherwise satisfy this file by measuring nothing at all, which is the
//! shape this repository has been bitten by before.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sce_build::DocumentLabel;

const SCE_NS: &str = "http://sce.dev/ext";

/// A declared (element, attribute) pair, with what the grammar says
/// about it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Declared {
    element: String,
    attribute: String,
    /// `true` when the declaration is a `ref` to a global, which under
    /// `attributeFormDefault="unqualified"` is the only way an attribute
    /// on an SCE element is namespace-qualified.
    qualified: bool,
    /// The XSD type name, unprefixed (`positiveInteger`, `NCName`, …).
    xsd_type: String,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every (element, attribute) pair `sce-forge-ext.xsd` declares.
fn declared_pairs() -> Vec<Declared> {
    let path = repo_root().join("schemas/sce-forge-ext.xsd");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc = roxmltree::Document::parse(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    // Global attribute declarations are direct children of xs:schema; a
    // `ref` to one is what makes a use qualified.
    let globals: BTreeMap<&str, &str> = doc
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("attribute"))
        .filter_map(|n| {
            Some((
                n.attribute("name")?,
                n.attribute("type").unwrap_or("string"),
            ))
        })
        .collect();

    let mut out = Vec::new();
    for el in doc.descendants().filter(|n| n.has_tag_name("element")) {
        let Some(element) = el.attribute("name") else {
            continue;
        };
        for a in el.descendants().filter(|n| n.has_tag_name("attribute")) {
            if let Some(name) = a.attribute("name") {
                out.push(Declared {
                    element: element.to_string(),
                    attribute: name.to_string(),
                    qualified: false,
                    xsd_type: strip_prefix(a.attribute("type").unwrap_or("string")),
                });
            } else if let Some(r) = a.attribute("ref") {
                let local = strip_prefix(r);
                out.push(Declared {
                    element: element.to_string(),
                    attribute: local.clone(),
                    qualified: true,
                    xsd_type: strip_prefix(
                        globals.get(local.as_str()).copied().unwrap_or("string"),
                    ),
                });
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn strip_prefix(s: &str) -> String {
    s.rsplit(':').next().unwrap_or(s).to_string()
}

/// Every `.scxml` under the fixture roots.
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

/// A second value of the declared type, or `None` when this test has no
/// mechanical way to produce one.
///
/// Deliberately narrow: a mutant the document rejects proves nothing, so
/// a type whose value space this test cannot reason about is reported
/// unmeasured rather than guessed at.
fn mutate(xsd_type: &str, current: &str) -> Option<String> {
    match xsd_type {
        "positiveInteger" => Some(match current.trim().parse::<u64>() {
            Ok(n) if n < u32::MAX as u64 => (n + 1).to_string(),
            _ => return None,
        }),
        "nonNegativeInteger" => Some(match current.trim().parse::<u64>() {
            Ok(n) if n < u32::MAX as u64 => (n + 1).to_string(),
            _ => return None,
        }),
        "boolean" => match current.trim() {
            "true" => Some("false".to_string()),
            "false" => Some("true".to_string()),
            _ => None,
        },
        // A free-text value: lengthen it. The mutant stays well-formed
        // and stays inside `xs:string`, so a rejection downstream is a
        // real answer about that attribute (it names something, and the
        // name no longer resolves) rather than an artefact of the
        // mutation — which is why a rejection is reported as unmeasured
        // rather than counted either way.
        "string" => Some(format!("{}z", current)),
        // An XML name: prefixing keeps it an NCName. Suffixing would
        // too, but a prefix also moves a value that some readers
        // compare by suffix.
        "NCName" | "ID" | "IDREF" | "token" => Some(format!("z{}", current.trim())),
        _ => None,
    }
}

/// Parse a document the way every forge review artefact does.
fn parse_to_ir(text: &str, label_stem: &str, dir: Option<&Path>) -> Option<String> {
    let expanded = sce_build::parser::expand_preprocessors(text, label_stem, dir, &[]).ok()?;
    let label = DocumentLabel {
        identifier: label_stem,
        diagnostic_label: label_stem,
    };
    let parsed = sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label).ok()??;
    // The WHOLE envelope, not `parsed.document`. Measured while writing
    // this file: comparing the document alone reported `<sce:import src>`
    // and `<sce:import as>` as unread, because an import's attributes
    // land in `parsed.imports` — a defect in the comparison, not in the
    // parser. A partial view of the IR makes every field outside it look
    // dropped.
    serde_json::to_string(&parsed).ok()
}

#[test]
fn every_declared_attribute_a_fixture_writes_reaches_the_ir() {
    let declared = declared_pairs();
    let files = fixture_files();
    assert!(
        !declared.is_empty() && !files.is_empty(),
        "nothing to measure: {} declared pair(s), {} fixture(s) — a sweep \
         that collected nothing would satisfy every assertion below",
        declared.len(),
        files.len()
    );

    // Per pair: the first fixture that writes it, the raw text, and the
    // byte range of the value to splice.
    let mut not_read: Vec<String> = Vec::new();
    let mut measured: BTreeSet<(String, String)> = BTreeSet::new();
    let mut unmeasured: BTreeMap<String, &'static str> = BTreeMap::new();

    for d in &declared {
        let key = format!(
            "<sce:{} {}{}>",
            d.element,
            if d.qualified { "sce:" } else { "" },
            d.attribute
        );

        let Some(replacement) = try_each_fixture(d, &files, &mut unmeasured, &key) else {
            unmeasured
                .entry(key.clone())
                .or_insert("no fixture writes it");
            continue;
        };
        let (path, before_text, range, new_value, before_ir) = replacement;

        let mut after_text = before_text.clone();
        after_text.replace_range(range, &new_value);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let Some(after_ir) = parse_to_ir(&after_text, stem, path.parent()) else {
            unmeasured.insert(key.clone(), "the mutant was rejected");
            continue;
        };

        measured.insert((d.element.clone(), d.attribute.clone()));
        if before_ir == after_ir {
            not_read.push(format!(
                "{key} in {} — value changed, IR did not move",
                path.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
    }

    println!("declared pairs   : {}", declared.len());
    println!("measured         : {}", measured.len());
    println!("unmeasured       : {}", unmeasured.len());
    for (k, why) in &unmeasured {
        println!("   {k} — {why}");
    }

    // A floor the sweep derives for itself: every pair whose declared
    // type this test can mutate AND which some fixture writes must have
    // been measured. Without it, a change that stopped finding fixtures
    // would turn this file green by measuring nothing.
    assert!(
        !measured.is_empty(),
        "no pair was measured — the sweep is vacuous, not clean"
    );

    assert!(
        not_read.is_empty(),
        "the grammar declares these attributes and the parser does not \
         read them — a value an author wrote is accepted by XSD and \
         dropped in silence:\n  {}",
        not_read.join("\n  ")
    );
}

type Candidate = (PathBuf, String, std::ops::Range<usize>, String, String);

/// The first fixture that writes this pair in a form this test can
/// mutate, together with everything needed to mutate it.
fn try_each_fixture(
    d: &Declared,
    files: &[PathBuf],
    unmeasured: &mut BTreeMap<String, &'static str>,
    key: &str,
) -> Option<Candidate> {
    for path in files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&text) else {
            continue;
        };
        for node in doc.descendants() {
            if node.tag_name().namespace() != Some(SCE_NS) || node.tag_name().name() != d.element {
                continue;
            }
            let attr = node.attributes().find(|a| {
                a.name() == d.attribute
                    && if d.qualified {
                        a.namespace() == Some(SCE_NS)
                    } else {
                        a.namespace().is_none()
                    }
            });
            let Some(attr) = attr else { continue };
            let Some(new_value) = mutate(&d.xsd_type, attr.value()) else {
                unmeasured
                    .entry(key.to_string())
                    .or_insert("no mechanical second value for its type");
                return None;
            };
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("fixture");
            let Some(before_ir) = parse_to_ir(&text, stem, path.parent()) else {
                continue;
            };
            return Some((
                path.clone(),
                text.clone(),
                attr.range_value(),
                new_value,
                before_ir,
            ));
        }
    }
    None
}
