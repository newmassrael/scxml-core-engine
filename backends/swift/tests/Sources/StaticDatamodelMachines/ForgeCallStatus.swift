// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: forge_call_status.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `forge_call_status` (sce:kind="enum").
public enum ForgeCallStatus: UInt8, CaseIterable, Sendable {
    case open = 1
    case held = 2
    case done = 3

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .open: return "open"
        case .held: return "held"
        case .done: return "done"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: ForgeCallStatus) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> ForgeCallStatus? {
        ForgeCallStatus(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "open": self = .open
        case "held": self = .held
        case "done": self = .done
        default: return nil
        }
    }
}
