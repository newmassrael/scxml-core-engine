// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: sheet_show.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sheet_show`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sheet_show

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_lww_cell "github.com/newmassrael/sce-go-tests/integration/static_datamodel/lww_cell"
	sce_sheet_shown "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sheet_shown"
)

func SheetShow(cells []sce_lww_cell.LwwCellPayload, trail []int64) (sceValue []sce_sheet_shown.SheetShownPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_sheet_shown.SheetShownPayload, 0, 256)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var started bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var lastRow uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var lastCol uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var more bool = true
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := more
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        var chosen bool = false
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestRow uint64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestCol uint64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var bestAt int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var cursor int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for _, s := range cells {
            if sceCond3 := (!started || s.Row > lastRow || s.Row == lastRow && s.Col > lastCol) && (!chosen || s.Row < bestRow || s.Row == bestRow && s.Col < bestCol); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                chosen = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestRow = s.Row;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestCol = s.Col;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bestAt = cursor;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            cursor = scealgorithm.AddInt64(&sceFailure, cursor, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        if sceCond2 := chosen; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            var shownValue int64 = 0
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var shownFault uint8 = 1
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var u int64 = 0
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            for {
                sceCond3 := u < int64(len(trail))
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                if !sceCond3 {
                    break
                }
                if sceCond4 := scealgorithm.At(&sceFailure, trail[:], int64(u)) == bestAt; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    shownValue = scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, u, 1)));
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    shownFault = scealgorithm.NarrowUint8FromInt(&sceFailure, int64(scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, u, 2)))));
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                u = scealgorithm.AddInt64(&sceFailure, u, 3);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            line := sce_sheet_shown.SheetShownPayload{Row: bestRow, Col: bestCol, Value: shownValue, Fault: shownFault}
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            out = append(out, line)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            started = true;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            lastRow = bestRow;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            lastCol = bestCol;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        } else {
            more = false;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_sheet_shown.SheetShownPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
