// SCE-GENERATED — DO NOT EDIT
// source-hash: 877621f37229fecc457ee8a018ed01bc608e1a8e4c0614dec2a1a4502368fff1
// source: pattern_kind.scxml
#![doc = "SCE-MAP: pattern_kind.scxml:18 :: _forge_body"]
// SCE-MAP: pattern_kind.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PatternKind {
    #[default]
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
    DurableReceipt = 22,
    CausalVector = 23,
}

#[allow(dead_code)]
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
            22 => Some(Self::DurableReceipt),
            23 => Some(Self::CausalVector),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::FireForget => "fireForget",
            Self::RpcRequest => "rpcRequest",
            Self::RpcReply => "rpcReply",
            Self::EventSubscribe => "eventSubscribe",
            Self::EventUnsubscribe => "eventUnsubscribe",
            Self::EventNotify => "eventNotify",
            Self::FieldRead => "fieldRead",
            Self::FieldWrite => "fieldWrite",
            Self::FieldNotify => "fieldNotify",
            Self::InvokeStart => "invokeStart",
            Self::InvokeStarted => "invokeStarted",
            Self::ChildEvent => "childEvent",
            Self::ParentEvent => "parentEvent",
            Self::InvokeDone => "invokeDone",
            Self::InvokeCancel => "invokeCancel",
            Self::InvokeError => "invokeError",
            Self::ParallelRegionDone => "parallelRegionDone",
            Self::DurableReceipt => "durableReceipt",
            Self::CausalVector => "causalVector",
        }
    }
}

/// docs/adr/0108, D1: a value of this enum is a key of a map by the number its
/// document declares for it.
impl ::sce_portable_bytes::MapKey for PatternKind {
    fn feed_key(&self, into: &mut dyn ::sce_portable_bytes::KeyFeed) {
        ::sce_portable_bytes::MapKey::feed_key(&self.to_underlying(), into);
    }
}
