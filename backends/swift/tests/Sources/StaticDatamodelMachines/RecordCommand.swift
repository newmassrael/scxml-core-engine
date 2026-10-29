// SCE-GENERATED — DO NOT EDIT
// source-hash: 634c022b7eb332297dcb6115177eef8cafdb472ead988f0bdc39838cb7f8d8ae
// SCE-MAP: record_command.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `record_command` (sce:kind="enum").
public enum RecordCommand: UInt8, CaseIterable, Sendable {
    case edit = 0
    case clear = 1
    case retire = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .edit: return "edit"
        case .clear: return "clear"
        case .retire: return "retire"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: RecordCommand) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> RecordCommand? {
        RecordCommand(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "edit": self = .edit
        case "clear": self = .clear
        case "retire": self = .retire
        default: return nil
        }
    }
}
