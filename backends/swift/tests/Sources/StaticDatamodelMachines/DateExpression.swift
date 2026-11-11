// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: date_expression.scxml:33 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="union")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

/// A value of `date_expression`: exactly one case, told apart by its tag `kind`, a value of
/// ``DateKind``. Two values are equal when they are in one case and their records are equal.
public enum DateExpression: Equatable, Sendable {
    /// The revision of the union: its tag enum's, `date_kind`'s (a union declares none of its own).
    public static let revision: UInt32 = 2

    /// The most bytes a value's JSON text takes: its largest case, every list full, every text at its
    /// bound, every number at its longest spelling.
    public static let dataMaxLen: UInt64 = 60

    /// The id of the union's schema, `urn:sce:schema:<document>:<revision>`: the `$id` of `jsonSchema`.
    public static let schemaID: String = "urn:sce:schema:date_expression:2"

    /// A value as a JSON Schema (2020-12): the `oneOf` of the cases, each its tag `kind` beside the
    /// fields of its record, every bound as this build resolved it.
    public static let jsonSchema: String = "{\"$schema\":\"https://json-schema.org/draft/2020-12/schema\",\"$id\":\"urn:sce:schema:date_expression:2\",\"x-sce-source-hash\":\"d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354\",\"description\":\"x-sce-max-bytes bounds a text in UTF-8 bytes; maxLength counts characters\",\"oneOf\":[{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"none\"}},\"required\":[\"kind\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"absolute\"},\"year\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647},\"month\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kind\",\"year\",\"month\",\"day\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"monthDay\"},\"month\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kind\",\"month\",\"day\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"relativeDays\"},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"count\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"relativeWeeks\"},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"count\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"relativeMonths\"},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"count\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"relativeYears\"},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"count\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"comingWeekday\"},\"weekday\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kind\",\"weekday\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"previousWeekday\"},\"weekday\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}},\"required\":[\"kind\",\"weekday\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"weekdayOfWeek\"},\"weekday\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"weekday\",\"count\"],\"additionalProperties\":false},{\"type\":\"object\",\"properties\":{\"kind\":{\"const\":\"dayOfMonthOffset\"},\"day\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"count\":{\"type\":\"integer\",\"minimum\":-2147483648,\"maximum\":2147483647}},\"required\":[\"kind\",\"day\",\"count\"],\"additionalProperties\":false}]}"

    /// The case `none`, its tag alone.
    case none

    /// The case `absolute`, holding its record.
    case absolute(CivilDatePayload)

    /// The case `monthDay`, holding its record.
    case monthDay(MonthDayPayload)

    /// The case `relativeDays`, holding its record.
    case relativeDays(DateShiftPayload)

    /// The case `relativeWeeks`, holding its record.
    case relativeWeeks(DateShiftPayload)

    /// The case `relativeMonths`, holding its record.
    case relativeMonths(DateShiftPayload)

    /// The case `relativeYears`, holding its record.
    case relativeYears(DateShiftPayload)

    /// The case `comingWeekday`, holding its record.
    case comingWeekday(DateWeekdayPayload)

    /// The case `previousWeekday`, holding its record.
    case previousWeekday(DateWeekdayPayload)

    /// The case `weekdayOfWeek`, holding its record.
    case weekdayOfWeek(WeekWeekdayPayload)

    /// The case `dayOfMonthOffset`, holding its record.
    case dayOfMonthOffset(MonthOffsetDayPayload)

    /// The case this value is in: the tag its text writes under `kind`.
    public var kind: DateKind {
        switch self {
        case .none:
            return DateKind.none
        case .absolute:
            return DateKind.absolute
        case .monthDay:
            return DateKind.monthDay
        case .relativeDays:
            return DateKind.relativeDays
        case .relativeWeeks:
            return DateKind.relativeWeeks
        case .relativeMonths:
            return DateKind.relativeMonths
        case .relativeYears:
            return DateKind.relativeYears
        case .comingWeekday:
            return DateKind.comingWeekday
        case .previousWeekday:
            return DateKind.previousWeekday
        case .weekdayOfWeek:
            return DateKind.weekdayOfWeek
        case .dayOfMonthOffset:
            return DateKind.dayOfMonthOffset
        }
    }

    /// The record of the case `absolute`, read where a test of the tag proved the value is in it.
    public var provenAbsolute: CivilDatePayload {
        guard case .absolute(let value) = self else {
            preconditionFailure("a date_expression read as absolute is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `monthDay`, read where a test of the tag proved the value is in it.
    public var provenMonthDay: MonthDayPayload {
        guard case .monthDay(let value) = self else {
            preconditionFailure("a date_expression read as monthDay is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `relativeDays`, read where a test of the tag proved the value is in it.
    public var provenRelativeDays: DateShiftPayload {
        guard case .relativeDays(let value) = self else {
            preconditionFailure("a date_expression read as relativeDays is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `relativeWeeks`, read where a test of the tag proved the value is in it.
    public var provenRelativeWeeks: DateShiftPayload {
        guard case .relativeWeeks(let value) = self else {
            preconditionFailure("a date_expression read as relativeWeeks is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `relativeMonths`, read where a test of the tag proved the value is in it.
    public var provenRelativeMonths: DateShiftPayload {
        guard case .relativeMonths(let value) = self else {
            preconditionFailure("a date_expression read as relativeMonths is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `relativeYears`, read where a test of the tag proved the value is in it.
    public var provenRelativeYears: DateShiftPayload {
        guard case .relativeYears(let value) = self else {
            preconditionFailure("a date_expression read as relativeYears is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `comingWeekday`, read where a test of the tag proved the value is in it.
    public var provenComingWeekday: DateWeekdayPayload {
        guard case .comingWeekday(let value) = self else {
            preconditionFailure("a date_expression read as comingWeekday is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `previousWeekday`, read where a test of the tag proved the value is in it.
    public var provenPreviousWeekday: DateWeekdayPayload {
        guard case .previousWeekday(let value) = self else {
            preconditionFailure("a date_expression read as previousWeekday is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `weekdayOfWeek`, read where a test of the tag proved the value is in it.
    public var provenWeekdayOfWeek: WeekWeekdayPayload {
        guard case .weekdayOfWeek(let value) = self else {
            preconditionFailure("a date_expression read as weekdayOfWeek is in \(kind.declaredName)")
        }
        return value
    }

    /// The record of the case `dayOfMonthOffset`, read where a test of the tag proved the value is in it.
    public var provenDayOfMonthOffset: MonthOffsetDayPayload {
        guard case .dayOfMonthOffset(let value) = self else {
            preconditionFailure("a date_expression read as dayOfMonthOffset is in \(kind.declaredName)")
        }
        return value
    }
}
