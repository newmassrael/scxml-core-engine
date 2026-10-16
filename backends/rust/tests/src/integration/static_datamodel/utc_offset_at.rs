// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
#![doc = "SCE-MAP: utc_offset_at.scxml:25 :: _forge_body"]
// SCE-MAP: utc_offset_at.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn utc_offset_at(
    starts: &[i64],
    offsets: &[i32],
    t: i64,
) -> Result<i32, sce_forge_runtime::algorithm::AlgorithmError> {
    if !((starts).len() == (offsets).len() && (starts).len() > 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    if !(t >= sce_forge_runtime::algorithm::at(&starts, 0)?) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let mut k: u32 = 0;
    let mut i: u32 = 1;
    while i < (starts).len() as u32 {
        if !(sce_forge_runtime::algorithm::at(&starts, i)?
            > sce_forge_runtime::algorithm::at(
                &starts,
                sce_forge_runtime::algorithm::sub::<u32>(i, 1)?,
            )?)
        {
            return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
        }
        if sce_forge_runtime::algorithm::at(&starts, i)? <= t {
            k = i;
        }
        i = sce_forge_runtime::algorithm::add::<u32>(i, 1)?;
    }
    return Ok(sce_forge_runtime::algorithm::at(&offsets, k)?);
}
