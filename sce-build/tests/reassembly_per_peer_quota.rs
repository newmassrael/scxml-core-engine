// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Reassembly per-peer-quota cross-doc validator — watching-zenoh RFC
// §5.M lines 2841-2861 invariant
// `peer_table.capacity × per_peer_quota >= slot_count`.
//
// Sibling declared-vs-consumed invariants are enforced by existing
// typed cross-doc validators (CodecPeekByteFlagLayoutMismatch,
// CodecVariantArmMidMismatch + CodecVariantArmInnerMidUndeclared,
// LinkNotDeclaredInDeploy + LinkNotDeclaredInForge). This file pins
// the full surface of the reassembly-quota validator
// (`reassembly/per-peer-quota-build-invariant-violated`).
//
// Three contracts:
//
//   1. Happy path: peer_table.capacity × per_peer_quota >= slot_count
//      silent-passes the validator.
//
//   2. Violation: peer_table.capacity × per_peer_quota < slot_count
//      fires `reassembly/per-peer-quota-build-invariant-violated` with
//      every input echoed in the diagnostic payload so authors repair
//      on the appropriate axis.
//
//   3. Silent-skip discipline when peer_table is absent.
//      The validator must not noise-up legacy deploys that haven't
//      declared the session-arming hardening block; only authors who
//      opted into peer_table.capacity get the invariant gate.

use std::collections::HashMap;

use sce_build::forge::error::ValidationError;
use sce_build::forge::model::{
    BackpressurePolicy, BufferPoolModel, BufferPoolVariant, CachePolicy, LinkClass, LinkModel,
    ReassemblyConfig,
};
use sce_build::mesh::deploy::{parse_deploy_str, validate_reassembly_cross_doc};

fn deploy_with_links(links_yaml: &str) -> String {
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

fn link_model_with_rx_pool(name: &str, rx_pool: &str) -> LinkModel {
    LinkModel {
        name: name.to_string(),
        class: LinkClass::Udp,
        framer: "test_framer".to_string(),
        backpressure: BackpressurePolicy::Drop,
        inbound: vec![],
        outbound: vec![],
        rx_pool: Some(rx_pool.to_string()),
        tx_pool: None,
        stage_pool: None,
        accept_stage_copy_rate: false,
        source_location: None,
    }
}

fn reassembly_pool(name: &str, slot_count: u32, per_peer_quota: u32) -> BufferPoolModel {
    BufferPoolModel {
        name: name.to_string(),
        slot_count,
        // slot_size dialed large enough to satisfy the pre-existing #2
        // check `reassembly/max-fragments-insufficient-for-mtu`
        // (`slot_size >= max_fragments × mtu_bytes` = 8 × 1500 =
        // 12000). 16384 ≥ 12000 keeps the test exclusively on the
        // per-peer-quota axis.
        slot_size: 16384,
        section: "sram1".to_string(),
        alignment: 32,
        dma_channel: None,
        cache_policy: CachePolicy::None,
        variant: BufferPoolVariant::Reassembly(ReassemblyConfig {
            max_fragments_per_message: 8,
            reassembly_timeout_ms: 100,
            per_peer_quota,
        }),
        source_location: None,
    }
}

/// Listener-pair deploy YAML with explicit peer_table.capacity.
/// `peer_table_capacity` is the only knob that varies between happy /
/// negative tests — every other field is constant. The
/// session_arming_quota / max_handshake_time_s pair is dialed low
/// (1 × 2 = 2) so the pre-existing
/// `SessionArmingQuotaVsPeerTableInvariantViolated` upstream check
/// (`session_arming_quota × max_handshake_time_s ≤ peer_table.capacity`)
/// stays satisfied at every peer_table.capacity ≥ 2 — letting this
/// test focus exclusively on the new
/// `ReassemblyPerPeerQuotaBuildInvariantViolated` axis.
fn listener_link_yaml(peer_table_capacity: u32) -> String {
    format!(
        r#"          udp_listener:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1500
            role: listener
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 1
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: vendor_hmac_sha256
              rng_extern: vendor_csprng
              max_handshake_time_s: 2
              peer_table:
                capacity: {peer_table_capacity}
"#
    )
}

/// Listener link without `stateless_accept`/`peer_table` — exercises
/// the silent-skip discipline.
fn listener_link_yaml_no_peer_table() -> String {
    r#"          udp_listener:
            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1500
            role: listener
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 1
            accept_rate_per_sec: 4
            accept_rate_burst: 8
"#
    .to_string()
}

// ── Contract 1: happy path silent-passes ──────────────────────────

#[test]
fn happy_path_capacity_times_quota_at_least_slot_count_passes() {
    // peer_table.capacity (4) × per_peer_quota (4) = 16 >= slot_count (16)
    let yaml = deploy_with_links(&listener_link_yaml(4));
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");

    let link = link_model_with_rx_pool("udp_listener", "rx_reassembly_pool");
    let pool = reassembly_pool("rx_reassembly_pool", 16, 4);
    let mut forge_links: HashMap<String, &LinkModel> = HashMap::new();
    forge_links.insert("udp_listener".to_string(), &link);
    let mut pool_registry: HashMap<String, &BufferPoolModel> = HashMap::new();
    pool_registry.insert("rx_reassembly_pool".to_string(), &pool);

    // Listener resolved (matches the deploy `role: listener`).
    let mut listener_links = std::collections::BTreeSet::new();
    listener_links.insert("udp_listener".to_string());

    validate_reassembly_cross_doc(&cfg, &forge_links, &pool_registry, &listener_links)
        .expect("peer_table.capacity × per_peer_quota >= slot_count must pass");
}

// ── Contract 2: violation fires the typed diagnostic ──────────────

#[test]
fn violation_capacity_times_quota_below_slot_count_fires() {
    // peer_table.capacity (2) × per_peer_quota (4) = 8 < slot_count (16)
    let yaml = deploy_with_links(&listener_link_yaml(2));
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");

    let link = link_model_with_rx_pool("udp_listener", "rx_reassembly_pool");
    let pool = reassembly_pool("rx_reassembly_pool", 16, 4);
    let mut forge_links: HashMap<String, &LinkModel> = HashMap::new();
    forge_links.insert("udp_listener".to_string(), &link);
    let mut pool_registry: HashMap<String, &BufferPoolModel> = HashMap::new();
    pool_registry.insert("rx_reassembly_pool".to_string(), &pool);

    let mut listener_links = std::collections::BTreeSet::new();
    listener_links.insert("udp_listener".to_string());

    let err = validate_reassembly_cross_doc(&cfg, &forge_links, &pool_registry, &listener_links)
        .expect_err("invariant violation must reject");
    match *err {
        ValidationError::ReassemblyPerPeerQuotaBuildInvariantViolated {
            pool_name,
            slot_count,
            machine,
            link_name,
            peer_table_capacity,
            per_peer_quota,
            product,
        } => {
            assert_eq!(pool_name, "rx_reassembly_pool");
            assert_eq!(slot_count, 16);
            assert_eq!(machine, "mcu_node");
            assert_eq!(link_name, "udp_listener");
            assert_eq!(peer_table_capacity, 2);
            assert_eq!(per_peer_quota, 4);
            assert_eq!(
                product, 8,
                "product must echo `capacity × quota` verbatim so authors don't recompute"
            );
        }
        other => panic!(
            "expected ReassemblyPerPeerQuotaBuildInvariantViolated, got: {:?}",
            other
        ),
    }
}

#[test]
fn boundary_at_equality_passes() {
    // peer_table.capacity (4) × per_peer_quota (4) = 16 == slot_count (16)
    // — exactly meets the bound, must silent-pass (>= is the spec form).
    let yaml = deploy_with_links(&listener_link_yaml(4));
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");

    let link = link_model_with_rx_pool("udp_listener", "rx_reassembly_pool");
    let pool = reassembly_pool("rx_reassembly_pool", 16, 4);
    let mut forge_links: HashMap<String, &LinkModel> = HashMap::new();
    forge_links.insert("udp_listener".to_string(), &link);
    let mut pool_registry: HashMap<String, &BufferPoolModel> = HashMap::new();
    pool_registry.insert("rx_reassembly_pool".to_string(), &pool);

    let mut listener_links = std::collections::BTreeSet::new();
    listener_links.insert("udp_listener".to_string());

    validate_reassembly_cross_doc(&cfg, &forge_links, &pool_registry, &listener_links)
        .expect("equality boundary 16 == 16 must pass (spec uses >=)");
}

// ── Contract 3: silent-skip when peer_table is absent ─────────────

#[test]
fn silent_skip_when_peer_table_absent() {
    // No stateless_accept block on the link — the per-peer-quota
    // invariant has no source for peer_table.capacity. Per the
    // Absent-input silent-skip discipline (mirror of every other reassembly
    // validator), the check silent-skips. Other reassembly checks
    // still run; we verify by giving a clearly violating quota/slot
    // combination and assert no error fires (because the silent-skip
    // applies).
    let yaml = deploy_with_links(&listener_link_yaml_no_peer_table());
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");

    let link = link_model_with_rx_pool("udp_listener", "rx_reassembly_pool");
    // slot_count (16) > capacity (would-be) × quota (4) — but no
    // peer_table to read, so the invariant silent-skips.
    let pool = reassembly_pool("rx_reassembly_pool", 16, 4);
    let mut forge_links: HashMap<String, &LinkModel> = HashMap::new();
    forge_links.insert("udp_listener".to_string(), &link);
    let mut pool_registry: HashMap<String, &BufferPoolModel> = HashMap::new();
    pool_registry.insert("rx_reassembly_pool".to_string(), &pool);

    let mut listener_links = std::collections::BTreeSet::new();
    listener_links.insert("udp_listener".to_string());

    validate_reassembly_cross_doc(&cfg, &forge_links, &pool_registry, &listener_links)
        .expect("peer_table absent ⇒ deploy-unaware silent-skip");
}
