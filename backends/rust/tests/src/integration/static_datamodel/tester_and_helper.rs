// SCE-GENERATED — DO NOT EDIT
// source-hash: 4154e2a94e5761146caffd9e99419e570b7123aab1b11fb20a04685ab39515fd
#![doc = "SCE-MAP: tester_and_helper.scxml:5 :: _forge_body"]
// SCE-MAP: tester_and_helper.scxml:5 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="test-configuration")
// Runtime: sce_runtime
// Do not edit — regenerate from the source SCXML file.

/// The test configuration `tester_and_helper` (docs/SCE_TEST_PROFILE.md §11.6) as the
/// profile's runtime reads it: the MTC's component type and each PTC's, the ports and timers
/// each type declares, each `<sce:connect>` and each `<sce:map>`.
pub fn configuration() -> ::sce_rust_runtime::test::Configuration {
    ::sce_rust_runtime::test::Configuration {
        mtc_type: "job_tester".to_string(),
        ptc_types: ::std::collections::BTreeMap::from([(
            "helper".to_string(),
            "helper".to_string(),
        )]),
        ports_of_type: ::std::collections::BTreeMap::from([
            (
                "job_tester".to_string(),
                vec!["sut".to_string(), "ctl".to_string()],
            ),
            ("helper".to_string(), vec!["ctl".to_string()]),
        ]),
        timers_of_type: ::std::collections::BTreeMap::from([(
            "job_tester".to_string(),
            vec!["deadline".to_string(), "settle".to_string()],
        )]),
        connections: vec![(
            ::sce_rust_runtime::test::Endpoint::new("mtc", "ctl"),
            ::sce_rust_runtime::test::Endpoint::new("helper", "ctl"),
        )],
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
        "job_tester" => Box::<super::job_tester_sm::JobTesterComponent>::default(),
        "helper" => Box::<super::helper_sm::HelperComponent>::default(),
        _ => return None,
    };
    Some(machine)
}

/// The module parameters the components of the configuration read
/// (docs/SCE_TEST_PROFILE.md §11.5), each of which a suite that runs it gives a value.
pub fn module_parameters() -> Vec<String> {
    vec!["elapsed_limit".to_string()]
}

/// The record `job_status` read from the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
/// §10.3), or refused, naming where its value does not belong to its type. The inverse of
/// `canonical_job_status`.
pub fn read_job_status(
    value: &::sce_rust_runtime::json::Value,
) -> Result<super::job_status::JobStatusPayload, ::sce_rust_runtime::test::TraceRefusal> {
    Ok({
        let record_0 = ::sce_rust_runtime::test::from_canonical::record(
            value,
            &["elapsed_ms", "note", "codes", "tag", "retry", "count"],
            "job_status",
        )?;
        super::job_status::JobStatusPayload { elapsed_ms: ::sce_rust_runtime::test::from_canonical::integer(::sce_rust_runtime::test::from_canonical::field(record_0, "elapsed_ms", "job_status.elapsed_ms")?, "job_status.elapsed_ms")?, note: ::sce_rust_runtime::test::from_canonical::optional_field(record_0, "note").map(|present_0| -> ::std::result::Result<_, ::sce_rust_runtime::test::TraceRefusal> { let read = ::sce_rust_runtime::test::from_canonical::text(present_0, 32, "job_status.note")?; Ok(read) }).transpose()?, codes: { let mut items_0 = ::std::vec::Vec::new(); for item_0 in ::sce_rust_runtime::test::from_canonical::elements(::sce_rust_runtime::test::from_canonical::field(record_0, "codes", "job_status.codes")?, 0, 4, "job_status.codes")? { items_0.push(::sce_rust_runtime::test::from_canonical::integer(item_0, "job_status.codes[]")?); } ::sce_rust_runtime::test::from_canonical::list(items_0, "job_status.codes")? }, tag: ::sce_rust_runtime::test::from_canonical::optional_field(record_0, "tag").map(|present_0| -> ::std::result::Result<_, ::sce_rust_runtime::test::TraceRefusal> { let read = ::sce_rust_runtime::test::from_canonical::integer(present_0, "job_status.tag")?; Ok(read) }).transpose()?, retry: ::sce_rust_runtime::test::from_canonical::optional_field(record_0, "retry").map(|present_0| -> ::std::result::Result<_, ::sce_rust_runtime::test::TraceRefusal> { let read = ::sce_rust_runtime::test::from_canonical::integer(present_0, "job_status.retry")?; Ok(read) }).transpose()?, count: ::sce_rust_runtime::test::from_canonical::integer(::sce_rust_runtime::test::from_canonical::field(record_0, "count", "job_status.count")?, "job_status.count")? }
    })
}

/// The record `job_status` in the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
pub fn canonical_job_status(
    value: &super::job_status::JobStatusPayload,
) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "elapsed_ms",
            Some(::sce_rust_runtime::test::canonical::integer(
                value.elapsed_ms,
            )),
        ),
        (
            "note",
            value
                .note
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::text(present_0.as_str())),
        ),
        (
            "codes",
            Some(::sce_rust_runtime::test::canonical::list(
                value
                    .codes
                    .as_slice()
                    .iter()
                    .map(|item_0| ::sce_rust_runtime::test::canonical::integer(*item_0)),
            )),
        ),
        (
            "tag",
            value
                .tag
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::integer(*present_0)),
        ),
        (
            "retry",
            value
                .retry
                .as_ref()
                .map(|present_0| ::sce_rust_runtime::test::canonical::integer(*present_0)),
        ),
        (
            "count",
            Some(::sce_rust_runtime::test::canonical::integer(value.count)),
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
        ("sut", "job_status") => Ok(::sce_rust_runtime::test::Message::new(
            super::job_status::EVENT_NAME,
            read_job_status(value)?,
            "job_status",
            canonical_job_status,
        )),
        _ => Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(
            system_port,
            type_name,
            &["sut"],
        )),
    }
}
