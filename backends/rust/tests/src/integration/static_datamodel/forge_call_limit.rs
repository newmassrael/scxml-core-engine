// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ec1a747bc5501f93d66b3929221786d0a45ece3c23f323814ee50eb28f290d9
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
