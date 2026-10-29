// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// An enum document (`sce:kind="enum"`) written for Swift: a closed enum over the
// document's own carrier, each case holding the number the document declares for it
// (which is not the variant's position), named lowerCamelCase, with the name the
// document gives it and the two conversions every backend's enum answers. An open
// enum (`sce:strict-variants="false"`) holds a value no variant declares, which a
// Swift enum cannot, so it is refused by name.

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

const STATUS: &str = "sce-build/tests/fixtures/static_datamodel/enum_item_status.scxml";

#[test]
fn a_closed_enum_is_an_enum_over_its_carrier_holding_the_declared_numbers() {
    let code = swift_of(STATUS).expect("a closed enum is written");
    assert!(
        code.contains("public enum EnumItemStatus: UInt8, CaseIterable, Sendable {"),
        "{code}"
    );
    // The values are the document's, not the variants' positions.
    assert!(code.contains("case open = 2"), "{code}");
    assert!(code.contains("case inProgress = 5"), "{code}");
    assert!(code.contains("case done = 7"), "{code}");
}

#[test]
fn a_value_is_named_and_converted_by_what_the_document_declares() {
    let code = swift_of(STATUS).expect("a closed enum is written");
    // The name is the document's spelling, with its underscore, not the case's.
    assert!(
        code.contains("case .inProgress: return \"in_progress\""),
        "{code}"
    );
    assert!(
        code.contains("public init?(declaredName: String)"),
        "{code}"
    );
    assert!(
        code.contains("public static func fromUnderlying(_ raw: UInt8) -> EnumItemStatus?"),
        "{code}"
    );
    assert!(
        code.contains("public static func toUnderlying(_ value: EnumItemStatus) -> UInt8"),
        "{code}"
    );
}

#[test]
fn a_field_of_an_enum_is_the_unit_of_the_enum_at_its_first_case() {
    let code = swift_of("sce-build/tests/fixtures/static_datamodel/schema_item.scxml")
        .expect("a schema whose field is an enum is written");
    assert!(
        code.contains("public var status: EnumItemStatus = EnumItemStatus.allCases[0]"),
        "{code}"
    );
    // The memberwise initializer takes the field in the enum's type.
    assert!(
        code.contains("public init(id: UInt32, status: EnumItemStatus)"),
        "{code}"
    );
}

#[test]
fn an_algorithm_takes_a_value_of_an_enum_as_the_unit_of_the_enum() {
    let code =
        swift_of("sce-build/tests/fixtures/static_datamodel/algorithm_item_status_set.scxml")
            .expect("an algorithm over an enum is written");
    assert!(
        code.contains("_ status: EnumItemStatus) throws -> [SchemaItemPayload]"),
        "{code}"
    );
}

#[test]
fn a_schema_whose_field_holds_an_enum_swift_does_not_write_is_refused_by_name() {
    // The field is held in the unit of the enum, which is not written for an open
    // set of variants: the schema is refused for it, and not left to name a type that
    // does not exist.
    let refusal = swift_of("sce-build/tests/fixtures/event_schema/schema_uds_response_open.scxml")
        .expect_err("a schema over an open enum is refused");
    assert!(
        refusal.contains("an enum with an open set of variants") && refusal.contains("Swift"),
        "{refusal}"
    );
}

#[test]
fn an_open_enum_is_refused_by_name() {
    let refusal = swift_of("sce-build/tests/fixtures/event_schema/uds_nrc_open.scxml")
        .expect_err("an open enum is not lowered for Swift");
    assert!(
        refusal.contains("open set of variants") && refusal.contains("Swift"),
        "{refusal}"
    );
}
