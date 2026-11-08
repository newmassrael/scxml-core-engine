// SCE-GENERATED — DO NOT EDIT
// source-hash: 9c4f25c71477b8168828f3c8f9ccad85a4253ff428db93b5b34a11bbacc1faa1
// SCE-MAP: algorithm_ids_above.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `ids_above`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package ids_above

func IdsAbove(ids []uint32, floor uint32) bool {
    var above bool = false
    for _, id := range ids {
        if (id > floor) {
            above = true;
        }
    }
    return above
}
