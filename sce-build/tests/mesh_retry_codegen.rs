// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// End-to-end codegen tests for SCE_MESH §16.7 row 3 retry-layer wiring.
//
// Drives the full `compile_mesh_transport` pipeline on a deploy.yaml
// that declares an `outbound_buffer:` section AND a per-binding
// `retry:` block, then asserts the generated `<machine>_transport.h`
// contains the expected RetryingDispatcher markers:
//   * `#include "mesh/RetryingDispatcher.h"`
//   * `SCE::Mesh::RetryingDispatcher motor_retry_;`  member
//   * `motor_retry_(deadline_scheduler_, SCE::Mesh::RetryingDispatcher::Policy{...},
//     [transport-send lambda], [raise lambda])` ctor init
//   * `motor_outbound_(... [this](...) { return motor_retry_.send_with_retry(env); } ...)`
//     OutboundBuffer dispatcher routed through the retry wrapper
//   * `SCE::Mesh::MeshDeadlineScheduler deadline_scheduler_;`
//
// Negative variant: a deploy.yaml without `retry:` must NOT emit any
// of the above markers — the OutboundBuffer dispatcher goes straight
// to the transport-send closure (Stage 1/2 behaviour).

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

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("sce_mesh_retry_{tag}"));
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

#[test]
fn retry_section_emits_retrying_dispatcher_wiring() {
    let fx = Fixture::new("on");
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
            retry:
              max_retries: 3
              initial_backoff_ms: 50
              backoff_multiplier: 2.0
              max_backoff_ms: 1000
              backoff_jitter_pct: 15
      motor:
        source: motor.scxml
"##,
    );

    let mut model = parse_brake();
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    // Header include for the RetryingDispatcher class.
    assert!(
        code.contains("#include \"mesh/RetryingDispatcher.h\""),
        "expected RetryingDispatcher header include in generated code"
    );

    // The shared MeshDeadlineScheduler is emitted (retry layer needs it).
    assert!(
        code.contains("SCE::Mesh::MeshDeadlineScheduler deadline_scheduler_"),
        "expected deadline_scheduler_ member declaration in generated code"
    );

    // Per-target RetryingDispatcher member declared before OutboundBuffer.
    assert!(
        code.contains("SCE::Mesh::RetryingDispatcher motor_retry_"),
        "expected motor_retry_ member declaration"
    );

    // Ctor init with the Policy fields baked from deploy.yaml.
    assert!(
        code.contains("motor_retry_(deadline_scheduler_,"),
        "retry wrapper must be constructed with the shared scheduler"
    );
    assert!(
        code.contains("/*max_retries=*/3"),
        "max_retries must be codegen-baked from deploy.yaml"
    );
    assert!(
        code.contains("/*initial_backoff=*/std::chrono::milliseconds(50)"),
        "initial_backoff_ms must be codegen-baked"
    );
    assert!(
        code.contains("/*max_backoff=*/std::chrono::milliseconds(1000)"),
        "max_backoff_ms must be codegen-baked"
    );
    assert!(
        code.contains("/*jitter_pct=*/15"),
        "backoff_jitter_pct must be codegen-baked"
    );
    assert!(
        code.contains("/*transport=*/\"someip\""),
        "transport literal must be codegen-baked"
    );
    assert!(
        code.contains("/*target=*/\"#motor\""),
        "target literal must be codegen-baked"
    );

    // The OutboundBuffer's dispatcher routes through the retry wrapper.
    assert!(
        code.contains("motor_retry_.send_with_retry(env)"),
        "OutboundBuffer dispatcher must route through the retry wrapper"
    );
}

/// SCE_MESH.md §16.7 row 3 follow-up — when a binding declares both a
/// retry policy and a mesh-rpc `<invoke>` site with a deadline, the
/// generated invoke-lifecycle cancel sites (deadline lambda, author
/// cancel, setup-fault cleanup, zenoh on_drop terminal) must fan out
/// through the new `cancelEnvelopeRetryById` helper so the retry
/// chain cannot fire DELIVERY_EXHAUSTED after the upstream invoke has
/// already terminated.
const BRAKE_INVOKE_RETRY_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="null" name="brake_invoke_retry" initial="idle">
  <state id="idle">
    <transition event="go" target="computing"/>
  </state>
  <state id="computing">
    <invoke type="sce:mesh-rpc" src="#motor">
      <param name="_mesh_event" expr="'service.request.compute'"/>
      <param name="_mesh_deadline_ms" expr="50"/>
    </invoke>
    <transition event="done.invoke" target="ok"/>
    <transition event="error.invoke" target="failed"/>
  </state>
  <final id="ok"/>
  <final id="failed"/>
</scxml>
"##;

#[test]
fn retry_plus_mesh_rpc_emits_cancel_envelope_retry_by_id_wiring() {
    let fx = Fixture::new("preempt");
    fx.write("vsomeip.json", VSOMEIP_JSON);
    fx.write("brake_invoke_retry.scxml", BRAKE_INVOKE_RETRY_SCXML);
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
      brake_invoke_retry:
        source: brake_invoke_retry.scxml
        outbound_buffer:
          max_pending_per_target: 8
        bindings:
          "#motor":
            transport: someip
            service: motor_control
            method: activate
            retry:
              max_retries: 3
              initial_backoff_ms: 50
              backoff_multiplier: 2.0
              max_backoff_ms: 1000
              backoff_jitter_pct: 15
      motor:
        source: motor.scxml
"##,
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser
        .parse_string(BRAKE_INVOKE_RETRY_SCXML, "brake_invoke_retry")
        .expect("parse brake_invoke_retry");
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    assert_eq!(result.output.files.len(), 1, "one generated file per machine");
    let (_name, code) = &result.output.files[0];

    // Helper method declaration appears exactly once.
    assert!(
        code.contains("void cancelEnvelopeRetryById("),
        "helper method cancelEnvelopeRetryById must be emitted when has_any_retry.v"
    );
    assert!(
        code.contains("(void)motor_retry_.cancelEnvelopeRetry(envelope_id);"),
        "helper body must invoke cancelEnvelopeRetry on the motor retry dispatcher"
    );

    // Split-uuid invariant (SCE_MESH.md §16.7 row 3 follow-up):
    // invokeMeshRpc emits TWO uuid v7 calls (one for invoke_uuid, one
    // for envelope_uuid). The previous single-uuid pattern aliased the
    // deadline scheduler key with the retry scheduler key, breaking
    // mesh-rpc + retry composition.
    assert!(
        code.contains("const auto invoke_uuid = SCE::uuid::v7();"),
        "invokeMeshRpc must allocate a dedicated invoke_uuid"
    );
    assert!(
        code.contains("const auto envelope_uuid = SCE::uuid::v7();"),
        "invokeMeshRpc must allocate a dedicated envelope_uuid"
    );
    assert!(
        code.contains("env.id = envelope_uuid;") &&
        code.contains("env.invoke_id = invoke_uuid;"),
        "envelope id must use envelope_uuid; invoke id must use invoke_uuid \
         so retry and invoke deadline keys are disjoint"
    );

    // Deadline scheduler keys off invoke_uuid; the lambda fans out
    // cancelEnvelopeRetryById with envelope_uuid.
    let hd_idx = code
        .find("(void)invoke_correlation_.handleDeadline(invoke_uuid)")
        .expect("deadline lambda must invoke handleDeadline with invoke_uuid");
    let tail = &code[hd_idx..];
    let next_stmt_idx = tail
        .find("cancelEnvelopeRetryById(envelope_uuid)")
        .expect("deadline lambda must invoke cancelEnvelopeRetryById(envelope_uuid)");
    assert!(
        next_stmt_idx < 200,
        "cancelEnvelopeRetryById call site is too far from handleDeadline \
         to be inside the same deadline lambda (delta={next_stmt_idx} chars)"
    );

    // active_invokes_ value is the ActiveInvokeRecord struct carrying
    // both uuids (cancelMeshRpc reads both halves).
    assert!(
        code.contains("ActiveInvokeRecord"),
        "active_invokes_ value type must be ActiveInvokeRecord (struct of \
         invoke_uuid + envelope_uuid)"
    );

    // The cancel sites fan out through the helper using the ENVELOPE
    // uuid (the retry scheduler key). For a someip binding the
    // emitted sites are the deadline lambda, cleanupAbandonedMeshInvoke,
    // and cancelMeshRpc (the fourth — onZenohQueryDropped — is only
    // emitted when the binding is zenoh-transported and the
    // has_zenoh_mesh_rpc gate fires). All sites must key off
    // envelope_uuid / rec.envelope_uuid / envelope_id_capture — NOT
    // invoke_uuid — because the retry scheduler key is env.id, not
    // env.invoke_id.
    let cancel_envelope_calls = code.matches("cancelEnvelopeRetryById(envelope_uuid)").count()
        + code.matches("cancelEnvelopeRetryById(rec.envelope_uuid)").count();
    assert!(
        cancel_envelope_calls >= 3,
        "expected cancelEnvelopeRetryById invoked from at least three someip cancel sites \
         (deadline lambda, cleanupAbandonedMeshInvoke, cancelMeshRpc); \
         observed {cancel_envelope_calls} call sites in generated code"
    );
    // Negative assertion: no cancel site may pass the INVOKE uuid to
    // cancelEnvelopeRetryById — that would re-introduce the row 3
    // key-space collision the split was designed to remove.
    assert!(
        !code.contains("cancelEnvelopeRetryById(invoke_uuid)"),
        "no cancel site may pass invoke_uuid to cancelEnvelopeRetryById — \
         the retry scheduler key is the envelope uuid"
    );
}

#[test]
fn no_retry_section_emits_no_retry_wiring() {
    let fx = Fixture::new("off");
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
        !code.contains("#include \"mesh/RetryingDispatcher.h\""),
        "retry header must NOT be included when no binding declares retry"
    );
    assert!(
        !code.contains("motor_retry_"),
        "retry member must NOT be emitted when no binding declares retry: {code}"
    );
    assert!(
        !code.contains("send_with_retry"),
        "OutboundBuffer dispatcher must go straight to the transport when retry is absent"
    );
}
