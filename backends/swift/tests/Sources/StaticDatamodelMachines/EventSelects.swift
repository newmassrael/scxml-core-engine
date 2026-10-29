// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: event_selects.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventSelects`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventSelects.eventSelects(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum EventSelects {
    public static func eventSelects(_ s: EventSchedulePayload, _ d: Int64, _ startDay: Int64) throws -> Bool {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        let freq: Int64 = s.freq
        try sceFailure.throwIfFailed()
        var ok: Bool = true
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = freq != 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let startWd: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.rem(sceFailure, startDay, 7), 11), 7)
                try sceFailure.throwIfFailed()
                let z0: Int64 = Checked.add(sceFailure, startDay, 719468)
                try sceFailure.throwIfFailed()
                let era0: Int64 = Checked.div(sceFailure, z0, 146097)
                try sceFailure.throwIfFailed()
                let doe0: Int64 = Checked.sub(sceFailure, z0, Checked.mul(sceFailure, era0, 146097))
                try sceFailure.throwIfFailed()
                let yoe0: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, doe0, Checked.div(sceFailure, doe0, 1460)), Checked.div(sceFailure, doe0, 36524)), Checked.div(sceFailure, doe0, 146096)), 365)
                try sceFailure.throwIfFailed()
                let doy0: Int64 = Checked.sub(sceFailure, doe0, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 365, yoe0), Checked.div(sceFailure, yoe0, 4)), Checked.div(sceFailure, yoe0, 100)))
                try sceFailure.throwIfFailed()
                let mp0: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 5, doy0), 2), 153)
                try sceFailure.throwIfFailed()
                let dom0: Int64 = Checked.add(sceFailure, Checked.sub(sceFailure, doy0, Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 153, mp0), 2), 5)), 1)
                try sceFailure.throwIfFailed()
                let m0: Int64 = mp0 < 10 ? Checked.add(sceFailure, mp0, 3) : Checked.sub(sceFailure, mp0, 9)
                try sceFailure.throwIfFailed()
                let dz: Int64 = Checked.add(sceFailure, d, 719468)
                try sceFailure.throwIfFailed()
                let dera: Int64 = Checked.div(sceFailure, dz, 146097)
                try sceFailure.throwIfFailed()
                let ddoe: Int64 = Checked.sub(sceFailure, dz, Checked.mul(sceFailure, dera, 146097))
                try sceFailure.throwIfFailed()
                let dyoe: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, ddoe, Checked.div(sceFailure, ddoe, 1460)), Checked.div(sceFailure, ddoe, 36524)), Checked.div(sceFailure, ddoe, 146096)), 365)
                try sceFailure.throwIfFailed()
                let ddoy: Int64 = Checked.sub(sceFailure, ddoe, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 365, dyoe), Checked.div(sceFailure, dyoe, 4)), Checked.div(sceFailure, dyoe, 100)))
                try sceFailure.throwIfFailed()
                let dmp: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 5, ddoy), 2), 153)
                try sceFailure.throwIfFailed()
                let dom: Int64 = Checked.add(sceFailure, Checked.sub(sceFailure, ddoy, Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 153, dmp), 2), 5)), 1)
                try sceFailure.throwIfFailed()
                let mon: Int64 = dmp < 10 ? Checked.add(sceFailure, dmp, 3) : Checked.sub(sceFailure, dmp, 9)
                try sceFailure.throwIfFailed()
                let yr: Int64 = mon <= 2 ? Checked.add(sceFailure, Checked.add(sceFailure, dyoe, Checked.mul(sceFailure, dera, 400)), 1) : Checked.add(sceFailure, dyoe, Checked.mul(sceFailure, dera, 400))
                try sceFailure.throwIfFailed()
                let leap: Bool = Checked.rem(sceFailure, yr, 4) == 0 && Checked.rem(sceFailure, yr, 100) != 0 || Checked.rem(sceFailure, yr, 400) == 0
                try sceFailure.throwIfFailed()
                let last: Int64 = mon == 2 ? (leap ? 29 : 28) : (mon == 4 || mon == 6 || mon == 9 || mon == 11 ? 30 : 31)
                try sceFailure.throwIfFailed()
                let ylen: Int64 = leap ? 366 : 365
                try sceFailure.throwIfFailed()
                let wd: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.rem(sceFailure, d, 7), 11), 7)
                try sceFailure.throwIfFailed()
                let yday: Int64 = Checked.add(sceFailure, Checked.add(sceFailure, Checked.div(sceFailure, Checked.sub(sceFailure, Checked.mul(sceFailure, 367, mon), 362), 12), mon <= 2 ? 0 : (leap ? -1 : -2)), dom)
                try sceFailure.throwIfFailed()
                let hasMonth: Bool = s.bymonth != 0
                try sceFailure.throwIfFailed()
                let hasMD: Bool = (s.mdPos | s.mdNeg) != 0
                try sceFailure.throwIfFailed()
                let hasDay: Bool = ((((((((((((((s.days | s.op0) | s.op1) | s.op2) | s.op3) | s.op4) | s.op5) | s.op6) | s.on0) | s.on1) | s.on2) | s.on3) | s.on4) | s.on5) | s.on6) != 0
                try sceFailure.throwIfFailed()
                let hasWN: Bool = (s.wnPos | s.wnNeg) != 0
                try sceFailure.throwIfFailed()
                let hasYD: Bool = (((((((((((s.yd0 | s.yd1) | s.yd2) | s.yd3) | s.yd4) | s.yd5) | s.ydn0) | s.ydn1) | s.ydn2) | s.ydn3) | s.ydn4) | s.ydn5) != 0
                try sceFailure.throwIfFailed()
                let mdMatch: Bool = ((s.mdPos >> dom) & 1) == 1 || ((s.mdNeg >> Checked.add(sceFailure, Checked.sub(sceFailure, last, dom), 1)) & 1) == 1
                try sceFailure.throwIfFailed()
                let plainDay: Bool = ((s.days >> wd) & 1) == 1
                try sceFailure.throwIfFailed()
                let inMonth: Bool = freq == 3 || hasMonth
                try sceFailure.throwIfFailed()
                let ordPos: Int64 = inMonth ? Checked.add(sceFailure, Checked.div(sceFailure, Checked.sub(sceFailure, dom, 1), 7), 1) : Checked.add(sceFailure, Checked.div(sceFailure, Checked.sub(sceFailure, yday, 1), 7), 1)
                try sceFailure.throwIfFailed()
                let ordNeg: Int64 = inMonth ? Checked.add(sceFailure, Checked.div(sceFailure, Checked.sub(sceFailure, last, dom), 7), 1) : Checked.add(sceFailure, Checked.div(sceFailure, Checked.sub(sceFailure, ylen, yday), 7), 1)
                try sceFailure.throwIfFailed()
                let wordFirst: Int64 = wd == 0 ? s.op0 : (wd == 1 ? s.op1 : (wd == 2 ? s.op2 : (wd == 3 ? s.op3 : (wd == 4 ? s.op4 : (wd == 5 ? s.op5 : s.op6)))))
                try sceFailure.throwIfFailed()
                let wordLast: Int64 = wd == 0 ? s.on0 : (wd == 1 ? s.on1 : (wd == 2 ? s.on2 : (wd == 3 ? s.on3 : (wd == 4 ? s.on4 : (wd == 5 ? s.on5 : s.on6)))))
                try sceFailure.throwIfFailed()
                let ordFirst: Bool = ordPos <= 53 && ((wordFirst >> Checked.sub(sceFailure, ordPos, 1)) & 1) == 1
                try sceFailure.throwIfFailed()
                let ordLast: Bool = ordNeg <= 53 && ((wordLast >> Checked.sub(sceFailure, ordNeg, 1)) & 1) == 1
                try sceFailure.throwIfFailed()
                let dayMatch: Bool = plainDay || ordFirst || ordLast
                try sceFailure.throwIfFailed()
                let clamped: Bool = s.invalid == 1 ? dom == dom0 : dom == (dom0 < last ? dom0 : last)
                try sceFailure.throwIfFailed()
                ok = !hasMonth || ((s.bymonth >> mon) & 1) == 1
                try sceFailure.throwIfFailed()
                do {
                    let sceCond4 = freq == 1
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceCond6 = hasMD
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && mdMatch
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasDay
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && plainDay
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
                do {
                    let sceCond4 = freq == 2
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        ok = ok && (hasDay ? plainDay : wd == startWd)
                        try sceFailure.throwIfFailed()
                    }
                }
                do {
                    let sceCond4 = freq == 3
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceCond6 = hasMD
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && mdMatch
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasDay
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && dayMatch
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = !hasMD && !hasDay
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && clamped
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
                do {
                    let sceCond4 = freq == 4
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        do {
                            let sceCond6 = hasWN
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                let wkst: Int64 = s.wkst >= 0 && s.wkst <= 6 ? s.wkst : 1
                                try sceFailure.throwIfFailed()
                                let wi: Int64 = Checked.add(sceFailure, Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, wd, wkst), 7), 7), 1)
                                try sceFailure.throwIfFailed()
                                let wk: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, yday, wi), 10), 7)
                                try sceFailure.throwIfFailed()
                                let wd28: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, wd, ylen), 3), yday), 7), 7)
                                try sceFailure.throwIfFailed()
                                let wi28: Int64 = Checked.add(sceFailure, Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, wd28, wkst), 7), 7), 1)
                                try sceFailure.throwIfFailed()
                                let wks: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, Checked.sub(sceFailure, ylen, 3), wi28), 10), 7)
                                try sceFailure.throwIfFailed()
                                let wkOk: Bool = wk >= 1 && wk <= wks && (((s.wnPos >> wk) & 1) == 1 || ((s.wnNeg >> Checked.add(sceFailure, Checked.sub(sceFailure, wks, wk), 1)) & 1) == 1)
                                try sceFailure.throwIfFailed()
                                ok = ok && wkOk
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasYD
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                let yp: Int64 = Checked.sub(sceFailure, yday, 1)
                                try sceFailure.throwIfFailed()
                                let yn: Int64 = Checked.sub(sceFailure, ylen, yday)
                                try sceFailure.throwIfFailed()
                                let jp: Int64 = Checked.div(sceFailure, yp, 61)
                                try sceFailure.throwIfFailed()
                                let jn: Int64 = Checked.div(sceFailure, yn, 61)
                                try sceFailure.throwIfFailed()
                                let wordPos: Int64 = jp == 0 ? s.yd0 : (jp == 1 ? s.yd1 : (jp == 2 ? s.yd2 : (jp == 3 ? s.yd3 : (jp == 4 ? s.yd4 : s.yd5))))
                                try sceFailure.throwIfFailed()
                                let wordNeg: Int64 = jn == 0 ? s.ydn0 : (jn == 1 ? s.ydn1 : (jn == 2 ? s.ydn2 : (jn == 3 ? s.ydn3 : (jn == 4 ? s.ydn4 : s.ydn5))))
                                try sceFailure.throwIfFailed()
                                let ydOk: Bool = ((wordPos >> Checked.rem(sceFailure, yp, 61)) & 1) == 1 || ((wordNeg >> Checked.rem(sceFailure, yn, 61)) & 1) == 1
                                try sceFailure.throwIfFailed()
                                ok = ok && ydOk
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasMD
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && mdMatch
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasDay
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && dayMatch
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = hasWN && !hasDay && !hasYD && !hasMD
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && wd == startWd
                                try sceFailure.throwIfFailed()
                            }
                        }
                        do {
                            let sceCond6 = !hasWN && !hasYD && !hasMD && !hasDay
                            try sceFailure.throwIfFailed()
                            if sceCond6 {
                                ok = ok && (hasMonth || mon == m0) && clamped
                                try sceFailure.throwIfFailed()
                            }
                        }
                    }
                }
            }
        }
        do {
            let sceReturned: Bool = ok
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
