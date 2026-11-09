// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d8e77ae6e1148d1818998f2f9d129b014c767dc3cded9cb0af2d4fc0c10af0a
// SCE-MAP: causal_admit.scxml:25 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `CausalAdmit`, a file of the
// module that calls it: a call is qualified by the enum
// (`CausalAdmit.causalAdmit(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum CausalAdmit {
    public static func causalAdmit(_ delivered: [CausalEntryPayload], _ dot: CausalEntryPayload, _ context: [CausalEntryPayload]) throws -> CausalAdmission {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var seen: UInt64 = 0
        try sceFailure.throwIfFailed()
        for mine in delivered {
            do {
                let sceCond3 = mine.hi == dot.hi && mine.lo == dot.lo
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    seen = mine.seq
                    try sceFailure.throwIfFailed()
                }
            }
        }
        var covered: Bool = true
        try sceFailure.throwIfFailed()
        var have: UInt64 = 0
        try sceFailure.throwIfFailed()
        for needed in context {
            have = 0
            try sceFailure.throwIfFailed()
            for held in delivered {
                do {
                    let sceCond4 = held.hi == needed.hi && held.lo == needed.lo
                    try sceFailure.throwIfFailed()
                    if sceCond4 {
                        have = held.seq
                        try sceFailure.throwIfFailed()
                    }
                }
            }
            do {
                let sceCond3 = (needed.hi != dot.hi || needed.lo != dot.lo) && needed.seq > have
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    covered = false
                    try sceFailure.throwIfFailed()
                }
            }
        }
        var result: CausalAdmission = CausalAdmission.hold
        try sceFailure.throwIfFailed()
        do {
            let sceCond2 = dot.seq <= seen
            try sceFailure.throwIfFailed()
            if sceCond2 {
                result = CausalAdmission.drop
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceCond2 = dot.seq == Checked.add(sceFailure, seen, 1) && covered
            try sceFailure.throwIfFailed()
            if sceCond2 {
                result = CausalAdmission.deliver
                try sceFailure.throwIfFailed()
            }
        }
        do {
            let sceReturned: CausalAdmission = result
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
