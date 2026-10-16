// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
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
