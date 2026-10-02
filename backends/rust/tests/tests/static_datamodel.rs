// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) — Rust
// compile+run gate, the twin of the Kotlin `StaticDatamodelTest` over the same
// fixtures.
//
// The committed SMs under `src/integration/static_datamodel/` are generated
// from `sce-build/tests/fixtures/static_datamodel/<machine>.scxml` (regen:
// `scripts/regen_static_datamodel_rust.sh`). Their variables are fields of
// the policy and every expression — the guard reading `count` and `In()`, the
// `<assign>`s, the `<if>` / `<elseif>` pair, a record's field updates, a
// bounded list — was lowered to Rust, so each machine is constructed with NO
// script engine and its datamodel is read through the published accessors.
// The assertions are the Kotlin test's: one document means one behaviour on
// both backends.

use sce_rust_runtime::saved_state::{wall_clock_ms, SavedState, StateRefusal};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_counter_sm::{
    StaticCounterData, StaticCounterObserve, StaticCounterPersist, StaticCounterPolicy,
    StaticCounterState,
};
use sce_rust_tests::integration::static_datamodel::static_enum_sm::{
    StaticEnumObserve, StaticEnumPersist, StaticEnumPolicy, StaticEnumViewModeEnum,
};
use sce_rust_tests::integration::static_datamodel::static_history_sm::{
    StaticHistoryObserve, StaticHistoryPersist, StaticHistoryPolicy, StaticHistoryState,
};
use sce_rust_tests::integration::static_datamodel::static_host_call_sm::{
    RecordingStaticHostCallActions, StaticHostCallActionsCall, StaticHostCallObserve,
    StaticHostCallPersist, StaticHostCallPolicy, StaticHostCallState,
};
use sce_rust_tests::integration::static_datamodel::static_list_sm::{
    StaticListDayPickedPayload, StaticListInject, StaticListObserve, StaticListPersist,
    StaticListPolicy, StaticListState,
};
use sce_rust_tests::integration::static_datamodel::static_overflow_sm::StaticOverflowPolicy;
use sce_rust_tests::integration::static_datamodel::static_record_sm::{
    StaticRecordDayPickedPayload, StaticRecordDayRecord, StaticRecordInject, StaticRecordObserve,
    StaticRecordPersist, StaticRecordPolicy,
};
use sce_rust_tests::integration::static_datamodel::static_timers_sm::{
    StaticTimersPersist, StaticTimersPolicy,
};

// ── static_counter: scalar variables, a typed guard, assignments ──────────

fn counter() -> Engine<StaticCounterPolicy> {
    let mut engine = Engine::new(StaticCounterPolicy::new());
    engine.initialize();
    engine
}

fn ticks(engine: &mut Engine<StaticCounterPolicy>, n: usize) {
    for _ in 0..n {
        engine.raise_external_by_name("tick", "");
        engine.step();
    }
}

#[test]
fn the_variables_start_at_their_declared_values() {
    // No script-engine argument: the model is engine-free by definition.
    let engine = counter();
    assert_eq!(engine.get_current_state(), StaticCounterState::Counting);
    assert_eq!(engine.policy().count(), 0, "count is declared expr=\"0\"");
    assert!(!engine.policy().ready(), "ready is declared expr=\"false\"");
}

#[test]
fn an_assignment_and_a_conditional_read_the_fields() {
    let mut engine = counter();
    ticks(&mut engine, 5);
    assert_eq!(engine.policy().count(), 5, "each tick assigns count + step");
    assert!(
        engine.policy().ready(),
        "the <if cond=\"count === 5\"> branch ran"
    );

    ticks(&mut engine, 3);
    assert_eq!(engine.policy().count(), 8);
    assert!(
        !engine.policy().ready(),
        "the <elseif cond=\"count > 7\"> branch ran"
    );
}

#[test]
fn the_guard_stops_the_counter_at_its_bound() {
    // `count < 10 && In('counting')`: the eleventh tick finds the guard
    // false, so no transition is taken and count stays where it was.
    let mut engine = counter();
    ticks(&mut engine, 11);
    assert_eq!(engine.policy().count(), 10, "the guard holds count at 10");
}

#[test]
fn a_guard_reading_a_bool_field_takes_its_transition() {
    let mut engine = counter();
    engine.raise_external_by_name("go", "");
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        StaticCounterState::Counting,
        "ready is false, so `go` is not taken"
    );

    ticks(&mut engine, 5);
    engine.raise_external_by_name("go", "");
    engine.step();
    assert!(engine.is_in_final_state(), "ready is true after five ticks");
}

#[test]
fn the_snapshot_is_taken_between_two_host_calls() {
    let mut engine = counter();
    // The first macrostep settles inside initialize().
    let first = engine.snapshot();
    assert_eq!(
        first.configuration.as_slice(),
        &[StaticCounterState::Counting]
    );
    assert_eq!(
        first.data,
        StaticCounterData {
            count: 0,
            ready: false
        }
    );
    assert!(!first.truncated);

    ticks(&mut engine, 5);
    assert_eq!(
        engine.snapshot().data,
        StaticCounterData {
            count: 5,
            ready: true
        },
        "the snapshot carries the datamodel as the macrostep left it"
    );
}

#[test]
fn a_snapshot_does_not_change_afterwards() {
    // Owned: a host holding an earlier snapshot keeps what it saw, whatever
    // the machine does next.
    let mut engine = counter();
    let before = engine.snapshot();
    ticks(&mut engine, 3);
    assert_eq!(
        before.data,
        StaticCounterData {
            count: 0,
            ready: false
        }
    );
    assert_eq!(
        before.configuration.as_slice(),
        &[StaticCounterState::Counting]
    );
}

// ── static_host_call: a host action taking typed datamodel arguments ──────

#[test]
fn a_host_action_takes_typed_datamodel_arguments_with_no_event_in_scope() {
    // `<sce:action name="showAttempts">` in `<onentry>`, its arguments a
    // variable and a comparison over it: each a typed expression, and the
    // host method's parameter types are theirs. The generated recording
    // host: no hand-written stand-in, and its calls cannot drift from the
    // trait they record.
    let mut engine = Engine::new(StaticHostCallPolicy::new(
        RecordingStaticHostCallActions::default(),
    ));
    engine.initialize();
    for _ in 0..4 {
        engine.raise_external_by_name("retry", "");
        engine.step();
    }
    let call = |count, exhausted| StaticHostCallActionsCall::ShowAttempts { count, exhausted };
    assert_eq!(
        engine.policy().actions().calls(),
        &[
            call(0, false),
            call(1, false),
            call(2, false),
            call(3, true)
        ],
        "one call per entry of `idle`, each with the datamodel as it stood; \
         the fourth retry finds `attempts < 3` false and re-enters nothing"
    );
}

#[test]
fn a_machine_that_publishes_no_variable_snapshots_its_configuration_alone() {
    // `attempts` is not declared sce:direction="out", so it is the machine's
    // own: the snapshot carries no data, only where the machine is.
    let mut engine = Engine::new(StaticHostCallPolicy::new(
        RecordingStaticHostCallActions::default(),
    ));
    engine.initialize();
    let snapshot = engine.snapshot();
    assert_eq!(
        snapshot.configuration.as_slice(),
        &[StaticHostCallState::Idle]
    );
    assert!(!snapshot.truncated);
}

// ── static_record: a record variable, built whole, updated field by field ──

fn day(year: u16, month: u8, day_of_month: u8) -> StaticRecordDayRecord {
    StaticRecordDayRecord {
        year,
        month,
        dayOfMonth: day_of_month,
    }
}

fn record() -> Engine<StaticRecordPolicy> {
    let mut engine = Engine::new(StaticRecordPolicy::new());
    engine.initialize();
    engine
}

fn next(engine: &mut Engine<StaticRecordPolicy>, n: usize) {
    for _ in 0..n {
        engine.raise_external_by_name("next", "");
        engine.step();
    }
}

fn pick_day(engine: &mut Engine<StaticRecordPolicy>, year: u16, month: u8, day_of_month: u8) {
    engine.raise_day_picked(StaticRecordDayPickedPayload {
        year,
        month,
        dayOfMonth: day_of_month,
    });
    engine.step();
}

#[test]
fn a_record_variable_starts_as_its_sets_built_it() {
    let engine = record();
    assert_eq!(
        engine.policy().shown(),
        day(2026, 9, 24),
        "one <sce:set> per field of Day"
    );
}

#[test]
fn a_field_is_updated_alone_and_a_guard_reads_it() {
    // `shown.dayOfMonth < DaysInMonth(shown.year, shown.month)` guards
    // `shown.dayOfMonth + 1`, the bound computed by the imported algorithm:
    // September has 30 days, so six steps from the 24th reach 30 and the
    // seventh finds the guard false. The other fields are carried over
    // untouched by each update.
    let mut engine = record();
    next(&mut engine, 7);
    assert_eq!(engine.policy().shown(), day(2026, 9, 30));
}

#[test]
fn the_imported_algorithm_bounds_the_step_by_the_month_it_is_in() {
    // The same guard in February: 28 days in 2027, 29 in the leap year 2028
    // — the algorithm is called with the record's fields each time, not
    // folded to a constant.
    let mut engine = record();
    pick_day(&mut engine, 2027, 2, 27);
    next(&mut engine, 3);
    assert_eq!(engine.policy().shown(), day(2027, 2, 28));

    pick_day(&mut engine, 2028, 2, 27);
    next(&mut engine, 3);
    assert_eq!(engine.policy().shown(), day(2028, 2, 29));
}

#[test]
fn a_typed_payload_replaces_the_record_field_by_field() {
    let mut engine = record();
    pick_day(&mut engine, 2027, 1, 3);
    assert_eq!(engine.policy().shown(), day(2027, 1, 3));
    assert_eq!(
        engine.snapshot().data.shown,
        day(2027, 1, 3),
        "the snapshot carries the record as one value"
    );
}

#[test]
fn a_delivery_without_the_payload_runs_none_of_the_content() {
    // W3C SCXML 3.12.2 / 4.9: the content reads a payload this delivery did
    // not carry — an execution error, which stops the block before any of it
    // runs and goes on the internal queue for the document to answer.
    let mut engine = record();
    engine.raise_external_by_name("day.picked", "");
    engine.step();
    assert_eq!(
        engine.policy().shown(),
        day(2026, 9, 24),
        "no field was assigned"
    );
    assert_eq!(
        engine.policy().refusals(),
        1,
        "error.execution reached the document once"
    );
}

// ── static_list: a list variable, appended to, cleared, held to its bound ──

fn list() -> Engine<StaticListPolicy> {
    let mut engine = Engine::new(StaticListPolicy::new());
    engine.initialize();
    engine
}

fn pick(engine: &mut Engine<StaticListPolicy>, day_of_month: u8) {
    engine.raise_day_picked(StaticListDayPickedPayload {
        year: 2026,
        month: 9,
        dayOfMonth: day_of_month,
    });
    engine.step();
}

#[test]
fn a_list_starts_empty_and_takes_each_append_in_order() {
    let mut engine = list();
    assert!(engine.policy().picked().is_empty());
    pick(&mut engine, 3);
    pick(&mut engine, 1);
    assert_eq!(engine.policy().picked(), &[3, 1]);
}

#[test]
fn a_list_is_measured_by_len_in_an_assignment_and_in_a_guard() {
    // `count` is assigned len(picked) after each append, and `full` is taken
    // only while len(picked) === 3.
    let mut engine = list();
    pick(&mut engine, 4);
    pick(&mut engine, 5);
    assert_eq!(engine.policy().count(), 2);
    engine.raise_external_by_name("full", "");
    engine.step();
    assert_eq!(
        engine.get_current_state(),
        StaticListState::Collecting,
        "two of three picked: the guard holds `full` back"
    );
    pick(&mut engine, 6);
    assert_eq!(engine.policy().count(), 3);
    engine.raise_external_by_name("full", "");
    engine.step();
    assert!(engine.is_in_final_state());
}

#[test]
fn an_append_past_the_capacity_appends_nothing_and_is_an_execution_error() {
    // sce:capacity="3" is kept on every backend: the fourth pick finds the
    // list full, leaves it as it was, and says so with error.execution (W3C
    // SCXML 3.12.2) rather than growing.
    let mut engine = list();
    for d in [5, 6, 7, 8] {
        pick(&mut engine, d);
    }
    assert_eq!(engine.policy().picked(), &[5, 6, 7]);
    assert_eq!(engine.policy().refusals(), 1);
}

// ── static_overflow: an integer operation that overflows is received ─────

fn overflow() -> Engine<StaticOverflowPolicy> {
    let mut engine = Engine::new(StaticOverflowPolicy::new());
    engine.initialize();
    engine
}

#[test]
fn an_assignment_that_overflows_is_skipped_and_is_an_execution_error() {
    // `level + 3` from 253 would be 256, which a uint8 cannot hold: the
    // assignment does not happen — no wrap to 0 — and error.execution says
    // so (SCE_FORGE.md §3.4.1, E12 D5).
    let mut engine = overflow();
    engine.raise_external_by_name("up", "");
    engine.step();
    assert_eq!(engine.policy().level(), 253);
    assert_eq!(engine.policy().refusals(), 0);

    engine.raise_external_by_name("up", "");
    engine.step();
    assert_eq!(
        engine.policy().level(),
        253,
        "the overflowing assignment wrote nothing"
    );
    assert_eq!(
        engine.policy().refusals(),
        1,
        "error.execution reached the document once"
    );
}

#[test]
fn a_condition_that_overflows_is_false_and_is_an_execution_error() {
    // W3C SCXML 5.9: a condition that cannot be evaluated is false, and
    // error.execution says why — `level + 10` overflows at 253.
    let mut engine = overflow();
    engine.raise_external_by_name("up", "");
    engine.step();
    engine.raise_external_by_name("probe", "");
    engine.step();
    assert!(
        !engine.is_in_final_state(),
        "the failing guard took nothing"
    );
    assert_eq!(engine.policy().refusals(), 1);
}

#[test]
fn a_sum_is_checked_at_its_operands_width_not_at_the_comparisons() {
    // `level + 10` is a uint8 operation (a uint8 and a literal that takes
    // its type), so from 250 the sum is 260 — past what it can hold — and
    // the guard fails even though 260 > 0 would hold in a wider type. The
    // width is the operation's own, as in an algorithm (SCE_FORGE.md
    // §3.4.1): one rule for every place an integer is computed.
    let mut engine = overflow();
    engine.raise_external_by_name("probe", "");
    engine.step();
    assert!(!engine.is_in_final_state());
    assert_eq!(engine.policy().refusals(), 1);
}

#[test]
fn a_clear_empties_the_list_and_a_snapshot_keeps_what_it_saw() {
    let mut engine = list();
    pick(&mut engine, 9);
    let before = engine.snapshot();
    engine.raise_external_by_name("reset", "");
    engine.step();
    assert!(engine.policy().picked().is_empty());
    assert!(engine.snapshot().data.picked.is_empty());
    assert_eq!(
        before.data.picked,
        vec![9],
        "the snapshot owns its copy, so the earlier one is unchanged"
    );
}

#[test]
fn the_snapshot_names_the_state_a_guard_held_the_machine_in() {
    let mut engine = list();
    pick(&mut engine, 1);
    engine.raise_external_by_name("full", "");
    engine.step();
    assert_eq!(
        engine.snapshot().configuration.as_slice(),
        &[StaticListState::Collecting],
        "one of three picked: the guard holds `full` back"
    );
}

// ── saving a machine and restoring it into a new process (E17) ───────────
//
// Each saved state goes through its JSON text and back before it is
// restored, as one that crossed a process boundary would.

fn through_json(saved: &SavedState) -> SavedState {
    SavedState::from_json(&saved.to_json()).expect("a saved state reads back")
}

/// The refusal a restore answered with. An `Engine` has no `Debug`, so
/// `expect_err` cannot print the machine it did not expect.
fn refused<M>(restored: Result<M, StateRefusal>) -> StateRefusal {
    match restored {
        Ok(_) => panic!("the restore was expected to be refused"),
        Err(refusal) => refusal,
    }
}

#[test]
fn a_restored_machine_carries_on_where_the_saved_one_stood() {
    // Seven ticks: count 7, and `ready` set at 5 and not yet cleared (the
    // <elseif count > 7> branch). `step` is the machine's own — it is saved
    // though no snapshot publishes it.
    let mut engine = counter();
    ticks(&mut engine, 7);
    let saved = through_json(&engine.save().expect("a running machine saves"));

    let mut restored = Engine::<StaticCounterPolicy>::restore(StaticCounterPolicy::new(), &saved)
        .expect("the same document restores");
    assert_eq!(restored.snapshot(), engine.snapshot());

    ticks(&mut engine, 2);
    ticks(&mut restored, 2);
    assert_eq!(
        restored.snapshot(),
        engine.snapshot(),
        "both run on alike: count 9, and the <elseif> cleared ready"
    );
}

#[test]
fn an_event_raised_and_not_yet_driven_through_is_saved_with_the_machine() {
    // The host raised `tick` and saved before stepping: the event is part of
    // the state — only the internal queue is empty at a macrostep boundary.
    let mut engine = counter();
    engine.raise_external_by_name("tick", "");
    let saved = through_json(&engine.save().expect("saves"));
    assert_eq!(saved.external.len(), 1, "{saved:?}");

    let mut restored = Engine::<StaticCounterPolicy>::restore(StaticCounterPolicy::new(), &saved)
        .expect("restores");
    assert_eq!(restored.policy().count(), 0, "not yet delivered");
    restored.step();
    engine.step();
    assert_eq!(restored.policy().count(), 1, "delivered after the restore");
    assert_eq!(restored.snapshot(), engine.snapshot());
}

#[test]
fn a_restore_runs_no_onentry() {
    // `idle`'s <onentry> calls the host. The saved run already made that
    // call; the restored machine must not make it again.
    let mut engine = Engine::new(StaticHostCallPolicy::new(
        RecordingStaticHostCallActions::default(),
    ));
    engine.initialize();
    engine.raise_external_by_name("retry", "");
    engine.step();
    let saved = through_json(&engine.save().expect("saves"));

    let mut restored = Engine::<StaticHostCallPolicy<_>>::restore(
        StaticHostCallPolicy::new(RecordingStaticHostCallActions::default()),
        &saved,
    )
    .expect("restores");
    assert!(
        restored.policy().actions().calls().is_empty(),
        "restoring entered nothing, so it called nothing"
    );
    restored.raise_external_by_name("retry", "");
    restored.step();
    assert_eq!(
        restored.policy().actions().calls(),
        &[StaticHostCallActionsCall::ShowAttempts {
            count: 2,
            exhausted: false
        }],
        "the next entry sees the attempts the saved run had made"
    );
}

#[test]
fn a_record_and_a_list_are_saved_whole() {
    let mut engine = record();
    pick_day(&mut engine, 2027, 2, 27);
    let saved = through_json(&engine.save().expect("saves"));
    let restored =
        Engine::<StaticRecordPolicy>::restore(StaticRecordPolicy::new(), &saved).expect("restores");
    assert_eq!(restored.policy().shown(), day(2027, 2, 27));

    let mut engine = list();
    pick(&mut engine, 4);
    pick(&mut engine, 2);
    let saved = through_json(&engine.save().expect("saves"));
    let restored =
        Engine::<StaticListPolicy>::restore(StaticListPolicy::new(), &saved).expect("restores");
    assert_eq!(restored.policy().picked(), &[4, 2]);
    assert_eq!(restored.snapshot(), engine.snapshot());
}

/// The saved states the shared fixtures hold — the text every backend must
/// save after the same run and must restore from, which is what makes a state
/// saved by one backend a state another can read.
const SHARED_RECORD: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/saved/static_record.json");
const SHARED_LIST: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/saved/static_list.json");

#[test]
fn a_record_and_a_list_save_the_text_every_backend_saves() {
    // The same runs the Kotlin suite makes, saving the shared fixture's text
    // byte for byte.
    let mut engine = record();
    pick_day(&mut engine, 2027, 2, 27);
    assert_eq!(
        engine.save().expect("saves").to_json(),
        SHARED_RECORD.trim()
    );

    let mut engine = list();
    pick(&mut engine, 4);
    pick(&mut engine, 2);
    assert_eq!(engine.save().expect("saves").to_json(), SHARED_LIST.trim());
}

#[test]
fn a_state_another_backend_saved_is_restored() {
    let restored = Engine::<StaticRecordPolicy>::restore(
        StaticRecordPolicy::new(),
        &SavedState::from_json(SHARED_RECORD).expect("reads"),
    )
    .expect("restores");
    assert_eq!(restored.policy().shown(), day(2027, 2, 27));

    let mut restored = Engine::<StaticListPolicy>::restore(
        StaticListPolicy::new(),
        &SavedState::from_json(SHARED_LIST).expect("reads"),
    )
    .expect("restores");
    assert_eq!(restored.policy().picked(), &[4, 2]);
    pick(&mut restored, 6);
    assert_eq!(
        restored.policy().count(),
        3,
        "the restored machine runs on from the saved values"
    );
}

// ── static_enum: a variable that holds a variant of an imported enum ──────

const SHARED_ENUM: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/saved/static_enum.json");

fn layouts() -> Engine<StaticEnumPolicy> {
    let mut engine = Engine::new(StaticEnumPolicy::new());
    engine.initialize();
    engine
}

fn send(engine: &mut Engine<StaticEnumPolicy>, event: &str) {
    engine.raise_external_by_name(event, "");
    engine.step();
}

#[test]
fn an_enum_variable_starts_at_its_variant_and_the_snapshot_carries_it() {
    let engine = layouts();
    assert_eq!(engine.policy().layout(), StaticEnumViewModeEnum::Month);
    assert_eq!(
        engine.snapshot().data.layout,
        StaticEnumViewModeEnum::Month,
        "a host reads the variable as the machine's own enum type"
    );
}

#[test]
fn an_enum_variable_is_saved_as_the_name_the_document_declares() {
    // The same run the Kotlin suite makes: `agenda_list`, with its
    // underscore, is the document's name for the variant — not the constant
    // this backend spells (`AgendaList`) nor Kotlin's (`AGENDA_LIST`).
    let mut engine = layouts();
    send(&mut engine, "swap");
    send(&mut engine, "agenda");
    assert_eq!(engine.policy().layout(), StaticEnumViewModeEnum::AgendaList);
    assert_eq!(engine.save().expect("saves").to_json(), SHARED_ENUM.trim());
}

#[test]
fn a_state_another_backend_saved_holds_an_enum_that_is_restored() {
    let mut restored = Engine::<StaticEnumPolicy>::restore(
        StaticEnumPolicy::new(),
        &SavedState::from_json(SHARED_ENUM).expect("reads"),
    )
    .expect("restores");
    assert_eq!(
        restored.policy().layout(),
        StaticEnumViewModeEnum::AgendaList
    );
    // `previous` is the machine's own and was restored too: back goes to it.
    send(&mut restored, "back");
    assert_eq!(restored.policy().layout(), StaticEnumViewModeEnum::Week);
}

#[test]
fn a_saved_enum_value_that_is_not_a_declared_variant_is_refused() {
    for (written, wanted) in [
        // Not a variant of the enum at all.
        (r#""layout":"yearly""#, "is not a variant of ViewMode"),
        // The constant a backend spells for it is not the name the document
        // declares, so it is not what a saved state holds.
        (r#""layout":"AgendaList""#, "is not a variant of ViewMode"),
        (r#""layout":"AGENDA_LIST""#, "is not a variant of ViewMode"),
        // A number is not a name.
        (r#""layout":3"#, "is not a text"),
    ] {
        let json = SHARED_ENUM
            .trim()
            .replace(r#""layout":"agenda_list""#, written);
        let refusal = refused(Engine::<StaticEnumPolicy>::restore(
            StaticEnumPolicy::new(),
            &SavedState::from_json(&json).expect("reads"),
        ));
        assert!(refusal.reason().contains(wanted), "{written}: {refusal}");
        assert!(
            refusal.reason().contains("layout"),
            "names the variable: {refusal}"
        );
    }
}

#[test]
fn a_state_saved_from_another_document_is_refused() {
    let mut engine = list();
    pick(&mut engine, 1);
    let saved = engine.save().expect("saves");
    let refusal = refused(Engine::<StaticCounterPolicy>::restore(
        StaticCounterPolicy::new(),
        &saved,
    ));
    assert!(refusal.reason().contains("shape"), "{refusal}");
}

#[test]
fn a_list_longer_than_its_bound_is_refused() {
    // A machine never holds more than sce:capacity="3"; a saved state that
    // claims it did is not one this machine wrote.
    let mut engine = list();
    pick(&mut engine, 1);
    let json = engine
        .save()
        .expect("saves")
        .to_json()
        .replace(r#""picked":[1]"#, r#""picked":[1,2,3,4]"#);
    let refusal = refused(Engine::<StaticListPolicy>::restore(
        StaticListPolicy::new(),
        &SavedState::from_json(&json).expect("reads"),
    ));
    assert!(refusal.reason().contains("bounded by"), "{refusal}");
}

#[test]
fn a_configuration_that_is_not_one_of_the_document_is_refused() {
    let mut engine = list();
    pick(&mut engine, 1);
    let mut saved = engine.save().expect("saves");
    saved.configuration = vec!["nowhere".to_string()];
    let refusal = refused(Engine::<StaticListPolicy>::restore(
        StaticListPolicy::new(),
        &saved,
    ));
    assert!(refusal.reason().contains("nowhere"), "{refusal}");
}

#[test]
fn a_machine_that_is_not_running_is_not_saved() {
    let engine = Engine::new(StaticCounterPolicy::new());
    assert!(engine.save().is_err(), "never started");

    let mut engine = list();
    for d in [1, 2, 3] {
        pick(&mut engine, d);
    }
    engine.raise_external_by_name("full", "");
    engine.step();
    assert!(engine.is_in_final_state());
    assert!(
        engine.save().is_err(),
        "ended at a top-level <final>: nothing left to resume"
    );
}

// ── what a <history> recorded is part of the saved state ─────────────────

const SHARED_HISTORY: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/saved/static_history.json");

/// The run `sce-build/tests/fixtures/static_datamodel/static_history.scxml`
/// describes: `last`, `deepest` and `crew_all` all recorded, standing in
/// `paused`.
const HISTORY_RUN: [&str; 7] = [
    "faster",
    "boost",
    "pause",
    "work",
    "left_next",
    "right_next",
    "break",
];

fn history_machine() -> Engine<StaticHistoryPolicy> {
    let mut engine = Engine::new(StaticHistoryPolicy::new());
    engine.initialize();
    engine
}

fn history_run(engine: &mut Engine<StaticHistoryPolicy>, events: &[&str]) {
    for event in events {
        engine.raise_external_by_name(event, "");
        engine.step();
    }
}

fn history_restored() -> Engine<StaticHistoryPolicy> {
    Engine::<StaticHistoryPolicy>::restore(
        StaticHistoryPolicy::new(),
        &SavedState::from_json(SHARED_HISTORY).expect("reads"),
    )
    .expect("restores")
}

#[test]
fn a_history_is_saved_with_the_machine_as_the_text_every_backend_saves() {
    let mut engine = history_machine();
    history_run(&mut engine, &HISTORY_RUN);
    assert_eq!(
        engine.save().expect("saves").to_json(),
        SHARED_HISTORY.trim()
    );
}

#[test]
fn a_machine_that_has_exited_nothing_records_no_history() {
    let engine = history_machine();
    let saved = engine.save().expect("saves");
    assert!(saved.history.is_empty(), "{:?}", saved.history);
}

#[test]
fn a_restored_machine_resumes_through_each_history_as_the_saved_one_would() {
    // Shallow: the child `fast` was active, and entering it takes its initial
    // child.
    let mut engine = history_restored();
    history_run(&mut engine, &["resume_last"]);
    assert_eq!(engine.get_current_state(), StaticHistoryState::Cruise);
    assert_eq!(engine.policy().resumed(), 1);

    // Deep: the atomic state itself.
    let mut engine = history_restored();
    history_run(&mut engine, &["resume_deep"]);
    assert_eq!(engine.get_current_state(), StaticHistoryState::Burst);

    // Deep, below a <parallel>: one state in EACH region.
    let mut engine = history_restored();
    history_run(&mut engine, &["resume_crew"]);
    let configuration = engine.snapshot().configuration;
    for state in [StaticHistoryState::L2, StaticHistoryState::R2] {
        assert!(configuration.contains(&state), "{configuration:?}");
    }
    for state in [StaticHistoryState::L1, StaticHistoryState::R1] {
        assert!(!configuration.contains(&state), "{configuration:?}");
    }
}

#[test]
fn a_history_that_was_never_recorded_takes_its_default_after_a_restore() {
    // Saved before any exit of `running`, `last` has nothing recorded, so
    // resuming through it takes its default transition, as an unsaved
    // machine would.
    let mut saved = SavedState::from_json(SHARED_HISTORY).expect("reads");
    saved.history.retain(|(id, _)| id != "last");
    let mut engine = Engine::<StaticHistoryPolicy>::restore(StaticHistoryPolicy::new(), &saved)
        .expect("restores");
    history_run(&mut engine, &["resume_last"]);
    assert_eq!(engine.get_current_state(), StaticHistoryState::Slow);
}

#[test]
fn a_saved_history_survives_its_machine_being_saved_again() {
    let mut engine = history_restored();
    assert_eq!(
        engine.save().expect("saves").to_json(),
        SHARED_HISTORY.trim(),
        "restoring and saving again changes nothing"
    );
    history_run(&mut engine, &["resume_last", "pause"]);
    let again = engine.save().expect("saves");
    assert_eq!(
        again.history,
        vec![
            (
                "crew_all".to_string(),
                vec!["l2".to_string(), "r2".to_string()]
            ),
            ("deepest".to_string(), vec!["cruise".to_string()]),
            ("last".to_string(), vec!["fast".to_string()]),
        ],
        "resumed through `last`, then left `running` again: `deepest` now records `cruise`"
    );
}

/// `SHARED_HISTORY` with `history` replaced by `entries`, and the refusal the
/// restore answers with.
fn refused_history(entries: &[(&str, &[&str])]) -> StateRefusal {
    let mut saved = SavedState::from_json(SHARED_HISTORY).expect("reads");
    saved.history = entries
        .iter()
        .map(|(id, states)| {
            (
                (*id).to_string(),
                states.iter().map(|s| (*s).to_string()).collect(),
            )
        })
        .collect();
    refused(Engine::<StaticHistoryPolicy>::restore(
        StaticHistoryPolicy::new(),
        &saved,
    ))
}

#[test]
fn a_history_the_document_does_not_declare_is_refused() {
    let refusal = refused_history(&[("nowhere", &["fast"])]);
    assert!(refusal.reason().contains("does not declare"), "{refusal}");

    // A document with no <history> refuses a state that records one.
    let mut engine = counter();
    ticks(&mut engine, 1);
    let mut saved = engine.save().expect("saves");
    saved.history = vec![("last".to_string(), vec!["counting".to_string()])];
    let refusal = refused(Engine::<StaticCounterPolicy>::restore(
        StaticCounterPolicy::new(),
        &saved,
    ));
    assert!(refusal.reason().contains("does not declare"), "{refusal}");
}

#[test]
fn a_history_naming_a_state_the_document_lacks_is_refused() {
    let refusal = refused_history(&[("last", &["warp"])]);
    assert!(refusal.reason().contains("warp"), "{refusal}");
}

#[test]
fn a_history_that_records_nothing_or_a_state_twice_is_refused() {
    let refusal = refused_history(&[("last", &[])]);
    assert!(refusal.reason().contains("at least one"), "{refusal}");
    let refusal = refused_history(&[("last", &["fast", "fast"])]);
    assert!(refusal.reason().contains("twice"), "{refusal}");
}

#[test]
fn a_history_naming_a_state_outside_its_parent_is_refused() {
    let refusal = refused_history(&[("last", &["l1"])]);
    assert!(refusal.reason().contains("not below"), "{refusal}");
    // The parent itself is below nothing.
    let refusal = refused_history(&[("last", &["running"])]);
    assert!(refusal.reason().contains("not below"), "{refusal}");
}

#[test]
fn a_shallow_history_records_children_and_a_deep_one_atomic_states() {
    let refusal = refused_history(&[("last", &["cruise"])]);
    assert!(refusal.reason().contains("not a child"), "{refusal}");
    let refusal = refused_history(&[("deepest", &["fast"])]);
    assert!(refusal.reason().contains("has children"), "{refusal}");
}

#[test]
fn a_history_that_is_part_of_no_configuration_is_refused() {
    // `running` holds one active child, and these are in two.
    let refusal = refused_history(&[("deepest", &["slow", "burst"])]);
    assert!(refusal.reason().contains("no configuration"), "{refusal}");
    // A <parallel> holds every region, and this is one of two.
    let refusal = refused_history(&[("crew_all", &["l2"])]);
    assert!(refusal.reason().contains("no configuration"), "{refusal}");
    // Two states of one region.
    let refusal = refused_history(&[("crew_all", &["l1", "l2", "r1"])]);
    assert!(refusal.reason().contains("no configuration"), "{refusal}");
}

#[test]
fn a_saved_state_whose_object_repeats_a_name_is_not_this_format() {
    // `{"last":[..],"last":[..]}` has no single meaning: a reader that took
    // the first and one that took the last would restore two machines from one
    // text, so neither backend reads it.
    let text = SHARED_HISTORY
        .trim()
        .replace(r#""last":["fast"]"#, r#""last":["fast"],"last":["slow"]"#);
    let refusal = SavedState::from_json(&text).expect_err("a repeated name");
    assert!(refusal.reason().contains("appears twice"), "{refusal}");
}

// ── a delayed <send> still waiting is part of the saved state ────────────────
//
// `static_timers.scxml` arms four on entering `waiting`; each appends a digit to
// `trace` when delivered, so the number says which arrived and in what order.
// Everything runs on a host-owned clock, so no case sleeps and none depends on
// how loaded the machine is.

const SHARED_TIMERS: &str =
    include_str!("../../../../sce-build/tests/fixtures/static_datamodel/saved/static_timers.json");
const SHARED_TIMERS_MIDWAY: &str = include_str!(
    "../../../../sce-build/tests/fixtures/static_datamodel/saved/static_timers_midway.json"
);

/// The wall-clock moment the shared fixtures were saved at, as the Kotlin suite
/// has it: 2023-11-14T22:13:20Z.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

fn timers() -> Engine<StaticTimersPolicy> {
    let mut engine = Engine::new(StaticTimersPolicy::new());
    engine.set_clock(SceClock::Manual(0));
    engine.initialize();
    engine
}

/// `text` restored `elapsed_ms` of wall-clock time after it was saved, into a
/// process whose own clock starts at 0.
fn timers_restored(text: &str, elapsed_ms: u64) -> Engine<StaticTimersPolicy> {
    Engine::<StaticTimersPolicy>::restore_with(
        StaticTimersPolicy::new(),
        &SavedState::from_json(text).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS + elapsed_ms,
    )
    .expect("restores")
}

#[test]
fn a_waiting_send_is_saved_as_the_wall_clock_moment_it_comes_due() {
    // The same runs the Kotlin suite makes, saving the shared fixtures' text
    // byte for byte: four sends in the order they would be delivered, `beat`
    // ahead of `echo` since they are due together and were sent in that order.
    let mut engine = timers();
    assert_eq!(
        engine.save_at(SAVED_AT_MS).expect("saves").to_json(),
        SHARED_TIMERS.trim()
    );

    // 1.5 s later `inner` has been delivered. What is left is due at the same
    // wall-clock moments as before, because the moment is what is saved — the
    // wait left is what changed, and a restore is told the time, not the wait.
    engine.advance_time_ms(1500);
    assert_eq!(engine.policy().trace(), 1);
    assert_eq!(
        engine.save_at(SAVED_AT_MS + 1500).expect("saves").to_json(),
        SHARED_TIMERS_MIDWAY.trim()
    );
}

#[test]
fn a_save_without_a_clock_reads_the_wall_clock() {
    let engine = timers();
    let before = wall_clock_ms();
    let saved = engine.save().expect("saves");
    let after = wall_clock_ms();
    // `inner` waits 1 s on an engine whose clock is at 0.
    let due = saved.pending[0].due;
    assert!(
        (before + 1000..=after + 1000).contains(&due),
        "due {due} is not 1 s past the wall clock at the save ({before}..={after})"
    );
}

#[test]
fn a_restored_machine_delivers_each_waiting_send_when_its_moment_comes() {
    // Back 500 ms after the save: `inner` has 500 ms left, `beat` and `echo`
    // 1500, `timeout` 4500.
    let mut engine = timers_restored(SHARED_TIMERS, 500);
    assert_eq!(engine.policy().trace(), 0);

    engine.advance_time_ms(499);
    assert_eq!(engine.policy().trace(), 0, "`inner` is not due yet");
    engine.advance_time_ms(1);
    assert_eq!(engine.policy().trace(), 1, "`inner`, at 500 ms");
    engine.advance_time_ms(999);
    assert_eq!(engine.policy().trace(), 1, "`beat` and `echo` are not due");
    engine.advance_time_ms(1);
    assert_eq!(
        engine.policy().trace(),
        124,
        "`beat` then `echo`, at 1500 ms"
    );
    assert!(!engine.is_in_final_state());
    engine.advance_time_ms(3000);
    assert_eq!(engine.policy().trace(), 1243, "`timeout`, at 4500 ms");
    assert!(engine.is_in_final_state());
}

#[test]
fn a_send_already_due_when_the_machine_comes_back_is_delivered_in_its_order() {
    // A minute away: every wait ran out while the process was dead. Each is
    // delivered, one macrostep apart, in the order the saved machine would
    // have — `beat` ahead of `echo`, which were due together.
    let mut engine = timers_restored(SHARED_TIMERS, 60_000);
    assert_eq!(
        engine.policy().trace(),
        0,
        "nothing is delivered by restoring"
    );
    engine.advance_time_ms(0);
    assert_eq!(engine.policy().trace(), 1243);
    assert!(engine.is_in_final_state());
}

#[test]
fn a_wait_that_was_half_over_when_saved_has_the_rest_to_run() {
    // Saved 1.5 s in with `inner` delivered and `trace` 1; restored at once.
    let mut engine = timers_restored(SHARED_TIMERS_MIDWAY, 1500);
    assert_eq!(engine.policy().trace(), 1);
    engine.advance_time_ms(499);
    assert_eq!(engine.policy().trace(), 1);
    engine.advance_time_ms(1);
    assert_eq!(engine.policy().trace(), 124, "`beat` then `echo`");
    engine.advance_time_ms(3000);
    assert_eq!(engine.policy().trace(), 1243);
}

#[test]
fn a_restored_send_can_still_be_cancelled_by_its_id() {
    // `stop` cancels `timer`, so what was saved has to carry the id it names.
    let mut engine = timers_restored(SHARED_TIMERS, 0);
    engine.raise_external_by_name("stop", "");
    engine.step();
    engine.advance_time_ms(60_000);
    assert_eq!(engine.policy().trace(), 124, "`timeout` was cancelled");
    assert!(!engine.is_in_final_state());
}

#[test]
fn a_machine_restored_from_a_text_saves_that_text_again() {
    // Nothing is lost on the way through: the order, the moments and the ids a
    // second save writes are the first's.
    for (text, elapsed) in [(SHARED_TIMERS, 0), (SHARED_TIMERS_MIDWAY, 1500)] {
        let engine = timers_restored(text, elapsed);
        assert_eq!(
            engine
                .save_at(SAVED_AT_MS + elapsed)
                .expect("saves")
                .to_json(),
            text.trim()
        );
    }
}

#[test]
fn a_waiting_send_naming_an_event_the_document_lacks_is_refused() {
    let text = SHARED_TIMERS.replace(r#""event":"beat""#, r#""event":"warp""#);
    let refusal = refused(Engine::<StaticTimersPolicy>::restore_with(
        StaticTimersPolicy::new(),
        &SavedState::from_json(&text).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    ));
    assert!(refusal.reason().contains("pending[1]"), "{refusal}");
    assert!(refusal.reason().contains("warp"), "{refusal}");
}
