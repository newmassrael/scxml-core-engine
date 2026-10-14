// SCE-GENERATED — DO NOT EDIT
// source-hash: 0022d2f7097a4a1eb0e7ba717e1da8948cc1ba80ab5d45ced6c3b314777dd2db
// SCE-MAP: merkle_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `merkle_insert`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package merkle_insert

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_merkle_node "github.com/newmassrael/sce-go-tests/integration/static_datamodel/merkle_node"
)

func MerkleInsert(nodes []sce_merkle_node.MerkleNodePayload, minute int64, hash uint32) (sceValue []sce_merkle_node.MerkleNodePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(minute >= 0); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    out := make([]sce_merkle_node.MerkleNodePayload, 0, 1024)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var pd uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var pp uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var div int64 = 14348907
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var prevDepth int32 = -1
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var prevPrefix uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, x := range nodes {
        if sceCond2 := !(x.Depth <= 16 && (int32(x.Depth) > prevDepth || int32(x.Depth) == prevDepth && x.Prefix > prevPrefix)); sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            return sceValue, scealgorithm.Precondition
        }
        prevDepth = int32(x.Depth);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        prevPrefix = x.Prefix;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for {
            sceCond2 := pd <= 16 && (pd < uint32(x.Depth) || pd == uint32(x.Depth) && pp < x.Prefix)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if !sceCond2 {
                break
            }
            created := sce_merkle_node.MerkleNodePayload{Depth: scealgorithm.NarrowUint8FromUint(&sceFailure, uint64(pd)), Prefix: pp, Hash: hash}
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            out = append(out, created)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            pd = scealgorithm.AddUint32(&sceFailure, pd, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            pp = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(scealgorithm.DivInt64(&sceFailure, minute, div)));
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := div > 1; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                div = scealgorithm.DivInt64(&sceFailure, div, 3);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := pd <= 16 && pd == uint32(x.Depth) && pp == x.Prefix; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            merged := sce_merkle_node.MerkleNodePayload{Depth: x.Depth, Prefix: x.Prefix, Hash: x.Hash ^ hash}
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            out = append(out, merged)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            pd = scealgorithm.AddUint32(&sceFailure, pd, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            pp = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(scealgorithm.DivInt64(&sceFailure, minute, div)));
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := div > 1; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                div = scealgorithm.DivInt64(&sceFailure, div, 3);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        } else {
            out = append(out, x)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    for {
        sceCond1 := pd <= 16
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        tail := sce_merkle_node.MerkleNodePayload{Depth: scealgorithm.NarrowUint8FromUint(&sceFailure, uint64(pd)), Prefix: pp, Hash: hash}
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        out = append(out, tail)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        pd = scealgorithm.AddUint32(&sceFailure, pd, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        pp = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(scealgorithm.DivInt64(&sceFailure, minute, div)));
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := div > 1; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            div = scealgorithm.DivInt64(&sceFailure, div, 3);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_merkle_node.MerkleNodePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
