// SCE-GENERATED — DO NOT EDIT
// source-hash: 81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c
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
    out: &mut SceOwnedList<record_member::RecordMemberPayload, 64>,
) -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {
    out.clear();
    out.reserve(64);
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
    return Ok(());
}
