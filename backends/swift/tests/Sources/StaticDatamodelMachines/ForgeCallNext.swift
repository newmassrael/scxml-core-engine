// SCE-GENERATED — DO NOT EDIT
// source-hash: 3e662d3bbddd658a2d3b6f6a012f7bf4d21db1a4e6f0de7d35c1bd71b309f5a0
// SCE-MAP: forge_call_next.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="transform")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A formula per output: a static function of the caseless enum `ForgeCallNext` for each, a
// file of the module that calls them. A call is qualified by the enum
// (`ForgeCallNext.<function>(…)`), which a member of the calling type cannot hide as it would
// hide a free function of the same name. An output that reads another output calls its function.

public enum ForgeCallNext {

    public static func computeAfter(_ status: ForgeCallStatus) -> ForgeCallStatus {
        status == ForgeCallStatus.open ? ForgeCallStatus.held : ForgeCallStatus.done
    }
}
