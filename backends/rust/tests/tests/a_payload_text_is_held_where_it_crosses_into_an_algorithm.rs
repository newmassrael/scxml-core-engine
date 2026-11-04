// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A payload's text handed to an imported algorithm inside a record is held to the
// bound of the algorithm's record where it crosses (docs/adr/0086, section 7.4), run
// with no script engine. The committed machine under `src/integration/static_datamodel/`
// is generated from
// `sce-build/tests/fixtures/static_datamodel/static_payload_text_call.scxml`
// (regen: `scripts/regen_static_datamodel_rust.sh`).
//
// `Echo` cannot fail and `Bump` declares `may-fail`. For each, a label within eight
// bytes crosses whole and the record the algorithm gives back is kept; a label one
// byte past them fails the call as an assignment past a bound fails: error.execution
// is raised once and the record keeps what it held. `Bump`'s own failure, an
// overflow, is taken the same way.

use sce_rust_runtime::Engine;
use sce_rust_tests::integration::static_datamodel::static_payload_text_call_sm::StaticPayloadTextCallPolicy;

fn machine() -> Engine<StaticPayloadTextCallPolicy> {
    let mut engine = Engine::new(StaticPayloadTextCallPolicy::new());
    engine.initialize();
    engine
}

/// Deliver `labelled.taken` with the reading `sensor` and the label `label`.
fn take(engine: &mut Engine<StaticPayloadTextCallPolicy>, sensor: u8, label: &str) {
    engine.raise_external_by_name(
        "labelled.taken",
        &format!(r#"{{"sensor":{sensor},"label":"{label}"}}"#),
    );
    engine.step();
}

/// From here on, a `labelled.taken` is handed to `Bump`.
fn use_bump(engine: &mut Engine<StaticPayloadTextCallPolicy>) {
    engine.raise_external_by_name("use.bump", "");
    engine.step();
}

/// Eight bytes, the bound of `label`.
const FITS: &str = "abcdefgh";
/// One byte past it.
const PAST: &str = "abcdefghi";

#[test]
fn a_label_within_its_bound_crosses_into_an_algorithm_that_cannot_fail() {
    let mut engine = machine();
    take(&mut engine, 3, FITS);
    let policy = engine.policy();
    assert_eq!(policy.errors(), 0, "a label that fits fails nothing");
    assert_eq!(
        policy.echoed().sensor,
        3,
        "the record the algorithm gave back"
    );
    assert_eq!(policy.echoed().label, FITS, "the label crossed whole");
}

#[test]
fn a_label_past_its_bound_fails_the_call_of_an_algorithm_that_cannot_fail() {
    let mut engine = machine();
    take(&mut engine, 3, FITS);
    take(&mut engine, 4, PAST);
    let policy = engine.policy();
    assert_eq!(policy.errors(), 1, "error.execution is raised once");
    assert_eq!(
        policy.echoed().sensor,
        3,
        "nothing of the delivery is written"
    );
    assert_eq!(policy.echoed().label, FITS, "the record keeps what it held");
}

#[test]
fn a_label_within_its_bound_crosses_into_an_algorithm_that_may_fail() {
    let mut engine = machine();
    use_bump(&mut engine);
    take(&mut engine, 4, FITS);
    let policy = engine.policy();
    assert_eq!(policy.errors(), 0, "a label that fits fails nothing");
    assert_eq!(
        policy.bumped().sensor,
        5,
        "the record the algorithm gave back"
    );
    assert_eq!(policy.bumped().label, FITS, "the label crossed whole");
    assert_eq!(
        policy.echoed().sensor,
        0,
        "the other algorithm was not called"
    );
}

#[test]
fn a_label_past_its_bound_fails_the_call_of_an_algorithm_that_may_fail() {
    let mut engine = machine();
    use_bump(&mut engine);
    take(&mut engine, 4, FITS);
    take(&mut engine, 6, PAST);
    let policy = engine.policy();
    assert_eq!(policy.errors(), 1, "error.execution is raised once");
    assert_eq!(
        policy.bumped().sensor,
        5,
        "nothing of the delivery is written"
    );
    assert_eq!(policy.bumped().label, FITS, "the record keeps what it held");
}

/// The algorithm's own failure is taken as the bound's is: one error.execution and
/// nothing written. The two differ only in the cause the adapter returns, which no
/// statement reads.
#[test]
fn an_algorithm_that_fails_on_its_own_is_taken_as_the_bound_is() {
    let mut engine = machine();
    use_bump(&mut engine);
    take(&mut engine, 4, FITS);
    take(&mut engine, 255, FITS);
    let policy = engine.policy();
    assert_eq!(
        policy.errors(),
        1,
        "the overflow raises error.execution once"
    );
    assert_eq!(
        policy.bumped().sensor,
        5,
        "nothing of the delivery is written"
    );
}
