// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The saved-state wire surface (`schemas/sce-saved-state.v1.schema.json` and
// `schemas/sce-saved-state.v2.schema.json`, SCE Accepted Subset §2.15) held to
// its instances.
//
// The shared fixtures under `tests/fixtures/static_datamodel/saved/` are the
// text every backend must save after the same run and must restore from —
// the Rust and Kotlin static suites each assert both. So they are the
// surface's instances, and each one must be what the schema says a saved
// state is. The directory is swept rather than listed, so a fixture added
// for a new backend or machine is held to the schema without an edit here.
//
// Format 2 (docs/adr/0045) is a tree of sessions. No backend writes it yet,
// so its instance is written here by hand, as the observation trace's was
// before any driver existed; the shared fixtures take format-2 texts when the
// first backend writes them.

use std::path::{Path, PathBuf};

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
    let validator = validator();
    // The shared instances of the static data model and of the host-run
    // invocation fixture, which lives with the other host fixtures.
    let dirs = [
        repo_root().join("sce-build/tests/fixtures/static_datamodel/saved"),
        repo_root().join("sce-build/tests/fixtures/host_processor/saved"),
    ];
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
    // A sweep over nothing certifies nothing: a record and a list when this
    // floor was set, and the host invocation's instance since.
    assert!(
        fixtures.len() >= 3,
        "swept only {} saved-state fixture(s) in {dirs:?}",
        fixtures.len(),
    );
    for path in fixtures {
        let text = std::fs::read_to_string(&path).expect("read fixture");
        let instance: serde_json::Value = serde_json::from_str(&text)
            .unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()));
        let found = errors(&validator, &instance);
        assert!(
            found.is_empty(),
            "{} is not a saved state:\n{}",
            path.display(),
            found.join("\n")
        );
    }
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
        }
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
            "a variable that is null",
            "variables",
            serde_json::json!({"count": null}),
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

    // `revisions` is the one member a state may leave out: one whose record
    // schemas are all in their first revision does, which is every state saved
    // before schemas had revisions (docs/adr/0044).
    let mut first_revision = good.clone();
    first_revision
        .as_object_mut()
        .expect("an object")
        .remove("revisions");
    assert!(
        errors(&validator, &first_revision).is_empty(),
        "{:?}",
        errors(&validator, &first_revision)
    );
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
    let grandchild = session('c', "", serde_json::json!([]));
    let mut child = session(
        'b',
        "s.working.worker.2",
        serde_json::json!([{
            "id": "helper", "start": "1", "invokeid": "s.working.worker.2.busy.helper.1",
            "document": "static_hybrid_holder", "session": grandchild
        }]),
    );
    child["variables"] = serde_json::json!({"completed": 1});
    child["invokestarts"] = serde_json::json!({"helper": "1"});
    child["pending"] = serde_json::json!([{
        "due": "1700000005000", "act": "parent", "event": "progress",
        "data": "{\"n\":1}", "sendid": "report", "origin": "s.working.worker.2"
    }]);
    child["external"] = serde_json::json!([{
        "name": "go", "data": "", "type": "external", "sendid": "",
        "origin": "", "origintype": "", "invokeid": "", "hostinvoketoken": null
    }]);
    let mut root = session(
        'a',
        "",
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
    root["pending"] = serde_json::json!([{
        "due": "1700000006000", "act": "child", "invokeid": "worker", "event": "nudge",
        "data": "", "sendid": "poke", "origin": ""
    }]);
    root["external"] = serde_json::json!([{
        "name": "done.step", "data": "", "type": "external", "sendid": "",
        "origin": "s.working.worker.2", "origintype": "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
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
    let broken: [(&str, Change); 16] = [
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
