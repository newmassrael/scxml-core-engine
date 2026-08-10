#![doc = "SCE-MAP: lookup_single_default:3 :: _forge_body"]
// SCE-MAP: lookup_single_default:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    None,
    Low,
    Medium,
    High,
}

pub fn lookup_quality(level: u8) -> Quality {
    match level {
        3 => Quality::High,
        1 => Quality::Low,
        2 => Quality::Medium,
        0 => Quality::None,
        _ => Quality::None,
    }
}
