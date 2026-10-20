// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b50ff6771257bf06531b96d0a47eeaa5c39fb71939d9bd5e537100a9105206
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
) -> Result<
    SceOwnedList<event_member::EventMemberPayload, 64>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<event_member::EventMemberPayload, 64> = SceOwnedList::new();
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
    return Ok(out);
}
