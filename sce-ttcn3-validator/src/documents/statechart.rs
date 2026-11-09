// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component (docs/SCE_TEST_PROFILE.md §11.5): its declarations, its
//! states, transitions, invocations and executable content as constructs,
//! and what its statechart fixes about which of them a rule writes
//! (§9.2 E6 to E8).
//!
//! At each atomic state that is no top-level `<final>` the export checks the
//! eventless transitions of the state and its ancestors, innermost first and
//! each state's in document order; a check with no guard ends them, and a
//! state whose checks do not end waits in an `alt` of the branches of the
//! state and its ancestors. So the documents fix, without the exit and
//! entry sets of any transition: which atomic states wait, which eventless
//! transitions are checked, which transitions are branches of an `alt`, and
//! the content of every transition written as one of these.

use super::{
    attribute, children, is_element, row, sce_attribute, Document, SCE_NAMESPACE, SCXML_NAMESPACE,
};
use crate::rules::{Action, Kind};
use crate::InputError;

/// The `type` of an `<invoke>` of a test component (§11.5).
const TEST_COMPONENT_INVOKE: &str = "http://sce.dev/ext/test-component";

/// The prefix of the event a done branch names (§4.2).
const DONE_INVOKE: &str = "done.invoke.";

/// A state, as the checks and the `alt`s read it.
struct State {
    /// Its construct, in the document's nodes.
    node: usize,
    parent: Option<usize>,
    /// An atomic state that is no top-level `<final>`: one with a label.
    labelled: bool,
    transitions: Vec<Transition>,
}

/// A transition, as the checks and the `alt`s read it.
struct Transition {
    node: usize,
    eventless: bool,
    guarded: bool,
    branch: bool,
    /// The constructs of its content, every one of them.
    content: Vec<usize>,
}

/// Name the constructs of the component whose root is `root`, and the facts
/// its statechart fixes.
pub(super) fn read(
    root: roxmltree::Node<'_, '_>,
    document: &mut Document,
) -> Result<(), InputError> {
    document.push("scxml".to_string(), None, Kind::Component);
    for port in children(root, SCE_NAMESPACE, "test-port") {
        let id = attribute(&port, "id").unwrap_or_default();
        document.push(
            format!("sce:test-port '{id}'"),
            Some(row(&port)),
            Kind::Port,
        );
    }
    for timer in children(root, SCE_NAMESPACE, "test-timer") {
        let id = attribute(&timer, "id").unwrap_or_default();
        document.push(
            format!("sce:test-timer '{id}'"),
            Some(row(&timer)),
            Kind::Timer,
        );
    }
    datamodel(root, document);
    let mut states = Vec::new();
    for child in state_children(root) {
        state(child, None, document, &mut states)?;
    }
    settle(&states, document);
    Ok(())
}

/// The child states of `element`.
fn state_children<'a, 'input>(
    element: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    element
        .children()
        .filter(|child| {
            ["state", "final", "parallel"]
                .iter()
                .any(|local| is_element(child, SCXML_NAMESPACE, local))
        })
        .collect()
}

/// The variables `element`'s `<datamodel>` declares, each one that is a
/// module parameter said so.
fn datamodel(element: roxmltree::Node<'_, '_>, document: &mut Document) {
    for model in children(element, SCXML_NAMESPACE, "datamodel") {
        for data in children(model, SCXML_NAMESPACE, "data") {
            let id = attribute(&data, "id").unwrap_or_default();
            let index = document.push(format!("data '{id}'"), Some(row(&data)), Kind::Variable);
            document.nodes[index].facts.module_parameter = matches!(
                sce_attribute(&data, "module-parameter").as_deref(),
                Some("true" | "1")
            );
        }
    }
}

/// The state `element`, child of `parent`, and every state below it.
fn state(
    element: roxmltree::Node<'_, '_>,
    parent: Option<usize>,
    document: &mut Document,
    states: &mut Vec<State>,
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
    let atomic = inner.is_empty() && tag != "parallel";
    let kind = if top_level_final {
        Kind::TopLevelFinal
    } else if atomic {
        Kind::AtomicState
    } else {
        Kind::CompoundState
    };
    let index = document.push(node.clone(), line, kind);

    datamodel(element, document);
    for (position, block) in children(element, SCXML_NAMESPACE, "onentry")
        .into_iter()
        .enumerate()
    {
        let prefix = format!("{node} / onentry {}", position + 1);
        actions(
            &element_children(block),
            &prefix,
            line,
            document,
            &mut Vec::new(),
        );
    }
    for (position, block) in children(element, SCXML_NAMESPACE, "onexit")
        .into_iter()
        .enumerate()
    {
        let prefix = format!("{node} / onexit {}", position + 1);
        actions(
            &element_children(block),
            &prefix,
            line,
            document,
            &mut Vec::new(),
        );
    }
    for initial in children(element, SCXML_NAMESPACE, "initial") {
        for transition in children(initial, SCXML_NAMESPACE, "transition") {
            let prefix = format!("{node} / initial");
            actions(
                &element_children(transition),
                &prefix,
                line,
                document,
                &mut Vec::new(),
            );
        }
    }
    for invoke in children(element, SCXML_NAMESPACE, "invoke") {
        if attribute(&invoke, "type").as_deref() != Some(TEST_COMPONENT_INVOKE) {
            continue;
        }
        document.links.invokes = true;
        let id = attribute(&invoke, "id").unwrap_or_default();
        document.push(format!("invoke '{id}'"), Some(row(&invoke)), Kind::Invoke);
    }

    let mut transitions = Vec::new();
    for (position, transition) in children(element, SCXML_NAMESPACE, "transition")
        .into_iter()
        .enumerate()
    {
        let transition_node = format!("{node} / transition {}", position + 1);
        let transition_line = Some(row(&transition));
        let node_index = document.push(transition_node.clone(), transition_line, Kind::Transition);
        let event = attribute(&transition, "event").unwrap_or_default();
        let receive = sce_attribute(&transition, "receive").is_some();
        let timeout = sce_attribute(&transition, "timeout").is_some();
        let template = sce_attribute(&transition, "test-template").is_some();
        let args = sce_attribute(&transition, "template-args")
            .is_some_and(|args| args.split_whitespace().next().is_some());
        let profile_branch = receive || timeout || template || args;
        if receive && template {
            document.received.insert(row(&transition));
        }
        let mut content = Vec::new();
        actions(
            &element_children(transition),
            &transition_node,
            transition_line,
            document,
            &mut content,
        );
        transitions.push(Transition {
            node: node_index,
            eventless: event.is_empty() && !profile_branch,
            guarded: attribute(&transition, "cond").is_some_and(|cond| !cond.is_empty()),
            branch: profile_branch || event.starts_with(DONE_INVOKE),
            content,
        });
    }
    let at = states.len();
    states.push(State {
        node: index,
        parent,
        labelled: atomic && !top_level_final,
        transitions,
    });
    for child in inner {
        state(child, Some(at), document, states)?;
    }
    Ok(())
}

/// The element children of `node`.
fn element_children<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    node.children().filter(|child| child.is_element()).collect()
}

/// The element `element` is as executable content: the name its node gives
/// it and the rule that translates it, or `None` for an element that is no
/// content.
fn action(element: &roxmltree::Node<'_, '_>) -> Option<(String, Action)> {
    let name = element.tag_name().name();
    match element.tag_name().namespace() {
        Some(SCXML_NAMESPACE) => Some((
            name.to_string(),
            match name {
                "send" => Action::Send,
                "assign" => Action::Assign,
                "log" => Action::Log,
                "if" => Action::If,
                _ => Action::Other,
            },
        )),
        // SCE's own statements, named as the statechart reader names them:
        // a test component's three (`sce_setverdict`, `sce_start_timer`,
        // `sce_stop_timer`) and the others of the namespace, which no rule
        // translates and which take a position all the same.
        Some(SCE_NAMESPACE) => match name {
            "setverdict" => Some(("sce_setverdict".to_string(), Action::Setverdict)),
            "start-timer" => Some(("sce_start_timer".to_string(), Action::Timer)),
            "stop-timer" => Some(("sce_stop_timer".to_string(), Action::Timer)),
            "append" | "clear" | "lift" | "render" => Some((format!("sce_{name}"), Action::Other)),
            "action" => Some((name.to_string(), Action::Other)),
            _ => None,
        },
        _ => None,
    }
}

/// The content `elements`, each named after `prefix` by its kind and its
/// position from 1 among them, on `line`; every construct added to
/// `content`.
fn actions(
    elements: &[roxmltree::Node<'_, '_>],
    prefix: &str,
    line: Option<u32>,
    document: &mut Document,
    content: &mut Vec<usize>,
) {
    let mut position = 0;
    for element in elements {
        let Some((name, action)) = action(element) else {
            continue;
        };
        position += 1;
        let node = format!("{prefix} / {name} {position}");
        content.push(document.push(node.clone(), line, Kind::Action(action)));
        if action == Action::If {
            conditional(*element, &node, line, document, content);
        }
    }
}

/// The blocks of the `<if>` `element`, named `node`: its own content up to
/// its first `<elseif>` or `<else>`, each `<elseif>` and its content, and the
/// content of its `<else>`.
fn conditional(
    element: roxmltree::Node<'_, '_>,
    node: &str,
    line: Option<u32>,
    document: &mut Document,
    content: &mut Vec<usize>,
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
    actions(&then, node, line, document, content);
    for (position, part) in elseifs.iter().enumerate() {
        let branch = format!("{node} / elseif {}", position + 1);
        content.push(document.push(branch.clone(), line, Kind::Elseif));
        actions(part, &branch, line, document, content);
    }
    actions(
        &otherwise,
        &format!("{node} / else"),
        line,
        document,
        content,
    );
}

/// The facts the checks and the `alt`s fix (§9.2 E6, E7): at each labelled
/// state, the eventless transitions of it and its ancestors are checked,
/// innermost first, until one with no guard ends the checks; a state whose
/// checks do not end waits, and each branch of it and its ancestors is an
/// alternative of its `alt`. A transition written so has its content written
/// too.
fn settle(states: &[State], document: &mut Document) {
    for (leaf, leaf_state) in states.iter().enumerate() {
        if !leaf_state.labelled {
            continue;
        }
        let mut chain = vec![leaf];
        while let Some(parent) = chain.last().and_then(|&state| states[state].parent) {
            chain.push(parent);
        }
        let mut ended = false;
        'checks: for &state in &chain {
            for transition in &states[state].transitions {
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
            for transition in states[state].transitions.iter().filter(|t| t.branch) {
                document.nodes[transition.node].facts.branch = true;
                written(transition, document);
            }
        }
    }
}

/// Mark the content of `transition` written.
fn written(transition: &Transition, document: &mut Document) {
    for &index in &transition.content {
        document.nodes[index].facts.written = true;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::super::tests::Memory;
    use super::super::{Documents, Node};
    use crate::rules::{required, Rule};

    const SUITE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-suite" name="s">
  <sce:import kind="test-case" src="case.scxml" as="Case"/>
  <sce:execute test-case="Case"/>
</scxml>"#;

    const CASE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="test-case" name="case"
       sce:configuration="Config">
  <sce:import kind="test-configuration" src="config.scxml" as="Config"/>
</scxml>"#;

    const CONFIG: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
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

    fn documents() -> Documents {
        let files = Memory(BTreeMap::from([
            ("s.scxml", SUITE),
            ("case.scxml", CASE),
            ("config.scxml", CONFIG),
            ("c.scxml", COMPONENT),
        ]));
        Documents::read(&files, "s.scxml").expect("documents")
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
}
