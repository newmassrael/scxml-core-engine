// SCE-GENERATED — DO NOT EDIT
// source-hash: b9ab4ddbd49e8ce2a3b4c80917bc3eecdf0d5285c15974d4e0a00636f93fd573
// SCE-MAP: event_agenda.scxml:54 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `com.sce.generated.event_agenda`, no instance
// state. `bytes` parameters lower to `ByteArray` (RFC §synth-5-J-5 emitter
// table). Iteration over `ByteArray` yields signed `Byte`, so the
// foreach lowering inserts a `Byte → UByte` reinterpretation per
// iteration to match the SCXML type-ctx contract that
// `<sce:foreach item>` is `uint8`.

package com.sce.generated.event_agenda

import com.sce.generated.mv_value.*
import com.sce.generated.event_occurrence.*
import com.sce.generated.event_later.*

fun eventAgenda(values: List<MvValuePayload>, id: ULong, windowStart: Long, windowEnd: Long): com.sce.forge.runtime.AlgorithmResult<List<EventOccurrencePayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<EventOccurrencePayload>(64)
        var emitted: Long = 0
        var day: Long = 86400000
        var startAt: Long = -1
        var gone: Long = 0
        var freq: Long = 0
        var length: Long = 0
        var every0: Long = 1
        var days: Long = 0
        var count: Long = 0
        var until: Long = 0
        var flag: UByte = 0.toUByte()
        var found: Boolean = false
        var held: Long = 0
        var bw: Long = 0
        var bc: UInt = 0.toUInt()
        var bn: ULong = 0.toULong()
        found = false
        held = 0
        for (w16 in values) {
            if (w16.id == id && w16.field == 16.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w16.wallTime, w16.counter, w16.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w16.wallTime
                    bc = w16.counter
                    bn = w16.nodeId
                    startAt = w16.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w9 in values) {
            if (w9.id == id && w9.field == 9.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w9.wallTime, w9.counter, w9.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w9.wallTime
                    bc = w9.counter
                    bn = w9.nodeId
                    gone = w9.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w18 in values) {
            if (w18.id == id && w18.field == 18.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w18.wallTime, w18.counter, w18.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w18.wallTime
                    bc = w18.counter
                    bn = w18.nodeId
                    freq = w18.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w17 in values) {
            if (w17.id == id && w17.field == 17.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w17.wallTime, w17.counter, w17.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w17.wallTime
                    bc = w17.counter
                    bn = w17.nodeId
                    length = w17.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w19 in values) {
            if (w19.id == id && w19.field == 19.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w19.wallTime, w19.counter, w19.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w19.wallTime
                    bc = w19.counter
                    bn = w19.nodeId
                    every0 = w19.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w20 in values) {
            if (w20.id == id && w20.field == 20.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w20.wallTime, w20.counter, w20.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w20.wallTime
                    bc = w20.counter
                    bn = w20.nodeId
                    days = w20.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w21 in values) {
            if (w21.id == id && w21.field == 21.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w21.wallTime, w21.counter, w21.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w21.wallTime
                    bc = w21.counter
                    bn = w21.nodeId
                    count = w21.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        found = false
        held = 0
        for (w22 in values) {
            if (w22.id == id && w22.field == 22.toULong()) {
                held = com.sce.forge.runtime.SceChecked.add(held, 1)
                if (!found || eventLater(w22.wallTime, w22.counter, w22.nodeId, bw, bc, bn)) {
                    found = true
                    bw = w22.wallTime
                    bc = w22.counter
                    bn = w22.nodeId
                    until = w22.value
                }
            }
        }
        if (held > 1) {
            flag = 1.toUByte()
        }
        if (startAt >= 0 && gone != 1L && freq >= 0 && freq <= 3 && windowEnd > windowStart && windowEnd > 0) {
            var every: Long = if (every0 < 1) 1 else every0
            var timeOfDay: Long = com.sce.forge.runtime.SceChecked.rem(startAt, day)
            var startDay: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(startAt, timeOfDay), day)
            var endDay: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(windowEnd, day), 1), day)
            var startWeekday: Long = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(startDay, 7), 11), 7)
            var weekStart: Long = com.sce.forge.runtime.SceChecked.sub(startDay, startWeekday)
            var one: Long = 1
            var weekdays: Long = if (days == 0L) one shl startWeekday.toInt() else days and 127
            var z: Long = com.sce.forge.runtime.SceChecked.add(startDay, 719468)
            var era: Long = com.sce.forge.runtime.SceChecked.div(z, 146097)
            var doe: Long = com.sce.forge.runtime.SceChecked.sub(z, com.sce.forge.runtime.SceChecked.mul(era, 146097))
            var yoe: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(doe, com.sce.forge.runtime.SceChecked.div(doe, 1460)), com.sce.forge.runtime.SceChecked.div(doe, 36524)), com.sce.forge.runtime.SceChecked.div(doe, 146096)), 365)
            var doy: Long = com.sce.forge.runtime.SceChecked.sub(doe, com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(365, yoe), com.sce.forge.runtime.SceChecked.div(yoe, 4)), com.sce.forge.runtime.SceChecked.div(yoe, 100)))
            var mp: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(5, doy), 2), 153)
            var startOfMonth: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(doy, com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(153, mp), 2), 5)), 1)
            var startMonth: Long = if (mp < 10) com.sce.forge.runtime.SceChecked.add(mp, 3) else com.sce.forge.runtime.SceChecked.sub(mp, 9)
            var startYear: Long = if (startMonth <= 2) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(yoe, com.sce.forge.runtime.SceChecked.mul(era, 400)), 1) else com.sce.forge.runtime.SceChecked.add(yoe, com.sce.forge.runtime.SceChecked.mul(era, 400))
            var months: Long = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(startYear, 12), startMonth), 1)
            var cursor: Long = 0
            var nth: Long = 0
            var going: Boolean = true
            while (going) {
                var d: Long = com.sce.forge.runtime.SceChecked.add(startDay, cursor)
                var valid: Boolean = false
                if (freq == 0L) {
                    valid = cursor == 0L
                }
                if (freq == 1L) {
                    valid = com.sce.forge.runtime.SceChecked.rem(cursor, every) == 0L
                }
                if (freq == 2L) {
                    valid = weekdays shr com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(d, 7), 11), 7).toInt() and 1 == 1L && com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(d, weekStart), 7), every) == 0L
                }
                if (freq == 3L) {
                    var total: Long = com.sce.forge.runtime.SceChecked.add(months, com.sce.forge.runtime.SceChecked.mul(cursor, every))
                    var landYear: Long = com.sce.forge.runtime.SceChecked.div(total, 12)
                    var landMonth: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(total, 12), 1)
                    var landLeap: Boolean = com.sce.forge.runtime.SceChecked.rem(landYear, 4) == 0L && com.sce.forge.runtime.SceChecked.rem(landYear, 100) != 0L || com.sce.forge.runtime.SceChecked.rem(landYear, 400) == 0L
                    var last: Long = if (landMonth == 2L) if (landLeap) 29 else 28 else if (landMonth == 4L || landMonth == 6L || landMonth == 9L || landMonth == 11L) 30 else 31
                    var landDay: Long = if (startOfMonth < last) startOfMonth else last
                    var cy: Long = if (landMonth <= 2) com.sce.forge.runtime.SceChecked.sub(landYear, 1) else landYear
                    var cera: Long = com.sce.forge.runtime.SceChecked.div(cy, 400)
                    var cyoe: Long = com.sce.forge.runtime.SceChecked.sub(cy, com.sce.forge.runtime.SceChecked.mul(cera, 400))
                    var cmp: Long = if (landMonth > 2) com.sce.forge.runtime.SceChecked.sub(landMonth, 3) else com.sce.forge.runtime.SceChecked.add(landMonth, 9)
                    var cdoy: Long = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(153, cmp), 2), 5), landDay), 1)
                    var cdoe: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(cyoe, 365), com.sce.forge.runtime.SceChecked.div(cyoe, 4)), com.sce.forge.runtime.SceChecked.div(cyoe, 100)), cdoy)
                    d = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(cera, 146097), cdoe), 719468)
                    valid = true
                }
                var occStart: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(d, day), timeOfDay)
                var occEnd: Long = com.sce.forge.runtime.SceChecked.add(occStart, length)
                if (d >= endDay || occStart >= windowEnd || until > 0 && occStart > until) {
                    going = false
                } else {
                    if (valid) {
                        if (count > 0 && nth >= count) {
                            going = false
                        } else {
                            if ((occEnd > windowStart || occStart >= windowStart) && emitted < 64) {
                                var excluded: Boolean = false
                                var mark: UByte = flag
                                var dayNo: Long = com.sce.forge.runtime.SceChecked.div(occStart, day)
                                if (dayNo >= 18262 && com.sce.forge.runtime.SceChecked.sub(dayNo, 18262) < 65472) {
                                    var xfield: ULong = com.sce.forge.runtime.SceChecked.narrowToULong(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(dayNo, 18262), 64))
                                    var xfound: Boolean = false
                                    var xheld: Long = 0
                                    var xw: Long = 0
                                    var xc: UInt = 0.toUInt()
                                    var xn: ULong = 0.toULong()
                                    var xv: Long = 0
                                    for (wx in values) {
                                        if (wx.id == id && wx.field == xfield) {
                                            xheld = com.sce.forge.runtime.SceChecked.add(xheld, 1)
                                            if (!xfound || eventLater(wx.wallTime, wx.counter, wx.nodeId, xw, xc, xn)) {
                                                xfound = true
                                                xw = wx.wallTime
                                                xc = wx.counter
                                                xn = wx.nodeId
                                                xv = wx.value
                                            }
                                        }
                                    }
                                    excluded = xfound && xv == 1L
                                    if (xheld > 1) {
                                        mark = 1.toUByte()
                                    }
                                }
                                if (!excluded) {
                                    var line = EventOccurrencePayload(id = id, start = occStart, end = occEnd, nth = com.sce.forge.runtime.SceChecked.narrowToUInt(nth), conflict = mark)
                                    out.add(line)
                                    emitted = com.sce.forge.runtime.SceChecked.add(emitted, 1)
                                }
                            }
                            nth = com.sce.forge.runtime.SceChecked.add(nth, 1)
                            if (freq == 0L) {
                                going = false
                            }
                        }
                    }
                }
                if (emitted >= 64) {
                    going = false
                }
                cursor = com.sce.forge.runtime.SceChecked.add(cursor, 1)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
