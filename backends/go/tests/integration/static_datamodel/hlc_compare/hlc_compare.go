// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: hlc_compare.scxml:18 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `hlc_compare`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package hlc_compare

import (
	sce_hlc_timestamp "github.com/newmassrael/sce-go-tests/integration/static_datamodel/hlc_timestamp"
)

func HlcCompare(a sce_hlc_timestamp.HlcTimestampPayload, b sce_hlc_timestamp.HlcTimestampPayload) int32 {
    if (a.WallTime != b.WallTime) {
        return func() int32 { if a.WallTime < b.WallTime { return -1 }; return 1 }()
    }
    if (a.Counter != b.Counter) {
        return func() int32 { if a.Counter < b.Counter { return -1 }; return 1 }()
    }
    if (a.NodeId != b.NodeId) {
        return func() int32 { if a.NodeId < b.NodeId { return -1 }; return 1 }()
    }
    return 0
}
