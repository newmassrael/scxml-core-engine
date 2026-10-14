// SCE-GENERATED — DO NOT EDIT
// source-hash: 0a170bbc1ca3025d9cc4009c0838fff8e752b3322334de465b99571c709f4da7
#![doc = "SCE-MAP: hlc_receive.scxml:19 :: _forge_body"]
// SCE-MAP: hlc_receive.scxml:19 :: _forge_body

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
pub fn hlc_receive(
    prev: hlc_timestamp::HlcTimestampPayload,
    msg: hlc_timestamp::HlcTimestampPayload,
    now: i64,
    node: u64,
) -> Result<hlc_timestamp::HlcTimestampPayload, sce_forge_runtime::algorithm::AlgorithmError> {
    let seen: i64 = if prev.wall_time > msg.wall_time {
        prev.wall_time
    } else {
        msg.wall_time
    };
    let mut next = hlc_timestamp::HlcTimestampPayload {
        wall_time: if now > seen { now } else { seen },
        counter: 0,
        node_id: node,
    };
    if next.wall_time == prev.wall_time && next.wall_time == msg.wall_time {
        next.counter = sce_forge_runtime::algorithm::add::<u32>(
            if prev.counter > msg.counter {
                prev.counter
            } else {
                msg.counter
            },
            1,
        )?;
    } else {
        if next.wall_time == prev.wall_time {
            next.counter = sce_forge_runtime::algorithm::add::<u32>(prev.counter, 1)?;
        } else {
            if next.wall_time == msg.wall_time {
                next.counter = sce_forge_runtime::algorithm::add::<u32>(msg.counter, 1)?;
            }
        }
    }
    return Ok(next);
}
