// SCE-GENERATED — DO NOT EDIT
// source-hash: d6639b829cb06d9b5a5c1de4a7c3840f285873e4f32155ae3a9c2c7c6d5b22c8
// source: enum_take_kind.scxml
// SCE-MAP: enum_take_kind.scxml:8 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_take_kind` (sce:kind="enum").
public enum EnumTakeKind: UInt8, CaseIterable, Sendable {
    case taken = 0
    case duplicate = 1
    case noRoom = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .taken: return "taken"
        case .duplicate: return "duplicate"
        case .noRoom: return "no_room"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumTakeKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumTakeKind? {
        EnumTakeKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "taken": self = .taken
        case "duplicate": self = .duplicate
        case "no_room": self = .noRoom
        default: return nil
        }
    }
}
