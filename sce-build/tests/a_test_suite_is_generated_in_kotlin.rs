// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Kotlin (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its entry
//! of the runner, through its configuration's unit; a suite's unit its module
//! parameters at their defaults, as saved values of their types, and its test
//! cases in run order, each with its guard and its selection. A port type
//! writes no file. The twin of `a_test_suite_is_generated_in_rust.rs`, over the
//! same fixtures.
//!
//! What the generated tables do when they run is held by
//! `backends/kotlin/tests/src/test/kotlin/com/sce/integration/ATestSuiteRunsThroughItsRunnerTest.kt`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_kotlin.sh`) and runs the suite through
//! `com.sce.runtime.test.runSuite`.

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
    sce_build::compile_forge_file(path, Language::Kotlin, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
}

/// The one file the document at `path` generates on Kotlin, `<file>.kt`.
fn file_at(path: &Path, file: &str) -> String {
    let files = generate(path).unwrap_or_else(|e| panic!("{file} generates on Kotlin: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, [format!("{file}.kt").as_str()], "{file}");
    files[0].1.clone()
}

/// The one file the runtime fixture `name` generates on Kotlin, `<file>.kt`.
fn unit(name: &str, file: &str) -> String {
    file_at(&runtime_dir().join(format!("{name}.scxml")), file)
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
/// components' machines named by the package their statecharts are generated
/// into, and the module parameters those components read.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = unit("asking_configuration", "AskingConfiguration");
    for needle in [
        "package com.sce.generated.asking_configuration",
        "fun configuration(): com.sce.runtime.test.Configuration = \
         com.sce.runtime.test.Configuration(",
        "mtcType = \"param_asker\",",
        "ptcTypes = emptyMap(),",
        "connections = emptyList(),",
        "com.sce.runtime.test.Endpoint(\"mtc\", \"sut\") to \"sut\",",
        "fun machine(componentType: String): com.sce.runtime.test.ComponentMachine? = \
         when (componentType) {",
        "\"param_asker\" -> com.sce.generated.param_asker.ParamAskerComponent()",
        "else -> null",
        "fun moduleParameters(): List<String> = listOf(\"job\")",
    ] {
        assert!(
            asking.contains(needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    let ports = between(&asking, "portsOfType =", "timersOfType =");
    assert!(
        ports.contains("\"param_asker\" to listOf(\"sut\"),"),
        "{ports}"
    );
    let timers = between(&asking, "timersOfType =", "connections =");
    assert!(
        timers.contains("\"param_asker\" to listOf(\"reply\"),"),
        "{timers}"
    );
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is named"
    );
    assert!(
        !asking.contains("\nimport "),
        "every unit is named by its package"
    );

    let serving = unit("serving_configuration", "ServingConfiguration");
    for needle in [
        "mtcType = \"coordinator\",",
        "\"answerer\" to \"answerer\",",
        "com.sce.runtime.test.Endpoint(\"mtc\", \"peer\") to \
         com.sce.runtime.test.Endpoint(\"answerer\", \"peer\"),",
        "mappings = emptyList(),",
        "timersOfType = emptyMap(),",
        "\"answerer\" -> com.sce.generated.answerer.AnswererComponent()",
        "fun moduleParameters(): List<String> = emptyList()",
    ] {
        assert!(
            serving.contains(needle),
            "serving_configuration holds `{needle}`:\n{serving}"
        );
    }
    assert!(
        at(
            &serving,
            "\"coordinator\" -> com.sce.generated.coordinator.CoordinatorComponent()"
        ) < at(
            &serving,
            "\"answerer\" -> com.sce.generated.answerer.AnswererComponent()"
        ),
        "the MTC's type comes first"
    );

    let keeping = unit("keeping_configuration", "KeepingConfiguration");
    for needle in [
        "\"ptc_keeper\" to listOf(\"patience\", \"linger\"),",
        "\"idler\" to listOf(\"peer\"),",
        "com.sce.runtime.test.Endpoint(\"idler\", \"peer\") to \"quiet\",",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`:\n{keeping}"
        );
    }
    let ports = between(&keeping, "portsOfType =", "timersOfType =");
    assert!(
        !ports.contains("\"ptc_keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// §12.3, §12.6: the package a configuration names a component's class by is
/// the one the component's statechart is generated into when no package
/// prefix is given, and the class is the component the statechart declares.
#[test]
fn a_configuration_names_each_component_where_its_statechart_declares_it() {
    let asking = unit("asking_configuration", "AskingConfiguration");
    let component = runtime_dir().join("param_asker.scxml");
    let machine = sce_build::compile_scxml_lang_typed(
        component.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Kotlin),
        Language::Kotlin,
    )
    .unwrap_or_else(|e| panic!("param_asker generates on Kotlin: {e:?}"))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n");
    assert!(
        machine.contains("package com.sce.generated.param_asker\n"),
        "{machine}"
    );
    assert!(
        machine.contains("class ParamAskerComponent : com.sce.runtime.test.ComponentMachine {"),
        "{machine}"
    );
    assert!(asking.contains("com.sce.generated.param_asker.ParamAskerComponent()"));
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the configuration's unit by its package.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = unit("answers_the_job", "AnswersTheJob");
    for needle in [
        "package com.sce.generated.answers_the_job",
        "const val NAME: String = \"answers_the_job\"",
        "fun testCase(): com.sce.runtime.test.TestCase = com.sce.runtime.test.TestCase(",
        "name = NAME,",
        "configuration = com.sce.generated.asking_configuration.configuration(),",
        "machines = { componentType -> \
         com.sce.generated.asking_configuration.machine(componentType) },",
        "moduleParameters = com.sce.generated.asking_configuration.moduleParameters(),",
    ] {
        assert!(
            case.contains(needle),
            "answers_the_job holds `{needle}`:\n{case}"
        );
    }
    assert!(!case.contains("\nimport "), "{case}");
}

/// The text of one `<sce:execute>` of a suite's unit: its test case, its
/// guard and its selection.
fn execute(test_case: &str, guard: &str, selection: &str) -> String {
    format!(
        "testCase = com.sce.generated.{test_case}.testCase(),\n                guard = {guard},\n                \
         selection = {selection},"
    )
}

/// The selection that holds in a world that enables what `body` tests.
fn selects(body: &str) -> String {
    format!("{{ world -> {body} }}")
}

/// The test of a world that enables `feature`.
fn enabled(feature: &str) -> String {
    format!("com.sce.runtime.test.enabled(world, \"{feature}\")")
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard and its selection over the
/// features, the outermost join of a selection bare as a lambda's body is; the
/// feature model writes no unit, so none is named.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = unit("runtime_suite", "RuntimeSuite");
    for needle in [
        "package com.sce.generated.runtime_suite",
        "fun suite(): com.sce.runtime.test.Suite {",
        "val moduleParameters = com.sce.runtime.test.ModuleParameters()",
        "moduleParameters.set(\"job\", com.sce.runtime.SavedValues.of(7u))",
        "name = \"runtime_suite\",",
        "moduleParameters = moduleParameters,",
    ] {
        assert!(
            suite.contains(needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    assert!(!suite.contains("\nimport "), "{suite}");
    let executes = [
        execute("answers_the_job", "10000000L", "null"),
        execute("serves_a_ptc", "null", &selects(&enabled("ptcs"))),
        execute(
            "keeps_a_ptc",
            "null",
            &selects(&format!("{} && {}", enabled("ptcs"), enabled("removal"))),
        ),
        execute(
            "answers_before_the_guard",
            "100L",
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
    let text = file_at(&path, "Nested");
    let body = format!(
        "{} && ({} || {})",
        enabled("ptcs"),
        enabled("guard"),
        enabled("removal")
    );
    assert!(
        text.contains(&execute("serves_a_ptc", "null", &selects(&body))),
        "{text}"
    );
    assert!(
        text.contains("val moduleParameters = com.sce.runtime.test.ModuleParameters()"),
        "{text}"
    );
    assert!(
        !text.contains("moduleParameters.set("),
        "a suite that declares no module parameter gives none: {text}"
    );

    let empty = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="empty_suite">
</scxml>
"#;
    let path = dir.path().join("empty_suite.scxml");
    std::fs::write(&path, empty).expect("the suite is written");
    let text = file_at(&path, "EmptySuite");
    assert!(text.contains("executes = emptyList(),"), "{text}");
}

/// §11.8, §12.2: each module parameter's default is the saved value of its
/// type, the one a machine's saved state writes a value of that type as: a
/// number, a `bool`, a text and a byte string by `SavedValues.of` of a literal
/// of the type, and a value of an enum as the name its document declares for
/// the variant.
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
        "moduleParameters.set(\"job\", com.sce.runtime.SavedValues.of(7u))",
        "moduleParameters.set(\"armed\", com.sce.runtime.SavedValues.of(true))",
        "moduleParameters.set(\"offset\", com.sce.runtime.SavedValues.of((-5).toByte()))",
        "moduleParameters.set(\"widest\", \
         com.sce.runtime.SavedValues.of(18446744073709551615uL))",
        "moduleParameters.set(\"label\", com.sce.runtime.SavedValues.of(\"nightly\"))",
        "moduleParameters.set(\"key\", \
         com.sce.runtime.SavedValues.of(byteArrayOf(0x0a.toByte(), 0xff.toByte())))",
        "moduleParameters.set(\"color\", \"green\")",
        &execute("answers_the_job", "null", "null"),
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
        panic!("a guard past the runtime's clock generated on Kotlin");
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

/// §11.3, §11.9: a port type writes no file on Kotlin: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on Kotlin: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on Kotlin, the suite's selection over
/// the example feature model.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
    let suite = file_at(&examples.join("test-suite.scxml"), "TestSuite");
    for needle in [
        "moduleParameters.set(\"limit\", com.sce.runtime.SavedValues.of(500u))".to_string(),
        execute(
            "test_case",
            "10000000L",
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
        case.contains("configuration = com.sce.generated.test_configuration.configuration(),"),
        "{case}"
    );
    let configuration = file_at(
        &examples.join("test-configuration.scxml"),
        "TestConfiguration",
    );
    assert!(
        configuration.contains("listOf(\"limit\")"),
        "{configuration}"
    );
}
