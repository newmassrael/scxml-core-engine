// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Axis-3 inversion (RFC `claudedocs/rfc-axis3-listener-role-declarations.md`)
// Phase B orchestrator wire-up. Three contracts:
//
//   1. Explicit-role join: deploy `role: listener` + SCXML
//      `<sce:session-role kind="accept-side"/>` resolves into
//      `listener_links` independently of the legacy `Accepting.*`
//      substate walker.
//
//   2. Three typed cross-doc partial-claim diagnostics fire on the
//      corresponding partial-shape:
//        - `link/deploy-role-listener-without-scxml-accept-side-role`
//        - `scxml/accept-side-role-without-listener-link`
//        - `link/role-listener-with-non-session-arming-trust-class`
//
//   3. Legacy fixtures (no explicit role + no SCXML session-role
//      declaration) silent-pass and continue resolving through the
//      C10-α walker — Phase B preserves Phase A's green-test
//      invariant for unmigrated fixtures.

use std::path::PathBuf;

use sce_build::forge::error::ValidationError;
use sce_build::mesh::deploy::{parse_deploy_str, LinkRole};
use sce_build::model::{SCXMLModel, SessionRoleKind, State};
use sce_build::{resolve_listener_links, validate_cross_doc_listener_roles};

// ── Shared deploy fixture (mirrors c10_alpha_listener_sibling_pair.rs
//    deploy_with_listener_source shape; the MCU baseline fields are
//    required by the deploy schema's MachineConfig). ────────────────

fn deploy_with_listener_source(source: &str, link_decl: &str) -> String {
    format!(
        r#"
version: "1.0"
topology:
  mcu_device:
    machines:
      mcu_node:
        source: {source}
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
{link_decl}
"#,
    )
}

fn listener_link_with_role_and_session_arming() -> String {
    [
        "          udp_listener:",
        "            bind: \"0.0.0.0:7447\"",
        "            driver: lwip_udp",
        "            mtu_bytes: 1500",
        "            role: listener",
        "            domain_attrs:",
        "              trust_class: session_arming",
        "            session_arming_quota: 8",
        "            accept_rate_per_sec: 4",
        "            accept_rate_burst: 8",
        "",
    ]
    .join("\n")
}

fn listener_link_with_role_but_untrusted() -> String {
    [
        "          udp_listener:",
        "            bind: \"0.0.0.0:7447\"",
        "            driver: lwip_udp",
        "            mtu_bytes: 1500",
        "            role: listener",
        "            domain_attrs:",
        "              trust_class: untrusted",
        "",
    ]
    .join("\n")
}

fn listener_link_role_only_no_domain_attrs() -> String {
    [
        "          udp_listener:",
        "            bind: \"0.0.0.0:7447\"",
        "            driver: lwip_udp",
        "            mtu_bytes: 1500",
        "            role: listener",
        "",
    ]
    .join("\n")
}

fn established_session_link_no_role() -> String {
    [
        "          udp_listener:",
        "            bind: \"0.0.0.0:7447\"",
        "            driver: lwip_udp",
        "            mtu_bytes: 1500",
        "            domain_attrs:",
        "              trust_class: established_session",
        "",
    ]
    .join("\n")
}

fn session_arming_link_no_role() -> String {
    [
        "          udp_listener:",
        "            bind: \"0.0.0.0:7447\"",
        "            driver: lwip_udp",
        "            mtu_bytes: 1500",
        "            domain_attrs:",
        "              trust_class: session_arming",
        "            session_arming_quota: 8",
        "            accept_rate_per_sec: 4",
        "            accept_rate_burst: 8",
        "",
    ]
    .join("\n")
}

fn scxml_with_accept_side() -> SCXMLModel {
    let mut m = SCXMLModel {
        name: "session_fsm".into(),
        ..SCXMLModel::default()
    };
    m.declared_session_roles.insert(SessionRoleKind::AcceptSide);
    m
}

fn scxml_with_legacy_accepting_substate() -> SCXMLModel {
    let mut m = SCXMLModel {
        name: "session_fsm".into(),
        ..SCXMLModel::default()
    };
    m.states
        .insert("Accepting.AwaitingInitSyn".to_string(), State::default());
    m
}

fn scxml_plain() -> SCXMLModel {
    SCXMLModel {
        name: "session_fsm".into(),
        ..SCXMLModel::default()
    }
}

// ── Direction 1: explicit-role join lands in listener_links ────────

#[test]
fn explicit_role_pair_resolves_into_listener_links() {
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &listener_link_with_role_and_session_arming(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_with_accept_side();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    validate_cross_doc_listener_roles(&cfg, &models)
        .expect("happy explicit-role pair passes validator");

    let listener_links = resolve_listener_links(&cfg, &models);
    assert!(
        listener_links.contains("udp_listener"),
        "explicit-role pair must resolve into listener_links, got {:?}",
        listener_links
    );
}

// ── Direction 2: pre-explicit-declaration fixtures silent-skip
//    after Phase D walker deletion ────────────────────────────────

#[test]
fn pre_axis3_fixture_without_explicit_role_silent_skips() {
    // Phase D deletion of the substate-driven walker means a deploy
    // that declares `session_arming` trust class but does not declare
    // `role: listener`, paired with an SCXML that carries `Accepting.*`
    // states but no `<sce:session-role>` declaration, no longer
    // resolves into listener_links. The matching parser-time
    // migration-helper diagnostic
    // `scxml/accept-side-states-without-role-declaration` catches
    // the partial-claim shape at parse time when the SCXML is read
    // from source; this test constructs the SCXMLModel directly so
    // the parser path is bypassed and the only observable behavior
    // is that the explicit-role join silent-skips.
    let yaml = deploy_with_listener_source("session_fsm.scxml", &session_arming_link_no_role());
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_with_legacy_accepting_substate();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    validate_cross_doc_listener_roles(&cfg, &models).expect(
        "pre-axis3 fixture (no explicit role on either side) silent-passes the cross-doc check",
    );

    let listener_links = resolve_listener_links(&cfg, &models);
    assert!(
        listener_links.is_empty(),
        "without explicit role declarations on BOTH sides, no listener pair resolves; got {:?}",
        listener_links
    );
}

// ── Direction 3: partial-claim diagnostics ─────────────────────────

#[test]
fn deploy_role_listener_without_scxml_accept_side_fires() {
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &listener_link_with_role_and_session_arming(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_plain();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    let err =
        validate_cross_doc_listener_roles(&cfg, &models).expect_err("partial-claim must reject");
    match *err {
        ValidationError::LinkDeployRoleListenerWithoutScxmlAcceptSideRole {
            machine,
            link_name,
        } => {
            assert_eq!(machine, "mcu_node");
            assert_eq!(link_name, "udp_listener");
        }
        other => panic!(
            "expected LinkDeployRoleListenerWithoutScxmlAcceptSideRole, got: {:?}",
            other
        ),
    }
}

#[test]
fn scxml_accept_side_without_deploy_listener_fires() {
    let yaml =
        deploy_with_listener_source("session_fsm.scxml", &established_session_link_no_role());
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_with_accept_side();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    let err =
        validate_cross_doc_listener_roles(&cfg, &models).expect_err("partial-claim must reject");
    match *err {
        ValidationError::ScxmlAcceptSideRoleWithoutListenerLink {
            machine,
            scxml_source,
        } => {
            assert_eq!(machine, "mcu_node");
            assert_eq!(scxml_source, "session_fsm.scxml");
        }
        other => panic!(
            "expected ScxmlAcceptSideRoleWithoutListenerLink, got: {:?}",
            other
        ),
    }
}

#[test]
fn role_listener_with_untrusted_trust_class_fires() {
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &listener_link_with_role_but_untrusted(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_with_accept_side();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    let err = validate_cross_doc_listener_roles(&cfg, &models)
        .expect_err("Q-A4 matrix violation must reject");
    match *err {
        ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass {
            machine,
            link_name,
            trust_class,
        } => {
            assert_eq!(machine, "mcu_node");
            assert_eq!(link_name, "udp_listener");
            assert_eq!(trust_class, "untrusted");
        }
        other => panic!(
            "expected LinkRoleListenerWithNonSessionArmingTrustClass, got: {:?}",
            other
        ),
    }
}

#[test]
fn role_listener_without_domain_attrs_fires_matrix_with_absent_payload() {
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &listener_link_role_only_no_domain_attrs(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_with_accept_side();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    let err = validate_cross_doc_listener_roles(&cfg, &models)
        .expect_err("absent domain_attrs with role: listener is still a matrix violation");
    match *err {
        ValidationError::LinkRoleListenerWithNonSessionArmingTrustClass { trust_class, .. } => {
            assert_eq!(
                trust_class, "(absent)",
                "absent domain_attrs surfaces as `(absent)` actual payload so the failure is \
                 distinguishable from a present-but-wrong trust tier"
            );
        }
        other => panic!(
            "expected LinkRoleListenerWithNonSessionArmingTrustClass, got: {:?}",
            other
        ),
    }
}

// ── Silent-pass cases (deploy.role = None / Initiator + no SCXML
//    accept-side; legacy fixtures pre-migration). ──────────────────

#[test]
fn legacy_session_arming_without_role_silent_passes_validator() {
    let yaml = deploy_with_listener_source("session_fsm.scxml", &session_arming_link_no_role());
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_plain();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    validate_cross_doc_listener_roles(&cfg, &models)
        .expect("session_arming + no explicit role + plain SCXML is a legacy fixture; silent-pass");
}

#[test]
fn role_initiator_silent_passes_validator() {
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &[
            "          udp_initiator:",
            "            bind: \"0.0.0.0:7447\"",
            "            driver: lwip_udp",
            "            mtu_bytes: 1500",
            "            role: initiator",
            "            domain_attrs:",
            "              trust_class: established_session",
            "",
        ]
        .join("\n"),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let scxml = scxml_plain();
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    validate_cross_doc_listener_roles(&cfg, &models)
        .expect("role: initiator is forward-compat — must silent-pass in v1");
}

#[test]
fn missing_scxml_model_silent_passes_validator() {
    let yaml = deploy_with_listener_source(
        "absent.scxml",
        &listener_link_with_role_and_session_arming(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    // scxml_models is intentionally empty — the orchestrator's
    // single source of truth (silent-skip when the source file is
    // not in this compile call) extends to the validator.
    let models: Vec<(PathBuf, SCXMLModel)> = vec![];

    // The Q-A4 matrix check still runs (it's deploy-internal). For
    // this happy-trust-tier fixture, no error fires.
    validate_cross_doc_listener_roles(&cfg, &models)
        .expect("missing SCXML model silent-passes the cross-doc partial-claim checks");

    // resolve_listener_links similarly silent-skips for the missing
    // model — the explicit-role join requires the SCXML side to
    // declare the role too.
    let listener_links = resolve_listener_links(&cfg, &models);
    assert!(
        listener_links.is_empty(),
        "no SCXML model ⇒ no listener resolved (silent-skip)"
    );
}

#[test]
fn both_join_paths_dedupe_into_single_listener_entry() {
    // When BOTH the explicit-role pair AND the legacy substate path
    // would resolve the same link, the BTreeSet deduplicates. This
    // is the Phase B transition window — fixtures that have already
    // adopted the explicit shape but still carry `Accepting.*` states
    // (Phase C migration in progress) should produce one entry, not
    // two.
    let yaml = deploy_with_listener_source(
        "session_fsm.scxml",
        &listener_link_with_role_and_session_arming(),
    );
    let cfg = parse_deploy_str(&yaml).expect("deploy parses");
    let mut scxml = scxml_with_accept_side();
    scxml
        .states
        .insert("Accepting.AwaitingInitSyn".to_string(), State::default());
    let models = vec![(PathBuf::from("session_fsm.scxml"), scxml)];

    validate_cross_doc_listener_roles(&cfg, &models).expect("happy fixture must pass");

    let listener_links = resolve_listener_links(&cfg, &models);
    assert_eq!(
        listener_links.len(),
        1,
        "both join paths resolving the same link must dedupe to one entry, got {:?}",
        listener_links
    );
    assert!(listener_links.contains("udp_listener"));
}

#[test]
fn link_role_enum_wire_form_is_canonical() {
    assert_eq!(LinkRole::Listener.as_str(), "listener");
    assert_eq!(LinkRole::Initiator.as_str(), "initiator");
}
