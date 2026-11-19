// SCE-GENERATED — DO NOT EDIT
// source-hash: e6b5e4359493e8a6138fc17278067a929ce4c2b14eb720c4c67c4682c61d11da
// source: sce:std/mesh/causal_lacks.scxml
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
