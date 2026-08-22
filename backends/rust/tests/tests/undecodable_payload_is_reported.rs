// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out. Rust AOT.
//
// The clause gives a payload three readings and names the third "otherwise".
// That word is where a belief leaves the system quietly. A host serializes
// `{"done":true}`, something truncates it to `{"done":`, and the clause is
// satisfied: the content becomes a string. The document then evaluates
// `_event.data.done`, finds nothing, and takes the transition it would have
// taken had the host sent a payload with no `done` field at all. Nothing is
// raised — the fallback is CORRECT behaviour, not an error — so before this
// fixture nothing anywhere said it had happened.
//
// These two deliveries are what no pre-existing accessor separates:
//
//   answer  {"done":              the payload never parsed
//   answer  {"ready":true}        it parsed; `done` is genuinely absent
//
// Same `_event.data.done`, same transition, same configuration, same counters.
// The difference is a broken sender versus a working one, and it is exactly the
// difference an operator is trying to establish at 3am.
//
// Fixture: integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
// (canonical, shared with the C++ / C11 / Go / Kotlin / Python channels).
//
// Regeneration (after fixture or template edit):
//   scripts/regen_undecodable_payload_is_reported.sh

use std::sync::Arc;

use sce_rust_runtime::{Engine, IScriptEngine};
use sce_rust_tests::integration::undecodable_payload_is_reported::{
    UndecodablePayloadIsReportedEvent as Event, UndecodablePayloadIsReportedPolicy as Policy,
    UndecodablePayloadIsReportedState as State,
};

/// Content that announces an object and stops. The shape a truncated write, a
/// half-flushed buffer or a serializer that died mid-record actually produces.
const TRUNCATED_OBJECT: &str = r#"{"done":"#;
/// The same failure announced with `[` instead of `{`, under the other event
/// name, so a channel that reports "the last event" rather than "the last event
/// that lost a payload" cannot pass by accident.
const TRUNCATED_ARRAY: &str = "[1,2";
/// W3C test 562 sends exactly this shape and requires it to arrive as a string.
/// Counting it would make the new statistic fire on documents that are working.
const PROSE: &str = "just a sentence";
/// What the host meant to send.
const INTACT_OBJECT: &str = r#"{"done":true}"#;

fn started() -> (Engine<Policy>, Arc<dyn IScriptEngine>) {
    let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
    let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
    engine.initialize();
    (engine, script_engine)
}

/// The fixture's `<assign>`s are the only witness that a delivery ran anything
/// at all — without them a passing run cannot be told apart from a run in which
/// no transition fired.
fn counter(engine: &Engine<Policy>, script_engine: &Arc<dyn IScriptEngine>, name: &str) -> i64 {
    sce_rust_runtime::helpers::datamodel_read::read_int(
        &**script_engine,
        engine.policy().session_id.as_deref(),
        name,
    )
    .unwrap_or_else(|| panic!("the fixture declares `{name}` in its datamodel"))
}

fn deliver(engine: &mut Engine<Policy>, event: Event, payload: &str) {
    engine.raise_external(event, payload, "");
    engine.step();
}

/// The axis: content that asked for the structured reading and did not get it
/// is counted.
#[test]
fn a_payload_that_announced_structure_and_did_not_parse_is_counted() {
    let (mut engine, se) = started();
    assert_eq!(
        engine.undecodable_payloads(),
        0,
        "nothing has been delivered before the first event"
    );

    deliver(&mut engine, Event::Answer, TRUNCATED_OBJECT);

    assert_eq!(
        counter(&engine, &se, "answers"),
        1,
        "the `answer` transition did not run, so nothing below is measuring a \
         delivery that reached the document"
    );
    assert_eq!(
        engine.undecodable_payloads(),
        1,
        "the host sent `{TRUNCATED_OBJECT}`, which announces an object and does \
         not parse as one. W3C SCXML B.2.8.1 correctly delivers it as a string; \
         the host that built it has no other way to learn its payload stopped \
         being structure"
    );
    assert_eq!(
        engine.get_current_state(),
        State::Waiting,
        "the reading a payload got must not change which transition fired"
    );
}

/// The other half. A count that also counts success cannot be used to detect
/// failure, and the reading the clause calls "otherwise" is the NORMAL outcome
/// for a document whose author wrote prose.
#[test]
fn prose_and_a_payload_that_parsed_are_not_counted() {
    let (mut engine, se) = started();

    deliver(&mut engine, Event::Note, PROSE);
    assert_eq!(
        counter(&engine, &se, "notes"),
        1,
        "the `note` transition did not run"
    );
    assert_eq!(
        engine.undecodable_payloads(),
        0,
        "`{PROSE}` is the third reading working as W3C SCXML B.2.8.1 specifies \
         and as W3C test 562 requires. A diagnostic that fires when nothing is \
         wrong is one nobody reads"
    );

    deliver(&mut engine, Event::Answer, INTACT_OBJECT);
    assert_eq!(
        engine.get_current_state(),
        State::Accepted,
        "the guard `_event.data.done` did not hold for `{INTACT_OBJECT}`, so the \
         structured reading did not happen and the zero below would be proving \
         nothing"
    );
    assert_eq!(
        engine.undecodable_payloads(),
        0,
        "a payload that parsed was counted as one that did not"
    );
}

/// Why the query has to exist at all: the two deliveries the fixture's comment
/// names are identical through every accessor a host had.
#[test]
fn the_loss_is_not_derivable_from_any_other_accessor() {
    let (mut broken, broken_se) = started();
    deliver(&mut broken, Event::Answer, TRUNCATED_OBJECT);

    let (mut intact, intact_se) = started();
    // Valid JSON, and `done` is genuinely absent — the innocent explanation an
    // operator has to rule out.
    deliver(&mut intact, Event::Answer, r#"{"ready":true}"#);

    assert_eq!(
        (
            broken.get_current_state(),
            broken.get_active_states().to_vec(),
            broken.is_running(),
            broken.is_in_final_state(),
            counter(&broken, &broken_se, "answers"),
        ),
        (
            intact.get_current_state(),
            intact.get_active_states().to_vec(),
            intact.is_running(),
            intact.is_in_final_state(),
            counter(&intact, &intact_se, "answers"),
        ),
        "this fixture exists because a lost payload and an absent field are \
         indistinguishable through the accessors a host had; if they ever \
         differ, the fixture stopped measuring what it claims"
    );

    assert_eq!(
        (broken.undecodable_payloads(), intact.undecodable_payloads()),
        (1, 0),
        "the two runs agree on everything else, so this count is the only thing \
         that separates a broken sender from a working one"
    );
}

/// A count says a payload was lost; a host debugging a stalled supervisor needs
/// to know which delivery lost it.
#[test]
fn the_engine_names_the_delivery_that_lost_its_payload() {
    let (mut engine, _se) = started();
    assert_eq!(
        engine.last_undecodable_payload(),
        None,
        "nothing has been delivered yet"
    );

    deliver(&mut engine, Event::Answer, TRUNCATED_OBJECT);
    assert_eq!(
        engine.last_undecodable_payload(),
        Some(Event::Answer),
        "the engine counted a lost payload but cannot say which delivery lost it"
    );

    // A second loss, under the other event name: the accessor has to track the
    // last event THAT LOST A PAYLOAD, not the last event.
    deliver(&mut engine, Event::Note, TRUNCATED_ARRAY);
    assert_eq!(
        engine.undecodable_payloads(),
        2,
        "the count is a count, not a flag"
    );
    assert_eq!(engine.last_undecodable_payload(), Some(Event::Note));

    // And a delivery that succeeds must leave both alone — otherwise the last
    // name would drift to whatever arrived most recently.
    deliver(&mut engine, Event::Answer, INTACT_OBJECT);
    assert_eq!(
        engine.get_current_state(),
        State::Accepted,
        "the intact payload did not take the guarded transition, so the two \
         assertions below are not measuring a successful delivery"
    );
    assert_eq!(
        (
            engine.undecodable_payloads(),
            engine.last_undecodable_payload()
        ),
        (2, Some(Event::Note)),
        "a delivery that parsed moved a record that belongs to one that did not"
    );
}
