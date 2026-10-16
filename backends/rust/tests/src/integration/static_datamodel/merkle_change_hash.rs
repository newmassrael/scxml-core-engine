// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
#![doc = "SCE-MAP: merkle_change_hash.scxml:40 :: _forge_body"]
// SCE-MAP: merkle_change_hash.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::orset_entry;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn merkle_change_hash(
    change: orset_entry::OrsetEntryPayload,
    kind: u8,
) -> Result<u32, sce_forge_runtime::algorithm::AlgorithmError> {
    if !(change.wall_time >= 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let element: u64 = change.element;
    let wall: u64 = sce_forge_runtime::algorithm::narrow::<u64, i64>(change.wall_time)?;
    let counter: u64 = change.counter as u64;
    let node: u64 = change.node_id;
    let mut h: u64 = 0;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::rem::<u64>(element, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::div::<u64>(element, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::rem::<u64>(wall, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::div::<u64>(wall, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            counter,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::rem::<u64>(node, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
            sce_forge_runtime::algorithm::div::<u64>(node, 4294967296)?,
        )?,
        4294967296,
    )?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::add::<u64>(
            sce_forge_runtime::algorithm::add::<u64>(
                sce_forge_runtime::algorithm::mul::<u64>(h, 1000003)?,
                kind as u64,
            )?,
            1,
        )?,
        4294967296,
    )?;
    h = h ^ sce_forge_runtime::algorithm::div::<u64>(h, 65536)?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::mul::<u64>(h, 2246822507)?,
        4294967296,
    )?;
    h = h ^ sce_forge_runtime::algorithm::div::<u64>(h, 8192)?;
    h = sce_forge_runtime::algorithm::rem::<u64>(
        sce_forge_runtime::algorithm::mul::<u64>(h, 3266489909)?,
        4294967296,
    )?;
    h = h ^ sce_forge_runtime::algorithm::div::<u64>(h, 65536)?;
    return Ok(sce_forge_runtime::algorithm::narrow::<u32, u64>(h)?);
}
