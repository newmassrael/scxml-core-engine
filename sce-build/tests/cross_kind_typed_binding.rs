//! NL→IR Mapping Roadmap Item 2 — cross-kind typed binding verification.
//!
//! What this file holds in place now:
//!
//! - In an algorithm, an import's alias is not a value. An algorithm is a
//!   free function with no instance state, so nothing binds a codec
//!   import's alias: every `frame.<member>` — a declared field, a
//!   misspelled one, a bare return, a read nested in a larger expression —
//!   is refused as the undeclared name `frame`, which is the one repair
//!   that applies to all of them.
//! - `validation/cross-kind-circular-dependency`: two codecs that import
//!   each other — the import-graph DFS surfaces the back-edge.
//!
//! ⚠ THE FIELD CHECK THAT USED TO LIVE HERE. A cross-kind validator walked
//! algorithm bodies and checked each `alias.field` against the imported
//! kind's fields, refusing `frame.msg_idd` with the codec's members as
//! candidates and a bare `return frame.msg_id` against a mismatched return
//! type. Both answered a question that did not arise: accepting the field
//! only led to the alias being refused next, and correcting a typo the
//! validator named still left a name that would never resolve. Where the
//! alias IS a value — a procedure, say — the validator never looked, and a
//! misspelled field generated with exit 0 (measured 2026-09-21). The member
//! check now lives in the expression layer, over every kind; its cases are
//! in `a_record_member_is_one_the_record_declares.rs`.
//!
//! Fixtures live in a per-test tempdir so the test stays self-contained
//! and doesn't proliferate one-off files under `tests/forge/resources/`.

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use sce_build::compile_forge_with_imports;
use sce_build::forge::error::ValidationError;
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// Minimal codec exposing two fields. Used as the imported kind below.
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

fn compile_expect_err(
    dir: &Path,
    name: &str,
) -> sce_build::forge::error::Located<sce_build::forge::error::ForgeError> {
    let content = fs::read_to_string(dir.join(name)).expect("read document");
    // `GeneratedOutput` doesn't implement `Debug`, so `.expect_err`
    // can't be used directly — destructure manually to surface the
    // Err side.
    match compile_forge_with_imports(
        &content,
        DocumentLabel::symmetric(name),
        Language::Rust,
        dir,
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("{name} must be refused, and generated"),
        Err(e) => e,
    }
}

/// An algorithm importing the codec as `frame`, returning `ret_type`, whose
/// body is `body`.
fn algorithm(ret_type: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="algorithm"
       name="route_msg"
       version="1.0">
  <sce:import src="frame_codec.scxml" kind="codec" as="frame"/>
  <sce:signature>
    <sce:param name="x" type="uint8"/>
    <sce:return type="{ret_type}"/>
  </sce:signature>
  <sce:body>
    {body}
  </sce:body>
</scxml>
"#
    )
}

/// Every read of the alias in an algorithm is refused as the undeclared
/// name, whatever member it asks for and wherever it sits.
///
/// ⚠ The first fixture used to declare `<sce:param name="frame"
/// type="uint8"/>` beside the import of the same name, so the name was
/// "declared" — as a byte — and the emitted `frame.msg_id` read a field of
/// a `u8`. A parameter shadowing an import alias is its own defect, refused
/// by the namespace check, and is not what this test is for.
#[test]
fn an_imports_alias_is_not_a_value_in_an_algorithm() {
    let cases = [
        (
            "declared_field",
            "bool",
            r#"<sce:return expr="frame.msg_id === 1"/>"#,
        ),
        (
            "misspelled_field",
            "bool",
            r#"<sce:return expr="frame.msg_idd === 1"/>"#,
        ),
        (
            "bare_return",
            "bool",
            r#"<sce:return expr="frame.msg_id"/>"#,
        ),
        (
            "nested",
            "bool",
            r#"<sce:if cond="(frame.msg_id &amp; 0xF0) === 0x10">
      <sce:return expr="true"/>
    </sce:if>
    <sce:return expr="false"/>"#,
        ),
    ];
    for (label, ret_type, body) in cases {
        let dir = tempdir().expect("tempdir");
        write_fixture(dir.path(), "frame_codec.scxml", CODEC_SCXML);
        let name = format!("algo_{label}.scxml");
        write_fixture(dir.path(), &name, &algorithm(ret_type, body));
        let err = compile_expect_err(dir.path(), &name);
        use sce_build::forge::error::{ExprError, ForgeError};
        match &err.error {
            ForgeError::Expression(ExprError::UnknownIdentifier { name, .. }) => {
                assert_eq!(
                    name, "frame",
                    "{label}: the undeclared name must be the alias"
                );
            }
            other => panic!("{label}: expected the alias refused as undeclared, got {other:?}"),
        }
    }
}

#[test]
fn negative_circular_import_dependency() {
    // Two codecs that mutually import each other — the import-graph
    // DFS surfaces the back-edge with the cycle path rendered in
    // traversal order. Without this check every walk of the import
    // closure would recurse without end.
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
    let err = compile_expect_err(dir.path(), "cycle_a.scxml");
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
