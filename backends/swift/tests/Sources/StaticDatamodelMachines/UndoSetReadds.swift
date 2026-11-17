// SCE-GENERATED — DO NOT EDIT
// source-hash: 451ed84f90f73b42f48909eca6f7a302d6cb0b852dd68c846f5e713406957876
// SCE-MAP: undo_set_readds.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `UndoSetReadds`, a file of the
// module that calls it: a call is qualified by the enum
// (`UndoSetReadds.undoSetReadds(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum UndoSetReadds {
    public static func undoSetReadds(_ batchTombstones: [OrsetEntryPayload], _ live: [OrsetEntryPayload]) throws(AlgorithmError) -> [UInt64] {
        if !(batchTombstones.count <= 256) {
            throw AlgorithmError.precondition
        }
        var out: [UInt64] = []
        out.reserveCapacity(256)
        for t in batchTombstones {
            var held: Bool = false
            for l in live {
                if l.element == t.element {
                    held = true
                }
            }
            var listed: Bool = false
            var i: UInt32 = 0
            while i < UInt32(truncatingIfNeeded: out.count) {
                if (try Checked.at(out, i)) == t.element {
                    listed = true
                }
                i = (try Checked.add(i, 1))
            }
            if !held && !listed {
                out.append(t.element)
            }
        }
        return out
    }
}
