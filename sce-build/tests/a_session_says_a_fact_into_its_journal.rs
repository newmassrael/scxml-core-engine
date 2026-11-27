// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A fact a keyed session says into its event journal (docs/adr/0114, D1 and D5;
// docs/SCE_ACCEPTED_SUBSET.md §2.15, "A fact said into the event journal"):
// `<send type="sce:journal">` of the event of an imported schema is judged where the
// document is read, lowered by Rust into an entry the engine keeps beside the event it
// raises, and refused by name on every other lane and by the Interpreter's lowering; a
// schema marks the field an entry's valid time is read from with `sce:valid-time`.

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

/// The schema of the fact the probe says, its valid time the field `at`.
const BOOKED: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="room_booked" sce:event-name="room.booked">
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
    <data id="at" sce:type="int64" sce:direction="in" sce:valid-time="from"/>
    <data id="note" sce:type="optional&lt;string[32]&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A statechart under `datamodel` that imports [`BOOKED`], whose `book` transition
/// holds `body` and whose `room.booked` transition applies the fact.
fn statechart(datamodel: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}" name="probe">
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <datamodel>
    <data id="booked" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="book" type="internal">
{body}
    </transition>
    <transition event="room.booked" type="internal">
      <assign location="booked" expr="booked + 1"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// A fact of every field the schema does not make optional.
const SAYS: &str = r#"      <send type="sce:journal" event="room.booked">
        <param name="room" expr="'east'"/>
        <param name="at" expr="1700000000000"/>
      </send>"#;

/// Run `sce-codegen <args...> probe.scxml` with JSON diagnostics, `schema` written
/// beside the document as `room_booked.scxml`, and return `(exit_ok, output, every
/// file the run wrote under `-o`, joined)`. `out` asks for `-o` to a directory of
/// the run's own, given after `args`.
fn run(args: &[&str], out: bool, doc: &str, schema: &str) -> (bool, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("room_booked.scxml"), schema).expect("write schema");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
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

fn check(args: &[&str], doc: &str) -> (bool, String) {
    let (ok, out, _) = run(args, false, doc, BOOKED);
    (ok, out)
}

/// [`BOOKED`] without its optional field, so that what a lane refuses of the
/// document is the fact and nothing it holds no lowering of yet.
fn plain() -> String {
    let plain = BOOKED.replace(
        "    <data id=\"note\" sce:type=\"optional&lt;string[32]&gt;\" sce:direction=\"in\"/>\n",
        "",
    );
    assert_ne!(plain, BOOKED, "the optional field is taken out");
    plain
}

/// docs/adr/0114, D1: the Rust machine makes the entry from the event's data by
/// the record's description, its valid time read from the field the schema marks,
/// keeps it on the engine, and raises the event. Red if the send is lowered as any
/// other send, or the description or the valid time is not written onto it.
#[test]
fn a_fact_is_made_an_entry_and_taken_in_the_same_macrostep() {
    let doc = statechart("sce-static", SAYS);
    let (ok, out) = check(&["check"], &doc);
    assert!(ok, "the judge takes a fact of an imported schema:\n{out}");
    let (ok, out, written) = run(&["generate", "-l", "rust"], true, &doc, BOOKED);
    assert!(ok, "the Rust machine says the fact:\n{out}");
    for said in [
        "::sce_rust_runtime::journal::entry(",
        "\"room.booked\"",
        "Some(\"at\")",
        "engine.record_journal_entry(entry);",
    ] {
        assert!(
            written.contains(said),
            "the generated machine holds `{said}`:\n{written}"
        );
    }
}

/// docs/adr/0114, Scope: only the Rust host core keeps a journal, so every other
/// lane, and the Interpreter's lowering, refuses the machine by name. Red if a lane
/// lowers the send as an event to the machine itself.
#[test]
fn every_lane_but_rust_refuses_a_fact_by_name() {
    let doc = statechart("sce-static", SAYS);
    let schema = plain();
    for language in Language::ALL.iter().filter(|l| **l != Language::Rust) {
        let lane = HostLane::Generated(*language).wire_name();
        let (ok, out, _) = run(&["check", "-l", lane], false, &doc, &schema);
        assert!(!ok, "{lane} keeps no journal:\n{out}");
        assert!(
            out.contains("generate/unsupported-feature") && out.contains("sce:journal"),
            "{lane} refuses the send by name:\n{out}"
        );
    }
    let (ok, out, _) = run(&["lower"], false, &doc, &schema);
    assert!(
        !ok && out.contains("the interpreter lane keeps no event journal"),
        "the Interpreter's lowering refuses the send by name:\n{out}"
    );
}

/// Assert `doc` is refused as `scxml/static-datamodel-rule` with `said` in its
/// reason.
fn assert_refused(doc: &str, said: &str) {
    let (ok, out) = check(&["check"], doc);
    assert!(
        !ok && out.contains("\"code\":\"scxml/static-datamodel-rule\"") && out.contains(said),
        "refused with `{said}`:\n{out}"
    );
}

/// docs/adr/0114, D1 and D2: an entry is the session's, made in the macrostep that
/// says it, never cancelled, and a record of its schema whole. Each send that could
/// be none of these is refused on the send.
#[test]
fn what_an_entry_cannot_be_is_refused_on_the_send() {
    let with = |attrs: &str, params: &str| {
        format!("      <send type=\"sce:journal\" {attrs}>\n{params}\n      </send>")
    };
    let fields =
        "        <param name=\"room\" expr=\"'east'\"/>\n        <param name=\"at\" expr=\"1\"/>";
    for (attrs, params, said) in [
        (
            r##"event="room.booked" target="#_internal""##,
            fields,
            "names no target",
        ),
        (
            r#"event="room.booked" delay="1s""#,
            fields,
            "it is not delayed",
        ),
        (
            r#"event="room.booked" id="b""#,
            fields,
            "takes no id",
        ),
        (
            r#"eventexpr="'room.booked'""#,
            fields,
            "names its event by `event`",
        ),
        (
            r#"event="room.cancelled""#,
            fields,
            "the event of no event schema the document imports",
        ),
        (
            r#"event="room.booked""#,
            "        <param name=\"at\" expr=\"1\"/>",
            "the field `room` of `room.booked` is given by no <param>",
        ),
        (
            r#"event="room.booked""#,
            "        <param name=\"room\" expr=\"'east'\"/>\n        <param name=\"room\" expr=\"'west'\"/>\n        <param name=\"at\" expr=\"1\"/>",
            "is given by two <param>s",
        ),
    ] {
        assert_refused(&statechart("sce-static", &with(attrs, params)), said);
    }
    assert_refused(
        &statechart("ecmascript", SAYS),
        "only datamodel=\\\"sce-static\\\" types",
    );
}

/// docs/adr/0114, D5: a schema marks one `int64` field of its first revision its
/// valid time, by the one value `from`. Red if a mark is taken on another field.
#[test]
fn a_schema_marks_one_int64_field_its_valid_time() {
    let schema = |fields: &str, revision: &str| {
        format!(
            r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="room_booked" sce:event-name="room.booked"{revision}>
  <datamodel>
    <data id="room" sce:type="string[16]" sce:direction="in"/>
{fields}
  </datamodel>
</scxml>
"#
        )
    };
    let doc = statechart("sce-static", SAYS);
    for (fields, revision, code, said) in [
        (
            r#"    <data id="at" sce:type="int64" sce:direction="in" sce:valid-time="to"/>"#,
            "",
            "validation/invalid-attribute",
            "sce:valid-time",
        ),
        (
            r#"    <data id="at" sce:type="uint32" sce:direction="in" sce:valid-time="from"/>"#,
            "",
            "validation/attribute-rule-violated",
            "an `int64`",
        ),
        (
            r#"    <data id="at" sce:type="int64" sce:direction="in" sce:valid-time="from"/>
    <data id="until" sce:type="int64" sce:direction="in" sce:valid-time="from"/>"#,
            "",
            "validation/attribute-rule-violated",
            "the field 'at' is marked already",
        ),
        (
            r#"    <data id="at" sce:type="int64" sce:direction="in"/>
    <data id="since" sce:type="int64" sce:direction="in" sce:since="2" sce:default="0" sce:valid-time="from"/>"#,
            r#" sce:revision="2""#,
            "validation/attribute-rule-violated",
            "revision 2 added this one",
        ),
    ] {
        let (ok, out, _) = run(&["check"], false, &doc, &schema(fields, revision));
        assert!(
            !ok && out.contains(&format!("\"code\":\"{code}\"")) && out.contains(said),
            "{fields}: refused as {code} with `{said}`:\n{out}"
        );
    }
}

/// docs/adr/0114, D1: a `no_std` Rust machine carries neither the record codec nor
/// the host core's write, so a fact is refused by name there.
#[test]
fn a_no_std_machine_says_no_fact() {
    let (ok, out, _) = run(
        &["generate", "-l", "rust", "--no-std"],
        true,
        &statechart("sce-static", SAYS),
        &plain(),
    );
    assert!(
        !ok && out.contains("a no_std Rust machine keeps no event journal"),
        "{out}"
    );
}

/// A statechart that imports [`BOOKED`] and holds `booked` to the journal's
/// facts, with `elsewhere` written in its state before its transitions and `book`
/// in its `book` transition; its `room.booked` transition applies the fact.
fn held(book: &str, elsewhere: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="room_booked.scxml" as="RoomBooked"/>
  <datamodel>
    <data id="booked" sce:type="uint32" expr="0" sce:from="journal"/>
  </datamodel>
  <state id="s">
{elsewhere}
    <transition event="book" type="internal">
{book}
    </transition>
    <transition event="room.booked" type="internal">
      <assign location="booked" expr="booked + 1"/>
    </transition>
  </state>
</scxml>
"#
    )
}

/// docs/adr/0114, D1: a value held to the journal's facts takes its initial value
/// and is written by the transition that applies a fact, on every lane that
/// judges it. Red if the transition on an entry's event is refused.
#[test]
fn a_value_held_to_the_facts_is_written_where_a_fact_is_applied() {
    let doc = held(SAYS, "");
    for args in [&["check"][..], &["check", "-l", "rust"][..]] {
        let (ok, out) = check(args, &doc);
        assert!(ok, "{args:?} takes the fold of the facts:\n{out}");
    }
}

/// docs/adr/0114, D1: a value held to the journal's facts is written nowhere a
/// fact is not applied: a transition on an event of no schema, one that takes no
/// event, one whose descriptor matches more than an entry's event, and an
/// `<onentry>`. Each is `validation/journal-value-assigned`, naming where it
/// stands. Red if any of the walks is dropped.
#[test]
fn a_value_held_to_the_facts_is_written_nowhere_else() {
    let assign = r#"      <assign location="booked" expr="booked + 1"/>"#;
    for (doc, site) in [
        (held(assign, ""), "the transition of state 's' on 'book'"),
        (
            held(
                SAYS,
                r#"    <transition cond="booked &lt; 1" type="internal"><assign location="booked" expr="1"/></transition>"#,
            ),
            "a transition of state 's' that takes no event",
        ),
        (
            held(
                SAYS,
                r#"    <transition event="room.*" type="internal"><assign location="booked" expr="2"/></transition>"#,
            ),
            "the transition of state 's' on 'room.*'",
        ),
        (
            held(
                SAYS,
                r#"    <onentry><assign location="booked" expr="1"/></onentry>"#,
            ),
            "the <onentry> of state 's'",
        ),
    ] {
        let (ok, out) = check(&["check"], &doc);
        assert!(
            !ok && out.contains("\"code\":\"validation/journal-value-assigned\"")
                && out.contains(site),
            "refused in {site}:\n{out}"
        );
    }
}

/// docs/adr/0114, D1: `sce:from` names the deployment or the journal, and nothing
/// else.
#[test]
fn a_value_comes_from_the_deployment_or_the_journal() {
    let doc = held(SAYS, "").replace(r#"sce:from="journal""#, r#"sce:from="ledger""#);
    let (ok, out) = check(&["check"], &doc);
    assert!(
        !ok && out.contains("`journal`, a fold"),
        "an unknown source is refused, naming both:\n{out}"
    );
}

/// Run `sce-codegen generate -l rust --deploy` on `doc` as `probe.scxml` with
/// `deploy` as deploy.yaml, [`BOOKED`] and `others` beside it; return
/// `(exit_ok, output)`.
fn generate_deployed(doc: &str, deploy: &str, others: &[(&str, &str)]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("room_booked.scxml"), BOOKED).expect("write schema");
    for (name, text) in others {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
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

/// A deployment whose keyed pool `probe` takes `lines`, beside a pool `rooms`
/// that keeps a journal.
fn pooled(lines: &str) -> String {
    format!(
        r#"version: "1.0"
topology:
  cloud:
    machines:
      rooms:
        source: rooms.scxml
        sessions: {{ key_from: data.room, idle_ms: 1000 }}
        journal: true
      probe:
        source: probe.scxml
        sessions: {{ key_from: data.room, idle_ms: 1000 }}
{lines}"#
    )
}

/// docs/adr/0114, D1: where the machine is generated with its deployment, a
/// document that says facts runs on a pool that keeps a journal, and a pool that
/// keeps one runs a document that says them. Red if either side is not read.
#[test]
fn a_document_and_its_deployment_agree_about_its_journal() {
    let rooms = [("rooms.scxml", statechart("sce-static", SAYS))];
    let rooms: Vec<(&str, &str)> = rooms.iter().map(|(n, t)| (*n, t.as_str())).collect();
    for (doc, lines, said) in [
        (
            statechart("sce-static", SAYS),
            "",
            "declare `journal: true`",
        ),
        (
            statechart("sce-static", ""),
            "        journal: true\n",
            "says no fact into it",
        ),
    ] {
        let (ok, out) = generate_deployed(&doc, &pooled(lines), &rooms);
        assert!(
            !ok && out.contains("\"code\":\"mesh/codegen-journal-mismatch\"") && out.contains(said),
            "refused with `{said}`:\n{out}"
        );
    }
}

/// docs/adr/0114, D4: a pool that follows a journal takes its entries in as it
/// takes requests, keyed by the field `key_from` names in each entry's schema,
/// so a pool that serves no request and follows one is a view of it. Red if the
/// followed entries are not counted, which refuses the pool as serving nothing.
#[test]
fn a_follower_is_keyed_by_the_entries_it_follows() {
    let rooms = statechart("sce-static", SAYS);
    let (_, out) = generate_deployed(
        &statechart("sce-static", ""),
        &pooled("        follows: [rooms]\n"),
        &[("rooms.scxml", rooms.as_str())],
    );
    assert!(
        !out.contains("mesh/deploy-sessions-key-not-in-request")
            && !out.contains("mesh/codegen-journal-mismatch"),
        "the view is keyed by the entries it follows:\n{out}"
    );
}
