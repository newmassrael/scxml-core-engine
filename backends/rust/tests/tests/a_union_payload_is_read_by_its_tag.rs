// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Rust machine reads a payload that holds a union by testing its tag
// (docs/adr/0092, section 6.1), run with no script engine. The committed
// machine under `src/integration/static_datamodel/` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_union.scxml` (regen:
// `scripts/regen_static_datamodel_rust.sh`).
//
// A delivery is lifted tag first into the machine's own sum (section 4), each
// branch the tag's test proves reads the fields of its case, a field two cases
// share is read under a test that leaves both, a list a case holds is walked,
// and each shape of a list is narrowed on the loop's item. A key of another
// case is stepped over; a tag no variant names and a field its case lacks are
// refused whole, naming the place.

use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};
use sce_rust_tests::integration::static_datamodel::static_union_sm::{
    StaticUnionEvent, StaticUnionPersist, StaticUnionPolicy,
};

/// A delivery of `shapes.sent`: `one` and the elements of `many` as written,
/// and no `maybe`.
fn shapes(one: &str, many: &[&str]) -> String {
    format!(
        r#"{{"one":{one},"many":[{}],"maybe":null}}"#,
        many.join(",")
    )
}

/// The present value of `maybe` is read where a test proves it present and in
/// its case (docs/adr/0055, section 3; docs/adr/0092, section 6.1), and not
/// where it is absent or in another case.
#[test]
fn the_present_value_of_an_optional_union_is_read_where_proven() {
    let mut engine = machine();
    deliver(
        &mut engine,
        r#"{"one":{"kind":"dot"},"many":[{"kind":"dot"}],"maybe":{"kind":"rect","width":6,"height":2}}"#,
    );
    assert_eq!(engine.policy().refusals(), 0);
    assert_eq!(engine.policy().maybe_width(), 6);
    deliver(
        &mut engine,
        r#"{"one":{"kind":"dot"},"many":[{"kind":"dot"}],"maybe":{"kind":"circle","radius":1}}"#,
    );
    assert_eq!(
        engine.policy().maybe_width(),
        6,
        "another case reads nothing"
    );
    deliver(
        &mut engine,
        &shapes(r#"{"kind":"dot"}"#, &[r#"{"kind":"dot"}"#]),
    );
    assert_eq!(
        engine.policy().maybe_width(),
        6,
        "an absent value reads nothing"
    );
}

/// A union variable starts at the case it builds where it is declared, a
/// field of it is written where its case is proven, and an optional one starts
/// absent and takes the payload's union whole (docs/adr/0092, section 6.1
/// items 5 and 6, and its amendment of 2026-11-07).
#[test]
fn a_union_variable_is_built_written_where_proven_and_taken_whole() {
    let mut engine = machine();
    assert_eq!(engine.policy().built().kind().sce_name(), "rect");
    assert_eq!(engine.policy().built().proven_rect().width, 1);
    assert_eq!(engine.policy().built().proven_rect().height, 2);
    assert!(engine.policy().kept().is_none(), "it starts absent");
    deliver(
        &mut engine,
        &shapes(r#"{"kind":"circle","radius":7}"#, &[r#"{"kind":"dot"}"#]),
    );
    assert_eq!(engine.policy().refusals(), 0);
    assert_eq!(
        engine.policy().built().proven_rect().width,
        2,
        "written where the case is proven"
    );
    assert_eq!(engine.policy().built().proven_rect().height, 2);
    let kept = engine.policy().kept().clone().expect("taken whole");
    assert_eq!(kept.kind().sce_name(), "circle");
    assert_eq!(kept.proven_circle().radius, 7);
}

/// A saved state holds a union variable as one object, its tag beside its
/// case's fields, an absent one as `null`, and a restore reads each back by its
/// tag (docs/adr/0092, section 6.1 item 7).
#[test]
fn a_saved_state_holds_a_union_variable_as_its_tag_beside_its_case() {
    let mut engine = machine();
    let text = engine.save_at(0).expect("saves").to_json();
    assert!(text.contains(r#""kept":null"#), "{text}");
    deliver(
        &mut engine,
        &shapes(r#"{"kind":"circle","radius":7}"#, &[r#"{"kind":"dot"}"#]),
    );
    let saved = engine.save_at(0).expect("saves");
    let text = saved.to_json();
    assert!(
        text.contains(r#""built":{"kind":"rect","width":2,"height":2}"#),
        "{text}"
    );
    assert!(
        text.contains(r#""kept":{"kind":"circle","radius":7}"#),
        "{text}"
    );
    let restored = Engine::<StaticUnionPolicy>::restore_with(
        StaticUnionPolicy::new(),
        &SavedState::from_json(&text).expect("reads back"),
        SceClock::Manual(0),
        0,
    )
    .expect("restores");
    assert_eq!(restored.policy().built(), engine.policy().built());
    assert_eq!(restored.policy().kept(), engine.policy().kept());
    // A tag no case of the union names is refused, naming the variable.
    let stray = text.replace(
        r#""kind":"circle","radius":7"#,
        r#""kind":"oval","radius":7"#,
    );
    let refusal = match Engine::<StaticUnionPolicy>::restore_with(
        StaticUnionPolicy::new(),
        &SavedState::from_json(&stray).expect("reads back"),
        SceClock::Manual(0),
        0,
    ) {
        Ok(_) => panic!("a tag of no case is refused"),
        Err(refusal) => refusal,
    };
    assert!(
        refusal
            .reason()
            .contains("'kept' (oval) is not a case of union_shape"),
        "{}",
        refusal.reason()
    );
}

fn machine() -> Engine<StaticUnionPolicy> {
    let mut engine = Engine::new(StaticUnionPolicy::new());
    let Ok(()) = engine.initialize();
    engine
}

fn deliver(engine: &mut Engine<StaticUnionPolicy>, data: &str) {
    engine.raise_external_by_name("shapes.sent", data);
    engine.step();
}

#[test]
fn each_case_is_read_where_its_tag_is_tested() {
    let mut engine = machine();
    deliver(
        &mut engine,
        &shapes(
            r#"{"kind":"circle","radius":7}"#,
            &[
                r#"{"kind":"rect","width":3,"height":4}"#,
                r#"{"kind":"disc","radius":5}"#,
                r#"{"kind":"dot"}"#,
            ],
        ),
    );
    let policy = engine.policy();
    assert_eq!(policy.refusals(), 0, "a delivery of its type is no error");
    assert_eq!(policy.last_kind().sce_name(), "circle");
    // `radius`, which `circle` and `disc` share, under the test of both.
    assert_eq!(policy.radius(), 7);
    assert_eq!(policy.many_shapes(), 3);
    assert_eq!(policy.many_width(), 3);
    assert_eq!(policy.many_radius(), 5);
}

#[test]
fn a_case_of_two_fields_and_a_case_of_a_list_are_read_by_their_tests() {
    let mut engine = machine();
    deliver(
        &mut engine,
        &shapes(
            r#"{"kind":"rect","width":3,"height":4}"#,
            &[r#"{"kind":"dot"}"#],
        ),
    );
    assert_eq!(engine.policy().width(), 3);
    assert_eq!(engine.policy().height(), 4);
    deliver(
        &mut engine,
        &shapes(
            r#"{"kind":"path","points":[1,2,3]}"#,
            &[r#"{"kind":"dot"}"#],
        ),
    );
    assert_eq!(engine.policy().points(), 3);
    deliver(
        &mut engine,
        &shapes(
            r#"{"kind":"label","text":"abc","note":null}"#,
            &[r#"{"kind":"dot"}"#],
        ),
    );
    assert_eq!(engine.policy().labels(), 1);
    deliver(
        &mut engine,
        &shapes(r#"{"kind":"dot"}"#, &[r#"{"kind":"dot"}"#]),
    );
    assert_eq!(engine.policy().dots(), 1);
    assert_eq!(engine.policy().refusals(), 0);
}

/// The tag is read wherever it stands, and a key of another case, or `null`
/// under one (the flat spelling), is stepped over (section 2.1).
#[test]
fn the_tag_last_and_another_cases_keys_are_read_as_its_case() {
    let mut engine = machine();
    deliver(
        &mut engine,
        &shapes(
            r#"{"radius":9,"width":null,"points":[1],"kind":"disc"}"#,
            &[r#"{"kind":"dot","radius":null}"#],
        ),
    );
    let policy = engine.policy();
    assert_eq!(policy.refusals(), 0);
    assert_eq!(policy.last_kind().sce_name(), "disc");
    assert_eq!(policy.radius(), 9);
    assert_eq!(policy.points(), 0, "the key of another case is not read");
}

#[test]
fn a_tag_no_variant_names_is_refused_whole() {
    let mut engine = machine();
    let data = shapes(
        r#"{"kind":"dot"}"#,
        &[r#"{"kind":"dot"}"#, r#"{"kind":"oval"}"#],
    );
    deliver(&mut engine, &data);
    let policy = engine.policy();
    assert_eq!(
        policy.refusals(),
        1,
        "error.execution reached the document once"
    );
    assert_eq!(policy.many_shapes(), 0, "none of the content ran");
    let refusal = StaticUnionPolicy::new()
        .lift_event_payload(StaticUnionEvent::ShapesSent, &data)
        .expect_err("a tag no variant names");
    assert_eq!(refusal.pointer(), "/many/1/kind");
    assert_eq!(
        refusal.reason(),
        "'many/1/kind' (oval) is not a variant of Kind"
    );
}

#[test]
fn a_field_its_case_reads_and_the_delivery_lacks_is_refused_naming_it() {
    let refusal = StaticUnionPolicy::new()
        .lift_event_payload(
            StaticUnionEvent::ShapesSent,
            &shapes(r#"{"kind":"rect","width":3}"#, &[r#"{"kind":"dot"}"#]),
        )
        .expect_err("a field of the case missing");
    assert_eq!(refusal.pointer(), "/one/height");
    assert_eq!(refusal.reason(), "the event's data has no 'one/height'");
}
