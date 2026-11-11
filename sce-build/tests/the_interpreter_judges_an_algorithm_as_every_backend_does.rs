// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Interpreter's lowering of an algorithm runs the judges of its body that
//! every backend's route runs before it renders one (`judge_algorithm_body`,
//! `sce-build/src/lib.rs`), so the Interpreter refuses the documents the
//! backends refuse, with the judge's own diagnostic, in the algorithm's
//! document: here an integer operation the range analysis cannot prove safe in
//! a body that does not declare `may-fail` (SCE_FORGE.md §3.4.1), which a
//! backend refuses and the Interpreter used to lower and run.

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::static_js::lower_algorithm_document;

/// The algorithm `probe_increment`, which returns its `uint8` parameter plus
/// one, `may-fail` or not, written into a scratch directory; what the
/// Interpreter's lowering of it answers, as the code of the refusal and the
/// file it is located in, or the installed text.
fn lowered(may_fail: bool) -> Result<String, (DiagnosticCode, String, String)> {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let fail = if may_fail { r#" may-fail="true""# } else { "" };
    let text = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm" name="probe_increment" version="1.0">
  <sce:signature>
    <sce:param name="n" type="uint8"/>
    <sce:return type="uint8"{fail}/>
  </sce:signature>
  <sce:body>
    <sce:return expr="n + 1"/>
  </sce:body>
</scxml>
"#
    );
    let path = dir.path().join("probe_increment.scxml");
    std::fs::write(&path, text).expect("write the probe");
    lower_algorithm_document(path.to_str().expect("a utf-8 path"))
        .map(|lowered| lowered.install)
        .map_err(|refusal| {
            let record = refusal.error.to_diagnostics().remove(0);
            (
                record.code,
                refusal.location.file.clone(),
                refusal.error.to_string(),
            )
        })
}

/// Pinned by the call of `judge_algorithm_body` in `static_js_algorithm::judge`:
/// without it the body is lowered to a function whose checked addition throws
/// where a backend refused the document.
#[test]
fn an_undeclared_integer_failure_is_refused_as_every_backend_refuses_it() {
    let (code, file, why) = lowered(false).expect_err("`n + 1` may overflow a uint8");
    assert!(
        matches!(code, DiagnosticCode::AlgorithmUndeclaredIntegerFailure),
        "{code:?}: {why}"
    );
    assert!(file.ends_with("probe_increment.scxml"), "{file}: {why}");
    // Declared, the failure is the function's to throw where it runs.
    let installed = lowered(true).unwrap_or_else(|(code, _, why)| panic!("{code:?}: {why}"));
    assert!(
        installed.contains("SceStatic.algorithms.probe_increment = function (n) { "),
        "{installed}"
    );
}
