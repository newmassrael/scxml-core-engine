// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in Go (docs/SCE_TEST_PROFILE.md §11.9, §12.3,
//! §12.6 slice 6): the statechart machine of the document, each branch a
//! transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives, which builds the machine on a policy its
//! context is set on. The twin of `a_test_component_is_generated_in_kotlin.rs`,
//! over the same fixtures.
//!
//! A machine imports the package of each template it names, under the module
//! path the run is given (`--go-module-prefix`), so the machine is generated
//! here through `sce-codegen generate`, the one entry that takes the path for a
//! statechart. Go holds a text, a byte string and an enum of a received message
//! where the record's unit holds them, so it copies a message the content reads
//! that Rust refuses, as Kotlin does. What Go refuses is held here too: a
//! component that declares a module parameter, whose value waits for Go's suite
//! tables; one that declares `<sce:action>`; and one whose template reaches a
//! union, which Go does not lower.
//!
//! What the generated components do when they run is held by
//! `backends/go/tests/integration/static_datamodel/a_test_component_runs_through_its_doors_test.go`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_go.sh`) and runs them through
//! `testprofile.TestCaseRun`.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::union_check::NOT_LOWERED_YET;

/// The module path the generated packages are imported under.
const PREFIX: &str = "example.com/generated";

/// The code of a refusal by name.
const UNSUPPORTED: &str = "generate/unsupported-feature";

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The directory of the components the runtime's end-to-end test runs.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The directory of the templates over imported records.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// The directory of the test profile's own fixtures.
fn profile_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile")
}

/// Every component of the runtime's end-to-end test that Go generates, each
/// with the name its machine is spelled by. `param_asker`, which declares a
/// module parameter, is refused
/// ([`a_component_that_declares_a_module_parameter_is_refused_on_go`]).
const COMPONENTS: [(&str, &str); 5] = [
    ("asker", "Asker"),
    ("coordinator", "Coordinator"),
    ("answerer", "Answerer"),
    ("keeper", "Keeper"),
    ("idler", "Idler"),
];

/// What `sce-codegen generate -l go` wrote for the document at `path`, given
/// the module path [`PREFIX`] when `prefixed`: whether it succeeded, its
/// diagnostics, and each Go file it wrote, by name, in order of name.
fn generate_with(path: &Path, prefixed: bool) -> (bool, String, Vec<(String, String)>) {
    let out = tempfile::tempdir().expect("a directory");
    let mut command = Command::new(sce_codegen_bin());
    command
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", "go"]);
    if prefixed {
        command.args(["--go-module-prefix", PREFIX]);
    }
    let run = command
        .arg(path)
        .arg("-o")
        .arg(out.path())
        .output()
        .expect("spawn sce-codegen");
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
    let diagnostics =
        String::from_utf8_lossy(&run.stderr).into_owned() + &String::from_utf8_lossy(&run.stdout);
    (run.status.success(), diagnostics, files)
}

/// The Go files of the document at `path`, which generates.
fn files_at(path: &Path) -> Vec<(String, String)> {
    let (ok, diagnostics, files) = generate_with(path, true);
    assert!(ok, "{} generates on Go:\n{diagnostics}", path.display());
    files
}

/// The whole Go text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    files_at(path)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole Go text of the component `name`.
fn text(name: &str) -> String {
    text_at(&runtime_dir().join(format!("{name}.scxml")))
}

/// The diagnostics of the document at `path`, which Go refuses by name.
fn refused(path: &Path) -> String {
    let (ok, diagnostics, _) = generate_with(path, true);
    assert!(!ok, "{} generated on Go", path.display());
    assert!(
        diagnostics.contains(UNSUPPORTED),
        "{} is refused by name:\n{diagnostics}",
        path.display()
    );
    diagnostics
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// The import spec of the generated package of the document `snake`, under
/// the alias every Go unit imports a sibling package by.
fn imported(snake: &str) -> String {
    format!("\tsce_{snake} \"{PREFIX}/{snake}\"")
}

/// §12.3: each component is its machine's file, and beside the machine the
/// component the runtime drives: the machine's policy is set on the runtime
/// context, under a session the component names, and the component registers
/// the two doors on the machine before it starts it and delivers the snapshot
/// event.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, machine) in COMPONENTS {
        let names: Vec<String> = files_at(&runtime_dir().join(format!("{name}.scxml")))
            .into_iter()
            .map(|(file, _)| file)
            .collect();
        assert_eq!(names, [format!("{name}_sm.go")]);
        let code = text(name);
        for needle in [
            "\tscetestprofile \"github.com/newmassrael/sce-go-runtime/testprofile\"".to_string(),
            "\tsceTestContext *scetestprofile.ComponentContext".to_string(),
            format!("type {machine}Component struct {{"),
            format!("machine *sce.Engine[{machine}State, {machine}Event]"),
            format!("var _ scetestprofile.ComponentMachine = (*{machine}Component)(nil)"),
            format!(
                "func (c *{machine}Component) Start(context *scetestprofile.ComponentContext) {{"
            ),
            format!("policy := New{machine}Policy()"),
            "if err := policy.SetSessionID(context.Name()); err != nil {".to_string(),
            "policy.sceTestContext = context".to_string(),
            format!("machine := sce.NewEngine[{machine}State, {machine}Event](&policy)"),
            "machine.RegisterEventProcessor(\"http://sce.dev/ext/test-port\", func(request \
             sce.HostSendRequest) []sce.HostSendResponse {"
                .to_string(),
            "panic(\"a test-port send carries its template's value\")".to_string(),
            "context.Send(request.Target, scetestprofile.Message{Event: request.EventName, \
             Value: request.Value})"
                .to_string(),
            "machine.RegisterInvoker(\"http://sce.dev/ext/test-component\", func(event \
             sce.HostInvokeEvent) *sce.HostInvokeResponse {"
                .to_string(),
            "context.Create(event.Start.InvokeID)".to_string(),
            "context.Kill(event.Cancel.InvokeID)".to_string(),
            format!("c.machine.ProcessEvent({machine}EventSceTestSnapshot)"),
            "return c.machine != nil && c.machine.IsInFinalState()".to_string(),
            format!("func (p *{machine}Policy) SetSessionID(id string) error {{"),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`:\n{code}");
        }
        let named = at(&code, "policy.SetSessionID(context.Name())");
        let set = at(&code, "policy.sceTestContext = context");
        let built = at(&code, "machine := sce.NewEngine[");
        let served = at(&code, "machine.RegisterEventProcessor(");
        let invoked = at(&code, "machine.RegisterInvoker(");
        let started = at(&code, "machine.Initialize()");
        assert!(
            named < set && set < built && built < served && served < invoked && invoked < started,
            "{name}: the policy is named and set on the context, and the doors are registered \
             on the machine before it starts"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks for its record's value under its record's event and calls
/// the template's `Matches`, both named through the template unit's package,
/// which the machine imports; a receive any, a timeout and a done read the
/// snapshot alone. A done branch is not taken on the event it names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "scetestprofile.Receives(p.sceTestContext, \"sut\", \"job.reply\", func(sceMessage \
         sce_reply_ok.Value) bool { return sce_reply_ok.Matches(sceMessage) })",
    );
    let timeout = at(&asker, "p.sceTestContext.TimedOut(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("if event == AskerEventSceTestSnapshot {"));
    for unit in ["reply_ok", "request_of"] {
        assert!(
            asker.contains(&imported(unit)),
            "the asker imports the package of {unit}, which its text names"
        );
    }

    let coordinator = text("coordinator");
    assert!(coordinator.contains("p.sceTestContext.Done(\"answerer\")"));
    assert!(
        !coordinator.contains("if event == CoordinatorEventDoneInvokeAnswerer {"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "scetestprofile.Receives(p.sceTestContext, \"peer\", \"job.request\", func(sceMessage \
         sce_any_request.Value) bool"
    ));
    for unit in ["any_request", "reply_of"] {
        assert!(coordinator.contains(&imported(unit)), "{unit}");
    }

    let idler = text("idler");
    assert!(idler.contains("p.sceTestContext.SnapshotPresent(\"peer\")"));
    assert!(
        !idler.contains("\tsce_"),
        "a component that names no template imports no template's package:\n{idler}"
    );
}

/// A branch's guard is its `cond` and then its match, so a `cond` that is
/// false never asks the snapshot (P2: the guard is evaluated before the
/// matching), and a branch whose guard is its match alone is still guarded.
#[test]
fn a_branch_with_a_cond_is_guarded_by_the_cond_and_then_its_match() {
    let dir = tempfile::tempdir().expect("a directory");
    let runtime = runtime_dir();
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="guarded"
       datamodel="sce-static" initial="waiting">
  <sce:import kind="test-port-type" src="{runtime}/exchange_port.scxml" as="ExchangePort"/>
  <sce:test-port id="sut" type="ExchangePort"/>
  <datamodel>
    <data id="armed" sce:type="bool" expr="true"/>
  </datamodel>
  <state id="waiting">
    <transition sce:receive="sut" cond="armed" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime.display()
    );
    let path = dir.path().join("guarded.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    assert!(
        code.contains(") && (p.sceTestContext.SnapshotPresent(\"sut\"))"),
        "the match follows the cond:\n{code}"
    );
    assert!(
        text("idler").contains("if p.sceTestContext.SnapshotPresent(\"peer\") {"),
        "a branch with no cond is guarded by its match"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch. A
/// receive branch whose content reads the message holds it in the payload
/// field of its record's event, which its content reads as `_event.data`, so
/// its content needs no check of a delivery's payload. A branch whose document
/// writes no content still has the statement as its content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let read = at(
        &asker,
        "{ sceValue, sceTaken := scetestprofile.ValueAs[sce_reply_ok.Value](p.sceTestContext.\
         Receive(\"sut\")); if !sceTaken { panic(\"a receive branch takes the message its \
         guard matched\") }; ",
    );
    let bound = at(
        &asker,
        "p.pendingJobReplyPayload = AskerJobReplyPayload{job: sceValue.Job, code: \
         sceValue.Code} }",
    );
    let content = at(&asker, "p.sceTestContext.StopTimer(\"reply\")");
    assert!(read < bound && bound < content);
    assert!(
        !asker.contains("if p.pendingPayloadTag != AskerPayloadTagJobReply {"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "p.sceTestContext.Timeout(\"reply\")");
    let failed = at(
        &asker,
        "p.sceTestContext.Setverdict(scetestprofile.VerdictFail, ",
    );
    assert!(timeout < failed);

    assert!(text("coordinator").contains("p.sceTestContext.DoneTaken(\"answerer\")"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer: each is a transition
    // with content, which the microstep runs.
    let idler = text("idler");
    assert!(idler.contains("p.sceTestContext.Receive(\"peer\")"));
    assert!(idler.contains("HasActions:      true,"), "{idler}");
    assert!(text("keeper").contains("p.sceTestContext.Timeout(\"patience\")"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and carries its specific template's value
/// to the test-port processor; a PTC is started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "p.sceTestContext.StartTimer(\"reply\", 1000000)",
        "p.sceTestContext.StopTimer(\"reply\")",
        "p.sceTestContext.Setverdict(scetestprofile.VerdictPass, \"\")",
        "no matching reply in time",
        "ProcessorType: \"http://sce.dev/ext/test-port\",",
        "\"job.request\"",
        "\"sut\",",
        "InvokeID: p.pendingEventInvokeid,",
        "Value: sce_request_of.Specific(p.vJob),",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`");
    }
    let coordinator = text("coordinator");
    for needle in [
        "Value: sce_reply_of.Specific(p.vJob, p.vOk),",
        "ProcessorType: \"http://sce.dev/ext/test-component\",",
        "InvokeID:      \"answerer\",",
    ] {
        assert!(coordinator.contains(needle), "coordinator holds `{needle}`");
    }
    assert!(text("keeper").contains("p.sceTestContext.StartTimer(\"patience\", 10000)"));
    assert!(text("idler")
        .contains("p.sceTestContext.Setverdict(scetestprofile.VerdictInconclusive, \"\")"));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label, a colon and its value as the engine's log
/// formats it, and never in the engine's log; a `<log>` that writes only a
/// label is recorded too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    assert!(
        asker.contains(
            "p.sceTestContext.Log(fmt.Sprintf(\"%s: %v\", \"answered\", \
             p.pendingJobReplyPayload.job))"
        ),
        "the asker logs the job of the message the branch took through the context:\n{asker}"
    );
    assert!(
        !asker.contains("fmt.Printf(\"%s%v\\n\", \"answered\""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("p.sceTestContext.Log(\"asked\")"));
}

/// The unit of a template takes every parameter by value, a number and a
/// text alike: the variables bound to its parameters are handed over as the
/// machine holds them.
#[test]
fn a_template_argument_is_handed_over_as_the_machine_holds_it() {
    let dir = tempfile::tempdir().expect("a directory");
    let profile = profile_dir();
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="reporter"
       datamodel="sce-static" initial="reporting">
  <sce:import kind="test-port-type" src="{profile}/job_port.scxml" as="JobPort"/>
  <sce:import kind="test-template" src="{profile}/job_report_of.scxml" as="JobReportOf"/>
  <sce:test-port id="sut" type="JobPort"/>
  <datamodel>
    <data id="elapsed" sce:type="uint32" expr="3"/>
    <data id="note" sce:type="string[32]" expr="'late'"/>
    <data id="code" sce:type="uint8" expr="9"/>
  </datamodel>
  <state id="reporting">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut"
            sce:test-template="JobReportOf" sce:template-args="elapsed note code"/>
    </onentry>
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        profile = profile.display()
    );
    let path = dir.path().join("reporter.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    assert!(
        code.contains("Value: sce_job_report_of.Specific(p.vElapsed, p.vNote, p.vCode),"),
        "{code}"
    );
    assert!(code.contains(&imported("job_report_of")), "{code}");
}

/// A machine names the package of each template it uses, which only the run's
/// module path places, so a component that uses a template is refused by name
/// without one, and a component that uses none generates without one.
#[test]
fn a_component_that_uses_a_template_needs_the_module_path() {
    let (ok, diagnostics, _) = generate_with(&runtime_dir().join("asker.scxml"), false);
    assert!(!ok, "the asker generated with no module path");
    assert!(diagnostics.contains("go_module_prefix"), "{diagnostics}");
    let (ok, diagnostics, _) = generate_with(&runtime_dir().join("keeper.scxml"), false);
    assert!(ok, "the keeper names no template:\n{diagnostics}");
}

/// A component over a record with a text field, whose receive branch
/// matches with a template that takes a number and whose content reads the
/// message.
fn card_watcher(dir: &Path) -> PathBuf {
    let imports = imports_dir();
    let port = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="card_port">
  <sce:import kind="event-schema" src="{imports}/card_payment.scxml" as="Card"/>
  <sce:message type="Card" direction="in"/>
</scxml>
"#,
        imports = imports.display()
    );
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="card_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="card_port.scxml" as="CardPort"/>
  <sce:import kind="test-template" src="{imports}/card_within.scxml" as="CardWithin"/>
  <sce:import kind="event-schema" src="{imports}/card_payment.scxml" as="Card"/>
  <sce:test-port id="sut" type="CardPort"/>
  <datamodel>
    <data id="most" sce:type="uint32" expr="5"/>
    <data id="paid" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="CardWithin"
                sce:template-args="most" target="done">
      <assign location="paid" expr="_event.data.amount"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    );
    std::fs::write(dir.join("card_port.scxml"), port).expect("the port type is written");
    let path = dir.join("card_watcher.scxml");
    std::fs::write(&path, component).expect("the component is written");
    path
}

/// The machine's payload struct and the record's unit hold a text in the same
/// `string`, so a receive branch whose content reads a message of a record
/// with a text field copies it on Go, as on Kotlin.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_copied_on_go() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = card_watcher(dir.path());
    let code = text_at(&path);
    for needle in [
        "scetestprofile.Receives(p.sceTestContext, \"sut\", \"payment.card\", func(sceMessage \
         sce_card_within.Value) bool { return sce_card_within.Matches(sceMessage, p.vMost) })",
        "sceValue, sceTaken := scetestprofile.ValueAs[sce_card_within.Value](",
        "p.pendingPaymentCardPayload = CardWatcherPaymentCardPayload{last4: sceValue.Last4, \
         amount: sceValue.Amount}",
    ] {
        assert!(
            code.contains(needle),
            "card_watcher holds `{needle}`:\n{code}"
        );
    }
}

/// Writes each `(file, text)` into `dir`.
fn write_all(dir: &Path, documents: &[(&str, String)]) {
    for (file, text) in documents {
        std::fs::write(dir.join(file), text).unwrap_or_else(|e| panic!("{file} is written: {e}"));
    }
}

/// A port type that receives the event schema `schema_file` declares,
/// imported as `alias`.
fn port_of(alias: &str, schema_file: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="watched_port">
  <sce:import kind="event-schema" src="{schema_file}" as="{alias}"/>
  <sce:message type="{alias}" direction="in"/>
</scxml>
"#
    )
}

/// A component that waits on its port `sut` for a message its template
/// `template` matches, given the variable `wanted` of type `wanted_type`
/// declared at `wanted_expr`.
fn color_watcher(template: &str, wanted_type: &str, wanted_expr: &str) -> String {
    let imports = imports_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="watched_port.scxml" as="WatchedPort"/>
  <sce:import kind="test-template" src="{template}.scxml" as="Watched"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:test-port id="sut" type="WatchedPort"/>
  <datamodel>
    <data id="wanted" sce:type="{wanted_type}" expr="{wanted_expr}"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="Watched"
                sce:template-args="wanted" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    )
}

/// A component that waits on its port `sut` for the exact order line and
/// keeps the line's color, which its content reads from the message.
fn color_reader() -> String {
    let imports = imports_dir();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_reader"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="watched_port.scxml" as="WatchedPort"/>
  <sce:import kind="test-template" src="{imports}/order_line_exact.scxml" as="ExactLine"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="OrderLine"/>
  <sce:test-port id="sut" type="WatchedPort"/>
  <datamodel>
    <data id="seen" sce:type="enum:Color" expr="Color.red"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="ExactLine" target="done">
      <assign location="seen" expr="_event.data.color"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    )
}

/// A template over an order line whose color is its one formal parameter.
fn line_of_color() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="line_of_color"
       sce:type="record:OrderLine">
  <sce:import kind="event-schema" src="{imports}/order_line.scxml" as="OrderLine"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="color" sce:type="enum:Color" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:any-value field="sku"/>
    <sce:any-value field="quantity"/>
    <sce:specific field="color" param="color"/>
  </sce:record-match>
</scxml>
"#,
        imports = imports_dir().display()
    )
}

/// The port type of an order line, written into `dir` as `watched_port.scxml`.
fn write_order_line_port(dir: &Path) {
    let schema = format!("{}/order_line.scxml", imports_dir().display());
    write_all(
        dir,
        &[("watched_port.scxml", port_of("OrderLine", &schema))],
    );
}

/// docs/SCE_TEST_PROFILE.md §11.9: the machine holds a value of an enum in its
/// own type and a template's unit takes the type the enum document's unit
/// declares, so a variable of an enum bound to a parameter crosses into the
/// variant of the same declared name, as a value handed to an algorithm does,
/// and the machine imports the enum's unit, whose type the crossing names.
#[test]
fn an_enum_argument_crosses_into_the_templates_enum() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(
        dir.path(),
        &[
            ("line_of_color.scxml", line_of_color()),
            (
                "color_watcher.scxml",
                color_watcher("line_of_color", "enum:Color", "Color.blue"),
            ),
        ],
    );
    let code = text_at(&dir.path().join("color_watcher.scxml"));
    assert!(
        code.contains(
            "sce_line_of_color.Matches(sceMessage, map[ColorWatcherColorEnum]sce_paint_color.\
             PaintColor{"
        ),
        "the argument crosses where the template is called: {code}"
    );
    assert!(code.contains("}[p.vWanted])"), "{code}");
    assert!(code.contains(&imported("paint_color")), "{code}");
    assert!(code.contains(&imported("line_of_color")), "{code}");
}

/// A message the content reads that holds an enum field is copied into the
/// machine's own enum type by the variant's declared name, read as a saved
/// value of that type is, where Rust and C++ refuse it by name.
#[test]
fn a_message_with_an_enum_field_its_content_reads_is_copied_on_go() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(dir.path(), &[("color_reader.scxml", color_reader())]);
    let code = text_at(&dir.path().join("color_reader.scxml"));
    for needle in [
        "scetestprofile.Receives(p.sceTestContext, \"sut\", \"order.line\", func(sceMessage \
         sce_order_line_exact.Value) bool { return sce_order_line_exact.Matches(sceMessage) })",
        "p.pendingOrderLinePayload = ColorReaderOrderLinePayload{sku: sceValue.Sku, quantity: \
         sceValue.Quantity, color: func() ColorReaderColorEnum { sceName, _ := \
         sceValue.Color.DeclaredName().Get(); sceHeld, sceErr := \
         ColorReaderColorEnumFromSaved(sceName, \"color\"); if sceErr != nil { panic(sceErr) \
         }; return sceHeld }()}",
        "func ColorReaderColorEnumFromSaved(value any, what string) (ColorReaderColorEnum, \
         error) {",
    ] {
        assert!(
            code.contains(needle),
            "color_reader holds `{needle}`:\n{code}"
        );
    }
}

/// An optional value of an enum has no crossing into a template's unit yet,
/// so a variable of one bound to a parameter is refused by name, on Go as on
/// Rust and Kotlin.
#[test]
fn an_optional_enum_argument_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let schema = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="tinted"
       sce:event-name="tinted">
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="tint" sce:type="optional&lt;enum:Color&gt;" sce:direction="in"/>
  </datamodel>
</scxml>
"#,
        imports = imports.display()
    );
    let template = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-template" name="tint_of"
       sce:type="record:Tinted">
  <sce:import kind="event-schema" src="tinted.scxml" as="Tinted"/>
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <datamodel>
    <data id="tint" sce:type="optional&lt;enum:Color&gt;" sce:direction="in"/>
  </datamodel>
  <sce:record-match>
    <sce:specific field="tint" param="tint"/>
  </sce:record-match>
</scxml>
"#,
        imports = imports.display()
    );
    write_all(
        dir.path(),
        &[
            ("tinted.scxml", schema),
            ("watched_port.scxml", port_of("Tinted", "tinted.scxml")),
            ("tint_of.scxml", template),
            (
                "color_watcher.scxml",
                color_watcher("tint_of", "optional&lt;enum:Color&gt;", "null"),
            ),
        ],
    );
    let diagnostics = refused(&dir.path().join("color_watcher.scxml"));
    assert!(
        diagnostics
            .contains("an optional value of the enum `Color` bound to a parameter of a test"),
        "{diagnostics}"
    );
}

/// §11.8, §12.6: Go generates a component and not yet the suite's tables, so
/// a component that declares a module parameter, whose value is the suite's,
/// is refused by that parameter's name, and no value is made up in its place.
#[test]
fn a_component_that_declares_a_module_parameter_is_refused_on_go() {
    assert!(
        !sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS
            .contains(&sce_build::generator::Language::Go),
        "the control: Go generates no suite's tables yet"
    );
    let diagnostics = refused(&runtime_dir().join("param_asker.scxml"));
    assert!(
        diagnostics.contains("the module parameter 'job' takes the value the suite declares"),
        "{diagnostics}"
    );
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives builds its
/// machine itself, and a Go policy of a machine that declares `<sce:action>`
/// is constructed with a host the component is given by nothing, so Go refuses
/// such a component by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_go() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="beeper"
       datamodel="sce-static" initial="beeping">
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="beeps" sce:type="uint32" expr="1"/>
  </datamodel>
  <state id="beeping">
    <onentry>
      <sce:action name="showBeeps" effect="notice">
        <sce:arg name="count" expr="beeps"/>
      </sce:action>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;
    let path = dir.path().join("beeper.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let diagnostics = refused(&path);
    for needle in [
        "test-component 'beeper' declares <sce:action>",
        "a parameter of the Go policy's constructor",
    ] {
        assert!(diagnostics.contains(needle), "`{needle}`:\n{diagnostics}");
    }
}

/// docs/adr/0092, section 7: a component whose template reaches a union is
/// refused on Go, which generates no unit of that template, by the union's
/// own refusal, as on C++.
#[test]
fn a_component_whose_template_reaches_a_union_is_refused_on_go() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let port = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-port-type" name="order_port">
  <sce:import kind="event-schema" src="{imports}/order_placed.scxml" as="OrderPlaced"/>
  <sce:message type="OrderPlaced" direction="in"/>
</scxml>
"#,
        imports = imports.display()
    );
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="order_watcher"
       datamodel="sce-static" initial="watching">
  <sce:import kind="test-port-type" src="order_port.scxml" as="OrderPort"/>
  <sce:import kind="test-template" src="{imports}/order_placed_match.scxml" as="OrderPlacedMatch"/>
  <sce:test-port id="sut" type="OrderPort"/>
  <datamodel>
    <data id="limit" sce:type="uint16" expr="3"/>
  </datamodel>
  <state id="watching">
    <transition sce:receive="sut" sce:test-template="OrderPlacedMatch"
                sce:template-args="limit" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports.display()
    );
    std::fs::write(dir.path().join("order_port.scxml"), port).expect("the port type is written");
    let path = dir.path().join("order_watcher.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let diagnostics = refused(&path);
    assert!(diagnostics.contains("`union:Payment`: "), "{diagnostics}");
    assert!(diagnostics.contains(NOT_LOWERED_YET), "{diagnostics}");
}
