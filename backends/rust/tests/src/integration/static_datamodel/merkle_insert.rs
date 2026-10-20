// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
#![doc = "SCE-MAP: merkle_insert.scxml:33 :: _forge_body"]
// SCE-MAP: merkle_insert.scxml:33 :: _forge_body

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
    SceOwnedList<merkle_node::MerkleNodePayload, 2048>,
    sce_forge_runtime::algorithm::AlgorithmError,
> {
    if !(minute >= 0 && minute <= 153722867280912) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut out: SceOwnedList<merkle_node::MerkleNodePayload, 2048> = SceOwnedList::new();
    let mut pd: u32 = 0;
    let mut pp: u64 = 0;
    let mut div: i64 = 68630377364883;
    let mut prev_depth: i32 = -1;
    let mut prev_prefix: u64 = 0;
    for &x in nodes.iter() {
        if !(x.depth <= 30
            && (x.depth as i32 > prev_depth
                || x.depth as i32 == prev_depth && x.prefix > prev_prefix))
        {
            return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
        }
        prev_depth = x.depth as i32;
        prev_prefix = x.prefix;
        while pd <= 30 && (pd < x.depth as u32 || pd == x.depth as u32 && pp < x.prefix) {
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
        if pd <= 30 && pd == x.depth as u32 && pp == x.prefix {
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
    while pd <= 30 {
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
