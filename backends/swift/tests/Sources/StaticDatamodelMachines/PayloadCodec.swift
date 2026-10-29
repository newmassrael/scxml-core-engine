// SCE-GENERATED — DO NOT EDIT
// source-hash: 9b4cfcaed7ada488617b52a841abc3e598466613b60c7e1131e1820f7a244b47
// SCE-MAP: payload_codec.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `payload_codec` (sce:kind="enum").
public enum PayloadCodec: UInt8, CaseIterable, Sendable {
    case none = 0
    case json = 1
    case cbor = 2
    case typed = 3
    case raw = 4

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .none: return "none"
        case .json: return "json"
        case .cbor: return "cbor"
        case .typed: return "typed"
        case .raw: return "raw"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: PayloadCodec) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> PayloadCodec? {
        PayloadCodec(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "none": self = .none
        case "json": self = .json
        case "cbor": self = .cbor
        case "typed": self = .typed
        case "raw": self = .raw
        default: return nil
        }
    }
}
