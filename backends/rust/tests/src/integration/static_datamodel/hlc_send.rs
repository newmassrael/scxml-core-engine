// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
#![doc = "SCE-MAP: hlc_send.scxml:17 :: _forge_body"]
// SCE-MAP: hlc_send.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::hlc_timestamp;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn hlc_send(
    prev: hlc_timestamp::HlcTimestampPayload,
    now: i64,
    node: u64,
) -> Result<hlc_timestamp::HlcTimestampPayload, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut next = hlc_timestamp::HlcTimestampPayload {
        wall_time: if now > prev.wall_time {
            now
        } else {
            prev.wall_time
        },
        counter: 0,
        node_id: node,
    };
    if next.wall_time == prev.wall_time {
        next.counter = sce_forge_runtime::algorithm::add::<u32>(prev.counter, 1)?;
    }
    return Ok(next);
}
