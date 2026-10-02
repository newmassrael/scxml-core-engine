// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which sites of a document send to its parent session — the fact that
//! the machine needs one.
//!
//! §scxml-6.2.4: `#_parent` names the session that invoked this one. A
//! machine started on its own has none, and a send to it raises
//! `error.communication` at runtime — while the document is valid SCXML,
//! passes every check, and builds. Measured 2026-09-28: a machine written
//! through the authoring MCP sent its notices to `#_parent`, and nothing
//! said so until someone read the document, because whether a parent
//! exists is a fact about the DEPLOYMENT and no single-document check can
//! decide it.
//!
//! So the answer is derived here and PUBLISHED, the way
//! [`crate::host_processor_analyzer`] publishes a processor the host must
//! supply: `sce-codegen` projects [`analyze`] onto the stdout manifest as
//! `needs_parent` + `parent_sends`, and a check that sees the composition
//! (a document set, a deployment) is where the answer can be judged.
//!
//! ⚠ Not `SCXMLModel::has_parent_communication`. That flag gates the
//! generated `ParentStateMachine` template parameter and is FORCED true by
//! `--as-child` on four backends, so it answers "was this built as a
//! child", not "does this document send to its parent".
//!
//! Scope: only a literal `target="#_parent"` is a site. A `targetexpr`
//! resolves at runtime and no build-time walk can name its value.

use crate::forge::error::SourceLocation;
use crate::model::SCXMLModel;

/// The parent session's target, spelled as §scxml-6.2.4 spells it.
pub const PARENT_TARGET: &str = "#_parent";

/// One `<send target="#_parent">` in the document.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ParentSend {
    /// The `event` attribute verbatim, or `None` when the site names its
    /// event with `eventexpr` — the parent is still required, and what it
    /// receives is decided at runtime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// The state whose executable content carries the send.
    pub state: String,
    /// Where in the source, in the `{file, line, col}` shape a diagnostic
    /// carries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SourceLocation>,
    /// The ids of the questions the specification leaves open that are marked
    /// ON this send (`sce:unresolved`, the kind that is a question and not a
    /// value chosen without an answer). A send to `#_parent` that carries one is
    /// a route nobody decided: the draft wrote `#_parent` because a send needs a
    /// target, and the owner has not said who the caller is. A consumer that
    /// plays the design reads it to say so, and not that the engine had no
    /// parent: an example that needs this send is blocked by that decision, which
    /// is not a defect of the design and not a fact about the machine that ran
    /// it. Empty when the send is written without one.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
}

/// Every `<send target="#_parent">` in `model`, ordered by where it is
/// written — the list is read by a person looking for the line to open.
/// Sites with no location sort last: absent position is not position zero.
pub fn analyze(model: &SCXMLModel) -> Vec<ParentSend> {
    let mut sends: Vec<ParentSend> = model
        .sends()
        .into_iter()
        .filter(|(_, send)| send.target == PARENT_TARGET)
        .map(|(state_id, send)| ParentSend {
            event: (!send.event.is_empty()).then(|| send.event.clone()),
            state: state_id.to_string(),
            location: send.source_location.clone(),
            decisions: send
                .unresolved
                .iter()
                .filter(|marker| marker.kind == crate::provenance::MarkerKind::Unresolved)
                .map(|marker| marker.id.clone())
                .collect(),
        })
        .collect();
    sends.sort_by_key(|s| {
        s.location
            .as_ref()
            .map(|l| (l.line.unwrap_or(0), l.col.unwrap_or(0)))
            .unwrap_or((u32::MAX, u32::MAX))
    });
    // The parser copies a history's default actions into
    // `State::initial_history_default_actions` of the state whose
    // `initial` names it, so one written site is walked twice — same
    // state, same location, adjacent after the sort.
    sends.dedup();
    sends
}

/// Whether the document sends to its parent at all.
///
/// Thin wrapper so no caller spells `!analyze(m).is_empty()` and a second,
/// subtly different predicate never gets written.
pub fn needs_parent(model: &SCXMLModel) -> bool {
    !analyze(model).is_empty()
}

/// [`analyze`], each location placed where the author wrote it — in the
/// fragment an `<xi:include>` spliced in, not in the expanded text — the
/// way every manifest cause is (`SCXMLModel::authored_location`).
pub fn records(model: &SCXMLModel) -> Vec<ParentSend> {
    let mut sends = analyze(model);
    for send in &mut sends {
        send.location = send.location.as_ref().map(|at| model.authored_location(at));
    }
    sends
}

/// What a document set can say about its members' parents.
#[derive(Debug)]
pub enum SetVerdict<'m> {
    /// Every member that sends to `#_parent` is invoked by another member.
    Satisfied,
    /// These members send to `#_parent` and no member invokes them, each
    /// with the first site that does.
    Orphaned(Vec<(&'m SCXMLModel, ParentSend)>),
    /// The set cannot be judged: some member names a child by expression
    /// (`srcexpr`), so whom it invokes is decided at runtime.
    Undecidable,
}

/// Judge a document set: which members send to a parent that no member
/// of the set can be.
///
/// A member is invoked when another member's `<invoke>` names it — by the
/// child name a static `<invoke src>` resolves to (its file stem, the name
/// the set registers it under), or by `#<name>` as a Mesh target. The
/// judgment is only as good as the set: a member invoked from OUTSIDE it
/// (a host, a peer in another build) looks orphaned here, which is why the
/// callers treat this as design advice (`--lint`), not as a refusal.
pub fn judge_set<'m>(models: &[&'m SCXMLModel]) -> SetVerdict<'m> {
    use crate::model::Invoke;
    let mut invoked: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for model in models {
        for state in model.states.values() {
            for invoke in &state.invokes {
                match invoke {
                    Invoke::Scxml(info) => {
                        invoked.insert(info.child_name.clone());
                        if let Some(peer) = info.src.strip_prefix('#') {
                            invoked.insert(peer.to_string());
                        }
                    }
                    Invoke::MeshRpc(info) => match info.target.src_literal() {
                        Some(src) => {
                            invoked.insert(src.trim_start_matches('#').to_string());
                        }
                        None => return SetVerdict::Undecidable,
                    },
                    Invoke::Hybrid(_) => return SetVerdict::Undecidable,
                    Invoke::Unsupported(_) => {}
                }
            }
        }
    }
    // `analyze`, not `records`: the caller places the refusal through
    // `SCXMLModel::locate`, which maps an expanded position to the authored
    // one itself — mapping it here first would map it twice.
    let orphaned: Vec<(&SCXMLModel, ParentSend)> = models
        .iter()
        .filter(|m| !invoked.contains(&m.name))
        .filter_map(|m| analyze(m).into_iter().next().map(|first| (*m, first)))
        .collect();
    if orphaned.is_empty() {
        SetVerdict::Satisfied
    } else {
        SetVerdict::Orphaned(orphaned)
    }
}

/// Refuse a document set's first member that sends to `#_parent` and that
/// no member invokes, as `scxml/parent-send-without-parent` located on the
/// send and carrying the enclosing anchor (SCE_ERROR_CONTRACT.md §2.1.2).
///
/// `members` pairs each model with the label its diagnostics carry. The
/// one construction of this refusal: the set compile and the anchor
/// contract's scenario runner both call it, so the record a test observes
/// is the record a build emits.
pub fn refuse_orphans(
    members: &[(&str, &SCXMLModel)],
) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
    let models: Vec<&SCXMLModel> = members.iter().map(|(_, m)| *m).collect();
    let SetVerdict::Orphaned(orphans) = judge_set(&models) else {
        return Ok(());
    };
    let (model, send) = &orphans[0];
    let label = members
        .iter()
        .find(|(_, m)| std::ptr::eq(*m, *model))
        .map(|(label, _)| *label)
        .unwrap_or_default();
    let refusal = crate::scxml_semantic::ScxmlSemanticError::ParentSendWithoutParent {
        machine: model.name.clone(),
        event: send.event.clone(),
    };
    Err(model.with_enclosing_anchor(model.locate(refusal.into(), send.location.as_ref(), label)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(body: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(body, "parent_send.scxml")
            .expect("parses")
    }

    /// Each place executable content sits, including nested, reaches the
    /// list — and only a literal `#_parent` does.
    #[test]
    fn every_parent_send_is_found_and_nothing_else() {
        // `r##` because the fixture itself contains `"#`.
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a">
    <onentry><send target="#_parent" event="entered"/></onentry>
    <transition event="go" target="b">
      <if cond="true"><send target="#_parent" eventexpr="'computed'"/></if>
    </transition>
  </state>
  <state id="b">
    <onexit><send target="#_internal" event="not.parent"/></onexit>
    <onentry><raise event="not.a.send"/></onentry>
  </state>
</scxml>"##,
        );
        let sends = analyze(&m);
        let found: Vec<(Option<&str>, &str)> = sends
            .iter()
            .map(|s| (s.event.as_deref(), s.state.as_str()))
            .collect();
        assert_eq!(found, [(Some("entered"), "a"), (None, "a")]);
        assert!(needs_parent(&m));
    }

    /// An inline child's `#_parent` is the child's parent — the document
    /// that invokes it — so it says nothing about whether THAT document
    /// needs one. Counting it would tell every parent of a talking child
    /// that it can only run as a child itself.
    #[test]
    fn an_inline_childs_send_to_its_parent_is_not_the_parents() {
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a">
    <invoke type="http://www.w3.org/TR/scxml/">
      <content>
        <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c">
          <state id="c"><onentry><send target="#_parent" event="child.ready"/></onentry></state>
        </scxml>
      </content>
    </invoke>
    <transition event="child.ready" target="done"/>
  </state>
  <final id="done"/>
</scxml>"##,
        );
        assert!(analyze(&m).is_empty(), "{:?}", analyze(&m));
        assert!(!needs_parent(&m));
    }

    #[test]
    fn a_machine_that_never_sends_to_its_parent_needs_none() {
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <state id="a"><onentry><send target="#_internal" event="x"/></onentry></state>
</scxml>"##,
        );
        assert!(analyze(&m).is_empty());
        assert!(!needs_parent(&m));
    }
}
