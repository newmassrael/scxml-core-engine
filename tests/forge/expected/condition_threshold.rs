// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

pub fn condition_threshold(coolant_temp: f64, oil_temp: f64, max_temp: f64) -> bool {
    coolant_temp > max_temp || oil_temp > max_temp
}