//! C13 deferred-2 — peer_table invariant + stateless_accept extern
//! allowlist validators.
//!
//! Per watching-zenoh RFC §5.K lines 2460-2462 + 2466-2469, this
//! atomic closes the two C13-β-deferred spec codes:
//!
//!   1. `deploy/session-arming-quota-vs-peer-table-invariant-violated`
//!      (line 2460-2462) — `session_arming_quota ×
//!      max_handshake_time_s ≤ peer_table.capacity`. Fires when a
//!      slow legitimate handshake can be evicted under attack. The
//!      validator slots into `validate_links` after the C13-β
//!      anti-flood checks; silent-skip when any of the three inputs
//!      is absent per Q-η5 (a) discipline.
//!
//!   2. `deploy/stateless-accept-extern-not-whitelisted` (line
//!      2466-2469) — `hmac_extern` / `rng_extern` symbol must
//!      resolve against the §5.I baseline whitelist OR a loaded
//!      `target_plugin` entry. Lives at the orchestrator level
//!      where target-plugin loading converges with the baseline.

use sce_build::forge::intrinsic_registry::Abi;
use sce_build::forge::target_plugin::PluginSymbol;
use sce_build::mesh::deploy::{parse_deploy_str, validate_stateless_accept_externs};
use sce_build::mesh::error::DeployError;

fn deploy_with_link(link_body: &str) -> String {
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
        scheduler:
          kind: cooperative
          tick_period_us: 1000
          worker_stack_budget: 4096
          worker_slot_budget_us: 200
          keepalive_jitter_budget_us: 5000
        links:
          udp_listener:
{link_body}
"#,
    )
}

// ── #1 deploy/session-arming-quota-vs-peer-table-invariant-violated ──

#[test]
fn peer_table_invariant_happy_path_parses() {
    // session_arming_quota=8, max_handshake_time_s=2, capacity=32
    // ⇒ 8 × 2 = 16 ≤ 32 satisfies the invariant.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: vendor_hmac_sha256
              rng_extern: vendor_csprng
              peer_table:
                capacity: 32
              max_handshake_time_s: 2
"#,
    );
    parse_deploy_str(&yaml).expect("invariant satisfied (16 <= 32) parses");
}

#[test]
fn peer_table_invariant_violation_fires() {
    // session_arming_quota=8, max_handshake_time_s=4, capacity=16
    // ⇒ 8 × 4 = 32 > 16 violates the invariant.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: vendor_hmac_sha256
              rng_extern: vendor_csprng
              peer_table:
                capacity: 16
              max_handshake_time_s: 4
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("invariant violated fires");
    let DeployError::SessionArmingQuotaVsPeerTableInvariantViolated {
        machine,
        link_name,
        session_arming_quota,
        max_handshake_time_s,
        peer_table_capacity,
        product,
    } = err
    else {
        panic!("expected SessionArmingQuotaVsPeerTableInvariantViolated, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
    assert_eq!(session_arming_quota, 8);
    assert_eq!(max_handshake_time_s, 4);
    assert_eq!(peer_table_capacity, 16);
    assert_eq!(product, 32);
}

#[test]
fn peer_table_invariant_silent_skip_when_block_absent() {
    // stateless_accept block omitted entirely on a session_arming link
    // that does NOT carry untrusted_source — the validator silent-skips
    // because there's no stateless_accept to read peer_table /
    // max_handshake_time_s from. The other anti-flood fields are still
    // required so the link parses cleanly.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
"#,
    );
    parse_deploy_str(&yaml).expect("no stateless_accept block ⇒ silent-skip parses");
}

#[test]
fn peer_table_invariant_silent_skip_when_max_handshake_time_absent() {
    // stateless_accept declared but max_handshake_time_s omitted — the
    // invariant has no LHS to compute, so silent-skip per Q-η5 (a).
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: vendor_hmac_sha256
              rng_extern: vendor_csprng
              peer_table:
                capacity: 8
"#,
    );
    parse_deploy_str(&yaml).expect("max_handshake_time_s absent ⇒ silent-skip");
}

#[test]
fn peer_table_invariant_silent_skip_when_peer_table_absent() {
    // stateless_accept declared, max_handshake_time_s declared, but
    // peer_table sub-block absent. The invariant cannot compute the
    // RHS bound — silent-skip per Q-η5 (a).
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: vendor_hmac_sha256
              rng_extern: vendor_csprng
              max_handshake_time_s: 2
"#,
    );
    parse_deploy_str(&yaml).expect("peer_table absent ⇒ silent-skip");
}

// ── #2 deploy/stateless-accept-extern-not-whitelisted ─────────────

fn extern_yaml(hmac: &str, rng: &str) -> String {
    deploy_with_link(&format!(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 5000
              key_rotation_s: 3600
              hmac_extern: {hmac}
              rng_extern: {rng}
              peer_table:
                capacity: 32
              max_handshake_time_s: 2
"#,
    ))
}

#[test]
fn extern_in_baseline_accepts_under_empty_plugin_set() {
    // §5.I baseline carries `sce_atomic_load_acquire_u32` etc. — those
    // are atomics, not hmac/csprng, but the allowlist check is name-
    // based: any baseline name resolves. Use an arbitrary baseline
    // entry to prove the baseline path is consulted.
    let cfg = parse_deploy_str(&extern_yaml(
        "sce_atomic_load_acquire_u32",
        "sce_atomic_load_relaxed_u32",
    ))
    .expect("invariant holds + parse");
    validate_stateless_accept_externs(&cfg, &[])
        .expect("baseline-resolved externs pass allowlist with empty plugin set");
}

#[test]
fn extern_in_target_plugin_accepts() {
    // Vendor symbols not in baseline; plugin_symbols carries them.
    let cfg = parse_deploy_str(&extern_yaml("vendor_hmac_sha256", "vendor_csprng"))
        .expect("invariant holds + parse");
    let plugin_symbols = vec![
        PluginSymbol {
            name: "vendor_hmac_sha256".into(),
            sig: "(*const u8, usize, *const u8, usize, *mut u8) -> ()".into(),
            abi: Abi::C,
            purpose: Some("hmac".into()),
            crate_name: None,
        },
        PluginSymbol {
            name: "vendor_csprng".into(),
            sig: "(*mut u8, usize) -> ()".into(),
            abi: Abi::C,
            purpose: Some("rng".into()),
            crate_name: None,
        },
    ];
    validate_stateless_accept_externs(&cfg, &plugin_symbols)
        .expect("plugin-loaded externs pass allowlist");
}

#[test]
fn extern_in_neither_fires_with_sorted_union_candidates() {
    // hmac_extern not in baseline and not in plugin set ⇒ fires.
    let cfg = parse_deploy_str(&extern_yaml("typo_hmac", "vendor_csprng"))
        .expect("invariant holds + parse");
    let plugin_symbols = vec![PluginSymbol {
        name: "vendor_csprng".into(),
        sig: "(*mut u8, usize) -> ()".into(),
        abi: Abi::C,
        purpose: Some("rng".into()),
        crate_name: None,
    }];
    let err = validate_stateless_accept_externs(&cfg, &plugin_symbols)
        .expect_err("typo'd hmac_extern fires");
    let DeployError::StatelessAcceptExternNotWhitelisted {
        machine,
        link_name,
        extern_name,
        role,
        candidates,
    } = err
    else {
        panic!("expected StatelessAcceptExternNotWhitelisted, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
    assert_eq!(extern_name, "typo_hmac");
    assert_eq!(role, "hmac");
    // The candidate list is the sorted union of §5.I baseline + the
    // single plugin entry — confirm the plugin entry shows up and the
    // list is sorted.
    assert!(
        candidates.contains(&"vendor_csprng".to_string()),
        "candidates missing plugin entry: {candidates:?}"
    );
    let mut sorted = candidates.clone();
    sorted.sort();
    assert_eq!(
        candidates, sorted,
        "candidates must be sorted for byte-stable wire output"
    );
    // The baseline has 101 atomics + cache + fences + IRQ entries;
    // adding one plugin entry yields a candidate list strictly larger
    // than the baseline alone (the dedup is on names, not on origin).
    assert!(
        candidates.len() > 100,
        "expected baseline ∪ plugin candidates > 100; got {}",
        candidates.len()
    );
}
