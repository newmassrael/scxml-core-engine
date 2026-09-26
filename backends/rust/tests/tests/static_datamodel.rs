// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

use sce_rust_runtime::saved_state::{SavedState, StateRefusal};
use sce_rust_runtime::Engine;
use sce_rust_tests::integration::static_datamodel::static_counter_sm::{
    StaticCounterData, StaticCounterObserve, StaticCounterPersist, StaticCounterPolicy,
    StaticCounterState,
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
