// SCE-GENERATED — DO NOT EDIT
// source-hash: 848898b5bbb016bf799fdf5a1235f8e512e7516e53f20e55bfd5887716ace728
// SCE-MAP: utf8_within.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `Utf8Within`, a file of the
// module that calls it: a call is qualified by the enum
// (`Utf8Within.utf8Within(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum Utf8Within {
    public static func utf8Within(_ data: [UInt8], _ limit: UInt32) throws(AlgorithmError) -> Bool {
        return (try Utf8Length.utf8Length(data)) <= limit
    }
}
