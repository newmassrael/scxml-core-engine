// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
//! The Lua sources every engine loads have exactly one definition.
//!
//! `sce/include/scripting/*.lua` is that definition. Five of the six
//! backends reach it directly — C++ through a CMake-generated header, Rust
//! through `include_str!`, Kotlin through a Gradle copy, Python through the
//! repository path, C11 through a codegen-time embed — and Go cannot,
//! because `go:embed` refuses paths outside the module. Go therefore keeps
//! a byte copy, and a byte copy with nothing checking it is a second
//! definition waiting to disagree.
//!
//! That matters more for `ecma_semantics.lua` than for anything else here:
//! it defines what `==` and `+` *mean*, so a drifted copy would not crash,
//! it would answer differently on one backend.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every `(canonical source, copy)` pair a backend maintains.
const COPIES: &[(&str, &str)] = &[
    (
        "sce/include/scripting/json_builtins.lua",
        "backends/go/lua/json_builtins.lua",
    ),
    (
        "sce/include/scripting/ecma_semantics.lua",
        "backends/go/lua/ecma_semantics.lua",
    ),
];

#[test]
fn every_copied_lua_asset_is_byte_identical_to_its_source() {
    for (source, copy) in COPIES {
        let source_path = repo_root().join(source);
        let copy_path = repo_root().join(copy);
        let source_bytes = std::fs::read(&source_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", source_path.display()));
        let copy_bytes = std::fs::read(&copy_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", copy_path.display()));
        assert_eq!(
            source_bytes, copy_bytes,
            "{copy} has drifted from {source}. The copy exists because `go:embed` \
             cannot reach outside the module, not because the two are allowed to \
             differ — copy the source over it rather than editing the copy."
        );
    }
}

/// Every engine that runs generated Lua loads the semantics file.
///
/// The emitted code calls `_scxml_add`, `_scxml_eq` and the bit helpers by
/// name. An engine that skips the file does not fail to build — it fails at
/// the first `+` in a document, at runtime, in whatever language embeds it.
/// So the load is asserted from the outside, once, rather than trusted to
/// six separate reviews.
#[test]
fn every_lua_engine_loads_the_shared_semantics() {
    // Each entry: the file that must reference the asset, and the token that
    // shows it is loaded rather than merely mentioned.
    const LOADERS: &[(&str, &str)] = &[
        ("backends/rust/lua/src/lib.rs", "ecma_semantics.lua"),
        ("backends/go/lua/lua_engine.go", "ecmaSemanticsLua"),
        (
            "backends/kotlin/lua/src/main/kotlin/com/sce/scripting/lua/LuaScriptEngine.kt",
            "loadEcmaSemantics()",
        ),
        (
            "backends/python/runtime/sce_runtime/scripting/lua_engine.py",
            "ecma_semantics.lua",
        ),
        ("sce/src/scripting/LuaEngine.cpp", "ECMA_SEMANTICS_LUA"),
        (
            "tools/codegen/templates/c/scriptengine.jinja2",
            "ecma_semantics_lua_c",
        ),
    ];
    let mut missing = Vec::new();
    for (file, token) in LOADERS {
        let path = repo_root().join(file);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if !text.contains(token) {
            missing.push(format!(
                "{file} does not load the shared semantics ({token})"
            ));
        }
    }
    assert!(
        missing.is_empty(),
        "an engine runs generated Lua without the definitions that Lua calls:\n{}",
        missing.join("\n")
    );
}

/// The C11 embed loads the shared assets in pieces — and the pieces work.
///
/// C11 cannot open a file at runtime, so codegen carries the same Lua as a
/// sequence of `luaL_dostring` calls, one per chunk, because ISO C99
/// guarantees only 4095 characters in a string literal. Two things could go
/// wrong there and neither is loud: a boundary could fall inside a function
/// body, and the generated code discards every `luaL_dostring` result, so a
/// chunk that failed to compile would leave its definitions simply absent.
/// The C11 suite would stay green — no W3C fixture calls `'a'.substring(0,1)`
/// — and a consumer's document would fail at runtime instead.
///
/// So the split is executed here rather than reviewed: the chunks go into one
/// interpreter one at a time, in order, and then the definitions are called.
#[test]
fn the_c11_embed_chunks_load_and_define_what_they_claim() {
    use sce_build::filters::c11_lua_chunks;
    use sce_rust_lua::LuaEngine;
    use sce_rust_runtime::scripting::{IScriptEngine, ScriptValue};

    let engine = LuaEngine::new();
    let session = "c11-embed-probe";
    engine.create_session(session);

    // Loaded the way the generated C11 bootstrap loads them: separately, in
    // order, into the one state. `_scxml_truthy` is a per-engine native that
    // `Boolean` calls, and this engine installs it with the session.
    for (name, source) in [
        ("ecma_semantics.lua", sce_build::filters::ECMA_SEMANTICS_LUA),
        ("json_builtins.lua", sce_build::filters::JSON_BUILTINS_LUA),
    ] {
        // Every PARAGRAPH first, one `execute_script` each. This is the worst
        // split the budget can ever produce, and asserting it is what keeps
        // the test from depending on where the boundaries happen to fall
        // today: measured while writing this, a blank line inserted into a
        // function body did not break the shipped chunking, because the
        // budget put the boundary somewhere else. It would break as soon as
        // the file grew. Asked this way, it breaks now.
        for (i, paragraph) in source.split("\n\n").enumerate() {
            if paragraph.trim().is_empty() {
                continue;
            }
            engine
                .execute_script(session, paragraph)
                .unwrap_or_else(|e| {
                    panic!(
                        "{name} paragraph {i} is not a complete Lua chunk: {e}\n\
                     The C11 embed splits this file on blank lines and hands each \
                     piece to a separate luaL_dostring, discarding the result — so \
                     a blank line inside a definition costs that definition, \
                     silently, on that backend only, as soon as the budget puts a \
                     boundary there."
                    )
                });
        }

        // Then the split that actually ships, so a change to the budget or the
        // joining is measured too rather than inferred from the paragraphs.
        for (i, chunk) in c11_lua_chunks(source).iter().enumerate() {
            engine
                .execute_script(session, chunk)
                .unwrap_or_else(|e| panic!("{name} chunk {i} does not load on its own: {e}"));
        }
    }

    // Named one by one rather than by counting definitions: what matters is
    // that the call works, and a count would pass against a chunk that
    // defined a function whose body had been cut in half.
    for (expression, expected) in [
        (
            "_scxml_substring('abcdef', 1, 3)",
            ScriptValue::String("bc".into()),
        ),
        ("_scxml_charat('abc', 1)", ScriptValue::String("b".into())),
        (
            "_scxml_tolowercase('AbC')",
            ScriptValue::String("abc".into()),
        ),
        ("#_scxml_split('a,b', ',')", ScriptValue::Int(2)),
        ("String(42)", ScriptValue::String("42".into())),
        ("Number('42')", ScriptValue::Int(42)),
        ("Boolean(0)", ScriptValue::Bool(false)),
        ("JSON.stringify(1)", ScriptValue::String("1".into())),
    ] {
        let answered = engine
            .evaluate_expression(session, expression)
            .unwrap_or_else(|e| panic!("`{expression}` after the chunked load: {e}"));
        assert_eq!(
            answered, expected,
            "`{expression}` answered {answered:?} after the chunks were loaded the \
             way C11 loads them"
        );
    }
}
