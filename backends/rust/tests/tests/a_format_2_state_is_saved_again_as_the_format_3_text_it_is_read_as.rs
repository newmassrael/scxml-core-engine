// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0090, section 7): a
// state saved in format 2 stored an event's origin as an `origin` text and an
// `origintype`, and a reader of format 3 reads them as the pair and the sender
// the table of that section gives. A machine restored from such a text writes
// format 3, holding those pairs.
//
// Each reading fixture in sce-build/tests/fixtures/static_datamodel/saved/format2/
// is restored at the wall-clock moment the shared fixtures were saved at, saved
// again at once, and held byte for byte to the format-3 text of the same name in
// `read_as/`, which every backend writes for it (`format2/README.md`). The trees
// are of `static_invoke_tree.scxml`: the shared tree as the backends other than
// C11 wrote it, the same tree with a queued event's origin stored as C11 stored
// it, and the tree whose root queue holds one event for each row of the table.
// The timers are of `static_timers.scxml`, whose `internal` act stored a sender
// that an internal event does not carry.

use sce_rust_runtime::saved_state::{SavedState, StateRefusal, FORMAT_2, FORMAT_3};
use sce_rust_runtime::{Engine, SceClock};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::{
    StaticInvokeTreePersist, StaticInvokeTreePolicy,
};
use sce_rust_tests::integration::static_datamodel::static_timers_sm::{
    StaticTimersPersist, StaticTimersPolicy,
};

/// The wall-clock moment the shared fixtures were saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

/// A reading fixture: its name, its format-2 text, and the format-3 text it is
/// read as.
type ReadingFixture = (&'static str, &'static str, &'static str);

/// The format-2 trees, each with the format-3 text it is read as.
const TREES: [ReadingFixture; 3] = [
    (
        "static_invoke_tree_working",
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/static_invoke_tree_working.json"
        ),
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/read_as/static_invoke_tree_working.json"
        ),
    ),
    (
        "static_invoke_tree_working_c11",
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/static_invoke_tree_working_c11.json"
        ),
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/read_as/static_invoke_tree_working_c11.json"
        ),
    ),
    (
        "static_invoke_tree_working_origins",
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/static_invoke_tree_working_origins.json"
        ),
        include_str!(
            "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/read_as/static_invoke_tree_working_origins.json"
        ),
    ),
];

/// The format-2 timers, with the format-3 text they are read as.
const TIMERS: ReadingFixture = (
    "static_timers",
    include_str!(
        "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/static_timers.json"
    ),
    include_str!(
        "../../../../sce-build/tests/fixtures/static_datamodel/saved/format2/read_as/static_timers.json"
    ),
);

/// `format2` read, after the check that it is the format-2 text it is named as.
fn read(name: &str, format2: &str) -> SavedState {
    assert!(
        format2.starts_with(&format!(r#"{{"format":{FORMAT_2},"#)),
        "{name} is a format-2 text"
    );
    SavedState::from_json(format2).unwrap_or_else(|refusal| panic!("{name} reads: {refusal}"))
}

/// The text the machine `restored` saves at once with `save`, after the check
/// that it is format 3. A refusal of the restore or of the save fails the case,
/// naming the fixture `name`.
fn saved_again<M>(
    name: &str,
    restored: Result<M, StateRefusal>,
    save: impl Fn(&M) -> Result<SavedState, StateRefusal>,
) -> String {
    let machine = restored.unwrap_or_else(|refusal| panic!("{name} restores: {refusal}"));
    let again = save(&machine).unwrap_or_else(|refusal| panic!("{name} saves: {refusal}"));
    assert_eq!(again.format(), FORMAT_3, "{name} is saved as format 3");
    again.to_json()
}

#[test]
fn a_format_2_tree_is_saved_again_as_the_format_3_tree_it_is_read_as() {
    for (name, format2, read_as) in TREES {
        let saved = read(name, format2);
        let text = saved_again(
            name,
            Engine::<StaticInvokeTreePolicy>::restore_with(
                StaticInvokeTreePolicy::new(),
                &saved,
                SceClock::Manual(0),
                SAVED_AT_MS,
            ),
            |engine| engine.save_at(SAVED_AT_MS),
        );
        assert_eq!(text, read_as.trim_end(), "{name}");
    }
}

#[test]
fn format_2_timers_are_saved_again_as_the_format_3_timers_they_are_read_as() {
    let (name, format2, read_as) = TIMERS;
    let saved = read(name, format2);
    let text = saved_again(
        name,
        Engine::<StaticTimersPolicy>::restore_with(
            StaticTimersPolicy::new(),
            &saved,
            SceClock::Manual(0),
            SAVED_AT_MS,
        ),
        |engine| engine.save_at(SAVED_AT_MS),
    );
    assert_eq!(text, read_as.trim_end(), "{name}");
}

#[test]
fn a_format_2_text_is_read_without_a_machine_as_the_pairs_it_is_written_back_with() {
    // The reader alone, before any machine: the text of the state read is the
    // `read_as/` text, so the pairs come from the reading, not from the restore.
    for (name, format2, read_as) in TREES.into_iter().chain([TIMERS]) {
        assert_eq!(read(name, format2).to_json(), read_as.trim_end(), "{name}");
    }
}
