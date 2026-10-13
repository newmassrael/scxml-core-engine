// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Rust machine with no script engine that reads its events' typed data and sends
// to a Mesh peer: the event's metadata is kept for the peer's reply
// (`_event.invokeid`, SCE_MESH.md), so the generated policy fills in the metadata of
// each event as it is taken, and the machine reads its data through its schema
// and not through a text field of the policy.
//
// The generator declared that text field (`pending_event_data`) only for a machine
// with a script engine, and filled it in for any machine that has event data and the
// function that fills metadata. A machine of this shape therefore named a field it
// did not have, and the Rust it was given did not compile. It was found by running
// two such machines against each other (backends/rust/world), which is the first
// thing to have built one.
//
// The test holds the output to its own declaration: wherever the field is named, it is
// declared.

use sce_build::{compile_scxml_lang_typed, find_template_dir_for};

const SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="net_ping" sce:event-name="net.ping">
  <datamodel>
    <data id="n" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

const MACHINE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="up" datamodel="sce-static" name="mesh_reader">
  <sce:import kind="event-schema" src="net_ping.scxml" as="Ping"/>
  <datamodel>
    <data id="last" sce:type="uint32" expr="0" sce:direction="out"/>
  </datamodel>
  <state id="up">
    <transition event="net.ping" type="internal">
      <assign location="last" expr="_event.data.n"/>
      <send target="#peer" event="net.pong"/>
    </transition>
  </state>
</scxml>
"##;

#[test]
fn a_field_the_policy_fills_in_is_a_field_it_declares() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(dir.path().join("net_ping.scxml"), SCHEMA).expect("the schema is written");
    let machine = dir.path().join("mesh_reader.scxml");
    std::fs::write(&machine, MACHINE).expect("the machine is written");

    let language = sce_build::generator::Language::Rust;
    let output = compile_scxml_lang_typed(
        machine.to_str().expect("a path"),
        &find_template_dir_for(language),
        language,
    )
    .expect("the machine generates");
    let code = output
        .files
        .iter()
        .find(|(name, _)| name == "mesh_reader_sm.rs")
        .map(|(_, code)| code.as_str())
        .expect("the machine's Rust is generated");

    // The control: the function that fills the metadata in is emitted, so the
    // machine is of the shape this holds, and a change that stopped emitting it
    // would not pass this test by saying nothing.
    assert!(
        code.contains("fn populate_event_metadata"),
        "a machine that sends to a Mesh peer fills in each event's metadata"
    );
    let declared = code.contains("    pending_event_data: ");
    let named = code.contains("self.pending_event_data");
    assert!(
        !named || declared,
        "the policy names `pending_event_data` and does not declare it"
    );
}
