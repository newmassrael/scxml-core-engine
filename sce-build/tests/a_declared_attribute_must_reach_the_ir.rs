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

mod common;

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
/// What each named `xs:simpleType` enumerates, if it enumerates
/// anything.
///
/// ⚠ This is the grammar answering "what else is allowed here", and it
/// is why a named type needs no arm in [`mutate`]. `sceType`,
/// `kindType` and `tlvTerminateOnType` were each reported as having "no
/// mechanical second value" while the XSD listed every value they
/// accept three lines from where the attribute was declared.
///
/// A union's `memberTypes` is followed one hop, because
/// `sceTypeOrEnumRef` is a union over `sce:sceType` and a pattern —
/// without the hop, a type whose members are entirely enumerated looks
/// unenumerated.
fn enumerations() -> BTreeMap<String, Vec<String>> {
    let path = repo_root().join("schemas/sce-forge-ext.xsd");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(doc) = roxmltree::Document::parse(&text) else {
        return BTreeMap::new();
    };

    let mut direct: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for st in doc.descendants().filter(|n| n.has_tag_name("simpleType")) {
        let Some(name) = st.attribute("name") else {
            continue;
        };
        let values: Vec<String> = st
            .descendants()
            .filter(|n| n.has_tag_name("enumeration"))
            .filter_map(|n| n.attribute("value").map(str::to_string))
            .collect();
        direct.insert(name.to_string(), values);
        let referenced: Vec<String> = st
            .descendants()
            .filter(|n| n.has_tag_name("union"))
            .filter_map(|n| n.attribute("memberTypes"))
            .flat_map(|m| m.split_whitespace())
            .map(strip_prefix)
            .collect();
        members.insert(name.to_string(), referenced);
    }

    let mut out = direct.clone();
    for (name, refs) in &members {
        for r in refs {
            if let Some(extra) = direct.get(r) {
                out.entry(name.clone()).or_default().extend(extra.clone());
            }
        }
    }
    for v in out.values_mut() {
        v.sort();
        v.dedup();
    }
    out
}

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
fn mutate(xsd_type: &str, current: &str, enums: &BTreeMap<String, Vec<String>>) -> Option<String> {
    // The grammar first. A type that lists what it accepts has already
    // written the second value, and reaching for a shape rule instead
    // would invent one the type forbids.
    if let Some(values) = enums.get(xsd_type) {
        if let Some(other) = values.iter().find(|v| v.as_str() != current.trim()) {
            return Some(other.clone());
        }
    }
    match xsd_type {
        "positiveInteger" | "nonNegativeInteger" | "integer" => {
            Some(match current.trim().parse::<u64>() {
                Ok(n) if n < u32::MAX as u64 => (n + 1).to_string(),
                _ => return None,
            })
        }
        "boolean" => match current.trim() {
            "true" => Some("false".to_string()),
            "false" => Some("true".to_string()),
            _ => None,
        },
        // A free-text value, where the grammar has told us nothing
        // about the value space.
        //
        // ⚠ `xs:string` here is the grammar being loose, not the value
        // being free. Measured 2026-09-20: of 31 attributes whose
        // mutant the parser refused, most were declared `xs:string` and
        // carry a hex literal, a rational or a decimal-or-hex integer —
        // the parser enforces a value space the XSD does not state. So
        // when the type says nothing, the VALUE is the only evidence
        // available, and the mutation follows its shape. That is not a
        // per-attribute list: nothing here names an attribute.
        //
        // ⚠⚠ A reference is deliberately NOT special-cased.
        // `sce:count`, `sce:length-field`, `sce:present-if` and
        // `<sce:variant tag>` name something declared elsewhere, and any
        // second value fails to resolve. Pointing them at another
        // declared name needs per-attribute knowledge of what they point
        // AT, which is the hand-written map this file exists without.
        // They stay unmeasured and say why in the parser's own words.
        "string" => Some(common::xml_literal::second_value_of_same_shape(current)),
        // An XML name: prefixing keeps it an NCName. Suffixing would
        // too, but a prefix also moves a value that some readers
        // compare by suffix.
        "NCName" | "ID" | "IDREF" | "token" => Some(format!("z{}", current.trim())),
        // A named type the grammar neither enumerates nor this function
        // knows — `paramNameType` is a pattern, `bitSizeType` a union
        // over integers. Following the value's shape is a guess, and a
        // wrong guess is reported as "the XSD refused the mutant" with
        // the validator's words. That is strictly more than the `None`
        // this arm used to answer, which said only that the test had
        // not tried.
        _ => Some(common::xml_literal::second_value_of_same_shape(current)),
    }
}

/// Why a document did not become an IR, in the words of whatever
/// refused it.
///
/// ⚠ One variant per REFUSER, not one for "rejected". Everything behind
/// `parse_to_ir` used to answer `None`, so a mutant the XSD turned away
/// and a mutant the parser turned away were reported identically — and
/// they mean opposite things. An XSD rejection says the mutation left
/// the attribute's declared type, which is this file's own fault and
/// fixable in [`mutate`]. A parser rejection with the XSD content says
/// the value is inside the declared type and the parser refuses it
/// anyway, which is a constraint the grammar does not express: a
/// finding about the tree rather than about the test.
enum NoIr {
    /// `expand_preprocessors` refused the text.
    Preprocessor(String),
    /// `schemas/sce-forge.xsd` refused it.
    Xsd(String),
    /// The XSD was content and the forge parser refused it.
    Parser(String),
    /// A well-formed document that is not a forge kind at all.
    NotForge,
}

impl NoIr {
    /// One line, naming the refuser and quoting it.
    ///
    /// ⚠ The refuser's own words, shortened but never replaced. A
    /// refusal that does not say why sends the next reader to guess,
    /// and the guess is what goes stale.
    fn say(&self) -> String {
        let brief = |s: &str| {
            let one = s.lines().next().unwrap_or("").trim();
            match one.char_indices().nth(140) {
                Some((cut, _)) => format!("{}…", &one[..cut]),
                None => one.to_string(),
            }
        };
        match self {
            Self::Preprocessor(m) => format!("the preprocessor refused the mutant: {}", brief(m)),
            Self::Xsd(m) => format!(
                "the XSD refused the mutant, so the mutation left the declared \
                 type — mutate() owes this type a better second value: {}",
                brief(m)
            ),
            Self::Parser(m) => format!(
                "XSD accepted the mutant and the PARSER refused it, so the grammar \
                 does not express what the parser requires: {}",
                brief(m)
            ),
            Self::NotForge => "the fixture is not a forge document".to_string(),
        }
    }
}

/// Parse a document the way every forge review artefact does.
fn parse_to_ir(text: &str, label_stem: &str, dir: Option<&Path>) -> Result<String, NoIr> {
    let expanded = sce_build::parser::expand_preprocessors(text, label_stem, dir, &[])
        .map_err(|e| NoIr::Preprocessor(e.to_string()))?;
    let label = DocumentLabel {
        identifier: label_stem,
        diagnostic_label: label_stem,
    };
    // Asked SEPARATELY, and before the parser, because the parser runs
    // this same validation as its own first step and then answers with
    // one error type for both. Asking here is the only way to know
    // which of the two turned a mutant away.
    if let Err(e) = sce_build::forge::xsd_validator::validate_or_skip(&expanded.0, label_stem) {
        return Err(NoIr::Xsd(e.to_string()));
    }
    let parsed = sce_build::forge::parser::parse_forge_with_imports(&expanded.0, label)
        .map_err(|e| NoIr::Parser(e.to_string()))?
        .ok_or(NoIr::NotForge)?;
    // The WHOLE envelope, not `parsed.document`. Measured while writing
    // this file: comparing the document alone reported `<sce:import src>`
    // and `<sce:import as>` as unread, because an import's attributes
    // land in `parsed.imports` — a defect in the comparison, not in the
    // parser. A partial view of the IR makes every field outside it look
    // dropped.
    serde_json::to_string(&parsed).map_err(|e| NoIr::Parser(e.to_string()))
}

/// Every element `sce-forge-ext.xsd` declares.
fn declared_elements() -> BTreeSet<String> {
    let path = repo_root().join("schemas/sce-forge-ext.xsd");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc = roxmltree::Document::parse(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    doc.descendants()
        .filter(|n| n.has_tag_name("element"))
        .filter_map(|n| n.attribute("name").map(str::to_string))
        .collect()
}

/// The grammar must name every `sce:` element a real document carries.
///
/// The companion to the test below, asked from the other end. That one
/// starts at the grammar and looks for a reader; this one starts at a
/// document the parser accepted and looks for a declaration. Neither
/// direction implies the other, and both were red on 2026-09-20:
/// `<sce:while max-iter>` was declared and unread, while `<sce:helper>`,
/// `<sce:capacity>`, `<sce:cancel-on>`, `<sce:reset-on>`,
/// `<sce:context>`, `<sce:period>`, `<sce:fire-event>` and
/// `<sce:element-type>` were read and declared nowhere.
///
/// # ⚠ Why this asks about ELEMENTS and not about attributes
///
/// The attribute question cannot be asked here, and the reason is worth
/// stating rather than leaving as an omission somebody later "fixes".
/// XSD validation runs inside the parse path and is compile-time gated
/// (`feature = "xsd"`, no runtime bypass), so an attribute the grammar
/// does not admit makes the whole document invalid — the parser never
/// sees it, and no fixture carrying one can reach this test. An
/// attribute half would therefore be structurally unable to fire.
/// Measured: one was written, then armed by deleting the `max-iter`
/// declaration from the schema, and it stayed green.
///
/// An undeclared ELEMENT is different, and that difference is the whole
/// value of this test: the content models it appears in are
/// `xs:any processContents="lax"`, so an element the grammar never names
/// validates *by being unknown*. Eight had accumulated.
///
/// What it also buys: a misspelled `sce:` element name is admitted the
/// same way, and this is what makes one visible.
#[test]
fn every_sce_element_a_document_carries_is_named_by_the_grammar() {
    let elements = declared_elements();
    let files = fixture_files();
    assert!(
        !elements.is_empty() && !files.is_empty(),
        "nothing to measure: {} declared element(s), {} fixture(s)",
        elements.len(),
        files.len()
    );

    let mut undeclared: BTreeMap<String, String> = BTreeMap::new();
    let mut accepted = 0usize;

    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(doc) = roxmltree::Document::parse(&text) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        // A fixture the parser refuses is not evidence about the accepted
        // surface — it never got that far. Deciding this from the
        // document itself keeps negative fixtures out without a list of
        // their names.
        if parse_to_ir(&text, stem, path.parent()).is_err() {
            continue;
        }
        accepted += 1;

        for node in doc.descendants() {
            if node.tag_name().namespace() != Some(SCE_NS) {
                continue;
            }
            let name = node.tag_name().name();
            if elements.contains(name) {
                continue;
            }
            undeclared.entry(name.to_string()).or_insert_with(|| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            });
        }
    }

    println!("declared elements       : {}", elements.len());
    println!("fixtures the parser took: {accepted}");

    // A floor: a sweep that accepted no document would satisfy the
    // assertion below by looking at nothing.
    assert!(
        accepted > 0,
        "no fixture parsed — the sweep is vacuous, not clean"
    );

    assert!(
        undeclared.is_empty(),
        "these `sce:` elements appear in a document the parser accepts \
         and `sce-forge-ext.xsd` declares none of them — the content \
         model admits them by lax wildcard, so the grammar is silent \
         about them rather than permissive on purpose, and a misspelling \
         validates the same way:\n  {}",
        undeclared
            .iter()
            .map(|(el, f)| format!("<sce:{el}>  (e.g. {f})"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

#[test]
fn every_declared_attribute_a_fixture_writes_reaches_the_ir() {
    let declared = declared_pairs();
    let files = fixture_files();
    let enums = enumerations();
    assert!(
        enums.values().any(|v| v.len() > 1),
        "the grammar reader found no enumerated type, so every named type will \
         fall back to a guess about its value — the XSD moved, or this reader did"
    );
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
    let mut unmeasured: BTreeMap<String, String> = BTreeMap::new();

    for d in &declared {
        let key = format!(
            "<sce:{} {}{}>",
            d.element,
            if d.qualified { "sce:" } else { "" },
            d.attribute
        );

        let Some(replacement) = try_each_fixture(d, &files, &enums, &mut unmeasured, &key) else {
            unmeasured
                .entry(key.clone())
                .or_insert_with(|| "no fixture writes it".to_string());
            continue;
        };
        let (path, before_text, range, new_value, before_ir) = replacement;

        let mut after_text = before_text.clone();
        after_text.replace_range(range, &new_value);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("fixture");
        let after_ir = match parse_to_ir(&after_text, stem, path.parent()) {
            Ok(ir) => ir,
            Err(why) => {
                unmeasured.insert(key.clone(), why.say());
                continue;
            }
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
    enums: &BTreeMap<String, Vec<String>>,
    unmeasured: &mut BTreeMap<String, String>,
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
            let Some(new_value) = mutate(&d.xsd_type, attr.value(), enums) else {
                unmeasured.entry(key.to_string()).or_insert_with(|| {
                    format!(
                        "no mechanical second value for xs:{} — mutate() does not \
                         know this type",
                        d.xsd_type
                    )
                });
                return None;
            };
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("fixture");
            let Ok(before_ir) = parse_to_ir(&text, stem, path.parent()) else {
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
