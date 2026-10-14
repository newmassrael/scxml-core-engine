// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
// SCE-MAP: rga_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `rga_insert`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package rga_insert

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_rga_node "github.com/newmassrael/sce-go-tests/integration/static_datamodel/rga_node"
)

func RgaInsert(nodes []sce_rga_node.RgaNodePayload, node sce_rga_node.RgaNodePayload) (sceValue []sce_rga_node.RgaNodePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_rga_node.RgaNodePayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var held bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, probe := range nodes {
        if sceCond2 := probe.Key == node.Key; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    if sceCond1 := held; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        for _, same := range nodes {
            out = append(out, same)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        {
            var sceReturned []sce_rga_node.RgaNodePayload = out
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            return sceReturned, nil
        }
    }
    var phase uint8 = func() uint8 { if node.After == 0 { return 1 }; return 0 }()
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, n := range nodes {
        if sceCond2 := phase == 0; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, n)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := n.Key == node.After; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                phase = 1;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        } else {
            if sceCond3 := phase == 1; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                if sceCond4 := n.Key > node.Key; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    out = append(out, n)
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                } else {
                    out = append(out, node)
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    out = append(out, n)
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    phase = 2;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
            } else {
                out = append(out, n)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := phase != 2; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        out = append(out, node)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_rga_node.RgaNodePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
