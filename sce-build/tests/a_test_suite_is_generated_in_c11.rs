// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in C11 (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's header declares the
//! configuration the profile's runtime reads as constant storage, the storage
//! and the factory of its components' machines and the module parameters they
//! read; a test case's header its entry of the runner, through its
//! configuration's unit; a suite's header the storage a caller places for it
//! and the suite in it, its module parameters at their defaults, as the JSON
//! text of the saved values of their types, and its test cases in run order,
//! each with its guard and its selection. A port type writes no file. The twin
//! of `a_test_suite_is_generated_in_cpp.rs`, over the same fixtures.
//!
//! What the generated tables do when they run is held by
//! `backends/c/tests/integration/test_a_test_suite_runs_through_its_runner.c`,
//! which compiles them from the same fixtures and runs the suite through
//! `sce_test_run_suite`.

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
    sce_build::compile_forge_file(path, Language::C11, &[], &ForgeCompileOptions::default())
        .map(|output| output.files)
}

/// The one header the runtime fixture `name` generates on C11, `<name>.h`.
fn header(name: &str) -> String {
    let files = generate(&runtime_dir().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on C11: {e:?}"));
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

/// §11.6, §12.2: a configuration is the table the runtime reads, constant
/// storage its function lends; the storage of its components' machines, one
/// component of each type for each component of that type, and the factory
/// that makes a machine from the first one no run holds; and the module
/// parameters those components read.
#[test]
fn a_configuration_is_its_table_its_machines_and_what_they_read() {
    let asking = header("asking_configuration");
    for needle in [
        "\n#include \"sce/test/test_case_run.h\"\n",
        "\n#include \"param_asker_sm.h\"\n",
        "\nstatic inline const sce_test_configuration_t *asking_configuration_configuration(void) {\n",
        "\n    static const sce_test_declared_t ports[] = {{\"param_asker\", \"sut\"}};\n",
        "\n    static const sce_test_declared_t timers[] = {{\"param_asker\", \"deadline\"}};\n",
        "\n    static const sce_test_mapping_t mappings[] = {{{\"mtc\", \"sut\"}, \"sut\"}};\n",
        "\n    static const sce_test_configuration_t configuration = {\n        \"param_asker\",\n        \
         NULL, 0u,\n        ports, 1u,\n        timers, 1u,\n        NULL, 0u,\n        \
         mappings, 1u,\n    };\n    return &configuration;\n}\n",
        "\ntypedef struct asking_configuration_machines_s {\n    param_asker_component_t \
         param_asker_components[1];\n} asking_configuration_machines_t;\n",
        "\n    asking_configuration_machines_t *machines = (asking_configuration_machines_t \
         *)user_data;\n",
        "\n    if (strcmp(component_type, \"param_asker\") == 0) {\n        for (size_t i = 0u; i \
         < 1u; ++i) {\n            sce_test_component_machine_t *made = \
         param_asker_component_make(&machines->param_asker_components[i]);\n",
        "\n    factory.make = asking_configuration_machine;\n    factory.user_data = machines;\n",
        "\nstatic inline size_t asking_configuration_module_parameter_count(void) {\n    return \
         1u;\n}\n",
        "\n    static const char *const ids[] = {\"job\"};\n    return ids;\n",
    ] {
        assert!(
            asking.contains(needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    assert!(
        !asking.contains("exchange_port"),
        "a port type writes no unit, so none is included"
    );

    let serving = header("serving_configuration");
    for needle in [
        "\n#include \"coordinator_sm.h\"\n",
        "\n#include \"answerer_sm.h\"\n",
        "\n    static const sce_test_ptc_t ptcs[] = {{\"answerer\", \"answerer\"}};\n",
        "\n    static const sce_test_declared_t ports[] = {{\"coordinator\", \"peer\"}, \
         {\"answerer\", \"peer\"}};\n",
        "\n    static const sce_test_connection_t connections[] = {{{\"mtc\", \"peer\"}, \
         {\"answerer\", \"peer\"}}};\n",
        "\n        \"coordinator\",\n        ptcs, 1u,\n        ports, 2u,\n        NULL, 0u,\n        \
         connections, 1u,\n        NULL, 0u,\n    };\n",
        "\n    coordinator_component_t coordinator_components[1];\n    answerer_component_t \
         answerer_components[1];\n",
        "\n    return 0u;\n",
        "\n    return NULL;\n}\n",
    ] {
        assert!(
            serving.contains(needle),
            "serving_configuration holds `{needle}`:\n{serving}"
        );
    }
    assert!(
        at(&serving, "strcmp(component_type, \"coordinator\")")
            < at(&serving, "strcmp(component_type, \"answerer\")"),
        "the MTC's type comes first"
    );
    assert!(
        !serving.contains("static const char *const ids[]"),
        "a configuration whose components read no module parameter lists none"
    );

    let keeping = header("keeping_configuration");
    for needle in [
        "\n    static const sce_test_declared_t timers[] = {{\"ptc_keeper\", \"patience\"}, \
         {\"ptc_keeper\", \"linger\"}};\n",
        "\n    static const sce_test_declared_t ports[] = {{\"idler\", \"peer\"}};\n",
        "\n    static const sce_test_mapping_t mappings[] = {{{\"idler\", \"peer\"}, \
         \"quiet\"}};\n",
    ] {
        assert!(
            keeping.contains(needle),
            "keeping_configuration holds `{needle}`:\n{keeping}"
        );
    }
    assert!(
        !keeping.contains("{\"ptc_keeper\", \"peer\"}"),
        "a type that declares no port has no entry"
    );
}

/// The text of `haystack` from the first `from` to the first `to` after it.
fn between<'h>(haystack: &'h str, from: &str, to: &str) -> &'h str {
    let start = at(haystack, from);
    let end = start
        + haystack[start..]
            .find(to)
            .unwrap_or_else(|| panic!("`{to}` after `{from}`:\n{haystack}"));
    &haystack[start..end]
}

/// docs/SCE_TTCN3_LANE.md §10.2, §10.5: a configuration's header reads each
/// record its system ports take from its canonical form and writes it back,
/// over the one walk of the form, and its `_input` reads an input of a system
/// port by the record its type's `in` list names, refusing any other by name.
/// A configuration with no system port refuses every input. The twin of the C++
/// test of the same name.
#[test]
fn a_configuration_reads_the_inputs_of_its_system_ports() {
    let asking = header("asking_configuration");
    for needle in [
        "\n#include \"sce/test/input_trace.h\"\n",
        "\n#include \"sce/test/from_canonical.h\"\n",
        "\n#include \"sce/test/canonical.h\"\n",
        "\n#include \"job_reply.h\"\n",
        "\nstatic inline bool asking_configuration_read_job_reply(sce_jv_t value, JobReplyPayload_t \
         *out, sce_saved_error_t *err) {\n    memset(out, 0, sizeof(*out));\n    {\n        static \
         const char *const sce_fields_0[] = {\"job\", \"code\"};\n        if \
         (!sce_test_from_canonical_record(value, sce_fields_0, 2u, \"job_reply\", err)) {\n            \
         return false;\n        }\n    }\n",
        "\n        sce_jv_t sce_field_0_job;\n        if (!sce_test_from_canonical_field(value, \
         \"job\", \"job_reply.job\", &sce_field_0_job, err)) {\n            return false;\n        \
         }\n",
        "\n            if (!sce_test_from_canonical_unsigned(sce_field_0_job, UINT32_MAX, \
         \"job_reply.job\", &sce_held, err)) {\n                return false;\n            }\n            \
         (*out).job = (uint32_t)sce_held;\n",
        "\nstatic inline void asking_configuration_canonical_job_reply(sce_jcs_t *jcs, const \
         JobReplyPayload_t *value) {\n",
        "\n    static const sce_test_record_type_t record_type = {\"job_reply\", \
         asking_configuration_canonical_job_reply_lent};\n",
        "\n    if (strcmp(system_port, \"sut\") == 0 && strcmp(type_name, \"job_reply\") == 0) {\n        \
         JobReplyPayload_t held;\n        if (!asking_configuration_read_job_reply(value, &held, \
         err)) {\n            return false;\n        }\n        return sce_test_input_message(\n            \
         message, JOB_REPLY_EVENT_NAME,\n            \
         sce_test_message_value(SCE_HOST_VALUE(JobReplyPayload_t, &held), \
         asking_configuration_record_type_of_job_reply()), err);\n    }\n",
        "\n    static const char *const system_ports[] = {\"sut\"};\n    return \
         sce_test_not_an_input(system_port, type_name, system_ports, 1u, err);\n}\n",
    ] {
        assert!(asking.contains(needle), "`{needle}` in\n{asking}");
    }
    assert_eq!(
        asking
            .matches("asking_configuration_read_job_reply(sce_jv_t")
            .count(),
        1,
        "a record is read by one function"
    );
    assert!(
        !asking.contains("job_request.h"),
        "a record the system port only sends is read by no one: {asking}"
    );

    let keeping = header("keeping_configuration");
    assert!(
        keeping.contains(
            "if (strcmp(system_port, \"quiet\") == 0 && strcmp(type_name, \"job_reply\") == 0) {"
        ),
        "{keeping}"
    );

    let serving = header("serving_configuration");
    assert!(
        serving.contains(
            "\nstatic inline bool serving_configuration_input(const char *system_port, const char \
             *type_name, sce_jv_t value,\n                                      sce_test_message_t \
             *message, sce_saved_error_t *err) {\n    (void)value;\n    (void)message;\n    return \
             sce_test_not_an_input(system_port, type_name, NULL, 0u, err);\n}\n"
        ),
        "a configuration with no system port refuses every input: {serving}"
    );
    assert!(!serving.contains("_read_"), "{serving}");
    assert!(!serving.contains("from_canonical.h"), "{serving}");
}

/// docs/SCE_TTCN3_LANE.md §10.3: the configuration whose system port takes a
/// record of every form C11 holds generates a reader and a writer of each,
/// which `backends/c/tests/integration/test_a_record_reads_back_the_form_it_writes.c`
/// compiles and round-trips; the one whose record holds a union, which C11 does
/// not lower, is refused by name.
#[test]
fn a_configuration_reads_a_record_of_every_form_it_holds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/trace_forms/unionless_forms_configuration.scxml");
    let files =
        generate(&path).unwrap_or_else(|e| panic!("the configuration generates on C11: {e:?}"));
    assert_eq!(files[0].0, "unionless_forms_configuration.h");
    let forms = &files[0].1;
    for record in ["order_taken", "job_status", "form_extras"] {
        for needle in [
            format!("\n#include \"{record}.h\"\n"),
            format!("\nstatic inline bool unionless_forms_configuration_read_{record}(sce_jv_t value, "),
            format!(
                "\nstatic inline void unionless_forms_configuration_canonical_{record}(sce_jcs_t *jcs, \
                 const "
            ),
            format!(
                "if (strcmp(system_port, \"sut\") == 0 && strcmp(type_name, \"{record}\") == 0) {{"
            ),
        ] {
            assert!(forms.contains(&needle), "`{needle}` in\n{forms}");
        }
    }
    for needle in [
        "static const uint64_t sce_numbers[] = {UINT64_C(0), UINT64_C(1), UINT64_C(2)};",
        "sce_test_from_canonical_open_unsigned(sce_field_0_shape, sce_names, sce_numbers, 3u, \
         UINT8_MAX, \"form_extras.shape\", &sce_held, err)",
        "(*out).shape = (ShapeKindOpen_t)sce_held;",
        "static const PaintColor_t sce_values[] = {PAINT_COLOR_RED, PAINT_COLOR_GREEN, \
         PAINT_COLOR_BLUE};",
        "(*out).lines.items[(*out).lines.len].color = sce_values[sce_index];",
        "if (sce_test_from_canonical_present(sce_item_1)) {",
        "if (sce_test_from_canonical_optional_field(value, \"note\", &sce_present_1)) {",
        "sce_test_from_canonical_bytes(sce_field_0_voucher_code, 4u, \"order_taken.voucher_code\", \
         (*out).voucher_code.data, sizeof((*out).voucher_code.data), &sce_length, err)",
        "(*out).voucher_code.len = sce_length;",
        "sce_test_from_canonical_elements(sce_field_0_flags, 1u, 2u, \"form_extras.flags\", \
         &sce_items_1, err)",
        "(*out).grid.items[(*out).grid.len].items[(*out).grid.items[(*out).grid.len].len]",
        "sce_test_from_canonical_signed(sce_field_0_least, INT64_MIN, INT64_MAX, \
         \"form_extras.least\", &sce_held, err)",
        "sce_test_from_canonical_elements(sce_present_1, 0u, 2u, \"form_extras.spare\", \
         &sce_items_2, err)",
    ] {
        assert!(forms.contains(needle), "`{needle}` in\n{forms}");
    }
    let readers = between(
        forms,
        "unionless_forms_configuration_read_order_taken(",
        "unionless_forms_configuration_input(",
    );
    assert!(
        !readers.contains("sce_case_"),
        "no record a C11 configuration reads holds a union:\n{readers}"
    );

    let union = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/trace_forms/forms_configuration.scxml");
    let Err(err) = generate(&union) else {
        panic!("a configuration whose record holds a union generated on C11");
    };
    assert!(format!("{err:?}").contains("union"), "{err:?}");
}

/// docs/SCE_TTCN3_LANE.md §10.5: two records a configuration's system ports
/// take that the export would name alike, the stems of their files, are
/// refused by name, since one reader and one type of the test log would hold
/// both. The twin of the Kotlin, Rust and C++ tests of the same name, over the
/// one refusal of the shared rows (`test_tables::trace_records`).
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
        panic!("two records named alike generated on C11");
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

/// §11.7: a test case is its name and its configuration's tables, reached
/// through the configuration's unit, over the storage of its configuration's
/// machines.
#[test]
fn a_test_case_names_its_configurations_tables() {
    let case = header("answers_the_job");
    for needle in [
        "\n#include \"sce/test/suite.h\"\n",
        "\n#include \"asking_configuration.h\"\n",
        "\n#define ANSWERS_THE_JOB_NAME \"answers_the_job\"\n",
        "\ntypedef asking_configuration_machines_t answers_the_job_machines_t;\n",
        "\nstatic inline sce_test_case_t answers_the_job_test_case(answers_the_job_machines_t \
         *machines) {\n",
        "\n    test_case.name = ANSWERS_THE_JOB_NAME;\n",
        "\n    test_case.configuration = asking_configuration_configuration();\n",
        "\n    test_case.machines = asking_configuration_machines(machines);\n",
        "\n    test_case.module_parameters = asking_configuration_module_parameters();\n",
        "\n    test_case.module_parameter_count = asking_configuration_module_parameter_count();\n",
        "\n    test_case.inputs = asking_configuration_input;\n",
    ] {
        assert!(
            case.contains(needle),
            "answers_the_job holds `{needle}`:\n{case}"
        );
    }
}

/// docs/SCE_TTCN3_LANE.md §4: a suite's header reads the value its runner's
/// command line gives each module parameter by the reader of its type, refuses
/// one the suite does not declare, and declares the runner program's `main`
/// over storage of static duration. The twin of the C++ test of the same name.
#[test]
fn a_suite_declares_its_runner_program() {
    let suite = header("runtime_suite");
    for needle in [
        "\n#include \"sce/test/runner.h\"\n",
        "\nstatic inline bool runtime_suite_module_parameter(const char *id, const char *literal, \
         char *out, size_t cap,\n",
        "\n    if (strcmp(id, \"job\") == 0) {\n        return \
         sce_test_runner_unsigned_parameter(id, literal, UINT32_MAX, false, out, cap, err);\n    \
         }\n",
        "\n    return sce_test_runner_undeclared_parameter(id, err);\n}\n",
        "\nstatic inline int runtime_suite_main(int argc, char **argv) {\n    static \
         runtime_suite_storage_t storage;\n    static sce_test_runner_t runner;\n    return \
         sce_test_runner_main(&runner, runtime_suite_suite(&storage), \
         runtime_suite_module_parameter, argc, argv);\n}\n",
    ] {
        assert!(suite.contains(needle), "`{needle}` in\n{suite}");
    }
    assert!(
        !suite.contains("(void)literal;"),
        "a suite that declares a module parameter reads its literal: {suite}"
    );
}

/// §11.8, P11: a suite holds its module parameters at their defaults and its
/// test cases in run order, each with its guard and its selection over the
/// features, in storage that holds the machines of each test case; the feature
/// model writes no unit, so none is included.
#[test]
fn a_suite_holds_its_cases_in_order_with_guards_and_selections() {
    let suite = header("runtime_suite");
    for needle in [
        "\n#include \"sce/test/suite.h\"\n",
        "\nSCE_STATIC_ASSERT(1u <= SCE_TEST_MAX_MODULE_PARAMETERS,\n",
        "\nstatic inline bool runtime_suite_selects_2(const sce_test_features_t *world) {\n    \
         return sce_test_enabled(world, \"ptcs\");\n}\n",
        "\nstatic inline bool runtime_suite_selects_3(const sce_test_features_t *world) {\n    \
         return sce_test_enabled(world, \"ptcs\") && sce_test_enabled(world, \"removal\");\n}\n",
        "\nstatic inline bool runtime_suite_selects_4(const sce_test_features_t *world) {\n    \
         return sce_test_enabled(world, \"guard\") || sce_test_enabled(world, \"removal\");\n}\n",
        "\ntypedef struct runtime_suite_storage_s {\n    sce_test_suite_t suite;\n    \
         sce_test_execute_t executes[4];\n    answers_the_job_machines_t \
         answers_the_job_machines;\n    serves_a_ptc_machines_t serves_a_ptc_machines;\n    \
         keeps_a_ptc_machines_t keeps_a_ptc_machines;\n    answers_before_the_guard_machines_t \
         answers_before_the_guard_machines;\n} runtime_suite_storage_t;\n",
        "\nstatic inline const sce_test_suite_t *runtime_suite_suite(runtime_suite_storage_t \
         *storage) {\n    memset(storage, 0, sizeof(*storage));\n",
        "\n    storage->suite.name = \"runtime_suite\";\n",
        "\n    (void)sce_test_module_parameters_set(&storage->suite.module_parameters, \"job\", \
         \"7\");\n",
        "\n    storage->suite.executes = storage->executes;\n    storage->suite.execute_count = \
         4u;\n    return &storage->suite;\n}\n",
    ] {
        assert!(
            suite.contains(needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
    }
    assert!(!suite.contains("runtime_features"), "{suite}");
    assert!(
        !suite.contains("runtime_suite_selects_1"),
        "a test case every world runs has no selection"
    );
    let executes = [
        "\n    storage->executes[0].test_case = \
         answers_the_job_test_case(&storage->answers_the_job_machines);\n    \
         storage->executes[0].guarded = true;\n    storage->executes[0].guard = \
         INT64_C(10000000);\n    storage->executes[0].selection = NULL;\n",
        "\n    storage->executes[1].test_case = \
         serves_a_ptc_test_case(&storage->serves_a_ptc_machines);\n    \
         storage->executes[1].guarded = false;\n    storage->executes[1].guard = 0;\n    \
         storage->executes[1].selection = runtime_suite_selects_2;\n",
        "\n    storage->executes[2].test_case = \
         keeps_a_ptc_test_case(&storage->keeps_a_ptc_machines);\n    \
         storage->executes[2].guarded = false;\n    storage->executes[2].guard = 0;\n    \
         storage->executes[2].selection = runtime_suite_selects_3;\n",
        "\n    storage->executes[3].test_case = \
         answers_before_the_guard_test_case(&storage->answers_before_the_guard_machines);\n    \
         storage->executes[3].guarded = true;\n    storage->executes[3].guard = INT64_C(100);\n    \
         storage->executes[3].selection = runtime_suite_selects_4;\n",
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
            suite.contains(&format!("\n#include \"{include}\"\n")),
            "{include}"
        );
    }
}

/// §11.8, §12.2: each module parameter's default is the compact JSON text of
/// the saved form of its type, the one a machine's saved state writes a value
/// of that type as: a number narrower than 64 bits and a `bool` as the JSON
/// value they are, a 64-bit number as its text, a text as the JSON text of
/// itself, a byte string as its Latin-1 text, and a value of an enum as the
/// name its document declares for the variant. Each is a C string literal.
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
    let files = generate(&path).unwrap_or_else(|e| panic!("the suite generates on C11: {e:?}"));
    let text = &files[0].1;
    let set = |id: &str, value: &str| {
        format!(
            "\n    (void)sce_test_module_parameters_set(&storage->suite.module_parameters, \
             \"{id}\", {value});\n"
        )
    };
    for needle in [
        "\nSCE_STATIC_ASSERT(7u <= SCE_TEST_MAX_MODULE_PARAMETERS,\n".to_string(),
        set("job", "\"7\""),
        set("armed", "\"true\""),
        set("offset", "\"-5\""),
        set("widest", "\"\\\"18446744073709551615\\\"\""),
        set("label", "\"\\\"nightly\\\"\""),
        set("key", "\"\\\"\\\\n\\303\\277\\\"\""),
        set("color", "\"\\\"green\\\"\""),
        "\n    storage->executes[0].guarded = false;\n    storage->executes[0].guard = 0;\n    \
         storage->executes[0].selection = NULL;\n"
            .to_string(),
        "\n        return sce_test_runner_bool_parameter(id, literal, out, cap, err);\n".to_string(),
        "\n        return sce_test_runner_signed_parameter(id, literal, INT8_MIN, INT8_MAX, false, \
         out, cap, err);\n"
            .to_string(),
        "\n        return sce_test_runner_unsigned_parameter(id, literal, UINT64_MAX, true, out, \
         cap, err);\n"
            .to_string(),
        "\n        return sce_test_runner_text_parameter(id, literal, out, cap, err);\n".to_string(),
        "\n        return sce_test_runner_bytes_parameter(id, literal, out, cap, err);\n".to_string(),
        "\n        return sce_test_runner_enum_parameter(id, literal, out, cap, err);\n".to_string(),
    ] {
        assert!(
            text.contains(&needle),
            "typed_defaults holds `{needle}`:\n{text}"
        );
    }
    assert!(
        !text.contains("paint_color.h"),
        "an enum's default is its variant's name, so its unit is not included"
    );
}

/// A suite that runs no test case holds no entry and no machine: its storage
/// is the suite alone, whose entries are none.
#[test]
fn a_suite_of_no_test_case_holds_no_entry() {
    let dir = tempfile::tempdir().expect("a directory");
    let suite = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-suite" name="empty_suite">
</scxml>
"#;
    let path = dir.path().join("empty_suite.scxml");
    std::fs::write(&path, suite).expect("the suite is written");
    let files = generate(&path).unwrap_or_else(|e| panic!("the suite generates on C11: {e:?}"));
    let text = &files[0].1;
    for needle in [
        "\ntypedef struct empty_suite_storage_s {\n    sce_test_suite_t suite;\n} \
         empty_suite_storage_t;\n",
        "\n    storage->suite.executes = NULL;\n    storage->suite.execute_count = 0u;\n",
        "\n    (void)literal;\n    (void)out;\n    (void)cap;\n    return \
         sce_test_runner_undeclared_parameter(id, err);\n}\n",
    ] {
        assert!(
            text.contains(needle),
            "empty_suite holds `{needle}`:\n{text}"
        );
    }
    assert!(!text.contains("SCE_STATIC_ASSERT("), "{text}");
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
        panic!("a guard past the runtime's clock generated on C11");
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

/// §11.3, §11.9: a port type writes no file on C11: the units of the
/// components and configurations that read it hold what they need of it.
#[test]
fn a_port_type_writes_no_file() {
    let files = generate(&runtime_dir().join("exchange_port.scxml"))
        .unwrap_or_else(|e| panic!("a port type generates on C11: {e:?}"));
    assert!(files.is_empty(), "{files:?}");
}

/// The examples of the kinds generate on C11, the suite's selection over the
/// example feature model.
#[test]
fn the_kind_examples_generate_their_tables() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("kind-examples");
    let suite = generate(&examples.join("test-suite.scxml"))
        .unwrap_or_else(|e| panic!("the example suite generates on C11: {e:?}"));
    assert_eq!(suite[0].0, "test_suite.h");
    for needle in [
        "\n    (void)sce_test_module_parameters_set(&storage->suite.module_parameters, \
         \"elapsed_limit\", \"500\");\n",
        "\n    return sce_test_enabled(world, \"session-unicast\");\n",
        "\n    storage->executes[0].guard = INT64_C(10000000);\n",
    ] {
        assert!(
            suite[0].1.contains(needle),
            "the example suite holds `{needle}`:\n{}",
            suite[0].1
        );
    }
    let case = generate(&examples.join("test-case.scxml"))
        .unwrap_or_else(|e| panic!("the example case generates on C11: {e:?}"));
    assert!(case[0]
        .1
        .contains("test_case.configuration = test_configuration_configuration();"));
    let configuration = generate(&examples.join("test-configuration.scxml"))
        .unwrap_or_else(|e| panic!("the example configuration generates on C11: {e:?}"));
    assert!(configuration[0]
        .1
        .contains("static const char *const ids[] = {\"elapsed_limit\"};"));
}
