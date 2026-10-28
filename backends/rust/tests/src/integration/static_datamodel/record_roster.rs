// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
#![doc = "SCE-MAP: record_roster.scxml:14 :: _forge_body"]
// SCE-MAP: record_roster.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::record_member;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn record_roster(
    roster: &[record_member::RecordMemberPayload],
    node: u64,
    role: u8,
) -> Result<
    SceOwnedList<record_member::RecordMemberPayload, 64>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    let mut out: SceOwnedList<record_member::RecordMemberPayload, 64> = SceOwnedList::new();
    for &m in roster.iter() {
        if m.node != node {
            out.push(m)?;
        }
    }
    if role != 0 {
        let made = record_member::RecordMemberPayload {
            node: node,
            role: role,
        };
        out.push(made)?;
    }
    return Ok(out);
}
