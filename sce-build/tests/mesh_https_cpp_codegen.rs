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
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = tempfile::Builder::new()
            .prefix(&format!("sce_mesh_https_cpp_{tag}_"))
            .tempdir()
            .expect("a directory of this fixture's own");
        Self { dir }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(content.as_bytes()).expect("write");
        path
    }
}

/// The brake of [`BRAKE_SCXML`], asking with `<invoke type="sce:mesh-rpc">`
/// rather than a `<send>`.
const BRAKE_INVOKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="waiting"/>
  </state>
  <state id="waiting">
    <invoke type="sce:mesh-rpc" src="#motor">
      <param name="_mesh_event" expr="'service.request.compute_force'"/>
    </invoke>
    <transition event="done.invoke" target="idle"/>
    <transition event="error.invoke" target="idle"/>
  </state>
</scxml>
"##;

/// Generate brake's C++ router with `#motor` bound by `binding` (YAML lines
/// at the binding's indentation) and `machine_extra` (YAML lines at the
/// machine's indentation, possibly empty).
fn generate(tag: &str, binding: &str, machine_extra: &str) -> Result<String, MeshError> {
    generate_brake(tag, BRAKE_SCXML, binding, machine_extra)
}

/// [`generate`] for the brake `brake_scxml`.
fn generate_brake(
    tag: &str,
    brake_scxml: &str,
    binding: &str,
    machine_extra: &str,
) -> Result<String, MeshError> {
    let fx = Fixture::new(tag);
    fx.write("motor.scxml", MOTOR_SCXML);
    fx.write("brake.scxml", brake_scxml);
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
        .parse_string(brake_scxml, "brake")
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

/// SCE_MESH.md section 10.9, invariant 10: the client reads the mailbox of the
/// one server its URL and TLS authenticated, so an envelope that names another
/// machine as its source is dropped where it enters, before the reply
/// admission, the duplicate window or the machine sees it. Red if the check
/// leaves `onHttpsInbound`, or if it follows the reply admission (a forged
/// reply would then retire a request as the responder's).
#[test]
fn what_the_client_receives_names_its_binding_as_its_source() {
    let code = generate("source", DIALING, "").expect("a dialing https binding generates");

    let entry = code
        .find("void onHttpsInbound(SCE::Mesh::MeshEnvelope env, const char* target) {")
        .expect("the client's one inbound path");
    let inbound = &code[entry..];
    let check = inbound
        .find("if (httpsLinkNamesSender(target) && env.source != target) {")
        .expect("the inbound path refuses a source its link does not name");
    let admission = inbound
        .find("admitAddressedReply(env, target)")
        .expect("the reply admission follows");
    assert!(
        check < admission,
        "the source is held to the link before a reply is admitted"
    );

    // `motor` is not a causal binding, so its link names its sender.
    let predicate = code
        .find("static bool httpsLinkNamesSender(const char* target) {")
        .expect("the predicate the check asks");
    let body = &code[predicate..];
    let end = body.find("return true;").expect("the predicate's default");
    assert!(
        !body[..end].contains("return false;"),
        "a plain binding is not exempted: {}",
        &body[..end]
    );
}

/// SCE_MESH.md section 22.4: a failed exchange loses no request, and a new
/// incarnation of the server's mailbox asks again. Each request is kept as
/// it leaves for the binding, the client's `restarted` sends again each one
/// that still waits, and a `<send>` request waits while its reply event is
/// pending. Red if no `restarted` is wired, or if a request is not kept as it
/// leaves.
#[test]
fn a_new_incarnation_sends_each_waiting_request_again() {
    let code = generate("restart", DIALING, "").expect("a dialing https binding generates");

    assert!(code.contains("SCE::Mesh::Https::WaitingRequests motor_waiting_;"));
    assert!(
        code.contains("motor_waiting_.record("),
        "a request is kept as it leaves for the binding"
    );
    assert!(code.contains("https_events.restarted = [this]() {"));
    assert!(code.contains("motor_waiting_.stillWaiting("));
    assert!(
        code.contains("(void)route_send(\"#motor\", request);"),
        "what still waits is sent again by the binding's own route"
    );
    assert!(code.contains("bool httpsRequestWaits(const SCE::Mesh::MeshEnvelope& request) const {"));
    assert!(
        code.contains(
            "return pending_rpcs_.find(CorrelationKey{*request.correlation_id}) != pending_rpcs_.end();"
        ),
        "a `<send>` request waits while its reply event is pending"
    );
}

/// The same for `<invoke type="sce:mesh-rpc">`: the request waits while its
/// correlation entry is live.
#[test]
fn a_mesh_rpc_invoke_waits_while_its_correlation_entry_is_live() {
    let code = generate_brake("restart-invoke", BRAKE_INVOKE_SCXML, DIALING, "")
        .expect("an https binding with a mesh-rpc invoke generates");

    assert!(code
        .contains("if (request.invoke_id && invoke_correlation_.contains(*request.invoke_id)) {"));
    assert!(code.contains("motor_waiting_.record("));
    assert!(code.contains("https_events.restarted = [this]() {"));
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

/// §mesh-22.9 (docs/adr/0109, D9): a binding that names `verify` hands its client the
/// trust set's name, and the `key` authenticator is the client's own
/// enumerator; one that names no `verify` sets none, so its client takes a
/// `200` as it comes. Red if the template drops `verify` or maps `key` to
/// another authenticator.
#[test]
fn a_binding_that_verifies_names_its_trust_set_to_the_client() {
    let code = generate(
        "verify",
        "            url: \"https://motor.example:9443/api\"\n            authenticator: key\n            verify: \"servers\"\n",
        "",
    )
    .expect("a dialing https binding that signs and verifies generates");
    assert!(code.contains("https_config.authenticator = SCE::Mesh::Https::Authenticator::Key;"));
    assert!(
        code.contains("https_config.verify = std::string(\"servers\");"),
        "the client verifies under the trust set the binding names"
    );

    let plain = generate("verify-none", DIALING, "").expect("a dialing https binding generates");
    assert!(
        !plain.contains("https_config.verify"),
        "a binding that names no trust set verifies nothing"
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
