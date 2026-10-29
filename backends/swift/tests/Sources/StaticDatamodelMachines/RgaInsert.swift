// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
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
    public static func rgaInsert(_ nodes: [RgaNodePayload], _ node: RgaNodePayload) throws -> [RgaNodePayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [RgaNodePayload] = []
        out.reserveCapacity(256)
        try sceFailure.throwIfFailed()
        var held: Bool = false
        try sceFailure.throwIfFailed()
        for probe in nodes {
            do {
                let sceCond3 = probe.key == node.key
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    held = true
                    try sceFailure.throwIfFailed()
                }
            }
        }
        do {
            let sceCond2 = held
            try sceFailure.throwIfFailed()
            if sceCond2 {
                for same in nodes {
                    out.append(same)
                    try sceFailure.throwIfFailed()
                }
                do {
                    let sceReturned: [RgaNodePayload] = out
                    try sceFailure.throwIfFailed()
                    return sceReturned
                }
            }
        }
        var phase: UInt8 = node.after == 0 ? 1 : 0
        try sceFailure.throwIfFailed()
        for n in nodes {
            do {
                let sceCond3 = phase == 0
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    out.append(n)
                    try sceFailure.throwIfFailed()
                    do {
                        let sceCond5 = n.key == node.after
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            phase = 1
                            try sceFailure.throwIfFailed()
                        }
                    }
                } else {
                    do {
                        let sceCond5 = phase == 1
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            do {
                                let sceCond7 = n.key > node.key
                                try sceFailure.throwIfFailed()
                                if sceCond7 {
                                    out.append(n)
                                    try sceFailure.throwIfFailed()
                                } else {
                                    out.append(node)
                                    try sceFailure.throwIfFailed()
                                    out.append(n)
                                    try sceFailure.throwIfFailed()
                                    phase = 2
                                    try sceFailure.throwIfFailed()
                                }
                            }
                        } else {
                            out.append(n)
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
            }
        }
        do {
            let sceCond2 = phase != 2
            try sceFailure.throwIfFailed()
            if sceCond2 {
                out.append(node)
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: [RgaNodePayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
