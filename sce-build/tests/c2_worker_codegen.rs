// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! Worker kind dual-emit codegen + cross-resolution integration fixtures.
//!
//! Per RFC §synth-5-D + §synth-5-P (Migration): worker cross-refs validate
//! directly against `parsed.imports` filtered by kind (η-precedent), the
//! inbox is a queue the worker imports and names by alias
//! (`<sce:inbox ref>`), and that queue must declare one consumer. The ring,
//! its capacity and its ordering are the queue document's, so the worker's
//! emission only names the queue's type.
//!
//! Coverage matrix:
//!   - Happy Rust emit (`worker.rs.jinja2` re-exports the queue as the inbox)
//!   - Happy C11 emit (`.h` only: the queue's header and type)
//!   - Cross-ref negative × 3 (link-rx-ref-unknown, inbox-ref-unknown,
//!     inbox-queue-not-single-consumer)
//!   - The removed inline form (`worker/inbox-inline-removed`) through the
//!     compile path, and its diagnostic code

use std::fs;
use std::path::Path;
use tempfile::tempdir;

use sce_build::compile_forge_with_imports;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::{DocumentLabel, ForgeCompileOptions};

/// Minimal forge link kind — declared as a sibling .scxml so the
/// worker doc's `<sce:import as="udp_scout" kind="link" src="...">`
/// can resolve to it via `validate_and_enrich_imports`. Embeds the
/// fields `parse_link` requires (class, framer, backpressure).
fn link_fixture() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="link" name="udp_scout" version="1.0">
  <sce:link-class>udp</sce:link-class>
  <sce:framer ref="scout_frame_codec"/>
  <sce:backpressure>drop</sce:backpressure>
</scxml>"##
}

/// The queue a worker names as its inbox: one producer (the link-rx
/// driver), one consumer (the worker), as the single-consumer rule needs.
fn inbox_queue_fixture() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="queue" name="rx_events" version="1.0">
  <sce:element-type>rx_event</sce:element-type>
  <sce:producers>one</sce:producers>
  <sce:consumers>one</sce:consumers>
  <sce:progress>wait-free</sce:progress>
  <sce:bounded capacity="15"/>
</scxml>"##
}

/// A queue with many consumers, which no worker may name as its inbox.
fn fanout_queue_fixture() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="queue" name="rx_fanout" version="1.0">
  <sce:element-type>rx_event</sce:element-type>
  <sce:producers>one</sce:producers>
  <sce:consumers>many</sce:consumers>
  <sce:progress>lock-free</sce:progress>
  <sce:bounded capacity="16"/>
  <sce:participants const="2"/>
</scxml>"##
}

/// Wrap the link and queue fixtures into a tempdir; return the path. The
/// worker fixture's `<sce:import>` elements reference them by relative `src`.
/// Outbox cross-resolution runs only in the orchestrator path
/// (`compile_scxml_with_imports`, covered in `c2_worker_outbox.rs`); in this
/// single-file harness outbox refs are parser-validated only (presence +
/// format), so no statechart fixture is needed.
fn build_workspace() -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("udp_scout.scxml"), link_fixture()).expect("write link");
    fs::write(dir.path().join("rx_events.scxml"), inbox_queue_fixture()).expect("write queue");
    fs::write(dir.path().join("rx_fanout.scxml"), fanout_queue_fixture()).expect("write queue");
    dir
}

fn compile(
    scxml: &str,
    lang: Language,
    base_dir: &Path,
    options: &ForgeCompileOptions,
) -> Result<GeneratedOutput, ForgeError> {
    compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("rx_loop"),
        lang,
        base_dir,
        options,
    )
    .map_err(|e| e.error)
}

/// A worker that imports the link and the queue `queue_src` under the alias
/// `rx_events`, and names `link_rx` and `inbox` as its refs.
fn worker_xml(link_rx: &str, outbox: Option<&str>, inbox: &str, queue_src: &str) -> String {
    let outbox_line = match outbox {
        Some(o) => format!(r##"<sce:outbox ref="{o}"/>"##),
        None => String::new(),
    };
    // Worker docs import their driving link kind and their inbox queue via
    // `<sce:import>` for the η-precedent cross-resolution shape. Outbox
    // owner-prefix (e.g. `session_fsm`) does NOT need an import declaration
    // — outbox cross-resolution defers to the orchestrator, so parse-time
    // validation accepts any non-empty `ref`.
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:import as="rx_events" src="{queue_src}" kind="queue"/>
  <sce:link-rx ref="{link_rx}"/>
  <sce:inbox ref="{inbox}"/>
  {outbox_line}
</scxml>"##
    )
}

// ─── Happy: Rust emit ───────────────────────────────────────────────

#[test]
fn happy_worker_rust_reexports_the_imported_queue_as_its_inbox() {
    let ws = build_workspace();
    let scxml = worker_xml(
        "udp_scout",
        Some("session_fsm.inbox"),
        "rx_events",
        "rx_events.scxml",
    );
    let out = compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy worker Rust compile");
    let (_, code) = out.files.first().expect("at least one file");
    // The inbox is the queue's own type, named once.
    assert!(
        code.contains("pub use super::rx_events::RxEvents as RxLoopInbox;"),
        "the inbox re-export missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const INBOX_QUEUE: &'static str = "rx_events";"#),
        "INBOX_QUEUE const missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const LINK_RX: &'static str = "udp_scout";"#),
        "LINK_RX const missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const OUTBOX: &'static str = "session_fsm.inbox";"#),
        "OUTBOX const missing:\n{code}"
    );
    // The ring, its depth and its ordering are the queue's, not restated.
    for gone in [
        "DEPTH",
        "ORDERING",
        "Ordering::",
        "AtomicUsize",
        "UnsafeCell",
        "struct RxLoopProducer",
        "struct RxLoopConsumer",
    ] {
        assert!(
            !code.contains(gone),
            "`{gone}` belongs to the queue, not the worker:\n{code}"
        );
    }
}

#[test]
fn worker_rust_without_outbox_elides_the_outbox_const() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "rx_events", "rx_events.scxml");
    let out = compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("worker without outbox compiles");
    let (_, code) = out.files.first().expect("at least one file");
    assert!(
        !code.contains("pub const OUTBOX:"),
        "OUTBOX const must be elided when outbox absent:\n{code}"
    );
}

// ─── Happy: C11 emit (.h only) ──────────────────────────────────────

#[test]
fn happy_worker_c11_names_the_imported_queue_and_has_no_unit_of_its_own() {
    let ws = build_workspace();
    let scxml = worker_xml(
        "udp_scout",
        Some("session_fsm.inbox"),
        "rx_events",
        "rx_events.scxml",
    );
    let out = compile(
        &scxml,
        Language::C11,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy worker C11 compile");
    let names: Vec<&str> = out.files.iter().map(|(name, _)| name.as_str()).collect();
    assert!(
        names.contains(&"rx_loop.h"),
        "rx_loop.h emitted, got {names:?}"
    );
    assert!(
        !names.contains(&"rx_loop.c"),
        "the queue's header carries the storage; a worker has no translation unit: {names:?}"
    );
    let header = out
        .files
        .iter()
        .find(|(name, _)| name == "rx_loop.h")
        .map(|(_, c)| c)
        .expect("rx_loop.h emitted");
    assert!(
        header.contains(r#"#include "rx_events.h""#),
        "queue header include missing:\n{header}"
    );
    assert!(
        header.contains("typedef rx_events_t rx_loop_inbox_t;"),
        "inbox typedef missing:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_INBOX_QUEUE "rx_events""#),
        "INBOX_QUEUE macro missing:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_LINK_RX "udp_scout""#),
        "LINK_RX macro missing:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_OUTBOX "session_fsm.inbox""#),
        "OUTBOX macro missing:\n{header}"
    );
    for gone in ["INBOX_DEPTH", "INBOX_ORDERING", "sce_atomic_", "try_push"] {
        assert!(
            !header.contains(gone),
            "`{gone}` belongs to the queue, not the worker:\n{header}"
        );
    }
}

// ─── Cross-ref negative: link-rx-ref-unknown ─────────────────────────

#[test]
fn negative_link_rx_ref_not_imported_fires_diagnostic() {
    let ws = build_workspace();
    // link_rx points at "wrong_link" — no such kind=link import exists.
    let scxml = worker_xml(
        "wrong_link",
        Some("session_fsm.inbox"),
        "rx_events",
        "rx_events.scxml",
    );
    let err = match compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("link-rx-ref-unknown must reject"),
        Err(e) => e,
    };
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerLinkRxRefUnknown {
                worker_name,
                ref_name,
                candidates,
                ..
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(ref_name, "wrong_link");
                // Candidate set carries the legal kind=link import alias.
                assert_eq!(candidates, vec!["udp_scout".to_string()]);
            }
            other => panic!("expected WorkerLinkRxRefUnknown, got {other:?}"),
        },
        other => panic!("expected WorkerLinkRxRefUnknown, got {other:?}"),
    }
}

// Outbox cross-resolution (`worker/outbox-ref-unknown`) runs in the
// orchestrator build tier (`c2_worker_outbox.rs`). Parse time accepts
// any non-empty outbox `ref` without cross-resolution; the happy
// paths above exercise the parse-time pass-through.

// ─── Cross-ref negative: inbox-ref-unknown ───────────────────────────

#[test]
fn negative_inbox_ref_not_imported_as_a_queue_fires_diagnostic() {
    let ws = build_workspace();
    // The inbox names "ghost": no kind=queue import carries that alias. The
    // link import is not a candidate: the set is the queue-kind aliases.
    let scxml = worker_xml("udp_scout", None, "ghost", "rx_events.scxml");
    let err = match compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("inbox-ref-unknown must reject"),
        Err(e) => e,
    };
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerInboxRefUnknown {
                worker_name,
                ref_name,
                candidates,
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(ref_name, "ghost");
                assert_eq!(candidates, vec!["rx_events".to_string()]);
            }
            other => panic!("expected WorkerInboxRefUnknown, got {other:?}"),
        },
        other => panic!("expected WorkerInboxRefUnknown, got {other:?}"),
    }
}

#[test]
fn negative_inbox_ref_naming_the_link_import_is_not_a_queue() {
    let ws = build_workspace();
    // `udp_scout` IS imported, but as a link: an inbox is a queue.
    let scxml = worker_xml("udp_scout", None, "udp_scout", "rx_events.scxml");
    let err = match compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("an inbox naming a link import must reject"),
        Err(e) => e,
    };
    let diagnostics = err.to_diagnostics();
    let diag = diagnostics.first().expect("at least one diagnostic");
    assert!(
        matches!(diag.code, DiagnosticCode::WorkerInboxRefUnknown),
        "expected WorkerInboxRefUnknown, got {:?}",
        diag.code,
    );
}

// ─── Cross-ref negative: inbox-queue-not-single-consumer ─────────────

#[test]
fn negative_inbox_queue_with_many_consumers_fires_diagnostic() {
    let ws = build_workspace();
    // The worker imports `rx_fanout`, which declares consumers `many`, as its
    // inbox: a worker is the only consumer of its inbox.
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:import as="fanout" src="rx_fanout.scxml" kind="queue"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox ref="fanout"/>
</scxml>"##;
    let err = match compile(
        scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("a many-consumer inbox queue must reject"),
        Err(e) => e,
    };
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerInboxQueueNotSingleConsumer {
                worker_name,
                queue_alias,
                queue_name,
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(queue_alias, "fanout");
                assert_eq!(queue_name, "rx_fanout");
            }
            other => panic!("expected WorkerInboxQueueNotSingleConsumer, got {other:?}"),
        },
        other => panic!("expected WorkerInboxQueueNotSingleConsumer, got {other:?}"),
    }
}

// ─── The removed inline form, through the compile path ───────────────

#[test]
fn negative_inline_depth_and_ordering_fire_the_removal_diagnostic() {
    let ws = build_workspace();
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox depth="16" ordering="acq_rel"/>
</scxml>"##;
    let err = match compile(
        scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("the inline inbox must reject"),
        Err(e) => e,
    };
    match &err {
        ForgeError::Validation(boxed) => match boxed.as_ref() {
            ValidationError::WorkerInboxInlineRemoved {
                worker_name,
                removed,
                ..
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(removed, "depth=\"16\" ordering=\"acq_rel\"");
            }
            other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
        },
        other => panic!("expected WorkerInboxInlineRemoved, got {other:?}"),
    }
    let diagnostics = err.to_diagnostics();
    let diag = diagnostics.first().expect("at least one diagnostic");
    assert!(
        matches!(diag.code, DiagnosticCode::WorkerInboxInlineRemoved),
        "expected WorkerInboxInlineRemoved, got {:?}",
        diag.code,
    );
}

// ─── Diagnostic surface coverage ────────────────────────────────────

#[test]
fn cross_ref_diagnostics_carry_replace_one_of_fix() {
    let ws = build_workspace();
    let scxml = worker_xml(
        "missing_link",
        Some("ghost.inbox"),
        "rx_events",
        "rx_events.scxml",
    );
    // GeneratedOutput lacks Debug; use match-on-Err pattern.
    let err = match compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("link-rx-ref-unknown must reject (fires before outbox check)"),
        Err(e) => e,
    };
    let diagnostics = err.to_diagnostics();
    let diag = diagnostics.first().expect("at least one diagnostic");
    // DiagnosticCode lacks PartialEq; use structural `matches!`.
    assert!(
        matches!(diag.code, DiagnosticCode::WorkerLinkRxRefUnknown),
        "expected WorkerLinkRxRefUnknown, got {:?}",
        diag.code,
    );
}
