// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fafd5dc7cb78db436777a56fff2cdefea18c3275c9e740c945e368e87cb13ce
#![doc = "SCE-MAP: forge_call_status.scxml:7 :: _forge_body"]
// SCE-MAP: forge_call_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForgeCallStatus {
    #[default]
    Open = 1,
    Held = 2,
    Done = 3,
}

#[allow(dead_code)]
impl ForgeCallStatus {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::Open),
            2 => Some(Self::Held),
            3 => Some(Self::Done),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Held => "held",
            Self::Done => "done",
        }
    }
}
