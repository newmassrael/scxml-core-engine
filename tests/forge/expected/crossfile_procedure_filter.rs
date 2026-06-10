#![doc = "SCE-MAP: crossfile_procedure_filter:10"]
// SCE-MAP: crossfile_procedure_filter:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedurePolicy trait.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::None transitions.

use super::filter_low_pass::FilterLowPass;

use std::collections::BTreeMap;
use sce_forge_runtime::procedure::{
    ProcedurePolicy, ProcedureRunResult, ProcedureServiceRequest, ProcedureServiceResponse,
    run_procedure,
};

// ── State and Event enums ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
pub enum State {
    Sample = 0,
    Done = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
pub enum Event {
    None = 0,
    ErrorExecution = 1,
    Fail = 2,
    Ok = 3,
}

// ── Generated procedure policy ──────────────────────────────────

// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
pub struct CrossfileProcedureFilter {
    raw_sample: f64,
    smoothed: f64,
    // Imported kinds (cross-file composition)
    pub smoother: FilterLowPass,
    service_handler: Option<Box<dyn Fn(&ProcedureServiceRequest) -> ProcedureServiceResponse>>,
    done_data: BTreeMap<String, String>,
    pending_event_data: String,
}

// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
impl CrossfileProcedureFilter {
    pub fn new() -> Self {
        Self {
            raw_sample: 0.0,
            smoothed: 0.0,
            smoother: FilterLowPass::new(),
            service_handler: None,
            done_data: BTreeMap::new(),
            pending_event_data: String::new(),
        }
    }

    pub fn set_service_handler(
        &mut self,
        handler: impl Fn(&ProcedureServiceRequest) -> ProcedureServiceResponse + 'static,
    ) {
        self.service_handler = Some(Box::new(handler));
    }

    pub fn set_raw_sample(&mut self, value: f64) {
        self.raw_sample = value;
    }

    pub fn run_to_completion(&mut self) -> ProcedureRunResult {
        run_procedure(self)
    }
}

impl ProcedurePolicy for CrossfileProcedureFilter {
    type State = State;
    type Event = Event;

    fn none_event() -> Event { Event::None }
    fn initial_state(&self) -> State { State::Sample }
    fn is_final(state: State) -> bool {
        matches!(state, State::Done)
    }
    fn final_state_name(state: State) -> &'static str {
        match state {
            State::Done => "done",
            _ => "",
        }
    }

    fn set_pending_event_data(&mut self, data: String) {
        self.pending_event_data = data;
    }

    fn done_data(&self) -> &BTreeMap<String, String> {
        &self.done_data
    }

    fn execute_entry_actions(&mut self, state: State) -> (Event, String) {
        match state {
            State::Done => {
                self.done_data.insert("result".to_string(), "success".to_string());
            }
            _ => {}
        }
        (Event::None, String::new())
    }

    fn process_transition(&self, state: State, event: Event) -> Option<(State, usize, bool)> {
        match state {
            State::Sample => {
                if event == Event::None {
                    return Some((State::Done, 0, true));
                }
            }
            _ => {}
        }
        None
    }

    fn execute_transition_actions(&mut self, source: State, tr_index: usize) -> Option<Event> {
        // Returns None for normal flow; Some(Event) when an assign-time
        // bytes-cap check raises an internal event that the shared
        // run_procedure() loop re-pumps through process_transition.
        let _ = (source, tr_index);  // pacify unused-warning for empty bodies
        if source == State::Sample {
            if tr_index == 0 {
                self.smoothed = self.smoother.update(self.raw_sample);
            }
        }
        None
    }
}

// ── Convenience wrapper function ────────────────────────────────

// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
pub fn execute(
    handler: impl Fn(&ProcedureServiceRequest) -> ProcedureServiceResponse + 'static,
    raw_sample: f64,
) -> ProcedureRunResult {
    let mut sm = CrossfileProcedureFilter::new();
    sm.set_service_handler(handler);
    sm.set_raw_sample(raw_sample);
    sm.run_to_completion()
}
