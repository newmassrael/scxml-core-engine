// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c
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
