// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Rust (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its entry
//! of the runner, through its configuration's unit; a suite's unit its module
//! parameters at their defaults, as saved values of their types, and its test
//! cases in run order, each with its guard and its selection. A port type
//! writes no file. The twin of `a_test_suite_is_generated_in_cpp.rs`, over the
//! same fixtures.
//!
//! What the generated tables do when they run is held by
//! `backends/rust/tests/tests/a_test_suite_runs_through_its_runner.rs`, which
//! compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_rust.sh`) and runs the suite through
//! `sce_rust_runtime::test::run_suite`.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The directory of the documents the runtime's end-to-end tests run.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The directory of the documents over imported records and enums.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Rust, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
}

/// The one file the document at `path`, named `name`, generates on Rust,
/// `<name>.rs`.
fn file_at(path: &Path, name: &str) -> String {
    let files = generate(path).unwrap_or_else(|e| panic!("{name} generates on Rust: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.rs").as_str()], "{name}");
    files[0].1.clone()
}

/// The one file the runtime fixture `name` generates on Rust.
fn unit(name: &str) -> String {
    file_at(&runtime_dir().join(format!("{name}.scxml")), name)
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// The text of `code` from `from` up to `to`, both of which it holds in that
/// order.
fn between<'c>(code: &'c str, from: &str, to: &str) -> &'c str {
    let start = at(code, from);
    let end = start + at(&code[start..], to);
    &code[start..end]
}

/// §11.6, §12.2: a configuration is the table the runtime reads, its
/// components' machines named by their documents, and the module parameters
/// those components read.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = unit("asking_configuration");
    for needle in [
        "pub fn configuration() -> ::sce_rust_runtime::test::Configuration {",
        "mtc_type: \"param_asker\".to_string(),",
        "ptc_types: ::std::collections::BTreeMap::new(),",
        "connections: Vec::new(),",
        "(::sce_rust_runtime::test::Endpoint::new(\"mtc\", \"sut\"), \"sut\".to_string()),",
        "pub fn machine(component_type: &str) -> \
         Option<Box<dyn ::sce_rust_runtime::test::ComponentMachine>> {",
        "\"param_asker\" => Box::<super::param_asker_sm::ParamAskerComponent>::default(),",
        "_ => return None,",
        "pub fn module_parameters() -> Vec<String> {",
        "vec![\"job\".to_string()]",
    ] {
        assert!(
            asking.contains(needle),
            "asking_configuration holds `{needle}`"
        );
    }
    let ports = between(&asking, "ports_of_type:", "timers_of_type:");
    assert!(
        ports.contains("(\"param_asker\".to_string(), vec![\"sut\".to_string()]),"),
        "{ports}"
    );
    let timers = between(&asking, "timers_of_type:", "connections:");
    assert!(
        timers.contains("(\"param_asker\".to_string(), vec![\"deadline\".to_string()]),"),
        "{timers}"
    );
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is named"
    );
    assert!(
        !asking.contains("\nuse "),
        "every unit is named through its sibling module"
    );

    let serving = unit("serving_configuration");
    for needle in [
        "mtc_type: \"coordinator\".to_string(),",
        "(\"answerer\".to_string(), \"answerer\".to_string()),",
        "(::sce_rust_runtime::test::Endpoint::new(\"mtc\", \"peer\"), \
         ::sce_rust_runtime::test::Endpoint::new(\"answerer\", \"peer\")),",
        "mappings: Vec::new(),",
        "timers_of_type: ::std::collections::BTreeMap::new(),",
        "\"answerer\" => Box::<super::answerer_sm::AnswererComponent>::default(),",
    ] {
        assert!(
            serving.contains(needle),
            "serving_configuration holds `{needle}`"
        );
    }
    assert!(
        at(
            &serving,
            "\"coordinator\" => Box::<super::coordinator_sm::CoordinatorComponent>"
        ) < at(
            &serving,
            "\"answerer\" => Box::<super::answerer_sm::AnswererComponent>"
        ),
        "the MTC's type comes first"
    );
    let reads = &serving[at(&serving, "pub fn module_parameters() -> Vec<String> {")..];
    assert!(
        reads.contains("Vec::new()") && !reads.contains("vec!["),
        "a configuration whose components read none lists none: {reads}"
    );

    let keeping = unit("keeping_configuration");
    for needle in [
        "(\"ptc_keeper\".to_string(), vec![\"patience\".to_string(), \"linger\".to_string()]),",
        "(\"idler\".to_string(), vec![\"peer\".to_string()]),",
        "(::sce_rust_runtime::test::Endpoint::new(\"idler\", \"peer\"), \"quiet\".to_string()),",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`"
        );
    }
    let ports = between(&keeping, "ports_of_type:", "timers_of_type:");
    assert!(
        !ports.contains("\"ptc_keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// docs/SCE_TTCN3_LANE.md §10.2, §10.5: a configuration reads the inputs of a
/// recorded trace. Each record on the `in` list of a system port's type is read
/// back from its canonical form and written in it, once, through the sibling
/// module of its event schema; an input on a port the configuration does not
/// have, or of a type its port does not take, is refused by name, and a
/// record only sent to the system under test is not read.
#[test]
fn a_configuration_reads_the_inputs_of_a_recorded_trace() {
    let asking = unit("asking_configuration");
    for needle in [
        "fn read_job_reply(value: &::sce_rust_runtime::json::Value) -> \
         Result<super::job_reply::JobReplyPayload, ::sce_rust_runtime::test::TraceRefusal> {",
        "let record_0 = ::sce_rust_runtime::test::from_canonical::record(value, \
         &[\"job\", \"code\"], \"job_reply\")?;",
        "super::job_reply::JobReplyPayload { job: \
         ::sce_rust_runtime::test::from_canonical::integer(\
         ::sce_rust_runtime::test::from_canonical::field(record_0, \"job\", \"job_reply.job\")?, \
         \"job_reply.job\")?, code: ",
        "fn canonical_job_reply(value: &super::job_reply::JobReplyPayload) -> \
         ::sce_rust_runtime::json::Value {",
        "::sce_rust_runtime::test::canonical::integer(value.code)",
        "pub fn input(system_port: &str, type_name: &str, value: &::sce_rust_runtime::json::Value) \
         -> Result<::sce_rust_runtime::test::Message, ::sce_rust_runtime::test::TraceRefusal> {",
        "(\"sut\", \"job_reply\") => Ok(::sce_rust_runtime::test::Message::new(\
         super::job_reply::EVENT_NAME, read_job_reply(value)?, \"job_reply\", canonical_job_reply)),",
        "_ => Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(system_port, type_name, \
         &[\"sut\"])),",
    ] {
        assert!(
            asking.contains(needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    assert!(
        !asking.contains("read_job_request"),
        "a record the components only send is not read: {asking}"
    );
    assert_eq!(
        asking.matches("fn read_job_reply(").count(),
        1,
        "a record is read by one function"
    );

    let keeping = unit("keeping_configuration");
    assert!(
        keeping.contains(
            "(\"quiet\", \"job_reply\") => Ok(::sce_rust_runtime::test::Message::new(\
             super::job_reply::EVENT_NAME, read_job_reply(value)?, \"job_reply\", \
             canonical_job_reply)),"
        ),
        "{keeping}"
    );

    let serving = unit("serving_configuration");
    assert!(
        serving.contains(
            "pub fn input(system_port: &str, type_name: &str, _value: \
             &::sce_rust_runtime::json::Value) -> Result<::sce_rust_runtime::test::Message, \
             ::sce_rust_runtime::test::TraceRefusal> {\n    \
             Err(::sce_rust_runtime::test::TraceRefusal::not_an_input(system_port, type_name, \
             &[]))\n}"
        ),
        "a configuration with no system port refuses every input: {serving}"
    );
    assert!(!serving.contains("fn read_"), "{serving}");
}

/// docs/SCE_TTCN3_LANE.md §10.3: the configuration whose system port takes a
/// record of every form of the canonical value generates a public reader and
/// writer of each, which `backends/rust/tests/tests/a_record_reads_back_the_form_it_writes.rs`
/// compiles and round-trips, in shapes clippy's `-D warnings` does not refuse.
#[test]
fn a_configuration_reads_a_record_of_every_form() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/trace_forms/forms_configuration.scxml");
    let forms = file_at(&path, "forms_configuration");
    for record in ["order_placed", "job_status", "form_extras"] {
        for needle in [
            format!("pub fn read_{record}(value: &::sce_rust_runtime::json::Value)"),
            format!("pub fn canonical_{record}(value: &"),
            format!("(\"sut\", \"{record}\") => Ok(::sce_rust_runtime::test::Message::new("),
        ] {
            assert!(forms.contains(&needle), "`{needle}` in\n{forms}");
        }
    }
    for needle in [
        "::sce_rust_runtime::test::from_canonical::open_enumerated(",
        "::sce_rust_runtime::test::from_canonical::nullable(",
        "::sce_rust_runtime::test::from_canonical::union_case(",
        "::sce_rust_runtime::test::from_canonical::bytes(",
        ".transpose()?",
    ] {
        assert!(forms.contains(needle), "`{needle}` in\n{forms}");
    }
    // The readers and writers stand between the factory of the machines, whose
    // `return None` is the template's, and `input`.
    let readers = between(&forms, "pub fn read_", "pub fn input(");
    for refused in ["return ", "None => None", "while let", "Ok(read?)"] {
        assert!(!readers.contains(refused), "`{refused}` in\n{readers}");
    }
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the configuration's unit.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = unit("answers_the_job");
    for needle in [
        "pub const NAME: &str = \"answers_the_job\";",
        "pub fn test_case() -> ::sce_rust_runtime::test::TestCase {",
        "name: NAME.to_string(),",
        "configuration: super::asking_configuration::configuration(),",
        "machines: ::std::sync::Arc::new(super::asking_configuration::machine),",
        "module_parameters: super::asking_configuration::module_parameters(),",
        "inputs: super::asking_configuration::input,",
    ] {
        assert!(case.contains(needle), "answers_the_job holds `{needle}`");
    }
}

/// The text of one `<sce:execute>` of a suite's unit: its test case, its
/// guard and its selection.
fn execute(test_case: &str, guard: &str, selection: &str) -> String {
    format!(
        "test_case: super::{test_case}::test_case(),\n                guard: {guard},\n                \
         selection: {selection},"
    )
}

/// The selection that holds in a world that enables what `body` tests.
fn selects(body: &str) -> String {
    format!("Some(|world: &::sce_rust_runtime::test::Features| {body})")
}

/// The test of a world that enables `feature`.
fn enabled(feature: &str) -> String {
    format!("::sce_rust_runtime::test::enabled(world, \"{feature}\")")
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard and its selection over the
/// features, the outermost join of a selection bare as a closure body is; the
/// feature model writes no unit, so none is named.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = unit("runtime_suite");
    for needle in [
        "pub fn suite() -> ::sce_rust_runtime::test::Suite {",
        "let mut module_parameters = ::sce_rust_runtime::test::ModuleParameters::default();",
        "module_parameters.set(\"job\", <u32 as \
         ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&7));",
        "name: \"runtime_suite\".to_string(),",
    ] {
        assert!(suite.contains(needle), "runtime_suite holds `{needle}`");
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    let executes = [
        execute("answers_the_job", "Some(10000000)", "None"),
        execute("serves_a_ptc", "None", &selects(&enabled("ptcs"))),
        execute(
            "keeps_a_ptc",
            "None",
            &selects(&format!("{} && {}", enabled("ptcs"), enabled("removal"))),
        ),
        execute(
            "answers_before_the_guard",
            "Some(100)",
            &selects(&format!("{} || {}", enabled("guard"), enabled("removal"))),
        ),
    ];
    let mut last = 0;
    for execute in executes {
        let found = at(&suite, &execute);
        assert!(found > last, "the test cases are in run order: `{execute}`");
        last = found;
    }
}

/// docs/SCE_TTCN3_LANE.md §4: a suite's unit is the runner program's: its
/// `main` hands the runner the suite and the reader of each module parameter's
/// value, a literal of its type, which an undeclared parameter is refused by.
#[test]
fn a_suite_is_the_runner_program_of_its_traces() {
    let suite = unit("runtime_suite");
    for needle in [
        "pub fn module_parameter(id: &str, literal: &str) -> \
         Result<::sce_rust_runtime::test::ModuleParameterValue, \
         ::sce_rust_runtime::test::OverrideRefusal> {",
        "\"job\" => ::sce_rust_runtime::test::runner::integer_parameter::<u32>(id, literal),",
        "_ => Err(::sce_rust_runtime::test::runner::undeclared_parameter(id)),",
        "pub fn main() -> ::std::process::ExitCode {",
        "::sce_rust_runtime::test::runner::main(&suite(), module_parameter, \
         ::std::env::args().skip(1))",
    ] {
        assert!(
            suite.contains(needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
}

/// A suite whose selection joins a join keeps the inner one in parentheses,
/// and one that declares no module parameter and runs no test case holds the
/// empty tables.
#[test]
fn a_selection_keeps_its_inner_joins_and_an_empty_suite_its_empty_tables() {
    let dir = tempfile::tempdir().expect("a directory");
    let nested = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="nested">
  <sce:import kind="feature-model" src="{runtime}/runtime_features.scxml" as="Features"/>
  <sce:import kind="test-case" src="{runtime}/serves_a_ptc.scxml" as="ServesAPtc"/>
  <sce:execute test-case="ServesAPtc" requires="all(ptcs, any(guard, removal))"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("nested.scxml");
    std::fs::write(&path, nested).expect("the suite is written");
    let text = file_at(&path, "nested");
    let body = format!(
        "{} && ({} || {})",
        enabled("ptcs"),
        enabled("guard"),
        enabled("removal")
    );
    assert!(
        text.contains(&execute("serves_a_ptc", "None", &selects(&body))),
        "{text}"
    );
    assert!(
        text.contains("module_parameters: ::sce_rust_runtime::test::ModuleParameters::default(),"),
        "a suite that declares no module parameter gives none: {text}"
    );
    assert!(!text.contains("let mut module_parameters"), "{text}");

    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="empty_suite">
</scxml>
"#;
    let path = dir.path().join("empty_suite.scxml");
    std::fs::write(&path, empty).expect("the suite is written");
    let text = file_at(&path, "empty_suite");
    assert!(text.contains("executes: Vec::new(),"), "{text}");
    assert!(
        text.contains(
            "pub fn module_parameter(id: &str, _literal: &str) -> \
             Result<::sce_rust_runtime::test::ModuleParameterValue, \
             ::sce_rust_runtime::test::OverrideRefusal> {\n    \
             Err(::sce_rust_runtime::test::runner::undeclared_parameter(id))\n}"
        ),
        "a suite that declares no module parameter refuses every value: {text}"
    );
}

/// §11.8, §12.2: each module parameter's default is the saved value of its
/// type, the one a machine's saved state writes a value of that type as: an
/// integer by the writer of its type, a value of an enum as the name its
/// document declares for the variant, and a byte string as its Latin-1 text.
#[test]
fn a_module_parameter_default_is_the_saved_value_of_its_type() {
    let dir = tempfile::tempdir().expect("a directory");
    let suite = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="typed_defaults">
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Paint"/>
  <sce:import kind="test-case" src="{runtime}/answers_the_job.scxml" as="AnswersTheJob"/>
  <sce:module-parameter id="job" type="uint32" value="7"/>
  <sce:module-parameter id="armed" type="bool" value="true"/>
  <sce:module-parameter id="offset" type="int8" value="-5"/>
  <sce:module-parameter id="widest" type="uint64" value="18446744073709551615"/>
  <sce:module-parameter id="label" type="string[8]" value="nightly"/>
  <sce:module-parameter id="key" type="bytes[2]" value="0aff"/>
  <sce:module-parameter id="color" type="enum:Paint" value="green"/>
  <sce:execute test-case="AnswersTheJob"/>
</scxml>
"#,
        imports = imports_dir().display(),
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("typed_defaults.scxml");
    std::fs::write(&path, suite).expect("the suite is written");
    let text = file_at(&path, "typed_defaults");
    for needle in [
        "module_parameters.set(\"armed\", ::sce_rust_runtime::json::Value::Bool(true));",
        "module_parameters.set(\"offset\", <i8 as \
         ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&-5));",
        "module_parameters.set(\"widest\", <u64 as \
         ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&18446744073709551615));",
        "module_parameters.set(\"label\", \
         ::sce_rust_runtime::json::Value::Text(\"nightly\".to_string()));",
        "module_parameters.set(\"key\", \
         ::sce_rust_runtime::saved_state::bytes_to_saved(b\"\\x0a\\xff\"));",
        "module_parameters.set(\"color\", \
         ::sce_rust_runtime::json::Value::Text(\"green\".to_string()));",
        &execute("answers_the_job", "None", "None"),
        // The runner reads each value given on its command line by its type.
        "\"armed\" => ::sce_rust_runtime::test::runner::bool_parameter(id, literal),",
        "\"offset\" => ::sce_rust_runtime::test::runner::integer_parameter::<i8>(id, literal),",
        "\"widest\" => ::sce_rust_runtime::test::runner::integer_parameter::<u64>(id, literal),",
        "\"label\" => Ok(::sce_rust_runtime::test::runner::text_parameter(literal)),",
        "\"key\" => ::sce_rust_runtime::test::runner::bytes_parameter(id, literal),",
        "\"color\" => Ok(::sce_rust_runtime::test::runner::enum_parameter(literal)),",
    ] {
        assert!(
            text.contains(needle),
            "typed_defaults holds `{needle}`:\n{text}"
        );
    }
    assert!(
        !text.contains("paint_color"),
        "an enum's default is its variant's name, so its unit is not named"
    );
}

/// §3.3, §12.4: a guard past the microseconds the runtime's clock counts to is
/// refused by name rather than wrapped.
#[test]
fn a_guard_past_the_runtimes_clock_is_refused() {
    let dir = tempfile::tempdir().expect("a directory");
    let suite = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="endless">
  <sce:import kind="test-case" src="{runtime}/answers_the_job.scxml" as="AnswersTheJob"/>
  <sce:module-parameter id="job" type="uint32" value="7"/>
  <sce:execute test-case="AnswersTheJob" guard="9300000000000s"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("endless.scxml");
    std::fs::write(&path, suite).expect("the suite is written");
    let Err(err) = generate(&path) else {
        panic!("a guard past the runtime's clock generated on Rust");
    };
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{err:?}"
    );
    let ForgeError::Generate(boxed) = err.error else {
        panic!("not a generation refusal");
    };
    let GenerateError::UnsupportedFeature { detail, .. } = *boxed else {
        panic!("not an unsupported feature");
    };
    assert!(detail.starts_with("test-suite 'endless': "), "{detail}");
    assert!(
        detail.contains("the guard of 'AnswersTheJob' is 9300000000000000000 microseconds"),
        "{detail}"
    );
}

/// §11.3, §11.9: a port type writes no file on Rust: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on Rust: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on Rust, the suite's selection over the
/// example feature model.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
    let suite = file_at(&examples.join("test-suite.scxml"), "test_suite");
    for needle in [
        "module_parameters.set(\"elapsed_limit\", <u32 as \
         ::sce_rust_runtime::saved_state::SavedValue>::to_saved(&500));"
            .to_string(),
        execute(
            "test_case",
            "Some(10000000)",
            &selects(&enabled("session-unicast")),
        ),
    ] {
        assert!(
            suite.contains(&needle),
            "the example suite holds `{needle}`"
        );
    }
    let case = file_at(&examples.join("test-case.scxml"), "test_case");
    assert!(case.contains("configuration: super::test_configuration::configuration(),"));
    let configuration = file_at(
        &examples.join("test-configuration.scxml"),
        "test_configuration",
    );
    assert!(configuration.contains("vec![\"elapsed_limit\".to_string()]"));
}

/// docs/SCE_TTCN3_LANE.md §10.5: two records a configuration's system ports
/// take that the export would name alike, the stems of their files, are
/// refused by name, since one reader and one type of the test log would hold
/// both. The twin of the Kotlin and C++ tests of the same name, over the one
/// refusal of the shared rows (`test_tables::trace_records`).
#[test]
fn two_records_named_alike_are_refused_by_name() {
    let dir = tempfile::tempdir().expect("a directory");
    let record = |name: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="{name}" sce:event-name="{name}.event">
  <datamodel>
    <data id="n" sce:type="uint8" sce:direction="in"/>
  </datamodel>
</scxml>
"#
        )
    };
    for (folder, name) in [("a", "twin_a"), ("b", "twin_b")] {
        let folder = dir.path().join(folder);
        std::fs::create_dir(&folder).expect("a folder");
        std::fs::write(folder.join("twin.scxml"), record(name)).expect("the record is written");
    }
    let documents = [
        (
            "twins_port.scxml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="twins_port">
  <sce:import kind="event-schema" src="a/twin.scxml" as="TwinA"/>
  <sce:import kind="event-schema" src="b/twin.scxml" as="TwinB"/>
  <sce:message type="TwinA" direction="in"/>
  <sce:message type="TwinB" direction="in"/>
</scxml>
"#,
        ),
        (
            "twins_listener.scxml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="twins_listener"
       datamodel="sce-static" initial="waiting">
  <sce:import kind="test-port-type" src="twins_port.scxml" as="TwinsPort"/>
  <sce:test-port id="sut" type="TwinsPort"/>
  <state id="waiting">
    <transition sce:receive="sut" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        ),
        (
            "twins_configuration.scxml",
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-configuration" name="twins_configuration">
  <sce:import kind="test-component" src="twins_listener.scxml" as="TwinsListener"/>
  <sce:import kind="test-port-type" src="twins_port.scxml" as="TwinsPort"/>
  <sce:mtc type="TwinsListener"/>
  <sce:system-port id="sut" type="TwinsPort"/>
  <sce:map component="mtc" port="sut" system-port="sut"/>
</scxml>
"#,
        ),
    ];
    for (file, text) in documents {
        std::fs::write(dir.path().join(file), text).expect("the document is written");
    }
    let Err(err) = generate(&dir.path().join("twins_configuration.scxml")) else {
        panic!("two records named alike generated on Rust");
    };
    let ForgeError::Generate(boxed) = err.error else {
        panic!("not a generation refusal: {err:?}");
    };
    let GenerateError::UnsupportedFeature { detail, .. } = *boxed else {
        panic!("not an unsupported feature");
    };
    assert!(
        detail.starts_with("test-configuration 'twins_configuration': "),
        "{detail}"
    );
    assert!(
        detail.contains("two records its system ports take are named 'twin': "),
        "{detail}"
    );
}
