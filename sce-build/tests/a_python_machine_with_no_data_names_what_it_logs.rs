// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A `<log>` of a `datamodel="sce-static"` machine generated in Python shows
//! its value through `_sce_log_name` (docs/SCE_ACCEPTED_SUBSET.md §2.15, "What
//! a `<log>` shows"): `null` for an absent optional value, the declared name
//! of an enum member, and any other value as it is. The module declares the
//! helper wherever a `<log>` writes a value, to the engine's log or to a test
//! component's test log (docs/SCE_TEST_PROFILE.md §12.3), and nowhere else: a
//! machine that holds no variable and logs a value declares it, which a module
//! that declared it only beside its variables did not, so its first `<log>`
//! raised `NameError` where it ran.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;

/// The directory of the test components the runtime's end-to-end test runs,
/// whose port type a component written here imports.
fn runtime_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime")
}

/// The Python text of the statechart `document`, written as `<name>.scxml` in
/// `dir`, which generates.
fn python_text(dir: &Path, name: &str, document: &str) -> String {
    let path = dir.join(format!("{name}.scxml"));
    std::fs::write(&path, document).expect("the document is written");
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(Language::Python),
        Language::Python,
    )
    .unwrap_or_else(|e| panic!("{name} generates on Python: {e:?}"))
    .files
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

/// The helper's declaration.
const HELPER: &str = "\ndef _sce_log_name(value):\n";

/// A machine that declares no `<data>` and logs a labelled value declares the
/// helper its `<log>` calls, at the module's level, before the policy that
/// calls it.
#[test]
fn a_machine_with_no_data_declares_the_helper_its_log_calls() {
    let dir = tempfile::tempdir().expect("a directory");
    let code = python_text(
        dir.path(),
        "quiet_logger",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="quiet_logger" datamodel="sce-static" initial="s">
  <state id="s">
    <onentry>
      <log label="greeting" expr="'hello'"/>
    </onentry>
  </state>
</scxml>
"#,
    );
    let declared = at(&code, HELPER);
    let called = at(&code, "self.log_hook(\"greeting\", _sce_log_name(");
    assert!(
        declared < called,
        "the helper is declared before the policy that calls it:\n{code}"
    );
    assert!(
        declared < at(&code, "\nclass QuietLoggerPolicy("),
        "the helper is declared at the module's level:\n{code}"
    );
    assert_eq!(code.matches(HELPER).count(), 1, "{code}");
}

/// A test component that declares no `<data>` and logs a labelled value to
/// its test log declares the helper too: its `<log>` shows the value as the
/// engine's does (docs/SCE_TEST_PROFILE.md §12.3).
#[test]
fn a_test_component_with_no_data_declares_the_helper_its_test_log_calls() {
    let dir = tempfile::tempdir().expect("a directory");
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="quiet_idler"
       datamodel="sce-static" initial="idling">
  <sce:import kind="test-port-type" src="{runtime}/exchange_port.scxml" as="ExchangePort"/>
  <sce:test-port id="peer" type="ExchangePort"/>
  <state id="idling">
    <onentry>
      <log label="idle" expr="'quiet'"/>
      <sce:setverdict value="inconclusive"/>
    </onentry>
    <transition sce:receive="peer" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#,
        runtime = runtime_dir().display()
    );
    let code = python_text(dir.path(), "quiet_idler", &document);
    let declared = at(&code, HELPER);
    let called = at(
        &code,
        "self._sce_test_context.log(\"idle: \" + str(_sce_log_name(",
    );
    assert!(
        declared < called,
        "the helper is declared before the policy that calls it:\n{code}"
    );
    assert!(
        !code.contains("self.log_hook(\"idle\""),
        "a test component's log is not the engine's:\n{code}"
    );
}

/// The helper is declared for what calls it, not for the variables: a machine
/// that holds a variable and logs no value declares none.
#[test]
fn a_machine_that_logs_no_value_declares_no_helper() {
    let dir = tempfile::tempdir().expect("a directory");
    let counted = python_text(
        dir.path(),
        "silent_counter",
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="silent_counter" datamodel="sce-static" initial="s">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="tick" type="internal">
      <assign location="count" expr="1"/>
    </transition>
  </state>
</scxml>
"#,
    );
    assert!(
        counted.contains("self.v_count = v_count"),
        "the variable is the policy's:\n{counted}"
    );
    assert!(!counted.contains("_sce_log_name"), "{counted}");
}
