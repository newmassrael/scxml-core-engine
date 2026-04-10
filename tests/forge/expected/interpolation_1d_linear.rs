// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

pub struct Interpolation1dLinear;

impl Interpolation1dLinear {
    const AXIS_RPM: [f64; 6] = [800.0, 1200.0, 2000.0, 3000.0, 4000.0, 6000.0];
    const VALUES: [f64; 6] = [120.0, 145.0, 200.0, 230.0, 210.0, 180.0];

    pub fn lookup(rpm: u16) -> f64 {
        Self::linear_interpolate(
            &Self::AXIS_RPM, &Self::VALUES,
            rpm as f64)
    }

    fn linear_interpolate(axis: &[f64], values: &[f64], x: f64) -> f64 {
        let n = axis.len();
        if x <= axis[0] { return values[0]; }
        if x >= axis[n - 1] { return values[n - 1]; }
        for i in 0..n - 1 {
            if x <= axis[i + 1] {
                let t = (x - axis[i]) / (axis[i + 1] - axis[i]);
                return values[i] + t * (values[i + 1] - values[i]);
            }
        }
        values[n - 1]
    }
}