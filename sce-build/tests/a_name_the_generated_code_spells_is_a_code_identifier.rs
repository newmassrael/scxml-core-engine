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

mod common;

use std::path::PathBuf;

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

    let documents: Vec<(PathBuf, String)> = scxml_documents()
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

/// A code identifier of the right shape that a target language reserves is
/// refused for every backend, on its own row, naming the language — `pass`
/// is a Python keyword, so the generated Python could not declare the input.
#[test]
fn a_forge_data_id_a_language_reserves_is_refused_naming_the_language() {
    let document = transform_with_input("pass");
    let refusal = forge_refusal(&document);
    assert_eq!(code_of(&refusal), "\"validation/reserved-code-identifier\"");
    let at = position_of(&document, "\"pass\"");
    assert_eq!(
        (refusal.location.line, refusal.location.col),
        (Some(at.0), Some(at.1 + 1)),
        "{refusal:?}"
    );
    let diagnostic = &refusal.error.to_diagnostics()[0];
    assert_eq!(diagnostic.actual.as_deref(), Some("pass"));
    assert!(
        refusal
            .error
            .to_string()
            .contains("reserved word in python"),
        "{}",
        refusal.error
    );
}

/// The spelling a backend actually uses is what is asked: `Override` is not
/// a keyword as written, but Rust folds a forge `<data id>` to snake_case,
/// and Rust reserves `override`. The message names both, because the word
/// refused is not the one the author typed.
#[test]
fn a_reserved_word_is_found_in_the_spelling_the_backend_uses() {
    let refusal = forge_refusal(&transform_with_input("Override"));
    assert_eq!(code_of(&refusal), "\"validation/reserved-code-identifier\"");
    assert!(
        refusal
            .error
            .to_string()
            .contains("rust spells 'Override' as 'override', a word it reserves"),
        "{}",
        refusal.error
    );
}

fn enum_with_variant(name: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="enum" name="named_input" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="{name}" value="1"/>
    </data>
  </datamodel>
</scxml>"#
    )
}

/// A variant is spelled `Pascal` in Rust and C++ and `UPPER_SNAKE` in
/// Kotlin and Python, and prefixed with its type in Go and C11 — so `self`
/// is Rust's `Self`, which no backend can escape, and is refused naming
/// that spelling.
#[test]
fn a_variant_is_asked_in_the_spelling_a_variant_gets() {
    let refusal = forge_refusal(&enum_with_variant("self"));
    assert_eq!(code_of(&refusal), "\"validation/reserved-code-identifier\"");
    assert!(
        refusal
            .error
            .to_string()
            .contains("rust spells 'self' as 'Self', a word it reserves"),
        "{}",
        refusal.error
    );
}

/// The other direction, which folding every name to snake_case got wrong: a
/// variant `match` reaches Rust as `Match` and Kotlin as `MATCH`, neither a
/// keyword, so it is accepted — and so is a const `DEFAULT`, which every
/// backend spells `DEFAULT` while C++ reserves only `default`.
#[test]
fn a_name_a_backend_folds_away_from_its_keyword_is_accepted() {
    let label = sce_build::DocumentLabel {
        identifier: "named_input",
        diagnostic_label: "named_input.scxml",
    };
    sce_build::forge::parser::parse_forge_with_imports(&enum_with_variant("match"), label)
        .expect("a variant `match` is spelled `Match` / `MATCH`, which nothing reserves");
    let constant = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" version="1.0" name="named_input">
  <sce:const name="default" type="uint8" init="0"/>
  <sce:signature>
    <sce:param name="x" type="uint8"/>
    <sce:return type="uint8"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="x"/>
  </sce:body>
</scxml>"#;
    sce_build::forge::parser::parse_forge_with_imports(constant, label)
        .expect("a const `default` is spelled `DEFAULT` everywhere, which nothing reserves");
}

/// The discriminator: the same document with a name no language reserves
/// parses, so the two tests above are about the word and not the document.
#[test]
fn a_name_no_language_reserves_is_accepted() {
    let label = sce_build::DocumentLabel {
        identifier: "named_input",
        diagnostic_label: "named_input.scxml",
    };
    sce_build::forge::parser::parse_forge_with_imports(
        &transform_with_input("manualOverride"),
        label,
    )
    .expect("a name no language reserves parses");
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

/// Every `.scxml` document this repository commits, asked of git for the
/// reason `an_identifier_is_checked_against_the_grammar_w3c_gives_it`
/// gives: what a build writes is not this tree's documents.
fn scxml_documents() -> Vec<PathBuf> {
    common::repository::files_git_tracks(&["*.scxml"])
}
