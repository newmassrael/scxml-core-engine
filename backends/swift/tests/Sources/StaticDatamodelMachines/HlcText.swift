// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
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
    public static func hlcText(_ ts: HlcTimestampPayload) throws(AlgorithmError) -> [UInt8] {
        if !(ts.wallTime >= 0 && ts.wallTime <= 253402300799999 && ts.counter <= 65535) {
            throw AlgorithmError.precondition
        }
        var out: [UInt8] = []
        out.reserveCapacity(46)
        let date: CivilDatePayload = (try CivilFromDays.civilFromDays((try Checked.div(ts.wallTime, 86400000))))
        let ms: Int64 = (try Checked.rem(ts.wallTime, 86400000))
        var value: Int64 = Int64(date.year)
        var unit: Int64 = 1000
        var field: UInt32 = 0
        while field < 7 {
            while unit > 0 {
                let digit: UInt8 = (try Checked.narrow((try Checked.add(48, (try Checked.rem((try Checked.div(value, unit)), 10)))), to: UInt8.self))
                out.append(digit)
                unit = (try Checked.div(unit, 10))
            }
            let separator: UInt8 = field < 2 ? 45 : (field == 2 ? 84 : (field < 5 ? 58 : (field == 5 ? 46 : 90)))
            out.append(separator)
            field = (try Checked.add(field, 1))
            value = field == 1 ? Int64(date.month) : (field == 2 ? Int64(date.day) : (field == 3 ? (try Checked.div(ms, 3600000)) : (field == 4 ? (try Checked.rem((try Checked.div(ms, 60000)), 60)) : (field == 5 ? (try Checked.rem((try Checked.div(ms, 1000)), 60)) : (try Checked.rem(ms, 1000))))))
            unit = field == 6 ? 100 : 10
        }
        let dash: UInt8 = 45
        out.append(dash)
        var shift: UInt32 = 12
        var nibble: UInt8 = 0
        var more: Bool = true
        while more {
            nibble = (try Checked.narrow((ts.counter >> shift) & 15, to: UInt8.self))
            let upper: UInt8 = nibble < 10 ? (try Checked.add(48, nibble)) : (try Checked.add(55, nibble))
            out.append(upper)
            more = shift > 0
            if more {
                shift = (try Checked.sub(shift, 4))
            }
        }
        out.append(dash)
        shift = 60
        more = true
        while more {
            nibble = (try Checked.narrow((ts.nodeId >> shift) & 15, to: UInt8.self))
            let lower: UInt8 = nibble < 10 ? (try Checked.add(48, nibble)) : (try Checked.add(87, nibble))
            out.append(lower)
            more = shift > 0
            if more {
                shift = (try Checked.sub(shift, 4))
            }
        }
        return out
    }
}
