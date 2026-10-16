// SCE-GENERATED — DO NOT EDIT
// source-hash: 68d77b5e0c96e361239e1512887ba2ef6d35bfab0188bf0e2afbd53ca387f3bc
#![doc = "SCE-MAP: utc_from_local.scxml:30 :: _forge_body"]
// SCE-MAP: utc_from_local.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn utc_from_local(
    starts: &[i64],
    offsets: &[i32],
    local: i64,
) -> Result<i64, sce_forge_runtime::algorithm::AlgorithmError> {
    if !((starts).len() == (offsets).len() && (starts).len() > 0) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    if !(sce_forge_runtime::algorithm::sub::<i64>(
        local,
        sce_forge_runtime::algorithm::at(&offsets, 0)? as i64,
    )? >= sce_forge_runtime::algorithm::at(&starts, 0)?)
    {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    let n: u32 = (starts).len() as u32;
    let mut k: u32 = 0;
    while k < n {
        if k > 0 {
            if !(sce_forge_runtime::algorithm::at(&starts, k)?
                > sce_forge_runtime::algorithm::at(
                    &starts,
                    sce_forge_runtime::algorithm::sub::<u32>(k, 1)?,
                )?)
            {
                return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
            }
        }
        let u: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            local,
            sce_forge_runtime::algorithm::at(&offsets, k)? as i64,
        )?;
        if u >= sce_forge_runtime::algorithm::at(&starts, k)?
            && (sce_forge_runtime::algorithm::add::<u32>(k, 1)? == n
                || u < sce_forge_runtime::algorithm::at(
                    &starts,
                    sce_forge_runtime::algorithm::add::<u32>(k, 1)?,
                )?)
        {
            return Ok(u);
        }
        k = sce_forge_runtime::algorithm::add::<u32>(k, 1)?;
    }
    let mut g: u32 = 1;
    while g < n {
        let before: i64 = sce_forge_runtime::algorithm::sub::<i64>(
            local,
            sce_forge_runtime::algorithm::at(
                &offsets,
                sce_forge_runtime::algorithm::sub::<u32>(g, 1)?,
            )? as i64,
        )?;
        if before >= sce_forge_runtime::algorithm::at(&starts, g)?
            && sce_forge_runtime::algorithm::sub::<i64>(
                local,
                sce_forge_runtime::algorithm::at(&offsets, g)? as i64,
            )? < sce_forge_runtime::algorithm::at(&starts, g)?
        {
            return Ok(before);
        }
        g = sce_forge_runtime::algorithm::add::<u32>(g, 1)?;
    }
    if !(false) {
        return Err(sce_forge_runtime::algorithm::AlgorithmError::Precondition);
    }
    return Ok(0);
}
