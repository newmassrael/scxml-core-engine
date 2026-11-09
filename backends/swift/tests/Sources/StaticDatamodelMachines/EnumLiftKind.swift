// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e
// SCE-MAP: enum_lift_kind.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_lift_kind` (sce:kind="enum").
public enum EnumLiftKind: UInt8, CaseIterable, Sendable {
    case meeting = 0
    case task = 1

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .meeting: return "meeting"
        case .task: return "task"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumLiftKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumLiftKind? {
        EnumLiftKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "meeting": self = .meeting
        case "task": self = .task
        default: return nil
        }
    }
}
