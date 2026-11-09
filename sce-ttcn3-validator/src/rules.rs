// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The rules of docs/SCE_TEST_PROFILE.md §9.2, and which of them translates a
//! construct of each kind, and when one must: the one table every check of
//! §9.4 reads ([`translations`]).
//!
//! A rule writes a construct always, or when a fact the documents fix holds
//! of it ([`Facts`]), or when the exit and entry sets of a branch reach it
//! ([`When::Taken`]): an `<onentry>` is written into every block that enters
//! its state, a top-level `<final>`'s `return` into every block that ends
//! there. What the sets reach is what a branch computes from the documents
//! (§9.2 E8), so an entry of [`When::Taken`] is held where those sets are,
//! and the first check of §9.4 asks only that it name a construct of the
//! documents.

use std::fmt;

use serde::Deserialize;

/// A rule of §9.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Rule {
    E1,
    E2,
    E3,
    E4,
    E5,
    E6,
    E7,
    E8,
    E9,
    E10,
    E11,
    E12,
    E13,
    E14,
    E15,
}

impl Rule {
    /// The rule's name, as §9.2 and the witness write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Rule::E1 => "E1",
            Rule::E2 => "E2",
            Rule::E3 => "E3",
            Rule::E4 => "E4",
            Rule::E5 => "E5",
            Rule::E6 => "E6",
            Rule::E7 => "E7",
            Rule::E8 => "E8",
            Rule::E9 => "E9",
            Rule::E10 => "E10",
            Rule::E11 => "E11",
            Rule::E12 => "E12",
            Rule::E13 => "E13",
            Rule::E14 => "E14",
            Rule::E15 => "E15",
        }
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The kind of a construct of the documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The `test-suite` document, translated whole.
    Suite,
    /// An `event-schema`, `enum` or `union` document, translated whole.
    Types,
    /// A `test-template` document, translated whole.
    Template,
    /// A `test-port-type` document, translated whole.
    PortType,
    /// A `test-case` document, translated whole.
    TestCase,
    /// A `test-configuration` document, translated whole.
    Configuration,
    /// The suite's `feature-model` document, whose features are translated
    /// and which is not.
    FeatureModel,
    /// A `test-component` document, its declarations and its statechart.
    Component,
    /// A `<sce:module-parameter>` of the suite.
    ModuleParameter,
    /// An `<sce:execute>` of the suite.
    Execute,
    /// An `<sce:feature>` of the feature model.
    Feature,
    /// An `<sce:test-port>` of a component.
    Port,
    /// An `<sce:test-timer>` of a component.
    Timer,
    /// A `<data>` of a component.
    Variable,
    /// An `<sce:system-port>` of a configuration.
    SystemPort,
    /// An atomic state that is no top-level `<final>`.
    AtomicState,
    /// A `<final>` child of the document's root.
    TopLevelFinal,
    /// A state with child states.
    CompoundState,
    /// An `<invoke>` of a test component.
    Invoke,
    /// A `<transition>`.
    Transition,
    /// An element of executable content.
    Action(Action),
    /// An `<elseif>` of an `<if>`.
    Elseif,
    /// The local a receive branch stores its message in, named after the
    /// record it holds (`received <record>`).
    Received,
}

/// An element of executable content, by the rule that translates it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `<send>`.
    Send,
    /// `<assign>`.
    Assign,
    /// `<log>`.
    Log,
    /// `<if>`.
    If,
    /// `<sce:setverdict>`.
    Setverdict,
    /// `<sce:start-timer>` and `<sce:stop-timer>`.
    Timer,
    /// Any other element, which no rule translates.
    Other,
}

/// When a rule writes a construct of a kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    /// Every construct of the kind.
    Always,
    /// A test case the suite executes, the configuration it runs in, and a
    /// system port of that configuration.
    Executed,
    /// A component held by a configuration as its MTC or a PTC, or one that
    /// invokes no PTC: the one whose behaviour has a function.
    Held,
    /// A variable that is no module parameter.
    Variable,
    /// An atomic state whose eventless checks do not end in a transition
    /// with no guard: the one that waits in an `alt`.
    Waits,
    /// An eventless transition with a guard, checked at an atomic state.
    Checked,
    /// A branch of an atomic state that waits, or of one of its ancestors:
    /// an alternative of its `alt`.
    Branch,
    /// Content of a transition that is written: a branch, a check, or the
    /// eventless transition that ends the checks.
    Written,
    /// A construct written where the exit and entry sets of a branch reach
    /// it.
    Taken,
}

/// What the documents fix about a construct, which [`When`] reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Facts {
    pub executed: bool,
    pub held: bool,
    pub module_parameter: bool,
    pub waits: bool,
    pub checked: bool,
    pub branch: bool,
    pub written: bool,
}

impl When {
    /// Whether a construct of these facts must have an entry of the rule.
    /// [`When::Taken`] is never required here: the exit and entry sets
    /// decide it.
    pub fn required(self, facts: &Facts) -> bool {
        match self {
            When::Always => true,
            When::Executed => facts.executed,
            When::Held => facts.held,
            When::Variable => !facts.module_parameter,
            When::Waits => facts.waits,
            When::Checked => facts.checked,
            When::Branch => facts.branch,
            When::Written => facts.written,
            When::Taken => false,
        }
    }
}

/// Each rule that translates a construct of `kind`, and when it does.
///
/// A transition is translated by E8 as an alternative when it is a branch,
/// and as the `goto` that ends its block when it is eventless and the block
/// does not end in a top-level `<final>`, which is a fact of its entry set:
/// [`When::Branch`] requires the first, and the second is held where the
/// entry sets are. A component's own E8 is the `goto` that ends the start of
/// its function, which its initial entry decides.
pub fn translations(kind: Kind) -> &'static [(Rule, When)] {
    match kind {
        Kind::Suite => &[(Rule::E1, When::Always), (Rule::E15, When::Always)],
        Kind::Types => &[(Rule::E1, When::Always)],
        Kind::Template => &[(Rule::E2, When::Always)],
        Kind::PortType => &[(Rule::E3, When::Always)],
        Kind::TestCase | Kind::Configuration | Kind::SystemPort => &[(Rule::E14, When::Executed)],
        Kind::FeatureModel | Kind::CompoundState => &[],
        Kind::Component => &[
            (Rule::E4, When::Always),
            (Rule::E5, When::Held),
            (Rule::E8, When::Taken),
        ],
        Kind::ModuleParameter | Kind::Execute | Kind::Feature => &[(Rule::E15, When::Always)],
        Kind::Port | Kind::Timer => &[(Rule::E4, When::Always)],
        Kind::Variable => &[(Rule::E4, When::Variable)],
        Kind::AtomicState => &[(Rule::E6, When::Always), (Rule::E7, When::Waits)],
        Kind::TopLevelFinal => &[(Rule::E12, When::Taken)],
        Kind::Invoke => &[(Rule::E11, When::Always)],
        Kind::Transition => &[(Rule::E6, When::Checked), (Rule::E8, When::Branch)],
        Kind::Action(Action::Send) => &[(Rule::E9, When::Written)],
        Kind::Action(Action::Assign) | Kind::Action(Action::If) | Kind::Elseif => {
            &[(Rule::E8, When::Written)]
        }
        Kind::Action(Action::Log) | Kind::Action(Action::Setverdict) => {
            &[(Rule::E13, When::Written)]
        }
        Kind::Action(Action::Timer) => &[(Rule::E10, When::Written)],
        Kind::Action(Action::Other) => &[],
        Kind::Received => &[(Rule::E8, When::Taken)],
    }
}

/// Whether `rule` translates a construct of `kind`.
pub fn applies(kind: Kind, rule: Rule) -> bool {
    translations(kind)
        .iter()
        .any(|(translating, _)| *translating == rule)
}

/// The rules a construct of `kind` and `facts` must have an entry of.
pub fn required(kind: Kind, facts: &Facts) -> Vec<Rule> {
    translations(kind)
        .iter()
        .filter(|(_, when)| when.required(facts))
        .map(|(rule, _)| *rule)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_is_read_and_written_by_its_name() {
        let rule: Rule = serde_json::from_str(r#""E13""#).expect("a rule");
        assert_eq!(rule, Rule::E13);
        assert_eq!(rule.to_string(), "E13");
        assert!(serde_json::from_str::<Rule>(r#""E16""#).is_err());
    }

    /// The suite is translated twice and always; a test case only where the
    /// suite executes it; a variable only where it is no module parameter.
    #[test]
    fn a_kind_is_required_by_its_facts() {
        let none = Facts::default();
        assert_eq!(required(Kind::Suite, &none), [Rule::E1, Rule::E15]);
        assert!(required(Kind::TestCase, &none).is_empty());
        let executed = Facts {
            executed: true,
            ..Facts::default()
        };
        assert_eq!(required(Kind::TestCase, &executed), [Rule::E14]);
        assert_eq!(required(Kind::Variable, &none), [Rule::E4]);
        let parameter = Facts {
            module_parameter: true,
            ..Facts::default()
        };
        assert!(required(Kind::Variable, &parameter).is_empty());
    }

    /// An atomic state always has its label and has its alt where it waits;
    /// a transition is required by E6 where it is checked and by E8 where it
    /// is a branch, and by neither where only its entry set decides.
    #[test]
    fn a_state_and_a_transition_are_required_by_what_the_documents_fix() {
        assert_eq!(required(Kind::AtomicState, &Facts::default()), [Rule::E6]);
        let waits = Facts {
            waits: true,
            ..Facts::default()
        };
        assert_eq!(required(Kind::AtomicState, &waits), [Rule::E6, Rule::E7]);
        assert!(required(Kind::Transition, &Facts::default()).is_empty());
        let branch = Facts {
            branch: true,
            ..Facts::default()
        };
        assert_eq!(required(Kind::Transition, &branch), [Rule::E8]);
        let checked = Facts {
            checked: true,
            ..Facts::default()
        };
        assert_eq!(required(Kind::Transition, &checked), [Rule::E6]);
        assert!(required(Kind::TopLevelFinal, &waits).is_empty());
        assert!(applies(Kind::TopLevelFinal, Rule::E12));
        assert!(applies(Kind::Transition, Rule::E8));
        assert!(!applies(Kind::Transition, Rule::E9));
    }

    /// Content is required where its transition is written, by the rule of
    /// its element.
    #[test]
    fn content_is_required_by_the_rule_of_its_element() {
        let written = Facts {
            written: true,
            ..Facts::default()
        };
        assert_eq!(required(Kind::Action(Action::Send), &written), [Rule::E9]);
        assert_eq!(required(Kind::Action(Action::Timer), &written), [Rule::E10]);
        assert_eq!(
            required(Kind::Action(Action::Setverdict), &written),
            [Rule::E13]
        );
        assert_eq!(required(Kind::Elseif, &written), [Rule::E8]);
        assert!(required(Kind::Action(Action::Other), &written).is_empty());
        assert!(required(Kind::Action(Action::Send), &Facts::default()).is_empty());
    }
}
