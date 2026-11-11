// SCE-GENERATED — DO NOT EDIT
// source-hash: db4336c6a9bf789bb8f0ab985d2bf7ad97273b841d99ca0c54e9bd1050729ae4
#![doc = "SCE-MAP: shape_kind_open.scxml:4 :: _forge_body"]
// SCE-MAP: shape_kind_open.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeKindOpen(pub u8);

#[allow(non_upper_case_globals, dead_code)]
impl ShapeKindOpen {
    pub const Dot: Self = Self(0);
    pub const Circle: Self = Self(1);
    pub const Rect: Self = Self(2);

    /// The carrier value this holds.
    pub const fn to_underlying(self) -> u8 {
        self.0
    }

    /// The value `raw` stands for. Total: the declared set is open, so
    /// every carrier value is one of this type's values.
    pub const fn from_underlying(raw: u8) -> Self {
        Self(raw)
    }

    /// The name the document declares for this value, or `None` for a
    /// value the open set holds without naming.
    pub const fn declared_name(self) -> Option<&'static str> {
        match self.0 {
            0 => Some("dot"),
            1 => Some("circle"),
            2 => Some("rect"),
            _ => None,
        }
    }
}

impl Default for ShapeKindOpen {
    fn default() -> Self {
        Self::Dot
    }
}
