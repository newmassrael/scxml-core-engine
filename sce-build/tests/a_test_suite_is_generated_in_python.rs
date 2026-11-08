// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Python (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its entry
//! of the runner, through its configuration's unit; a suite's unit its module
//! parameters at their defaults, as saved values of their types, and its test
//! cases in run order, each with its guard and its selection. A port type
//! writes no file. The twin of `a_test_suite_is_generated_in_go.rs`, over the
//! same fixtures.
//!
//! A Python unit imports each module it reads as a sibling, under the alias
//! every Python unit imports a sibling by (`from . import <snake> as
//! sce_<snake>`): a suite its test cases', a test case its configuration's,
//! and a configuration each component's machine, which is the module the
//! component's statechart is generated as, `<machine>_sm`, beside it.
//!
//! What the generated tables do when they run is held by
//! `backends/python/tests/integration/static_datamodel/test_a_test_suite_runs_through_its_runner.py`,
//! which imports them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_python.sh`) and runs the suite through
//! `sce_runtime.testprofile.run_suite`.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The import of the profile's runtime, which every table names.
const RUNTIME_IMPORT: &str = "\nimport sce_runtime.testprofile as _sce_testprofile\n";

/// The import of the runtime's writers of a saved value, which a suite's
/// defaults are written by.
const SAVED_VALUES_IMPORT: &str = "\nimport sce_runtime.saved_values as _sce_saved\n";

/// The directory of the documents the runtime's end-to-end tests run.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The directory of the documents over imported records and enums.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// The directory of the kinds' examples.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples")
}

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Python, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
}

/// The one file the document at `path` generates on Python, `<file>.py`.
fn file_at(path: &Path, file: &str) -> String {
    let files = generate(path).unwrap_or_else(|e| panic!("{file} generates on Python: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, [format!("{file}.py").as_str()], "{file}");
    files[0].1.clone()
}

/// The one file the runtime fixture `name` generates on Python, `<name>.py`.
fn unit(name: &str) -> String {
    file_at(&runtime_dir().join(format!("{name}.scxml")), name)
}

/// Each Python file the statechart at `path`, which generates, is written as,
/// by name.
fn machine_files(path: &Path) -> Vec<(String, String)> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Python),
        Language::Python,
    )
    .unwrap_or_else(|e| panic!("{} generates on Python: {e:?}", path.display()))
    .files
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

/// The import of the sibling module `module`, under the alias every Python
/// unit imports a sibling by.
fn imported(module: &str) -> String {
    format!("\nfrom . import {module} as sce_{module}\n")
}

/// §11.6, §12.2: a configuration is the table the runtime reads, its
/// components' machines named through their modules, and the module
/// parameters those components read; every table is written, an empty one as
/// empty.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = unit("asking_configuration");
    for needle in [
        RUNTIME_IMPORT.to_string(),
        imported("param_asker_sm"),
        "\nfrom typing import List, Optional\n".to_string(),
        "def configuration() -> _sce_testprofile.Configuration:".to_string(),
        "    return _sce_testprofile.Configuration(\n".to_string(),
        "        mtc_type=\"param_asker\",\n".to_string(),
        "        ptc_types={},\n".to_string(),
        "        connections=[],\n".to_string(),
        "            _sce_testprofile.Mapping(port=_sce_testprofile.Endpoint(component=\"mtc\", \
         port=\"sut\"), system_port=\"sut\"),\n"
            .to_string(),
        "def machine(component_type: str) -> Optional[_sce_testprofile.ComponentMachine]:"
            .to_string(),
        "    if component_type == \"param_asker\":\n        return \
         sce_param_asker_sm.ParamAskerComponent()\n"
            .to_string(),
        "    return None\n".to_string(),
        "def module_parameters() -> List[str]:".to_string(),
    ] {
        assert!(
            asking.contains(&needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    // The last statement of the unit: a render ends without the newline the
    // write adds (`generator::with_trailing_newline`).
    assert!(
        asking.ends_with("    return [\"job\"]"),
        "asking_configuration ends with its module parameters:\n{asking}"
    );
    let ports = between(&asking, "ports_of_type=", "timers_of_type=");
    assert!(
        ports.contains("            \"param_asker\": [\"sut\"],\n"),
        "{ports}"
    );
    let timers = between(&asking, "timers_of_type=", "connections=");
    assert!(
        timers.contains("            \"param_asker\": [\"reply\"],\n"),
        "{timers}"
    );
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is named or imported"
    );

    let serving = unit("serving_configuration");
    for needle in [
        "        mtc_type=\"coordinator\",\n".to_string(),
        "            \"answerer\": \"answerer\",\n".to_string(),
        "            _sce_testprofile.Connection(a=_sce_testprofile.Endpoint(component=\"mtc\", \
         port=\"peer\"), b=_sce_testprofile.Endpoint(component=\"answerer\", port=\"peer\")),\n"
            .to_string(),
        "        mappings=[],\n".to_string(),
        "        timers_of_type={},\n".to_string(),
        "        return sce_answerer_sm.AnswererComponent()\n".to_string(),
        "    return []\n".to_string(),
        imported("coordinator_sm"),
        imported("answerer_sm"),
    ] {
        assert!(
            serving.contains(&needle),
            "serving_configuration holds `{needle}`:\n{serving}"
        );
    }
    assert!(
        at(&serving, "    if component_type == \"coordinator\":")
            < at(&serving, "    if component_type == \"answerer\":"),
        "the MTC's type comes first"
    );

    let keeping = unit("keeping_configuration");
    for needle in [
        "            \"ptc_keeper\": [\"patience\", \"linger\"],\n",
        "            \"idler\": [\"peer\"],\n",
        "            _sce_testprofile.Mapping(port=_sce_testprofile.Endpoint(component=\"idler\", \
         port=\"peer\"), system_port=\"quiet\"),\n",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`:\n{keeping}"
        );
    }
    let ports = between(&keeping, "ports_of_type=", "timers_of_type=");
    assert!(
        !ports.contains("\"ptc_keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// §12.3, §12.6: a configuration imports a component's machine from the
/// module its statechart is generated as, `<machine>_sm`, beside it, and names
/// the component that module declares.
#[test]
fn a_configuration_names_each_component_where_its_statechart_declares_it() {
    let asking = unit("asking_configuration");
    let machine = machine_files(&runtime_dir().join("param_asker.scxml"));
    let names: Vec<&str> = machine.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["param_asker_sm.py"]);
    let machine = &machine[0].1;
    assert!(
        machine.contains("\nclass ParamAskerComponent:\n"),
        "{machine}"
    );
    assert!(asking.contains(&imported("param_asker_sm")), "{asking}");
    assert!(
        asking.contains("return sce_param_asker_sm.ParamAskerComponent()"),
        "{asking}"
    );
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the module of the configuration's unit. Its entry begins as a test
/// of pytest's does, so it says it is none.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = unit("answers_the_job");
    for needle in [
        RUNTIME_IMPORT.to_string(),
        imported("asking_configuration"),
        "\nNAME = \"answers_the_job\"\n".to_string(),
        "def test_case() -> _sce_testprofile.TestCase:".to_string(),
        "    return _sce_testprofile.TestCase(\n".to_string(),
        "        name=NAME,\n".to_string(),
        "        configuration=sce_asking_configuration.configuration(),\n".to_string(),
        "        machines=sce_asking_configuration.machine,\n".to_string(),
        "        module_parameters=sce_asking_configuration.module_parameters(),\n".to_string(),
        "\ntest_case.__test__ = False".to_string(),
    ] {
        assert!(
            case.contains(&needle),
            "answers_the_job holds `{needle}`:\n{case}"
        );
    }
    assert!(!case.contains(SAVED_VALUES_IMPORT), "{case}");
}

/// The text of one `<sce:execute>` of a suite's unit: its test case, its
/// guard and its selection.
fn execute(test_case: &str, guard: &str, selection: &str) -> String {
    let indent = " ".repeat(16);
    format!(
        "{indent}test_case=sce_{test_case}.test_case(),\n{indent}guard={guard},\n\
         {indent}selection={selection},\n"
    )
}

/// The selection that holds in a world that enables what `body` tests.
fn selects(body: &str) -> String {
    format!("lambda world: {body}")
}

/// The test of a world that enables `feature`.
fn enabled(feature: &str) -> String {
    format!("_sce_testprofile.enabled(world, \"{feature}\")")
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard in microseconds, or `None`,
/// and its selection over the features in Python's `and` and `or`, the
/// outermost join bare as a lambda's body is; the feature model writes no
/// unit, so none is imported.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = unit("runtime_suite");
    for needle in [
        RUNTIME_IMPORT.to_string(),
        SAVED_VALUES_IMPORT.to_string(),
        imported("answers_the_job"),
        imported("serves_a_ptc"),
        imported("keeps_a_ptc"),
        imported("answers_before_the_guard"),
        "def suite() -> _sce_testprofile.Suite:".to_string(),
        "    module_parameters = _sce_testprofile.ModuleParameters()\n".to_string(),
        "    module_parameters.set(\"job\", _sce_saved.saved_write_uint32(7))\n".to_string(),
        "        name=\"runtime_suite\",\n".to_string(),
        "        module_parameters=module_parameters,\n".to_string(),
    ] {
        assert!(
            suite.contains(&needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    let executes = [
        execute("answers_the_job", "10000000", "None"),
        execute("serves_a_ptc", "None", &selects(&enabled("ptcs"))),
        execute(
            "keeps_a_ptc",
            "None",
            &selects(&format!("{} and {}", enabled("ptcs"), enabled("removal"))),
        ),
        execute(
            "answers_before_the_guard",
            "100",
            &selects(&format!("{} or {}", enabled("guard"), enabled("removal"))),
        ),
    ];
    let mut last = 0;
    for execute in executes {
        let found = at(&suite, &execute);
        assert!(found > last, "the test cases are in run order: `{execute}`");
        last = found;
    }
}

/// A suite whose selection joins a join keeps the inner one in parentheses;
/// one that imports one test case under two aliases imports its module once
/// and runs each alias; and one that declares no module parameter and runs no
/// test case holds the empty tables and imports neither the writers of a saved
/// value nor a test case.
#[test]
fn a_selection_keeps_its_inner_joins_a_case_is_imported_once_and_an_empty_suite_is_empty() {
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
        "{} and ({} or {})",
        enabled("ptcs"),
        enabled("guard"),
        enabled("removal")
    );
    assert!(
        text.contains(&execute("serves_a_ptc", "None", &selects(&body))),
        "{text}"
    );
    assert!(
        text.contains("    module_parameters = _sce_testprofile.ModuleParameters()\n"),
        "{text}"
    );
    assert!(
        !text.contains("module_parameters.set("),
        "a suite that declares no module parameter gives none: {text}"
    );
    assert!(
        !text.contains("_sce_saved"),
        "a suite that writes no default imports no writer: {text}"
    );

    // An alias is executed once (a second `<sce:execute>` of it is refused as
    // a duplicate), but one document may be imported under two aliases: both
    // rows then name one module, which is imported once.
    let twice = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="twice">
  <sce:import kind="test-case" src="{runtime}/answers_the_job.scxml" as="AnswersTheJob"/>
  <sce:import kind="test-case" src="{runtime}/answers_the_job.scxml" as="AnswersAgain"/>
  <sce:module-parameter id="job" type="uint32" value="7"/>
  <sce:execute test-case="AnswersTheJob"/>
  <sce:execute test-case="AnswersAgain" guard="1s"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("twice.scxml");
    std::fs::write(&path, twice).expect("the suite is written");
    let text = file_at(&path, "twice");
    assert_eq!(
        text.matches(&imported("answers_the_job")).count(),
        1,
        "{text}"
    );
    assert!(
        at(&text, &execute("answers_the_job", "None", "None"))
            < at(&text, &execute("answers_the_job", "1000000", "None")),
        "{text}"
    );

    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="empty_suite">
</scxml>
"#;
    let path = dir.path().join("empty_suite.scxml");
    std::fs::write(&path, empty).expect("the suite is written");
    let text = file_at(&path, "empty_suite");
    assert!(text.contains("        executes=[],\n"), "{text}");
    assert!(text.contains(RUNTIME_IMPORT), "{text}");
    assert!(!text.contains("_sce_saved"), "{text}");
    assert!(
        !text.contains("\nfrom . import "),
        "a suite that runs no test case imports none: {text}"
    );
}

/// §11.8, §12.2: each module parameter's default is the saved value of its
/// type, the one a machine's saved state writes a value of that type as: a
/// number, a `bool`, a text and a byte string by the writer of the type, and a
/// value of an enum as the name its document declares for the variant, written
/// as a text is.
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
        "    module_parameters.set(\"job\", _sce_saved.saved_write_uint32(7))\n".to_string(),
        "    module_parameters.set(\"armed\", _sce_saved.saved_write_bool(True))\n".to_string(),
        "    module_parameters.set(\"offset\", _sce_saved.saved_write_int8(-5))\n".to_string(),
        "    module_parameters.set(\"widest\", \
         _sce_saved.saved_write_uint64(18446744073709551615))\n"
            .to_string(),
        "    module_parameters.set(\"label\", _sce_saved.saved_write_string(\"nightly\"))\n"
            .to_string(),
        r#"    module_parameters.set("key", _sce_saved.saved_write_bytes(b"\x0a\xff"))"#
            .to_string(),
        "    module_parameters.set(\"color\", _sce_saved.saved_write_string(\"green\"))\n"
            .to_string(),
        SAVED_VALUES_IMPORT.to_string(),
        execute("answers_the_job", "None", "None"),
    ] {
        assert!(
            text.contains(&needle),
            "typed_defaults holds `{needle}`:\n{text}"
        );
    }
    assert!(
        !text.contains("paint_color"),
        "an enum's default is its variant's name, so its unit is not imported"
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
        panic!("a guard past the runtime's clock generated on Python");
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

/// §11.3, §11.9: a port type writes no file on Python: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on Python: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on Python, the suite's selection over
/// the example feature model, and the example component reads the module
/// parameter its configuration names.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = examples_dir();
    let suite = file_at(&examples.join("test-suite.scxml"), "test_suite");
    for needle in [
        "    module_parameters.set(\"limit\", _sce_saved.saved_write_uint32(500))\n".to_string(),
        execute(
            "test_case",
            "10000000",
            &selects(&enabled("session-unicast")),
        ),
    ] {
        assert!(
            suite.contains(&needle),
            "the example suite holds `{needle}`:\n{suite}"
        );
    }
    let case = file_at(&examples.join("test-case.scxml"), "test_case");
    assert!(
        case.contains("        configuration=sce_test_configuration.configuration(),\n"),
        "{case}"
    );
    assert!(case.contains("\ntest_case.__test__ = False"), "{case}");
    let configuration = file_at(
        &examples.join("test-configuration.scxml"),
        "test_configuration",
    );
    assert!(
        configuration.ends_with("    return [\"limit\"]"),
        "{configuration}"
    );
    let component = machine_files(&examples.join("test-component.scxml"));
    let component = &component[0].1;
    assert!(
        component.contains(
            "if not context.read_module_parameters(engine.policy._sce_accept_module_parameters):"
        ),
        "{component}"
    );
    assert!(component.contains("given.of(\"limit\")"), "{component}");
}
