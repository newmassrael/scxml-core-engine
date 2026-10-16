// SCE-GENERATED — DO NOT EDIT
// source-hash: b8448cd407b1027864e71ab2f9c10d7e6a28146f7cc75313b45ffc4e7287b0f4
// SCE-MAP: merkle_change_hash.scxml:40 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `merkle_change_hash`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package merkle_change_hash

import (
	scealgorithm "github.com/newmassrael/sce-forge-runtime/algorithm"
	sce_orset_entry "github.com/newmassrael/sce-go-tests/integration/static_datamodel/orset_entry"
)

func MerkleChangeHash(change sce_orset_entry.OrsetEntryPayload, kind uint8) (sceValue uint32, sceErr error) {
    // SCE_FORGE.md §3.4.1: each checked operation records a failure here, and
    // the statement around it returns it before the next statement runs.
    var sceFailure scealgorithm.Failure
    if sceCond1 := !(change.WallTime >= 0); sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    } else if sceCond1 {
        return sceValue, scealgorithm.Precondition
    }
    var element uint64 = change.Element
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var wall uint64 = scealgorithm.NarrowUint64FromInt(&sceFailure, int64(change.WallTime))
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var counter uint64 = uint64(change.Counter)
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var node uint64 = change.NodeId
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    var h uint64 = 0
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.RemUint64(&sceFailure, element, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.DivUint64(&sceFailure, element, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.RemUint64(&sceFailure, wall, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.DivUint64(&sceFailure, wall, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), counter), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.RemUint64(&sceFailure, node, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), scealgorithm.DivUint64(&sceFailure, node, 4294967296)), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.AddUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 1000003), uint64(kind)), 1), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = h ^ scealgorithm.DivUint64(&sceFailure, h, 65536);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 2246822507), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = h ^ scealgorithm.DivUint64(&sceFailure, h, 8192);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = scealgorithm.RemUint64(&sceFailure, scealgorithm.MulUint64(&sceFailure, h, 3266489909), 4294967296);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    h = h ^ scealgorithm.DivUint64(&sceFailure, h, 65536);
    if sceFailure.Failed() {
        return sceValue, sceFailure.Err()
    }
    {
        var sceReturned uint32 = scealgorithm.NarrowUint32FromUint(&sceFailure, uint64(h))
        if sceFailure.Failed() {
            return sceValue, sceFailure.Err()
        }
        return sceReturned, nil
    }
}
