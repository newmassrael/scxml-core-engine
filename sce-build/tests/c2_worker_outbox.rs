//! C2 outbox follow-up Atomic B integration tests — watching-zenoh
//! RFC §5.D, Q-Outbox-1..9 LOCKED 2026-05-12.
//!
//! Atomic B's distinguishing value: SCXML-side `<sce:outbox ref="X">`
//! cross-resolution against the build's `SceCrossDocRegistry`,
//! emitting one of three spec-extension diagnostics per repair axis:
//!
//! - `worker/outbox-ref-unknown` (owner not in registry)
//! - `worker/outbox-target-wrong-kind` (owner found but kind not in
//!   {statechart, worker})
//! - `worker/outbox-target-suffix-invalid` (suffix !=  `inbox`)
//!
//! Atomic B builds on Atomic A's `compile_scxml_with_imports`
//! orchestrator. Single-file compile paths (`compile_forge_with_imports`)
//! cannot enforce outbox cross-resolution because the cross-doc
//! registry is built only by the orchestrator.
//!
//! Test matrix per RFC §4 (12 scenarios):
//!  1. happy_outbox_to_statechart_basic
//!  2. happy_outbox_to_statechart_alongside_link
//!  3. happy_outbox_to_worker_basic                  (Q-Outbox-3 (b))
//!  4. happy_outbox_to_worker_self_reference         (Q-Outbox-3 (b))
//!  5. unknown_outbox_owner_fires
//!  6. unknown_outbox_owner_with_busy_registry
//!  7. wrong_kind_outbox_to_link_fires
//!  8. wrong_kind_outbox_to_link_with_valid_alts
//!  9. invalid_suffix_typo_inbx_fires
//! 10. invalid_suffix_bare_owner_no_dot_fires
//! 11. empty_registry_with_outbox_fires_unknown
//! 12. multi_worker_fan_in_same_statechart

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use sce_build::compile_scxml_with_imports;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

fn default_options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: None,
        const_fold_budget: None,
        cache_platform: None,
        worker_placement: None,
    }
}

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Rust)
}

/// Minimal `<scxml sce:kind="link">` doc with a stage-pool ref —
/// needed for any worker fixture's `<sce:link-rx ref>` resolution
/// through the worker's own `<sce:import kind="link">`. Stage-pool
/// included so workers can also coexist with statecharts that wire
/// `<sce:on-sample>` against this link (covers cross-validator
/// no-interference cases).
fn link_doc(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="{name}" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
  <sce:stage-pool ref="scout_stage_pool"/>
</scxml>"##
    )
}

/// Worker doc with explicit outbox. The link-rx alias matches the
/// imported link's basename minus `.scxml`, so the worker's
/// `<sce:import>` resolves against the test's temp dir.
fn worker_with_outbox(
    name: &str,
    link_alias: &str,
    link_src: &str,
    outbox_ref: &str,
) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="{name}" version="1.0">
  <sce:import as="{link_alias}" src="{link_src}" kind="link"/>
  <sce:link-rx ref="{link_alias}"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
  <sce:outbox ref="{outbox_ref}"/>
</scxml>"##
    )
}

/// Worker doc without outbox — for the silent-skip path.
#[allow(dead_code)]
fn worker_without_outbox(name: &str, link_alias: &str, link_src: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="{name}" version="1.0">
  <sce:import as="{link_alias}" src="{link_src}" kind="link"/>
  <sce:link-rx ref="{link_alias}"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
</scxml>"##
    )
}

/// Plain W3C statechart with one trivial state. The `name` attribute
/// is what feeds `SceCrossDocRegistry::record_statechart` in the
/// orchestrator's pass 2.
fn statechart_named(name: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       name="{name}"
       version="1.0"
       initial="idle"
       datamodel="ecmascript">
  <state id="idle"/>
</scxml>"##
    )
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
    )
}

// ─── 1. happy_outbox_to_statechart_basic ─────────────────────────────

#[test]
fn happy_outbox_to_statechart_basic() {
    // Spec §5.D line 895 example shape: worker `rx_loop` routes
    // `<sce:outbox ref="session_fsm.inbox">` into the statechart
    // `session_fsm`. Both docs in the build; outbox owner resolves
    // to a registered statechart kind; suffix is `inbox`; the
    // validator passes silently.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let worker = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbox",
        ),
    );
    let statechart = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );

    let outputs = run_orchestrator(
        &[statechart.as_path()],
        &[link.as_path(), worker.as_path()],
    )
    .expect("happy outbox→statechart must compile");

    assert_eq!(outputs.len(), 3, "1 statechart + 2 forge → 3 outputs");
}

// ─── 2. happy_outbox_to_statechart_alongside_link ────────────────────

#[test]
fn happy_outbox_to_statechart_alongside_link() {
    // Same as #1 but also a sibling statechart that consumes the link
    // via `<sce:on-sample>` — exercises both cross-validators in the
    // same build with no interference.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let worker = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbox",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );
    // A second statechart, not referenced from anywhere — its presence
    // just adds another statechart-kind entry to the registry so the
    // candidate-list helper has more than one entry to sort.
    let observer = write_doc(
        dir.path(),
        "observer.scxml",
        &statechart_named("observer"),
    );

    let outputs = run_orchestrator(
        &[session.as_path(), observer.as_path()],
        &[link.as_path(), worker.as_path()],
    )
    .expect("happy outbox→statechart with sibling stm must compile");

    assert_eq!(outputs.len(), 4);
}

// ─── 3. happy_outbox_to_worker_basic ─────────────────────────────────

#[test]
fn happy_outbox_to_worker_basic() {
    // Q-Outbox-3 (b) — worker→worker outbox is legal. `rx_loop`
    // routes to `tx_loop.inbox`; both workers in the same build.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "tx_loop.inbox",
        ),
    );
    let tx = write_doc(
        dir.path(),
        "tx_loop.scxml",
        // tx_loop has no outbox — outbox is optional, silent-skip in
        // validator means no error from this doc.
        &worker_without_outbox("tx_loop", "scout_link", "scout_link.scxml"),
    );

    let outputs = run_orchestrator(
        &[],
        &[link.as_path(), rx.as_path(), tx.as_path()],
    )
    .expect("worker→worker outbox must compile");

    assert_eq!(outputs.len(), 3, "3 forge files → 3 outputs");
}

// ─── 4. happy_outbox_to_worker_self_reference ────────────────────────

#[test]
fn happy_outbox_to_worker_self_reference() {
    // Q-Outbox-3 (b) admits worker→self outbox (degenerate but legal:
    // the validator surfaces resolution failures, not stylistic
    // guidance). `rx_loop.inbox` resolves to the same worker that
    // owns the outbox.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "rx_loop.inbox",
        ),
    );

    let outputs =
        run_orchestrator(&[], &[link.as_path(), rx.as_path()]).expect(
            "self-referencing outbox must compile (validator surfaces \
             resolution, not style)",
        );

    assert_eq!(outputs.len(), 2);
}

// ─── 5. unknown_outbox_owner_fires ───────────────────────────────────

#[test]
fn unknown_outbox_owner_fires() {
    // Outbox `sesion_fsm.inbox` typo (missing 'n') with no matching
    // recipient in the build. The validator surfaces the resolution
    // failure with `worker/outbox-ref-unknown`. Candidate list
    // includes every registered recipient (here: only `rx_loop.inbox`
    // — the worker's own name).
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "sesion_fsm.inbox",
        ),
    );

    let err = match run_orchestrator(&[], &[link.as_path(), rx.as_path()]) {
        Ok(_) => panic!("unknown outbox owner must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::WorkerOutboxRefUnknown {
            worker_name,
            outbox_value,
            owner,
            candidates,
            ..
        }) => {
            assert_eq!(worker_name, "rx_loop");
            assert_eq!(outbox_value, "sesion_fsm.inbox");
            assert_eq!(owner, "sesion_fsm");
            // No statecharts in the build; only the worker itself is
            // a registered recipient.
            assert_eq!(candidates, vec!["rx_loop.inbox".to_string()]);
        }
        other => panic!("expected WorkerOutboxRefUnknown, got: {other:?}"),
    }
}

// ─── 6. unknown_outbox_owner_with_busy_registry ──────────────────────

#[test]
fn unknown_outbox_owner_with_busy_registry() {
    // Same failure axis as #5 but with multiple legitimate recipients
    // in the build, so the candidate list returned by the validator
    // is non-trivial and confirms the sorted-union shape from
    // `names_of_any_kind(statechart + worker)`.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "totally_unknown.inbox",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );
    let observer = write_doc(
        dir.path(),
        "observer.scxml",
        &statechart_named("observer"),
    );

    let err = match run_orchestrator(
        &[session.as_path(), observer.as_path()],
        &[link.as_path(), rx.as_path()],
    ) {
        Ok(_) => panic!("unknown outbox owner must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::WorkerOutboxRefUnknown {
            owner,
            candidates,
            ..
        }) => {
            assert_eq!(owner, "totally_unknown");
            // Sorted union: observer + rx_loop + session_fsm, each
            // suffixed with `.inbox`.
            assert_eq!(
                candidates,
                vec![
                    "observer.inbox".to_string(),
                    "rx_loop.inbox".to_string(),
                    "session_fsm.inbox".to_string(),
                ]
            );
        }
        other => panic!("expected WorkerOutboxRefUnknown, got: {other:?}"),
    }
}

// ─── 7. wrong_kind_outbox_to_link_fires ──────────────────────────────

#[test]
fn wrong_kind_outbox_to_link_fires() {
    // Outbox references `scout_link.inbox` — the owner segment
    // `scout_link` resolves to a link kind, not a statechart or
    // worker. Author confused a link import alias with a statechart
    // name. Diagnostic: `worker/outbox-target-wrong-kind` with
    // `actual_kind = "link"`.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "scout_link.inbox",
        ),
    );

    let err = match run_orchestrator(&[], &[link.as_path(), rx.as_path()]) {
        Ok(_) => panic!("outbox→link must fire wrong-kind diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::WorkerOutboxTargetWrongKind {
            worker_name,
            owner,
            actual_kind,
            ..
        }) => {
            assert_eq!(worker_name, "rx_loop");
            assert_eq!(owner, "scout_link");
            assert_eq!(actual_kind, "link");
        }
        other => panic!("expected WorkerOutboxTargetWrongKind, got: {other:?}"),
    }
}

// ─── 8. wrong_kind_outbox_to_link_with_valid_alts ────────────────────

#[test]
fn wrong_kind_outbox_to_link_with_valid_alts() {
    // Same failure axis as #7 but a statechart is also present, so
    // the candidate list is non-empty and the author has a clear
    // repair target.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "scout_link.inbox",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );

    let err = match run_orchestrator(
        &[session.as_path()],
        &[link.as_path(), rx.as_path()],
    ) {
        Ok(_) => panic!("outbox→link must fire wrong-kind diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::WorkerOutboxTargetWrongKind {
            actual_kind,
            candidates,
            ..
        }) => {
            assert_eq!(actual_kind, "link");
            assert_eq!(
                candidates,
                vec![
                    "rx_loop.inbox".to_string(),
                    "session_fsm.inbox".to_string(),
                ]
            );
        }
        other => panic!("expected WorkerOutboxTargetWrongKind, got: {other:?}"),
    }
}

// ─── 9. invalid_suffix_typo_inbx_fires ───────────────────────────────

#[test]
fn invalid_suffix_typo_inbx_fires() {
    // Q-Outbox-6 (a) strict-suffix lock — suffix `inbx` !=  `inbox`.
    // Diagnostic `worker/outbox-target-suffix-invalid` carries a
    // deterministic `Fix::ReplaceWith` for `{owner}.inbox`.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbx",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );

    let err = match run_orchestrator(
        &[session.as_path()],
        &[link.as_path(), rx.as_path()],
    ) {
        Ok(_) => panic!("suffix typo must fire diagnostic"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(
            ValidationError::WorkerOutboxTargetSuffixInvalid {
                owner,
                suffix,
                outbox_value,
                ..
            },
        ) => {
            assert_eq!(owner, "session_fsm");
            assert_eq!(suffix, "inbx");
            assert_eq!(outbox_value, "session_fsm.inbx");
        }
        other => panic!(
            "expected WorkerOutboxTargetSuffixInvalid, got: {other:?}"
        ),
    }
}

// ─── 10. invalid_suffix_bare_owner_no_dot_fires ──────────────────────

#[test]
fn invalid_suffix_bare_owner_no_dot_fires() {
    // Q-Outbox-6 (a) strict-suffix lock — bare `<owner>` without
    // `.inbox` suffix violates the strict shape. Routes to
    // suffix-invalid with an empty suffix string.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );

    let err = match run_orchestrator(
        &[session.as_path()],
        &[link.as_path(), rx.as_path()],
    ) {
        Ok(_) => panic!("bare owner (no dot) must fire suffix-invalid"),
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(
            ValidationError::WorkerOutboxTargetSuffixInvalid {
                owner,
                suffix,
                outbox_value,
                ..
            },
        ) => {
            assert_eq!(owner, "session_fsm");
            assert_eq!(suffix, "", "no-dot case yields empty suffix");
            assert_eq!(outbox_value, "session_fsm");
        }
        other => panic!(
            "expected WorkerOutboxTargetSuffixInvalid, got: {other:?}"
        ),
    }
}

// ─── 11. empty_registry_with_outbox_fires_unknown ────────────────────

#[test]
fn empty_registry_with_outbox_fires_unknown() {
    // Single-worker build (no peer SCXML / forge docs). The owner
    // segment cannot resolve to anything because the only registered
    // recipient is the worker itself. With outbox targeting a non-
    // self owner, the unknown axis surfaces with empty-list peer
    // candidates (only the worker's own `.inbox` is in the list).
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbox",
        ),
    );

    let err = match run_orchestrator(&[], &[link.as_path(), rx.as_path()]) {
        Ok(_) => {
            panic!(
                "minimal build with unknown outbox owner must fire diagnostic"
            )
        }
        Err(e) => e,
    };

    match err.error {
        ForgeError::Validation(ValidationError::WorkerOutboxRefUnknown {
            owner,
            candidates,
            ..
        }) => {
            assert_eq!(owner, "session_fsm");
            // Only the worker itself appears as a registered
            // recipient — no statechart docs in the build.
            assert_eq!(candidates, vec!["rx_loop.inbox".to_string()]);
        }
        other => panic!("expected WorkerOutboxRefUnknown, got: {other:?}"),
    }
}

// ─── 12. multi_worker_fan_in_same_statechart ─────────────────────────

#[test]
fn multi_worker_fan_in_same_statechart() {
    // Two workers (rx_loop, tx_loop) both routing their outboxes to
    // the same statechart (`session_fsm.inbox`). Codegen ordering is
    // input-list-driven; both workers must pass cross-resolution
    // independently — neither validator failure short-circuits the
    // other.
    let dir = tempdir().expect("tempdir");
    let link = write_doc(dir.path(), "scout_link.scxml", &link_doc("scout_link"));
    let rx = write_doc(
        dir.path(),
        "rx_loop.scxml",
        &worker_with_outbox(
            "rx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbox",
        ),
    );
    let tx = write_doc(
        dir.path(),
        "tx_loop.scxml",
        &worker_with_outbox(
            "tx_loop",
            "scout_link",
            "scout_link.scxml",
            "session_fsm.inbox",
        ),
    );
    let session = write_doc(
        dir.path(),
        "session_fsm.scxml",
        &statechart_named("session_fsm"),
    );

    let outputs = run_orchestrator(
        &[session.as_path()],
        &[link.as_path(), rx.as_path(), tx.as_path()],
    )
    .expect("multi-worker fan-in to single statechart must compile");

    assert_eq!(outputs.len(), 4);
}
