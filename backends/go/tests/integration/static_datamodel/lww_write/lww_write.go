// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
// SCE-MAP: lww_write.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `lww_write`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package lww_write

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_lww_cell "github.com/newmassrael/sce-go-tests/integration/static_datamodel/lww_cell"
	sce_hlc_timestamp "github.com/newmassrael/sce-go-tests/integration/static_datamodel/hlc_timestamp"
	sce_hlc_compare "github.com/newmassrael/sce-go-tests/integration/static_datamodel/hlc_compare"
)

func LwwWrite(cells []sce_lww_cell.LwwCellPayload, write sce_lww_cell.LwwCellPayload) (sceValue []sce_lww_cell.LwwCellPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_lww_cell.LwwCellPayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var held bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    incoming := sce_hlc_timestamp.HlcTimestampPayload{WallTime: write.WallTime, Counter: write.Counter, NodeId: write.NodeId}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, c := range cells {
        if sceCond2 := c.Row == write.Row && c.Col == write.Col; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            stored := sce_hlc_timestamp.HlcTimestampPayload{WallTime: c.WallTime, Counter: c.Counter, NodeId: c.NodeId}
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := sce_hlc_compare.HlcCompare(incoming, stored) > 0; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                out = append(out, write)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            } else {
                out = append(out, c)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        } else {
            out = append(out, c)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    if sceCond1 := !held; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        out = append(out, write)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_lww_cell.LwwCellPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
