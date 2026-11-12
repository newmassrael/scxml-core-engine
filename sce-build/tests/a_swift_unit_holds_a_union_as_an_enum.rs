// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Swift lowers a union (docs/adr/0092, section 7, the Swift row), held to the
//! documents under `sce-build/tests/fixtures/union/`:
//!
//! * `union_shape_minimal.scxml` generates an enum named for the document,
//!   `Equatable` and `Sendable`, one case per variant of the tag's enum in the
//!   enum's order (not the document's), holding the case's record or nothing for
//!   an empty case; `kind` on it, and one `proven<Case>` per case that holds a
//!   record. Beside it, as members of the enum, `revision` (the tag enum's),
//!   `dataMaxLen`, `schemaID` and `jsonSchema` (section 3.1, item 4): the units
//!   of one module never share a top-level name.
//! * `event_shapes_minimal.scxml`, whose fields hold the union alone, in a list
//!   and optionally, generates a payload struct that names the enum at each
//!   depth. No field holds a value nobody wrote (docs/adr/0098, section 4), so
//!   the payload is built whole by its initializer of every field.
//! * The units, with the enum and the records they name, type-check together
//!   under `swiftc` with a file that builds every case, reads one through its
//!   `proven` read and matches a value exhaustively.
//!
//! A case whose Swift name is another's is refused by name, as is a case that
//! holds a record no unit declares a type for.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::ForgeCompileOptions;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/union")
}

/// What Swift generates for the document at `path`.
fn swift(path: &Path) -> Result<GeneratedOutput, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Swift, &[], &ForgeCompileOptions::default())
}

/// The one Swift file the fixture `name` generates, as `(file, text)`.
fn unit(name: &str) -> (String, String) {
    let output = swift(&fixtures().join(name))
        .unwrap_or_else(|e| panic!("{name} does not generate on Swift: {}", e.error));
    let mut files: Vec<(String, String)> = output
        .files
        .into_iter()
        .filter(|(file, _)| file.ends_with(".swift"))
        .collect();
    assert_eq!(files.len(), 1, "{name}: one Swift unit, got {files:?}");
    files.remove(0)
}

/// Where `needle` first stands in `text`, which must hold it.
fn position(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("the unit holds `{needle}`:\n{text}"))
}

#[test]
fn a_union_is_an_enum_with_one_case_per_variant_in_the_enums_order() {
    let (file, text) = unit("union_shape_minimal.scxml");
    assert_eq!(file, "UnionShapeMinimal.swift");
    for fragment in [
        "public enum UnionShapeMinimal: Equatable, Sendable {",
        "    case dot\n",
        "    case circle(ShapeRadiusMinimalPayload)\n",
        "    case rect(ShapeRectMinimalPayload)\n",
        "    public var kind: ShapeKind {",
        "        case .dot:\n            return ShapeKind.dot\n",
        "        case .circle:\n            return ShapeKind.circle\n",
        "        case .rect:\n            return ShapeKind.rect\n",
        "    public var provenCircle: ShapeRadiusMinimalPayload {",
        "guard case .circle(let value) = self else {",
        "    public var provenRect: ShapeRectMinimalPayload {",
        "guard case .rect(let value) = self else {",
    ] {
        position(&text, fragment);
    }
    // An empty case has nothing to read.
    assert!(!text.contains("provenDot"), "{text}");
    // The document writes rect, dot, circle; the enum declares dot, circle,
    // rect, and the cases follow the enum.
    let dot = position(&text, "    case dot\n");
    let circle = position(&text, "    case circle(");
    let rect = position(&text, "    case rect(");
    assert!(dot < circle && circle < rect, "{text}");
    // Every case is declared before the tag and the reads that name them.
    assert!(rect < position(&text, "public var kind"), "{text}");
    assert!(
        position(&text, "public var kind") < position(&text, "public var provenCircle"),
        "{text}"
    );
}

/// The constants of a union's unit (docs/adr/0092, section 3.1, item 4): its
/// revision is its tag enum's (section 8.1), and its largest text is its
/// largest case, `{"kind":"rect","width":65535,"height":65535}`.
#[test]
fn a_union_unit_names_its_revision_its_largest_text_and_its_schema() {
    let (_, text) = unit("union_shape_minimal.scxml");
    for fragment in [
        "public static let revision: UInt32 = 1\n",
        "public static let dataMaxLen: UInt64 = 44\n",
        "public static let schemaID: String = \"urn:sce:schema:union_shape_minimal:1\"\n",
        "public static let jsonSchema: String = \"",
    ] {
        position(&text, fragment);
    }
    assert!(text.contains("oneOf"), "{text}");
    assert!(!text.contains("No jsonSchema"), "{text}");
    // A member of the enum, not a top-level name of a module that holds many
    // units.
    let enum_start = position(&text, "public enum UnionShapeMinimal");
    assert!(
        enum_start < position(&text, "public static let revision"),
        "{text}"
    );
}

#[test]
fn a_payload_names_the_enum_at_every_depth_and_is_built_whole() {
    let (file, text) = unit("event_shapes_minimal.scxml");
    assert_eq!(file, "EventShapesMinimal.swift");
    for fragment in [
        "public struct EventShapesMinimalPayload: Equatable, Sendable {",
        "    public var one: UnionShapeMinimal\n",
        "    public var many: [UnionShapeMinimal]\n",
        "    public var maybe: UnionShapeMinimal?\n",
        "public init(one: UnionShapeMinimal, many: [UnionShapeMinimal], maybe: UnionShapeMinimal?) {",
        "public static let manyMaxCount: UInt32 = 3\n",
    ] {
        position(&text, fragment);
    }
    // No type has a zero: no `init()` that would stand for one, and no field
    // starts at a value nobody wrote.
    assert!(!text.contains("public init() {}"), "{text}");
    assert!(!text.contains("= []"), "{text}");
    assert!(!text.contains("= nil"), "{text}");
}

/// A file that uses every part of the union's unit: a case of each kind built,
/// a payload that holds the union at each depth, a `proven` read, the tag, and
/// a `switch` with no `default` that the compiler holds to every case.
const PROBE: &str = r#"func named(_ shape: UnionShapeMinimal) -> String {
    switch shape {
    case .dot:
        return "dot"
    case .circle(let record):
        return "circle of \(record.radius)"
    case .rect(let record):
        return "rect of \(record.width) by \(record.height)"
    }
}

func radiusOf(_ shape: UnionShapeMinimal) -> UInt16 {
    shape.provenCircle.radius
}

func tagOf(_ shape: UnionShapeMinimal) -> ShapeKind {
    shape.kind
}

func sent() -> EventShapesMinimalPayload {
    EventShapesMinimalPayload(
        one: .circle(ShapeRadiusMinimalPayload(radius: 2)),
        many: [.rect(ShapeRectMinimalPayload(width: 3, height: 4)), .dot],
        maybe: nil
    )
}
"#;

/// The part of the forge runtime a unit may name: the failure a may-fail
/// algorithm throws (docs/adr/0098). A stand-in module is enough to type-check
/// the units against; the runtime's own tests hold the real one.
const RUNTIME_STAND_IN: &str = "public enum AlgorithmError: Error {\n    case overflow\n}\n";

#[test]
fn the_swift_of_a_union_and_the_payload_that_holds_it_type_checks() {
    let Some(swiftc) =
        sce_build::toolchain::require_any_or_skip(&["swiftc"], "type-check a generated union")
    else {
        return;
    };
    let dir = tempfile::tempdir().expect("a scratch directory");
    let runtime = dir.path().join("SceForgeRuntime.swift");
    std::fs::write(&runtime, RUNTIME_STAND_IN).expect("write the stand-in runtime");
    let modules = dir.path().join("modules");
    std::fs::create_dir(&modules).expect("a module directory");
    let built = Command::new(&swiftc)
        .args([
            "-emit-module",
            "-module-name",
            "SceForgeRuntime",
            "-parse-as-library",
        ])
        .arg("-emit-module-path")
        .arg(modules.join("SceForgeRuntime.swiftmodule"))
        .arg(&runtime)
        .output()
        .expect("swiftc runs");
    assert!(
        built.status.success(),
        "the stand-in runtime does not build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let mut sources = Vec::new();
    for name in [
        "shape_kind.scxml",
        "shape_radius_minimal.scxml",
        "shape_rect_minimal.scxml",
        "union_shape_minimal.scxml",
        "event_shapes_minimal.scxml",
    ] {
        let (file, text) = unit(name);
        let path = dir.path().join(file);
        std::fs::write(&path, text).expect("write the unit");
        sources.push(path);
    }
    let probe = dir.path().join("Probe.swift");
    std::fs::write(&probe, PROBE).expect("write the probe");
    sources.push(probe);
    let out = Command::new(&swiftc)
        .args([
            "-typecheck",
            "-warnings-as-errors",
            "-swift-version",
            "6",
            "-I",
        ])
        .arg(&modules)
        .args(&sources)
        .output()
        .expect("swiftc runs");
    assert!(
        out.status.success(),
        "the generated Swift does not type-check:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The refusal Swift gives the union `union` written beside `documents`, each
/// `(file, text)`, and the record of `shape_radius_minimal.scxml`; its detail.
fn refused(union: &str, documents: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().expect("a scratch directory");
    std::fs::copy(
        fixtures().join("shape_radius_minimal.scxml"),
        dir.path().join("shape_radius_minimal.scxml"),
    )
    .expect("copy the record");
    for (file, text) in documents {
        std::fs::write(dir.path().join(file), text).expect("write a document");
    }
    let path = dir.path().join("probe_union.scxml");
    std::fs::write(&path, union).expect("write the union");
    let err = match swift(&path) {
        Ok(_) => panic!("Swift generated the union"),
        Err(refusal) => refusal,
    };
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

/// A union document `probe_union` whose tag is a value of the enum in
/// `enum_file`, which imports `shape_radius_minimal.scxml` as `Radius` and the
/// further imports `imports`, and whose cases are `cases`.
fn union_document(enum_file: &str, imports: &str, cases: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="union" name="probe_union">
  <sce:import kind="enum" src="{enum_file}" as="Kind"/>
  <sce:import kind="event-schema" src="shape_radius_minimal.scxml" as="Radius"/>
{imports}  <datamodel>
    <data id="kind" sce:type="enum:Kind">
{cases}    </data>
  </datamodel>
</scxml>
"#
    )
}

/// Two variants an enum tells apart, `two_part` and `twoPart`, are one Swift
/// case in lower camel case, which two cases cannot both be. (`dot` and `Dot`
/// are two: Swift tells a case's letters apart, and spells a variant as the
/// tag's own enum does.)
#[test]
fn two_cases_of_one_swift_name_are_refused_by_name() {
    let pair = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="pair_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="two_part" value="0"/>
      <sce:variant name="twoPart" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;
    let detail = refused(
        &union_document(
            "pair_kind.scxml",
            "",
            "      <sce:case variant=\"two_part\"/>\n      <sce:case variant=\"twoPart\" record=\"Radius\"/>\n",
        ),
        &[("pair_kind.scxml", pair)],
    );
    assert!(
        detail.contains("the cases `two_part` and `twoPart` are both the Swift case `twoPart`"),
        "{detail}"
    );
}

/// A record of no field has no payload struct (`carries_payload`), so a case
/// that holds one would name a type no unit declares.
#[test]
fn a_case_that_holds_a_record_of_no_field_is_refused_by_name() {
    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="shape_empty" sce:event-name="shape.empty">
  <datamodel sce:payload="none"/>
</scxml>
"#;
    let kinds = std::fs::read_to_string(fixtures().join("shape_kind.scxml")).expect("the enum");
    let detail = refused(
        &union_document(
            "shape_kind.scxml",
            "  <sce:import kind=\"event-schema\" src=\"shape_empty.scxml\" as=\"Empty\"/>\n",
            "      <sce:case variant=\"dot\"/>\n      <sce:case variant=\"circle\" record=\"Radius\"/>\n      <sce:case variant=\"rect\" record=\"Empty\"/>\n",
        ),
        &[("shape_kind.scxml", kinds.as_str()), ("shape_empty.scxml", empty)],
    );
    assert!(
        detail.contains("the case `rect` holds `Empty`, which declares no field"),
        "{detail}"
    );
}
