// SCE-GENERATED — DO NOT EDIT
// source-hash: fbe1b9e6d3d44a234eeb738162c670f9c393ccc06557a4f68a8f54a0834b710c
// SCE-MAP: record_roster.scxml:14 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `record_roster`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package record_roster

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_record_member "github.com/newmassrael/sce-go-tests/integration/static_datamodel/record_member"
)

func RecordRoster(roster []sce_record_member.RecordMemberPayload, node uint64, role uint8) (sceValue []sce_record_member.RecordMemberPayload, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    out := make([]sce_record_member.RecordMemberPayload, 0, 64)
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
        made := sce_record_member.RecordMemberPayload{Node: node, Role: role}
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        out = append(out, made)
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
    }
    {
        var sceReturned []sce_record_member.RecordMemberPayload = out
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
