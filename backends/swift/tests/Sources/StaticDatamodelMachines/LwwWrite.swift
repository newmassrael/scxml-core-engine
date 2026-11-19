// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fbddaacb0929e8ad90869b26ffa965fb426df54d709f78595861c487419adf5
// source: sce:std/merge/lww_write.scxml
// SCE-MAP: lww_write.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `LwwWrite`, a file of the
// module that calls it: a call is qualified by the enum
// (`LwwWrite.lwwWrite(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum LwwWrite {
    public static func lwwWrite(_ cells: [LwwCellPayload], _ write: LwwCellPayload) throws(AlgorithmError) -> [LwwCellPayload] {
        var out: [LwwCellPayload] = []
        out.reserveCapacity(256)
        var held: Bool = false
        let incoming: HlcTimestampPayload = HlcTimestampPayload(wallTime: write.wallTime, counter: write.counter, nodeId: write.nodeId)
        for c in cells {
            if c.row == write.row && c.col == write.col {
                held = true
                let stored: HlcTimestampPayload = HlcTimestampPayload(wallTime: c.wallTime, counter: c.counter, nodeId: c.nodeId)
                if HlcCompare.hlcCompare(incoming, stored) > 0 {
                    out.append(write)
                } else {
                    out.append(c)
                }
            } else {
                out.append(c)
            }
        }
        if !held {
            out.append(write)
        }
        return out
    }
}
