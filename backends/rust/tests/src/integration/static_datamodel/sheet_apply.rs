// SCE-GENERATED — DO NOT EDIT
// source-hash: 8149e47f56da0bbb92d7b1c4600956bbfd585c43526372f3618b2e600843296d
#![doc = "SCE-MAP: sheet_apply.scxml:21 :: _forge_body"]
// SCE-MAP: sheet_apply.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function, no instance state. `#![no_std]`-clean when no `bytes`
// parameter (this fixture: no_std_clean = true).

use super::sheet_value;

#[allow(clippy::all)]
#[allow(unused_assignments)]
pub fn sheet_apply(
    op: u8,
    a: i64,
    b: i64,
) -> Result<sheet_value::SheetValuePayload, sce_forge_runtime::algorithm::AlgorithmError> {
    let top: i64 = 9223372036854775807;
    let bottom: i64 = sce_forge_runtime::algorithm::sub::<i64>(
        sce_forge_runtime::algorithm::neg::<i64>(top)?,
        1,
    )?;
    let mut value: i64 = 0;
    let mut fault: u8 = 0;
    if op == 2 {
        if b > 0 && a > sce_forge_runtime::algorithm::sub::<i64>(top, b)?
            || b < 0 && a < sce_forge_runtime::algorithm::sub::<i64>(bottom, b)?
        {
            fault = 3;
        } else {
            value = sce_forge_runtime::algorithm::add::<i64>(a, b)?;
        }
    } else {
        if op == 3 {
            if b < 0 && a > sce_forge_runtime::algorithm::add::<i64>(top, b)?
                || b > 0 && a < sce_forge_runtime::algorithm::add::<i64>(bottom, b)?
            {
                fault = 3;
            } else {
                value = sce_forge_runtime::algorithm::sub::<i64>(a, b)?;
            }
        } else {
            if op == 4 {
                if a == 0 || b == 0 {
                    value = 0;
                } else {
                    if a > 0 && b > 0 && a > sce_forge_runtime::algorithm::div::<i64>(top, b)?
                        || a > 0
                            && b < 0
                            && b < sce_forge_runtime::algorithm::div::<i64>(bottom, a)?
                        || a < 0
                            && b > 0
                            && a < sce_forge_runtime::algorithm::div::<i64>(bottom, b)?
                        || a < 0 && b < 0 && b < sce_forge_runtime::algorithm::div::<i64>(top, a)?
                    {
                        fault = 3;
                    } else {
                        value = sce_forge_runtime::algorithm::mul::<i64>(a, b)?;
                    }
                }
            } else {
                if op == 5 {
                    if b == 0 {
                        fault = 2;
                    } else {
                        if a == bottom && b == -1 {
                            fault = 3;
                        } else {
                            value = sce_forge_runtime::algorithm::div::<i64>(a, b)?;
                        }
                    }
                } else {
                    fault = 5;
                }
            }
        }
    }
    let result = sheet_value::SheetValuePayload {
        value: value,
        fault: fault,
    };
    return Ok(result);
}
