// SCE-GENERATED — DO NOT EDIT
// source-hash: f7347a8b98455a65e31ae2b3af12ef65d68a284242bd71a8102ab373827910fd
// SCE-MAP: text_slot_retitled.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `TextSlotRetitled`, a file of the
// module that calls it: a call is qualified by the enum
// (`TextSlotRetitled.textSlotRetitled(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

import SceForgeRuntime

public enum TextSlotRetitled {
    public static func textSlotRetitled(_ slots: [TextSlotPayload]) throws -> [TextSlotPayload] {
        // A failure a checked operation records, thrown once the statement is done
        // (SCE_FORGE.md §3.4.1).
        let sceFailure = Failure()
        var out: [TextSlotPayload] = []
        out.reserveCapacity(16)
        try sceFailure.throwIfFailed()
        for s in slots {
            var shown: TextSlotPayload = TextSlotPayload(slot: s.slot, version: s.version, text: s.text)
            try sceFailure.throwIfFailed()
            do {
                let sceCond3 = s.text.utf8.elementsEqual("".utf8)
                try sceFailure.throwIfFailed()
                if sceCond3 {
                    shown.text = "untitled"
                    try sceFailure.throwIfFailed()
                }
            }
            out.append(shown)
            try sceFailure.throwIfFailed()
        }
        do {
            let sceReturned: [TextSlotPayload] = out
            try sceFailure.throwIfFailed()
            return sceReturned
        }
    }
}
