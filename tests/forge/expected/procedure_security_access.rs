#![doc = "SCE-MAP: procedure_security_access:1"]
// SCE-MAP: procedure_security_access:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedurePolicy trait.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event::None transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   computeKey(seed)

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
    SendTesterPresent = 0,
    RequestSeed = 1,
    SendKey = 2,
    Retry = 3,
    Done = 4,
    Error = 5,
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
pub struct ProcedureSecurityAccess {
    ecu_addr: u32,
    seed: Vec<u8>,
    max_retries: i32,
    retry_count: i32,
    service_handler: Option<Box<dyn Fn(&ProcedureServiceRequest) -> ProcedureServiceResponse>>,
    compute_key: Box<dyn Fn(&[u8]) -> Vec<u8>>,
    done_data: BTreeMap<String, String>,
    pending_event_data: String,
}

// pub API: procedures expose State/Event enums and the policy struct as
// downstream library types (SCE_FORGE.md §8 procedure). Generated code
// keeps them pub so embedders can inspect transitions; the conformance
// harness only ever observes the run_procedure() entrypoint, leaving the
// other items dead from the test binary's perspective.
#[allow(dead_code)]
impl ProcedureSecurityAccess {
    pub fn new() -> Self {
        Self {
            ecu_addr: 0,
            seed: Vec::new(),
            max_retries: 3,
            retry_count: 0,
            service_handler: None,
            compute_key: Box::new(|_arg0| panic!("helper 'computeKey' not set — call set_compute_key() before run_to_completion()")),
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

    pub fn set_compute_key(&mut self, f: impl Fn(&[u8]) -> Vec<u8> + 'static) {
        self.compute_key = Box::new(f);
    }

    pub fn set_ecu_addr(&mut self, value: u32) {
        self.ecu_addr = value;
    }

    pub fn run_to_completion(&mut self) -> ProcedureRunResult {
        run_procedure(self)
    }
}

impl ProcedurePolicy for ProcedureSecurityAccess {
    type State = State;
    type Event = Event;

    fn none_event() -> Event { Event::None }
    fn initial_state(&self) -> State { State::SendTesterPresent }
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
            State::SendTesterPresent => {
                if let Some(ref handler) = self.service_handler {
                    let req = ProcedureServiceRequest {
                        service: "TesterPresent".to_string(),
                        subfunc: None,
                        addr: Some((self.ecu_addr).to_string()),
                        payload: None,
                    };
                    let resp = handler(&req);
                    let event = if resp.success { Event::Ok } else { Event::Fail };
                    return (event, resp.data);
                }
            }
            State::RequestSeed => {
                if let Some(ref handler) = self.service_handler {
                    let req = ProcedureServiceRequest {
                        service: "SecurityAccess".to_string(),
                        subfunc: Some("0x01".to_string()),
                        addr: None,
                        payload: None,
                    };
                    let resp = handler(&req);
                    let event = if resp.success { Event::Ok } else { Event::Fail };
                    return (event, resp.data);
                }
            }
            State::SendKey => {
                if let Some(ref handler) = self.service_handler {
                    let req = ProcedureServiceRequest {
                        service: "SecurityAccess".to_string(),
                        subfunc: Some("0x02".to_string()),
                        addr: None,
                        payload: Some((self.compute_key)(&self.seed)),
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
            State::SendTesterPresent => {
                if event == Event::Ok {
                    return Some((State::RequestSeed, 0, false));
                }
                if event == Event::Fail {
                    return Some((State::Error, 1, false));
                }
            }
            State::RequestSeed => {
                if event == Event::Ok {
                    return Some((State::SendKey, 0, true));
                }
                if event == Event::Fail {
                    return Some((State::Retry, 1, false));
                }
            }
            State::SendKey => {
                if event == Event::Ok {
                    return Some((State::Done, 0, false));
                }
                if event == Event::Fail {
                    return Some((State::Retry, 1, false));
                }
            }
            State::Retry => {
                if event == Event::None {
                    if self.retry_count < self.max_retries {
                        return Some((State::RequestSeed, 0, true));
                    }
                }
                if event == Event::None {
                    if self.retry_count >= self.max_retries {
                        return Some((State::Error, 1, false));
                    }
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
        if source == State::RequestSeed {
            if tr_index == 0 {
                {
                    let _scope_tmp = self.pending_event_data.as_bytes().to_vec();
                    if _scope_tmp.len() > 64 {
                        return Some(Event::ErrorExecution);
                    }
                    self.seed = _scope_tmp;
                }
            }
        }
        if source == State::Retry {
            if tr_index == 0 {
                self.retry_count = self.retry_count + 1;
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
    compute_key: impl Fn(&[u8]) -> Vec<u8> + 'static,
    ecu_addr: u32,
) -> ProcedureRunResult {
    let mut sm = ProcedureSecurityAccess::new();
    sm.set_service_handler(handler);
    sm.set_compute_key(compute_key);
    sm.set_ecu_addr(ecu_addr);
    sm.run_to_completion()
}
