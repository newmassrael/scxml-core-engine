// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: hlc_text.scxml:28 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `HlcText`, a file of the
// module that calls it: a call is qualified by the enum
// (`HlcText.hlcText(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum HlcText {
    public static func hlcText(_ ts: HlcTimestampPayload) throws -> [UInt8] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceCond2 = !(ts.wallTime >= 0 && ts.wallTime <= 253402300799999 && ts.counter <= 65535)
            try sceFailure.throwIfFailed()
            if sceCond2 {
                throw AlgorithmError.precondition
            }
        }
        var out: [UInt8] = []
        out.reserveCapacity(46)
        try sceFailure.throwIfFailed()
        let date: CivilDatePayload = Checked.takeDefault(sceFailure, { try CivilFromDays.civilFromDays(Checked.div(sceFailure, ts.wallTime, 86400000)) })
        try sceFailure.throwIfFailed()
        let ms: Int64 = Checked.rem(sceFailure, ts.wallTime, 86400000)
        try sceFailure.throwIfFailed()
        var value: Int64 = Int64(date.year)
        try sceFailure.throwIfFailed()
        var unit: Int64 = 1000
        try sceFailure.throwIfFailed()
        var field: UInt32 = 0
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = field < 7
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            while true {
                let sceCond3 = unit > 0
                try sceFailure.throwIfFailed()
                if !sceCond3 { break }
                let digit: UInt8 = Checked.narrow(sceFailure, Checked.add(sceFailure, 48, Checked.rem(sceFailure, Checked.div(sceFailure, value, unit), 10)), to: UInt8.self)
                try sceFailure.throwIfFailed()
                out.append(digit)
                try sceFailure.throwIfFailed()
                unit = Checked.div(sceFailure, unit, 10)
                try sceFailure.throwIfFailed()
            }
            let separator: UInt8 = field < 2 ? 45 : (field == 2 ? 84 : (field < 5 ? 58 : (field == 5 ? 46 : 90)))
            try sceFailure.throwIfFailed()
            out.append(separator)
            try sceFailure.throwIfFailed()
            field = Checked.add(sceFailure, field, 1)
            try sceFailure.throwIfFailed()
            value = field == 1 ? Int64(date.month) : (field == 2 ? Int64(date.day) : (field == 3 ? Checked.div(sceFailure, ms, 3600000) : (field == 4 ? Checked.rem(sceFailure, Checked.div(sceFailure, ms, 60000), 60) : (field == 5 ? Checked.rem(sceFailure, Checked.div(sceFailure, ms, 1000), 60) : Checked.rem(sceFailure, ms, 1000)))))
            try sceFailure.throwIfFailed()
            unit = field == 6 ? 100 : 10
            try sceFailure.throwIfFailed()
        }
        let dash: UInt8 = 45
        try sceFailure.throwIfFailed()
        out.append(dash)
        try sceFailure.throwIfFailed()
        var shift: UInt32 = 12
        try sceFailure.throwIfFailed()
        var nibble: UInt8 = 0
        try sceFailure.throwIfFailed()
        var more: Bool = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            nibble = Checked.narrow(sceFailure, (ts.counter >> shift) & 15, to: UInt8.self)
            try sceFailure.throwIfFailed()
            let upper: UInt8 = nibble < 10 ? Checked.add(sceFailure, 48, nibble) : Checked.add(sceFailure, 55, nibble)
            try sceFailure.throwIfFailed()
            out.append(upper)
            try sceFailure.throwIfFailed()
            more = shift > 0
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = more
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    shift = Checked.sub(sceFailure, shift, 4)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        out.append(dash)
        try sceFailure.throwIfFailed()
        shift = 60
        try sceFailure.throwIfFailed()
        more = true
        try sceFailure.throwIfFailed()
        while true {
            let sceCond2 = more
            try sceFailure.throwIfFailed()
            if !sceCond2 { break }
            nibble = Checked.narrow(sceFailure, (ts.nodeId >> shift) & 15, to: UInt8.self)
            try sceFailure.throwIfFailed()
            let lower: UInt8 = nibble < 10 ? Checked.add(sceFailure, 48, nibble) : Checked.add(sceFailure, 87, nibble)
            try sceFailure.throwIfFailed()
            out.append(lower)
            try sceFailure.throwIfFailed()
            more = shift > 0
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = more
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    shift = Checked.sub(sceFailure, shift, 4)
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceReturned: [UInt8] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
