// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096 — Rust AOT path: a generated machine whose script session
// cannot be established does not start, and its host is told why.
//
// The script engine is a double over the real Lua engine. It fails, on
// request, one of the three steps that establish a session — creating it,
// loading the shared library into it, binding the system variables — and
// counts every call that runs a document's expressions, so "no executable
// content ran" is read from the engine the machine runs its content in.
//
// Fixture: integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an
// inline child, whose `<onentry>` sends `tick` to the parent 110 times. The
// child declares `<data>` of its own, so it has a script session of its own to
// fail (a child without one cannot be refused that way: its failure type is
// `Infallible`). A start that ran would initialize `ticks`, enter `s0` and
// start the child; a child that ran would send `tick`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use sce_rust_lua::LuaEngine;
use sce_rust_runtime::helpers::io_processors::IoProcessorDescriptor;
use sce_rust_runtime::scripting::{
    IScriptEngine, NativeMethod, PayloadReading, ScriptError, ScriptResult, ScriptValue,
    SetCurrentEventArgs, StateQueryCallback,
};
use sce_rust_runtime::{
    ConfigurationRejection, Engine, RootStartRefusal, ScriptSessionStage, StatePolicy,
};
use sce_rust_tests::integration::a_child_may_send_many_events_in_one_tick::{
    AChildMaySendManyEventsInOneTickEvent as Event,
    AChildMaySendManyEventsInOneTickPolicy as Policy,
    AChildMaySendManyEventsInOneTickState as State,
};

/// Which step of establishing a session the double fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fail {
    /// None: the double is the Lua engine, counted.
    Never,
    /// Every session's creation fails as the engine's own failure.
    Create,
    /// Every session's creation fails as a shared library that did not load.
    Library,
    /// Every bind of the system variables fails.
    SystemVariables,
    /// The first session asked for, the parent's, is created; every later
    /// one, a child's, fails to be.
    CreateAfterFirst,
}

/// The Lua engine, with one step of establishing a session failing on
/// request and every call that runs the document's expressions counted.
struct FailingEngine {
    inner: LuaEngine,
    fail: Fail,
    /// Every session id `create_session` was asked for, in order.
    asked: Mutex<Vec<String>>,
    /// Every session id `destroy_session` was asked for, in order.
    destroyed: Mutex<Vec<String>>,
    /// Calls that run a document's expressions or write its datamodel.
    ran: AtomicUsize,
}

impl FailingEngine {
    fn new(fail: Fail) -> Self {
        Self {
            inner: LuaEngine::new(),
            fail,
            asked: Mutex::new(Vec::new()),
            destroyed: Mutex::new(Vec::new()),
            ran: AtomicUsize::new(0),
        }
    }

    fn count(&self) {
        self.ran.fetch_add(1, Ordering::SeqCst);
    }

    fn ran(&self) -> usize {
        self.ran.load(Ordering::SeqCst)
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    fn destroyed(&self) -> Vec<String> {
        self.destroyed.lock().unwrap().clone()
    }
}

impl IScriptEngine for FailingEngine {
    fn execute_script(&self, session_id: &str, script: &str) -> ScriptResult<ScriptValue> {
        self.count();
        self.inner.execute_script(session_id, script)
    }

    fn evaluate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<ScriptValue> {
        self.count();
        self.inner.evaluate_expression(session_id, expression)
    }

    fn validate_expression(&self, session_id: &str, expression: &str) -> ScriptResult<bool> {
        self.inner.validate_expression(session_id, expression)
    }

    fn set_variable(&self, session_id: &str, name: &str, value: ScriptValue) -> ScriptResult<()> {
        self.count();
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
        self.count();
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
        if self.fail == Fail::SystemVariables {
            return Err(ScriptError::RuntimeError(
                "the double binds no system variable".to_string(),
            ));
        }
        self.inner
            .setup_system_variables(session_id, session_name, io_processors)
    }

    fn set_current_event(
        &self,
        session_id: &str,
        args: SetCurrentEventArgs<'_>,
    ) -> ScriptResult<PayloadReading> {
        self.count();
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
        let first = {
            let mut asked = self.asked.lock().unwrap();
            asked.push(session_id.to_string());
            asked.len() == 1
        };
        match self.fail {
            Fail::Create => Err(ScriptError::EngineError(
                "the double creates no session".to_string(),
            )),
            Fail::Library => Err(ScriptError::LibraryNotLoaded(
                "the double loads no library".to_string(),
            )),
            Fail::CreateAfterFirst if !first => Err(ScriptError::EngineError(
                "the double creates no second session".to_string(),
            )),
            _ => self.inner.create_session(session_id),
        }
    }

    fn destroy_session(&self, session_id: &str) {
        self.destroyed.lock().unwrap().push(session_id.to_string());
        self.inner.destroy_session(session_id)
    }

    fn has_session(&self, session_id: &str) -> bool {
        self.inner.has_session(session_id)
    }
}

/// The fixture's machine over a double that fails as `fail` says, and the
/// double, to read afterwards.
fn machine(fail: Fail) -> (Engine<Policy>, Arc<FailingEngine>) {
    let double = Arc::new(FailingEngine::new(fail));
    let script_engine: Arc<dyn IScriptEngine> = double.clone();
    (Engine::new(Policy::new(script_engine)), double)
}

/// The machine's own session id, which its constructor bound.
fn session_of(engine: &Engine<Policy>) -> String {
    engine
        .policy()
        .session_id
        .clone()
        .expect("a session id is bound when the policy is built")
}

/// A refused start left nothing behind: no state, no content run, no
/// session kept, not running.
fn assert_nothing_started(engine: &Engine<Policy>, double: &FailingEngine, case: Fail) {
    assert!(
        !engine.is_running(),
        "{case:?}: a refused start leaves the machine not running"
    );
    assert!(
        engine.get_active_states().is_empty(),
        "{case:?}: a refused start enters no state, and {:?} is active",
        engine.get_active_states()
    );
    assert_eq!(
        engine.terminal_state(),
        None,
        "{case:?}: a refused start reaches no final state"
    );
    assert_eq!(
        double.ran(),
        0,
        "{case:?}: a refused start evaluates nothing: no <data>, no <onentry>, no cond"
    );
    assert!(
        !double.has_session(&session_of(engine)),
        "{case:?}: a refused start keeps no session"
    );
}

#[test]
fn a_start_over_the_double_that_fails_nothing_runs_the_fixture() {
    // The control: the double fails nothing, so the machine is the fixture's,
    // and its child's reply arrives. Without it the cases below could pass
    // over a double that breaks every machine.
    let (mut engine, double) = machine(Fail::Never);
    engine
        .initialize()
        .expect("a session that is established starts the machine");
    engine.raise_external(Event::Finish, "");
    engine.step();
    assert_eq!(engine.terminal_state(), Some(State::Done));
    assert_eq!(engine.policy().ticks(), Some(110));
    assert!(double.ran() > 0);
}

#[test]
fn a_start_whose_session_cannot_be_established_is_refused_with_the_step() {
    for (fail, stage) in [
        (Fail::Create, ScriptSessionStage::Create),
        (Fail::Library, ScriptSessionStage::Library),
        (Fail::SystemVariables, ScriptSessionStage::SystemVariables),
    ] {
        let (mut engine, double) = machine(fail);
        let refused = engine
            .initialize()
            .expect_err("a session that cannot be established refuses the start");
        assert_eq!(refused.stage, stage, "{fail:?}: the refusal names the step");
        assert_nothing_started(&engine, &double, fail);
    }
}

#[test]
fn a_refused_start_is_refused_again_rather_than_run_without_a_session() {
    // A session that failed is not marked established: the next start asks
    // for it again and is refused again, rather than finding it "done" and
    // running the machine over no session.
    for fail in [Fail::Create, Fail::Library, Fail::SystemVariables] {
        let (mut engine, double) = machine(fail);
        assert!(engine.initialize().is_err());
        assert!(
            engine.initialize().is_err(),
            "{fail:?}: a second start over the same failing engine is refused too"
        );
        assert_eq!(
            double.asked().len(),
            2,
            "{fail:?}: each start asks for the session once"
        );
        assert_nothing_started(&engine, &double, fail);
    }
}

#[test]
fn a_session_whose_system_variables_cannot_be_bound_is_released() {
    let (mut engine, double) = machine(Fail::SystemVariables);
    assert!(engine.initialize().is_err());
    let session = session_of(&engine);
    assert_eq!(double.asked(), std::slice::from_ref(&session));
    assert_eq!(
        double.destroyed(),
        [session],
        "the session the failed bind was made in is destroyed"
    );
}

#[test]
fn a_refused_machine_takes_no_event() {
    let (mut engine, double) = machine(Fail::Create);
    assert!(engine.initialize().is_err());
    engine.process_event(Event::Finish);
    assert_eq!(
        engine.unseen_external_events(),
        1,
        "the event is refused as one posted to a machine that is not running"
    );
    assert_nothing_started(&engine, &double, Fail::Create);
}

#[test]
fn the_checked_root_start_is_refused_for_the_session() {
    let (mut engine, double) = machine(Fail::Library);
    match engine.initialize_as_root() {
        Err(RootStartRefusal::ScriptSession(failure)) => {
            assert_eq!(failure.stage, ScriptSessionStage::Library);
        }
        other => panic!("the root start answered {other:?}, not a refusal for its session"),
    }
    assert_nothing_started(&engine, &double, Fail::Library);
}

#[test]
fn a_resumed_start_whose_session_cannot_be_established_is_refused() {
    let (mut engine, double) = machine(Fail::Create);
    match engine.enter_at(&[State::S0], State::S0) {
        Err(ConfigurationRejection::ScriptSessionUnavailable(failure)) => {
            assert_eq!(failure.stage, ScriptSessionStage::Create);
        }
        other => panic!("enter_at answered {other:?}, not a refusal for its session"),
    }
    assert_nothing_started(&engine, &double, Fail::Create);
}

#[test]
fn a_child_whose_session_fails_leaves_its_parent_with_error_execution_and_no_child() {
    let (mut engine, double) = machine(Fail::CreateAfterFirst);
    engine
        .initialize()
        .expect("the parent's own session is established");
    engine.step();

    let asked = double.asked();
    assert_eq!(
        asked.len(),
        2,
        "the parent asked for its own session and the child's: {asked:?}"
    );
    assert!(
        !double.has_session(&asked[1]),
        "the child whose session failed keeps none"
    );
    assert!(
        !engine.policy().is_child_session_running(&asked[1]),
        "the child whose start was refused is not kept as a session of the parent"
    );
    // §scxml-6.4: the invoke ends without a child, with error.execution, which
    // no transition of `s0` takes.
    assert_eq!(engine.last_unhandled_error(), Some(Event::ErrorExecution));
    assert_eq!(engine.unhandled_error_events(), 1);
    // No child ran, so no `tick` and no `done.invoke` ever arrived.
    assert_eq!(engine.policy().ticks(), Some(0));
    assert_eq!(engine.discarded_external_events(), 0);

    // The parent itself runs on.
    assert!(engine.is_running());
    assert_eq!(engine.get_current_state(), State::S0);
    engine.raise_external(Event::Finish, "");
    engine.step();
    assert_eq!(engine.terminal_state(), Some(State::Done));
}
