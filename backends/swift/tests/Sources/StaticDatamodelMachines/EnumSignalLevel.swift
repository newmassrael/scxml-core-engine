// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
// SCE-MAP: enum_signal_level.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_signal_level` (sce:kind="enum").
public enum EnumSignalLevel: UInt8, CaseIterable, Sendable {
    case low = 0
    case high = 1
    case critical = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .low: return "low"
        case .high: return "high"
        case .critical: return "critical"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumSignalLevel) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumSignalLevel? {
        EnumSignalLevel(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "low": self = .low
        case "high": self = .high
        case "critical": self = .critical
        default: return nil
        }
    }
}
