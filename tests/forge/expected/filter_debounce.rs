// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Do not edit — regenerate from the source SCXML file.

pub struct FilterDebounce {
    stable_value: bool,
    candidate: bool,
    count: usize,
    initialized: bool,
}

impl FilterDebounce {
    pub fn new() -> Self {
        Self { stable_value: Default::default(), candidate: Default::default(), count: 0, initialized: false }
    }

    pub fn update(&mut self, rawButton: bool) -> bool {
        let value = rawButton;
        if !self.initialized {
            self.stable_value = value;
            self.candidate = value;
            self.count = 1;
            self.initialized = true;
            return self.stable_value;
        }
        if value == self.candidate {
            self.count += 1;
            if self.count >= 3 {
                self.stable_value = self.candidate;
            }
        } else {
            self.candidate = value;
            self.count = 1;
        }
        self.stable_value
    }

    pub fn reset(&mut self) {
        self.stable_value = Default::default();
        self.candidate = Default::default();
        self.count = 0;
        self.initialized = false;
    }
}
