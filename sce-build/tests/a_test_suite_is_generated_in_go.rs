// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The tables of a test generated in Go (docs/SCE_TEST_PROFILE.md §11.6 to
//! §11.9, §12.2, §12.6 slice 6): a test configuration's unit declares the
//! configuration the profile's runtime reads, the factory of its components'
//! machines and the module parameters they read; a test case's unit its entry
//! of the runner, through its configuration's unit; a suite's unit its module
//! parameters at their defaults, as saved values of their types, and its test
//! cases in run order, each with its guard and its selection. A port type
//! writes no file. A configuration's unit also reads the inputs of a recorded
//! trace, by a reader and a writer of each record its system ports take, and a
//! suite's is the entry of its runner program (docs/SCE_TTCN3_LANE.md §4,
//! §10.5). The twin of `a_test_suite_is_generated_in_kotlin.rs`, over the same
//! fixtures.
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
        timers.contains("\t\t\t\"param_asker\": {\"deadline\"},\n"),
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
        "\t\t\t\"ptc_keeper\": {\"patience\", \"linger\"},\n",
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
        !ports.contains("\"ptc_keeper\""),
        "a type that declares no port has no entry: {ports}"
    );
}

/// The import spec of the profile's package of the canonical form, whose
/// readers and writers a configuration's records are read and written by.
const CANONICAL_IMPORT: &str =
    "\t\"github.com/newmassrael/sce-go-runtime/testprofile/canonical\"\n";

/// The import spec of the forge runtime's optional value.
const OPTIONAL_IMPORT: &str = "\t\"github.com/newmassrael/sce-forge-runtime/optional\"\n";

/// docs/SCE_TTCN3_LANE.md §10.2, §10.5: a configuration reads the inputs of a
/// recorded trace. Each record on the `in` list of a system port's type is read
/// back from its canonical form and written in it, once, through the package of
/// its event schema; an input on a port the configuration does not have, or of
/// a type its port does not take, is refused by name, and a record only sent to
/// the system under test is not read. A configuration with no system port
/// refuses every input. The twin of the Rust, Kotlin and C++ cases.
#[test]
fn a_configuration_reads_the_inputs_of_a_recorded_trace() {
    let asking = unit("asking_configuration");
    for needle in [
        CANONICAL_IMPORT.to_string(),
        imported("job_reply"),
        "func ReadJobReply(value any) (sce_job_reply.JobReplyPayload, error) {\n\treturn func() \
         (sce_job_reply.JobReplyPayload, error) { object_0, err := canonical.ReadRecord(value, \
         []string{\"job\", \"code\"}, \"job_reply\"); if err != nil { return \
         sce_job_reply.JobReplyPayload{}, err }; var read sce_job_reply.JobReplyPayload; "
            .to_string(),
        "if read.Job, err = canonical.ReadInteger[uint32](canonical.Member(object_0, \"job\"), \
         \"job_reply.job\"); err != nil { return sce_job_reply.JobReplyPayload{}, err }; "
            .to_string(),
        "return read, nil }()\n}".to_string(),
        "func CanonicalJobReply(value sce_job_reply.JobReplyPayload) any {".to_string(),
        "canonical.Integer(value.Code)".to_string(),
        "func Input(systemPort, typeName string, value any) (testprofile.Message, error) {"
            .to_string(),
        "\tif systemPort == \"sut\" && typeName == \"job_reply\" {\n\t\tread, err := \
         ReadJobReply(value)\n"
            .to_string(),
        "\t\treturn testprofile.NewMessage(sce_job_reply.EventName, read, \"job_reply\", \
         CanonicalJobReply), nil\n"
            .to_string(),
        "\treturn testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, \
         []string{\"sut\"})\n"
            .to_string(),
    ] {
        assert!(
            asking.contains(&needle),
            "asking_configuration holds `{needle}`:\n{asking}"
        );
    }
    assert!(
        !asking.contains("ReadJobRequest") && !asking.contains(&imported("job_request")),
        "a record the components only send is not read: {asking}"
    );
    assert_eq!(
        asking.matches("func ReadJobReply(").count(),
        1,
        "a record is read by one function"
    );
    assert!(
        !asking.contains(OPTIONAL_IMPORT),
        "a record that holds no optional value names none: {asking}"
    );

    let keeping = unit("keeping_configuration");
    assert!(
        keeping.contains("\tif systemPort == \"quiet\" && typeName == \"job_reply\" {\n"),
        "{keeping}"
    );

    let serving = unit("serving_configuration");
    assert!(
        serving.contains(
            "func Input(systemPort, typeName string, _ any) (testprofile.Message, error) {\n\t\
             return testprofile.Message{}, testprofile.NotAnInput(systemPort, typeName, \
             []string{})\n}"
        ),
        "a configuration with no system port refuses every input: {serving}"
    );
    assert!(!serving.contains("func Read"), "{serving}");
    assert!(!serving.contains(CANONICAL_IMPORT), "{serving}");
}

/// docs/SCE_TTCN3_LANE.md §10.3: the configuration whose system port takes a
/// record of every form of the canonical value generates a reader and a writer
/// of each, which
/// `backends/go/tests/integration/static_datamodel/a_record_reads_back_the_form_it_writes_test.go`
/// compiles and round-trips: an open enum read through its package's
/// `FromUnderlying`, a closed one by its variants, a list of optional elements
/// and an optional field each by a function literal that answers the absent
/// value or the value read, a list of records each record by an object of its
/// own, a union by a `switch` on its case whose `default` is its last, each
/// case built by its constructor, a byte string by its hex. The twin of the
/// Rust and Kotlin cases, over the same configuration.
#[test]
fn a_configuration_reads_a_record_of_every_form_it_holds() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/trace_forms/forms_configuration.scxml");
    let forms = file_at(&path, "forms_configuration");
    for (record, function) in [
        ("order_placed", "OrderPlaced"),
        ("job_status", "JobStatus"),
        ("form_extras", "FormExtras"),
    ] {
        for needle in [
            imported(record),
            format!("func Read{function}(value any) (sce_{record}.{function}Payload, error) {{"),
            format!("func Canonical{function}(value sce_{record}.{function}Payload) any {{"),
            format!("\tif systemPort == \"sut\" && typeName == \"{record}\" {{\n"),
        ] {
            assert!(forms.contains(&needle), "`{needle}` in\n{forms}");
        }
    }
    for needle in [
        OPTIONAL_IMPORT.to_string(),
        imported("order_line"),
        imported("order_customer"),
        imported("order_payment"),
        imported("card_payment"),
        imported("voucher_payment"),
        imported("paint_color"),
        imported("shape_kind_open"),
        "canonical.ReadOpenEnumerated(canonical.Member(object_0, \"shape\"), \
         []canonical.Variant[sce_shape_kind_open.ShapeKindOpen]{{Name: \"dot\", Value: \
         sce_shape_kind_open.ShapeKindOpenDot}, "
            .to_string(),
        "}, sce_shape_kind_open.ShapeKindOpenFromUnderlying, \"form_extras.shape\")".to_string(),
        "canonical.ReadEnumerated(canonical.Member(object_1, \"color\"), \
         []canonical.Variant[sce_paint_color.PaintColor]{{Name: \"red\", Value: \
         sce_paint_color.PaintColorRed}, "
            .to_string(),
        "object_1, err := canonical.ReadRecord(item_0, []string{\"sku\", \"quantity\", \
         \"color\"}, \"order_placed.lines[]\")"
            .to_string(),
        "func(present_1 any) (optional.Of[uint16], error) { if present_1 == nil { return \
         optional.None[uint16](), nil }; read, err := canonical.ReadInteger[uint16](present_1, \
         \"form_extras.slots[]\")"
            .to_string(),
        "return optional.Some(read), nil }(item_0)".to_string(),
        "func(present_0 any, held bool) (optional.Of[string], error) { if !held { return \
         optional.None[string](), nil }; read, err := canonical.ReadText(present_0, 32, \
         \"order_placed.note\")"
            .to_string(),
        "}(object_0.Get(\"note\"))".to_string(),
        "if read.Payment, err = func() (sce_order_payment.OrderPayment, error) { variant_0, \
         case_0, err := canonical.ReadUnionCase(canonical.Member(object_0, \"payment\"), \
         []string{\"card\", \"cash\", \"voucher\"}, \"order_placed.payment\"); if err != nil { \
         return sce_order_payment.OrderPayment{}, err }; switch variant_0 { case \"card\": read, \
         err := func() (sce_card_payment.CardPaymentPayload, error) { object_1, err := \
         canonical.ReadRecord(case_0, []string{\"last4\", \"amount\"}, \"order_placed.payment\")"
            .to_string(),
        "canonical.ReadText(canonical.Member(object_1, \"last4\"), 4, \
         \"order_placed.payment.last4\")"
            .to_string(),
        "return sce_order_payment.OrderPaymentCard(read), nil; case \"cash\": if err := \
         canonical.ReadNoRecord(case_0, \"order_placed.payment.cash\"); err != nil { return \
         sce_order_payment.OrderPayment{}, err }; return sce_order_payment.OrderPaymentCash(), \
         nil; default: read, err := func() (sce_voucher_payment.VoucherPaymentPayload, error) {"
            .to_string(),
        "canonical.ReadBytes(canonical.Member(object_1, \"voucher_code\"), 4, \
         \"order_placed.payment.voucher_code\")"
            .to_string(),
        "return sce_order_payment.OrderPaymentVoucher(read), nil } }()".to_string(),
        "canonical.ReadElements(canonical.Member(object_0, \"flags\"), 1, 2, \
         \"form_extras.flags\")"
            .to_string(),
        "func() ([][]uint8, error) { items_0, err := ".to_string(),
        "items_1, err := canonical.ReadElements(item_0, 0, 2, \"form_extras.grid[]\")".to_string(),
        "if read.Least, err = canonical.ReadInteger[int64](canonical.Member(object_0, \
         \"least\"), \"form_extras.least\")"
            .to_string(),
    ] {
        assert!(forms.contains(&needle), "`{needle}` in\n{forms}");
    }
    // The reader names no package of the tag's enum: the variant is read as its
    // declared name, and each case is built by its constructor.
    assert!(!forms.contains(&imported("payment_kind")), "{forms}");
}

/// docs/SCE_TTCN3_LANE.md §10.5: two records a configuration's system ports
/// take that the export would name alike, the stems of their files, are
/// refused by name, since one reader and one type of the test log would hold
/// both. The twin of the Rust, Kotlin and C++ tests of the same name, over the
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
        panic!("two records named alike generated on Go");
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
        "\t\tInputs:           sce_asking_configuration.Input,\n".to_string(),
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

/// docs/SCE_TTCN3_LANE.md §4: a suite's unit is the runner program's: its
/// `Main` hands the runtime's runner the suite and the reader of each module
/// parameter's value, a literal of its type, which an undeclared parameter is
/// refused by. The twin of the Rust, Kotlin and C++ cases.
#[test]
fn a_suite_is_the_runner_program_of_its_traces() {
    let suite = unit("runtime_suite");
    for needle in [
        "func ModuleParameter(id, literal string) (testprofile.ModuleParameterValue, error) {\n\t\
         switch id {\n\tcase \"job\":\n\t\treturn testprofile.IntegerParameter(id, literal, \
         sce.SavedWriteUint32)\n\t}\n\treturn nil, testprofile.UndeclaredParameter(id)\n}",
        "func Main() {\n\ttestprofile.RunnerMain(Suite(), ModuleParameter)\n}",
    ] {
        assert!(
            suite.contains(needle),
            "runtime_suite holds `{needle}`:\n{suite}"
        );
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
    assert!(
        text.contains(
            "func ModuleParameter(id, _ string) (testprofile.ModuleParameterValue, error) {\n\t\
             return nil, testprofile.UndeclaredParameter(id)\n}"
        ),
        "a suite that declares no module parameter refuses every one: {text}"
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
        // The runner's reader of each, a literal of its type (docs/SCE_TTCN3_LANE.md
        // §4): an integer at its width, saved by the writer of its type.
        "\tcase \"armed\":\n\t\treturn testprofile.BoolParameter(id, literal)\n".to_string(),
        "\tcase \"offset\":\n\t\treturn testprofile.IntegerParameter(id, literal, \
         sce.SavedWriteInt8)\n"
            .to_string(),
        "\tcase \"widest\":\n\t\treturn testprofile.IntegerParameter(id, literal, \
         sce.SavedWriteUint64)\n"
            .to_string(),
        "\tcase \"label\":\n\t\treturn testprofile.TextParameter(literal)\n".to_string(),
        "\tcase \"key\":\n\t\treturn testprofile.BytesParameter(id, literal)\n".to_string(),
        "\tcase \"color\":\n\t\treturn testprofile.EnumParameter(literal)\n".to_string(),
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
        "\tmoduleParameters.Set(\"elapsed_limit\", sce.SavedWriteUint32(500))\n".to_string(),
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
        configuration.contains("\treturn []string{\"elapsed_limit\"}\n"),
        "{configuration}"
    );
    let component = machine_files(&examples.join("test-component.scxml"));
    let component = &component[0].1;
    assert!(
        component.contains("if !context.ReadModuleParameters(policy.sceAcceptModuleParameters) {"),
        "{component}"
    );
    assert!(
        component.contains("given.Of(\"elapsed_limit\")"),
        "{component}"
    );
}
