// SCE-GENERATED — DO NOT EDIT
// source-hash: 9ec1a747bc5501f93d66b3929221786d0a45ece3c23f323814ee50eb28f290d9
// SCE-MAP: event_selects.scxml:19 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `event_selects`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package event_selects

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_event_schedule "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_schedule"
)

func EventSelects(s sce_event_schedule.EventSchedulePayload, d int64, startDay int64) (sceValue bool, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var freq int64 = s.Freq
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var ok bool = true
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    if sceCond1 := freq != 0; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        var startWd int64 = scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, startDay, 7), 11), 7)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var z0 int64 = scealgorithm.AddInt64(&sceFailure, startDay, 719468)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var era0 int64 = scealgorithm.DivInt64(&sceFailure, z0, 146097)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var doe0 int64 = scealgorithm.SubInt64(&sceFailure, z0, scealgorithm.MulInt64(&sceFailure, era0, 146097))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var yoe0 int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, doe0, scealgorithm.DivInt64(&sceFailure, doe0, 1460)), scealgorithm.DivInt64(&sceFailure, doe0, 36524)), scealgorithm.DivInt64(&sceFailure, doe0, 146096)), 365)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var doy0 int64 = scealgorithm.SubInt64(&sceFailure, doe0, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 365, yoe0), scealgorithm.DivInt64(&sceFailure, yoe0, 4)), scealgorithm.DivInt64(&sceFailure, yoe0, 100)))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var mp0 int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 5, doy0), 2), 153)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dom0 int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, doy0, scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 153, mp0), 2), 5)), 1)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var m0 int64 = func() int64 { if mp0 < 10 { return scealgorithm.AddInt64(&sceFailure, mp0, 3) }; return scealgorithm.SubInt64(&sceFailure, mp0, 9) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dz int64 = scealgorithm.AddInt64(&sceFailure, d, 719468)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dera int64 = scealgorithm.DivInt64(&sceFailure, dz, 146097)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ddoe int64 = scealgorithm.SubInt64(&sceFailure, dz, scealgorithm.MulInt64(&sceFailure, dera, 146097))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dyoe int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, ddoe, scealgorithm.DivInt64(&sceFailure, ddoe, 1460)), scealgorithm.DivInt64(&sceFailure, ddoe, 36524)), scealgorithm.DivInt64(&sceFailure, ddoe, 146096)), 365)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ddoy int64 = scealgorithm.SubInt64(&sceFailure, ddoe, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 365, dyoe), scealgorithm.DivInt64(&sceFailure, dyoe, 4)), scealgorithm.DivInt64(&sceFailure, dyoe, 100)))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dmp int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 5, ddoy), 2), 153)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dom int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, ddoy, scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 153, dmp), 2), 5)), 1)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var mon int64 = func() int64 { if dmp < 10 { return scealgorithm.AddInt64(&sceFailure, dmp, 3) }; return scealgorithm.SubInt64(&sceFailure, dmp, 9) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var yr int64 = func() int64 { if mon <= 2 { return scealgorithm.AddInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, dyoe, scealgorithm.MulInt64(&sceFailure, dera, 400)), 1) }; return scealgorithm.AddInt64(&sceFailure, dyoe, scealgorithm.MulInt64(&sceFailure, dera, 400)) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var leap bool = scealgorithm.RemInt64(&sceFailure, yr, 4) == 0 && scealgorithm.RemInt64(&sceFailure, yr, 100) != 0 || scealgorithm.RemInt64(&sceFailure, yr, 400) == 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var last int64 = func() int64 { if mon == 2 { return func() int64 { if leap { return 29 }; return 28 }() }; return func() int64 { if mon == 4 || mon == 6 || mon == 9 || mon == 11 { return 30 }; return 31 }() }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ylen int64 = func() int64 { if leap { return 366 }; return 365 }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var wd int64 = scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, d, 7), 11), 7)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var yday int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 367, mon), 362), 12), func() int64 { if mon <= 2 { return 0 }; return func() int64 { if leap { return -1 }; return -2 }() }()), dom)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasMonth bool = s.Bymonth != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasMD bool = s.MdPos | s.MdNeg != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasDay bool = s.Days | s.Op0 | s.Op1 | s.Op2 | s.Op3 | s.Op4 | s.Op5 | s.Op6 | s.On0 | s.On1 | s.On2 | s.On3 | s.On4 | s.On5 | s.On6 != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasWN bool = s.WnPos | s.WnNeg != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var hasYD bool = s.Yd0 | s.Yd1 | s.Yd2 | s.Yd3 | s.Yd4 | s.Yd5 | s.Ydn0 | s.Ydn1 | s.Ydn2 | s.Ydn3 | s.Ydn4 | s.Ydn5 != 0
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var mdMatch bool = s.MdPos >> dom & 1 == 1 || s.MdNeg >> scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, last, dom), 1) & 1 == 1
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var plainDay bool = s.Days >> wd & 1 == 1
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var inMonth bool = freq == 3 || hasMonth
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ordPos int64 = func() int64 { if inMonth { return scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, dom, 1), 7), 1) }; return scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, yday, 1), 7), 1) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ordNeg int64 = func() int64 { if inMonth { return scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, last, dom), 7), 1) }; return scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, ylen, yday), 7), 1) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var wordFirst int64 = func() int64 { if wd == 0 { return s.Op0 }; return func() int64 { if wd == 1 { return s.Op1 }; return func() int64 { if wd == 2 { return s.Op2 }; return func() int64 { if wd == 3 { return s.Op3 }; return func() int64 { if wd == 4 { return s.Op4 }; return func() int64 { if wd == 5 { return s.Op5 }; return s.Op6 }() }() }() }() }() }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var wordLast int64 = func() int64 { if wd == 0 { return s.On0 }; return func() int64 { if wd == 1 { return s.On1 }; return func() int64 { if wd == 2 { return s.On2 }; return func() int64 { if wd == 3 { return s.On3 }; return func() int64 { if wd == 4 { return s.On4 }; return func() int64 { if wd == 5 { return s.On5 }; return s.On6 }() }() }() }() }() }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ordFirst bool = ordPos <= 53 && wordFirst >> scealgorithm.SubInt64(&sceFailure, ordPos, 1) & 1 == 1
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var ordLast bool = ordNeg <= 53 && wordLast >> scealgorithm.SubInt64(&sceFailure, ordNeg, 1) & 1 == 1
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var dayMatch bool = plainDay || ordFirst || ordLast
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var clamped bool = func() bool { if s.Invalid == 1 { return dom == dom0 }; return dom == (func() int64 { if dom0 < last { return dom0 }; return last }()) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        ok = !hasMonth || s.Bymonth >> mon & 1 == 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        if sceCond2 := freq == 1; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            if sceCond3 := hasMD; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && mdMatch;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasDay; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && plainDay;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := freq == 2; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            ok = ok && (func() bool { if hasDay { return plainDay }; return wd == startWd }());
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
        if sceCond2 := freq == 3; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            if sceCond3 := hasMD; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && mdMatch;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasDay; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && dayMatch;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := !hasMD && !hasDay; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && clamped;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
        if sceCond2 := freq == 4; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            if sceCond3 := hasWN; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                var wkst int64 = func() int64 { if s.Wkst >= 0 && s.Wkst <= 6 { return s.Wkst }; return 1 }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wi int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, wd, wkst), 7), 7), 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wk int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, yday, wi), 10), 7)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wd28 int64 = scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, wd, ylen), 3), yday), 7), 7)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wi28 int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, wd28, wkst), 7), 7), 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wks int64 = scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, ylen, 3), wi28), 10), 7)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wkOk bool = wk >= 1 && wk <= wks && (s.WnPos >> wk & 1 == 1 || s.WnNeg >> scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, wks, wk), 1) & 1 == 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                ok = ok && wkOk;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasYD; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                var yp int64 = scealgorithm.SubInt64(&sceFailure, yday, 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var yn int64 = scealgorithm.SubInt64(&sceFailure, ylen, yday)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var jp int64 = scealgorithm.DivInt64(&sceFailure, yp, 61)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var jn int64 = scealgorithm.DivInt64(&sceFailure, yn, 61)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wordPos int64 = func() int64 { if jp == 0 { return s.Yd0 }; return func() int64 { if jp == 1 { return s.Yd1 }; return func() int64 { if jp == 2 { return s.Yd2 }; return func() int64 { if jp == 3 { return s.Yd3 }; return func() int64 { if jp == 4 { return s.Yd4 }; return s.Yd5 }() }() }() }() }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var wordNeg int64 = func() int64 { if jn == 0 { return s.Ydn0 }; return func() int64 { if jn == 1 { return s.Ydn1 }; return func() int64 { if jn == 2 { return s.Ydn2 }; return func() int64 { if jn == 3 { return s.Ydn3 }; return func() int64 { if jn == 4 { return s.Ydn4 }; return s.Ydn5 }() }() }() }() }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var ydOk bool = wordPos >> scealgorithm.RemInt64(&sceFailure, yp, 61) & 1 == 1 || wordNeg >> scealgorithm.RemInt64(&sceFailure, yn, 61) & 1 == 1
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                ok = ok && ydOk;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasMD; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && mdMatch;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasDay; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && dayMatch;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := hasWN && !hasDay && !hasYD && !hasMD; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && wd == startWd;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := !hasWN && !hasYD && !hasMD && !hasDay; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                ok = ok && (hasMonth || mon == m0) && clamped;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    {
        var sceReturned bool = ok
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
