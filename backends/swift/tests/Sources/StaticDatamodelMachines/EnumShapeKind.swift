// SCE-GENERATED — DO NOT EDIT
// source-hash: ad4bedcb756956ce54bdaf3d75bb21306a16abe1845fd50a6c67924ec38392a8
// SCE-MAP: enum_shape_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_shape_kind` (sce:kind="enum").
public enum EnumShapeKind: UInt8, CaseIterable, Sendable {
    case dot = 0
    case circle = 1
    case disc = 2
    case rect = 3
    case label = 4
    case path = 5

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .dot: return "dot"
        case .circle: return "circle"
        case .disc: return "disc"
        case .rect: return "rect"
        case .label: return "label"
        case .path: return "path"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumShapeKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumShapeKind? {
        EnumShapeKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "dot": self = .dot
        case "circle": self = .circle
        case "disc": self = .disc
        case "rect": self = .rect
        case "label": self = .label
        case "path": self = .path
        default: return nil
        }
    }
}
