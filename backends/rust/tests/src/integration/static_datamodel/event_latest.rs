// SCE-GENERATED — DO NOT EDIT
// source-hash: 22249655389e33dabe1467cad0f6465e75f1c83f40cd0207380a1fc15324f922
#![doc = "SCE-MAP: event_latest.scxml:10 :: _forge_body"]
// SCE-MAP: event_latest.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::event_later;
use super::mv_value;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_latest(values: &[mv_value::MvValuePayload], id: u64, field: u64, none: i64) -> i64 {
    let mut found: bool = false;
    let mut best_wall: i64 = 0;
    let mut best_counter: u32 = 0;
    let mut best_node: u64 = 0;
    let mut best: i64 = none;
    for v in values.iter() {
        if v.id == id
            && v.field == field
            && (!found
                || event_later::event_later(
                    v.wall_time,
                    v.counter,
                    v.node_id,
                    best_wall,
                    best_counter,
                    best_node,
                ))
        {
            found = true;
            best_wall = v.wall_time;
            best_counter = v.counter;
            best_node = v.node_id;
            best = v.value;
        }
    }
    return best;
}
