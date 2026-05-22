//! NL→IR Mapping Roadmap Item 2 — cross-kind typed binding verification.
//!
//! Exercises the three new validators wired into
//! `compile_forge_from_parsed` after `validate_and_enrich_imports`:
//!
//! - Positive: algorithm imports a codec and references a declared
//!   field via `<alias>.<field>` — compiles clean.
//! - Negative 1 (`validation/cross-kind-field-not-found`): same shape
//!   but with a typo on the field name. Diagnostic carries the imported
//!   kind's full member surface as the `Fix::ReplaceOneOf` candidate
//!   list (`did_you_mean`-style repair).
//! - Negative 2 (`validation/cross-kind-type-mismatch`): bare
//!   `<sce:return expr="alias.field"/>` whose declared field type
//!   conflicts with the algorithm signature's `<sce:return type=...>`.
//! - Negative 3 (`validation/cross-kind-circular-dependency`): two
//!   codecs that import each other — the import-graph DFS surfaces the
//!   back-edge.
//!
//! Fixtures live in a per-test tempdir so the test stays self-contained
//! and doesn't proliferate one-off files under `tests/forge/resources/`.
//! The committed conformance fixtures (`algorithm_keyexpr_match_first`,
//! `subscription_entry`, …) cover the happy multi-doc compile path
//! separately.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::compile_forge_with_imports;
use sce_build::forge::error::ValidationError;
use sce_build::forge::model::ForgeKind;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// Minimal codec exposing two fields. Used as the imported kind in
/// every positive / negative case below.
const CODEC_SCXML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec"
       name="frame_codec"
       version="1.0">
  <datamodel>
    <sce:field id="msg_id"  sce:type="uint8"  sce:byte="0" sce:bit-size="8"/>
    <sce:field id="payload" sce:type="uint32" sce:byte="1" sce:bit-size="32"/>
  </datamodel>
</scxml>
"#;

fn write_fixture(dir: &Path, name: &str, content: &str) {
    let path = dir.join(name);
    fs::write(&path, content).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
}

fn compile_algorithm(dir: &Path, algo_name: &str) -> sce_build::generator::GeneratedOutput {
    let algo_path = dir.join(algo_name);
    let content = fs::read_to_string(&algo_path).expect("read algo");
    compile_forge_with_imports(
        &content,
        DocumentLabel::symmetric(algo_name),
        Language::Rust,
        dir,
        &ForgeCompileOptions::default(),
    )
    .expect("happy path compile succeeds")
}

fn compile_algorithm_expect_err(
    dir: &Path,
    algo_name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let algo_path = dir.join(algo_name);
    let content = fs::read_to_string(&algo_path).expect("read algo");
    // `GeneratedOutput` doesn't implement `Debug`, so `.expect_err`
    // can't be used directly — destructure manually to surface the
    // Err side.
    match compile_forge_with_imports(
        &content,
        DocumentLabel::symmetric(algo_name),
        Language::Rust,
        dir,
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("cross-kind validator must reject {algo_name}"),
        Err(e) => e,
    }
}

#[test]
fn positive_alias_field_resolves() {
    // Algorithm imports the codec and references the *declared* field
    // `msg_id` — happy path. Confirms the validator's silent-success
    // mode: when alias.field resolves, no diagnostic interrupts codegen.
    let dir = tempdir().expect("tempdir");
    write_fixture(dir.path(), "frame_codec.scxml", CODEC_SCXML);
    write_fixture(
        dir.path(),
        "algo_positive.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="route_msg"
       version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="frame" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="frame.msg_id === 1"/>
  </sce:body>
</scxml>
"#,
    );
    let output = compile_algorithm(dir.path(), "algo_positive.scxml");
    assert!(
        !output.files.is_empty(),
        "happy-path compile must produce output files"
    );
}

#[test]
fn negative_field_not_found_emits_did_you_mean_candidates() {
    // Same shape as positive but with a typo: `msg_idd` instead of
    // `msg_id`. Validator emits `CrossKindFieldNotFound` with the
    // imported codec's full member surface (`msg_id`, `payload`) as
    // the closed `Fix::ReplaceOneOf` candidate list — drives the
    // `did_you_mean`-style repair surface upstream consumers
    // (IDE / NL→IR pipelines / human authors) wire off.
    let dir = tempdir().expect("tempdir");
    write_fixture(dir.path(), "frame_codec.scxml", CODEC_SCXML);
    write_fixture(
        dir.path(),
        "algo_typo.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="route_msg"
       version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="frame" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="frame.msg_idd === 1"/>
  </sce:body>
</scxml>
"#,
    );
    let err = compile_algorithm_expect_err(dir.path(), "algo_typo.scxml");
    match err.error {
        sce_build::forge::error::ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindFieldNotFound {
                importing_kind,
                alias,
                field,
                imported_kind,
                candidates,
                ..
            } => {
                assert_eq!(importing_kind, ForgeKind::Algorithm);
                assert_eq!(alias, "frame");
                assert_eq!(field, "msg_idd");
                assert_eq!(imported_kind, ForgeKind::Codec);
                assert_eq!(
                    candidates,
                    vec!["msg_id".to_string(), "payload".to_string()],
                    "candidate set must be the imported codec's full sorted member surface"
                );
            }
            other => panic!("expected CrossKindFieldNotFound, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn negative_type_mismatch_on_bare_return_expression() {
    // Algorithm declares `<sce:return type="bool"/>` but the body
    // returns `frame.msg_id` (uint8). The bare Member access shape
    // qualifies for the type-mismatch check — composite expressions
    // (`frame.msg_id != 0`) carry implicit boolean promotion and
    // resolve via the typed-expression pipeline at transpile time;
    // this validator only catches the structurally-obvious mismatch.
    let dir = tempdir().expect("tempdir");
    write_fixture(dir.path(), "frame_codec.scxml", CODEC_SCXML);
    write_fixture(
        dir.path(),
        "algo_type_mismatch.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="route_msg"
       version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="frame" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:return expr="frame.msg_id"/>
  </sce:body>
</scxml>
"#,
    );
    let err = compile_algorithm_expect_err(dir.path(), "algo_type_mismatch.scxml");
    match err.error {
        sce_build::forge::error::ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindTypeMismatch {
                importing_kind,
                alias,
                field,
                actual,
                expected,
                ..
            } => {
                assert_eq!(importing_kind, ForgeKind::Algorithm);
                assert_eq!(alias, "frame");
                assert_eq!(field, "msg_id");
                assert_eq!(actual, "uint8");
                assert_eq!(expected, "bool");
            }
            other => panic!("expected CrossKindTypeMismatch, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn negative_circular_import_dependency() {
    // Two codecs that mutually import each other — the import-graph
    // DFS surfaces the back-edge with the cycle path rendered in
    // traversal order. Without this check the surface-table builder
    // would recurse into infinite open-file work.
    //
    // The cycle is structural (alias.field references not required —
    // the cycle detector runs before the field walker).
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "cycle_a.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec"
       name="cycle_a"
       version="1.0">
  <sce:import src="cycle_b.scxml" kind="codec" as="b"/>
  <datamodel>
    <sce:field id="x" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#,
    );
    write_fixture(
        dir.path(),
        "cycle_b.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec"
       name="cycle_b"
       version="1.0">
  <sce:import src="cycle_a.scxml" kind="codec" as="a"/>
  <datamodel>
    <sce:field id="y" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#,
    );
    let err = compile_algorithm_expect_err(dir.path(), "cycle_a.scxml");
    match err.error {
        sce_build::forge::error::ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindCircularDependency { cycle } => {
                // Cycle traversal starts at the root caller's import
                // edge, walks `cycle_a → cycle_b`, then the back-edge
                // re-enters `cycle_a`. Two `cycle_a` entries close
                // the loop.
                assert!(
                    cycle.iter().any(|p| p.ends_with("cycle_a.scxml")),
                    "cycle path must include cycle_a.scxml; got: {cycle:?}"
                );
                assert!(
                    cycle.iter().any(|p| p.ends_with("cycle_b.scxml")),
                    "cycle path must include cycle_b.scxml; got: {cycle:?}"
                );
                assert!(
                    cycle.len() >= 2,
                    "cycle must have at least two nodes; got: {cycle:?}"
                );
            }
            other => panic!("expected CrossKindCircularDependency, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn silent_when_alias_resolves_to_known_field_inside_nested_expression() {
    // Defensive regression test: a member access nested inside a
    // larger expression (Binary, Call, etc.) must still pass the
    // typed-binding check when the field resolves. Without the
    // recursive walker, a positive nested case would silently fail
    // through to codegen and miss the validator entirely.
    let dir = tempdir().expect("tempdir");
    write_fixture(dir.path(), "frame_codec.scxml", CODEC_SCXML);
    write_fixture(
        dir.path(),
        "algo_nested.scxml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="route_msg"
       version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="x" type="uint8"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body>
    <sce:if cond="(frame.msg_id &amp; 0xF0) === 0x10">
      <sce:return expr="true"/>
    </sce:if>
    <sce:return expr="false"/>
  </sce:body>
</scxml>
"#,
    );
    let output = compile_algorithm(dir.path(), "algo_nested.scxml");
    assert!(
        !output.files.is_empty(),
        "nested-positive compile must produce output files"
    );
}
