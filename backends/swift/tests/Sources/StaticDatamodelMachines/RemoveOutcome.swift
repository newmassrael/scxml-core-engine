// SCE-GENERATED — DO NOT EDIT
// source-hash: 9a6be21e7f94af3da6eb99a825e9ce20dc69051ff4e97c97f42ecfd8241594f7
// SCE-MAP: remove_outcome.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `remove_outcome` (sce:kind="enum").
public enum RemoveOutcome: UInt8, CaseIterable, Sendable {
    case removed = 0
    case absent = 1

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .removed: return "removed"
        case .absent: return "absent"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: RemoveOutcome) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> RemoveOutcome? {
        RemoveOutcome(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "removed": self = .removed
        case "absent": self = .absent
        default: return nil
        }
    }
}
