// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `docs/SCE_ACCEPTED_SUBSET.md` §2.14 — two names a forge document declares
//! must not become one member of the generated code.
//!
//! # What was wrong
//!
//! An author's names are case-sensitive, so `minRpm` and `min_rpm` are two of
//! them, and Rust, C11 and Python each spell a parameter or a codec field
//! snake_case. A document that declared both was accepted, and generated a
//! duplicate-argument `SyntaxError` in Python, a duplicate field in Rust and,
//! for a codec decoder, two fields read into one local — wrong values with
//! nothing refused.
//!
//! # What this measures
//!
//! 1. Each namespace in `forge::declared_names::SCOPES` is reachable: a pair
//!    the scope's backends spell alike is refused as
//!    `validation/colliding-code-identifier`, on the later declaration's own
//!    row and column, naming both names and a backend that folds them.
//! 2. The policy is every backend at once: a pair that folds in Go and Kotlin
//!    alone is refused, and the message does not name Rust, which builds it.
//! 3. A pair no backend spells alike is accepted. The rule is as narrow as
//!    the generated code is.
//! 4. No committed document is refused by it, which is what separates a rule
//!    that finds a defect from one that makes a new one.

mod common;

use sce_build::forge::declared_names::SCOPES;
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::error::{ForgeError, Located};
use sce_build::reader_names::Case;

const CODE: &str = "\"validation/colliding-code-identifier\"";

fn transform_with_inputs(first: &str, second: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform"
       name="two_inputs"
       version="1.0">
  <datamodel>
    <data id="{first}" sce:type="int32" sce:direction="in"/>
    <data id="{second}" sce:type="int32" sce:direction="in"/>
    <data id="out" sce:type="int32" sce:direction="out" expr="1"/>
  </datamodel>
</scxml>"#
    )
}

fn codec_with_fields(first: &str, second: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="two_fields">
  <datamodel>
    <sce:field id="{first}" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:field id="{second}" sce:type="uint8" sce:byte="1" sce:bit-size="8"/>
  </datamodel>
</scxml>"#
    )
}

fn codec_with_flags(first: &str, second: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="two_flags">
  <datamodel>
    <sce:flags id="header" sce:type="uint8" sce:byte="0" sce:bit-size="8">
      <sce:flag name="{first}" bit="0"/>
      <sce:flag name="{second}" bit="1"/>
    </sce:flags>
  </datamodel>
</scxml>"#
    )
}

/// A field and a flag of one codec: the one place two kinds of declaration
/// share a namespace in some backends and not in others.
fn codec_with_field_and_flag(field: &str, flag: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="field_and_flag">
  <datamodel>
    <sce:field id="{field}" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
    <sce:flags id="header" sce:type="uint8" sce:byte="1" sce:bit-size="8">
      <sce:flag name="{flag}" bit="0"/>
    </sce:flags>
  </datamodel>
</scxml>"#
    )
}

fn codec_with_flag_inputs(first: &str, second: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="two_inputs">
  <sce:flag-inputs>
    <sce:flag-input name="{first}" width="1"/>
    <sce:flag-input name="{second}" width="1"/>
  </sce:flag-inputs>
  <datamodel>
    <sce:field id="version" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>"#
    )
}

fn algorithm_with_consts(first: &str, second: &str) -> String {
    format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="two_consts" version="1.0">
  <sce:signature>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:const name="{first}" type="uint16" init="1"/>
  <sce:const name="{second}" type="uint16" init="2"/>
  <sce:body>
    <sce:return expr="1"/>
  </sce:body>
</scxml>"#
    )
}

fn parse(document: &str) -> Result<(), Located<ForgeError>> {
    let label = sce_build::DocumentLabel {
        identifier: "probe",
        diagnostic_label: "probe.scxml",
    };
    sce_build::forge::parser::parse_forge_with_imports(document, label).map(|_| ())
}

fn code_of(refusal: &Located<ForgeError>) -> String {
    serde_json::to_string(&refusal.error.to_diagnostics()[0].code).unwrap()
}

/// Where `needle` first sits in `document`, as the 1-based row and column a
/// refusal must name.
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

/// One scope, one pair the scope's backends spell alike, and the document
/// that declares it.
struct Pair {
    scope: &'static str,
    first: &'static str,
    second: &'static str,
    document: fn(&str, &str) -> String,
    /// What the refusal's later declaration is.
    noun: &'static str,
}

const CASES: &[Pair] = &[
    Pair {
        scope: "forge-data",
        first: "minRpm",
        second: "min_rpm",
        document: transform_with_inputs,
        noun: "data id",
    },
    Pair {
        scope: "codec-member",
        first: "minRpm",
        second: "min_rpm",
        document: codec_with_fields,
        noun: "codec member",
    },
    Pair {
        scope: "codec-flag",
        first: "hasX",
        second: "has_x",
        document: codec_with_flags,
        noun: "flag",
    },
    Pair {
        scope: "codec-flag-input",
        first: "A",
        second: "a",
        document: codec_with_flag_inputs,
        noun: "flag input",
    },
    Pair {
        scope: "const",
        first: "maxRpm",
        second: "MAX_RPM",
        document: algorithm_with_consts,
        noun: "const",
    },
    // The scope that compares two kinds of declaration: a member
    // `has_x` and a flag `hasX` are one name in C++, Go and Python, where a
    // class holds a field and a method in one table.
    Pair {
        scope: "codec-class",
        first: "has_x",
        second: "hasX",
        document: codec_with_field_and_flag,
        noun: "flag",
    },
];

#[test]
fn every_scope_is_reached_by_a_pair_it_spells_alike() {
    // A scope nothing here reaches reads as protection and is none.
    for scope in SCOPES {
        assert!(
            CASES.iter().any(|c| c.scope == scope.id),
            "no case reaches the scope `{}`",
            scope.id
        );
    }
    for case in CASES {
        assert!(
            SCOPES.iter().any(|s| s.id == case.scope),
            "the case names a scope the table does not have: `{}`",
            case.scope
        );
    }
}

#[test]
fn a_pair_a_backend_spells_alike_is_refused_on_the_later_declaration() {
    for case in CASES {
        let document = (case.document)(case.first, case.second);
        let refusal = parse(&document).expect_err(case.scope);
        assert_eq!(code_of(&refusal), CODE, "{}: {refusal:?}", case.scope);

        // On the later declaration's own value, which is what the contract's
        // `actual` requires: the token sits on the reported row.
        let at = position_of(&document, &format!("\"{}\"", case.second));
        assert_eq!(
            (refusal.location.line, refusal.location.col),
            (Some(at.0), Some(at.1 + 1)),
            "{}: {refusal:?}",
            case.scope
        );
        let diagnostic = &refusal.error.to_diagnostics()[0];
        assert_eq!(
            diagnostic.actual.as_deref(),
            Some(case.second),
            "{}",
            case.scope
        );

        let message = refusal.error.to_string();
        assert!(
            message.contains(&format!("'{}'", case.first))
                && message.contains(&format!("'{}'", case.second))
                && message.contains(case.noun),
            "{}: the message names neither the pair nor what it is: {message}",
            case.scope
        );
        assert!(
            message.contains("rename one of them"),
            "{}: {message}",
            case.scope
        );
    }
}

/// Reversing the pair moves the refusal to the other declaration: the record
/// is placed on the one that came second.
#[test]
fn the_later_declaration_is_the_one_refused() {
    let case = &CASES[0];
    let document = (case.document)(case.second, case.first);
    let refusal = parse(&document).expect_err("refused");
    let at = position_of(&document, &format!("\"{}\"", case.first));
    assert_eq!(
        (refusal.location.line, refusal.location.col),
        (Some(at.0), Some(at.1 + 1)),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.error.to_diagnostics()[0].actual.as_deref(),
        Some(case.first)
    );
}

/// `a_1` and `a1` are two names in Rust, Python and C11, which spell a codec
/// member snake_case, and in C++ and Kotlin, which write it as written, and
/// one in Go, which capitalises it and folds the underscore away. Only Go
/// declares one field twice, and the document is refused all the same:
/// whether SCE accepts a document must not depend on which backend a
/// deployment builds.
#[test]
fn a_pair_only_one_backend_folds_is_still_refused_for_all_of_them() {
    let (first, second) = ("a1", "a_1");
    assert_ne!(Case::Snake.spell(first), Case::Snake.spell(second));
    assert_eq!(Case::Pascal.spell(first), Case::Pascal.spell(second));

    let document = codec_with_fields(first, second);
    let refusal = parse(&document).expect_err("refused");
    assert_eq!(code_of(&refusal), CODE, "{refusal:?}");
    // The one backend that folds the pair names it, by its own convention,
    // derived here from the same filter; the others build it and are not
    // named.
    let message = refusal.error.to_string();
    let go = format!("go code spells both '{}'", Case::Pascal.spell(first));
    assert!(message.contains(&go), "{message}");
    for builds in ["rust", "python", "c11", "cpp", "kotlin"] {
        assert!(
            !message.contains(&format!("{builds} code")),
            "{builds} builds it: {message}"
        );
    }
}

#[test]
fn a_pair_no_backend_spells_alike_is_accepted() {
    let (first, second) = ("rpmMin", "rpmMax");
    assert_ne!(Case::Snake.spell(first), Case::Snake.spell(second));
    parse(&codec_with_fields(first, second)).expect("distinct names");
    parse(&transform_with_inputs(first, second)).expect("distinct names");
}

/// No document this tree commits is refused by the rule. A rule that refuses a
/// committed fixture is making a defect, not finding one.
#[test]
fn no_committed_document_is_refused_as_a_collision() {
    let documents = common::repository::files_git_tracks(&["*.scxml"]);
    let mut refused: Vec<String> = Vec::new();
    let mut parsed = 0usize;
    for path in &documents {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        parsed += 1;
        // Any other refusal is another rule's business — a fixture that needs
        // its imports, or one that is refused on purpose.
        if let Err(refusal) = parse(&text) {
            if code_of(&refusal) == CODE {
                refused.push(format!("{}: {}", path.display(), refusal.error));
            }
        }
    }
    assert!(parsed > 800, "only {parsed} documents were read");
    assert!(
        refused.is_empty(),
        "{} committed document(s) are refused as a collision:\n{}",
        refused.len(),
        refused.join("\n")
    );
}
