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
    public static func resolveDate(_ today: Int64, _ expression: DateExpression) throws -> Int64? {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = expression.kind == DateKind.none
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: Int64? = nil
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.absolute
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(expression.provenAbsolute.year, expression.provenAbsolute.month, expression.provenAbsolute.day) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.monthDay
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(expression.provenMonthDay.day >= 1 && expression.provenMonthDay.day <= Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(2000, expression.provenMonthDay.month) }))
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let dayToday: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                var landing: Int32 = dayToday.year
                try sceFailure.throwIfFailed()
                var landingLast: UInt8 = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month) })
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = expression.provenMonthDay.day <= landingLast
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        let thisYear: Int64 = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(landing, expression.provenMonthDay.month, expression.provenMonthDay.day) })
                        try sceFailure.throwIfFailed()
                        do {
                            let sceCond6 = thisYear >= today
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                do {
                                    let sceReturned: Int64? = thisYear
                                    try sceFailure.throwIfFailed()
                                    return sceReturned
                                }
                            }
                        }
                    }
                }
                landing = Checked.add(sceFailure, landing, 1)
                try sceFailure.throwIfFailed()
                landingLast = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month) })
                try sceFailure.throwIfFailed()
                while true {
                    let sceCond4 = expression.provenMonthDay.day > landingLast
                    try sceFailure.throwIfFailed()
                    if !sceCond4 { break }
                    landing = Checked.add(sceFailure, landing, 1)
                    try sceFailure.throwIfFailed()
                    landingLast = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, expression.provenMonthDay.month) })
                    try sceFailure.throwIfFailed()
                }
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(landing, expression.provenMonthDay.month, expression.provenMonthDay.day) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.relativeDays
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let dayShift: Int64 = Int64(expression.provenRelativeDays.count)
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, dayShift)
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.relativeWeeks
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let weekShift: Int64 = Int64(expression.provenRelativeWeeks.count)
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, Checked.mul(sceFailure, weekShift, 7))
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.relativeMonths
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let monthFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let monthTo: CivilDatePayload = Checked.takeDefault(sceFailure, { try AddMonths.addMonths(monthFrom.year, monthFrom.month, monthFrom.day, expression.provenRelativeMonths.count) })
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(monthTo.year, monthTo.month, monthTo.day) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.relativeYears
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let yearFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let yearTo: Int32 = Checked.add(sceFailure, yearFrom.year, expression.provenRelativeYears.count)
                try sceFailure.throwIfFailed()
                let yearLast: UInt8 = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(yearTo, yearFrom.month) })
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(yearTo, yearFrom.month, yearFrom.day < yearLast ? yearFrom.day : yearLast) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.comingWeekday
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(expression.provenComingWeekday.weekday <= 6)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let comingWanted: Int64 = Int64(expression.provenComingWeekday.weekday)
                try sceFailure.throwIfFailed()
                let comingNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
                try sceFailure.throwIfFailed()
                let comingGap: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, comingWanted, comingNow), 7), 7)
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, comingGap == 0 ? 7 : comingGap)
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.previousWeekday
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(expression.provenPreviousWeekday.weekday <= 6)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let previousWanted: Int64 = Int64(expression.provenPreviousWeekday.weekday)
                try sceFailure.throwIfFailed()
                let previousNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
                try sceFailure.throwIfFailed()
                let previousGap: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, previousNow, previousWanted), 7), 7)
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.sub(sceFailure, today, previousGap == 0 ? 7 : previousGap)
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.dayOfMonthOffset
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(expression.provenDayOfMonthOffset.day >= 1 && expression.provenDayOfMonthOffset.day <= 31)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let offsetFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let offsetTo: CivilDatePayload = Checked.takeDefault(sceFailure, { try AddMonths.addMonths(offsetFrom.year, offsetFrom.month, 1, expression.provenDayOfMonthOffset.count) })
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(offsetTo.year, offsetTo.month, expression.provenDayOfMonthOffset.day) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = expression.kind == DateKind.weekdayOfWeek
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(expression.provenWeekdayOfWeek.weekday <= 6)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let isoWanted: Int64 = Int64(expression.provenWeekdayOfWeek.weekday)
                try sceFailure.throwIfFailed()
                let isoNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
                try sceFailure.throwIfFailed()
                let weeks: Int64 = Int64(expression.provenWeekdayOfWeek.count)
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, weeks, 7), Checked.rem(sceFailure, Checked.add(sceFailure, isoWanted, 6), 7)), Checked.rem(sceFailure, Checked.add(sceFailure, isoNow, 6), 7)))
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        preconditionFailure("resolve_date: every case of its union returned before the end of its body")
    }
}
