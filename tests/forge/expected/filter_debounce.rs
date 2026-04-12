// SCE Forge: Auto-generated from Extended SCXML (sce:kind="filter")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

use sce_forge_runtime::filter::Debounce;

pub struct FilterDebounce {
    impl_: Debounce<bool, 3>,
}

impl FilterDebounce {
    pub fn new() -> Self {
        Self {
            impl_: Debounce::new(),
        }
    }

    pub fn update(&mut self, rawButton: bool) -> bool {
        self.impl_.update(rawButton as bool)
    }

    pub fn reset(&mut self) {
        self.impl_.reset();
    }
}

impl Default for FilterDebounce {
    fn default() -> Self {
        Self::new()
    }
}
