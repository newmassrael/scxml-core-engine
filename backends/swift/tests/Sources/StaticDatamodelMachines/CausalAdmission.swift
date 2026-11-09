// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: causal_admission.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `causal_admission` (sce:kind="enum").
public enum CausalAdmission: UInt8, CaseIterable, Sendable {
    case drop = 0
    case deliver = 1
    case hold = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .drop: return "drop"
        case .deliver: return "deliver"
        case .hold: return "hold"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: CausalAdmission) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> CausalAdmission? {
        CausalAdmission(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "drop": self = .drop
        case "deliver": self = .deliver
        case "hold": self = .hold
        default: return nil
        }
    }
}
