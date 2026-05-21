//! C13-γ — pool_defaults.stage_copy_policy promotion + opt-out
//! rejection.
//!
//! Per watching-zenoh RFC §5.K lines 2350-2369 + 2504-2519: machine-
//! wide `pool_defaults.stage_copy_policy` (warn | error | forbid)
//! drives the C13-α-2 `reassembly/expected-fragmentation-rate-high`
//! warning's promotion to hard error (`error` / `forbid`) plus the
//! `<sce:accept-stage-copy-rate>` opt-out semantics:
//!   - warn (default): warning fires unless opt-out present.
//!   - error: warning promoted to `pool/stage-copy-policy-error`;
//!     opt-out still suppresses.
//!   - forbid: same promotion AND opt-out itself rejected via
//!     `pool/stage-copy-accept-rejected-under-forbid`.
//!
//! Parse-time typo guard: unknown policy value fires
//! `deploy/stage-copy-policy-unknown` with FixCarriesCandidates over
//! the closed set.

use std::collections::HashMap;

use sce_build::forge::error::ValidationError;
use sce_build::forge::model::{
    BackpressurePolicy, BufferPoolModel, BufferPoolVariant, CachePolicy, LinkClass, LinkModel,
};
use sce_build::mesh::deploy::{parse_deploy_str, validate_reassembly_cross_doc, StageCopyPolicy};
use sce_build::mesh::error::DeployError;

fn link_model(name: &str, rx_pool: Option<&str>, accept_opt_out: bool) -> LinkModel {
    LinkModel {
        name: name.to_string(),
        class: LinkClass::Udp,
        framer: "test_framer".to_string(),
        backpressure: BackpressurePolicy::Drop,
        inbound: vec![],
        outbound: vec![],
        rx_pool: rx_pool.map(str::to_string),
        tx_pool: None,
        stage_pool: None,
        accept_stage_copy_rate: accept_opt_out,
        source_location: None,
    }
}

fn default_pool(name: &str, slot_count: u32, slot_size: u32) -> BufferPoolModel {
    BufferPoolModel {
        name: name.to_string(),
        slot_count,
        slot_size,
        section: "sram1".to_string(),
        alignment: 32,
        dma_channel: None,
        cache_policy: CachePolicy::None,
        variant: BufferPoolVariant::Default,
        source_location: None,
    }
}

/// Deploy fixture with `pool_defaults.stage_copy_policy: <policy_value>`
/// plugged in. The link `udp_data` declares `expected_p99_bytes: 1024`
/// with no `mtu_bytes` and no `domain_attrs` (the latter two would
/// trigger C13-α-1 parse-time checks that prevent reaching the
/// promotion site). Pool's `slot_size = 700` makes the rate
/// `(1024-700)/1024 × 100 = 31% > 25% threshold`. #1 (slot < mtu)
/// silent-skips since mtu_bytes is absent; #4/#5 silent-skip since
/// the bound pool is Default-variant (not reassembly).
fn deploy_with_policy(policy_value: &str) -> String {
    format!(
        r#"
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
          clock_freq_mhz: 400
          memcpy_cycles_per_byte: 1.0
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
        memory:
          sram_regions:
            sram1: {{ base: 0x08000000, size: 65536, attr: [dma_coherent, cacheable] }}
          dma_channels: [DW0_CH0]
        pool_defaults:
          stage_copy_policy: {policy_value}
        links:
          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            expected_p99_bytes: 1024
"#,
    )
}

fn deploy_without_pool_defaults() -> String {
    r#"
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
          clock_freq_mhz: 400
          memcpy_cycles_per_byte: 1.0
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
          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            expected_p99_bytes: 1024
"#
    .to_string()
}

fn fixture_link_and_pool() -> (HashMap<String, LinkModel>, HashMap<String, BufferPoolModel>) {
    let mut link_map = HashMap::new();
    link_map.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), false),
    );
    let mut pool_map = HashMap::new();
    pool_map.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    (link_map, pool_map)
}

fn views<'a>(
    links: &'a HashMap<String, LinkModel>,
    pools: &'a HashMap<String, BufferPoolModel>,
) -> (
    HashMap<String, &'a LinkModel>,
    HashMap<String, &'a BufferPoolModel>,
) {
    let link_view: HashMap<String, &LinkModel> =
        links.iter().map(|(k, v)| (k.clone(), v)).collect();
    let pool_view: HashMap<String, &BufferPoolModel> =
        pools.iter().map(|(k, v)| (k.clone(), v)).collect();
    (link_view, pool_view)
}

// ── Parse-time typo guard: deploy/stage-copy-policy-unknown ────

#[test]
fn unknown_policy_value_fires_at_parse_time() {
    let yaml = deploy_with_policy("errr");
    let err = parse_deploy_str(&yaml).expect_err("typo must be rejected at parse time");
    let DeployError::StageCopyPolicyUnknown {
        machine,
        value,
        candidates,
        ..
    } = err
    else {
        panic!("expected StageCopyPolicyUnknown, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(value, "errr");
    assert_eq!(
        candidates,
        vec![
            "warn".to_string(),
            "error".to_string(),
            "forbid".to_string()
        ],
    );
}

#[test]
fn warn_policy_parses_and_resolves() {
    let yaml = deploy_with_policy("warn");
    let cfg = parse_deploy_str(&yaml).expect("warn parses");
    let machine = cfg
        .topology
        .get("mcu_device")
        .unwrap()
        .machines
        .get("mcu_node")
        .unwrap();
    assert_eq!(machine.resolved_stage_copy_policy(), StageCopyPolicy::Warn);
}

#[test]
fn missing_pool_defaults_resolves_to_warn() {
    let yaml = deploy_without_pool_defaults();
    let cfg = parse_deploy_str(&yaml).expect("absent pool_defaults parses");
    let machine = cfg
        .topology
        .get("mcu_device")
        .unwrap()
        .machines
        .get("mcu_node")
        .unwrap();
    assert_eq!(machine.resolved_stage_copy_policy(), StageCopyPolicy::Warn);
}

// ── Warn (default): #3 fires unless opt-out ────────────────────

#[test]
fn warn_policy_fires_expected_fragmentation_rate_high() {
    let yaml = deploy_with_policy("warn");
    let cfg = parse_deploy_str(&yaml).expect("warn parses");
    let (links, pools) = fixture_link_and_pool();
    let (link_view, pool_view) = views(&links, &pools);
    let err = validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect_err("rate > 25 fires");
    match *err {
        ValidationError::ReassemblyExpectedFragmentationRateHigh { rate_percent, .. } => {
            assert_eq!(rate_percent, 31);
        }
        other => panic!("expected ExpectedFragmentationRateHigh, got {other:?}"),
    }
}

#[test]
fn warn_policy_opt_out_suppresses_warning() {
    let yaml = deploy_with_policy("warn");
    let cfg = parse_deploy_str(&yaml).expect("warn parses");
    let mut links = HashMap::new();
    links.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), true), // opt-out = true
    );
    let mut pools = HashMap::new();
    pools.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    let (link_view, pool_view) = views(&links, &pools);
    validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect("opt-out under warn ⇒ silent-skip");
}

// ── Error policy: promotion ─────────────────────────────────────

#[test]
fn error_policy_promotes_to_pool_stage_copy_policy_error() {
    let yaml = deploy_with_policy("error");
    let cfg = parse_deploy_str(&yaml).expect("error parses");
    let (links, pools) = fixture_link_and_pool();
    let (link_view, pool_view) = views(&links, &pools);
    let err = validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect_err("rate > 25 + error policy fires");
    match *err {
        ValidationError::PoolStageCopyPolicyError {
            rate_percent,
            policy,
            ..
        } => {
            assert_eq!(rate_percent, 31);
            assert_eq!(policy, "error");
        }
        other => panic!("expected PoolStageCopyPolicyError, got {other:?}"),
    }
}

#[test]
fn error_policy_opt_out_suppresses_promotion() {
    let yaml = deploy_with_policy("error");
    let cfg = parse_deploy_str(&yaml).expect("error parses");
    let mut links = HashMap::new();
    links.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), true), // opt-out = true
    );
    let mut pools = HashMap::new();
    pools.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    let (link_view, pool_view) = views(&links, &pools);
    validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect("opt-out under error ⇒ silent-skip per spec line 2358-2361");
}

// ── Forbid policy: opt-out rejection + promotion ────────────────

#[test]
fn forbid_policy_promotes_to_pool_stage_copy_policy_error_without_opt_out() {
    let yaml = deploy_with_policy("forbid");
    let cfg = parse_deploy_str(&yaml).expect("forbid parses");
    let (links, pools) = fixture_link_and_pool();
    let (link_view, pool_view) = views(&links, &pools);
    let err = validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect_err("rate > 25 + forbid policy + no opt-out fires");
    match *err {
        ValidationError::PoolStageCopyPolicyError { policy, .. } => {
            assert_eq!(policy, "forbid");
        }
        other => panic!("expected PoolStageCopyPolicyError, got {other:?}"),
    }
}

#[test]
fn forbid_policy_with_opt_out_rejects_outright() {
    let yaml = deploy_with_policy("forbid");
    let cfg = parse_deploy_str(&yaml).expect("forbid parses");
    let mut links = HashMap::new();
    links.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), true), // opt-out = true
    );
    let mut pools = HashMap::new();
    pools.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    let (link_view, pool_view) = views(&links, &pools);
    let err = validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect_err("forbid rejects opt-out outright");
    match *err {
        ValidationError::PoolStageCopyAcceptRejectedUnderForbid { machine, link_name } => {
            assert_eq!(machine, "mcu_node");
            assert_eq!(link_name, "udp_data");
        }
        other => panic!("expected PoolStageCopyAcceptRejectedUnderForbid, got {other:?}"),
    }
}

#[test]
fn warn_policy_with_opt_out_does_not_reject() {
    // Opt-out under `warn` only suppresses the warning; it does not
    // fire the forbid-only rejection. Drift guard.
    let yaml = deploy_with_policy("warn");
    let cfg = parse_deploy_str(&yaml).expect("warn parses");
    let mut links = HashMap::new();
    links.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), true),
    );
    let mut pools = HashMap::new();
    pools.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    let (link_view, pool_view) = views(&links, &pools);
    validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect("warn + opt-out ⇒ silent-skip (no rejection)");
}

#[test]
fn error_policy_with_opt_out_does_not_reject() {
    // Opt-out under `error` only suppresses the promoted error; it
    // does not fire the forbid-only rejection. Drift guard.
    let yaml = deploy_with_policy("error");
    let cfg = parse_deploy_str(&yaml).expect("error parses");
    let mut links = HashMap::new();
    links.insert(
        "udp_data".to_string(),
        link_model("udp_data", Some("rx_data_pool"), true),
    );
    let mut pools = HashMap::new();
    pools.insert(
        "rx_data_pool".to_string(),
        default_pool("rx_data_pool", 16, 700),
    );
    let (link_view, pool_view) = views(&links, &pools);
    validate_reassembly_cross_doc(
        &cfg,
        &link_view,
        &pool_view,
        &std::collections::BTreeSet::new(),
    )
    .expect("error + opt-out ⇒ silent-skip (no rejection)");
}

// ── Closed-enum drift guard for StageCopyPolicy::ALL ────────────

#[test]
fn stage_copy_policy_all_matches_enum_variants() {
    // If anyone adds a new variant to StageCopyPolicy without
    // extending ALL or from_str, this test trips at compile time
    // (exhaustive match) and at runtime (count mismatch).
    let expected: Vec<&str> = StageCopyPolicy::ALL.to_vec();
    assert_eq!(expected, vec!["warn", "error", "forbid"]);
    assert!(StageCopyPolicy::from_wire_str("warn").is_some());
    assert!(StageCopyPolicy::from_wire_str("error").is_some());
    assert!(StageCopyPolicy::from_wire_str("forbid").is_some());
    assert!(StageCopyPolicy::from_wire_str("unknown").is_none());
}
