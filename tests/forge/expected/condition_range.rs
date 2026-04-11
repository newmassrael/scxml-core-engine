// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Do not edit — regenerate from the source SCXML file.

pub fn condition_range(rpm: u32, min_rpm: u32, max_rpm: u32) -> bool {
    rpm >= min_rpm && rpm <= max_rpm
}
