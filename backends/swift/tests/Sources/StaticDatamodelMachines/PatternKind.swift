// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
// SCE-MAP: pattern_kind.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint16"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `pattern_kind` (sce:kind="enum").
public enum PatternKind: UInt16, CaseIterable, Sendable {
    case fireForget = 1
    case rpcRequest = 2
    case rpcReply = 3
    case eventSubscribe = 4
    case eventUnsubscribe = 5
    case eventNotify = 6
    case fieldRead = 7
    case fieldWrite = 8
    case fieldNotify = 9
    case invokeStart = 14
    case invokeStarted = 15
    case childEvent = 16
    case parentEvent = 17
    case invokeDone = 18
    case invokeCancel = 19
    case invokeError = 20
    case parallelRegionDone = 21

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .fireForget: return "fireForget"
        case .rpcRequest: return "rpcRequest"
        case .rpcReply: return "rpcReply"
        case .eventSubscribe: return "eventSubscribe"
        case .eventUnsubscribe: return "eventUnsubscribe"
        case .eventNotify: return "eventNotify"
        case .fieldRead: return "fieldRead"
        case .fieldWrite: return "fieldWrite"
        case .fieldNotify: return "fieldNotify"
        case .invokeStart: return "invokeStart"
        case .invokeStarted: return "invokeStarted"
        case .childEvent: return "childEvent"
        case .parentEvent: return "parentEvent"
        case .invokeDone: return "invokeDone"
        case .invokeCancel: return "invokeCancel"
        case .invokeError: return "invokeError"
        case .parallelRegionDone: return "parallelRegionDone"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: PatternKind) -> UInt16 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt16) -> PatternKind? {
        PatternKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "fireForget": self = .fireForget
        case "rpcRequest": self = .rpcRequest
        case "rpcReply": self = .rpcReply
        case "eventSubscribe": self = .eventSubscribe
        case "eventUnsubscribe": self = .eventUnsubscribe
        case "eventNotify": self = .eventNotify
        case "fieldRead": self = .fieldRead
        case "fieldWrite": self = .fieldWrite
        case "fieldNotify": self = .fieldNotify
        case "invokeStart": self = .invokeStart
        case "invokeStarted": self = .invokeStarted
        case "childEvent": self = .childEvent
        case "parentEvent": self = .parentEvent
        case "invokeDone": self = .invokeDone
        case "invokeCancel": self = .invokeCancel
        case "invokeError": self = .invokeError
        case "parallelRegionDone": self = .parallelRegionDone
        default: return nil
        }
    }
}
