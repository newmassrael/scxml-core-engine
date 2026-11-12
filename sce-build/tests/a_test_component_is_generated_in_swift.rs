// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in Swift (docs/SCE_TEST_PROFILE.md §11.9,
//! §12.3, §12.6 slice 6): the statechart machine of the document, each branch
//! a transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives, which binds its context on the policy and
//! names the session by the component before the machine starts. The twin of
//! `a_test_component_is_generated_in_python.rs`, over the same fixtures.
//!
//! Every unit is a file of the machine's module, so a machine names a
//! template's `Value`, `matches` and `value` through the enum its unit
//! declares and imports nothing for it. Swift holds a number, a text, a byte
//! string and an optional one of them in the same types as the record's unit,
//! and an enum in the machine's own enum, which a received message crosses
//! into by the variant's declared name, as on Kotlin, Go and Python. A module
//! parameter starts at its type's zero, or its enum's first variant, and is
//! read from the run as a saved value of its type is read back, after the
//! component binds its context and before it builds the engine. What Swift
//! refuses is held here too: a component that declares `<sce:action>`, and
//! one whose template reaches a union, which Swift does not lower.
//!
//! What the generated components do when they run is held by
//! `backends/swift/tests/Tests/StaticDatamodelTests/ATestComponentRunsThroughItsDoorsTests.swift`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_swift.sh`) and runs them through
//! `SceTestProfile.TestCaseRun`.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
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

/// Every component of the runtime's end-to-end tests, each with the name its
/// machine is spelled by: `param_asker`, which declares a module parameter, is
/// run by the suite's (`ATestSuiteRunsThroughItsRunnerTests.swift`).
const COMPONENTS: [(&str, &str); 6] = [
    ("asker", "Asker"),
    ("coordinator", "Coordinator"),
    ("answerer", "Answerer"),
    ("ptc_keeper", "PtcKeeper"),
    ("idler", "Idler"),
    ("param_asker", "ParamAsker"),
];

fn generate(path: &Path) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Swift),
        Language::Swift,
    )
    .map(|output| output.files)
}

/// The Swift files of the component `name`.
fn files(name: &str) -> Vec<(String, String)> {
    generate(&runtime_dir().join(format!("{name}.scxml")))
        .unwrap_or_else(|e| panic!("{name} generates on Swift: {e:?}"))
}

/// The whole Swift text of the component `name`.
fn text(name: &str) -> String {
    files(name)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole Swift text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    generate(path)
        .unwrap_or_else(|e| panic!("{} generates on Swift: {e:?}", path.display()))
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
        "{name} on Swift: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Swift: {other:?}"),
        },
        other => panic!("{name} on Swift: {other:?}"),
    }
}

/// The detail of the refusal by name of the document at `path`.
fn refused(path: &Path, name: &str) -> String {
    let Err(err) = generate(path) else {
        panic!("{} generated on Swift", path.display());
    };
    unsupported(name, err)
}

/// §12.3: each component is its machine's file, and beside the machine the
/// component the runtime drives: the component builds the policy, names its
/// session by the component, binds the runtime context on the policy, builds
/// the machine on it and registers the two doors before it starts it and
/// delivers the snapshot event. The policy reads the context through one
/// accessor, which stops the program where none is bound.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, machine) in COMPONENTS {
        let names: Vec<String> = files(name).into_iter().map(|(file, _)| file).collect();
        assert_eq!(names, [format!("{name}_sm.swift")]);
        let code = text(name);
        for needle in [
            "\nimport SceTestProfile\n".to_string(),
            "    fileprivate var sceTestContextBound: SceTestProfile.ComponentContext?\n"
                .to_string(),
            "    private var sceTestContext: SceTestProfile.ComponentContext {\n".to_string(),
            "preconditionFailure(\"a test component's context is bound before its machine \
             starts\")"
                .to_string(),
            format!(
                "\npublic final class {machine}Component: SceTestProfile.ComponentMachine {{\n"
            ),
            format!("    private var sceMachine: Engine<{machine}Policy>?\n"),
            "    public init() {}\n".to_string(),
            "    public func start(_ context: SceTestProfile.ComponentContext) {\n".to_string(),
            format!("        let policy = {machine}Policy()\n"),
            "            try policy.setSessionID(context.name)\n".to_string(),
            "        policy.sceTestContextBound = context\n".to_string(),
            "        let machine = Engine(policy: policy)\n".to_string(),
            "try machine.registerEventProcessor(\"http://sce.dev/ext/test-port\") { request in\n"
                .to_string(),
            "guard let sent = request.value?.value as? any SceTestProfile.TestValue else {\n"
                .to_string(),
            "preconditionFailure(\"a test-port send carries its template's value\")".to_string(),
            "context.send(request.target, SceTestProfile.Message(event: request.eventName, \
             value: sent))"
                .to_string(),
            "try machine.registerInvoker(\"http://sce.dev/ext/test-component\") { event in\n"
                .to_string(),
            "context.create(request.invokeID)".to_string(),
            "context.kill(cancel.invokeID)".to_string(),
            "    public func deliverSnapshot() {\n".to_string(),
            "        sceMachine?.processEvent(.sceTestSnapshot)\n".to_string(),
            "    public var terminated: Bool {\n        sceMachine?.isInFinalState ?? false\n"
                .to_string(),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`:\n{code}");
        }
        let component = &code[at(&code, &format!("\npublic final class {machine}Component: "))..];
        let built = at(component, &format!("let policy = {machine}Policy()"));
        let named = at(component, "try policy.setSessionID(context.name)");
        let bound = at(component, "policy.sceTestContextBound = context");
        let engine = at(component, "let machine = Engine(policy: policy)");
        let served = at(component, "try machine.registerEventProcessor(");
        let invoked = at(component, "try machine.registerInvoker(");
        let started = at(component, "\n            try machine.initialize()\n");
        let kept = at(component, "\n        sceMachine = machine\n");
        assert!(
            built < named
                && named < bound
                && bound < engine
                && engine < served
                && served < invoked
                && invoked < started
                && started < kept,
            "{name}: the policy is named and bound to the context, and the doors are \
             registered on its machine, before the machine starts:\n{component}"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks the runtime, by its module, whether the snapshot's head is of
/// its template's type under its record's event and calls the template's
/// `matches`, both named through the template unit's enum; a receive any, a
/// timeout and a done read the snapshot alone. A done branch is not taken on
/// the event it names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "SceTestProfile.receives(self.sceTestContext, \"sut\", \"job.reply\", \
         as: ReplyOk.Value.self, { (sceMessage: ReplyOk.Value) -> Bool in \
         ReplyOk.matches(sceMessage) })",
    );
    let timeout = at(&asker, "self.sceTestContext.timedOut(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("            if event == .sceTestSnapshot {\n"));
    assert!(asker.contains("// docs/SCE_TEST_PROFILE.md §12.3: a branch of the test component,\n"));

    let coordinator = text("coordinator");
    assert!(coordinator.contains("self.sceTestContext.done(\"answerer\")"));
    assert!(
        !coordinator.contains("if event == .doneInvokeAnswerer"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "SceTestProfile.receives(self.sceTestContext, \"peer\", \"job.request\", \
         as: AnyRequest.Value.self, { (sceMessage: AnyRequest.Value) -> Bool in \
         AnyRequest.matches(sceMessage) })"
    ));

    let idler = text("idler");
    assert!(idler.contains("if (self.sceTestContext.snapshotPresent(\"peer\")) {"));
}

/// A branch's guard is its `cond` and then its match, joined by `&&`, so a
/// `cond` that is false never asks the snapshot (P2: the guard is evaluated
/// before the matching), and a branch whose guard is its match alone is still
/// guarded.
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
        code.contains("if ((self.vArmed) && (self.sceTestContext.snapshotPresent(\"sut\"))) {"),
        "the match follows the cond:\n{code}"
    );
    assert!(
        code.contains("// docs/SCE_TEST_PROFILE.md §12.3: a branch of the test component,"),
        "a branch with a cond is commented as a branch:\n{code}"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch, in a
/// block of its own. A receive branch whose content reads the message holds it
/// in the payload property of its record's event, which its content reads as
/// `_event.data`, so its content needs no check of a delivery's payload. A
/// branch whose document writes no content has the statement alone as its
/// content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let read = at(
        &asker,
        "do { let sceMessage = self.sceTestContext.receive(\"sut\"); ",
    );
    let checked = at(
        &asker,
        "guard let sceValue = sceMessage.valueAs(ReplyOk.Value.self) else { \
         preconditionFailure(\"a receive branch takes the message its guard matched\") }; ",
    );
    let bound = at(
        &asker,
        "self.pendingJobReplyPayload = AskerJobReplyPayload(job: sceValue.job, \
         code: sceValue.code) }",
    );
    let content = at(&asker, "self.sceTestContext.stopTimer(\"reply\")");
    assert!(read < checked && checked < bound && bound < content);
    assert!(
        !asker.contains("if pendingPayloadTag != .jobReply {"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "self.sceTestContext.timeout(\"reply\")");
    let failed = at(&asker, "self.sceTestContext.setverdict(.fail, ");
    assert!(timeout < failed);

    assert!(text("coordinator").contains("self.sceTestContext.doneTaken(\"answerer\")"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer: each is a transition
    // with content, which the microstep runs, and no block to run after it.
    let idler = text("idler");
    assert!(
        idler.contains("hasActions: true,"),
        "the receive any branch has content: {idler}"
    );
    let taken = at(&idler, "_ = self.sceTestContext.receive(\"peer\")\n");
    let rest = &idler[taken..];
    let next_case = at(rest, "default:");
    assert!(
        !rest[..next_case].contains("sceBlock"),
        "a branch with no content of the document's has no block after its statement:\n{idler}"
    );
    assert!(text("ptc_keeper").contains("self.sceTestContext.timeout(\"patience\")"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none,
/// a verdict as the runtime's case; a send names its record's event and
/// carries its specific template's value to the test-port processor as a
/// `HostSendValue`, with the name of its record type and the unit's writer of
/// its canonical form in the test log (§12.5); a PTC is started through the
/// host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "self.sceTestContext.startTimer(\"reply\", 1000000)",
        "self.sceTestContext.stopTimer(\"reply\")",
        "self.sceTestContext.setverdict(.pass, \"\")",
        "no matching reply in time",
        "processorType: \"http://sce.dev/ext/test-port\",",
        "eventName: \"job.request\",",
        "target: \"sut\",",
        "invokeID: engine.currentEventInvokeID,\n",
        "value: HostSendValue(SceTestProfile.messageValue(RequestOf.value(self.vJob), \
         RequestOf.TYPE, { sceSent in RequestOf.canonical(sceSent) }))\n",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`:\n{asker}");
    }
    let coordinator = text("coordinator");
    for needle in [
        "value: HostSendValue(SceTestProfile.messageValue(ReplyOf.value(self.vRequested, \
         self.vOk), ReplyOf.TYPE, { sceSent in ReplyOf.canonical(sceSent) }))\n",
        "processorType: \"http://sce.dev/ext/test-component\",",
        "invokeID: \"answerer\",",
    ] {
        assert!(
            coordinator.contains(needle),
            "coordinator holds `{needle}`:\n{coordinator}"
        );
    }
    assert!(text("ptc_keeper").contains("self.sceTestContext.startTimer(\"patience\", 10000)"));
    assert!(text("idler").contains("self.sceTestContext.setverdict(.inconclusive, \"\")"));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label, a colon and its value as the engine's log
/// shows it (`String(describing:)`), and never in the engine's log; a `<log>`
/// that writes only a label is recorded too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    assert!(
        asker.contains(
            "self.sceTestContext.log(\"answered: \" + \
             String(describing: self.pendingJobReplyPayload.job))"
        ),
        "the asker logs the job of the message the branch took through the context:\n{asker}"
    );
    assert!(
        !asker.contains("print(\"answered"),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("self.sceTestContext.log(\"asked\")"));
}

/// An optional value logged by a test component is its present value, or
/// `null` where it is absent, as the other backends record it, and not
/// Swift's own `Optional(…)` or `nil`.
#[test]
fn an_optional_value_is_logged_as_its_present_value_or_null() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="noter"
       datamodel="sce-static" initial="noting">
  <sce:test-timer id="pause" duration="1ms"/>
  <datamodel>
    <data id="seen" sce:type="optional&lt;uint32&gt;" expr="null"/>
  </datamodel>
  <state id="noting">
    <onentry>
      <log label="seen" expr="seen"/>
      <sce:start-timer timer="pause"/>
    </onentry>
    <transition sce:timeout="pause" target="done">
      <sce:setverdict value="pass"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>
"#;
    let path = dir.path().join("noter.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    assert!(
        code.contains(
            "self.sceTestContext.log(\"seen: \" + ((self.vSeen).map { (scePresent) in \
             String(describing: scePresent) } ?? \"null\"))"
        ),
        "{code}"
    );
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
        code.contains(
            "value: HostSendValue(SceTestProfile.messageValue(\
             JobReportOf.value(self.vElapsed, self.vNote, self.vCode), JobReportOf.TYPE, \
             { sceSent in JobReportOf.canonical(sceSent) }))"
        ),
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

/// The machine's payload struct and the record's unit hold a text in the same
/// `String`, so a receive branch whose content reads a message of a record
/// with a text field copies it on Swift, as on Kotlin, Go and Python.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_copied_on_swift() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = card_watcher(dir.path());
    let code = text_at(&path);
    for needle in [
        "SceTestProfile.receives(self.sceTestContext, \"sut\", \"payment.card\", \
         as: CardWithin.Value.self, { (sceMessage: CardWithin.Value) -> Bool in \
         CardWithin.matches(sceMessage, self.vMost) })",
        "guard let sceValue = sceMessage.valueAs(CardWithin.Value.self) else {",
        "self.pendingPaymentCardPayload = CardWatcherPaymentCardPayload(\
         last4: sceValue.last4, amount: sceValue.amount)",
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
/// own enum and a template's unit takes the type the enum document's unit
/// declares, so a variable of an enum bound to a parameter crosses into the
/// variant of the same declared name, as a value handed to an algorithm does,
/// by an exhaustive `switch` written where the template is called.
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
            "LineOfColor.matches(sceMessage, { (sceValue: ColorWatcherColorEnum) -> PaintColor \
             in switch sceValue {"
        ),
        "the argument crosses where the template is called: {code}"
    );
    assert!(code.contains("}(self.vWanted)) })"), "{code}");
}

/// A message the content reads that holds an enum field is copied into the
/// machine's own enum by the variant's declared name, where Rust and C++
/// refuse it by name. Both enums are written from one enum document, so a
/// name one declares and the other does not stops the program.
#[test]
fn a_message_with_an_enum_field_its_content_reads_is_copied_on_swift() {
    let dir = tempfile::tempdir().expect("a directory");
    write_order_line_port(dir.path());
    write_all(dir.path(), &[("color_reader.scxml", color_reader())]);
    let code = text_at(&dir.path().join("color_reader.scxml"));
    for needle in [
        "SceTestProfile.receives(self.sceTestContext, \"sut\", \"order.line\", \
         as: OrderLineExact.Value.self, { (sceMessage: OrderLineExact.Value) -> Bool in \
         OrderLineExact.matches(sceMessage) })",
        "self.pendingOrderLinePayload = ColorReaderOrderLinePayload(sku: sceValue.sku, \
         quantity: sceValue.quantity, color: { (sceName: String) -> ColorReaderColorEnum in \
         guard let sceCrossed = ColorReaderColorEnum(declaredName: sceName) else { \
         preconditionFailure(\"the machine's enum of `color` declares every variant its \
         unit's does, \\(sceName) among them\") }; return sceCrossed \
         }(sceValue.color.declaredName)) }",
    ] {
        assert!(
            code.contains(needle),
            "color_reader holds `{needle}`:\n{code}"
        );
    }
}

/// An optional value of an enum has no crossing into a template's unit yet,
/// so a variable of one bound to a parameter is refused by name, on Swift as
/// on the other backends.
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
    let detail = refused(&dir.path().join("color_watcher.scxml"), "color_watcher");
    assert!(
        detail.contains("an optional value of the enum `Color` bound to a parameter of a test"),
        "{detail}"
    );
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's, and starts at its type's zero, which nothing
/// reads; the component reads the run's value into it, as a saved value of its
/// type is read back, after it binds the context on the policy and before it
/// builds the engine, and builds none when the read is refused. Every value is
/// read before any is written. The twin of the Go, C11 and Rust tests of the
/// same name.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    assert!(sce_build::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&Language::Swift));
    let code = text("param_asker");
    for needle in [
        "    /// docs/SCE_TEST_PROFILE.md §11.5: the module parameter `job`, which takes the \
         suite's value\n    /// before the machine is built.\n    var vJob: UInt32 = 0\n",
        "    fileprivate func sceAcceptModuleParameters(_ given: SceTestProfile.ModuleParameters) \
         throws {\n",
        "        let value0 = try SavedValues.uint32(try given.of(\"job\"), \"job\")\n",
        "        vJob = value0\n    }\n",
        "        guard context.readModuleParameters({ given in \
         try policy.sceAcceptModuleParameters(given) }) else {\n            return\n        }\n",
    ] {
        assert!(
            code.contains(needle),
            "param_asker holds `{needle}`:\n{code}"
        );
    }
    // A restore of a machine that saves reads its variables by the same names,
    // so the order is read inside the method alone.
    let accept = &code[at(&code, "fileprivate func sceAcceptModuleParameters(")..];
    let read_back = at(accept, "let value0 = try SavedValues.uint32(");
    let written = at(accept, "vJob = value0");
    assert!(
        read_back < written,
        "the value is read before it is written"
    );
    let component = &code[at(&code, "\npublic final class ParamAskerComponent: ")..];
    let bound = at(component, "policy.sceTestContextBound = context");
    let read = at(component, "guard context.readModuleParameters(");
    let built = at(component, "let machine = Engine(policy: policy)");
    let started = at(component, "\n            try machine.initialize()\n");
    assert!(
        bound < read && read < built && built < started,
        "the parameters are read after the context is bound and before the engine is built"
    );
    assert!(
        component[read..built].contains("return"),
        "a refused read leaves the engine unbuilt"
    );
    // The variable that is no module parameter starts at its `expr`.
    assert!(
        code.contains(
            "    /// W3C SCXML 5.2: the `answered` datamodel variable.\n    var vAnswered: UInt32 = 0\n"
        ),
        "{code}"
    );
    assert!(
        !text("asker").contains("sceAcceptModuleParameters"),
        "a component with no module parameter reads none"
    );
}

/// §11.8: a byte string module parameter is read back held to its bound, as a
/// restored byte string is, and a text too; each starts at the empty value,
/// which the type that holds a value of any length holds.
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
        "    var vKey: [UInt8] = []\n",
        "    var vLabel: String = \"\"\n",
        "        let value0 = try SavedValues.bytes(try given.of(\"key\"), \"key\", capacity: 2)\n",
        "        let value1 = try SavedValues.string(try given.of(\"label\"), \"label\", \
         capacity: 8)\n",
        "        vKey = value0\n        vLabel = value1\n",
    ] {
        assert!(code.contains(needle), "keyed holds `{needle}`:\n{code}");
    }
}

/// §11.8: a module parameter of an enum is read through the enum's saved
/// form, by the conformance its machine type declares beside it whether or
/// not a saved state of the machine can be written. Zero may name no variant
/// of the enum, so the variable starts at the variant its document declares
/// first, which nothing reads before the run's value replaces it.
#[test]
fn an_enum_module_parameter_is_read_through_the_enums_saved_form() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = format!(
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
    );
    let path = dir.path().join("color_picker.scxml");
    std::fs::write(&path, component).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "extension ColorPickerColorEnum: SavedEnumeration {",
        "    var vTint: ColorPickerColorEnum = ColorPickerColorEnum.red\n",
        "        let value0 = try SavedValues.enumeration(try given.of(\"tint\"), \"tint\", \
         as: ColorPickerColorEnum.self)\n",
        "        vTint = value0\n",
    ] {
        assert!(
            code.contains(needle),
            "color_picker holds `{needle}`:\n{code}"
        );
    }
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives builds its
/// machine itself, and a Swift policy of a machine that declares
/// `<sce:action>` is initialized with a host the component is given by
/// nothing, so Swift refuses such a component by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_swift() {
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
    let detail = refused(&path, "beeper");
    assert!(
        detail.starts_with("test-component 'beeper' declares <sce:action>"),
        "{detail}"
    );
    assert!(
        detail.contains("a parameter of the Swift policy's initializer"),
        "{detail}"
    );
}

/// docs/adr/0092, section 7: Swift lowers a union (`UNION_BACKENDS`), so a
/// component whose template reaches one is generated, its branch matched by the
/// template's unit, where C++, Go, Python and C11 refuse it by the union's own
/// refusal.
#[test]
fn a_component_whose_template_reaches_a_union_is_generated_on_swift() {
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
    let files = generate(&path).unwrap_or_else(|err| panic!("order_watcher on Swift: {err:?}"));
    let code: String = files.into_iter().map(|(_, text)| text).collect();
    assert!(code.contains("OrderPlacedMatch.matches("), "{code}");
}
