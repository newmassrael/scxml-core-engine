// SCE-GENERATED — DO NOT EDIT
// source-hash: 6b431984688d288798089f16432b8bdf8a4e08b28da85ae7117e434e9ad27889
// SCE-MAP: algorithm_task_phase_reopened.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmTaskPhaseReopened`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmTaskPhaseReopened.algorithmTaskPhaseReopened(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmTaskPhaseReopened {
    public static func algorithmTaskPhaseReopened(_ task: EventSchemaTaskPayload) -> EventSchemaTaskPayload {
        var out: EventSchemaTaskPayload = EventSchemaTaskPayload(taskId: task.taskId, phase: task.phase)
        if task.phase == EnumTaskPhase.shipped {
            out.phase = EnumTaskPhase.todo
        }
        return out
    }
}
