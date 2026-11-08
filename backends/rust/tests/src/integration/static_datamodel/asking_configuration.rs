// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6
#![doc = "SCE-MAP: asking_configuration.scxml:5 :: _forge_body"]
// SCE-MAP: asking_configuration.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `asking_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "param_asker".to_string(),
        ptc_types: ::std::collections::BTreeMap::new(),
        ports_of_type: ::std::collections::BTreeMap::from([(
            "param_asker".to_string(),
            vec!["sut".to_string()],
        )]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "param_asker".to_string(),
            vec!["reply".to_string()],
        )]),
        connections: Vec::new(),
        mappings: vec![(
            ::sce_rust_runtime::test::Endpoint::new("mtc", "sut"),
            "sut".to_string(),
        )],
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
        "param_asker" => Box::<super::param_asker_sm::ParamAskerComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    vec!["job".to_string()]
}
