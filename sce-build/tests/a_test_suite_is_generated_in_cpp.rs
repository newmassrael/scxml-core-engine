// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in C++ (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 5): a test configuration's header declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's header its
//! entry of the runner, through its configuration's unit; a suite's header its
//! module parameters at their defaults, as saved values of their types, and its
//! test cases in run order, each with its guard and its selection. A port type
//! writes no file.
//!
//! What the generated tables do when they run is held by
//! `tests/integration/ATestSuiteRunsEndToEndTest.cpp`, which compiles them from
//! the same fixtures and runs the suite through `SCE::Test::runSuite`.

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

fn options() -> ForgeCompileOptions {
    ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    }
}

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_forge_file(path, Language::Cpp, &[], &options()).map(|output| output.files)
}

/// The one header the runtime fixture `name` generates on C++, `<name>.h`.
fn header(name: &str) -> String {
    let files = generate(&runtime_dir().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on C++: {e:?}"));
    let names: Vec<&str> = files.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(names, [format!("{name}.h").as_str()], "{name}");
    files[0].1.clone()
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// §11.6, §12.2: a configuration is the table the runtime reads, its
/// components' machines named by their documents, and the module parameters
/// those components read.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = header("asking_configuration");
    for needle in [
        "namespace SCE::Generated::AskingConfiguration {",
        "#include \"test/TestCaseRun.h\"",
        "#include \"param_asker_sm.h\"",
        "configuration.mtcType = \"param_asker\";",
        "configuration.portsOfType.emplace(\"param_asker\", ::std::vector<::std::string>{\"sut\"});",
        "configuration.timersOfType.emplace(\"param_asker\", ::std::vector<::std::string>{\"reply\"});",
        "configuration.mappings.push_back({::SCE::Test::Endpoint{\"mtc\", \"sut\"}, \"sut\"});",
        "if (componentType == \"param_asker\") {",
        "return ::std::make_unique<::SCE::Generated::param_asker::param_askerComponent>();",
        "return nullptr;",
        "return {\"job\"};",
    ] {
        assert!(asking.contains(needle), "asking_configuration holds `{needle}`");
    }
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is included"
    );

    let serving = header("serving_configuration");
    for needle in [
        "#include \"coordinator_sm.h\"",
        "#include \"answerer_sm.h\"",
        "configuration.mtcType = \"coordinator\";",
        "configuration.ptcTypes.emplace(\"answerer\", \"answerer\");",
        "configuration.connections.push_back({::SCE::Test::Endpoint{\"mtc\", \"peer\"}, \
         ::SCE::Test::Endpoint{\"answerer\", \"peer\"}});",
        "return {};",
    ] {
        assert!(
            serving.contains(needle),
            "serving_configuration holds `{needle}`"
        );
    }
    assert!(
        at(&serving, "componentType == \"coordinator\"")
            < at(&serving, "componentType == \"answerer\""),
        "the MTC's type comes first"
    );

    let keeping = header("keeping_configuration");
    for needle in [
        "configuration.timersOfType.emplace(\"keeper\", ::std::vector<::std::string>{\"patience\", \
         \"linger\"});",
        "configuration.portsOfType.emplace(\"idler\", ::std::vector<::std::string>{\"peer\"});",
        "configuration.mappings.push_back({::SCE::Test::Endpoint{\"idler\", \"peer\"}, \"quiet\"});",
    ] {
        assert!(keeping.contains(needle), "keeping_configuration holds `{needle}`");
    }
    assert!(
        !keeping.contains("configuration.portsOfType.emplace(\"keeper\""),
        "a type that declares no port has no entry"
    );
}

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the configuration's unit.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = header("answers_the_job");
    for needle in [
        "namespace SCE::Generated::AnswersTheJob {",
        "#include \"test/Suite.h\"",
        "#include \"asking_configuration.h\"",
        "inline constexpr const char *NAME = \"answers_the_job\";",
        "::SCE::Generated::AskingConfiguration::configuration()",
        "&::SCE::Generated::AskingConfiguration::machine",
        "::SCE::Generated::AskingConfiguration::moduleParameters()",
    ] {
        assert!(case.contains(needle), "answers_the_job holds `{needle}`");
    }
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard and its selection over the
/// features; the feature model writes no unit, so none is included.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = header("runtime_suite");
    for needle in [
        "namespace SCE::Generated::RuntimeSuite {",
        "#include \"saved/SavedValues.h\"",
        "#include \"test/Suite.h\"",
        "suite.name = \"runtime_suite\";",
        "suite.moduleParameters.set(\"job\", ::SCE::Saved::savedWriteUint32(uint32_t{7}));",
    ] {
        assert!(suite.contains(needle), "runtime_suite holds `{needle}`");
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    let executes = [
        "::SCE::Test::Execute{::SCE::Generated::AnswersTheJob::testCase(), \
         ::SCE::Test::Duration{10000000}, nullptr}",
        "::SCE::Test::Execute{::SCE::Generated::ServesAPtc::testCase(), ::std::nullopt, \
         [](const ::SCE::Test::Features &world) { return ::SCE::Test::enabled(world, \
         \"ptcs\"); }}",
        "::SCE::Test::Execute{::SCE::Generated::KeepsAPtc::testCase(), ::std::nullopt, \
         [](const ::SCE::Test::Features &world) { return (::SCE::Test::enabled(world, \
         \"ptcs\") && ::SCE::Test::enabled(world, \"removal\")); }}",
        "::SCE::Test::Execute{::SCE::Generated::AnswersBeforeTheGuard::testCase(), \
         ::SCE::Test::Duration{100}, [](const ::SCE::Test::Features &world) { return \
         (::SCE::Test::enabled(world, \"guard\") || ::SCE::Test::enabled(world, \
         \"removal\")); }}",
    ];
    let mut last = 0;
    for execute in executes {
        let found = at(&suite, execute);
        assert!(found > last, "the test cases are in run order: `{execute}`");
        last = found;
    }
    for include in [
        "answers_the_job.h",
        "serves_a_ptc.h",
        "keeps_a_ptc.h",
        "answers_before_the_guard.h",
    ] {
        assert!(
            suite.contains(&format!("#include \"{include}\"")),
            "{include}"
        );
    }
}

/// §11.8, §12.2: each module parameter's default is written by the writer of
/// its type, the one a machine's saved state writes a value of that type with,
/// and a value of an enum as the name its document declares for the variant.
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
    let files = generate(&path).unwrap_or_else(|e| panic!("the suite generates on C++: {e:?}"));
    let text = &files[0].1;
    for needle in [
        "suite.moduleParameters.set(\"armed\", ::SCE::Saved::savedWriteBool(true));",
        "suite.moduleParameters.set(\"offset\", ::SCE::Saved::savedWriteInt8(int8_t{-5}));",
        "suite.moduleParameters.set(\"widest\", ::SCE::Saved::savedWriteUint64(uint64_t{",
        "suite.moduleParameters.set(\"label\", ::SCE::Saved::savedWriteString(\"nightly\"));",
        "suite.moduleParameters.set(\"key\", \
         ::SCE::Saved::savedWriteBytes(::std::vector<::std::uint8_t>{0x0a, 0xff}));",
        "suite.moduleParameters.set(\"color\", ::SCE::Saved::JsonValue(\"green\"));",
        "::SCE::Test::Execute{::SCE::Generated::AnswersTheJob::testCase(), ::std::nullopt, \
         nullptr}",
    ] {
        assert!(
            text.contains(needle),
            "typed_defaults holds `{needle}`:\n{text}"
        );
    }
    assert!(
        !text.contains("paint_color.h"),
        "an enum's default is its variant's name, so its unit is not included"
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
        panic!("a guard past the runtime's clock generated on C++");
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

/// §11.3, §11.9: a port type writes no file on C++: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on C++: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on C++, the suite's selection over the
/// example feature model.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
    let suite = generate(&examples.join("test-suite.scxml"))
        .unwrap_or_else(|e| panic!("the example suite generates on C++: {e:?}"));
    assert_eq!(suite[0].0, "test_suite.h");
    for needle in [
        "suite.moduleParameters.set(\"limit\", ::SCE::Saved::savedWriteUint32(uint32_t{500}));",
        "::SCE::Test::Execute{::SCE::Generated::TestCase::testCase(), \
         ::SCE::Test::Duration{10000000}, [](const ::SCE::Test::Features &world) { return \
         ::SCE::Test::enabled(world, \"session-unicast\"); }}",
    ] {
        assert!(
            suite[0].1.contains(needle),
            "the example suite holds `{needle}`"
        );
    }
    let case = generate(&examples.join("test-case.scxml"))
        .unwrap_or_else(|e| panic!("the example case generates on C++: {e:?}"));
    assert!(case[0]
        .1
        .contains("::SCE::Generated::TestConfiguration::configuration()"));
    let configuration = generate(&examples.join("test-configuration.scxml"))
        .unwrap_or_else(|e| panic!("the example configuration generates on C++: {e:?}"));
    assert!(configuration[0].1.contains("return {\"limit\"};"));
}
