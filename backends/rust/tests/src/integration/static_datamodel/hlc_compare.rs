// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
#![doc = "SCE-MAP: hlc_compare.scxml:18 :: _forge_body"]
// SCE-MAP: hlc_compare.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::hlc_timestamp;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn hlc_compare(
    a: hlc_timestamp::HlcTimestampPayload,
    b: hlc_timestamp::HlcTimestampPayload,
) -> i32 {
    if a.wall_time != b.wall_time {
        return if a.wall_time < b.wall_time { -1 } else { 1 };
    }
    if a.counter != b.counter {
        return if a.counter < b.counter { -1 } else { 1 };
    }
    if a.node_id != b.node_id {
        return if a.node_id < b.node_id { -1 } else { 1 };
    }
    return 0;
}
