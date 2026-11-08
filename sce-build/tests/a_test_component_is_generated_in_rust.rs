// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component generated in Rust (docs/SCE_TEST_PROFILE.md §11.9, §12.3,
//! §12.6 slice 6): the statechart machine of the document, each branch a
//! transition on the round's snapshot event guarded by its `cond` and its
//! match, the statement taking it before its content, the statements of the
//! profile, a `<log>` and a send as calls on the component's runtime context,
//! a PTC through the host invoker door; and beside the machine the component
//! the profile's runtime drives. The twin of
//! `a_test_component_is_generated_in_cpp.rs`, over the same fixtures.
//!
//! A module parameter is read from the run before the machine starts, as on
//! C++. What Rust refuses that C++ does not is held here too: a receive branch
//! whose content reads a message of a record the machine spells otherwise than
//! the record's unit.
//!
//! What the generated components do when they run is held by
//! `backends/rust/tests/tests/a_test_component_runs_through_its_doors.rs`,
//! which compiles them from the same fixtures (regen:
//! `scripts/regen_static_datamodel_rust.sh`) and runs them through
//! `sce_rust_runtime::test::TestCaseRun`.

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

/// Every component of the runtime's end-to-end test that Rust generates, each
/// with the name its machine is spelled by.
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

/// The Rust files of the component `name`.
fn files(name: &str) -> Vec<(String, String)> {
    generate(&runtime_dir().join(format!("{name}.scxml")), Language::Rust)
        .unwrap_or_else(|e| panic!("{name} generates on Rust: {e:?}"))
}

/// The whole Rust text of the component `name`.
fn text(name: &str) -> String {
    files(name)
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole Rust text of the document at `path`, which generates.
fn text_at(path: &Path) -> String {
    generate(path, Language::Rust)
        .unwrap_or_else(|e| panic!("{} generates on Rust: {e:?}", path.display()))
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
fn unsupported(name: &str, err: Located<ForgeError>) -> String {
    assert!(
        matches!(
            err.error.to_diagnostics()[0].code,
            DiagnosticCode::GenerateUnsupportedFeature
        ),
        "{name} on Rust: {err:?}"
    );
    match err.error {
        ForgeError::Generate(boxed) => match *boxed {
            GenerateError::UnsupportedFeature { detail, .. } => detail,
            other => panic!("{name} on Rust: {other:?}"),
        },
        other => panic!("{name} on Rust: {other:?}"),
    }
}

/// §12.3: each component is its machine's file, and beside the machine the
/// component the runtime drives: the policy holds the runtime context, which
/// the component binds before the machine starts; it registers the two doors
/// and delivers the snapshot event.
#[test]
fn each_component_is_its_machine_and_the_component_the_runtime_drives() {
    for (name, machine) in COMPONENTS {
        let names: Vec<String> = files(name).into_iter().map(|(file, _)| file).collect();
        assert_eq!(names, [format!("{name}_sm.rs")]);
        let code = text(name);
        for needle in [
            "test_context: Option<::sce_rust_runtime::test::ComponentContext>,".to_string(),
            "test_context: None,".to_string(),
            "fn test_context(&self) -> &::sce_rust_runtime::test::ComponentContext {".to_string(),
            format!("pub struct {machine}Component {{"),
            format!("machine: Option<Engine<{machine}Policy>>,"),
            format!("impl ::sce_rust_runtime::test::ComponentMachine for {machine}Component {{"),
            format!("{machine}Policy::new_with_session_id(context.name())"),
            "policy.test_context = Some(context.clone());".to_string(),
            "\"http://sce.dev/ext/test-port\",".to_string(),
            "\"http://sce.dev/ext/test-component\",".to_string(),
            format!("machine.process_event({machine}Event::SceTestSnapshot);"),
            ".is_some_and(|machine| machine.is_in_final_state())".to_string(),
        ] {
            assert!(code.contains(&needle), "{name} holds `{needle}`");
        }
        let bound = at(&code, "policy.test_context = Some(context.clone());");
        assert!(
            code[bound..].contains("machine.initialize();"),
            "{name}: the context is bound before the machine starts"
        );
    }
}

/// P2, P3, §12.3: every branch is a transition on the snapshot event, tried in
/// its document place, its guard the match of its kind: a receive with a
/// template asks for its record's value under its record's event and calls
/// the template's `matches`; a receive any, a timeout and a done read the
/// snapshot alone. A done branch is not taken on the event it names.
#[test]
fn each_branch_is_a_transition_on_the_snapshot_event_guarded_by_its_match() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "::sce_rust_runtime::test::receives::<reply_ok::Value>(self.test_context(), \"sut\", \
         \"job.reply\", |sce_message| reply_ok::matches(sce_message))",
    );
    let timeout = at(&asker, "self.test_context().timed_out(\"reply\")");
    assert!(
        receive < timeout,
        "the branches are tried in document order"
    );
    assert!(asker.contains("event == AskerEvent::SceTestSnapshot"));
    assert!(asker.contains("use super::reply_ok;"));
    assert!(asker.contains("use super::request_of;"));

    let coordinator = text("coordinator");
    assert!(coordinator.contains("self.test_context().done(\"answerer\")"));
    assert!(
        !coordinator.contains("event == CoordinatorEvent::DoneInvokeAnswerer"),
        "a done branch is read from the snapshot, not from a completion"
    );
    assert!(coordinator.contains(
        "::sce_rust_runtime::test::receives::<any_request::Value>(self.test_context(), \"peer\", \
         \"job.request\""
    ));

    let idler = text("idler");
    assert!(idler.contains("self.test_context().snapshot_present(\"peer\")"));
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
    let code = text_at(&path);
    assert!(
        code.contains(") && (self.test_context().snapshot_present(\"sut\"))"),
        "the match follows the cond"
    );
}

/// P4 step 1, §12.3: taking a branch runs, before its content, the statement
/// that removes what it matched and tells the runtime it took a branch. A
/// receive branch whose content reads the message binds it as `ev`, the
/// `_event.data` its content reads, so its content needs no binding of its
/// own from the snapshot's event. A branch whose document writes no content
/// still has the statement as its content.
#[test]
fn taking_a_branch_removes_what_it_matched_before_its_content() {
    let asker = text("asker");
    let receive = at(
        &asker,
        "let sce_message = self.test_context().receive(\"sut\");",
    );
    let read = at(
        &asker,
        "let sce_value = sce_message.value_as::<reply_ok::Value>().expect(",
    );
    let bound = at(
        &asker,
        "let ev = AskerJobReplyPayload { job: sce_value.job, code: sce_value.code };",
    );
    let content = at(&asker, "self.test_context().stop_timer(\"reply\");");
    assert!(receive < read && read < bound && bound < content);
    assert!(
        !asker.contains("let ev = match &self.pending_payload"),
        "the content reads the message the statement bound, not the snapshot's event"
    );
    let timeout = at(&asker, "self.test_context().timeout(\"reply\");");
    let failed = at(
        &asker,
        "self.test_context().setverdict(::sce_rust_runtime::test::Verdict::Fail, &sce_reason);",
    );
    assert!(timeout < failed);

    assert!(text("coordinator").contains("self.test_context().done_taken(\"answerer\");"));
    // A receive any branch with no content still takes its message, and a
    // timeout branch with none still takes its timer.
    assert!(text("idler").contains("self.test_context().receive(\"peer\");"));
    assert!(text("ptc_keeper").contains("self.test_context().timeout(\"patience\");"));
}

/// §11.5, §12.3: the statements are calls on the runtime context, a duration
/// in whole microseconds and a timer's own where the statement writes none;
/// a send names its record's event and carries its specific template's value
/// to the test-port processor; a PTC is started through the host invoker.
#[test]
fn the_statements_the_send_and_the_ptc_go_through_the_runtime() {
    let asker = text("asker");
    for needle in [
        "self.test_context().start_timer(\"reply\", 1000000);",
        "self.test_context().stop_timer(\"reply\");",
        "self.test_context().setverdict(::sce_rust_runtime::test::Verdict::Pass, \"\");",
        "let sce_reason = ",
        "no matching reply in time",
        "processor_type: \"http://sce.dev/ext/test-port\".to_string(),",
        "event_name: \"job.request\".to_string(),",
        "target: \"sut\".to_string(),",
        "value: Some(::std::sync::Arc::new(request_of::value(self.job))),",
    ] {
        assert!(asker.contains(needle), "asker holds `{needle}`");
    }
    let coordinator = text("coordinator");
    for needle in [
        "value: Some(::std::sync::Arc::new(reply_of::value(self.job, self.ok))),",
        "processor_type: \"http://sce.dev/ext/test-component\".to_string(),",
        "invoke_id: \"answerer\".to_string(),",
    ] {
        assert!(coordinator.contains(needle), "coordinator holds `{needle}`");
    }
    assert!(text("ptc_keeper").contains("self.test_context().start_timer(\"patience\", 10000);"));
    assert!(text("idler").contains(
        "self.test_context().setverdict(::sce_rust_runtime::test::Verdict::Inconclusive, \"\");"
    ));
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
            "self.test_context().log(&::std::format!(\"{}: {:?}\", \"answered\", ev.job));"
        ),
        "the log reads the message the branch took"
    );
    assert!(
        !asker.contains("sce_log_info!(\"{}: {:?}\", \"answered\""),
        "a test component's log is not the engine's"
    );
    assert!(text("answerer").contains("self.test_context().log(\"asked\");"));
}

/// The unit of a template takes a number by value and lends a text: the
/// variables bound to its parameters are handed over as it takes them.
#[test]
fn a_template_argument_is_handed_over_as_the_template_takes_it() {
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
    assert!(text_at(&path).contains(
        "value: Some(::std::sync::Arc::new(job_report_of::value(self.elapsed, &self.note, \
         self.code))),"
    ));
}

/// A component over a record with a text field, whose receive branch
/// matches with a template that takes a number. Its content reads the message
/// when `reads` is given.
fn card_watcher(dir: &Path, reads: bool) -> PathBuf {
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
    let content = if reads {
        r#"<assign location="paid" expr="_event.data.amount"/>"#
    } else {
        r#"<sce:setverdict value="pass"/>"#
    };
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
      {content}
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

/// The machine's payload struct holds a text in the runtime's `SceString`, and
/// the record's unit in the portable crate's bounded text, so a receive branch
/// whose content reads a message of a record with a text field is refused on
/// Rust by name. C++, whose two records hold a text alike, generates it, and
/// Rust generates the same branch where its content reads nothing of the
/// message, its argument a number handed over by value.
#[test]
fn a_message_with_a_text_field_its_content_reads_is_refused_on_rust() {
    let dir = tempfile::tempdir().expect("a directory");
    let reading = card_watcher(dir.path(), true);
    let Err(err) = generate(&reading, Language::Rust) else {
        panic!("a branch reading a message with a text field generated on Rust");
    };
    let detail = unsupported("card_watcher", err);
    assert!(
        detail.contains("taking a branch of a test component has no Rust lowering yet"),
        "{detail}"
    );
    if let Err(err) = generate(&reading, Language::Cpp) {
        panic!("the same component generates on C++: {err:?}");
    }

    let other = tempfile::tempdir().expect("a directory");
    let watching = card_watcher(other.path(), false);
    assert!(text_at(&watching).contains(
        "::sce_rust_runtime::test::receives::<card_within::Value>(self.test_context(), \"sut\", \
         \"payment.card\", |sce_message| card_within::matches(sce_message, self.most))"
    ));
}

/// §11.5, §11.8, §12.3: a module parameter is declared with its type and no
/// value of the document's; the component reads the run's value into it, as
/// a saved value of its type is read back, after the context is bound and
/// before the machine is built, and does not start the machine when the read
/// is refused. The twin of the C++ test of the same name.
#[test]
fn a_module_parameter_is_read_from_the_run_before_the_machine_starts() {
    let code = text("param_asker");
    for needle in [
        "/// docs/SCE_TEST_PROFILE.md §11.5: the module parameter `job`, which",
        "let job: u32 = Default::default();",
        "fn accept_module_parameters(",
        "given: &::sce_rust_runtime::test::ModuleParameters,",
        "self.job = ::sce_rust_runtime::saved_state::SavedValue::from_saved(given.of(\"job\")?, \
         \"job\")?;",
    ] {
        assert!(code.contains(needle), "param_asker holds `{needle}`");
    }
    let bound = at(&code, "policy.test_context = Some(context.clone());");
    let read = at(
        &code,
        "if !context.read_module_parameters(|given| policy.accept_module_parameters(given)) {",
    );
    let built = at(&code, "let mut machine = Engine::new(policy);");
    let started = at(&code, "machine.initialize();");
    assert!(
        bound < read && read < built && built < started,
        "the parameters are read after the context is bound and before the machine starts"
    );
    assert!(
        code[read..built].contains("return;"),
        "a refused read leaves the machine unbuilt"
    );
    // The variable that is no module parameter starts at its `expr`.
    assert!(
        !code.contains("let answered: u32 = Default::default();"),
        "{code}"
    );
    assert!(
        !text("asker").contains("accept_module_parameters"),
        "a component with no module parameter reads none"
    );
}

/// §11.8: a byte string module parameter is read back held to its bound, as a
/// restored byte string is, and a text by its own bounded type's reader.
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
        "self.key = ::sce_rust_runtime::saved_state::bounded_bytes(given.of(\"key\")?, \"key\")?;",
        "self.label = ::sce_rust_runtime::saved_state::SavedValue::from_saved(given.of(\"label\")?, \
         \"label\")?;",
    ] {
        assert!(code.contains(needle), "keyed holds `{needle}`:\n{code}");
    }
}

/// A component whose module parameter `tint` is of the enum `Color`, and
/// whose state waits a millisecond before it passes; `extra` is written into
/// that state.
fn color_picker(extra: &str) -> String {
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
      <sce:start-timer timer="pause"/>{extra}
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
/// form, which the machine holds whether or not a saved state of it can be
/// written: on Rust the `SavedValue` of the machine's own enum type, emitted
/// for a machine with no saved shape too, and on C++ the `FromSaved` written
/// beside the type. The machine's own enum type has no `Default`, so the
/// variable starts at the variant its document declares first, which nothing
/// reads before the run's value replaces it.
#[test]
fn an_enum_module_parameter_is_read_through_the_enums_saved_form() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = dir.path().join("color_picker.scxml");
    std::fs::write(&path, color_picker("")).expect("the component is written");
    let code = text_at(&path);
    for needle in [
        "impl ::sce_rust_runtime::saved_state::SavedValue for ColorPickerColorEnum {",
        "let tint: ColorPickerColorEnum = ColorPickerColorEnum::Red;",
        "self.tint = ::sce_rust_runtime::saved_state::SavedValue::from_saved(given.of(\"tint\")?, \
         \"tint\")?;",
    ] {
        assert!(
            code.contains(needle),
            "color_picker holds `{needle}`:\n{code}"
        );
    }
    assert!(!code.contains("let tint: ColorPickerColorEnum = Default::default();"));

    let header = generate(&path, Language::Cpp)
        .unwrap_or_else(|e| panic!("color_picker generates on C++: {e:?}"))
        .into_iter()
        .map(|(_, code)| code)
        .collect::<Vec<_>>()
        .join("\n");
    for needle in [
        "FromSaved(const ::SCE::Saved::JsonValue &value, const std::string &what) {",
        "FromSaved(given.of(\"tint\"), \"tint\");",
    ] {
        assert!(
            header.contains(needle),
            "color_picker holds `{needle}`:\n{header}"
        );
    }
}

/// What would leave a component's machine with no saved shape is refused by
/// the profile itself: a component waits on no session outside its tree, since
/// its only sends leave through its ports and have no delay, and its only
/// `<invoke>` creates a PTC its host serves. So a refused delayed send is the
/// test of that, on both backends.
#[test]
fn a_delayed_send_that_would_leave_no_saved_shape_is_refused() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = dir.path().join("color_picker.scxml");
    let delayed =
        "\n      <send type=\"http://sce.dev/ext/test-port\" target=\"sut\" delay=\"1s\"/>";
    std::fs::write(&path, color_picker(delayed)).expect("the component is written");
    for language in [Language::Rust, Language::Cpp] {
        let Err(err) = generate(&path, language) else {
            panic!("a delayed send of a test component generated on {language:?}");
        };
        let refusal = format!("{err:?}");
        assert!(
            refusal.contains("<send> writes delay") && refusal.contains("a send has no delay"),
            "{language:?}: {refusal}"
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

/// docs/SCE_TEST_PROFILE.md §11.9: the machine holds a value of an enum in its
/// own type and a template's unit takes the type the enum document's unit
/// declares, so a variable of an enum bound to a parameter crosses into the
/// variant of the same declared name, as a value handed to an algorithm
/// does, and the machine imports the enum's unit. C++ generates the same
/// component through the same crossing.
#[test]
fn an_enum_argument_crosses_into_the_templates_enum() {
    let dir = tempfile::tempdir().expect("a directory");
    let imports = imports_dir();
    let template = format!(
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
        imports = imports.display()
    );
    let schema = format!("{}/order_line.scxml", imports.display());
    write_all(
        dir.path(),
        &[
            ("watched_port.scxml", port_of("OrderLine", &schema)),
            ("line_of_color.scxml", template),
            (
                "color_watcher.scxml",
                color_watcher("line_of_color", "enum:Color", "Color.blue"),
            ),
        ],
    );
    let path = dir.path().join("color_watcher.scxml");
    let code = text_at(&path);
    assert!(
        code.contains("line_of_color::matches(sce_message, match self.wanted {"),
        "the argument crosses where the template is called: {code}"
    );
    assert!(code.contains("=> paint_color::PaintColor::"), "{code}");
    assert!(code.contains("use super::paint_color;"), "{code}");
    if let Err(err) = generate(&path, Language::Cpp) {
        panic!("the same component generates on C++: {err:?}");
    }
}

/// An optional value of an enum has no crossing into a template's unit yet,
/// so a variable of one bound to a parameter is refused by name.
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
    let Err(err) = generate(&dir.path().join("color_watcher.scxml"), Language::Rust) else {
        panic!("an optional enum argument generated on Rust");
    };
    let detail = unsupported("color_watcher", err);
    assert!(
        detail.contains("an optional value of the enum `Color` bound to a parameter of a test"),
        "{detail}"
    );
}

/// docs/SCE_TEST_PROFILE.md §12.3: the component the runtime drives builds its
/// machine itself, and a machine that declares `<sce:action>` is a policy over
/// a host the component is given by nothing, so Rust refuses such a component
/// by name.
#[test]
fn a_component_that_declares_a_host_action_is_refused_on_rust() {
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
    let Err(err) = generate(&path, Language::Rust) else {
        panic!("a component that declares <sce:action> generated on Rust");
    };
    let detail = unsupported("beeper", err);
    assert!(
        detail.starts_with("test-component 'beeper' declares <sce:action>"),
        "{detail}"
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
    let Err(err) = generate(&path, Language::Rust) else {
        panic!("an <invoke id> that is no XML Name generated on Rust");
    };
    match err.error {
        ForgeError::Validation(boxed) => assert!(
            matches!(&*boxed, ValidationError::MalformedIdentifier { attr, .. } if attr == "id"),
            "{boxed:?}"
        ),
        other => panic!("refused at the parse: {other:?}"),
    }
}
