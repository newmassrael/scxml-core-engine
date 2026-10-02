// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A `<send>` whose route is computed says what that route rests on.
//!
//! # What was wrong
//!
//! Measured 2026-10-02: a draft wrote `targetexpr="callerTarget"` over a
//! `<data id="callerTarget">` marked `sce:unresolved`, because a send needs a
//! target and the specification never says who the caller is. A scenario run
//! of it ended in the engine's `error.communication` (§scxml-6.2.4), and the
//! verdict read as a fault of the design. The literal `#_parent` case already
//! names the open question on its site (`parent_sends[].decisions`); a route
//! chosen by `typeexpr` / `targetexpr` is decided by the datamodel when the
//! send runs, so the question sits on the data item and not on the send.
//!
//! So the manifest lists those sends too: `computed_routes`, each with the data
//! items its route expressions name and the ids of the questions open on the
//! send or on any of them. These cases hold that list to the document on the
//! binary a host runs, and to the wire schema a consumer reads it with.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn scratch(label: &str) -> PathBuf {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// The manifest `check -l rust` writes for `body`, validated against the
/// checked-in schema before it is returned.
fn manifest(label: &str, body: &str) -> serde_json::Value {
    let dir = scratch(label);
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["--error-format", "json", "check"])
        .arg(&path)
        .args(["-l", "rust"])
        .output()
        .expect("run sce-codegen check");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        out.status.success(),
        "{label} is valid SCXML and must check: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("the manifest is one JSON line");

    let schema: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_root().join("schemas/sce-manifest.v1.schema.json"))
            .expect("read manifest schema"),
    )
    .expect("manifest schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("manifest schema compiles");
    let violations: Vec<String> = match validator.validate(&manifest) {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    };
    assert!(violations.is_empty(), "{label}: {violations:?}\n{manifest}");
    manifest
}

fn strings(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|v| v.as_str().expect("a string"))
                .collect()
        })
        .unwrap_or_default()
}

/// A send routed by a data item the specification leaves open names the
/// question, on the item it reads: the draft wrote the route because a send
/// needs one, and nobody has said who the caller is.
#[test]
fn a_route_read_from_an_open_data_item_names_the_question() {
    let m = manifest(
        "asks-where",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="asks_where" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="callerTarget" expr="''" sce:unresolved="caller-routing"
          sce:unresolved-reason="the specification does not say who the caller is"/>
  </datamodel>
  <state id="idle">
    <onentry>
      <send event="notice" targetexpr="callerTarget"/>
    </onentry>
  </state>
</scxml>
"##,
    );
    let routes = m["computed_routes"]
        .as_array()
        .expect("computed_routes is listed");
    assert_eq!(routes.len(), 1, "{m}");
    let route = &routes[0];
    assert_eq!(route["event"], "notice", "{m}");
    assert_eq!(route["state"], "idle", "{m}");
    assert_eq!(route["location"]["line"], 10, "{m}");
    assert_eq!(strings(&route["reads"]), ["callerTarget"], "{m}");
    assert_eq!(strings(&route["decisions"]), ["caller-routing"], "{m}");
}

/// A value chosen without an answer (`sce:assumed`) is applied and is not a
/// route nobody decided, whether it sits on the data item or on the send; a
/// route read from a plain item rests on nothing. All are listed, because the
/// route is still computed, and name no decision.
#[test]
fn only_a_question_is_a_decision_a_route_rests_on() {
    let m = manifest(
        "assumed-and-plain",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="assumed_and_plain" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="chosen" expr="'#_internal'" sce:assumed="route-by-default"
          sce:assumed-reason="the standing rule says a notice stays inside"/>
    <data id="plain" expr="'#_internal'"/>
  </datamodel>
  <state id="idle">
    <onentry>
      <send event="a" targetexpr="chosen"/>
      <send event="b" targetexpr="plain"/>
      <send event="c" targetexpr="plain" sce:assumed="route-by-default"
            sce:assumed-reason="the standing rule says a notice stays inside"/>
    </onentry>
  </state>
</scxml>
"##,
    );
    let routes = m["computed_routes"]
        .as_array()
        .expect("computed_routes is listed");
    let seen: Vec<(&str, Vec<&str>, Vec<&str>)> = routes
        .iter()
        .map(|r| {
            (
                r["event"].as_str().expect("event"),
                strings(&r["reads"]),
                strings(&r["decisions"]),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("a", vec!["chosen"], vec![]),
            ("b", vec!["plain"], vec![]),
            ("c", vec!["plain"], vec![])
        ],
        "{m}"
    );
    assert!(
        routes.iter().all(|r| r.get("decisions").is_none()),
        "omitted, not []: {m}"
    );
}

/// The question can sit on the send itself, and on both a `typeexpr` item and a
/// `targetexpr` item at once, one question spanning two of them: every id is
/// listed, each once, the send's own first and the items' after it in the order
/// they are read.
#[test]
fn a_route_resting_on_several_questions_lists_each_once() {
    let m = manifest(
        "asks-twice",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="asks_twice" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="processor" expr="''" sce:unresolved="caller-routing"
          sce:unresolved-reason="the specification names no transport"/>
    <data id="destination" expr="''" sce:unresolved="caller-routing"
          sce:unresolved-reason="the specification names no destination"/>
    <data id="spare" expr="''" sce:unresolved="not-read"
          sce:unresolved-reason="nothing routes by this"/>
  </datamodel>
  <state id="idle">
    <onentry>
      <send event="out" typeexpr="processor" targetexpr="destination"
            sce:unresolved="send-shape"
            sce:unresolved-reason="the specification does not say a notice is sent"/>
    </onentry>
  </state>
</scxml>
"##,
    );
    let routes = m["computed_routes"]
        .as_array()
        .expect("computed_routes is listed");
    assert_eq!(routes.len(), 1, "{m}");
    assert_eq!(
        strings(&routes[0]["reads"]),
        ["processor", "destination"],
        "{m}"
    );
    assert_eq!(
        strings(&routes[0]["decisions"]),
        ["send-shape", "caller-routing"],
        "an item the route does not name adds nothing: {m}"
    );
}

/// A name after a `.` is a property and not a data item: `_event.data.target`
/// reads `_event`, which no `<data>` declares, so a route taken from the event
/// is listed and traced to nothing — and a data item that merely shares the
/// property's name is not read.
#[test]
fn a_property_that_shares_a_data_items_name_is_not_a_read() {
    let m = manifest(
        "property",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="property" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="target" expr="''" sce:unresolved="caller-routing"
          sce:unresolved-reason="the specification does not say who the caller is"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="idle">
      <send event="forward" targetexpr="_event.data.target"/>
    </transition>
  </state>
</scxml>
"##,
    );
    let routes = m["computed_routes"]
        .as_array()
        .expect("computed_routes is listed");
    assert_eq!(routes.len(), 1, "{m}");
    assert!(routes[0].get("reads").is_none(), "omitted, not []: {m}");
    assert!(
        routes[0].get("decisions").is_none(),
        "no question is on a property: {m}"
    );
}

/// A document with no computed route lists none — omitted, not `[]` — and a
/// literal route is never one.
#[test]
fn a_document_with_only_literal_routes_lists_none() {
    let m = manifest(
        "literal",
        r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="literal" initial="idle">
  <state id="idle">
    <onentry><send target="#_internal" event="tick"/></onentry>
    <transition event="tick" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##,
    );
    assert!(m.get("computed_routes").is_none(), "omitted, not []: {m}");
}

/// The Python machine `generate -l python` writes for `body`.
fn generated_python(label: &str, body: &str) -> String {
    let dir = scratch(label);
    let path = dir.join(format!("{label}.scxml"));
    std::fs::write(&path, body).expect("write fixture");
    let out = Command::new(sce_codegen_bin())
        .args(["generate"])
        .arg(&path)
        .args(["-l", "python", "-o"])
        .arg(&dir)
        .output()
        .expect("run sce-codegen generate");
    assert!(
        out.status.success(),
        "{label} must generate: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let machine = std::fs::read_dir(&dir)
        .expect("read the output directory")
        .map(|entry| entry.expect("a directory entry").path())
        .find(|p| p.to_string_lossy().ends_with("_sm.py"))
        .expect("a generated machine");
    let text = std::fs::read_to_string(machine).expect("read the generated machine");
    let _ = std::fs::remove_dir_all(&dir);
    text
}

/// The lines of `machine` that mention `needle`, every one of them.
fn lines_with<'a>(machine: &'a str, needle: &str) -> Vec<&'a str> {
    machine.lines().filter(|l| l.contains(needle)).collect()
}

const ASKING: &str = r#"rests_on=("caller-target", )"#;

const ONE_OF_EACH_FAULT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="each_fault" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="open" expr="'#_internal'" sce:unresolved="caller-target"
          sce:unresolved-reason="the specification does not say who the caller is"/>
  </datamodel>
  <state id="idle">
    <transition event="route" target="idle">
      <send event="a" targetexpr="open"/>
    </transition>
    <transition event="kind" target="idle">
      <send event="b" typeexpr="open"/>
    </transition>
    <transition event="name" target="idle">
      <send eventexpr="missingEvent" targetexpr="open"/>
    </transition>
    <transition event="wait" target="idle">
      <send event="d" delayexpr="missingDelay" targetexpr="open"/>
    </transition>
    <transition event="list" target="idle">
      <send event="e" targetexpr="open" namelist="missingName"/>
    </transition>
  </state>
</scxml>
"##;

/// The generated machine hands the questions a route rests on to the places the
/// ROUTE fails, and to no other. Measured 2026-10-02: the questions were
/// registered when a send began and attached to every error that send raised, so
/// an event name, a delay or a namelist that failed beside an open route was
/// told to the owner as the route's open question.
#[test]
fn only_the_places_the_route_fails_carry_its_questions() {
    let machine = generated_python("each-fault", ONE_OF_EACH_FAULT);

    // The route: its evaluation, the address it produces, the type, and the
    // delivery. Every one of these names the question.
    for place in [
        "\"<send> targetexpr could not be evaluated\"",
        "\"<send> targetexpr produced a target this processor cannot address\"",
        "\"<send> typeexpr could not be evaluated\"",
        "\"<send> typeexpr names a processor this platform does not support\"",
        "\"<send> targetexpr evaluated to nothing, so there is no target to reach\"",
    ] {
        let found = lines_with(&machine, place);
        assert!(!found.is_empty(), "the machine has no place saying {place}");
        for line in found {
            assert!(
                line.contains(ASKING),
                "{place} does not carry the question: {line}"
            );
        }
    }
    let delivery: Vec<&str> = machine
        .lines()
        .filter(|l| l.contains("engine.raise_internal(") && l.contains("send_id=_sid"))
        .filter(|l| !l.contains("data="))
        .collect();
    assert!(!delivery.is_empty(), "the machine has no delivery failure");
    for line in delivery {
        assert!(
            line.contains(ASKING),
            "a delivery failure carries no question: {line}"
        );
    }

    // A delayed send is refused, or its target found gone, when its wait is over,
    // long after this site returned. The questions have to be handed over with
    // the send, or the scheduler has none to raise the refusal with.
    for call in ["engine.send_to_target(", "engine.schedule_host_send("] {
        let found = lines_with(&machine, call);
        assert!(!found.is_empty(), "the machine has no {call}");
        for line in found {
            assert!(
                line.contains(ASKING),
                "{call} is not told the route's questions: {line}"
            );
        }
    }

    // Everything else about the send: a fault of the document, whatever the
    // route's data leaves open.
    for place in ["missingEvent", "missingDelay", "missingName"] {
        let found = lines_with(&machine, place);
        assert!(
            !found.is_empty(),
            "the machine has no place evaluating {place}"
        );
        for line in found {
            assert!(
                !line.contains("rests_on"),
                "{place} must not be laid to the route's question: {line}"
            );
        }
    }
    let empty_name = lines_with(
        &machine,
        "eventexpr could not be evaluated to an event name",
    );
    assert!(!empty_name.is_empty());
    for line in empty_name {
        assert!(
            !line.contains("rests_on"),
            "the event name is not the route: {line}"
        );
    }
}

const TYPE_ASKING: &str = r#"rests_on=("which-processor", )"#;
const TARGET_ASKING: &str = r#"rests_on=("caller-target", )"#;

const TYPE_AND_TARGET_APART: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="apart" initial="idle" datamodel="ecmascript">
  <datamodel>
    <data id="kind" expr="'x-unsupported'" sce:unresolved="which-processor"
          sce:unresolved-reason="the specification does not say how the caller is reached"/>
    <data id="where" expr="'#_internal'" sce:unresolved="caller-target"
          sce:unresolved-reason="the specification does not say who the caller is"/>
  </datamodel>
  <state id="idle">
    <transition event="go" target="idle">
      <send event="a" typeexpr="kind" targetexpr="where"/>
    </transition>
    <transition event="later" target="idle">
      <send event="b" typeexpr="kind" targetexpr="where" delayexpr="'1s'"/>
    </transition>
  </state>
</scxml>
"##;

/// The type and the target fail independently, so each failure is handed only the
/// questions its own expression rests on. Measured 2026-10-02: one merged list was
/// handed to both, so a type that named a variable nobody declared was told to the
/// owner as waiting on who the caller is.
#[test]
fn the_type_and_the_target_are_each_handed_their_own_questions() {
    let machine = generated_python("apart", TYPE_AND_TARGET_APART);

    for place in [
        "\"<send> typeexpr could not be evaluated\"",
        "\"<send> typeexpr names a processor this platform does not support\"",
    ] {
        let found = lines_with(&machine, place);
        assert!(!found.is_empty(), "the machine has no place saying {place}");
        for line in found {
            assert!(
                line.contains(TYPE_ASKING) && !line.contains("caller-target"),
                "{place} is not handed the type's question alone: {line}"
            );
        }
    }
    for place in [
        "\"<send> targetexpr could not be evaluated\"",
        "\"<send> targetexpr produced a target this processor cannot address\"",
        "\"<send> targetexpr evaluated to nothing, so there is no target to reach\"",
        "engine.send_to_target(",
    ] {
        let found = lines_with(&machine, place);
        assert!(!found.is_empty(), "the machine has no place saying {place}");
        for line in found {
            assert!(
                line.contains(TARGET_ASKING) && !line.contains("which-processor"),
                "{place} is not handed the target's question alone: {line}"
            );
        }
    }
}

/// A document with no open question gets the machine it always got: not one
/// `rests_on`, so nothing committed and generated moves.
#[test]
fn a_route_resting_on_no_question_changes_nothing_in_the_machine() {
    let machine = generated_python(
        "no-question",
        &ONE_OF_EACH_FAULT.replace(
            " sce:unresolved=\"caller-target\"\n          sce:unresolved-reason=\"the specification does not say who the caller is\"",
            "",
        ),
    );
    // The helpers every machine carries name the parameter (`rests_on: tuple`,
    // `rests_on=rests_on`); what a send hands them is spelled `rests_on=(`.
    assert!(
        !machine.contains("rests_on=("),
        "a route that rests on no question emits no question: {:?}",
        lines_with(&machine, "rests_on=(")
    );
}
