// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Accepted Subset §2.15, "Saving and restoring", for a variable of a record that
// holds a list whose type states a lower count (docs/adr/0086, section 15.6;
// docs/adr/0087, "Landing order" item 8).
//
// `static_lift_list.scxml`: `text.given` lifts the text its payload carries into
// `answer`, an `optional<record:Plan>` of `lift_answer_list.scxml` whose `items` is a
// list of one to three records of `lift_item.scxml`. A `BoundedList<T, 3>` holds the
// bound of three and not the lower count of one, so a restore holds the count where
// it reads the list, as a delivery's reader does: a saved state whose list is one
// short of it is refused by name, never restored as a value the type forbids.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::Engine;
use sce_rust_tests::integration::static_datamodel::static_lift_list_sm::{
    StaticLiftListPersist as Persist, StaticLiftListPolicy as Policy,
};

/// The text of a plan of two items, which the machine lifts whole.
const WEEK: &str =
    r#"{"title":"Week","items":[{"title":"Plan","at":9},{"title":"Ship","at":17}],"priority":1}"#;

/// The items of [`WEEK`] as a saved state writes them.
const WEEK_ITEMS: &str = r#""items":[{"title":"Plan","at":9},{"title":"Ship","at":17}]"#;

/// The text a running machine that has lifted [`WEEK`] once saves.
fn saved_week() -> String {
    let mut engine = Engine::new(Policy::new());
    engine.initialize().expect("the machine starts");
    let data = serde_json::json!({ "text": WEEK }).to_string();
    engine.raise_external_by_name("text.given", &data);
    engine.step();
    let text = engine.save().expect("saves").to_json();
    assert!(
        text.contains(WEEK_ITEMS),
        "the record's list is written: {text}"
    );
    text
}

/// The machine `text` restores, or why it is refused.
fn restored(text: &str) -> Result<Engine<Policy>, String> {
    let saved = SavedState::from_json(text).expect("reads");
    Engine::<Policy>::restore(Policy::new(), &saved).map_err(|refusal| refusal.reason().to_string())
}

#[test]
fn a_saved_list_within_its_counts_is_restored_as_it_was_saved() {
    let text = saved_week();
    let engine = restored(&text).unwrap_or_else(|why| panic!("restores: {why}"));
    // What was restored is saved as it was read, at every depth of the record.
    assert_eq!(engine.save().expect("saves again").to_json(), text);
}

#[test]
fn a_saved_list_short_of_its_lower_count_is_refused_by_name() {
    let whole = saved_week();
    let empty = whole.replace(WEEK_ITEMS, r#""items":[]"#);
    assert_ne!(whole, empty, "the record's list is written");
    match restored(&empty) {
        Ok(_) => panic!("a list short of its lower count is restored: {empty}"),
        Err(why) => assert!(
            why.contains("'answer.items' holds 0 elements, fewer than the 1 its type declares"),
            "{why}"
        ),
    }
}
