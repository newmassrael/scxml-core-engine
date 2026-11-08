// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
#![doc = "SCE-MAP: algorithm_item_pick_first.scxml:9 :: _forge_body"]
// SCE-MAP: algorithm_item_pick_first.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::schema_item_pick;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn item_pick_first(
    items: &[schema_item_pick::SchemaItemPickPayload],
    least: i32,
) -> Option<schema_item_pick::SchemaItemPickPayload> {
    let mut found: Option<schema_item_pick::SchemaItemPickPayload> = None;
    for it in items.iter() {
        if found.is_none() && it.count >= least {
            found = Some(it.clone());
        }
    }
    return found;
}
