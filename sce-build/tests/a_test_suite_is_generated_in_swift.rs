// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Swift (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its name
//! and its entry of the runner, through its configuration's unit; a suite's
//! unit its module parameters at their defaults, as saved values of their
//! types, and its test cases in run order, each with its guard and its
//! selection. Each declares its members in a caseless enum named by its
//! document, as every Swift unit does, and a port type writes no file. The
//! twin of `a_test_suite_is_generated_in_kotlin.rs`, over the same fixtures.
//!
//! What the generated tables do when they run is held by
//! `backends/swift/tests/Tests/StaticDatamodelTests/ATestSuiteRunsThroughItsRunnerTests.swift`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_swift.sh`) and runs the suite through
//! `SceTestProfile.runSuite`.

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

/// The directory of the examples of the kinds.
fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples")
}

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Swift, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
}

/// The one file the document at `path` generates on Swift, `<file>.swift`.
fn file_at(path: &Path, file: &str) -> String {
    let files = generate(path).unwrap_or_else(|e| panic!("{file} generates on Swift: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, [format!("{file}.swift").as_str()], "{file}");
    files[0].1.clone()
}

/// The one file the runtime fixture `name` generates on Swift, `<file>.swift`.
fn unit(name: &str, file: &str) -> String {
    file_at(&runtime_dir().join(format!("{name}.scxml")), file)
}

/// The class of the component the test component at `path` generates beside
/// its machine on Swift, as its file declares it.
fn component_class_of(path: &Path) -> String {
    let machine = sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Swift),
        Language::Swift,
    )
    .unwrap_or_else(|e| panic!("{} generates on Swift: {e:?}", path.display()))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n");
    // The policy is declared first, as a class too; the component is the class
    // that conforms to the runtime's machine.
    let end = at(&machine, ": SceTestProfile.ComponentMachine {");
    let opening = "public final class ";
    let start = machine[..end]
        .rfind(opening)
        .unwrap_or_else(|| panic!("the component is a final class:\n{machine}"))
        + opening.len();
    machine[start..end].to_string()
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

/// The lines of `code` that import a module, in order.
fn imports(code: &str) -> Vec<&str> {
    code.lines()
        .filter(|line| line.starts_with("import "))
        .collect()
}

/// §11.6, §12.2: a configuration is the table the runtime reads, its
/// components' classes named bare, as the files of the module declare them,
/// and the module parameters those components read; the runtime's types are
/// named by its module, the one module the unit imports.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = unit("asking_configuration", "AskingConfiguration");
    for needle in [
        "\npublic enum AskingConfiguration {\n",
        "    public static func configuration() -> SceTestProfile.Configuration {\n        \
         SceTestProfile.Configuration(\n",
        "            mtcType: \"param_asker\",\n",
        "            ptcTypes: [:],\n",
        "            connections: [],\n",
        "                SceTestProfile.Mapping(SceTestProfile.Endpoint(component: \"mtc\", port: \
         \"sut\"), systemPort: \"sut\"),\n            ])\n",
        "    public static func machine(_ componentType: String) -> \
         (any SceTestProfile.ComponentMachine)? {\n        switch componentType {\n",
        "        case \"param_asker\": return ParamAskerComponent()\n",
        "        default: return nil\n",
        "    public static func moduleParameters() -> [String] {\n        [\"job\"]\n    }\n",
    ] {
        assert!(
            asking.contains(needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    let ports = between(&asking, "portsOfType:", "timersOfType:");
    assert!(ports.contains("\"param_asker\": [\"sut\"],"), "{ports}");
    let timers = between(&asking, "timersOfType:", "connections:");
    assert!(timers.contains("\"param_asker\": [\"reply\"],"), "{timers}");
    assert!(
        !asking.contains("exchange_port") && !asking.contains("ExchangePort"),
        "a port type writes no unit, so none is named"
    );
    assert_eq!(imports(&asking), ["import SceTestProfile"], "{asking}");

    let serving = unit("serving_configuration", "ServingConfiguration");
    for needle in [
        "            mtcType: \"coordinator\",\n",
        "                \"answerer\": \"answerer\",\n",
        "                SceTestProfile.Connection(SceTestProfile.Endpoint(component: \"mtc\", \
         port: \"peer\"), SceTestProfile.Endpoint(component: \"answerer\", port: \"peer\")),\n",
        "            mappings: [])\n",
        "            timersOfType: [:],\n",
        "        case \"answerer\": return AnswererComponent()\n",
        "    public static func moduleParameters() -> [String] {\n        []\n    }\n",
    ] {
        assert!(
            serving.contains(needle),
            "serving_configuration holds `{needle}`:\n{serving}"
        );
    }
    assert!(
        at(
            &serving,
            "case \"coordinator\": return CoordinatorComponent()"
        ) < at(&serving, "case \"answerer\": return AnswererComponent()"),
        "the MTC's type comes first"
    );

    let keeping = unit("keeping_configuration", "KeepingConfiguration");
    for needle in [
        "                \"ptc_keeper\": [\"patience\", \"linger\"],\n",
        "                \"idler\": [\"peer\"],\n",
        "                SceTestProfile.Mapping(SceTestProfile.Endpoint(component: \"idler\", \
         port: \"peer\"), systemPort: \"quiet\"),\n",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`:\n{keeping}"
        );
    }
    let ports = between(&keeping, "portsOfType:", "timersOfType:");
    assert!(
        !ports.contains("\"ptc_keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// §12.3, §12.6: the class a configuration names a component by is the one
/// the component's machine file declares beside the machine, a file of the
/// same module.
#[test]
fn a_configuration_names_each_component_as_its_machine_declares_it() {
    let asking = unit("asking_configuration", "AskingConfiguration");
    let class = component_class_of(&runtime_dir().join("param_asker.scxml"));
    assert_eq!(class, "ParamAskerComponent");
    assert!(asking.contains(&format!("return {class}()")), "{asking}");
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the enum of the configuration's unit.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = unit("answers_the_job", "AnswersTheJob");
    for needle in [
        "\npublic enum AnswersTheJob {\n",
        "    public static let name: String = \"answers_the_job\"\n",
        "    public static func testCase() -> SceTestProfile.TestCase {\n        \
         SceTestProfile.TestCase(\n",
        "            name: Self.name,\n",
        "            configuration: AskingConfiguration.configuration(),\n",
        "            machines: { componentType in AskingConfiguration.machine(componentType) },\n",
        "            moduleParameters: AskingConfiguration.moduleParameters())\n",
    ] {
        assert!(
            case.contains(needle),
            "answers_the_job holds `{needle}`:\n{case}"
        );
    }
    assert_eq!(imports(&case), ["import SceTestProfile"], "{case}");
}

/// The text of one `<sce:execute>` of a suite's unit: the enum of its test
/// case's unit, its guard and its selection.
fn execute(test_case: &str, guard: &str, selection: &str) -> String {
    format!(
        "                SceTestProfile.Execute(\n                    {test_case}.testCase(),\n                    \
         guardAt: {guard},\n                    selection: {selection}),\n"
    )
}

/// The selection that holds in a world that enables what `body` tests.
fn selects(body: &str) -> String {
    format!("{{ world in {body} }}")
}

/// The test of a world that enables `feature`.
fn enabled(feature: &str) -> String {
    format!("SceTestProfile.enabled(world, \"{feature}\")")
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard in microseconds and its
/// selection over the features, the outermost join of a selection bare as a
/// closure's body is; the feature model writes no unit, so none is named. The
/// runtime, which writes each default, is imported beside the profile's.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = unit("runtime_suite", "RuntimeSuite");
    for needle in [
        "\npublic enum RuntimeSuite {\n",
        "    public static func suite() -> SceTestProfile.Suite {\n",
        "        var moduleParameters = SceTestProfile.ModuleParameters()\n",
        "        moduleParameters.set(\"job\", SceRuntime.SavedValues.of(7 as UInt32))\n",
        "        return SceTestProfile.Suite(\n            name: \"runtime_suite\",\n            \
         moduleParameters: moduleParameters,\n            executes: [\n",
    ] {
        assert!(
            suite.contains(needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
    assert!(
        !suite.contains("runtime_features") && !suite.contains("RuntimeFeatures"),
        "{suite}"
    );
    assert_eq!(
        imports(&suite),
        ["import SceRuntime", "import SceTestProfile"],
        "{suite}"
    );
    let executes = [
        execute("AnswersTheJob", "10000000", "nil"),
        execute("ServesAPtc", "nil", &selects(&enabled("ptcs"))),
        execute(
            "KeepsAPtc",
            "nil",
            &selects(&format!("{} && {}", enabled("ptcs"), enabled("removal"))),
        ),
        execute(
            "AnswersBeforeTheGuard",
            "100",
            &selects(&format!("{} || {}", enabled("guard"), enabled("removal"))),
        ),
    ];
    let mut last = 0;
    for execute in &executes {
        let found = at(&suite, execute);
        assert!(found > last, "the test cases are in run order: `{execute}`");
        last = found;
    }
    let closed = format!("{}            ])\n", executes[3]);
    assert!(
        suite[last..].starts_with(&closed),
        "the last entry closes the call, with no comma after its last argument:\n{suite}"
    );
}

/// A suite whose selection joins a join keeps the inner one in parentheses;
/// one that declares no module parameter holds an empty set of them it never
/// writes to and imports no runtime it does not name; and one that runs no
/// test case holds the empty table.
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
    let text = file_at(&path, "Nested");
    let body = format!(
        "{} && ({} || {})",
        enabled("ptcs"),
        enabled("guard"),
        enabled("removal")
    );
    assert!(
        text.contains(&execute("ServesAPtc", "nil", &selects(&body))),
        "{text}"
    );
    assert!(
        text.contains("        let moduleParameters = SceTestProfile.ModuleParameters()\n"),
        "a set no default is written to is a constant: {text}"
    );
    assert!(
        !text.contains("moduleParameters.set("),
        "a suite that declares no module parameter gives none: {text}"
    );
    assert_eq!(imports(&text), ["import SceTestProfile"], "{text}");

    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="empty_suite">
</scxml>
"#;
    let path = dir.path().join("empty_suite.scxml");
    std::fs::write(&path, empty).expect("the suite is written");
    let text = file_at(&path, "EmptySuite");
    assert!(
        text.contains(
            "            moduleParameters: moduleParameters,\n            executes: [])\n"
        ),
        "{text}"
    );
}

/// §11.8, §12.2: each module parameter's default is the saved value of its
/// type, the one a machine's saved state writes a value of that type as: a
/// number, a `bool`, a text and a byte string by `SavedValues.of` of a literal
/// of the type, a number's and a byte string's coerced to it so the overload of
/// that type writes it, and a value of an enum as the name its document
/// declares for the variant.
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
    let text = file_at(&path, "TypedDefaults");
    for needle in [
        "moduleParameters.set(\"job\", SceRuntime.SavedValues.of(7 as UInt32))\n",
        "moduleParameters.set(\"armed\", SceRuntime.SavedValues.of(true))\n",
        "moduleParameters.set(\"offset\", SceRuntime.SavedValues.of(-5 as Int8))\n",
        "moduleParameters.set(\"widest\", \
         SceRuntime.SavedValues.of(18446744073709551615 as UInt64))\n",
        "moduleParameters.set(\"label\", SceRuntime.SavedValues.of(\"nightly\"))\n",
        "moduleParameters.set(\"key\", SceRuntime.SavedValues.of([0x0a, 0xff] as [UInt8]))\n",
        "moduleParameters.set(\"color\", SceRuntime.SavedValues.of(\"green\"))\n",
        &execute("AnswersTheJob", "nil", "nil"),
    ] {
        assert!(
            text.contains(needle),
            "typed_defaults holds `{needle}`:\n{text}"
        );
    }
    assert!(
        !text.contains("paint_color") && !text.contains("PaintColor"),
        "an enum's default is its variant's name, so its unit is not named"
    );
    let set = between(
        &text,
        "moduleParameters.set(\"job\"",
        "return SceTestProfile.Suite(",
    );
    assert!(
        at(set, "\"job\"") < at(set, "\"armed\"") && at(set, "\"key\"") < at(set, "\"color\""),
        "the defaults are written in document order: {set}"
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
        panic!("a guard past the runtime's clock generated on Swift");
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

/// §11.3, §11.9: a port type writes no file on Swift: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on Swift: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on Swift, the suite's selection over the
/// example feature model, and the example configuration names the class the
/// example component's machine file declares.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = examples_dir();
    let suite = file_at(&examples.join("test-suite.scxml"), "TestSuite");
    for needle in [
        "moduleParameters.set(\"limit\", SceRuntime.SavedValues.of(500 as UInt32))\n".to_string(),
        execute(
            "TestCase",
            "10000000",
            &selects(&enabled("session-unicast")),
        ),
    ] {
        assert!(
            suite.contains(&needle),
            "the example suite holds `{needle}`:\n{suite}"
        );
    }
    let case = file_at(&examples.join("test-case.scxml"), "TestCase");
    assert!(
        case.contains("            configuration: TestConfiguration.configuration(),\n"),
        "{case}"
    );
    assert!(
        case.contains("    public static func testCase() -> SceTestProfile.TestCase {\n"),
        "the unit's own enum, `TestCase`, hides none of the runtime's types, which are \
         named by their module:\n{case}"
    );
    let configuration = file_at(
        &examples.join("test-configuration.scxml"),
        "TestConfiguration",
    );
    assert!(
        configuration.contains("        [\"limit\"]\n"),
        "{configuration}"
    );
    let class = component_class_of(&examples.join("test-component.scxml"));
    assert!(
        configuration.contains(&format!("return {class}()")),
        "`{class}`:\n{configuration}"
    );
}
