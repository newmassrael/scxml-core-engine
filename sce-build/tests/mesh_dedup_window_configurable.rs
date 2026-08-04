// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §10.5 states the duplicate-suppression window's "size is
// configurable; default 256 entries". This file holds that claim to the
// deploy.yaml surface: a machine that declares a window size must see it
// reach the generated router, and a machine that declares none must keep
// the documented default.
//
// The claim is worth a test rather than a comment because the alternative
// failure is the worst kind — an author reads "configurable", writes the
// key, and gets the default silently. Every reference middleware at this
// layer exposes the equivalent knob (Cyclone DDS alone exposes
// `PrimaryReorderMaxSamples`, `SecondaryReorderMaxSamples`,
// `DefragReliableMaxSamples`, `ReceiveBufferSize`), so the size is a
// deployment property, not an implementation detail.

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
        let dir = std::env::temp_dir().join(format!("sce_mesh_dedup_window_{tag}"));
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

/// Zenoh rather than SOME/IP because zenoh's `supplies_dedup` is false with
/// no per-binding escape (a SOME/IP binding can pin `protocol: tcp` and opt
/// out of dedup entirely), so the DedupRouter is guaranteed to be emitted.
fn deploy_yaml(dedup_section: &str) -> String {
    format!(
        r##"
version: "1.0"
topology:
  ecu1:
    transports:
      zenoh:
        mode: peer
    machines:
      brake:
        source: brake.scxml
{dedup_section}        bindings:
          "#motor":
            transport: zenoh
            key: "brake/motor"
      motor:
        source: motor.scxml
"##
    )
}

fn generate(tag: &str, dedup_section: &str) -> String {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_yaml(dedup_section));

    let mut model = parse_brake();
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    let (_name, code) = &result.output.files[0];
    code.clone()
}

#[test]
fn declared_dedup_window_reaches_the_generated_router() {
    // §10.5's "configurable" claim, exercised through the only surface an
    // author has. 512 is chosen so it cannot be confused with the default.
    let code = generate("declared", "        dedup:\n          window_size: 512\n");

    assert!(
        code.contains("mesh/DedupRouter.h"),
        "zenoh binding must emit the dedup layer at all"
    );
    assert!(
        code.contains("512"),
        "declared window_size must reach the generated router; §10.5 says the \
         size is configurable, so a declared 512 cannot silently become 256"
    );
}

#[test]
fn absent_dedup_section_still_emits_the_dedup_layer() {
    // Guards the other direction: making the size configurable must not
    // make the layer conditional. A zenoh binding always needs dedup
    // (§10.5), declared window or not.
    let code = generate("default", "");

    assert!(
        code.contains("mesh/DedupRouter.h"),
        "an undeclared window must still emit the dedup layer"
    );
    assert!(
        code.contains("DedupRouterT<256>"),
        "the §10.5 documented default must be emitted explicitly rather than \
         left to a runtime fallback — deploy.yaml is the only source of the size"
    );
}

#[test]
fn zero_window_is_rejected_at_parse_time() {
    // A zero-length window is not a narrow filter, it is no filter: every
    // duplicate §10.5 exists to suppress would reach the engine while the
    // deployment still reads as having duplicate suppression configured.
    // Rejecting at parse time also keeps `DedupRouterT<0>`'s static_assert
    // from being the thing the author sees — a C++ error pointing at
    // generated code is a worse diagnostic than a deploy.yaml one.
    let fx = Fixture::new("zero");
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write(
        "deploy.yaml",
        &deploy_yaml("        dedup:\n          window_size: 0\n"),
    );

    let mut model = parse_brake();
    let err = match sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp) {
        Err(e) => e,
        Ok(_) => panic!("window_size: 0 must not compile"),
    };

    let text = err.to_string();
    assert!(
        text.contains("window_size"),
        "the diagnostic must name the offending key: {text}"
    );
    assert!(
        text.contains("256"),
        "the diagnostic must name the default the author gets by omitting \
         the section, so the repair does not require reading the spec: {text}"
    );
}
