// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: lookup_engine_status.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="lookup")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// A table read by exact match: a static function of the caseless enum `LookupEngineStatus`, a
// file of the module that calls it. A call is qualified by the enum
// (`LookupEngineStatus.lookupStatus(…)`), which a member of the calling type cannot hide as
// it would hide a free function of the same name.
public enum LookupEngineStatus {
    /// The names the table answers, each case holding the name the document spells.
    public enum Status: String, Sendable {
        case STOP = "STOP"
        case RUNNING = "RUNNING"
        case FAULT = "FAULT"
    }

    public static func lookupStatus(_ engSta: UInt8) -> Status {
        switch engSta {
        case 0x07:
            return .FAULT
        case 0x03:
            return .RUNNING
        case 0x00, 0x01, 0x02:
            return .STOP
        default:
            return .STOP
        }
    }
}
