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

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use mlua::prelude::*;

use sce_rust_runtime::scripting::{
    set_script_engine, IScriptEngine, NativeMethod, ScriptEngineAlreadyRegistered, ScriptError,
    ScriptResult, ScriptValue,
};

// ═══════════════════════════════════════════════════════════════════════════
// W3C SCXML: Undeclared variable detection (C++ LuaEngine parity)
// JavaScript throws ReferenceError for undeclared variables; Lua returns nil.
// ═══════════════════════════════════════════════════════════════════════════

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
    "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
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
    let is_nil: bool = lua.globals().get::<LuaValue>(name)
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
    let base_end = expr.bytes()
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

        // W3C SCXML B.2: ECMAScript truthiness semantics
        // _scxml_truthy(val): nil/false -> false, 0/0.0/"" -> false, else true
        let truthy_fn = lua.create_function(|_, val: LuaValue| {
            Ok(match val {
                LuaValue::Nil => false,
                LuaValue::Boolean(b) => b,
                LuaValue::Integer(i) => i != 0,
                LuaValue::Number(f) => f != 0.0 && !f.is_nan(),
                LuaValue::String(s) => !s.to_string_lossy().is_empty(),
                _ => true,
            })
        })?;
        globals.set("_scxml_truthy", truthy_fn)?;

        // _typeof(val): Lua type -> JS typeof string
        let typeof_fn = lua.create_function(|_, val: LuaValue| {
            Ok(match val {
                LuaValue::Nil => "undefined".to_string(),
                LuaValue::Boolean(_) => "boolean".to_string(),
                LuaValue::Integer(_) | LuaValue::Number(_) => "number".to_string(),
                LuaValue::String(_) => "string".to_string(),
                LuaValue::Function(_) => "function".to_string(),
                LuaValue::Table(_) => "object".to_string(),
                _ => "object".to_string(),
            })
        })?;
        globals.set("_typeof", typeof_fn)?;

        // _isArray(val): Check if value is a Lua sequence table (array)
        let is_array_fn = lua.create_function(|_, val: LuaValue| {
            Ok(match val {
                LuaValue::Table(t) => t.raw_len() > 0 || {
                    // Empty table or pure sequence check
                    let mut count = 0;
                    for pair in t.pairs::<LuaValue, LuaValue>() {
                        if pair.is_err() {
                            break;
                        }
                        count += 1;
                    }
                    count == 0 // Empty table is array-like
                },
                _ => false,
            })
        })?;
        globals.set("_isArray", is_array_fn)?;

        // _indexOf(arr_or_str, search, from): ECMAScript indexOf (returns -1 on not found)
        let index_of_fn = lua.create_function(|_, args: LuaMultiValue| {
            let values: Vec<LuaValue> = args.into_vec();
            if values.is_empty() {
                return Ok(LuaValue::Integer(-1));
            }
            match &values[0] {
                LuaValue::Table(t) => {
                    let search = values.get(1).cloned().unwrap_or(LuaValue::Nil);
                    let from = match values.get(2) {
                        Some(LuaValue::Integer(n)) => *n as usize,
                        Some(LuaValue::Number(n)) => *n as usize,
                        _ => 0,
                    };
                    let len = t.raw_len();
                    for i in (from + 1)..=len {
                        if let Ok(val) = t.raw_get::<LuaValue>(i) {
                            if lua_values_equal(&val, &search) {
                                return Ok(LuaValue::Integer((i as i64) - 1)); // 0-based
                            }
                        }
                    }
                    Ok(LuaValue::Integer(-1))
                }
                LuaValue::String(s) => {
                    let haystack = &s.to_string_lossy();
                    let needle = match values.get(1) {
                        Some(LuaValue::String(s)) => s.to_string_lossy().to_string(),
                        Some(LuaValue::Integer(n)) => n.to_string(),
                        Some(LuaValue::Number(n)) => n.to_string(),
                        _ => return Ok(LuaValue::Integer(-1)),
                    };
                    match haystack.find(&needle) {
                        Some(pos) => Ok(LuaValue::Integer(pos as i64)),
                        None => Ok(LuaValue::Integer(-1)),
                    }
                }
                _ => Ok(LuaValue::Integer(-1)),
            }
        })?;
        globals.set("_indexOf", index_of_fn)?;

        // _concat(arr1, arr2): ECMAScript Array.concat
        let concat_fn = lua.create_function(|lua, (arr1, arr2): (LuaTable, LuaValue)| {
            let result = lua.create_table()?;
            let mut idx = 1;
            // Copy arr1 elements
            for i in 1..=arr1.raw_len() {
                if let Ok(v) = arr1.raw_get::<LuaValue>(i) {
                    result.raw_set(idx, v)?;
                    idx += 1;
                }
            }
            // Append arr2 (table or single value)
            match arr2 {
                LuaValue::Table(t2) => {
                    for i in 1..=t2.raw_len() {
                        if let Ok(v) = t2.raw_get::<LuaValue>(i) {
                            result.raw_set(idx, v)?;
                            idx += 1;
                        }
                    }
                }
                other => {
                    result.raw_set(idx, other)?;
                }
            }
            Ok(result)
        })?;
        globals.set("_concat", concat_fn)?;

        // parseInt(str, radix?): 1:1 with C++ LuaEngine which uses strtoll()
        let parse_int_fn = lua.create_function(|_, args: LuaMultiValue| {
            let values: Vec<LuaValue> = args.into_vec();
            let s = match values.first() {
                Some(LuaValue::String(s)) => s.to_string_lossy().trim().to_string(),
                Some(LuaValue::Integer(n)) => return Ok(LuaValue::Integer(*n)),
                Some(LuaValue::Number(n)) => return Ok(LuaValue::Integer(*n as i64)),
                _ => return Ok(LuaValue::Nil),
            };
            let radix = match values.get(1) {
                Some(LuaValue::Integer(r)) => *r as u32,
                Some(LuaValue::Number(r)) => *r as u32,
                _ => 10,
            };
            // 1:1 with C++ strtoll(str, &endptr, base): parse leading valid digits
            let trimmed = s.trim();
            // Determine effective string and radix (auto-detect hex for radix 16 or 0)
            let (parse_str, effective_radix) = if (radix == 16 || radix == 0)
                && (trimmed.starts_with("0x") || trimmed.starts_with("0X"))
            {
                (&trimmed[2..], 16u32)
            } else if radix == 0 {
                (trimmed, 10u32)
            } else {
                (trimmed, radix)
            };
            // Parse leading digits in the given radix (strtoll semantics)
            let valid_prefix: String = parse_str.chars()
                .take_while(|c| c.is_digit(effective_radix))
                .collect();
            if valid_prefix.is_empty() {
                // Handle leading sign
                if parse_str.starts_with('-') || parse_str.starts_with('+') {
                    let sign = if parse_str.starts_with('-') { "-" } else { "" };
                    let digits: String = parse_str[1..].chars()
                        .take_while(|c| c.is_digit(effective_radix))
                        .collect();
                    if digits.is_empty() {
                        return Ok(LuaValue::Number(f64::NAN));
                    }
                    let full = format!("{}{}", sign, digits);
                    match i64::from_str_radix(&full, effective_radix) {
                        Ok(n) => return Ok(LuaValue::Integer(n)),
                        Err(_) => return Ok(LuaValue::Number(f64::NAN)),
                    }
                }
                Ok(LuaValue::Number(f64::NAN))
            } else {
                match i64::from_str_radix(&valid_prefix, effective_radix) {
                    Ok(n) => Ok(LuaValue::Integer(n)),
                    Err(_) => Ok(LuaValue::Number(f64::NAN)),
                }
            }
        })?;
        globals.set("parseInt", parse_int_fn)?;

        // parseFloat(str): ECMAScript parseFloat (1:1 with C++ strtod pattern)
        let parse_float_fn = lua.create_function(|_, s: LuaValue| {
            let text = match s {
                LuaValue::String(s) => s.to_string_lossy().trim().to_string(),
                LuaValue::Integer(n) => return Ok(LuaValue::Number(n as f64)),
                LuaValue::Number(n) => return Ok(LuaValue::Number(n)),
                _ => return Ok(LuaValue::Number(f64::NAN)),
            };
            match text.parse::<f64>() {
                Ok(f) => Ok(LuaValue::Number(f)),
                Err(_) => Ok(LuaValue::Number(f64::NAN)),
            }
        })?;
        globals.set("parseFloat", parse_float_fn)?;

        // _NULL / _UNDEFINED sentinel values for array literal null/undefined preservation
        globals.set("_NULL", LuaValue::Nil)?;
        globals.set("_UNDEFINED", LuaValue::Nil)?;

        // String metatable: make + work as concatenation via __add
        lua.load(r#"
            local mt = getmetatable("")
            if mt then
                mt.__add = function(a, b)
                    return tostring(a) .. tostring(b)
                end
            end
        "#).exec()?;

        // W3C SCXML B.2: JSON.stringify / JSON.parse (Single Source of Truth)
        // Shared with C++ LuaEngine via sce/include/scripting/json_builtins.lua
        lua.load(include_str!("../../sce/include/scripting/json_builtins.lua"))
            .exec()?;

        // Object.keys(tbl): returns array of string keys
        let object_table = lua.create_table()?;
        let keys_fn = lua.create_function(|lua, tbl: LuaValue| {
            let result = lua.create_table()?;
            if let LuaValue::Table(t) = tbl {
                let mut idx = 1;
                for (k, _) in t.pairs::<LuaValue, LuaValue>().flatten() {
                    let key_str = match k {
                        LuaValue::String(s) => s.to_string_lossy().to_string(),
                        LuaValue::Integer(n) => n.to_string(),
                        _ => continue,
                    };
                    result.raw_set(idx, key_str)?;
                    idx += 1;
                }
            }
            Ok(result)
        })?;
        object_table.set("keys", keys_fn)?;
        globals.set("Object", object_table)?;

        Ok(())
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the process-wide LuaEngine singleton reference.
pub fn lua_engine_singleton() -> &'static LuaEngine {
    static SINGLETON: OnceLock<LuaEngine> = OnceLock::new();
    SINGLETON.get_or_init(LuaEngine::new)
}

/// Register the LuaEngine singleton with the runtime's ScriptEngineProvider.
pub fn register() -> Result<(), ScriptEngineAlreadyRegistered> {
    set_script_engine(lua_engine_singleton())
}

// ═══════════════════════════════════════════════════════════════════════════
// Conversion helpers: ScriptValue <-> LuaValue
// ═══════════════════════════════════════════════════════════════════════════

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
        ScriptValue::Dom(xml) => {
            // Store as userdata-like table with __xml field
            let table = lua.create_table()?;
            table.set("__xml", xml.as_str())?;
            // Set DOM metatable for getElementsByTagName etc.
            setup_dom_metatable_for(lua, &table)?;
            Ok(LuaValue::Table(table))
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
        LuaValue::String(s) => {
            ScriptValue::String(s.to_string_lossy().to_string())
        }
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
                if let Ok(pairs) = t.clone().pairs::<LuaValue, LuaValue>().collect::<Result<Vec<_>, _>>() {
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

fn lua_values_equal(a: &LuaValue, b: &LuaValue) -> bool {
    match (a, b) {
        (LuaValue::Nil, LuaValue::Nil) => true,
        (LuaValue::Boolean(a), LuaValue::Boolean(b)) => a == b,
        (LuaValue::Integer(a), LuaValue::Integer(b)) => a == b,
        (LuaValue::Integer(a), LuaValue::Number(b)) => (*a as f64) == *b,
        (LuaValue::Number(a), LuaValue::Integer(b)) => *a == (*b as f64),
        (LuaValue::Number(a), LuaValue::Number(b)) => a == b,
        (LuaValue::String(a), LuaValue::String(b)) => a.as_bytes() == b.as_bytes(),
        _ => false,
    }
}

/// W3C SCXML B.2 test 578: Convert JSON object/array notation to Lua table syntax.
/// Transforms `"key" : value` → `["key"] = value` and handles nested structures.
fn json_to_lua_table(json: &str) -> String {
    let mut result = String::with_capacity(json.len());
    let bytes = json.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'"' {
            // Capture the full quoted string
            let mut key = String::from('"');
            i += 1;
            while i < len {
                let c = bytes[i] as char;
                key.push(c);
                i += 1;
                if c == '"' { break; }
                if c == '\\' && i < len {
                    key.push(bytes[i] as char);
                    i += 1;
                }
            }
            // Skip whitespace after string
            let mut spaces = String::new();
            while i < len && (bytes[i] as char).is_whitespace() {
                spaces.push(bytes[i] as char);
                i += 1;
            }
            if i < len && bytes[i] == b':' {
                i += 1; // consume ':'
                // JSON key → Lua: ["key"] =
                result.push('[');
                result.push_str(&key);
                result.push(']');
                result.push_str(&spaces);
                result.push('=');
            } else {
                // Just a string value
                result.push_str(&key);
                result.push_str(&spaces);
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }
    result
}

fn setup_dom_metatable_for(lua: &Lua, table: &LuaTable) -> LuaResult<()> {
    let get_elements = lua.create_function(|lua, (tbl, tag): (LuaTable, String)| {
        let xml: String = tbl.get("__xml").unwrap_or_default();
        let result = lua.create_table()?;
        let open_tag = format!("<{}", tag);
        let mut idx = 1;
        let mut pos = 0;
        while let Some(start) = xml[pos..].find(&open_tag) {
            let abs_start = pos + start;
            let after_tag = abs_start + open_tag.len();
            // W3C SCXML B.2: Handle both self-closing (<tag .../>) and paired (<tag ...>...</tag>) elements
            let close_tag = format!("</{}>", tag);
            if let Some(self_close) = xml[after_tag..].find("/>") {
                let self_close_abs = after_tag + self_close;
                // Check if self-closing comes before any closing tag
                let paired_end = xml[abs_start..].find(&close_tag).map(|e| abs_start + e);
                if paired_end.is_none() || self_close_abs < paired_end.unwrap() {
                    // Self-closing tag: <tag .../>
                    let element_xml = &xml[abs_start..self_close_abs + 2];
                    let elem = lua.create_table()?;
                    elem.set("__xml", element_xml)?;
                    setup_dom_metatable_for(lua, &elem)?;
                    result.raw_set(idx, elem)?;
                    idx += 1;
                    pos = self_close_abs + 2;
                    continue;
                }
            }
            if let Some(end) = xml[abs_start..].find(&close_tag) {
                let element_xml = &xml[abs_start..abs_start + end + close_tag.len()];
                let elem = lua.create_table()?;
                elem.set("__xml", element_xml)?;
                setup_dom_metatable_for(lua, &elem)?;
                result.raw_set(idx, elem)?;
                idx += 1;
                pos = abs_start + end + close_tag.len();
            } else {
                break;
            }
        }
        Ok(result)
    })?;

    let get_attribute = lua.create_function(|lua, (tbl, attr_name): (LuaTable, String)| -> LuaResult<LuaValue> {
        let xml: String = tbl.get("__xml").unwrap_or_default();
        // Support both name="value" and name='value' patterns
        for quote in ['"', '\''] {
            let pattern = format!("{}={}", attr_name, quote);
            if let Some(start) = xml.find(&pattern) {
                let val_start = start + pattern.len();
                if let Some(end) = xml[val_start..].find(quote) {
                    return Ok(LuaValue::String(lua.create_string(&xml[val_start..val_start + end])?));
                }
            }
        }
        Ok(LuaValue::Nil)
    })?;

    let get_tag_name = lua.create_function(|lua, tbl: LuaTable| -> LuaResult<LuaValue> {
        let xml: String = tbl.get("__xml").unwrap_or_default();
        // Extract tag name from first < >
        if let Some(start) = xml.find('<') {
            let after = start + 1;
            if let Some(end) = xml[after..].find(|c: char| c.is_whitespace() || c == '>' || c == '/') {
                return Ok(LuaValue::String(lua.create_string(&xml[after..after + end])?));
            }
        }
        Ok(LuaValue::Nil)
    })?;

    table.set("getElementsByTagName", get_elements)?;
    table.set("getAttribute", get_attribute)?;
    table.set("getTagName", get_tag_name)?;

    Ok(())
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
        let session = sessions.get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let result = session.lua.load(script).eval::<LuaValue>().map_err(map_lua_err)?;
        Ok(lua_value_to_script(&result))
    }

    fn evaluate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<ScriptValue> {
        if expression.is_empty() {
            return Ok(ScriptValue::Null);
        }
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;

        // W3C SCXML: Detect undeclared simple variable references (C++ LuaEngine parity)
        // JavaScript throws ReferenceError for undeclared variables; Lua silently returns nil.
        if is_undeclared_simple_variable(expression, &session.declared_vars, &session.lua) {
            return Err(ScriptError::RuntimeError(format!("ReferenceError: {} is not defined", expression)));
        }

        // Wrap as return expression for Lua evaluation
        let lua_expr = format!("return {}", expression);
        let result = session.lua.load(&lua_expr)
            .eval::<LuaValue>()
            .or_else(|first_err| {
                // Fallback: try executing as statement (e.g., assignment)
                log::debug!("evaluate_expression: 'return {}' failed ({}), trying as statement", expression, first_err);
                session.lua.load(expression).eval::<LuaValue>()
            })
            .map_err(map_lua_err)?;
        Ok(lua_value_to_script(&result))
    }

    fn validate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<bool> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(session_id)
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
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let lua_val = script_value_to_lua(&session.lua, &value).map_err(map_lua_err)?;
        session.lua.globals().set(name, lua_val).map_err(map_lua_err)?;
        session.declared_vars.insert(name.to_string());
        Ok(())
    }

    fn get_variable(&self, session_id: &str, name: &str) -> ScriptResult<ScriptValue> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let val: LuaValue = session.lua.globals().get(name).map_err(map_lua_err)?;
        Ok(lua_value_to_script(&val))
    }

    fn set_variable_as_dom(&self, session_id: &str, name: &str, xml_content: &str) -> ScriptResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let table = session.lua.create_table().map_err(map_lua_err)?;
        table.set("__xml", xml_content).map_err(map_lua_err)?;
        setup_dom_metatable_for(&session.lua, &table).map_err(map_lua_err)?;
        session.lua.globals().set(name, table).map_err(map_lua_err)?;
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
        io_processors: &[String],
    ) -> ScriptResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;
        let globals = session.lua.globals();

        // _sessionid
        globals.set("_sessionid", session_id).map_err(map_lua_err)?;
        session.declared_vars.insert("_sessionid".to_string());

        // _name
        globals.set("_name", session_name).map_err(map_lua_err)?;
        session.declared_vars.insert("_name".to_string());

        // _ioprocessors: table of { location = "..." }
        let io_table = session.lua.create_table().map_err(map_lua_err)?;
        for proc_name in io_processors {
            let proc_entry = session.lua.create_table().map_err(map_lua_err)?;
            proc_entry.set("location", format!("#{}", proc_name)).map_err(map_lua_err)?;
            io_table.set(proc_name.as_str(), proc_entry).map_err(map_lua_err)?;
        }
        globals.set("_ioprocessors", io_table).map_err(map_lua_err)?;
        session.declared_vars.insert("_ioprocessors".to_string());

        // _event is NOT initialized here — W3C SCXML B.2: _event is unbound
        // before any event is processed (test 319). It gets set via
        // set_current_event() when an event is actually being processed.

        Ok(())
    }

    fn set_current_event(
        &self,
        session_id: &str,
        event_name: &str,
        event_data: &str,
        event_type: &str,
        send_id: &str,
        origin: &str,
        origin_type: &str,
        invoke_id: &str,
    ) -> ScriptResult<()> {
        let sessions = self.sessions.lock().unwrap();
        let session = sessions.get(session_id)
            .ok_or_else(|| ScriptError::SessionNotFound(session_id.to_string()))?;

        let event_table = session.lua.create_table().map_err(map_lua_err)?;
        event_table.set("name", event_name).map_err(map_lua_err)?;

        // Parse event_data as Lua value if non-empty
        if !event_data.is_empty() {
            // W3C SCXML B.2: XML data → DOM object (test 561)
            if event_data.trim_start().starts_with('<') {
                let dom_table = session.lua.create_table().map_err(map_lua_err)?;
                dom_table.set("__xml", event_data).map_err(map_lua_err)?;
                setup_dom_metatable_for(&session.lua, &dom_table).map_err(map_lua_err)?;
                event_table.set("data", dom_table).map_err(map_lua_err)?;
            } else {
                // Try to evaluate as Lua expression first
                let data_result = session.lua.load(format!("return {}", event_data))
                    .eval::<LuaValue>();
                match data_result {
                    Ok(val) => { event_table.set("data", val).map_err(map_lua_err)?; }
                    Err(_) => {
                        // W3C SCXML B.2 test 578: Try JSON-to-Lua conversion ("key": val → ["key"] = val)
                        let lua_syntax = json_to_lua_table(event_data);
                        let json_result = session.lua.load(format!("return {}", lua_syntax))
                            .eval::<LuaValue>();
                        match json_result {
                            Ok(val) => { event_table.set("data", val).map_err(map_lua_err)?; }
                            Err(_) => {
                                // W3C SCXML B.2 test 562: Fall back to whitespace-normalized string
                                let normalized: String = event_data.split_whitespace().collect::<Vec<&str>>().join(" ");
                                event_table.set("data", normalized).map_err(map_lua_err)?;
                            }
                        }
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
        event_table.set("origintype", origin_type).map_err(map_lua_err)?;
        if !invoke_id.is_empty() {
            event_table.set("invokeid", invoke_id).map_err(map_lua_err)?;
        }

        session.lua.globals().set("_event", event_table).map_err(map_lua_err)?;
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
                let script_args: Vec<ScriptValue> = args.into_vec().iter()
                    .map(lua_value_to_script)
                    .collect();
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
                let script_args: Vec<ScriptValue> = args.into_vec().iter()
                    .map(lua_value_to_script)
                    .collect();
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
                    log::error!("bindNativeObject: failed to create method '{}': {}", name, e);
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
            log::error!("Failed to setup Lua builtins for session {}: {}", session_id, e);
        }

        // Register any global functions added via register_global_function()
        // (1:1 with C++ LuaEngine::createSession which calls registerBuiltins)
        if let Ok(gf) = self.global_functions.lock() {
            for (name, cb) in gf.iter() {
                let cb = cb.clone();
                if let Ok(f) = lua.create_function(move |_, args: LuaMultiValue| {
                    let script_args: Vec<ScriptValue> = args.into_vec().iter()
                        .map(lua_value_to_script)
                        .collect();
                    let result = cb(&script_args);
                    Ok(result.to_bool())
                }) {
                    let _ = lua.globals().set(name.as_str(), f);
                }
            }
        }

        sessions.insert(session_id.to_string(), Session {
            lua,
            declared_vars: HashSet::new(),
            state_query_callback: None,
        });
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
