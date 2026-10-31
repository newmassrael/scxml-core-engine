// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A validator (`sce:kind="validator"`, range, rate-of-change and plausibility rules over its
// inputs), written for Swift: a `final class` whose `validate` answers the runtime's
// `ValidationResult`, named by its module, and keeps the last value of each input a
// rate-of-change rule reads.
//
// A validator that imports another kind composes its unit or holds a member of it; one whose
// import Swift does not write (`swift_unlowered`) is refused by name, never written without it.

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
fn a_validator_is_a_class_answering_the_runtimes_verdict() {
    let code = swift_of("tests/forge/resources/validator_rpm_check.scxml")
        .expect("a validator is written");
    assert!(
        code.contains("public final class ValidatorRpmCheck {"),
        "{code}"
    );
    assert!(
        code.contains("private var scePrevRpm: UInt16 = 0"),
        "the kept value is a member under the generator's prefix: {code}"
    );
    assert!(
        code.contains("-> SceForgeRuntime.ValidationResult {"),
        "the verdict is the runtime's, named by its module: {code}"
    );
}

#[test]
fn the_rules_are_checked_range_first_then_rate_of_change_then_plausibility() {
    let code = swift_of("tests/forge/resources/validator_rpm_check.scxml")
        .expect("a validator is written");
    let range = code.find("rpm_out_of_range").expect("a range rule");
    let rate = code
        .find("rpm_rate_of_change_exceeded")
        .expect("a rate rule");
    let plausible = code
        .find("plausibility_failed")
        .expect("a plausibility rule");
    assert!(range < rate && rate < plausible, "{code}");
}

#[test]
fn a_signed_distance_is_taken_in_sixty_four_bits_without_a_trap() {
    let code = swift_of("tests/forge/resources/validator_signed_roc.scxml")
        .expect("a validator is written");
    assert!(
        code.contains("&- Int64(scePrevSpeed)"),
        "the distance of two signed values wraps in 64 bits and is read as unsigned: {code}"
    );
    assert!(code.contains("UInt64(bitPattern:"), "{code}");
}

#[test]
fn a_bound_at_a_types_extreme_carries_no_check() {
    let code = swift_of("tests/forge/resources/validator_integer_extremes.scxml")
        .expect("a validator is written");
    assert!(
        !code.contains("level <") && !code.contains("level >"),
        "an int8 ranged over its whole range is compared against nothing: {code}"
    );
}

/// A validator calls the unit of a stateless kind it imports, through the unit's name.
#[test]
fn a_validator_calls_the_unit_of_a_stateless_kind_it_imports() {
    let code = swift_of("tests/forge/resources/crossfile_validator_condition.scxml")
        .expect("an import of a condition is written");
    assert!(
        code.contains("ConditionThreshold.conditionThreshold("),
        "the condition is called through its unit: {code}"
    );
}

/// A validator owns the unit of a kind that keeps state it imports, a filter's smoothed
/// value being its own, and calls it by the member's name.
#[test]
fn a_validator_holds_the_unit_of_a_filter_it_imports_as_a_member() {
    let code = swift_of("tests/forge/resources/crossfile_validator_filter.scxml")
        .expect("an import of a filter is written");
    assert!(
        code.contains("private let smoother = FilterLowPass()"),
        "the filter is a member of the validator: {code}"
    );
    assert!(
        code.contains("smoother.update("),
        "the member is called by its name: {code}"
    );
}

/// A validator holds the unit of a codec it imports as a member, a frame of zeros until it is
/// given one, and reads a field of it by the member's name in its plausibility.
#[test]
fn a_validator_holds_the_unit_of_a_codec_it_imports_as_a_member() {
    let code = swift_of("tests/forge/resources/crossfile_validator_codec.scxml")
        .expect("an import of a codec is written");
    assert!(
        code.contains("private let frame = CodecSimpleFrame()"),
        "the codec is a member of the validator: {code}"
    );
    assert!(
        code.contains("frame.msgId == msgId") && code.contains("frame.payload == payload"),
        "a field of the member is read by the member's name: {code}"
    );
}
