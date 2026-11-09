// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Rust machine sends a composed value by its name (docs/adr/0086, section
// 5.2), run with no script engine. The committed machine under
// `src/integration/static_datamodel/` is generated from
// `sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml`
// (regen: `scripts/regen_static_datamodel_rust.sh`).
//
// The first delivery of `batch.sent` is forwarded to the machine itself: four
// lists and an optional list named as the parts of the payload they are, and a
// list variable the machine took the payload's `late` into. The delivery that
// comes back is lifted as any delivery is (section 4), so it is refused whole
// if anything written is outside its type; it is not, and every value read
// from it at depths 1 to 3 is what the first delivery held.

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::static_datamodel::static_composed_send_sm::StaticComposedSendPolicy;

/// One item of the batch: `tags` holds `tags` texts.
fn item(id: u32, tags: usize) -> String {
    let tags: Vec<String> = (0..tags).map(|n| format!("\"tag{n}\"")).collect();
    format!(
        r#"{{"id":{id},"amount":"9007199254740993","label":"abcdefgh","level":"high","due":null,"blob":"wxyz","tags":[{}]}}"#,
        tags.join(",")
    )
}

/// A delivery of `batch.sent` whose every list is full, an element of `maybe`
/// absent and `spare` present.
fn batch() -> String {
    format!(
        r#"{{"items":[{},{},{}],"grid":[[1,2],[3,4]],"levels":["low","critical"],"maybe":[7,null],"spare":[1,2],"note":"abcd","late":[5,6]}}"#,
        item(1, 4),
        item(2, 3),
        item(3, 4)
    )
}

#[test]
fn a_delivery_forwarded_by_its_names_comes_back_whole() {
    let mut engine = Engine::new(StaticComposedSendPolicy::new());
    let Ok(()) = engine.initialize();
    engine.raise_external_by_name("batch.sent", &batch());
    engine.step();
    let policy = engine.policy();
    assert_eq!(
        policy.refusals(),
        0,
        "the delivery the machine sent itself fits its schema"
    );
    assert_eq!(
        policy.deliveries(),
        2,
        "the forwarded delivery came back on the internal queue"
    );
    // Depth 1: the lengths of a list of records and a list of enum values.
    assert_eq!(policy.item_count(), 3);
    assert_eq!(policy.level_count(), 2);
    // Depth 2: each item's list, and a field of an element read by its index.
    assert_eq!(policy.tag_count(), 11);
    assert_eq!(policy.id_sum(), 6);
    // A list of lists, walked at both depths.
    assert_eq!(policy.cell_sum(), 10);
    // An element that may be absent, present.
    assert_eq!(policy.first_maybe(), Some(7));
    // The list variable, sent by its name.
    assert_eq!(policy.late_sum(), 11);
    assert_eq!(policy.late(), &[5u8, 6][..]);
}
