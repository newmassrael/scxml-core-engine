// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An algorithm's else branch is what its `<sce:else>` contains, and an
//! `<sce:if>` ends with it (SCE_FORGE.md §4.12). A statement written after the
//! `<sce:else>` is refused as `algorithm/statement-after-else`, placed on the
//! statement, with the `<sce:else>` as a `must-follow` site.
//!
//! # What was wrong
//!
//! Reported by a consumer on 2026-11-23: `<sce:if cond="n === process">S1
//! <sce:else/><sce:if cond="!there">S2</sce:if></sce:if>` passed `check` and
//! generated `if n == process { S1; if !there { S2 } } else { }`. A statement
//! after the `<sce:else>` was read into the then branch, so an author who
//! wrote SCXML's separator form got the else branch run under the condition and
//! an empty else, without a word.

use sce_build::forge::error::{ForgeError, RelatedRole, ValidationError};
use sce_build::generator::Language;
use sce_build::DocumentLabel;

/// An algorithm `pick(x)` whose `<sce:if cond="x === 1">` holds `branches`.
fn algorithm(branches: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm" name="pick" version="1.0">
  <sce:signature>
    <sce:param name="x" type="uint32"/>
    <sce:return type="uint32"/>
  </sce:signature>
  <sce:body>
    <sce:var name="r" type="uint32" init="0"/>
    <sce:if cond="x === 1">
{branches}
    </sce:if>
    <sce:return expr="r"/>
  </sce:body>
</scxml>
"#
    )
}

fn compile(
    document: &str,
    language: Language,
) -> Result<String, sce_build::forge::error::Located<ForgeError>> {
    sce_build::compile_forge_from_string(document, DocumentLabel::symmetric("pick"), language).map(
        |out| {
            out.files
                .into_iter()
                .map(|(_, content)| content)
                .collect::<Vec<_>>()
                .join("\n")
        },
    )
}

/// SCXML's separator form, `<sce:else/>` with the else branch after it, is
/// refused on every backend, on the row of the first statement after the
/// `<sce:else>`, and names the `<sce:else>`'s row as the site that must
/// follow it.
#[test]
fn a_statement_after_the_else_is_refused_and_names_the_else() {
    let document = algorithm(
        r#"      <sce:assign target="r" expr="10"/>
      <sce:else/>
      <sce:assign target="r" expr="20"/>"#,
    );
    for language in [Language::Rust, Language::Python, Language::Cpp] {
        let err =
            compile(&document, language).expect_err("a statement after <sce:else> is refused");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ref boxed)
                    if matches!(
                        **boxed,
                        ValidationError::AlgorithmStatementAfterElse { ref statement, ref else_tag }
                            if statement == "sce:assign" && else_tag == "sce:else"
                    )
            ),
            "{language:?}: {:?}",
            err.error
        );
        assert_eq!(err.location.line, Some(13), "{language:?}: {err:?}");
        let related: Vec<_> = err
            .related()
            .iter()
            .map(|site| (site.role, site.location.line, site.actual.as_deref()))
            .collect();
        assert_eq!(
            related,
            [(RelatedRole::MustFollow, Some(12), Some("sce:else"))],
            "{language:?}"
        );
    }
}

/// A statement after a container `<sce:else>`, the form that carries its
/// branch, is refused the same way: no statement follows the else branch.
#[test]
fn a_statement_after_a_container_else_is_refused() {
    let document = algorithm(
        r#"      <sce:assign target="r" expr="10"/>
      <sce:else><sce:assign target="r" expr="20"/></sce:else>
      <sce:assign target="r" expr="30"/>"#,
    );
    let err =
        compile(&document, Language::Rust).expect_err("a statement after <sce:else> is refused");
    assert!(
        matches!(
            err.error,
            ForgeError::Validation(ref boxed)
                if matches!(**boxed, ValidationError::AlgorithmStatementAfterElse { .. })
        ),
        "{:?}",
        err.error
    );
    assert_eq!(err.location.line, Some(13), "{err:?}");
}

/// The container form generates the else branch's statements in the else
/// branch, and the then branch's before it.
#[test]
fn the_container_else_carries_its_branch() {
    let document = algorithm(
        r#"      <sce:assign target="r" expr="10"/>
      <sce:else>
        <sce:assign target="r" expr="20"/>
      </sce:else>"#,
    );
    let rust = compile(&document, Language::Rust).expect("the container form generates");
    let then_at = rust.find("r = 10;").expect("the then branch is generated");
    let else_at = rust.find("} else {").expect("the else branch is generated");
    let else_stmt_at = rust
        .find("r = 20;")
        .expect("the else statement is generated");
    assert!(then_at < else_at && else_at < else_stmt_at, "{rust}");
}
