// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! W3C SCXML §6.4.3 has one wording on every backend, and this is where that
//! is true rather than remembered.
//!
//! # What it is for
//!
//! Measured 2026-09-20, the same fact — an `<invoke>` expression that cannot
//! be evaluated — reached authors in four different spellings across the
//! seven emitters: one named the attribute and nothing else, two named the
//! expression too, one named neither, and one folded the failure together
//! with "the child could not be started" so a reader could not tell which had
//! happened.
//!
//! The §6.4.1 sibling was collapsed the same way in an earlier round, and its
//! test records why a per-backend wording is not something a table should be
//! able to express: a dialect per backend is a contract nobody can act on.
//! This file is that clause's other half.
//!
//! # Why the text and not the behaviour
//!
//! The behaviour has its own witness —
//! `integration_resources/invoke_expression_failure_is_reported/`, driven on
//! all seven channels — and it asserts that the raise happens. It cannot
//! assert WHAT the raise says without reading a payload on every channel,
//! which is seven harnesses' work for one string. This reads the emitted text
//! once per backend instead, and the Interpreter's half is asserted at
//! runtime in `tests/integration/InvokeExpressionFailureIsReportedTest.cpp`,
//! which is the only way to reach a literal that is compiled rather than
//! generated.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn codegen_bin() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_sce-codegen")).to_path_buf()
}

fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sce-invoke-expr-wording-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch output directory");
    dir
}

/// The one text an `<invoke>` expression failure raises, on every emitter.
///
/// Shaped after the §6.4.1 sibling: name the offending thing, then state the
/// fact. The expression is included because it is what an author needs to
/// find the line, and because three of the seven emitters could not tell
/// them.
fn evaluation_refusal(attribute: &str, expression: &str) -> String {
    format!("<invoke {attribute}='{expression}'> could not be evaluated")
}

/// The expression the canonical fixture names, and what it is written into.
const FIXTURE_EXPRESSION: &str = "target.path";

/// Every backend and the file its machine lands in.
const EMITTED_FILE: [(&str, &str); 6] = [
    ("rust", "invoke_expression_failure_is_reported_sm.rs"),
    ("cpp", "invoke_expression_failure_is_reported_sm.inl"),
    ("go", "invoke_expression_failure_is_reported_sm.go"),
    ("c11", "invoke_expression_failure_is_reported_sm.c"),
    ("python", "invoke_expression_failure_is_reported_sm.py"),
    ("kotlin", "invoke_expression_failure_is_reportedSm.kt"),
];

/// The spellings this replaced. A backend that grows one back is not a
/// dialect, it is a regression, and naming them is what makes the failure
/// message say which one came back.
///
/// ⚠ Each entry names THIS fact and no other. The first draft carried the
/// bare fragment `failed to evaluate"`, which also matches the wordings a
/// `<transition cond>` and a `<data expr>` raise — different facts with
/// their own settled texts — so the sweep failed on a backend that was
/// correct. A retirement list wider than its subject reports the innocent.
const RETIRED_SPELLINGS: [&str; 7] = [
    "srcexpr evaluation failed",
    "contentexpr evaluation failed",
    "<invoke> srcexpr '",
    "<invoke> contentexpr '",
    "<invoke> srcexpr failed to evaluate",
    "<invoke> contentexpr failed to evaluate",
    "could not evaluate its source or start a child",
];

#[test]
fn every_backend_raises_the_same_text_for_an_unevaluatable_invoke_expression() {
    // The floor is the language set itself: a backend dropped from the table
    // would otherwise shrink this sweep silently.
    assert_eq!(
        EMITTED_FILE.len(),
        6,
        "the emission table no longer covers every backend SCE generates",
    );

    let fixture = repo_root()
        .join("integration_resources/invoke_expression_failure_is_reported")
        .join("invoke_expression_failure_is_reported.scxml");
    let expected = evaluation_refusal("srcexpr", FIXTURE_EXPRESSION);

    for (lang, file) in EMITTED_FILE {
        let dir = out_dir(lang);
        let run = Command::new(codegen_bin())
            .args([
                "generate",
                fixture.to_str().unwrap(),
                "-l",
                lang,
                "-o",
                dir.to_str().unwrap(),
            ])
            .current_dir(repo_root())
            .output()
            .expect("sce-codegen runs");
        assert!(
            run.status.success(),
            "{lang} refused the canonical fixture: {}",
            String::from_utf8_lossy(&run.stderr),
        );

        let emitted = std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("{lang} did not write {file}: {e}"));

        assert!(
            emitted.contains(&expected),
            "{lang} does not raise the one §6.4.3 wording. A backend-shaped \
             variant of it is a regression rather than a dialect. Expected to \
             find:\n  {expected}",
        );

        for retired in RETIRED_SPELLINGS {
            assert!(
                !emitted.contains(retired),
                "{lang} emitted a retired spelling of the §6.4.3 failure \
                 (`{retired}`), so the same fact reaches authors two ways again",
            );
        }
    }
}
