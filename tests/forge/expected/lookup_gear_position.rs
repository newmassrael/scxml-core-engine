// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Do not edit — regenerate from the source SCXML file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gear {
    Park,
    Reverse,
    Neutral,
    Drive,
    Sport,
}

pub fn lookup_gear(gear_raw: u8) -> Gear {
    match gear_raw {
        3 => Gear::Drive,
        2 => Gear::Neutral,
        0 => Gear::Park,
        1 => Gear::Reverse,
        4 => Gear::Sport,
        _ => Gear::Neutral,
    }
}
