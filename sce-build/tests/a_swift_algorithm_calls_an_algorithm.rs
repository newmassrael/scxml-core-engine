// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An algorithm that calls another (`<sce:import kind="algorithm">`, SCE_FORGE.md
// §4.12), written for Swift: the callee is a static member of the enum its own unit
// declares, in the same module, so nothing is imported to name it. A callee that may
// fail has its failure passed on by the caller's own `Failure`.
//
// The unit a call names is written only when the callee is itself written for Swift,
// so a caller is refused for what its callee holds, by name, and never left to name a
// unit nothing writes.

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
fn a_callee_is_a_member_of_the_enum_its_own_unit_declares() {
    let code = swift_of("stdlib/merge/lww_write.scxml").expect("a call of an algorithm is written");
    assert!(
        code.contains("HlcCompare.hlcCompare(incoming, stored)"),
        "{code}"
    );
}

#[test]
fn a_callee_that_may_fail_has_its_failure_passed_on() {
    let code = swift_of("stdlib/time/add_months.scxml").expect("a may-fail call is written");
    // The callee throws; the caller records it in its own `Failure` under the
    // callee's name, which the statement around the call then throws.
    assert!(
        code.contains(
            "Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(year, month) })"
        ),
        "{code}"
    );
}

const DOCUMENT_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" sce:kind="algorithm""#;

#[test]
fn a_caller_is_refused_for_what_its_callee_holds() {
    // The callee carries a `<sce:test-vector>`, which Swift does not write yet, so no
    // unit is written for it: the caller would name a unit that does not exist.
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("tests/forge/resources/algorithm_crc16.scxml"),
        dir.path().join("crc16.scxml"),
    )
    .expect("copy the callee");
    std::fs::write(
        dir.path().join("calls_crc16.scxml"),
        format!(
            r#"{DOCUMENT_HEAD} name="calls_crc16" version="1.0">
  <sce:import kind="algorithm" src="crc16.scxml" as="crc"/>
  <sce:signature>
    <sce:param name="data" type="bytes"/>
    <sce:return type="uint16"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="crc(data)"/>
  </sce:body>
</scxml>
"#
        ),
    )
    .expect("write the caller");
    let output = compile_forge_file(
        &dir.path().join("calls_crc16.scxml"),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    );
    let refusal = format!(
        "{:?}",
        output
            .err()
            .expect("a caller of an algorithm Swift does not write is refused")
    );
    assert!(
        refusal.contains("an algorithm that"),
        "the refusal names that it is the callee: {refusal}"
    );
    assert!(
        refusal.contains("unsupported-feature") || refusal.contains("UnsupportedFeature"),
        "{refusal}"
    );
}
