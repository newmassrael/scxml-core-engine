// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in Kotlin (docs/SCE_TEST_PROFILE.md §11.9,
//! §12.3, §12.6 slice 6): the statechart machine of the document, each branch
//! a transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives, which constructs the machine with its context.
//! The twin of `a_test_component_is_generated_in_rust.rs`, over the same
//! fixtures.
//!
//! A module parameter is read from the run before the machine starts, as on
//! Rust and C++. Kotlin holds a text, a byte string and an enum of a received
//! message where the record's unit holds them, so it copies a message the
//! content reads that Rust refuses. What Kotlin refuses is held here too: a
//! component that declares `<sce:action>`.
//!
//! What the generated components do when they run is held by
//! `backends/kotlin/tests/src/test/kotlin/com/sce/integration/ATestComponentRunsThroughItsDoorsTest.kt`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_kotlin.sh`) and runs them through
//! `com.sce.runtime.test.TestCaseRun`.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use sce_build::generator::Language;

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

/// Every component of the runtime's end-to-end test that Kotlin generates,
/// each with the name its machine is spelled by.
const COMPONENTS: [(&str, &str); 6] = [
    ("asker", "Asker"),
    ("coordinator", "Coordinator"),
    ("answerer", "Answerer"),
    ("ptc_keeper", "PtcKeeper"),
    ("idler", "Idler"),
    ("param_asker", "ParamAsker"),
];

fn generate(path: &Path, language: Language) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(language),
        language,
    )
    .map(|output| output.files)
}

/// The Kotlin files of the component `name`.
fn files(name: &str) -> Vec<(String, String)> {
    generate(
        &runtime_dir().join(format!("{name}.scxml")),
        Language::Kotlin,
    )
    .unwrap_or_else(|e| panic!("{name} generates on Kotlin: {e:?}"))
}

/// The whole Kotlin text of the component `name`.
fn text(name: &str) -> String {
    files(name)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole Kotlin text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    generate(path, Language::Kotlin)
        .unwrap_or_else(|e| panic!("{} generates on Kotlin: {e:?}", path.display()))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`:\n{haystack}"))
}

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on Kotlin: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Kotlin: {other:?}"),
        },
        other => panic!("{name} on Kotlin: {other:?}"),
    }
}

/// §12.3: each component is its machine's file, and beside the machine the
/// component the runtime drives: the machine is constructed with the runtime
/// context, under a session the component names, and the component registers
/// the two doors on it before it starts it and delivers the snapshot event.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, machine) in COMPONENTS {
        let names: Vec<String> = files(name).into_iter().map(|(file, _)| file).collect();
        assert_eq!(names, [format!("{name}Sm.kt")]);
        let code = text(name);
        for needle in [
            "private val sceTestContext: com.sce.runtime.test.ComponentContext,".to_string(),
            format!("class {machine}Component : com.sce.runtime.test.ComponentMachine {{"),
            format!("private var sceMachine: {machine}StateMachine? = null"),
            "override fun start(context: com.sce.runtime.test.ComponentContext) {".to_string(),
            format!(
                "val machine = {machine}StateMachine(sceTestContext = context, sessionId = \
                 context.name)"
            ),
            "machine.registerEventProcessor(\"http://sce.dev/ext/test-port\") { request ->"
                .to_string(),
            "checkNotNull(request.value) { \"a test-port send carries its template's value\" }"
                .to_string(),
            "machine.registerInvoker(\"http://sce.dev/ext/test-component\") { event ->".to_string(),
            "event.start?.let { context.create(it.invokeId) }".to_string(),
            "event.cancel?.let { context.kill(it.invokeId) }".to_string(),
            format!("machine.send({machine}Event.Sce.Test.Snapshot)"),
            "machine.tick()".to_string(),
            "override fun terminated(): Boolean = sceMachine?.isInFinalState == true".to_string(),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`");
        }
        let built = at(&code, &format!("val machine = {machine}StateMachine("));
        let served = at(&code, "machine.registerEventProcessor(");
        let invoked = at(&code, "machine.registerInvoker(");
        let started = at(&code, "machine.initialize()");
        assert!(
            built < served && served < invoked && invoked < started,
            "{name}: the doors are registered on the machine before it starts"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks for its record's value under its record's event and calls
/// the template's `matches`, both named by the template unit's package; a
/// receive any, a timeout and a done read the snapshot alone. A done branch is
/// not taken on the event it names, and no template's package is imported.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "com.sce.runtime.test.receives<com.sce.generated.reply_ok.Value>(sceTestContext, \"sut\", \
         \"job.reply\", { sceMessage -> com.sce.generated.reply_ok.matches(sceMessage) })",
    );
    let timeout = at(&asker, "sceTestContext.timedOut(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("event is AskerEvent.Sce.Test.Snapshot"));
    for unit in ["reply_ok", "request_of"] {
        assert!(
            !asker.contains(&format!("import com.sce.generated.{unit}")),
            "the unit of {unit} is named by its package, not imported"
        );
    }

    let coordinator = text("coordinator");
    assert!(coordinator.contains("sceTestContext.done(\"answerer\")"));
    assert!(
        !coordinator.contains("event is CoordinatorEvent.Done.Invoke.Answerer"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "com.sce.runtime.test.receives<com.sce.generated.any_request.Value>(sceTestContext, \
         \"peer\", \"job.request\""
    ));

    let idler = text("idler");
    assert!(idler.contains("sceTestContext.snapshotPresent(\"peer\")"));
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
        code.contains(") && (sceTestContext.snapshotPresent(\"sut\"))"),
        "the match follows the cond:\n{code}"
    );
    assert!(
        text("idler").contains(
            "event is IdlerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent(\"peer\")"
        ),
        "a branch with no cond is guarded by its match"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch. A
/// receive branch whose content reads the message holds it in the payload
/// member of its record's event, which its content reads as `_event.data`, so
/// its content needs no check of a delivery's payload. A branch whose document
/// writes no content still has the statement as its content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let receive = at(&asker, "val sceMessage = sceTestContext.receive(\"sut\")");
    let read = at(
        &asker,
        "val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.reply_ok.Value>()) {",
    );
    let bound = at(
        &asker,
        "pendingJobReplyPayload = AskerJobReplyPayload(job = sceValue.job, code = sceValue.code)",
    );
    let content = at(&asker, "sceTestContext.stopTimer(\"reply\")");
    assert!(receive < read && read < bound && bound < content);
    assert!(
        !asker.contains("if (pendingJobReplyPayload == null) {"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "sceTestContext.timeout(\"reply\")");
    let failed = at(
        &asker,
        "sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, ",
    );
    assert!(timeout < failed);

    assert!(text("coordinator").contains("sceTestContext.doneTaken(\"answerer\")"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer: each is a transition
    // with content, which the microstep runs.
    let idler = text("idler");
    assert!(idler.contains("sceTestContext.receive(\"peer\")"));
    assert!(idler.contains("hasActions = true,"), "{idler}");
    assert!(text("ptc_keeper").contains("sceTestContext.timeout(\"patience\")"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and carries its specific template's value
/// to the test-port processor; a PTC is started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "sceTestContext.startTimer(\"reply\", 1000000L)",
        "sceTestContext.stopTimer(\"reply\")",
        "sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, \"\")",
        "no matching reply in time",
        "processorType = \"http://sce.dev/ext/test-port\",",
        "eventName = \"job.request\",",
        "target = \"sut\",",
        "invokeId = currentEventMetadata.invokeId,",
        "value = com.sce.generated.request_of.value(vJob)",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`");
    }
    let coordinator = text("coordinator");
    for needle in [
        "value = com.sce.generated.reply_of.value(vRequested, vOk)",
        "processorType = \"http://sce.dev/ext/test-component\",",
        "invokeId = \"answerer\",",
    ] {
        assert!(coordinator.contains(needle), "coordinator holds `{needle}`");
    }
    assert!(text("ptc_keeper").contains("sceTestContext.startTimer(\"patience\", 10000L)"));
    assert!(text("idler")
        .contains("sceTestContext.setverdict(com.sce.runtime.test.Verdict.INCONCLUSIVE, \"\")"));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label and its value as the engine's log shows
/// them, and never in the engine's log; a `<log>` that writes only a label is
/// recorded too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    let logged = asker
        .lines()
        .find(|line| line.contains("sceTestContext.log(\"answered: \" + ("))
        .unwrap_or_else(|| panic!("the asker logs its label through the context:\n{asker}"));
    assert!(
        logged.contains("pendingJobReplyPayload!!") && logged.contains(".job"),
        "the log reads the message the branch took: {logged}"
    );
    assert!(
        logged.trim_end().ends_with(").toString())"),
        "the value is shown as the engine's log shows it: {logged}"
    );
    assert!(
        !asker.contains("println(\"answered: \""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("sceTestContext.log(\"asked\")"));
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
        code.contains("value = com.sce.generated.job_report_of.value(vElapsed, vNote, vCode)"),
        "{code}"
    );
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

/// The machine's payload data class and the record's unit hold a text in the
/// same `String`, so a receive branch whose content reads a message of a
/// record with a text field copies it on Kotlin, where Rust, whose two records
/// spell a text otherwise, refuses it by name.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_copied_on_kotlin() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = card_watcher(dir.path());
    let code = text_at(&path);
    for needle in [
        "com.sce.runtime.test.receives<com.sce.generated.card_within.Value>(sceTestContext, \
         \"sut\", \"payment.card\", { sceMessage -> \
         com.sce.generated.card_within.matches(sceMessage, vMost) })",
        "val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.card_within.Value>()) {",
        "pendingPaymentCardPayload = CardWatcherPaymentCardPayload(last4 = sceValue.last4, \
         amount = sceValue.amount)",
    ] {
        assert!(
            code.contains(needle),
            "card_watcher holds `{needle}`:\n{code}"
        );
    }
    let Err(err) = generate(&path, Language::Rust) else {
        panic!("a branch reading a message with a text field generated on Rust");
    };
    let refusal = format!("{err:?}");
    assert!(
        refusal.contains("taking a branch of a test component has no Rust lowering yet"),
        "{refusal}"
    );
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
/// own enum class and a template's unit takes the class the enum document's
/// unit declares, so a variable of an enum bound to a parameter crosses into
/// the variant of the same declared name, as a value handed to an algorithm
/// does, and the machine imports the enum's unit, whose class the crossing
/// names.
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
    let path = dir.path().join("color_watcher.scxml");
    let code = text_at(&path);
    assert!(
        code.contains("com.sce.generated.line_of_color.matches(sceMessage, when (vWanted) {"),
        "the argument crosses where the template is called: {code}"
    );
    assert!(code.contains("-> PaintColor."), "{code}");
    assert!(
        code.contains("import com.sce.generated.paint_color.*"),
        "{code}"
    );
}

/// A message the content reads that holds an enum field is copied into the
/// machine's own enum class by the variant's declared name, as the payload
/// channel lifts a delivery's, where Rust and C++ refuse it by name.
#[test]
fn a_message_with_an_enum_field_its_content_reads_is_copied_on_kotlin() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(dir.path(), &[("color_reader.scxml", color_reader())]);
    let path = dir.path().join("color_reader.scxml");
    let code = text_at(&path);
    assert!(
        code.contains(
            "com.sce.runtime.test.receives<com.sce.generated.order_line_exact.Value>(\
             sceTestContext, \"sut\", \"order.line\", { sceMessage -> \
             com.sce.generated.order_line_exact.matches(sceMessage) })"
        ),
        "{code}"
    );
    assert!(
        code.contains(
            "pendingOrderLinePayload = ColorReaderOrderLinePayload(sku = sceValue.sku, \
             quantity = sceValue.quantity, color = ColorReaderColorEnum.entries.first { \
             it.declaredName == sceValue.color.declaredName() })"
        ),
        "{code}"
    );
    for language in [Language::Rust, Language::Cpp] {
        if generate(&path, language).is_ok() {
            panic!("a branch reading a message with an enum field generated on {language:?}");
        }
    }
}

/// An optional value of an enum has no crossing into a template's unit yet,
/// so a variable of one bound to a parameter is refused by name, on Kotlin as
/// on Rust.
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
    let Err(err) = generate(&dir.path().join("color_watcher.scxml"), Language::Kotlin) else {
        panic!("an optional enum argument generated on Kotlin");
    };
    let detail = unsupported("color_watcher", err);
    assert!(
        detail.contains("an optional value of the enum `Color` bound to a parameter of a test"),
        "{detail}"
    );
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's, at its type's zero, which nothing reads; the
/// component reads the run's value into it, as a saved value of its type is
/// read back, after the machine is constructed with the context and before
/// any door is registered on it or it starts, and does not start the machine
/// when the read is refused. The twin of the Rust test of the same name.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    assert!(sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&Language::Kotlin));
    let code = text("param_asker");
    for needle in [
        "* docs/SCE_TEST_PROFILE.md §11.5: the module parameter `job`, which takes the",
        "private var vJob: UInt = 0u",
        "internal fun acceptModuleParameters(given: com.sce.runtime.test.ModuleParameters) {",
        "acceptModuleParametersPart0(given)",
        "private fun acceptModuleParametersPart0(given: com.sce.runtime.test.ModuleParameters) {",
        "vJob = SavedValues.uint32(given.of(\"job\"), \"job\")",
    ] {
        assert!(
            code.contains(needle),
            "param_asker holds `{needle}`:\n{code}"
        );
    }
    let built = at(&code, "val machine = ParamAskerStateMachine(");
    let read = at(
        &code,
        "if (!context.readModuleParameters { given -> machine.acceptModuleParameters(given) }) {",
    );
    let served = at(&code, "machine.registerEventProcessor(");
    let started = at(&code, "machine.initialize()");
    assert!(
        built < read && read < served && served < started,
        "the parameters are read after the machine is built and before it starts"
    );
    assert!(
        code[read..served].contains("return"),
        "a refused read leaves the machine unstarted"
    );
    // The variable that is no module parameter starts at its `expr`.
    assert!(
        code.contains("private var vAnswered: UInt = 0.toUInt()"),
        "{code}"
    );
    assert!(
        !text("asker").contains("acceptModuleParameters"),
        "a component with no module parameter reads none"
    );
}

/// §11.8: a byte string module parameter is read back held to its bound, as a
/// restored byte string is, and a text by the reader of its bounded type; each
/// starts at its type's zero.
#[test]
fn a_bounded_module_parameter_is_read_back_held_to_its_bound() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="keyed"
       datamodel="sce-static" initial="checking">
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="key" sce:type="bytes[2]" sce:module-parameter="true"/>
    <data id="label" sce:type="string[8]" sce:module-parameter="true"/>
  </datamodel>
  <state id="checking">
    <onentry>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;
    let path = dir.path().join("keyed.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "private var vKey: ByteArray = byteArrayOf()",
        "private var vLabel: String = \"\"",
        "vKey = SavedValues.bytes(given.of(\"key\"), \"key\", 2)",
        "vLabel = SavedValues.string(given.of(\"label\"), \"label\", 8)",
    ] {
        assert!(code.contains(needle), "keyed holds `{needle}`:\n{code}");
    }
}

/// A component whose module parameter `tint` is of the enum `Color`, and
/// whose state waits a millisecond before it passes.
fn color_picker() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="color_picker"
       datamodel="sce-static" initial="picking">
  <sce:import kind="enum" src="{imports}/paint_color.scxml" as="Color"/>
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="tint" sce:type="enum:Color" sce:module-parameter="true"/>
  </datamodel>
  <state id="picking">
    <onentry>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#,
        imports = imports_dir().display()
    )
}

/// §11.8: a module parameter of an enum is read through the enum's saved
/// form, the `fromSaved` its machine type declares whether or not a saved
/// state of the machine can be written. The machine's enum type has no zero,
/// so the variable starts at the variant its document declares first, which
/// nothing reads before the run's value replaces it.
#[test]
fn an_enum_module_parameter_is_read_through_the_enums_saved_form() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = dir.path().join("color_picker.scxml");
    std::fs::write(&path, color_picker()).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "fun fromSaved(value: Any?, what: String): ColorPickerColorEnum {",
        "private var vTint: ColorPickerColorEnum = ColorPickerColorEnum.RED",
        "vTint = ColorPickerColorEnum.fromSaved(given.of(\"tint\"), \"tint\")",
    ] {
        assert!(
            code.contains(needle),
            "color_picker holds `{needle}`:\n{code}"
        );
    }
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives builds its
/// machine itself, and a Kotlin machine that declares `<sce:action>` is
/// constructed with a host the component is given by nothing, so Kotlin
/// refuses such a component by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_kotlin() {
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
    let Err(err) = generate(&path, Language::Kotlin) else {
        panic!("a component that declares <sce:action> generated on Kotlin");
    };
    let detail = unsupported("beeper", err);
    assert!(
        detail.starts_with("test-component 'beeper' declares <sce:action>"),
        "{detail}"
    );
    assert!(
        detail.contains("a constructor parameter of the Kotlin machine"),
        "{detail}"
    );
}

/// The component constructs its machine under the session id the component's
/// name gives, which a machine takes only when it has a saved shape. What
/// would leave a component's machine with none is refused by the profile
/// itself: a delayed send, refused here, and an invoke a restore cannot start.
#[test]
fn a_delayed_send_that_would_leave_no_saved_shape_is_refused() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="delayer"
       datamodel="sce-static" initial="sending">
  <sce:import kind="test-port-type" src="{runtime}/exchange_port.scxml" as="ExchangePort"/>
  <sce:test-port id="sut" type="ExchangePort"/>
  <sce:test-timer id="pause" duration="1ms"/>
  <state id="sending">
    <onentry>
      <send type="http://sce.dev/ext/test-port" target="sut" delay="1s"/>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("delayer.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let Err(err) = generate(&path, Language::Kotlin) else {
        panic!("a delayed send of a test component generated on Kotlin");
    };
    let refusal = format!("{err:?}");
    assert!(
        refusal.contains("<send> writes delay") && refusal.contains("a send has no delay"),
        "{refusal}"
    );
}

/// The component names its session by the component's name, `mtc` or the id
/// of the `<invoke>` that creates it, which the runtime's session id holds to
/// the Nmtoken rule. An `<invoke id>` that is not an XML Name is refused where
/// the document is parsed, so no generated component creates a PTC under a
/// name its session cannot take.
#[test]
fn an_invoke_id_that_is_no_xml_name_is_refused_where_it_is_parsed() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="spawner"
       datamodel="sce-static" initial="spawning">
  <sce:test-timer id="pause" duration="1ms"/>
  <state id="spawning">
    <invoke type="http://sce.dev/ext/test-component" src="{runtime}/idler.scxml" id="no peer"/>
    <onentry>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let path = dir.path().join("spawner.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let Err(err) = generate(&path, Language::Kotlin) else {
        panic!("an <invoke id> that is no XML Name generated on Kotlin");
    };
    match err.error {
        ForgeError::Validation(boxed) => assert!(
            matches!(&*boxed, ValidationError::MalformedIdentifier { attr, .. } if attr == "id"),
            "{boxed:?}"
        ),
        other => panic!("refused at the parse: {other:?}"),
    }
}
