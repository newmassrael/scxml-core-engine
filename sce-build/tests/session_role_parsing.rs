// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Session-role declaration parsing. Pins the SCXML-side
// `<sce:session-role kind="..."/>` parser path + the deploy-side
// `LinkConfig.role:` field parser path. The orchestrator join of the
// two declarations is covered by `orchestrator_role_join.rs`; this
// file only proves:
//
//   1. `<sce:session-role kind="accept-side"/>` is captured into
//      `SCXMLModel.declared_session_roles`.
//   2. An unknown `kind` value surfaces
//      `scxml/unknown-session-role-kind` with the v1 vocabulary list
//      under `Fix::ReplaceOneOf`.
//   3. The same kind declared twice surfaces
//      `scxml/duplicate-session-role-declaration`.
//   4. Missing `kind` attribute surfaces `validation/missing-attribute`.
//   5. Deploy.yaml `role: listener` parses into
//      `LinkConfig.role = Some(LinkRole::Listener)`; absence parses as
//      `None`.
//   6. Deploy.yaml `role: <unknown>` is rejected by serde at parse time.

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::model::SessionRoleKind;
use sce_build::parser::SCXMLParser;

const SCXML_NO_ROLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0">
  <state id="s0"/>
</scxml>
"#;

const SCXML_ACCEPT_SIDE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0">
  <sce:session-role kind="accept-side"/>
  <state id="s0"/>
</scxml>
"#;

const SCXML_UNKNOWN_KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0">
  <sce:session-role kind="listener"/>
  <state id="s0"/>
</scxml>
"#;

const SCXML_DUPLICATE_ROLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0">
  <sce:session-role kind="accept-side"/>
  <sce:session-role kind="accept-side"/>
  <state id="s0"/>
</scxml>
"#;

const SCXML_MISSING_KIND: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s0">
  <sce:session-role/>
  <state id="s0"/>
</scxml>
"#;

#[test]
fn scxml_without_session_role_leaves_declared_roles_empty() {
    let mut parser = SCXMLParser::new();
    let model = parser
        .parse_string(SCXML_NO_ROLE, "no_role")
        .expect("plain SCXML must parse");
    assert!(
        model.declared_session_roles.is_empty(),
        "absence of <sce:session-role> must leave declared_session_roles empty, got {:?}",
        model.declared_session_roles
    );
}

#[test]
fn scxml_with_accept_side_role_populates_declared_roles() {
    let mut parser = SCXMLParser::new();
    let model = parser
        .parse_string(SCXML_ACCEPT_SIDE, "accept_side")
        .expect("<sce:session-role kind=\"accept-side\"/> must parse");
    assert!(
        model
            .declared_session_roles
            .contains(&SessionRoleKind::AcceptSide),
        "accept-side declaration must land in declared_session_roles, got {:?}",
        model.declared_session_roles
    );
    assert_eq!(
        model.declared_session_roles.len(),
        1,
        "single declaration must produce a single-element set"
    );
}

#[test]
fn scxml_with_unknown_session_role_kind_rejects() {
    let mut parser = SCXMLParser::new();
    let err = parser
        .parse_string(SCXML_UNKNOWN_KIND, "unknown_kind")
        .expect_err("unknown kind must reject at parse time");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::ScxmlUnknownSessionRoleKind { kind, allowed } => {
                assert_eq!(kind, "listener", "kind value must be echoed verbatim");
                assert!(
                    allowed.iter().any(|s| s == "accept-side"),
                    "v1 vocabulary list must contain 'accept-side', got {allowed:?}"
                );
            }
            other => panic!("expected ScxmlUnknownSessionRoleKind, got: {:?}", other),
        },
        other => panic!("expected ScxmlUnknownSessionRoleKind, got: {:?}", other),
    }
}

#[test]
fn scxml_with_duplicate_session_role_kind_rejects() {
    let mut parser = SCXMLParser::new();
    let err = parser
        .parse_string(SCXML_DUPLICATE_ROLE, "duplicate")
        .expect_err("duplicate role kind must reject at parse time");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::ScxmlDuplicateSessionRoleDeclaration { kind } => {
                assert_eq!(kind, "accept-side", "duplicated kind must be echoed");
            }
            other => panic!(
                "expected ScxmlDuplicateSessionRoleDeclaration, got: {:?}",
                other
            ),
        },
        other => panic!(
            "expected ScxmlDuplicateSessionRoleDeclaration, got: {:?}",
            other
        ),
    }
}

#[test]
fn scxml_session_role_missing_kind_attr_rejects() {
    let mut parser = SCXMLParser::new();
    let err = parser
        .parse_string(SCXML_MISSING_KIND, "missing_kind")
        .expect_err("missing kind attribute must reject");
    match err.error {
        ForgeError::Validation(boxed) => match *boxed {
            ValidationError::MissingAttribute { element, attr } => {
                assert_eq!(element, "<sce:session-role>");
                assert_eq!(attr, "kind");
            }
            other => panic!(
                "expected MissingAttribute on <sce:session-role>, got: {:?}",
                other
            ),
        },
        other => panic!(
            "expected MissingAttribute on <sce:session-role>, got: {:?}",
            other
        ),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Deploy.yaml side — `LinkConfig.role` parses into Option<LinkRole>.
// ─────────────────────────────────────────────────────────────────────

// LinkConfig parses standalone — bypasses the larger DeployConfig
// schema (which carries platform / scheduler / memory required fields
// orthogonal to role declarations) and exercises just the `role:` field.

fn parse_link_config(
    yaml: &str,
) -> Result<sce_build::mesh::deploy::LinkConfig, serde_yaml_ng::Error> {
    serde_yaml_ng::from_str(yaml)
}

#[test]
fn deploy_link_without_role_parses_as_none() {
    let yaml = r#"bind: "0.0.0.0:7447"
driver: "lwip_udp"
"#;
    let link = parse_link_config(yaml).expect("link must parse");
    assert!(
        link.role.is_none(),
        "absence of `role:` must parse as None, got {:?}",
        link.role
    );
}

#[test]
fn deploy_link_with_role_listener_parses_into_link_role_listener() {
    use sce_build::mesh::deploy::LinkRole;
    let yaml = r#"bind: "0.0.0.0:7447"
driver: "lwip_udp"
role: listener
"#;
    let link = parse_link_config(yaml).expect("link must parse");
    assert_eq!(
        link.role,
        Some(LinkRole::Listener),
        "role: listener must parse into LinkRole::Listener"
    );
}

#[test]
fn deploy_link_with_role_initiator_parses_into_link_role_initiator() {
    use sce_build::mesh::deploy::LinkRole;
    let yaml = r#"bind: "0.0.0.0:7447"
driver: "lwip_udp"
role: initiator
"#;
    let link = parse_link_config(yaml).expect("link must parse");
    assert_eq!(
        link.role,
        Some(LinkRole::Initiator),
        "role: initiator must parse into LinkRole::Initiator"
    );
}

#[test]
fn deploy_link_with_unknown_role_rejects() {
    let yaml = r#"bind: "0.0.0.0:7447"
driver: "lwip_udp"
role: broker
"#;
    let res = parse_link_config(yaml);
    assert!(
        res.is_err(),
        "unknown role value must be rejected by serde (closed enum vocabulary)"
    );
}

#[test]
fn session_role_kind_wire_form_is_canonical() {
    assert_eq!(SessionRoleKind::AcceptSide.as_str(), "accept-side");
    assert_eq!(
        SessionRoleKind::all_wire_names(),
        &["accept-side"] as &[&str],
        "v1 vocabulary must be the closed `[accept-side]` set"
    );
    assert_eq!(
        SessionRoleKind::parse("accept-side"),
        Some(SessionRoleKind::AcceptSide)
    );
    assert!(SessionRoleKind::parse("listener").is_none());
    assert!(SessionRoleKind::parse("").is_none());
}
