// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c860c2693fcb3366bc871ff801c9b0ee9e491a1759830f8d83238267cc3b45
// SCE-MAP: algorithm_task_phase_advance.scxml:12 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTaskPhaseAdvance`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTaskPhaseAdvance.algorithmTaskPhaseAdvance(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum AlgorithmTaskPhaseAdvance {
    public static func algorithmTaskPhaseAdvance(_ tasks: [EventSchemaTaskPayload]) throws -> [EventSchemaTaskPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [EventSchemaTaskPayload] = []
        out.reserveCapacity(4)
        try sceFailure.throwIfFailed()
        for t in tasks {
            var next: EventSchemaTaskPayload = EventSchemaTaskPayload(taskId: t.taskId, phase: t.phase)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = t.phase == EnumTaskPhase.todo
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    next.phase = EnumTaskPhase.inReview
                    try sceFailure.throwIfFailed()
                } else {
                    do {
                        let sceCond5 = t.phase != EnumTaskPhase.shipped
                        try sceFailure.throwIfFailed()
                        if sceCond5 {
                            next.phase = EnumTaskPhase.shipped
                            try sceFailure.throwIfFailed()
                        }
                    }
                }
            }
            out.append(next)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [EventSchemaTaskPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
