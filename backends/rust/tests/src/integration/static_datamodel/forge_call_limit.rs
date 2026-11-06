// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
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
