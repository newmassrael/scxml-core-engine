// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9
#![doc = "SCE-MAP: serving_configuration.scxml:4 :: _forge_body"]
// SCE-MAP: serving_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `serving_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "coordinator".to_string(),
        ptc_types: ::std::collections::BTreeMap::from([(
            "answerer".to_string(),
            "answerer".to_string(),
        )]),
        ports_of_type: ::std::collections::BTreeMap::from([
            ("coordinator".to_string(), vec!["peer".to_string()]),
            ("answerer".to_string(), vec!["peer".to_string()]),
        ]),
        timers_of_type: ::std::collections::BTreeMap::new(),
        connections: vec![(
            ::sce_rust_runtime::test::Endpoint::new("mtc", "peer"),
            ::sce_rust_runtime::test::Endpoint::new("answerer", "peer"),
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
        "coordinator" => Box::<super::coordinator_sm::CoordinatorComponent>::default(),
        "answerer" => Box::<super::answerer_sm::AnswererComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    Vec::new()
}
