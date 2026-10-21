// SCE-GENERATED — DO NOT EDIT
// source-hash: 9313ff05336b94eeb54552b43ab70c62d4e55585e0cedc6ece4c89ceb09077d5
// SCE-MAP: record_settled.scxml:21 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `record_settled`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package record_settled

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_orset_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orset_entry"
	sce_merkle_change_hash "github.com/newmassrael/sce-go-tests/integration/static_datamodel/merkle_change_hash"
)

func RecordSettled(adds []sce_orset_entry.OrsetEntryPayload, tombstones []sce_orset_entry.OrsetEntryPayload, before int64) (sceValue uint32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    var h uint32 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    for _, a := range adds {
        if sceCond2 := a.WallTime <= before; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            h = h ^ scealgorithm.Take[uint32](&sceFailure)(sce_merkle_change_hash.MerkleChangeHash(a, 0));
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    for _, t := range tombstones {
        if sceCond2 := t.WallTime <= before; sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        } else if sceCond2 {
            h = h ^ scealgorithm.Take[uint32](&sceFailure)(sce_merkle_change_hash.MerkleChangeHash(t, 1));
            if sceFailure.Failed() {
                return sceValue, sceFailure.Err()
            }
        }
    }
    {
        var sceReturned uint32 = h
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
