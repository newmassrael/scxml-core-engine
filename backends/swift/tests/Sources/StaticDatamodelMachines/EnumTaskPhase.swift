// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: enum_task_phase.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_task_phase` (sce:kind="enum").
public enum EnumTaskPhase: UInt8, CaseIterable, Sendable {
    case todo = 3
    case inReview = 6
    case shipped = 9

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .todo: return "todo"
        case .inReview: return "in_review"
        case .shipped: return "shipped"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumTaskPhase) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumTaskPhase? {
        EnumTaskPhase(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "todo": self = .todo
        case "in_review": self = .inReview
        case "shipped": self = .shipped
        default: return nil
        }
    }
}
