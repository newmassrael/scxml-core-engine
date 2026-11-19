// SCE-GENERATED — DO NOT EDIT
// source-hash: ea7e545ce0c93d7b1b4b84ee4bba748c0d77612c47d1cf94108da91e2a9f1bce
// source: sce:std/collection/put_outcome.scxml
// SCE-MAP: put_outcome.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `put_outcome` (sce:kind="enum").
public enum PutOutcome: UInt8, CaseIterable, Sendable {
    case inserted = 0
    case replaced = 1
    case full = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .inserted: return "inserted"
        case .replaced: return "replaced"
        case .full: return "full"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: PutOutcome) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> PutOutcome? {
        PutOutcome(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "inserted": self = .inserted
        case "replaced": self = .replaced
        case "full": self = .full
        default: return nil
        }
    }
}
