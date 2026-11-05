// SCE-GENERATED — DO NOT EDIT
// source-hash: b2575d48129e1acf0f70c09ad24f44255a7a62f71444c27101300757ede12222
// SCE-MAP: resolve_date.scxml:52 :: _forge_body

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
    public static func resolveDate(_ today: Int64, _ kind: DateKind, _ year: Int32?, _ month: UInt8?, _ day: UInt8?, _ weekday: UInt8?, _ count: Int32?) throws -> Int64? {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = kind == DateKind.none
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
            let sceCond2 = kind == DateKind.absolute
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(year != nil && month != nil && day != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(SceOptional.proven(year), SceOptional.proven(month), SceOptional.proven(day)) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = kind == DateKind.monthDay
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(month != nil && day != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                do {
                    let sceCond4 = !(SceOptional.proven(day) >= 1 && SceOptional.proven(day) <= Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(2000, SceOptional.proven(month)) }))
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let dayToday: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                var landing: Int32 = dayToday.year
                try sceFailure.throwIfFailed()
                var landingLast: UInt8 = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, SceOptional.proven(month)) })
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = SceOptional.proven(day) <= landingLast
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        let thisYear: Int64 = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(landing, SceOptional.proven(month), SceOptional.proven(day)) })
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
                landingLast = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, SceOptional.proven(month)) })
                try sceFailure.throwIfFailed()
                while true {
                    let sceCond4 = SceOptional.proven(day) > landingLast
                    try sceFailure.throwIfFailed()
                    if !sceCond4 { break }
                    landing = Checked.add(sceFailure, landing, 1)
                    try sceFailure.throwIfFailed()
                    landingLast = Checked.take(sceFailure, fallback: 0, { try DaysInMonth.daysInMonth(landing, SceOptional.proven(month)) })
                    try sceFailure.throwIfFailed()
                }
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(landing, SceOptional.proven(month), SceOptional.proven(day)) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = kind == DateKind.relativeDays
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(count != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let dayShift: Int64 = Int64(SceOptional.proven(count))
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, dayShift)
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = kind == DateKind.relativeWeeks
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(count != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let weekShift: Int64 = Int64(SceOptional.proven(count))
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.add(sceFailure, today, Checked.mul(sceFailure, weekShift, 7))
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = kind == DateKind.relativeMonths
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(count != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let monthFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let monthTo: CivilDatePayload = Checked.takeDefault(sceFailure, { try AddMonths.addMonths(monthFrom.year, monthFrom.month, monthFrom.day, SceOptional.proven(count)) })
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(monthTo.year, monthTo.month, monthTo.day) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = kind == DateKind.relativeYears
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(count != nil)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let yearFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let yearTo: Int32 = Checked.add(sceFailure, yearFrom.year, SceOptional.proven(count))
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
            let sceCond2 = kind == DateKind.comingWeekday
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(weekday != nil && SceOptional.proven(weekday) <= 6)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let comingWanted: Int64 = Int64(SceOptional.proven(weekday))
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
            let sceCond2 = kind == DateKind.previousWeekday
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(weekday != nil && SceOptional.proven(weekday) <= 6)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let previousWanted: Int64 = Int64(SceOptional.proven(weekday))
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
            let sceCond2 = kind == DateKind.dayOfMonthOffset
            try sceFailure.throwIfFailed()
            if sceCond2 {
                do {
                    let sceCond4 = !(day != nil && count != nil && SceOptional.proven(day) >= 1 && SceOptional.proven(day) <= 31)
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        throw AlgorithmError.precondition
                    }
                }
                let offsetFrom: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(today) })
                try sceFailure.throwIfFailed()
                let offsetTo: CivilDatePayload = Checked.takeDefault(sceFailure, { try AddMonths.addMonths(offsetFrom.year, offsetFrom.month, 1, SceOptional.proven(count)) })
                try sceFailure.throwIfFailed()
                do {
                    let sceReturned: Int64? = Checked.take(sceFailure, fallback: 0, { try DaysFromCivil.daysFromCivil(offsetTo.year, offsetTo.month, SceOptional.proven(day)) })
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        do {
            let sceCond2 = !(kind == DateKind.weekdayOfWeek)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        do {
            let sceCond2 = !(weekday != nil && count != nil && SceOptional.proven(weekday) <= 6)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        let isoWanted: Int64 = Int64(SceOptional.proven(weekday))
        try sceFailure.throwIfFailed()
        let isoNow: Int64 = Int64(WeekdayFromDays.weekdayFromDays(today))
        try sceFailure.throwIfFailed()
        let weeks: Int64 = Int64(SceOptional.proven(count))
        try sceFailure.throwIfFailed()
        do {
            let sceReturned: Int64? = Checked.add(sceFailure, today, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, weeks, 7), Checked.rem(sceFailure, Checked.add(sceFailure, isoWanted, 6), 7)), Checked.rem(sceFailure, Checked.add(sceFailure, isoNow, 6), 7)))
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
