// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in C++ (docs/SCE_TEST_PROFILE.md §11.9, §12.3,
//! §12.6 slice 4): the statechart machine of the document, each branch a
//! transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile and a send as calls on the component's runtime context, a PTC
//! through the host invoker door; and beside the machine the component the
//! profile's runtime drives, a component whose template reaches a union among
//! them.
//!
//! What the generated components do when they run is held by
//! `tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp`, which
//! compiles them from the same fixtures and runs them through
//! `SCE::Test::TestCaseRun`.

use std::path::{Path, PathBuf};

use sce_build::forge::error::{ForgeError, Located};
use sce_build::generator::Language;

/// The directory of the components the runtime's end-to-end test runs.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The directory of the templates over imported records.
fn imports_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/imports")
}

/// Every component of the runtime's end-to-end test.
const COMPONENTS: [&str; 6] = [
    "asker",
    "coordinator",
    "answerer",
    "ptc_keeper",
    "idler",
    "param_asker",
];

fn generate(path: &Path, language: Language) -> Result<Vec<(String, String)>, Located<ForgeError>> {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(language),
        language,
    )
    .map(|output| output.files)
}

/// The C++ files of the component `name`: its machine's header and the body
/// it includes, in that order.
fn files(name: &str) -> Vec<(String, String)> {
    generate(&runtime_dir().join(format!("{name}.scxml")), Language::Cpp)
        .unwrap_or_else(|e| panic!("{name} generates on C++: {e:?}"))
}

/// The whole C++ text of the component `name`.
fn text(name: &str) -> String {
    files(name)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where `needle` first stands in `haystack`, which holds it.
fn at(haystack: &str, needle: &str) -> usize {
    haystack
        .find(needle)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}`"))
}

/// Where `needle` first stands in `haystack` at or after `from`, which holds
/// it there: a line of one branch is found in that branch, not where another
/// member writes the same text earlier in the file.
fn at_after(haystack: &str, from: usize, needle: &str) -> usize {
    haystack[from..]
        .find(needle)
        .map(|offset| from + offset)
        .unwrap_or_else(|| panic!("the generated code holds `{needle}` after byte {from}"))
}

/// §12.3: each component is its machine's header and body, and beside the
/// machine, in the header, the component the runtime drives: it binds the
/// runtime context, registers the two doors and delivers the snapshot event.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for name in COMPONENTS {
        let names: Vec<String> = files(name).into_iter().map(|(file, _)| file).collect();
        assert_eq!(names, [format!("{name}_sm.h"), format!("{name}_sm.inl")]);
        let code = text(name);
        for needle in [
            "#include \"test/TestCaseRun.h\"".to_string(),
            "::SCE::Test::ComponentContext* testContext_ = nullptr;".to_string(),
            format!("class {name}Component final : public ::SCE::Test::ComponentMachine"),
            "machine_.bindTestContext(context);".to_string(),
            "machine_.registerEventProcessor(\"http://sce.dev/ext/test-port\",".to_string(),
            "\"http://sce.dev/ext/test-component\",".to_string(),
            "machine_.processEvent(Event::Sce_test_snapshot);".to_string(),
            "return machine_.isInFinalState();".to_string(),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`");
        }
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks for its record's value under its record's event and calls
/// the template's `match`; a receive any, a timeout and a done read the
/// snapshot alone. A done branch is not taken on the event it names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "::SCE::Test::receives<::SCE::Generated::ReplyOk::Value>(*this->testContext_, \"sut\", \
         \"job.reply\", [&](const ::SCE::Generated::ReplyOk::Value &sceMessage) { return \
         ::SCE::Generated::ReplyOk::match(sceMessage); })",
    );
    let timeout = at(&asker, "this->testContext_->timedOut(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("event == Event::Sce_test_snapshot"));
    assert!(asker.contains("#include \"reply_ok.h\""));
    assert!(asker.contains("#include \"request_of.h\""));

    let coordinator = text("coordinator");
    assert!(coordinator.contains("this->testContext_->done(\"answerer\")"));
    assert!(
        !coordinator.contains("event == Event::Done_invoke_answerer"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "::SCE::Test::receives<::SCE::Generated::AnyRequest::Value>(*this->testContext_, \"peer\", \
         \"job.request\""
    ));

    let idler = text("idler");
    assert!(idler.contains("this->testContext_->snapshotOf(\"peer\") != nullptr"));
}

/// A branch's guard is its `cond` and then its match, so a `cond` that is
/// false never asks the snapshot (P2: the guard is evaluated before the
/// matching).
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
    let code = generate(&path, Language::Cpp)
        .unwrap_or_else(|e| panic!("the guarded component generates on C++: {e:?}"))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains(") && (this->testContext_->snapshotOf(\"sut\") != nullptr)"),
        "the match follows the cond"
    );
}

/// P4 step 1, §4.4, §12.3: taking a branch is the policy's `takeTransition`,
/// which the engine runs before the microstep exits any state
/// (`SCE::Core::HasTakeTransitionTrait`) and never in the content: it removes
/// what the branch matched, tells the runtime it took a branch, and holds a
/// message the content reads as `_event.data` in the payload channel, so the
/// content, after the exits, needs no check of its own.
#[test]
fn taking_a_branch_runs_before_the_exits_and_its_content_after() {
    // The transitions' bodies are in the body file; the header's payload lift
    // for an external event sets the same tag, so the order is read there.
    let asker = files("asker")
        .into_iter()
        .find(|(path, _)| path.ends_with(".inl"))
        .map(|(_, code)| code)
        .expect("asker has a body file");
    let take = at(&asker, "void takeTransition(");
    let content_fn = at(&asker, "void executeTransitionActions(");
    let receive = at(
        &asker,
        "const ::SCE::Test::Message sceMessage = this->testContext_->receive(\"sut\");",
    );
    let bound = at(&asker, "pendingJobReplyPayload_.job = sceValue.job;");
    assert!(asker.contains("pendingJobReplyPayload_.code = sceValue.code;"));
    assert!(asker.contains(
        "const auto &sceValue = sceMessage.value.get<::SCE::Generated::ReplyOk::Value>();"
    ));
    let tagged = at_after(
        &asker,
        bound,
        "pendingPayloadTag_ = AskerPayloadTag::JobReply;",
    );
    let timeout = at(&asker, "this->testContext_->timeout(\"reply\");");
    let content = at(&asker, "this->testContext_->stopTimer(\"reply\");");
    assert!(take < receive && receive < bound && bound < tagged && tagged < content_fn);
    assert!(take < timeout && timeout < content_fn);
    assert!(content_fn < content);
    assert!(
        !asker.contains("pendingPayloadTag_ != AskerPayloadTag::Sce_test_snapshot"),
        "the content reads the message the take held, not the snapshot's event"
    );
    let failed = at(
        &asker,
        "this->testContext_->setverdict(::SCE::Test::Verdict::Fail,",
    );
    assert!(content_fn < failed);

    assert!(text("coordinator").contains("this->testContext_->doneTaken(\"answerer\");"));
    // A receive any branch with no content still takes its message.
    assert!(text("idler").contains("this->testContext_->receive(\"peer\");"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and carries its specific template's value
/// to the test-port processor; a PTC is started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "this->testContext_->startTimer(\"reply\", ::SCE::Test::Duration{1000000});",
        "this->testContext_->stopTimer(\"reply\");",
        "this->testContext_->setverdict(::SCE::Test::Verdict::Pass, ::std::string());",
        "no matching reply in time",
        "hostRequest.processorType = \"http://sce.dev/ext/test-port\";",
        "hostRequest.eventName = \"job.request\";",
        "hostRequest.target = \"sut\";",
        "hostRequest.value = ::SCE::Test::messageValue(::SCE::Generated::RequestOf::value(v_job), \
         ::SCE::Generated::RequestOf::TYPE, ::SCE::Generated::RequestOf::canonical);",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`");
    }
    let coordinator = text("coordinator");
    for needle in [
        "hostRequest.value = ::SCE::Test::messageValue(\
         ::SCE::Generated::ReplyOf::value(v_requested, v_ok), ::SCE::Generated::ReplyOf::TYPE, \
         ::SCE::Generated::ReplyOf::canonical);",
        "hostInvoke.processorType = \"http://sce.dev/ext/test-component\";",
        "hostInvoke.invokeId = \"answerer\";",
    ] {
        assert!(coordinator.contains(needle), "coordinator holds `{needle}`");
    }
    assert!(text("ptc_keeper")
        .contains("this->testContext_->startTimer(\"patience\", ::SCE::Test::Duration{10000});"));
}

/// P8, §12.3: a test component's `<log>` is recorded in the test log through
/// its runtime context, its label and its value as the engine's log shows
/// them, and never in the engine's log; a `<log>` that writes only a label is
/// recorded too.
#[test]
fn a_log_is_recorded_in_the_test_log() {
    let asker = text("asker");
    assert!(
        asker.contains(
            "this->testContext_->log(::std::string(\"answered: \") + \
             ::SCE::Forge::presentText(sceLogName(pendingJobReplyPayload_.job)));"
        ),
        "the log reads the message the branch took"
    );
    assert!(
        !asker.contains("SCE_LOG_INFO(\"{}: {}\", \"answered\""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("this->testContext_->log(::std::string(\"asked\"));"));
}

/// P8, §12.5: a component that declares no variable and logs as a state exits
/// (`boundary/leaving_logger`) records the log through its runtime context
/// from its exit actions, which are the policy's own method and not a static
/// one, so the onexit block's lambda reaches the context through `this`.
#[test]
fn a_log_as_a_state_exits_is_recorded_from_the_policys_own_exit_actions() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/boundary/leaving_logger.scxml");
    let code: String = generate(&path, Language::Cpp)
        .unwrap_or_else(|e| panic!("leaving_logger generates on C++: {e:?}"))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !code.contains("static void executeExitActions("),
        "the exit actions reach `this`:\n{code}"
    );
    let exits = at(&code, "SCE_HOT void executeExitActions(");
    let logged = at(
        &code,
        "this->testContext_->log(::std::string(\"leaving waiting\"));",
    );
    assert!(exits < logged, "{code}");
}

/// The text of the file of the component `name` whose name ends in `suffix`.
fn file_of(name: &str, suffix: &str) -> String {
    files(name)
        .into_iter()
        .find(|(path, _)| path.ends_with(suffix))
        .map(|(_, code)| code)
        .unwrap_or_else(|| panic!("{name} has a file ending in {suffix}"))
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's; the component reads the run's value into it, as
/// a saved value of its type is read back, before the machine starts, and does
/// not start the machine when the read is refused.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    let header = file_of("param_asker", "_sm.h");
    for needle in [
        "v_job {};",
        "void acceptModuleParameters(const ::SCE::Test::ModuleParameters& given) {",
        "this->policy_.v_job = ::SCE::Saved::savedReadUint32(given.of(\"job\"), \"job\");",
        "[this](const ::SCE::Test::ModuleParameters& given) { machine_.acceptModuleParameters(given); }",
    ] {
        assert!(header.contains(needle), "param_asker holds `{needle}`");
    }
    let read = at(&header, "if (!context.readModuleParameters(");
    let bound = at(&header, "machine_.bindTestContext(context);");
    let started = at(&header, "static_cast<void>(machine_.initialize());");
    assert!(
        bound < read && read < started,
        "the parameters are read after the context is bound and before the machine starts"
    );
    // The variable that is no module parameter starts at its `expr`.
    assert!(!header.contains("v_answered {};"), "{header}");
    assert!(
        !text("asker").contains("acceptModuleParameters"),
        "a component with no module parameter reads none"
    );
}

/// docs/adr/0045: a component's session is named by the component, `mtc` or
/// the PTC's id, as on every other backend: the machine is given the context's
/// name as the id a host gives a root, before the context is bound and the
/// machine started, and every component's machine takes one, whether or not a
/// saved state can hold it. What the names are when the suite runs is held by
/// `tests/integration/ATestSuiteRunsEndToEndTest.cpp`
/// (`EachComponentRunsUnderItsConfigurationName`).
#[test]
fn a_component_names_its_session_after_itself() {
    for name in COMPONENTS {
        let header = file_of(name, "_sm.h");
        let named = at(&header, "machine_.bindSessionId(context.name());");
        let bound = at(&header, "machine_.bindTestContext(context);");
        let started = at(&header, "static_cast<void>(machine_.initialize());");
        assert!(
            named < bound && bound < started,
            "{name}: the session is named before the context is bound and the machine starts"
        );
        for needle in [
            "    void bindSessionId(std::string id) {\n",
            "    std::string sessionId() const {\n        return machine_.sessionId();\n    }\n",
            "#include \"saved/SessionId.h\"",
        ] {
            assert!(header.contains(needle), "{name} holds `{needle}`");
        }
    }
}

/// docs/adr/0092, section 7: a component whose template reaches a union is
/// generated on C++, whose test profile matches a union
/// (`codegen_matrix::TEST_PROFILE_UNION_BACKENDS`): its branch matches through
/// the template's unit, as one over a record of entries does.
#[test]
fn a_component_whose_template_reaches_a_union_is_generated_on_cpp() {
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
    let code = generate(&path, Language::Cpp)
        .unwrap_or_else(|e| panic!("order_watcher generates on C++: {e:?}"))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("::SCE::Generated::OrderPlacedMatch::match(sceMessage, "),
        "{code}"
    );
    assert!(code.contains("#include \"order_placed_match.h\""), "{code}");
}

/// docs/SCE_TEST_PROFILE.md §12.3, §12.6: a receive branch whose content reads
/// a message of a record that holds a composition (`job_status`: a list, a
/// text that may be absent, numbers that may be absent) copies it into the
/// machine's payload member by member, each made from the value the record's
/// unit holds, which holds each in the runtime's types the machine does, so
/// each is copied whole.
#[test]
fn a_composed_message_its_content_reads_is_bound_in_the_machines_types_on_cpp() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/job_tester.scxml");
    let code: String = generate(&path, Language::Cpp)
        .unwrap_or_else(|e| panic!("job_tester generates on C++: {e:?}"))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    let read = at(
        &code,
        "const auto &sceValue = sceMessage.value.get<::SCE::Generated::EveryMatcher::Value>();",
    );
    // The header's payload setters set the same tag earlier in the file, so
    // the copies and the tag are read in the receive branch, after its read.
    let first = at_after(
        &code,
        read,
        "pendingJobStatusPayload_.elapsed_ms = sceValue.elapsed_ms;",
    );
    let tagged = at_after(
        &code,
        first,
        "pendingPayloadTag_ = JobTesterPayloadTag::JobStatus;",
    );
    assert!(read < first && first < tagged, "{code}");
    for copy in [
        "pendingJobStatusPayload_.note = sceValue.note;",
        "pendingJobStatusPayload_.codes = sceValue.codes;",
        "pendingJobStatusPayload_.tag = sceValue.tag;",
        "pendingJobStatusPayload_.retry = sceValue.retry;",
        "pendingJobStatusPayload_.count = sceValue.count;",
    ] {
        let copied = at_after(&code, first, copy);
        assert!(
            copied < tagged,
            "{copy} is copied before the tag is set:\n{code}"
        );
    }
}
