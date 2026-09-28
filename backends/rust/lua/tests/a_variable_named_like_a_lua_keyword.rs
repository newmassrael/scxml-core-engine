// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// ECMAScript's identifier set is wider than Lua's: a datamodel variable may be
// spelled `local`, `end` or `repeat`, which Lua reserves. The frontend reads
// such a name through the globals table, as `_ENV["name"]`, and this engine's
// W3C ReferenceError check has to look at that name — not at `_ENV`, which is
// an upvalue and never a declared variable. It did not, so every read of a
// keyword-named variable failed as undeclared.
//
// The cases are tests/scripting/undeclared_reads.json, which the Go and Python
// engines' checks read too.

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::{IScriptEngine, ScriptValue};
use std::path::Path;

#[test]
fn a_read_is_refused_exactly_when_the_shared_table_says() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/scripting/undeclared_reads.json");
    let table: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .expect("the table is JSON");
    let cases = table["cases"].as_array().expect("the table has cases");
    // A floor: an empty table would pass every assertion below.
    assert!(cases.len() >= 10, "the table lost cases: {}", cases.len());
    for (i, case) in cases.iter().enumerate() {
        let engine = LuaEngine::new();
        let session = format!("s{i}");
        engine.create_session(&session);
        for name in case["declared"].as_array().expect("declared is a list") {
            let name = name.as_str().expect("a declared name is a string");
            engine
                .set_variable(&session, name, ScriptValue::String(format!("#{name}")))
                .expect("any name is a table key");
        }
        let expr = case["expr"].as_str().expect("expr is a string");
        let refused = case["refused"].as_bool().expect("refused is a bool");
        let got = engine.evaluate_expression(&session, expr);
        assert_eq!(
            got.is_err(),
            refused,
            "{expr:?} with {:?}: {got:?}",
            case["declared"]
        );
    }
}
