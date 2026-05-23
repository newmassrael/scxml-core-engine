//! NL→IR Mapping Roadmap Item C1 — EventSchema kind integration tests.
//!
//! Drives the receive-side typecheck through the production
//! orchestrator (`compile_scxml_with_imports`) for the seven fixtures
//! catalogued in design RFC §3 DL-10 (Atomic A subset). Each test
//! copies the relevant fixture files into a tempdir, invokes the
//! orchestrator, and asserts the expected outcome:
//!
//!   * Positive fixtures (minimal / multi-field / cross-doc with
//!     enum import) compile cleanly to Rust source.
//!   * Negative fixtures route through the typed [`ValidationError`]
//!     variants reused per Item 4 precedent:
//!       - unknown field   → CrossKindFieldNotFound (with did-you-mean
//!                                                    candidates)
//!       - type mismatch   → CrossKindTypeMismatch
//!       - builtin event   → EventSchemaOnBuiltinEvent
//!
//! Fixtures live in `sce-build/tests/fixtures/event_schema/`. The
//! orchestrator pass that runs the validator is wired in
//! `compile_scxml_with_imports` between pass-2 SCXML parsing and the
//! worker/BC cross-doc validators (single-pass walk over every
//! `<transition>` `cond` in every parsed statechart).

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, Located, ValidationError};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

const FIXTURES_DIR: &str = "tests/fixtures/event_schema";

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

fn default_options() -> ForgeCompileOptions {
    ForgeCompileOptions::default()
}

/// Copy the named fixture files from the canonical fixture directory
/// into the tempdir. Returns absolute paths in the same order the
/// names were supplied so callers can build the orchestrator
/// `scxml_files` / `forge_files` slices without re-deriving the
/// paths.
fn stage_fixtures(dst: &Path, names: &[&str]) -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    names
        .iter()
        .map(|name| {
            let src = manifest_dir.join(FIXTURES_DIR).join(name);
            let dst_path = dst.join(name);
            fs::copy(&src, &dst_path)
                .unwrap_or_else(|e| panic!("copy fixture {name}: {e} (src={src:?})"));
            dst_path
        })
        .collect()
}

/// Invoke the orchestrator with `scxml_files` + `forge_files` slices
/// derived from the staged paths. Returns the same shape as
/// `compile_scxml_with_imports` — Ok of generated outputs on success,
/// `Located<ForgeError>` on failure.
fn run(
    scxml_paths: &[&Path],
    forge_paths: &[&Path],
) -> Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>> {
    compile_scxml_with_imports(
        scxml_paths,
        forge_paths,
        &template_dir(),
        Language::Rust,
        &default_options(),
        None,
    )
}

// ─── Positive: minimal schema + primitive-typed receive-side cond ───

#[test]
fn positive_minimal_event_schema_compiles() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "statechart_minimal.scxml",
        ],
    );
    let schema_path = &staged[0];
    let scxml_path = &staged[1];
    let outputs = run(&[scxml_path.as_path()], &[schema_path.as_path()])
        .expect("orchestrator should accept minimal schema + statechart");
    assert!(
        !outputs.is_empty(),
        "expected at least one generated output for the statechart"
    );
}

// ─── Positive: multi-field schema exercising distinct typed axes ───

#[test]
fn positive_multi_field_schema_compiles() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_multi.scxml",
            "statechart_multi_field.scxml",
        ],
    );
    let schema_path = &staged[0];
    let scxml_path = &staged[1];
    run(&[scxml_path.as_path()], &[schema_path.as_path()])
        .expect("orchestrator should accept multi-field schema + statechart");
}

// ─── Positive: cross-doc enum import (schema imports a Lookup) ───

#[test]
fn positive_cross_doc_enum_import_compiles() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "result_enum.scxml",
            "schema_job_completed_with_enum.scxml",
            "statechart_cross_doc.scxml",
        ],
    );
    let lookup_path = &staged[0];
    let schema_path = &staged[1];
    let scxml_path = &staged[2];
    run(
        &[scxml_path.as_path()],
        &[lookup_path.as_path(), schema_path.as_path()],
    )
    .expect("orchestrator should accept cross-doc enum schema + statechart");
}

// ─── Negative: unknown _event.data.<field> reference ───

#[test]
fn negative_unknown_field_rejects_with_did_you_mean() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "statechart_unknown_field.scxml",
        ],
    );
    let schema_path = &staged[0];
    let scxml_path = &staged[1];
    let err = run(&[scxml_path.as_path()], &[schema_path.as_path()])
        .err()
        .expect("orchestrator should reject statechart with unknown _event.data field");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindFieldNotFound {
                importing_kind: _,
                importing_name: _,
                alias,
                field,
                imported_kind: _,
                imported_name: _,
                candidates,
            } => {
                assert_eq!(alias, "_event.data");
                assert_eq!(field, "status");
                assert!(
                    candidates.iter().any(|c| c == "elapsed_ms"),
                    "expected did-you-mean candidate `elapsed_ms`, got {candidates:?}"
                );
            }
            other => panic!("expected CrossKindFieldNotFound, got {other:?}"),
        },
        other => panic!("expected Validation error, got {other:?}"),
    }
}

// ─── Negative: comparison type-mismatch (uint32 vs string literal) ───

#[test]
fn negative_type_mismatch_rejects_with_typed_diagnostic() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "statechart_type_mismatch.scxml",
        ],
    );
    let schema_path = &staged[0];
    let scxml_path = &staged[1];
    let err = run(&[scxml_path.as_path()], &[schema_path.as_path()])
        .err()
        .expect("orchestrator should reject statechart with uint32-vs-string comparison");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindTypeMismatch {
                field,
                actual,
                expected,
                ..
            } => {
                assert_eq!(field, "elapsed_ms");
                assert_eq!(actual, "string");
                assert_eq!(expected, "uint32");
            }
            other => panic!("expected CrossKindTypeMismatch, got {other:?}"),
        },
        other => panic!("expected Validation error, got {other:?}"),
    }
}

// ─── Negative: EventSchema declared against W3C built-in event ───

#[test]
fn negative_event_schema_on_builtin_event_rejects() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(dir.path(), &["schema_on_builtin_error.scxml"]);
    let schema_path = &staged[0];
    let err = run(&[], &[schema_path.as_path()])
        .err()
        .expect("orchestrator should reject schema declared against `error.execution`");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::EventSchemaOnBuiltinEvent { event_name } => {
                assert_eq!(event_name, "error.execution");
            }
            other => panic!("expected EventSchemaOnBuiltinEvent, got {other:?}"),
        },
        other => panic!("expected Validation error, got {other:?}"),
    }
}

// ─── Schemaless fallback (DL-9): statechart without schema imports
//     is unaffected by the validator ──────────────────────────────────

#[test]
fn schemaless_statechart_passes_unchanged() {
    // Statechart with a transition cond that reads `_event.data.X`
    // for an event with NO imported schema. The validator must
    // no-op (schemaless fallback) — the existing dynamic-payload
    // behavior of `_event.data` remains the contract.
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://newmassrael.io/scxml/sce"
       name="schemaless_statechart"
       version="1.0"
       initial="waiting"
       datamodel="ecmascript">
  <state id="waiting">
    <transition event="other.event" cond="_event.data.anything === 0" target="done"/>
  </state>
  <state id="done"/>
</scxml>
"##;
    let scxml_path = dir.path().join("schemaless_statechart.scxml");
    fs::write(&scxml_path, scxml).expect("write scxml");
    run(&[scxml_path.as_path()], &[])
        .expect("schemaless statechart should pass — DL-9 fallback no-ops the validator");
}

// ─── Atomic B (DL-4) send-side payload typecheck ─────────────────

/// Positive — `<raise event="X"><param name="F" expr="...">` whose
/// `F` is declared on the schema and whose `expr` is a primitive
/// literal compatible with the field's declared `sce_type` passes
/// the send-side validator.
#[test]
fn positive_send_typed_compiles() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "statechart_send_typed.scxml",
        ],
    );
    run(&[staged[1].as_path()], &[staged[0].as_path()])
        .expect("orchestrator should accept typed send payload matching the schema");
}

/// Negative — `<param name="F">` whose `F` is NOT declared on the
/// schema surfaces `validation/event-payload-field-unknown` with the
/// schema's declared field surface as the closed `Fix::ReplaceOneOf`
/// candidate set.
#[test]
fn negative_send_extra_param_rejects_with_did_you_mean() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "negative_statechart_send_extra_param.scxml",
        ],
    );
    let err = run(&[staged[1].as_path()], &[staged[0].as_path()])
        .err()
        .expect("orchestrator should reject statechart with unknown <param name>");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::EventPayloadFieldUnknown {
                event_name,
                field,
                candidates,
                ..
            } => {
                assert_eq!(event_name, "job.completed");
                assert_eq!(field, "missing_field");
                assert!(
                    candidates.iter().any(|c| c == "elapsed_ms"),
                    "expected did-you-mean candidate `elapsed_ms`, got {candidates:?}"
                );
            }
            other => panic!("expected EventPayloadFieldUnknown, got {other:?}"),
        },
        other => panic!("expected Validation error, got {other:?}"),
    }
}

/// Negative — `<param name="F" expr="literal">` whose literal type
/// does not unify with the field's declared `sce_type` surfaces
/// `validation/cross-kind-type-mismatch` (reused per Item 4
/// precedent — same code, distinct typed message context).
#[test]
fn negative_send_type_mismatch_rejects_with_typed_diagnostic() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "negative_statechart_send_type_mismatch.scxml",
        ],
    );
    let err = run(&[staged[1].as_path()], &[staged[0].as_path()])
        .err()
        .expect("orchestrator should reject statechart with string-vs-uint32 param");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::CrossKindTypeMismatch {
                field,
                actual,
                expected,
                alias,
                ..
            } => {
                assert_eq!(field, "elapsed_ms");
                assert_eq!(actual, "string");
                assert_eq!(expected, "uint32");
                // Send-side alias names the offending action shape so
                // the diagnostic message distinguishes it from the
                // receive-side `_event.data` alias.
                assert!(
                    alias.contains("send") && alias.contains("job.completed"),
                    "expected alias to name the <send event=\"job.completed\"> site, got {alias:?}"
                );
            }
            other => panic!("expected CrossKindTypeMismatch, got {other:?}"),
        },
        other => panic!("expected Validation error, got {other:?}"),
    }
}

// ─── Atomic B (DL-7) mesh cross-machine schema validation ────────

fn run_with_deploy(
    scxml_paths: &[&Path],
    forge_paths: &[&Path],
    deploy: &sce_build::mesh::deploy::DeployConfig,
) -> Result<Vec<(String, sce_build::generator::GeneratedOutput)>, Located<ForgeError>> {
    compile_scxml_with_imports(
        scxml_paths,
        forge_paths,
        &template_dir(),
        Language::Rust,
        &default_options(),
        Some(deploy),
    )
}

fn stage_deploy(dst: &Path, name: &str) -> sce_build::mesh::deploy::DeployConfig {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join(FIXTURES_DIR).join(name);
    let dst_path = dst.join(name);
    fs::copy(&src, &dst_path).unwrap_or_else(|e| panic!("copy {name}: {e}"));
    sce_build::mesh::deploy::parse_deploy(&dst_path).expect("parse deploy yaml")
}

/// Positive — two machines on a mesh both import the same schema
/// document; mesh validator computes matching canonical hashes and
/// accepts.
#[test]
fn positive_mesh_matched_compiles() {
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "mesh_sender_matched.scxml",
            "mesh_receiver_matched.scxml",
        ],
    );
    let deploy = stage_deploy(dir.path(), "mesh_matched_deploy.yaml");
    run_with_deploy(
        &[staged[1].as_path(), staged[2].as_path()],
        &[staged[0].as_path()],
        &deploy,
    )
    .expect("mesh validator should accept matching schemas across machines");
}

/// Negative — sender imports a schema for `job.completed`; receiver
/// declares no schema for it. Mesh validator rejects with
/// `mesh/event-schema-mismatch` reason=SenderOnly.
#[test]
fn negative_mesh_sender_only_rejects() {
    use sce_build::mesh::error::{DeployError, EventSchemaMismatchReason, MeshError};
    let dir = tempdir().expect("tempdir");
    let staged = stage_fixtures(
        dir.path(),
        &[
            "schema_job_completed_minimal.scxml",
            "negative_mesh_sender_only.scxml",
            "negative_mesh_receiver_schemaless.scxml",
        ],
    );
    let deploy = stage_deploy(dir.path(), "negative_mesh_mismatch_deploy.yaml");
    let err = run_with_deploy(
        &[staged[1].as_path(), staged[2].as_path()],
        &[staged[0].as_path()],
        &deploy,
    )
    .err()
    .expect("mesh validator should reject partial-coverage schema across machines");
    match err.error {
        ForgeError::Mesh(boxed_mesh) => match boxed_mesh.as_ref() {
            MeshError::Deploy(boxed_deploy) => match boxed_deploy.as_ref() {
                DeployError::EventSchemaMismatch {
                    event_name,
                    sender_machine,
                    receiver_machine,
                    reason,
                } => {
                    assert_eq!(event_name, "job.completed");
                    assert_eq!(sender_machine, "alpha");
                    assert_eq!(receiver_machine, "beta");
                    assert!(
                        matches!(reason, EventSchemaMismatchReason::SenderOnly),
                        "expected SenderOnly, got {reason:?}"
                    );
                }
                other => panic!("expected EventSchemaMismatch, got {other:?}"),
            },
            other => panic!("expected MeshError::Deploy, got {other:?}"),
        },
        other => panic!("expected Mesh error, got {other:?}"),
    }
}
