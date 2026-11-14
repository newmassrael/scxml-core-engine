// SCE-GENERATED — DO NOT EDIT
// source-hash: 242afb729b9865bf6d727ab9bf3b98d23b9e6f9c59d9d146a0381a194dfbe13b
#![doc = "SCE-MAP: signed_configuration.scxml:4 :: _forge_body"]
// SCE-MAP: signed_configuration.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `signed_configuration` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "signed_tester".to_string(),
        ptc_types: ::std::collections::BTreeMap::new(),
        ports_of_type: ::std::collections::BTreeMap::from([(
            "signed_tester".to_string(),
            vec!["sut".to_string()],
        )]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "signed_tester".to_string(),
            vec!["deadline".to_string()],
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
        "signed_tester" => Box::<super::signed_tester_sm::SignedTesterComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    Vec::new()
}

/// The record `signed_reading` read from the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
/// §10.3), or refused, naming where its value does not belong to its type. The inverse of
/// `canonical_signed_reading`.
pub fn read_signed_reading(
    value: &::sce_rust_runtime::json::Value,
) -> Result<super::signed_reading::SignedReadingPayload, ::sce_rust_runtime::test::TraceRefusal> {
    Ok({
        let record_0 = ::sce_rust_runtime::test::from_canonical::record(
            value,
            &["delta", "offset", "level", "stamp", "total"],
            "signed_reading",
        )?;
        super::signed_reading::SignedReadingPayload {
            delta: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "delta",
                    "signed_reading.delta",
                )?,
                "signed_reading.delta",
            )?,
            offset: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "offset",
                    "signed_reading.offset",
                )?,
                "signed_reading.offset",
            )?,
            level: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "level",
                    "signed_reading.level",
                )?,
                "signed_reading.level",
            )?,
            stamp: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "stamp",
                    "signed_reading.stamp",
                )?,
                "signed_reading.stamp",
            )?,
            total: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "total",
                    "signed_reading.total",
                )?,
                "signed_reading.total",
            )?,
        }
    })
}

/// The record `signed_reading` in the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
pub fn canonical_signed_reading(
    value: &super::signed_reading::SignedReadingPayload,
) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "delta",
            Some(::sce_rust_runtime::test::canonical::integer(value.delta)),
        ),
        (
            "offset",
            Some(::sce_rust_runtime::test::canonical::integer(value.offset)),
        ),
        (
            "level",
            Some(::sce_rust_runtime::test::canonical::integer(value.level)),
        ),
        (
            "stamp",
            Some(::sce_rust_runtime::test::canonical::integer(value.stamp)),
        ),
        (
            "total",
            Some(::sce_rust_runtime::test::canonical::integer(value.total)),
        ),
    ])
}

/// The reader of the inputs of a recorded trace (docs/SCE_TTCN3_LANE.md §10.2,
/// `sce_rust_runtime::test::InputReader`): the message an input of the record type `type_name`
/// on the system port `system_port` carries, its value read from the canonical form. Refused,
/// naming why, where the configuration has no such system port, the `in` list of its type does
/// not name the type, or the value does not belong to it.
pub fn input(
    system_port: &str,
    type_name: &str,
    value: &::sce_rust_runtime::json::Value,
) -> Result<::sce_rust_runtime::test::Message, ::sce_rust_runtime::test::TraceRefusal> {
    match (system_port, type_name) {
        ("sut", "signed_reading") => Ok(::sce_rust_runtime::test::Message::new(
            super::signed_reading::EVENT_NAME,
            read_signed_reading(value)?,
            "signed_reading",
            canonical_signed_reading,
        )),
        _ => Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(
            system_port,
            type_name,
            &["sut"],
        )),
    }
}
