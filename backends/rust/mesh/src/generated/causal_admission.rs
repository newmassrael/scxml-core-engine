// SCE-GENERATED — DO NOT EDIT
// source-hash: 6e3fa3ae1d1df10dd29fa4c2afbffd093df1f9518f7a44d21f2f01a956798b04
// source: causal_admission.scxml
#![doc = "SCE-MAP: causal_admission.scxml:11 :: _forge_body"]
// SCE-MAP: causal_admission.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CausalAdmission {
    #[default]
    Drop = 0,
    Deliver = 1,
    Hold = 2,
}

#[allow(dead_code)]
impl CausalAdmission {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Drop),
            1 => Some(Self::Deliver),
            2 => Some(Self::Hold),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::Drop => "drop",
            Self::Deliver => "deliver",
            Self::Hold => "hold",
        }
    }
}

/// docs/adr/0108, D1: a value of this enum is a key of a map by the number its
/// document declares for it.
impl ::sce_portable_bytes::MapKey for CausalAdmission {
    fn feed_key(&self, into: &mut dyn ::sce_portable_bytes::KeyFeed) {
        ::sce_portable_bytes::MapKey::feed_key(&self.to_underlying(), into);
    }
}
