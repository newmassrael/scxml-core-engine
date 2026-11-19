// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0109, D11 (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Saving and restoring"):
// a state the first version of a document saved is restored into the next version
// through the migration the next version declares. `static_migration_1` stands in
// `waiting` with `count` and `total` moved; `static_migration_2` renamed the state
// `pending`, widened `total` from int32 to int64 and added `ceiling`, and declares
// the step from the first version's committed shape to its own. The restore carries
// the state by that step, and a state of a shape no step starts from is still
// refused as one of another document.
//
// Both machines are generated from `sce-build/tests/fixtures/static_datamodel/`
// (`scripts/regen_static_datamodel_rust.sh`). The C++ half is
// `tests/integration/ASavedStateIsMigratedIntoTheNextVersionTest.cpp`.

use sce_rust_runtime::saved_state::{SavedState, StateRefusal};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_migration_1_sm::{
    StaticMigration1Persist, StaticMigration1Policy,
};
use sce_rust_tests::integration::static_datamodel::static_migration_2_sm::{
    StaticMigration2Persist, StaticMigration2Policy, StaticMigration2State,
};

/// The wall-clock moment both machines are saved and restored at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// A state of the first version, standing in `waiting` after three ticks, as a
/// process boundary carries it: written to JSON and read back.
fn saved_by_version_1() -> SavedState {
    let mut engine = Engine::new(StaticMigration1Policy::new());
    let Ok(()) = engine.initialize();
    engine.raise_external_by_name("go", "");
    engine.step();
    for _ in 0..3 {
        engine.raise_external_by_name("tick", "");
        engine.step();
    }
    let saved = engine
        .save_at(SAVED_AT_MS)
        .expect("a running machine saves");
    SavedState::from_json(&saved.to_json()).expect("a saved state reads back")
}

fn restored_into_version_2(
    saved: &SavedState,
) -> Result<Engine<StaticMigration2Policy>, StateRefusal> {
    Engine::<StaticMigration2Policy>::restore_with(
        StaticMigration2Policy::new(),
        saved,
        SceClock::default(),
        SAVED_AT_MS,
    )
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
fn a_state_of_the_first_version_restores_into_the_next_through_its_step() {
    let saved = saved_by_version_1();
    assert_eq!(
        saved.configuration,
        vec!["waiting".to_string()],
        "{saved:?}"
    );

    let restored = restored_into_version_2(&saved).expect("the declared step carries the state");
    assert_eq!(
        restored.get_current_state(),
        StaticMigration2State::Pending,
        "`waiting` is renamed `pending`"
    );
    assert_eq!(restored.policy().count(), 3, "a variable kept as it was");
    assert_eq!(
        restored.policy().total(),
        -6,
        "an int32 widened to an int64 holds the value it held"
    );
    assert_eq!(
        restored.policy().ceiling(),
        16,
        "a variable the step adds holds the constant the step gives it"
    );
}

#[test]
fn a_migrated_machine_runs_on_as_the_next_version_and_saves_under_its_shape() {
    let mut restored = restored_into_version_2(&saved_by_version_1())
        .expect("the declared step carries the state");
    restored.raise_external_by_name("tick", "");
    restored.step();
    assert_eq!(
        restored.policy().count(),
        4,
        "`pending` takes the tick `waiting` took"
    );
    assert_eq!(restored.policy().total(), -8);

    let mut fresh = Engine::new(StaticMigration2Policy::new());
    let Ok(()) = fresh.initialize();
    let own = fresh.save_at(SAVED_AT_MS).expect("saves");
    let again = restored
        .save_at(SAVED_AT_MS)
        .expect("a migrated machine saves");
    assert_eq!(
        again.shape, own.shape,
        "a restored machine saves under the shape of the version it runs"
    );
    restored_into_version_2(&SavedState::from_json(&again.to_json()).expect("reads back"))
        .expect("a state the next version saved restores with no step applied");
}

#[test]
fn a_state_of_a_shape_no_step_starts_from_is_refused_by_its_shape() {
    let saved = SavedState {
        shape: "0".repeat(64),
        ..saved_by_version_1()
    };
    let refusal = refused(restored_into_version_2(&saved));
    assert!(
        refusal.reason().contains("of a document of shape"),
        "{}",
        refusal.reason()
    );
}
