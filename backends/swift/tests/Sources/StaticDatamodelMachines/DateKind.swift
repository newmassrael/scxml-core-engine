// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c71031c0d27e4d67ce7e4f29c8b0629804b2b22ba0439698c8a940297a0f1bd
// SCE-MAP: date_kind.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="enum")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A closed enum over the document's own carrier (`sce:underlying-type="uint8"`),
// a file of the module that names it: each case holds the number the document
// declares for it, which is not the variant's position. An algorithm and an event
// schema of the module hold a value of it by this type, and a `sce-static` machine
// holds the same enum under a name of its own, written by the same function.

/// The enum `date_kind` (sce:kind="enum").
public enum DateKind: UInt8, CaseIterable, Sendable {
    case none = 0
    case absolute = 1
    case monthDay = 2
    case relativeDays = 3
    case relativeWeeks = 4
    case relativeMonths = 5
    case relativeYears = 6
    case comingWeekday = 7
    case previousWeekday = 8
    case weekdayOfWeek = 9

    /// The name the enum document declares for the value.
    public var declaredName: String {
        switch self {
        case .none: return "none"
        case .absolute: return "absolute"
        case .monthDay: return "monthDay"
        case .relativeDays: return "relativeDays"
        case .relativeWeeks: return "relativeWeeks"
        case .relativeMonths: return "relativeMonths"
        case .relativeYears: return "relativeYears"
        case .comingWeekday: return "comingWeekday"
        case .previousWeekday: return "previousWeekday"
        case .weekdayOfWeek: return "weekdayOfWeek"
        }
    }

    /// The number the enum document declares for the value.
    public static func toUnderlying(_ value: DateKind) -> UInt8 {
        value.rawValue
    }

    /// The variant that declares `raw`, and nil for a number no variant declares.
    public static func fromUnderlying(_ raw: UInt8) -> DateKind? {
        DateKind(rawValue: raw)
    }

    /// The variant the enum document gives `name`, and nil for a name it does not
    /// declare.
    public init?(declaredName: String) {
        switch declaredName {
        case "none": self = .none
        case "absolute": self = .absolute
        case "monthDay": self = .monthDay
        case "relativeDays": self = .relativeDays
        case "relativeWeeks": self = .relativeWeeks
        case "relativeMonths": self = .relativeMonths
        case "relativeYears": self = .relativeYears
        case "comingWeekday": self = .comingWeekday
        case "previousWeekday": self = .previousWeekday
        case "weekdayOfWeek": self = .weekdayOfWeek
        default: return nil
        }
    }
}
