// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A DDS deployment must be able to name its vendor config file, the way a
// Zenoh (`transports.zenoh.config: zenoh.json5`) and a SOME/IP
// (`transports.someip.config: vsomeip.json`) deployment already can.
//
// Without it, `domain_id` is the entire DDS surface a deployment controls
// and everything Cyclone DDS exposes — discovery peers, transport
// selection, buffer sizes, thread priorities, tracing — is reachable only
// by setting `CYCLONEDDS_URI` in the process environment, outside the file
// that describes the deployment. That is a real asymmetry between siblings,
// not a design choice: the mesh schema has no way to say it, so a deployment
// cannot be reproduced from deploy.yaml alone.
//
// CycloneDDS-CXX takes the config on the participant constructor
// (`TDomainParticipant(id, qos, listener, mask, config)`, where config is
// "the name of the file containing the configuration or, when it starts
// with '<', the XML representation"), so the value belongs in generated
// code rather than in an environment variable the generated code cannot
// see.

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
        let dir = std::env::temp_dir().join(format!("sce_mesh_dds_vendor_config_{tag}"));
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

/// Generate `brake`'s transport header with `dds_keys` spliced into the
/// device's `transports.dds:` block.
fn generate(tag: &str, dds_keys: &str) -> String {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    fx.write(
        "cyclonedds.xml",
        "<CycloneDDS><Domain id=\"any\"/></CycloneDDS>\n",
    );
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
{dds_keys}    machines:
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
    let result = sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .expect("compile_mesh_transport");
    result.output.files[0].1.clone()
}

#[test]
fn declared_vendor_config_reaches_the_participant() {
    let code = generate("declared", "        config: cyclonedds.xml\n");

    assert!(
        code.contains("cyclonedds.xml"),
        "a declared `transports.dds.config:` must reach the generated \
         participant — otherwise the file is named in deploy.yaml and read \
         by nothing"
    );
    assert!(
        code.contains("42"),
        "the domain id must still be emitted alongside the config"
    );
}

#[test]
fn absent_vendor_config_leaves_the_participant_on_defaults() {
    // The other direction: adding the knob must not make the config
    // mandatory, and must not emit an empty path that Cyclone would try
    // to open. An absent `config:` keeps today's behaviour, where the
    // participant falls back to `CYCLONEDDS_URI` or the built-in default.
    let code = generate("absent", "");

    assert!(
        code.contains("DDS_DOMAIN_ID"),
        "the dds arm must still be emitted without a config"
    );
    assert!(
        !code.contains("DDS_CONFIG"),
        "no config declared means no config constant — an empty string \
         would be a path Cyclone cannot open"
    );
}
