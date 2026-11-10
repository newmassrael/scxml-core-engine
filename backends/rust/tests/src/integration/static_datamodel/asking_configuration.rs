// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01
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

/// The record `job_reply` read from the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), as a recorded input trace holds it (docs/SCE_TTCN3_LANE.md
/// §10.3), or refused, naming where its value does not belong to its type. The inverse of
/// `canonical_job_reply`.
pub fn read_job_reply(
    value: &::sce_rust_runtime::json::Value,
) -> Result<super::job_reply::JobReplyPayload, ::sce_rust_runtime::test::TraceRefusal> {
    Ok({
        let record_0 =
            ::sce_rust_runtime::test::from_canonical::record(value, &["job", "code"], "job_reply")?;
        super::job_reply::JobReplyPayload {
            job: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(record_0, "job", "job_reply.job")?,
                "job_reply.job",
            )?,
            code: ::sce_rust_runtime::test::from_canonical::integer(
                ::sce_rust_runtime::test::from_canonical::field(
                    record_0,
                    "code",
                    "job_reply.code",
                )?,
                "job_reply.code",
            )?,
        }
    })
}

/// The record `job_reply` in the canonical form of the test log
/// (docs/SCE_TEST_PROFILE.md §12.5), which a message of it a trace hands the run carries.
pub fn canonical_job_reply(
    value: &super::job_reply::JobReplyPayload,
) -> ::sce_rust_runtime::json::Value {
    ::sce_rust_runtime::test::canonical::record([
        (
            "job",
            Some(::sce_rust_runtime::test::canonical::integer(value.job)),
        ),
        (
            "code",
            Some(::sce_rust_runtime::test::canonical::integer(value.code)),
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
        ("sut", "job_reply") => Ok(::sce_rust_runtime::test::Message::new(
            super::job_reply::EVENT_NAME,
            read_job_reply(value)?,
            "job_reply",
            canonical_job_reply,
        )),
        _ => Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(
            system_port,
            type_name,
            &["sut"],
        )),
    }
}
