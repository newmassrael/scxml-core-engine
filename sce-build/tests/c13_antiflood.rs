//! Anti-flood + stateless_accept parse-time validators.
//!
//! Per SCE Protocol-Synthesis RFC §synth-5-K lines 2272-2349 + 2449-2473:
//! `trust_class: session_arming` listener links require the
//! anti-flood quota + per-source token-bucket rate-limit; links
//! with `domain_attrs.untrusted_source: true` additionally require
//! the HMAC stateless_accept block. Five spec-named diagnostics
//! pin the conditional requirement, dead-config rejection, opt-out
//! requirement, and key-rotation invariant.
//!
//! Two additional spec codes defer per
//! `[[feedback-silently-broken-hooks]]`:
//!   - `deploy/session-arming-quota-vs-peer-table-invariant-violated`
//!     (peer_table.capacity + max_handshake_time_s schema fields not
//!     yet declared in spec).
//!   - `deploy/stateless-accept-extern-not-whitelisted` (cross-doc
//!     resolution against baseline + target_plugin symbols).

use sce_build::mesh::deploy::parse_deploy_str;
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

// ── #1 deploy/session-arming-quota-missing ─────────────────────

#[test]
fn session_arming_quota_missing_fires() {
    // session_arming trust_class without session_arming_quota.
    // accept_rate_* present so the *-missing-config diagnostic doesn't
    // pre-empt; the spec walk order fires session-arming-quota-missing
    // first when only the quota is absent.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            accept_rate_per_sec: 4
            accept_rate_burst: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("quota absent under session_arming fires");
    let DeployError::SessionArmingQuotaMissing { machine, link_name } = err else {
        panic!("expected SessionArmingQuotaMissing, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
}

#[test]
fn session_arming_quota_with_other_required_fields_parses() {
    // Complete session_arming declaration parses cleanly.
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
    parse_deploy_str(&yaml).expect("complete session_arming declaration parses");
}

// ── #2 deploy/accept-rate-config-missing ───────────────────────

#[test]
fn accept_rate_config_missing_fires_when_one_of_two_absent() {
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 8
            accept_rate_per_sec: 4
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("missing burst fires");
    let DeployError::AcceptRateConfigMissing {
        machine,
        link_name,
        missing_fields,
    } = err
    else {
        panic!("expected AcceptRateConfigMissing, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
    assert!(missing_fields.contains("accept_rate_burst"));
}

#[test]
fn accept_rate_config_missing_fires_when_both_absent() {
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("both missing fires");
    let DeployError::AcceptRateConfigMissing { missing_fields, .. } = err else {
        panic!("expected AcceptRateConfigMissing, got {err:?}");
    };
    assert!(missing_fields.contains("accept_rate_per_sec"));
    assert!(missing_fields.contains("accept_rate_burst"));
}

// ── #3 deploy/session-arming-fields-on-non-arming-link ─────────

#[test]
fn anti_flood_fields_on_established_session_link_fire() {
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            mtu_bytes: 1500
            domain_attrs:
              trust_class: established_session
            session_arming_quota: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("anti-flood on established_session fires");
    let DeployError::SessionArmingFieldsOnNonArmingLink {
        machine,
        link_name,
        trust_class,
        offending_fields,
    } = err
    else {
        panic!("expected SessionArmingFieldsOnNonArmingLink, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
    assert_eq!(trust_class, "established_session");
    assert!(offending_fields.contains("session_arming_quota"));
}

#[test]
fn stateless_accept_on_untrusted_link_fires_dead_config() {
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: untrusted
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 30000
              key_rotation_s: 3600
              hmac_extern: sce_hmac_sha256
              rng_extern: sce_random_fill
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("stateless_accept on untrusted fires");
    let DeployError::SessionArmingFieldsOnNonArmingLink {
        trust_class,
        offending_fields,
        ..
    } = err
    else {
        panic!("expected SessionArmingFieldsOnNonArmingLink, got {err:?}");
    };
    assert_eq!(trust_class, "untrusted");
    assert!(offending_fields.contains("stateless_accept"));
}

#[test]
fn anti_flood_fields_on_link_without_domain_attrs_fire_dead_config() {
    // domain_attrs absent ⇒ no Accepting.* path ⇒ dead config (the
    // walk-order #3 diagnostic). "trust_class" wire field = "<absent>".
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            session_arming_quota: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("anti-flood on no-domain-attrs fires");
    let DeployError::SessionArmingFieldsOnNonArmingLink { trust_class, .. } = err else {
        panic!("expected SessionArmingFieldsOnNonArmingLink, got {err:?}");
    };
    assert_eq!(trust_class, "<absent>");
}

// ── #4 deploy/stateless-accept-required-on-untrusted-source ────

#[test]
fn stateless_accept_required_on_untrusted_source_fires() {
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
              untrusted_source: true
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("untrusted_source without stateless_accept fires");
    let DeployError::StatelessAcceptRequiredOnUntrustedSource { machine, link_name } = err else {
        panic!("expected StatelessAcceptRequiredOnUntrustedSource, got {err:?}");
    };
    assert_eq!(machine, "mcu_node");
    assert_eq!(link_name, "udp_listener");
}

#[test]
fn stateless_accept_with_untrusted_source_parses() {
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
              cookie_lifetime_ms: 30000
              key_rotation_s: 3600
              hmac_extern: sce_hmac_sha256
              rng_extern: sce_random_fill
"#,
    );
    parse_deploy_str(&yaml).expect("complete untrusted_source declaration parses");
}

// ── #5 deploy/stateless-accept-key-rotation-shorter-than-lifetime ──

#[test]
fn key_rotation_shorter_than_lifetime_fires() {
    // key_rotation_s × 1000 = 30000; 2 × cookie_lifetime_ms = 60000;
    // 30000 ≤ 60000 ⇒ fires.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 30000
              key_rotation_s: 30
              hmac_extern: sce_hmac_sha256
              rng_extern: sce_random_fill
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("key_rotation ≤ 2 × lifetime fires");
    let DeployError::StatelessAcceptKeyRotationShorterThanLifetime {
        key_rotation_s,
        cookie_lifetime_ms,
        rotation_ms,
        lifetime_doubled,
        ..
    } = err
    else {
        panic!("expected StatelessAcceptKeyRotationShorterThanLifetime, got {err:?}");
    };
    assert_eq!(key_rotation_s, 30);
    assert_eq!(cookie_lifetime_ms, 30_000);
    assert_eq!(rotation_ms, 30_000);
    assert_eq!(lifetime_doubled, 60_000);
}

#[test]
fn key_rotation_strictly_greater_passes() {
    // key_rotation_s × 1000 = 70_000; 2 × cookie_lifetime_ms = 60_000;
    // 70_000 > 60_000 ⇒ passes.
    let yaml = deploy_with_link(
        r#"            bind: "0.0.0.0:7447"
            driver: lwip_udp
            domain_attrs:
              trust_class: session_arming
            session_arming_quota: 8
            accept_rate_per_sec: 4
            accept_rate_burst: 8
            stateless_accept:
              mode: cookie_hmac_sha256
              cookie_lifetime_ms: 30000
              key_rotation_s: 70
              hmac_extern: sce_hmac_sha256
              rng_extern: sce_random_fill
"#,
    );
    parse_deploy_str(&yaml).expect("strictly-greater key_rotation parses");
}

// ── Closed-enum drift guard for HmacMode ────────────────────────

#[test]
fn unknown_hmac_mode_rejected_at_parse_time() {
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
              mode: blake2s
              cookie_lifetime_ms: 30000
              key_rotation_s: 3600
              hmac_extern: sce_hmac_blake2s
              rng_extern: sce_random_fill
"#,
    );
    let err = parse_deploy_str(&yaml).expect_err("unknown HMAC mode rejects");
    // Serde rejects with the generic "unknown variant" error wrapped
    // as DeployError::Yaml. The wire surface is the typed
    // diagnostic family that contains "blake2s" or "unknown" text.
    let s = format!("{err:?}");
    assert!(
        s.contains("blake2s") || s.contains("unknown") || s.contains("variant"),
        "expected unknown-variant rejection, got {s:?}"
    );
}
