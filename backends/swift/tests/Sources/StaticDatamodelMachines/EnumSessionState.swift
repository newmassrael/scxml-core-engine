// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ba579cdc10cbe049a3766817dcc6397d58e8716381e63be0e21ee74788ac719
// SCE-MAP: enum_session_state.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `enum_session_state` (sce:kind="enum").
public enum EnumSessionState: UInt8, CaseIterable, Sendable {
    case defaultSession = 1
    case programming = 2
    case extendedDiagnostic = 3
    case safetySystem = 64

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .defaultSession: return "defaultSession"
        case .programming: return "programming"
        case .extendedDiagnostic: return "extendedDiagnostic"
        case .safetySystem: return "safetySystem"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: EnumSessionState) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> EnumSessionState? {
        EnumSessionState(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "defaultSession": self = .defaultSession
        case "programming": self = .programming
        case "extendedDiagnostic": self = .extendedDiagnostic
        case "safetySystem": self = .safetySystem
        default: return nil
        }
    }
}
