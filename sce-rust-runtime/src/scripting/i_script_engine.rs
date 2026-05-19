// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! [`IScriptEngine`] — the script engine trait, 1:1 port of C++ `IScriptEngine.h`.
//!
//! Port rules:
//! - C++ `std::future<ScriptResult>` return types become synchronous `Result<ScriptValue, ScriptError>`.
//!   C++ resolves futures via `.get()` in practice (see callers in
//!   `sce/common/AssignmentExecutionHelper.cpp` etc.), so the sync shape preserves semantics.
//! - C++ `const std::string&` parameters become `&str`.
//! - C++ `const std::vector<...>&` parameters become `&[...]`.
//! - C++ inheritance `ISessionLifecycle` is flattened into this trait.
//! - C++ `std::function` callbacks become `Box<dyn Fn(...) + Send + Sync>`.

use std::collections::HashMap;

use thiserror::Error;

/// A value that can cross the script engine boundary.
///
/// Matches C++ `SCE::ScriptValue` (a `std::variant<...>`-like discriminated union).
/// Generated code converts datamodel variable values to/from `ScriptValue` at assignment
/// and guard evaluation sites.
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptValue {
    /// JavaScript `null` / Lua `nil`.
    Null,
    /// `undefined` (JavaScript only; Lua treats as `nil`).
    Undefined,
    /// Boolean.
    Bool(bool),
    /// 64-bit signed integer.
    Int(i64),
    /// Double-precision float.
    Double(f64),
    /// UTF-8 string.
    String(String),
    /// Ordered array of values.
    Array(Vec<ScriptValue>),
    /// Key-value object (iteration order is implementation-defined).
    Object(HashMap<String, ScriptValue>),
    /// Opaque DOM reference (W3C SCXML B.2 — XML datamodel support).
    /// Stored as the original XML string; the script engine parses on demand.
    Dom(String),
}

impl ScriptValue {
    /// Attempt to coerce to `bool` per SCXML truthiness rules
    /// (W3C SCXML B.2.3: ECMAScript truthy/falsy semantics).
    pub fn to_bool(&self) -> bool {
        match self {
            ScriptValue::Null | ScriptValue::Undefined => false,
            ScriptValue::Bool(b) => *b,
            ScriptValue::Int(i) => *i != 0,
            ScriptValue::Double(f) => *f != 0.0 && !f.is_nan(),
            ScriptValue::String(s) => !s.is_empty(),
            ScriptValue::Array(_) | ScriptValue::Object(_) | ScriptValue::Dom(_) => true,
        }
    }

    /// W3C SCXML 5.5: Convert to a Lua literal string representation.
    ///
    /// Used by donedata evaluation to produce event data strings that the Lua
    /// script engine can parse back via `return <literal>`. Ports the C++
    /// `DoneDataHelper::convertScriptValueToJson` but emits Lua syntax instead
    /// of JSON so the Lua engine's `load("return " .. data)` path works.
    pub fn to_lua_literal(&self) -> String {
        match self {
            ScriptValue::Null | ScriptValue::Undefined => "nil".to_string(),
            ScriptValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            ScriptValue::Int(i) => i.to_string(),
            ScriptValue::Double(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{:.1}", f)
                } else {
                    format!("{}", f)
                }
            }
            ScriptValue::String(s) => {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                format!("\"{}\"", escaped)
            }
            ScriptValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_lua_literal()).collect();
                format!("{{{}}}", items.join(", "))
            }
            ScriptValue::Object(map) => {
                let items: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("[\"{}\"] = {}", k, v.to_lua_literal()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            ScriptValue::Dom(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{}\"", escaped)
            }
        }
    }
}

/// Script engine error. 1:1 port of C++ `ScriptResult::error` field.
#[derive(Debug, Clone, Error)]
pub enum ScriptError {
    /// Syntax error during compilation (W3C SCXML 5.9: raises `error.execution`).
    #[error("script syntax error: {0}")]
    SyntaxError(String),

    /// Runtime error during evaluation (W3C SCXML 5.9: raises `error.execution`).
    #[error("script runtime error: {0}")]
    RuntimeError(String),

    /// Attempted to access a session that does not exist.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Variable not declared in the session's datamodel.
    #[error("variable not declared: {0}")]
    VariableNotDeclared(String),

    /// Attempted to assign to a read-only system variable (_event, _sessionid, etc.).
    #[error("cannot assign to system variable: {0}")]
    ReadOnlySystemVariable(String),

    /// Engine is not initialized; call [`IScriptEngine::initialize`] first.
    #[error("script engine not initialized")]
    NotInitialized,

    /// Engine-specific error (e.g., mlua error wrapper).
    #[error("engine error: {0}")]
    EngineError(String),
}

/// Convenience type alias.
pub type ScriptResult<T> = Result<T, ScriptError>;

/// A native Rust function callable from script code.
///
/// Matches C++ `NativeMethod = std::function<ScriptValue(const std::vector<ScriptValue>&)>`.
pub type NativeMethod = Box<dyn Fn(&[ScriptValue]) -> ScriptValue + Send + Sync>;

/// Callback for resolving the SCXML `In(stateId)` predicate (W3C SCXML 5.9.2).
///
/// Receives a state ID string and returns `true` if that state is in the engine's
/// current active configuration. Matches the C++ `std::function<bool(const std::string &)>`
/// signature passed to `setStateQueryCallback`.
pub type StateQueryCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// The script engine trait — 1:1 port of C++ `SCE::IScriptEngine`.
///
/// Implementations (`sce-rust-lua`, future `sce-rust-quickjs`) provide ECMAScript
/// evaluation for W3C SCXML B.1 datamodel support. Engine DI Parity RFC
/// (Path B+): consumers pass an `Arc<dyn IScriptEngine>` to the generated
/// `Policy::new(engine)` constructor; there is no process-global singleton.
pub trait IScriptEngine: Send + Sync {
    // ════════════════════════════════════════
    // Core Script Execution
    // ════════════════════════════════════════

    /// Execute a script block in the specified session (W3C SCXML 3.8.6).
    ///
    /// Matches C++ `executeScript(const string&, const string&) -> future<ScriptResult>`.
    /// The future is resolved synchronously — returns the final expression value.
    fn execute_script(&self, session_id: &str, script: &str) -> ScriptResult<ScriptValue>;

    /// Evaluate an expression and return its value (W3C SCXML 5.3).
    ///
    /// Matches C++ `evaluateExpression(...) -> future<ScriptResult>`. Used for
    /// variable init, `<param expr="...">`, guard conditions, etc.
    fn evaluate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<ScriptValue>;

    /// Validate expression syntax without executing.
    ///
    /// Matches C++ `validateExpression(...) -> future<ScriptResult>`. Returns `true`
    /// if the syntax is valid. Used by the datamodel parser for early error detection.
    fn validate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<bool>;

    // ════════════════════════════════════════
    // Variable Management
    // ════════════════════════════════════════

    /// Set a variable in the specified session (W3C SCXML 5.3).
    fn set_variable(&self, session_id: &str, name: &str, value: ScriptValue) -> ScriptResult<()>;

    /// Get a variable from the specified session (W3C SCXML 5.3).
    fn get_variable(&self, session_id: &str, name: &str) -> ScriptResult<ScriptValue>;

    /// Set a variable to an XML DOM object parsed from the given XML content (W3C SCXML B.2).
    ///
    /// Matches C++ `setVariableAsDOM`. Used for `<data src="...xml">` loading and
    /// inline `<content>` with XML payloads.
    fn set_variable_as_dom(
        &self,
        session_id: &str,
        name: &str,
        xml_content: &str,
    ) -> ScriptResult<()>;

    /// Check if a variable is declared in the session scope (W3C SCXML 4.6 / 6.4).
    ///
    /// Returns `true` if declared even if the value is `null`/`undefined`. Used by
    /// foreach to distinguish declared-but-empty from undeclared variables.
    fn has_variable(&self, session_id: &str, name: &str) -> bool;

    /// Check if a variable was pre-initialized (e.g., by invoke `<param>`).
    ///
    /// Matches C++ `isVariablePreInitialized`. Used by datamodel init to skip
    /// re-initializing variables already set by the parent SM.
    fn is_variable_pre_initialized(&self, session_id: &str, name: &str) -> bool;

    // ════════════════════════════════════════
    // SCXML-specific Features
    // ════════════════════════════════════════

    /// Set up SCXML system variables: `_sessionid`, `_name`, `_ioprocessors` (W3C SCXML 5.10).
    fn setup_system_variables(
        &self,
        session_id: &str,
        session_name: &str,
        io_processors: &[String],
    ) -> ScriptResult<()>;

    /// Set the `_event` system variable for the currently-processing event (W3C SCXML 5.10).
    ///
    /// Called before guard evaluation and action execution for each event. The
    /// individual-fields overload matches C++ `setCurrentEvent(sessionId, eventName, ...)`.
    // Justification (clippy::too_many_arguments): 8 fields mirror the C++
    // `IScriptEngine::setCurrentEvent` signature; bundling into a struct would
    // break the documented 1:1 port and require every script engine
    // implementation to redo the field plumbing.
    #[allow(clippy::too_many_arguments)]
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
    ) -> ScriptResult<()>;

    // ════════════════════════════════════════
    // Global Function Management
    // ════════════════════════════════════════

    /// Register a native function accessible from script code.
    ///
    /// Matches C++ `registerGlobalFunction`. Returns `true` on success. Used by
    /// runtime integrations (e.g., logging, state query callbacks) to expose Rust
    /// code to generated Lua/JS expressions.
    fn register_global_function(&self, function_name: &str, callback: NativeMethod) -> bool;

    // ════════════════════════════════════════
    // Native Object Binding
    // ════════════════════════════════════════

    /// Bind a native object with method table as a script-accessible object.
    ///
    /// Matches C++ `bindNativeObject`. Creates `objectName.methodName()` accessor
    /// in the session's datamodel scope. Used for Dependency Injection of native
    /// hardware/service shims into SCXML datamodels.
    fn bind_native_object(
        &self,
        session_id: &str,
        object_name: &str,
        methods: Vec<(String, NativeMethod)>,
    ) -> bool;

    // ════════════════════════════════════════
    // Engine Information
    // ════════════════════════════════════════

    /// Human-readable engine info string (name + version).
    fn get_engine_info(&self) -> String;

    /// Current memory usage in bytes (heap allocated by the script engine).
    fn get_memory_usage(&self) -> usize;

    /// Force garbage collection.
    fn collect_garbage(&self);

    // ════════════════════════════════════════
    // State Query Callback (W3C SCXML 5.9.2 In() predicate)
    // ════════════════════════════════════════

    /// Register a callback that resolves the SCXML `In()` predicate.
    ///
    /// See [`StateQueryCallback`] for the callback signature.
    ///
    /// Matches C++ `setStateQueryCallback`. The callback takes a state ID string
    /// and returns `true` if the state is in the current active configuration.
    /// Passing `None` unregisters the callback.
    fn set_state_query_callback(&self, session_id: &str, callback: Option<StateQueryCallback>);

    // ════════════════════════════════════════
    // Engine Lifecycle
    // ════════════════════════════════════════

    /// Initialize the engine. Called once per process. Returns `true` on success.
    fn initialize(&self) -> bool;

    /// Shutdown and release all sessions. Called during program exit.
    fn shutdown(&self);

    /// Whether the engine has been successfully initialized.
    fn is_initialized(&self) -> bool;

    /// Reset engine state for test isolation.
    ///
    /// Destroys all sessions, clears registered functions/callbacks, then
    /// re-initializes. Matches C++ `reset()`.
    fn reset(&self);

    // ════════════════════════════════════════
    // Session Lifecycle (C++: inherited from ISessionLifecycle)
    // ════════════════════════════════════════

    /// Create a new evaluation session.
    ///
    /// Each state machine instance has its own session with an isolated variable scope.
    fn create_session(&self, session_id: &str);

    /// Destroy a session and release its resources.
    fn destroy_session(&self, session_id: &str);

    /// Check whether a session exists.
    fn has_session(&self, session_id: &str) -> bool;
}
