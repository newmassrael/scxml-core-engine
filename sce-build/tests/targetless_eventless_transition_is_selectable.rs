// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: a transition with no `event` is eventless whether or not it
// names a `target`, and a transition with no `target` exits and enters nothing
// and runs its content in place. Both halves of that sentence are true at once
// for a transition that has neither attribute, and the Kotlin backend used to
// drop it.
//
// The drop was in selection only. `executeTransitionActions` emitted the
// transition's content under `event == null && <guard>`, so the generated
// machine carried actions that `processNullEvent` could never ask for. Measured
// 2026-08-20 on the seven-channel fixture
// `targetless_transition_completes_macrostep`: the Kotlin engine walked the
// chain up to a targetless link and then stopped, one microstep short, with the
// configuration left where that link began — `chained == 1, polished == 0`.
//
// The runtime witness for that is the Kotlin channel, which runs under Gradle.
// This file is the same contract where a Rust round can reach it: the emitted
// selection surface either offers the transition or it does not, and the
// mutation harness runs `cargo`.

use std::path::{Path, PathBuf};

use sce_build::generator::{generate_kotlin, Language};
use sce_build::parser::SCXMLParser;

fn template_dir() -> PathBuf {
    sce_build::find_template_dir_for(Language::Kotlin)
}

fn model_of(content: &str, label: &str) -> sce_build::model::SCXMLModel {
    let mut parser = SCXMLParser::new();
    let mut model = parser
        .parse_string(content, label)
        .unwrap_or_else(|e| panic!("parse failed for {label}: {:?}", e.error));
    sce_build::analyzer::analyze(&mut model, "");
    model
}

fn kotlin_of(content: &str, label: &str) -> String {
    let model = model_of(content, label);
    generate_kotlin(&model, Path::new(&template_dir()), None).expect("Kotlin codegen succeeds")
}

/// A document whose only eventless transition in `settled` is targetless: it
/// runs content and leaves the machine where it is. `idle` reaches `settled`
/// with an ordinary targeted eventless transition, so the emitted machine has
/// one state of each kind and the two can be told apart in the output.
const BOTH_KINDS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="idle" name="both_kinds">
    <datamodel>
        <data id="armed" expr="0"/>
        <data id="polished" expr="0"/>
    </datamodel>
    <state id="idle">
        <transition cond="armed == 1" target="settled"/>
        <transition event="arm">
            <assign location="armed" expr="1"/>
        </transition>
    </state>
    <state id="settled">
        <transition cond="polished == 0">
            <assign location="polished" expr="polished + 1"/>
        </transition>
    </state>
</scxml>"#;

/// A compound state whose targetless eventless transition its child inherits.
/// W3C SCXML 3.13 selects an eventless transition from the atomic state and its
/// ancestors alike, so `inner` answers through `outer`'s — and the dispatch has
/// to name `inner`, because the drain asks about the leaf.
const INHERITED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="outer" name="inherited">
    <datamodel>
        <data id="polished" expr="0"/>
    </datamodel>
    <state id="outer" initial="inner">
        <transition cond="polished == 0">
            <assign location="polished" expr="polished + 1"/>
        </transition>
        <state id="inner">
            <transition event="arm" target="parked"/>
        </state>
    </state>
    <state id="parked"/>
</scxml>"#;

/// The axis: the targetless eventless transition is offered by the selection
/// surface, not only carried by the action surface.
///
/// `TransitionResult.Internal` is what the same backend already returns for an
/// event-driven targetless transition, so the assertion is that the eventless
/// side spells the same answer rather than a new one.
#[test]
fn a_targetless_eventless_transition_reaches_the_null_selection_surface() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    assert!(
        code.contains("private fun processNullSettled("),
        "the state whose ONLY eventless transition is targetless must still get \
         a null handler; without one the machine cannot take a microstep the \
         document spells.\n{code}"
    );
    assert!(
        code.contains("safeEvaluateGuard(\"polished == 0\") -> TransitionResult.Internal"),
        "and the handler must offer it as an in-place microstep — the same \
         answer this backend gives an event-driven targetless transition.\n{code}"
    );
    assert!(
        code.contains("is BothKindsScxmlState.Settled -> processNullSettled()"),
        "and `processNullEvent` must dispatch to it, since that is the function \
         the engine's eventless drain calls.\n{code}"
    );
}

/// The other half, and the reason the assertion above is about SELECTION: the
/// action surface always carried the content. An engine that emitted the
/// actions and no way to select them compiles, runs, and silently skips a
/// microstep — which is exactly what shipped.
#[test]
fn the_action_surface_carried_it_all_along() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    assert!(
        code.contains("event == null && safeEvaluateGuard(\"polished == 0\")"),
        "the transition's content is emitted under the null-event arm, which is \
         what made the missing selection silent rather than a compile error.\n{code}"
    );
}

/// The control: a targeted eventless transition is unaffected, and still
/// selects as a state change rather than an in-place microstep.
///
/// Without this, a change that answered `TransitionResult.Internal` for every
/// eventless transition would pass the axis above and break every document that
/// moves.
#[test]
fn a_targeted_eventless_transition_still_selects_as_a_state_change() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    assert!(
        code.contains(
            "safeEvaluateGuard(\"armed == 1\") -> TransitionResult.External(BothKindsScxmlState.Settled, BothKindsScxmlState.Idle)"
        ),
        "a transition that names a target must still exit and enter.\n{code}"
    );
}

/// A leaf answers through its ancestor's targetless eventless transition, the
/// way §scxml-3.13 selects one.
///
/// The ancestor map is computed in Rust rather than in the template, so this is
/// the half of the same filter that a template-only repair would miss: the
/// child's dispatch arm exists only if the ancestor was recorded as having an
/// eventless transition at all.
#[test]
fn a_leaf_inherits_its_ancestors_targetless_eventless_transition() {
    let code = kotlin_of(INHERITED, "inherited.scxml");

    assert!(
        code.contains("is InheritedScxmlState.Inner -> processNullOuter()"),
        "the drain asks about the active LEAF, so the leaf's arm is what has to \
         reach the ancestor's transition.\n{code}"
    );
    assert!(
        code.contains("safeEvaluateGuard(\"polished == 0\") -> TransitionResult.Internal"),
        "and the ancestor's handler must offer it as an in-place microstep.\n{code}"
    );
}

/// The `when (state)` over the sealed state hierarchy keeps its `else` exactly
/// while some state is left unanswered, because Kotlin rejects a redundant one
/// under `-Werror`.
///
/// Both states above have an eventless transition, so that `when` is
/// exhaustive; a document with a state that has none needs the `else` back.
/// This pair is what keeps `process_null_event_needs_else` the negation of the
/// template's branch condition rather than a guess that happens to hold for one
/// document.
#[test]
fn the_null_dispatch_carries_an_else_exactly_when_a_state_is_unanswered() {
    let exhaustive = kotlin_of(BOTH_KINDS, "both_kinds.scxml");
    let idx = exhaustive
        .find("override fun processNullEvent(")
        .expect("the document has eventless transitions, so the override is emitted");
    let dispatch = &exhaustive[idx..];
    let end = dispatch
        .find("// --- Per-State Null")
        .unwrap_or(dispatch.len());
    assert!(
        !dispatch[..end].contains("else -> TransitionResult.Ignored"),
        "every state here has an eventless transition, so the `when` is \
         exhaustive and Kotlin rejects the `else` under -Werror.\n{}",
        &dispatch[..end]
    );

    // One state more, with no eventless transition of its own or an ancestor's.
    let with_gap = BOTH_KINDS.replace(
        "</scxml>",
        "    <state id=\"parked\"><transition event=\"arm\" target=\"idle\"/></state>\n</scxml>",
    );
    let code = kotlin_of(&with_gap, "with_gap.scxml");
    let idx = code
        .find("override fun processNullEvent(")
        .expect("the override is still emitted");
    let dispatch = &code[idx..];
    let end = dispatch
        .find("// --- Per-State Null")
        .unwrap_or(dispatch.len());
    assert!(
        dispatch[..end].contains("else -> TransitionResult.Ignored"),
        "`parked` has no eventless transition, so the `when` is not exhaustive \
         and the `else` is what answers for it.\n{}",
        &dispatch[..end]
    );
}
