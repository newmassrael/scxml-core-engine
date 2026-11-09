// SCE-GENERATED — DO NOT EDIT
// source-hash: 81cc59c6b8f46736a80b4b69ee489274d9b0ae32efae821805ba44c3bac3af2c
// SCE-MAP: record_outcome.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `record_outcome` (sce:kind="enum").
public enum RecordOutcome: UInt8, CaseIterable, Sendable {
    case none = 0
    case accepted = 1
    case held = 2
    case waiting = 3
    case refused = 4

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .none: return "none"
        case .accepted: return "accepted"
        case .held: return "held"
        case .waiting: return "waiting"
        case .refused: return "refused"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: RecordOutcome) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> RecordOutcome? {
        RecordOutcome(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "none": self = .none
        case "accepted": self = .accepted
        case "held": self = .held
        case "waiting": self = .waiting
        case "refused": self = .refused
        default: return nil
        }
    }
}
