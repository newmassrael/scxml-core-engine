//! C1 integration tests — Timer kind shape migration to watching-zenoh
//! RFC §synth-5-D line 880-886 + 2 spec-named codes (timer/period-below-tick-rate +
//! timer/slot-overflow).
//!
//! Test matrix:
//! - Duration unit parsing: us / ms / s / m
//! - Happy compile (Rust + C11) with full lifecycle (period + reset-on +
//!   cancel-on + fire-event)
//! - Negative: missing <sce:period> → validation/missing-element
//! - Negative: missing <sce:fire-event> → validation/missing-element
//! - Negative: unsupported duration unit → validation/numeric-parse
//! - Forge-side negative: timer/period-below-tick-rate fires under
//!   compile_forge_with_deploy when period < scheduler.tick_period_us
//!   on cooperative
//! - Deploy-side negative: timer/slot-overflow fires when
//!   machines.<m>.timers.len() > scheduler.timer_wheel_depth
//! - Silent-skip: deploy-unaware path does not fire period check
//! - Silent-skip: scheduler.kind != cooperative does not fire period check
//! - Silent-skip: tick_period_us absent does not fire period check

use std::path::Path;
use tempfile::tempdir;

use sce_build::compile_forge_with_deploy;
use sce_build::compile_forge_with_imports;
use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::error::DeployError;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn timer_xml(period: &str, fire_event: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="keepalive">
  <sce:period>{period}</sce:period>
  <sce:fire-event>{fire_event}</sce:fire-event>
</scxml>"##
    )
}

fn compile_via_imports(scxml: &str, lang: Language, base_dir: &Path) -> Result<(), ForgeError> {
    compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("keepalive"),
        lang,
        base_dir,
        &ForgeCompileOptions::default(),
    )
    .map(|_| ())
    .map_err(|e| e.error)
}

// ─── Duration unit parsing ────────────────────────────────────────────

#[test]
fn duration_seconds_unit_parses() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("5s", "tick");
    compile_via_imports(&scxml, Language::Rust, dir.path()).expect("5s must parse");
}

#[test]
fn duration_milliseconds_unit_parses() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("250ms", "tick");
    compile_via_imports(&scxml, Language::Rust, dir.path()).expect("250ms must parse");
}

#[test]
fn duration_microseconds_unit_parses() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("500us", "tick");
    compile_via_imports(&scxml, Language::Rust, dir.path()).expect("500us must parse");
}

#[test]
fn duration_minutes_unit_parses() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("2m", "tick");
    compile_via_imports(&scxml, Language::Rust, dir.path()).expect("2m must parse");
}

#[test]
fn duration_unknown_unit_rejected() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("5h", "tick");
    let err = compile_via_imports(&scxml, Language::Rust, dir.path())
        .expect_err("unknown unit must reject");
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::NumericParse { detail, .. } => {
                assert!(
                    detail.contains("unsupported duration unit"),
                    "expected unit-error detail, got: {detail}"
                );
            }
            other => panic!("expected NumericParse, got {other:?}"),
        },
        other => panic!("expected NumericParse, got {other:?}"),
    }
}

#[test]
fn duration_missing_unit_rejected() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("500", "tick");
    let err = compile_via_imports(&scxml, Language::Rust, dir.path())
        .expect_err("missing unit must reject");
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::NumericParse { detail, .. } => {
                assert!(
                    detail.contains("missing unit suffix"),
                    "expected missing-unit detail, got: {detail}"
                );
            }
            other => panic!("expected NumericParse, got {other:?}"),
        },
        other => panic!("expected NumericParse, got {other:?}"),
    }
}

// ─── Happy compile: full §synth-5-D lifecycle ───────────────────────────────

#[test]
fn full_lifecycle_emits_rust_struct_with_reset_and_cancel() {
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="keepalive">
  <sce:period>5s</sce:period>
  <sce:reset-on event="session.msg.received"/>
  <sce:cancel-on state-exit="established"/>
  <sce:fire-event>keepalive.tick</sce:fire-event>
</scxml>"##;
    let out = compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        dir.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("full lifecycle must compile");
    let (_, code) = out.files.first().expect("at least one file");
    assert!(
        code.contains("pub const PERIOD_US: u64 = 5000000;"),
        "PERIOD_US constant missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const RESET_ON_EVENT: &'static str = "session.msg.received";"#),
        "RESET_ON_EVENT constant missing:\n{code}"
    );
    assert!(
        code.contains(r#"pub const CANCEL_ON_STATE_EXIT: &'static str = "established";"#),
        "CANCEL_ON_STATE_EXIT constant missing:\n{code}"
    );
    assert!(
        code.contains("fn on_reset_session_msg_received"),
        "reset hook method missing:\n{code}"
    );
    assert!(
        code.contains("fn on_cancel_established_exit"),
        "cancel hook method missing:\n{code}"
    );
    assert!(
        code.contains("fn fire_keepalive_tick"),
        "fire callback missing:\n{code}"
    );
}

#[test]
fn minimal_shape_emits_rust_struct_without_optional_hooks() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("100ms", "tick");
    let out = compile_forge_with_imports(
        scxml.as_str(),
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        dir.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("minimal shape must compile");
    let (_, code) = out.files.first().expect("at least one file");
    assert!(
        code.contains("pub const PERIOD_US: u64 = 100000;"),
        "PERIOD_US constant missing:\n{code}"
    );
    assert!(
        !code.contains("RESET_ON_EVENT"),
        "RESET_ON_EVENT must elide when reset-on absent:\n{code}"
    );
    assert!(
        !code.contains("CANCEL_ON_STATE_EXIT"),
        "CANCEL_ON_STATE_EXIT must elide when cancel-on absent:\n{code}"
    );
}

#[test]
fn full_lifecycle_emits_c11_header_with_reset_and_cancel() {
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="keepalive">
  <sce:period>1s</sce:period>
  <sce:reset-on event="heartbeat"/>
  <sce:cancel-on state-exit="idle"/>
  <sce:fire-event>tick</sce:fire-event>
</scxml>"##;
    let out = compile_forge_with_imports(
        scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::C11,
        dir.path(),
        &ForgeCompileOptions::default(),
    )
    .expect("C11 full lifecycle must compile");
    let (_, code) = out.files.first().expect("at least one file");
    assert!(
        code.contains("KEEPALIVE_PERIOD_US"),
        "PERIOD_US macro missing:\n{code}"
    );
    assert!(
        code.contains("keepalive_on_reset_heartbeat"),
        "reset hook missing:\n{code}"
    );
    assert!(
        code.contains("keepalive_on_cancel_idle_exit"),
        "cancel hook missing:\n{code}"
    );
}

// ─── Negative: missing required elements ──────────────────────────────

#[test]
fn missing_period_element_rejected() {
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="bad">
  <sce:fire-event>tick</sce:fire-event>
</scxml>"##;
    let err = compile_via_imports(scxml, Language::Rust, dir.path())
        .expect_err("missing period must reject");
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingElement { element, .. } => {
                assert_eq!(element, "sce:period");
            }
            other => panic!("expected MissingElement(sce:period), got {other:?}"),
        },
        other => panic!("expected MissingElement(sce:period), got {other:?}"),
    }
}

#[test]
fn missing_fire_event_rejected() {
    let dir = tempdir().expect("tempdir");
    let scxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="timer" name="bad">
  <sce:period>1s</sce:period>
</scxml>"##;
    let err = compile_via_imports(scxml, Language::Rust, dir.path())
        .expect_err("missing fire-event must reject");
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingElement { element, .. } => {
                assert_eq!(element, "sce:fire-event");
            }
            other => panic!("expected MissingElement(sce:fire-event), got {other:?}"),
        },
        other => panic!("expected MissingElement(sce:fire-event), got {other:?}"),
    }
}

// ─── timer/period-below-tick-rate (forge-side) ────────────────────────

#[test]
fn period_below_tick_rate_fires_on_cooperative() {
    let dir = tempdir().expect("tempdir");
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
"##;
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    // period_us = 500 < tick_period_us = 1000 → fires
    let scxml = timer_xml("500us", "tick");
    let err = compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        Some(&deploy),
        Some("mcu_node"),
    )
    .map(|_| ())
    .map_err(|e| e.error)
    .expect_err("period-below-tick-rate must fire");
    match err {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::TimerPeriodBelowTickRate {
                timer_name,
                machine,
                period_us,
                tick_period_us,
            } => {
                assert_eq!(timer_name, "keepalive");
                assert_eq!(machine, "mcu_node");
                assert_eq!(period_us, 500);
                assert_eq!(tick_period_us, 1000);
            }
            other => panic!("expected TimerPeriodBelowTickRate, got {other:?}"),
        },
        other => panic!("expected TimerPeriodBelowTickRate, got {other:?}"),
    }
    let _ = dir;
}

#[test]
fn period_at_or_above_tick_rate_passes() {
    let dir = tempdir().expect("tempdir");
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
"##;
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    // period_us = 1000 == tick_period_us → boundary, must pass
    let scxml = timer_xml("1000us", "tick");
    compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        Some(&deploy),
        Some("mcu_node"),
    )
    .expect("period == tick_period must pass (boundary)");
    let _ = dir;
}

#[test]
fn period_check_silent_skips_on_tokio_scheduler() {
    let dir = tempdir().expect("tempdir");
    // tokio runtime owns its own dispatch granularity; the forge-side
    // anchor must NOT fire.
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      ap_node:
        source: ap_node.scxml
        scheduler:
          kind: tokio
"##;
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    let scxml = timer_xml("100us", "tick");
    compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        Some(&deploy),
        Some("ap_node"),
    )
    .expect("tokio scheduler must silent-skip the period check");
    let _ = dir;
}

#[test]
fn period_check_silent_skips_when_tick_period_absent() {
    let dir = tempdir().expect("tempdir");
    // cooperative without tick_period_us → period check has no
    // reference point, silent-skip per the shared silent-skip
    // discipline. Other validators catch missing fields.
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
"##;
    let deploy = parse_deploy_str(yaml).expect("deploy parses");
    let scxml = timer_xml("100us", "tick");
    compile_forge_with_deploy(
        &scxml,
        DocumentLabel::symmetric("keepalive"),
        Language::Rust,
        Some(&deploy),
        Some("mcu_node"),
    )
    .expect("missing tick_period_us must silent-skip period check");
    let _ = dir;
}

#[test]
fn period_check_silent_skips_on_deploy_unaware_path() {
    let dir = tempdir().expect("tempdir");
    let scxml = timer_xml("100us", "tick");
    // No deploy at all → cannot enforce period check.
    compile_via_imports(&scxml, Language::Rust, dir.path())
        .expect("deploy-unaware path must silent-skip");
}

// ─── timer/slot-overflow (deploy-side) ────────────────────────────────

#[test]
fn slot_overflow_fires_when_timer_count_exceeds_wheel_depth() {
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
          timer_wheel_depth: 2
        timers:
          a: {}
          b: {}
          c: {}
"##;
    match parse_deploy_str(yaml) {
        Err(DeployError::TimerSlotOverflow {
            machine,
            timer_count,
            wheel_depth,
        }) => {
            assert_eq!(machine, "mcu_node");
            assert_eq!(timer_count, 3);
            assert_eq!(wheel_depth, 2);
        }
        other => panic!("expected TimerSlotOverflow, got {other:?}"),
    }
}

#[test]
fn slot_overflow_boundary_passes() {
    // count == depth — on the boundary, must pass.
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
          timer_wheel_depth: 3
        timers:
          a: {}
          b: {}
          c: {}
"##;
    parse_deploy_str(yaml).expect("count == depth must pass");
}

#[test]
fn slot_overflow_silent_skips_when_wheel_depth_absent() {
    // No timer_wheel_depth declared → validator silent-skips. The
    // timers section can grow without bound until the field is set.
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
        timers:
          a: {}
          b: {}
          c: {}
          d: {}
          e: {}
"##;
    parse_deploy_str(yaml).expect("absent wheel_depth must silent-skip");
}

#[test]
fn slot_overflow_silent_skips_on_empty_timers() {
    // Empty timers block → no overflow possible.
    let yaml = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      mcu_node:
        source: mcu_node.scxml
        scheduler:
          kind: cooperative
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          tick_period_us: 1000
          keepalive_jitter_budget_us: 5000
          timer_wheel_depth: 2
"##;
    parse_deploy_str(yaml).expect("empty timers must pass");
}
