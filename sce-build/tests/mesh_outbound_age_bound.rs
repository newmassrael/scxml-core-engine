// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE_MESH.md §4.5 states that "events arriving before READY are buffered
// (configurable timeout)". §10.10's `OutboundBuffer` is what does that
// buffering, and until this landing its only bound was a count
// (`max_pending_per_target`) — the timeout the sentence promised had no
// field to declare it in.
//
// This file holds the declaration end of that claim: a machine that
// declares `max_age_ms` must see the value reach the emitted
// `OutboundBuffer` constructor, and a machine that declares none must
// reach it as the runtime's "no age bound" sentinel rather than as a
// zero-length window that would discard everything.
//
// The runtime end — that an over-age envelope is dropped at drain and
// raises §16.7 row 15 — is `tests/mesh/OutboundBufferTest.cpp`. Split
// this way because the two can fail independently: a codegen that drops
// the argument and a runtime that ignores it look identical from either
// side alone.

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
        let dir = std::env::temp_dir().join(format!("sce_mesh_outbound_age_{tag}"));
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

/// Zenoh rather than SOME/IP so the fixture needs no vsomeip.json; the
/// buffer is emitted for both (`target.state.kind in ("someip", "zenoh")`).
fn deploy_yaml(buffer_section: &str) -> String {
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
        outbound_buffer:
{buffer_section}        bindings:
          "#motor":
            transport: zenoh
            key: "brake/motor"
      motor:
        source: motor.scxml
"##
    )
}

fn compile(tag: &str, buffer_section: &str) -> Result<String, sce_build::mesh::error::MeshError> {
    let fx = Fixture::new(tag);
    fx.write("brake.scxml", BRAKE_SCXML);
    fx.write("motor.scxml", MOTOR_SCXML);
    let deploy_path = fx.write("deploy.yaml", &deploy_yaml(buffer_section));

    let mut parser = sce_build::parser::SCXMLParser::new();
    let mut model = parser.parse_string(BRAKE_SCXML, "brake").expect("parse");
    sce_build::compile_mesh_transport(&mut model, &deploy_path, Language::Cpp)
        .map(|r| r.output.files[0].1.clone())
}

#[test]
fn declared_max_age_reaches_the_emitted_buffer() {
    // 2500 is chosen so it cannot be confused with any default or with
    // the sibling `max_pending_per_target`.
    let code = compile(
        "declared",
        "          max_pending_per_target: 8\n          max_age_ms: 2500\n",
    )
    .expect("compile");

    assert!(
        code.contains("std::chrono::milliseconds(2500)"),
        "a declared `max_age_ms` must reach the OutboundBuffer constructor — \
         otherwise the deploy.yaml says one thing and the router does another"
    );
}

#[test]
fn absent_max_age_emits_the_no_bound_sentinel() {
    // The runtime reads `milliseconds(0)` as "no age bound", which is the
    // pre-existing behaviour a deployment that never asked for one keeps.
    // Emitting some other placeholder — or omitting the argument — would
    // either not compile or silently mean "drop everything".
    let code = compile("absent", "          max_pending_per_target: 8\n").expect("compile");

    assert!(
        code.contains("std::chrono::milliseconds(0)"),
        "an undeclared `max_age_ms` must emit the runtime's no-bound sentinel"
    );
}

#[test]
fn zero_max_age_is_rejected_at_parse_time() {
    // Zero is the one value that cannot mean what it says: as a bound it
    // would discard every buffered envelope at drain, which is
    // indistinguishable from not buffering — while the deployment still
    // reads as having buffering configured. Rejected for the same reason
    // `max_pending_per_target: 0` is.
    let err = match compile(
        "zero",
        "          max_pending_per_target: 8\n          max_age_ms: 0\n",
    ) {
        Err(e) => e,
        Ok(_) => panic!("max_age_ms: 0 must not compile"),
    };

    let text = err.to_string();
    assert!(
        text.contains("max_age_ms"),
        "the diagnostic must name the offending field: {text}"
    );
    assert!(
        text.contains("omit"),
        "the diagnostic must name the repair — omitting the field is what \
         gives the author an unbounded hold: {text}"
    );
}
