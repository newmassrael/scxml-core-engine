#![doc = "SCE-MAP: crossfile_observer_condition:4 :: _forge_body"]
// SCE-MAP: crossfile_observer_condition:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use super::condition_threshold;
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
    RAISE_ALARM,
    CLEAR_ALARM,
}

impl EventDomain for ForgeDomain {
    type Tag = ForgeDomainTag;
}

pub struct CrossfileObserverCondition {
    alarm: ThresholdState,
}

impl CrossfileObserverCondition {
    pub fn new() -> Self {
        Self {
            alarm: ThresholdState::new(),
        }
    }

    pub fn update(&mut self, coolant_temp: f64, oil_temp: f64) -> EventQueue<ForgeDomain> {
        let mut sce_events: EventQueue<ForgeDomain> = EventQueue::new();
        if self.alarm.enter_if(condition_threshold::condition_threshold(coolant_temp, oil_temp, 110.0)) {
            sce_events.push(ForgeDomainTag::RAISE_ALARM);
        }
        else if self.alarm.leave_if(condition_threshold::condition_threshold(coolant_temp, oil_temp, 120.0) == false) {
            sce_events.push(ForgeDomainTag::CLEAR_ALARM);
        }
        sce_events
    }
}

impl Default for CrossfileObserverCondition {
    fn default() -> Self {
        Self::new()
    }
}
