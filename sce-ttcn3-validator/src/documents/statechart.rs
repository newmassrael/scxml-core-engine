// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component (docs/SCE_TEST_PROFILE.md §11.5): its declarations, its
//! states, transitions, invocations and executable content as constructs,
//! what its statechart fixes about which of them a rule writes (§9.2 E6 to
//! E8), and the statechart itself ([`Statechart`]), which the third check of
//! §9.4 computes each branch's exit and entry sets over.
//!
//! At each atomic state that is no top-level `<final>` the export checks the
//! eventless transitions of the state and its ancestors, innermost first and
//! each state's in document order; a check with no guard ends them, and a
//! state whose checks do not end waits in an `alt` of the branches of the
//! state and its ancestors. So the documents fix, without the exit and
//! entry sets of any transition: which atomic states wait, which eventless
//! transitions are checked, which transitions are branches of an `alt`, and
//! the content of every transition written as one of these.
//!
//! A test component holds no `<parallel>`, `<history>`, `<script>`,
//! `<donedata>`, `<raise>` or `<cancel>` (§8.2), and the profile's check
//! refuses each before an export is written. This reader refuses each by
//! name too: it models a statechart of one active atomic state and its
//! ancestors, and does not read one it would model wrongly.

use std::collections::BTreeMap;

use super::{
    attribute, children, is_element, row, sce_attribute, Detail, Document, SCE_NAMESPACE,
    SCXML_NAMESPACE,
};
use crate::rules::{Action, Kind};
use crate::InputError;

/// The `type` of an `<invoke>` of a test component (§11.5).
const TEST_COMPONENT_INVOKE: &str = "http://sce.dev/ext/test-component";

/// The prefix of the event a done branch names (§4.2).
const DONE_INVOKE: &str = "done.invoke.";

/// The elements of the SCXML namespace a test component does not hold
/// (§8.2), as the profile's check lists them.
const REFUSED: [&str; 6] = [
    "parallel", "history", "script", "donedata", "raise", "cancel",
];

/// A test component's statechart: its states in document order, each by its
/// position.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Statechart {
    pub states: Vec<ChartState>,
    /// The targets of the document's initial transition: the root's
    /// `initial`, or its first child state.
    pub initial: Vec<usize>,
}

/// A `<state>` or a `<final>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartState {
    pub id: String,
    /// Its construct, in the document's nodes.
    pub node: usize,
    pub parent: Option<usize>,
    /// Its child states, in document order.
    pub children: Vec<usize>,
    /// A `<final>` child of the document's root: entering it ends the
    /// component (P10).
    pub top_level_final: bool,
    /// The content of its `<onentry>`s, in document order.
    pub onentry: Vec<Content>,
    /// The content of its `<onexit>`s, in document order.
    pub onexit: Vec<Content>,
    /// A compound state's default entry: the targets of its `<initial>`'s
    /// transition, its `initial`, or its first child state.
    pub initial: Vec<usize>,
    /// The content of its `<initial>`'s transition.
    pub initial_content: Vec<Content>,
    /// The constructs of its invocations of a PTC, in document order.
    pub invokes: Vec<usize>,
    pub transitions: Vec<ChartTransition>,
}

impl ChartState {
    /// Whether the state is atomic and no top-level `<final>`: a state the
    /// export writes a label for (E6).
    pub fn labelled(&self) -> bool {
        self.children.is_empty() && !self.top_level_final
    }
}

/// A `<transition>` of a state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartTransition {
    /// Its construct, in the document's nodes.
    pub node: usize,
    /// The states it targets; none for a targetless transition.
    pub targets: Vec<usize>,
    /// `type="internal"`.
    pub internal: bool,
    /// It names no event and is no branch of the profile.
    pub eventless: bool,
    pub guarded: bool,
    /// A receive, timeout or done branch (§4.2).
    pub branch: bool,
    /// Its executable content.
    pub content: Vec<Content>,
}

/// One element of executable content, and the blocks of an `<if>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// Its construct, in the document's nodes.
    pub node: usize,
    /// An `<if>`'s own block, up to its first `<elseif>` or `<else>`.
    pub then: Vec<Content>,
    /// Each `<elseif>` of an `<if>`, its construct and its block.
    pub elseifs: Vec<(usize, Vec<Content>)>,
    /// The block of an `<if>`'s `<else>`.
    pub otherwise: Vec<Content>,
}

/// What a branch waits for (§4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    /// `sce:receive`, the alias of its `sce:test-template` where it names
    /// one, and its `sce:template-args`.
    Receive {
        port: String,
        template: Option<String>,
        args: Vec<String>,
    },
    /// `sce:timeout`.
    Timeout { timer: String },
    /// `done.invoke.<id>`, by the id.
    Done { invoke: String },
}

impl Statechart {
    /// `leaf` and each of its ancestors, innermost first: the states active
    /// while `leaf` is the atomic one, in exit order.
    pub fn chain(&self, leaf: usize) -> Vec<usize> {
        let mut chain = vec![leaf];
        while let Some(parent) = chain.last().and_then(|&state| self.states[state].parent) {
            chain.push(parent);
        }
        chain
    }

    /// The state whose id is `id`.
    pub fn state(&self, id: &str) -> Option<usize> {
        self.states.iter().position(|state| state.id == id)
    }
}

/// The ids a `target` or an `initial` writes.
fn ids(text: Option<&str>) -> Vec<String> {
    text.map(|text| text.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

/// The states and transitions read, each target still the id it is written
/// as, and the row of what wrote it.
#[derive(Default)]
struct Reader {
    states: Vec<ChartState>,
    /// For each state, what it names by id.
    pending: Vec<Pending>,
}

/// The ids a state's default entry names, and those each of its
/// transitions targets, with the transition's row.
type Pending = (Vec<String>, Vec<(Vec<String>, u32)>);

/// Name the constructs of the component whose root is `root`, and the facts
/// its statechart fixes; its statechart.
pub(super) fn read(
    root: roxmltree::Node<'_, '_>,
    document: &mut Document,
) -> Result<Statechart, InputError> {
    if let Some(refused) = root.descendants().find(|node| {
        REFUSED
            .iter()
            .any(|local| is_element(node, SCXML_NAMESPACE, local))
    }) {
        return Err(InputError::Document {
            document: document.name.clone(),
            reason: format!(
                "it holds a <{}> on row {}, which a test component does not hold \
                 (docs/SCE_TEST_PROFILE.md section 8.2) and this validator does not read",
                refused.tag_name().name(),
                row(&refused)
            ),
        });
    }
    document.push("scxml".to_string(), None, Kind::Component);
    for port in children(root, SCE_NAMESPACE, "test-port") {
        let id = attribute(&port, "id").unwrap_or_default();
        let port_type = attribute(&port, "type").unwrap_or_default();
        document.push_detailed(
            format!("sce:test-port '{id}'"),
            Some(row(&port)),
            Kind::Port,
            Detail::Port { id, port_type },
        );
    }
    for timer in children(root, SCE_NAMESPACE, "test-timer") {
        let id = attribute(&timer, "id").unwrap_or_default();
        let duration = attribute(&timer, "duration");
        document.push_detailed(
            format!("sce:test-timer '{id}'"),
            Some(row(&timer)),
            Kind::Timer,
            Detail::Timer { id, duration },
        );
    }
    datamodel(root, document);
    let mut reader = Reader::default();
    for child in state_children(root) {
        reader.state(child, None, document)?;
    }
    let initial = match attribute(&root, "initial") {
        Some(written) => ids(Some(written.as_str())),
        None => reader
            .states
            .iter()
            .find(|state| state.parent.is_none())
            .map(|state| vec![state.id.clone()])
            .unwrap_or_default(),
    };
    let chart = reader.resolve(&initial, document)?;
    settle(&chart, document);
    Ok(chart)
}

/// The child states of `element`.
fn state_children<'a, 'input>(
    element: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    element
        .children()
        .filter(|child| {
            is_element(child, SCXML_NAMESPACE, "state")
                || is_element(child, SCXML_NAMESPACE, "final")
        })
        .collect()
}

/// The variables `element`'s `<datamodel>` declares, each one that is a
/// module parameter said so.
fn datamodel(element: roxmltree::Node<'_, '_>, document: &mut Document) {
    for model in children(element, SCXML_NAMESPACE, "datamodel") {
        for data in children(model, SCXML_NAMESPACE, "data") {
            let id = attribute(&data, "id").unwrap_or_default();
            let ty = sce_attribute(&data, "type");
            let index = document.push_detailed(
                format!("data '{id}'"),
                Some(row(&data)),
                Kind::Variable,
                Detail::Variable { id, ty },
            );
            document.nodes[index].facts.module_parameter = matches!(
                sce_attribute(&data, "module-parameter").as_deref(),
                Some("true" | "1")
            );
        }
    }
}

impl Reader {
    /// The state `element`, child of `parent`, and every state below it.
    fn state(
        &mut self,
        element: roxmltree::Node<'_, '_>,
        parent: Option<usize>,
        document: &mut Document,
    ) -> Result<(), InputError> {
        let tag = element.tag_name().name();
        let id = attribute(&element, "id")
            .filter(|id| !id.is_empty())
            .ok_or_else(|| InputError::Document {
                document: document.name.clone(),
                reason: format!(
                    "the <{tag}> on row {} has no id, and a state is named by its id",
                    row(&element)
                ),
            })?;
        let node = if tag == "final" {
            format!("final '{id}'")
        } else {
            format!("state '{id}'")
        };
        let line = Some(row(&element));
        let inner = state_children(element);
        let top_level_final = parent.is_none() && tag == "final";
        let kind = if top_level_final {
            Kind::TopLevelFinal
        } else if inner.is_empty() {
            Kind::AtomicState
        } else {
            Kind::CompoundState
        };
        let index = document.push(node.clone(), line, kind);

        datamodel(element, document);
        let mut onentry = Vec::new();
        for (position, block) in children(element, SCXML_NAMESPACE, "onentry")
            .into_iter()
            .enumerate()
        {
            let prefix = format!("{node} / onentry {}", position + 1);
            onentry.extend(actions(&element_children(block), &prefix, line, document));
        }
        let mut onexit = Vec::new();
        for (position, block) in children(element, SCXML_NAMESPACE, "onexit")
            .into_iter()
            .enumerate()
        {
            let prefix = format!("{node} / onexit {}", position + 1);
            onexit.extend(actions(&element_children(block), &prefix, line, document));
        }
        let mut initial_ids = Vec::new();
        let mut initial_content = Vec::new();
        for initial in children(element, SCXML_NAMESPACE, "initial") {
            for transition in children(initial, SCXML_NAMESPACE, "transition") {
                let prefix = format!("{node} / initial");
                initial_content.extend(actions(
                    &element_children(transition),
                    &prefix,
                    line,
                    document,
                ));
                initial_ids.extend(ids(transition.attribute("target")));
            }
        }
        if initial_ids.is_empty() {
            initial_ids = ids(element.attribute("initial"));
        }
        let mut invokes = Vec::new();
        for invoke in children(element, SCXML_NAMESPACE, "invoke") {
            if attribute(&invoke, "type").as_deref() != Some(TEST_COMPONENT_INVOKE) {
                continue;
            }
            document.links.invokes = true;
            let invoke_id = attribute(&invoke, "id").unwrap_or_default();
            let src = attribute(&invoke, "src").unwrap_or_default();
            invokes.push(document.push_detailed(
                format!("invoke '{invoke_id}'"),
                Some(row(&invoke)),
                Kind::Invoke,
                Detail::Invoke { id: invoke_id, src },
            ));
        }

        let mut transitions = Vec::new();
        let mut targets = Vec::new();
        for (position, transition) in children(element, SCXML_NAMESPACE, "transition")
            .into_iter()
            .enumerate()
        {
            transitions.push(self::transition(
                transition,
                &format!("{node} / transition {}", position + 1),
                document,
            ));
            targets.push((ids(transition.attribute("target")), row(&transition)));
        }
        let at = self.states.len();
        self.states.push(ChartState {
            id,
            node: index,
            parent,
            children: Vec::new(),
            top_level_final,
            onentry,
            onexit,
            initial: Vec::new(),
            initial_content,
            invokes,
            transitions,
        });
        self.pending.push((initial_ids, targets));
        if let Some(parent) = parent {
            self.states[parent].children.push(at);
        }
        for child in inner {
            self.state(child, Some(at), document)?;
        }
        Ok(())
    }

    /// The statechart, each id it writes resolved to its state; a compound
    /// state that names no initial state enters its first child.
    fn resolve(
        mut self,
        initial: &[String],
        document: &Document,
    ) -> Result<Statechart, InputError> {
        let (initial, resolved) = self.resolved(initial, document)?;
        for (state, (default, targets)) in self.states.iter_mut().zip(resolved) {
            state.initial = default;
            for (transition, targets) in state.transitions.iter_mut().zip(targets) {
                transition.targets = targets;
            }
        }
        Ok(Statechart {
            states: self.states,
            initial,
        })
    }

    /// The root's `initial` resolved, and for each state its default entry
    /// and each transition's targets resolved.
    fn resolved(
        &self,
        initial: &[String],
        document: &Document,
    ) -> Result<(Vec<usize>, Vec<Resolved>), InputError> {
        let positions: BTreeMap<&str, usize> = self
            .states
            .iter()
            .enumerate()
            .map(|(position, state)| (state.id.as_str(), position))
            .collect();
        let resolve = |written: &[String], what: String| -> Result<Vec<usize>, InputError> {
            written
                .iter()
                .map(|id| {
                    positions
                        .get(id.as_str())
                        .copied()
                        .ok_or_else(|| InputError::Document {
                            document: document.name.clone(),
                            reason: format!("{what} names '{id}', which is no state of it"),
                        })
                })
                .collect()
        };
        let initial = resolve(initial, "the initial state of its root".to_string())?;
        let mut resolved = Vec::with_capacity(self.states.len());
        for (state, (initial_ids, targets)) in self.states.iter().zip(&self.pending) {
            let default = if state.children.is_empty() {
                Vec::new()
            } else if initial_ids.is_empty() {
                vec![state.children[0]]
            } else {
                resolve(
                    initial_ids.as_slice(),
                    format!("the initial state of the state '{}'", state.id),
                )?
            };
            let mut transitions = Vec::with_capacity(targets.len());
            for (written, line) in targets {
                transitions.push(resolve(
                    written.as_slice(),
                    format!("the <transition> on row {line}"),
                )?);
            }
            resolved.push((default, transitions));
        }
        Ok((initial, resolved))
    }
}

/// A state's default entry and each of its transitions' targets, resolved.
type Resolved = (Vec<usize>, Vec<Vec<usize>>);

/// The transition `element`, named `node`, its targets left to resolve.
fn transition(
    element: roxmltree::Node<'_, '_>,
    node: &str,
    document: &mut Document,
) -> ChartTransition {
    let line = Some(row(&element));
    let event = attribute(&element, "event").unwrap_or_default();
    let receive = sce_attribute(&element, "receive");
    let timeout = sce_attribute(&element, "timeout");
    let template = sce_attribute(&element, "test-template");
    let args: Vec<String> = sce_attribute(&element, "template-args")
        .map(|args| args.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    let profile_branch =
        receive.is_some() || timeout.is_some() || template.is_some() || !args.is_empty();
    if receive.is_some() && template.is_some() {
        document.received.insert(row(&element));
    }
    let guarded = attribute(&element, "cond").is_some_and(|cond| !cond.is_empty());
    let trigger = match (receive, timeout, event.strip_prefix(DONE_INVOKE)) {
        (Some(port), _, _) => Some(Trigger::Receive {
            port,
            template,
            args,
        }),
        (None, Some(timer), _) => Some(Trigger::Timeout { timer }),
        (None, None, Some(invoke)) => Some(Trigger::Done {
            invoke: invoke.to_string(),
        }),
        (None, None, None) => None,
    };
    let index = document.push_detailed(
        node.to_string(),
        line,
        Kind::Transition,
        Detail::Transition { trigger, guarded },
    );
    let content = actions(&element_children(element), node, line, document);
    ChartTransition {
        node: index,
        targets: Vec::new(),
        internal: attribute(&element, "type").as_deref() == Some("internal"),
        eventless: event.is_empty() && !profile_branch,
        guarded,
        branch: profile_branch || event.starts_with(DONE_INVOKE),
        content,
    }
}

/// The element children of `node`.
fn element_children<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.children().filter(|child| child.is_element()).collect()
}

/// The element `element` is as executable content: the name its node gives
/// it, the rule that translates it and what it writes, or `None` for an
/// element that is no content.
fn action(element: &roxmltree::Node<'_, '_>) -> Option<(String, Action, Detail)> {
    let name = element.tag_name().name();
    match element.tag_name().namespace() {
        Some(SCXML_NAMESPACE) => Some(match name {
            "send" => (
                name.to_string(),
                Action::Send,
                Detail::Send {
                    port: attribute(element, "target").unwrap_or_default(),
                    template: sce_attribute(element, "test-template").unwrap_or_default(),
                    args: sce_attribute(element, "template-args")
                        .map(|args| args.split_whitespace().map(str::to_string).collect())
                        .unwrap_or_default(),
                },
            ),
            "assign" => (
                name.to_string(),
                Action::Assign,
                Detail::Assign {
                    location: attribute(element, "location").unwrap_or_default(),
                },
            ),
            "log" => (
                name.to_string(),
                Action::Log,
                Detail::Log {
                    label: element.attribute("label").unwrap_or_default().to_string(),
                    expr: attribute(element, "expr").is_some_and(|expr| !expr.is_empty()),
                },
            ),
            "if" => (name.to_string(), Action::If, Detail::Bare),
            _ => (name.to_string(), Action::Other, Detail::Bare),
        }),
        // SCE's own statements, named as the statechart reader names them:
        // a test component's three (`sce_setverdict`, `sce_start_timer`,
        // `sce_stop_timer`) and the others of the namespace, which no rule
        // translates and which take a position all the same.
        Some(SCE_NAMESPACE) => match name {
            "setverdict" => Some((
                "sce_setverdict".to_string(),
                Action::Setverdict,
                Detail::Setverdict {
                    value: attribute(element, "value").unwrap_or_default(),
                    reason: element.attribute("reason").is_some(),
                },
            )),
            "start-timer" => Some((
                "sce_start_timer".to_string(),
                Action::Timer,
                Detail::StartTimer {
                    timer: attribute(element, "timer").unwrap_or_default(),
                    duration: attribute(element, "duration"),
                },
            )),
            "stop-timer" => Some((
                "sce_stop_timer".to_string(),
                Action::Timer,
                Detail::StopTimer {
                    timer: attribute(element, "timer").unwrap_or_default(),
                },
            )),
            "append" | "clear" | "lift" | "render" => {
                Some((format!("sce_{name}"), Action::Other, Detail::Bare))
            }
            "action" => Some((name.to_string(), Action::Other, Detail::Bare)),
            _ => None,
        },
        _ => None,
    }
}

/// The content `elements`, each named after `prefix` by its kind and its
/// position from 1 among them, on `line`.
fn actions(
    elements: &[roxmltree::Node<'_, '_>],
    prefix: &str,
    line: Option<u32>,
    document: &mut Document,
) -> Vec<Content> {
    let mut content = Vec::new();
    let mut position = 0;
    for element in elements {
        let Some((name, action, detail)) = self::action(element) else {
            continue;
        };
        position += 1;
        let node = format!("{prefix} / {name} {position}");
        let index = document.push_detailed(node.clone(), line, Kind::Action(action), detail);
        let mut item = Content {
            node: index,
            then: Vec::new(),
            elseifs: Vec::new(),
            otherwise: Vec::new(),
        };
        if action == Action::If {
            conditional(*element, &node, line, document, &mut item);
        }
        content.push(item);
    }
    content
}

/// The blocks of the `<if>` `element`, named `node`, into `item`: its own
/// content up to its first `<elseif>` or `<else>`, each `<elseif>` and its
/// content, and the content of its `<else>`.
fn conditional(
    element: roxmltree::Node<'_, '_>,
    node: &str,
    line: Option<u32>,
    document: &mut Document,
    item: &mut Content,
) {
    let mut then = Vec::new();
    let mut elseifs: Vec<Vec<roxmltree::Node<'_, '_>>> = Vec::new();
    let mut otherwise = Vec::new();
    let mut in_else = false;
    for child in element_children(element) {
        if is_element(&child, SCXML_NAMESPACE, "elseif") {
            elseifs.push(Vec::new());
            in_else = false;
        } else if is_element(&child, SCXML_NAMESPACE, "else") {
            in_else = true;
        } else if in_else {
            otherwise.push(child);
        } else if let Some(part) = elseifs.last_mut() {
            part.push(child);
        } else {
            then.push(child);
        }
    }
    item.then = actions(&then, node, line, document);
    for (position, part) in elseifs.iter().enumerate() {
        let branch = format!("{node} / elseif {}", position + 1);
        let index = document.push(branch.clone(), line, Kind::Elseif);
        let block = actions(part, &branch, line, document);
        item.elseifs.push((index, block));
    }
    item.otherwise = actions(&otherwise, &format!("{node} / else"), line, document);
}

/// The constructs of `content`, every one of them: each element, and the
/// `<elseif>`s and blocks of an `<if>`.
pub(crate) fn each_node(content: &[Content], visit: &mut impl FnMut(usize)) {
    for item in content {
        visit(item.node);
        each_node(&item.then, visit);
        for (elseif, block) in &item.elseifs {
            visit(*elseif);
            each_node(block, visit);
        }
        each_node(&item.otherwise, visit);
    }
}

/// The facts the checks and the `alt`s fix (§9.2 E6, E7): at each labelled
/// state, the eventless transitions of it and its ancestors are checked,
/// innermost first, until one with no guard ends the checks; a state whose
/// checks do not end waits, and each branch of it and its ancestors is an
/// alternative of its `alt`. A transition written so has its content written
/// too.
fn settle(chart: &Statechart, document: &mut Document) {
    for (leaf, leaf_state) in chart.states.iter().enumerate() {
        if !leaf_state.labelled() {
            continue;
        }
        let chain = chart.chain(leaf);
        let mut ended = false;
        'checks: for &state in &chain {
            for transition in &chart.states[state].transitions {
                if !transition.eventless {
                    continue;
                }
                written(transition, document);
                if !transition.guarded {
                    ended = true;
                    break 'checks;
                }
                document.nodes[transition.node].facts.checked = true;
            }
        }
        if ended {
            continue;
        }
        document.nodes[leaf_state.node].facts.waits = true;
        for &state in &chain {
            for transition in chart.states[state].transitions.iter().filter(|t| t.branch) {
                document.nodes[transition.node].facts.branch = true;
                written(transition, document);
            }
        }
    }
}

/// Mark the content of `transition` written.
fn written(transition: &ChartTransition, document: &mut Document) {
    each_node(&transition.content, &mut |index| {
        document.nodes[index].facts.written = true;
    });
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;

    use super::super::tests::Memory;
    use super::super::{Documents, Node};
    use crate::rules::{required, Rule};
    use crate::InputError;

    pub(crate) const SUITE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-suite" name="s">
  <sce:import kind="test-case" src="case.scxml" as="Case"/>
  <sce:execute test-case="Case"/>
</scxml>"#;

    pub(crate) const CASE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-case" name="case"
       sce:configuration="Config">
  <sce:import kind="test-configuration" src="config.scxml" as="Config"/>
</scxml>"#;

    pub(crate) const CONFIG: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-configuration" name="config">
  <sce:import kind="test-component" src="c.scxml" as="C"/>
  <sce:mtc type="C"/>
</scxml>"#;

    /// A component whose `checking` state checks a guarded transition and
    /// ends its checks with an unguarded one, so it has no `alt`; whose
    /// `asking` state waits on two branches of its own and the done branch
    /// of its parent; and whose `<if>` has an `<elseif>` and an `<else>`.
    const COMPONENT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-component" name="c"
       datamodel="sce-static" initial="working">
  <sce:test-port id="sut" type="Port"/>
  <sce:test-timer id="deadline" duration="1s"/>
  <datamodel>
    <data id="limit" sce:type="uint32" sce:module-parameter="true"/>
    <data id="n" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="working" initial="asking">
    <invoke type="http://sce.dev/ext/test-component" src="h.scxml" id="helper"/>
    <transition event="done.invoke.helper" target="passed"/>
    <state id="asking">
      <onentry>
        <send type="http://sce.dev/ext/test-port" target="sut" sce:test-template="Ask"/>
        <sce:start-timer timer="deadline"/>
      </onentry>
      <transition sce:receive="sut" sce:test-template="Reply" target="checking">
        <assign location="n" expr="_event.data.n"/>
        <if cond="n === 1">
          <log label="one"/>
        <elseif cond="n === 2"/>
          <log label="two"/>
        <else/>
          <sce:setverdict value="fail"/>
        </if>
      </transition>
      <transition sce:timeout="deadline" target="failed"/>
    </state>
    <state id="checking">
      <transition cond="n === limit" target="passed"/>
      <transition target="failed">
        <sce:setverdict value="fail"/>
      </transition>
    </state>
  </state>
  <final id="passed"/>
  <final id="failed"/>
</scxml>"#;

    /// The documents of a suite whose one test case runs the component
    /// `component` as its MTC.
    pub(crate) fn suite_of(component: &str) -> Result<Documents, InputError> {
        let files = Memory(BTreeMap::from([
            ("s.scxml", SUITE.to_string()),
            ("case.scxml", CASE.to_string()),
            ("config.scxml", CONFIG.to_string()),
            ("c.scxml", component.to_string()),
        ]));
        Documents::read(&files, "s.scxml")
    }

    fn documents() -> Documents {
        suite_of(COMPONENT).expect("documents")
    }

    fn node<'d>(documents: &'d Documents, name: &str) -> &'d Node {
        documents
            .document("c.scxml")
            .expect("the component")
            .nodes
            .iter()
            .find(|node| node.node == name)
            .unwrap_or_else(|| panic!("no node {name}"))
    }

    fn rules(documents: &Documents, name: &str) -> Vec<Rule> {
        let node = node(documents, name);
        required(node.kind, &node.facts)
    }

    /// The declarations: the component's E4 and E5 always, a port and a
    /// timer by E4, a variable by E4 unless it is a module parameter.
    #[test]
    fn a_components_declarations_are_named() {
        let documents = documents();
        assert_eq!(node(&documents, "scxml").line, None);
        assert_eq!(rules(&documents, "scxml"), [Rule::E4, Rule::E5]);
        assert_eq!(node(&documents, "sce:test-port 'sut'").line, Some(4));
        assert_eq!(rules(&documents, "sce:test-timer 'deadline'"), [Rule::E4]);
        assert!(rules(&documents, "data 'limit'").is_empty());
        assert_eq!(rules(&documents, "data 'n'"), [Rule::E4]);
        assert_eq!(rules(&documents, "invoke 'helper'"), [Rule::E11]);
    }

    /// A state that waits has its label and its `alt`, and each branch of it
    /// and its ancestors is an alternative; a state whose checks end has its
    /// label alone, its guarded transition a check, and its unguarded one
    /// written by no rule of its own.
    #[test]
    fn the_checks_and_the_alts_are_fixed_by_the_statechart() {
        let documents = documents();
        assert_eq!(rules(&documents, "state 'asking'"), [Rule::E6, Rule::E7]);
        assert_eq!(rules(&documents, "state 'checking'"), [Rule::E6]);
        assert!(rules(&documents, "state 'working'").is_empty());
        assert!(rules(&documents, "final 'passed'").is_empty());
        assert_eq!(
            rules(&documents, "state 'asking' / transition 1"),
            [Rule::E8]
        );
        assert_eq!(
            rules(&documents, "state 'asking' / transition 2"),
            [Rule::E8]
        );
        assert_eq!(
            rules(&documents, "state 'working' / transition 1"),
            [Rule::E8]
        );
        assert_eq!(
            rules(&documents, "state 'checking' / transition 1"),
            [Rule::E6]
        );
        assert!(rules(&documents, "state 'checking' / transition 2").is_empty());
        assert_eq!(
            rules(
                &documents,
                "state 'checking' / transition 2 / sce_setverdict 1"
            ),
            [Rule::E13]
        );
    }

    /// Content is named by its kind and position on the row of the state or
    /// transition that holds it, an `<if>`'s blocks under it; content of a
    /// written transition is required, an `<onentry>`'s is not.
    #[test]
    fn content_is_named_under_its_block() {
        let documents = documents();
        let transition = node(&documents, "state 'asking' / transition 1");
        assert_eq!(transition.line, Some(18));
        for name in [
            "state 'asking' / transition 1 / assign 1",
            "state 'asking' / transition 1 / if 2",
            "state 'asking' / transition 1 / if 2 / log 1",
            "state 'asking' / transition 1 / if 2 / elseif 1",
            "state 'asking' / transition 1 / if 2 / elseif 1 / log 1",
            "state 'asking' / transition 1 / if 2 / else / sce_setverdict 1",
        ] {
            assert_eq!(node(&documents, name).line, Some(18), "{name}");
            assert_eq!(rules(&documents, name).len(), 1, "{name}");
        }
        let entry = node(&documents, "state 'asking' / onentry 1 / sce_start_timer 2");
        assert_eq!(entry.line, Some(13), "the row of the state that holds it");
        assert!(rules(&documents, "state 'asking' / onentry 1 / send 1").is_empty());
        assert!(documents.holds(&crate::witness::Construct {
            document: "c.scxml".to_string(),
            node: "received reply".to_string(),
            line: Some(18),
        }));
        assert!(!documents.holds(&crate::witness::Construct {
            document: "c.scxml".to_string(),
            node: "received reply".to_string(),
            line: Some(29),
        }));
    }

    /// The statechart keeps its states in document order, each target and
    /// initial state resolved to one of them; a compound state that names
    /// no initial state enters its first child, and the root its first
    /// state.
    #[test]
    fn the_statechart_resolves_every_state_it_names() {
        let documents = documents();
        let chart = documents
            .document("c.scxml")
            .and_then(|document| document.statechart.as_ref())
            .expect("the statechart");
        let ids: Vec<&str> = chart.states.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["working", "asking", "checking", "passed", "failed"]);
        let at = |id: &str| chart.state(id).expect("a state");
        assert_eq!(chart.initial, [at("working")]);
        assert_eq!(chart.states[at("working")].initial, [at("asking")]);
        assert_eq!(
            chart.states[at("working")].children,
            [at("asking"), at("checking")]
        );
        assert_eq!(
            chart.states[at("asking")].transitions[0].targets,
            [at("checking")]
        );
        assert_eq!(chart.states[at("working")].invokes.len(), 1);
        assert_eq!(chart.states[at("asking")].onentry.len(), 2);
        let conditional = &chart.states[at("asking")].transitions[0].content[1];
        assert_eq!(conditional.then.len(), 1);
        assert_eq!(conditional.elseifs.len(), 1);
        assert_eq!(conditional.otherwise.len(), 1);
        assert!(chart.states[at("passed")].top_level_final);
        assert!(!chart.states[at("working")].labelled());
        assert!(chart.states[at("asking")].labelled());
        assert_eq!(chart.chain(at("asking")), [at("asking"), at("working")]);

        let first_child = COMPONENT
            .replace(" initial=\"asking\"", "")
            .replace(" initial=\"working\"", "");
        let documents = suite_of(&first_child).expect("documents");
        let chart = documents
            .document("c.scxml")
            .and_then(|document| document.statechart.as_ref())
            .expect("the statechart");
        assert_eq!(chart.initial, [0]);
        assert_eq!(chart.states[0].initial, [1]);
    }

    /// What a test component does not hold, and a target that names no
    /// state, are not read, each named.
    #[test]
    fn a_statechart_the_validator_does_not_model_is_refused_by_name() {
        for (written, named) in [
            (
                "<parallel id=\"both\"><state id=\"x\"/></parallel>",
                "<parallel>",
            ),
            ("<history id=\"back\"/>", "<history>"),
            ("<onentry><raise event=\"e\"/></onentry>", "<raise>"),
        ] {
            let changed = COMPONENT.replace(
                "    <state id=\"checking\">",
                &format!("    {written}\n    <state id=\"checking\">"),
            );
            let refused = suite_of(&changed);
            assert!(
                matches!(&refused, Err(InputError::Document { document, reason })
                    if document == "c.scxml" && reason.contains(named)),
                "{named}: {refused:?}"
            );
        }
        let dangling = COMPONENT.replace("target=\"checking\"", "target=\"nowhere\"");
        let refused = suite_of(&dangling);
        assert!(
            matches!(&refused, Err(InputError::Document { reason, .. }) if reason.contains("'nowhere'")),
            "{refused:?}"
        );
    }
}
