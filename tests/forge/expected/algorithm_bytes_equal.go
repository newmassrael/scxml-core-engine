// SCE-MAP: algorithm_bytes_equal:18

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §5.A: pure synchronous function with bounded loops. Free
// function in package `bytes_equal`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §5.J.5 emitter table).

package bytes_equal

func BytesEqual(a []byte, b []byte) bool {
    if (len(a) != len(b)) {
        return false;
    }
    var i uint32 = 0
    for i < uint32(len(a)) {
        if (a[i] != b[i]) {
            return false;
        }
        i = i + 1;
    }
    return true;
}
