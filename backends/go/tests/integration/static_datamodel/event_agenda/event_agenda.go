// SCE-GENERATED — DO NOT EDIT
// source-hash: 38a2b8ab9943c3f76e7281d40f9dae3ea2effb54aa8b7ab5baa1ac0eaba97f5b
// SCE-MAP: event_agenda.scxml:71 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `event_agenda`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package event_agenda

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_mv_value "github.com/newmassrael/sce-go-tests/integration/static_datamodel/mv_value"
	sce_event_occurrence "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_occurrence"
	sce_event_schedule "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_schedule"
	sce_event_later "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_later"
	sce_event_selects "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_selects"
)

func EventAgenda(s sce_event_schedule.EventSchedulePayload, values []sce_mv_value.MvValuePayload, id uint64, windowStart int64, windowEnd int64, skip uint32) (sceValue []sce_event_occurrence.EventOccurrencePayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_event_occurrence.EventOccurrencePayload, 0, 64)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var emitted int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var day int64 = 86400000
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var skipped int64 = int64(skip)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var startAt int64 = s.Start
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var freq int64 = s.Freq
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := startAt >= 0 && s.Gone != 1 && freq >= 0 && freq <= 4 && windowEnd > windowStart && windowEnd > 0; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        var every int64 = func() int64 { if s.Interval < 1 { return 1 }; return s.Interval }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var length int64 = s.Length
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var count int64 = s.Count
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var until int64 = s.Until
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasSP bool = s.SpPos | s.SpNeg != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var timeOfDay int64 = scealgorithm.RemInt64(&sceFailure, startAt, day)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var startDay int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, startAt, timeOfDay), day)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var endDay int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, windowEnd, day), 1), day)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var startWeekday int64 = scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, startDay, 7), 11), 7)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var wkst0 int64 = s.Wkst
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var wkst int64 = func() int64 { if wkst0 >= 0 && wkst0 <= 6 { return wkst0 }; return 1 }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var week0Start int64 = scealgorithm.SubInt64(&sceFailure, startDay, scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, startWeekday, wkst), 7), 7))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var z int64 = scealgorithm.AddInt64(&sceFailure, startDay, 719468)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var era int64 = scealgorithm.DivInt64(&sceFailure, z, 146097)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var doe int64 = scealgorithm.SubInt64(&sceFailure, z, scealgorithm.MulInt64(&sceFailure, era, 146097))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var yoe int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, doe, scealgorithm.DivInt64(&sceFailure, doe, 1460)), scealgorithm.DivInt64(&sceFailure, doe, 36524)), scealgorithm.DivInt64(&sceFailure, doe, 146096)), 365)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var doy int64 = scealgorithm.SubInt64(&sceFailure, doe, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 365, yoe), scealgorithm.DivInt64(&sceFailure, yoe, 4)), scealgorithm.DivInt64(&sceFailure, yoe, 100)))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var mp int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 5, doy), 2), 153)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var m0 int64 = func() int64 { if mp < 10 { return scealgorithm.AddInt64(&sceFailure, mp, 3) }; return scealgorithm.SubInt64(&sceFailure, mp, 9) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var y0 int64 = func() int64 { if m0 <= 2 { return scealgorithm.AddInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, yoe, scealgorithm.MulInt64(&sceFailure, era, 400)), 1) }; return scealgorithm.AddInt64(&sceFailure, yoe, scealgorithm.MulInt64(&sceFailure, era, 400)) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var months0 int64 = scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, y0, 12), m0), 1)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var p int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var nth int64 = 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var going bool = true
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        for {
            sceCond2 := going
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if !sceCond2 {
                break
            }
            var ps int64 = startDay
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var plen int64 = 1
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := freq == 1; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ps = scealgorithm.AddInt64(&sceFailure, startDay, scealgorithm.MulInt64(&sceFailure, p, every));
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := freq == 2; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ps = scealgorithm.AddInt64(&sceFailure, week0Start, scealgorithm.MulInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, p, every), 7));
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                plen = 7;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := freq == 3 || freq == 4; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                var pty int64 = func() int64 { if freq == 3 { return scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, months0, scealgorithm.MulInt64(&sceFailure, p, every)), 12) }; return scealgorithm.AddInt64(&sceFailure, y0, scealgorithm.MulInt64(&sceFailure, p, every)) }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var ptm int64 = func() int64 { if freq == 3 { return scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, months0, scealgorithm.MulInt64(&sceFailure, p, every)), 12), 1) }; return 1 }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pleap bool = scealgorithm.RemInt64(&sceFailure, pty, 4) == 0 && scealgorithm.RemInt64(&sceFailure, pty, 100) != 0 || scealgorithm.RemInt64(&sceFailure, pty, 400) == 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pcy int64 = func() int64 { if ptm <= 2 { return scealgorithm.SubInt64(&sceFailure, pty, 1) }; return pty }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pcera int64 = scealgorithm.DivInt64(&sceFailure, pcy, 400)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pcyoe int64 = scealgorithm.SubInt64(&sceFailure, pcy, scealgorithm.MulInt64(&sceFailure, pcera, 400))
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pcmp int64 = func() int64 { if ptm > 2 { return scealgorithm.SubInt64(&sceFailure, ptm, 3) }; return scealgorithm.AddInt64(&sceFailure, ptm, 9) }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var pcdoy int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 153, pcmp), 2), 5)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                ps = scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, pcera, 146097), scealgorithm.MulInt64(&sceFailure, pcyoe, 365)), scealgorithm.DivInt64(&sceFailure, pcyoe, 4)), scealgorithm.DivInt64(&sceFailure, pcyoe, 100)), pcdoy), 719468);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                plen = func() int64 { if freq == 4 { return func() int64 { if pleap { return 366 }; return 365 }() }; return func() int64 { if ptm == 2 { return func() int64 { if pleap { return 29 }; return 28 }() }; return func() int64 { if ptm == 4 || ptm == 6 || ptm == 9 || ptm == 11 { return 30 }; return 31 }() }() }();
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := ps >= endDay || until > 0 && scealgorithm.MulInt64(&sceFailure, ps, day) > until; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                going = false;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            } else {
                var n int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var k int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                for {
                    sceCond4 := k < plen
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    if !sceCond4 {
                        break
                    }
                    if sceCond5 := scealgorithm.Take[bool](&sceFailure)(sce_event_selects.EventSelects(s, scealgorithm.AddInt64(&sceFailure, ps, k), startDay)); sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        n = scealgorithm.AddInt64(&sceFailure, n, 1);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    }
                    k = scealgorithm.AddInt64(&sceFailure, k, 1);
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                var i int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var rank int64 = 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                for {
                    sceCond4 := i < plen && going
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    if !sceCond4 {
                        break
                    }
                    var dd int64 = scealgorithm.AddInt64(&sceFailure, ps, i)
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                    if sceCond5 := scealgorithm.Take[bool](&sceFailure)(sce_event_selects.EventSelects(s, dd, startDay)); sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        rank = scealgorithm.AddInt64(&sceFailure, rank, 1);
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        var keep bool = true
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                        if sceCond6 := hasSP; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            keep = rank <= 62 && s.SpPos >> rank & 1 == 1 || scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, n, rank), 1) <= 62 && s.SpNeg >> scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, n, rank), 1) & 1 == 1;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                        if sceCond6 := keep && dd >= startDay; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            var occStart int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, dd, day), timeOfDay)
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var occEnd int64 = scealgorithm.AddInt64(&sceFailure, occStart, length)
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            if sceCond7 := occStart >= windowEnd || until > 0 && occStart > until; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                going = false;
                                if sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                }
                            } else {
                                if sceCond8 := count > 0 && nth >= count; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    going = false;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                } else {
                                    if sceCond9 := (occEnd > windowStart || occStart >= windowStart) && emitted < 64 && nth >= skipped; sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    } else if sceCond9 {
                                        var excluded bool = false
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        var mark uint8 = scealgorithm.NarrowUint8FromInt(&sceFailure, int64(s.Flag))
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        if sceCond10 := dd >= 0 && dd < 65472; sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
                                            var xfield uint64 = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(scealgorithm.AddInt64(&sceFailure, dd, 64)))
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xfound bool = false
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xheld int64 = 0
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xw int64 = 0
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xc uint32 = 0
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xn uint64 = 0
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            var xv int64 = 0
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            for _, wx := range values {
                                                if sceCond12 := wx.Id == id && wx.Field == xfield; sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                } else if sceCond12 {
                                                    xheld = scealgorithm.AddInt64(&sceFailure, xheld, 1);
                                                    if sceFailure.Failed() {
                                                        return sceValue, sceFailure.Err()
                                                    }
                                                    if sceCond13 := !xfound || sce_event_later.EventLater(wx.WallTime, wx.Counter, wx.NodeId, xw, xc, xn); sceFailure.Failed() {
                                                        return sceValue, sceFailure.Err()
                                                    } else if sceCond13 {
                                                        xfound = true;
                                                        if sceFailure.Failed() {
                                                            return sceValue, sceFailure.Err()
                                                        }
                                                        xw = wx.WallTime;
                                                        if sceFailure.Failed() {
                                                            return sceValue, sceFailure.Err()
                                                        }
                                                        xc = wx.Counter;
                                                        if sceFailure.Failed() {
                                                            return sceValue, sceFailure.Err()
                                                        }
                                                        xn = wx.NodeId;
                                                        if sceFailure.Failed() {
                                                            return sceValue, sceFailure.Err()
                                                        }
                                                        xv = wx.Value;
                                                        if sceFailure.Failed() {
                                                            return sceValue, sceFailure.Err()
                                                        }
                                                    }
                                                }
                                            }
                                            excluded = xfound && xv == 1;
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            if sceCond11 := xheld > 1; sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            } else if sceCond11 {
                                                mark = 1;
                                                if sceFailure.Failed() {
                                                    return sceValue, sceFailure.Err()
                                                }
                                            }
                                        }
                                        if sceCond10 := !excluded; sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
                                            line := sce_event_occurrence.EventOccurrencePayload{Id: id, Start: occStart, End: occEnd, Nth: scealgorithm.NarrowUint32FromInt(&sceFailure, int64(nth)), Conflict: mark}
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            out = append(out, line)
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                            emitted = scealgorithm.AddInt64(&sceFailure, emitted, 1);
                                            if sceFailure.Failed() {
                                                return sceValue, sceFailure.Err()
                                            }
                                        }
                                    }
                                    nth = scealgorithm.AddInt64(&sceFailure, nth, 1);
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                        }
                    }
                    if sceCond5 := emitted >= 64; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        going = false;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    }
                    i = scealgorithm.AddInt64(&sceFailure, i, 1);
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
                if sceCond4 := freq == 0; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    going = false;
                    if sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    }
                }
            }
            p = scealgorithm.AddInt64(&sceFailure, p, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned []sce_event_occurrence.EventOccurrencePayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
