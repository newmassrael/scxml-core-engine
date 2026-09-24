// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An in-line child document that does not parse refuses its parent, at the
//! row and column its author wrote.
//!
//! `<invoke>`'s `<content>` may hold a whole `<scxml>` (W3C SCXML 6.4), which
//! the generator parses as a document of its own. When that parse failed it
//! printed one `Warning:` line on stderr and went on without the child: the
//! parent generated, exit 0, and its code still included the child's header,
//! so the build failed later, far from the cause (measured 2026-09-24). Its
//! message, had anyone acted on it, named a row in a document nobody wrote:
//! the child's own text, behind an XML declaration and with the bindings it
//! inherits inserted on its first line.
//!
//! The oracle is the refusal the parent's parse returns: its file is the
//! parent's, and its row and column are where the child's author wrote the
//! element it refuses.

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::parser::SCXMLParser;

/// A child written across lines; its `<cancel>` names neither `sendid` nor
/// `sendidexpr` (W3C SCXML 6.3), on row 8, column 22.
const ACROSS_LINES: &str = "<?xml version=\"1.0\"?>
<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"s0\">
  <state id=\"s0\">
    <invoke id=\"kid\" type=\"http://www.w3.org/TR/scxml/\">
      <content>
        <scxml version=\"1.0\" initial=\"c\">
          <state id=\"c\">
            <onentry><cancel/></onentry>
          </state>
        </scxml>
      </content>
    </invoke>
  </state>
</scxml>
";

/// A child written on one line — its root's — so the refused `<cancel>`
/// stands after the bindings inserted there: row 5, column 57.
const ON_THE_ROOT_LINE: &str = "<?xml version=\"1.0\"?>
<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"s0\">
  <state id=\"s0\">
    <invoke id=\"kid\" type=\"http://www.w3.org/TR/scxml/\"><content>
<scxml version=\"1.0\" initial=\"c\"><state id=\"c\"><onentry><cancel/></onentry></state></scxml>
    </content></invoke>
  </state>
</scxml>
";

/// The refusal parsing `document` under `label` returns, as its file, row,
/// column and code.
fn refusal(document: &str, label: &str) -> (String, Option<u32>, Option<u32>, DiagnosticCode) {
    let err = SCXMLParser::new()
        .parse_string(document, label)
        .expect_err("a child that does not parse refuses its parent");
    let code = err
        .to_diagnostics()
        .into_iter()
        .next()
        .expect("the refusal reports a diagnostic")
        .code;
    (
        err.location.file.clone(),
        err.location.line,
        err.location.col,
        code,
    )
}

/// Where `needle` starts in `document`, as 1-based (row, column).
fn position_of(document: &str, needle: &str) -> (u32, u32) {
    let offset = document
        .find(needle)
        .expect("the fixture holds the element");
    let before = &document[..offset];
    let row = before.matches('\n').count() + 1;
    let col = offset - before.rfind('\n').map_or(0, |n| n + 1) + 1;
    (row as u32, col as u32)
}

#[test]
fn a_refusal_in_the_child_is_placed_in_the_parent() {
    let (file, line, col, code) = refusal(ACROSS_LINES, "parent");
    assert_eq!(
        file, "parent",
        "the refusal names the file its author wrote"
    );
    let (row, column) = position_of(ACROSS_LINES, "<cancel/>");
    assert_eq!((row, column), (8, 22), "the fixture's own arithmetic");
    assert_eq!((line, col), (Some(row), Some(column)));
    assert!(
        matches!(code, DiagnosticCode::ValidationRequireEither),
        "the child's own refusal, not a stand-in for it: {code:?}"
    );
}

#[test]
fn a_refusal_on_the_childs_root_line_keeps_its_column() {
    let (file, line, col, _) = refusal(ON_THE_ROOT_LINE, "parent");
    assert_eq!(file, "parent");
    let (row, column) = position_of(ON_THE_ROOT_LINE, "<cancel/>");
    assert_eq!((row, column), (5, 57), "the fixture's own arithmetic");
    assert_eq!(
        (line, col),
        (Some(row), Some(column)),
        "the bindings inserted before the <cancel> on this line are not the author's"
    );
}
