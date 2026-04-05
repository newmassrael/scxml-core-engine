// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! # SCE Rust Lua — Lua 5.4 script engine for SCXML
//!
//! Implements [`sce_rust_runtime::IScriptEngine`] using `mlua` (Lua 5.4, vendored).
//! Matches the C++ `LuaEngine` at `sce/src/scripting/LuaEngine.cpp` and the Kotlin
//! `LuaScriptEngine` at `sce-kotlin-lua/.../LuaScriptEngine.kt`.
//!
//! ## Phase 1 scope
//!
//! All trait methods are `unimplemented!()` stubs. The crate exists solely so the
//! runtime `ScriptEngineProvider` has a registration target and the workspace
//! compiles as a whole. Phase 3 fills in the real implementation.
//!
//! ## ECMAScript compatibility
//!
//! W3C SCXML mandates the `ecmascript` datamodel. The code generator invokes a
//! Python-hosted ECMAScript→Lua transformer (Phase 3) so that by the time
//! expressions reach this engine they are already valid Lua 5.4 source. This
//! engine therefore just compiles and executes Lua — it does not know about
//! JavaScript syntax.

use std::sync::OnceLock;

use sce_rust_runtime::scripting::{
    set_script_engine, IScriptEngine, NativeMethod, ScriptError, ScriptResult, ScriptValue,
};

/// The Lua 5.4 script engine singleton.
///
/// Phase 1 is a stub. Phase 3 wires up `mlua::Lua` per session with the
/// three-layer expression cache (session → transformer → chunk registry).
pub struct LuaEngine {
    // Phase 3: `sessions: Mutex<HashMap<String, mlua::Lua>>`
}

impl LuaEngine {
    /// Construct a new LuaEngine instance. Typically called once via the singleton.
    pub const fn new() -> Self {
        Self {}
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
///
/// Matches the C++ `ScriptEngineProvider::getScriptEngine()` static initializer
/// pattern. Applications must call this once at startup before constructing any
/// state machine — typically from `main()` or a `ctor` macro.
///
/// Returns `Ok(())` on first registration, `Err(())` if a script engine was
/// already registered (programming error).
pub fn register() -> Result<(), ()> {
    set_script_engine(lua_engine_singleton())
}

// ═══════════════════════════════════════════════════════════════════════════
// IScriptEngine implementation (Phase 1: stubs)
// ═══════════════════════════════════════════════════════════════════════════

impl IScriptEngine for LuaEngine {
    fn execute_script(&self, _session_id: &str, _script: &str) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized) // Phase 1 stub
    }

    fn evaluate_expression(
        &self,
        _session_id: &str,
        _expression: &str,
    ) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized)
    }

    fn validate_expression(&self, _session_id: &str, _expression: &str) -> ScriptResult<bool> {
        Err(ScriptError::NotInitialized)
    }

    fn set_variable(
        &self,
        _session_id: &str,
        _name: &str,
        _value: ScriptValue,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }

    fn get_variable(&self, _session_id: &str, _name: &str) -> ScriptResult<ScriptValue> {
        Err(ScriptError::NotInitialized)
    }

    fn set_variable_as_dom(
        &self,
        _session_id: &str,
        _name: &str,
        _xml_content: &str,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }

    fn has_variable(&self, _session_id: &str, _name: &str) -> bool {
        false
    }

    fn is_variable_pre_initialized(&self, _session_id: &str, _name: &str) -> bool {
        false
    }

    fn setup_system_variables(
        &self,
        _session_id: &str,
        _session_name: &str,
        _io_processors: &[String],
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }

    fn set_current_event(
        &self,
        _session_id: &str,
        _event_name: &str,
        _event_data: &str,
        _event_type: &str,
        _send_id: &str,
        _origin: &str,
        _origin_type: &str,
        _invoke_id: &str,
    ) -> ScriptResult<()> {
        Err(ScriptError::NotInitialized)
    }

    fn register_global_function(&self, _function_name: &str, _callback: NativeMethod) -> bool {
        false
    }

    fn bind_native_object(
        &self,
        _session_id: &str,
        _object_name: &str,
        _methods: Vec<(String, NativeMethod)>,
    ) -> bool {
        false
    }

    fn get_engine_info(&self) -> String {
        "Lua 5.4 (Phase 1 stub)".to_string()
    }

    fn get_memory_usage(&self) -> usize {
        0
    }

    fn collect_garbage(&self) {}

    fn set_state_query_callback(
        &self,
        _session_id: &str,
        _callback: Option<Box<dyn Fn(&str) -> bool + Send + Sync>>,
    ) {
    }

    fn initialize(&self) -> bool {
        false
    }

    fn shutdown(&self) {}

    fn is_initialized(&self) -> bool {
        false
    }

    fn reset(&self) {}

    fn create_session(&self, _session_id: &str) {}

    fn destroy_session(&self, _session_id: &str) {}

    fn has_session(&self, _session_id: &str) -> bool {
        false
    }
}
