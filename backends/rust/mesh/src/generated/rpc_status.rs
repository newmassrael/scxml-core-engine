// SCE-GENERATED — DO NOT EDIT
// source-hash: 85f4ad9a236dfd429987d41a78aed6eef77c3c1fde7cd2d1c2fa1def9f03d5e8
// source: rpc_status.scxml
#![doc = "SCE-MAP: rpc_status.scxml:12 :: _forge_body"]
// SCE-MAP: rpc_status.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RpcStatus {
    #[default]
    Ok = 0,
    Cancelled = 1,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    Unimplemented = 12,
    InternalError = 13,
    Unavailable = 14,
}

#[allow(dead_code)]
impl RpcStatus {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Ok),
            1 => Some(Self::Cancelled),
            3 => Some(Self::InvalidArgument),
            4 => Some(Self::DeadlineExceeded),
            5 => Some(Self::NotFound),
            12 => Some(Self::Unimplemented),
            13 => Some(Self::InternalError),
            14 => Some(Self::Unavailable),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Cancelled => "cancelled",
            Self::InvalidArgument => "invalidArgument",
            Self::DeadlineExceeded => "deadlineExceeded",
            Self::NotFound => "notFound",
            Self::Unimplemented => "unimplemented",
            Self::InternalError => "internalError",
            Self::Unavailable => "unavailable",
        }
    }
}

/// docs/adr/0108, D1: a value of this enum is a key of a map by the number its
/// document declares for it.
impl ::sce_portable_bytes::MapKey for RpcStatus {
    fn feed_key(&self, into: &mut dyn ::sce_portable_bytes::KeyFeed) {
        ::sce_portable_bytes::MapKey::feed_key(&self.to_underlying(), into);
    }
}
