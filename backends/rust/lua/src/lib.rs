// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! # SCE Rust Lua — Lua 5.4 script engine for SCXML
//!
//! Implements [`sce_rust_runtime::IScriptEngine`] using `mlua` (Lua 5.4, vendored).
//! Matches the C++ `LuaEngine` at `sce/src/scripting/LuaEngine.cpp` and the Kotlin
//! `LuaScriptEngine` at `sce-kotlin-lua/.../LuaScriptEngine.kt`.
//!
//! ## ECMAScript compatibility
//!
//! W3C SCXML mandates the `ecmascript` datamodel. The code generator invokes a
//! Python-hosted ECMAScript-to-Lua transformer at codegen time, so by the time
//! expressions reach this engine they are already valid Lua 5.4 source. This
//! engine therefore just compiles and executes Lua — it does not know about
//! JavaScript syntax.
//!
//! ## Session model
//!
//! Each SCXML state machine instance gets its own `Session` (isolated `mlua::Lua`
//! VM with its own globals, variables, and `_event` table). Session isolation
//! matches the C++ `ISessionLifecycle` contract.

mod dom;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use dom::{XmlDoc, XmlRef};

use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::scripting::{
    IScriptEngine, NativeMethod, ScriptError, ScriptResult, ScriptValue,
};

// ═══════════════════════════════════════════════════════════════════════════
// W3C SCXML: Undeclared variable detection (C++ LuaEngine parity)
// JavaScript throws ReferenceError for undeclared variables; Lua returns nil.
// ═══════════════════════════════════════════════════════════════════════════

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

/// Check if a single identifier is undeclared (not a keyword, not in declared_vars,
/// and not a Lua standard library global like `math`, `string`, `table`).
fn is_undeclared_identifier(name: &str, declared_vars: &HashSet<String>, lua: &Lua) -> bool {
    if LUA_KEYWORDS.contains(&name) {
        return false;
    }
    if declared_vars.contains(name) {
        return false;
    }
    // Check if it's a Lua standard library global
    let is_nil: bool = lua
        .globals()
        .get::<LuaValue>(name)
        .map(|v| matches!(v, LuaValue::Nil))
        .unwrap_or(true);
    is_nil
}

/// Detect undeclared variable references in simple expressions.
/// Handles both simple identifiers (`Var1`) and member access (`Var1.bar`, `Var1["key"]`).
fn is_undeclared_simple_variable(expr: &str, declared_vars: &HashSet<String>, lua: &Lua) -> bool {
    if expr.is_empty() {
        return false;
    }
    let first = expr.as_bytes()[0];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    // Extract base identifier (before first '.' or '[')
    let base_end = expr
        .bytes()
        .position(|b| !b.is_ascii_alphanumeric() && b != b'_')
        .unwrap_or(expr.len());
    if base_end == 0 {
        return false;
    }
    let base_name = &expr[..base_end];
    is_undeclared_identifier(base_name, declared_vars, lua)
}

/// Refcounted SCXML `In(stateId)` predicate (W3C SCXML 5.9.2).
///
/// Internally `Arc` rather than `Box` — the same callback may be cloned into
/// multiple Lua closures registered against a single session.
type SharedStateQuery = Arc<dyn Fn(&str) -> bool + Send + Sync>;

/// Refcounted Rust function exposed to Lua via `register_global_function`.
///
/// `Arc`-wrapped so a single registration can be cloned into every session
/// without re-boxing on each `createSession`.
type SharedNativeMethod = Arc<dyn Fn(&[ScriptValue]) -> ScriptValue + Send + Sync>;

// ═══════════════════════════════════════════════════════════════════════════
// Session — one per state machine instance
// ═══════════════════════════════════════════════════════════════════════════

struct Session {
    lua: Lua,
    declared_vars: HashSet<String>,
    state_query_callback: Option<SharedStateQuery>,
}

// SAFETY: Session is only ever accessed through `LuaEngine::sessions: Mutex<HashMap<_, Session>>`,
// so concurrent access is impossible. mlua::Lua is !Sync, but the Mutex guarantees exclusive access.
// The engine is single-threaded in practice (W3C SCXML macrostep loop runs on one thread).
unsafe impl Send for Session {}
unsafe impl Sync for Session {}

// ═══════════════════════════════════════════════════════════════════════════
// LuaEngine — the IScriptEngine implementation
// ═══════════════════════════════════════════════════════════════════════════

/// The Lua 5.4 script engine.
///
/// Session-per-SM isolation with `mlua::Lua` per session, matching C++
/// `LuaEngine` and Kotlin `LuaScriptEngine`.
pub struct LuaEngine {
    sessions: Mutex<HashMap<String, Session>>,
    initialized: Mutex<bool>,
    /// Global functions registered via `register_global_function()`.
    /// Stored here so they can be registered into newly created sessions.
    global_functions: Mutex<HashMap<String, SharedNativeMethod>>,
}

impl LuaEngine {
    /// Construct a new LuaEngine.
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            initialized: Mutex::new(false),
            global_functions: Mutex::new(HashMap::new()),
        }
    }

    /// Register ECMAScript-compatible builtin functions into a Lua session.
    ///
    /// Matches the C++ `LuaEngine::setupBuiltins()` and Kotlin
    /// `LuaScriptEngine.setupSession()` — registers global helpers that
    /// provide ECMAScript semantics on top of Lua 5.4.
    fn setup_builtins(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();

        // W3C SCXML B.2: `_scxml_truthy`, `_typeof`, `_isArray`, `_indexOf`,
        // `_concat`, `parseInt` and `parseFloat` were seven Rust closures
        // here, one of six implementations of the same seven meanings. They
        // are in the shared sce/include/scripting/ecma_semantics.lua now,
        // loaded at the end of this function.
        //
        // The drift they were predicted to accumulate had arrived. Measured
        // 2026-08-16 against tests/ecmascript/ecma262_semantics.json, once
        // every Lua backend had a reader: Go's `_indexOf` and `_concat` had
        // no Array branch at all, Python called `typeof [1,2,3]` "function",
        // and this copy read `indexOf`'s second argument as the search
        // START on a table while ignoring it entirely on a string.

        // _NULL / _UNDEFINED sentinel values for array literal null/undefined preservation
        globals.set("_NULL", LuaValue::Nil)?;
        globals.set("_UNDEFINED", LuaValue::Nil)?;

        // String metatable: make + work as concatenation via __add
        lua.load(
            r#"
            local mt = getmetatable("")
            if mt then
                mt.__add = function(a, b)
                    return tostring(a) .. tostring(b)
                end
            end
        "#,
        )
        .exec()?;

        // W3C SCXML B.2: the ECMAScript operators Lua does not share —
        // `+`, `==`, and the bitwise family, which coerce their operands
        // where Lua either refuses or answers differently. Single Source of
        // Truth at sce/include/scripting/ecma_semantics.lua: the generated
        // code calls these by name on every backend, so one definition is
        // what keeps the six engines from disagreeing about what `==` means.
        lua.load(include_str!(
            "../../../../sce/include/scripting/ecma_semantics.lua"
        ))
        .exec()?;

        // W3C SCXML B.2: JSON.stringify / JSON.parse (Single Source of Truth)
        // Shared with C++ LuaEngine via sce/include/scripting/json_builtins.lua
        lua.load(include_str!(
            "../../../../sce/include/scripting/json_builtins.lua"
        ))
        .exec()?;

        // …except for `parse`, which the shared file implements as a textual
        // JSON→Lua rewrite fed to `load()`. That makes its accepted input
        // "JSON that also happens to be Lua", and the two are not the same
        // language: `\uXXXX` and `\/` are legal JSON escapes Lua's lexer
        // rejects. Overriding it with the same decoder `_event.data` uses
        // means a document reaching a script through `JSON.parse` and the
        // same document arriving as event data cannot disagree — which they
        // could while one was decoded and the other rewritten. Go's engine
        // already overrides `parse` for this reason; this is the Rust half.
        // `stringify` stays shared: emitting JSON has no such mismatch.
        {
            let json_table: LuaTable = lua.globals().get("JSON")?;
            let parse_fn = lua.create_function(|lua, text: Option<LuaString>| {
                let Some(text) = text else {
                    return Ok(LuaValue::Nil);
                };
                // A payload that is not JSON is `nil`, not an error — the
                // shared implementation answers that way and callers test the
                // result rather than pcall it.
                match text.to_str() {
                    Ok(s) => Ok(json_to_lua_value(lua, &s).unwrap_or(LuaValue::Nil)),
                    Err(_) => Ok(LuaValue::Nil),
                }
            })?;
            json_table.set("parse", parse_fn)?;
        }

        // `Object.keys` moved to the shared semantics file with the rest of
        // the engine vocabulary. This copy enumerated in `pairs` order, which
        // is the hash layout rather than an order at all; the shared one
        // sorts, so the six backends answer the same array.

        Ok(())
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Conversion helpers: ScriptValue <-> LuaValue
// ═══════════════════════════════════════════════════════════════════════════

/// A value as Lua source — this engine's answer to
/// [`IScriptEngine::to_script_literal`].
///
/// Every spelling below is Lua's and nobody else's: `nil` for absence, a
/// braced list for a sequence, `["k"] = v` for a keyed table. The engine that
/// reads it back does so through `load("return " .. literal)`, so the text has
/// to be a Lua expression, and a value that stayed engine-neutral could not
/// have known that.
fn lua_literal(val: &ScriptValue) -> String {
    match val {
        ScriptValue::Null | ScriptValue::Undefined => "nil".to_string(),
        ScriptValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        ScriptValue::Int(i) => i.to_string(),
        ScriptValue::Double(f) => {
            if f.fract() == 0.0 && f.is_finite() {
                // Lua 5.4 keeps the float subtype only if the literal has a
                // decimal point; `5` would come back as an integer.
                format!("{:.1}", f)
            } else {
                format!("{}", f)
            }
        }
        ScriptValue::String(s) => format!("\"{}\"", escape_lua_string(s)),
        ScriptValue::Array(arr) => {
            let items: Vec<String> = arr.iter().map(lua_literal).collect();
            format!("{{{}}}", items.join(", "))
        }
        ScriptValue::Object(map) => {
            // Keys sorted: a table literal that reordered itself per run would
            // make identical values compare unequal as text.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let items: Vec<String> = keys
                .iter()
                .map(|k| format!("[\"{}\"] = {}", escape_lua_string(k), lua_literal(&map[*k])))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        // The DOM crosses as the document text it was parsed from; the
        // receiving side parses it again.
        ScriptValue::Dom(s) => format!("\"{}\"", escape_lua_string(s)),
    }
}

fn escape_lua_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn script_value_to_lua(lua: &Lua, val: &ScriptValue) -> LuaResult<LuaValue> {
    match val {
        ScriptValue::Null | ScriptValue::Undefined => Ok(LuaValue::Nil),
        ScriptValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        ScriptValue::Int(i) => Ok(LuaValue::Integer(*i)),
        ScriptValue::Double(f) => Ok(LuaValue::Number(*f)),
        ScriptValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        ScriptValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.raw_set(i + 1, script_value_to_lua(lua, item)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        ScriptValue::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), script_value_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        // A `ScriptValue::Dom` is a caller that already decided the content is
        // a document, so a refusal leaves the value nil exactly as before.
        ScriptValue::Dom(xml) => {
            Ok(push_xml_as_userdata(lua, xml.as_str())?.unwrap_or(LuaValue::Nil))
        }
    }
}

fn lua_value_to_script(val: &LuaValue) -> ScriptValue {
    match val {
        LuaValue::Nil => ScriptValue::Null,
        LuaValue::Boolean(b) => ScriptValue::Bool(*b),
        LuaValue::Integer(i) => ScriptValue::Int(*i),
        LuaValue::Number(f) => {
            // Normalize integer-valued floats to Int
            if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 {
                ScriptValue::Int(*f as i64)
            } else {
                ScriptValue::Double(*f)
            }
        }
        LuaValue::String(s) => ScriptValue::String(s.to_string_lossy().to_string()),
        LuaValue::Table(t) => {
            // Heuristic: sequence table (1..n keys) = Array, else Object
            let len = t.raw_len();
            if len > 0 {
                let mut arr = Vec::with_capacity(len);
                for i in 1..=len {
                    if let Ok(v) = t.raw_get::<LuaValue>(i) {
                        arr.push(lua_value_to_script(&v));
                    }
                }
                ScriptValue::Array(arr)
            } else {
                let mut map = HashMap::new();
                let mut is_empty = true;
                if let Ok(pairs) = t
                    .clone()
                    .pairs::<LuaValue, LuaValue>()
                    .collect::<Result<Vec<_>, _>>()
                {
                    for (k, v) in pairs {
                        is_empty = false;
                        let key = match &k {
                            LuaValue::String(s) => s.to_string_lossy().to_string(),
                            LuaValue::Integer(n) => n.to_string(),
                            _ => continue,
                        };
                        map.insert(key, lua_value_to_script(&v));
                    }
                }
                if is_empty {
                    // Empty table -> empty Array (matches ECMAScript [] -> {})
                    ScriptValue::Array(vec![])
                } else {
                    ScriptValue::Object(map)
                }
            }
        }
        LuaValue::Function(_) => ScriptValue::String("[function]".to_string()),
        _ => ScriptValue::Null,
    }
}

/// W3C SCXML B.2 test 578: decode a JSON event payload into a Lua value.
///
/// Returns `None` when the payload is not JSON, which is the caller's signal to
/// fall through to the plain-string treatment §B.2 gives non-structured data.
///
/// This replaced a rewrite that turned JSON text into Lua *source* and handed
/// it to `load()`. That design makes the accepted input set "JSON that also
/// happens to be Lua", and the gap is not small: `\uXXXX` and `\/` are legal
/// JSON escapes Lua's lexer rejects, and `[1, 2]` is Lua's index syntax, not a
/// table constructor. Each of those made `load()` fail, and the failure was not
/// reported — it fell through to the string branch, so `_event.data` quietly
/// stopped being a table and every field read on it came back nil. Decoding
/// removes the whole class rather than patching its instances.
fn json_to_lua_value(lua: &Lua, json: &str) -> Option<LuaValue> {
    let parsed: serde_json::Value = serde_json::from_str(json).ok()?;
    json_value_to_lua(lua, &parsed).ok()
}

/// Build the Lua representation of one decoded JSON node.
fn json_value_to_lua(lua: &Lua, value: &serde_json::Value) -> LuaResult<LuaValue> {
    match value {
        // W3C SCXML B.2: JSON null is the datamodel's null, which Lua spells
        // `nil`. Assigned into a table this removes the key, matching the
        // ECMAScript datamodel's "reading an absent member yields undefined".
        serde_json::Value::Null => Ok(LuaValue::Nil),
        serde_json::Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        serde_json::Value::Number(n) => {
            // Integers stay integers. Lua 5.4 has a distinct integer subtype
            // and `ScriptValue` reports it separately, so a payload's `200`
            // must not arrive as `200.0` — a guard comparing against an
            // integer literal would still pass, but anything that renders the
            // value back into text would not round-trip.
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else {
                Ok(LuaValue::Number(n.as_f64().unwrap_or(f64::NAN)))
            }
        }
        serde_json::Value::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        serde_json::Value::Array(items) => {
            let table = lua.create_table()?;
            // Lua sequences are 1-based, which is what `#t`, `ipairs` and the
            // generated `foreach` lowering all assume.
            for (index, item) in items.iter().enumerate() {
                table.set(index + 1, json_value_to_lua(lua, item)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        serde_json::Value::Object(fields) => {
            let table = lua.create_table()?;
            for (key, item) in fields {
                table.set(lua.create_string(key)?, json_value_to_lua(lua, item)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

/// Parse `xml_content` into a full DOM tree and push the resulting document as
/// Lua userdata, or answer `None` if the content is not a valid XML document.
///
/// `None` rather than `LuaValue::Nil`, because the two callers want opposite
/// things from a refusal and the old signature could only give them one. The
/// `<data>` paths want the variable left unbound — the content there was
/// parsed by the SCXML parser before it reached this crate, so a refusal is
/// this engine's own invariant breaking. The `_event.data` path must fall
/// through instead: §scxml-B-2-8-1 conditions the DOM reading on the content
/// BEING a document and closes with "Otherwise, the Processor MUST treat the
/// content as a space-normalized string literal", so a payload that merely
/// opens with `<` has a reading below this one and answering nil drops it.
/// Mirrors cpp `LuaDOMBinding::pushDOMObject` (sce/src/scripting/LuaDOMBinding.cpp:74).
fn push_xml_as_userdata(lua: &Lua, xml_content: &str) -> LuaResult<Option<LuaValue>> {
    let doc = Arc::new(XmlDoc::parse(xml_content));
    if !doc.is_valid() {
        return Ok(None);
    }
    let xref = match XmlRef::document(doc) {
        Some(r) => r,
        None => return Ok(None),
    };
    let ud = lua.create_userdata(xref)?;
    Ok(Some(LuaValue::UserData(ud)))
}

fn map_lua_err(e: mlua::Error) -> ScriptError {
    match &e {
        mlua::Error::SyntaxError { message, .. } => ScriptError::SyntaxError(message.clone()),
        mlua::Error::RuntimeError(msg) => ScriptError::RuntimeError(msg.clone()),
        _ => ScriptError::EngineError(e.to_string()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// IScriptEngine implementation
// ═══════════════════════════════════════════════════════════════════════════

impl IScriptEngine for LuaEngine {
    fn execute_script(&self, session_id: &str, script: &str) -> ScriptResult<ScriptValue> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let result = session
            .lua
            .load(script)
            .eval::<LuaValue>()
            .map_err(map_lua_err)?;
        Ok(lua_value_to_script(&result))
    }

    fn to_script_literal(&self, value: &ScriptValue) -> String {
        lua_literal(value)
    }

    fn evaluate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<ScriptValue> {
        if expression.is_empty() {
            return Ok(ScriptValue::Null);
        }
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;

        // W3C SCXML: Detect undeclared simple variable references (C++ LuaEngine parity)
        // JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
        if is_undeclared_simple_variable(expression, &session.declared_vars, &session.lua) {
            return Err(ScriptError::RuntimeError(format!(
                "ReferenceError: {} is not defined",
                expression
            )));
        }

        // Wrap as return expression for Lua evaluation
        let lua_expr = format!("return {}", expression);
        let result = session
            .lua
            .load(&lua_expr)
            .eval::<LuaValue>()
            .or_else(|first_err| {
                // Fallback: try executing as statement (e.g., assignment)
                log::debug!(
                    "evaluate_expression: 'return {}' failed ({}), trying as statement",
                    expression,
                    first_err
                );
                session.lua.load(expression).eval::<LuaValue>()
            })
            .map_err(map_lua_err)?;
        Ok(lua_value_to_script(&result))
    }

    fn validate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<bool> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let lua_expr = format!("return {}", expression);
        match session.lua.load(&lua_expr).into_function() {
            Ok(_) => Ok(true),
            Err(_) => match session.lua.load(expression).into_function() {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            },
        }
    }

    fn set_variable(&self, session_id: &str, name: &str, value: ScriptValue) -> ScriptResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let lua_val = script_value_to_lua(&session.lua, &value).map_err(map_lua_err)?;
        session
            .lua
            .globals()
            .set(name, lua_val)
            .map_err(map_lua_err)?;
        session.declared_vars.insert(name.to_string());
        Ok(())
    }

    fn get_variable(&self, session_id: &str, name: &str) -> ScriptResult<ScriptValue> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let val: LuaValue = session.lua.globals().get(name).map_err(map_lua_err)?;
        Ok(lua_value_to_script(&val))
    }

    fn set_variable_as_dom(
        &self,
        session_id: &str,
        name: &str,
        xml_content: &str,
    ) -> ScriptResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        // Parse into a full DOM tree (cpp pugixml mirror) and bind as
        // userdata.  Parse failure yields nil — same observable as cpp
        // `LuaDOMBinding::pushDOMObject` on `XMLDocument::isValid()` =
        // false, which leaves the var unbound rather than raising.
        let value = push_xml_as_userdata(&session.lua, xml_content)
            .map_err(map_lua_err)?
            .unwrap_or(LuaValue::Nil);
        session
            .lua
            .globals()
            .set(name, value)
            .map_err(map_lua_err)?;
        session.declared_vars.insert(name.to_string());
        Ok(())
    }

    fn has_variable(&self, session_id: &str, name: &str) -> bool {
        let sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get(session_id) {
            session.declared_vars.contains(name)
        } else {
            false
        }
    }

    fn is_variable_pre_initialized(&self, session_id: &str, name: &str) -> bool {
        self.has_variable(session_id, name)
    }

    fn setup_system_variables(
        &self,
        session_id: &str,
        session_name: &str,
        io_processors: &[IoProcessorDescriptor],
    ) -> ScriptResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let globals = session.lua.globals();

        // _sessionid
        globals.set("_sessionid", session_id).map_err(map_lua_err)?;
        session.declared_vars.insert("_sessionid".to_string());

        // _name
        globals.set("_name", session_name).map_err(map_lua_err)?;
        session.declared_vars.insert("_name".to_string());

        // §scxml-C-1-1 / §scxml-C-2-3: one entry per processor the deployment
        // supports, each with a 'location' field holding the address that
        // reaches this session through it. Both names and locations are
        // decided by `helpers::io_processors::build`, so this engine's view of
        // `_ioprocessors` matches every other backend's.
        let io_table = session.lua.create_table().map_err(map_lua_err)?;
        for processor in io_processors {
            let proc_entry = session.lua.create_table().map_err(map_lua_err)?;
            proc_entry
                .set("location", processor.location.as_str())
                .map_err(map_lua_err)?;
            io_table
                .set(processor.name.as_str(), proc_entry)
                .map_err(map_lua_err)?;
        }
        globals
            .set("_ioprocessors", io_table)
            .map_err(map_lua_err)?;
        session.declared_vars.insert("_ioprocessors".to_string());

        // _event is NOT initialized here — W3C SCXML B.2: _event is unbound
        // before any event is processed (test 319). It gets set via
        // set_current_event() when an event is actually being processed.

        Ok(())
    }

    fn set_current_event(
        &self,
        session_id: &str,
        args: sce_rust_runtime::SetCurrentEventArgs<'_>,
    ) -> ScriptResult<()> {
        let event_name = args.event_name;
        let event_data = args.event_data;
        let event_type = args.event_type;
        let send_id = args.send_id;
        let origin = args.origin;
        let origin_type = args.origin_type;
        let invoke_id = args.invoke_id;

        let sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;

        let event_table = session.lua.create_table().map_err(map_lua_err)?;
        event_table.set("name", event_name).map_err(map_lua_err)?;

        // Parse event_data as Lua value if non-empty
        if !event_data.is_empty() {
            // W3C SCXML B.2: XML data → DOM object (test 561).
            //
            // The leading `<` is a GUESS about which reading applies, not the
            // reading itself. §scxml-B-2-8-1 conditions this rung on the
            // content being one — "if the Processor can interpret the content
            // as a valid XML document, it MUST create the corresponding DOM
            // structure" — and closes with "Otherwise, the Processor MUST
            // treat the content as a space-normalized string literal". So a
            // guess that turns out wrong falls through to the rungs below.
            //
            // It used to end here with nil, which is the shape that is easy to
            // write and hard to notice: nothing sends a malformed document on
            // purpose. Then the repository filled `_event.data` in at 192
            // `error.*` raise sites with messages that name the failing
            // construct — `<assign> to detail failed` — and every one of them
            // opens with `<`. Three of the eight engines delivered nil.
            let dom_value = if event_data.trim_start().starts_with('<') {
                push_xml_as_userdata(&session.lua, event_data).map_err(map_lua_err)?
            } else {
                None
            };
            if let Some(dom_value) = dom_value {
                event_table.set("data", dom_value).map_err(map_lua_err)?;
            } else {
                // §scxml-B-2-8-1 gives `_event.data` three readings and no
                // fourth: XML becomes a DOM, JSON becomes the value, and
                // anything else becomes a space-normalized string. There used
                // to be a rung above these two — `load("return " .. payload)`,
                // evaluating the payload as Lua source before anything looked
                // at it — and it decided all three of the following, measured
                // 2026-08-17:
                //
                //   * `2 + 3` from a host arrived as the number 5 here and as
                //     the string "2 + 3" on the cpp and Rhino engines, which
                //     read the clause instead. One payload, two answers.
                //   * a payload `(function() ... end)()` RAN, in the session's
                //     own globals. `_event.data` is the one field an SCXML
                //     document takes from outside itself.
                //   * it was load-bearing: `<send>` shipped
                //     `_scxml_params({...})` — Lua source — so this rung was
                //     the deserializer, and 27 tests turned red when it was
                //     removed without moving the sender.
                //
                // The sender now ships JSON (§scxml-B-2-9: data that leaves
                // the data model is serialized to JSON), which is what the cpp
                // engine has always shipped, so the two rungs the clause names
                // are the two that are here.
                match json_to_lua_value(&session.lua, event_data) {
                    Some(val) => {
                        event_table.set("data", val).map_err(map_lua_err)?;
                    }
                    None => {
                        // W3C SCXML B.2 test 562: Fall back to whitespace-normalized string
                        let normalized: String = event_data
                            .split_whitespace()
                            .collect::<Vec<&str>>()
                            .join(" ");
                        event_table.set("data", normalized).map_err(map_lua_err)?;
                    }
                }
            }
        }

        if !event_type.is_empty() {
            event_table.set("type", event_type).map_err(map_lua_err)?;
        }
        if !send_id.is_empty() {
            event_table.set("sendid", send_id).map_err(map_lua_err)?;
        }
        // W3C SCXML 5.10.1: Always set origin/origintype so targetexpr="_event.origin"
        // evaluates to empty string (not nil) when origin is unset (test 336).
        event_table.set("origin", origin).map_err(map_lua_err)?;
        event_table
            .set("origintype", origin_type)
            .map_err(map_lua_err)?;
        if !invoke_id.is_empty() {
            event_table
                .set("invokeid", invoke_id)
                .map_err(map_lua_err)?;
        }

        session
            .lua
            .globals()
            .set("_event", event_table)
            .map_err(map_lua_err)?;
        Ok(())
    }

    fn register_global_function(&self, function_name: &str, callback: NativeMethod) -> bool {
        // 1:1 with C++ LuaEngine::registerGlobalFunction:
        // Store callback, then register in all existing sessions.
        let cb_arc: SharedNativeMethod = Arc::from(callback);
        {
            let mut gf = self.global_functions.lock().unwrap();
            gf.insert(function_name.to_string(), cb_arc.clone());
        }
        let sessions = self.sessions.lock().unwrap();
        let fname = function_name.to_string();
        for session in sessions.values() {
            let cb = cb_arc.clone();
            if let Ok(f) = session.lua.create_function(move |_, args: LuaMultiValue| {
                let script_args: Vec<ScriptValue> =
                    args.into_vec().iter().map(lua_value_to_script).collect();
                let result = cb(&script_args);
                Ok(result.to_bool()) // simplified return
            }) {
                let _ = session.lua.globals().set(fname.as_str(), f);
            }
        }
        true
    }

    fn bind_native_object(
        &self,
        session_id: &str,
        object_name: &str,
        methods: Vec<(String, NativeMethod)>,
    ) -> bool {
        // 1:1 with C++ LuaEngine::bindNativeObject:
        // Create Lua table, bind each method as closure with NativeMethod callback.
        let sessions = self.sessions.lock().unwrap();
        let session = match sessions.get(session_id) {
            Some(s) => s,
            None => return false,
        };
        let table = match session.lua.create_table() {
            Ok(t) => t,
            Err(_) => return false,
        };
        for (name, callback) in methods {
            let cb_arc: SharedNativeMethod = Arc::from(callback);
            let method_fn = match session.lua.create_function(move |_, args: LuaMultiValue| {
                let script_args: Vec<ScriptValue> =
                    args.into_vec().iter().map(lua_value_to_script).collect();
                let result = cb_arc(&script_args);
                Ok(match result {
                    ScriptValue::Bool(b) => LuaValue::Boolean(b),
                    ScriptValue::Int(i) => LuaValue::Integer(i),
                    ScriptValue::Double(f) => LuaValue::Number(f),
                    ScriptValue::Null | ScriptValue::Undefined => LuaValue::Nil,
                    _ => LuaValue::Nil,
                })
            }) {
                Ok(f) => f,
                Err(e) => {
                    log::error!(
                        "bindNativeObject: failed to create method '{}': {}",
                        name,
                        e
                    );
                    return false;
                }
            };
            if table.set(name.as_str(), method_fn).is_err() {
                return false;
            }
        }
        let _ = session.lua.globals().set(object_name, table);
        true
    }

    fn get_engine_info(&self) -> String {
        "Lua 5.4 (mlua)".to_string()
    }

    fn get_memory_usage(&self) -> usize {
        let sessions = self.sessions.lock().unwrap();
        sessions.values().map(|s| s.lua.used_memory()).sum()
    }

    fn collect_garbage(&self) {
        let sessions = self.sessions.lock().unwrap();
        for session in sessions.values() {
            let _ = session.lua.gc_collect();
        }
    }

    fn set_state_query_callback(
        &self,
        session_id: &str,
        callback: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
    ) {
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(session_id) {
            match callback {
                Some(cb) => {
                    // Wrap callback in Arc for shared ownership between Lua closure and session
                    let cb_arc: Arc<dyn Fn(&str) -> bool + Send + Sync> = Arc::from(cb);
                    let cb_for_lua = cb_arc.clone();
                    if let Ok(in_fn) = session.lua.create_function(move |_, state_id: String| {
                        Ok(cb_for_lua(state_id.as_str()))
                    }) {
                        let _ = session.lua.globals().set("In", in_fn);
                    }
                    session.state_query_callback = Some(cb_arc);
                }
                None => {
                    // Unregister In() function
                    let _ = session.lua.globals().set("In", LuaValue::Nil);
                    session.state_query_callback = None;
                }
            }
        }
    }

    fn initialize(&self) -> bool {
        let mut init = self.initialized.lock().unwrap();
        if *init {
            return true;
        }
        *init = true;
        true
    }

    fn shutdown(&self) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.clear();
        let mut init = self.initialized.lock().unwrap();
        *init = false;
    }

    fn is_initialized(&self) -> bool {
        *self.initialized.lock().unwrap()
    }

    fn reset(&self) {
        self.shutdown();
        self.initialize();
    }

    fn create_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        if sessions.contains_key(session_id) {
            return; // Idempotent
        }

        let lua = Lua::new();
        // Setup builtins for this session
        if let Err(e) = Self::setup_builtins(&lua) {
            log::error!(
                "Failed to setup Lua builtins for session {}: {}",
                session_id,
                e
            );
        }

        // Register any global functions added via register_global_function()
        // (1:1 with C++ LuaEngine::createSession which calls registerBuiltins)
        if let Ok(gf) = self.global_functions.lock() {
            for (name, cb) in gf.iter() {
                let cb = cb.clone();
                if let Ok(f) = lua.create_function(move |_, args: LuaMultiValue| {
                    let script_args: Vec<ScriptValue> =
                        args.into_vec().iter().map(lua_value_to_script).collect();
                    let result = cb(&script_args);
                    Ok(result.to_bool())
                }) {
                    let _ = lua.globals().set(name.as_str(), f);
                }
            }
        }

        sessions.insert(
            session_id.to_string(),
            Session {
                lua,
                declared_vars: HashSet::new(),
                state_query_callback: None,
            },
        );
    }

    fn destroy_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(session_id);
    }

    fn has_session(&self, session_id: &str) -> bool {
        let sessions = self.sessions.lock().unwrap();
        sessions.contains_key(session_id)
    }
}
