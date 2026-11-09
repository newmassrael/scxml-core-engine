// SCE-GENERATED — DO NOT EDIT
// source-hash: a7faa6958690c5f997097ffa5111459d925f4ceff9942f639c58478c1f89cf17
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
