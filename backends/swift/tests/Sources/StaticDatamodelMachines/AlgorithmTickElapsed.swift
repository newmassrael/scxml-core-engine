// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_tick_elapsed.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTickElapsed`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTickElapsed.algorithmTickElapsed(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTickElapsed {
    public static func algorithmTickElapsed(_ now: UInt32, _ last: UInt32, _ timeout: UInt32) -> Bool {
        return Wrapping.sub(now, last) >= timeout
    }
}
