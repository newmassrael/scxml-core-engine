// SCE-GENERATED — DO NOT EDIT
// source-hash: 20f48a3b336c719e7175b6da73bb16a60ea45d33fcaf7f01e6a1ec4ed70af971
// source: algorithm_text_filled.scxml
// SCE-MAP: algorithm_text_filled.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `TextFilled`, a file of the
// module that calls it: a call is qualified by the enum
// (`TextFilled.textFilled(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum TextFilled {
    public static func textFilled(_ text: String) -> Bool {
        return !text.utf8.elementsEqual("".utf8)
    }
}
