// SCE Forge: Auto-generated from Extended SCXML (sce:kind="interpolation")
// Do not edit — regenerate from the source SCXML file.

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
        Self::bilinear_interpolate(
            &Self::AXIS_RPM, &Self::AXIS_LOAD, &Self::VALUES,
            rpm as f64, load as f64)
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

    fn bilinear_interpolate(
            axis_x: &[f64], axis_y: &[f64],
            table: &[[f64; 3]; 4],
            x_in: f64, y_in: f64) -> f64 {
        let x = x_in.clamp(axis_x[0], axis_x[axis_x.len() - 1]);
        let y = y_in.clamp(axis_y[0], axis_y[axis_y.len() - 1]);
        let mut ix = 0;
        let mut iy = 0;
        for i in 0..axis_x.len() - 1 { if x <= axis_x[i + 1] { ix = i; break; } ix = i; }
        for i in 0..axis_y.len() - 1 { if y <= axis_y[i + 1] { iy = i; break; } iy = i; }
        let tx = (x - axis_x[ix]) / (axis_x[ix + 1] - axis_x[ix]);
        let ty = (y - axis_y[iy]) / (axis_y[iy + 1] - axis_y[iy]);
        let a = table[ix][iy] + tx * (table[ix + 1][iy] - table[ix][iy]);
        let b = table[ix][iy + 1] + tx * (table[ix + 1][iy + 1] - table[ix][iy + 1]);
        a + ty * (b - a)
    }
}