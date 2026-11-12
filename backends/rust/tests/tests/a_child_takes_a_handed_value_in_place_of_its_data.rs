// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 (docs/adr/0098) — Rust AOT path: a value an `<invoke>` hands its
// child is the initial value of the child's top-level `<data>` of that name, in
// place of its own, which is then not evaluated.
//
// The child reports what it read to its parent, which copies it into its own
// `<data>`; the cases read those. A child that evaluated each `<data>` and then
// wrote the handed value over it raises the error.execution of `failing`'s own
// expr and leaves the entry of `effectful`'s own `trail.push('own')`; one that
// seeded every variable its session holds overwrites the state's `local_only`;
// one that added every name handed declares `stray`; one that took the values
// after its `<script>` or its first `<onentry>` reads its own values there.
//
// The script engine of the last case is a double over the Lua engine that
// refuses to hold one value, so a handed value the child's session cannot
// hold is reached through the engine the machine runs in, as a host's would be.
//
// Fixture: tests/integration/a_child_takes_a_handed_value_in_place_of_its_data.scxml.
// It is outside `integration_resources/` for the reason
// `tools/codegen/trees/rust-integration.toml` states.
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh rust-integration

use std::sync::Arc;
use std::time::Duration;

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::scripting::{
    IScriptEngine, NativeMethod, PayloadReading, ScriptError, ScriptResult, ScriptValue,
    SetCurrentEventArgs, StateQueryCallback,
};
use sce_rust_runtime::Engine;
use sce_rust_tests::integration::a_child_takes_a_handed_value_in_place_of_its_data::{
    AChildTakesAHandedValueInPlaceOfItsDataPolicy as Policy,
    AChildTakesAHandedValueInPlaceOfItsDataState as State,
};

/// The fixture's machine over `script_engine`, started and run until the
/// child's report arrived or `timeout` passed; whether it arrived.
fn run(script_engine: Arc<dyn IScriptEngine>, timeout: Duration) -> (Engine<Policy>, bool) {
    let mut engine = Engine::new(Policy::new(script_engine));
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(timeout, Duration::from_millis(10));
    (engine, completed)
}

/// The fixture over the Lua engine, which the child's report reached.
fn reported() -> Engine<Policy> {
    let (engine, completed) = run(Arc::new(LuaEngine::new()), Duration::from_secs(3));
    assert!(
        completed,
        "the parent never received the child's report: it stands in {:?}",
        engine.get_active_states()
    );
    assert_eq!(engine.terminal_state(), Some(State::Received));
    engine
}

#[test]
fn a_seeded_data_whose_own_expr_would_fail_takes_the_handed_value_and_raises_nothing() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_failing().as_deref(),
        Some("handed-failing"),
        "`failing` holds the value its invoke handed it"
    );
    assert_eq!(
        engine.policy().saw_errors(),
        Some(0),
        "`failing`'s own expr `holder.missing.deep` raises error.execution when it \
         is evaluated, and a seeded <data>'s own value is never evaluated"
    );
}

#[test]
fn a_seeded_data_whose_own_expr_acts_takes_the_handed_value_without_the_act() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_effectful().as_deref(),
        Some("handed-effectful")
    );
    assert_eq!(
        engine.policy().saw_trail(),
        Some(0),
        "`effectful`'s own expr `trail.push('own')` leaves an entry in `trail` when \
         it is evaluated, and a seeded <data>'s own value is never evaluated"
    );
}

#[test]
fn a_state_data_of_a_handed_name_keeps_its_own_value() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_local().as_deref(),
        Some("own"),
        "only a top-level <data> takes a handed value; `local_only` is declared by \
         the child's state `report`"
    );
}

#[test]
fn a_handed_name_no_top_level_data_declares_is_not_added() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_stray().as_deref(),
        Some("undefined"),
        "the child declares no `stray`, so the value handed under it is not added \
         to its datamodel"
    );
}

#[test]
fn an_object_value_arrives_whole() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_label().as_deref(),
        Some("whole"),
        "the handed object's own field"
    );
    assert_eq!(
        engine.policy().saw_depth(),
        Some(2),
        "the field of the object inside the handed object"
    );
}

#[test]
fn the_childs_script_and_first_onentry_read_the_handed_values() {
    let engine = reported();
    assert_eq!(
        engine.policy().saw_script().as_deref(),
        Some("handed-failing"),
        "the top-level <script> runs after the top-level <data> are declared, \
         seeded, and reads `failing`"
    );
    assert_eq!(
        engine.policy().saw_entry().as_deref(),
        Some("handed-effectful"),
        "the initial state's <onentry> reads `effectful` as seeded"
    );
}

/// The id of the one top-level `<data>` of the child the double refuses to set.
const REFUSED: &str = "failing";

/// The Lua engine, refusing to hold any value set under [`REFUSED`].
struct RefusingEngine {
    inner: LuaEngine,
}

impl IScriptEngine for RefusingEngine {
    fn execute_script(&self, session_id: &str, script: &str) -> ScriptResult<ScriptValue> {
        self.inner.execute_script(session_id, script)
    }

    fn evaluate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<ScriptValue> {
        self.inner.evaluate_expression(session_id, expression)
    }

    fn validate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<bool> {
        self.inner.validate_expression(session_id, expression)
    }

    fn set_variable(&self, session_id: &str, name: &str, value: ScriptValue) -> ScriptResult<()> {
        if name == REFUSED {
            return Err(ScriptError::RuntimeError(format!(
                "the double holds no value under '{name}'"
            )));
        }
        self.inner.set_variable(session_id, name, value)
    }

    fn get_variable(&self, session_id: &str, name: &str) -> ScriptResult<ScriptValue> {
        self.inner.get_variable(session_id, name)
    }

    fn set_variable_as_dom(
        &self,
        session_id: &str,
        name: &str,
        xml_content: &str,
    ) -> ScriptResult<()> {
        self.inner
            .set_variable_as_dom(session_id, name, xml_content)
    }

    fn has_variable(&self, session_id: &str, name: &str) -> bool {
        self.inner.has_variable(session_id, name)
    }

    fn is_variable_pre_initialized(&self, session_id: &str, name: &str) -> bool {
        self.inner.is_variable_pre_initialized(session_id, name)
    }

    fn setup_system_variables(
        &self,
        session_id: &str,
        session_name: &str,
        io_processors: &[IoProcessorDescriptor],
    ) -> ScriptResult<()> {
        self.inner
            .setup_system_variables(session_id, session_name, io_processors)
    }

    fn set_current_event(
        &self,
        session_id: &str,
        args: SetCurrentEventArgs<'_>,
    ) -> ScriptResult<PayloadReading> {
        self.inner.set_current_event(session_id, args)
    }

    fn register_global_function(&self, function_name: &str, callback: NativeMethod) -> bool {
        self.inner.register_global_function(function_name, callback)
    }

    fn bind_native_object(
        &self,
        session_id: &str,
        object_name: &str,
        methods: Vec<(String, NativeMethod)>,
    ) -> bool {
        self.inner
            .bind_native_object(session_id, object_name, methods)
    }

    fn get_engine_info(&self) -> String {
        self.inner.get_engine_info()
    }

    fn get_memory_usage(&self) -> usize {
        self.inner.get_memory_usage()
    }

    fn collect_garbage(&self) {
        self.inner.collect_garbage()
    }

    fn set_state_query_callback(&self, session_id: &str, callback: Option<StateQueryCallback>) {
        self.inner.set_state_query_callback(session_id, callback)
    }

    fn initialize(&self) -> bool {
        self.inner.initialize()
    }

    fn shutdown(&self) {
        self.inner.shutdown()
    }

    fn is_initialized(&self) -> bool {
        self.inner.is_initialized()
    }

    fn reset(&self) {
        self.inner.reset()
    }

    fn create_session(&self, session_id: &str) -> ScriptResult<()> {
        self.inner.create_session(session_id)
    }

    fn destroy_session(&self, session_id: &str) {
        self.inner.destroy_session(session_id)
    }

    fn has_session(&self, session_id: &str) -> bool {
        self.inner.has_session(session_id)
    }
}

#[test]
fn a_handed_value_the_session_cannot_hold_is_its_datas_initialization_error() {
    // Whether the rest of the report arrives over a `failing` that holds
    // nothing is not this case's question, so its arrival is not asserted:
    // `counted` is sent first, and is all this reads.
    let (engine, _) = run(
        Arc::new(RefusingEngine {
            inner: LuaEngine::new(),
        }),
        Duration::from_secs(1),
    );
    assert_eq!(
        engine.policy().saw_errors(),
        Some(1),
        "the session refused the value handed for `failing`: that is the one \
         error.execution of `failing`'s initialization (§scxml-5.3), and its own \
         expr, which would raise a second, is not evaluated in its place"
    );
    assert_eq!(
        engine.policy().saw_trail(),
        Some(0),
        "`effectful` was seeded all the same"
    );
}
