// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A page of the event journal a document reads (docs/adr/0114, D5;
// docs/SCE_ACCEPTED_SUBSET.md §2.15, "A page of the event journal"):
// `<invoke type="sce-host-journal-read">` names its stream by `sce:journal` or
// `sce:stream` and the union of its facts by `sce:entries`; the build writes the
// stream into the request, derives the completion's record, and the Rust machine
// opens the page where the completion is lifted. Every other lane refuses the
// operation by name, and the build generated with its deployment refuses a read of
// a journal the machine neither keeps nor follows.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::standard_host::HostLane;
use sce_build::generator::Language;
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

const BOOKED: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="room_booked" sce:event-name="room.booked">
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
    <data id="at" sce:type="int64" sce:direction="in" sce:valid-time="from"/>
  </datamodel>
</scxml>
"#;

const CANCELLED: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="room_cancelled" sce:event-name="room.cancelled">
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

const KIND: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="fact_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="booked" value="0"/>
      <sce:variant name="cancelled" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;

/// An enum of the one fact a narrower union holds.
const KIND_ONE: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="fact_kind" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="cancelled" value="1"/>
    </data>
  </datamodel>
</scxml>
"#;

/// The union of the two facts, by `cases`, its `<sce:case>` lines.
fn union(cases: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="union" name="room_facts">
  <sce:import kind="enum" src="fact_kind.scxml" as="Kind"/>
  <sce:import kind="event-schema" src="room_booked.scxml" as="Booked"/>
  <sce:import kind="event-schema" src="room_cancelled.scxml" as="Cancelled"/>
  <datamodel>
    <data id="kind" sce:type="enum:Kind">
{cases}
    </data>
  </datamodel>
</scxml>
"#
    )
}

const BOTH: &str = r#"      <sce:case variant="booked" record="Booked"/>
      <sce:case variant="cancelled" record="Cancelled"/>"#;

/// A statechart that reads a page of the journal `invoke` names, its `<invoke>`
/// carrying `attrs` and `extra` `<param>`s. It imports the enum of the union's
/// tag, as every statechart that reads a payload holding a union does
/// (docs/adr/0086, section 7.3): the completion's record holds the union.
fn reader(attrs: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="view">
  <sce:import kind="event-schema" src="sce:std/host/journal_read_request.scxml" as="ReadRequest"/>
  <sce:import kind="event-schema" src="sce:std/host/journal_read_result.scxml" as="ReadResult"/>
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <sce:import kind="union" src="room_facts.scxml" as="Facts"/>
  <sce:import kind="enum" src="fact_kind.scxml" as="Kind"/>
  <datamodel>
    <data id="room" sce:type="string[256]" expr="''"/>
    <data id="as_of" sce:type="uint64" expr="0"/>
    <data id="valid" sce:type="int64" expr="9223372036854775807"/>
    <data id="after_valid" sce:type="int64" expr="0"/>
    <data id="after" sce:type="uint64" expr="0"/>
    <data id="limit" sce:type="uint16" expr="16"/>
    <data id="horizon" sce:type="uint64" expr="0"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="reading"/>
  </state>
  <state id="reading">
    <invoke type="sce-host-journal-read" id="page"{attrs} sce:request="ReadRequest" sce:result="ReadResult">
      <param name="key" expr="room"/>
      <param name="as_of_position" expr="as_of"/>
      <param name="valid_ms" expr="valid"/>
      <param name="after_valid_ms" expr="after_valid"/>
      <param name="after_position" expr="after"/>
      <param name="limit" expr="limit"/>{extra}
    </invoke>
    <transition event="done.invoke.page" target="idle">
      <assign location="horizon" expr="_event.data.horizon"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const READS_ROOMS: &str = r#" sce:journal="rooms" sce:entries="Facts""#;

/// The documents beside the reader: the two schemas, the enum of the union's tag
/// (`KIND_ONE` for a union of the one case `cancelled`, [`KIND`] otherwise) and
/// the union of `cases`.
fn siblings(cases: &str) -> Vec<(&'static str, String)> {
    let kind = if cases.contains(r#"variant="booked""#) {
        KIND
    } else {
        KIND_ONE
    };
    vec![
        ("room_booked.scxml", BOOKED.to_string()),
        ("room_cancelled.scxml", CANCELLED.to_string()),
        ("fact_kind.scxml", kind.to_string()),
        ("room_facts.scxml", union(cases)),
    ]
}

/// Run `sce-codegen <args...> view.scxml`, `others` beside it; with `out`, `-o` to
/// a directory of the run's own, whose files are returned joined.
fn run(args: &[&str], out: bool, doc: &str, others: &[(&str, String)]) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in others {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("view.scxml");
    std::fs::write(&path, doc).expect("write the reader");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("out dir");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args);
    if out {
        command.arg("-o").arg(&out_dir);
    }
    let output = command.arg(&path).output().expect("invoke sce-codegen");
    let mut written = String::new();
    for entry in std::fs::read_dir(&out_dir).expect("read out").flatten() {
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

/// docs/adr/0114, D5: the Rust machine is served the read with no declaration,
/// writes the stream into the request, and opens the page into the derived record
/// where the completion is lifted. Red if the build writes no stream, or the
/// lift opens no page.
#[test]
fn a_page_is_read_and_opened_by_the_rust_machine() {
    let doc = reader(READS_ROOMS, "");
    let (ok, out, written) = run(&["generate", "-l", "rust"], true, &doc, &siblings(BOTH));
    assert!(ok, "the Rust machine reads a page:\n{out}");
    for said in [
        "::sce_rust_runtime::journal::open_page(",
        "\"room.booked\"",
        "\"rooms\"",
    ] {
        assert!(
            written.contains(said),
            "the machine holds `{said}`:\n{written}"
        );
    }
}

/// docs/adr/0114, Scope: only the Rust host serves the read, and every other
/// lane refuses the machine by name.
#[test]
fn every_lane_but_rust_refuses_a_read_by_name() {
    let doc = reader(READS_ROOMS, "");
    for language in Language::ALL.iter().filter(|l| **l != Language::Rust) {
        let lane = HostLane::Generated(*language).wire_name();
        let (ok, out, _) = run(&["check", "-l", lane], false, &doc, &siblings(BOTH));
        assert!(
            !ok && out.contains("codegen/standard-host-type-not-served")
                && out.contains("sce-host-journal-read"),
            "{lane} refuses the read by name:\n{out}"
        );
    }
}

/// docs/adr/0114, D5: a read names one stream by a name and a union of event
/// schemas, one case each, and a document never writes the stream itself. Each
/// refusal stands on the attribute. Red if an attribute is read and not judged.
#[test]
fn what_a_read_cannot_be_is_refused_on_its_attribute() {
    for (attrs, extra, cases, code, said) in [
        (
            r#" sce:journal="rooms""#,
            "",
            BOTH,
            "validation/journal-read-schema",
            "names by sce:entries the union",
        ),
        (
            r#" sce:entries="Facts""#,
            "",
            BOTH,
            "validation/journal-read-schema",
            "and this one names none",
        ),
        (
            r#" sce:journal="rooms" sce:stream="directory" sce:entries="Facts""#,
            "",
            BOTH,
            "validation/journal-read-schema",
            "names two",
        ),
        (
            r#" sce:journal="1rooms" sce:entries="Facts""#,
            "",
            BOTH,
            "validation/journal-read-schema",
            "names no stream",
        ),
        (
            r#" sce:journal="rooms" sce:entries="Others""#,
            "",
            BOTH,
            "validation/journal-read-schema",
            "names no union this document imports",
        ),
        (
            READS_ROOMS,
            "",
            r#"      <sce:case variant="booked" record="Booked"/>
      <sce:case variant="cancelled" record="Booked"/>"#,
            "validation/journal-read-schema",
            "two of whose cases hold the record",
        ),
        (
            READS_ROOMS,
            r#"
      <param name="stream" expr="'lobby'"/>"#,
            BOTH,
            "validation/typed-invoke-request",
            "written by the build from sce:journal or sce:stream",
        ),
    ] {
        let (ok, out, _) = run(&["check"], false, &reader(attrs, extra), &siblings(cases));
        assert!(
            !ok && out.contains(&format!("\"code\":\"{code}\"")) && out.contains(said),
            "{attrs}: refused as {code} with `{said}`:\n{out}"
        );
    }
}

/// The deployment a reading pool `view` runs in: `rooms`, whose sessions keep a
/// journal of room facts, and `view`, which takes `lines`.
fn deployment(lines: &str) -> String {
    format!(
        r#"version: "1.0"
topology:
  cloud:
    machines:
      rooms:
        source: rooms.scxml
        sessions: {{ key_from: data.room, idle_ms: 1000 }}
        journal: true
      view:
        source: view.scxml
        sessions: {{ key_from: data.room, idle_ms: 1000 }}
{lines}"#
    )
}

/// The document of `rooms`: it says the fact `room.booked`.
fn rooms() -> String {
    r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="rooms">
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <state id="s">
    <transition event="book" type="internal">
      <send type="sce:journal" event="room.booked">
        <param name="room" expr="'east'"/>
        <param name="at" expr="1"/>
      </send>
    </transition>
  </state>
</scxml>
"#
    .to_string()
}

/// Generate `doc` for Rust with `deploy` as its deployment; `(ok, output)`.
fn generate_deployed(doc: &str, deploy: &str, cases: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let mut others = siblings(cases);
    others.push(("rooms.scxml", rooms()));
    for (name, text) in &others {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("view.scxml");
    std::fs::write(&path, doc).expect("write the reader");
    let deploy_path = dir.path().join("deploy.yaml");
    std::fs::write(&deploy_path, deploy).expect("write deploy");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).expect("out dir");
    let output = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("generate")
        .arg("-l")
        .arg("rust")
        .arg("-o")
        .arg(&out_dir)
        .arg("--deploy")
        .arg(&deploy_path)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout),
    )
}

/// docs/adr/0114, D5 and Open question 1: a document reads only its own journal
/// and those its machine follows, with a union that holds every schema the
/// stream's facts are of. The refusal stands on `sce:journal`. Red if the
/// deployment is not read for the read, or the refusal carries no place.
#[test]
fn a_read_of_a_journal_the_machine_does_not_follow_is_refused_on_its_attribute() {
    let doc = reader(READS_ROOMS, "");
    let line = doc
        .lines()
        .position(|row| row.contains("sce-host-journal-read"))
        .expect("written")
        + 1;
    let (_, out) = generate_deployed(&doc, &deployment("        follows: [rooms]\n"), BOTH);
    assert!(
        !out.contains("mesh/codegen-journal-read-refused"),
        "a followed journal is read:\n{out}"
    );
    let unfollowed = deployment("");
    let (ok, out) = generate_deployed(&doc, &unfollowed, BOTH);
    let record = out
        .lines()
        .find(|row| row.contains("\"code\":\"mesh/codegen-journal-read-refused\""))
        .unwrap_or_else(|| panic!("refused:\n{out}"));
    assert!(!ok, "{out}");
    assert!(
        record.contains("neither this machine nor one it follows"),
        "{record}"
    );
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "placed on the read, line {line}: {record}"
    );
    let narrow = r#"      <sce:case variant="cancelled" record="Cancelled"/>"#;
    let (ok, out) = generate_deployed(&doc, &deployment("        follows: [rooms]\n"), narrow);
    assert!(
        !ok && out.contains("mesh/codegen-journal-read-refused")
            && out.contains("`room.booked` are no case of the union"),
        "a union short of the stream's schemas:\n{out}"
    );
}
