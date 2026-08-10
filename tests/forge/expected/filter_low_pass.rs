#![doc = "SCE-MAP: filter_low_pass:1 :: _forge_body"]
// SCE-MAP: filter_low_pass:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::filter::LowPass;

pub struct FilterLowPass {
    impl_: LowPass<f64>,
}

impl FilterLowPass {
    pub fn new() -> Self {
        Self {
            impl_: LowPass::new(0.1_f64),
        }
    }

    pub fn update(&mut self, raw_signal: f64) -> f64 {
        self.impl_.update(raw_signal as f64)
    }

    pub fn reset(&mut self) {
        self.impl_.reset();
    }
}

impl Default for FilterLowPass {
    fn default() -> Self {
        Self::new()
    }
}
