// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_refusal.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `record_refusal` (sce:kind="enum").
public enum RecordRefusal: UInt8, CaseIterable, Sendable {
    case notWriter = 1
    case noRoomForValue = 2
    case noRoomForHistory = 3
    case noRoomForDigest = 4
    case textTooLong = 5
    case changeTooLarge = 6
    case registerNamedTwice = 7
    case clockExhausted = 8

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .notWriter: return "notWriter"
        case .noRoomForValue: return "noRoomForValue"
        case .noRoomForHistory: return "noRoomForHistory"
        case .noRoomForDigest: return "noRoomForDigest"
        case .textTooLong: return "textTooLong"
        case .changeTooLarge: return "changeTooLarge"
        case .registerNamedTwice: return "registerNamedTwice"
        case .clockExhausted: return "clockExhausted"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: RecordRefusal) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> RecordRefusal? {
        RecordRefusal(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "notWriter": self = .notWriter
        case "noRoomForValue": self = .noRoomForValue
        case "noRoomForHistory": self = .noRoomForHistory
        case "noRoomForDigest": self = .noRoomForDigest
        case "textTooLong": self = .textTooLong
        case "changeTooLarge": self = .changeTooLarge
        case "registerNamedTwice": self = .registerNamedTwice
        case "clockExhausted": self = .clockExhausted
        default: return nil
        }
    }
}
