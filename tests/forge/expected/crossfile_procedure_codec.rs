#![doc = "SCE-MAP: crossfile_procedure_codec:3"]
// SCE-MAP: crossfile_procedure_codec:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedurePolicy trait.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::None transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   frame.encode()

use super::codec_simple_frame::CodecSimpleFrame;

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
    SendRequest = 0,
    Decode = 1,
    Done = 2,
    Error = 3,
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
pub struct CrossfileProcedureCodec {
    ecu_addr: u32,
    response: Vec<u8>,
    // Imported kinds (cross-file composition)
    pub frame: CodecSimpleFrame,
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
impl CrossfileProcedureCodec {
    pub fn new() -> Self {
        Self {
            ecu_addr: 0,
            response: Vec::new(),
            frame: CodecSimpleFrame::new(),
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

    pub fn set_ecu_addr(&mut self, value: u32) {
        self.ecu_addr = value;
    }

    pub fn run_to_completion(&mut self) -> ProcedureRunResult {
        run_procedure(self)
    }
}

impl ProcedurePolicy for CrossfileProcedureCodec {
    type State = State;
    type Event = Event;

    fn none_event() -> Event { Event::None }
    fn initial_state(&self) -> State { State::SendRequest }
    fn is_final(state: State) -> bool {
        matches!(state, State::Done | State::Error)
    }
    fn final_state_name(state: State) -> &'static str {
        match state {
            State::Done => "done",
            State::Error => "error",
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
            State::SendRequest => {
                if let Some(ref handler) = self.service_handler {
                    let req = ProcedureServiceRequest {
                        service: "Diag".to_string(),
                        subfunc: None,
                        addr: Some((self.ecu_addr).to_string()),
                        payload: Some(self.frame.encode_to_vec()),
                    };
                    let resp = handler(&req);
                    let event = if resp.success { Event::Ok } else { Event::Fail };
                    return (event, resp.data);
                }
            }
            State::Done => {
                self.done_data.insert("result".to_string(), "success".to_string());
            }
            State::Error => {
                self.done_data.insert("result".to_string(), "failure".to_string());
            }
            _ => {}
        }
        (Event::None, String::new())
    }

    fn process_transition(&self, state: State, event: Event) -> Option<(State, usize, bool)> {
        match state {
            State::SendRequest => {
                if event == Event::Ok {
                    return Some((State::Decode, 0, true));
                }
                if event == Event::Fail {
                    return Some((State::Error, 1, false));
                }
            }
            State::Decode => {
                if event == Event::None {
                    return Some((State::Done, 0, false));
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
        if source == State::SendRequest {
            if tr_index == 0 {
                {
                    let _scope_tmp = self.pending_event_data.as_bytes().to_vec();
                    if _scope_tmp.len() > 256 {
                        return Some(Event::ErrorExecution);
                    }
                    self.response = _scope_tmp;
                }
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
    ecu_addr: u32,
) -> ProcedureRunResult {
    let mut sm = CrossfileProcedureCodec::new();
    sm.set_service_handler(handler);
    sm.set_ecu_addr(ecu_addr);
    sm.run_to_completion()
}
