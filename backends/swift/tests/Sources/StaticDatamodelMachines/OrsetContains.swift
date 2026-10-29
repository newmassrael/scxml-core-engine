// SCE-GENERATED — DO NOT EDIT
// source-hash: b3df842d016676bf8989832cb164de51027fadf6ca9e7b5543c7bac0c9a13d1a
// SCE-MAP: orset_contains.scxml:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="algorithm")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// RFC §synth-5-A: pure synchronous function with bounded loops, no instance
// state. A static function of the caseless enum `OrsetContains`, a file of the
// module that calls it: a call is qualified by the enum
// (`OrsetContains.orsetContains(…)`), which a member of the calling type
// cannot hide as it would hide a free function of the same name.

public enum OrsetContains {
    public static func orsetContains(_ live: [OrsetEntryPayload], _ element: UInt64) -> Bool {
        var found: Bool = false
        for x in live {
            if x.element == element {
                found = true
            }
        }
        return found
    }
}
