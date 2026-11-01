// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
// SCE-MAP: rpc_status.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `rpc_status` (sce:kind="enum").
public enum RpcStatus: UInt8, CaseIterable, Sendable {
    case ok = 0
    case cancelled = 1
    case invalidArgument = 3
    case deadlineExceeded = 4
    case notFound = 5
    case unimplemented = 12
    case internalError = 13
    case unavailable = 14

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .ok: return "ok"
        case .cancelled: return "cancelled"
        case .invalidArgument: return "invalidArgument"
        case .deadlineExceeded: return "deadlineExceeded"
        case .notFound: return "notFound"
        case .unimplemented: return "unimplemented"
        case .internalError: return "internalError"
        case .unavailable: return "unavailable"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: RpcStatus) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> RpcStatus? {
        RpcStatus(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "ok": self = .ok
        case "cancelled": self = .cancelled
        case "invalidArgument": self = .invalidArgument
        case "deadlineExceeded": self = .deadlineExceeded
        case "notFound": self = .notFound
        case "unimplemented": self = .unimplemented
        case "internalError": self = .internalError
        case "unavailable": self = .unavailable
        default: return nil
        }
    }
}
