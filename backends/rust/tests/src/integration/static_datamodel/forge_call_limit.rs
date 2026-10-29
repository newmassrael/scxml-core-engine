// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
#![doc = "SCE-MAP: forge_call_limit.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_limit.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::interpolation::linear as sce_linear;

pub struct ForgeCallLimit;

impl ForgeCallLimit {
    const AXIS_RPM: [f64; 3] = [0.0, 1000.0, 2000.0];
    const VALUES: [f64; 3] = [100.0, 150.0, 120.0];

    pub fn lookup(rpm: u16) -> f64 {
        sce_linear(&Self::AXIS_RPM, &Self::VALUES, rpm as f64)
    }
}
