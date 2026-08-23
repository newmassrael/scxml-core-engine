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

use crate::helpers::io_processors::IoProcessorDescriptor;
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
    /// Opaque DOM reference (§scxml-B-2 — XML datamodel support).
    /// Stored as the original XML string; the script engine parses on demand.
    Dom(String),
}

impl ScriptValue {
    /// Attempt to coerce to `bool` per SCXML truthiness rules
    /// (§scxml-B-2-3: ECMAScript truthy/falsy semantics).
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
}

/// Script engine error. 1:1 port of C++ `ScriptResult::error` field.
#[derive(Debug, Clone, Error)]
pub enum ScriptError {
    /// Syntax error during compilation (§scxml-5.9: raises `error.execution`).
    #[error("script syntax error: {0}")]
    SyntaxError(String),

    /// Runtime error during evaluation (§scxml-5.9: raises `error.execution`).
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

/// Callback for resolving the SCXML `In(stateId)` predicate (§scxml-5.9.2).
///
/// Receives a state ID string and returns `true` if that state is in the engine's
/// current active configuration. Matches the C++ `std::function<bool(const std::string &)>`
/// signature passed to `setStateQueryCallback`.
pub type StateQueryCallback = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// Which reading §scxml-B-2-8-1 gave a delivered payload.
///
/// Defined in [`crate::payload_reading`] and re-exported here, where it is
/// PRODUCED. It cannot be defined here: this module is gated out of `no_std`
/// builds, and [`Engine`](crate::Engine) — which counts these readings and is
/// the surface an MCU consumer builds — would then name a type that does not
/// exist. See that module for the measurement that moved it.
pub use crate::payload_reading::PayloadReading;

/// Parameter object for the §scxml-5.10 `set_current_event` boundary.
///
/// Bundles the seven `_event.*` metadata fields (name + 6 metadata) that every
/// script engine impl must surface before guard evaluation / action execution.
/// Cross-language sibling: `SCE::SetCurrentEventArgs` in
/// `sce/include/scripting/IScriptEngine.h`. Fields borrow from the caller for
/// the call duration to avoid per-event `String` allocations.
#[derive(Debug, Clone, Copy)]
pub struct SetCurrentEventArgs<'a> {
    /// `_event.name` — fully-qualified event name (§scxml-5.10).
    pub event_name: &'a str,
    /// `_event.data` — event payload (JSON string or platform-specific serialization).
    pub event_data: &'a str,
    /// `_event.type` — classification ("internal" / "external" / "platform").
    pub event_type: &'a str,
    /// `_event.sendid` — send ID from originating `<send>` (W3C 5.10.1).
    pub send_id: &'a str,
    /// `_event.origin` — origin URI (W3C 5.10.1).
    pub origin: &'a str,
    /// `_event.origintype` — type of origin (W3C 5.10.1).
    pub origin_type: &'a str,
    /// `_event.invokeid` — invoke ID when event came from a child invoke (W3C 6.4.1).
    pub invoke_id: &'a str,
}

/// The script engine trait — 1:1 port of C++ `SCE::IScriptEngine`.
///
/// Implementations (`sce-rust-lua`, future `sce-rust-quickjs`) provide ECMAScript
/// evaluation for §scxml-B-1 datamodel support. Engine DI Parity RFC
/// (Path B+): consumers pass an `Arc<dyn IScriptEngine>` to the generated
/// `Policy::new(engine)` constructor; there is no process-global singleton.
pub trait IScriptEngine: Send + Sync {
    // ════════════════════════════════════════
    // Core Script Execution
    // ════════════════════════════════════════

    /// Execute a script block in the specified session (§scxml-5.8).
    ///
    /// Matches C++ `executeScript(const string&, const string&) -> future<ScriptResult>`.
    /// The future is resolved synchronously — returns the final expression value.
    fn execute_script(&self, session_id: &str, script: &str) -> ScriptResult<ScriptValue>;

    /// Evaluate an expression and return its value (§scxml-5.3).
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

    /// Set a variable in the specified session (§scxml-5.3).
    fn set_variable(&self, session_id: &str, name: &str, value: ScriptValue) -> ScriptResult<()>;

    /// Get a variable from the specified session (§scxml-5.3).
    fn get_variable(&self, session_id: &str, name: &str) -> ScriptResult<ScriptValue>;

    /// Set a variable to an XML DOM object parsed from the given XML content (§scxml-B-2).
    ///
    /// Matches C++ `setVariableAsDOM`. Used for `<data src="...xml">` loading and
    /// inline `<content>` with XML payloads.
    fn set_variable_as_dom(
        &self,
        session_id: &str,
        name: &str,
        xml_content: &str,
    ) -> ScriptResult<()>;

    /// Check if a variable is declared in the session scope (§scxml-4.6 / 6.4).
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

    /// Set up SCXML system variables: `_sessionid`, `_name`, `_ioprocessors` (§scxml-5.10).
    ///
    /// The descriptors arrive fully resolved from
    /// [`crate::helpers::io_processors::build`]. An implementation files each
    /// one under its name with its location and invents neither, so
    /// `_ioprocessors` reads identically whichever engine backs the session.
    fn setup_system_variables(
        &self,
        session_id: &str,
        session_name: &str,
        io_processors: &[IoProcessorDescriptor],
    ) -> ScriptResult<()>;

    /// Set the `_event` system variable for the currently-processing event (§scxml-5.10).
    ///
    /// Called before guard evaluation and action execution for each event. Mirrors
    /// the C++ `IScriptEngine::setCurrentEvent(sessionId, const SetCurrentEventArgs&)`
    /// overload (the seven W3C 5.10 metadata fields bundled into one struct).
    ///
    /// Returns which rung of §scxml-B-2-8-1 the payload got. The implementation
    /// is walking that ladder either way, and the rung is the one fact about a
    /// delivered event that nothing else can recover afterwards — see
    /// [`PayloadReading`]. An implementation that binds no payload returns
    /// [`PayloadReading::Absent`]; one that cannot tell the rungs apart must
    /// not guess, because a wrong `Undecodable` is a host chasing a payload
    /// that arrived intact.
    fn set_current_event(
        &self,
        session_id: &str,
        args: SetCurrentEventArgs<'_>,
    ) -> ScriptResult<PayloadReading>;

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
    // State Query Callback (§scxml-5.9.2 In() predicate)
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
