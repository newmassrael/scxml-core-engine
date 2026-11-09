// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10 and §scxml-B-2-8: what the read-only binding of the system
// variables leaves a host able to do, beside the script writes
// `system_variable_writes.rs` holds to the shared table.
//
// The binding makes `_event` and `_ioprocessors` read-only views whose own
// tables are empty. A value the engine hands back to the host is read through
// the view, so `<data id="v" expr="_ioprocessors"/>` copies the processors
// rather than an empty table. A variable the document names is set by name,
// and a system variable's name is refused there as it is in a script.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::{IScriptEngine, ScriptValue, SetCurrentEventArgs};
use std::collections::HashMap;

const LOCATION: &str = "sce://scxml/s";

/// A session `s` named `machine`, with one processor and one current event.
fn bound_session() -> LuaEngine {
    let engine = LuaEngine::new();
    engine.create_session("s").expect("the session is created");
    engine
        .setup_system_variables(
            "s",
            "machine",
            &[IoProcessorDescriptor {
                name: "scxml".to_string(),
                location: LOCATION.to_string(),
            }],
        )
        .expect("setup_system_variables");
    engine
        .set_current_event(
            "s",
            SetCurrentEventArgs {
                event_name: "brief",
                event_data: "{\"n\": 1}",
                event_type: "external",
                send_id: "",
                origin: "",
                origin_type: "",
                invoke_id: "",
            },
        )
        .expect("set_current_event");
    engine
}

fn object(entries: &[(&str, ScriptValue)]) -> ScriptValue {
    ScriptValue::Object(
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.clone()))
            .collect::<HashMap<_, _>>(),
    )
}

#[test]
fn a_read_only_system_variable_reads_whole() {
    let engine = bound_session();
    assert_eq!(
        engine
            .evaluate_expression("s", "_ioprocessors")
            .expect("_ioprocessors evaluates"),
        object(&[(
            "scxml",
            object(&[("location", ScriptValue::String(LOCATION.to_string()))]),
        )]),
        "§scxml-B-2-8: `_ioprocessors` read whole holds each processor's entry"
    );
    assert_eq!(
        engine
            .evaluate_expression("s", "_event")
            .expect("_event evaluates"),
        object(&[
            ("name", ScriptValue::String("brief".to_string())),
            ("type", ScriptValue::String("external".to_string())),
            ("data", object(&[("n", ScriptValue::Int(1))])),
        ]),
        "§scxml-B-2-8: `_event` read whole holds its bound fields"
    );
}

#[test]
fn a_system_variable_is_not_set_by_name() {
    let engine = bound_session();
    for name in ["_event", "_sessionid", "_name", "_ioprocessors"] {
        assert!(
            engine.set_variable("s", name, ScriptValue::Int(1)).is_err(),
            "§scxml-5.10: setting `{name}` by name must fail"
        );
        assert!(
            engine.set_variable_as_dom("s", name, "<a/>").is_err(),
            "§scxml-5.10: binding a document to `{name}` must fail"
        );
    }
    for (expression, expected) in [
        ("_sessionid", ScriptValue::String("s".to_string())),
        ("_name", ScriptValue::String("machine".to_string())),
        ("_event.name", ScriptValue::String("brief".to_string())),
        (
            "_ioprocessors['scxml'].location",
            ScriptValue::String(LOCATION.to_string()),
        ),
    ] {
        assert_eq!(
            engine
                .evaluate_expression("s", expression)
                .unwrap_or_else(|e| panic!("{expression} evaluates: {e}")),
            expected,
            "§scxml-5.10: a refused set leaves `{expression}` as it was bound"
        );
    }
    // The refusal is of the system names only: an ordinary variable is set.
    engine
        .set_variable("s", "v", ScriptValue::Int(1))
        .expect("an ordinary variable is set by name");
    assert_eq!(
        engine.get_variable("s", "v").expect("v reads"),
        ScriptValue::Int(1)
    );
}
