// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
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
    public static func utf8Within(_ data: [UInt8], _ limit: UInt32) throws -> Bool {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        do {
            let sceReturned: Bool = Checked.take(sceFailure, fallback: 0, { try Utf8Length.utf8Length(data) }) <= limit
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
