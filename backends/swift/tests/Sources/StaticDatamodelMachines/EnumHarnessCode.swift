// SCE-GENERATED — DO NOT EDIT
// source-hash: 22249655389e33dabe1467cad0f6465e75f1c83f40cd0207380a1fc15324f922
// SCE-MAP: enum_harness_code.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_harness_code` (sce:kind="enum").
public enum EnumHarnessCode: UInt8, CaseIterable, Sendable {
    case notJson = 1
    case notObject = 2
    case tooDeep = 3
    case missing = 4
    case nullNotOptional = 5
    case wrongKind = 6
    case badNumber = 7
    case outOfRange = 8
    case tooLong = 9
    case tooMany = 10
    case unknownVariant = 11
    case notByteText = 12
    case nulInText = 13
    case tooFew = 14
    case notOffered = 15

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .notJson: return "notJson"
        case .notObject: return "notObject"
        case .tooDeep: return "tooDeep"
        case .missing: return "missing"
        case .nullNotOptional: return "nullNotOptional"
        case .wrongKind: return "wrongKind"
        case .badNumber: return "badNumber"
        case .outOfRange: return "outOfRange"
        case .tooLong: return "tooLong"
        case .tooMany: return "tooMany"
        case .unknownVariant: return "unknownVariant"
        case .notByteText: return "notByteText"
        case .nulInText: return "nulInText"
        case .tooFew: return "tooFew"
        case .notOffered: return "notOffered"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumHarnessCode) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumHarnessCode? {
        EnumHarnessCode(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "notJson": self = .notJson
        case "notObject": self = .notObject
        case "tooDeep": self = .tooDeep
        case "missing": self = .missing
        case "nullNotOptional": self = .nullNotOptional
        case "wrongKind": self = .wrongKind
        case "badNumber": self = .badNumber
        case "outOfRange": self = .outOfRange
        case "tooLong": self = .tooLong
        case "tooMany": self = .tooMany
        case "unknownVariant": self = .unknownVariant
        case "notByteText": self = .notByteText
        case "nulInText": self = .nulInText
        case "tooFew": self = .tooFew
        case "notOffered": self = .notOffered
        default: return nil
        }
    }
}
