// SCE-GENERATED — DO NOT EDIT
// source-hash: 9313ff05336b94eeb54552b43ab70c62d4e55585e0cedc6ece4c89ceb09077d5
#![doc = "SCE-MAP: register_values.scxml:21 :: _forge_body"]
// SCE-MAP: register_values.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::mv_value;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn register_values(
    values: &[mv_value::MvValuePayload],
    id: u64,
    field: u64,
) -> Result<SceOwnedList<mv_value::MvValuePayload, 512>, sce_forge_runtime::algorithm::AlgorithmError>
{
    let mut out: SceOwnedList<mv_value::MvValuePayload, 512> = SceOwnedList::new();
    let mut started: bool = false;
    let mut last_wall: i64 = 0;
    let mut last_counter: u32 = 0;
    let mut last_node: u64 = 0;
    let mut more: bool = true;
    while more {
        let mut chosen: bool = false;
        let mut best_wall: i64 = 0;
        let mut best_counter: u32 = 0;
        let mut best_node: u64 = 0;
        for v in values.iter() {
            if v.id == id
                && v.field == field
                && (!started
                    || v.wall_time > last_wall
                    || v.wall_time == last_wall
                        && (v.counter > last_counter
                            || v.counter == last_counter && v.node_id > last_node))
                && (!chosen
                    || v.wall_time < best_wall
                    || v.wall_time == best_wall
                        && (v.counter < best_counter
                            || v.counter == best_counter && v.node_id < best_node))
            {
                chosen = true;
                best_wall = v.wall_time;
                best_counter = v.counter;
                best_node = v.node_id;
            }
        }
        if chosen {
            for w in values.iter() {
                if w.id == id
                    && w.field == field
                    && w.wall_time == best_wall
                    && w.counter == best_counter
                    && w.node_id == best_node
                {
                    out.push(w.clone())?;
                }
            }
            started = true;
            last_wall = best_wall;
            last_counter = best_counter;
            last_node = best_node;
        } else {
            more = false;
        }
    }
    return Ok(out);
}
