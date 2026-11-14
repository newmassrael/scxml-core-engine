// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
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
    public static func algorithmTaskPhaseAdvance(_ tasks: [EventSchemaTaskPayload]) throws(AlgorithmError) -> [EventSchemaTaskPayload] {
        var out: [EventSchemaTaskPayload] = []
        out.reserveCapacity(4)
        for t in tasks {
            var next: EventSchemaTaskPayload = EventSchemaTaskPayload(taskId: t.taskId, phase: t.phase)
            if t.phase == EnumTaskPhase.todo {
                next.phase = EnumTaskPhase.inReview
            } else {
                if t.phase != EnumTaskPhase.shipped {
                    next.phase = EnumTaskPhase.shipped
                }
            }
            out.append(next)
        }
        return out
    }
}
