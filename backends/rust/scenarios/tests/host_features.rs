// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The standard host features (docs/SCE_HOST_FEATURES.md) as a generated Rust
// machine sees them through the reference host: every scenario of
// `tests/host_features/scenarios` replayed against the machine it names, with
// `sce_rust_host_features::ReferenceHost` registered as the invoker of every
// standard type.
//
// A scenario's `host` directives set what the host is like before a step; its
// `expect.host_invokes` and `expect.host_state` are what the host was asked and
// what it holds afterwards. The expected values are derived from the
// specification, not observed from the host.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use sce_rust_host_features::{
    serve, AnswerSchema, ClockSource, ExhaustiveProvider, HttpRequest, HttpResponse, OutcomeClass,
    Reading, ReferenceHost, ScriptedAnswer, ScriptedClock, ScriptedProvider, Shared, Transport,
    TransportFailure, Zone,
};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock, StatePolicy, TransitionTaken};
use sce_rust_scenarios::integration::static_datamodel::{
    static_host_clock_sm::{StaticHostClockPersist, StaticHostClockPolicy},
    static_host_http_sm::{StaticHostHttpPersist, StaticHostHttpPolicy},
    static_host_kv_sm::{StaticHostKvPersist, StaticHostKvPolicy},
    static_host_model_sm::{StaticHostModelPersist, StaticHostModelPolicy, ANSWER_SCHEMAS},
    static_host_notify_sm::{StaticHostNotifyPersist, StaticHostNotifyPolicy},
    static_model_harness_sm::{
        StaticModelHarnessPersist, StaticModelHarnessPolicy, StaticModelHarnessState,
        StaticModelHarnessTransition, ANSWER_SCHEMAS as HARNESS_ANSWER_SCHEMAS,
    },
};
use serde_json::{json, Map, Value};

fn scenario(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/scenarios")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).expect("the scenario file is readable");
    serde_json::from_str(&text).expect("a scenario is JSON")
}

/// What a scenario says the transport answers, and what it was given.
#[derive(Default)]
struct Script {
    answer: Option<Result<Option<HttpResponse>, TransportFailure>>,
    log: Vec<HttpRequest>,
}

struct Scripted(Arc<Mutex<Script>>);

impl Transport for Scripted {
    fn exchange(
        &mut self,
        request: &HttpRequest,
    ) -> Result<Option<HttpResponse>, TransportFailure> {
        let mut script = self.0.lock().expect("the script is not left mid-update");
        script.log.push(request.clone());
        script.answer.clone().unwrap_or(Err(TransportFailure))
    }
}

fn pairs(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .expect("headers are a list of pairs")
        .iter()
        .map(|pair| {
            (
                pair[0].as_str().expect("a name").to_string(),
                pair[1].as_str().expect("a value").to_string(),
            )
        })
        .collect()
}

/// The answer an `http` directive describes.
fn answer(directive: &Value) -> Result<Option<HttpResponse>, TransportFailure> {
    if directive.get("hold").and_then(Value::as_bool) == Some(true) {
        return Ok(None);
    }
    if directive.get("fail").is_some() {
        return Err(TransportFailure);
    }
    let body = if let Some(text) = directive.get("body").and_then(Value::as_str) {
        text.as_bytes().to_vec()
    } else if let Some(hex) = directive.get("body_hex").and_then(Value::as_str) {
        (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("hex"))
            .collect()
    } else if let Some(repeat) = directive.get("body_repeat") {
        repeat[0]
            .as_str()
            .expect("a text")
            .repeat(repeat[1].as_u64().expect("a count") as usize)
            .into_bytes()
    } else {
        Vec::new()
    };
    Ok(Some(HttpResponse {
        status: directive["status"].as_u64().expect("a status") as u16,
        headers: pairs(&directive["headers"]),
        body,
    }))
}

type Clock = Arc<Mutex<ScriptedClock>>;

/// The text a `model` answer gives: `text`, or `text_repeat` as a text and a
/// count.
fn answer_text(answer: &Value) -> String {
    if let Some(text) = answer.get("text").and_then(Value::as_str) {
        return text.to_string();
    }
    let repeat = &answer["text_repeat"];
    repeat[0]
        .as_str()
        .expect("a text")
        .repeat(repeat[1].as_u64().expect("a count") as usize)
}

/// Apply a `model` directive to the scripted provider: each member it names.
fn script_model(provider: &mut ScriptedProvider, directive: &Value) {
    if let Some(answers) = directive.get("answers").and_then(Value::as_array) {
        provider.answers = answers
            .iter()
            .map(|answer| ScriptedAnswer {
                text: answer_text(answer),
                finish: answer
                    .get("finish")
                    .and_then(Value::as_str)
                    .unwrap_or("stop")
                    .to_string(),
            })
            .collect();
    }
    if let Some(constrained) = directive.get("constrained").and_then(Value::as_bool) {
        provider.constrained = constrained;
    }
    if let Some(name) = directive.get("name").and_then(Value::as_str) {
        provider.name = name.to_string();
    }
    if let Some(context) = directive.get("context").and_then(Value::as_u64) {
        provider.context = context;
    }
    if let Some(hold) = directive.get("hold").and_then(Value::as_bool) {
        provider.hold = hold;
    }
}

/// The request fields a scenario writes as numbers; every other is text.
const INTEGER_FIELDS: [&str; 3] = ["at_ms", "max_tokens", "temperature_milli"];

struct Rig {
    host: Shared<Scripted>,
    clock: Clock,
    script: Arc<Mutex<Script>>,
    model: Arc<Mutex<ScriptedProvider>>,
}

impl Rig {
    fn directives(&self, directives: &Value) {
        let mut host = self.host.lock().expect("the host is not left mid-update");
        for (name, value) in directives
            .as_object()
            .expect("host directives are an object")
        {
            match name.as_str() {
                "clock" => {
                    let mut clock = self.clock.lock().expect("the clock");
                    let mut reading = clock.reading.unwrap_or(Reading {
                        utc_ms: 0,
                        monotonic_ms: 0,
                    });
                    let mut zone = clock.zone.clone().unwrap_or(Zone {
                        id: String::new(),
                        utc_offset_s: 0,
                    });
                    if let Some(n) = value.get("utc_ms").and_then(Value::as_i64) {
                        reading.utc_ms = n;
                    }
                    if let Some(n) = value.get("monotonic_ms").and_then(Value::as_i64) {
                        reading.monotonic_ms = n;
                    }
                    if let Some(id) = value.get("zone_id").and_then(Value::as_str) {
                        zone.id = id.to_string();
                    }
                    if let Some(n) = value.get("utc_offset_s").and_then(Value::as_i64) {
                        zone.utc_offset_s = n as i32;
                    }
                    clock.reading = Some(reading);
                    clock.zone = Some(zone);
                }
                "permission" => host.set_permission(value.as_str().expect("a state")),
                "user_decision" => host.set_user_decision(value.as_str().expect("a decision")),
                "quota" => host.set_quota(value.as_u64().expect("a count") as usize),
                "fail" => host.set_faults(
                    value
                        .as_object()
                        .expect("failures are an object")
                        .iter()
                        .map(|(operation, code)| {
                            (
                                operation.clone(),
                                code.as_str().expect("a code").to_string(),
                            )
                        })
                        .collect(),
                ),
                "http" => self.script.lock().expect("the script").answer = Some(answer(value)),
                "model" => script_model(&mut self.model.lock().expect("the provider"), value),
                other => panic!("`{other}` is not a host directive"),
            }
        }
    }

    /// What the host was started with since the last step, as a scenario writes
    /// it: the request as its record, the integer fields as numbers.
    fn starts(&self) -> Value {
        let starts = self.host.lock().expect("the host").take_starts();
        Value::Array(
            starts
                .into_iter()
                .map(|start| {
                    let mut request = Map::new();
                    for (name, text) in start.params {
                        let value = if INTEGER_FIELDS.contains(&name.as_str()) {
                            json!(text.parse::<i64>().expect("digits"))
                        } else {
                            Value::String(text)
                        };
                        request.insert(name, value);
                    }
                    json!({ "type": start.operation, "id": start.id, "request": request })
                })
                .collect(),
        )
    }

    fn host_state(&self, wanted: &Value) -> Value {
        let host = self.host.lock().expect("the host");
        let mut state = Map::new();
        for name in wanted.as_object().expect("host_state is an object").keys() {
            let value = match name.as_str() {
                "store" => Value::Object(
                    host.store_entries()
                        .into_iter()
                        .map(|(key, value)| (key, Value::String(value)))
                        .collect(),
                ),
                "waiting" => json!(host.waiting()),
                "shown" => json!(host.shown()),
                "cancelled" => json!(host.cancelled()),
                "http" => Value::Array(
                    self.script
                        .lock()
                        .expect("the script")
                        .log
                        .iter()
                        .map(|r| {
                            json!({
                                "method": r.method,
                                "url": r.url,
                                "headers": r.headers.iter()
                                    .map(|(n, v)| json!({ "name": n, "value": v }))
                                    .collect::<Vec<_>>(),
                                "body": r.body,
                            })
                        })
                        .collect(),
                ),
                other => panic!("`{other}` is not host state"),
            };
            state.insert(name.clone(), value);
        }
        Value::Object(state)
    }
}

const SETTLE_TICKS: usize = 5;

fn settle<P: StatePolicy>(engine: &mut Engine<P>) {
    if P::NEEDS_EVENT_SCHEDULER {
        for _ in 0..SETTLE_TICKS {
            engine.tick();
        }
    } else {
        engine.step();
    }
}

fn holds(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::String(text), Value::Number(n)) => {
            (n.is_i64() || n.is_u64()) && *text == n.to_string()
        }
        _ => got == want,
    }
}

/// Replay the scenario of `machine`'s own name against it.
fn replay<P: StatePolicy>(
    engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    machine: &str,
    answer_schemas: &[AnswerSchema],
) {
    replay_scenario(engine, save, machine, machine, answer_schemas);
}

/// Replay the scenario `stem` against `machine`, the machine the scenario names: a
/// machine whose run ends has one scenario a run (the reference harness's cases).
fn replay_scenario<P: StatePolicy>(
    mut engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    stem: &str,
    machine: &str,
    answer_schemas: &[AnswerSchema],
) {
    let scenario = scenario(stem);
    assert_eq!(
        scenario["machine"], machine,
        "the scenario {stem} names its machine"
    );
    let clock: Clock = Arc::new(Mutex::new(ScriptedClock::new()));
    let script = Arc::new(Mutex::new(Script::default()));
    let model = Arc::new(Mutex::new(ScriptedProvider::default()));
    let mut reference = ReferenceHost::new(
        Box::new(Arc::clone(&clock)) as Box<dyn ClockSource + Send>,
        Scripted(Arc::clone(&script)),
    );
    reference.set_answer_schemas(answer_schemas);
    reference.set_model_provider(Box::new(Arc::clone(&model)));
    let host: Shared<Scripted> = Arc::new(Mutex::new(reference));
    serve(&mut engine, &host);
    let rig = Rig {
        host,
        clock,
        script,
        model,
    };
    engine.set_clock(SceClock::Manual(0));
    engine.initialize().expect("the machine starts");
    settle(&mut engine);
    let steps = scenario["steps"].as_array().expect("steps");
    assert!(!steps.is_empty(), "a scenario with no steps judges nothing");
    for (n, step) in steps.iter().enumerate() {
        let note = step.get("note").and_then(Value::as_str).unwrap_or("");
        if let Some(directives) = step.get("host") {
            rig.directives(directives);
        }
        if let Some(ms) = step.get("advance_ms").and_then(Value::as_u64) {
            engine.advance_time_ms(ms);
            settle(&mut engine);
        }
        if let Some(event) = step.get("event").and_then(Value::as_str) {
            assert!(
                P::resolve_event_by_name(event).is_some(),
                "{stem} step {n}: the machine's events do not match `{event}`"
            );
            let data = step.get("data").map(Value::to_string).unwrap_or_default();
            engine.raise_external_by_name(event, &data);
            settle(&mut engine);
        }
        let expect = &step["expect"];
        let started = rig.starts();
        if let Some(want) = expect.get("host_invokes") {
            assert_eq!(
                &started, want,
                "{stem} step {n} ({note}): what the host was asked"
            );
        }
        if let Some(want) = expect.get("host_state") {
            assert_eq!(
                &rig.host_state(want),
                want,
                "{stem} step {n} ({note}): what the host holds"
            );
        }
        let saved: Value =
            serde_json::from_str(&save(&engine).to_json()).expect("a saved state is JSON");
        if let Some(state) = expect.get("state") {
            assert_eq!(
                &saved["current"], state,
                "{stem} step {n} ({note}): the current state"
            );
        }
        if let Some(variables) = expect.get("variables").and_then(Value::as_object) {
            for (name, want) in variables {
                let got = &saved["variables"][name];
                assert!(
                    holds(got, want),
                    "{stem} step {n} ({note}): variable `{name}` is {got}, not {want}"
                );
            }
        }
    }
}

#[test]
fn a_machine_asks_the_key_value_store_of_its_host() {
    replay(
        Engine::new(StaticHostKvPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_kv",
        &[],
    );
}

#[test]
fn a_machine_reads_the_clock_of_its_host() {
    replay(
        Engine::new(StaticHostClockPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_clock",
        &[],
    );
}

#[test]
fn a_machine_posts_notifications_through_its_host() {
    replay(
        Engine::new(StaticHostNotifyPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_notify",
        &[],
    );
}

#[test]
fn a_machine_makes_an_http_exchange_through_its_host() {
    replay(
        Engine::new(StaticHostHttpPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_http",
        &[],
    );
}

#[test]
fn a_machine_asks_its_host_for_a_text_under_an_answer_schema() {
    replay(
        Engine::new(StaticHostModelPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_model",
        ANSWER_SCHEMAS,
    );
}

/// The cases of the reference harness's scenario (docs/adr/0087, section 9.1 item
/// 8), one scenario each: a run of the harness ends at one of its finals, so each
/// case is a run of its own.
const HARNESS_SCENARIOS: [&str; 5] = [
    "static_model_harness_accepted",
    "static_model_harness_exhausted",
    "static_model_harness_not_offered",
    "static_model_harness_repaired",
    "static_model_harness_timeout",
];

#[test]
fn the_reference_harness_runs_each_case_of_its_scenario() {
    for stem in HARNESS_SCENARIOS {
        replay_scenario(
            Engine::new(StaticModelHarnessPolicy::new()),
            |engine| engine.save_at(0).expect("saves"),
            stem,
            "static_model_harness",
            HARNESS_ANSWER_SCHEMAS,
        );
    }
}

/// The table of the outcome classes of one generation every lane reads
/// (docs/adr/0087, section 9.4).
fn outcome_classes() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/host_features/model_outcome_classes.json");
    let text = std::fs::read_to_string(&path).expect("the class table is readable");
    serde_json::from_str(&text).expect("the class table is JSON")
}

/// What the exhaustive provider answers for `class`.
fn outcome_class(class: &Value) -> OutcomeClass {
    match class["end"].as_str().expect("an end") {
        "answer" => OutcomeClass::Answer {
            text: class["text"].as_str().expect("a text").to_string(),
            finish: class["finish"].as_str().expect("a finish").to_string(),
        },
        "failure" => OutcomeClass::Failure(class["error"].as_str().expect("a code").to_string()),
        "refusal" => OutcomeClass::Refusal,
        "hold" => OutcomeClass::Hold,
        other => panic!("`{other}` is not an end"),
    }
}

/// Every sequence of `count` indices below `classes`, in lexicographic order.
fn sequences(classes: usize, count: usize) -> Vec<Vec<usize>> {
    let mut all: Vec<Vec<usize>> = vec![Vec::new()];
    for _ in 0..count {
        all = all
            .into_iter()
            .flat_map(|prefix| {
                (0..classes).map(move |next| {
                    let mut longer = prefix.clone();
                    longer.push(next);
                    longer
                })
            })
            .collect();
    }
    all
}

/// Run `static_host_model` against the exhaustive provider answering the classes
/// `sequence` names, one call a class, and hold what the machine publishes after
/// each call to what its class says.
fn run_sequence(table: &Value, sequence: &[usize]) {
    let classes = table["classes"].as_array().expect("classes");
    let named: Vec<&str> = sequence
        .iter()
        .map(|&i| classes[i]["class"].as_str().expect("a class name"))
        .collect();
    let mut engine = Engine::new(StaticHostModelPolicy::new());
    let clock: Clock = Arc::new(Mutex::new(ScriptedClock::new()));
    let mut reference = ReferenceHost::new(
        Box::new(Arc::clone(&clock)) as Box<dyn ClockSource + Send>,
        Scripted(Arc::new(Mutex::new(Script::default()))),
    );
    reference.set_answer_schemas(ANSWER_SCHEMAS);
    reference.set_model_provider(Box::new(ExhaustiveProvider::new(
        sequence
            .iter()
            .map(|&i| outcome_class(&classes[i]))
            .collect(),
    )));
    let host: Shared<Scripted> = Arc::new(Mutex::new(reference));
    serve(&mut engine, &host);
    engine.set_clock(SceClock::Manual(0));
    let Ok(()) = engine.initialize();
    settle(&mut engine);
    let ask = &table["ask"];
    let event = ask["event"].as_str().expect("the ask event");
    let deadline_ms = table["deadline_ms"].as_u64().expect("a deadline");
    let leave = table["leave"].as_str().expect("the leave event");
    let mut answer = Value::Null;
    for (n, &i) in sequence.iter().enumerate() {
        let class = &classes[i];
        engine.raise_external_by_name(event, &ask["data"].to_string());
        settle(&mut engine);
        match class.get("then").and_then(Value::as_str) {
            Some("advance") => {
                engine.advance_time_ms(deadline_ms);
                settle(&mut engine);
            }
            Some("leave") => {
                engine.raise_external_by_name(leave, "");
                settle(&mut engine);
            }
            Some(other) => panic!("`{other}` is not how a held call ends"),
            None => {}
        }
        let expect = &class["expect"];
        if let Some(read) = expect.get("answer") {
            answer = read.clone();
        }
        let saved: Value = serde_json::from_str(&engine.save_at(0).expect("saves").to_json())
            .expect("a saved state is JSON");
        let at = format!("{named:?}, call {}", n + 1);
        assert_eq!(saved["current"], "idle", "{at}: the machine is idle");
        let variables = &saved["variables"];
        for name in ["outcome", "why", "finish"] {
            assert!(
                holds(&variables[name], &expect[name]),
                "{at}: `{name}` is {}, not {}",
                variables[name],
                expect[name]
            );
        }
        assert!(
            holds(&variables["answer"], &answer),
            "{at}: `answer` is {}, not {answer}",
            variables["answer"]
        );
        assert!(
            holds(&variables["calls"], &json!(n + 1)),
            "{at}: `calls` is {}",
            variables["calls"]
        );
    }
}

/// The machine around a model is tested exhaustively (docs/adr/0087, section
/// 9.4): every sequence of outcome classes up to the table's bound, each call
/// read as its class says, and every refusal code of the library either reached
/// by a class or named as deferred with its reason.
#[test]
fn every_sequence_of_outcome_classes_is_read_as_its_classes_say() {
    let table = outcome_classes();
    assert_eq!(table["machine"], "static_host_model");
    assert_eq!(
        table["answer_schema"].as_str(),
        ANSWER_SCHEMAS.first().map(|schema| schema.id),
        "the table's answer schema is the machine's"
    );
    let classes = table["classes"].as_array().expect("classes").len();
    let bound = table["bound"].as_u64().expect("a bound") as usize;
    assert!(
        classes > 0 && bound > 0,
        "a table that names nothing judges nothing"
    );
    let mut runs = 0usize;
    for count in 1..=bound {
        for sequence in sequences(classes, count) {
            run_sequence(&table, &sequence);
            runs += 1;
        }
    }
    let expected: usize = (1..=bound).map(|k| classes.pow(k as u32)).sum();
    assert_eq!(runs, expected, "every sequence up to the bound was run");
    let reached: std::collections::BTreeSet<&str> = table["classes"]
        .as_array()
        .expect("classes")
        .iter()
        .filter_map(|class| class["expect"]["why"]["code"].as_str())
        .chain(
            table["deferred"]
                .as_array()
                .expect("deferred")
                .iter()
                .filter_map(|deferred| deferred["code"].as_str()),
        )
        .collect();
    // The fourteen codes, `tooFew` the last (docs/adr/0086, section 15.11).
    assert_eq!(
        reached.len(),
        14,
        "every code of sce:std/lift/refusal_code.scxml is reached or deferred: {reached:?}"
    );
}

/// A row of the reference harness's transition table, as the table and the
/// transition observer name it: the state it is written in and its index there.
type HarnessRow = (StaticModelHarnessState, u32);

/// The rows a transition observer is told, in the order it is told them.
type HarnessSeen = Arc<Mutex<Vec<HarnessRow>>>;

/// The codes of the application's own checks, after the fourteen of the lift
/// that its enum repeats (enum_harness_code.scxml), each named for the class that
/// reaches it.
const APPLICATION_CODES: [&str; 1] = ["notOffered"];

/// The ticks a harness run is given after each event: a run may make its calls
/// and take each completion within one event, a call a round.
const HARNESS_SETTLE_TICKS: usize = 8;

fn settle_harness(engine: &mut Engine<StaticModelHarnessPolicy>) {
    for _ in 0..HARNESS_SETTLE_TICKS {
        engine.tick();
    }
}

/// The end of the host operation a class gives a call (docs/adr/0087, section
/// 9.4): an answer by its `finish`, a failure by its code, the refusal, and a held
/// call, whichever way it is left.
fn host_outcome(class: &Value) -> String {
    match class["end"].as_str().expect("an end") {
        "answer" => format!("answer:{}", class["finish"].as_str().expect("a finish")),
        "failure" => class["error"].as_str().expect("a code").to_string(),
        "refusal" => "invalid_argument".to_string(),
        "hold" => "hold".to_string(),
        other => panic!("`{other}` is not an end"),
    }
}

/// What one run of the harness against the exhaustive provider showed.
struct HarnessRun {
    /// Every row the transition observer was told, in order.
    rows: Vec<HarnessRow>,
    /// How many calls the host was asked to start.
    calls: usize,
    /// The state the run ended in.
    ended: &'static str,
    /// The code of each refusal the harness kept, in order.
    codes: Vec<String>,
}

/// Run the reference harness against the exhaustive provider answering the
/// classes `sequence` names, one call a class: offer it the table's ids, start
/// it, and leave each call it holds as the table's harness section says, until
/// it ends.
fn run_harness(table: &Value, sequence: &[usize]) -> HarnessRun {
    let classes = table["classes"].as_array().expect("classes");
    let harness = &table["harness"];
    let bound = harness["bound"].as_u64().expect("a bound") as usize;
    let deadline_ms = harness["deadline_ms"].as_u64().expect("a deadline");
    assert_eq!(
        harness["then"], "advance",
        "a held call of the harness is left by its own deadline"
    );
    let seen = HarnessSeen::default();
    let mut engine = Engine::new(StaticModelHarnessPolicy::new());
    let sink = Arc::clone(&seen);
    engine.set_transition_observer(move |taken: TransitionTaken<StaticModelHarnessState>| {
        sink.lock()
            .expect("the observer's list is not poisoned")
            .push((taken.source, taken.index));
    });
    let clock: Clock = Arc::new(Mutex::new(ScriptedClock::new()));
    let mut reference = ReferenceHost::new(
        Box::new(Arc::clone(&clock)) as Box<dyn ClockSource + Send>,
        Scripted(Arc::new(Mutex::new(Script::default()))),
    );
    reference.set_answer_schemas(HARNESS_ANSWER_SCHEMAS);
    reference.set_model_provider(Box::new(ExhaustiveProvider::new(
        sequence
            .iter()
            .map(|&i| outcome_class(&classes[i]))
            .collect(),
    )));
    let host: Shared<Scripted> = Arc::new(Mutex::new(reference));
    serve(&mut engine, &host);
    engine.set_clock(SceClock::Manual(0));
    let Ok(()) = engine.initialize();
    settle_harness(&mut engine);
    let offer = harness["offer"].as_str().expect("the offer event");
    for id in harness["offered"].as_array().expect("the offered ids") {
        engine.raise_external_by_name(offer, &json!({ "id": id }).to_string());
        settle_harness(&mut engine);
    }
    let start = harness["start"].as_str().expect("the start event");
    engine.raise_external_by_name(start, &harness["data"].to_string());
    settle_harness(&mut engine);
    let ended = |engine: &Engine<StaticModelHarnessPolicy>| {
        matches!(
            StaticModelHarnessPolicy::get_state_name(engine.get_current_state()),
            "accepted" | "gave_up"
        )
    };
    let mut waits = 0usize;
    while !ended(&engine) {
        assert!(
            waits <= bound,
            "{sequence:?}: the run neither ended nor held a call it could be moved past"
        );
        engine.advance_time_ms(deadline_ms);
        settle_harness(&mut engine);
        waits += 1;
    }
    let calls = host.lock().expect("the host").take_starts().len();
    let saved: Value = serde_json::from_str(&engine.save_at(0).expect("saves").to_json())
        .expect("a saved state is JSON");
    let codes = saved["variables"]["refusals"]
        .as_array()
        .expect("the refusals are a list")
        .iter()
        .map(|refusal| refusal["code"].as_str().expect("a code").to_string())
        .collect();
    let rows = seen.lock().expect("not poisoned").clone();
    HarnessRun {
        rows,
        calls,
        ended: StaticModelHarnessPolicy::get_state_name(engine.get_current_state()),
        codes,
    }
}

/// `row` as a failure names it: its state's id and its index.
fn harness_row_name(row: &HarnessRow) -> String {
    format!(
        "{}[{}]",
        StaticModelHarnessPolicy::get_state_name(row.0),
        row.1
    )
}

/// The reference harness is tested exhaustively, and its coverage is measured
/// (docs/adr/0087, section 9.4 item 1, Landing order item 7): every sequence of
/// `bound` outcome classes is a run, the transition observer of docs/adr/0089
/// tells each row of the emitted transition table the run takes, and the runs
/// together take every row of the table, reach every refusal code, and reach every
/// host outcome at every call site (a row that enters the state whose invoke is the
/// call).
///
/// The harness makes at most `bound` calls and the provider answers a call by its
/// position alone, so every sequence of fewer classes the harness can consume is a
/// prefix of a sequence run here, and none is asked past.
#[test]
fn every_sequence_of_outcome_classes_takes_every_row_of_the_reference_harness() {
    let table = outcome_classes();
    let harness = &table["harness"];
    assert_eq!(harness["machine"], "static_model_harness");
    assert_eq!(
        table["answer_schema"].as_str(),
        HARNESS_ANSWER_SCHEMAS.first().map(|schema| schema.id),
        "the table's answer schema is the harness's"
    );
    let classes = table["classes"].as_array().expect("classes");
    let bound = harness["bound"].as_u64().expect("a bound") as usize;
    let call = harness["call"].as_str().expect("the call's state");
    assert!(
        !classes.is_empty() && bound > 0,
        "a table that names nothing judges nothing"
    );
    let rows: Vec<HarnessRow> = StaticModelHarnessTransition::ALL
        .iter()
        .map(|row| (row.from, row.index))
        .collect();
    let sites: Vec<HarnessRow> = StaticModelHarnessTransition::ALL
        .iter()
        .filter(|row| row.targets == call)
        .map(|row| (row.from, row.index))
        .collect();
    assert!(!sites.is_empty(), "no row of the table enters `{call}`");
    let outcomes: BTreeSet<String> = classes.iter().map(host_outcome).collect();

    let mut taken: HashSet<HarnessRow> = HashSet::new();
    let mut codes: BTreeSet<String> = BTreeSet::new();
    let mut reached: HashSet<(HarnessRow, String)> = HashSet::new();
    let mut runs = 0usize;
    for sequence in sequences(classes.len(), bound) {
        let named: Vec<&str> = sequence
            .iter()
            .map(|&i| classes[i]["class"].as_str().expect("a class name"))
            .collect();
        let run = run_harness(&table, &sequence);
        assert!(
            run.ended == "accepted" || run.ended == "gave_up",
            "{named:?}: the run ended in `{}`, not a final",
            run.ended
        );
        let called: Vec<HarnessRow> = run
            .rows
            .iter()
            .copied()
            .filter(|row| sites.contains(row))
            .collect();
        assert_eq!(
            called.len(),
            run.calls,
            "{named:?}: each call is made by one row that enters `{call}`"
        );
        assert!(
            run.calls <= bound,
            "{named:?}: {} calls, past the bound",
            run.calls
        );
        for (site, &class) in called.iter().zip(&sequence) {
            reached.insert((*site, host_outcome(&classes[class])));
        }
        taken.extend(run.rows);
        codes.extend(run.codes);
        runs += 1;
    }
    assert_eq!(
        runs,
        classes.len().pow(bound as u32),
        "every sequence was run"
    );

    let told_no_row: Vec<String> = taken
        .iter()
        .filter(|row| !rows.contains(*row))
        .map(harness_row_name)
        .collect();
    assert!(
        told_no_row.is_empty(),
        "the observer was told pairs that are no row of the table: {told_no_row:?}"
    );
    let never_taken: Vec<String> = rows
        .iter()
        .filter(|row| !taken.contains(*row))
        .map(harness_row_name)
        .collect();
    assert!(
        never_taken.is_empty(),
        "every row of the harness's transition table is taken; never taken: {never_taken:?}"
    );

    let deferred: BTreeSet<String> = table["deferred"]
        .as_array()
        .expect("deferred")
        .iter()
        .map(|entry| entry["code"].as_str().expect("a code").to_string())
        .collect();
    let mut expected: BTreeSet<String> = classes
        .iter()
        .filter_map(|class| class["expect"]["why"]["code"].as_str())
        .map(str::to_string)
        .chain(APPLICATION_CODES.iter().map(|code| code.to_string()))
        .collect();
    expected.extend(deferred.iter().cloned());
    assert_eq!(
        expected.len(),
        15,
        "the fourteen lift codes and the application's own: {expected:?}"
    );
    let never_reached: Vec<&String> = expected
        .iter()
        .filter(|code| !codes.contains(*code) && !deferred.contains(*code))
        .collect();
    assert!(
        never_reached.is_empty(),
        "every refusal code is reached; never reached: {never_reached:?}"
    );
    let unknown: Vec<&String> = codes
        .iter()
        .filter(|code| !expected.contains(*code))
        .collect();
    assert!(
        unknown.is_empty(),
        "the harness kept codes no class reaches: {unknown:?}"
    );

    let mut not_at_every_site: Vec<String> = Vec::new();
    for site in &sites {
        for outcome in &outcomes {
            if !reached.contains(&(*site, outcome.clone())) {
                not_at_every_site.push(format!("{} {outcome}", harness_row_name(site)));
            }
        }
    }
    assert!(
        not_at_every_site.is_empty(),
        "every host outcome is reached at every call site; never reached: {not_at_every_site:?}"
    );
}

/// The scenarios the lanes replay are the ones the directory holds, and this
/// lane replays each of them.
#[test]
fn every_scenario_of_the_directory_is_replayed_here() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/host_features/scenarios");
    let mut on_disk: Vec<String> = std::fs::read_dir(dir)
        .expect("the scenario directory")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            p.file_stem()
                .expect("a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    on_disk.sort();
    let mut replayed = vec![
        "static_host_clock",
        "static_host_http",
        "static_host_kv",
        "static_host_model",
        "static_host_notify",
    ];
    replayed.extend(HARNESS_SCENARIOS);
    replayed.sort_unstable();
    assert_eq!(on_disk, replayed);
}
