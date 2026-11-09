// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
#![doc = "SCE-MAP: keeping_configuration.scxml:6 :: _forge_body"]
// SCE-MAP: keeping_configuration.scxml:6 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `keeping_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "ptc_keeper".to_string(),
        ptc_types: ::std::collections::BTreeMap::from([("idler".to_string(), "idler".to_string())]),
        ports_of_type: ::std::collections::BTreeMap::from([(
            "idler".to_string(),
            vec!["peer".to_string()],
        )]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "ptc_keeper".to_string(),
            vec!["patience".to_string(), "linger".to_string()],
        )]),
        connections: Vec::new(),
        mappings: vec![(
            ::sce_rust_runtime::test::Endpoint::new("idler", "peer"),
            "quiet".to_string(),
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
        "ptc_keeper" => Box::<super::ptc_keeper_sm::PtcKeeperComponent>::default(),
        "idler" => Box::<super::idler_sm::IdlerComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    Vec::new()
}
