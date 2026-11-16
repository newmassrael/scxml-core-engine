// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4 (docs/adr/0098) — Rust AOT path: a value an `<invoke>` hands its
// child is the initial value of the child's top-level `<data>` of that name, in
// place of its own, which is then not evaluated.
//
// Each child reports what it read to its parent, which keeps it in its own
// `<data>` and judges it, landing a failure in a `<final>` named for the
// sentence that broke. A child that evaluated each `<data>` and then wrote the
// handed value over it pushes onto `effects` (`FailOwnValueRan`) and raises the
// error.execution of `raising`'s own expr (`FailChildRaised`); one that seeded
// every variable its session holds overwrites the state's `kept`; one that
// added every name handed declares `stranger`; one that took the values after
// its `<script>` or its first `<onentry>` reads its own values there.
//
// The script engine of the last case is a double over the Lua engine that
// refuses to hold one value, so a handed value the child's session cannot
// hold is reached through the engine the machine runs in, as a host's would be.
//
// Fixture: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
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

/// The fixture's machine over `script_engine`, started and run until it
/// reached a final state; panics, naming where it stood, when it did not.
fn run(script_engine: Arc<dyn IScriptEngine>) -> Engine<Policy> {
    let mut engine = Engine::new(Policy::new(script_engine));
    engine
        .initialize()
        .expect("the machine's script session is established");
    let completed = engine.run_until_completion(Duration::from_secs(3), Duration::from_millis(10));
    assert!(
        completed,
        "the parent reached no final state: it stands in {:?}. ByParam / ByNamelist / \
         LateBound: that phase's child never reported. FirstMacrostep: the child's \
         first macrostep read its own value of `handed` and stands in `missed`",
        engine.get_active_states()
    );
    engine
}

#[test]
fn a_child_takes_each_handed_value_in_place_of_its_top_level_data() {
    let engine = run(Arc::new(LuaEngine::new()));
    let reached = engine.terminal_state();
    assert_eq!(
        reached,
        Some(State::Pass),
        "reached {reached:?}. \
         FailOwnValueRan: `effectful`'s own expr `effects.push(..)` was evaluated \
         although a value was handed in its place. \
         FailChildRaised: the child saw error.execution, so `raising`'s own expr \
         `undefined.invalidProperty` was evaluated (errors_seen={:?}). \
         FailHandedValueLost: a top-level <data> kept its own value. \
         FailStateDataTookTheValue / FailLateStateDataTookTheValue: a state's \
         <data>, early- or late-bound, took a handed value. \
         FailUnmatchedNameAdded / FailNamelistNameAdded: a handed name no \
         top-level <data> declares became a variable. \
         FailObjectNotWhole: an object lost its structure. \
         FailChildSharedTheParentsObject: the child was handed the parent's \
         object rather than a copy of it. \
         FailScriptReadItsOwnValue / FailEntryReadItsOwnValue: the top-level \
         <script> or the initial <onentry> ran before the values were declared. \
         FailNamelistOwnValueRaised / FailNamelistValueLost: the same, for a \
         namelist value. FailParentRaised: the parent could not evaluate what \
         it hands.",
        engine.policy().errors_seen()
    );
}

/// The id of the one top-level `<data>` of the child the double refuses to set.
const REFUSED: &str = "refusable";

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
    let engine = run(Arc::new(RefusingEngine {
        inner: LuaEngine::new(),
    }));
    // The parent judges every other value of the child before its error
    // count, so reaching `FailChildRaised` says they were all in place.
    assert_eq!(
        engine.terminal_state(),
        Some(State::FailChildRaised),
        "the session refused the value handed for `refusable`, which is that \
         <data>'s initialization error (§scxml-5.3): the child sees it, and every \
         other handed value is still declared in its place"
    );
    assert_eq!(
        engine.policy().errors_seen(),
        Some(1),
        "the refusal is the one error.execution of `refusable`'s initialization, \
         and no other <data> raised"
    );
    assert_eq!(
        engine.policy().refusable_seen().as_deref(),
        Some("unbound"),
        "a <data> whose handed value was refused is left as one whose \
         initialization failed, and does not take its own value instead"
    );
}
