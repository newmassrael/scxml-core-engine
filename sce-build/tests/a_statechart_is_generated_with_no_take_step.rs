// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The take step of a test component (docs/SCE_TEST_PROFILE.md §4.4, P4 step
//! 1) is generated only for a test component. Appendix D has no such step, and
//! a statechart that is not a test component pays nothing for it:
//!
//! - on Rust its policy names neither `TAKES_TRANSITIONS` nor
//!   `take_transition`, the two items only the runtime's `test-component`
//!   feature declares, so it compiles in a crate that does not ask for the
//!   feature, whose microstep makes no call for the step;
//! - on C11 the machine carries its own microstep, which has neither
//!   `take_transitions` nor a `take_transition` to call;
//! - on C++ its policy has no `takeTransition`, so the engine's
//!   `SCE::Core::HasTakeTransitionTrait` is false and the take compiles to
//!   nothing.
//!
//! Each language is held both ways: a test component of the runtime's
//! end-to-end test names the step on it, so a needle that stopped matching
//! would fail here rather than pass on output that no longer spells it.

use std::path::{Path, PathBuf};

use sce_build::generator::Language;

/// A statechart with a transition of each kind a microstep takes: one with a
/// target, a targetless one and an eventless one.
const STATECHART: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" name="untaken" initial="idle" datamodel="null">
  <state id="idle">
    <transition event="go" target="busy"/>
    <transition event="poke"/>
  </state>
  <state id="busy">
    <transition target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// Each language whose generated machine carries the take step of a test
/// component, with the names the step is spelled by on it.
const LANGUAGES: [(Language, &[&str]); 3] = [
    (
        Language::Rust,
        &["TAKES_TRANSITIONS", "fn take_transition("],
    ),
    (Language::C11, &["take_transitions(", "take_transition("]),
    (Language::Cpp, &["takeTransition("]),
];

/// The whole generated text of the document at `path` in `language`.
fn text_at(path: &Path, language: Language) -> String {
    sce_build::compile_scxml_lang_typed(
        path.to_str().expect("a UTF-8 path"),
        &sce_build::find_template_dir_for(language),
        language,
    )
    .unwrap_or_else(|e| panic!("{} generates on {language:?}: {e:?}", path.display()))
    .files
    .into_iter()
    .map(|(_, code)| code)
    .collect::<Vec<_>>()
    .join("\n")
}

/// A test component of the runtime's end-to-end test, whose branches are
/// taken.
fn component() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/runtime/asker.scxml")
}

#[test]
fn a_statechart_names_no_take_step_and_a_test_component_does() {
    let dir = tempfile::tempdir().expect("a directory");
    let statechart = dir.path().join("untaken.scxml");
    std::fs::write(&statechart, STATECHART).expect("the statechart is written");
    for (language, names) in LANGUAGES {
        let plain = text_at(&statechart, language);
        let taking = text_at(&component(), language);
        for name in names {
            assert!(
                !plain.contains(name),
                "a statechart on {language:?} names no `{name}`:\n{plain}"
            );
            assert!(
                taking.contains(name),
                "a test component on {language:?} names `{name}`"
            );
        }
    }
}
