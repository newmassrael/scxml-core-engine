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

/// The emitted line that guards on the author's @p expr, or a panic naming
/// what was searched.
///
/// ⚠ Not a convenience, and NOT a spelling. These cases are about the
/// SELECTION SURFACE — whether a targetless eventless transition is offered at
/// all — and they have now gone red twice over the guard ARGUMENT, which is
/// not their subject:
///
/// * 2026-08-29, when the Kotlin templates crossed the translation seam and a
///   bare `"polished == 0"` became `ScriptSource.ecmascript("polished == 0")`.
///   Repaired by writing the new spelling down here, once.
/// * 2026-08-30, when `Language::Kotlin.default_script_engine_target()` moved
///   to Lua and the SAME guard became
///   `ScriptSource.lua("_scxml_eq(polished, 0)", "polished == 0")`. The
///   written-down spelling was wrong again, one day later.
///
/// Twice is the measurement: a case that names the argument at all is a case
/// that the seam re-breaks every time it moves. So nothing here spells the
/// call. What it asks for is the author's own text, which
/// `com.sce.runtime.ScriptSource` GUARANTEES appears under either target — it
/// is the `source` half of the pair, kept precisely so a diagnostic can name
/// the expression back to whoever wrote it — and it returns the whole emitted
/// line so each case can ask its own question about what that guard leads to.
///
/// A missing guard is a panic rather than an empty string: "the selection
/// surface does not guard on this expression" is the exact defect these cases
/// exist to catch, and returning something falsy would let a caller's
/// `contains` report it as a different failure.
fn guard_line<'a>(code: &'a str, function: &str, expr: &str) -> &'a str {
    let start = code
        .find(function)
        .unwrap_or_else(|| panic!("`{function}` is not emitted at all.\n{code}"));
    let body = &code[start..];
    let end = body[function.len()..]
        .find("\n    private fun ")
        .map(|i| i + function.len())
        .unwrap_or(body.len());
    let needle = format!("\"{expr}\"");
    body[..end]
        .lines()
        .find(|line| line.contains("safeEvaluateGuard") && line.contains(&needle))
        .unwrap_or_else(|| {
            panic!(
                "`{function}` guards on no expression spelled `{expr}`. The author's text is \
                 the `source` half of every `ScriptSource`, so it appears whichever language \
                 the artifact was emitted for — its absence means the transition is not \
                 offered here at all.\n{}",
                &body[..end]
            )
        })
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
        guard_line(&code, "private fun processNullSettled(", "polished == 0")
            .contains("-> TransitionResult.Internal"),
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
///
/// ⚠ The action surface stopped re-deciding on 2026-08-30 (`c5cfa53bd9`): it
/// used to guard the arm with `event == null && <the same guard>` and now
/// switches on the `transitionIndex` the SELECTION handed it. This case asked
/// for the old string and went red over a repair that only strengthened its
/// own point — so it asks through the INDEX now, which makes the claim
/// sharper than it was: the content is not merely present, it hangs off
/// exactly the number `processNullSettled` answers with. A dispatch that
/// carried the content under some other index would still compile and still
/// silently skip the microstep, and the old string could not tell.
#[test]
fn the_action_surface_carried_it_all_along() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    let selected = guard_line(&code, "private fun processNullSettled(", "polished == 0");
    let index = selected
        .split("TransitionResult.Internal(")
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .unwrap_or_else(|| {
            panic!("the selection answers no transition index for this microstep:\n{selected}")
        });

    let dispatch_start = code
        .find("override fun executeTransitionActions(")
        .unwrap_or_else(|| panic!("the action surface is not emitted at all.\n{code}"));
    let arm_start = code[dispatch_start..]
        .find("is BothKindsScxmlState.Settled -> when (transitionIndex) {")
        .map(|i| i + dispatch_start)
        .unwrap_or_else(|| {
            panic!(
                "the action surface has no arm for `settled`.\n{}",
                &code[dispatch_start..]
            )
        });
    let arm = &code[arm_start..];
    let arm_end = arm.find("\n        is ").unwrap_or(arm.len());
    let arm = &arm[..arm_end];

    assert!(
        arm.contains(&format!("{index} -> {{")),
        "the transition's content is emitted under the index the selection \
         answers with ({index}), which is what made the missing selection \
         silent rather than a compile error.\n{arm}"
    );
    assert!(
        arm.contains("\"polished\"") && arm.contains("\"polished + 1\""),
        "and the content under that index is the assign the document spells — \
         the author's text is the `source` half of every `ScriptSource`, so it \
         is there whichever language this artifact was emitted for.\n{arm}"
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
        guard_line(&code, "private fun processNullIdle(", "armed == 1").contains(
            "-> TransitionResult.External(BothKindsScxmlState.Settled, \
             BothKindsScxmlState.Idle,"
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
        guard_line(&code, "private fun processNullOuter(", "polished == 0")
            .contains("-> TransitionResult.Internal"),
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
