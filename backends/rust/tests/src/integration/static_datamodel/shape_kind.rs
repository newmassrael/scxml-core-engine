// SCE-GENERATED — DO NOT EDIT
// source-hash: 185d995fbef5e4650ae7c20d165e4a208237e26dd7d946178edb7b10ba93cc9e
#![doc = "SCE-MAP: shape_kind.scxml:4 :: _forge_body"]
// SCE-MAP: shape_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShapeKind {
    #[default]
    Dot = 0,
    Circle = 1,
    Rect = 2,
}

#[allow(dead_code)]
impl ShapeKind {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Dot),
            1 => Some(Self::Circle),
            2 => Some(Self::Rect),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::Dot => "dot",
            Self::Circle => "circle",
            Self::Rect => "rect",
        }
    }
}
