// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §8.2 derives DDS QoS from what each leg means, and the debt
// register recorded that as "22 QoS policies not exposed". Measuring the
// implementation gives a different shape: there are eleven policy
// settings, and every one of them is load-bearing —
//
//   request/reply : RELIABLE, KEEP_ALL          (+ IGNORE_LOCAL on read)
//   notification  : RELIABLE, KEEP_LAST(1),
//                   TRANSIENT_LOCAL             (+ IGNORE_LOCAL on read)
//
// Inverting the durability pair hands a late-joining server a backlog of
// stale requests, or denies a late subscriber the current value.
// Dropping IGNORE_LOCAL makes a device read its own writes. So the debt
// is not "let a deployment override these" — it should not be able to.
// The real gap is the policies SCE sets *nowhere*, which a deployment
// therefore cannot reach at all.
//
// This file pins both halves of that reading: the orthogonal overlay
// reaches the emitted router, and the derived policies remain
// unreachable through it.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
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
        let dir = std::env::temp_dir().join(format!("sce_mesh_dds_qos_{tag}"));
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

fn compile(tag: &str, qos_block: &str) -> Result<String, sce_build::mesh::error::MeshError> {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        &format!(
            r##"
version: "1.0"
topology:
  ecu1:
    transports:
      dds:
        domain_id: 42
{qos_block}    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
            transport: dds
            topic: "SceBrakeMotor"
      motor:
        source: motor.scxml
"##
        ),
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(BRAKE_SCXML, "brake").expect("parse");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .map(|r| r.output.files[0].1.clone())
}

#[test]
fn declared_overlay_reaches_the_emitted_participant() {
    let code = compile(
        "declared",
        "        qos:\n\
         \x20         notify_lifespan_ms: 3000\n\
         \x20         latency_budget_ms: 7\n\
         \x20         transport_priority: 11\n\
         \x20         partition: \"sce/lab7\"\n\
         \x20         deadline_ms: 250\n\
         \x20         liveliness_lease_ms: 4000\n",
    )
    .expect("compile");

    for expected in [
        "q.notify_lifespan_ms = 3000",
        "q.latency_budget_ms = 7",
        "q.transport_priority = 11",
        "q.partition = \"sce/lab7\"",
        "q.deadline_ms = 250",
        "q.liveliness_lease_ms = 4000",
        "sceDdsQosOverlay()",
        // The obligation the other four policies do not carry: a declared
        // DEADLINE / LIVELINESS must reach `raiseCommunicationError`, or
        // it is a setting whose violation nobody hears.
        "setQosViolationHandler",
        "NotificationDeadlineMissed",
        "PeerPartitioned",
    ] {
        assert!(
            code.contains(expected),
            "declared QoS must reach the emitted overlay: expected `{expected}`"
        );
    }
}

#[test]
fn overlay_without_deadline_or_liveliness_emits_no_raise_path() {
    // The raise wiring is emitted only for the two policies that need it.
    // A deployment that declares, say, only a partition gets no listener
    // registration — a callback for statuses that can never fire would be
    // cost with no signal behind it.
    let code = compile(
        "no_watch",
        "        qos:\n          partition: \"sce/lab7\"\n",
    )
    .expect("compile");

    assert!(code.contains("q.partition = \"sce/lab7\""));
    assert!(
        !code.contains("setQosViolationHandler"),
        "no deadline and no liveliness declared ⇒ no status listener"
    );
}

#[test]
fn absent_overlay_emits_no_overlay_at_all() {
    // Not "emits zeros": a deployment that declares no `qos:` must reach
    // the pre-existing constructor unchanged, so the participant is
    // built exactly as it was before this surface existed.
    let code = compile("absent", "").expect("compile");

    assert!(
        code.contains("DDS_DOMAIN_ID"),
        "the dds arm must still be emitted"
    );
    assert!(
        !code.contains("sceDdsQosOverlay"),
        "an undeclared overlay must emit no overlay code"
    );
}

#[test]
fn derived_policies_are_not_declarable() {
    // The half of §8.2 that matters most. Each of these four names a
    // policy SCE derives from leg semantics; a deployment that could set
    // one could break the pattern with no build-time or runtime signal.
    // The overlay's `deny_unknown_fields` is what forecloses them, so
    // this test is what keeps a future "just add a passthrough map" from
    // quietly reopening the door.
    for policy in ["reliability", "durability", "history", "ignore_local"] {
        let result = compile(
            &format!("derived_{policy}"),
            &format!("        qos:\n          {policy}: something\n"),
        );
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("`{policy}:` is derived from leg semantics and must not be declarable"),
        };
        assert!(
            err.to_string().contains(policy),
            "the diagnostic must name the refused policy `{policy}`: {err}"
        );
    }
}

#[test]
fn declaring_a_dds_default_is_rejected() {
    // Zero latency budget and the empty partition ARE the DDS defaults,
    // so declaring them reads as a setting while changing nothing —
    // the same class of lie as a key that parses and is never read.
    for (field, value) in [("latency_budget_ms", "0"), ("partition", "\"\"")] {
        let result = compile(
            &format!("default_{field}"),
            &format!("        qos:\n          {field}: {value}\n"),
        );
        let err = match result {
            Err(e) => e,
            Ok(_) => panic!("declaring the DDS default for `{field}` must not compile"),
        };
        let text = err.to_string();
        assert!(
            text.contains(field),
            "the diagnostic must name the offending field `{field}`: {text}"
        );
        assert!(
            text.contains("omit"),
            "the diagnostic must name the repair — omitting the field: {text}"
        );
    }
}
