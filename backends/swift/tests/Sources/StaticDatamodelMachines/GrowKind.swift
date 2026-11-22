// SCE-GENERATED — DO NOT EDIT
// source-hash: be25ee8255a32c7ec240ba31f48b3e3888b421310628dd926f72cb5a0da29c66
// source: grow_kind.scxml
// SCE-MAP: grow_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `grow_kind` (sce:kind="enum").
public enum GrowKind: UInt8, CaseIterable, Sendable {
    case single = 1
    case none = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .single: return "single"
        case .none: return "none"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: GrowKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> GrowKind? {
        GrowKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "single": self = .single
        case "none": self = .none
        default: return nil
        }
    }
}
