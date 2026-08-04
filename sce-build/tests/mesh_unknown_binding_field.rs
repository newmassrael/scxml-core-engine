// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A binding key that no transport reads must be rejected, not absorbed.
//
// `BindingConfig` collects unrecognised keys into `extra` via
// `serde(flatten)` because transport-native keys (`key:` for zenoh,
// `topic:` for dds, `protocol:` for someip) are not modellable as typed
// fields on a shared struct. That mechanism is load-bearing, so
// `deny_unknown_fields` is not the repair — it would reject the very keys
// the flatten exists to carry. What it needs is a per-transport known-key
// set, which the transport registry is already the single source of truth
// for (`required_binding_fields`).
//
// The failure this closes is the one the deploy schema is otherwise built
// to prevent: an author writes a tuning key, the build succeeds, and the
// setting never reaches the wire. Every sibling surface already rejects
// its unknown keys — the device-level `transports:` blocks all carry
// `deny_unknown_fields`, and reserved SOME/IP ID keys are named and
// refused. The per-binding surface was the hole.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use sce_build::generator::Language;
use sce_build::mesh::error::{DeployError, MeshError};

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
        let dir = std::env::temp_dir().join(format!("sce_mesh_unknown_binding_field_{tag}"));
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

/// Compile `brake` against a deploy.yaml whose `#motor` binding carries
/// `binding_keys` verbatim (already indented to the binding's level).
fn compile(tag: &str, transports: &str, binding_keys: &str) -> Result<(), MeshError> {
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
{transports}
    machines:
      brake:
        source: brake.scxml
        bindings:
          "#motor":
{binding_keys}
      motor:
        source: motor.scxml
"##
        ),
    );

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(BRAKE_SCXML, "brake").expect("parse");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp).map(|_| ())
}

const ZENOH_DEVICE: &str = "      zenoh:\n        mode: peer\n";

#[test]
fn unknown_binding_key_is_rejected() {
    // `qos:` is not a key any transport reads. Today it lands in `extra`
    // and evaporates; the author's deployment runs with none of the
    // reliability settings they wrote.
    let err = compile(
        "unknown",
        ZENOH_DEVICE,
        "            transport: zenoh\n            key: \"brake/motor\"\n            qos: reliable\n",
    )
    .expect_err("a key no transport reads must not compile");

    match &err {
        MeshError::Deploy(boxed) if matches!(**boxed, DeployError::UnknownBindingField { .. }) => {
            let DeployError::UnknownBindingField {
                field,
                transport,
                location,
                ..
            } = &**boxed
            else {
                unreachable!("guarded by the arm's matches!")
            };
            assert_eq!(field, "qos");
            assert_eq!(transport, "zenoh");
            assert!(
                location.ends_with("machines.brake.bindings.#motor"),
                "location must address the offending binding: {location}"
            );
        }
        other => panic!("expected UnknownBindingField, got {other:?}"),
    }

    // The diagnostic must name the key and offer the legal set — an agent
    // repairing the file needs the candidates, not just a refusal.
    let text = err.to_string();
    assert!(
        text.contains("qos"),
        "message must name the offending key: {text}"
    );
    assert!(
        text.contains("key"),
        "message must list the transport's legal keys: {text}"
    );
}

#[test]
fn near_miss_key_is_rejected_with_its_neighbour_named() {
    // A typo is the common case, so the candidate list alone is not the
    // whole repair — the closest legal key is the answer, and the
    // diagnostic should say so rather than making the reader diff two
    // lists by eye.
    let err = compile(
        "typo",
        ZENOH_DEVICE,
        "            transport: zenoh\n            key: \"brake/motor\"\n            orderng: required\n",
    )
    .expect_err("a misspelt key must not compile");

    let text = err.to_string();
    assert!(
        text.contains("closest legal key: `ordering`"),
        "a one-character typo of `ordering` must be named as the suggestion, \
         not left for the reader to spot in the candidate list: {text}"
    );
}

#[test]
fn transport_native_keys_still_compile() {
    // The other direction, and the reason `deny_unknown_fields` is wrong
    // here: `key:` is not a `BindingConfig` field — it reaches the
    // template through `extra`. Rejecting unknown keys must not reject
    // the known transport-native ones.
    compile(
        "native",
        ZENOH_DEVICE,
        "            transport: zenoh\n            key: \"brake/motor\"\n",
    )
    .expect("a zenoh binding's own `key:` must remain legal");
}

#[test]
fn someip_protocol_selector_is_not_an_unknown_key() {
    // `protocol: tcp` is read by the someip template arm (it selects the
    // reliable vsomeip path) but is likewise not a typed field. Accepting
    // it here is what proves the known-key set is per-transport rather
    // than one global list — a global list would have to admit `protocol:`
    // on zenoh too, where nothing reads it.
    //
    // The fixture has no vsomeip.json, so this binding fails external
    // resolution; the assertion is narrow on purpose — whatever else
    // fails, it must not be the unknown-key gate.
    let result = compile(
        "someip",
        "      someip:\n        config: vsomeip.json\n        application_name: ecu1\n",
        "            transport: someip\n            service: MotorService\n            method: activate\n            protocol: tcp\n",
    );
    if let Err(MeshError::Deploy(boxed)) = &result {
        if let DeployError::UnknownBindingField { field, .. } = &**boxed {
            panic!(
                "`protocol:` is read by the someip arm and must not be gated as unknown \
                 (got '{field}')"
            );
        }
    }
}
