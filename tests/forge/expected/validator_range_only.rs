// SCE Forge: Auto-generated from Extended SCXML (sce:kind="validator")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub valid: bool,
    pub reason: String,
}

pub struct ValidatorRangeOnly {
}

impl ValidatorRangeOnly {
    pub fn new() -> Self {
        Self {
        }
    }

    pub fn validate(&mut self, temperature: f64) -> ValidationResult {
        if temperature < -40.0 || temperature > 150.0 {
            return ValidationResult { valid: false, reason: "temperature_out_of_range".to_string() };
        }
        ValidationResult { valid: true, reason: String::new() }
    }
}
