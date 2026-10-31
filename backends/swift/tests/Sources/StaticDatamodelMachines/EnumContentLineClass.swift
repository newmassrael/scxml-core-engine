// SCE-GENERATED — DO NOT EDIT
// source-hash: 90dfacafb9c3b3dd4ea571704b12a814c54947805373c06b331ad43ea5f220d0
// SCE-MAP: enum_content_line_class.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// An open enum over the document's own carrier (`sce:underlying-type="uint8"`,
// `sce:strict-variants="false"`), a file of the module that names it: a struct that holds
// any number the carrier holds, declared by a variant or not, and names each declared
// variant as a constant of the number the document declares for it. A record, an
// algorithm and a machine do not hold a value of it yet.

/// The enum `enum_content_line_class` (sce:kind="enum").
public struct EnumContentLineClass: RawRepresentable, Hashable, Sendable {
    public let rawValue: UInt8

    /// Holds the number, declared by a variant or not: the set is open.
    public init(rawValue: UInt8) {
        self.rawValue = rawValue
    }

    public static let unclassified = EnumContentLineClass(rawValue: 0)
    public static let restricted = EnumContentLineClass(rawValue: 1)
    public static let secret = EnumContentLineClass(rawValue: 2)

    /// The name the enum document declares for the value, and nil for a number no
    /// variant declares.
    public var declaredName: String? {
        switch rawValue {
        case 0: return "unclassified"
        case 1: return "restricted"
        case 2: return "secret"
        default: return nil
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumContentLineClass) -> UInt8 {
        value.rawValue
    }

    /// The value of `raw`, which is total: no number is undeclared in an open set.
    public static func fromUnderlying(_ raw: UInt8) -> EnumContentLineClass {
        EnumContentLineClass(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "unclassified": self = .unclassified
        case "restricted": self = .restricted
        case "secret": self = .secret
        default: return nil
        }
    }
}
