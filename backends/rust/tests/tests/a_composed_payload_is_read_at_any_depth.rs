// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Rust machine reads a composed payload at any depth (docs/adr/0086,
// section 6), run with no script engine. The committed machine under
// `src/integration/static_datamodel/` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`
// (regen: `scripts/regen_static_datamodel_rust.sh`).
//
// A delivery whose every list is full is lifted into the machine's own types,
// and each read at depths 1 to 3 is what a host reads back. A delivery one
// past a bound at depth 2 is refused whole (section 4.2): none of the content
// runs, `error.execution` is raised once, and the refusal names the path to
// the place as a JSON Pointer.

use sce_rust_runtime::{Engine, StatePolicy};
use sce_rust_tests::integration::static_datamodel::static_composed_payload_sm::{
    StaticComposedPayloadEvent, StaticComposedPayloadPolicy,
};

/// One item of the batch: `tags` holds `tags` texts.
fn item(id: u32, tags: usize) -> String {
    let tags: Vec<String> = (0..tags).map(|n| format!("\"tag{n}\"")).collect();
    format!(
        r#"{{"id":{id},"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[{}]}}"#,
        tags.join(",")
    )
}

/// A delivery of `batch.sent` whose every list is full, the second item's
/// `tags` holding `second_tags` texts.
fn batch(second_tags: usize) -> String {
    format!(
        r#"{{"items":[{},{},{}],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}}"#,
        item(1, 4),
        item(2, second_tags),
        item(3, 4)
    )
}

fn machine() -> Engine<StaticComposedPayloadPolicy> {
    let mut engine = Engine::new(StaticComposedPayloadPolicy::new());
    let Ok(()) = engine.initialize();
    engine
}

fn deliver(engine: &mut Engine<StaticComposedPayloadPolicy>, data: &str) {
    engine.raise_external_by_name("batch.sent", data);
    engine.step();
}

#[test]
fn a_full_delivery_is_read_at_every_depth() {
    let mut engine = machine();
    deliver(&mut engine, &batch(4));
    let policy = engine.policy();
    assert_eq!(policy.refusals(), 0, "a delivery of its type is no error");
    // Depth 1: the length of a list of the payload.
    assert_eq!(policy.item_count(), 3);
    // Depth 2: a loop over each item's list, and a field of an element read
    // through its index.
    assert_eq!(policy.tag_count(), 12);
    assert_eq!(policy.word_count(), 12);
    assert_eq!(policy.id_sum(), 6);
    // A list of lists, walked at both depths and read by two indices.
    assert_eq!(policy.cell_sum(), 10);
    assert_eq!(policy.cell(), 3);
    // Depth 3: the length of the first item's `tags`.
    assert_eq!(policy.first_tags(), 4);
    // An element that may be absent, present.
    assert_eq!(policy.first_maybe(), Some(7));
    // A list taken whole into a list of a larger bound.
    assert_eq!(policy.late(), &[5u8, 6][..]);
}

#[test]
fn a_list_past_its_bound_at_depth_two_is_refused_whole() {
    let mut engine = machine();
    deliver(&mut engine, &batch(5));
    let policy = engine.policy();
    assert_eq!(
        policy.refusals(),
        1,
        "error.execution reached the document once"
    );
    assert_eq!(policy.item_count(), 0, "none of the content ran");
    assert_eq!(
        policy.late(),
        &[] as &[u8],
        "nothing of the delivery was kept"
    );
}

#[test]
fn the_refusal_names_the_path_to_the_place() {
    let mut policy = StaticComposedPayloadPolicy::new();
    let refusal = policy
        .lift_event_payload(StaticComposedPayloadEvent::BatchSent, &batch(5))
        .expect_err("a list past its bound at depth two");
    assert_eq!(
        refusal.reason(),
        "'items/1/tags' holds 5 elements, past the 4 its type declares"
    );
}

/// Every bound of the schema is held where the delivery is read (docs/adr/0086,
/// section 4.4): a text and a byte string inside an element, the top-level text,
/// and the count of the top-level list, each one past its bound in a delivery
/// otherwise full, refused whole with the path to the place.
#[test]
fn every_bound_of_the_schema_is_held_where_the_delivery_is_read() {
    let full = batch(4);
    let cases = [
        (
            full.replacen("\"label\":\"abcdefgh\"", "\"label\":\"abcdefghi\"", 1),
            "'items/0/label' is 9 bytes, past the 8 its type declares",
        ),
        (
            full.replacen("\"blob\":\"wxyz\"", "\"blob\":\"vwxyz\"", 1),
            "'items/0/blob' is 5 bytes, past the 4 its type declares",
        ),
        (
            full.replacen("\"note\":\"abcd\"", "\"note\":\"abcde\"", 1),
            "'note' is 5 bytes, past the 4 its type declares",
        ),
        (
            full.replacen("\"items\":[", &format!("\"items\":[{},", item(0, 1)), 1),
            "'items' holds 4 elements, past the 3 its type declares",
        ),
    ];
    for (delivery, reason) in &cases {
        assert_ne!(delivery, &full, "the case changes the delivery: {reason}");
        let mut policy = StaticComposedPayloadPolicy::new();
        let refusal = policy
            .lift_event_payload(StaticComposedPayloadEvent::BatchSent, delivery)
            .expect_err(reason);
        assert_eq!(refusal.reason(), *reason);
        // Refused whole: the machine runs none of the content.
        let mut engine = machine();
        deliver(&mut engine, delivery);
        assert_eq!(engine.policy().refusals(), 1, "{reason}");
        assert_eq!(engine.policy().item_count(), 0, "{reason}");
    }
}
