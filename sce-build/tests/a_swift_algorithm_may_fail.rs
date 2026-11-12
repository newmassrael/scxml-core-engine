// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm that declares `<sce:return may-fail="true">` (SCE_FORGE.md §3.4.1),
// written for Swift: the function `throws(AlgorithmError)`, every integer operation
// is a checked helper that throws its failure where it happens, marked `try` where it
// stands, and a `<sce:require>` that does not hold throws the precondition. A failure
// is not a value (docs/adr/0098): nothing is recorded, and nothing after it runs.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{compile_forge_file, ForgeCompileOptions};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

#[test]
fn a_may_fail_function_throws_its_typed_failure_where_an_operation_fails() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/algorithm_ledger_ids.scxml")
        .expect("a may-fail algorithm over a list is written");
    assert!(
        code.contains("func ledgerIds(_ ids: [UInt32]) throws(AlgorithmError) -> UInt32"),
        "{code}"
    );
    // The sum is the runtime's checked one, which throws an overflow and answers no
    // value in its place.
    assert!(code.contains("sum = (try Checked.add(sum, id))"), "{code}");
    assert!(code.contains("import SceForgeRuntime"), "{code}");
}

#[test]
fn no_failure_is_recorded_and_none_is_read_back_after_a_statement() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/algorithm_ledger_ids.scxml")
        .expect("written");
    for gone in ["sceFailure", "Failure()", "throwIfFailed", "sceReturned"] {
        assert!(!code.contains(gone), "`{gone}` in:\n{code}");
    }
    // The value is returned as it is computed: a failure in it has already left.
    assert!(code.contains("return sum"), "{code}");
}

#[test]
fn a_precondition_that_does_not_hold_throws_it() {
    let code = swift_of("stdlib/http/precondition.scxml").expect("a require is written");
    assert!(
        code.contains("throw AlgorithmError.precondition"),
        "a `<sce:require>` is a thrown precondition: {code}"
    );
    // A condition decides its branch where it is written: a failure in it throws
    // before any branch runs, so it is no local of its own.
    assert!(!code.contains("let sceCond"), "{code}");
}

#[test]
fn a_loop_condition_that_can_fail_is_the_loops_own_condition() {
    let code = swift_of("stdlib/time/utc_from_local.scxml").expect("a loop is written");
    // The condition is read at the top of every pass by the loop itself; a failure in
    // it throws before it decides whether the loop goes on.
    assert!(!code.contains("while true {"), "{code}");
    assert!(code.contains("while "), "{code}");
    // An element is read by its index through the runtime's checked read, which
    // throws an out-of-range index instead of trapping.
    assert!(code.contains("(try Checked.at("), "{code}");
}
