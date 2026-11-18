// SCE-GENERATED — DO NOT EDIT
// source-hash: 228574de0bc19d5365ce601baf2f5ab947df25fe447cdc72ac9bd281728c1abf
// SCE-MAP: causal_lacks.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CausalLacks`, a file of the
// module that calls it: a call is qualified by the enum
// (`CausalLacks.causalLacks(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum CausalLacks {
    public static func causalLacks(_ vector: [CausalEntryPayload], _ dot: CausalEntryPayload) -> Bool {
        var seen: UInt64 = 0
        for entry in vector {
            if entry.hi == dot.hi && entry.lo == dot.lo {
                seen = entry.seq
            }
        }
        return dot.seq > seen
    }
}
