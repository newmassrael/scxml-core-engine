// SCE-GENERATED — DO NOT EDIT
// source-hash: 62067cbe5198ad923571ee5243de08cf004a9a483bf54ae3e6db2a14e8ad77a7
// SCE-MAP: enum_uds_nrc.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// An open enum over the document's own carrier (`sce:underlying-type="uint8"`,
// `sce:strict-variants="false"`), a file of the module that names it: a struct that holds
// any number the carrier holds, declared by a variant or not, and names each declared
// variant as a constant of the number the document declares for it. A record, an
// algorithm and a machine do not hold a value of it yet.

/// The enum `enum_uds_nrc` (sce:kind="enum").
public struct EnumUdsNrc: RawRepresentable, Hashable, Sendable {
    public let rawValue: UInt8

    /// Holds the number, declared by a variant or not: the set is open.
    public init(rawValue: UInt8) {
        self.rawValue = rawValue
    }

    public static let generalReject = EnumUdsNrc(rawValue: 16)
    public static let serviceNotSupported = EnumUdsNrc(rawValue: 17)
    public static let conditionsNotCorrect = EnumUdsNrc(rawValue: 34)
    public static let requestOutOfRange = EnumUdsNrc(rawValue: 49)
    public static let responsePending = EnumUdsNrc(rawValue: 120)

    /// The name the enum document declares for the value, and nil for a number no
    /// variant declares.
    public var declaredName: String? {
        switch rawValue {
        case 16: return "generalReject"
        case 17: return "serviceNotSupported"
        case 34: return "conditionsNotCorrect"
        case 49: return "requestOutOfRange"
        case 120: return "responsePending"
        default: return nil
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumUdsNrc) -> UInt8 {
        value.rawValue
    }

    /// The value of `raw`, which is total: no number is undeclared in an open set.
    public static func fromUnderlying(_ raw: UInt8) -> EnumUdsNrc {
        EnumUdsNrc(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "generalReject": self = .generalReject
        case "serviceNotSupported": self = .serviceNotSupported
        case "conditionsNotCorrect": self = .conditionsNotCorrect
        case "requestOutOfRange": self = .requestOutOfRange
        case "responsePending": self = .responsePending
        default: return nil
        }
    }
}
