// SCE-GENERATED — DO NOT EDIT
// source-hash: e0db191c33f6320574ec0932251d0bc53e81d78915dbb25f9f9c999c3f62dbc4
// source: algorithm_shift_count.scxml
// SCE-MAP: algorithm_shift_count.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmShiftCount`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmShiftCount.algorithmShiftCount(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmShiftCount {
    public static func algorithmShiftCount(_ x: UInt8, _ h: Int16, _ w: UInt32, _ s: Int32, _ q: UInt64, _ l: Int64, _ n: UInt8, _ op: UInt8) -> Int64 {
        var r: Int64 = 0
        if op == 0 {
            r = Int64(x << n)
        }
        if op == 1 {
            r = Int64(x >> n)
        }
        if op == 2 {
            r = Int64(h << n)
        }
        if op == 3 {
            r = Int64(h >> n)
        }
        if op == 4 {
            r = Int64(Int16(truncatingIfNeeded: UInt16(truncatingIfNeeded: h) >> n))
        }
        if op == 5 {
            r = Int64(w << n)
        }
        if op == 6 {
            r = Int64(w >> n)
        }
        if op == 7 {
            r = Int64(s << n)
        }
        if op == 8 {
            r = Int64(s >> n)
        }
        if op == 9 {
            r = Int64(Int32(truncatingIfNeeded: UInt32(truncatingIfNeeded: s) >> n))
        }
        if op == 10 {
            r = Int64(truncatingIfNeeded: q << n & 9223372036854775807)
        }
        if op == 11 {
            r = Int64(truncatingIfNeeded: q >> n & 9223372036854775807)
        }
        if op == 12 {
            r = l << n
        }
        if op == 13 {
            r = l >> n
        }
        if op == 14 {
            r = Int64(truncatingIfNeeded: UInt64(truncatingIfNeeded: l) >> n)
        }
        if op == 15 && n < 32 {
            r = Int64(s << n)
        }
        return r
    }
}
