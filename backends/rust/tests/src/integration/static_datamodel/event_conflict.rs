// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
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
