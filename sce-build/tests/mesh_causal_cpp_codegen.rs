// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The generated C++ router carries a causal binding on every transport it
// serves (docs/adr/0085, section 3.2: the causal route is a layer of the Mesh
// core above the transport). Each case drives `compile_mesh_transport` on a
// deploy whose one binding, `#motor`, is the route of a causal group over one
// transport, and reads in the generated router what that transport needs for
// the route's vector, its re-sent originals and its peer's envelopes to reach
// the route. Each names the change that turns it red.

use std::fs;
use std::path::PathBuf;

use sce_build::generator::Language;

const BRAKE_SCXML: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="brake" initial="idle">
  <state id="idle">
    <transition event="start" target="idle">
      <send target="#motor" event="service.fire_forget.activate"/>
    </transition>
  </state>
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

/// The motor's service interface, with one method for the event and one
/// for the causal route's vector.
const VSOMEIP_JSON: &str = r#"{
  "applications": [ { "name": "brake_app" } ],
  "services": [{
    "name": "motor_control",
    "service": "0x1234",
    "instance": "0x0001",
    "methods": [
      { "name": "activate", "method": "0x0421" },
      { "name": "causal_vector", "method": "0x0422" }
    ]
  }]
}"#;

/// The keys every case's binding carries: the causal group of the two.
const CAUSAL_KEYS: &str = "            delivery: causal\n            \
     group: { name: pair, view: [ecu1/brake, ecu1/motor] }\n";

struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sce_mesh_causal_cpp_{tag}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("brake.scxml"), BRAKE_SCXML).expect("write brake");
        fs::write(dir.join("motor.scxml"), MOTOR_SCXML).expect("write motor");
        fs::write(dir.join("vsomeip.json"), VSOMEIP_JSON).expect("write vsomeip.json");
        Self { dir }
    }

    /// The brake's router for a deploy whose device carries `transports`
    /// and whose `#motor` binding carries `binding` (each line indented for
    /// the binding's map), or the build's refusal as text.
    fn router(&self, transports: &str, binding: &str) -> Result<String, String> {
        let deploy = format!(
            "version: \"1.0\"\ntopology:\n  ecu1:\n{transports}    machines:\n      brake:\n        \
             source: brake.scxml\n        bindings:\n          \"#motor\":\n{binding}{CAUSAL_KEYS}      \
             motor:\n        source: motor.scxml\n"
        );
        let deploy_path = self.dir.join("deploy.yaml");
        fs::write(&deploy_path, deploy).expect("write deploy.yaml");
        let mut parser = sce_build::parser::SCXMLParser::new();
        let mut model = parser
            .parse_string(BRAKE_SCXML, "brake")
            .expect("parse brake");
        let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
            .map_err(|error| error.to_string())?;
        Ok(result
            .output
            .files
            .iter()
            .find(|(name, _)| name.ends_with("_transport.h"))
            .map(|(_, code)| code.clone())
            .expect("the C++ backend writes `<document>_transport.h`"))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// What every causal router holds whatever its transport: the route, the
/// offer of the binding's sends and the interception of its peer's envelopes.
fn assert_routes_through_the_causal_route(router: &str) {
    for expected in [
        "SCE::Mesh::CausalRoute causal_motor_{",
        "return offerCausal(\"#motor\", causal_motor_,",
        "if (env.source == \"motor\") {",
        "if (takeCausal(env, session_idx)) {",
    ] {
        assert!(router.contains(expected), "missing `{expected}`:\n{router}");
    }
}

#[test]
fn a_causal_binding_over_zenoh_puts_its_vector_on_the_bindings_key() {
    // Red if the vector is left out of the put arm of `send_zenoh`: the
    // switch then refuses pattern 23 and no repair is ever asked for.
    let fx = Fixture::new("zenoh");
    let router = fx
        .router(
            "",
            "            transport: zenoh_keyexpr\n            key: \"sce/motor\"\n",
        )
        .expect("C++ routes a causal binding over zenoh_keyexpr");
    assert_routes_through_the_causal_route(&router);
    assert!(router.contains("case PK::CausalVector: {"), "{router}");
}

#[test]
fn a_causal_binding_over_dds_writes_every_envelope_on_its_topic() {
    // DDS carries the envelope whole on the binding's request topic, and the
    // peer's server dispatches it through its router. Red if the C++ emitter
    // drops the binding's group, which leaves the binding a plain route.
    let fx = Fixture::new("dds");
    let router = fx
        .router(
            "    transports:\n      dds:\n        domain_id: 42\n",
            "            transport: dds\n            topic: \"SceBrakeMotor\"\n",
        )
        .expect("C++ routes a causal binding over dds");
    assert_routes_through_the_causal_route(&router);
}

#[test]
fn a_causal_binding_over_local_writes_only_through_the_peers_router() {
    // Red if an unlinked causal target is dispatched into the target
    // engine's queue, where no route judges its tag.
    let fx = Fixture::new("local");
    let router = fx
        .router("", "            transport: local\n")
        .expect("C++ routes a causal binding over local");
    assert_routes_through_the_causal_route(&router);
    assert!(
        router.contains("the peer reads through its router"),
        "{router}"
    );
    assert!(
        !router.contains("SCE::Mesh::dispatchEnvelope<TargetPolicy>(env, motor_);"),
        "a causal local target is never written into its engine directly:\n{router}"
    );
}

#[test]
fn a_causal_binding_over_shm_drains_its_peers_channel_through_the_router() {
    // Red if the router does not open the channel the peer creates for its
    // binding back, or drains it into the engine rather than through the
    // route.
    let fx = Fixture::new("shm");
    let router = fx
        .router("", "            transport: shm\n")
        .expect("C++ routes a causal binding over shm");
    assert_routes_through_the_causal_route(&router);
    for expected in [
        "CAUSAL_SHM_FROM_MOTOR = \"/sce_motor_brake\";",
        "std::size_t pumpCausalShm() {",
        "(void)this->dispatchToSession(env, 0); },",
    ] {
        assert!(router.contains(expected), "missing `{expected}`:\n{router}");
    }
}

#[test]
fn a_causal_binding_over_someip_sends_its_vector_on_the_method_it_declares() {
    // SOME/IP addresses a message by the method its service interface gives
    // a name. The vector carries no event of the document, so the binding
    // declares its method under the reserved name `mesh.causal_vector`,
    // resolved against vsomeip.json like every event. Red if the send arm
    // for pattern 23 is removed: the vector then falls to the unknown-event
    // return and is never sent.
    let fx = Fixture::new("someip");
    let transports = "    transports:\n      someip:\n        config: vsomeip.json\n        \
                      application_name: brake_app\n";
    let router = fx
        .router(
            transports,
            "            transport: someip\n            service: motor_control\n            \
             events:\n              \"service.fire_forget.activate\":\n                \
             method: activate\n              \"mesh.causal_vector\":\n                \
             method: causal_vector\n",
        )
        .expect("C++ routes a causal binding over someip");
    assert_routes_through_the_causal_route(&router);
    assert!(
        router.contains("SOMEIP_CAUSAL_VECTOR_METHOD_MOTOR = 0x0422;"),
        "{router}"
    );
    assert!(
        router.contains("if (env.pattern == SCE::Mesh::PatternKind::CausalVector) {"),
        "{router}"
    );

    // Without the declaration the binding is refused by name rather than
    // built without its vectors.
    let refused = fx
        .router(
            transports,
            "            transport: someip\n            service: motor_control\n            \
             events:\n              \"service.fire_forget.activate\":\n                \
             method: activate\n",
        )
        .expect_err("a causal SOME/IP binding without its vector's method is refused");
    assert!(refused.contains("mesh.causal_vector"), "{refused}");
}
