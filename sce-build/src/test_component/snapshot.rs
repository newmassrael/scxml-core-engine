// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component lowered for generation: the doors of
//! docs/SCE_TEST_PROFILE.md §12.3, written once for every backend that
//! generates a component.
//!
//! A component's machine is the statechart the backend's generator already
//! writes. What the profile changes is which transitions an event enables
//! (§7), and the lowering says it in the statechart's own terms:
//!
//! - every branch — receive, receive any, timeout and done — becomes a
//!   transition on the one event of the round's snapshot
//!   ([`super::SNAPSHOT_EVENT`]), in its place in the document, so the
//!   engine's `selectTransitions` tries the branches in the order of P3
//!   unchanged. What it matches on the snapshot and what taking
//!   it removes ride on the transition ([`SnapshotBranch`]); a backend writes
//!   the match into the guard after the branch's `cond`, and the removal as
//!   the statement before its content;
//! - a send names the event of its template's record and carries the
//!   template's value ([`TemplateRef`]) to the test-port processor, which the
//!   runtime serves through the host processor door (docs/adr/0084);
//! - a PTC's `<invoke>` is run by the host invoker door: its start is P9's
//!   create, its cancel P9's kill;
//! - a `<sce:start-timer>` with no duration takes its timer's.
//!
//! Eventless transitions stay as they are. Nothing here is a backend's
//! spelling: that is the target's ([`crate::forge::static_lowering`]).

use std::collections::BTreeSet;

use super::{
    TemplateUse, TestBranch, TestStatement, SNAPSHOT_EVENT, TEST_COMPONENT_INVOKE_TYPE,
    TEST_PORT_PROCESSOR,
};
use crate::forge::error::GenerateError;
use crate::generator::Language;
use crate::model::SCXMLModel;

/// The prefix of the event a done branch names: `done.invoke.<id>`, `<id>`
/// the PTC its state invokes (docs/SCE_TEST_PROFILE.md §4.2).
const DONE_INVOKE_PREFIX: &str = "done.invoke.";

/// A template as a generated machine names it: where its unit is, the event
/// its record's messages travel under, and the component variables bound to
/// its formal parameters, in order (docs/SCE_TEST_PROFILE.md §11.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateRef {
    /// The alias the component imports the template under.
    pub alias: String,
    /// The name the template's generated unit is named by
    /// ([`super::TestTemplateUnit::unit`]).
    pub unit: String,
    /// The event name of the template's record.
    pub event: String,
    /// `sce:template-args`: the variables, in order.
    pub args: Vec<String>,
}

/// What a branch of a test component matches on the round's snapshot, and so
/// what taking it removes (docs/SCE_TEST_PROFILE.md §4.2, §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotBranch {
    /// `sce:receive`: the snapshot saw a message at the head of `port`, of
    /// the template's record and matching it; with no template, any message.
    /// Taking it removes the head of the port's queue.
    Receive {
        port: String,
        template: Option<TemplateRef>,
    },
    /// `sce:timeout`: the snapshot saw `timer` timed out. Taking it sets the
    /// timer idle.
    Timeout { timer: String },
    /// `event="done.invoke.<ptc>"`: the snapshot's `DONE` holds `ptc`. Taking
    /// it removes nothing (-4 §9.16).
    Done { ptc: String },
}

impl SnapshotBranch {
    /// The template a receive branch matches with, if it names one.
    pub fn template(&self) -> Option<&TemplateRef> {
        match self {
            Self::Receive { template, .. } => template.as_ref(),
            Self::Timeout { .. } | Self::Done { .. } => None,
        }
    }
}

/// `model`, a test component, lowered for generation on `language` through the
/// doors of docs/SCE_TEST_PROFILE.md §12.3 (see the module's text). A model of
/// any other statechart is returned as it is.
///
/// Refused by name where the component names a template whose unit or record
/// was not read, which happens only for a document read without the
/// directory its imports are written relative to; where it uses a template
/// whose type reaches a union on a backend that lowers none, which generates
/// no unit of that template (docs/adr/0092, section 7;
/// docs/SCE_TEST_PROFILE.md §11.9).
///
/// A module parameter needs nothing here: its variable is the statechart's,
/// declared with no initial value ([`crate::forge::static_lowering::StaticField::module_parameter`]),
/// and the component reads the suite's value into it before the machine
/// starts (§11.5, §12.3). That value is the suite's, so a backend that does not
/// generate the suite's tables yet ([`crate::forge::codegen_matrix::TEST_SUITE_BACKENDS`])
/// refuses a component that declares one by name: its component has no read
/// of the value, and none is made up in its place.
pub fn lower(model: &SCXMLModel, language: Language) -> Result<SCXMLModel, GenerateError> {
    let Some(profile) = &model.test_component else {
        return Ok(model.clone());
    };
    if let Some(parameter) = profile
        .module_parameters
        .first()
        .filter(|_| !crate::forge::codegen_matrix::TEST_SUITE_BACKENDS.contains(&language))
    {
        return Err(GenerateError::unsupported(format!(
            "test-component '{}': the module parameter '{parameter}' takes the value the suite \
             declares, which {language:?} reads into a component once it generates the tables \
             of a suite (docs/SCE_TEST_PROFILE.md §11.8, §12.6)",
            model.name
        )));
    }
    let mut lowered = model.clone();

    // §12.3: a send leaves through the test-port processor, and a PTC is run
    // by the test-component invoker, each served by the runtime as a host
    // serves the types it declares (docs/adr/0084). Declared only where the
    // component uses them, beside what the build declared already.
    let mut sends_a_message = false;
    crate::host_processor_analyzer::walk_model_actions(model, &mut |_, action| {
        sends_a_message |= action.action_type == "send" && action.test_template.is_some();
    });
    let mut send_types = model.host_processor_types.clone();
    if sends_a_message && !send_types.iter().any(|t| t == TEST_PORT_PROCESSOR) {
        send_types.push(TEST_PORT_PROCESSOR.to_string());
    }
    let mut invoke_types = model.host_invoker_types.clone();
    if !profile.invocations.is_empty()
        && !invoke_types.iter().any(|t| t == TEST_COMPONENT_INVOKE_TYPE)
    {
        invoke_types.push(TEST_COMPONENT_INVOKE_TYPE.to_string());
    }
    crate::host_processor_analyzer::declare_host_surfaces(&mut lowered, &send_types, &invoke_types)
        .map_err(|reserved| GenerateError::unsupported(reserved.to_string()))?;

    let lowers_union = crate::forge::codegen_matrix::UNION_BACKENDS.contains(&language);
    let template_ref = |used: &TemplateUse, site: &str| -> Result<TemplateRef, GenerateError> {
        let Some(alias) = used.template.as_deref() else {
            return Err(GenerateError::unsupported(format!(
                "{site} of the test component '{}' names no template",
                model.name
            )));
        };
        let unread = |what: &str| {
            GenerateError::unsupported(format!(
                "{site} of the test component '{}' names the template '{alias}', whose {what} \
                 was not read: a test component is generated from its file, beside the \
                 documents it imports (docs/SCE_TEST_PROFILE.md §11.5)",
                model.name
            ))
        };
        let unit = model
            .test_templates
            .get(alias)
            .ok_or_else(|| unread("document"))?;
        if let Some(union) = unit.union.as_deref().filter(|_| !lowers_union) {
            return Err(GenerateError::unsupported(format!(
                "{site} of the test component '{}' names the template '{alias}', whose type \
                 reaches {}",
                model.name,
                crate::forge::value_type::union_unlowered(union)
            )));
        }
        let event = unit
            .event
            .clone()
            .ok_or_else(|| unread("record's event schema"))?;
        Ok(TemplateRef {
            alias: alias.to_string(),
            unit: unit.unit.clone(),
            event,
            args: used.args.clone(),
        })
    };

    // P2, P3: each branch a transition on the snapshot event, in its place.
    let mut received: BTreeSet<String> = BTreeSet::new();
    let mut branches = 0usize;
    for state in lowered.states.values_mut() {
        for transition in &mut state.transitions {
            let branch = match &transition.test_branch {
                Some(TestBranch {
                    receive: Some(port),
                    template,
                    ..
                }) => SnapshotBranch::Receive {
                    port: port.clone(),
                    template: if template.template.is_some() {
                        Some(template_ref(template, "a receive branch")?)
                    } else {
                        None
                    },
                },
                Some(TestBranch {
                    timeout: Some(timer),
                    ..
                }) => SnapshotBranch::Timeout {
                    timer: timer.clone(),
                },
                // A template with no port is refused where the document is
                // read (`test-profile/branch`), so none reaches here.
                Some(_) => {
                    return Err(GenerateError::unsupported(format!(
                        "a branch of the test component '{}' in the state '{}' names neither a \
                         port nor a timer (docs/SCE_TEST_PROFILE.md §4.2)",
                        model.name, state.id
                    )))
                }
                None => match transition.event.trim().strip_prefix(DONE_INVOKE_PREFIX) {
                    Some(ptc) => SnapshotBranch::Done {
                        ptc: ptc.to_string(),
                    },
                    // Eventless: taken as SCXML takes it.
                    None => continue,
                },
            };
            if let Some(template) = branch.template() {
                received.insert(template.event.clone());
            }
            transition.event = SNAPSHOT_EVENT.to_string();
            transition.matching_enum_values = vec![SNAPSHOT_EVENT.to_string()];
            transition.prefix_matching_events = vec![SNAPSHOT_EVENT.to_string()];
            transition.needs_string_matching = false;
            transition.matches_any_event = false;
            transition.snapshot_branch = Some(branch);
            branches += 1;
        }
    }

    // P6: a send carries its template's value under its record's event; P7: a
    // timer started with no duration runs for its own.
    let mut refused: Option<GenerateError> = None;
    crate::host_processor_analyzer::visit_actions_mut(&mut lowered, &mut |action| {
        if refused.is_some() {
            return;
        }
        if action.action_type == "send" {
            if let Some(used) = &action.test_template {
                match template_ref(used, "a <send>") {
                    Ok(message) => {
                        action.event = message.event.clone();
                        action.test_message = Some(message);
                    }
                    Err(why) => refused = Some(why),
                }
            }
        }
        if let Some(TestStatement::StartTimer { timer, duration }) = &mut action.test_statement {
            if duration.is_none() {
                *duration = profile
                    .timers
                    .iter()
                    .find(|declared| declared.id == *timer)
                    .and_then(|declared| declared.duration.clone());
            }
        }
    });
    if let Some(why) = refused {
        return Err(why);
    }

    // The event the snapshot is delivered as, and the event of each message a
    // branch takes, which the payload channel of a machine that reads one is
    // keyed by.
    lowered.events.insert(SNAPSHOT_EVENT.to_string());
    lowered.events.extend(received);
    // P4 step 1: every branch runs a statement before its content, so a
    // machine with a branch has transition content even where the document
    // wrote none.
    if branches > 0 {
        lowered.has_transition_actions = true;
    }
    Ok(lowered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, State, Transition};
    use crate::test_component::{TestComponentProfile, TestInvocation, TestTimer};

    fn branch(receive: Option<&str>, timeout: Option<&str>, template: Option<&str>) -> Transition {
        Transition {
            target: "next".to_string(),
            test_branch: Some(TestBranch {
                receive: receive.map(str::to_string),
                timeout: timeout.map(str::to_string),
                template: TemplateUse {
                    template: template.map(str::to_string),
                    args: vec!["limit".to_string()],
                },
            }),
            ..Default::default()
        }
    }

    /// A component in one state with a receive, a receive any, a timeout, a
    /// done and an eventless transition, in that order, a send of a template
    /// and a timer started with no duration.
    fn component() -> SCXMLModel {
        let mut model = SCXMLModel {
            name: "tester".to_string(),
            initial: "waiting".to_string(),
            test_component: Some(TestComponentProfile {
                timers: vec![TestTimer {
                    id: "reply".to_string(),
                    duration: Some("2s".to_string()),
                    source_location: None,
                }],
                invocations: vec![TestInvocation {
                    state: "waiting".to_string(),
                    id: Some("helper".to_string()),
                    src: Some("helper.scxml".to_string()),
                    source_location: None,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        model.test_templates.insert(
            "Done".to_string(),
            crate::test_component::TestTemplateUnit {
                unit: "job_done".to_string(),
                event: Some("job.done".to_string()),
                union: None,
            },
        );
        let done = Transition {
            event: "done.invoke.helper".to_string(),
            target: "next".to_string(),
            ..Default::default()
        };
        let eventless = Transition {
            target: "next".to_string(),
            ..Default::default()
        };
        // As the parser leaves it: a type no build serves until it is declared.
        let send = Action {
            action_type: "send".to_string(),
            send_type: TEST_PORT_PROCESSOR.to_string(),
            send_type_unsupported: true,
            target: "sut".to_string(),
            test_template: Some(TemplateUse {
                template: Some("Done".to_string()),
                args: vec!["limit".to_string()],
            }),
            ..Default::default()
        };
        let start = Action {
            action_type: "sce_start_timer".to_string(),
            test_statement: Some(TestStatement::StartTimer {
                timer: "reply".to_string(),
                duration: None,
            }),
            ..Default::default()
        };
        let waiting = State {
            id: "waiting".to_string(),
            transitions: vec![
                branch(Some("sut"), None, Some("Done")),
                branch(Some("sut"), None, None),
                branch(None, Some("reply"), None),
                done,
                eventless,
            ],
            on_entry_blocks: vec![vec![send, start]],
            ..Default::default()
        };
        model.states.insert("waiting".to_string(), waiting);
        model
    }

    /// docs/SCE_TEST_PROFILE.md §12.3: every branch is a transition on the
    /// snapshot event, in its place, carrying what it matches; an eventless
    /// transition stays eventless.
    #[test]
    fn every_branch_is_a_transition_on_the_snapshot_event_in_its_place() {
        let lowered = lower(&component(), Language::Cpp).expect("the component lowers");
        let transitions = &lowered.states["waiting"].transitions;
        for transition in &transitions[..4] {
            assert_eq!(transition.event, SNAPSHOT_EVENT);
            assert_eq!(transition.matching_enum_values, [SNAPSHOT_EVENT]);
        }
        assert_eq!(
            transitions[0].snapshot_branch,
            Some(SnapshotBranch::Receive {
                port: "sut".to_string(),
                template: Some(TemplateRef {
                    alias: "Done".to_string(),
                    unit: "job_done".to_string(),
                    event: "job.done".to_string(),
                    args: vec!["limit".to_string()],
                }),
            })
        );
        assert_eq!(
            transitions[1].snapshot_branch,
            Some(SnapshotBranch::Receive {
                port: "sut".to_string(),
                template: None,
            })
        );
        assert_eq!(
            transitions[2].snapshot_branch,
            Some(SnapshotBranch::Timeout {
                timer: "reply".to_string()
            })
        );
        assert_eq!(
            transitions[3].snapshot_branch,
            Some(SnapshotBranch::Done {
                ptc: "helper".to_string()
            })
        );
        assert_eq!(transitions[4].event, "");
        assert_eq!(transitions[4].snapshot_branch, None);
        assert!(lowered.events.contains(SNAPSHOT_EVENT));
        assert!(lowered.events.contains("job.done"));
        assert!(lowered.has_transition_actions);
    }

    /// §12.3: a send names its template's record's event and carries the
    /// template; the doors it and the PTC go through are declared; a timer
    /// started with no duration runs for its own.
    #[test]
    fn a_send_carries_its_template_and_the_doors_are_declared() {
        let lowered = lower(&component(), Language::Cpp).expect("the component lowers");
        let entry = &lowered.states["waiting"].on_entry_blocks[0];
        assert_eq!(entry[0].event, "job.done");
        assert_eq!(
            entry[0].test_message.as_ref().map(|m| m.unit.as_str()),
            Some("job_done")
        );
        assert!(entry[0].send_type_host_served);
        assert_eq!(
            entry[1].test_statement,
            Some(TestStatement::StartTimer {
                timer: "reply".to_string(),
                duration: Some("2s".to_string()),
            })
        );
        assert_eq!(lowered.host_processor_types, [TEST_PORT_PROCESSOR]);
        assert!(lowered
            .host_invoker_types
            .iter()
            .any(|t| t == TEST_COMPONENT_INVOKE_TYPE));
    }

    /// A template whose unit was not read is refused by name, not sent as an
    /// event no record names.
    #[test]
    fn a_template_that_was_not_read_is_refused() {
        let mut model = component();
        model.test_templates.clear();
        let refused = lower(&model, Language::Cpp).expect_err("the template was not read");
        assert!(
            format!("{refused:?}").contains("names the template 'Done'"),
            "{refused:?}"
        );
    }

    /// docs/adr/0092, section 7: a template whose type reaches a union is used
    /// only where a backend lowers a union, which generates its unit.
    #[test]
    fn a_template_that_reaches_a_union_is_refused_where_no_union_is_lowered() {
        let mut model = component();
        if let Some(unit) = model.test_templates.get_mut("Done") {
            unit.union = Some("Payment".to_string());
        }
        let refused = lower(&model, Language::Cpp).expect_err("C++ lowers no union");
        assert!(
            format!("{refused:?}").contains("union:Payment"),
            "{refused:?}"
        );
        assert!(lower(&model, Language::Rust).is_ok());
    }

    /// §11.5, §12.3: a module parameter is a variable of the statechart, which
    /// the component reads the suite's value into; the lowering leaves the
    /// declaration as it is.
    #[test]
    fn a_component_that_declares_a_module_parameter_lowers_with_it() {
        let mut model = component();
        if let Some(profile) = model.test_component.as_mut() {
            profile.module_parameters.push("limit".to_string());
        }
        let lowered = lower(&model, Language::Cpp).expect("a module parameter lowers");
        assert_eq!(
            lowered
                .test_component
                .as_ref()
                .map(|profile| profile.module_parameters.clone()),
            Some(vec!["limit".to_string()])
        );
    }

    /// §11.8, §12.6: a backend that does not generate the suite's tables has no
    /// value for a module parameter, so it refuses a component that declares
    /// one by name, and lowers one that declares none; a backend that generates
    /// them, Rust, C++, Kotlin, Go, Python and C11 among them, lowers it.
    #[test]
    fn a_module_parameter_is_refused_where_the_suites_tables_are_not_generated() {
        let suites = crate::forge::codegen_matrix::TEST_SUITE_BACKENDS;
        for language in [
            Language::Rust,
            Language::Cpp,
            Language::Kotlin,
            Language::Go,
            Language::Python,
            Language::C11,
        ] {
            assert!(suites.contains(&language), "{language:?}");
        }
        let mut model = component();
        if let Some(profile) = model.test_component.as_mut() {
            profile.module_parameters.push("limit".to_string());
        }
        for &language in Language::ALL {
            assert!(lower(&component(), language).is_ok(), "{language:?}");
            let lowered = lower(&model, language);
            if suites.contains(&language) {
                assert!(lowered.is_ok(), "{language:?}: {lowered:?}");
                continue;
            }
            let refused = lowered.expect_err("no suite of this backend gives the value");
            assert!(
                format!("{refused:?}").contains("the module parameter 'limit' takes the value"),
                "{language:?}: {refused:?}"
            );
        }
    }
}
