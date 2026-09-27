// SCE-GENERATED — DO NOT EDIT
// source-hash: ab119d19c373fb9e83e74bd30f74186a9e2f87ef70ba045b5c5ab8bb9e9d1849
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
}
