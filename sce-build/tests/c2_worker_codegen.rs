// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! Worker kind dual-emit codegen + cross-resolution + inbox
//! ordering integration fixtures.
//!
//! Per RFC §5.D + §5.I (textbook narrowing 2026-05-11 after
//! Gate B preflight): worker cross-refs validate directly against
//! `parsed.imports` filtered by kind (η-precedent), inbox ordering is
//! a required parse-time attribute, and cross-core ordering is a
//! codegen-invariant guard gated on `ForgeCompileOptions.worker_placement`.
//!
//! Coverage matrix:
//!   - Happy Rust dual-emit (`worker.rs.jinja2` ring buffer)
//!   - Happy C11 dual-emit (`.h` + `.c` sibling)
//!   - Cross-ref negative × 2 (link-rx-ref-unknown, outbox-ref-unknown)
//!   - Ordering negative × 2 (inbox-ordering-unspecified, inbox-ordering-
//!     relaxed-across-cores via populated worker_placement slice)
//!   - Codegen-invariant force fixture (placement-aware silent-skip
//!     on deploy-unaware path)
//!   - Ordering attribute round-trip on both backends

use std::fs;
use std::path::Path;
use tempfile::tempdir;

use sce_build::compile_forge_with_imports;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::{DocumentLabel, ForgeCompileOptions, WorkerPlacement};

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

/// Wrap the link fixture into a tempdir; return the path. Worker
/// fixture's `<sce:import kind="link">` references it by relative
/// `src`. Outbox cross-resolution runs only in the orchestrator
/// path (`compile_scxml_with_imports`, covered in
/// `c2_worker_outbox.rs`); in this single-file harness outbox refs
/// are parser-validated only (presence + format), so no statechart
/// fixture is needed.
fn build_workspace() -> tempfile::TempDir {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("udp_scout.scxml"), link_fixture()).expect("write link");
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

fn worker_xml(link_rx: &str, outbox: Option<&str>, ordering: &str, depth: u32) -> String {
    let outbox_line = match outbox {
        Some(o) => format!(r##"<sce:outbox ref="{o}"/>"##),
        None => String::new(),
    };
    // Worker docs import their driving link kind via `<sce:import>`
    // for the η-precedent cross-resolution shape. Outbox owner-prefix
    // (e.g. `session_fsm`) does NOT need an import declaration today
    // — outbox cross-resolution defers to a follow-up SCXML-side
    // atomic, so parse-time validation accepts any non-empty `ref`.
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:link-rx ref="{link_rx}"/>
  <sce:inbox depth="{depth}" ordering="{ordering}"/>
  {outbox_line}
</scxml>"##
    )
}

// ─── Happy: Rust dual-emit ──────────────────────────────────────────

#[test]
fn happy_worker_rust_emits_inbox_producer_consumer_split() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", Some("session_fsm.inbox"), "acq_rel", 16);
    let out = compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy worker Rust compile");
    let (_, code) = out.files.first().expect("at least one file");
    // Surface assertions on the emitted template structure.
    assert!(
        code.contains("pub const DEPTH: usize = 16;"),
        "DEPTH const missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const ORDERING: &'static str = "acq_rel";"#),
        "ORDERING const missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const LINK_RX: &'static str = "udp_scout";"#),
        "LINK_RX const missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const OUTBOX: &'static str = "session_fsm.inbox";"#),
        "OUTBOX const missing:\n{code}"
    );
    assert!(
        code.contains("pub struct RxLoopInbox<E>"),
        "Inbox struct missing:\n{code}"
    );
    assert!(
        code.contains("pub struct RxLoopProducer<'a, E>"),
        "Producer struct missing:\n{code}"
    );
    assert!(
        code.contains("pub struct RxLoopConsumer<'a, E>"),
        "Consumer struct missing:\n{code}"
    );
    assert!(
        code.contains("fn try_push(&mut self, event: E) -> Result<(), E>"),
        "try_push missing:\n{code}"
    );
    assert!(
        code.contains("fn try_pop(&mut self) -> Option<E>"),
        "try_pop missing:\n{code}"
    );
    // acq_rel ordering selects Acquire/Release atomic ops.
    assert!(
        code.contains("const HEAD_LOAD_ORD: Ordering = Ordering::Acquire;"),
        "Acquire head load missing:\n{code}"
    );
    assert!(
        code.contains("const TAIL_STORE_ORD: Ordering = Ordering::Release;"),
        "Release tail store missing:\n{code}"
    );
    assert!(
        !code.contains("Ordering::Relaxed"),
        "Relaxed ordering must NOT appear in acq_rel template output:\n{code}"
    );
}

#[test]
fn happy_worker_rust_relaxed_ordering_emits_relaxed_ops() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "relaxed", 8);
    let out = compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy relaxed worker Rust compile");
    let (_, code) = out.files.first().expect("at least one file");
    assert!(
        code.contains(r#"pub const ORDERING: &'static str = "relaxed";"#),
        "ORDERING relaxed const missing:\n{code}"
    );
    assert!(
        code.contains("const HEAD_LOAD_ORD: Ordering = Ordering::Relaxed;"),
        "Relaxed head load missing:\n{code}"
    );
    assert!(
        code.contains("const TAIL_STORE_ORD: Ordering = Ordering::Relaxed;"),
        "Relaxed tail store missing:\n{code}"
    );
    assert!(
        !code.contains("Ordering::Acquire"),
        "Acquire must NOT appear in relaxed template output:\n{code}"
    );
    // Outbox absent → no OUTBOX const emitted.
    assert!(
        !code.contains("pub const OUTBOX:"),
        "OUTBOX const must be elided when outbox absent:\n{code}"
    );
}

// ─── Happy: C11 dual-emit (.h + .c sibling) ──────────────────────────

#[test]
fn happy_worker_c11_emits_header_and_impl_sibling() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", Some("session_fsm.inbox"), "acq_rel", 32);
    let out = compile(
        &scxml,
        Language::C11,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy worker C11 compile");
    let mut header = None;
    let mut source = None;
    for (name, content) in &out.files {
        if name == "rx_loop.h" {
            header = Some(content);
        } else if name == "rx_loop.c" {
            source = Some(content);
        }
    }
    let header = header.expect("rx_loop.h emitted");
    let source = source.expect("rx_loop.c emitted");
    // Header shape:
    assert!(
        header.contains("#define RX_LOOP_INBOX_DEPTH ((size_t)32)"),
        "DEPTH macro missing in .h:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_INBOX_ORDERING "acq_rel""#),
        "ORDERING macro missing:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_LINK_RX "udp_scout""#),
        "LINK_RX macro missing:\n{header}"
    );
    assert!(
        header.contains(r#"#define RX_LOOP_OUTBOX "session_fsm.inbox""#),
        "OUTBOX macro missing:\n{header}"
    );
    assert!(
        header.contains("typedef struct rx_loop_inbox_producer_s rx_loop_inbox_producer_t;"),
        "Producer typedef missing:\n{header}"
    );
    assert!(
        header.contains("typedef struct rx_loop_inbox_consumer_s rx_loop_inbox_consumer_t;"),
        "Consumer typedef missing:\n{header}"
    );
    assert!(
        header.contains("bool rx_loop_inbox_try_push("),
        "try_push prototype missing:\n{header}"
    );
    assert!(
        header.contains("bool rx_loop_inbox_try_pop("),
        "try_pop prototype missing:\n{header}"
    );
    // Source shape — acq_rel selects acquire/release variants of the
    // §5.I baseline atomic family.
    assert!(
        source.contains("sce_atomic_load_acquire_u32"),
        "acquire load missing in .c:\n{source}"
    );
    assert!(
        source.contains("sce_atomic_store_release_u32"),
        "release store missing in .c:\n{source}"
    );
    assert!(
        !source.contains("sce_atomic_load_relaxed_u32(&g_head)")
            && !source.contains("sce_atomic_load_relaxed_u32(&g_tail)"),
        "relaxed atomic ops must NOT appear when ordering=acq_rel:\n{source}"
    );
    assert!(
        source.contains("static volatile uint32_t g_storage[RX_LOOP_INBOX_DEPTH]"),
        "storage array missing in .c:\n{source}"
    );
}

#[test]
fn happy_worker_c11_relaxed_emits_relaxed_atomic_variants() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "relaxed", 4);
    let out = compile(
        &scxml,
        Language::C11,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("happy relaxed worker C11 compile");
    let source = out
        .files
        .iter()
        .find(|(name, _)| name == "rx_loop.c")
        .map(|(_, c)| c)
        .expect("rx_loop.c emitted");
    assert!(
        source.contains("sce_atomic_load_relaxed_u32"),
        "relaxed load missing in .c:\n{source}"
    );
    assert!(
        source.contains("sce_atomic_store_relaxed_u32"),
        "relaxed store missing in .c:\n{source}"
    );
    assert!(
        !source.contains("sce_atomic_load_acquire_u32(&g_head)")
            && !source.contains("sce_atomic_load_acquire_u32(&g_tail)"),
        "acquire variant must NOT appear in head/tail when ordering=relaxed:\n{source}"
    );
}

// ─── Cross-ref negative: link-rx-ref-unknown ─────────────────────────

#[test]
fn negative_link_rx_ref_not_imported_fires_diagnostic() {
    let ws = build_workspace();
    // link_rx points at "wrong_link" — no such kind=link import exists.
    let scxml = worker_xml("wrong_link", Some("session_fsm.inbox"), "acq_rel", 16);
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

// ─── Ordering negative: inbox-ordering-unspecified ───────────────────

#[test]
fn negative_inbox_missing_ordering_fires_diagnostic() {
    let ws = build_workspace();
    // Hand-build worker SCXML without ordering attribute on inbox.
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox depth="16"/>
</scxml>"##;
    let err = match compile(
        scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("inbox-ordering-unspecified must reject"),
        Err(e) => e,
    };
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerInboxOrderingUnspecified { worker_name } => {
                assert_eq!(worker_name, "rx_loop");
            }
            other => panic!("expected WorkerInboxOrderingUnspecified, got {other:?}"),
        },
        other => panic!("expected WorkerInboxOrderingUnspecified, got {other:?}"),
    }
}

// ─── Ordering negative: inbox-ordering-relaxed-across-cores ──────────

#[test]
fn negative_relaxed_ordering_with_cross_core_placement_fires_diagnostic() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "relaxed", 16);
    // Populated placement slice: producer on core 0, consumer on core 1.
    // Combined with `ordering="relaxed"` → codegen-invariant fires.
    let options = ForgeCompileOptions {
        worker_placement: Some(vec![WorkerPlacement {
            worker_name: "rx_loop".to_string(),
            producer_core: 0,
            consumer_core: 1,
        }]),
        ..Default::default()
    };
    let err = match compile(&scxml, Language::Rust, ws.path(), &options) {
        Ok(_) => panic!("relaxed-across-cores must reject"),
        Err(e) => e,
    };
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::WorkerInboxOrderingRelaxedAcrossCores {
                worker_name,
                producer_core,
                consumer_core,
            } => {
                assert_eq!(worker_name, "rx_loop");
                assert_eq!(producer_core, 0);
                assert_eq!(consumer_core, 1);
            }
            other => panic!("expected WorkerInboxOrderingRelaxedAcrossCores, got {other:?}"),
        },
        other => panic!("expected WorkerInboxOrderingRelaxedAcrossCores, got {other:?}"),
    }
}

// ─── Codegen-invariant silent-skip behavior ─────────────────────────

#[test]
fn relaxed_ordering_with_same_core_placement_passes() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "relaxed", 16);
    // Same core for producer + consumer → relaxed is legal.
    let options = ForgeCompileOptions {
        worker_placement: Some(vec![WorkerPlacement {
            worker_name: "rx_loop".to_string(),
            producer_core: 0,
            consumer_core: 0,
        }]),
        ..Default::default()
    };
    compile(&scxml, Language::Rust, ws.path(), &options).expect("same-core relaxed must compile");
}

#[test]
fn relaxed_ordering_silent_skip_when_placement_absent() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "relaxed", 16);
    // No placement → silent-skip per the absent-input precedent.
    compile(
        &scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("deploy-unaware path must silent-skip the codegen-invariant");
}

#[test]
fn acq_rel_ordering_with_cross_core_placement_passes() {
    let ws = build_workspace();
    let scxml = worker_xml("udp_scout", None, "acq_rel", 16);
    let options = ForgeCompileOptions {
        worker_placement: Some(vec![WorkerPlacement {
            worker_name: "rx_loop".to_string(),
            producer_core: 0,
            consumer_core: 1,
        }]),
        ..Default::default()
    };
    compile(&scxml, Language::Rust, ws.path(), &options)
        .expect("acq_rel + cross-core must compile (safe combination)");
}

// ─── Diagnostic surface coverage ────────────────────────────────────

#[test]
fn cross_ref_diagnostics_carry_replace_one_of_fix() {
    let ws = build_workspace();
    let scxml = worker_xml("missing_link", Some("ghost.inbox"), "acq_rel", 16);
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

#[test]
fn ordering_diagnostics_resolve_to_their_codes() {
    let ws = build_workspace();
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="worker" name="rx_loop" version="1.0">
  <sce:import as="udp_scout" src="udp_scout.scxml" kind="link"/>
  <sce:link-rx ref="udp_scout"/>
  <sce:inbox depth="16"/>
</scxml>"##;
    let err = match compile(
        scxml,
        Language::Rust,
        ws.path(),
        &ForgeCompileOptions::default(),
    ) {
        Ok(_) => panic!("inbox-ordering-unspecified must fire"),
        Err(e) => e,
    };
    let diagnostics = err.to_diagnostics();
    let diag = diagnostics.first().expect("at least one diagnostic");
    assert!(
        matches!(diag.code, DiagnosticCode::WorkerInboxOrderingUnspecified),
        "expected WorkerInboxOrderingUnspecified, got {:?}",
        diag.code,
    );
}
