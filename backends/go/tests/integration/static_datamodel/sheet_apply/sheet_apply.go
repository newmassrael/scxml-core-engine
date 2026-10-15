// SCE-GENERATED — DO NOT EDIT
// source-hash: 58e52c40de99a046030dfe178fb69ffa932b0a6a8146307d7430d1e6de80a470
// SCE-MAP: sheet_apply.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sheet_apply`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sheet_apply

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_sheet_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sheet_value"
)

func SheetApply(op uint8, a int64, b int64) (sceValue sce_sheet_value.SheetValuePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var top int64 = 9223372036854775807
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var bottom int64 = scealgorithm.SubInt64(&sceFailure, scealgorithm.NegInt64(&sceFailure, top), 1)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var value int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var fault uint8 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := op == 2; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        if sceCond2 := b > 0 && a > scealgorithm.SubInt64(&sceFailure, top, b) || b < 0 && a < scealgorithm.SubInt64(&sceFailure, bottom, b); sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            fault = 3;
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        } else {
            value = scealgorithm.AddInt64(&sceFailure, a, b);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    } else {
        if sceCond2 := op == 3; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            if sceCond3 := b < 0 && a > scealgorithm.AddInt64(&sceFailure, top, b) || b > 0 && a < scealgorithm.AddInt64(&sceFailure, bottom, b); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                fault = 3;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            } else {
                value = scealgorithm.SubInt64(&sceFailure, a, b);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        } else {
            if sceCond3 := op == 4; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                if sceCond4 := a == 0 || b == 0; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    value = 0;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                } else {
                    if sceCond5 := a > 0 && b > 0 && a > scealgorithm.DivInt64(&sceFailure, top, b) || a > 0 && b < 0 && b < scealgorithm.DivInt64(&sceFailure, bottom, a) || a < 0 && b > 0 && a < scealgorithm.DivInt64(&sceFailure, bottom, b) || a < 0 && b < 0 && b < scealgorithm.DivInt64(&sceFailure, top, a); sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        fault = 3;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        value = scealgorithm.MulInt64(&sceFailure, a, b);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    }
                }
            } else {
                if sceCond4 := op == 5; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    if sceCond5 := b == 0; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        fault = 2;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        if sceCond6 := a == bottom && b == -1; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            fault = 3;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        } else {
                            value = scealgorithm.DivInt64(&sceFailure, a, b);
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                    }
                } else {
                    fault = 5;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
            }
        }
    }
    result := sce_sheet_value.SheetValuePayload{Value: value, Fault: fault}
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    {
        var sceReturned sce_sheet_value.SheetValuePayload = result
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
