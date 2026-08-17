// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// ECMA-262 15.12.2 / RFC 8259: `sce/include/scripting/json_builtins.lua`
// parses JSON, and refuses what is not JSON.
//
// The file is loaded here RAW — into a bare Lua state with nothing installed
// over it — because that is how one backend consumes it. The Rust and Go
// engines install a native `JSON.parse` on top; C11 does not, and C11 decodes
// every arriving `_event.data` through this reader. Measured 2026-08-17: a
// mutation to this file left the whole Rust payload suite green, so the
// engine tests cannot answer for it and this file is where the question goes.
//
// What it used to do is why the question matters. `JSON.parse` rewrote its
// argument into Lua source and `load`ed it, so `2 + 3` "parsed" to 5 — and
// the receiving boundary that decodes through it was therefore EVALUATING a
// payload that a host, a peer session or an HTTP sender wrote. §scxml-B-2-8-1
// gives that payload three readings and "whatever Lua makes of it" is not
// among them.
//
// The last test is about packaging rather than JSON: the C11 backend loads
// this file as a sequence of `luaL_dostring` chunks split on blank lines, so
// a function long enough to be split silently fails to compile there and
// leaves `JSON.parse` nil at run time. That happened while this parser was
// being written, and the C11 suite reported it as nine unrelated failures.

use sce_build::filters::{c11_lua_chunks, JSON_BUILTINS_LUA};

/// The shared file in a bare Lua state — no engine, no overrides.
fn shared_reader() -> mlua::Lua {
    let lua = mlua::Lua::new();
    lua.load(JSON_BUILTINS_LUA)
        .exec()
        .expect("the shared JSON file loads");
    lua
}

fn parses_to_nil(lua: &mlua::Lua, text: &str) -> bool {
    lua.globals().set("_probe", text).expect("bind");
    lua.load("local v, ok = JSON._parse_document(_probe); return (v == nil) and not ok")
        .eval::<bool>()
        .expect("the reader answers")
}

/// ECMA-262 15.12.2 gives `JSON.parse` one return, and a second one is not
/// invisible: `return JSON.parse(x)` in tail position propagates every value
/// a Lua function returns, and the Python backend's lupa bridge surfaces that
/// as a tuple. The two-value contract therefore lives on the internal
/// `_parse_document` and this pins the author-facing shape.
#[test]
fn the_author_facing_parse_returns_one_value() {
    let lua = shared_reader();
    lua.globals().set("_probe", r#"{"a":1}"#).expect("bind");
    let count: usize = lua
        .load("return select('#', JSON.parse(_probe))")
        .eval()
        .expect("the reader answers");
    assert_eq!(count, 1, "`JSON.parse` returned {count} values");
}

/// Text that is a valid expression in Lua but is not a JSON document.
#[test]
fn a_lua_expression_is_not_json() {
    let lua = shared_reader();
    for text in ["2 + 3", "#'abc'", "{1, 2}", "nil", "1 == 1"] {
        assert!(
            parses_to_nil(&lua, text),
            "`{text}` is Lua, not JSON — a reader that accepts it is evaluating"
        );
    }
}

/// RFC 8259 is one value and nothing after it.
#[test]
fn trailing_text_is_refused() {
    let lua = shared_reader();
    for text in [r#"{"a":1} trailing"#, "1 2", r#""x" ,"#, "[1,2] ["] {
        assert!(parses_to_nil(&lua, text), "`{text}` has trailing content");
    }
}

/// Malformed JSON is refused rather than half-read.
#[test]
fn malformed_json_is_refused() {
    let lua = shared_reader();
    for text in [
        "{",
        "[1,",
        r#"{"a" 1}"#,
        r#"{a:1}"#,
        "+1",
        "01",
        "1.",
        ".5",
        "1e",
        r#""unterminated"#,
    ] {
        assert!(parses_to_nil(&lua, text), "`{text}` is not JSON");
    }
}

/// The other direction, so "refuses everything" cannot pass as correct.
#[test]
fn every_json_kind_is_read() {
    let lua = shared_reader();
    let cases: &[(&str, &str, &str)] = &[
        ("object member", r#"{"a":{"b":[1,2]}}"#, "v.a.b[2] == 2"),
        ("bare string", r#""plain""#, "v == 'plain'"),
        ("bare number", "-1.5e2", "v == -150"),
        ("fraction", "1.25", "v == 1.25"),
        ("true", "true", "v == true"),
        ("false", "false", "v == false"),
        (
            "empty object",
            "{}",
            "type(v) == 'table' and next(v) == nil",
        ),
        ("empty array", "[]", "type(v) == 'table' and #v == 0"),
        ("escapes", r#""a\"b\\c\/d\te""#, r#"v == 'a"b\\c/d\te'"#),
        ("whitespace", " {\n \"a\" : 1 }\t", "v.a == 1"),
        // 1-based, the way every other sequence in this datamodel is.
        ("array indexing", "[10,20,30]", "v[1] == 10 and v[3] == 30"),
    ];
    for (name, text, check) in cases {
        lua.globals().set("_probe", *text).expect("bind");
        let ok: bool = lua
            .load(format!(
                "local v, ok = JSON._parse_document(_probe); if not ok then return false end return {check}"
            ))
            .eval()
            .unwrap_or_else(|e| panic!("{name}: the reader raised on `{text}`: {e}"));
        assert!(ok, "{name}: `{text}` did not read as JSON");
    }
}

/// JSON `null` is a successful parse of an absent value, which is why the
/// reader answers with two returns: a caller that cannot tell it from a
/// refusal would fall back to the string reading and hand the document the
/// text "null".
#[test]
fn null_parses_and_says_it_did() {
    let lua = shared_reader();
    lua.globals().set("_probe", "null").expect("bind");
    let (value_is_nil, ok): (bool, bool) = lua
        .load("local v, ok = JSON._parse_document(_probe); return v == nil, ok")
        .eval()
        .expect("the reader answers");
    assert!(value_is_nil, "JSON null is the datamodel's absent value");
    assert!(ok, "and it is a successful parse, not a refusal");
}

/// RFC 8259 §7's escape, including the surrogate pair an encoder emits for a
/// character outside the BMP.
#[test]
fn unicode_escapes_decode_to_utf8() {
    let lua = shared_reader();
    let b = '\\';
    for (text, want) in [
        (format!(r#""{b}u0041{b}ubd81""#), "A북"),
        (format!(r#""{b}ud83d{b}ude00""#), "😀"),
    ] {
        lua.globals().set("_probe", text.as_str()).expect("bind");
        let got: String = lua
            .load("return JSON.parse(_probe)")
            .eval()
            .expect("the reader answers");
        assert_eq!(got, want, "`{text}` did not decode to UTF-8");
    }
}

/// The C11 packaging constraint, asked of the file rather than assumed.
///
/// C11 emits this library as one `luaL_dostring` per chunk, and the chunks
/// are split on blank lines. A function with a blank line inside it is
/// therefore split across two calls, each of which fails to COMPILE — and
/// nothing reports it, because the emitted calls are `(void)`-cast. The
/// symptom is `JSON.parse` being nil at run time, which surfaces as failures
/// in whatever happens to decode a payload next.
///
/// Compiled rather than executed, one state per chunk: the chunks reference
/// each other's globals by design (`JSON = {}` is in the first and every
/// function after it indexes that table), so running one alone is expected to
/// fail and would say nothing about the split.
#[test]
fn every_c11_chunk_of_the_shared_reader_compiles_alone() {
    let chunks = c11_lua_chunks(JSON_BUILTINS_LUA);
    assert!(
        chunks.len() > 1,
        "the splitter returned one chunk, so this test is checking nothing"
    );
    let lua = mlua::Lua::new();
    for (i, chunk) in chunks.iter().enumerate() {
        lua.load(chunk.as_str())
            .into_function()
            .unwrap_or_else(|e| {
                panic!(
                    "chunk {i} of {} does not compile on its own — C11 loads it \
                     as its own `luaL_dostring` and would silently lose it: {e}",
                    chunks.len()
                )
            });
    }
}

/// And the whole point of loading them: the reader is callable after the
/// chunks are executed in order, the way C11 executes them.
#[test]
fn the_reader_works_after_being_loaded_in_c11_chunks() {
    let lua = mlua::Lua::new();
    for chunk in c11_lua_chunks(JSON_BUILTINS_LUA) {
        lua.load(chunk.as_str()).exec().expect("chunk executes");
    }
    lua.globals().set("_probe", r#"{"a":1}"#).expect("bind");
    let ok: bool = lua
        .load("local v, ok = JSON._parse_document(_probe); return ok and v.a == 1")
        .eval()
        .expect("the reader answers");
    assert!(ok, "`JSON.parse` is not usable after a chunked load");
}
