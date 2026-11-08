// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Go (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its entry
//! of the runner, through its configuration's unit; a suite's unit its module
//! parameters at their defaults, as saved values of their types, and its test
//! cases in run order, each with its guard and its selection. A port type
//! writes no file. The twin of `a_test_suite_is_generated_in_kotlin.rs`, over
//! the same fixtures.
//!
//! A Go unit names another package only by an import path, so every unit a
//! table reads is a sibling package below the module path the run is given
//! (`--go-module-prefix`): a suite its test cases', a test case its
//! configuration's, and a configuration each component's machine, which is in
//! the package its statechart declares, `<module path>/<snake of the machine's
//! name>`. A component's machine is generated here through `sce-codegen
//! generate`, the one entry that takes the module path for a statechart.
//!
//! What the generated tables do when they run is held by
//! `backends/go/tests/integration/static_datamodel/a_test_suite_runs_through_its_runner_test.go`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_go.sh`) and runs the suite through
//! `testprofile.RunSuite`.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::generator::Language;
use sce_build::ForgeCompileOptions;

/// The module path the generated packages are imported under.
const PREFIX: &str = "example.com/generated";

/// The import spec of the profile's runtime, which every table names.
const RUNTIME_IMPORT: &str = "\t\"github.com/newmassrael/sce-go-runtime/testprofile\"\n";

/// The import spec of the SCE runtime, whose writers a suite's defaults are
/// written by.
const SAVED_VALUES_IMPORT: &str = "\tsce \"github.com/newmassrael/sce-go-runtime\"\n";

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

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

/// The options of a run whose module path is `prefix`.
fn under(prefix: Option<&str>) -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: prefix.map(str::to_string),
        ..Default::default()
    }
}

fn generate_under(
    path: &Path,
    prefix: Option<&str>,
) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Go, &[], &under(prefix))
        .map(|output| output.files)
}

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    generate_under(path, Some(PREFIX))
}

/// The one file the document at `path` generates on Go, `<file>.go`.
fn file_at(path: &Path, file: &str) -> String {
    let files = generate(path).unwrap_or_else(|e| panic!("{file} generates on Go: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, [format!("{file}.go").as_str()], "{file}");
    files[0].1.clone()
}

/// The one file the runtime fixture `name` generates on Go, `<name>.go`.
fn unit(name: &str) -> String {
    file_at(&runtime_dir().join(format!("{name}.scxml")), name)
}

/// Each Go file `sce-codegen generate -l go --go-module-prefix` [`PREFIX`]
/// wrote for the statechart at `path`, which generates, by name.
fn machine_files(path: &Path) -> Vec<(String, String)> {
    let out = tempfile::tempdir().expect("a directory");
    let run = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "go", "--go-module-prefix", PREFIX])
        .arg(path)
        .arg("-o")
        .arg(out.path())
        .output()
        .expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "{} generates on Go:\n{}{}",
        path.display(),
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    let mut files = Vec::new();
    for entry in std::fs::read_dir(out.path()).expect("the output directory") {
        let file = entry.expect("an entry").path();
        if file.extension().is_some_and(|e| e == "go") {
            let name = file
                .file_name()
                .and_then(|name| name.to_str())
                .expect("a UTF-8 file name")
                .to_string();
            let code = std::fs::read_to_string(&file).expect("a written file");
            files.push((name, code));
        }
    }
    files.sort();
    files
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

/// The import spec of the sibling package of the document `snake` below
/// [`PREFIX`], under the alias every Go unit imports a sibling by.
fn imported(snake: &str) -> String {
    format!("\tsce_{snake} \"{PREFIX}/{snake}\"\n")
}

/// §11.6, §12.2: a configuration is the table the runtime reads, its
/// components' machines named through their packages, and the module
/// parameters those components read; every table is written, an empty one as
/// empty.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = unit("asking_configuration");
    for needle in [
        "package asking_configuration\n".to_string(),
        RUNTIME_IMPORT.to_string(),
        imported("param_asker"),
        "func Configuration() testprofile.Configuration {".to_string(),
        "\t\tMTCType: \"param_asker\",\n".to_string(),
        "\t\tPTCTypes: map[string]string{},\n".to_string(),
        "\t\tConnections: []testprofile.Connection{},\n".to_string(),
        "\t\t\t{Port: testprofile.Endpoint{Component: \"mtc\", Port: \"sut\"}, SystemPort: \
         \"sut\"},\n"
            .to_string(),
        "func Machine(componentType string) testprofile.ComponentMachine {".to_string(),
        "\tcase \"param_asker\":\n\t\treturn &sce_param_asker.ParamAskerComponent{}\n".to_string(),
        "\treturn nil\n".to_string(),
        "func ModuleParameters() []string {\n\treturn []string{\"job\"}\n}".to_string(),
    ] {
        assert!(
            asking.contains(&needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    let ports = between(&asking, "PortsOfType:", "TimersOfType:");
    assert!(
        ports.contains("\t\t\t\"param_asker\": {\"sut\"},\n"),
        "{ports}"
    );
    let timers = between(&asking, "TimersOfType:", "Connections:");
    assert!(
        timers.contains("\t\t\t\"param_asker\": {\"reply\"},\n"),
        "{timers}"
    );
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is named or imported"
    );

    let serving = unit("serving_configuration");
    for needle in [
        "\t\tMTCType: \"coordinator\",\n".to_string(),
        "\t\t\t\"answerer\": \"answerer\",\n".to_string(),
        "\t\t\t{A: testprofile.Endpoint{Component: \"mtc\", Port: \"peer\"}, B: \
         testprofile.Endpoint{Component: \"answerer\", Port: \"peer\"}},\n"
            .to_string(),
        "\t\tMappings: []testprofile.Mapping{},\n".to_string(),
        "\t\tTimersOfType: map[string][]string{},\n".to_string(),
        "\t\treturn &sce_answerer.AnswererComponent{}\n".to_string(),
        "\treturn []string{}\n".to_string(),
        imported("coordinator"),
        imported("answerer"),
    ] {
        assert!(
            serving.contains(&needle),
            "serving_configuration holds `{needle}`:\n{serving}"
        );
    }
    assert!(
        at(&serving, "\tcase \"coordinator\":") < at(&serving, "\tcase \"answerer\":"),
        "the MTC's type comes first"
    );

    let keeping = unit("keeping_configuration");
    for needle in [
        "\t\t\t\"keeper\": {\"patience\", \"linger\"},\n",
        "\t\t\t\"idler\": {\"peer\"},\n",
        "\t\t\t{Port: testprofile.Endpoint{Component: \"idler\", Port: \"peer\"}, SystemPort: \
         \"quiet\"},\n",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`:\n{keeping}"
        );
    }
    let ports = between(&keeping, "PortsOfType:", "TimersOfType:");
    assert!(
        !ports.contains("\"keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// §12.3, §12.6: a configuration imports a component's machine from the
/// directory of the snake of its machine's name below the run's module path,
/// and names the component the statechart declares in the package that
/// directory holds, `package <snake>`; a trailing `/` of the module path names
/// the same path, and a configuration given no module path is refused by name,
/// as every Go unit that imports another is.
#[test]
fn a_configuration_names_each_component_where_its_statechart_declares_it() {
    let asking = unit("asking_configuration");
    let machine = machine_files(&runtime_dir().join("param_asker.scxml"));
    let names: Vec<&str> = machine.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["param_asker_sm.go"]);
    let machine = &machine[0].1;
    assert!(machine.contains("\npackage param_asker\n"), "{machine}");
    assert!(
        machine.contains("type ParamAskerComponent struct {"),
        "{machine}"
    );
    assert!(asking.contains(&imported("param_asker")), "{asking}");
    assert!(
        asking.contains("return &sce_param_asker.ParamAskerComponent{}"),
        "{asking}"
    );

    let path = runtime_dir().join("asking_configuration.scxml");
    let slashed = generate_under(&path, Some("example.com/generated/"))
        .unwrap_or_else(|e| panic!("a module path with a trailing slash: {e:?}"));
    assert!(
        slashed[0].1.contains(&imported("param_asker")),
        "{}",
        slashed[0].1
    );

    let Err(err) = generate_under(&path, None) else {
        panic!("a configuration generated on Go with no module path");
    };
    let refusal = format!("{err:?}");
    assert!(
        refusal.contains("requires ForgeCompileOptions.go_module_prefix"),
        "{refusal}"
    );
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the package of the configuration's unit.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = unit("answers_the_job");
    for needle in [
        "package answers_the_job\n".to_string(),
        RUNTIME_IMPORT.to_string(),
        imported("asking_configuration"),
        "const Name string = \"answers_the_job\"\n".to_string(),
        "func TestCase() testprofile.TestCase {".to_string(),
        "\t\tName:             Name,\n".to_string(),
        "\t\tConfiguration:    sce_asking_configuration.Configuration(),\n".to_string(),
        "\t\tMachines:         sce_asking_configuration.Machine,\n".to_string(),
        "\t\tModuleParameters: sce_asking_configuration.ModuleParameters(),\n".to_string(),
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
    format!(
        "\t\t\t\tTestCase:  sce_{test_case}.TestCase(),\n\t\t\t\tGuard:     {guard},\n\
         \t\t\t\tSelection: {selection},\n"
    )
}

/// The guard of `us` microseconds.
fn guard(us: u64) -> String {
    format!("testprofile.GuardOf({us})")
}

/// The guard of a test case that has none.
const NO_GUARD: &str = "testprofile.NoGuard";

/// The selection that holds in a world that enables what `body` tests.
fn selects(body: &str) -> String {
    format!("func(world testprofile.Features) bool {{ return {body} }}")
}

/// The test of a world that enables `feature`.
fn enabled(feature: &str) -> String {
    format!("testprofile.Enabled(world, \"{feature}\")")
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard and its selection over the
/// features, the outermost join of a selection bare as a function's returned
/// expression is; the feature model writes no unit, so none is imported.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = unit("runtime_suite");
    for needle in [
        "package runtime_suite\n".to_string(),
        RUNTIME_IMPORT.to_string(),
        SAVED_VALUES_IMPORT.to_string(),
        imported("answers_the_job"),
        imported("serves_a_ptc"),
        imported("keeps_a_ptc"),
        imported("answers_before_the_guard"),
        "func Suite() testprofile.Suite {".to_string(),
        "\tmoduleParameters := &testprofile.ModuleParameters{}\n".to_string(),
        "\tmoduleParameters.Set(\"job\", sce.SavedWriteUint32(7))\n".to_string(),
        "\t\tName:             \"runtime_suite\",\n".to_string(),
        "\t\tModuleParameters: moduleParameters,\n".to_string(),
    ] {
        assert!(
            suite.contains(&needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    let executes = [
        execute("answers_the_job", &guard(10_000_000), "nil"),
        execute("serves_a_ptc", NO_GUARD, &selects(&enabled("ptcs"))),
        execute(
            "keeps_a_ptc",
            NO_GUARD,
            &selects(&format!("{} && {}", enabled("ptcs"), enabled("removal"))),
        ),
        execute(
            "answers_before_the_guard",
            &guard(100),
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

/// A suite whose selection joins a join keeps the inner one in parentheses;
/// one that runs a test case twice imports its package once, as Go refuses a
/// second import of one name; and one that declares no module parameter and
/// runs no test case holds the empty tables and imports neither the SCE
/// runtime nor a test case.
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
        "{} && ({} || {})",
        enabled("ptcs"),
        enabled("guard"),
        enabled("removal")
    );
    assert!(
        text.contains(&execute("serves_a_ptc", NO_GUARD, &selects(&body))),
        "{text}"
    );
    assert!(
        text.contains("\tmoduleParameters := &testprofile.ModuleParameters{}\n"),
        "{text}"
    );
    assert!(
        !text.contains("moduleParameters.Set("),
        "a suite that declares no module parameter gives none: {text}"
    );
    assert!(
        !text.contains(SAVED_VALUES_IMPORT),
        "a suite that writes no default names no writer, so Go refuses its import: {text}"
    );

    // An alias is executed once (a second `<sce:execute>` of it is refused as
    // a duplicate), but one document may be imported under two aliases: both
    // rows then name one Go package, which Go accepts only when imported once.
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
        at(&text, &execute("answers_the_job", NO_GUARD, "nil"))
            < at(&text, &execute("answers_the_job", &guard(1_000_000), "nil")),
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
    assert!(
        text.contains("\t\tExecutes: []testprofile.Execute{},\n"),
        "{text}"
    );
    let imports = between(&text, "import (", ")");
    assert_eq!(imports, format!("import (\n{RUNTIME_IMPORT}"), "{text}");
}

/// §11.8, §12.2: each module parameter's default is the saved value of its
/// type, the one a machine's saved state writes a value of that type as: a
/// number, a `bool`, a text and a byte string by the writer of the type, an
/// integer as an untyped constant the writer's parameter types, and a value of
/// an enum as the name its document declares for the variant, written as a
/// text is.
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
        "\tmoduleParameters.Set(\"job\", sce.SavedWriteUint32(7))\n".to_string(),
        "\tmoduleParameters.Set(\"armed\", sce.SavedWriteBool(true))\n".to_string(),
        "\tmoduleParameters.Set(\"offset\", sce.SavedWriteInt8(-5))\n".to_string(),
        "\tmoduleParameters.Set(\"widest\", sce.SavedWriteUint64(18446744073709551615))\n"
            .to_string(),
        "\tmoduleParameters.Set(\"label\", sce.SavedWriteString(\"nightly\"))\n".to_string(),
        "\tmoduleParameters.Set(\"key\", sce.SavedWriteBytes([]byte{0x0a, 0xff}))\n".to_string(),
        "\tmoduleParameters.Set(\"color\", sce.SavedWriteString(\"green\"))\n".to_string(),
        SAVED_VALUES_IMPORT.to_string(),
        execute("answers_the_job", NO_GUARD, "nil"),
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
        panic!("a guard past the runtime's clock generated on Go");
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

/// §11.3, §11.9: a port type writes no file on Go: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on Go: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on Go, the suite's selection over the
/// example feature model, and the example component, given the module path
/// its template's package is imported under, reads the module parameter its
/// configuration names.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = examples_dir();
    let suite = file_at(&examples.join("test-suite.scxml"), "test_suite");
    for needle in [
        "\tmoduleParameters.Set(\"limit\", sce.SavedWriteUint32(500))\n".to_string(),
        execute(
            "test_case",
            &guard(10_000_000),
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
        case.contains("\t\tConfiguration:    sce_test_configuration.Configuration(),\n"),
        "{case}"
    );
    let configuration = file_at(
        &examples.join("test-configuration.scxml"),
        "test_configuration",
    );
    assert!(
        configuration.contains("\treturn []string{\"limit\"}\n"),
        "{configuration}"
    );
    let component = machine_files(&examples.join("test-component.scxml"));
    let component = &component[0].1;
    assert!(
        component.contains("if !context.ReadModuleParameters(policy.sceAcceptModuleParameters) {"),
        "{component}"
    );
    assert!(component.contains("given.Of(\"limit\")"), "{component}");
}
