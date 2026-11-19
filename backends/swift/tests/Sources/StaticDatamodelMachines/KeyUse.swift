// SCE-GENERATED — DO NOT EDIT
// source-hash: 433a8d2b45446434b6b3092ce2ff4baf2620ffa237a0c904bbd5c51b6b6c25ad
// source: sce:std/host/key_use.scxml
// SCE-MAP: key_use.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `key_use` (sce:kind="enum").
public enum KeyUse: UInt8, CaseIterable, Sendable {
    case sign = 0
    case seal = 1
    case mac = 2
    case derive = 3

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .sign: return "sign"
        case .seal: return "seal"
        case .mac: return "mac"
        case .derive: return "derive"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: KeyUse) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> KeyUse? {
        KeyUse(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "sign": self = .sign
        case "seal": self = .seal
        case "mac": self = .mac
        case "derive": self = .derive
        default: return nil
        }
    }
}
