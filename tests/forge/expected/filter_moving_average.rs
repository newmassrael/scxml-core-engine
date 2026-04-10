// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

pub struct FilterMovingAverage {
    buffer: [f64; 5],
    index: usize,
    filled: bool,
}

impl FilterMovingAverage {
    pub fn new() -> Self {
        Self { buffer: [0.0; 5], index: 0, filled: false }
    }

    pub fn update(&mut self, rawTemp: f64) -> f64 {
        self.buffer[self.index] = rawTemp as f64;
        self.index = (self.index + 1) % 5;
        if !self.filled && self.index == 0 { self.filled = true; }
        let count = if self.filled { 5 } else { self.index };
        let sum: f64 = self.buffer[..count].iter().sum();
        sum / count as f64
    }

    pub fn reset(&mut self) {
        self.buffer = [0.0; 5];
        self.index = 0;
        self.filled = false;
    }
}
