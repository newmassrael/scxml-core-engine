// SCE-GENERATED — DO NOT EDIT
// source-hash: c431fa9e45990e8b729e69a4bf81ba13f9cc4e8066ddecbd5ff6346ac56dd5f3
#![doc = "SCE-MAP: merkle_insert.scxml:25 :: _forge_body"]
// SCE-MAP: merkle_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::merkle_node;

use sce_portable_bytes::SceOwnedList;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn merkle_insert(
    nodes: &[merkle_node::MerkleNodePayload],
    minute: i64,
    hash: u32,
) -> Result<
    SceOwnedList<merkle_node::MerkleNodePayload, 1024>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    if !(minute >= 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut out: SceOwnedList<merkle_node::MerkleNodePayload, 1024> = SceOwnedList::new();
    let mut pd: u32 = 0;
    let mut pp: u64 = 0;
    let mut div: i64 = 14348907;
    let mut prev_depth: i32 = -1;
    let mut prev_prefix: u64 = 0;
    for &x in nodes.iter() {
        if !(x.depth <= 16
            && (x.depth as i32 > prev_depth
                || x.depth as i32 == prev_depth && x.prefix > prev_prefix))
        {
            return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
        }
        prev_depth = x.depth as i32;
        prev_prefix = x.prefix;
        while pd <= 16 && (pd < x.depth as u32 || pd == x.depth as u32 && pp < x.prefix) {
            let created = merkle_node::MerkleNodePayload {
                depth: sce_forge_runtime::algorithm::narrow::<u8, u32>(pd)?,
                prefix: pp,
                hash: hash,
            };
            out.push(created)?;
            pd = sce_forge_runtime::algorithm::add::<u32>(pd, 1)?;
            pp = sce_forge_runtime::algorithm::narrow::<u64, i64>(
                sce_forge_runtime::algorithm::div::<i64>(minute, div)?,
            )?;
            if div > 1 {
                div = sce_forge_runtime::algorithm::div::<i64>(div, 3)?;
            }
        }
        if pd <= 16 && pd == x.depth as u32 && pp == x.prefix {
            let merged = merkle_node::MerkleNodePayload {
                depth: x.depth,
                prefix: x.prefix,
                hash: x.hash ^ hash,
            };
            out.push(merged)?;
            pd = sce_forge_runtime::algorithm::add::<u32>(pd, 1)?;
            pp = sce_forge_runtime::algorithm::narrow::<u64, i64>(
                sce_forge_runtime::algorithm::div::<i64>(minute, div)?,
            )?;
            if div > 1 {
                div = sce_forge_runtime::algorithm::div::<i64>(div, 3)?;
            }
        } else {
            out.push(x)?;
        }
    }
    while pd <= 16 {
        let tail = merkle_node::MerkleNodePayload {
            depth: sce_forge_runtime::algorithm::narrow::<u8, u32>(pd)?,
            prefix: pp,
            hash: hash,
        };
        out.push(tail)?;
        pd = sce_forge_runtime::algorithm::add::<u32>(pd, 1)?;
        pp =
            sce_forge_runtime::algorithm::narrow::<u64, i64>(sce_forge_runtime::algorithm::div::<
                i64,
            >(minute, div)?)?;
        if div > 1 {
            div = sce_forge_runtime::algorithm::div::<i64>(div, 3)?;
        }
    }
    return Ok(out);
}
