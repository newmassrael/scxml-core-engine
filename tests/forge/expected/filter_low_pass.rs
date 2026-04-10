// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

pub struct FilterLowPass {
    prev: f64,
    initialized: bool,
}

impl FilterLowPass {
    pub fn new() -> Self {
        Self { prev: 0.0, initialized: false }
    }

    pub fn update(&mut self, rawSignal: f64) -> f64 {
        if !self.initialized {
            self.prev = rawSignal as f64;
            self.initialized = true;
            return self.prev;
        }
        self.prev = 0.1_f64 * rawSignal as f64 + (1.0 - 0.1_f64) * self.prev;
        self.prev
    }

    pub fn reset(&mut self) {
        self.prev = 0.0;
        self.initialized = false;
    }
}
