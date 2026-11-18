// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The client arm of the `https` binding in the generated C++ router
// (docs/adr/0107, D2).
//
// The router carries only the wiring; the binding's semantics are the
// client's (sce/include/mesh/transports/HttpsClient.h), compiled once. What
// these tests hold is that wiring: the host's adapter reaches the router at
// construction, each binding gets one client built in init() from the
// binding's resolved fields, what the client receives enters through one
// inbound path with the reply admission the other addressed transports
// share, and the binding's retry policy is the client's rather than a
// RetryingDispatcher wrapped around a send that only queues. The side that
// accepts is refused by name, since the C++ router has no server for it.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::mesh::error::{CodegenError, CppRouterGap, MeshError, TopologyError};

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

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
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

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_https_cpp_{tag}"));
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

/// Generate brake's C++ router with `#motor` bound by `binding` (YAML lines
/// at the binding's indentation) and `machine_extra` (YAML lines at the
/// machine's indentation, possibly empty).
fn generate(tag: &str, binding: &str, machine_extra: &str) -> Result<String, MeshError> {
    let fx = Fixture::new(tag);
    fx.write("motor.scxml", MOTOR_SCXML);
    fx.write("brake.scxml", BRAKE_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        &format!(
            r##"
version: "1.0"
topology:
  phone:
    machines:
      brake:
        source: brake.scxml
{machine_extra}        bindings:
          "#motor":
            transport: https
{binding}
  cloud:
    machines:
      motor:
        source: motor.scxml
"##
        ),
    );
    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser
        .parse_string(BRAKE_SCXML, "brake")
        .expect("parse brake");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .map(|result| result.output.files[0].1.clone())
}

const DIALING: &str =
    "            url: \"https://motor.example:8443/api\"\n            authenticator: bearer\n";

#[test]
fn a_dialing_binding_builds_its_client_from_the_hosts_adapter() {
    let code = generate("dial", DIALING, "").expect("a dialing https binding generates");

    assert!(
        code.contains("#include \"mesh/transports/HttpsClient.h\""),
        "the client's one implementation is included, not restated"
    );
    // The host owns network policy: the adapter and credentials reach the
    // router at construction.
    assert!(
        code.contains("const SCE::Mesh::Https::ClientHost& https_host"),
        "the constructor takes the host's adapter"
    );
    assert!(
        code.contains("https_host_(https_host)"),
        "the host is kept for init()"
    );
    assert!(
        code.contains("std::unique_ptr<SCE::Mesh::Https::Client> motor_https_;"),
        "one client per binding"
    );
    // The binding's resolved fields, the hold's default among them.
    assert!(code.contains("https_config.base_url = \"https://motor.example:8443/api\";"));
    assert!(code.contains("https_config.client = \"brake\";"));
    assert!(code.contains("https_config.peer = \"motor\";"));
    assert!(code.contains("https_config.target = \"#motor\";"));
    assert!(code.contains("https_config.authenticator = SCE::Mesh::Https::Authenticator::Bearer;"));
    assert!(
        code.contains("https_config.hold = std::chrono::milliseconds(25000);"),
        "an absent hold_ms is the default, resolved once in topology"
    );
    assert!(code.contains("motor_https_->start()"));
    assert!(
        code.contains("motor_https_->stop();"),
        "shutdown joins the client before the router goes"
    );
    // Without a buffer a send goes straight to the client's outbox.
    assert!(code.contains("return motor_https_->send(env);"));
    // What the client receives enters through one path, which carries the
    // binding's own identity for the responder check.
    assert!(code.contains("this->onHttpsInbound(std::move(env), \"motor\")"));
    assert!(
        code.contains("admitAddressedReply(env, target)"),
        "a request's reply is admitted as the other addressed transports admit it"
    );
    assert!(
        code.contains("pending_rpcs_.erase(it)"),
        "the reply retires its one-shot correlation entry"
    );
    // A send that failed reaches the author through the client's events.
    assert!(code.contains("https_events.notReady"));
    assert!(code.contains("raiseCommunicationError(std::move(err));"));
    // No buffer was declared, so readiness gates nothing here.
    assert!(!code.contains("motor_outbound_"));
}

#[test]
fn the_retry_policy_is_the_clients_and_wraps_no_dispatcher() {
    let code = generate(
        "retry",
        &format!("{DIALING}            retry:\n              max_retries: 3\n"),
        "        outbound_buffer:\n          max_pending_per_target: 8\n",
    )
    .expect("an https binding with a retry policy generates");

    assert!(
        code.contains("https_config.retry = SCE::Mesh::Https::RetryPolicy{"),
        "the client applies the policy to an exchange no answer confirmed"
    );
    assert!(code.contains("/*max_retries=*/3"));
    assert!(
        !code.contains("RetryingDispatcher motor_retry_"),
        "a dispatcher around a send that only queues would retry nothing"
    );
}

#[test]
fn a_buffer_is_released_by_the_clients_first_answered_exchange() {
    let code = generate(
        "buffer",
        DIALING,
        "        outbound_buffer:\n          max_pending_per_target: 8\n",
    )
    .expect("an https binding with an outbound buffer generates");

    assert!(code.contains("SCE::Mesh::OutboundBuffer motor_outbound_;"));
    assert!(
        code.contains("https_events.ready = [this]() { motor_outbound_.markReady(); };"),
        "the first exchange answered 200 is the ready edge"
    );
    assert!(code.contains("motor_outbound_.markNotReady();"));
    assert!(code.contains("return motor_outbound_.admit(env);"));
    assert!(
        code.contains("motor_https_->offer(env)"),
        "the buffer's dispatcher offers to the outbox and reports a refusal itself"
    );
}

#[test]
fn the_cpp_router_refuses_the_side_that_accepts() {
    let err = generate(
        "accept",
        "            listen: \"0.0.0.0:8443\"\n            authenticator: bearer\n",
        "",
    )
    .expect_err("the C++ router carries only the client");
    match err {
        MeshError::Codegen(CodegenError::CppRouterUnsupported { machine, feature }) => {
            assert_eq!(machine, "brake");
            assert_eq!(
                feature,
                CppRouterGap::AcceptingSide {
                    transport: "https".to_string()
                }
            );
        }
        other => panic!("expected CppRouterUnsupported, got {other:?}"),
    }
}

#[test]
fn a_binding_without_an_authenticator_never_reaches_codegen() {
    let err = generate(
        "unauthenticated",
        "            url: \"https://motor.example\"\n",
        "",
    )
    .expect_err("a mailbox binding without an authenticator is refused");
    match err {
        MeshError::Topology(boxed) => match *boxed {
            TopologyError::BindingWithoutAuthenticator { transport, .. } => {
                assert_eq!(transport, "https");
            }
            other => panic!("expected BindingWithoutAuthenticator, got {other:?}"),
        },
        other => panic!("expected a topology refusal, got {other:?}"),
    }
}
