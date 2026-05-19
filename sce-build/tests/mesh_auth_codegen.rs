// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// End-to-end codegen tests for SCE_MESH §16.7 row 10 auth-layer wiring.
//
// Drives the full `compile_mesh_transport` pipeline on a deploy.yaml
// that declares `outbound_buffer:` and a per-binding `auth:` block,
// then asserts the generated `<machine>_transport.h` contains the
// expected row-10 markers:
//
//   * Zenoh path:
//     - per-target `<target>_auth_unauthorized_fired_` atomic flag
//     - ZException catch block inspects `ZException::what()` lowercased
//       against `certificate / tls / auth / handshake` keyword set
//     - on match, raises UNAUTHORIZED for the auth-required target with
//       `transport_status = <what>` and the one-shot flag exchanged
//
//   * SOMEIP path:
//     - per-target `<target>_auth_unauthorized_fired_` atomic flag
//     - `register_availability_handler` callback, on `is_available=false`,
//       one-shot exchanges the flag and raises UNAUTHORIZED with
//       `transport_status = "vsomeip SD denial ..."`
//
// Negative variant: a deploy.yaml without `auth:` must NOT emit any
// of the above markers — the existing row 1 / row 8 classifications
// remain the only path.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.io/mesh"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="active">
      <send target="#motor" event="service.fire_forget.activate"/>
    </transition>
  </state>
  <state id="active"/>
</scxml>
"##;

const MOTOR_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="motor" initial="ready">
  <state id="ready">
    <transition event="service.fire_forget.activate" target="ready"/>
  </state>
</scxml>
"##;

const VSOMEIP_JSON: &str = r#"{
  "applications": [ { "name": "brake_app" } ],
  "services": [{
    "name": "motor_control",
    "service": "0x1234",
    "instance": "0x0001",
    "methods": [
      { "name": "activate", "method": "0x0421" }
    ]
  }]
}"#;

const PINNED_FP: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_auth_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        Self { dir }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.join(name);
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn parse_brake() -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser
        .parse_string(BRAKE_SCXML, "brake")
        .expect("parse brake")
}

#[test]
fn zenoh_auth_section_emits_zexception_classifier_and_unauthorized_raise() {
    let fx = Fixture::new("zenoh_on");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        &format!(
            r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pending_per_target: 8
        bindings:
          "#motor":
            transport: zenoh
            key: sce/brake/motor
            auth:
              required: true
              peer_fingerprint: "{PINNED_FP}"
      motor:
        source: motor.scxml
"##,
        ),
    );

    let mut model = parse_brake();
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    // Per-target one-shot flag emitted.
    assert!(
        code.contains("motor_auth_unauthorized_fired_"),
        "per-target auth one-shot flag must be emitted for zenoh bindings"
    );

    // ZException catch block delegates classification to the shared
    // helper (single source of truth for the keyword set; unit-tested
    // separately in mesh_auth_classifier_test.cpp).
    assert!(
        code.contains("#include \"mesh/AuthClassifier.h\""),
        "auth header include must be emitted when any binding declares auth"
    );
    assert!(
        code.contains("::SCE::Mesh::isZenohAuthFailMessage("),
        "ZException catch block must delegate to the shared classifier"
    );

    // The raise sequence: one-shot exchange → UNAUTHORIZED with
    // transport_status carrying the ZException::what() text.
    assert!(
        code.contains("motor_auth_unauthorized_fired_.exchange("),
        "auth raise must guard on the one-shot flag exchange"
    );
    assert!(
        code.contains("__sce_auth_err.reason = \"UNAUTHORIZED\""),
        "auth raise must stamp reason=UNAUTHORIZED"
    );
    assert!(
        code.contains("__sce_auth_err.transport = \"zenoh\""),
        "auth raise must stamp transport=zenoh"
    );
    assert!(
        code.contains("__sce_auth_err.transport_status = __sce_what_msg;"),
        "auth raise must carry the ZException::what() text as transport_status"
    );
}

#[test]
fn someip_auth_section_emits_availability_classifier_and_unauthorized_raise() {
    let fx = Fixture::new("someip_on");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pending_per_target: 8
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: activate
            auth:
              required: true
              sd_denied_classifies_as_unauthorized: true
      motor:
        source: motor.scxml
"##,
    );

    let mut model = parse_brake();
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    // Per-target one-shot flag emitted.
    assert!(
        code.contains("motor_auth_unauthorized_fired_"),
        "per-target auth one-shot flag must be emitted for someip bindings"
    );

    // The availability handler arm raises UNAUTHORIZED on
    // is_available=false, one-shot-guarded, with the SOMEIP-specific
    // transport_status string.
    assert!(
        code.contains("motor_auth_unauthorized_fired_.exchange("),
        "auth raise must guard on the one-shot flag exchange"
    );
    assert!(
        code.contains("__sce_auth_err.reason = \"UNAUTHORIZED\""),
        "auth raise must stamp reason=UNAUTHORIZED"
    );
    assert!(
        code.contains("__sce_auth_err.transport = \"someip\""),
        "auth raise must stamp transport=someip"
    );
    assert!(
        code.contains("vsomeip SD denial"),
        "auth raise must carry the SD-denial sentinel as transport_status"
    );
}

#[test]
fn no_auth_section_emits_no_row10_wiring() {
    let fx = Fixture::new("off");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        outbound_buffer:
          max_pending_per_target: 8
        bindings:
          "#motor":
            transport: zenoh
            key: sce/brake/motor
      motor:
        source: motor.scxml
"##,
    );

    let mut model = parse_brake();
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    assert!(
        !code.contains("auth_unauthorized_fired_"),
        "auth one-shot flag must NOT be emitted when no binding declares auth"
    );
    assert!(
        !code.contains("UNAUTHORIZED"),
        "no UNAUTHORIZED raise may appear when auth is absent — row-10 stays \
         dormant and rejection signals route through row 1 / row 8 as before"
    );
}
