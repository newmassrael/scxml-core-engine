// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A filter (`sce:kind="filter"`, a signal filter that holds its history), written for Swift: a
// `final class` whose `update` reads one sample and whose `reset` forgets what it has seen,
// holding the runtime's own filter as a member. The runtime's class is named by its module, so
// a document named for one of them (`moving_average`) cannot hide it.

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
fn a_moving_average_holds_the_runtimes_class_named_by_its_module() {
    let code =
        swift_of("tests/forge/resources/filter_moving_average.scxml").expect("a filter is written");
    assert!(
        code.contains("public final class FilterMovingAverage {"),
        "{code}"
    );
    assert!(
        code.contains("private let sceImpl = SceForgeRuntime.MovingAverage(window: 5)"),
        "the window is the document's, the class the runtime's: {code}"
    );
    assert!(
        code.contains("public func update(_ rawTemp: Double) -> Double {"),
        "{code}"
    );
    assert!(code.contains("public func reset() {"), "{code}");
}

#[test]
fn a_low_pass_holds_its_alpha() {
    let code =
        swift_of("tests/forge/resources/filter_low_pass.scxml").expect("a filter is written");
    assert!(
        code.contains("SceForgeRuntime.LowPass(alpha: 0.1)"),
        "{code}"
    );
}

#[test]
fn a_debounce_is_over_the_type_it_answers() {
    let code =
        swift_of("tests/forge/resources/filter_debounce.scxml").expect("a filter is written");
    assert!(
        code.contains("SceForgeRuntime.Debounce<Bool>(window: 3)"),
        "{code}"
    );
    assert!(
        code.contains("sceImpl.update(Bool(rawButton))"),
        "the sample is handed over as the type the latch holds: {code}"
    );
}
