// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::interpolation::linear;

pub struct Interpolation1dLinear;

impl Interpolation1dLinear {
    const AXIS_RPM: [f64; 6] = [800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0];
    const VALUES: [f64; 6] = [120.0, 145.0, 200.0, 230.0, 210.0, 180.0];

    pub fn lookup(rpm: u16) -> f64 {
        linear(
            &Self::AXIS_RPM,
            &Self::VALUES,
            rpm as f64,
        )
    }
}