// SCE-MAP: algorithm_cobs_encode:32 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops. Free
// function in package `algorithm_cobs_encode`, no instance state. `bytes`
// parameters lower to `[]byte` (RFC §synth-5-J-5 emitter table).

package algorithm_cobs_encode

func AlgorithmCobsEncode(data []byte) []byte {
    var n uint16 = uint16(len(data))
    out := []byte{}
    var p uint16 = 0
    var done bool = false
    for done == false {
        var q uint16 = p
        for q < n && q - p < 254 && data[q] != 0 {
            q = q + 1;
        }
        var run uint16 = q - p
        var code uint8 = uint8(run + 1)
        out = append(out, byte(code))
        var k uint16 = p
        for k < q {
            out = append(out, byte(data[k]))
            k = k + 1;
        }
        if (q >= n) {
            done = true;
        } else {
            if (run < 254) {
                p = q + 1;
                if (p >= n) {
                    var last uint8 = 1
                    out = append(out, byte(last))
                    done = true;
                }
            } else {
                p = q;
            }
        }
    }
    return out
}
