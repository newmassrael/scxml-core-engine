// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
// SCE-MAP: event_roster.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `event_roster`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package event_roster

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_event_member "github.com/newmassrael/sce-go-tests/integration/static_datamodel/event_member"
)

func EventRoster(roster []sce_event_member.EventMemberPayload, node uint64, role uint8) (sceValue []sce_event_member.EventMemberPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_event_member.EventMemberPayload, 0, 64)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, m := range roster {
        if sceCond2 := m.Node != node; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            out = append(out, m)
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    if sceCond1 := role != 0; sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        made := sce_event_member.EventMemberPayload{Node: node, Role: role}
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        out = append(out, made)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_event_member.EventMemberPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
