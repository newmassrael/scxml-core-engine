// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4
// SCE-MAP: event_agenda.scxml:71 :: _forge_body

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
import com.sce.generated.event_schedule.*
import com.sce.generated.event_later.*
import com.sce.generated.event_selects.*

fun eventAgenda(s: EventSchedulePayload, values: List<MvValuePayload>, id: ULong, windowStart: Long, windowEnd: Long, skip: UInt): com.sce.forge.runtime.AlgorithmResult<List<EventOccurrencePayload>> {
    // SCE_FORGE.md §3.4.1: a checked operation throws its failure, and this
    // boundary hands it to the caller as a value — never as an exception.
    try {
        val out = ArrayList<EventOccurrencePayload>(64)
        var emitted: Long = 0
        var day: Long = 86400000
        var skipped: Long = skip.toLong()
        var startAt: Long = s.start
        var freq: Long = s.freq
        if (startAt >= 0 && s.gone != 1L && freq >= 0 && freq <= 4 && windowEnd > windowStart && windowEnd > 0) {
            var every: Long = if (s.interval < 1) 1 else s.interval
            var length: Long = s.length
            var count: Long = s.count
            var until: Long = s.until
            var hasSP: Boolean = s.spPos or s.spNeg != 0L
            var timeOfDay: Long = com.sce.forge.runtime.SceChecked.rem(startAt, day)
            var startDay: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(startAt, timeOfDay), day)
            var endDay: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(windowEnd, day), 1), day)
            var startWeekday: Long = com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(startDay, 7), 11), 7)
            var wkst0: Long = s.wkst
            var wkst: Long = if (wkst0 >= 0 && wkst0 <= 6) wkst0 else 1
            var week0Start: Long = com.sce.forge.runtime.SceChecked.sub(startDay, com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(startWeekday, wkst), 7), 7))
            var z: Long = com.sce.forge.runtime.SceChecked.add(startDay, 719468)
            var era: Long = com.sce.forge.runtime.SceChecked.div(z, 146097)
            var doe: Long = com.sce.forge.runtime.SceChecked.sub(z, com.sce.forge.runtime.SceChecked.mul(era, 146097))
            var yoe: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(doe, com.sce.forge.runtime.SceChecked.div(doe, 1460)), com.sce.forge.runtime.SceChecked.div(doe, 36524)), com.sce.forge.runtime.SceChecked.div(doe, 146096)), 365)
            var doy: Long = com.sce.forge.runtime.SceChecked.sub(doe, com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(365, yoe), com.sce.forge.runtime.SceChecked.div(yoe, 4)), com.sce.forge.runtime.SceChecked.div(yoe, 100)))
            var mp: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(5, doy), 2), 153)
            var m0: Long = if (mp < 10) com.sce.forge.runtime.SceChecked.add(mp, 3) else com.sce.forge.runtime.SceChecked.sub(mp, 9)
            var y0: Long = if (m0 <= 2) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(yoe, com.sce.forge.runtime.SceChecked.mul(era, 400)), 1) else com.sce.forge.runtime.SceChecked.add(yoe, com.sce.forge.runtime.SceChecked.mul(era, 400))
            var months0: Long = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(y0, 12), m0), 1)
            var p: Long = 0
            var nth: Long = 0
            var going: Boolean = true
            while (going) {
                var ps: Long = startDay
                var plen: Long = 1
                if (freq == 1L) {
                    ps = com.sce.forge.runtime.SceChecked.add(startDay, com.sce.forge.runtime.SceChecked.mul(p, every))
                }
                if (freq == 2L) {
                    ps = com.sce.forge.runtime.SceChecked.add(week0Start, com.sce.forge.runtime.SceChecked.mul(com.sce.forge.runtime.SceChecked.mul(p, every), 7))
                    plen = 7
                }
                if (freq == 3L || freq == 4L) {
                    var pty: Long = if (freq == 3L) com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(months0, com.sce.forge.runtime.SceChecked.mul(p, every)), 12) else com.sce.forge.runtime.SceChecked.add(y0, com.sce.forge.runtime.SceChecked.mul(p, every))
                    var ptm: Long = if (freq == 3L) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.rem(com.sce.forge.runtime.SceChecked.add(months0, com.sce.forge.runtime.SceChecked.mul(p, every)), 12), 1) else 1
                    var pleap: Boolean = com.sce.forge.runtime.SceChecked.rem(pty, 4) == 0L && com.sce.forge.runtime.SceChecked.rem(pty, 100) != 0L || com.sce.forge.runtime.SceChecked.rem(pty, 400) == 0L
                    var pcy: Long = if (ptm <= 2) com.sce.forge.runtime.SceChecked.sub(pty, 1) else pty
                    var pcera: Long = com.sce.forge.runtime.SceChecked.div(pcy, 400)
                    var pcyoe: Long = com.sce.forge.runtime.SceChecked.sub(pcy, com.sce.forge.runtime.SceChecked.mul(pcera, 400))
                    var pcmp: Long = if (ptm > 2) com.sce.forge.runtime.SceChecked.sub(ptm, 3) else com.sce.forge.runtime.SceChecked.add(ptm, 9)
                    var pcdoy: Long = com.sce.forge.runtime.SceChecked.div(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(153, pcmp), 2), 5)
                    ps = com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(pcera, 146097), com.sce.forge.runtime.SceChecked.mul(pcyoe, 365)), com.sce.forge.runtime.SceChecked.div(pcyoe, 4)), com.sce.forge.runtime.SceChecked.div(pcyoe, 100)), pcdoy), 719468)
                    plen = if (freq == 4L) if (pleap) 366 else 365 else if (ptm == 2L) if (pleap) 29 else 28 else if (ptm == 4L || ptm == 6L || ptm == 9L || ptm == 11L) 30 else 31
                }
                if (ps >= endDay || until > 0 && com.sce.forge.runtime.SceChecked.mul(ps, day) > until) {
                    going = false
                } else {
                    var n: Long = 0
                    var k: Long = 0
                    while (k < plen) {
                        if (com.sce.forge.runtime.SceChecked.take(eventSelects(s, com.sce.forge.runtime.SceChecked.add(ps, k), startDay))) {
                            n = com.sce.forge.runtime.SceChecked.add(n, 1)
                        }
                        k = com.sce.forge.runtime.SceChecked.add(k, 1)
                    }
                    var i: Long = 0
                    var rank: Long = 0
                    while (i < plen && going) {
                        var dd: Long = com.sce.forge.runtime.SceChecked.add(ps, i)
                        if (com.sce.forge.runtime.SceChecked.take(eventSelects(s, dd, startDay))) {
                            rank = com.sce.forge.runtime.SceChecked.add(rank, 1)
                            var keep: Boolean = true
                            if (hasSP) {
                                keep = rank <= 62 && s.spPos shr rank.toInt() and 1 == 1L || com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(n, rank), 1) <= 62 && s.spNeg shr com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.sub(n, rank), 1).toInt() and 1 == 1L
                            }
                            if (keep && dd >= startDay) {
                                var occStart: Long = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(dd, day), timeOfDay)
                                var occEnd: Long = com.sce.forge.runtime.SceChecked.add(occStart, length)
                                if (occStart >= windowEnd || until > 0 && occStart > until) {
                                    going = false
                                } else {
                                    if (count > 0 && nth >= count) {
                                        going = false
                                    } else {
                                        if ((occEnd > windowStart || occStart >= windowStart) && emitted < 64 && nth >= skipped) {
                                            var excluded: Boolean = false
                                            var mark: UByte = com.sce.forge.runtime.SceChecked.narrowToUByte(s.flag)
                                            if (dd >= 0 && dd < 65472) {
                                                var xfield: ULong = com.sce.forge.runtime.SceChecked.narrowToULong(com.sce.forge.runtime.SceChecked.add(dd, 64))
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
                                    }
                                }
                            }
                        }
                        if (emitted >= 64) {
                            going = false
                        }
                        i = com.sce.forge.runtime.SceChecked.add(i, 1)
                    }
                    if (freq == 0L) {
                        going = false
                    }
                }
                p = com.sce.forge.runtime.SceChecked.add(p, 1)
            }
        }
        return com.sce.forge.runtime.AlgorithmResult.Ok(out)
    } catch (failure: com.sce.forge.runtime.AlgorithmFailure) {
        return com.sce.forge.runtime.AlgorithmResult.Failed(failure.error)
    }
}
