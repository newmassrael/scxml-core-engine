// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f
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
    public static func eventAgenda(_ s: EventSchedulePayload, _ values: [MvValuePayload], _ id: UInt64, _ windowStart: Int64, _ windowEnd: Int64, _ skip: UInt32) throws -> [EventOccurrencePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [EventOccurrencePayload] = []
        out.reserveCapacity(64)
        try sceFailure.throwIfFailed()
        var emitted: Int64 = 0
        try sceFailure.throwIfFailed()
        let day: Int64 = 86400000
        try sceFailure.throwIfFailed()
        let skipped: Int64 = Int64(skip)
        try sceFailure.throwIfFailed()
        let startAt: Int64 = s.start
        try sceFailure.throwIfFailed()
        let freq: Int64 = s.freq
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = startAt >= 0 && s.gone != 1 && freq >= 0 && freq <= 4 && windowEnd > windowStart && windowEnd > 0
            try sceFailure.throwIfFailed()
            if sceCond2 {
                let every: Int64 = s.interval < 1 ? 1 : s.interval
                try sceFailure.throwIfFailed()
                let length: Int64 = s.length
                try sceFailure.throwIfFailed()
                let count: Int64 = s.count
                try sceFailure.throwIfFailed()
                let until: Int64 = s.until
                try sceFailure.throwIfFailed()
                let hasSP: Bool = (s.spPos | s.spNeg) != 0
                try sceFailure.throwIfFailed()
                let timeOfDay: Int64 = Checked.rem(sceFailure, startAt, day)
                try sceFailure.throwIfFailed()
                let startDay: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, startAt, timeOfDay), day)
                try sceFailure.throwIfFailed()
                let endDay: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, windowEnd, day), 1), day)
                try sceFailure.throwIfFailed()
                let startWeekday: Int64 = Checked.rem(sceFailure, Checked.add(sceFailure, Checked.rem(sceFailure, startDay, 7), 11), 7)
                try sceFailure.throwIfFailed()
                let wkst0: Int64 = s.wkst
                try sceFailure.throwIfFailed()
                let wkst: Int64 = wkst0 >= 0 && wkst0 <= 6 ? wkst0 : 1
                try sceFailure.throwIfFailed()
                let week0Start: Int64 = Checked.sub(sceFailure, startDay, Checked.rem(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, startWeekday, wkst), 7), 7))
                try sceFailure.throwIfFailed()
                let z: Int64 = Checked.add(sceFailure, startDay, 719468)
                try sceFailure.throwIfFailed()
                let era: Int64 = Checked.div(sceFailure, z, 146097)
                try sceFailure.throwIfFailed()
                let doe: Int64 = Checked.sub(sceFailure, z, Checked.mul(sceFailure, era, 146097))
                try sceFailure.throwIfFailed()
                let yoe: Int64 = Checked.div(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, doe, Checked.div(sceFailure, doe, 1460)), Checked.div(sceFailure, doe, 36524)), Checked.div(sceFailure, doe, 146096)), 365)
                try sceFailure.throwIfFailed()
                let doy: Int64 = Checked.sub(sceFailure, doe, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 365, yoe), Checked.div(sceFailure, yoe, 4)), Checked.div(sceFailure, yoe, 100)))
                try sceFailure.throwIfFailed()
                let mp: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 5, doy), 2), 153)
                try sceFailure.throwIfFailed()
                let m0: Int64 = mp < 10 ? Checked.add(sceFailure, mp, 3) : Checked.sub(sceFailure, mp, 9)
                try sceFailure.throwIfFailed()
                let y0: Int64 = m0 <= 2 ? Checked.add(sceFailure, Checked.add(sceFailure, yoe, Checked.mul(sceFailure, era, 400)), 1) : Checked.add(sceFailure, yoe, Checked.mul(sceFailure, era, 400))
                try sceFailure.throwIfFailed()
                let months0: Int64 = Checked.sub(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, y0, 12), m0), 1)
                try sceFailure.throwIfFailed()
                var p: Int64 = 0
                try sceFailure.throwIfFailed()
                var nth: Int64 = 0
                try sceFailure.throwIfFailed()
                var going: Bool = true
                try sceFailure.throwIfFailed()
                while true {
                    let sceCond4 = going
                    try sceFailure.throwIfFailed()
                    if !sceCond4 { break }
                    var ps: Int64 = startDay
                    try sceFailure.throwIfFailed()
                    var plen: Int64 = 1
                    try sceFailure.throwIfFailed()
                    do {
                        let sceCond5 = freq == 1
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            ps = Checked.add(sceFailure, startDay, Checked.mul(sceFailure, p, every))
                            try sceFailure.throwIfFailed()
                        }
                    }
                    do {
                        let sceCond5 = freq == 2
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            ps = Checked.add(sceFailure, week0Start, Checked.mul(sceFailure, Checked.mul(sceFailure, p, every), 7))
                            try sceFailure.throwIfFailed()
                            plen = 7
                            try sceFailure.throwIfFailed()
                        }
                    }
                    do {
                        let sceCond5 = freq == 3 || freq == 4
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            let pty: Int64 = freq == 3 ? Checked.div(sceFailure, Checked.add(sceFailure, months0, Checked.mul(sceFailure, p, every)), 12) : Checked.add(sceFailure, y0, Checked.mul(sceFailure, p, every))
                            try sceFailure.throwIfFailed()
                            let ptm: Int64 = freq == 3 ? Checked.add(sceFailure, Checked.rem(sceFailure, Checked.add(sceFailure, months0, Checked.mul(sceFailure, p, every)), 12), 1) : 1
                            try sceFailure.throwIfFailed()
                            let pleap: Bool = Checked.rem(sceFailure, pty, 4) == 0 && Checked.rem(sceFailure, pty, 100) != 0 || Checked.rem(sceFailure, pty, 400) == 0
                            try sceFailure.throwIfFailed()
                            let pcy: Int64 = ptm <= 2 ? Checked.sub(sceFailure, pty, 1) : pty
                            try sceFailure.throwIfFailed()
                            let pcera: Int64 = Checked.div(sceFailure, pcy, 400)
                            try sceFailure.throwIfFailed()
                            let pcyoe: Int64 = Checked.sub(sceFailure, pcy, Checked.mul(sceFailure, pcera, 400))
                            try sceFailure.throwIfFailed()
                            let pcmp: Int64 = ptm > 2 ? Checked.sub(sceFailure, ptm, 3) : Checked.add(sceFailure, ptm, 9)
                            try sceFailure.throwIfFailed()
                            let pcdoy: Int64 = Checked.div(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, 153, pcmp), 2), 5)
                            try sceFailure.throwIfFailed()
                            ps = Checked.sub(sceFailure, Checked.add(sceFailure, Checked.sub(sceFailure, Checked.add(sceFailure, Checked.add(sceFailure, Checked.mul(sceFailure, pcera, 146097), Checked.mul(sceFailure, pcyoe, 365)), Checked.div(sceFailure, pcyoe, 4)), Checked.div(sceFailure, pcyoe, 100)), pcdoy), 719468)
                            try sceFailure.throwIfFailed()
                            plen = freq == 4 ? (pleap ? 366 : 365) : (ptm == 2 ? (pleap ? 29 : 28) : (ptm == 4 || ptm == 6 || ptm == 9 || ptm == 11 ? 30 : 31))
                            try sceFailure.throwIfFailed()
                        }
                    }
                    do {
                        let sceCond5 = ps >= endDay || until > 0 && Checked.mul(sceFailure, ps, day) > until
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            going = false
                            try sceFailure.throwIfFailed()
                        } else {
                            var n: Int64 = 0
                            try sceFailure.throwIfFailed()
                            var k: Int64 = 0
                            try sceFailure.throwIfFailed()
                            while true {
                                let sceCond7 = k < plen
                                try sceFailure.throwIfFailed()
                                if !sceCond7 { break }
                                do {
                                    let sceCond8 = Checked.take(sceFailure, fallback: false, { try EventSelects.eventSelects(s, Checked.add(sceFailure, ps, k), startDay) })
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        n = Checked.add(sceFailure, n, 1)
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                                k = Checked.add(sceFailure, k, 1)
                                try sceFailure.throwIfFailed()
                            }
                            var i: Int64 = 0
                            try sceFailure.throwIfFailed()
                            var rank: Int64 = 0
                            try sceFailure.throwIfFailed()
                            while true {
                                let sceCond7 = i < plen && going
                                try sceFailure.throwIfFailed()
                                if !sceCond7 { break }
                                let dd: Int64 = Checked.add(sceFailure, ps, i)
                                try sceFailure.throwIfFailed()
                                do {
                                    let sceCond8 = Checked.take(sceFailure, fallback: false, { try EventSelects.eventSelects(s, dd, startDay) })
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        rank = Checked.add(sceFailure, rank, 1)
                                        try sceFailure.throwIfFailed()
                                        var keep: Bool = true
                                        try sceFailure.throwIfFailed()
                                        do {
                                            let sceCond10 = hasSP
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                keep = rank <= 62 && ((s.spPos >> rank) & 1) == 1 || Checked.add(sceFailure, Checked.sub(sceFailure, n, rank), 1) <= 62 && ((s.spNeg >> Checked.add(sceFailure, Checked.sub(sceFailure, n, rank), 1)) & 1) == 1
                                                try sceFailure.throwIfFailed()
                                            }
                                        }
                                        do {
                                            let sceCond10 = keep && dd >= startDay
                                            try sceFailure.throwIfFailed()
                                            if sceCond10 {
                                                let occStart: Int64 = Checked.add(sceFailure, Checked.mul(sceFailure, dd, day), timeOfDay)
                                                try sceFailure.throwIfFailed()
                                                let occEnd: Int64 = Checked.add(sceFailure, occStart, length)
                                                try sceFailure.throwIfFailed()
                                                do {
                                                    let sceCond12 = occStart >= windowEnd || until > 0 && occStart > until
                                                    try sceFailure.throwIfFailed()
                                                    if sceCond12 {
                                                        going = false
                                                        try sceFailure.throwIfFailed()
                                                    } else {
                                                        do {
                                                            let sceCond14 = count > 0 && nth >= count
                                                            try sceFailure.throwIfFailed()
                                                            if sceCond14 {
                                                                going = false
                                                                try sceFailure.throwIfFailed()
                                                            } else {
                                                                do {
                                                                    let sceCond16 = (occEnd > windowStart || occStart >= windowStart) && emitted < 64 && nth >= skipped
                                                                    try sceFailure.throwIfFailed()
                                                                    if sceCond16 {
                                                                        var excluded: Bool = false
                                                                        try sceFailure.throwIfFailed()
                                                                        var mark: UInt8 = Checked.narrow(sceFailure, s.flag, to: UInt8.self)
                                                                        try sceFailure.throwIfFailed()
                                                                        do {
                                                                            let sceCond18 = dd >= 0 && dd < 65472
                                                                            try sceFailure.throwIfFailed()
                                                                            if sceCond18 {
                                                                                let xfield: UInt64 = Checked.narrow(sceFailure, Checked.add(sceFailure, dd, 64), to: UInt64.self)
                                                                                try sceFailure.throwIfFailed()
                                                                                var xfound: Bool = false
                                                                                try sceFailure.throwIfFailed()
                                                                                var xheld: Int64 = 0
                                                                                try sceFailure.throwIfFailed()
                                                                                var xw: Int64 = 0
                                                                                try sceFailure.throwIfFailed()
                                                                                var xc: UInt32 = 0
                                                                                try sceFailure.throwIfFailed()
                                                                                var xn: UInt64 = 0
                                                                                try sceFailure.throwIfFailed()
                                                                                var xv: Int64 = 0
                                                                                try sceFailure.throwIfFailed()
                                                                                for wx in values {
                                                                                    do {
                                                                                        let sceCond21 = wx.id == id && wx.field == xfield
                                                                                        try sceFailure.throwIfFailed()
                                                                                        if sceCond21 {
                                                                                            xheld = Checked.add(sceFailure, xheld, 1)
                                                                                            try sceFailure.throwIfFailed()
                                                                                            do {
                                                                                                let sceCond23 = !xfound || EventLater.eventLater(wx.wallTime, wx.counter, wx.nodeId, xw, xc, xn)
                                                                                                try sceFailure.throwIfFailed()
                                                                                                if sceCond23 {
                                                                                                    xfound = true
                                                                                                    try sceFailure.throwIfFailed()
                                                                                                    xw = wx.wallTime
                                                                                                    try sceFailure.throwIfFailed()
                                                                                                    xc = wx.counter
                                                                                                    try sceFailure.throwIfFailed()
                                                                                                    xn = wx.nodeId
                                                                                                    try sceFailure.throwIfFailed()
                                                                                                    xv = wx.value
                                                                                                    try sceFailure.throwIfFailed()
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                                excluded = xfound && xv == 1
                                                                                try sceFailure.throwIfFailed()
                                                                                do {
                                                                                    let sceCond20 = xheld > 1
                                                                                    try sceFailure.throwIfFailed()
                                                                                    if sceCond20 {
                                                                                        mark = 1
                                                                                        try sceFailure.throwIfFailed()
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                        do {
                                                                            let sceCond18 = !excluded
                                                                            try sceFailure.throwIfFailed()
                                                                            if sceCond18 {
                                                                                let line: EventOccurrencePayload = EventOccurrencePayload(id: id, start: occStart, end: occEnd, nth: Checked.narrow(sceFailure, nth, to: UInt32.self), conflict: mark)
                                                                                try sceFailure.throwIfFailed()
                                                                                out.append(line)
                                                                                try sceFailure.throwIfFailed()
                                                                                emitted = Checked.add(sceFailure, emitted, 1)
                                                                                try sceFailure.throwIfFailed()
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                                nth = Checked.add(sceFailure, nth, 1)
                                                                try sceFailure.throwIfFailed()
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                do {
                                    let sceCond8 = emitted >= 64
                                    try sceFailure.throwIfFailed()
                                    if sceCond8 {
                                        going = false
                                        try sceFailure.throwIfFailed()
                                    }
                                }
                                i = Checked.add(sceFailure, i, 1)
                                try sceFailure.throwIfFailed()
                            }
                            do {
                                let sceCond7 = freq == 0
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    going = false
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        }
                    }
                    p = Checked.add(sceFailure, p, 1)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [EventOccurrencePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
