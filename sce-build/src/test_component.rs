// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a statechart of `sce:kind="test-component"` declares and writes of
//! the test profile (docs/adr/0091; docs/SCE_TEST_PROFILE.md §11.5).
//!
//! A test component is a `sce-static` statechart, read by the statechart
//! parser ([`crate::parser::SCXMLParser`]). What the profile adds is recorded
//! on the model where it belongs: [`read`] takes the declarations of the
//! root (`<sce:test-port>`, `<sce:test-timer>`), the `sce:module-parameter`
//! of a `<data>` and the PTCs the component invokes, into
//! [`TestComponentProfile`]; a transition's branch ([`TestBranch`]) and a
//! send's template ([`TemplateUse`]) are read where the parser reads the
//! transition and the send. The statements the profile adds to executable
//! content (`<sce:setverdict>`, `<sce:start-timer>`, `<sce:stop-timer>`) are
//! actions of that content, read by [`read_statement`] where the parser reads
//! the rest of it, so each runs in its place among the others (P4).
//!
//! In any other statechart nothing reads them: each attribute is refused as
//! unread (`validation/sce-attribute-unread`), and each statement as an
//! element its executable content does not take.
//!
//! What this module refuses is what makes a record of the document
//! impossible: a data model other than `sce-static` (§11.2), a declaration or
//! a statement without the attribute it is named by, a verdict outside the
//! four. The rules of the profile for a component (docs/adr/0091 §3;
//! docs/SCE_TEST_PROFILE.md §8.2, §11.5) are asked by the submodule `rules`,
//! of the document and of this record.

use serde::Serialize;

use crate::forge::error::{ForgeError, Located, SourceLocation, TestProfileRule, ValidationError};
use crate::forge::model::{ForgeKind, SCE_NAMESPACE};
use crate::model::{Datamodel, SCXML_NAMESPACE};

pub(crate) mod rules;
pub mod snapshot;

/// The type URI of the Event I/O Processor whose addresses are ports: the
/// `type` of every `<send>` of a test component (docs/SCE_TEST_PROFILE.md
/// §11.1).
pub const TEST_PORT_PROCESSOR: &str = "http://sce.dev/ext/test-port";

/// The type URI of the invocation of a PTC: the `type` of every `<invoke>` of
/// a test component (docs/SCE_TEST_PROFILE.md §11.1).
pub const TEST_COMPONENT_INVOKE_TYPE: &str = "http://sce.dev/ext/test-component";

/// The one external event a generated component is delivered each round: the
/// round's snapshot (docs/SCE_TEST_PROFILE.md §3.2 step 3, §12.3). Every
/// branch is a transition on it once the document is lowered for generation
/// ([`snapshot::lower`]). No document can write it: a transition of a test
/// component names no `event` but `done.invoke.<id>` (`test-profile/branch`).
pub const SNAPSHOT_EVENT: &str = "sce.test.snapshot";

/// What a test component reads of a `test-template` it imports, for the
/// machine it is generated into (docs/SCE_TEST_PROFILE.md §11.5, §12.3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestTemplateUnit {
    /// The name the template's generated unit is named by: its document's
    /// file stem, as every forge unit's is.
    pub unit: String,
    /// The event name a message of the template's record travels under, read
    /// from the record's event schema; `None` for a template of no record, or
    /// one whose record could not be read.
    pub event: Option<String>,
    /// The alias of the first union the template's type reaches at any depth
    /// ([`crate::forge::test_profile::union_reached`]), read in the template's
    /// own document: a backend that lowers no union generates no unit of the
    /// template, and so no component that uses it. `None` for every other
    /// template.
    pub union: Option<String>,
}

/// The declarations and PTCs of a test component, in document order.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestComponentProfile {
    /// The `<sce:test-port>` declarations.
    pub ports: Vec<TestPort>,
    /// The `<sce:test-timer>` declarations.
    pub timers: Vec<TestTimer>,
    /// The `id` of each `<data sce:module-parameter="true">`.
    pub module_parameters: Vec<String>,
    /// The `<invoke>` of each PTC the component creates.
    pub invocations: Vec<TestInvocation>,
}

/// One `<sce:test-port id type>`: a port of an imported `test-port-type`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestPort {
    pub id: String,
    /// The alias of the imported `test-port-type`.
    pub port_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// One `<sce:test-timer id duration>`: a one-shot timer (P7).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestTimer {
    pub id: String,
    /// The default duration, as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// A statement the profile adds to executable content, one variant per
/// element, named by the element's local name. It is the
/// [`crate::model::Action::test_statement`] of an action whose
/// `action_type` is [`TestStatement::action_type`], in its place among the
/// other actions of its block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TestStatement {
    /// `<sce:setverdict value reason>` (P12): `reason` is a `sce-static`
    /// expression of type `string`, as written.
    Setverdict {
        verdict: Verdict,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    /// `<sce:start-timer timer duration>` (P7): `duration` as written.
    StartTimer {
        timer: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration: Option<String>,
    },
    /// `<sce:stop-timer timer>` (P7).
    StopTimer { timer: String },
}

impl TestStatement {
    /// The `action_type` of each statement's action, prefixed as the other
    /// statements of the SCE namespace are, so no reader mistakes it for an
    /// SCXML element's name.
    pub const ACTION_TYPES: [&'static str; 3] =
        ["sce_setverdict", "sce_start_timer", "sce_stop_timer"];

    /// The `action_type` of this statement's action.
    pub fn action_type(&self) -> &'static str {
        match self {
            Self::Setverdict { .. } => Self::ACTION_TYPES[0],
            Self::StartTimer { .. } => Self::ACTION_TYPES[1],
            Self::StopTimer { .. } => Self::ACTION_TYPES[2],
        }
    }
}

/// A verdict a document sets (docs/SCE_TEST_PROFILE.md §5.1). `error` is the
/// runtime's alone (§5.2), so a document has no way to write it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Inconclusive,
    Fail,
    None,
}

impl Verdict {
    /// Every verdict a document sets, in the order of §5.1.
    pub const ALL: [Self; 4] = [Self::Pass, Self::Inconclusive, Self::Fail, Self::None];

    /// The `value` attribute's spelling.
    pub fn as_attr(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Inconclusive => "inconclusive",
            Self::Fail => "fail",
            Self::None => "none",
        }
    }

    /// The verdict `value` names, if it names one.
    pub fn from_attr(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_attr() == value)
    }
}

/// One `<invoke type="http://sce.dev/ext/test-component">`: the creation of a
/// PTC, in the state that invokes it (P9).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestInvocation {
    pub state: String,
    /// The `id`: the PTC's name in the configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The `src`: the test component the PTC is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// The template a send sends, or a receive branch matches with, and the
/// component variables bound in order to its formal parameters
/// (`sce:test-template`, `sce:template-args`; docs/SCE_TEST_PROFILE.md
/// §11.5), as written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TemplateUse {
    /// The alias of the imported `test-template`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// The variables, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

impl TemplateUse {
    /// Whether `node` names neither a template nor an argument.
    pub fn is_empty(&self) -> bool {
        self.template.is_none() && self.args.is_empty()
    }

    /// The `sce:test-template` and `sce:template-args` of `node`, each read
    /// through the channel that records the read.
    pub(crate) fn read(node: &roxmltree::Node) -> Self {
        Self {
            template: crate::sce_attr::read(node, "test-template").map(|t| t.trim().to_string()),
            args: crate::sce_attr::read(node, "template-args")
                .map(|args| args.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
        }
    }
}

/// What makes a transition a branch of a test component
/// (docs/SCE_TEST_PROFILE.md §4.2): the port it receives on, the timer whose
/// timeout it takes, and the template it matches with, as written. A
/// transition that is a done branch names `done.invoke.<id>` in its `event`,
/// and carries none of these.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestBranch {
    /// `sce:receive`: the port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive: Option<String>,
    /// `sce:timeout`: the timer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,
    /// `sce:test-template` and `sce:template-args`.
    #[serde(default, skip_serializing_if = "TemplateUse::is_empty")]
    pub template: TemplateUse,
}

impl TestBranch {
    /// The branch `transition` is, or `None` when it carries none of the
    /// profile's attributes.
    pub(crate) fn read(transition: &roxmltree::Node) -> Option<Self> {
        let branch = Self {
            receive: crate::sce_attr::read(transition, "receive").map(|p| p.trim().to_string()),
            timeout: crate::sce_attr::read(transition, "timeout").map(|t| t.trim().to_string()),
            template: TemplateUse::read(transition),
        };
        (branch != Self::default()).then_some(branch)
    }

    /// Whether every snapshot that lets `other` match lets this branch match
    /// too, so that this one, unguarded and tried first, leaves `other`
    /// nothing (docs/SCE_TEST_PROFILE.md §4.2, §4.3): a receive on one port
    /// with no template, or with the same template and arguments, and a
    /// timeout of one timer.
    pub fn hides(&self, other: &Self) -> bool {
        match (&self.receive, &self.timeout, &other.receive, &other.timeout) {
            (Some(port), None, Some(other_port), None) => {
                port == other_port
                    && (self.template.template.is_none() || self.template == other.template)
            }
            (None, Some(timer), None, Some(other_timer)) => timer == other_timer,
            _ => false,
        }
    }
}

fn is_sce(node: &roxmltree::Node, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(SCE_NAMESPACE)
        && node.tag_name().name() == local
}

fn is_scxml(node: &roxmltree::Node, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(SCXML_NAMESPACE)
        && node.tag_name().name() == local
}

/// The value of `node`'s attribute `attr`, which a record of the element
/// cannot be made without.
fn required(
    node: &roxmltree::Node,
    element: &str,
    attr: &str,
    diag_label: &str,
) -> Result<String, Located<ForgeError>> {
    match node.attribute(attr).map(str::trim) {
        Some(value) if !value.is_empty() => Ok(value.to_string()),
        Some(_) => Err(crate::forge::parser::located(
            node,
            diag_label,
            ValidationError::EmptyValue {
                element: element.to_string(),
                attr: attr.to_string(),
            },
        )),
        None => Err(crate::forge::parser::located(
            node,
            diag_label,
            ValidationError::MissingAttribute {
                element: element.to_string(),
                attr: attr.to_string(),
            },
        )),
    }
}

/// The profile's declarations and PTCs of the test component whose root is
/// `root`, named `name`, which declares `datamodel`.
pub(crate) fn read(
    root: &roxmltree::Node,
    name: &str,
    datamodel: Datamodel,
    diag_label: &str,
) -> Result<TestComponentProfile, Located<ForgeError>> {
    if datamodel != Datamodel::SceStatic {
        let written = root.attribute("datamodel").unwrap_or_default();
        return Err(crate::forge::parser::located(
            root,
            diag_label,
            ValidationError::TestProfile {
                rule: TestProfileRule::Datamodel,
                kind: ForgeKind::TestComponent,
                document: name.to_string(),
                actual: written.to_string(),
                detail: format!(
                    "declares the data model '{}': a test-component declares \
                     datamodel=\"sce-static\", the one data model of the test profile that \
                     holds expressions (docs/SCE_TEST_PROFILE.md §11.2)",
                    datamodel.as_str()
                ),
            },
        ));
    }

    let mut profile = TestComponentProfile::default();
    for node in root.children().filter(|n| is_sce(n, "test-port")) {
        profile.ports.push(TestPort {
            id: required(&node, "<sce:test-port>", "id", diag_label)?,
            port_type: required(&node, "<sce:test-port>", "type", diag_label)?,
            source_location: crate::parser::source_location_of(&node, diag_label),
        });
    }
    for node in root.children().filter(|n| is_sce(n, "test-timer")) {
        profile.timers.push(TestTimer {
            id: required(&node, "<sce:test-timer>", "id", diag_label)?,
            duration: node.attribute("duration").map(|d| d.trim().to_string()),
            source_location: crate::parser::source_location_of(&node, diag_label),
        });
    }

    // Everything below the root, in document order. A nested `<scxml>` is a
    // document of its own, an invoked child, and is not walked.
    let mut pending: Vec<roxmltree::Node> = root.children().filter(|n| n.is_element()).collect();
    pending.reverse();
    while let Some(node) = pending.pop() {
        if is_scxml(&node, "scxml") {
            continue;
        }
        if is_scxml(&node, "data") {
            if let Some(flag) = crate::sce_attr::read(&node, "module-parameter") {
                match flag.trim() {
                    "true" | "1" => profile.module_parameters.push(required(
                        &node,
                        "<data sce:module-parameter=\"true\">",
                        "id",
                        diag_label,
                    )?),
                    "false" | "0" => {}
                    other => {
                        return Err(crate::forge::parser::located(
                            &node,
                            diag_label,
                            ValidationError::InvalidAttribute {
                                element: "<data>".into(),
                                attr: "sce:module-parameter".into(),
                                value: other.to_string(),
                                allowed: vec!["true".into(), "false".into()],
                            },
                        ));
                    }
                }
            }
        }
        if is_scxml(&node, "invoke")
            && node.attribute("type").map(str::trim) == Some(TEST_COMPONENT_INVOKE_TYPE)
        {
            profile.invocations.push(TestInvocation {
                state: crate::parser::owning_state(&node),
                id: node.attribute("id").map(|id| id.trim().to_string()),
                src: node.attribute("src").map(|src| src.trim().to_string()),
                source_location: crate::parser::source_location_of(&node, diag_label),
            });
        }
        let mut children: Vec<roxmltree::Node> =
            node.children().filter(|n| n.is_element()).collect();
        children.reverse();
        pending.extend(children);
    }
    Ok(profile)
}

/// The statement `node` is, when it is one of the profile's: the parser asks
/// it of each element of the SCE namespace in a test component's executable
/// content.
pub(crate) fn read_statement(
    node: &roxmltree::Node,
    diag_label: &str,
) -> Result<Option<TestStatement>, Located<ForgeError>> {
    if node.tag_name().namespace() != Some(SCE_NAMESPACE) {
        return Ok(None);
    }
    Ok(Some(match node.tag_name().name() {
        "setverdict" => {
            let written = required(node, "<sce:setverdict>", "value", diag_label)?;
            let verdict = Verdict::from_attr(&written).ok_or_else(|| {
                crate::forge::parser::located(
                    node,
                    diag_label,
                    ValidationError::InvalidAttribute {
                        element: "<sce:setverdict>".into(),
                        attr: "value".into(),
                        value: written.clone(),
                        allowed: Verdict::ALL
                            .iter()
                            .map(|v| v.as_attr().to_string())
                            .collect(),
                    },
                )
            })?;
            TestStatement::Setverdict {
                verdict,
                reason: node.attribute("reason").map(str::to_string),
            }
        }
        "start-timer" => TestStatement::StartTimer {
            timer: required(node, "<sce:start-timer>", "timer", diag_label)?,
            duration: node.attribute("duration").map(|d| d.trim().to_string()),
        },
        "stop-timer" => TestStatement::StopTimer {
            timer: required(node, "<sce:stop-timer>", "timer", diag_label)?,
        },
        _ => return Ok(None),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(receive: Option<&str>, timeout: Option<&str>, template: Option<&str>) -> TestBranch {
        TestBranch {
            receive: receive.map(str::to_string),
            timeout: timeout.map(str::to_string),
            template: TemplateUse {
                template: template.map(str::to_string),
                args: Vec::new(),
            },
        }
    }

    /// A receive with no template takes every message of its port, so it
    /// hides every later receive on that port; a receive with a template
    /// hides only the same receive; a timeout hides the same timeout.
    #[test]
    fn a_branch_hides_only_what_it_always_matches_first() {
        let any = branch(Some("sut"), None, None);
        let done = branch(Some("sut"), None, Some("Done"));
        let other = branch(Some("sut"), None, Some("Other"));
        let elsewhere = branch(Some("ctl"), None, None);
        let reply = branch(None, Some("reply"), None);
        assert!(any.hides(&done));
        assert!(any.hides(&any));
        assert!(done.hides(&done));
        assert!(!done.hides(&other));
        assert!(!done.hides(&any));
        assert!(!any.hides(&elsewhere));
        assert!(reply.hides(&reply));
        assert!(!reply.hides(&any));
        assert!(!any.hides(&reply));
    }

    #[test]
    fn every_verdict_reads_back_from_its_spelling() {
        for verdict in Verdict::ALL {
            assert_eq!(Verdict::from_attr(verdict.as_attr()), Some(verdict));
        }
        assert_eq!(Verdict::from_attr("error"), None);
    }
}
