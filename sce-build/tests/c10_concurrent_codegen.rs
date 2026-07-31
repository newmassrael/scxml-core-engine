//! Multi-link concurrency contract + 3 codes (260 → 263).
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-N lines 3031-3062: cooperative scheduler
//! slot ceiling, per-link budget sanity, FSM event-queue sizing, plus
//! per-machine `LinkBus` + scheduler artifact emit.
//!
//! Validators tested here:
//! - `link/concurrent-count-exceeds-scheduler-slots` (MCU-only)
//! - `link/per-link-budget-exceeds-tick-period`
//! - `link/inbound-event-queue-unsized` (cross-doc orchestrator)
//!
//! Codegen tested here:
//! - `render_machine_concurrency_artifacts` for Rust (LinkBus +
//!   scheduler) and C11 (scheduler only).
//! - silent-skip arms (deploy absent, budget absent, language without
//!   link templates).

use sce_build::forge::generator::render_machine_concurrency_artifacts;
use sce_build::generator::Language;
use sce_build::mesh::deploy::parse_deploy_str;
use sce_build::mesh::error::DeployError;

fn template_root() -> std::path::PathBuf {
    // Mirrors `sce_build::find_template_base()` — locate the
    // workspace `tools/codegen/templates` root once per test by
    // climbing parents from the crate manifest until the directory
    // appears. Keeps the test resilient to in-tree vs out-of-tree
    // build layouts.
    let mut cursor: std::path::PathBuf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        let candidate = cursor.join("tools/codegen/templates");
        if candidate.is_dir() {
            return candidate;
        }
        if !cursor.pop() {
            panic!("template root not found from CARGO_MANIFEST_DIR ancestry");
        }
    }
}

// ── #1 link/concurrent-count-exceeds-scheduler-slots ──────────

#[test]
fn concurrent_count_fires_when_links_exceed_slot_count() {
    // tick_period_us=1000, per_link_budget_us=500 → slot_count = 2.
    // 4 links > 2 slots ⇒ fires (MCU-only).
    let yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
          per_link_budget_us: 500
        memory:
          sram_regions:
            sram1: { base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }
          dma_channels: [DW0_CH0]
        links:
          udp_a: { bind: "0.0.0.0:1", driver: lwip_udp }
          udp_b: { bind: "0.0.0.0:2", driver: lwip_udp }
          udp_c: { bind: "0.0.0.0:3", driver: lwip_udp }
          udp_d: { bind: "0.0.0.0:4", driver: lwip_udp }
"#;
    let err = parse_deploy_str(yaml).expect_err("4 links > 2 slots fires");
    let DeployError::LinkConcurrentCountExceedsSchedulerSlots {
        machine,
        link_count,
        slot_count,
        tick_period_us,
        per_link_budget_us,
    } = err
    else {
        panic!("expected LinkConcurrentCountExceedsSchedulerSlots, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_count, 4);
    assert_eq!(slot_count, 2);
    assert_eq!(tick_period_us, 1000);
    assert_eq!(per_link_budget_us, 500);
}

#[test]
fn concurrent_count_silent_skip_on_non_mcu() {
    // Same overrun arithmetic but `class: ap` ⇒ silent-skip
    // (AP path uses tokio::spawn per link, no slot accounting).
    let yaml = r#"
version: "1.0"
topology:
  ap_device:
    machines:
      ap_node:
        source: ap_node.scxml
        platform:
          class: ap
          os: linux
          has_dcache: true
          dcache_line_size: 64
          has_speculative_prefetch: true
          core_count: 4
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
          per_link_budget_us: 500
        memory:
          sram_regions:
            sram1: { base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }
          dma_channels: [DW0_CH0]
        links:
          udp_a: { bind: "0.0.0.0:1", driver: lwip_udp }
          udp_b: { bind: "0.0.0.0:2", driver: lwip_udp }
          udp_c: { bind: "0.0.0.0:3", driver: lwip_udp }
          udp_d: { bind: "0.0.0.0:4", driver: lwip_udp }
"#;
    parse_deploy_str(yaml).expect("AP class silent-skips slot check");
}

#[test]
fn concurrent_count_silent_skip_on_per_link_budget_absent() {
    // No per_link_budget_us ⇒ no slot derivation possible ⇒ silent-skip.
    let yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
        memory:
          sram_regions:
            sram1: { base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }
          dma_channels: [DW0_CH0]
        links:
          udp_a: { bind: "0.0.0.0:1", driver: lwip_udp }
          udp_b: { bind: "0.0.0.0:2", driver: lwip_udp }
          udp_c: { bind: "0.0.0.0:3", driver: lwip_udp }
          udp_d: { bind: "0.0.0.0:4", driver: lwip_udp }
"#;
    parse_deploy_str(yaml).expect("per_link_budget_us absent silent-skips");
}

// ── #2 link/per-link-budget-exceeds-tick-period ────────────────

#[test]
fn per_link_budget_exceeds_tick_period_fires() {
    // per_link_budget_us=2000 > tick_period_us=1000 ⇒ fires.
    let yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
          per_link_budget_us: 2000
        memory:
          sram_regions:
            sram1: { base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }
          dma_channels: [DW0_CH0]
        links:
          udp_a: { bind: "0.0.0.0:1", driver: lwip_udp }
"#;
    let err = parse_deploy_str(yaml).expect_err("budget > tick fires");
    let DeployError::LinkPerLinkBudgetExceedsTickPeriod {
        machine,
        per_link_budget_us,
        tick_period_us,
    } = err
    else {
        panic!("expected LinkPerLinkBudgetExceedsTickPeriod, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(per_link_budget_us, 2000);
    assert_eq!(tick_period_us, 1000);
}

#[test]
fn per_link_budget_happy_when_fits_within_tick() {
    let yaml = r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: mcu_node.scxml
        platform:
          class: mcu
          os: bare_metal
          has_dcache: true
          dcache_line_size: 32
          has_speculative_prefetch: false
          core_count: 1
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
          per_link_budget_us: 500
        memory:
          sram_regions:
            sram1: { base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }
          dma_channels: [DW0_CH0]
        links:
          udp_a: { bind: "0.0.0.0:1", driver: lwip_udp }
"#;
    parse_deploy_str(yaml).expect("500 ≤ 1000 ⇒ passes");
}

// ── #3 link/inbound-event-queue-unsized ────────────────────────
//
// The cross-doc validator joins forge link models with deploy +
// SCXML. Exercising it end-to-end requires the full
// compile_scxml_with_imports orchestrator path with fixture forge
// docs + SCXML imports + deploy.yaml — out of scope for this
// micro-test suite. The diagnostic shape is pinned by the byte-stable
// golden + the variant is exercised via direct construction below.

#[test]
fn inbound_event_queue_unsized_variant_shape() {
    use sce_build::forge::error::ValidationError;
    let err = ValidationError::LinkInboundEventQueueUnsized {
        machine: "mcu_node".into(),
        link_name: "udp_data".into(),
        inbound_event_count: 3,
    };
    let rendered = err.to_string();
    assert!(
        rendered.contains("3 inbound event(s)"),
        "message must surface event count: {rendered}"
    );
    assert!(
        rendered.contains("SCE Protocol-Synthesis RFC §5.N line 3062"),
        "spec anchor must be quoted: {rendered}"
    );
    assert!(
        rendered.contains("sce:capacity") && rendered.contains("default_event_queue_capacity"),
        "two-axis repair must be named: {rendered}"
    );
}

// ── Codegen emit: AP LinkBus + MCU scheduler ──────────────────

#[test]
fn render_rust_emits_link_bus_and_scheduler_when_budgets_set() {
    let files = render_machine_concurrency_artifacts(
        &template_root(),
        Language::Rust,
        "mcu_node",
        &["udp_scout".to_string(), "udp_data".to_string()],
        Some(1000),
        Some(500),
    )
    .expect("render succeeds");
    assert_eq!(files.len(), 2, "expects link_bus.rs + scheduler.rs");
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"mcu_node_link_bus.rs"));
    assert!(names.contains(&"mcu_node_scheduler.rs"));
    let (_, bus) = files
        .iter()
        .find(|(n, _)| n == "mcu_node_link_bus.rs")
        .unwrap();
    assert!(
        bus.contains("McuNodeLinkBus"),
        "LinkBus struct uses pascal-case machine name"
    );
    assert!(
        bus.contains("UdpScout(Vec<u8>)") && bus.contains("UdpData(Vec<u8>)"),
        "LinkBusEvent variants emit per link in pascal-case"
    );
    let (_, sched) = files
        .iter()
        .find(|(n, _)| n == "mcu_node_scheduler.rs")
        .unwrap();
    assert!(sched.contains("TICK_PERIOD_US: u32 = 1000"));
    assert!(sched.contains("PER_LINK_BUDGET_US: u32 = 500"));
    assert!(sched.contains(r#""udp_scout","#));
    assert!(sched.contains(r#""udp_data","#));
    // Runtime poll(deadline_us) extension closure: scheduler must
    // import sce-link-runtime's Link trait directly and call its
    // poll method — no per-machine LinkDriver trait redundancy.
    assert!(
        sched.contains("use sce_link_runtime::Link;"),
        "scheduler must import the runtime Link trait directly"
    );
    assert!(
        !sched.contains("pub trait LinkDriver"),
        "no per-machine LinkDriver trait — gap closed by Link::poll"
    );
    assert!(
        sched.contains(".poll(PER_LINK_BUDGET_US);"),
        "scheduler must call Link::poll with the per-link budget"
    );
}

#[test]
fn render_rust_emits_link_bus_only_when_budget_absent() {
    let files = render_machine_concurrency_artifacts(
        &template_root(),
        Language::Rust,
        "ap_node",
        &["udp_data".to_string()],
        None,
        None,
    )
    .expect("render succeeds");
    assert_eq!(files.len(), 1, "AP-only mode emits LinkBus, no scheduler");
    assert_eq!(files[0].0, "ap_node_link_bus.rs");
}

#[test]
fn render_c11_emits_scheduler_header() {
    let files = render_machine_concurrency_artifacts(
        &template_root(),
        Language::C11,
        "mcu_node",
        &["udp_scout".to_string(), "udp_data".to_string()],
        Some(1000),
        Some(500),
    )
    .expect("render succeeds");
    assert_eq!(files.len(), 1, "C11 emits scheduler.h only");
    assert_eq!(files[0].0, "mcu_node_scheduler.h");
    let body = &files[0].1;
    assert!(body.contains("SCE_FORGE_MCU_NODE_TICK_PERIOD_US"));
    assert!(body.contains("SCE_FORGE_MCU_NODE_PER_LINK_BUDGET_US"));
    assert!(body.contains("mcu_node_scheduler_tick"));
    assert!(body.contains(r#""udp_scout""#));
    assert!(body.contains(r#""udp_data""#));
    // Runtime poll(deadline_us) extension closure: C11 scheduler
    // must call ops->poll directly with the budget — no rx+dispatch
    // callback workaround machinery.
    assert!(
        body.contains("drv->ops->poll(drv->self, SCE_FORGE_MCU_NODE_PER_LINK_BUDGET_US)"),
        "C11 scheduler must call ops->poll with the per-link budget"
    );
    assert!(
        !body.contains("scheduler_dispatch_fn"),
        "no dispatch callback typedef — gap closed by ops->poll"
    );
}

#[test]
fn render_c11_silent_skips_without_budgets() {
    let files = render_machine_concurrency_artifacts(
        &template_root(),
        Language::C11,
        "mcu_node",
        &["udp_data".to_string()],
        None,
        None,
    )
    .expect("render succeeds");
    assert!(
        files.is_empty(),
        "C11 has no LinkBus and silent-skips scheduler without budgets"
    );
}

#[test]
fn render_other_backends_silent_skip() {
    for lang in [
        Language::Cpp,
        Language::Kotlin,
        Language::Go,
        Language::Python,
    ] {
        let files = render_machine_concurrency_artifacts(
            &template_root(),
            lang,
            "mcu_node",
            &["udp_data".to_string()],
            Some(1000),
            Some(500),
        )
        .expect("non-Rust/C11 silent-skip succeeds");
        assert!(
            files.is_empty(),
            "{:?} has no link template footprint (rust+c11 only)",
            lang
        );
    }
}

#[test]
fn render_empty_link_list_silent_skips() {
    let files = render_machine_concurrency_artifacts(
        &template_root(),
        Language::Rust,
        "empty_machine",
        &[],
        Some(1000),
        Some(500),
    )
    .expect("empty links silent-skip");
    assert!(files.is_empty(), "no links ⇒ no LinkBus, no scheduler");
}
