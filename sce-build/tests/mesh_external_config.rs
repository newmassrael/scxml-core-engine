// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end tests for SCE_MESH §13 external infrastructure config integration.
//
// Exercises the full `compile_mesh_transport` pipeline with name-based
// SOME/IP bindings that resolve against a real vsomeip.json side-file.
// Complements the per-module unit tests in `mesh/external.rs` by driving
// the integration through the public API a build.rs / CLI user would see.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::mesh::error::{ExternalConfigError, MeshError};

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.io/mesh"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="active">
      <send target="#motor" event="service.request.compute_force"/>
    </transition>
  </state>
  <state id="active">
    <transition event="service.response.compute_force" target="idle"/>
  </state>
</scxml>
"##;

const MOTOR_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="motor" initial="ready">
  <state id="ready">
    <transition event="service.request.compute_force" target="ready">
      <send target="#brake" event="service.response.compute_force"/>
    </transition>
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
      { "name": "compute_force", "method": "0x0421" }
    ]
  }]
}"#;

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_ext_{tag}"));
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

fn deploy_with_names(vsomeip_rel: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: {vsomeip_rel}
        application_name: brake_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: compute_force
            protocol: udp
      motor:
        source: motor.scxml
"##
    )
}

fn parse_brake() -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser
        .parse_string(BRAKE_SCXML, "brake")
        .expect("parse brake")
}

#[test]
fn end_to_end_name_resolution_produces_numeric_constants() {
    // Writes vsomeip.json + deploy.yaml + SCXML sources, then drives the
    // full mesh pipeline. The generated transport header must carry the
    // numeric IDs that vsomeip.json declares for the named entities.
    let fx = Fixture::new("ok");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_with_names("vsomeip.json"));

    let model = parse_brake();
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");

    // No inline IDs → no deprecation warnings.
    assert!(
        result.deprecation_warnings.is_empty(),
        "unexpected deprecation warnings: {:?}",
        result.deprecation_warnings
    );

    // Exactly one generated mesh header for the brake machine.
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    // Verify the generated template embedded the resolved IDs. Hex
    // formatting is the injection convention (`0x{:04X}`).
    assert!(
        code.contains("0x1234"),
        "expected service_id 0x1234 in generated code: {code}"
    );
    assert!(
        code.contains("0x0001"),
        "expected instance_id 0x0001 in generated code"
    );
    assert!(
        code.contains("0x0421"),
        "expected method_id 0x0421 in generated code"
    );
    // Per-event constant naming (SCE_MESH.md §14): the BRAKE_SCXML
    // fixture sends `service.request.compute_force` so the generated
    // header must declare `SOMEIP_METHOD_MOTOR_SERVICE_REQUEST_COMPUTE_FORCE`.
    assert!(
        code.contains("SOMEIP_METHOD_MOTOR_SERVICE_REQUEST_COMPUTE_FORCE"),
        "expected per-event method constant in generated code: {code}"
    );
}

#[test]
fn per_event_block_resolves_distinct_methods_into_separate_constants() {
    // Two SCXML events on the same target, each mapped to a different
    // method via the spec-canonical events: block. The generated header
    // must carry one constant per (target, event) pair so the dispatch
    // switch can route by event name.
    let fx = Fixture::new("per_event");
    let vsomeip = r#"{
      "applications": [{ "name": "brake_app" }],
      "services": [{
        "name": "motor_control",
        "service": "0x1234",
        "instance": "0x0001",
        "methods": [
          { "name": "compute_force", "method": "0x0421" },
          { "name": "release_force", "method": "0x0422" }
        ]
      }]
    }"#;
    let brake = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="press" target="active">
      <send target="#motor" event="service.request.compute_force"/>
    </transition>
  </state>
  <state id="active">
    <transition event="release" target="idle">
      <send target="#motor" event="service.request.release_force"/>
    </transition>
    <transition event="service.response.compute_force" target="idle"/>
    <transition event="service.response.release_force" target="idle"/>
  </state>
</scxml>
"##;
    let motor = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" name="motor" initial="ready">
  <state id="ready">
    <transition event="service.request.compute_force" target="ready">
      <send target="#brake" event="service.response.compute_force"/>
    </transition>
    <transition event="service.request.release_force" target="ready">
      <send target="#brake" event="service.response.release_force"/>
    </transition>
  </state>
</scxml>
"##;
    let deploy = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            events:
              "service.request.compute_force":
                method: compute_force
              "service.request.release_force":
                method: release_force
      motor:
        source: motor.scxml
"##;
    fx.write("vsomeip.json", vsomeip);
    fx.write("brake.scxml", brake);
    fx.write("motor.scxml", motor);
    let deploy_path = fx.write("deploy.yaml", deploy);

    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_string(brake, "brake")
        .expect("parse brake");
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    let (_name, code) = &result.output.files[0];

    // Both per-event constants must appear with their distinct IDs.
    assert!(
        code.contains("SOMEIP_METHOD_MOTOR_SERVICE_REQUEST_COMPUTE_FORCE = 0x0421"),
        "compute_force constant missing: {code}"
    );
    assert!(
        code.contains("SOMEIP_METHOD_MOTOR_SERVICE_REQUEST_RELEASE_FORCE = 0x0422"),
        "release_force constant missing: {code}"
    );
    // Dispatch must branch on env.type so the right method is picked.
    assert!(
        code.contains(r#"env.type == "service.request.compute_force""#),
        "dispatch for compute_force missing: {code}"
    );
    assert!(
        code.contains(r#"env.type == "service.request.release_force""#),
        "dispatch for release_force missing: {code}"
    );
}

#[test]
fn unused_event_binding_rejected() {
    // events: declares an event that the SCXML model never <send>s to
    // this target → likely a typo, must fail at build time.
    let fx = Fixture::new("unused_event");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            events:
              "service.request.compute_force":
                method: compute_force
              "service.request.never_sent":
                method: compute_force
      motor:
        source: motor.scxml
"##;
    let deploy_path = fx.write("deploy.yaml", deploy);

    let model = parse_brake();
    let err = match sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp) {
        Ok(_) => panic!("must fail on unused event binding"),
        Err(e) => e,
    };
    let msg = format!("{err}");
    assert!(
        msg.contains("never_sent"),
        "error must name the unused event: {msg}"
    );
}

#[test]
fn unresolved_method_name_fails_build_with_config_path() {
    // Seeds a deploy.yaml referencing a method that does not exist in
    // vsomeip.json. compile_mesh_transport must fail with the consolidated
    // diagnostic that includes the vsomeip.json path (spec §13 format).
    let fx = Fixture::new("bad_method");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let bad_deploy = deploy_with_names("vsomeip.json").replace(
        "method: compute_force",
        "method: nonexistent_method",
    );
    let deploy_path = fx.write("deploy.yaml", &bad_deploy);

    let model = parse_brake();
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp);
    let err = match result {
        Ok(_) => panic!("must fail on unresolved method"),
        Err(e) => e,
    };

    match err {
        MeshError::External(ExternalConfigError::UnresolvedNames {
            config_path,
            missing,
            ..
        }) => {
            assert!(
                config_path.contains("vsomeip.json"),
                "error must reference vsomeip.json path: {config_path}"
            );
            assert!(
                missing.iter().any(|m| m.kind == "method" && m.name == "nonexistent_method"),
                "missing entries should include the bad method: {missing:?}"
            );
        }
        other => panic!("expected MeshError::External(UnresolvedNames), got {other}"),
    }
}

#[test]
fn missing_external_config_file_reports_path() {
    // deploy.yaml references a vsomeip.json that does not exist on disk.
    let fx = Fixture::new("missing_file");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_with_names("vsomeip_missing.json"));

    let model = parse_brake();
    let err = match sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp) {
        Ok(_) => panic!("must fail when vsomeip.json missing"),
        Err(e) => e,
    };

    match err {
        MeshError::External(ExternalConfigError::Read { path, .. }) => {
            assert!(
                path.contains("vsomeip_missing.json"),
                "error must reference the missing path: {path}"
            );
        }
        other => panic!("expected MeshError::External(Read), got {other}"),
    }
}

#[test]
fn malformed_external_config_reports_parse_error() {
    // vsomeip.json is present but not valid JSON.
    let fx = Fixture::new("bad_json");
    fx.write("vsomeip.json", "{ not valid json");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_with_names("vsomeip.json"));

    let model = parse_brake();
    let err = match sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp) {
        Ok(_) => panic!("must fail on malformed JSON"),
        Err(e) => e,
    };

    match err {
        MeshError::External(ExternalConfigError::Parse { path, .. }) => {
            assert!(path.contains("vsomeip.json"));
        }
        other => panic!("expected MeshError::External(Parse), got {other}"),
    }
}

#[test]
fn application_name_from_deploy_yaml_reaches_generated_code() {
    // SCE_MESH.md §13: `transports.someip.application_name` binds the
    // generated vsomeip::application to an entry in vsomeip.json's
    // applications[*].name. The template must use that literal instead
    // of the legacy `<machine>_<target>` default.
    let fx = Fixture::new("appname");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_with_names("vsomeip.json"));

    let model = parse_brake();
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");

    let (_name, code) = &result.output.files[0];
    assert!(
        code.contains(r#"create_application("brake_app")"#),
        "generated code must use deploy.yaml application_name literal: {code}"
    );
    assert!(
        !code.contains(r#"create_application("brake_motor")"#),
        "legacy `<machine>_<target>` default must NOT appear when \
         application_name is declared: {code}"
    );
}

#[test]
fn zenoh_config_file_reaches_generated_code() {
    // SCE_MESH.md §14: `transports.zenoh.config:` references an external
    // zenoh.json5. Generated init() uses `Config::from_file(<path>)` as
    // the base; deploy.yaml overrides merge on top.
    let fx = Fixture::new("zenoh_cfg");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    // zenoh.json5 need only exist for the Config::from_file call at
    // runtime; sce-build does not parse it. Write a minimal stub.
    fx.write("zenoh.json5", "{ mode: \"peer\" }");
    let deploy = r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        config: zenoh.json5
        mode: peer
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: zenoh
            key: "brake/cmd"
      motor:
        source: motor.scxml
"##;
    let deploy_path = fx.write("deploy.yaml", deploy);

    let model = parse_brake();
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");

    let (_name, code) = &result.output.files[0];
    assert!(
        code.contains(r#"zenoh::Config::from_file("zenoh.json5")"#),
        "generated code must emit Config::from_file with the escaped path: {code}"
    );
    // The default `create_default()` path must NOT coexist when config
    // file is set — they are mutually exclusive in the template.
    let default_call_count = code.matches("zenoh::Config::create_default()").count();
    assert_eq!(
        default_call_count, 0,
        "create_default must not appear when config: is declared: {code}"
    );
    // Overrides must still be applied on top.
    assert!(
        code.contains(r#"insert_json5("mode""#),
        "deploy.yaml mode override must still be applied: {code}"
    );
}

#[test]
fn inline_numeric_ids_still_compile_with_deprecation_warning() {
    // Legacy fixture shape: all IDs inline, no transports.someip.config.
    // compile_mesh_transport must still succeed (Stage 1 deprecation) but
    // surface a DeprecationWarning per inline ID for the CLI to emit.
    let fx = Fixture::new("legacy_inline");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let legacy = r##"
version: "1.0"
topology:
  ecu1:
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service_id: "0x1234"
            instance_id: "0x0001"
            method_id: "0x0421"
            protocol: udp
      motor:
        source: motor.scxml
"##;
    let deploy_path = fx.write("deploy.yaml", legacy);

    let model = parse_brake();
    let result = sce_build::compile_mesh_transport(&model, &deploy_path, Language::Cpp)
        .expect("inline IDs stay valid under Stage 1 deprecation");

    // At least the three inline IDs on #motor must be reported.
    assert!(
        result.deprecation_warnings.len() >= 3,
        "expected at least 3 deprecation warnings, got {:?}",
        result.deprecation_warnings
    );
    let fields: Vec<_> = result
        .deprecation_warnings
        .iter()
        .map(|w| w.field.as_str())
        .collect();
    assert!(fields.contains(&"service_id"));
    assert!(fields.contains(&"method_id"));
}
