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
    CheckVoltage = 0,
    CheckTemp = 1,
    Success = 2,
    FailVoltage = 3,
    FailOvertemp = 4,
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
pub struct ProcedureStartupCheck {
    voltage: f32,
    temperature: f32,
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
impl ProcedureStartupCheck {
    pub fn new() -> Self {
        Self {
            voltage: 0.0,
            temperature: 0.0,
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

    pub fn set_voltage(&mut self, value: f32) {
        self.voltage = value;
    }

    pub fn set_temperature(&mut self, value: f32) {
        self.temperature = value;
    }

    pub fn run_to_completion(&mut self) -> ProcedureRunResult {
        run_procedure(self)
    }
}

impl ProcedurePolicy for ProcedureStartupCheck {
    type State = State;
    type Event = Event;

    fn none_event() -> Event { Event::None }
    fn initial_state(&self) -> State { State::CheckVoltage }
    fn is_final(state: State) -> bool {
        matches!(state, State::Success | State::FailVoltage | State::FailOvertemp)
    }
    fn final_state_name(state: State) -> &'static str {
        match state {
            State::Success => "success",
            State::FailVoltage => "fail_voltage",
            State::FailOvertemp => "fail_overtemp",
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
            State::CheckVoltage => {
                if event == Event::None {
                    if self.voltage >= 11.5 && self.voltage <= 14.5 {
                        return Some((State::CheckTemp, 0, false));
                    }
                }
                if event == Event::None {
                    return Some((State::FailVoltage, 1, false));
                }
            }
            State::CheckTemp => {
                if event == Event::None {
                    if self.temperature < 80.0 {
                        return Some((State::Success, 0, false));
                    }
                }
                if event == Event::None {
                    return Some((State::FailOvertemp, 1, false));
                }
            }
            _ => {}
        }
        None
    }

    fn execute_transition_actions(&mut self, source: State, tr_index: usize) -> Option<Event> {
        // Returns None for normal flow; Some(Event) when an assign-time
        // check (RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B4 bytes
        // cap violation) raises an internal event that the shared
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
    voltage: f32,
    temperature: f32,
) -> ProcedureRunResult {
    let mut sm = ProcedureStartupCheck::new();
    sm.set_service_handler(handler);
    sm.set_voltage(voltage);
    sm.set_temperature(temperature);
    sm.run_to_completion()
}
