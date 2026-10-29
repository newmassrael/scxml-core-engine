// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b431984688d288798089f16432b8bdf8a4e08b28da85ae7117e434e9ad27889
// SCE-MAP: enum_ast_export_min.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_ast_export_min` (sce:kind="enum").
public enum EnumAstExportMin: UInt8, CaseIterable, Sendable {
    case a = 0
    case b = 1

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .a: return "a"
        case .b: return "b"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumAstExportMin) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumAstExportMin? {
        EnumAstExportMin(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "a": self = .a
        case "b": self = .b
        default: return nil
        }
    }
}
