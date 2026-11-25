// SCE-GENERATED — DO NOT EDIT
// source-hash: 770bd3897d32ee4fd3e4fdc69c16b5078e06eea73b37b69c9e7aca118c8fa25a
// source: algorithm_narrow_shift.scxml
// SCE-MAP: algorithm_narrow_shift.scxml:22 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmNarrowShift`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmNarrowShift.algorithmNarrowShift(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmNarrowShift {
    public static func algorithmNarrowShift(_ x: UInt8, _ h: Int16, _ n: UInt8, _ op: UInt8) -> Int64 {
        var r: Int64 = 0
        if op == 0 {
            r = Int64(x << n)
        }
        if op == 1 {
            r = Int64((x << n) >> n)
        }
        if op == 2 {
            r = Int64(~x)
        }
        if op == 3 {
            r = Int64(h << n)
        }
        if op == 4 {
            r = Int64((h << n) >> n)
        }
        if op == 5 {
            r = Int64(~h)
        }
        return r
    }
}
