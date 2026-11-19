// SCE-GENERATED — DO NOT EDIT
// source-hash: f3257b59a26f3d49e9a676fe866320945ea801a05fc84cede7e3229b9d164fa1
// source: removing_configuration.scxml
// SCE-MAP: removing_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

import SceRuntime
import SceTestProfile

/// The test configuration `removing_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it, the members of a caseless enum of the module as every unit's are:
/// the configuration, the factory of its components' machines, the module parameters they read,
/// and the reader of the inputs a recorded trace carries on its system ports
/// (docs/SCE_TTCN3_LANE.md §10.5).
public enum RemovingConfiguration {
    /// The configuration: the MTC's component type and each PTC's, the ports and timers each type
    /// declares, each `<sce:connect>` and each `<sce:map>`.
    public static func configuration() -> SceTestProfile.Configuration {
        SceTestProfile.Configuration(
            mtcType: "ptc_remover",
            ptcTypes: [
                "listener": "request_listener",
            ],
            portsOfType: [
                "ptc_remover": ["peer"],
                "request_listener": ["peer"],
            ],
            timersOfType: [
                "ptc_remover": ["patience"],
            ],
            connections: [
                SceTestProfile.Connection(SceTestProfile.Endpoint(component: "mtc", port: "peer"), SceTestProfile.Endpoint(component: "listener", port: "peer")),
            ],
            mappings: [])
    }

    /// The machine of each component type of the configuration: the component the profile's
    /// runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
    /// §12.3). Nil for a type the configuration does not name, which the run ends with `error`
    /// where it is created.
    public static func machine(_ componentType: String) -> (any SceTestProfile.ComponentMachine)? {
        switch componentType {
        case "ptc_remover": return PtcRemoverComponent()
        case "request_listener": return RequestListenerComponent()
        default: return nil
        }
    }

    /// The module parameters the components of the configuration read (docs/SCE_TEST_PROFILE.md
    /// §11.5), each of which a suite that runs it gives a value.
    public static func moduleParameters() -> [String] {
        []
    }

    /// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
    /// `SceTestProfile.InputReader`): the message an input of the record type `typeName` on the
    /// system port `systemPort` carries, its value read from the canonical form.
    ///
    /// - Throws: `SceTestProfile.TraceRefusal` naming why, where the configuration has no such system
    ///   port, the `in` list of its type does not name the type, or the value does not belong to it.
    public static func input(systemPort: String, typeName: String, value: SceRuntime.JSONValue)
        throws(SceTestProfile.TraceRefusal) -> SceTestProfile.Message
    {
        throw SceTestProfile.TraceRefusal.notAnInput(systemPort, typeName, [])
    }
}
