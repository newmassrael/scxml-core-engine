// SCE-GENERATED — DO NOT EDIT
// source-hash: b9ab4ddbd49e8ce2a3b4c80917bc3eecdf0d5285c15974d4e0a00636f93fd573
// SCE-MAP: event_agenda.scxml:54 :: _forge_body

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
	sce_event_later "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_later"
)

func EventAgenda(values []sce_mv_value.MvValuePayload, id uint64, windowStart int64, windowEnd int64) (sceValue []sce_event_occurrence.EventOccurrencePayload, sceErr error) {
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
    var startAt int64 = -1
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var gone int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var freq int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var length int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var every0 int64 = 1
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var days int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var count int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var until int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var flag uint8 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var found bool = false
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var held int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var bw int64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var bc uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var bn uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w16 := range values {
        if sceCond2 := w16.Id == id && w16.Field == 16; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w16.WallTime, w16.Counter, w16.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w16.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w16.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w16.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                startAt = w16.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w9 := range values {
        if sceCond2 := w9.Id == id && w9.Field == 9; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w9.WallTime, w9.Counter, w9.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w9.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w9.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w9.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                gone = w9.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w18 := range values {
        if sceCond2 := w18.Id == id && w18.Field == 18; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w18.WallTime, w18.Counter, w18.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w18.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w18.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w18.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                freq = w18.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w17 := range values {
        if sceCond2 := w17.Id == id && w17.Field == 17; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w17.WallTime, w17.Counter, w17.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w17.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w17.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w17.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                length = w17.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w19 := range values {
        if sceCond2 := w19.Id == id && w19.Field == 19; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w19.WallTime, w19.Counter, w19.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w19.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w19.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w19.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                every0 = w19.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w20 := range values {
        if sceCond2 := w20.Id == id && w20.Field == 20; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w20.WallTime, w20.Counter, w20.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w20.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w20.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w20.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                days = w20.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w21 := range values {
        if sceCond2 := w21.Id == id && w21.Field == 21; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w21.WallTime, w21.Counter, w21.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w21.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w21.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w21.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                count = w21.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    found = false;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    held = 0;
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, w22 := range values {
        if sceCond2 := w22.Id == id && w22.Field == 22; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            held = scealgorithm.AddInt64(&sceFailure, held, 1);
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := !found || sce_event_later.EventLater(w22.WallTime, w22.Counter, w22.NodeId, bw, bc, bn); sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                found = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bw = w22.WallTime;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bc = w22.Counter;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                bn = w22.NodeId;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                until = w22.Value;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
        }
    }
    if sceCond1 := held > 1; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        flag = 1;
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    if sceCond1 := startAt >= 0 && gone != 1 && freq >= 0 && freq <= 3 && windowEnd > windowStart && windowEnd > 0; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        var every int64 = func() int64 { if every0 < 1 { return 1 }; return every0 }()
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
        var weekStart int64 = scealgorithm.SubInt64(&sceFailure, startDay, startWeekday)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var one int64 = 1
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var weekdays int64 = func() int64 { if days == 0 { return one << startWeekday }; return days & 127 }()
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
        var startOfMonth int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, doy, scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 153, mp), 2), 5)), 1)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var startMonth int64 = func() int64 { if mp < 10 { return scealgorithm.AddInt64(&sceFailure, mp, 3) }; return scealgorithm.SubInt64(&sceFailure, mp, 9) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var startYear int64 = func() int64 { if startMonth <= 2 { return scealgorithm.AddInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, yoe, scealgorithm.MulInt64(&sceFailure, era, 400)), 1) }; return scealgorithm.AddInt64(&sceFailure, yoe, scealgorithm.MulInt64(&sceFailure, era, 400)) }()
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var months int64 = scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, startYear, 12), startMonth), 1)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        var cursor int64 = 0
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
            var d int64 = scealgorithm.AddInt64(&sceFailure, startDay, cursor)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var valid bool = false
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := freq == 0; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                valid = cursor == 0;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := freq == 1; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                valid = scealgorithm.RemInt64(&sceFailure, cursor, every) == 0;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := freq == 2; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                valid = weekdays >> scealgorithm.RemInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, d, 7), 11), 7) & 1 == 1 && scealgorithm.RemInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, d, weekStart), 7), every) == 0;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            if sceCond3 := freq == 3; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                var total int64 = scealgorithm.AddInt64(&sceFailure, months, scealgorithm.MulInt64(&sceFailure, cursor, every))
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var landYear int64 = scealgorithm.DivInt64(&sceFailure, total, 12)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var landMonth int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.RemInt64(&sceFailure, total, 12), 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var landLeap bool = scealgorithm.RemInt64(&sceFailure, landYear, 4) == 0 && scealgorithm.RemInt64(&sceFailure, landYear, 100) != 0 || scealgorithm.RemInt64(&sceFailure, landYear, 400) == 0
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var last int64 = func() int64 { if landMonth == 2 { return func() int64 { if landLeap { return 29 }; return 28 }() }; return func() int64 { if landMonth == 4 || landMonth == 6 || landMonth == 9 || landMonth == 11 { return 30 }; return 31 }() }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var landDay int64 = func() int64 { if startOfMonth < last { return startOfMonth }; return last }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cy int64 = func() int64 { if landMonth <= 2 { return scealgorithm.SubInt64(&sceFailure, landYear, 1) }; return landYear }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cera int64 = scealgorithm.DivInt64(&sceFailure, cy, 400)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cyoe int64 = scealgorithm.SubInt64(&sceFailure, cy, scealgorithm.MulInt64(&sceFailure, cera, 400))
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cmp int64 = func() int64 { if landMonth > 2 { return scealgorithm.SubInt64(&sceFailure, landMonth, 3) }; return scealgorithm.AddInt64(&sceFailure, landMonth, 9) }()
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cdoy int64 = scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.DivInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, 153, cmp), 2), 5), landDay), 1)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                var cdoe int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, cyoe, 365), scealgorithm.DivInt64(&sceFailure, cyoe, 4)), scealgorithm.DivInt64(&sceFailure, cyoe, 100)), cdoy)
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                d = scealgorithm.SubInt64(&sceFailure, scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, cera, 146097), cdoe), 719468);
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
                valid = true;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            var occStart int64 = scealgorithm.AddInt64(&sceFailure, scealgorithm.MulInt64(&sceFailure, d, day), timeOfDay)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            var occEnd int64 = scealgorithm.AddInt64(&sceFailure, occStart, length)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
            if sceCond3 := d >= endDay || occStart >= windowEnd || until > 0 && occStart > until; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                going = false;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            } else {
                if sceCond4 := valid; sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                } else if sceCond4 {
                    if sceCond5 := count > 0 && nth >= count; sceFailure.Failed() {
                        return sceValue, sceFailure.Err()
                    } else if sceCond5 {
                        going = false;
                        if sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        }
                    } else {
                        if sceCond6 := (occEnd > windowStart || occStart >= windowStart) && emitted < 64; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            var excluded bool = false
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var mark uint8 = flag
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            var dayNo int64 = scealgorithm.DivInt64(&sceFailure, occStart, day)
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                            if sceCond7 := dayNo >= 18262 && scealgorithm.SubInt64(&sceFailure, dayNo, 18262) < 65472; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
                                var xfield uint64 = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(scealgorithm.AddInt64(&sceFailure, scealgorithm.SubInt64(&sceFailure, dayNo, 18262), 64)))
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
                                    if sceCond9 := wx.Id == id && wx.Field == xfield; sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    } else if sceCond9 {
                                        xheld = scealgorithm.AddInt64(&sceFailure, xheld, 1);
                                        if sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        }
                                        if sceCond10 := !xfound || sce_event_later.EventLater(wx.WallTime, wx.Counter, wx.NodeId, xw, xc, xn); sceFailure.Failed() {
                                            return sceValue, sceFailure.Err()
                                        } else if sceCond10 {
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
                                if sceCond8 := xheld > 1; sceFailure.Failed() {
                                    return sceValue, sceFailure.Err()
                                } else if sceCond8 {
                                    mark = 1;
                                    if sceFailure.Failed() {
                                        return sceValue, sceFailure.Err()
                                    }
                                }
                            }
                            if sceCond7 := !excluded; sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            } else if sceCond7 {
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
                        if sceCond6 := freq == 0; sceFailure.Failed() {
                            return sceValue, sceFailure.Err()
                        } else if sceCond6 {
                            going = false;
                            if sceFailure.Failed() {
                                return sceValue, sceFailure.Err()
                            }
                        }
                    }
                }
            }
            if sceCond3 := emitted >= 64; sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            } else if sceCond3 {
                going = false;
                if sceFailure.Failed() {
                    return sceValue, sceFailure.Err()
                }
            }
            cursor = scealgorithm.AddInt64(&sceFailure, cursor, 1);
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
