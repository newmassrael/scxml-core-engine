// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A document of the static class printed as its own step function runs as the
// same document printed for the shared interpreter (docs/adr/0129).
//
// Each document is generated twice by tools/codegen/trees/rust-integration.toml:
// into `step_function_interpreted` for the shared interpreter, and into
// `step_function_printed` with `--step-function`. Seeded random sequences of
// external events drive both, and `step_function_differential` holds every
// observation of the printed form to the interpreted one: the configuration,
// the atomic state, whether the machine runs and where it ended, each
// transition taken, each call of the host's actions, and the counts of
// discarded, unseen and unanswered error events.
//
// The documents: the two consumer-shaped fixtures of
// tests/integration/step_function (a unicast session machine and a reassembly
// slot), and the class documents of integration_resources (the flat and the
// nested lifecycle).

use sce_rust_runtime::{Engine, StatePolicy};
use sce_rust_tests::integration::{
    step_function_interpreted as interpreted, step_function_printed as printed,
};
use sce_rust_tests::step_function_differential::{
    drive, events, expect_same_journals, journal, note, transitions_taken, Journal,
};

/// Seeds per document, and events per seed. A run that ends in a top-level
/// final takes no event after it, so many short runs reach more of a document
/// than a few long ones.
const SEEDS: u64 = 256;
const EVENTS: usize = 120;

/// The host's actions, each call written to the journal by its name.
struct Recorder(Journal);

/// Implement a machine's actions trait for [`Recorder`]: every action is a
/// notice with no argument, recorded by its name.
macro_rules! recorded {
    ($actions:path { $($action:ident),* $(,)? }) => {
        impl $actions for Recorder {
            $(
                fn $action(&mut self) {
                    note(&self.0, format!("act {}", stringify!($action)));
                }
            )*
        }
    };
}

recorded!(interpreted::UnicastSessionActions {
    on_closed,
    open_requested,
    reset_counters,
    on_opening,
    left_opening,
    on_resolving,
    on_handshaking,
    on_established,
    left_established,
    note_keepalive,
    on_idle,
    on_sending,
    acknowledged,
    drop_data,
    on_failed,
    report_failure,
});
recorded!(printed::UnicastSessionActions {
    on_closed,
    open_requested,
    reset_counters,
    on_opening,
    left_opening,
    on_resolving,
    on_handshaking,
    on_established,
    left_established,
    note_keepalive,
    on_idle,
    on_sending,
    acknowledged,
    drop_data,
    on_failed,
    report_failure,
});
recorded!(interpreted::FragmentSlotActions {
    clear_slot,
    start_message,
    deliver_single,
    append_fragment,
    restart_message,
    deliver_message,
    report_expired,
});
recorded!(printed::FragmentSlotActions {
    clear_slot,
    start_message,
    deliver_single,
    append_fragment,
    restart_message,
    deliver_message,
    report_expired,
});

/// Drive both forms of one document with every seed, hold each pair of
/// journals to each other, and answer the transitions the interpreted form
/// took over all of them. `interpreted` and `printed` build a machine of each
/// form that writes its host's calls to the journal they are handed.
fn both_forms_agree<I: StatePolicy, P: StatePolicy>(
    document: &str,
    interpreted: impl Fn(&Journal) -> Engine<I>,
    printed: impl Fn(&Journal) -> Engine<P>,
) -> Vec<String> {
    assert!(
        I::STEP_FUNCTION.is_none(),
        "{document}: the interpreted form is printed for the shared interpreter"
    );
    assert!(
        P::STEP_FUNCTION.is_some(),
        "{document}: the printed form is its own step function; without \
         --step-function the two forms would be one and agree by construction"
    );
    let reached = journal();
    for seed in 0..SEEDS {
        let sequence = events::<I>(seed, EVENTS);
        let expected = journal();
        drive(interpreted(&expected), &expected, &sequence);
        let actual = journal();
        drive(printed(&actual), &actual, &sequence);
        expect_same_journals(&expected, &actual, &format!("{document}, seed {seed}"));
        for line in transitions_taken(&expected) {
            note(&reached, line);
        }
    }
    transitions_taken(&reached)
}

/// The drives reached at least half of a document's written transitions, so a
/// drive that never reached the step function's arms cannot pass as agreement.
fn expect_reached(document: &str, reached: &[String], written: usize) {
    assert!(
        reached.len() * 2 >= written,
        "{document}: the drives took {} of {written} transitions: {reached:?}",
        reached.len()
    );
}

#[test]
fn a_unicast_session_runs_alike_in_both_forms() {
    let reached = both_forms_agree(
        "unicast_session",
        |journal| {
            Engine::new(interpreted::UnicastSessionPolicy::new(Recorder(
                journal.clone(),
            )))
        },
        |journal| {
            Engine::new(printed::UnicastSessionPolicy::new(Recorder(
                journal.clone(),
            )))
        },
    );
    expect_reached("unicast_session", &reached, 30);
}

#[test]
fn a_fragment_slot_runs_alike_in_both_forms() {
    let reached = both_forms_agree(
        "fragment_slot",
        |journal| {
            Engine::new(interpreted::FragmentSlotPolicy::new(Recorder(
                journal.clone(),
            )))
        },
        |journal| Engine::new(printed::FragmentSlotPolicy::new(Recorder(journal.clone()))),
    );
    expect_reached("fragment_slot", &reached, 12);
}

#[test]
fn a_flat_lifecycle_runs_alike_in_both_forms() {
    let reached = both_forms_agree(
        "flat_lifecycle",
        |_| Engine::new(interpreted::FlatLifecyclePolicy::new()),
        |_| Engine::new(printed::FlatLifecyclePolicy::new()),
    );
    expect_reached("flat_lifecycle", &reached, 7);
}

#[test]
fn a_nested_lifecycle_runs_alike_in_both_forms() {
    let reached = both_forms_agree(
        "nested_lifecycle",
        |_| Engine::new(interpreted::NestedLifecyclePolicy::new()),
        |_| Engine::new(printed::NestedLifecyclePolicy::new()),
    );
    expect_reached("nested_lifecycle", &reached, 12);
}
