//! Cross-doc orchestrator integration tests — watching-zenoh RFC §5.D
//! C2 outbox follow-up Atomic A (Q-Outbox-1 (a) + Q-Outbox-5 (a)).
//!
//! Atomic A's distinguishing value: production wire-up of
//! `validate_on_sample_link_references` (parser.rs:3187) via the new
//! `compile_scxml_with_imports` orchestrator entry point. Before this
//! atomic landed, the on-sample cross-ref validator existed only as a
//! pub fn callable from tests — every production build path
//! (`compile_scxml`, `compile_scxml_to_string`, `compile_forge_with_imports`,
//! `sce-codegen generate`) processed files one at a time with no
//! cross-doc registry construction, so `<sce:on-sample link="undeclared">`
//! references silently passed (`feedback_silently_broken_hooks.md`
//! instance). These tests pin that the orchestrator surface CLOSES
//! the silent hole — undeclared refs now fire production-side.
//!
//! Test matrix:
//! 1. happy_orchestrator_compiles_multi_doc — 1 statechart + 1 link
//!    with stage-pool resolves cleanly; outputs emitted per-doc.
//! 2. on_sample_link_not_declared_fires_in_production — name not in
//!    cross-doc registry → `scxml/on-sample-link-not-declared` fires.
//! 3. on_sample_sample_take_without_stage_pool_fires — name resolves
//!    to a link without `<sce:stage-pool>` → `pool/sample-take-
//!    without-stage-pool` fires.
//! 4. on_sample_link_wrong_kind_fires — name resolves to a non-link
//!    kind (statechart with colliding name) → `scxml/on-sample-link-
//!    wrong-kind` fires (this arm became reachable only after Atomic
//!    A's registry extension landed multi-kind variants).
//! 5. empty_file_lists_yield_empty_outputs — no-op edge with both
//!    slices empty; orchestrator does not crash, returns Vec::new().
//! 6. worker_doc_records_into_cross_doc_registry — Atomic B prereq:
//!    a worker doc's name reaches the registry so future
//!    `<sce:outbox ref="worker.inbox">` resolution lands cleanly on
//!    the now-present foundation.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn default_options() -> ForgeCompileOptions {
    ForgeCompileOptions::default()
}

/// Locate the workspace `tools/codegen/templates` directory once per
/// test. Mirrors `sce_build::find_template_dir_for(Language::Rust)`'s
/// semantics without crossing the private boundary — tests can use
/// the same root irrespective of language because the orchestrator's
/// per-language dispatch happens inside `compile_scxml_lang_typed`.
fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

/// Minimal statechart SCXML with one `<sce:on-sample>` block. The
/// `name` attribute lets the cross-doc registry classify this doc as
/// a statechart; the `<sce:on-sample>` block triggers cross-ref
/// validation against the orchestrator's link registry.
fn statechart_with_on_sample(name: &str, link: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       name="{name}"
       version="1.0"
       initial="running"
       datamodel="ecmascript">
  <state id="running">
    <sce:on-sample link="{link}" event="scout.tick"/>
    <transition event="scout.tick" target="running"/>
  </state>
</scxml>"##
    )
}

fn link_with_stage_pool() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="scout_link" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:stage-pool ref="scout_stage_pool"/>
</scxml>"##
}

fn link_without_stage_pool() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="scout_link" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##
}

fn worker_minimal(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="{name}" version="1.0">
  <sce:import as="scout_link" src="scout_link.scxml" kind="link"/>
  <sce:link-rx ref="scout_link"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
</scxml>"##
    )
}

fn write_doc(dir: &Path, basename: &str, content: &str) -> PathBuf {
    let path = dir.join(basename);
    fs::write(&path, content).expect("write doc");
    path
}

// ─── 1. Happy multi-doc compile ───────────────────────────────────────

#[test]
fn happy_orchestrator_compiles_multi_doc() {
    // Statechart with on-sample references link "scout_link"; forge
    // link doc declares "scout_link" with a stage-pool ref. Both
    // cross-ref checks succeed; orchestrator returns one output per
    // input doc.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[forge.as_path()];

    let outputs = compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    )
    .expect("happy multi-doc compile must succeed");

    assert_eq!(outputs.len(), 2, "expected 1 forge + 1 scxml = 2 outputs");
    // Forge emits first (input order), then SCXML.
    assert_eq!(outputs[0].0, "scout_link.scxml");
    assert_eq!(outputs[1].0, "session_fsm.scxml");
}

// ─── 2. on-sample-link-not-declared fires in production ─────────────

#[test]
fn on_sample_link_not_declared_fires_in_production() {
    // The on-sample reference names "unknown_link"; the only forge
    // link in the build is "scout_link". Before Atomic A this passed
    // silently because no production path built the registry. The
    // orchestrator now closes that hole.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "unknown_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[forge.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    ) {
        Ok(_) => panic!("undeclared on-sample link must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::OnSampleLinkNotDeclared {
            link,
            candidates,
            ..
        }) => {
            assert_eq!(link, "unknown_link");
            assert_eq!(candidates, vec!["scout_link".to_string()]);
        }
        other => panic!("expected OnSampleLinkNotDeclared, got: {other:?}"),
    }
}

// ─── 3. sample-take-without-stage-pool fires in production ───────────

#[test]
fn on_sample_sample_take_without_stage_pool_fires() {
    // Link "scout_link" exists in the registry, but lacks
    // `<sce:stage-pool>`. The on-sample callback that takes
    // ownership would route through a runtime panic hook today —
    // Atomic A's wire-up surfaces the gap at compile time.
    let dir = tempdir().expect("tempdir");
    let scxml = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_link"),
    );
    let forge = write_doc(dir.path(), "scout_link.scxml", link_without_stage_pool());

    let scxml_refs: &[&Path] = &[scxml.as_path()];
    let forge_refs: &[&Path] = &[forge.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    ) {
        Ok(_) => panic!("missing stage-pool must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::PoolSampleTakeWithoutStagePool {
            link,
            ..
        }) => {
            assert_eq!(link, "scout_link");
        }
        other => panic!("expected PoolSampleTakeWithoutStagePool, got: {other:?}"),
    }
}

// ─── 4. on-sample-link-wrong-kind fires in production ───────────────

#[test]
fn on_sample_link_wrong_kind_fires() {
    // The on-sample reference names "scout_helper"; the cross-doc
    // registry holds "scout_helper" as a STATECHART (sibling doc),
    // not a link kind. The on-sample validator's wrong-kind arm
    // became reachable in production only after Atomic A's registry
    // extension introduced multi-kind variants; before the rename
    // the registry could only hold Link kinds so this arm was
    // forward-compat-only.
    let dir = tempdir().expect("tempdir");
    let scxml_main = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "scout_helper"),
    );
    // A statechart named "scout_helper" — wrong kind for on-sample.
    let scxml_collider = write_doc(
        dir.path(),
        "scout_helper.scxml",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="scout_helper"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##,
    );

    let scxml_refs: &[&Path] = &[scxml_main.as_path(), scxml_collider.as_path()];
    let forge_refs: &[&Path] = &[];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    ) {
        Ok(_) => panic!("wrong-kind on-sample target must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::OnSampleLinkWrongKind {
            link,
            actual_kind,
            ..
        }) => {
            assert_eq!(link, "scout_helper");
            assert_eq!(actual_kind, "statechart");
        }
        other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
    }
}

// ─── 5. Empty file lists are a legal no-op ──────────────────────────

#[test]
fn empty_file_lists_yield_empty_outputs() {
    // The orchestrator MUST handle the no-doc edge gracefully —
    // callers that gate on a manifest may invoke it with empty
    // slices when the manifest is empty.
    let scxml_refs: &[&Path] = &[];
    let forge_refs: &[&Path] = &[];

    let outputs = compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    )
    .expect("empty file lists must not error");

    assert!(outputs.is_empty(), "empty input must yield empty output");
}

// ─── 6. Worker doc lands in cross-doc registry (Atomic B prereq) ────

#[test]
fn worker_doc_records_into_cross_doc_registry() {
    // The C2-α worker schema includes a `name` attribute and the
    // C2-outbox follow-up Atomic A extends the cross-doc registry
    // to record worker docs alongside statecharts + links.
    // This test pins that an SCXML on-sample reference targeting a
    // WORKER name (mispoint — workers aren't link subscribers) now
    // fires wrong-kind, proving the worker's name reached the
    // registry. Atomic B's `<sce:outbox ref="rx_loop.inbox">` will
    // walk the same registry — this fixture is the first lit-up
    // consumer of the worker arm of the registry.
    let dir = tempdir().expect("tempdir");
    // Worker fixture needs sibling link doc for its `<sce:import>`
    // to resolve at parse time.
    let link_sib = write_doc(dir.path(), "scout_link.scxml", link_with_stage_pool());
    let worker = write_doc(dir.path(), "rx_loop.scxml", &worker_minimal("rx_loop"));
    let main = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_with_on_sample("session_fsm", "rx_loop"),
    );

    let scxml_refs: &[&Path] = &[main.as_path()];
    // Order matters: the link sibling registers first so the worker
    // doc's `<sce:import as="scout_link" kind="link"/>` resolves to a
    // known-registered link when the worker parse runs.
    let forge_refs: &[&Path] = &[link_sib.as_path(), worker.as_path()];

    let err = match compile_scxml_with_imports(
        scxml_refs,
        forge_refs,
        &template_dir(),
        Language::Rust,
        &default_options(),
    ) {
        Ok(_) => panic!("on-sample targeting worker name must fire wrong-kind"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::OnSampleLinkWrongKind {
            link,
            actual_kind,
            ..
        }) => {
            assert_eq!(link, "rx_loop");
            assert_eq!(
                actual_kind, "worker",
                "registry must classify rx_loop as worker — \
                 Atomic A's record_document Worker arm landed"
            );
        }
        other => panic!("expected OnSampleLinkWrongKind, got: {other:?}"),
    }
}
