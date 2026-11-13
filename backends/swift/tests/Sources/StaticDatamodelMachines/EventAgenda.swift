// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d
// SCE-MAP: event_agenda.scxml:71 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `EventAgenda`, a file of the
// module that calls it: a call is qualified by the enum
// (`EventAgenda.eventAgenda(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum EventAgenda {
    public static func eventAgenda(_ s: EventSchedulePayload, _ values: [MvValuePayload], _ id: UInt64, _ windowStart: Int64, _ windowEnd: Int64, _ skip: UInt32) throws(AlgorithmError) -> [EventOccurrencePayload] {
        var out: [EventOccurrencePayload] = []
        out.reserveCapacity(64)
        var emitted: Int64 = 0
        let day: Int64 = 86400000
        let skipped: Int64 = Int64(skip)
        let startAt: Int64 = s.start
        let freq: Int64 = s.freq
        if startAt >= 0 && s.gone != 1 && freq >= 0 && freq <= 4 && windowEnd > windowStart && windowEnd > 0 {
            let every: Int64 = s.interval < 1 ? 1 : s.interval
            let length: Int64 = s.length
            let count: Int64 = s.count
            let until: Int64 = s.until
            let hasSP: Bool = (s.spPos | s.spNeg) != 0
            let timeOfDay: Int64 = (try Checked.rem(startAt, day))
            let startDay: Int64 = (try Checked.div((try Checked.sub(startAt, timeOfDay)), day))
            let endDay: Int64 = (try Checked.div((try Checked.sub((try Checked.add(windowEnd, day)), 1)), day))
            let startWeekday: Int64 = (try Checked.rem((try Checked.add((try Checked.rem(startDay, 7)), 11)), 7))
            let wkst0: Int64 = s.wkst
            let wkst: Int64 = wkst0 >= 0 && wkst0 <= 6 ? wkst0 : 1
            let week0Start: Int64 = (try Checked.sub(startDay, (try Checked.rem((try Checked.add((try Checked.sub(startWeekday, wkst)), 7)), 7))))
            let z: Int64 = (try Checked.add(startDay, 719468))
            let era: Int64 = (try Checked.div(z, 146097))
            let doe: Int64 = (try Checked.sub(z, (try Checked.mul(era, 146097))))
            let yoe: Int64 = (try Checked.div((try Checked.sub((try Checked.add((try Checked.sub(doe, (try Checked.div(doe, 1460)))), (try Checked.div(doe, 36524)))), (try Checked.div(doe, 146096)))), 365))
            let doy: Int64 = (try Checked.sub(doe, (try Checked.sub((try Checked.add((try Checked.mul(365, yoe)), (try Checked.div(yoe, 4)))), (try Checked.div(yoe, 100))))))
            let mp: Int64 = (try Checked.div((try Checked.add((try Checked.mul(5, doy)), 2)), 153))
            let m0: Int64 = mp < 10 ? (try Checked.add(mp, 3)) : (try Checked.sub(mp, 9))
            let y0: Int64 = m0 <= 2 ? (try Checked.add((try Checked.add(yoe, (try Checked.mul(era, 400)))), 1)) : (try Checked.add(yoe, (try Checked.mul(era, 400))))
            let months0: Int64 = (try Checked.sub((try Checked.add((try Checked.mul(y0, 12)), m0)), 1))
            var p: Int64 = 0
            var nth: Int64 = 0
            var going: Bool = true
            while going {
                var ps: Int64 = startDay
                var plen: Int64 = 1
                if freq == 1 {
                    ps = (try Checked.add(startDay, (try Checked.mul(p, every))))
                }
                if freq == 2 {
                    ps = (try Checked.add(week0Start, (try Checked.mul((try Checked.mul(p, every)), 7))))
                    plen = 7
                }
                if freq == 3 || freq == 4 {
                    let pty: Int64 = freq == 3 ? (try Checked.div((try Checked.add(months0, (try Checked.mul(p, every)))), 12)) : (try Checked.add(y0, (try Checked.mul(p, every))))
                    let ptm: Int64 = freq == 3 ? (try Checked.add((try Checked.rem((try Checked.add(months0, (try Checked.mul(p, every)))), 12)), 1)) : 1
                    let pleap: Bool = (try Checked.or((try Checked.and((try Checked.rem(pty, 4)) == 0, { () throws(AlgorithmError) -> Bool in (try Checked.rem(pty, 100)) != 0 })), { () throws(AlgorithmError) -> Bool in (try Checked.rem(pty, 400)) == 0 }))
                    let pcy: Int64 = ptm <= 2 ? (try Checked.sub(pty, 1)) : pty
                    let pcera: Int64 = (try Checked.div(pcy, 400))
                    let pcyoe: Int64 = (try Checked.sub(pcy, (try Checked.mul(pcera, 400))))
                    let pcmp: Int64 = ptm > 2 ? (try Checked.sub(ptm, 3)) : (try Checked.add(ptm, 9))
                    let pcdoy: Int64 = (try Checked.div((try Checked.add((try Checked.mul(153, pcmp)), 2)), 5))
                    ps = (try Checked.sub((try Checked.add((try Checked.sub((try Checked.add((try Checked.add((try Checked.mul(pcera, 146097)), (try Checked.mul(pcyoe, 365)))), (try Checked.div(pcyoe, 4)))), (try Checked.div(pcyoe, 100)))), pcdoy)), 719468))
                    plen = freq == 4 ? (pleap ? 366 : 365) : (ptm == 2 ? (pleap ? 29 : 28) : (ptm == 4 || ptm == 6 || ptm == 9 || ptm == 11 ? 30 : 31))
                }
                if (try Checked.or(ps >= endDay, { () throws(AlgorithmError) -> Bool in (try Checked.and(until > 0, { () throws(AlgorithmError) -> Bool in (try Checked.mul(ps, day)) > until })) })) {
                    going = false
                } else {
                    var n: Int64 = 0
                    var k: Int64 = 0
                    while k < plen {
                        if (try EventSelects.eventSelects(s, (try Checked.add(ps, k)), startDay)) {
                            n = (try Checked.add(n, 1))
                        }
                        k = (try Checked.add(k, 1))
                    }
                    var i: Int64 = 0
                    var rank: Int64 = 0
                    while i < plen && going {
                        let dd: Int64 = (try Checked.add(ps, i))
                        if (try EventSelects.eventSelects(s, dd, startDay)) {
                            rank = (try Checked.add(rank, 1))
                            var keep: Bool = true
                            if hasSP {
                                keep = (try Checked.or(rank <= 62 && ((s.spPos >> rank) & 1) == 1, { () throws(AlgorithmError) -> Bool in (try Checked.and((try Checked.add((try Checked.sub(n, rank)), 1)) <= 62, { () throws(AlgorithmError) -> Bool in ((s.spNeg >> (try Checked.add((try Checked.sub(n, rank)), 1))) & 1) == 1 })) }))
                            }
                            if keep && dd >= startDay {
                                let occStart: Int64 = (try Checked.add((try Checked.mul(dd, day)), timeOfDay))
                                let occEnd: Int64 = (try Checked.add(occStart, length))
                                if occStart >= windowEnd || until > 0 && occStart > until {
                                    going = false
                                } else {
                                    if count > 0 && nth >= count {
                                        going = false
                                    } else {
                                        if (occEnd > windowStart || occStart >= windowStart) && emitted < 64 && nth >= skipped {
                                            var excluded: Bool = false
                                            var mark: UInt8 = (try Checked.narrow(s.flag, to: UInt8.self))
                                            if dd >= 0 && dd < 65472 {
                                                let xfield: UInt64 = (try Checked.narrow((try Checked.add(dd, 64)), to: UInt64.self))
                                                var xfound: Bool = false
                                                var xheld: Int64 = 0
                                                var xw: Int64 = 0
                                                var xc: UInt32 = 0
                                                var xn: UInt64 = 0
                                                var xv: Int64 = 0
                                                for wx in values {
                                                    if wx.id == id && wx.field == xfield {
                                                        xheld = (try Checked.add(xheld, 1))
                                                        if !xfound || EventLater.eventLater(wx.wallTime, wx.counter, wx.nodeId, xw, xc, xn) {
                                                            xfound = true
                                                            xw = wx.wallTime
                                                            xc = wx.counter
                                                            xn = wx.nodeId
                                                            xv = wx.value
                                                        }
                                                    }
                                                }
                                                excluded = xfound && xv == 1
                                                if xheld > 1 {
                                                    mark = 1
                                                }
                                            }
                                            if !excluded {
                                                let line: EventOccurrencePayload = EventOccurrencePayload(id: id, start: occStart, end: occEnd, nth: (try Checked.narrow(nth, to: UInt32.self)), conflict: mark)
                                                out.append(line)
                                                emitted = (try Checked.add(emitted, 1))
                                            }
                                        }
                                        nth = (try Checked.add(nth, 1))
                                    }
                                }
                            }
                        }
                        if emitted >= 64 {
                            going = false
                        }
                        i = (try Checked.add(i, 1))
                    }
                    if freq == 0 {
                        going = false
                    }
                }
                p = (try Checked.add(p, 1))
            }
        }
        return out
    }
}
