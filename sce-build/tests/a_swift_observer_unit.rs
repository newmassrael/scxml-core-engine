// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// An observer (`sce:kind="observer"`, threshold monitors over the inputs of one sample), written
// for Swift: a `final class` that holds the runtime's `ThresholdState` for each monitor, whose
// `update` answers an `EventQueue` of the events the sample raised, over a `Tag` enumeration nested
// in the class. An event Swift cannot spell as a case, and two events that would be one, are
// refused by name and not written as one.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;
use sce_build::{
    compile_forge_file, compile_forge_from_string, DocumentLabel, ForgeCompileOptions,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The Swift source `document` (a path under the repository) is written as, or the
/// refusal it is answered with.
fn swift_of(document: &str) -> Result<String, String> {
    let output = compile_forge_file(
        &repo_root().join(document),
        Language::Swift,
        &[],
        &ForgeCompileOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (_, code) = output
        .files
        .into_iter()
        .next()
        .expect("a document that is written writes a file");
    Ok(code)
}

/// An observer named `watch` whose one monitor enters with `on_enter` and leaves with
/// `on_leave`, written to Swift from a string.
fn swift_of_monitor(id: &str, on_enter: &str, on_leave: Option<&str>) -> Result<String, String> {
    let leave = on_leave
        .map(|event| format!(r#" sce:leave="level &lt; 1.0" sce:on-leave="{event}""#))
        .unwrap_or_default();
    let document = format!(
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="observer" name="watch">
  <datamodel>
    <data id="level" sce:type="float64" sce:direction="in"/>
    <data id="{id}" sce:monitor="threshold" sce:enter="level &gt; 2.0" sce:on-enter="{on_enter}"{leave}/>
  </datamodel>
</scxml>"#
    );
    compile_forge_from_string(
        &document,
        DocumentLabel {
            identifier: "watch",
            diagnostic_label: "watch.scxml",
        },
        Language::Swift,
    )
    .map_err(|e| format!("{e:?}"))
    .map(|output| {
        output
            .files
            .into_iter()
            .next()
            .expect("a document that is written writes a file")
            .1
    })
}

#[test]
fn an_observer_is_a_class_over_the_runtimes_threshold_state() {
    let code =
        swift_of("tests/forge/resources/observer_coolant.scxml").expect("an observer is written");
    assert!(
        code.contains("public final class ObserverCoolant {"),
        "{code}"
    );
    assert!(
        code.contains("private let warning = SceForgeRuntime.ThresholdState()"),
        "one state for each monitor, named for the runtime's module: {code}"
    );
    assert!(
        code.contains(
            "public func update(_ coolantTemp: Double) -> SceForgeRuntime.EventQueue<Tag> {"
        ),
        "{code}"
    );
}

#[test]
fn the_events_are_cases_of_an_enumeration_nested_in_the_class() {
    let code =
        swift_of("tests/forge/resources/observer_coolant.scxml").expect("an observer is written");
    assert!(code.contains("public enum Tag {"), "{code}");
    for case in ["emitWarning", "clearWarning", "emergencyShutdown"] {
        assert!(code.contains(&format!("case {case}\n")), "{case}: {code}");
    }
    assert!(
        !code.contains("enum ForgeDomainTag"),
        "a file-local enumeration would be declared again by every observer of the module: {code}"
    );
}

#[test]
fn a_monitor_that_names_no_leave_event_leaves_without_raising() {
    let code =
        swift_of("tests/forge/resources/observer_coolant.scxml").expect("an observer is written");
    assert!(
        code.contains("} else if warning.leaveIf(coolantTemp < 100.0) {"),
        "{code}"
    );
    assert!(
        code.contains("} else {\n            critical.leaveIf(coolantTemp < 105.0)\n        }"),
        "the state is left and nothing is raised: {code}"
    );
}

#[test]
fn a_monitor_may_call_the_function_of_a_condition_it_imports() {
    let code = swift_of("tests/forge/resources/crossfile_observer_condition.scxml")
        .expect("an observer over a condition is written");
    assert!(
        code.contains(
            "alarm.enterIf(ConditionThreshold.conditionThreshold(coolantTemp, oilTemp, 110.0))"
        ),
        "the call is qualified by the unit that holds it: {code}"
    );
}

#[test]
fn an_event_swift_reserves_is_refused_by_name() {
    let refusal = swift_of_monitor("hot", "default", None).expect_err("`default` is not a case");
    assert!(refusal.contains("a case name Swift reserves"), "{refusal}");
    assert!(refusal.contains("`default`"), "{refusal}");
}

#[test]
fn two_events_that_are_one_case_are_refused_and_not_merged() {
    let refusal = swift_of_monitor("hot", "hotEntered", Some("hot_entered"))
        .expect_err("two events are not one");
    assert!(
        refusal.contains("which are the one case `hotEntered`"),
        "{refusal}"
    );
}

#[test]
fn a_monitor_named_for_the_method_that_reads_it_is_refused() {
    let refusal =
        swift_of_monitor("update", "raised", None).expect_err("a property is not a method");
    assert!(
        refusal.contains("the name of the method that reads it"),
        "{refusal}"
    );
}

#[test]
fn an_event_raised_on_entering_and_on_leaving_is_one_case() {
    let code = swift_of_monitor("hot", "alarm", Some("alarm")).expect("one event is one case");
    assert_eq!(code.matches("case alarm\n").count(), 1, "{code}");
}
