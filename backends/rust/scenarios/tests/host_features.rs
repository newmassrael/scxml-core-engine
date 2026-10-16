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

use std::path::Path;
use std::sync::{Arc, Mutex};

use sce_rust_host_features::{
    serve, ClockSource, HttpRequest, HttpResponse, Reading, ReferenceHost, ScriptedClock, Shared,
    Transport, TransportFailure, Zone,
};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};
use sce_rust_scenarios::integration::static_datamodel::{
    static_host_clock_sm::{StaticHostClockPersist, StaticHostClockPolicy},
    static_host_http_sm::{StaticHostHttpPersist, StaticHostHttpPolicy},
    static_host_kv_sm::{StaticHostKvPersist, StaticHostKvPolicy},
    static_host_notify_sm::{StaticHostNotifyPersist, StaticHostNotifyPolicy},
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

struct Rig {
    host: Shared<Scripted>,
    clock: Clock,
    script: Arc<Mutex<Script>>,
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
                other => panic!("`{other}` is not a host directive"),
            }
        }
    }

    /// What the host was started with since the last step, as a scenario writes
    /// it: the request as its record, `at_ms` being the one integer field.
    fn starts(&self) -> Value {
        let starts = self.host.lock().expect("the host").take_starts();
        Value::Array(
            starts
                .into_iter()
                .map(|start| {
                    let mut request = Map::new();
                    for (name, text) in start.params {
                        let value = if name == "at_ms" {
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

fn replay<P: StatePolicy>(
    mut engine: Engine<P>,
    save: impl Fn(&Engine<P>) -> SavedState,
    machine: &str,
) {
    let scenario = scenario(machine);
    assert_eq!(
        scenario["machine"], machine,
        "the scenario names its machine"
    );
    let clock: Clock = Arc::new(Mutex::new(ScriptedClock::new()));
    let script = Arc::new(Mutex::new(Script::default()));
    let host: Shared<Scripted> = Arc::new(Mutex::new(ReferenceHost::new(
        Box::new(Arc::clone(&clock)) as Box<dyn ClockSource + Send>,
        Scripted(Arc::clone(&script)),
    )));
    serve(&mut engine, &host);
    let rig = Rig {
        host,
        clock,
        script,
    };
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
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
                "step {n}: the machine's events do not match `{event}`"
            );
            let data = step.get("data").map(Value::to_string).unwrap_or_default();
            engine.raise_external_by_name(event, &data);
            settle(&mut engine);
        }
        let expect = &step["expect"];
        let started = rig.starts();
        if let Some(want) = expect.get("host_invokes") {
            assert_eq!(&started, want, "step {n} ({note}): what the host was asked");
        }
        if let Some(want) = expect.get("host_state") {
            assert_eq!(
                &rig.host_state(want),
                want,
                "step {n} ({note}): what the host holds"
            );
        }
        let saved: Value =
            serde_json::from_str(&save(&engine).to_json()).expect("a saved state is JSON");
        if let Some(state) = expect.get("state") {
            assert_eq!(
                &saved["current"], state,
                "step {n} ({note}): the current state"
            );
        }
        if let Some(variables) = expect.get("variables").and_then(Value::as_object) {
            for (name, want) in variables {
                let got = &saved["variables"][name];
                assert!(
                    holds(got, want),
                    "step {n} ({note}): variable `{name}` is {got}, not {want}"
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
    );
}

#[test]
fn a_machine_reads_the_clock_of_its_host() {
    replay(
        Engine::new(StaticHostClockPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_clock",
    );
}

#[test]
fn a_machine_posts_notifications_through_its_host() {
    replay(
        Engine::new(StaticHostNotifyPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_notify",
    );
}

#[test]
fn a_machine_makes_an_http_exchange_through_its_host() {
    replay(
        Engine::new(StaticHostHttpPolicy::new()),
        |engine| engine.save_at(0).expect("saves"),
        "static_host_http",
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
    assert_eq!(
        on_disk,
        [
            "static_host_clock",
            "static_host_http",
            "static_host_kv",
            "static_host_notify"
        ]
    );
}
