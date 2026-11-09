// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The machine generated for every build of a product line, built under one
//! feature set, is the machine that set's variant generates alone (docs/adr/0057
//! §6, the commuting law `select(gen(D)) = gen(project(D))`).
//!
//! The generator proves it of the text, build by build, before it writes
//! anything. This builds the crate under the feature set it is run with, so
//! rustc resolves the `#[cfg]` itself, and drives both machines through every
//! sequence of the events either one names: they must agree on every
//! observation a host has of a machine — where it is, what it asked its host to
//! do, which events it discarded and whether it ended.
//!
//! Run under each of the feature sets of the product line (`scripts/gates/
//! workspace-tests.sh` does), since a test cannot choose its own features.

use sce_feature_conformance::{lifted, variant};
use sce_rust_runtime::saved_state::SavedState;
use sce_rust_runtime::{Engine, SceClock, StatePolicy};

/// Every event name either machine knows, and one neither does: a name a build
/// does not hold is discarded by both.
const EVENTS: &[&str] = &[
    "ready",
    "inbound.start",
    "connect",
    "done",
    "close",
    "unknown",
];

/// What a host observes of a machine after a run of events.
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    active: Vec<&'static str>,
    calls: Vec<String>,
    discarded: u32,
    terminal: Option<&'static str>,
}

macro_rules! observe {
    ($module:ident, $sequence:expr) => {{
        use $module as machine;
        let policy = machine::SessionPolicy::new(machine::RecordingSessionActions::default());
        let mut engine =
            Engine::<machine::SessionPolicy<machine::RecordingSessionActions>>::new(policy);
        let Ok(()) = engine.initialize();
        for name in $sequence {
            engine.raise_external_by_name(name, "");
            engine.step();
        }
        let mut active: Vec<&'static str> = engine
            .get_active_states()
            .into_iter()
            .map(machine::SessionPolicy::<machine::RecordingSessionActions>::get_state_name)
            .collect();
        active.sort_unstable();
        Observation {
            active,
            calls: engine
                .policy()
                .actions()
                .calls()
                .iter()
                .map(|call| format!("{call:?}"))
                .collect(),
            discarded: engine.discarded_external_events(),
            terminal: engine
                .terminal_state()
                .map(machine::SessionPolicy::<machine::RecordingSessionActions>::get_state_name),
        }
    }};
}

/// Every sequence of up to `length` events, the empty one included.
fn sequences(length: usize) -> Vec<Vec<&'static str>> {
    let mut all = vec![Vec::new()];
    let mut layer: Vec<Vec<&'static str>> = vec![Vec::new()];
    for _ in 0..length {
        layer = layer
            .iter()
            .flat_map(|prefix| {
                EVENTS.iter().map(move |event| {
                    let mut longer = prefix.clone();
                    longer.push(*event);
                    longer
                })
            })
            .collect();
        all.extend(layer.iter().cloned());
    }
    all
}

#[test]
fn every_run_of_events_is_observed_alike() {
    let all = sequences(4);
    for sequence in &all {
        let for_every_build = observe!(lifted, sequence.iter().copied());
        let alone = observe!(variant, sequence.iter().copied());
        assert_eq!(
            for_every_build, alone,
            "the machine for every build and this build's own part on {sequence:?}"
        );
    }
    // Held to something, not only to itself: a run that reaches a final state and
    // one that calls the host were compared.
    assert!(
        all.iter()
            .any(|sequence| observe!(variant, sequence.iter().copied())
                .terminal
                .is_some()),
        "no run ended the machine"
    );
    assert!(
        all.iter()
            .any(|sequence| !observe!(variant, sequence.iter().copied()).calls.is_empty()),
        "no run called the host"
    );
}

/// The features this build turns on, sorted: the configuration a saved state
/// of either machine records (docs/adr/0057 §7).
fn this_build() -> Vec<String> {
    [
        ("session-unicast", cfg!(feature = "session-unicast")),
        (
            "session-unicast-accept",
            cfg!(feature = "session-unicast-accept"),
        ),
    ]
    .into_iter()
    .filter(|(_, on)| *on)
    .map(|(feature, _)| feature.to_string())
    .collect()
}

/// The wall-clock moment the states below are saved and restored at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

macro_rules! saved {
    ($module:ident) => {{
        use machine::SessionPersist as _;
        use $module as machine;
        let policy = machine::SessionPolicy::new_with_session_id(
            "fixture-root",
            machine::RecordingSessionActions::default(),
        )
        .expect("an Nmtoken");
        let mut engine =
            Engine::<machine::SessionPolicy<machine::RecordingSessionActions>>::new(policy);
        engine.set_clock(SceClock::Manual(0));
        let Ok(()) = engine.initialize();
        engine.save_at(SAVED_AT_MS).expect("saves")
    }};
}

macro_rules! restored {
    ($module:ident, $saved:expr) => {{
        use machine::SessionPersist as _;
        use $module as machine;
        Engine::<machine::SessionPolicy<machine::RecordingSessionActions>>::restore_with(
            machine::SessionPolicy::new(machine::RecordingSessionActions::default()),
            $saved,
            SceClock::Manual(0),
            SAVED_AT_MS,
        )
        .map(|_| ())
        .map_err(|refused| refused.reason().to_string())
    }};
}

/// A saved state records the configuration it was taken in, and the machine
/// for every build saves exactly what this build's own machine saves: the same
/// text, its features this build's. A state of another configuration is refused
/// by the features that differ, before the shape is judged.
#[test]
fn a_saved_state_records_this_builds_configuration() {
    let lifted_state = saved!(lifted);
    let alone = saved!(variant);
    assert_eq!(lifted_state.to_json(), alone.to_json());
    assert_eq!(lifted_state.features, Some(this_build()));
    assert_eq!(restored!(lifted, &alone), Ok(()));
    assert_eq!(restored!(variant, &lifted_state), Ok(()));

    let other: Vec<String> = if this_build().is_empty() {
        vec!["session-unicast".to_string()]
    } else {
        Vec::new()
    };
    let elsewhere = SavedState {
        features: Some(other.clone()),
        ..lifted_state
    };
    let refused = restored!(lifted, &elsewhere).expect_err("another configuration is refused");
    assert!(
        refused.starts_with(&format!(
            "the saved state was taken with the features [{}] on, and this machine is built \
             with [{}]",
            other.join(", "),
            this_build().join(", ")
        )),
        "{refused}"
    );
}

/// The machine's default state is a value the document computes, and differs
/// between builds: the first child is `probe` only where `session-unicast` is on.
#[test]
fn the_default_state_is_this_builds() {
    let expected = if cfg!(feature = "session-unicast") {
        "probe"
    } else {
        "idle"
    };
    assert_eq!(
        observe!(lifted, std::iter::empty::<&str>()).active,
        [expected]
    );
    assert_eq!(
        observe!(variant, std::iter::empty::<&str>()).active,
        [expected]
    );
}
