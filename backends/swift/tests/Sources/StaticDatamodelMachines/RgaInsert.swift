// SCE-GENERATED — DO NOT EDIT
// source-hash: fb4f42a34e7ac40a28d8e5daee5d633c3c45dddac513816edf2bb83fdc85f91c
// SCE-MAP: rga_insert.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `RgaInsert`, a file of the
// module that calls it: a call is qualified by the enum
// (`RgaInsert.rgaInsert(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum RgaInsert {
    public static func rgaInsert(_ nodes: [RgaNodePayload], _ node: RgaNodePayload) throws(AlgorithmError) -> [RgaNodePayload] {
        var out: [RgaNodePayload] = []
        out.reserveCapacity(256)
        var held: Bool = false
        for probe in nodes {
            if probe.key == node.key {
                held = true
            }
        }
        if held {
            for same in nodes {
                out.append(same)
            }
            return out
        }
        var phase: UInt8 = node.after == 0 ? 1 : 0
        for n in nodes {
            if phase == 0 {
                out.append(n)
                if n.key == node.after {
                    phase = 1
                }
            } else {
                if phase == 1 {
                    if n.key > node.key {
                        out.append(n)
                    } else {
                        out.append(node)
                        out.append(n)
                        phase = 2
                    }
                } else {
                    out.append(n)
                }
            }
        }
        if phase != 2 {
            out.append(node)
        }
        return out
    }
}
