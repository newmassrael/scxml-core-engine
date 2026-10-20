// SCE-GENERATED — DO NOT EDIT
// source-hash: 3590e0da727b5c60e46e46571a637da3e7beb956941ee2491fcd012174a020af
#![doc = "SCE-MAP: payload_codec.scxml:6 :: _forge_body"]
// SCE-MAP: payload_codec.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayloadCodec {
    #[default]
    None = 0,
    Json = 1,
    Cbor = 2,
    Typed = 3,
    Raw = 4,
}

#[allow(dead_code)]
impl PayloadCodec {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::None),
            1 => Some(Self::Json),
            2 => Some(Self::Cbor),
            3 => Some(Self::Typed),
            4 => Some(Self::Raw),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Json => "json",
            Self::Cbor => "cbor",
            Self::Typed => "typed",
            Self::Raw => "raw",
        }
    }
}
