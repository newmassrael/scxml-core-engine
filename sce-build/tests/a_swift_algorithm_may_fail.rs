// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm that declares `<sce:return may-fail="true">` (SCE_FORGE.md §3.4.1),
// written for Swift: the function `throws`, every integer operation is a checked
// helper that records its failure, the body reads that record after each statement
// and throws it, and a `<sce:require>` that does not hold throws the precondition.

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
fn a_may_fail_function_throws_and_checks_its_arithmetic() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/algorithm_ledger_ids.scxml")
        .expect("a may-fail algorithm over a list is written");
    assert!(
        code.contains("func ledgerIds(_ ids: [UInt32]) throws -> UInt32"),
        "{code}"
    );
    assert!(code.contains("let sceFailure = Failure()"), "{code}");
    // The sum is the runtime's checked one, which records an overflow and yields 0.
    assert!(
        code.contains("sum = Checked.add(sceFailure, sum, id)"),
        "{code}"
    );
    assert!(code.contains("import SceForgeRuntime"), "{code}");
}

#[test]
fn a_failure_is_read_after_each_statement_and_before_a_value_is_returned() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/algorithm_ledger_ids.scxml")
        .expect("written");
    let throws = code.matches("try sceFailure.throwIfFailed()").count();
    // After the local's initializer, after the loop's statement, and before the
    // value is handed back.
    assert_eq!(throws, 3, "{code}");
    let value = code
        .find("let sceReturned: UInt32 = sum")
        .expect("the value is computed into a local first");
    let check = code[value..]
        .find("try sceFailure.throwIfFailed()")
        .expect("and the failure is read before it is returned");
    let returned = code[value..].find("return sceReturned").expect("returned");
    assert!(check < returned, "{code}");
}

#[test]
fn a_precondition_that_does_not_hold_throws_it() {
    let code = swift_of("stdlib/http/precondition.scxml").expect("a require is written");
    assert!(
        code.contains("throw AlgorithmError.precondition"),
        "a `<sce:require>` is a thrown precondition: {code}"
    );
    // A condition is computed in a block of its own, so two `if`s of one scope do
    // not declare it twice, and its failure is read before it decides a branch.
    assert!(code.contains("let sceCond"), "{code}");
}

#[test]
fn a_loop_condition_is_computed_every_pass_and_a_failure_in_it_is_thrown() {
    let code = swift_of("stdlib/time/utc_from_local.scxml").expect("a loop is written");
    assert!(code.contains("while true {"), "{code}");
    // The condition is read at the top of every pass, its failure thrown before it
    // decides whether the loop goes on.
    let head = code.find("while true {").expect("a loop");
    let rest = &code[head..];
    let cond = rest.find("let sceCond").expect("its condition is a local");
    let check = rest
        .find("try sceFailure.throwIfFailed()")
        .expect("its failure is read");
    let leave = rest
        .find("{ break }")
        .expect("and it leaves when the condition is false");
    assert!(cond < check && check < leave, "{code}");
    // An element is read by its index through the runtime's checked read, which
    // records an out-of-range index instead of trapping.
    assert!(code.contains("Checked.at(sceFailure,"), "{code}");
}
