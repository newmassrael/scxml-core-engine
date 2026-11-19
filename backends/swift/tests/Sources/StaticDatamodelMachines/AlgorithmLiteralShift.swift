// SCE-GENERATED — DO NOT EDIT
// source-hash: 05c9c5105f4f53744c707e3bcc30045ab23368e39c745b42653d7ef8d652b913
// source: algorithm_literal_shift.scxml
// SCE-MAP: algorithm_literal_shift.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmLiteralShift`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmLiteralShift.algorithmLiteralShift(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmLiteralShift {
    public static func algorithmLiteralShift(_ prev: UInt64, _ m: UInt8, _ op: UInt8) -> UInt64 {
        var r: UInt64 = prev
        if op == 0 {
            r = prev | (UInt64(1) << m)
        }
        if op == 1 {
            r = prev ^ (UInt64(1) << m)
        }
        if op == 2 {
            r = prev & ~(UInt64(1) << m)
        }
        if op == 3 {
            r = ~(UInt64(1) << m)
        }
        if op == 4 {
            r = prev | (UInt64(4294967295) >> m)
        }
        if op == 5 {
            r = prev | (UInt64(1) << 40)
        }
        return r
    }
}
