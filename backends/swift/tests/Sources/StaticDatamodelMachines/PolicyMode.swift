// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b431984688d288798089f16432b8bdf8a4e08b28da85ae7117e434e9ad27889
// SCE-MAP: policy_mode.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `policy_mode` (sce:kind="enum").
public enum PolicyMode: UInt8, CaseIterable, Sendable {
    case relaxed = 0
    case strict = 1

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .relaxed: return "relaxed"
        case .strict: return "strict"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: PolicyMode) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> PolicyMode? {
        PolicyMode(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "relaxed": self = .relaxed
        case "strict": self = .strict
        default: return nil
        }
    }
}
