// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `StatePolicy::HAS_HOST_INVOKES`: a machine whose document hands no `<invoke>`
// to a host runs no host invocation, so the engine leaves out of it the code
// that starts, ends and expires one. These cases hold the engine to what that
// leaves, on a machine written by hand, so that what they read is the runtime:
//
//   * a machine that says it has such invocations starts one through the host's
//     invoker and takes its `done.invoke` when the host completes it (the
//     control: the same machine, the same calls);
//   * a machine that says it has none hands its invoker no start, answers that
//     nothing started, and refuses an ending, direct or posted, raising nothing.
//
// The machine: one state, `s`, whose targetless transition takes
// `done.invoke.job` and records that it did.

// Host-run invocations are the `std` profile's.
#![cfg(feature = "std")]

use std::sync::{Arc, Mutex, PoisonError};

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, DocumentTables, Engine, EventId, HistoryId, HostInvokeEnding,
    HostInvokeEvent, HostInvokeRequest, InboxInput, NoHistory, RunStore, StateId, StateKind,
    StatePolicy, StateRow, Target, TransitionId, TransitionRow, Workspace,
};

/// The type the machine's invocation names, which the host serves.
const TYPE: &str = "x-sce-host";
/// The invocation's id.
const JOB: &str = "job";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its
// `EventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    DoneJob,
}

/// `s` on `done.invoke.job`, targetless.
const S_DONE: TransitionId = TransitionId(0);

/// A machine that says whether its document hands an `<invoke>` to a host
/// (`RUNS`), and records each `done.invoke.job` it took.
struct HostPolicy<const RUNS: bool> {
    taken: usize,
}

impl<const RUNS: bool> StatePolicy for HostPolicy<RUNS> {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = ();
    type Hal = sce_rust_runtime::StdHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // No delayed send, no `<cancel>`, no `<donedata>`, and no deadline.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // One state and no history: one transition is enabled at a time.
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;

    const HAS_HOST_INVOKES: bool = RUNS;

    // One child of the `<scxml>` element with a targetless transition that has
    // content; the document's initial transition, last, enters it.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[
            // s: done.invoke.job, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // <scxml initial="s">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(1),
        default_state: StateId(St::S as u16),
        null_event: EventId(0),
        error_events: &[],
        history: None,
        parallel: None,
    };
    const STATES: &'static [St] = &[St::S];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::DoneJob];

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
    }

    fn null_event() -> Self::Event {
        Ev::Null
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            Ev::Null => "",
            Ev::DoneJob => "done.invoke.job",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "done.invoke.job" => Some(Ev::DoneJob),
            _ => None,
        }
    }

    fn get_state_name(_state: Self::State) -> &'static str {
        "s"
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "s" => Some(St::S),
            _ => None,
        }
    }

    // `_is_default_entry` is unused: this policy has no compound states.
    fn execute_entry_actions(
        &mut self,
        _state: Self::State,
        _engine: &mut Engine<Self>,
        _is_default_entry: bool,
    ) {
    }

    fn execute_exit_actions(&mut self, _state: Self::State, _engine: &mut Engine<Self>) {}

    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        _engine: &mut Engine<Self>,
    ) -> Option<TransitionId> {
        match (state, event) {
            (St::S, Ev::DoneJob) => Some(S_DONE),
            _ => None,
        }
    }

    fn execute_transition_content(&mut self, transition: TransitionId, _engine: &mut Engine<Self>) {
        if transition == S_DONE {
            self.taken += 1;
        }
    }
}

/// A running machine whose host serves `TYPE`, after it asked the host to start
/// `JOB`: the engine, whether it answered that the invocation started, and the
/// token of each start the host's invoker was handed, in order.
fn asked_to_start<const RUNS: bool>() -> (Engine<HostPolicy<RUNS>>, bool, Vec<u64>) {
    let mut engine = Engine::new(HostPolicy::<RUNS> { taken: 0 });
    let starts = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&starts);
    engine.register_invoker(TYPE, move |event| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.token);
        }
        None
    });
    engine.initialize();
    let started = engine.perform_host_invoke(HostInvokeRequest {
        processor_type: TYPE.to_string(),
        invoke_id: JOB.to_string(),
        ..HostInvokeRequest::default()
    });
    let starts = starts
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    (engine, started, starts)
}

#[test]
fn a_machine_with_host_run_invokes_starts_one_and_takes_its_completion() {
    let (mut engine, started, starts) = asked_to_start::<true>();
    assert!(
        started,
        "the engine answered that nothing ran the invocation"
    );
    assert_eq!(
        starts.len(),
        1,
        "the host's invoker was not handed one start"
    );

    assert!(
        engine.complete_host_invoke(TYPE, JOB, starts[0], ""),
        "the completion of a running start was refused"
    );
    engine.step();
    assert_eq!(
        engine.policy().taken,
        1,
        "the machine did not take the completion's done.invoke"
    );
}

#[test]
fn a_machine_with_no_host_run_invoke_starts_none_and_refuses_every_ending() {
    let (mut engine, started, starts) = asked_to_start::<false>();
    assert!(
        !started,
        "a machine with no host-run invoke answered that the invocation started"
    );
    assert!(
        starts.is_empty(),
        "a machine with no host-run invoke handed the host a start"
    );

    // Token 0 is the one the first start would have been given.
    assert!(
        !engine.complete_host_invoke(TYPE, JOB, 0, ""),
        "a completion was taken for an invocation that never started"
    );
    assert!(
        !engine.fail_host_invoke(TYPE, JOB, 0, "", None),
        "a failure was taken for an invocation that never started"
    );
    engine
        .inbox()
        .post(InboxInput::host_invoke_ending(
            None,
            HostInvokeEnding {
                processor_type: TYPE,
                invoke_id: JOB,
                token: 0,
                done: true,
                data: "",
                origin: None,
            },
        ))
        .expect("an engine's own inbox takes every post while the engine lives");
    engine.step();
    assert_eq!(
        engine.policy().taken,
        0,
        "an ending of an invocation that never started raised its event"
    );
}
