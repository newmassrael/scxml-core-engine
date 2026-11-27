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

mod common;

use sce_build::forge::standard_host::HostLane;
use sce_build::generator::Language;
use sce_build::toolchain;
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
/// carrying `attrs` and `extra` `<param>`s. It imports every document the
/// completion's record names at any depth (docs/adr/0086, section 7.3), as every
/// statechart that reads a payload holding a union does: the union, the enum of
/// its tag, and the schema of each record its cases hold.
fn reader(attrs: &str, extra: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="view">
  <sce:import kind="event-schema" src="sce:std/host/journal_read_request.scxml" as="ReadRequest"/>
  <sce:import kind="event-schema" src="sce:std/host/journal_read_result.scxml" as="ReadResult"/>
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <sce:import kind="event-schema" src="room_cancelled.scxml" as="RoomCancelled"/>
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
/// where the completion is lifted, each entry by the case its schema's id (the
/// event the schema declares) names. The cases are JSON the machine holds in a
/// Rust string literal, so each quote of theirs is escaped there. Red if the
/// build writes no stream, the lift opens no page, or a case is not written.
#[test]
fn a_page_is_read_and_opened_by_the_rust_machine() {
    let doc = reader(READS_ROOMS, "");
    let (ok, out, written) = run(&["generate", "-l", "rust"], true, &doc, &siblings(BOTH));
    assert!(ok, "the Rust machine reads a page:\n{out}");
    for said in [
        "::sce_rust_runtime::journal::open_page(",
        r#"[\"room.booked\",\"kind\",\"booked\","#,
        r#"[\"room.cancelled\",\"kind\",\"cancelled\","#,
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

/// The document of `rooms`: it says the fact `room.booked` on `book` and
/// `room.cancelled` on `cancel`, both of the room `east`.
fn rooms() -> String {
    r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="rooms">
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <sce:import kind="event-schema" src="room_cancelled.scxml" as="RoomCancelled"/>
  <state id="s">
    <transition event="book" type="internal">
      <send type="sce:journal" event="room.booked">
        <param name="room" expr="'east'"/>
        <param name="at" expr="1"/>
      </send>
    </transition>
    <transition event="cancel" type="internal">
      <send type="sce:journal" event="room.cancelled">
        <param name="room" expr="'east'"/>
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

/// Generate `doc`, a document of `work`, for Rust into `out`, and answer the
/// path of its machine, `<stem>_sm.rs`.
fn generate_machine(work: &Path, doc: &str, out: &Path) -> PathBuf {
    std::fs::create_dir_all(out).expect("create the output directory");
    let output = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["generate", "-l", "rust", "-o"])
        .arg(out)
        .arg(work.join(doc))
        .output()
        .expect("invoke sce-codegen");
    assert!(
        output.status.success(),
        "{doc} generates for Rust:\n{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let stem = doc.strip_suffix(".scxml").expect("a document is an .scxml");
    let machine = out.join(format!("{stem}_sm.rs"));
    assert!(machine.is_file(), "{} is written", machine.display());
    machine
}

/// The program [`both_machines_compile_and_a_page_of_both_facts_is_read`] runs:
/// `@ROOMS@` and `@VIEW@` are the paths of the two generated machines. It
/// prints `opened 2` when every assertion holds.
const PROGRAM: &str = r#"#![deny(warnings)]

#[path = "@ROOMS@"]
mod rooms;
#[path = "@VIEW@"]
mod view;

use std::sync::{Arc, Mutex};

use sce_rust_runtime::journal::{write_page, PageEntry};
use sce_rust_runtime::{Engine, StatePolicy};
use view::ViewHostInvokers;

/// The moment the store recorded both entries, in milliseconds.
const RECORDED_MS: u64 = 1_700_000_000_000;
const RECORDED_AT: i64 = RECORDED_MS as i64;

/// The host of the read: it keeps each start's token and the stream the
/// request names, and completes the read later.
struct Reads(Arc<Mutex<Vec<(u64, String)>>>);

impl view::ViewSceHostJournalReadInvoker for Reads {
    fn start_page(
        &mut self,
        request: view::ViewPageRequest,
        token: u64,
    ) -> Option<view::ViewPageResult> {
        self.0
            .lock()
            .expect("the starts")
            .push((token, request.stream.to_string()));
        None
    }

    fn cancel_page(&mut self, _token: u64) {}
}

fn main() {
    // The machine that says the facts, one of each schema.
    let mut rooms = Engine::new(rooms::RoomsPolicy::new());
    let Ok(()) = rooms.initialize();
    rooms.raise_external_by_name("book", "");
    rooms.step();
    rooms.raise_external_by_name("cancel", "");
    rooms.step();
    let said = rooms.take_journal();
    let schemas: Vec<&str> = said.iter().map(|entry| entry.schema).collect();
    assert_eq!(schemas, ["room.booked", "room.cancelled"], "the facts, in the order said");
    assert_eq!(said[0].valid_ms, Some(1), "a booking holds from the time it names");
    assert_eq!(said[1].valid_ms, None, "a cancellation names none");

    // The page the host answers: both entries, at positions 1 and 2.
    let entries: Vec<PageEntry<'_>> = said
        .iter()
        .zip(1u64..)
        .map(|(entry, position)| PageEntry {
            position,
            recorded_ms: RECORDED_MS,
            holds_ms: entry.valid_ms.unwrap_or(RECORDED_AT),
            origin: "",
            schema: entry.schema,
            revision: entry.revision,
            data: &entry.data,
        })
        .collect();
    let page = write_page(&entries, 64, 65536)
        .unwrap_or_else(|why| panic!("a page of two entries is written: {}", why.reason));
    let result = view::ViewPageResult {
        error: String::new(),
        horizon: 2,
        as_of_position: 2,
        entries: sce_rust_runtime::SceBytes::from_slice(&page)
            .unwrap_or_else(|_| panic!("a page of {} bytes is within its bound", page.len())),
    };

    // The completion lifted as the engine lifts it: the page opened into the
    // union, one value of each case.
    let mut payload = view::ViewPayload::default();
    if let Err(why) = <view::ViewPolicy as StatePolicy>::lift_event_payload(
        view::ViewEvent::DoneInvokePage,
        &result.wire(),
        &mut payload,
    ) {
        panic!("the completion is refused: {why:?}");
    }
    let read = match payload {
        view::ViewPayload::DoneInvokePage(read) => read,
        other => panic!("the completion is lifted into no payload of its event: {other:?}"),
    };
    assert_eq!(read.error.as_str(), "", "the read did not fail");
    assert_eq!(read.horizon, 2, "the horizon is the host's");
    let opened = read.entries.as_slice();
    assert_eq!(opened.len(), 2, "both entries are opened");
    match &opened[0].fact {
        view::ViewRoomFactsUnion::Booked(fact) => {
            assert_eq!(fact.room.as_str(), "east", "the booking's room");
            assert_eq!(fact.at, 1, "the booking's time");
        }
        other => panic!("the first entry is the booking: {other:?}"),
    }
    assert_eq!((opened[0].position, opened[0].holds_ms), (1, 1));
    match &opened[1].fact {
        view::ViewRoomFactsUnion::Cancelled(fact) => {
            assert_eq!(fact.room.as_str(), "east", "the cancellation's room");
        }
        other => panic!("the second entry is the cancellation: {other:?}"),
    }
    assert_eq!((opened[1].position, opened[1].holds_ms), (2, RECORDED_AT));
    assert_eq!(opened[1].recorded_ms, RECORDED_MS);
    assert_eq!(opened[1].origin.as_str(), "", "a session's entry has no origin");

    // The read as the machine makes it: the stream the build wrote into the
    // request, and the completion taken on `done.invoke.page`, which returns
    // the machine to `idle`. A completion the lift refused would be
    // `error.execution` and leave it reading.
    let starts: Arc<Mutex<Vec<(u64, String)>>> = Arc::default();
    let mut engine = Engine::new(view::ViewPolicy::new());
    engine.register_sce_host_journal_read_invoker(Reads(Arc::clone(&starts)));
    let Ok(()) = engine.initialize();
    engine.raise_external_by_name("go", "");
    engine.step();
    let (token, stream) = starts
        .lock()
        .expect("the starts")
        .first()
        .cloned()
        .expect("the read was started");
    assert_eq!(stream, "rooms", "the request names the stream sce:journal names");
    assert!(engine.complete_page(token, result), "the read's completion is taken");
    engine.step();
    assert!(engine.is_active(view::ViewState::Idle), "the completion was taken");
    println!("opened {}", opened.len());
}
"#;

/// docs/adr/0114, D1 and D5: the machine that says facts and the machine that
/// reads them compile against the Rust runtime with its record codec, under
/// `#![deny(warnings)]`, and a page the host writes (`write_page`) of the
/// entries the first made, one of each schema, is opened by the second into the
/// union, each entry by the case its schema names, and taken as the read's
/// completion. Red if the generated send, the derived record, the lift's
/// `open_page` or the case table does not compile or does not open the page.
#[test]
fn both_machines_compile_and_a_page_of_both_facts_is_read() {
    let Some(cargo) = toolchain::locate("cargo") else {
        toolchain::skipped(
            "both_machines_compile_and_a_page_of_both_facts_is_read: cargo not on PATH",
        );
        return;
    };
    let work = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("journal_page");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).expect("create the scratch");
    let mut documents = siblings(BOTH);
    documents.push(("rooms.scxml", rooms()));
    documents.push(("view.scxml", reader(READS_ROOMS, "")));
    for (name, text) in &documents {
        std::fs::write(work.join(name), text).expect("write a document");
    }
    let rooms_machine = generate_machine(&work, "rooms.scxml", &work.join("out_rooms"));
    let view_machine = generate_machine(&work, "view.scxml", &work.join("out_view"));

    let project = work.join("project");
    std::fs::create_dir_all(project.join("src")).expect("create the project");
    // A machine that holds typed values names `sce_forge_runtime` beside the
    // runtime (the fallible closure each value it computes is read in), so the
    // crate that holds it depends on both, the forge runtime with the heap tier
    // the runtime's record codec asks of it.
    std::fs::write(
        project.join("Cargo.toml"),
        format!(
            "[package]\nname = \"journal-page-check\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n\
             [dependencies]\nsce-rust-runtime = {{ path = {:?}, features = [\"record-cbor\"] }}\n\
             sce-forge-runtime = {{ path = {:?}, default-features = false, features = [\"alloc\"] }}\n\n[workspace]\n",
            repo_root().join("backends/rust/runtime").to_str().unwrap(),
            repo_root().join("backends/rust/forge-runtime").to_str().unwrap()
        ),
    )
    .expect("write the manifest");
    std::fs::write(
        project.join("src/main.rs"),
        PROGRAM
            .replace("@ROOMS@", rooms_machine.to_str().unwrap())
            .replace("@VIEW@", view_machine.to_str().unwrap()),
    )
    .expect("write the program");
    // The repo's own lock, so the runtime's dependencies resolve to the
    // versions already fetched.
    std::fs::copy(repo_root().join("Cargo.lock"), project.join("Cargo.lock"))
        .expect("copy the lock file");

    let manifest = project.join("Cargo.toml");
    let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("journal_page_rust_target");
    let run = common::run_cargo_offline_first(|| {
        let mut build = Command::new(&cargo);
        build
            .args(["run", "--quiet", "--manifest-path"])
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", &target_dir);
        build
    });
    let printed = String::from_utf8_lossy(&run.output.stdout).into_owned();
    assert!(
        run.output.status.success(),
        "the two machines compile and the page is read:\nstdout: {printed}\nstderr: {}",
        String::from_utf8_lossy(&run.output.stderr)
    );
    assert_eq!(printed.trim(), "opened 2", "the program ran to its end");
}
