// SCE-GENERATED — DO NOT EDIT
// source-hash: c5adbeaab86f399e2a5f61b0df743727628707f2c9fb98227d9c3b11c4e0758a
// source: payment_kind.scxml
// SCE-MAP: payment_kind.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `payment_kind` (sce:kind="enum").
public enum PaymentKind: UInt8, CaseIterable, Sendable {
    case card = 0
    case cash = 1
    case voucher = 2

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .card: return "card"
        case .cash: return "cash"
        case .voucher: return "voucher"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: PaymentKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> PaymentKind? {
        PaymentKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "card": self = .card
        case "cash": self = .cash
        case "voucher": self = .voucher
        default: return nil
        }
    }
}
