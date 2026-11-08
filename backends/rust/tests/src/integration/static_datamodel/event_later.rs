// SCE-GENERATED — DO NOT EDIT
// source-hash: a0d514a5bb9842543c3a83f1c232fcee900a165f2a3ed19fb29e42d249c8aa22
#![doc = "SCE-MAP: event_later.scxml:9 :: _forge_body"]
// SCE-MAP: event_later.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn event_later(
    wall: i64,
    counter: u32,
    node: u64,
    best_wall: i64,
    best_counter: u32,
    best_node: u64,
) -> bool {
    return wall > best_wall
        || wall == best_wall
            && (counter > best_counter || counter == best_counter && node > best_node);
}
