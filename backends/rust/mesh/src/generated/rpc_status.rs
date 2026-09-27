// SCE-GENERATED — DO NOT EDIT
// source-hash: 0ffd4f5aaee672eb45b262966e33f5e18902327fd6170e44dc26c067a1ddada4
#![doc = "SCE-MAP: rpc_status.scxml:7 :: _forge_body"]
// SCE-MAP: rpc_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcStatus {
    Ok = 0,
    Cancelled = 1,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
}

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
            13 => Some(Self::Internal),
            14 => Some(Self::Unavailable),
            _ => None,
        }
    }
}

impl Default for RpcStatus {
    fn default() -> Self {
        Self::Ok
    }
}
