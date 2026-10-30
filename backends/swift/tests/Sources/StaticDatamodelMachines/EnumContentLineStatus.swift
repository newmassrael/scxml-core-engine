// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: enum_content_line_status.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_content_line_status` (sce:kind="enum").
public enum EnumContentLineStatus: UInt8, CaseIterable, Sendable {
    case needsAction = 1
    case inProcess = 2
    case completed = 4
    case cancelled = 8

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .needsAction: return "needsAction"
        case .inProcess: return "inProcess"
        case .completed: return "completed"
        case .cancelled: return "cancelled"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumContentLineStatus) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumContentLineStatus? {
        EnumContentLineStatus(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "needsAction": self = .needsAction
        case "inProcess": self = .inProcess
        case "completed": self = .completed
        case "cancelled": self = .cancelled
        default: return nil
        }
    }
}
