// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A literal a `send` carries lands in the field of the schema the document
// imports for the event (SCE_FORGE.md, "Typing convention"): it takes that
// field's type, is held to its range, and is spelled in it, since no statement
// of the language declares a place around the value the wire helper takes.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A schema of a 64-bit and an unsigned 32-bit field.
const BOOKED: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="room_booked" sce:event-name="room.booked">
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
    <data id="at" sce:type="int64" sce:direction="in"/>
    <data id="count" sce:type="uint32" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A statechart that sends `room.booked` with the literals `params`.
fn statechart(params: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <state id="s">
    <transition event="go" type="internal">
      <send event="room.booked">
        <param name="room" expr="'east'"/>
{params}
      </send>
    </transition>
  </state>
</scxml>
"#
    )
}

/// Generate `doc` for Rust, [`BOOKED`] beside it; `(ok, output, written)`.
fn generate(doc: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("room_booked.scxml"), BOOKED).expect("write the schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write the statechart");
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out dir");
    let output = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "rust", "-o"])
        .arg(&out)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    let mut written = String::new();
    for entry in std::fs::read_dir(&out).expect("read out").flatten() {
        if let Ok(text) = std::fs::read_to_string(entry.path()) {
            written.push_str(&text);
        }
    }
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout),
        written,
    )
}

/// A literal past `int32` sent into an `int64` field, and one past `int32` into
/// a `uint32` field, cross in their fields' types, spelled in them. Red if a
/// literal is typed without the field (an `int32`, which refuses both), or is
/// left bare where the wire helper takes it (an `i32` to rustc).
#[test]
fn a_literal_crosses_in_the_type_of_the_field_it_lands_in() {
    let (ok, out, written) = generate(&statechart(
        r#"        <param name="at" expr="1700000000000"/>
        <param name="count" expr="4000000000"/>"#,
    ));
    assert!(ok, "the literals fit their fields:\n{out}");
    for said in [
        "ScriptValue::String((1700000000000i64).to_string())",
        "ScriptValue::Int(i64::from(4000000000u32))",
    ] {
        assert!(
            written.contains(said),
            "the machine holds `{said}`:\n{written}"
        );
    }
}

/// A literal its field's type cannot hold is refused where it is written, as a
/// mistake of the document and not as a lowering the backend lacks. Red if a
/// literal is held to no range, to one wider than its field's, or only where
/// the machine is lowered.
#[test]
fn a_literal_its_field_cannot_hold_is_refused() {
    let (ok, out, _) = generate(&statechart(
        r#"        <param name="at" expr="1"/>
        <param name="count" expr="5000000000"/>"#,
    ));
    assert!(
        !ok && out.contains("\"code\":\"expression/literal-out-of-range\"")
            && out.contains("5000000000 does not fit in uint32"),
        "refused by the field's range:\n{out}"
    );
}
