#![doc = "SCE-MAP: algorithm_checked_arith:18 :: _forge_body"]
// SCE-MAP: algorithm_checked_arith:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn algorithm_checked_arith(a: i32, b: i32, op: u8) -> Result<i32, sce_forge_runtime::algorithm::AlgorithmError> {
    let mut r: i32 = 0;
    if op == 0 {
        r = sce_forge_runtime::algorithm::add::<i32>(a, b)?;
    }
    if op == 1 {
        r = sce_forge_runtime::algorithm::sub::<i32>(a, b)?;
    }
    if op == 2 {
        r = sce_forge_runtime::algorithm::mul::<i32>(a, b)?;
    }
    if op == 3 {
        r = sce_forge_runtime::algorithm::div::<i32>(a, b)?;
    }
    if op == 4 {
        r = sce_forge_runtime::algorithm::rem::<i32>(a, b)?;
    }
    if op == 5 {
        r = sce_forge_runtime::algorithm::neg::<i32>(a)?;
    }
    if op == 6 {
        r = sce_forge_runtime::algorithm::sub::<u8>(op, 7)? as i32;
    }
    return Ok(r);
}
