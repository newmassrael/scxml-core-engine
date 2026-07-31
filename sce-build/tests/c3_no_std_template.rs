//! Integration tests — `<sce:capacity>` SCXML attribute +
//! `default_event_queue_capacity` deploy.yaml fallback +
//! Rust codegen template `pub const EVENT_QUEUE_CAPACITY`
//! emission + `.cargo/config.toml` thumbv7em target registration.
//!
//! SCE Protocol-Synthesis RFC §synth-5-J-2 + §synth-5-L. The per-document capacity
//! flows: parser reads `<scxml sce:capacity>`
//! → SCXMLModel.event_queue_capacity → (optional fallback from
//! deploy.yaml) → Rust template emits `pub const EVENT_QUEUE_CAPACITY`
//! when present. The no_std runtime port consumes the const for
//! `heapless::spsc::Queue<E, EVENT_QUEUE_CAPACITY>`.
//!
//! The template-level `#![no_std]` switch is gated on the
//! `--no-std` flag (emission contract pinned in the section below);
//! the capacity plumbing covered here is profile-independent.

use std::path::Path;

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::generate;
use sce_build::parser::SCXMLParser;

const PLAIN_FSM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <state id="s1"/>
</scxml>
"#;

const FSM_WITH_CAPACITY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0" sce:capacity="32">
  <state id="s0">
    <transition event="go" target="s1"/>
  </state>
  <state id="s1"/>
</scxml>
"#;

// Parallel-state fixture: the microstep dedup set emission lives under
// `{% if model.has_parallel_states %}` in process_transition.rs.jinja2 —
// atomic-only fixtures never reach the dedup block, so the `SceDedupSet`
// alias assertions need a fixture that actually exercises the
// parallel-state transition path.
const PARALLEL_FSM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
  <parallel id="p">
    <state id="r1">
      <transition event="go" target="r1"/>
    </state>
    <state id="r2">
      <transition event="go" target="r2"/>
    </state>
  </parallel>
</scxml>
"#;

fn parse(content: &str, label: &str) -> sce_build::model::SCXMLModel {
    let mut parser = SCXMLParser::new();
    parser
        .parse_string(content, label)
        .unwrap_or_else(|e| panic!("parse failed for {label}: {:?}", e.error))
}

fn template_dir() -> std::path::PathBuf {
    sce_build::find_template_dir_for(sce_build::generator::Language::Rust)
}

#[test]
fn sce_capacity_attribute_is_parsed_into_model() {
    let model = parse(FSM_WITH_CAPACITY, "cap_fsm");
    assert_eq!(model.event_queue_capacity, Some(32));
}

#[test]
fn missing_sce_capacity_attribute_leaves_model_field_none() {
    let model = parse(PLAIN_FSM, "plain");
    assert_eq!(model.event_queue_capacity, None);
}

#[test]
fn malformed_sce_capacity_attribute_emits_invalid_attribute() {
    let malformed = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0" sce:capacity="not-a-number">
  <state id="s0"/>
</scxml>
"#;
    let mut parser = SCXMLParser::new();
    let err = parser
        .parse_string(malformed, "bad_cap")
        .expect_err("non-numeric sce:capacity must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::InvalidAttribute {
                element,
                attr,
                value,
                ..
            } => {
                assert_eq!(element, "scxml");
                assert_eq!(attr, "sce:capacity");
                assert_eq!(value, "not-a-number");
            }
            other => panic!("expected InvalidAttribute, got: {other:?}"),
        },
        other => panic!("expected InvalidAttribute, got: {other:?}"),
    }
}

#[test]
fn zero_sce_capacity_attribute_rejects() {
    // Capacity is the event-queue size; zero is
    // not a meaningful value. Reject at parse time so downstream
    // codegen cannot emit a 0-sized `heapless::Vec<E, 0>` (which
    // compiles but always overflows on first push).
    let zero_cap = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0" sce:capacity="0">
  <state id="s0"/>
</scxml>
"#;
    let mut parser = SCXMLParser::new();
    let err = parser
        .parse_string(zero_cap, "zero_cap")
        .expect_err("zero sce:capacity must reject");
    assert!(matches!(
        err.error,
        ForgeError::Validation(ref boxed) if matches!(**boxed, ValidationError::InvalidAttribute { .. })
    ));
}

#[test]
fn rust_template_emits_event_queue_capacity_const_when_set() {
    let model = parse(FSM_WITH_CAPACITY, "cap_fsm");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        code.contains("pub const EVENT_QUEUE_CAPACITY: usize = 32;"),
        "expected `pub const EVENT_QUEUE_CAPACITY: usize = 32;` in generated code, got first 100 lines:\n{}",
        code.lines().take(100).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn rust_template_omits_event_queue_capacity_const_when_absent() {
    let model = parse(PLAIN_FSM, "plain");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        !code.contains("EVENT_QUEUE_CAPACITY"),
        "no capacity declared ⇒ no const emitted"
    );
}

/// Sub-template `std::*` → `core::*` swaps keep the generated code
/// no_std-compatible.
/// Under std builds the swap is behavior-preserving — `core::time::*`
/// and `core::sync::atomic::*` are the same types `std::time::*` /
/// `std::sync::atomic::*` re-export. The drift guard locks in the
/// `core::*` spelling so a future template edit that reverts back to
/// `std::*` paths surfaces the regression here rather than waiting
/// for a `--no-std` build to fire compile errors against
/// the generated code.
#[test]
fn rust_template_emits_core_time_duration_not_std() {
    let model = parse(PLAIN_FSM, "plain");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        code.contains("use core::time::Duration;"),
        "template must emit `use core::time::Duration;` (B-γ2a swap)"
    );
    assert!(
        !code.contains("use std::time::Duration;"),
        "template must not emit `use std::time::Duration;` after B-γ2a swap"
    );
}

#[test]
fn populate_event_queue_capacity_from_deploy_fills_missing_attribute() {
    // Author omits `<scxml sce:capacity>`; deploy.yaml supplies
    // `machines.<m>.scheduler.default_event_queue_capacity`. The
    // populator threads the fallback into the model so codegen can
    // emit the same `pub const EVENT_QUEUE_CAPACITY` it would have
    // for a per-instance declaration.
    let dir = tempfile::tempdir().expect("tempdir");
    let deploy_path = dir.path().join("deploy.yaml");
    std::fs::write(
        &deploy_path,
        r#"version: "1.0"
topology:
  dev1:
    machines:
      plain:
        source: plain.scxml
        platform:
          class: ap
          os: linux
        scheduler:
          kind: cooperative
          worker_stack_budget: 1024
          tick_period_us: 1000
          worker_slot_budget_us: 100
          keepalive_jitter_budget_us: 50
          default_event_queue_capacity: 64
"#,
    )
    .expect("deploy.yaml write");

    let mut model = parse(PLAIN_FSM, "plain");
    assert_eq!(model.event_queue_capacity, None);
    sce_build::populate_event_queue_capacity_from_deploy(&mut model, &deploy_path)
        .expect("populator must succeed");
    assert_eq!(model.event_queue_capacity, Some(64));
}

#[test]
fn populate_event_queue_capacity_from_deploy_preserves_instance_attribute() {
    // Per-instance attribute wins over deploy.yaml fallback per
    // the capacity resolution rule.
    let dir = tempfile::tempdir().expect("tempdir");
    let deploy_path = dir.path().join("deploy.yaml");
    std::fs::write(
        &deploy_path,
        r#"version: "1.0"
topology:
  dev1:
    machines:
      cap_fsm:
        source: cap_fsm.scxml
        platform:
          class: ap
          os: linux
        scheduler:
          kind: cooperative
          worker_stack_budget: 1024
          tick_period_us: 1000
          worker_slot_budget_us: 100
          keepalive_jitter_budget_us: 50
          default_event_queue_capacity: 128
"#,
    )
    .expect("deploy.yaml write");

    let mut model = parse(FSM_WITH_CAPACITY, "cap_fsm");
    assert_eq!(model.event_queue_capacity, Some(32));
    sce_build::populate_event_queue_capacity_from_deploy(&mut model, &deploy_path)
        .expect("populator must succeed");
    // 32 from the instance attribute survives — not overwritten by 128.
    assert_eq!(model.event_queue_capacity, Some(32));
}

#[test]
fn populate_event_queue_capacity_from_deploy_silent_skip_when_field_absent() {
    // Deploy parses but lacks the new field ⇒ model stays None.
    // Mirrors cache_platform / worker_placement silent-skip
    // precedent (absent-input silent-skip).
    let dir = tempfile::tempdir().expect("tempdir");
    let deploy_path = dir.path().join("deploy.yaml");
    std::fs::write(
        &deploy_path,
        r#"version: "1.0"
topology:
  dev1:
    machines:
      plain:
        source: plain.scxml
        platform:
          class: ap
          os: linux
"#,
    )
    .expect("deploy.yaml write");

    let mut model = parse(PLAIN_FSM, "plain");
    sce_build::populate_event_queue_capacity_from_deploy(&mut model, &deploy_path)
        .expect("populator must succeed");
    assert_eq!(model.event_queue_capacity, None);
}

// ═══════════════════════════════════════════════════════════════════
// Single-emit portability — `--no-std` emission contract
// ═══════════════════════════════════════════════════════════════════
//
// The generator takes a `no_std: bool` parameter (the `--no-std`
// CLI flag, wired through `cmd_generate`). When set, the Rust template
// emits `#![no_std]` at the crate root and elides the
// `parent_external_queue` field whose `Arc<Mutex<...>>` type is
// alloc-coupled (per SCE Protocol-Synthesis RFC §synth-5-J-2 lines 1989-1994 —
// "no path from generated no_std code into alloc::*"). Owned
// collection TYPES, however, are profile-neutral: both modes name the
// runtime's profile-resolving aliases (SceBytes / SceString /
// SceTransitionBuf / SceIndexBuf / SceDedupSet / StateChain), and the
// runtime's own cfg selects std vs heapless. One emission therefore
// compiles against both runtime profiles (sce-portable-emit-probe
// gates this); these tests pin the emission side.

#[test]
fn rust_template_emits_no_std_attribute_under_no_std_flag() {
    let model = parse(PLAIN_FSM, "plain_nostd");
    let code = generate(&model, &template_dir(), true).expect("template render must succeed");
    assert!(
        code.contains("#![no_std]"),
        "--no-std flag must emit `#![no_std]` crate attribute"
    );
}

#[test]
fn rust_template_omits_no_std_attribute_under_default_std() {
    let model = parse(PLAIN_FSM, "plain_std");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        !code.contains("#![no_std]"),
        "default (std) codegen must not emit `#![no_std]`"
    );
}

#[test]
fn rust_template_emits_dedup_set_alias_under_no_std() {
    let model = parse(PARALLEL_FSM, "parallel_nostd_dedup");
    let code = generate(&model, &template_dir(), true).expect("template render must succeed");
    // The runtime owns the std-vs-heapless set choice: the emission names the
    // profile-resolving `SceDedupSet` alias + `dedup_insert` helper, never a
    // raw `heapless::FnvIndexSet` or `std::collections::HashSet` (those are the
    // runtime's concern, cfg-selected behind the alias).
    assert!(
        code.contains("::sce_rust_runtime::SceDedupSet"),
        "--no-std emit must name the SceDedupSet alias for the microstep dedup set"
    );
    assert!(
        code.contains("::sce_rust_runtime::dedup_insert"),
        "--no-std emit must use the dedup_insert helper"
    );
    assert!(
        !code.contains("sce_rust_runtime::heapless"),
        "emit must not reference the no_std-only heapless re-export (runtime owns it via SceDedupSet)"
    );
    assert!(
        !code.contains("std::collections::HashSet"),
        "--no-std emit must not name std::collections::HashSet"
    );
}

#[test]
fn rust_template_emits_dedup_set_alias_under_default_std() {
    let model = parse(PARALLEL_FSM, "parallel_std_dedup");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    // Profile-neutral: the std emission names the SAME `SceDedupSet` alias as
    // the no_std emission (the runtime resolves it to `HashSet` under std), so
    // one emission is portable across both runtime profiles.
    assert!(
        code.contains("::sce_rust_runtime::SceDedupSet"),
        "default (std) emit must name the SceDedupSet alias for the microstep dedup set"
    );
    assert!(
        code.contains("::sce_rust_runtime::dedup_insert"),
        "default (std) emit must use the dedup_insert helper"
    );
    assert!(
        !code.contains("std::collections::HashSet"),
        "emit must not name std::collections::HashSet directly (runtime owns it via SceDedupSet)"
    );
    assert!(
        !code.contains("sce_rust_runtime::heapless"),
        "default (std) emit must not reference the no_std-only heapless re-export"
    );
}

#[test]
fn rust_template_emits_profile_neutral_collection_aliases() {
    // Core of the single-emit portability contract: a parallel chart's
    // transition buffers name profile-resolving runtime aliases
    // (SceTransitionBuf / SceIndexBuf) identically under both codegen modes,
    // and NEITHER emit references the no_std-only `sce_rust_runtime::heapless`
    // re-export — whose presence in a no_std emit was the
    // `E0433: cannot find heapless in sce_rust_runtime` bug against the std
    // runtime. `sce-portable-emit-probe` is the compile-level twin gate.
    let model = parse(PARALLEL_FSM, "parallel_neutral");
    for no_std in [false, true] {
        let code = generate(&model, &template_dir(), no_std).expect("render");
        assert!(
            code.contains("::sce_rust_runtime::SceTransitionBuf<TransitionInfo>"),
            "emit (no_std={no_std}) must name SceTransitionBuf for the transition buffer"
        );
        assert!(
            code.contains("::sce_rust_runtime::SceIndexBuf"),
            "emit (no_std={no_std}) must name SceIndexBuf for the index buffer"
        );
        assert!(
            !code.contains("sce_rust_runtime::heapless"),
            "emit (no_std={no_std}) must not reference the no_std-only heapless re-export"
        );
    }
}

#[test]
fn rust_template_elides_parent_external_queue_field_under_no_std() {
    let model = parse(PLAIN_FSM, "plain_nostd_pq");
    let code = generate(&model, &template_dir(), true).expect("template render must succeed");
    // The Arc<Mutex<Vec<...>>> type is alloc-coupled (`alloc::sync::Arc`
    // / `std::sync::Mutex`), so the field is elided entirely under
    // --no-std. SCXML <invoke> is also codegen-rejected under --no-std
    // (B-γ2c diagnostic), so no usage site for this field ever appears
    // in a valid --no-std generation.
    assert!(
        !code.contains("parent_external_queue"),
        "--no-std codegen must elide the parent_external_queue field"
    );
}

#[test]
fn rust_template_emits_parent_external_queue_field_under_default_std() {
    let model = parse(PLAIN_FSM, "plain_std_pq");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        code.contains("pub parent_external_queue:"),
        "default (std) codegen must emit the parent_external_queue field"
    );
    assert!(
        code.contains("parent_external_queue: None"),
        "default (std) codegen must initialize parent_external_queue to None"
    );
}

#[test]
fn rust_template_omits_std_collections_hashset_under_no_std_with_capacity() {
    let model = parse(FSM_WITH_CAPACITY, "cap_nostd");
    let code = generate(&model, &template_dir(), true).expect("template render must succeed");
    // Capacity-aware fixture still must not pull std::collections::HashSet.
    assert!(
        !code.contains("std::collections::HashSet"),
        "no_std + capacity must not regress to std::collections::HashSet"
    );
    assert!(
        code.contains("pub const EVENT_QUEUE_CAPACITY: usize = 32;"),
        "no_std + capacity must still emit EVENT_QUEUE_CAPACITY const (B-γ1)"
    );
}

#[test]
fn rust_template_no_std_emission_is_byte_idempotent() {
    // Same model + same no_std flag = byte-identical output (jinja2
    // determinism). Locks in that the no_std switch has no nondetermin-
    // istic side effects (e.g., HashMap iteration order in context).
    let model = parse(PLAIN_FSM, "plain_idempotent");
    let a = generate(&model, &template_dir(), true).expect("first render");
    let b = generate(&model, &template_dir(), true).expect("second render");
    assert_eq!(
        a, b,
        "no_std=true must produce byte-identical output across calls"
    );
}

#[test]
fn cargo_config_declares_thumbv7em_target() {
    // Workspace `.cargo/config.toml` registers the canonical Cortex-
    // M4F target so consumers can build with
    // `cargo --target=thumbv7em-none-eabihf` from any directory.
    // The no_std runtime port makes the build actually succeed;
    // this test pins presence of the section + linker config.
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let cfg = std::fs::read_to_string(workspace_root.join(".cargo").join("config.toml"))
        .expect("workspace `.cargo/config.toml` must exist");
    assert!(
        cfg.contains("[target.thumbv7em-none-eabihf]"),
        ".cargo/config.toml must declare the thumbv7em-none-eabihf target"
    );
    assert!(
        cfg.contains("linker = \"rust-lld\""),
        "thumbv7em target must specify rust-lld as linker"
    );
}
