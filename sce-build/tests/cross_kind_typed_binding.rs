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
                // The document compiled is on the walk from the start, so
                // the cycle starts and ends at it: the import in cycle_b
                // that comes back to cycle_a closes it. It read
                // `cycle_b → cycle_a → cycle_b` while the entry was not on
                // the stack and was read again as an import.
                assert_eq!(
                    cycle,
                    vec!["cycle_a.scxml", "cycle_b.scxml", "cycle_a.scxml"],
                    "the cycle runs from the document compiled"
                );
            }
            other => panic!("expected CrossKindCircularDependency, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}

/// A codec named `name` importing each of `imports` (`src`, `as`).
fn codec_importing(name: &str, imports: &[(&str, &str)]) -> String {
    let imports: String = imports
        .iter()
        .map(|(src, alias)| {
            format!("  <sce:import src=\"{src}\" kind=\"codec\" as=\"{alias}\"/>\n")
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec"
       name="{name}"
       version="1.0">
{imports}  <datamodel>
    <sce:field id="x" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
  </datamodel>
</scxml>
"#
    )
}

/// A cycle below the document compiled is the documents on it and no
/// others, whichever spelling of a document's `src` reached it.
///
/// `top` imports `a`, which imports `./b.scxml`, and `c` comes back to
/// `b.scxml`. The cycle is `b → c → b`. Found by comparing `src` spellings,
/// `b.scxml` matched nothing on the stack and the whole stack was taken,
/// `a` included.
#[test]
fn a_cycle_holds_only_its_documents_whatever_spelling_reached_them() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "top.scxml",
        &codec_importing("top", &[("a.scxml", "a")]),
    );
    write_fixture(
        dir.path(),
        "a.scxml",
        &codec_importing("a", &[("./b.scxml", "b")]),
    );
    write_fixture(
        dir.path(),
        "b.scxml",
        &codec_importing("b", &[("c.scxml", "c")]),
    );
    write_fixture(
        dir.path(),
        "c.scxml",
        &codec_importing("c", &[("b.scxml", "b")]),
    );
    let err = compile_expect_err(dir.path(), "top.scxml");
    assert_eq!(
        err.location.file, "c.scxml",
        "the import that closes the cycle"
    );
    assert_eq!(err.location.line, Some(7), "the row of c's import");
    match err.error {
        sce_build::forge::error::ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindCircularDependency { cycle } => {
                assert_eq!(cycle, vec!["./b.scxml", "c.scxml", "b.scxml"]);
            }
            other => panic!("expected CrossKindCircularDependency, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}

/// The document compiled is on the walk when a run holds it in memory rather
/// than on disk, as a plan or a child generation does
/// (`sce_build::confine::with_documents`): an import that comes back to it
/// reads the held text, so the cycle runs from it as it does for a file. It
/// read `held_b -> held_a -> held_b` while the entry was looked for on disk
/// alone and was read again as an import.
#[test]
fn a_cycle_through_a_document_held_in_memory_runs_from_it() {
    let dir = tempdir().expect("tempdir");
    write_fixture(
        dir.path(),
        "held_b.scxml",
        &codec_importing("held_b", &[("held_a.scxml", "a")]),
    );
    let entry = codec_importing("held_a", &[("held_b.scxml", "b")]);
    let held = vec![(dir.path().join("held_a.scxml"), entry.clone())];
    let refused = sce_build::confine::with_documents(held, || {
        compile_forge_with_imports(
            &entry,
            DocumentLabel::symmetric("held_a.scxml"),
            Language::Rust,
            dir.path(),
            &ForgeCompileOptions::default(),
        )
        .err()
    });
    let Some(err) = refused else {
        panic!("held_a.scxml imports itself through held_b.scxml and was generated");
    };
    match err.error {
        sce_build::forge::error::ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindCircularDependency { cycle } => {
                assert_eq!(
                    cycle,
                    vec!["held_a.scxml", "held_b.scxml", "held_a.scxml"],
                    "the cycle runs from the document compiled"
                );
            }
            other => panic!("expected CrossKindCircularDependency, got {other:?}"),
        },
        other => panic!("expected Validation, got {other:?}"),
    }
}
