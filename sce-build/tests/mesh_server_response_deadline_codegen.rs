// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Codegen tests for the SCE Mesh §mesh-9.5 server response deadline.
//
// The knob is transport-neutral but its *notice* is not, and that split
// is the thing these tests pin. Every server arm parks inbound request
// state that leaks when the engine never answers; what differs is what
// the requesting peer learns on expiry, which the transport registry
// records as `TransportDescriptor::server_deadline_notice`:
//
//   * someip → `ActiveError`: expiry answers MT_ERROR (0x81) with
//     E_TIMEOUT (0x06) carrying `RpcStatus::DeadlineExceeded`, so the
//     peer can tell a server that gave up from one that vanished.
//     Neither vsomeip nor `ara::com` arms a server-side deadline at all,
//     so this arm has no reference behaviour to match — only to exceed.
//   * zenoh → `DropSilently`: expiry destructs the stored `zenoh::Query`
//     and the peer infers `RpcStatus::Unavailable` from the drop. The
//     query model carries no server-authored failure channel.
//
// A byte-level assertion is the only way to keep the two arms from
// converging by accident: both compile, and a zenoh arm that silently
// grew an MT_ERROR emit (or a SOME/IP arm that quietly degraded to a
// drop) would still build.
//
// Negative variant: a deploy.yaml without the knob must emit none of
// the markers — the deadline is opt-in and costs zero lines when absent.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sce_build::generator::Language;

const MOTOR_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="motor" initial="ready">
  <state id="ready">
    <transition event="service.request.compute_force" target="computing"/>
  </state>
  <state id="computing">
    <onentry>
      <raise event="service.response.compute_force"/>
    </onentry>
    <transition target="ready"/>
  </state>
</scxml>
"##;

const VSOMEIP_JSON: &str = r#"{
  "applications": [ { "name": "motor_app" } ],
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
        let dir = std::env::temp_dir().join(format!("sce_mesh_server_deadline_{tag}"));
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

fn parse_motor() -> sce_build::model::SCXMLModel {
    let mut parser = sce_build::parser::SCXMLParser::new();
    parser
        .parse_string(MOTOR_SCXML, "motor")
        .expect("parse motor")
}

fn generate(deploy_path: &Path) -> String {
    let mut model = parse_motor();
    let result = sce_build::compile_mesh_transport(&mut model, deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(
        result.output.files.len(),
        1,
        "one generated file per machine"
    );
    result.output.files[0].1.clone()
}

/// Markers that must appear on every arm that arms the deadline at all,
/// regardless of the notice it emits.
fn assert_deadline_armed(code: &str, arm: &str) {
    assert!(
        code.contains("#include \"mesh/MeshDeadlineScheduler.h\""),
        "{arm}: deadline scheduler header must be included"
    );
    assert!(
        code.contains("SCE::Mesh::MeshDeadlineScheduler deadline_scheduler_"),
        "{arm}: scheduler member must be emitted"
    );
    assert!(
        code.contains("onServerRequestTimedOut"),
        "{arm}: expiry callback must be emitted"
    );
    assert!(
        code.contains("deadline_scheduler_.registerDeadline"),
        "{arm}: an inbound request must arm a scheduler entry"
    );
    assert!(
        code.contains("std::chrono::milliseconds(500)"),
        "{arm}: the deploy.yaml value must reach the generated arm verbatim"
    );
    assert!(
        code.contains("server_shutdown_in_progress_"),
        "{arm}: teardown must be able to suppress a late callback"
    );
    assert!(
        code.contains("deadline_scheduler_.cancelDeadline"),
        "{arm}: answering before expiry must retire the timer"
    );
}

#[test]
fn someip_server_deadline_answers_mt_error_with_e_timeout() {
    let fx = Fixture::new("someip_on");
    fx.write("vsomeip.json", VSOMEIP_JSON);
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
        application_name: motor_app
    machines:
      motor:
        source: motor.scxml
        server:
          transport: someip
          service: motor_control
          response_deadline_ms: 500
          events:
            "service.request.compute_force":
              method: compute_force
"##,
    );

    let code = generate(&deploy_path);
    assert_deadline_armed(&code, "someip");

    // The notice itself. AUTOSAR SOME/IP reserves MT_ERROR / E_TIMEOUT
    // for exactly this condition, and using the protocol's own slot is
    // what makes the timeout legible to a non-SCE peer as well.
    assert!(
        code.contains("set_message_type(vsomeip::message_type_e::MT_ERROR)"),
        "someip expiry must answer with the protocol's error message type"
    );
    assert!(
        code.contains("set_return_code(vsomeip::return_code_e::E_TIMEOUT)"),
        "someip expiry must name the timeout in the return code"
    );
    assert!(
        code.contains("SCE::Mesh::RpcStatus::DeadlineExceeded"),
        "the carried envelope must state DeadlineExceeded, not the \
         Unavailable a vanished peer produces"
    );
    assert!(
        code.contains("\"error.rpc.deadline\""),
        "the notice must carry its own event name so the requester can \
         branch on it"
    );
    assert!(
        code.contains("service.request.compute_force"),
        "the reason text must name the abandoned call"
    );

    // `create_response` copies the request's client/session/service/
    // method header, so the error reply correlates on the wire exactly
    // as a normal response would. Hand-building a message here would
    // strand it.
    assert!(
        code.contains("create_response(original)"),
        "the notice must be derived from the stored request so it correlates"
    );
}

#[test]
fn zenoh_server_deadline_drops_without_a_notice() {
    let fx = Fixture::new("zenoh_on");
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      motor:
        source: motor.scxml
        server:
          transport: zenoh
          key: "sce/motor/rpc"
          response_deadline_ms: 500
"##,
    );

    let code = generate(&deploy_path);
    assert_deadline_armed(&code, "zenoh");

    assert!(
        code.contains("pending_server_queries_.erase"),
        "zenoh expiry releases the stored query"
    );
    // The registry says `DropSilently` for zenoh because the query model
    // has no server-authored failure channel — not because SCE chose to
    // stay quiet. If this arm ever grows a notice, the registry entry
    // has to move first.
    assert!(
        !code.contains("MT_ERROR"),
        "zenoh has no protocol slot for a timeout notice; emitting one \
         here would contradict the registry's DropSilently classification"
    );
    assert!(
        !code.contains("RpcStatus::DeadlineExceeded"),
        "zenoh expiry cannot stamp a status — the client infers \
         Unavailable from the drop"
    );
}

#[test]
fn absent_knob_emits_no_deadline_code() {
    let fx = Fixture::new("off");
    fx.write("vsomeip.json", VSOMEIP_JSON);
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
        application_name: motor_app
    machines:
      motor:
        source: motor.scxml
        server:
          transport: someip
          service: motor_control
          events:
            "service.request.compute_force":
              method: compute_force
"##,
    );

    let code = generate(&deploy_path);
    for marker in [
        "onServerRequestTimedOut",
        "server_shutdown_in_progress_",
        "MT_ERROR",
        "E_TIMEOUT",
        "registerDeadline",
    ] {
        assert!(
            !code.contains(marker),
            "absent knob must emit zero deadline code, found `{marker}`"
        );
    }
}

#[test]
fn someip_client_accepts_an_error_reply_without_renaming_it() {
    // The requesting half. Two defects would each turn the server's
    // notice back into silence, and both are one line away:
    //
    //   * a message-type gate that admits only MT_RESPONSE drops the
    //     notice on the floor, leaving the request to age out on the
    //     client's own timer — the very behaviour the server deadline
    //     exists to replace;
    //   * renaming a failed reply to the declared reply event hands the
    //     author's `<transition event="service.response.X">` an empty
    //     payload and calls it success.
    let fx = Fixture::new("client");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("motor.scxml", MOTOR_SCXML);
    let brake = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="waiting">
      <send target="#motor" event="service.request.compute_force"/>
    </transition>
  </state>
  <state id="waiting">
    <transition event="service.response.compute_force" target="idle"/>
  </state>
</scxml>
"##;
    fx.write("brake.scxml", brake);
    let deploy_path = fx.write(
        "deploy.yaml",
        r##"
version: "1.0"
topology:
  ecu1:
    transports:
      someip:
        config: vsomeip.json
        application_name: motor_app
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: compute_force
      motor:
        source: motor.scxml
"##,
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(brake, "brake").expect("parse brake");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    let code = &result.output.files[0].1;

    assert!(
        code.contains("vsomeip::message_type_e::MT_ERROR"),
        "the client reply handler must admit MT_ERROR — dropping it turns \
         a server-authored failure into silence"
    );
    assert!(
        code.contains("*env.rpc_status == SCE::Mesh::RpcStatus::Ok"),
        "the reply rewrite must be status-aware so a failure is not \
         renamed into the declared success event"
    );
}
