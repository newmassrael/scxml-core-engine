// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
// SCE-MAP: sheet_settle.scxml:63 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `sheet_settle`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package sheet_settle

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_lww_cell "github.com/newmassrael/sce-go-tests/integration/static_datamodel/lww_cell"
	sce_rga_node "github.com/newmassrael/sce-go-tests/integration/static_datamodel/rga_node"
	sce_rga_gone "github.com/newmassrael/sce-go-tests/integration/static_datamodel/rga_gone"
	sce_sheet_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sheet_value"
	sce_sheet_apply "github.com/newmassrael/sce-go-tests/integration/static_datamodel/sheet_apply"
)

func SheetSettle(cells []sce_lww_cell.LwwCellPayload, rows []sce_rga_node.RgaNodePayload, gone []sce_rga_gone.RgaGonePayload, cols []sce_rga_node.RgaNodePayload, goneCols []sce_rga_gone.RgaGonePayload) (sceValue []int64, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    trail := make([]int64, 0, 768)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var moving bool = true
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var passes int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for {
        sceCond1 := moving && passes <= int64(len(cells))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if !sceCond1 {
            break
        }
        moving = false;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        passes = scealgorithm.AddInt64(&sceFailure, passes, 1);
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var at int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for _, c := range cells {
            var settled bool = false
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var d int64 = 0
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            for {
                sceCond3 := d < int64(len(trail))
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                if !sceCond3 {
                    break
                }
                if sceCond4 := scealgorithm.At(&sceFailure, trail[:], int64(d)) == at; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    settled = true;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                d = scealgorithm.AddInt64(&sceFailure, d, 3);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := !settled; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                var ready bool = true
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var v int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var f int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var va int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var fa int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var vb int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var fb int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var sides int64 = func() int64 { if c.Op >= 1 && c.Op <= 5 { return func() int64 { if c.Op == 1 { return 1 }; return 2 }() }; return 0 }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var side int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                for {
                    sceCond4 := side < sides
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    if !sceCond4 {
                        break
                    }
                    var isRef bool = func() bool { if side == 0 { return c.ARef }; return c.BRef }()
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var wantRow uint64 = func() uint64 { if side == 0 { return c.ARow }; return c.BRow }()
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var wantCol uint64 = func() uint64 { if side == 0 { return c.ACol }; return c.BCol }()
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var xv int64 = func() int64 { if side == 0 { return c.AVal }; return c.BVal }()
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var xf int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    if sceCond5 := isRef; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        xv = 0;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var found bool = false
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var holder int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var j int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        for _, probe := range cells {
                            if sceCond7 := !found && probe.Row == wantRow && probe.Col == wantCol; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                found = true;
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                holder = j;
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            }
                            j = scealgorithm.AddInt64(&sceFailure, j, 1);
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                        if sceCond6 := found; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            var hit bool = false
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var m int64 = 0
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            for {
                                sceCond7 := m < int64(len(trail))
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                if !sceCond7 {
                                    break
                                }
                                if sceCond8 := scealgorithm.At(&sceFailure, trail[:], int64(m)) == holder; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    hit = true;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                    xv = scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, m, 1)));
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                    xf = scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, m, 2)));
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                                m = scealgorithm.AddInt64(&sceFailure, m, 3);
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            }
                            if sceCond7 := !hit; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                ready = false;
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            }
                        }
                    }
                    if sceCond5 := side == 0; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        va = xv;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        fa = xf;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        vb = xv;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        fb = xf;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    }
                    side = scealgorithm.AddInt64(&sceFailure, side, 1);
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                if sceCond4 := c.Op == 0; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    v = c.Value;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                if sceCond4 := c.Op == 1; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    v = va;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    f = fa;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                if sceCond4 := c.Op >= 2 && c.Op <= 5; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    if sceCond5 := fa != 0 || fb != 0; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        f = func() int64 { if fa == 0 { return fb }; return func() int64 { if fb == 0 { return fa }; return func() int64 { if fa < fb { return fa }; return fb }() }() }();
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        applied := scealgorithm.Take[sce_sheet_value.SheetValuePayload](&sceFailure)(sce_sheet_apply.SheetApply(c.Op, va, vb))
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        v = applied.Value;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        f = int64(applied.Fault);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    }
                }
                if sceCond4 := c.Op > 6; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    f = 5;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                if sceCond4 := c.Op == 6; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    var ra int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var rb int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var ca int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var cb int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    var spin int64 = 0
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    for _, rw := range rows {
                        spin = scealgorithm.AddInt64(&sceFailure, spin, 1);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        if sceCond6 := rw.Key == c.ARow; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            ra = spin;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                        if sceCond6 := rw.Key == c.BRow; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            rb = spin;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                    }
                    spin = 0;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    for _, cl := range cols {
                        spin = scealgorithm.AddInt64(&sceFailure, spin, 1);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        if sceCond6 := cl.Key == c.ACol; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            ca = spin;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                        if sceCond6 := cl.Key == c.BCol; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            cb = spin;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                    }
                    if sceCond5 := !c.ARef || !c.BRef || ra == 0 || rb == 0 || ca == 0 || cb == 0; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        f = 4;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        var rowLow int64 = func() int64 { if ra < rb { return ra }; return rb }()
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var rowHigh int64 = func() int64 { if ra < rb { return rb }; return ra }()
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var colLow int64 = func() int64 { if ca < cb { return ca }; return cb }()
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var colHigh int64 = func() int64 { if ca < cb { return cb }; return ca }()
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var above int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var below int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var spilled bool = false
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var worst int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var walked int64 = 0
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        for _, q := range cells {
                            var qr int64 = 0
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var qc int64 = 0
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var qs int64 = 0
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            for _, rq := range rows {
                                qs = scealgorithm.AddInt64(&sceFailure, qs, 1);
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                if sceCond8 := rq.Key == q.Row; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    qr = qs;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            qs = 0;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            for _, cq := range cols {
                                qs = scealgorithm.AddInt64(&sceFailure, qs, 1);
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                if sceCond8 := cq.Key == q.Col; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    qc = qs;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            var qgone bool = false
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            for _, gq := range gone {
                                if sceCond8 := gq.Key == q.Row; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    qgone = true;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            for _, hq := range goneCols {
                                if sceCond8 := hq.Key == q.Col; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    qgone = true;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            if sceCond7 := !qgone && qr >= rowLow && qr <= rowHigh && qc >= colLow && qc <= colHigh; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                var seen bool = false
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                var w int64 = 0
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                                for {
                                    sceCond8 := w < int64(len(trail))
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                    if !sceCond8 {
                                        break
                                    }
                                    if sceCond9 := scealgorithm.At(&sceFailure, trail[:], int64(w)) == walked; sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    } else if sceCond9 {
                                        seen = true;
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        var mv int64 = scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, w, 1)))
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        var mf int64 = scealgorithm.At(&sceFailure, trail[:], int64(scealgorithm.AddInt64(&sceFailure, w, 2)))
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        if sceCond10 := mf != 0; sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
                                            worst = func() int64 { if worst == 0 { return mf }; return func() int64 { if mf < worst { return mf }; return worst }() }();
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                        }
                                        if sceCond10 := mf == 0 && mv > 0; sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
                                            grown := scealgorithm.Take[sce_sheet_value.SheetValuePayload](&sceFailure)(sce_sheet_apply.SheetApply(2, above, mv))
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            if sceCond11 := grown.Fault != 0; sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            } else if sceCond11 {
                                                spilled = true;
                                                if sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                }
                                            } else {
                                                above = grown.Value;
                                                if sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                }
                                            }
                                        }
                                        if sceCond10 := mf == 0 && mv < 0; sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
                                            shrunk := scealgorithm.Take[sce_sheet_value.SheetValuePayload](&sceFailure)(sce_sheet_apply.SheetApply(2, below, mv))
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            if sceCond11 := shrunk.Fault != 0; sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            } else if sceCond11 {
                                                spilled = true;
                                                if sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                }
                                            } else {
                                                below = shrunk.Value;
                                                if sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                }
                                            }
                                        }
                                    }
                                    w = scealgorithm.AddInt64(&sceFailure, w, 3);
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                                if sceCond8 := !seen; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    ready = false;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            walked = scealgorithm.AddInt64(&sceFailure, walked, 1);
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                        if sceCond6 := worst != 0; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            f = worst;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        } else {
                            if sceCond7 := spilled; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                f = 3;
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            } else {
                                v = scealgorithm.AddInt64(&sceFailure, above, below);
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            }
                        }
                    }
                }
                if sceCond4 := ready; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    trail = append(trail, int64(at))
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    trail = append(trail, int64(v))
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    trail = append(trail, int64(f))
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    moving = true;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
            }
            at = scealgorithm.AddInt64(&sceFailure, at, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []int64 = trail
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
