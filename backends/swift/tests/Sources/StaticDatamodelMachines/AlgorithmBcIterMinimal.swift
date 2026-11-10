// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f0e28d53caaa29bc10122250abf2dd34aa356f984c7dd48a910705065c74b3a
// SCE-MAP: algorithm_bc_iter_minimal.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `AlgorithmBcIterMinimal`, a file of the
// module that calls it: a call is qualified by the enum
// (`AlgorithmBcIterMinimal.algorithmBcIterMinimal(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum AlgorithmBcIterMinimal {
    public static func algorithmBcIterMinimal(_ target: UInt32, _ subs: LocalSubTable) -> UInt16 {
        for sceSlotIdx in 0..<LocalSubTable.capacity {
            guard let entry = subs.getBySlot(UInt32(sceSlotIdx)) else { continue }
            if entry.callback_id == target {
                return 0
            }
        }
        return 0xFFFF
    }
}
