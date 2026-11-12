// SCE-GENERATED — DO NOT EDIT
// source-hash: d5a1192f784ff99a5aa95877e35eef81f703dd6fbaf723312b776f25706b4354
// SCE-MAP: resolve_date.scxml:56 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `ResolveDate`, a file of the
// module that calls it: a call is qualified by the enum
// (`ResolveDate.resolveDate(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum ResolveDate {
    public static func resolveDate(_ today: Int64, _ expression: DateExpression) throws(AlgorithmError) -> Int64? {
        if expression.kind == DateKind.none {
            return nil
        }
        if expression.kind == DateKind.absolute {
            return (try DaysFromCivil.daysFromCivil(expression.provenAbsolute.year, expression.provenAbsolute.month, expression.provenAbsolute.day))
        }
        if expression.kind == DateKind.monthDay {
            if !((try Checked.and(expression.provenMonthDay.day >= 1, { expression.provenMonthDay.day <= (try DaysInMonth.daysInMonth(2000, expression.provenMonthDay.month)) }))) {
                throw AlgorithmError.precondition
            }
            let dayToday: CivilDatePayload = (try CivilFromDays.civilFromDays(today))
            var landing: Int32 = dayToday.year
            var landingLast: UInt8 = (try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month))
            if expression.provenMonthDay.day <= landingLast {
                let thisYear: Int64 = (try DaysFromCivil.daysFromCivil(landing, expression.provenMonthDay.month, expression.provenMonthDay.day))
                if thisYear >= today {
                    return thisYear
                }
            }
            landing = (try Checked.add(landing, 1))
            landingLast = (try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month))
            while expression.provenMonthDay.day > landingLast {
                landing = (try Checked.add(landing, 1))
                landingLast = (try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month))
            }
            return (try DaysFromCivil.daysFromCivil(landing, expression.provenMonthDay.month, expression.provenMonthDay.day))
        }
        if expression.kind == DateKind.relativeDays {
            let dayShift: Int64 = Int64(expression.provenRelativeDays.count)
            return (try Checked.add(today, dayShift))
        }
        if expression.kind == DateKind.relativeWeeks {
            let weekShift: Int64 = Int64(expression.provenRelativeWeeks.count)
            return (try Checked.add(today, (try Checked.mul(weekShift, 7))))
        }
        if expression.kind == DateKind.relativeMonths {
            let monthFrom: CivilDatePayload = (try CivilFromDays.civilFromDays(today))
            let monthTo: CivilDatePayload = (try AddMonths.addMonths(monthFrom.year, monthFrom.month, monthFrom.day, expression.provenRelativeMonths.count))
            return (try DaysFromCivil.daysFromCivil(monthTo.year, monthTo.month, monthTo.day))
        }
        if expression.kind == DateKind.relativeYears {
            let yearFrom: CivilDatePayload = (try CivilFromDays.civilFromDays(today))
            let yearTo: Int32 = (try Checked.add(yearFrom.year, expression.provenRelativeYears.count))
            let yearLast: UInt8 = (try DaysInMonth.daysInMonth(yearTo, yearFrom.month))
            return (try DaysFromCivil.daysFromCivil(yearTo, yearFrom.month, yearFrom.day < yearLast ? yearFrom.day : yearLast))
        }
        if expression.kind == DateKind.comingWeekday {
            if !(expression.provenComingWeekday.weekday <= 6) {
                throw AlgorithmError.precondition
            }
            let comingWanted: Int64 = Int64(expression.provenComingWeekday.weekday)
            let comingNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
            let comingGap: Int64 = (try Checked.rem((try Checked.add((try Checked.sub(comingWanted, comingNow)), 7)), 7))
            return (try Checked.add(today, comingGap == 0 ? 7 : comingGap))
        }
        if expression.kind == DateKind.previousWeekday {
            if !(expression.provenPreviousWeekday.weekday <= 6) {
                throw AlgorithmError.precondition
            }
            let previousWanted: Int64 = Int64(expression.provenPreviousWeekday.weekday)
            let previousNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
            let previousGap: Int64 = (try Checked.rem((try Checked.add((try Checked.sub(previousNow, previousWanted)), 7)), 7))
            return (try Checked.sub(today, previousGap == 0 ? 7 : previousGap))
        }
        if expression.kind == DateKind.dayOfMonthOffset {
            if !(expression.provenDayOfMonthOffset.day >= 1 && expression.provenDayOfMonthOffset.day <= 31) {
                throw AlgorithmError.precondition
            }
            let offsetFrom: CivilDatePayload = (try CivilFromDays.civilFromDays(today))
            let offsetTo: CivilDatePayload = (try AddMonths.addMonths(offsetFrom.year, offsetFrom.month, 1, expression.provenDayOfMonthOffset.count))
            return (try DaysFromCivil.daysFromCivil(offsetTo.year, offsetTo.month, expression.provenDayOfMonthOffset.day))
        }
        if expression.kind == DateKind.weekdayOfWeek {
            if !(expression.provenWeekdayOfWeek.weekday <= 6) {
                throw AlgorithmError.precondition
            }
            let isoWanted: Int64 = Int64(expression.provenWeekdayOfWeek.weekday)
            let isoNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
            let weeks: Int64 = Int64(expression.provenWeekdayOfWeek.count)
            return (try Checked.add(today, (try Checked.sub((try Checked.add((try Checked.mul(weeks, 7)), (try Checked.rem((try Checked.add(isoWanted, 6)), 7)))), (try Checked.rem((try Checked.add(isoNow, 6)), 7))))))
        }
        preconditionFailure("resolve_date: every case of its union returned before the end of its body")
    }
}
