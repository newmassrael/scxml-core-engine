// SCE-GENERATED — DO NOT EDIT
// source-hash: 242afb729b9865bf6d727ab9bf3b98d23b9e6f9c59d9d146a0381a194dfbe13b
#![doc = "SCE-MAP: removing_configuration.scxml:4 :: _forge_body"]
// SCE-MAP: removing_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `removing_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "ptc_remover".to_string(),
        ptc_types: ::std::collections::BTreeMap::from([(
            "listener".to_string(),
            "request_listener".to_string(),
        )]),
        ports_of_type: ::std::collections::BTreeMap::from([
            ("ptc_remover".to_string(), vec!["peer".to_string()]),
            ("request_listener".to_string(), vec!["peer".to_string()]),
        ]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "ptc_remover".to_string(),
            vec!["patience".to_string()],
        )]),
        connections: vec![(
            ::sce_rust_runtime::test::Endpoint::new("mtc", "peer"),
            ::sce_rust_runtime::test::Endpoint::new("listener", "peer"),
        )],
        mappings: Vec::new(),
    }
}

/// The machine of each component type of the configuration: the component the profile's
/// runtime drives, generated beside its test component's machine (docs/SCE_TEST_PROFILE.md
/// §12.3). None for a type the configuration does not name, which the run ends with `error`
/// where it is created.
pub fn machine(
    component_type: &str,
) -> Option<Box<dyn ::sce_rust_runtime::test::ComponentMachine>> {
    let machine: Box<dyn ::sce_rust_runtime::test::ComponentMachine> = match component_type {
        "ptc_remover" => Box::<super::ptc_remover_sm::PtcRemoverComponent>::default(),
        "request_listener" => {
            Box::<super::request_listener_sm::RequestListenerComponent>::default()
        }
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    Vec::new()
}

/// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
/// `sce_rust_runtime::test::InputReader`): the message an input of the record type `type_name`
/// on the system port `system_port` carries, its value read from the canonical form. Refused,
/// naming why, where the configuration has no such system port, the `in` list of its type does
/// not name the type, or the value does not belong to it.
pub fn input(
    system_port: &str,
    type_name: &str,
    _value: &::sce_rust_runtime::json::Value,
) -> Result<::sce_rust_runtime::test::Message, ::sce_rust_runtime::test::TraceRefusal> {
    Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(
        system_port,
        type_name,
        &[],
    ))
}
