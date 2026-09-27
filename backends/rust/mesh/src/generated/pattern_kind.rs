// SCE-GENERATED — DO NOT EDIT
// source-hash: 418438a744050ac9cd6cb0691018fdb0ef433f86cf92fa1a16f148307512f21e
#![doc = "SCE-MAP: pattern_kind.scxml:11 :: _forge_body"]
// SCE-MAP: pattern_kind.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    FireForget = 1,
    RpcRequest = 2,
    RpcReply = 3,
    EventSubscribe = 4,
    EventUnsubscribe = 5,
    EventNotify = 6,
    FieldRead = 7,
    FieldWrite = 8,
    FieldNotify = 9,
    InvokeStart = 14,
    InvokeStarted = 15,
    ChildEvent = 16,
    ParentEvent = 17,
    InvokeDone = 18,
    InvokeCancel = 19,
    InvokeError = 20,
    ParallelRegionDone = 21,
}

impl PatternKind {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u16 {
        self as u16
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u16) -> Option<Self> {
        match raw {
            1 => Some(Self::FireForget),
            2 => Some(Self::RpcRequest),
            3 => Some(Self::RpcReply),
            4 => Some(Self::EventSubscribe),
            5 => Some(Self::EventUnsubscribe),
            6 => Some(Self::EventNotify),
            7 => Some(Self::FieldRead),
            8 => Some(Self::FieldWrite),
            9 => Some(Self::FieldNotify),
            14 => Some(Self::InvokeStart),
            15 => Some(Self::InvokeStarted),
            16 => Some(Self::ChildEvent),
            17 => Some(Self::ParentEvent),
            18 => Some(Self::InvokeDone),
            19 => Some(Self::InvokeCancel),
            20 => Some(Self::InvokeError),
            21 => Some(Self::ParallelRegionDone),
            _ => None,
        }
    }
}

impl Default for PatternKind {
    fn default() -> Self {
        Self::FireForget
    }
}
