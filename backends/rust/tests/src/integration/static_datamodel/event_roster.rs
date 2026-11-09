// SCE-GENERATED — DO NOT EDIT
// source-hash: d05ce240c5f3b48b3c83b6bee1ba58feef40d57c310e9a29ef5279604bbaaa42
#![doc = "SCE-MAP: event_roster.scxml:10 :: _forge_body"]
// SCE-MAP: event_roster.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::event_member;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_roster(
    roster: &[event_member::EventMemberPayload],
    node: u64,
    role: u8,
    out: &mut SceOwnedList<event_member::EventMemberPayload, 64>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(64);
    for &m in roster.iter() {
        if m.node != node {
            out.push(m)?;
        }
    }
    if role != 0 {
        let made = event_member::EventMemberPayload {
            node: node,
            role: role,
        };
        out.push(made)?;
    }
    return Ok(());
}
