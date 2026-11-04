// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What the reference host holds apart from the scenarios the lanes share
// (docs/SCE_HOST_FEATURES.md): its system clock, what it refuses, and what it does
// not serve.

use sce_rust_host_features::{
    AnswerSchema, ExhaustiveProvider, HttpRequest, HttpResponse, ModelProvider, Outcome,
    OutcomeClass, Params, ReferenceHost, ScriptedAnswer, ScriptedClock, ScriptedProvider,
    SystemClock, Transport, TransportFailure, ANSWER_MAX_BYTES, MODEL_GENERATE,
};

struct Unreachable;

impl Transport for Unreachable {
    fn exchange(&mut self, _: &HttpRequest) -> Result<Option<HttpResponse>, TransportFailure> {
        Err(TransportFailure)
    }
}

fn done(outcome: Option<Outcome>) -> serde_json::Value {
    match outcome.expect("a standard type is served") {
        Outcome::Done(data) => serde_json::from_str(&data).expect("a result is JSON"),
        other => panic!("the host did not complete: {other:?}"),
    }
}

/// The system clock's monotonic reading never goes below one already answered.
#[test]
fn the_system_clock_answers_a_monotonic_reading_that_does_not_decrease() {
    let mut host = ReferenceHost::new(Box::new(SystemClock::new()), Unreachable);
    let mut last = -1i64;
    for _ in 0..50 {
        let fields = done(host.start("sce-host-clock-now", "now", &Params::new()));
        assert_eq!(fields["error"], "");
        let mono: i64 = fields["monotonic_ms"]
            .as_str()
            .expect("digits")
            .parse()
            .expect("a number");
        assert!(
            mono >= last,
            "the monotonic reading went from {last} to {mono}"
        );
        last = mono;
    }
    #[cfg(unix)]
    assert_eq!(
        done(host.start("sce-host-clock-zone", "zone", &Params::new()))["error"],
        ""
    );
}

#[test]
fn an_operation_outside_the_standard_set_is_not_served() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    assert!(host.start("x-app-host", "id", &Params::new()).is_none());
}

#[test]
fn a_key_with_a_nul_is_refused() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    let params: Params = [("key".to_string(), "a\0b".to_string())].into();
    assert!(matches!(
        host.start("sce-host-kv-get", "id", &params),
        Some(Outcome::Refuse(_))
    ));
}

/// Keys are ordered by their UTF-8 bytes.
#[test]
fn keys_are_ordered_by_their_utf8_bytes() {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    for key in ["\u{FF5E}", "\u{1F600}"] {
        let params: Params = [
            ("key".to_string(), key.to_string()),
            ("value".to_string(), "v".to_string()),
        ]
        .into();
        host.start("sce-host-kv-put", "put", &params);
    }
    let walk: Params = [
        ("after".to_string(), String::new()),
        ("prefix".to_string(), String::new()),
    ]
    .into();
    assert_eq!(
        done(host.start("sce-host-kv-next-key", "walk", &walk))["key"],
        "\u{FF5E}"
    );
}

// ── Generated text (docs/SCE_HOST_FEATURES.md, section 6) ──

/// The table of answer schemas a machine would hand its host.
const SCHEMAS: &[AnswerSchema] = &[AnswerSchema {
    id: "urn:sce:schema:answer:1",
    json_schema: "{}",
    gbnf: "root ::= \"{}\"",
}];

/// A generation's request.
fn ask(prompt: &str, answer_schema: &str, max_tokens: &str, temperature: &str) -> Params {
    [
        ("prompt".to_string(), prompt.to_string()),
        ("answer_schema".to_string(), answer_schema.to_string()),
        ("max_tokens".to_string(), max_tokens.to_string()),
        ("temperature_milli".to_string(), temperature.to_string()),
    ]
    .into()
}

fn model_host(provider: impl ModelProvider + Send + 'static) -> ReferenceHost<Unreachable> {
    let mut host = ReferenceHost::new(Box::new(ScriptedClock::new()), Unreachable);
    host.set_answer_schemas(SCHEMAS);
    host.set_model_provider(Box::new(provider));
    host
}

fn scripted(texts: &[&str]) -> ScriptedProvider {
    ScriptedProvider {
        answers: texts
            .iter()
            .map(|text| ScriptedAnswer {
                text: (*text).to_string(),
                finish: "stop".to_string(),
            })
            .collect(),
        constrained: true,
        ..ScriptedProvider::default()
    }
}

/// Each argument the operation does not accept is refused before the provider
/// is asked, so the answer it would give stays.
#[test]
fn a_generation_the_operation_does_not_accept_is_refused() {
    let mut host = model_host(scripted(&["kept"]));
    for (why, params) in [
        ("an empty prompt", ask("", "", "8", "0")),
        ("max_tokens zero", ask("p", "", "0", "0")),
        ("a temperature past 2000", ask("p", "", "8", "2001")),
        (
            "an id the table does not hold",
            ask("p", "urn:sce:schema:other:1", "8", "0"),
        ),
    ] {
        assert!(
            matches!(
                host.start(MODEL_GENERATE, "gen", &params),
                Some(Outcome::Refuse(_))
            ),
            "{why} was not refused"
        );
    }
    let fields = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "2000")));
    assert_eq!(fields["text"], "kept", "a refusal took an answer");
}

/// A constrained provider says so only for a request that names an answer
/// schema; its tokens are bytes.
#[test]
fn a_scripted_provider_is_constrained_only_under_an_answer_schema() {
    let mut host = model_host(scripted(&["one", "two"]));
    let free = done(host.start(MODEL_GENERATE, "gen", &ask("ab", "", "8", "0")));
    assert_eq!(free["constrained"], false);
    assert_eq!(free["error"], "");
    let held = done(host.start(
        MODEL_GENERATE,
        "gen",
        &ask("abc", "urn:sce:schema:answer:1", "8", "0"),
    ));
    assert_eq!(held["constrained"], true);
    assert_eq!(held["prompt_tokens"], 3);
    assert_eq!(held["answer_tokens"], 3);
    assert_eq!(held["finish"], "stop");
    assert_eq!(held["model"], "scripted");
    let none = done(host.start(MODEL_GENERATE, "gen", &ask("ab", "", "8", "0")));
    assert_eq!(none["error"], "unavailable", "no answer left");
    assert_eq!(none["text"], "");
}

/// An answer past the bound of `text` is never cut.
#[test]
fn an_answer_past_its_bound_fails_whole() {
    let long = "a".repeat(ANSWER_MAX_BYTES + 1);
    let fits = "a".repeat(ANSWER_MAX_BYTES);
    let mut host = model_host(scripted(&[&long, &fits]));
    let past = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")));
    assert_eq!(past["error"], "answer_too_large");
    let whole = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")));
    assert_eq!(whole["text"].as_str().map(str::len), Some(ANSWER_MAX_BYTES));
}

/// A prompt and `max_tokens` past the context fail and take no answer.
#[test]
fn a_generation_past_the_context_fails_and_keeps_the_answer() {
    let mut host = model_host(ScriptedProvider {
        context: 10,
        ..scripted(&["kept"])
    });
    let past = done(host.start(MODEL_GENERATE, "gen", &ask("abc", "", "8", "0")));
    assert_eq!(past["error"], "context_exceeded");
    let within = done(host.start(MODEL_GENERATE, "gen", &ask("abc", "", "7", "0")));
    assert_eq!(within["text"], "kept");
}

/// The exhaustive provider answers each class it is named, in order, behind the
/// host's own judgement of the request.
#[test]
fn the_exhaustive_provider_answers_each_class_in_turn() {
    let mut host = model_host(ExhaustiveProvider::new(vec![
        OutcomeClass::Answer {
            text: "t".to_string(),
            finish: "length".to_string(),
        },
        OutcomeClass::Failure("busy".to_string()),
        OutcomeClass::Refusal,
        OutcomeClass::Hold,
    ]));
    let answer = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")));
    assert_eq!(
        (answer["text"].as_str(), answer["finish"].as_str()),
        (Some("t"), Some("length"))
    );
    assert_eq!(answer["model"], "exhaustive");
    assert_eq!(answer["constrained"], false);
    let failed = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")));
    assert_eq!(failed["error"], "busy");
    assert!(matches!(
        host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")),
        Some(Outcome::Refuse(_))
    ));
    assert_eq!(
        host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")),
        Some(Outcome::Hold)
    );
    let past = done(host.start(MODEL_GENERATE, "gen", &ask("p", "", "8", "0")));
    assert_eq!(past["error"], "unavailable", "asked past the sequence");
}
