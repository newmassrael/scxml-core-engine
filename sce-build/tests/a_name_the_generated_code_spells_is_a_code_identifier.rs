// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `docs/SCE_ACCEPTED_SUBSET.md` §2.14 — a name the generated code spells
//! is a code identifier, refused at parse on the attribute's own line.
//!
//! # What was wrong
//!
//! The forge pipeline checked none of its names. Measured 2026-09-22: a
//! transform whose `<data id>` was two Hangul letters passed the parse and
//! was refused by the expression lexer with no line, and one whose
//! `<data id>` was `raw-value` passed `check` in all six languages and
//! generated the C++ parameter `int32_t raw - value`.
//!
//! # What this measures
//!
//! 1. Both parsers reach the sweep: a forge document and a statechart are
//!    each refused under `validation/malformed-code-identifier`, on the
//!    row and column the value sits at.
//! 2. The dialects part where they should: a statechart's `<data id>` is
//!    W3C's `xs:ID`, which admits `-`, and a forge document's is not.
//! 3. Every row of `SCE_IDENTIFIER_ATTRIBUTES` is reachable — each is fed
//!    a hostile value inside a document this tree commits that carries it,
//!    and must be refused for that row. A row nothing can reach reads as
//!    protection and is none.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::scxml_identifier::{reject_malformed, Dialect, SCE_IDENTIFIER_ATTRIBUTES};

/// Two Hangul letters, the first three UTF-8 bytes wide — the id that
/// reached the expression lexer. Escaped: the file needs a letter no ASCII
/// grammar admits, not this script in particular.
const WIDE: &str = "\u{c628}\u{b3c4}";

fn transform_with_input(id: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform"
       name="named_input"
       version="1.0">
  <datamodel>
    <data id="{id}" sce:type="int32" sce:direction="in"/>
    <data id="out" sce:type="int32" sce:direction="out" expr="1"/>
  </datamodel>
</scxml>"#
    )
}

fn forge_refusal(document: &str) -> Located<ForgeError> {
    let label = sce_build::DocumentLabel {
        identifier: "named_input",
        diagnostic_label: "named_input.scxml",
    };
    sce_build::forge::parser::parse_forge_with_imports(document, label)
        .expect_err("the document is refused")
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_string(&refusal.error.to_diagnostics()[0].code).unwrap()
}

/// Where `needle` first sits in `document`, as the 1-based row and column
/// a refusal must name.
fn position_of(document: &str, needle: &str) -> (u32, u32) {
    let at = document.find(needle).expect("the value is in the document");
    let before = &document[..at];
    let row = before.matches('\n').count() as u32 + 1;
    let col = before[before.rfind('\n').map_or(0, |nl| nl + 1)..]
        .chars()
        .count() as u32
        + 1;
    (row, col)
}

#[test]
fn a_forge_data_id_with_a_dash_is_refused_on_its_own_row() {
    let document = transform_with_input("raw-value");
    let refusal = forge_refusal(&document);
    assert_eq!(
        code_of(&refusal),
        "\"validation/malformed-code-identifier\""
    );
    let at = position_of(&document, "raw-value");
    assert_eq!(
        (refusal.location.line, refusal.location.col),
        (Some(at.0), Some(at.1)),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.error.to_diagnostics()[0].actual.as_deref(),
        Some("raw-value")
    );
}

#[test]
fn a_forge_data_id_in_another_script_is_refused_on_its_own_row() {
    assert_eq!(WIDE.chars().next().map(char::len_utf8), Some(3));
    let document = transform_with_input(WIDE);
    let refusal = forge_refusal(&document);
    assert_eq!(
        code_of(&refusal),
        "\"validation/malformed-code-identifier\""
    );
    let at = position_of(&document, WIDE);
    assert_eq!(
        (refusal.location.line, refusal.location.col),
        (Some(at.0), Some(at.1)),
        "{refusal:?}"
    );
}

/// The statechart parser reaches the same sweep: an `sce:` element's name
/// inside a statechart is held to the same grammar.
#[test]
fn a_statechart_sce_name_is_refused_on_its_own_row() {
    let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="idle">
  <state id="idle">
    <onentry>
      <sce:action name="ring-bell"/>
    </onentry>
  </state>
</scxml>"#;
    let refusal = sce_build::parser::SCXMLParser::new()
        .parse_string(document, "ring.scxml")
        .expect_err("the document is refused");
    assert_eq!(
        code_of(&refusal),
        "\"validation/malformed-code-identifier\""
    );
    let at = position_of(document, "ring-bell");
    assert_eq!(
        (refusal.location.line, refusal.location.col),
        (Some(at.0), Some(at.1)),
        "{refusal:?}"
    );
}

/// A statechart's `<data id>` is W3C's `xs:ID`, which admits `-`; a forge
/// document's is a code identifier, which does not.
#[test]
fn the_two_dialects_part_at_the_data_id() {
    let statechart = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript" initial="s">
  <datamodel><data id="raw-value" expr="0"/></datamodel>
  <state id="s"/>
</scxml>"#;
    let forge = transform_with_input("raw-value");
    for (document, dialect, refused) in [
        (statechart, Dialect::Statechart, false),
        (forge.as_str(), Dialect::Forge, true),
    ] {
        let parsed = roxmltree::Document::parse(document).expect("well-formed");
        let root = parsed.root_element();
        assert_eq!(Dialect::of(&root), dialect);
        assert_eq!(
            reject_malformed(&root, "doc.scxml", dialect).is_err(),
            refused,
            "{dialect:?}"
        );
    }
}

/// Every row fires, found in a document this tree commits and fed a value
/// no code identifier grammar admits.
#[test]
fn every_row_of_the_sce_table_is_reachable_from_a_committed_document() {
    // `-` is an XML Name character and an operator in every target, so a
    // row that fires on it is enforcing the code identifier rule and not
    // merely W3C's.
    const HOSTILE: &str = "x-y";

    let root = repo_root();
    let documents: Vec<(PathBuf, String)> = scxml_documents(&root)
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            Some((path, text))
        })
        .collect();

    let mut unreachable: Vec<String> = Vec::new();
    for &(element, attr, _) in SCE_IDENTIFIER_ATTRIBUTES {
        let Some((path, mutated)) = documents.iter().find_map(|(path, text)| {
            with_value_replaced(text, element, attr, HOSTILE).map(|m| (path, m))
        }) else {
            unreachable.push(format!(
                "  <sce:{element} {attr}>: no committed document carries it"
            ));
            continue;
        };
        let parsed = roxmltree::Document::parse(&mutated).expect("the splice keeps it well-formed");
        let root_element = parsed.root_element();
        match reject_malformed(&root_element, "mutated.scxml", Dialect::of(&root_element)) {
            Err(refusal) => match refusal.error {
                ForgeError::Validation(boxed) => match *boxed {
                    ValidationError::MalformedCodeIdentifier {
                        element: refused_element,
                        attr: refused_attr,
                        ..
                    } if refused_element.ends_with(&format!(":{element}"))
                        && refused_attr == attr => {}
                    other => unreachable.push(format!(
                        "  <sce:{element} {attr}> in {}: refused for {other:?}",
                        path.display()
                    )),
                },
                other => unreachable.push(format!(
                    "  <sce:{element} {attr}> in {}: refused for {other:?}",
                    path.display()
                )),
            },
            Ok(()) => unreachable.push(format!(
                "  <sce:{element} {attr}=\"{HOSTILE}\"> in {} was ACCEPTED — the row protects nothing",
                path.display()
            )),
        }
    }
    assert!(
        unreachable.is_empty(),
        "rows of scxml_identifier::SCE_IDENTIFIER_ATTRIBUTES that do not hold:\n{}",
        unreachable.join("\n"),
    );
}

/// `text` with the value of the first `<sce:{element} {attr}="…">` replaced
/// by `value`, or `None` when the document carries no such attribute.
fn with_value_replaced(text: &str, element: &str, attr: &str, value: &str) -> Option<String> {
    let parsed = roxmltree::Document::parse(text).ok()?;
    let range = parsed
        .descendants()
        .filter(|node| {
            node.is_element()
                && node.tag_name().namespace() == Some(sce_build::forge::model::SCE_NAMESPACE)
                && node.tag_name().name() == element
        })
        .find_map(|node| {
            node.attributes()
                .find(|a| a.namespace().is_none() && a.name() == attr)
                .map(|a| a.range_value())
        })?;
    let mut mutated = text.to_string();
    mutated.replace_range(range, value);
    Some(mutated)
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits under the repository root")
        .to_path_buf()
}

/// Every `.scxml` document this repository commits — `git ls-files`, for
/// the reason `an_identifier_is_checked_against_the_grammar_w3c_gives_it`
/// gives: what a build writes is not this tree's documents.
fn scxml_documents(root: &Path) -> Vec<PathBuf> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "*.scxml"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files must succeed");
    let mut documents: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| root.join(p))
        .collect();
    documents.sort();
    documents
}
