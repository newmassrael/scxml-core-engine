// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13: a transition with no `event` is eventless whether or not it
// names a `target`, and a transition with no `target` exits and enters nothing
// and runs its content in place. Both halves of that sentence are true at once
// for a transition that has neither attribute, and the Kotlin backend used to
// drop it.
//
// The drop was in selection only. The action surface emitted the transition's
// content, so the generated machine carried actions its eventless selection
// could never ask for. Measured 2026-08-20 on the seven-channel fixture
// `targetless_transition_completes_macrostep`: the Kotlin engine walked the
// chain up to a targetless link and then stopped, one microstep short, with the
// configuration left where that link began — `chained == 1, polished == 0`.
//
// The selection surface is now the one Appendix D splits off for the document:
// `firstEnabledTransition(state, event)`, the first of a state's OWN
// transitions an event — or, for `null`, the eventless selection — enables. The
// walk up through a state's ancestors is the runtime's
// (`com.sce.runtime.Microstep`), pinned by the Kotlin suite's `MicrostepTest`.
//
// The runtime witness for all of it is the Kotlin channel, which runs under
// Gradle. This file is the same contract where a Rust round can reach it: the
// emitted selection surface either offers the transition or it does not, and
// the mutation harness runs `cargo`.

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

/// The text of one `is <State> -> …` arm of the `when (state)` inside
/// `function`, or a panic naming what was searched.
fn arm_of<'a>(code: &'a str, function: &str, state_arm: &str) -> &'a str {
    let start = code
        .find(function)
        .unwrap_or_else(|| panic!("`{function}` is not emitted at all.\n{code}"));
    let body = &code[start..];
    let arm_start = body
        .find(state_arm)
        .unwrap_or_else(|| panic!("`{function}` has no arm `{state_arm}`.\n{body}"));
    let arm = &body[arm_start..];
    // The arm ends where the next state's arm, or the function, begins.
    let end = arm[state_arm.len()..]
        .find("\n        is ")
        .or_else(|| arm[state_arm.len()..].find("\n    }"))
        .map(|i| i + state_arm.len())
        .unwrap_or(arm.len());
    &arm[..end]
}

/// The emitted line of `arm` that guards on the author's @p expr, or a panic.
///
/// ⚠ Not a convenience, and NOT a spelling. These cases are about the
/// SELECTION SURFACE — whether a targetless eventless transition is offered at
/// all — and they have gone red twice over the guard ARGUMENT, which is not
/// their subject: on 2026-08-29 when the Kotlin templates crossed the
/// translation seam, and on 2026-08-30 when the default script engine target
/// moved to Lua. A case that names the argument at all is a case the seam
/// re-breaks every time it moves, so nothing here spells the call. What it asks
/// for is the author's own text, which `com.sce.runtime.ScriptSource`
/// GUARANTEES appears under either target — it is the `source` half of the
/// pair, kept precisely so a diagnostic can name the expression back.
///
/// A missing guard is a panic rather than an empty string: "the selection
/// surface does not guard on this expression" is the exact defect these cases
/// exist to catch.
fn guard_line<'a>(arm: &'a str, expr: &str) -> &'a str {
    let needle = format!("\"{expr}\"");
    arm.lines()
        .find(|line| line.contains("safeEvaluateGuard") && line.contains(&needle))
        .unwrap_or_else(|| {
            panic!(
                "this arm guards on no expression spelled `{expr}`. The author's text is \
                 the `source` half of every `ScriptSource`, so it appears whichever language \
                 the artifact was emitted for — its absence means the transition is not \
                 offered here at all.\n{arm}"
            )
        })
}

/// The name of the transition object a selection line answers with.
fn answered_transition(line: &str) -> &str {
    line.rsplit("-> ")
        .next()
        .map(str::trim)
        .filter(|name| name.starts_with("transition"))
        .unwrap_or_else(|| panic!("the selection answers with no transition object:\n{line}"))
}

/// The declaration of the transition object `name`, up to its closing paren.
fn transition_declaration<'a>(code: &'a str, name: &str) -> &'a str {
    let head = format!("val {name} = EnabledTransition<");
    let start = code
        .find(&head)
        .unwrap_or_else(|| panic!("`{name}` is answered with but never declared.\n{code}"));
    let decl = &code[start..];
    let end = decl.find("\n        )").unwrap_or(decl.len());
    &decl[..end]
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

/// A compound state whose targetless eventless transition its child reaches.
/// W3C SCXML 3.13 selects an eventless transition from the atomic state and its
/// ancestors alike, so `inner` answers through `outer`'s.
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

const SELECTION: &str = "override fun firstEnabledTransition(";

/// The axis: the targetless eventless transition is offered by the selection
/// surface, not only carried by the action surface — to the eventless
/// selection, and as a transition that moves nothing.
#[test]
fn a_targetless_eventless_transition_reaches_the_selection_surface() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    let arm = arm_of(&code, SELECTION, "is BothKindsScxmlState.Settled -> when {");
    let line = guard_line(arm, "polished == 0");
    assert!(
        line.trim_start().starts_with("event == null &&"),
        "the state whose ONLY eventless transition is targetless must offer it to \
         the eventless selection; without that the machine cannot take a microstep \
         the document spells.\n{arm}"
    );
    let offered = transition_declaration(&code, answered_transition(line));
    assert!(
        offered.contains("emptyList()"),
        "and it must be offered with no targets — an in-place microstep that exits \
         and enters nothing, as the runtime reads an empty target list.\n{offered}"
    );
}

/// The other half, and the reason the assertion above is about SELECTION: the
/// action surface always carried the content. An engine that emitted the
/// actions and no way to select them compiles, runs, and silently skips a
/// microstep — which is exactly what shipped.
///
/// Asked through the INDEX: the content is not merely present, it hangs off
/// exactly the number the selected transition carries. A dispatch that carried
/// the content under some other index would still compile and still silently
/// skip the microstep.
#[test]
fn the_action_surface_carried_it_all_along() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    let arm = arm_of(&code, SELECTION, "is BothKindsScxmlState.Settled -> when {");
    let offered =
        transition_declaration(&code, answered_transition(guard_line(arm, "polished == 0")));
    let index = offered
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_suffix(','))
        .find(|value| !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or_else(|| panic!("the offered transition carries no index:\n{offered}"));

    let content = arm_of(
        &code,
        "override fun executeTransitionContent(",
        "is BothKindsScxmlState.Settled -> when (transitionIndex) {",
    );
    assert!(
        content.contains(&format!("{index} -> {{")),
        "the transition's content is emitted under the index the selection \
         answers with ({index}), which is what made the missing selection \
         silent rather than a compile error.\n{content}"
    );
    assert!(
        content.contains("\"polished\"") && content.contains("\"polished + 1\""),
        "and the content under that index is the assign the document spells — \
         the author's text is the `source` half of every `ScriptSource`, so it \
         is there whichever language this artifact was emitted for.\n{content}"
    );
}

/// The control: a targeted eventless transition is unaffected, and is still
/// offered as a state change rather than an in-place microstep.
///
/// Without this, a change that offered every eventless transition with no
/// targets would pass the axis above and break every document that moves.
#[test]
fn a_targeted_eventless_transition_still_selects_as_a_state_change() {
    let code = kotlin_of(BOTH_KINDS, "both_kinds.scxml");

    let arm = arm_of(&code, SELECTION, "is BothKindsScxmlState.Idle -> when {");
    let offered = transition_declaration(&code, answered_transition(guard_line(arm, "armed == 1")));
    assert!(
        offered.contains("listOf(StateTarget(BothKindsScxmlState.Settled))"),
        "a transition that names a target must still exit and enter.\n{offered}"
    );
}

/// An ancestor's targetless eventless transition is offered under the
/// ANCESTOR, and only there.
///
/// §scxml-D-selectTransitions reaches it from the atomic state by walking up,
/// and that walk is the runtime's. A copy of the transition under the leaf
/// would make the leaf's own transitions and the ancestor's indistinguishable
/// to the dispatch — the collision the machine-wide numbering once existed to
/// resolve.
#[test]
fn an_ancestors_targetless_eventless_transition_is_offered_under_the_ancestor() {
    let code = kotlin_of(INHERITED, "inherited.scxml");

    let outer = arm_of(&code, SELECTION, "is InheritedScxmlState.Outer -> when {");
    let offered = transition_declaration(
        &code,
        answered_transition(guard_line(outer, "polished == 0")),
    );
    assert!(
        offered.contains("InheritedScxmlState.Outer,") && offered.contains("emptyList()"),
        "the ancestor offers its own transition, sourced at itself and moving \
         nothing.\n{offered}"
    );
    let inner = arm_of(&code, SELECTION, "is InheritedScxmlState.Inner -> when {");
    assert!(
        !inner.contains("\"polished == 0\""),
        "and the leaf does not offer it a second time: the runtime walks from the \
         leaf to the ancestor.\n{inner}"
    );
}

/// The `when (state)` over the sealed state hierarchy keeps its `else` exactly
/// while some state has no transition of its own, because Kotlin rejects a
/// redundant one under `-Werror`.
#[test]
fn the_selection_carries_an_else_exactly_when_a_state_has_no_transition() {
    let selection_of = |code: &str| -> String {
        let start = code
            .find(SELECTION)
            .unwrap_or_else(|| panic!("the selection surface is not emitted.\n{code}"));
        let body = &code[start..];
        let end = body.find("\n    }\n").unwrap_or(body.len());
        body[..end].to_string()
    };

    let exhaustive = selection_of(&kotlin_of(BOTH_KINDS, "both_kinds.scxml"));
    assert!(
        !exhaustive.contains("\n        else -> null"),
        "every state here has a transition, so the `when` is exhaustive and \
         Kotlin rejects the `else` under -Werror.\n{exhaustive}"
    );

    // One state more, with no transition at all.
    let with_gap = BOTH_KINDS.replace("</scxml>", "    <state id=\"parked\"/>\n</scxml>");
    let gapped = selection_of(&kotlin_of(&with_gap, "with_gap.scxml"));
    assert!(
        gapped.contains("\n        else -> null"),
        "`parked` has no transition, so the `when` is not exhaustive and the \
         `else` is what answers for it.\n{gapped}"
    );
}
