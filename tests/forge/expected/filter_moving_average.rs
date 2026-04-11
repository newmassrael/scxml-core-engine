// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::filter::MovingAverage;

pub struct FilterMovingAverage {
    impl_: MovingAverage<f64, 5>,
}

impl FilterMovingAverage {
    pub fn new() -> Self {
        Self {
            impl_: MovingAverage::new(),
        }
    }

    pub fn update(&mut self, rawTemp: f64) -> f64 {
        self.impl_.update(rawTemp as f64)
    }

    pub fn reset(&mut self) {
        self.impl_.reset();
    }
}

impl Default for FilterMovingAverage {
    fn default() -> Self {
        Self::new()
    }
}
