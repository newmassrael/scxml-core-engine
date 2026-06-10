#![doc = "SCE-MAP: procedure_diamond:2"]
// SCE-MAP: procedure_diamond:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedurePolicy trait.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::None transitions.

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
    Classify = 0,
    HighPath = 1,
    MidPath = 2,
    LowPath = 3,
    Accept = 4,
    Reject = 5,
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
pub struct ProcedureDiamond {
    sensor_value: u16,
    mode: String,
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
impl ProcedureDiamond {
    pub fn new() -> Self {
        Self {
            sensor_value: 0,
            mode: String::new(),
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

    pub fn set_sensor_value(&mut self, value: u16) {
        self.sensor_value = value;
    }

    pub fn set_mode(&mut self, value: &str) {
        self.mode = value.to_string();
    }

    pub fn run_to_completion(&mut self) -> ProcedureRunResult {
        run_procedure(self)
    }
}

impl ProcedurePolicy for ProcedureDiamond {
    type State = State;
    type Event = Event;

    fn none_event() -> Event { Event::None }
    fn initial_state(&self) -> State { State::Classify }
    fn is_final(state: State) -> bool {
        matches!(state, State::Accept | State::Reject)
    }
    fn final_state_name(state: State) -> &'static str {
        match state {
            State::Accept => "accept",
            State::Reject => "reject",
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
            _ => {}
        }
        (Event::None, String::new())
    }

    fn process_transition(&self, state: State, event: Event) -> Option<(State, usize, bool)> {
        match state {
            State::Classify => {
                if event == Event::None {
                    if self.sensor_value > 1000 {
                        return Some((State::HighPath, 0, false));
                    }
                }
                if event == Event::None {
                    if self.sensor_value > 500 {
                        return Some((State::MidPath, 1, false));
                    }
                }
                if event == Event::None {
                    return Some((State::LowPath, 2, false));
                }
            }
            State::HighPath => {
                if event == Event::None {
                    if self.mode == "strict" {
                        return Some((State::Reject, 0, false));
                    }
                }
                if event == Event::None {
                    return Some((State::Accept, 1, false));
                }
            }
            State::MidPath => {
                if event == Event::None {
                    return Some((State::Accept, 0, false));
                }
            }
            State::LowPath => {
                if event == Event::None {
                    return Some((State::Accept, 0, false));
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
    sensor_value: u16,
    mode: &str,
) -> ProcedureRunResult {
    let mut sm = ProcedureDiamond::new();
    sm.set_service_handler(handler);
    sm.set_sensor_value(sensor_value);
    sm.set_mode(mode);
    sm.run_to_completion()
}
