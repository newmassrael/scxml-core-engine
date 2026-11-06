// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The saved-state wire surface (`schemas/sce-saved-state.v1.schema.json`,
// `schemas/sce-saved-state.v2.schema.json` and
// `schemas/sce-saved-state.v3.schema.json`, SCE Accepted Subset §2.15) held to
// its instances.
//
// The shared fixtures under `tests/fixtures/static_datamodel/saved/` are the
// text every backend must save after the same run and must restore from. So
// they are the surface's instances, and each one must be what the schema says
// a saved state is. Each directory is swept rather than listed, so a fixture
// added for a new backend or machine is held to the schema without an edit
// here.
//
// Format 3 (docs/adr/0090, section 7) is format 2's tree of sessions with an
// event's origin held as one pair: the texts directly under `saved/` are
// format 3, the format every backend writes, and are held to the format-3
// schema and to the one text form the ADR fixes. The format-1 texts every
// backend saved before trees are in `saved/format1/` and are held to the
// format-1 schema. The format-2 reading fixtures are in `saved/format2/`, held
// to the format-2 schema, and beside them in `saved/format2/read_as/` the
// format-3 text each one is read as, which is held to the table of section 7
// applied to its fixture. Hand-written trees below exercise every member
// formats 2 and 3 add.

use std::path::{Path, PathBuf};

use sce_rust_runtime::json;

/// The `.json` files directly under each of `dirs`, sorted.
fn instances(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut fixtures: Vec<PathBuf> = dirs
        .iter()
        .flat_map(|dir| {
            std::fs::read_dir(dir)
                .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
                .map(|entry| entry.expect("directory entry").path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect::<Vec<_>>()
        })
        .collect();
    fixtures.sort();
    fixtures
}

/// Hold every file of `fixtures` to `validator`.
fn each_is_a_saved_state(validator: &jsonschema::JSONSchema, fixtures: Vec<PathBuf>) {
    for path in fixtures {
        let text = std::fs::read_to_string(&path).expect("read fixture");
        let instance: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
        let found = errors(validator, &instance);
        assert!(
            found.is_empty(),
            "{} is not a saved state:\n{}",
            path.display(),
            found.join("\n")
        );
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn compile(text: &str) -> jsonschema::JSONSchema {
    let schema: serde_json::Value =
        serde_json::from_str(text).expect("the saved-state schema is JSON");
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the saved-state schema compiles as draft-07")
}

fn validator() -> jsonschema::JSONSchema {
    compile(include_str!("../../schemas/sce-saved-state.v1.schema.json"))
}

fn validator_v2() -> jsonschema::JSONSchema {
    compile(include_str!("../../schemas/sce-saved-state.v2.schema.json"))
}

fn validator_v3() -> jsonschema::JSONSchema {
    compile(include_str!("../../schemas/sce-saved-state.v3.schema.json"))
}

/// The directory of the shared saved states of the static data model.
fn shared_saved() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/static_datamodel/saved")
}

/// The format-2 reading fixtures (docs/adr/0090, section 7).
fn format_2_readings() -> PathBuf {
    shared_saved().join("format2")
}

/// The format-3 text each format-2 reading fixture is read as, under its name.
fn format_2_read_as() -> PathBuf {
    format_2_readings().join("read_as")
}

/// The directories whose texts are format 3: the shared instances every
/// backend writes, and what each format-2 reading fixture is read as.
fn format_3_directories() -> [PathBuf; 3] {
    [
        shared_saved(),
        repo_root().join("sce-build/tests/fixtures/host_processor/saved"),
        format_2_read_as(),
    ]
}

/// `path`'s text without the line end a file closes with: the text a backend
/// writes and compares.
fn saved_text(path: &Path) -> String {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    text.strip_suffix('\n').unwrap_or(&text).to_string()
}

/// `text` read as JSON with every object's members in the order written.
fn ordered(text: &str, what: &Path) -> json::Value {
    json::parse(text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", what.display()))
}

fn errors(validator: &jsonschema::JSONSchema, instance: &serde_json::Value) -> Vec<String> {
    match validator.validate(instance) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .map(|e| format!("{e} at {}", e.instance_path))
            .collect(),
    }
}

#[test]
fn every_shared_saved_state_fixture_is_a_saved_state() {
    // The shared instances of the static data model and of the host-run
    // invocation fixture, which lives with the other host fixtures.
    let format1 = [
        repo_root().join("sce-build/tests/fixtures/static_datamodel/saved/format1"),
        repo_root().join("sce-build/tests/fixtures/host_processor/saved/format1"),
    ];
    let format2 = [format_2_readings()];
    let format3 = format_3_directories();
    let (first, second, third) = (
        instances(&format1),
        instances(&format2),
        instances(&format3),
    );
    // A sweep over nothing certifies nothing: the sixteen texts every backend
    // saved before trees moved to `format1/`, the four format-2 reading
    // fixtures, and the fifteen shared format-3 instances with the four texts
    // those fixtures are read as, when these floors were set.
    assert!(
        first.len() >= 16,
        "swept only {} format-1 fixture(s) in {format1:?}",
        first.len()
    );
    assert!(
        second.len() >= 4,
        "swept only {} format-2 fixture(s) in {format2:?}",
        second.len()
    );
    assert!(
        third.len() >= 19,
        "swept only {} format-3 fixture(s) in {format3:?}",
        third.len()
    );
    each_is_a_saved_state(&validator(), first);
    each_is_a_saved_state(&validator_v2(), second);
    each_is_a_saved_state(&validator_v3(), third);
}

/// The canonical type of the SCXML Event I/O Processor (docs/adr/0090,
/// section 4).
const SCXML_PROCESSOR: &str = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";

/// What the SCXML processor's address of a session starts with.
const SCXML_LOCATION: &str = "sce://scxml/";

/// The SCXML processor's address of `session_id` (docs/adr/0090, section 7,
/// "Text form"): the prefix and the id's UTF-8 bytes, each byte outside
/// `A-Z a-z 0-9 - . _ ~` written as `%` and two uppercase hexadecimal digits.
/// Written here from the ADR's words, not taken from a runtime, so the
/// instances are held to the format and not to one backend's reading of it.
fn scxml_location(session_id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut location = String::from(SCXML_LOCATION);
    for byte in session_id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            location.push(char::from(byte));
        } else {
            location.push('%');
            location.push(char::from(HEX[usize::from(byte >> 4)]));
            location.push(char::from(HEX[usize::from(byte & 0x0F)]));
        }
    }
    location
}

/// `text` with every `%` and two hexadecimal digits read as the byte they
/// name, and any other `%` kept as it is.
fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let hex = |b: u8| char::from(b).to_digit(16);
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = if bytes[at] == b'%' && at + 2 < bytes.len() {
            hex(bytes[at + 1]).zip(hex(bytes[at + 2]))
        } else {
            None
        };
        match escaped {
            Some((high, low)) => {
                decoded.push(u8::try_from(high * 16 + low).expect("two hex digits are a byte"));
                at += 3;
            }
            None => {
                decoded.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8(decoded).expect("a location names its session in UTF-8")
}

/// The session a value that starts with the SCXML processor's prefix names,
/// or `None` for any other value and for the prefix alone.
fn located_session(origin: &str) -> Option<String> {
    origin
        .strip_prefix(SCXML_LOCATION)
        .filter(|id| !id.is_empty())
        .map(percent_decoded)
}

fn text(value: &str) -> json::Value {
    json::Value::Text(value.to_string())
}

/// An origin as format 3 writes it: `processor` first, then `address`.
fn pair(processor: &str, address: &str) -> json::Value {
    json::Value::Object(vec![
        ("processor".to_string(), text(processor)),
        ("address".to_string(), text(address)),
    ])
}

/// The `origin` and `sender` of an event the session `sender` sent.
fn sent_by(sender: &str) -> (json::Value, json::Value) {
    (pair(SCXML_PROCESSOR, &scxml_location(sender)), text(sender))
}

/// The `origin` and `sender` a format-2 `origin` and `origintype` are read as:
/// the table of docs/adr/0090, section 7, row by row.
fn read_format_2_origin(origin: &str, origin_type: &str) -> (json::Value, json::Value) {
    // Row 1: an empty address is none, whatever type was saved beside it.
    if origin.is_empty() {
        return (json::Value::Null, json::Value::Null);
    }
    // Row 3, before row 4 whatever the type: a location names its session.
    if let Some(session) = located_session(origin) {
        return sent_by(&session);
    }
    // An empty type beside an address is the SCXML processor's.
    let processor = if origin_type.is_empty() {
        SCXML_PROCESSOR
    } else {
        origin_type
    };
    // Row 2: no `://`, and the SCXML type, is a session's id.
    if !origin.contains("://") && processor == SCXML_PROCESSOR {
        return sent_by(origin);
    }
    // Row 4: the pair as saved, sent by no session.
    (pair(processor, origin), json::Value::Null)
}

/// The text member `key` of `record`.
fn member_text<'v>(record: &'v json::Value, key: &str) -> &'v str {
    match record.member(key) {
        Some(json::Value::Text(value)) => value,
        other => panic!("'{key}' is not a text: {other:?}"),
    }
}

/// The members of `value`, an object.
fn members(value: &json::Value) -> &[(String, json::Value)] {
    match value {
        json::Value::Object(members) => members,
        other => panic!("not an object: {other:?}"),
    }
}

/// A format-2 queued event as format 3 writes it: `origin` and `origintype`
/// become `origin` and `sender` in their place.
fn read_format_2_event(event: &json::Value) -> json::Value {
    let (origin, sender) = read_format_2_origin(
        member_text(event, "origin"),
        member_text(event, "origintype"),
    );
    json::Value::Object(
        members(event)
            .iter()
            .map(|(key, value)| match key.as_str() {
                "origin" => (key.clone(), origin.clone()),
                "origintype" => ("sender".to_string(), sender.clone()),
                _ => (key.clone(), value.clone()),
            })
            .collect(),
    )
}

/// The sender a format-2 `raise`, `parent` or `child` act of the session
/// `holder` names by its stored `origin` (docs/adr/0090, section 7): the
/// holder for an empty one, the session a location names, and a bare id as it
/// is. `None` for the prefix `sce://scxml/` alone, which names no session and
/// is no id: a reader refuses the text.
fn format_2_act_sender(stored: &str, holder: &str) -> Option<String> {
    if stored.is_empty() {
        return Some(holder.to_string());
    }
    if let Some(session) = located_session(stored) {
        return Some(session);
    }
    (stored != SCXML_LOCATION).then(|| stored.to_string())
}

/// A format-2 waiting send of the session `holder` as format 3 writes it. Its
/// stored `origin` is the sending session's id, or that session's location as
/// C11 stored it, and an empty one is the holder's own send (docs/adr/0090,
/// section 7); an `internal` act's event is internal and carries none; a
/// `host` act carries neither member in either format.
fn read_format_2_act(act: &json::Value, holder: &str) -> json::Value {
    let kind = member_text(act, "act");
    if kind == "host" {
        return act.clone();
    }
    let (origin, sender) = if kind == "internal" {
        (json::Value::Null, json::Value::Null)
    } else {
        let stored = member_text(act, "origin");
        let sender = format_2_act_sender(stored, holder)
            .unwrap_or_else(|| panic!("a reader refuses the act stored as {stored:?}"));
        sent_by(&sender)
    };
    let mut read = Vec::new();
    for (key, value) in members(act) {
        if key == "origin" {
            read.push(("origin".to_string(), origin.clone()));
            read.push(("sender".to_string(), sender.clone()));
        } else {
            read.push((key.clone(), value.clone()));
        }
    }
    json::Value::Object(read)
}

fn read_format_2_items(items: &json::Value, read: fn(&json::Value) -> json::Value) -> json::Value {
    match items {
        json::Value::Array(items) => json::Value::Array(items.iter().map(read).collect()),
        other => panic!("not an array: {other:?}"),
    }
}

/// A running child of a format-2 session, its own session read as format 3.
fn read_format_2_child(child: &json::Value) -> json::Value {
    json::Value::Object(
        members(child)
            .iter()
            .map(|(key, value)| match key.as_str() {
                "session" => (key.clone(), read_format_2_session(value)),
                _ => (key.clone(), value.clone()),
            })
            .collect(),
    )
}

/// A format-2 session, the root with its `format` or a child without it, as
/// format 3 writes it: every member as it was but the format, the queued
/// events, the waiting sends and each running child.
fn read_format_2_session(session: &json::Value) -> json::Value {
    let holder = member_text(session, "sessionid");
    json::Value::Object(
        members(session)
            .iter()
            .map(|(key, value)| {
                let read = match key.as_str() {
                    "format" => json::Value::Number("3".to_string()),
                    "external" => read_format_2_items(value, read_format_2_event),
                    "pending" => match value {
                        json::Value::Array(acts) => json::Value::Array(
                            acts.iter()
                                .map(|act| read_format_2_act(act, holder))
                                .collect(),
                        ),
                        other => panic!("not an array: {other:?}"),
                    },
                    "invokes" => read_format_2_items(value, read_format_2_child),
                    _ => value.clone(),
                };
                (key.clone(), read)
            })
            .collect(),
    )
}

#[test]
fn each_format_2_reading_fixture_is_read_as_the_table_of_section_7_says() {
    // docs/adr/0090, section 7: every backend restores each format-2 reading
    // fixture and saves at once the format-3 text `read_as/` holds under its
    // name. That text is held here to the table, applied row by row to the
    // fixture, byte for byte, so the backends are held to the table and not to
    // a text written by hand.
    fn names(paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|p| {
                p.file_name()
                    .expect("a file")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
    let fixtures = instances(&[format_2_readings()]);
    let readings = instances(&[format_2_read_as()]);
    assert_eq!(
        names(&fixtures),
        names(&readings),
        "every reading fixture has the text it is read as, and only those"
    );
    assert!(
        fixtures.len() >= 4,
        "swept only {} fixture(s)",
        fixtures.len()
    );
    for (fixture, reading) in fixtures.iter().zip(&readings) {
        let stored = ordered(&saved_text(fixture), fixture);
        assert_eq!(
            stored.member("format"),
            Some(&json::Value::Number("2".to_string())),
            "{} is a format-2 text",
            fixture.display()
        );
        assert_eq!(
            json::write(&read_format_2_session(&stored)),
            saved_text(reading),
            "{} is not what {} is read as",
            reading.display(),
            fixture.display()
        );
    }
    // The tree as C11 stored it and as the other backends stored it mean one
    // tree, and the timers one timers: each reading is the shared instance.
    for (reading, shared) in [
        (
            "static_invoke_tree_working.json",
            "static_invoke_tree_working.json",
        ),
        (
            "static_invoke_tree_working_c11.json",
            "static_invoke_tree_working.json",
        ),
        ("static_timers.json", "static_timers.json"),
    ] {
        assert_eq!(
            saved_text(&format_2_read_as().join(reading)),
            saved_text(&shared_saved().join(shared)),
            "{reading} is read as the shared {shared}"
        );
    }
}

#[test]
fn the_scxml_prefix_alone_names_no_session_in_a_format_2_text() {
    // docs/adr/0090, section 7. A queued event stored with the prefix and no id
    // is row 4: row 3 needs an id after the prefix, and row 2 a text without
    // `://`. It is read as the pair it stores, sent by no session (the root
    // event `question` of `saved/format2/static_invoke_tree_working_origins.json`).
    assert_eq!(
        read_format_2_origin(SCXML_LOCATION, SCXML_PROCESSOR),
        (pair(SCXML_PROCESSOR, SCXML_LOCATION), json::Value::Null)
    );
    assert_eq!(
        read_format_2_origin(SCXML_LOCATION, ""),
        (pair(SCXML_PROCESSOR, SCXML_LOCATION), json::Value::Null)
    );
    // A session's own waiting send must name a session: the prefix alone names
    // none and is no id, so a reader refuses it rather than take it as a sender.
    assert_eq!(format_2_act_sender(SCXML_LOCATION, "fixture-root"), None);
    // The neighbours it must not be confused with.
    assert_eq!(
        format_2_act_sender("", "fixture-root").as_deref(),
        Some("fixture-root")
    );
    assert_eq!(
        format_2_act_sender("sce://scxml/peer%3A1", "fixture-root").as_deref(),
        Some("peer:1")
    );
    assert_eq!(
        format_2_act_sender("peer:1", "fixture-root").as_deref(),
        Some("peer:1")
    );
}

/// The members of a queued event, in the order every backend writes them.
const EVENT_MEMBERS: [&str; 8] = [
    "name",
    "data",
    "type",
    "sendid",
    "origin",
    "sender",
    "invokeid",
    "hostinvoketoken",
];

/// The members of a waiting send of `kind`, in the order every backend writes
/// them.
fn act_members(kind: &str) -> &'static [&'static str] {
    match kind {
        "host" => &[
            "due", "act", "type", "event", "target", "content", "params", "sendid", "data",
            "invokeid",
        ],
        "child" => &[
            "due", "act", "invokeid", "event", "data", "sendid", "origin", "sender",
        ],
        _ => &["due", "act", "event", "data", "sendid", "origin", "sender"],
    }
}

fn keys(record: &json::Value) -> Vec<&str> {
    members(record).iter().map(|(k, _)| k.as_str()).collect()
}

/// Hold one origin to the text form: `null`, or the pair with `processor`
/// first and `address` second, and never a sender beside no pair. A session's
/// own waiting send (`own_send`) carries the SCXML pair of its sender, byte for
/// byte; a queued event's pair is the reply address the delivering processor
/// wrote, which no rule ties to its sender (docs/adr/0090, section 1).
fn holds_one_origin(record: &json::Value, what: &str, own_send: bool) {
    match (record.member("origin"), record.member("sender")) {
        (Some(json::Value::Null), Some(json::Value::Null)) => {}
        (Some(origin), Some(sender)) if matches!(origin, json::Value::Object(_)) => {
            assert_eq!(keys(origin), ["processor", "address"], "{what}: the pair");
            if let json::Value::Text(sender) = sender {
                if own_send {
                    assert_eq!(
                        origin,
                        &sent_by(sender).0,
                        "{what}: a session's own send carries the location it publishes"
                    );
                }
            } else {
                assert_eq!(sender, &json::Value::Null, "{what}: the sender");
            }
        }
        other => panic!("{what}: an origin and a sender written as {other:?}"),
    }
}

/// Every `raise`, `parent` and `child` act of `session` and of each session
/// below it whose `sender` is not that session's own `sessionid`, as the
/// reason a reader refuses it (docs/adr/0090, section 7, "Text form"): a
/// session's waiting send is its own send. The schema cannot compare two
/// members, so this is the check.
fn acts_sent_by_another_session(session: &json::Value, what: &str) -> Vec<String> {
    let own = member_text(session, "sessionid");
    let mut found = Vec::new();
    if let Some(json::Value::Array(acts)) = session.member("pending") {
        for (i, act) in acts.iter().enumerate() {
            let kind = member_text(act, "act");
            if !matches!(kind, "raise" | "parent" | "child") {
                continue;
            }
            let sender = match act.member("sender") {
                Some(json::Value::Text(sender)) if sender == own => continue,
                Some(json::Value::Text(sender)) => format!("'{sender}'"),
                other => format!("{other:?}"),
            };
            found.push(format!(
                "'{what}pending[{i}].sender' is {sender}, and this session is '{own}': a {kind} \
                 act is this session's own send"
            ));
        }
    }
    if let Some(json::Value::Array(children)) = session.member("invokes") {
        for (i, child) in children.iter().enumerate() {
            let child_session = child.member("session").expect("a child's session");
            found.extend(acts_sent_by_another_session(
                child_session,
                &format!("{what}invokes[{i}].session."),
            ));
        }
    }
    found
}

#[test]
fn a_session_act_sent_by_another_session_is_refused() {
    // The shared tree holds a `child` act in the root and a `parent` act in the
    // child, each its own session's send.
    let path = shared_saved().join("static_invoke_tree_working.json");
    let written = saved_text(&path);
    let found = acts_sent_by_another_session(&ordered(&written, &path), "");
    assert!(found.is_empty(), "{}", found.join("\n"));
    // The root's act given the child as its sender, with the child's own pair
    // beside it: every other rule of the text form holds, and this one does not.
    let root = "fixture-root";
    let child = "fixture-root.working.2.worker";
    let own = format!(
        "\"sendid\":\"poke\",\"origin\":{{\"processor\":\"{SCXML_PROCESSOR}\",\"address\":\"{}\"}},\"sender\":\"{root}\"}}",
        scxml_location(root)
    );
    let foreign = format!(
        "\"sendid\":\"poke\",\"origin\":{{\"processor\":\"{SCXML_PROCESSOR}\",\"address\":\"{}\"}},\"sender\":\"{child}\"}}",
        scxml_location(child)
    );
    assert_eq!(
        written.matches(&own).count(),
        1,
        "the root's act as written"
    );
    let changed = written.replace(&own, &foreign);
    let found = acts_sent_by_another_session(&ordered(&changed, &path), "");
    assert_eq!(
        found,
        [format!(
            "'pending[0].sender' is '{child}', and this session is '{root}': a child act is \
             this session's own send"
        )]
    );
}

/// Hold `session` and every session below it to the text form.
fn holds_the_text_form(session: &json::Value, what: &str) {
    if let Some(json::Value::Array(events)) = session.member("external") {
        for (i, event) in events.iter().enumerate() {
            let at = format!("{what}external[{i}]");
            assert_eq!(keys(event), EVENT_MEMBERS, "{at}: its members");
            holds_one_origin(event, &at, false);
        }
    }
    if let Some(json::Value::Array(acts)) = session.member("pending") {
        for (i, act) in acts.iter().enumerate() {
            let at = format!("{what}pending[{i}]");
            let kind = member_text(act, "act");
            assert_eq!(keys(act), act_members(kind), "{at}: its members");
            if kind == "internal" {
                assert_eq!(act.member("origin"), Some(&json::Value::Null), "{at}");
                assert_eq!(act.member("sender"), Some(&json::Value::Null), "{at}");
            } else if kind != "host" {
                assert!(
                    matches!(act.member("sender"), Some(json::Value::Text(_))),
                    "{at}: a session's own send names it"
                );
                holds_one_origin(act, &at, true);
            }
        }
    }
    if let Some(json::Value::Array(children)) = session.member("invokes") {
        for (i, child) in children.iter().enumerate() {
            let child_session = child.member("session").expect("a child's session");
            assert!(
                child_session.member("format").is_none(),
                "{what}invokes[{i}].session states the format again"
            );
            holds_the_text_form(child_session, &format!("{what}invokes[{i}].session."));
        }
    }
}

#[test]
fn every_format_3_instance_is_written_in_the_one_text_form() {
    // docs/adr/0090, section 7, "Text form": the bytes every backend writes.
    // The schema cannot say the order of members or the spelling of the text,
    // so each format-3 instance is held to them here.
    let paths = instances(&format_3_directories());
    assert!(paths.len() >= 19, "swept only {} instance(s)", paths.len());
    let mut origins = 0usize;
    for path in paths {
        let written = saved_text(&path);
        let state = ordered(&written, &path);
        let what = path.display().to_string();
        // Compact, each text escaped by the one rule, every member where it
        // was written: what the runtime's writer writes back is the file.
        assert_eq!(json::write(&state), written, "{what} is not compact JSON");
        assert_eq!(
            members(&state).first().map(|(k, v)| (k.as_str(), v)),
            Some(("format", &json::Value::Number("3".to_string()))),
            "{what}: `format` is 3, the root's first member"
        );
        assert!(
            !written.contains("\"origintype\""),
            "{what} keeps format 2's origintype"
        );
        holds_the_text_form(&state, &format!("{what}: "));
        let foreign = acts_sent_by_another_session(&state, "");
        assert!(foreign.is_empty(), "{what}: {}", foreign.join("\n"));
        origins += written.matches("\"origin\":{").count();
    }
    // The sweep reached pairs, not only texts that hold none.
    assert!(
        origins >= 20,
        "only {origins} pair(s) were held to the form"
    );
}

#[test]
fn the_schema_refuses_what_no_backend_writes() {
    // The control that keeps the sweep above from passing a schema that
    // admits anything: each instance breaks one rule a reader relies on.
    let validator = validator();
    let good = serde_json::json!({
        "format": 1,
        "shape": "0".repeat(64),
        "configuration": ["s"],
        "current": "s",
        "variables": {"count": 1},
        "history": {"mode": ["slow"], "zone": ["a", "b"]},
        "pending": [
            {
                "due": "1700000005000", "act": "raise", "event": "timeout",
                "data": "", "sendid": "timer", "origin": ""
            },
            {
                "due": "1700000006000", "act": "internal", "event": "inner",
                "data": "{\"n\":2}", "sendid": "__send_1", "origin": ""
            },
            {
                "due": "9223372036854775807", "act": "host", "type": "BasicHTTP",
                "event": "notify", "target": "http://host/x", "content": "",
                "params": {"a": ["1", "2"], "b": []},
                "sendid": "__send_2", "data": "", "invokeid": ""
            }
        ],
        "invokes": ["worker"],
        "hostinvokes": [
            {
                "type": "x-host", "id": "h", "src": "job://1",
                "params": {"a": ["1", "2"], "b": []}, "data": "{\"a\":[1,2]}",
                "content": "body", "due": "1700000005000"
            },
            {
                "type": "x-host", "id": "open", "src": "",
                "params": {}, "data": "", "content": "", "due": null
            }
        ],
        "hostinvoketoken": "7",
        "sendseq": "3",
        "external": [
            {
                "name": "tick", "data": "", "type": "external",
                "sendid": "", "origin": "", "origintype": "", "invokeid": "",
                "hostinvoketoken": null
            },
            {
                "name": "done.invoke.h", "data": "", "type": "external",
                "sendid": "", "origin": "", "origintype": "", "invokeid": "h",
                "hostinvoketoken": "6"
            }
        ],
        "revisions": {
            "reading.taken": {"revision": 3, "fields": "0".repeat(64)}
        },
        "features": ["session-unicast", "tls"]
    });
    assert!(
        errors(&validator, &good).is_empty(),
        "{:?}",
        errors(&validator, &good)
    );

    let broken = [
        ("another format", "format", serde_json::json!(2)),
        (
            "a shape that is not a digest",
            "shape",
            serde_json::json!("abc"),
        ),
        (
            "an empty configuration",
            "configuration",
            serde_json::json!([]),
        ),
        (
            "variables that are not an object keyed by their ids",
            "variables",
            serde_json::json!([null]),
        ),
        (
            "a history whose value is not a list of state ids",
            "history",
            serde_json::json!({"mode": "slow"}),
        ),
        (
            "a history that recorded no state",
            "history",
            serde_json::json!({"mode": []}),
        ),
        (
            "a waiting send that is none of raise, internal or host",
            "pending",
            serde_json::json!([{
                "due": "1", "act": "parent", "event": "e",
                "data": "", "sendid": "", "origin": ""
            }]),
        ),
        (
            "a moment written as a number",
            "pending",
            serde_json::json!([{
                "due": 5000, "act": "raise", "event": "e",
                "data": "", "sendid": "", "origin": ""
            }]),
        ),
        (
            "a moment that is not a whole number of milliseconds",
            "pending",
            serde_json::json!([{
                "due": "-1", "act": "raise", "event": "e",
                "data": "", "sendid": "", "origin": ""
            }]),
        ),
        (
            "a waiting send with no id field",
            "pending",
            serde_json::json!([{
                "due": "1", "act": "internal", "event": "e",
                "data": "", "origin": ""
            }]),
        ),
        (
            "a host send whose params are not lists of texts",
            "pending",
            serde_json::json!([{
                "due": "1", "act": "host", "type": "t", "event": "e", "target": "",
                "content": "", "params": {"p": "v"}, "sendid": "", "data": "", "invokeid": ""
            }]),
        ),
        (
            "a running invocation that is not an id",
            "invokes",
            serde_json::json!([5]),
        ),
        (
            "a running invocation named twice",
            "invokes",
            serde_json::json!(["worker", "worker"]),
        ),
        (
            "a running invocation with no id",
            "invokes",
            serde_json::json!([""]),
        ),
        (
            "a queued event of no type",
            "external",
            serde_json::json!([{
                "name": "tick", "data": "", "type": "sideways",
                "sendid": "", "origin": "", "origintype": "", "invokeid": "",
                "hostinvoketoken": null
            }]),
        ),
        (
            "a queued event with no host token member",
            "external",
            serde_json::json!([{
                "name": "tick", "data": "", "type": "external",
                "sendid": "", "origin": "", "origintype": "", "invokeid": ""
            }]),
        ),
        (
            "a queued event whose host token is a number",
            "external",
            serde_json::json!([{
                "name": "tick", "data": "", "type": "external",
                "sendid": "", "origin": "", "origintype": "", "invokeid": "",
                "hostinvoketoken": 6
            }]),
        ),
        (
            "a host invocation with no deadline member",
            "hostinvokes",
            serde_json::json!([{
                "type": "x-host", "id": "h", "src": "", "params": {},
                "data": "", "content": ""
            }]),
        ),
        (
            "a host invocation whose deadline is a number",
            "hostinvokes",
            serde_json::json!([{
                "type": "x-host", "id": "h", "src": "", "params": {},
                "data": "", "content": "", "due": 5000
            }]),
        ),
        (
            "a host invocation whose params are not lists of texts",
            "hostinvokes",
            serde_json::json!([{
                "type": "x-host", "id": "h", "src": "", "params": {"p": "v"},
                "data": "", "content": "", "due": null
            }]),
        ),
        (
            "a host invocation with no id",
            "hostinvokes",
            serde_json::json!([{
                "type": "x-host", "id": "", "src": "", "params": {},
                "data": "", "content": "", "due": null
            }]),
        ),
        (
            "a host token that is not a text of digits",
            "hostinvoketoken",
            serde_json::json!("-1"),
        ),
        (
            "a host token that is a number",
            "hostinvoketoken",
            serde_json::json!(7),
        ),
        (
            "a send count that is not a text of digits",
            "sendseq",
            serde_json::json!("-1"),
        ),
        (
            "a send count that is a number",
            "sendseq",
            serde_json::json!(3),
        ),
        (
            "a revisions member that names no schema",
            "revisions",
            serde_json::json!({}),
        ),
        (
            "a schema named at its first revision, which is left out",
            "revisions",
            serde_json::json!({"reading.taken": {"revision": 1, "fields": "0".repeat(64)}}),
        ),
        (
            "a revision whose fields are not a digest",
            "revisions",
            serde_json::json!({"reading.taken": {"revision": 2, "fields": "abc"}}),
        ),
        (
            "a revision written as a text",
            "revisions",
            serde_json::json!({"reading.taken": {"revision": "2", "fields": "0".repeat(64)}}),
        ),
        (
            "a configuration that names a feature twice",
            "features",
            serde_json::json!(["tls", "tls"]),
        ),
        (
            "a configuration that names a feature by no name",
            "features",
            serde_json::json!([""]),
        ),
        (
            "a configuration that is not a list of names",
            "features",
            serde_json::json!("tls"),
        ),
    ];
    for (what, key, value) in broken {
        let mut instance = good.clone();
        instance[key] = value;
        assert!(
            !errors(&validator, &instance).is_empty(),
            "the schema admitted {what}"
        );
    }

    // Every field is always present: a state with nothing waiting says so
    // with `[]`, and one that does not say is not this format. For `invokes`
    // that is what keeps a document with an `<invoke>` from restoring a state
    // that says "working" with nobody working, and for the host-run ones the
    // same, with the token a restore carries on from. The send count is the
    // same kind of field: without it a restored machine would hand out an id
    // a send still waiting in the saved state already holds.
    for field in [
        "pending",
        "invokes",
        "hostinvokes",
        "hostinvoketoken",
        "sendseq",
    ] {
        let mut without = good.clone();
        without.as_object_mut().expect("an object").remove(field);
        assert!(
            !errors(&validator, &without).is_empty(),
            "the schema admitted a saved state with no {field} field"
        );
    }

    // `revisions` and `features` are the members a state may leave out: one
    // whose record schemas are all in their first revision leaves out the
    // first, which is every state saved before schemas had revisions
    // (docs/adr/0044), and a machine of no product line the second, as every
    // state saved before configurations were recorded does (docs/adr/0057 §7).
    // The configuration that turns no feature on is `[]`, not an absence.
    for optional in ["revisions", "features"] {
        let mut without = good.clone();
        without.as_object_mut().expect("an object").remove(optional);
        assert!(
            errors(&validator, &without).is_empty(),
            "{optional}: {:?}",
            errors(&validator, &without)
        );
    }
    let mut none_on = good.clone();
    none_on["features"] = serde_json::json!([]);
    assert!(
        errors(&validator, &none_on).is_empty(),
        "{:?}",
        errors(&validator, &none_on)
    );
}

/// The variables of a machine that holds values that may be absent
/// (docs/adr/0055, section 4): absent ones as `null` and present ones as their
/// type writes them, in a variable, a record's field and a field of a record of
/// a list.
fn optional_variables() -> serde_json::Value {
    serde_json::json!({
        "due": null,
        "other": "7",
        "title": null,
        "status": "open",
        "item": {"id": 0, "due": null, "title": "first", "status": null},
        "items": [{"id": 1, "due": "12", "title": null, "status": "done"}]
    })
}

#[test]
fn an_absent_value_is_null_in_both_formats() {
    // The format-1 control the refusals above start from, holding them.
    let mut flat = serde_json::json!({
        "format": 1,
        "shape": "0".repeat(64),
        "configuration": ["s"],
        "current": "s",
        "variables": {},
        "history": {},
        "pending": [],
        "invokes": [],
        "hostinvokes": [],
        "hostinvoketoken": "0",
        "sendseq": "0",
        "external": []
    });
    flat["variables"] = optional_variables();
    let found = errors(&validator(), &flat);
    assert!(found.is_empty(), "{}", found.join("\n"));
    // ... and a format-2 tree, in the root and in a child it runs.
    let mut tree = a_tree();
    tree["variables"] = optional_variables();
    tree["invokes"][0]["session"]["variables"] = optional_variables();
    let found = errors(&validator_v2(), &tree);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

/// One session of a tree, with the running children it is given. Every
/// field a session must state is stated, with the values a session that
/// waits on nothing has.
fn session(shape: char, sessionid: &str, invokes: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "shape": shape.to_string().repeat(64),
        "sessionid": sessionid,
        "configuration": ["working"],
        "current": "working",
        "variables": {"completed": 0},
        "history": {},
        "pending": [],
        "invokes": invokes,
        "invokestarts": {},
        "hostinvokes": [],
        "hostinvoketoken": "0",
        "sendseq": "0",
        "external": []
    })
}

/// A format-2 saved state that uses every member the format adds: a parent
/// running a child that has taken an event and runs a grandchild, a send
/// waiting for each direction of the link, a child's event queued in its
/// parent, an invoke started twice and one whose id was generated.
fn a_tree() -> serde_json::Value {
    // The root's id given, as a host may give it; each child's its parent's id,
    // a `.` and the id it was started under (docs/adr/0045).
    let grandchild = session('c', "planner-1.worker.busy.1.helper", serde_json::json!([]));
    let mut child = session(
        'b',
        "planner-1.worker",
        serde_json::json!([{
            "id": "helper", "start": "1", "invokeid": "busy.1.helper",
            "document": "static_hybrid_holder", "session": grandchild
        }]),
    );
    child["variables"] = serde_json::json!({"completed": 1});
    child["invokestarts"] = serde_json::json!({"helper": "1"});
    child["pending"] = serde_json::json!([{
        "due": "1700000005000", "act": "parent", "event": "progress",
        "data": "{\"n\":1}", "sendid": "report", "origin": "planner-1.worker"
    }]);
    child["external"] = serde_json::json!([{
        "name": "go", "data": "", "type": "external", "sendid": "",
        "origin": "", "origintype": "", "invokeid": "", "hostinvoketoken": null
    }]);
    let mut root = session(
        'a',
        "planner-1",
        serde_json::json!([{
            "id": "worker", "start": "2", "invokeid": "worker",
            "document": "static_invoke_child", "session": child
        }]),
    );
    root["format"] = serde_json::json!(2);
    root["invokestarts"] = serde_json::json!({"worker": "2"});
    root["revisions"] = serde_json::json!({
        "todo.write": {"revision": 2, "fields": "d".repeat(64)}
    });
    // Each session records the configuration of its own document's product
    // line (docs/adr/0057 §7): the root's, and the child's, which turns none on.
    root["features"] = serde_json::json!(["session-unicast"]);
    root["invokes"][0]["session"]["features"] = serde_json::json!([]);
    root["pending"] = serde_json::json!([{
        "due": "1700000006000", "act": "child", "invokeid": "worker", "event": "nudge",
        "data": "", "sendid": "poke", "origin": "planner-1"
    }]);
    root["external"] = serde_json::json!([{
        "name": "done.step", "data": "", "type": "external", "sendid": "",
        "origin": "planner-1.worker", "origintype": "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
        "invokeid": "worker", "hostinvoketoken": null
    }]);
    root
}

#[test]
fn a_tree_of_sessions_is_a_format_2_saved_state() {
    let tree = a_tree();
    let found = errors(&validator_v2(), &tree);
    assert!(found.is_empty(), "{}", found.join("\n"));
    // The two formats are told apart by `format` before anything else, so a
    // reader of one never reads the other's text as its own.
    assert!(
        !errors(&validator(), &tree).is_empty(),
        "the format-1 schema admitted a tree"
    );
    assert!(
        !errors(&validator_v3(), &tree).is_empty(),
        "the format-3 schema admitted a format-2 tree"
    );
}

#[test]
fn the_format_2_schema_refuses_what_no_backend_writes() {
    // The control: the tree is a saved state, so each refusal below is the
    // one change it makes.
    let validator = validator_v2();
    let good = a_tree();
    assert!(
        errors(&validator, &good).is_empty(),
        "{:?}",
        errors(&validator, &good)
    );

    type Change = fn(&mut serde_json::Value);
    let broken: [(&str, Change); 22] = [
        // docs/adr/0045, "Questions the owner answered" 2: a session id is an
        // Nmtoken, so never empty and never a text an address cannot carry.
        ("a root with the empty session id", |t| {
            t["sessionid"] = serde_json::json!("")
        }),
        ("a grandchild with the empty session id", |t| {
            t["invokes"][0]["session"]["invokes"][0]["session"]["sessionid"] = serde_json::json!("")
        }),
        ("a session id with a space in it", |t| {
            t["sessionid"] = serde_json::json!("planner 1")
        }),
        ("a session id with a character no name holds", |t| {
            t["invokes"][0]["session"]["sessionid"] = serde_json::json!("planner-1#worker")
        }),
        ("format 1", |t| t["format"] = serde_json::json!(1)),
        (
            "a running child named by its id alone, as format 1 does",
            |t| t["invokes"] = serde_json::json!(["worker"]),
        ),
        ("a child with no state", |t| {
            t["invokes"][0]
                .as_object_mut()
                .expect("a child")
                .remove("session");
        }),
        ("a child whose state states the format again", |t| {
            t["invokes"][0]["session"]["format"] = serde_json::json!(2)
        }),
        ("a start counted from 0", |t| {
            t["invokes"][0]["start"] = serde_json::json!("0")
        }),
        ("a start written as a number", |t| {
            t["invokes"][0]["start"] = serde_json::json!(2)
        }),
        ("a child of no document", |t| {
            t["invokes"][0]["document"] = serde_json::json!("")
        }),
        ("a child with no invoke id", |t| {
            t["invokes"][0]["invokeid"] = serde_json::json!("")
        }),
        ("a grandchild whose shape is not a digest", |t| {
            t["invokes"][0]["session"]["invokes"][0]["session"]["shape"] = serde_json::json!("abc")
        }),
        ("a grandchild with no start count member", |t| {
            t["invokes"][0]["session"]["invokes"][0]["session"]
                .as_object_mut()
                .expect("a session")
                .remove("invokestarts");
        }),
        ("a tree with no session id", |t| {
            t.as_object_mut().expect("a tree").remove("sessionid");
        }),
        ("a start count of 0", |t| {
            t["invokestarts"] = serde_json::json!({"worker": "0"})
        }),
        ("a send to a child that names no child", |t| {
            t["pending"][0]
                .as_object_mut()
                .expect("a send")
                .remove("invokeid");
        }),
        ("a send to the parent that names a child", |t| {
            t["invokes"][0]["session"]["pending"][0]["invokeid"] = serde_json::json!("worker")
        }),
        ("a waiting send of an act no session makes", |t| {
            t["pending"][0]["act"] = serde_json::json!("sideways")
        }),
        ("revisions that name none", |t| {
            t["revisions"] = serde_json::json!({})
        }),
        ("a configuration that names a feature twice", |t| {
            t["features"] = serde_json::json!(["tls", "tls"])
        }),
        ("a child's configuration that is not a list of names", |t| {
            t["invokes"][0]["session"]["features"] = serde_json::json!({"tls": true})
        }),
    ];
    for (what, change) in broken {
        let mut instance = good.clone();
        change(&mut instance);
        assert!(
            !errors(&validator, &instance).is_empty(),
            "the format-2 schema admitted {what}"
        );
    }
}

/// The origin the SCXML processor writes for an event the session `id` sent,
/// as a schema instance.
fn scxml_origin(id: &str) -> serde_json::Value {
    serde_json::json!({"processor": SCXML_PROCESSOR, "address": scxml_location(id)})
}

/// A format-3 saved state that uses every member format 3 changes: the tree of
/// `a_tree`, with each queued event and each waiting send of an event carrying
/// its pair and its sender. The root holds a child's event (the SCXML pair and
/// the child as sender), a host invocation's ending (the host's own pair and no
/// sender), an event relayed from an address that names no session (the SCXML
/// pair and no sender), an event no processor stamped (neither) and a Mesh
/// remote child's event (the SCXML type at the child's session endpoint, and
/// the child's id as sender: docs/adr/0090, section 1); it waits on
/// a send to the child, one to itself, one to its internal queue (neither
/// member) and a host's (no member at all). The child waits on a send to the
/// root and holds an event no processor stamped.
fn a_format_3_tree() -> serde_json::Value {
    let mut tree = a_tree();
    tree["format"] = serde_json::json!(3);
    tree["pending"] = serde_json::json!([
        {
            "due": "1700000006000", "act": "child", "invokeid": "worker", "event": "nudge",
            "data": "", "sendid": "poke", "origin": scxml_origin("planner-1"), "sender": "planner-1"
        },
        {
            "due": "1700000007000", "act": "raise", "event": "tick", "data": "",
            "sendid": "tick", "origin": scxml_origin("planner-1"), "sender": "planner-1"
        },
        {
            "due": "1700000008000", "act": "internal", "event": "inner", "data": "",
            "sendid": "__send_1", "origin": null, "sender": null
        },
        {
            "due": "1700000009000", "act": "host", "type": "x-sce-host", "event": "audit",
            "target": "job://report", "content": "", "params": {}, "sendid": "a", "data": "",
            "invokeid": ""
        }
    ]);
    tree["external"] = serde_json::json!([
        {
            "name": "done.step", "data": "", "type": "external", "sendid": "",
            "origin": scxml_origin("planner-1.worker"), "sender": "planner-1.worker",
            "invokeid": "worker", "hostinvoketoken": null
        },
        {
            "name": "done.invoke.h", "data": "", "type": "external", "sendid": "",
            "origin": {"processor": "x-sce-host", "address": "motor"}, "sender": null,
            "invokeid": "h", "hostinvoketoken": "6"
        },
        {
            "name": "relayed", "data": "", "type": "external", "sendid": "",
            "origin": {"processor": SCXML_PROCESSOR, "address": "mesh://ecu"}, "sender": null,
            "invokeid": "", "hostinvoketoken": null
        },
        {
            "name": "tick", "data": "", "type": "external", "sendid": "",
            "origin": null, "sender": null, "invokeid": "", "hostinvoketoken": null
        },
        // The C++ Mesh transport's remote child (§mesh-9.6.3): its session id
        // is `<parent machine>:<invokeid>`, and its endpoint names that id,
        // percent-encoded, under the device and machine that run it.
        {
            "name": "progress", "data": "", "type": "external", "sendid": "",
            "origin": {"processor": SCXML_PROCESSOR, "address": "mesh://ecu/motor/planner%3Aremote"},
            "sender": "planner:remote", "invokeid": "remote", "hostinvoketoken": null
        }
    ]);
    let child = &mut tree["invokes"][0]["session"];
    child["pending"] = serde_json::json!([{
        "due": "1700000005000", "act": "parent", "event": "progress", "data": "{\"n\":1}",
        "sendid": "report", "origin": scxml_origin("planner-1.worker"),
        "sender": "planner-1.worker"
    }]);
    child["external"] = serde_json::json!([{
        "name": "go", "data": "", "type": "external", "sendid": "",
        "origin": null, "sender": null, "invokeid": "", "hostinvoketoken": null
    }]);
    tree
}

#[test]
fn a_tree_of_sessions_is_a_format_3_saved_state() {
    let mut tree = a_format_3_tree();
    let found = errors(&validator_v3(), &tree);
    assert!(found.is_empty(), "{}", found.join("\n"));
    // `format` tells the formats apart before anything else: a reader of
    // formats 1 and 2 refuses a format-3 text rather than read part of it.
    for (format, older) in [(1, validator()), (2, validator_v2())] {
        assert!(
            !errors(&older, &tree).is_empty(),
            "the format-{format} schema admitted a format-3 tree"
        );
    }
    // A value that may be absent is null in format 3 as in format 2
    // (docs/adr/0055), in the root and in a child it runs.
    tree["variables"] = optional_variables();
    tree["invokes"][0]["session"]["variables"] = optional_variables();
    let found = errors(&validator_v3(), &tree);
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn the_format_3_schema_refuses_what_no_backend_writes() {
    // The control: the tree is a saved state, so each refusal below is the
    // one change it makes.
    let validator = validator_v3();
    let good = a_format_3_tree();
    assert!(
        errors(&validator, &good).is_empty(),
        "{:?}",
        errors(&validator, &good)
    );

    type Change = fn(&mut serde_json::Value);
    let broken: [(&str, Change); 25] = [
        ("format 2", |t| t["format"] = serde_json::json!(2)),
        ("a child whose state states the format again", |t| {
            t["invokes"][0]["session"]["format"] = serde_json::json!(3)
        }),
        // docs/adr/0090, section 7, "Text form": the pair replaces both of
        // format 2's members, and both new members are always written.
        ("a queued event that keeps format 2's origintype", |t| {
            t["external"][0]["origintype"] = serde_json::json!(SCXML_PROCESSOR)
        }),
        (
            "a queued event whose origin is a text, as format 2 wrote it",
            |t| t["external"][0]["origin"] = serde_json::json!("planner-1.worker"),
        ),
        ("a queued event with no origin member", |t| {
            t["external"][3]
                .as_object_mut()
                .expect("an event")
                .remove("origin");
        }),
        ("a queued event with no sender member", |t| {
            t["external"][3]
                .as_object_mut()
                .expect("an event")
                .remove("sender");
        }),
        (
            "a grandchild's queued event written as format 2 wrote it",
            |t| {
                t["invokes"][0]["session"]["invokes"][0]["session"]["external"] = serde_json::json!([{
                    "name": "go", "data": "", "type": "external", "sendid": "",
                    "origin": "", "origintype": "", "invokeid": "", "hostinvoketoken": null
                }])
            },
        ),
        // Never half a pair (docs/adr/0090, section 1).
        ("a processor with no address", |t| {
            t["external"][1]["origin"] = serde_json::json!({"processor": "x-sce-host"})
        }),
        ("an address with no processor", |t| {
            t["external"][1]["origin"] = serde_json::json!({"address": "motor"})
        }),
        ("a pair with an empty address", |t| {
            t["external"][1]["origin"]["address"] = serde_json::json!("")
        }),
        ("a pair with an empty processor", |t| {
            t["external"][1]["origin"]["processor"] = serde_json::json!("")
        }),
        ("a pair with a member it does not have", |t| {
            t["external"][1]["origin"]["sender"] = serde_json::json!("planner-1")
        }),
        // A session that sent an event always has a reply address
        // (docs/adr/0090, section 7, "Text form").
        ("a queued event with a sender beside a null origin", |t| {
            t["external"][3]["sender"] = serde_json::json!("planner-1")
        }),
        ("a remote child's event that lost its reply address", |t| {
            t["external"][4]["origin"] = serde_json::json!(null)
        }),
        ("an empty sender", |t| {
            t["external"][0]["sender"] = serde_json::json!("")
        }),
        ("a sender that is not a session id", |t| {
            t["external"][0]["sender"] = serde_json::json!("planner 1")
        }),
        // A waiting send of an event is the session's own, and its event is
        // what it carries; an internal event carries none.
        ("a send to a child that names no sender", |t| {
            t["pending"][0]["sender"] = serde_json::json!(null)
        }),
        ("a send to itself with no origin", |t| {
            t["pending"][1]["origin"] = serde_json::json!(null)
        }),
        (
            "a send to itself whose sender is the SCXML prefix alone",
            |t| t["pending"][1]["sender"] = serde_json::json!(SCXML_LOCATION),
        ),
        (
            "a send to itself whose origin is a text, as format 2 wrote it",
            |t| t["pending"][1]["origin"] = serde_json::json!("planner-1"),
        ),
        ("a send to the parent from another processor", |t| {
            t["invokes"][0]["session"]["pending"][0]["origin"] =
                serde_json::json!({"processor": "x-sce-host", "address": "motor"})
        }),
        ("a send to the internal queue that names a sender", |t| {
            t["pending"][2]["sender"] = serde_json::json!("planner-1")
        }),
        ("a send to the internal queue that carries an origin", |t| {
            t["pending"][2]["origin"] = scxml_origin("planner-1")
        }),
        ("a send to the internal queue with no sender member", |t| {
            t["pending"][2]
                .as_object_mut()
                .expect("a send")
                .remove("sender");
        }),
        ("a host's send that carries an origin", |t| {
            t["pending"][3]["origin"] = serde_json::json!(null)
        }),
    ];
    for (what, change) in broken {
        let mut instance = good.clone();
        change(&mut instance);
        assert!(
            !errors(&validator, &instance).is_empty(),
            "the format-3 schema admitted {what}"
        );
    }

    // `origin` is the reply address and `sender` the identity of the sending
    // session, and no rule ties them on a queued event (docs/adr/0090,
    // section 1): each of these is a text a backend writes, so the schema
    // admits it.
    let kept: [(&str, Change); 3] = [
        ("a sender beside a host's pair", |t| {
            t["external"][1]["sender"] = serde_json::json!("planner-1.worker")
        }),
        (
            "a sender beside an SCXML pair at an address that names no session",
            |t| t["external"][2]["sender"] = serde_json::json!("planner-1"),
        ),
        ("a remote child's event with no sender", |t| {
            t["external"][4]["sender"] = serde_json::json!(null)
        }),
    ];
    for (what, change) in kept {
        let mut instance = good.clone();
        change(&mut instance);
        let found = errors(&validator, &instance);
        assert!(
            found.is_empty(),
            "the format-3 schema refused {what}: {}",
            found.join("\n")
        );
    }
}
