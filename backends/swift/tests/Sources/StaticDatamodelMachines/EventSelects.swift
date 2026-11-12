// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f
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
    public static func eventSelects(_ s: EventSchedulePayload, _ d: Int64, _ startDay: Int64) throws(AlgorithmError) -> Bool {
        let freq: Int64 = s.freq
        var ok: Bool = true
        if freq != 0 {
            let startWd: Int64 = (try Checked.rem((try Checked.add((try Checked.rem(startDay, 7)), 11)), 7))
            let z0: Int64 = (try Checked.add(startDay, 719468))
            let era0: Int64 = (try Checked.div(z0, 146097))
            let doe0: Int64 = (try Checked.sub(z0, (try Checked.mul(era0, 146097))))
            let yoe0: Int64 = (try Checked.div((try Checked.sub((try Checked.add((try Checked.sub(doe0, (try Checked.div(doe0, 1460)))), (try Checked.div(doe0, 36524)))), (try Checked.div(doe0, 146096)))), 365))
            let doy0: Int64 = (try Checked.sub(doe0, (try Checked.sub((try Checked.add((try Checked.mul(365, yoe0)), (try Checked.div(yoe0, 4)))), (try Checked.div(yoe0, 100))))))
            let mp0: Int64 = (try Checked.div((try Checked.add((try Checked.mul(5, doy0)), 2)), 153))
            let dom0: Int64 = (try Checked.add((try Checked.sub(doy0, (try Checked.div((try Checked.add((try Checked.mul(153, mp0)), 2)), 5)))), 1))
            let m0: Int64 = mp0 < 10 ? (try Checked.add(mp0, 3)) : (try Checked.sub(mp0, 9))
            let dz: Int64 = (try Checked.add(d, 719468))
            let dera: Int64 = (try Checked.div(dz, 146097))
            let ddoe: Int64 = (try Checked.sub(dz, (try Checked.mul(dera, 146097))))
            let dyoe: Int64 = (try Checked.div((try Checked.sub((try Checked.add((try Checked.sub(ddoe, (try Checked.div(ddoe, 1460)))), (try Checked.div(ddoe, 36524)))), (try Checked.div(ddoe, 146096)))), 365))
            let ddoy: Int64 = (try Checked.sub(ddoe, (try Checked.sub((try Checked.add((try Checked.mul(365, dyoe)), (try Checked.div(dyoe, 4)))), (try Checked.div(dyoe, 100))))))
            let dmp: Int64 = (try Checked.div((try Checked.add((try Checked.mul(5, ddoy)), 2)), 153))
            let dom: Int64 = (try Checked.add((try Checked.sub(ddoy, (try Checked.div((try Checked.add((try Checked.mul(153, dmp)), 2)), 5)))), 1))
            let mon: Int64 = dmp < 10 ? (try Checked.add(dmp, 3)) : (try Checked.sub(dmp, 9))
            let yr: Int64 = mon <= 2 ? (try Checked.add((try Checked.add(dyoe, (try Checked.mul(dera, 400)))), 1)) : (try Checked.add(dyoe, (try Checked.mul(dera, 400))))
            let leap: Bool = (try Checked.or((try Checked.and((try Checked.rem(yr, 4)) == 0, { () throws(AlgorithmError) -> Bool in (try Checked.rem(yr, 100)) != 0 })), { () throws(AlgorithmError) -> Bool in (try Checked.rem(yr, 400)) == 0 }))
            let last: Int64 = mon == 2 ? (leap ? 29 : 28) : (mon == 4 || mon == 6 || mon == 9 || mon == 11 ? 30 : 31)
            let ylen: Int64 = leap ? 366 : 365
            let wd: Int64 = (try Checked.rem((try Checked.add((try Checked.rem(d, 7)), 11)), 7))
            let yday: Int64 = (try Checked.add((try Checked.add((try Checked.div((try Checked.sub((try Checked.mul(367, mon)), 362)), 12)), mon <= 2 ? 0 : (leap ? -1 : -2))), dom))
            let hasMonth: Bool = s.bymonth != 0
            let hasMD: Bool = (s.mdPos | s.mdNeg) != 0
            let hasDay: Bool = ((((((((((((((s.days | s.op0) | s.op1) | s.op2) | s.op3) | s.op4) | s.op5) | s.op6) | s.on0) | s.on1) | s.on2) | s.on3) | s.on4) | s.on5) | s.on6) != 0
            let hasWN: Bool = (s.wnPos | s.wnNeg) != 0
            let hasYD: Bool = (((((((((((s.yd0 | s.yd1) | s.yd2) | s.yd3) | s.yd4) | s.yd5) | s.ydn0) | s.ydn1) | s.ydn2) | s.ydn3) | s.ydn4) | s.ydn5) != 0
            let mdMatch: Bool = (try Checked.or(((s.mdPos >> dom) & 1) == 1, { () throws(AlgorithmError) -> Bool in ((s.mdNeg >> (try Checked.add((try Checked.sub(last, dom)), 1))) & 1) == 1 }))
            let plainDay: Bool = ((s.days >> wd) & 1) == 1
            let inMonth: Bool = freq == 3 || hasMonth
            let ordPos: Int64 = inMonth ? (try Checked.add((try Checked.div((try Checked.sub(dom, 1)), 7)), 1)) : (try Checked.add((try Checked.div((try Checked.sub(yday, 1)), 7)), 1))
            let ordNeg: Int64 = inMonth ? (try Checked.add((try Checked.div((try Checked.sub(last, dom)), 7)), 1)) : (try Checked.add((try Checked.div((try Checked.sub(ylen, yday)), 7)), 1))
            let wordFirst: Int64 = wd == 0 ? s.op0 : (wd == 1 ? s.op1 : (wd == 2 ? s.op2 : (wd == 3 ? s.op3 : (wd == 4 ? s.op4 : (wd == 5 ? s.op5 : s.op6)))))
            let wordLast: Int64 = wd == 0 ? s.on0 : (wd == 1 ? s.on1 : (wd == 2 ? s.on2 : (wd == 3 ? s.on3 : (wd == 4 ? s.on4 : (wd == 5 ? s.on5 : s.on6)))))
            let ordFirst: Bool = (try Checked.and(ordPos <= 53, { () throws(AlgorithmError) -> Bool in ((wordFirst >> (try Checked.sub(ordPos, 1))) & 1) == 1 }))
            let ordLast: Bool = (try Checked.and(ordNeg <= 53, { () throws(AlgorithmError) -> Bool in ((wordLast >> (try Checked.sub(ordNeg, 1))) & 1) == 1 }))
            let dayMatch: Bool = plainDay || ordFirst || ordLast
            let clamped: Bool = s.invalid == 1 ? dom == dom0 : dom == (dom0 < last ? dom0 : last)
            ok = !hasMonth || ((s.bymonth >> mon) & 1) == 1
            if freq == 1 {
                if hasMD {
                    ok = ok && mdMatch
                }
                if hasDay {
                    ok = ok && plainDay
                }
            }
            if freq == 2 {
                ok = ok && (hasDay ? plainDay : wd == startWd)
            }
            if freq == 3 {
                if hasMD {
                    ok = ok && mdMatch
                }
                if hasDay {
                    ok = ok && dayMatch
                }
                if !hasMD && !hasDay {
                    ok = ok && clamped
                }
            }
            if freq == 4 {
                if hasWN {
                    let wkst: Int64 = s.wkst >= 0 && s.wkst <= 6 ? s.wkst : 1
                    let wi: Int64 = (try Checked.add((try Checked.rem((try Checked.add((try Checked.sub(wd, wkst)), 7)), 7)), 1))
                    let wk: Int64 = (try Checked.div((try Checked.add((try Checked.sub(yday, wi)), 10)), 7))
                    let wd28: Int64 = (try Checked.rem((try Checked.add((try Checked.sub((try Checked.sub((try Checked.add(wd, ylen)), 3)), yday)), 7)), 7))
                    let wi28: Int64 = (try Checked.add((try Checked.rem((try Checked.add((try Checked.sub(wd28, wkst)), 7)), 7)), 1))
                    let wks: Int64 = (try Checked.div((try Checked.add((try Checked.sub((try Checked.sub(ylen, 3)), wi28)), 10)), 7))
                    let wkOk: Bool = (try Checked.and(wk >= 1 && wk <= wks, { () throws(AlgorithmError) -> Bool in (try Checked.or(((s.wnPos >> wk) & 1) == 1, { () throws(AlgorithmError) -> Bool in ((s.wnNeg >> (try Checked.add((try Checked.sub(wks, wk)), 1))) & 1) == 1 })) }))
                    ok = ok && wkOk
                }
                if hasYD {
                    let yp: Int64 = (try Checked.sub(yday, 1))
                    let yn: Int64 = (try Checked.sub(ylen, yday))
                    let jp: Int64 = (try Checked.div(yp, 61))
                    let jn: Int64 = (try Checked.div(yn, 61))
                    let wordPos: Int64 = jp == 0 ? s.yd0 : (jp == 1 ? s.yd1 : (jp == 2 ? s.yd2 : (jp == 3 ? s.yd3 : (jp == 4 ? s.yd4 : s.yd5))))
                    let wordNeg: Int64 = jn == 0 ? s.ydn0 : (jn == 1 ? s.ydn1 : (jn == 2 ? s.ydn2 : (jn == 3 ? s.ydn3 : (jn == 4 ? s.ydn4 : s.ydn5))))
                    let ydOk: Bool = (try Checked.or(((wordPos >> (try Checked.rem(yp, 61))) & 1) == 1, { () throws(AlgorithmError) -> Bool in ((wordNeg >> (try Checked.rem(yn, 61))) & 1) == 1 }))
                    ok = ok && ydOk
                }
                if hasMD {
                    ok = ok && mdMatch
                }
                if hasDay {
                    ok = ok && dayMatch
                }
                if hasWN && !hasDay && !hasYD && !hasMD {
                    ok = ok && wd == startWd
                }
                if !hasWN && !hasYD && !hasMD && !hasDay {
                    ok = ok && (hasMonth || mon == m0) && clamped
                }
            }
        }
        return ok
    }
}
