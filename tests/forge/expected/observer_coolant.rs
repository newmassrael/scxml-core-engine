// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::observer::{EventDomain, EventQueue, ThresholdState};

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting Event<> type cannot be composed with
// other observers. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.
pub struct ForgeDomain;

// SCXML event names flow into enum variants verbatim (W3C SCXML 3.12).
// `EMIT_WARNING`, `coolant.high`, etc. cannot be normalised to UpperCamel
// without breaking SCE_FORGE.md §4.11 cross-file event composition.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDomainTag {
    EMIT_WARNING,
    CLEAR_WARNING,
    EMERGENCY_SHUTDOWN,
}

impl EventDomain for ForgeDomain {
    type Tag = ForgeDomainTag;
}

pub struct ObserverCoolant {
    warning: ThresholdState,
    critical: ThresholdState,
}

impl ObserverCoolant {
    pub fn new() -> Self {
        Self {
            warning: ThresholdState::new(),
            critical: ThresholdState::new(),
        }
    }

    pub fn update(&mut self, coolant_temp: f64) -> EventQueue<ForgeDomain> {
        let mut events: EventQueue<ForgeDomain> = EventQueue::new();
        if self.warning.enter_if(coolant_temp > 110.0) {
            events.push(ForgeDomainTag::EMIT_WARNING);
        }
        else if self.warning.leave_if(coolant_temp < 100.0) {
            events.push(ForgeDomainTag::CLEAR_WARNING);
        }
        if self.critical.enter_if(coolant_temp > 120.0) {
            events.push(ForgeDomainTag::EMERGENCY_SHUTDOWN);
        }
        else {
            self.critical.leave_if(coolant_temp < 105.0);
        }
        events
    }
}

impl Default for ObserverCoolant {
    fn default() -> Self {
        Self::new()
    }
}
