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

// Parallel-state fixture: a chart with a `<parallel>` is one whose
// configuration holds several states at once and whose transitions select
// across regions, so the collection-alias and microstep assertions need a
// fixture that actually has one.
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
fn malformed_sce_capacity_attribute_is_refused_by_its_rule() {
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
    // "A positive u32" is a rule, not a set of values, so the refusal
    // carries it as the rule and offers no candidates.
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::AttributeRuleViolated {
                element,
                attr,
                value,
                ..
            } => {
                assert_eq!(element, "scxml");
                assert_eq!(attr, "sce:capacity");
                assert_eq!(value, "not-a-number");
            }
            other => panic!("expected AttributeRuleViolated, got: {other:?}"),
        },
        other => panic!("expected AttributeRuleViolated, got: {other:?}"),
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
        ForgeError::Validation(ref boxed) if matches!(**boxed, ValidationError::AttributeRuleViolated { .. })
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
    // Mirrors the cache_platform silent-skip precedent (absent-input
    // silent-skip).
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
// BoundedVec), and the runtime's own cfg selects std vs heapless. One
// emission therefore compiles against both runtime profiles
// (sce-portable-emit-probe gates this); these tests pin the emission side.

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
fn rust_template_leaves_the_microstep_to_the_runtime() {
    // W3C SCXML Appendix D's microstep is the runtime's
    // (`sce_rust_runtime::helpers::microstep`), for a `<parallel>` chart as
    // for every other: the emission answers only which of a state's
    // transitions an event enables, as the number of its row in the
    // document's tables (docs/adr/0064), and carries no conflict resolution,
    // exit set or entry walk of its own — nor the collections one would need,
    // nor the structure the runtime reads from the tables. It used to carry a
    // whole second microstep for exactly this shape, with its own dedup set
    // and buffers.
    //
    // The chart is p(0) > r1(1), r2(2), and its rows are r1's `go` (0), r2's
    // `go` (1) and the document's initial transition (2).
    let model = parse(PARALLEL_FSM, "parallel_microstep");
    for no_std in [false, true] {
        let code = generate(&model, &template_dir(), no_std).expect("render");
        for expected in [
            "fn first_enabled_transition(",
            ") -> Option<::sce_rust_runtime::TransitionId> {",
            "return Some(::sce_rust_runtime::TransitionId(0));",
            "return Some(::sce_rust_runtime::TransitionId(1));",
            "transition: ::sce_rust_runtime::TransitionId,",
            "fn execute_exit_actions(\n        &mut self,\n        state: Self::State,\n        \
             engine: &mut sce_rust_runtime::Engine<Self>,\n    ) {",
        ] {
            assert!(
                code.contains(expected),
                "emit (no_std={no_std}) must carry `{expected}`:\n{code}"
            );
        }
        for procedure in [
            "fn remove_conflicting_transitions",
            "fn compute_exit_set",
            "fn execute_microstep",
            "fn try_transition_in_state",
            "SceDedupSet",
            "EnabledTransition",
            "EntryTarget",
            "fn get_parent(",
            "fn is_final_state(",
            "fn is_compound_state(",
            "fn is_parallel_state(",
            "fn get_child_states(",
            "fn get_initial_targets(",
            "fn get_document_initial_targets(",
            "fn get_history_parent(",
            "fn get_history_default_targets(",
            "fn get_document_order(",
            "fn history_value(",
            "fn get_active_states(",
            "fn set_active_states(",
            "fn activate(",
            "fn deactivate(",
            "configuration_before_exit",
            "PolicyDocument",
        ] {
            assert!(
                !code.contains(procedure),
                "emit (no_std={no_std}) must not carry `{procedure}` — the microstep is the runtime's"
            );
        }
    }
}

#[test]
fn rust_template_emits_profile_neutral_collection_aliases() {
    // Core of the single-emit portability contract: a parallel chart's active
    // set names the profile-resolving `BoundedVec` alias identically under both
    // codegen modes, and NEITHER emit names a std or heapless collection
    // directly — the no_std-only `sce_rust_runtime::heapless` re-export's
    // presence in a no_std emit was the `E0433: cannot find heapless in
    // sce_rust_runtime` bug against the std runtime. `sce-portable-emit-probe`
    // is the compile-level twin gate.
    let model = parse(PARALLEL_FSM, "parallel_neutral");
    for no_std in [false, true] {
        let code = generate(&model, &template_dir(), no_std).expect("render");
        // The configuration a host observes is the machine's own
        // Configuration, sized to its tree (p, r1 and r2); the engine keeps
        // the configuration itself, so the policy holds no active set
        // (docs/adr/0064).
        assert!(
            code.contains(
                "pub type ParallelNeutralConfiguration = \
                 ::sce_rust_runtime::helpers::hierarchy::BoundedVec<ParallelNeutralState, 3>;"
            ),
            "emit (no_std={no_std}) must name BoundedVec for the configuration"
        );
        assert!(
            code.contains("type Configuration = ParallelNeutralConfiguration;"),
            "emit (no_std={no_std}) must answer the configuration in the machine's own \
             Configuration"
        );
        for own_set in ["active_states:", "self.active_states"] {
            assert!(
                !code.contains(own_set),
                "emit (no_std={no_std}) must not keep an active set of its own (`{own_set}`)"
            );
        }
        assert!(
            !code.contains("sce_rust_runtime::heapless"),
            "emit (no_std={no_std}) must not reference the no_std-only heapless re-export"
        );
        assert!(
            !code.contains("std::collections::HashSet"),
            "emit (no_std={no_std}) must not name std::collections::HashSet directly"
        );
    }
}

/// The lines of `code` that start, once their indentation is trimmed, with
/// `head`, trimmed, in the order they are emitted.
fn lines_starting(code: &str, head: &str) -> Vec<String> {
    code.lines()
        .map(str::trim)
        .filter(|line| line.starts_with(head))
        .map(str::to_string)
        .collect()
}

/// The variants of the enum `pub enum <name> {` declares, each line trimmed,
/// without its comments and attributes.
fn enum_variants(code: &str, name: &str) -> Vec<String> {
    let open = format!("pub enum {name} {{");
    code.lines()
        .skip_while(|line| line.trim() != open)
        .skip(1)
        .take_while(|line| line.trim() != "}")
        .map(str::trim)
        .filter(|line| !line.starts_with("//") && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

#[test]
fn rust_template_sizes_the_microstep_to_the_documents_state_tree() {
    // The runtime names no capacity of its own for the buffers of Appendix D's
    // microstep, and reads a document only as its tables (docs/adr/0064): the
    // generator reads the document's tree (`sce_build::document_tables`,
    // `sce_build::microstep_bounds`) and writes the rows and the sizes on the
    // machine. The MCU probe machine is a `<parallel>` of three regions, one of
    // them holding a second `<parallel>`, numbered in document order:
    //
    //   top(0) (parallel)
    //     ra(1) > a1(2) a2(3) a3(4)      + ha_deep, deep history (0)
    //     rb(5) > b1(6) b2(7)            + hb_shallow, shallow history (1)
    //     rc(8) > pc(9) (parallel) > c1(10) c2(11)
    //
    // so the largest configuration is top, ra and a state of it, rb and a state
    // of it, rc, pc, c1 and c2 (9), a selection takes one transition from each
    // of the four atomic states a configuration holds, and two parents have a
    // history. The walks over the entry set run over a stack of three steps for
    // each of the four levels, one for each of the two histories and three of
    // headroom: 17. Each history records one state, one of its parent's
    // atomic children, so the records hold 2.
    let model = parse(
        include_str!("fixtures/no_std/parallel_history_probe.scxml"),
        "parallel_history_probe",
    );
    let state_row = |parent: Option<u16>,
                     kind: &str,
                     last: u16,
                     children: (u16, u16),
                     initial: Option<u16>| {
        let parent = match parent {
            Some(p) => format!("Some(StateId({p}))"),
            None => "None".to_string(),
        };
        let initial = match initial {
            Some(t) => format!("Some(TransitionId({t}))"),
            None => "None".to_string(),
        };
        format!(
            "StateRow {{ parent: {parent}, kind: StateKind::{kind}, last_descendant: StateId({last}), \
             children: Span {{ start: {}, len: {} }}, initial: {initial} }},",
            children.0, children.1
        )
    };
    let transition_row = |source: Option<u16>, target: u16| {
        let source = match source {
            Some(s) => format!("Some(StateId({s}))"),
            None => "None".to_string(),
        };
        format!(
            "TransitionRow {{ source: {source}, targets: Span {{ start: {target}, len: 1 }}, \
             internal: false, has_content: false }},"
        )
    };
    for no_std in [false, true] {
        let code = generate(&model, &template_dir(), no_std).expect("render");
        assert_eq!(
            enum_variants(&code, "ParallelHistoryProbeState"),
            [
                "Top = 0,", "Ra = 1,", "A1 = 2,", "A2 = 3,", "A3 = 4,", "Rb = 5,", "B1 = 6,",
                "B2 = 7,", "Rc = 8,", "Pc = 9,", "C1 = 10,", "C2 = 11,",
            ],
            "emit (no_std={no_std}): the states are declared in document order, numbered by it"
        );
        assert_eq!(
            enum_variants(&code, "ParallelHistoryProbeHistory"),
            ["HaDeep = 0,", "HbShallow = 1,"],
            "emit (no_std={no_std}): the histories are numbered by their ids' order"
        );
        // Each event is numbered by its place in the enum, the null event
        // last, and `EVENTS` lists them in that order.
        let events = enum_variants(&code, "ParallelHistoryProbeEvent");
        assert_eq!(
            events.last().map(String::as_str),
            Some(format!("Null = {},", events.len() - 1).as_str()),
            "emit (no_std={no_std}): the null event is numbered after every named event"
        );
        for (number, variant) in events.iter().enumerate() {
            assert!(
                variant.ends_with(&format!(" = {number},")),
                "emit (no_std={no_std}): `{variant}` is numbered by its place"
            );
        }
        assert!(
            code.contains("ParallelHistoryProbeEvent::Null,\n    ];"),
            "emit (no_std={no_std}): `EVENTS` ends with the null event"
        );
        assert_eq!(
            lines_starting(&code, "StateRow {"),
            [
                state_row(None, "Parallel", 11, (0, 3), None),
                state_row(Some(0), "Compound", 4, (3, 3), Some(14)),
                state_row(Some(1), "Atomic", 2, (6, 0), None),
                state_row(Some(1), "Atomic", 3, (6, 0), None),
                state_row(Some(1), "Atomic", 4, (6, 0), None),
                state_row(Some(0), "Compound", 7, (6, 2), Some(15)),
                state_row(Some(5), "Atomic", 6, (8, 0), None),
                state_row(Some(5), "Atomic", 7, (8, 0), None),
                state_row(Some(0), "Compound", 11, (8, 1), Some(16)),
                state_row(Some(8), "Parallel", 11, (9, 2), None),
                state_row(Some(9), "Atomic", 10, (11, 0), None),
                state_row(Some(9), "Atomic", 11, (11, 0), None),
            ],
            "emit (no_std={no_std}): the state rows"
        );
        assert_eq!(
            lines_starting(&code, "HistoryRow {"),
            [
                "HistoryRow { parent: StateId(1), deep: true, default: TransitionId(17), \
                 record: Span { start: 0, len: 1 } },",
                "HistoryRow { parent: StateId(5), deep: false, default: TransitionId(18), \
                 record: Span { start: 1, len: 1 } },",
            ],
            "emit (no_std={no_std}): the history rows"
        );
        // The written transitions by source (a1's four, a2's two, a3's, b1's,
        // b2's two, c1's two, c2's two), then the initial transitions of ra,
        // rb and rc, the defaults of ha_deep and hb_shallow, and the
        // document's: each one target, the n-th of the pool.
        let sources = [
            Some(2),
            Some(2),
            Some(2),
            Some(2),
            Some(3),
            Some(3),
            Some(4),
            Some(6),
            Some(7),
            Some(7),
            Some(10),
            Some(10),
            Some(11),
            Some(11),
            Some(1),
            Some(5),
            Some(8),
            Some(1),
            Some(5),
            None,
        ];
        assert_eq!(
            lines_starting(&code, "TransitionRow {"),
            sources
                .iter()
                .zip(0u16..)
                .map(|(source, target)| transition_row(*source, target))
                .collect::<Vec<_>>(),
            "emit (no_std={no_std}): the transition rows"
        );
        for expected in [
            "children: &[StateId(1), StateId(5), StateId(8), StateId(2), StateId(3), StateId(4), \
             StateId(6), StateId(7), StateId(9), StateId(10), StateId(11)],",
            "targets: &[Target::State(StateId(3)), Target::State(StateId(4)), \
             Target::State(StateId(3)), Target::State(StateId(4)), Target::State(StateId(2)), \
             Target::History(HistoryId(0)), Target::State(StateId(2)), Target::State(StateId(7)), \
             Target::State(StateId(6)), Target::History(HistoryId(1)), \
             Target::State(StateId(10)), Target::State(StateId(10)), Target::State(StateId(11)), \
             Target::State(StateId(11)), Target::State(StateId(2)), Target::State(StateId(6)), \
             Target::State(StateId(9)), Target::State(StateId(2)), Target::State(StateId(6)), \
             Target::State(StateId(0))],",
            "initial: TransitionId(19),",
            "history_records: 2,",
            "max_configuration: 9,",
            "max_enabled: 4,",
            "max_history_defaults: 2,",
            "max_walk: 17,",
            "pub const PARALLEL_HISTORY_PROBE_TABLES: ::sce_rust_runtime::DocumentTables = {",
            "const TABLES: &'static ::sce_rust_runtime::DocumentTables = \
             &PARALLEL_HISTORY_PROBE_TABLES;",
            "type Workspace = ::sce_rust_runtime::Workspace<\
             { ::sce_rust_runtime::state_scratch(9) }, 4, 17, 2>;",
            "type RunStore = ::sce_rust_runtime::RunStore<9, 2, 2>;",
            "pub type ParallelHistoryProbeConfiguration = \
             ::sce_rust_runtime::helpers::hierarchy::BoundedVec<ParallelHistoryProbeState, 9>;",
            "ParallelHistoryProbeHistory::HaDeep,\n        ParallelHistoryProbeHistory::HbShallow,\n    ];",
            "::sce_rust_runtime::StateId(state as u16)",
            "::sce_rust_runtime::HistoryId(history as u16)",
            "::sce_rust_runtime::EventId(event as u16)",
        ] {
            assert!(
                code.contains(expected),
                "emit (no_std={no_std}) must carry `{expected}`"
            );
        }
        for gone in [
            "MAX_CONFIGURATION",
            "MAX_ENABLED",
            "MAX_DEPTH",
            "type EnabledSet",
            "type HistoryDefaults",
            "type EntryWalk",
            "history_ha_deep",
            "history_hb_shallow",
        ] {
            assert!(
                !code.contains(gone),
                "emit (no_std={no_std}) must not carry `{gone}`: the runtime reads the tables"
            );
        }
    }

    // A flat machine holds one state and enables one transition, whatever the
    // size of a constant the runtime might once have chosen; it has no
    // history, so nothing to record and an uninhabited history type.
    let flat = generate(&parse(PLAIN_FSM, "plain_bounds"), &template_dir(), true)
        .expect("template render must succeed");
    assert_eq!(
        enum_variants(&flat, "PlainBoundsState"),
        ["S0 = 0,", "S1 = 1,"]
    );
    assert_eq!(
        lines_starting(&flat, "StateRow {"),
        [
            state_row(None, "Atomic", 0, (0, 0), None),
            state_row(None, "Atomic", 1, (0, 0), None),
        ]
    );
    assert_eq!(
        lines_starting(&flat, "TransitionRow {"),
        [transition_row(Some(0), 0), transition_row(None, 1)]
    );
    assert!(lines_starting(&flat, "HistoryRow {").is_empty());
    for expected in [
        "children: &[],",
        "targets: &[Target::State(StateId(1)), Target::State(StateId(0))],",
        "initial: TransitionId(1),",
        "history_records: 0,",
        "max_configuration: 1,",
        "max_enabled: 1,",
        "max_history_defaults: 0,",
        "max_walk: 6,",
        "type Workspace = ::sce_rust_runtime::Workspace<\
         { ::sce_rust_runtime::state_scratch(1) }, 1, 6, 0>;",
        "type RunStore = ::sce_rust_runtime::RunStore<1, 0, 0>;",
        "type History = sce_rust_runtime::NoHistory;",
        "const HISTORIES: &'static [Self::History] = &[];",
        "match history {}",
    ] {
        assert!(
            flat.contains(expected),
            "a flat machine must carry `{expected}`"
        );
    }
}

/// The `_MAX_SELECTED` a C11 machine is sized by.
fn c11_max_selected(model: &sce_build::model::SCXMLModel, stem: &str) -> String {
    let out = sce_build::generator::generate_c11(
        model,
        &sce_build::find_template_dir_for(sce_build::generator::Language::C11),
        stem,
        None,
    )
    .expect("C11 render");
    let source = out
        .files
        .iter()
        .find(|(name, _)| name.ends_with("_sm.c"))
        .map(|(_, code)| code.as_str())
        .expect("a C11 source file");
    source
        .lines()
        .find_map(|line| {
            let rest = line.strip_prefix("#define ")?;
            let (name, value) = rest.split_once(' ')?;
            name.ends_with("_MAX_SELECTED").then(|| value.to_string())
        })
        .expect("a `_MAX_SELECTED` definition")
}

#[test]
fn c11_template_sizes_a_selection_by_the_bound_the_rust_policy_carries() {
    // One bound, one function: the C11 selection, filter and entry buffers hold
    // `MicrostepBounds::enabled`, the number the Rust policy writes as
    // `MAX_ENABLED`. The document's atomic state count is not that bound, since a
    // configuration holds one path through the tree, so a compound state of many
    // leaves enables one transition and not one per leaf.
    let probe = parse(
        include_str!("fixtures/no_std/parallel_history_probe.scxml"),
        "parallel_history_probe",
    );
    assert_eq!(
        c11_max_selected(&probe, "parallel_history_probe"),
        "4u",
        "four atomic states are active at once in the probe's largest configuration"
    );

    let wide = parse(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0">
  <state id="s0"><transition event="go" target="s1"/></state>
  <state id="s1"><transition event="go" target="s2"/></state>
  <state id="s2"><transition event="go" target="s3"/></state>
  <state id="s3"><transition event="go" target="s4"/></state>
  <state id="s4"/>
</scxml>"#,
        "wide_flat",
    );
    assert_eq!(
        c11_max_selected(&wide, "wide_flat"),
        "1u",
        "five sibling states are never active together"
    );
}

/// A sce-static document that asks `In()` in a `<send>` param, inside a
/// `<foreach>` (an `<assign>` and an `<if>` of its body) and in an `<invoke>`
/// param handed to its child.
const IN_EVERYWHERE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="running" datamodel="sce-static" name="in_everywhere">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="hits" sce:type="uint32" expr="0" sce:direction="out"/>
    <data id="seen" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="running">
    <invoke type="scxml" id="child">
      <param name="flag" expr="In('running')"/>
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
               version="1.0" initial="waiting" datamodel="sce-static">
          <datamodel>
            <data id="flag" sce:type="bool" expr="false"/>
          </datamodel>
          <state id="waiting">
            <transition cond="flag" target="leaf"/>
          </state>
          <final id="leaf"/>
        </scxml>
      </content>
    </invoke>
    <transition event="walk" type="internal">
      <foreach array="picked" item="v">
        <assign location="seen" expr="In('running')"/>
        <if cond="In('running')">
          <assign location="hits" expr="hits + v"/>
        </if>
      </foreach>
      <send event="report">
        <param name="busy" expr="In('running')"/>
      </send>
    </transition>
    <transition event="report" type="internal">
      <assign location="hits" expr="hits + 1"/>
    </transition>
    <transition event="done.invoke.child" type="internal"/>
  </state>
</scxml>
"#;

/// Every file the Rust backend writes for the document at `path`, joined.
fn rust_output(path: &Path) -> Result<String, String> {
    let output = sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &template_dir(),
        sce_build::generator::Language::Rust,
    )
    .map_err(|refusal| refusal.to_string())?;
    Ok(output
        .files
        .iter()
        .map(|(_, code)| code.as_str())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[test]
fn rust_asks_the_engine_for_every_in_the_static_lowering_writes() {
    // docs/adr/0064: the configuration is the engine's, so a generated machine
    // has no `is_state_active` of its own. The sce-static lowering spells an
    // In() as a call of it wherever an expression stands, and every template
    // site that writes lowered text turns it into the engine's test of the
    // state's variant (`rust_in_predicate`); one site that did not would leave
    // a call of a method that does not exist.
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("in_everywhere.scxml");
    std::fs::write(&path, IN_EVERYWHERE).expect("write the document");
    let code = rust_output(&path).expect("the document generates");
    assert!(
        !code.contains("is_state_active("),
        "every In() is asked of the engine:\n{code}"
    );
    // The send's param, the foreach's assignment and condition, and the
    // invoke's param: four tests of the one state.
    assert!(
        code.matches("engine.is_active(InEverywhereState::Running)")
            .count()
            >= 4,
        "each In() names the state's variant:\n{code}"
    );

    // And every document of the fixtures that asks In() and generates for
    // Rust.
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut pending = vec![fixtures];
    let mut generated: Vec<String> = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read a fixture directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("scxml") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("read a fixture");
            if !text.contains("In(") {
                continue;
            }
            // A fixture written for another backend, or to be refused, is
            // not a Rust machine.
            let Ok(code) = rust_output(&path) else {
                continue;
            };
            assert!(
                !code.contains("is_state_active("),
                "{}: every In() is asked of the engine",
                path.display()
            );
            generated.push(path.display().to_string());
        }
    }
    assert!(
        generated
            .iter()
            .any(|p| p.ends_with("static_datamodel/static_counter.scxml")),
        "the sweep reached the sce-static counter, whose guard asks In(): {generated:?}"
    );
}

#[test]
fn rust_template_builds_a_bare_metal_machine_in_a_constant_expression() {
    // A `static` holds the machine and its engine, so nothing runs at start-up
    // and no copy is made on a stack (`Engine::new_const`). The generated policy
    // supplies what that needs, and only where every field starts at a constant:
    // the bare-metal profile, with no typed payload and no datamodel variable.
    let bare = generate(&parse(PLAIN_FSM, "plain_const"), &template_dir(), true)
        .expect("template render must succeed");
    for expected in [
        "pub const fn new_const(session_id: &'static str) -> Self {",
        "const fn build(session_id: &'static str) -> Self {",
        "impl sce_rust_runtime::ConstStatePolicy for PlainConstPolicy {",
        "const EMPTY_EVENT_QUEUE: Self::EventQueue = sce_rust_runtime::EventQueueManager::new();",
        "<Self as sce_rust_runtime::ConstStatePolicy>::INITIAL_STATE",
        // The tables are a constant the policy refers to (docs/adr/0064): a
        // `static` could not be, on the toolchain the runtime supports.
        "pub const PLAIN_CONST_TABLES: ::sce_rust_runtime::DocumentTables = {",
        "const TABLES: &'static ::sce_rust_runtime::DocumentTables = &PLAIN_CONST_TABLES;",
    ] {
        assert!(
            bare.contains(expected),
            "a bare-metal machine must carry `{expected}`"
        );
    }
    assert!(
        !bare.contains("static PLAIN_CONST_TABLES"),
        "a constant cannot refer to a static before Rust 1.83"
    );

    // Under std the engine holds a hash set and a registry no constant
    // expression can build, so the machine has no constant constructor either.
    let hosted = generate(&parse(PLAIN_FSM, "plain_const"), &template_dir(), false)
        .expect("template render must succeed");
    for absent in ["new_const", "ConstStatePolicy", "const fn build"] {
        assert!(
            !hosted.contains(absent),
            "a std machine must not carry `{absent}`"
        );
    }
}

#[test]
fn rust_template_builds_no_string_for_a_send_id_or_an_origin() {
    // A `<send>` names an id and stamps an origin, and the bare-metal profile keeps
    // neither on the event it raises. An owned string for each (`SceString`, a
    // buffer of the string bound) made at the send site and cloned into metadata
    // that drops it sat in the frame of the action, several times over where an
    // entry block holds several sends; the id is text, borrowed, and the engine
    // builds the one string it stores. The MCU probe's worst-case stack and
    // `.text` hold the same fact from the object's side.
    let mut model = parse(
        include_str!("fixtures/no_std/parallel_history_probe.scxml"),
        "parallel_history_probe",
    );
    // Analysed, as the command line does before it renders: whether the document
    // cancels is what decides whether the scheduler keeps its sends' ids.
    sce_build::analyzer::analyze(&mut model, "parallel_history_probe");
    let code = generate(&model, &template_dir(), true).expect("render");
    assert!(
        code.contains("let send_id: &str = \"__send_0\";"),
        "a generated send id is text"
    );
    // A delayed send hands its id over and gets none back: the scheduler's other
    // entry point returns an owned string, a buffer of the string bound that the
    // send site would drop, on the stack of the deepest call chain.
    assert!(
        code.contains("engine.schedule_event_as("),
        "a delayed send names its id and takes none back"
    );
    assert!(
        !code.contains("engine.schedule_event("),
        "a delayed send must not take an owned id back"
    );
    // The probe cancels, so the scheduler keeps its sends' ids, and keeps them in
    // the document's own size: its longest is `d_delayed`, nine bytes, where a
    // buffer of the string bound would be 256 for each slot of the scheduler.
    assert!(
        code.contains("type ScheduledSendId = ::sce_rust_runtime::SendIdString<9>;"),
        "the cancel key is the document's longest send id"
    );
    assert!(
        !code.contains("type ScheduledSendId = sce_rust_runtime::SceString;"),
        "the cancel key must not be a buffer of the string bound"
    );
    for owned in [
        "let send_id = ::sce_rust_runtime::sce_string_from_str(",
        "EventMetadata::external(send_id.clone()",
        "&::sce_rust_runtime::SceString::new()",
    ] {
        assert!(
            !code.contains(owned),
            "a bare-metal emission must not build `{owned}` for a send"
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

/// The predicate the runtime compiles `helpers::session_id::drawn` under: the
/// targets with a source of randomness it reads.
const DRAWS: &str =
    r#"#[cfg(not(any(target_os = "none", all(target_arch = "wasm32", target_os = "unknown"))))]"#;

/// The line above the first line of `code` that starts (after indentation)
/// with `item`, trimmed.
fn line_above(code: &str, item: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim_start().starts_with(item))
        .unwrap_or_else(|| panic!("`{item}` is emitted:\n{code}"));
    assert!(at > 0, "`{item}` is the first line");
    lines[at - 1].trim().to_string()
}

#[test]
fn a_no_std_machine_is_built_only_under_a_session_id_its_host_gives() {
    // docs/adr/0045, "Questions the owner answered" 2: a `no_std` target has no
    // source of randomness, so its host gives the session id as a required
    // argument of the one constructor there is, held to the Nmtoken rule there
    // with nothing allocated, and held by the policy as the text it was given.
    let model = parse(PLAIN_FSM, "plain_nostd_session_id");
    let code = generate(&model, &template_dir(), true).expect("template render must succeed");
    assert!(
        code.contains("pub fn new_with_session_id(session_id: &'static str"),
        "the no_std constructor takes the id:\n{code}"
    );
    assert!(
        code.contains("::sce_rust_runtime::helpers::session_id::checked(session_id)?"),
        "the id is held to the rule where the machine is built:\n{code}"
    );
    assert!(
        code.contains("pub session_id: &'static str,"),
        "the policy holds the text it was given:\n{code}"
    );
    for absent in [
        "pub fn new(",
        "impl Default for",
        "helpers::session_id::drawn",
    ] {
        assert!(
            !code.contains(absent),
            "a no_std machine has no way to be built without an id, so `{absent}` \
             is not emitted:\n{code}"
        );
    }
}

#[test]
fn a_std_machine_draws_its_session_id_only_where_the_target_can() {
    // On a std target the host may give the id or have one drawn; `new`, which
    // draws, and the `Default` that calls it are compiled only where the runtime
    // has a source of randomness, so on `wasm32-unknown-unknown` the machine is
    // built through `new_with_session_id` alone.
    let model = parse(PLAIN_FSM, "plain_std_session_id");
    let code = generate(&model, &template_dir(), false).expect("template render must succeed");
    assert!(
        code.contains("pub fn new_with_session_id(session_id: &str"),
        "a std host may give the id:\n{code}"
    );
    assert!(
        code.contains("::sce_rust_runtime::helpers::session_id::given(session_id)?"),
        "the id given is held to the rule where the machine is built:\n{code}"
    );
    assert_eq!(line_above(&code, "pub fn new("), DRAWS);
    if code.contains("impl Default for") {
        assert_eq!(line_above(&code, "impl Default for"), DRAWS);
    }
    assert!(
        !code.contains("pub fn with_session_id("),
        "an id is given where the machine is built, not bound afterwards:\n{code}"
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
