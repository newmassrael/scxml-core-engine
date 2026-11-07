// SCE-GENERATED — DO NOT EDIT
// source-hash: 22249655389e33dabe1467cad0f6465e75f1c83f40cd0207380a1fc15324f922
// SCE-MAP: forge_call_high.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="condition")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A named boolean guard: a static function of the caseless enum `ForgeCallHigh`, a
// file of the module that calls it. A call is qualified by the enum
// (`ForgeCallHigh.forgeCallHigh(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.

public enum ForgeCallHigh {
    public static func forgeCallHigh(_ level: Int32) -> Bool {
        level > 20
    }
}
