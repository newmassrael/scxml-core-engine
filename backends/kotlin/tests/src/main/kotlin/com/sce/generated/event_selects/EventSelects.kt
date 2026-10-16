// SCE-GENERATED — DO NOT EDIT
// source-hash: 5b579589f1ff33cd05a081b40dc01d4d0d0b2ef410a9718dd0ae11e84c00a952
// SCE-MAP: event_selects.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_selects`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_selects

import com.sce.generated.event_schedule.*

fun eventSelects(s: EventSchedulePayload, d: Long, startDay: Long): com.sce.forge.runtime.AlgorithmResult<Boolean> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        var freq: Long = s.freq
        var ok: Boolean = true
        if (freq != 0L) {
            var startWd: Long = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(startDay, 7), 11), 7)
            var z0: Long = com.sce.forge.runtime.SceChecked.add(startDay, 719468)
            var era0: Long = com.sce.forge.runtime.SceChecked.div(z0, 146097)
            var doe0: Long = com.sce.forge.runtime.SceChecked.sub(z0, com.sce.forge.runtime.SceChecked.mul(era0, 146097))
            var yoe0: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(doe0, com.sce.forge.runtime.SceChecked.div(doe0, 1460)), com.sce.forge.runtime.SceChecked.div(doe0, 36524)), com.sce.forge.runtime.SceChecked.div(doe0, 146096)), 365)
            var doy0: Long = com.sce.forge.runtime.SceChecked.sub(doe0, com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(365, yoe0), com.sce.forge.runtime.SceChecked.div(yoe0, 4)), com.sce.forge.runtime.SceChecked.div(yoe0, 100)))
            var mp0: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(5, doy0), 2), 153)
            var dom0: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(doy0, com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(153, mp0), 2), 5)), 1)
            var m0: Long = if (mp0 < 10) com.sce.forge.runtime.SceChecked.add(mp0, 3) else com.sce.forge.runtime.SceChecked.sub(mp0, 9)
            var dz: Long = com.sce.forge.runtime.SceChecked.add(d, 719468)
            var dera: Long = com.sce.forge.runtime.SceChecked.div(dz, 146097)
            var ddoe: Long = com.sce.forge.runtime.SceChecked.sub(dz, com.sce.forge.runtime.SceChecked.mul(dera, 146097))
            var dyoe: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(ddoe, com.sce.forge.runtime.SceChecked.div(ddoe, 1460)), com.sce.forge.runtime.SceChecked.div(ddoe, 36524)), com.sce.forge.runtime.SceChecked.div(ddoe, 146096)), 365)
            var ddoy: Long = com.sce.forge.runtime.SceChecked.sub(ddoe, com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(365, dyoe), com.sce.forge.runtime.SceChecked.div(dyoe, 4)), com.sce.forge.runtime.SceChecked.div(dyoe, 100)))
            var dmp: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(5, ddoy), 2), 153)
            var dom: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(ddoy, com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(153, dmp), 2), 5)), 1)
            var mon: Long = if (dmp < 10) com.sce.forge.runtime.SceChecked.add(dmp, 3) else com.sce.forge.runtime.SceChecked.sub(dmp, 9)
            var yr: Long = if (mon <= 2) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(dyoe, com.sce.forge.runtime.SceChecked.mul(dera, 400)), 1) else com.sce.forge.runtime.SceChecked.add(dyoe, com.sce.forge.runtime.SceChecked.mul(dera, 400))
            var leap: Boolean = com.sce.forge.runtime.SceChecked.rem(yr, 4) == 0L && com.sce.forge.runtime.SceChecked.rem(yr, 100) != 0L || com.sce.forge.runtime.SceChecked.rem(yr, 400) == 0L
            var last: Long = if (mon == 2L) if (leap) 29 else 28 else if (mon == 4L || mon == 6L || mon == 9L || mon == 11L) 30 else 31
            var ylen: Long = if (leap) 366 else 365
            var wd: Long = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(d, 7), 11), 7)
            var yday: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.mul(367, mon), 362), 12), if (mon <= 2) 0 else if (leap) -1 else -2), dom)
            var hasMonth: Boolean = s.bymonth != 0L
            var hasMD: Boolean = s.mdPos or s.mdNeg != 0L
            var hasDay: Boolean = s.days or s.dayA or s.dayB != 0L
            var hasWN: Boolean = s.wnPos or s.wnNeg != 0L
            var hasYD: Boolean = s.yd0 or s.yd1 or s.yd2 or s.yd3 or s.yd4 or s.yd5 or s.ydn0 or s.ydn1 or s.ydn2 or s.ydn3 or s.ydn4 or s.ydn5 != 0L
            var mdMatch: Boolean = s.mdPos shr dom.toInt() and 1 == 1L || s.mdNeg shr com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(last, dom), 1).toInt() and 1 == 1L
            var plainDay: Boolean = s.days shr wd.toInt() and 1 == 1L
            var inMonth: Boolean = freq == 3L || hasMonth
            var ordPos: Long = if (inMonth) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(dom, 1), 7), 1) else com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(yday, 1), 7), 1)
            var ordNeg: Long = if (inMonth) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(last, dom), 7), 1) else com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(ylen, yday), 7), 1)
            var ordMask: Long = if (wd < 4) s.dayA else s.dayB
            var ordBase: Long = com.sce.forge.runtime.SceChecked.mul(com.sce.forge.runtime.SceChecked.rem(wd, 4), 10)
            var ordFirst: Boolean = ordPos <= 5 && ordMask shr com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(ordBase, ordPos), 1).toInt() and 1 == 1L
            var ordLast: Boolean = ordNeg <= 5 && ordMask shr com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(ordBase, 4), ordNeg).toInt() and 1 == 1L
            var dayMatch: Boolean = plainDay || ordFirst || ordLast
            var clamped: Boolean = dom == (if (dom0 < last) dom0 else last)
            ok = !hasMonth || s.bymonth shr mon.toInt() and 1 == 1L
            if (freq == 1L) {
                if (hasMD) {
                    ok = ok && mdMatch
                }
                if (hasDay) {
                    ok = ok && plainDay
                }
            }
            if (freq == 2L) {
                ok = ok && (if (hasDay) plainDay else wd == startWd)
            }
            if (freq == 3L) {
                if (hasMD) {
                    ok = ok && mdMatch
                }
                if (hasDay) {
                    ok = ok && dayMatch
                }
                if (!hasMD && !hasDay) {
                    ok = ok && clamped
                }
            }
            if (freq == 4L) {
                if (hasWN) {
                    var wkst: Long = if (s.wkst >= 0 && s.wkst <= 6) s.wkst else 1
                    var wi: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(wd, wkst), 7), 7), 1)
                    var wk: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(yday, wi), 10), 7)
                    var wd28: Long = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(wd, ylen), 3), yday), 7), 7)
                    var wi28: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(wd28, wkst), 7), 7), 1)
                    var wks: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.sub(ylen, 3), wi28), 10), 7)
                    var wkOk: Boolean = wk >= 1 && wk <= wks && (s.wnPos shr wk.toInt() and 1 == 1L || s.wnNeg shr com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(wks, wk), 1).toInt() and 1 == 1L)
                    ok = ok && wkOk
                }
                if (hasYD) {
                    var yp: Long = com.sce.forge.runtime.SceChecked.sub(yday, 1)
                    var yn: Long = com.sce.forge.runtime.SceChecked.sub(ylen, yday)
                    var jp: Long = com.sce.forge.runtime.SceChecked.div(yp, 61)
                    var jn: Long = com.sce.forge.runtime.SceChecked.div(yn, 61)
                    var wordPos: Long = if (jp == 0L) s.yd0 else if (jp == 1L) s.yd1 else if (jp == 2L) s.yd2 else if (jp == 3L) s.yd3 else if (jp == 4L) s.yd4 else s.yd5
                    var wordNeg: Long = if (jn == 0L) s.ydn0 else if (jn == 1L) s.ydn1 else if (jn == 2L) s.ydn2 else if (jn == 3L) s.ydn3 else if (jn == 4L) s.ydn4 else s.ydn5
                    var ydOk: Boolean = wordPos shr com.sce.forge.runtime.SceChecked.rem(yp, 61).toInt() and 1 == 1L || wordNeg shr com.sce.forge.runtime.SceChecked.rem(yn, 61).toInt() and 1 == 1L
                    ok = ok && ydOk
                }
                if (hasMD) {
                    ok = ok && mdMatch
                }
                if (hasDay) {
                    ok = ok && dayMatch
                }
                if (hasWN && !hasDay && !hasYD && !hasMD) {
                    ok = ok && wd == startWd
                }
                if (!hasWN && !hasYD && !hasMD && !hasDay) {
                    ok = ok && (hasMonth || mon == m0) && clamped
                }
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(ok)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
