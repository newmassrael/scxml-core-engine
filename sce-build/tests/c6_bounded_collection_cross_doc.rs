//! Bounded-collection cross-doc resolution integration tests.
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-L lines 2566-2567 + 2615 + 2560-2562:
//! three failure axes against the build's forge-doc set, exercised
//! through the `compile_scxml_with_imports` orchestrator (the validator
//! cannot be exercised through `compile_forge_with_imports` because
//! the cross-doc layer is orchestrator-only — single-file callers do
//! not assemble the element-type candidate map or aggregate the
//! build's `<sce:extern>` declarations).
//!
//! Test matrix (8 scenarios):
//!  1. happy_codec_element_type_no_index_by
//!  2. happy_codec_element_type_with_valid_index_by
//!  3. happy_procedure_element_type
//!  4. happy_multi_writer_with_atomic_extern
//!  5. element_type_not_a_kind_unknown_fires
//!  6. element_type_not_a_kind_resolves_to_link_fires
//!  7. index_by_field_missing_fires
//!  8. multi_writer_without_atomic_extern_fires
//!
//! Existing `c6_bounded_collection.rs` (parse-time scope) is left untouched.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, GenerateError, ValidationError};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn default_options() -> ForgeCompileOptions {
    ForgeCompileOptions::default()
}

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

fn write_doc(dir: &Path, basename: &str, content: &str) -> PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write doc");
    path
}

fn run_orchestrator(
    scxml_files: &[&Path],
    forge_files: &[&Path],
) -> Result<
    Vec<(String, sce_build::generator::GeneratedOutput)>,
    sce_build::forge::error::Located<ForgeError>,
> {
    compile_scxml_with_imports(
        scxml_files,
        forge_files,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
}

/// Happy-path assertion helper. The cross-doc validator runs before
/// codegen (pass-3). With the bounded-collection codegen template
/// shipped, the happy path is `Ok(_)`; the
/// `CodegenGenericKindBackendEmitMissing { kind: "bounded-collection" }`
/// arm is also accepted because reaching that downstream error still
/// proves the validator silent-passed.
fn assert_validator_silent_passed(
    result: Result<
        Vec<(String, sce_build::generator::GeneratedOutput)>,
        sce_build::forge::error::Located<ForgeError>,
    >,
) {
    match result {
        Ok(_) => {} // Validator pass + codegen succeeded.
        Err(located) => match &located.error {
            ForgeError::Generate(boxed) => match boxed.as_ref() {
                GenerateError::CodegenGenericKindBackendEmitMissing { kind, .. }
                    if kind == "bounded-collection" =>
                {
                    // Codegen reached the BC emit site, which
                    // proves the cross-doc validator passed (a
                    // validator failure would short-circuit before
                    // codegen).
                }
                other => {
                    panic!("validator must silent-pass; got an unrelated error: {other:?}")
                }
            },
            other => panic!("validator must silent-pass; got an unrelated error: {other:?}"),
        },
    }
}

/// Codec doc with two fields; both field ids are exposed via
/// `discover_stateful_member_fields`'s codec arm so the cross-doc
/// index-by validator can enumerate them.
fn codec_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="{name}" version="1.0">
  <datamodel>
    <sce:field id="key_expr_id" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
    <sce:field id="callback_id" sce:type="uint32" sce:byte="4" sce:bit-size="32"/>
  </datamodel>
</scxml>"##
    )
}

/// Procedure doc with one input + one internal field. Both ids enter
/// `discover_stateful_member_fields`'s procedure arm via inputs +
/// internals concatenation.
fn procedure_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="procedure" name="{name}" version="1.0" initial="run">
  <datamodel>
    <data id="trigger" sce:type="uint32" sce:direction="in"/>
    <data id="counter" sce:type="uint32" sce:direction="internal" expr="0"/>
  </datamodel>
  <state id="run">
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>"##
    )
}

/// Link doc — used only to prove that a same-name forge doc of a
/// non-element-type kind is NOT in the candidate map (the registry
/// rejects same-name across kinds, so this is also a name-collision
/// scenario; we test the "wrong kind" axis by naming the link
/// differently and having the BC point at the link's name).
fn link_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:import as="scout_frame_codec" src="scout_frame_codec.scxml" kind="codec"/>
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##
    )
}

/// Stage [`link_doc`] together with the codec its `<sce:framer ref>`
/// names — the ref has to resolve to a document the build can see.
///
/// The codec arrives through the link's own `<sce:import>` rather than
/// the build's input list on purpose: the element-type candidate map
/// this file asserts against is built from the build's forge inputs, so
/// listing the framer codec there would change the very set under test.
fn write_link_with_framer(dir: &Path, name: &str) -> PathBuf {
    write_doc(
        dir,
        "scout_frame_codec.scxml",
        &codec_doc("scout_frame_codec"),
    );
    write_doc(dir, &format!("{name}.scxml"), &link_doc(name))
}

/// Bounded-collection doc. Optional `index_by_field`, `concurrency`,
/// and `extern_decls` body so each test can dial in the axis under
/// test without redefining a fixture template per scenario.
fn bc_doc(
    name: &str,
    element_type: &str,
    index_by_field: Option<&str>,
    concurrency: &str,
    extern_decls: &str,
) -> String {
    let index_by_xml = match index_by_field {
        Some(f) => format!("\n  <sce:index-by field=\"{f}\"/>"),
        None => String::new(),
    };
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="bounded-collection" name="{name}" version="1.0">
  {extern_decls}
  <sce:element-type>{element_type}</sce:element-type>
  <sce:capacity const="8"/>{index_by_xml}
  <sce:concurrency>{concurrency}</sce:concurrency>
</scxml>"##
    )
}

// ─── 1. happy_codec_element_type_no_index_by ──────────────────────────

#[test]
fn happy_codec_element_type_no_index_by() {
    // Codec doc present in the build; bounded-collection points at it
    // by name; no index_by; single-writer concurrency. All three β
    // axes silent-pass.
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(
        dir.path(),
        "subscription_entry.scxml",
        &codec_doc("subscription_entry"),
    );
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "subscription_entry",
            None,
            "single-writer",
            "",
        ),
    );

    assert_validator_silent_passed(run_orchestrator(&[], &[codec.as_path(), bc.as_path()]));
}

// ─── 2. happy_codec_element_type_with_valid_index_by ──────────────────

#[test]
fn happy_codec_element_type_with_valid_index_by() {
    // index_by points at `key_expr_id`, which is a declared field on
    // the codec. Index-by enumeration arm runs but finds the field;
    // validator silent-passes.
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(
        dir.path(),
        "subscription_entry.scxml",
        &codec_doc("subscription_entry"),
    );
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "subscription_entry",
            Some("key_expr_id"),
            "single-writer",
            "",
        ),
    );

    assert_validator_silent_passed(run_orchestrator(&[], &[codec.as_path(), bc.as_path()]));
}

// ─── 3. happy_procedure_element_type ─────────────────────────────────

#[test]
fn happy_procedure_element_type() {
    // Procedure kind admitted as element-type per spec lines 2566-2567.
    // Tests the second arm of `enumerate_element_type_field_names`
    // (inputs + internals concatenation).
    let dir = tempdir().expect("tempdir");
    let proc_doc = write_doc(
        dir.path(),
        "session_step.scxml",
        &procedure_doc("session_step"),
    );
    let bc = write_doc(
        dir.path(),
        "step_history.scxml",
        &bc_doc(
            "step_history",
            "session_step",
            Some("counter"),
            "single-writer",
            "",
        ),
    );

    assert_validator_silent_passed(run_orchestrator(&[], &[proc_doc.as_path(), bc.as_path()]));
}

// ─── 4. happy_multi_writer_with_atomic_extern ────────────────────────

#[test]
fn happy_multi_writer_with_atomic_extern() {
    // Multi-writer BC; atomic intrinsic declared via `<sce:extern>` in
    // a sibling forge doc (the codec doc carries it). The build-wide
    // aggregation surfaces the atomic-purpose extern, so the
    // multi-writer axis silent-passes.
    let dir = tempdir().expect("tempdir");
    // Codec doc carries the atomic extern declaration — `<sce:extern>`
    // is a doc-root sibling of `<datamodel>` per the §synth-5-I parse-time
    // grammar.
    let codec_with_extern = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="codec" sce:default-endian="big" name="subscription_entry" version="1.0">
  <sce:extern name="sce_atomic_load_acquire_u32" sig="(*const u32) -> u32" abi="c"/>
  <datamodel>
    <sce:field id="key_expr_id" sce:type="uint32" sce:byte="0" sce:bit-size="32"/>
    <sce:field id="callback_id" sce:type="uint32" sce:byte="4" sce:bit-size="32"/>
  </datamodel>
</scxml>"##
        .to_string();
    let codec = write_doc(dir.path(), "subscription_entry.scxml", &codec_with_extern);
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "subscription_entry",
            None,
            "multi-writer",
            "",
        ),
    );

    assert_validator_silent_passed(run_orchestrator(&[], &[codec.as_path(), bc.as_path()]));
}

// ─── 5. element_type_not_a_kind_unknown_fires ────────────────────────

#[test]
fn element_type_not_a_kind_unknown_fires() {
    // BC names an element-type that does not exist in the build at
    // all. Cross-doc lookup misses; candidate list carries the empty
    // sorted union of codec + procedure names (none in this build).
    let dir = tempdir().expect("tempdir");
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "nonexistent_entry",
            None,
            "single-writer",
            "",
        ),
    );

    let err = match run_orchestrator(&[], &[bc.as_path()]) {
        Ok(_) => panic!("missing element-type kind must fire"),
        Err(e) => e,
    };

    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::CollectionElementTypeNotAKind {
                collection_name,
                element_type,
                candidates,
                ..
            } => {
                assert_eq!(collection_name, "local_sub_table");
                assert_eq!(element_type, "nonexistent_entry");
                assert!(
                    candidates.is_empty(),
                    "no codec/procedure docs in the build → empty candidate list"
                );
            }
            other => panic!("expected CollectionElementTypeNotAKind, got {other:?}"),
        },
        other => panic!("expected CollectionElementTypeNotAKind, got {other:?}"),
    }
}

// ─── 6. element_type_not_a_kind_resolves_to_link_fires ───────────────

#[test]
fn element_type_not_a_kind_resolves_to_link_fires() {
    // BC names an existing forge doc, but the doc's kind is link —
    // NOT codec or procedure. The element-type-candidate map (built
    // only from codec + procedure docs in pass-1) does not contain
    // the link's name, so the lookup misses and the diagnostic fires.
    // Candidate list carries the sorted set of codec + procedure
    // names in the build (one codec here so authors see legal
    // alternatives even when their reference resolves to the wrong
    // kind).
    let dir = tempdir().expect("tempdir");
    let link = write_link_with_framer(dir.path(), "wire_endpoint");
    let codec = write_doc(
        dir.path(),
        "subscription_entry.scxml",
        &codec_doc("subscription_entry"),
    );
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "wire_endpoint",
            None,
            "single-writer",
            "",
        ),
    );

    let err = match run_orchestrator(&[], &[link.as_path(), codec.as_path(), bc.as_path()]) {
        Ok(_) => panic!("element-type pointing at a link kind must fire"),
        Err(e) => e,
    };

    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::CollectionElementTypeNotAKind {
                element_type,
                candidates,
                ..
            } => {
                assert_eq!(element_type, "wire_endpoint");
                assert_eq!(
                    candidates,
                    &vec!["subscription_entry".to_string()],
                    "codec doc surfaces as the legal alternative"
                );
            }
            other => panic!("expected CollectionElementTypeNotAKind, got {other:?}"),
        },
        other => panic!("expected CollectionElementTypeNotAKind, got {other:?}"),
    }
}

// ─── 7. index_by_field_missing_fires ─────────────────────────────────

#[test]
fn index_by_field_missing_fires() {
    // Element-type resolves to a codec, but the index_by field names a
    // field absent from the codec's field set. Candidate list carries
    // the sorted declared fields.
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(
        dir.path(),
        "subscription_entry.scxml",
        &codec_doc("subscription_entry"),
    );
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "subscription_entry",
            Some("key_id"),
            "single-writer",
            "",
        ),
    );

    let err = match run_orchestrator(&[], &[codec.as_path(), bc.as_path()]) {
        Ok(_) => panic!("index_by field absent from codec must fire"),
        Err(e) => e,
    };

    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::CollectionIndexByFieldMissing {
                collection_name,
                field,
                element_type,
                element_kind,
                candidates,
                ..
            } => {
                assert_eq!(collection_name, "local_sub_table");
                assert_eq!(field, "key_id");
                assert_eq!(element_type, "subscription_entry");
                assert_eq!(element_kind, "codec");
                assert_eq!(
                    candidates,
                    &vec!["callback_id".to_string(), "key_expr_id".to_string(),],
                    "sorted declared codec fields"
                );
            }
            other => panic!("expected CollectionIndexByFieldMissing, got {other:?}"),
        },
        other => panic!("expected CollectionIndexByFieldMissing, got {other:?}"),
    }
}

// ─── 8. multi_writer_without_atomic_extern_fires ─────────────────────

#[test]
fn multi_writer_without_atomic_extern_fires() {
    // Multi-writer BC; element-type resolves to a codec (so the
    // element-type axis silent-passes) but no atomic intrinsic
    // declared anywhere in the build. The multi-writer axis fires.
    let dir = tempdir().expect("tempdir");
    let codec = write_doc(
        dir.path(),
        "subscription_entry.scxml",
        &codec_doc("subscription_entry"),
    );
    let bc = write_doc(
        dir.path(),
        "local_sub_table.scxml",
        &bc_doc(
            "local_sub_table",
            "subscription_entry",
            None,
            "multi-writer",
            "",
        ),
    );

    let err = match run_orchestrator(&[], &[codec.as_path(), bc.as_path()]) {
        Ok(_) => panic!("multi-writer without atomic extern must fire"),
        Err(e) => e,
    };

    match &err.error {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::CollectionMultiWriterWithoutAtomics { collection_name } => {
                assert_eq!(collection_name, "local_sub_table");
            }
            other => panic!("expected CollectionMultiWriterWithoutAtomics, got {other:?}"),
        },
        other => panic!("expected CollectionMultiWriterWithoutAtomics, got {other:?}"),
    }
}
