// SCE-GENERATED — DO NOT EDIT
// source-hash: 38a2b8ab9943c3f76e7281d40f9dae3ea2effb54aa8b7ab5baa1ac0eaba97f5b
#![doc = "SCE-MAP: event_conflict.scxml:10 :: _forge_body"]
// SCE-MAP: event_conflict.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::mv_value;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_conflict(values: &[mv_value::MvValuePayload], id: u64) -> bool {
    let mut twice: bool = false;
    for a in values.iter() {
        if a.id == id && (a.field == 9 || a.field >= 15 && a.field <= 58) {
            for b in values.iter() {
                if b.id == id
                    && b.field == a.field
                    && (b.wall_time != a.wall_time
                        || b.counter != a.counter
                        || b.node_id != a.node_id)
                {
                    twice = true;
                }
            }
        }
    }
    return twice;
}
