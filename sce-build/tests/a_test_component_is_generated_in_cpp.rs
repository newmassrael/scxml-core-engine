// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in C++ (docs/SCE_TEST_PROFILE.md §11.9, §12.3,
//! §12.6 slice 4): the statechart machine of the document, each branch a
//! transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile and a send as calls on the component's runtime context, a PTC
//! through the host invoker door; and beside the machine the component the
//! profile's runtime drives. A component whose template reaches a union is
//! refused on C++, and every backend that has not landed a component refuses
//! one by name.
//!
//! What the generated components do when they run is held by
//! `tests/integration/ATestComponentRunsThroughItsDoorsTest.cpp`, which
//! compiles them from the same fixtures and runs them through
//! `SCE::Test::TestCaseRun`.

use std::path::{Path, PathBuf};

use sce_build::forge::codegen_matrix::TEST_COMPONENT_BACKENDS;
use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, GenerateError, Located};
use sce_build::forge::test_profile::NOT_GENERATED_YET;
use sce_build::forge::union_check::NOT_LOWERED_YET;
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

/// The detail of a `generate/unsupported-feature` refusal.
fn unsupported(name: &str, language: Language, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on {language:?}: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on {language:?}: {other:?}"),
        },
        other => panic!("{name} on {language:?}: {other:?}"),
    }
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

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch. A
/// receive branch whose content reads the message holds it as `_event.data`
/// in the payload channel, so its content needs no check of its own.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    // The transitions' bodies are in the body file; the header's payload lift
    // for an external event sets the same tag, so the order is read there.
    let asker = files("asker")
        .into_iter()
        .find(|(path, _)| path.ends_with(".inl"))
        .map(|(_, code)| code)
        .expect("asker has a body file");
    let receive = at(
        &asker,
        "const ::SCE::Test::Message sceMessage = this->testContext_->receive(\"sut\");",
    );
    let bound = at(&asker, "pendingJobReplyPayload_.job = sceValue.job;");
    assert!(asker.contains("pendingJobReplyPayload_.code = sceValue.code;"));
    assert!(asker.contains(
        "const auto &sceValue = ::std::any_cast<const ::SCE::Generated::ReplyOk::Value &>(sceMessage.value);"
    ));
    let tagged = at(&asker, "pendingPayloadTag_ = AskerPayloadTag::JobReply;");
    let content = at(&asker, "this->testContext_->stopTimer(\"reply\");");
    assert!(receive < bound && bound < tagged && tagged < content);
    assert!(
        !asker.contains("pendingPayloadTag_ != AskerPayloadTag::Sce_test_snapshot"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "this->testContext_->timeout(\"reply\");");
    let failed = at(
        &asker,
        "this->testContext_->setverdict(::SCE::Test::Verdict::Fail,",
    );
    assert!(timeout < failed);

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
        "hostRequest.value = ::SCE::Generated::RequestOf::value(v_job);",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`");
    }
    let coordinator = text("coordinator");
    for needle in [
        "hostRequest.value = ::SCE::Generated::ReplyOf::value(v_job, v_ok);",
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
    let started = at(&header, "machine_.initialize();");
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

/// docs/adr/0092, section 7: a component whose template reaches a union is
/// refused on C++, which generates no unit of that template, by the union's
/// own refusal.
#[test]
fn a_component_whose_template_reaches_a_union_is_refused_on_cpp() {
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
    let Err(err) = generate(&path, Language::Cpp) else {
        panic!("a component whose template reaches a union generated on C++");
    };
    let detail = unsupported("order_watcher", Language::Cpp, err);
    assert!(detail.contains("`union:Payment`: "), "{detail}");
    assert!(detail.contains(NOT_LOWERED_YET), "{detail}");
}

/// Every backend that does not generate a test component refuses one by
/// name, until the profile's runtime lands there; Rust's is held by
/// `a_test_component_is_generated_in_rust.rs`, Kotlin's by
/// `a_test_component_is_generated_in_kotlin.rs`, Go's by
/// `a_test_component_is_generated_in_go.rs`, Python's by
/// `a_test_component_is_generated_in_python.rs` and C11's by
/// `a_test_component_is_generated_in_c11.rs`.
#[test]
fn every_other_backend_refuses_a_test_component_by_name() {
    let path = runtime_dir().join("asker.scxml");
    for &language in Language::ALL {
        if TEST_COMPONENT_BACKENDS.contains(&language) {
            continue;
        }
        let Err(err) = generate(&path, language) else {
            panic!("a test component generated on {language:?}");
        };
        let detail = unsupported("asker", language, err);
        assert!(detail.starts_with("test-component 'asker': "), "{detail}");
        assert!(detail.contains(NOT_GENERATED_YET), "{detail}");
    }
}
