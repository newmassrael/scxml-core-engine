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

use sce_build::filters::{c11_lua_chunks, ECMA_SEMANTICS_LUA, JSON_BUILTINS_LUA};

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

/// The same question of the OTHER shared asset, and of the property the
/// chunker relies on rather than of one file's current byte offsets.
///
/// `ecma_semantics.lua` carries 25 blank lines inside function bodies
/// (measured 2026-08-18). While the chunker broke at every blank line, whether
/// one of those became a chunk boundary — and deleted the function it was
/// inside — depended on the running byte count, so adding a paragraph anywhere
/// earlier in the file could do it. Asserting per-file compilation is what
/// makes that a red instead of a silent loss.
#[test]
fn every_c11_chunk_of_each_shared_asset_compiles_alone() {
    for (name, source) in [
        ("json_builtins.lua", JSON_BUILTINS_LUA),
        ("ecma_semantics.lua", ECMA_SEMANTICS_LUA),
    ] {
        let chunks = c11_lua_chunks(source);
        assert!(
            chunks.len() > 1,
            "{name}: the splitter returned one chunk, so this is checking nothing"
        );
        let lua = mlua::Lua::new();
        for (i, chunk) in chunks.iter().enumerate() {
            lua.load(chunk.as_str())
                .into_function()
                .unwrap_or_else(|e| {
                    panic!(
                        "{name}: chunk {i} of {} does not compile on its own — C11 \
                         loads it as its own `luaL_dostring` and would silently \
                         lose it: {e}",
                        chunks.len()
                    )
                });
        }
    }
}

/// A blank line inside a function is harmless BY CONSTRUCTION, not by luck.
///
/// The input below forces the old rule to break inside a body: one long
/// definition, well past the chunker's size threshold, with a blank line in
/// the middle of it. Breaking at that blank line produces two fragments that
/// neither compile; the depth guard keeps them together.
#[test]
fn a_blank_line_inside_a_function_is_not_a_chunk_boundary() {
    // Assignments rather than `local`s: Lua caps a function at 200 locals,
    // and hitting that cap would fail this test for a reason that has nothing
    // to do with chunking.
    let filler = "    _pad[#_pad + 1] = 1\n".repeat(400);
    let source = format!(
        "function _probe_a()\n{filler}\n{filler}    return 1\nend\n\nfunction _probe_b()\n    return 2\nend\n"
    );
    let chunks = c11_lua_chunks(&source);
    let lua = mlua::Lua::new();
    for (i, chunk) in chunks.iter().enumerate() {
        lua.load(chunk.as_str())
            .into_function()
            .unwrap_or_else(|e| {
                panic!(
                    "chunk {i} of {} split a function at a blank line inside its \
                 body: {e}",
                    chunks.len()
                )
            });
    }
}

/// Chunks stay under the size the splitter exists to respect.
///
/// C99 guarantees only 4095 characters in a string literal after
/// concatenation, and each chunk becomes exactly one such literal in the C11
/// emit. The depth guard lets a chunk grow past the 2500-byte target — that is
/// its whole point, since it will not cut a function in half to hit it — so
/// the target is not the invariant and this ceiling is. Without it a broken
/// depth count is invisible: mistaking one definition for an open body merges
/// every chunk after it into one, and nothing else in this file notices.
#[test]
fn no_c11_chunk_exceeds_the_c99_literal_floor() {
    const C99_FLOOR: usize = 4095;
    for (name, source) in [
        ("json_builtins.lua", JSON_BUILTINS_LUA),
        ("ecma_semantics.lua", ECMA_SEMANTICS_LUA),
    ] {
        for (i, chunk) in c11_lua_chunks(source).iter().enumerate() {
            assert!(
                chunk.len() < C99_FLOOR,
                "{name}: chunk {i} is {} bytes, over the {C99_FLOOR} a C string \
                 literal is guaranteed to hold — either one definition grew past \
                 it, or the depth guard stopped finding a boundary",
                chunk.len()
            );
        }
    }
}

/// The depth count the guard above relies on returns to zero for each shared
/// asset — so a definition written in a style the counter cannot see (an
/// indented `end`, a `function` behind an assignment) is a red here rather
/// than a boundary in the wrong place later.
#[test]
fn the_chunker_tracks_function_depth_in_both_shared_assets() {
    for (name, source) in [
        ("json_builtins.lua", JSON_BUILTINS_LUA),
        ("ecma_semantics.lua", ECMA_SEMANTICS_LUA),
    ] {
        let mut depth: i64 = 0;
        let mut lowest: i64 = 0;
        for line in source.lines() {
            if line.starts_with("function ") || line.starts_with("local function ") {
                // A one-liner opens and closes on the same line.
                if !line.ends_with(" end") {
                    depth += 1;
                }
            } else if line == "end" {
                depth -= 1;
                lowest = lowest.min(depth);
            }
        }
        assert_eq!(depth, 0, "{name}: unbalanced column-0 function/end count");
        assert_eq!(lowest, 0, "{name}: a column-0 `end` closed nothing");
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
