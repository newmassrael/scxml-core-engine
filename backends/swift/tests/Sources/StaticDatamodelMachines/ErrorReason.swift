// SCE-GENERATED — DO NOT EDIT
// source-hash: 55e9ecc57d956027bb6dd08c16d46cb9a2c0a99fe474a4b0312c33d8ac0ac34c
// source: sce:std/engine/error_reason.scxml
// SCE-MAP: error_reason.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `error_reason` (sce:kind="enum").
public enum ErrorReason: UInt8, CaseIterable, Sendable {
    case expressionFailed = 1
    case locationInvalid = 2
    case assignmentFailed = 3
    case scriptFailed = 4
    case dataInitFailed = 5
    case foreachInvalid = 6
    case sendTypeUnsupported = 7
    case sendTargetInvalid = 8
    case sendDelayInvalid = 9
    case paramUnfit = 10
    case processorUnregistered = 11
    case invokeTypeUnsupported = 12
    case invokeSrcNotFound = 13
    case invokeChildInitFailed = 14
    case reservedParamConflict = 15
    case sessionTransportUnavailable = 16
    case payloadRefused = 17
    case capacityExceeded = 18
    case targetUnreachable = 64
    case httpTargetInvalid = 65
    case transportUnavailable = 66
    case sendFailed = 67
    case deliveryExhausted = 68
    case envelopeCorrupt = 69
    case invokeChildLost = 70
    case parallelBarrierTimeout = 71
    case dedupWindowOverflow = 72
    case peerPartitioned = 73
    case backpressureDrop = 74
    case unauthorized = 75
    case missingSequence = 76
    case orderingGap = 77
    case regionPartitioned = 78
    case rpcReplyFromUndeclaredPeer = 79
    case outboundStaleDrop = 80
    case notificationDeadlineMissed = 81
    case restored = 82
    case causalViewMismatch = 83

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .expressionFailed: return "expressionFailed"
        case .locationInvalid: return "locationInvalid"
        case .assignmentFailed: return "assignmentFailed"
        case .scriptFailed: return "scriptFailed"
        case .dataInitFailed: return "dataInitFailed"
        case .foreachInvalid: return "foreachInvalid"
        case .sendTypeUnsupported: return "sendTypeUnsupported"
        case .sendTargetInvalid: return "sendTargetInvalid"
        case .sendDelayInvalid: return "sendDelayInvalid"
        case .paramUnfit: return "paramUnfit"
        case .processorUnregistered: return "processorUnregistered"
        case .invokeTypeUnsupported: return "invokeTypeUnsupported"
        case .invokeSrcNotFound: return "invokeSrcNotFound"
        case .invokeChildInitFailed: return "invokeChildInitFailed"
        case .reservedParamConflict: return "reservedParamConflict"
        case .sessionTransportUnavailable: return "sessionTransportUnavailable"
        case .payloadRefused: return "payloadRefused"
        case .capacityExceeded: return "capacityExceeded"
        case .targetUnreachable: return "targetUnreachable"
        case .httpTargetInvalid: return "httpTargetInvalid"
        case .transportUnavailable: return "transportUnavailable"
        case .sendFailed: return "sendFailed"
        case .deliveryExhausted: return "deliveryExhausted"
        case .envelopeCorrupt: return "envelopeCorrupt"
        case .invokeChildLost: return "invokeChildLost"
        case .parallelBarrierTimeout: return "parallelBarrierTimeout"
        case .dedupWindowOverflow: return "dedupWindowOverflow"
        case .peerPartitioned: return "peerPartitioned"
        case .backpressureDrop: return "backpressureDrop"
        case .unauthorized: return "unauthorized"
        case .missingSequence: return "missingSequence"
        case .orderingGap: return "orderingGap"
        case .regionPartitioned: return "regionPartitioned"
        case .rpcReplyFromUndeclaredPeer: return "rpcReplyFromUndeclaredPeer"
        case .outboundStaleDrop: return "outboundStaleDrop"
        case .notificationDeadlineMissed: return "notificationDeadlineMissed"
        case .restored: return "restored"
        case .causalViewMismatch: return "causalViewMismatch"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: ErrorReason) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> ErrorReason? {
        ErrorReason(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "expressionFailed": self = .expressionFailed
        case "locationInvalid": self = .locationInvalid
        case "assignmentFailed": self = .assignmentFailed
        case "scriptFailed": self = .scriptFailed
        case "dataInitFailed": self = .dataInitFailed
        case "foreachInvalid": self = .foreachInvalid
        case "sendTypeUnsupported": self = .sendTypeUnsupported
        case "sendTargetInvalid": self = .sendTargetInvalid
        case "sendDelayInvalid": self = .sendDelayInvalid
        case "paramUnfit": self = .paramUnfit
        case "processorUnregistered": self = .processorUnregistered
        case "invokeTypeUnsupported": self = .invokeTypeUnsupported
        case "invokeSrcNotFound": self = .invokeSrcNotFound
        case "invokeChildInitFailed": self = .invokeChildInitFailed
        case "reservedParamConflict": self = .reservedParamConflict
        case "sessionTransportUnavailable": self = .sessionTransportUnavailable
        case "payloadRefused": self = .payloadRefused
        case "capacityExceeded": self = .capacityExceeded
        case "targetUnreachable": self = .targetUnreachable
        case "httpTargetInvalid": self = .httpTargetInvalid
        case "transportUnavailable": self = .transportUnavailable
        case "sendFailed": self = .sendFailed
        case "deliveryExhausted": self = .deliveryExhausted
        case "envelopeCorrupt": self = .envelopeCorrupt
        case "invokeChildLost": self = .invokeChildLost
        case "parallelBarrierTimeout": self = .parallelBarrierTimeout
        case "dedupWindowOverflow": self = .dedupWindowOverflow
        case "peerPartitioned": self = .peerPartitioned
        case "backpressureDrop": self = .backpressureDrop
        case "unauthorized": self = .unauthorized
        case "missingSequence": self = .missingSequence
        case "orderingGap": self = .orderingGap
        case "regionPartitioned": self = .regionPartitioned
        case "rpcReplyFromUndeclaredPeer": self = .rpcReplyFromUndeclaredPeer
        case "outboundStaleDrop": self = .outboundStaleDrop
        case "notificationDeadlineMissed": self = .notificationDeadlineMissed
        case "restored": self = .restored
        case "causalViewMismatch": self = .causalViewMismatch
        default: return nil
        }
    }
}
