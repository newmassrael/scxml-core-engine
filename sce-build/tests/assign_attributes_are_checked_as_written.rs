// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! W3C SCXML 5.4.1: `<assign>`'s `location` is required, and its `expr`
//! "must not occur in an <assign> element that has children".
//!
//! Neither was checked until 2026-09-25: a document with both was read with
//! the attribute winning, and one with no `location` assigned to nowhere at
//! run time. The frontend now refuses both, where the document is read, so
//! every generated backend refuses alike.
//!
//! What is required is the ATTRIBUTE. `location=""` names no location, which
//! §5.4 answers at run time with `error.execution` — the W3C suite writes
//! exactly that (test286 and its siblings) — so it must still parse. The
//! same goes for an `expr` beside whitespace, which is not a child value.

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::parser::SCXMLParser;

/// A one-state document whose entry holds `assign`.
fn document(assign: &str) -> String {
    format!(
        "<?xml version=\"1.0\"?>
<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" datamodel=\"ecmascript\" initial=\"s\">
  <datamodel><data id=\"v\" expr=\"0\"/></datamodel>
  <state id=\"s\">
    <onentry>{assign}</onentry>
  </state>
</scxml>
"
    )
}

/// The code the parse of `assign`'s document is refused with.
fn refused_with(assign: &str) -> DiagnosticCode {
    let err = SCXMLParser::new()
        .parse_string(&document(assign), "assign.scxml")
        .expect_err("the document is refused");
    err.to_diagnostics()
        .into_iter()
        .next()
        .expect("the refusal reports a diagnostic")
        .code
}

fn accepted(assign: &str) {
    if let Err(err) = SCXMLParser::new().parse_string(&document(assign), "assign.scxml") {
        panic!("`{assign}` must parse, and was refused: {:?}", err.error);
    }
}

#[test]
fn an_assign_without_a_location_is_refused() {
    let code = refused_with("<assign expr=\"1\"/>");
    assert!(
        matches!(code, DiagnosticCode::ValidationMissingAttribute),
        "{code:?}"
    );
}

#[test]
fn an_empty_location_is_the_run_time_error_the_w3c_suite_expects() {
    accepted("<assign location=\"\" expr=\"1\"/>");
}

#[test]
fn an_expr_beside_element_children_is_refused() {
    let code = refused_with("<assign location=\"v\" expr=\"1\"><x xmlns=\"\">2</x></assign>");
    assert!(
        matches!(code, DiagnosticCode::ValidationIncompatibleAttributes),
        "{code:?}"
    );
}

#[test]
fn an_expr_beside_text_is_refused() {
    let code = refused_with("<assign location=\"v\" expr=\"1\">2</assign>");
    assert!(
        matches!(code, DiagnosticCode::ValidationIncompatibleAttributes),
        "{code:?}"
    );
}

/// The controls: each way of giving the value on its own, and whitespace,
/// which a pretty-printed document puts between tags without meaning a value.
#[test]
fn one_value_on_its_own_is_accepted() {
    accepted("<assign location=\"v\" expr=\"1\"/>");
    accepted("<assign location=\"v\">2</assign>");
    accepted("<assign location=\"v\" expr=\"1\">\n      </assign>");
}
