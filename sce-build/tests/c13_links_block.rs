//! C13-α-1 — deploy.yaml §5.K `machines.<n>.links.<name>` block
//! schema + parse-time validators + cross-doc link-name resolution.
//!
//! Per watching-zenoh RFC §5.K lines 2232-2540: per-machine `links:`
//! HashMap with required `bind` + `driver` and optional `mtu_bytes`,
//! `expected_p99_bytes`, `burst_pps`, `rx_dispatch`, `domain_attrs`.
//! Five intra-link parse-time validators (driver-unknown, mtu-below-
//! driver-floor, expected-p99-exceeds-mtu, burst-pps-missing-on-isr-
//! dispatch, mtu-missing-on-fragmenting-link) + two cross-doc
//! validators (link-not-declared-in-deploy + link-not-declared-in-forge)
//! per Q-C13-5 (a) lock.
//!
//! Two spec codes (`deploy/link-burst-absorption-insufficient` line
//! 2489-2495 + `deploy/link-rx-dispatch-worker-tick-on-high-burst`
//! line 2496-2500) defer to C13-α-2 — both require RX pool slot_count
//! cross-doc resolution against forge `<sce:link>` + `ForgePoolRegistry`,
//! infrastructure that lands in the follow-up atomic. Six C9-β
//! reassembly cross-doc codes (mem/reassembly-slot-size-below-declared-
//! mtu + 5 reassembly/*) also defer to C13-α-2 per Q-C9-2 (a) lock.
//!
//! PlatformConfig WCET extensions (`clock_freq_mhz`,
//! `memcpy_cycles_per_byte`, `vle_decode_cycles_per_byte`,
//! `tlv_chain_per_entry_overhead_us`) per Q-C13-6 (a) — parse-only in
//! C13-α-1; §5.B aggregate WCET + C9-β stage-copy-wcet consumers fire
//! when the corresponding consumer-side atomic lands.

use sce_build::mesh::deploy::{
    parse_deploy_str, validate_links_cross_doc, RxDispatch, TrustClass,
};
use sce_build::mesh::error::DeployError;

/// Standard MCU deploy.yaml prelude used across most tests below.
/// Carries one machine with platform + scheduler + memory already
/// landed; only the `links:` block changes per-test.
fn deploy_prelude_with_links(links_yaml: &str) -> String {
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
        links:
{links_yaml}
"#,
    )
}

/// Happy path: minimal links block — `bind` + `driver` only. Verifies
/// the LinkConfig defaults: `mtu_bytes` / `expected_p99_bytes` /
/// `burst_pps` / `rx_dispatch` / `domain_attrs` all None.
#[test]
fn minimal_links_block_parses_with_defaults() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("minimal links block parses");
    let machine = cfg
        .topology
        .get("mcu_device")
        .and_then(|d| d.machines.get("mcu_node"))
        .expect("mcu_node");
    assert_eq!(machine.links.len(), 1);
    let link = machine.links.get("udp_scout").expect("udp_scout entry");
    assert_eq!(link.bind, "224.0.0.224:7446");
    assert_eq!(link.driver, "lwip_udp");
    assert_eq!(link.mtu_bytes, None);
    assert_eq!(link.expected_p99_bytes, None);
    assert_eq!(link.burst_pps, None);
    assert_eq!(link.rx_dispatch, None);
    assert!(link.domain_attrs.is_none());
    // Q-C13-3 (a) conditional default: WorkerTick when burst_pps absent.
    assert_eq!(link.resolved_rx_dispatch(), RxDispatch::WorkerTick);
}

/// Q-C13-3 (a) conditional default: `IsrToPool` when `burst_pps`
/// declared (and `rx_dispatch` not explicitly set).
#[test]
fn rx_dispatch_default_isr_when_burst_pps_declared() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1472
            burst_pps: 200
            domain_attrs:
              trust_class: established_session
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("burst_pps + no rx_dispatch parses");
    let link = cfg
        .topology["mcu_device"]
        .machines["mcu_node"]
        .links["udp_data"]
        .clone();
    assert_eq!(link.resolved_rx_dispatch(), RxDispatch::IsrToPool);
}

/// Full schema: every C13-α-1 link field present + WCET-extension
/// platform fields populated.
#[test]
fn full_links_schema_parses() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1472
            expected_p99_bytes: 1024
            burst_pps: 200
            rx_dispatch: isr_to_pool
            domain_attrs:
              trust_class: established_session
              untrusted_source: false
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("full schema parses");
    let machine = &cfg.topology["mcu_device"].machines["mcu_node"];
    let link = &machine.links["udp_data"];
    assert_eq!(link.mtu_bytes, Some(1472));
    assert_eq!(link.expected_p99_bytes, Some(1024));
    assert_eq!(link.burst_pps, Some(200));
    assert_eq!(link.rx_dispatch, Some(RxDispatch::IsrToPool));
    let domain = link.domain_attrs.as_ref().expect("domain_attrs");
    assert_eq!(domain.trust_class, TrustClass::EstablishedSession);
    assert!(!domain.untrusted_source);
    let platform = machine.platform.as_ref().expect("platform");
    assert_eq!(platform.clock_freq_mhz, Some(400));
    assert_eq!(platform.memcpy_cycles_per_byte, Some(1.0));
}

/// Negative: unknown driver → `deploy/link-driver-unknown` with closed
/// candidate baseline.
#[test]
fn link_driver_unknown_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: foo_udp
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("unknown driver rejects");
    let DeployError::LinkDriverUnknown {
        machine,
        link_name,
        driver,
        candidates,
        ..
    } = err
    else {
        panic!("expected LinkDriverUnknown, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
    assert_eq!(driver, "foo_udp");
    // Sorted baseline known-driver set.
    // Sorted KNOWN_DRIVERS baseline: lwip_tcp / lwip_udp / serial_uart (spec C11 added serial).
    assert_eq!(
        candidates,
        vec![
            "lwip_tcp".to_string(),
            "lwip_udp".to_string(),
            "serial_uart".to_string(),
        ]
    );
}

/// Negative: `mtu_bytes` below driver floor (`lwip_udp` floor = 28).
#[test]
fn link_mtu_below_driver_floor_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 20
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("mtu below floor rejects");
    let DeployError::LinkMtuBelowDriverFloor {
        machine,
        link_name,
        driver,
        declared_mtu,
        driver_floor,
    } = err
    else {
        panic!("expected LinkMtuBelowDriverFloor, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
    assert_eq!(driver, "lwip_udp");
    assert_eq!(declared_mtu, 20);
    assert_eq!(driver_floor, 28);
}

/// Negative: `expected_p99_bytes > mtu_bytes`.
#[test]
fn link_expected_p99_exceeds_mtu_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1472
            expected_p99_bytes: 2048
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("p99 > mtu rejects");
    let DeployError::LinkExpectedP99ExceedsMtu {
        machine,
        link_name,
        expected_p99_bytes,
        mtu_bytes,
    } = err
    else {
        panic!("expected LinkExpectedP99ExceedsMtu, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
    assert_eq!(expected_p99_bytes, 2048);
    assert_eq!(mtu_bytes, 1472);
}

/// Negative: `rx_dispatch: isr_to_pool` explicit + `burst_pps` absent.
#[test]
fn link_burst_pps_missing_on_isr_dispatch_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            rx_dispatch: isr_to_pool
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("isr without burst_pps rejects");
    let DeployError::LinkBurstPpsMissingOnIsrDispatch { machine, link_name } = err else {
        panic!("expected LinkBurstPpsMissingOnIsrDispatch, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
}

/// Negative: `trust_class: established_session` (Fragment-carrying)
/// + `mtu_bytes` absent. Under-approximation per
/// `MeshDeployLinkMtuMissingOnFragmentingLink` doc-comment — uses
/// trust-class as proxy for "Fragment-FSM-bound link" until C13-α-2's
/// precise reassembly-pool cross-doc step lands.
#[test]
fn link_mtu_missing_on_fragmenting_link_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: established_session
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("established_session w/o mtu rejects");
    let DeployError::LinkMtuMissingOnFragmentingLink { machine, link_name } = err else {
        panic!("expected LinkMtuMissingOnFragmentingLink, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
}

/// Q-C13-2 (a) lock evolution: C13-β landed the anti-flood fields, so
/// the prior "deferred field rejects" pin no longer holds. The test
/// is repointed to assert that the deny_unknown_fields gate still
/// rejects an actually-unknown field name (`bogus_future_field`) so
/// the schema-stability contract stays exercised. C13-β's own
/// integration tests (`c13_beta_antiflood.rs`) cover positive +
/// negative semantics for the new fields.
#[test]
fn unknown_link_field_rejects_under_deny_unknown_fields() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            bogus_future_field: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("unknown field rejects");
    // The DeployError::Parse variant carries the serde rejection.
    let s = format!("{err:?}");
    assert!(
        s.contains("bogus_future_field") || s.contains("unknown field"),
        "expected unknown-field rejection naming bogus_future_field, got {s:?}"
    );
}

/// Cross-doc Q-C13-5 (a): forge `<sce:link name="X">` exists but no
/// deploy `machines.<n>.links.X` entry. Validator
/// [`validate_links_cross_doc`] is exposed for orchestrator wiring.
#[test]
fn link_not_declared_in_deploy_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses with one link");
    // Forge registry knows about `udp_data` but deploy only has `udp_scout`.
    let forge_names = vec!["udp_data".to_string(), "udp_scout".to_string()];
    let err = validate_links_cross_doc(&cfg, &forge_names)
        .expect_err("forge has udp_data but deploy doesn't");
    let DeployError::LinkNotDeclaredInDeploy {
        link_name,
        candidates,
        ..
    } = err
    else {
        panic!("expected LinkNotDeclaredInDeploy, got {err:?}");
    };
    assert_eq!(link_name, "udp_data");
    assert_eq!(candidates, vec!["udp_scout".to_string()]);
}

/// Cross-doc Q-C13-5 (a): deploy declares `udp_data` but no forge link
/// doc by that name exists.
#[test]
fn link_not_declared_in_forge_fires() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses with one link");
    // Forge registry declares nothing — Pass A (forge → deploy) is
    // silent, Pass B (deploy → forge) fires for `udp_data`.
    let forge_names: Vec<String> = vec![];
    let err = validate_links_cross_doc(&cfg, &forge_names)
        .expect_err("deploy has udp_data but forge doesn't");
    let DeployError::LinkNotDeclaredInForge {
        machine,
        link_name,
        candidates,
        ..
    } = err
    else {
        panic!("expected LinkNotDeclaredInForge, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_data");
    // Pass B candidate set = forge link-doc names (empty for this fixture).
    assert!(candidates.is_empty());
}

/// Cross-doc happy path: forge + deploy declare matching link names.
/// Validator returns Ok.
#[test]
fn link_cross_doc_happy_when_names_match() {
    let yaml = deploy_prelude_with_links(
        r#"          udp_data:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
          udp_scout:
            bind: "224.0.0.224:7446"
            driver: lwip_udp
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let forge_names = vec!["udp_data".to_string(), "udp_scout".to_string()];
    validate_links_cross_doc(&cfg, &forge_names).expect("names match → Ok");
}

/// PlatformConfig WCET extensions: all 4 new fields parse + carry
/// through unchanged. Per Q-C13-6 (a) lock — consumers (§5.B aggregate
/// WCET + C9-β stage-copy-WCET) attach in separate atomics.
#[test]
fn platform_wcet_extensions_parse() {
    let yaml = format!(
        r#"
version: "1.0"
topology:
  ap_device:
    machines:
      ap_node:
        source: ap_node.scxml
        platform:
          class: mcu
          os: bare_metal
          clock_freq_mhz: 168
          memcpy_cycles_per_byte: 2.0
          vle_decode_cycles_per_byte: 8.0
          tlv_chain_per_entry_overhead_us: 0.8
"#,
    );
    let cfg = parse_deploy_str(&yaml).expect("WCET extensions parse");
    let platform = cfg
        .topology
        .get("ap_device")
        .and_then(|d| d.machines.get("ap_node"))
        .and_then(|m| m.platform.as_ref())
        .expect("platform");
    assert_eq!(platform.clock_freq_mhz, Some(168));
    assert_eq!(platform.memcpy_cycles_per_byte, Some(2.0));
    assert_eq!(platform.vle_decode_cycles_per_byte, Some(8.0));
    assert_eq!(platform.tlv_chain_per_entry_overhead_us, Some(0.8));
}
