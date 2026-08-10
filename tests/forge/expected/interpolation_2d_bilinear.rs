#![doc = "SCE-MAP: interpolation_2d_bilinear:1 :: _forge_body"]
// SCE-MAP: interpolation_2d_bilinear:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::interpolation::bilinear;

pub struct Interpolation2dBilinear;

impl Interpolation2dBilinear {
    const AXIS_RPM: [f64; 4] = [800.0, 1200.0, 2000.0, 3000.0];
    const AXIS_LOAD: [f64; 3] = [10.0, 50.0, 100.0];
    const VALUES: [[f64; 3]; 4] = [
        [2.1, 4.5, 7.0],
        [2.5, 5.0, 8.0],
        [3.0, 6.0, 9.5],
        [3.5, 7.0, 11.0],
    ];

    pub fn lookup(rpm: u16, load: u8) -> f64 {
        bilinear(
            &Self::AXIS_RPM,
            &Self::AXIS_LOAD,
            &Self::VALUES,
            rpm as f64,
            load as f64,
        )
    }
}
