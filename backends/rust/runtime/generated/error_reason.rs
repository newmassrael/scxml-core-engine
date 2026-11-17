// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e7868e2e35cbb777852462588336307ea70f196d6e164600b644a4b693ccd2e
#![doc = "SCE-MAP: error_reason.scxml:55 :: _forge_body"]
// SCE-MAP: error_reason.scxml:55 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorReason {
    #[default]
    ExpressionFailed = 1,
    LocationInvalid = 2,
    AssignmentFailed = 3,
    ScriptFailed = 4,
    DataInitFailed = 5,
    ForeachInvalid = 6,
    SendTypeUnsupported = 7,
    SendTargetInvalid = 8,
    SendDelayInvalid = 9,
    ParamUnfit = 10,
    ProcessorUnregistered = 11,
    InvokeTypeUnsupported = 12,
    InvokeSrcNotFound = 13,
    InvokeChildInitFailed = 14,
    ReservedParamConflict = 15,
    SessionTransportUnavailable = 16,
    PayloadRefused = 17,
    TargetUnreachable = 64,
    HttpTargetInvalid = 65,
    TransportUnavailable = 66,
    SendFailed = 67,
    DeliveryExhausted = 68,
    EnvelopeCorrupt = 69,
    InvokeChildLost = 70,
    ParallelBarrierTimeout = 71,
    DedupWindowOverflow = 72,
    PeerPartitioned = 73,
    BackpressureDrop = 74,
    Unauthorized = 75,
    MissingSequence = 76,
    OrderingGap = 77,
    RegionPartitioned = 78,
    RpcReplyFromUndeclaredPeer = 79,
    OutboundStaleDrop = 80,
    NotificationDeadlineMissed = 81,
    Restored = 82,
    CausalViewMismatch = 83,
}

#[allow(dead_code)]
impl ErrorReason {
    /// The carrier value this variant declares.
    pub const fn to_underlying(self) -> u8 {
        self as u8
    }

    /// The variant `raw` declares, or `None` — the declared set is closed,
    /// so a value outside it is not a value of this type.
    pub const fn from_underlying(raw: u8) -> Option<Self> {
        match raw {
            1 => Some(Self::ExpressionFailed),
            2 => Some(Self::LocationInvalid),
            3 => Some(Self::AssignmentFailed),
            4 => Some(Self::ScriptFailed),
            5 => Some(Self::DataInitFailed),
            6 => Some(Self::ForeachInvalid),
            7 => Some(Self::SendTypeUnsupported),
            8 => Some(Self::SendTargetInvalid),
            9 => Some(Self::SendDelayInvalid),
            10 => Some(Self::ParamUnfit),
            11 => Some(Self::ProcessorUnregistered),
            12 => Some(Self::InvokeTypeUnsupported),
            13 => Some(Self::InvokeSrcNotFound),
            14 => Some(Self::InvokeChildInitFailed),
            15 => Some(Self::ReservedParamConflict),
            16 => Some(Self::SessionTransportUnavailable),
            17 => Some(Self::PayloadRefused),
            64 => Some(Self::TargetUnreachable),
            65 => Some(Self::HttpTargetInvalid),
            66 => Some(Self::TransportUnavailable),
            67 => Some(Self::SendFailed),
            68 => Some(Self::DeliveryExhausted),
            69 => Some(Self::EnvelopeCorrupt),
            70 => Some(Self::InvokeChildLost),
            71 => Some(Self::ParallelBarrierTimeout),
            72 => Some(Self::DedupWindowOverflow),
            73 => Some(Self::PeerPartitioned),
            74 => Some(Self::BackpressureDrop),
            75 => Some(Self::Unauthorized),
            76 => Some(Self::MissingSequence),
            77 => Some(Self::OrderingGap),
            78 => Some(Self::RegionPartitioned),
            79 => Some(Self::RpcReplyFromUndeclaredPeer),
            80 => Some(Self::OutboundStaleDrop),
            81 => Some(Self::NotificationDeadlineMissed),
            82 => Some(Self::Restored),
            83 => Some(Self::CausalViewMismatch),
            _ => None,
        }
    }

    /// The name the document declares for this variant.
    pub const fn declared_name(self) -> &'static str {
        match self {
            Self::ExpressionFailed => "expressionFailed",
            Self::LocationInvalid => "locationInvalid",
            Self::AssignmentFailed => "assignmentFailed",
            Self::ScriptFailed => "scriptFailed",
            Self::DataInitFailed => "dataInitFailed",
            Self::ForeachInvalid => "foreachInvalid",
            Self::SendTypeUnsupported => "sendTypeUnsupported",
            Self::SendTargetInvalid => "sendTargetInvalid",
            Self::SendDelayInvalid => "sendDelayInvalid",
            Self::ParamUnfit => "paramUnfit",
            Self::ProcessorUnregistered => "processorUnregistered",
            Self::InvokeTypeUnsupported => "invokeTypeUnsupported",
            Self::InvokeSrcNotFound => "invokeSrcNotFound",
            Self::InvokeChildInitFailed => "invokeChildInitFailed",
            Self::ReservedParamConflict => "reservedParamConflict",
            Self::SessionTransportUnavailable => "sessionTransportUnavailable",
            Self::PayloadRefused => "payloadRefused",
            Self::TargetUnreachable => "targetUnreachable",
            Self::HttpTargetInvalid => "httpTargetInvalid",
            Self::TransportUnavailable => "transportUnavailable",
            Self::SendFailed => "sendFailed",
            Self::DeliveryExhausted => "deliveryExhausted",
            Self::EnvelopeCorrupt => "envelopeCorrupt",
            Self::InvokeChildLost => "invokeChildLost",
            Self::ParallelBarrierTimeout => "parallelBarrierTimeout",
            Self::DedupWindowOverflow => "dedupWindowOverflow",
            Self::PeerPartitioned => "peerPartitioned",
            Self::BackpressureDrop => "backpressureDrop",
            Self::Unauthorized => "unauthorized",
            Self::MissingSequence => "missingSequence",
            Self::OrderingGap => "orderingGap",
            Self::RegionPartitioned => "regionPartitioned",
            Self::RpcReplyFromUndeclaredPeer => "rpcReplyFromUndeclaredPeer",
            Self::OutboundStaleDrop => "outboundStaleDrop",
            Self::NotificationDeadlineMissed => "notificationDeadlineMissed",
            Self::Restored => "restored",
            Self::CausalViewMismatch => "causalViewMismatch",
        }
    }
}
