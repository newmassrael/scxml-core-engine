// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! What a requirement's evidence actually depends on.
//!
//! Requirement-closure RFC §7a: the acceptance report puts one block per
//! requirement in front of a person, and that block has to contain
//! everything a violation could hide in. Choosing its contents by
//! `sce:req` does not do that.
//!
//! # ⭐ Why a closure and not "the nodes carrying the id"
//!
//! Measured over the real pair in `tests/fixtures/requirement_closure/`:
//! of 72 (requirement, dependency) pairs reachable from annotated
//! transitions, **39 lie on nodes that do not carry the id** — every
//! delay, every `<cancel>` of that delay, every target entry action,
//! every source exit action and every one of the transition's own
//! actions. Ten of the thirteen annotated requirements have at least
//! one.
//!
//! So a block chosen by id would stay byte-identical under RFC §7a.1's
//! own first example — retiming a delay from three seconds to five —
//! which is the completion test for this atomic. The fragment is the
//! DEPENDENCY CLOSURE of each cited node instead, over relations the IR
//! already carries.
//!
//! # The relations, and why each one is here
//!
//! ```text
//!   cited transition ──┬─ its own row: event, guard, target, actions
//!                      ├─ the <send> whose event IS this transition's
//!                      │  event — the timer that arms it. Without this
//!                      │  edge no delay is in the fragment at all.
//!                      ├─ the <cancel> naming that send — a requirement
//!                      │  about a timer is violated by cancelling it
//!                      │  as surely as by retiming it
//!                      ├─ the target state's entry actions
//!                      └─ the source state's exit actions
//! ```
//!
//! # One renderer, not two
//!
//! A fragment is a set of `node_path`s, and the report prints the
//! [`crate::transition_table`] rows those paths name. It never renders a
//! node itself. C's earlier round had to repair exactly that shape: the
//! table's action cell printed only the action's kind, so renaming a
//! timer's event left the page identical while the behaviour moved. A
//! second renderer is how the two drift apart again.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::model::{Action, SCXMLModel};
use crate::requirement_manifest::{
    classify, Classification, Outcome, RequirementEntry, RequirementManifest,
};
use crate::requirement_sidecar::RequirementSidecar;
use crate::requirements_report::{walk_nodes, ActionSite, NodeSubject};
use crate::transition_table::{transition_table, TransitionRow};

/// Why a node is in a requirement's fragment.
///
/// Carried rather than flattened away so a sweep can say which relation
/// it is exercising — "the arming send" is a finding a reader can act
/// on, where a bare node path is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Relation {
    /// The node carries the requirement id itself.
    Cited,
    /// A `<send>` that raises the event a cited transition waits for.
    ArmingSend,
    /// A `<cancel>` naming that send.
    CancelOfArmingSend,
    /// An entry action of a cited transition's target state.
    TargetEntry,
    /// An exit action of a cited transition's source state.
    SourceExit,
    /// Executable content of the cited transition itself.
    TransitionAction,
}

/// One node a requirement's evidence depends on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    /// Identity, as the shared walk assigns it — the same string the
    /// transition table puts in its own rows, so a fragment is a
    /// selection of that table rather than a second rendering.
    pub node_path: String,
    pub relation: Relation,
}

/// Every node the requirement's behaviour depends on, cited or not.
///
/// Returns an empty set when the document does not annotate `id` at
/// all; a caller reporting coverage must treat that as "no evidence"
/// rather than as a fragment that happens to be small.
pub fn fragment(model: &SCXMLModel, id: &str) -> Vec<Dependency> {
    let nodes = walk_nodes(model);

    // Pass 1 — what the document itself attributes to this requirement,
    // and the context each cited transition drags in with it.
    let mut found: Vec<Dependency> = Vec::new();
    let mut cited_paths: HashSet<String> = HashSet::new();
    let mut target_states: HashSet<&str> = HashSet::new();
    let mut source_states: HashSet<&str> = HashSet::new();
    let mut awaited_events: HashSet<&str> = HashSet::new();
    // Keyed by (state, position), never by state alone: a state holds
    // several transitions, and only the cited one's executable content
    // is this requirement's evidence. Keying on the state would pull in
    // a sibling transition's actions, and the fragment would then move
    // when something unrelated to the requirement changed — a page that
    // cries wolf is read exactly as carefully as one that stays silent.
    let mut cited_transitions: HashSet<(&str, usize)> = HashSet::new();

    for node in &nodes {
        if !node.record.requirement_ids.contains(&id) {
            continue;
        }
        cited_paths.insert(node.record.node_path.clone());
        found.push(Dependency {
            node_path: node.record.node_path.clone(),
            relation: Relation::Cited,
        });

        match &node.subject {
            NodeSubject::Transition {
                state,
                transition,
                index,
            } => {
                cited_transitions.insert((state.id.as_str(), *index));
                source_states.insert(state.id.as_str());
                if !transition.target.is_empty() {
                    target_states.insert(transition.target.as_str());
                }
                if !transition.event.is_empty() {
                    awaited_events.insert(transition.event.as_str());
                }
            }
            NodeSubject::State(state) => {
                // A requirement on a state is about being in it, so both
                // sides of that boundary are its evidence.
                source_states.insert(state.id.as_str());
                target_states.insert(state.id.as_str());
            }
            _ => {}
        }
    }

    // The ids of the sends that raise an awaited event. Collected before
    // pass 2 so a `<cancel>` can be matched against them wherever it
    // sits, which is rarely the state the requirement is annotated on.
    let mut arming_send_ids: BTreeSet<&str> = BTreeSet::new();
    for node in &nodes {
        if let NodeSubject::Action { action, .. } = &node.subject {
            if is_arming_send(action, &awaited_events) {
                for id in [action.id.as_str(), action.auto_send_id.as_str()] {
                    if !id.is_empty() {
                        arming_send_ids.insert(id);
                    }
                }
            }
        }
    }

    // Pass 2 — the nodes that carry no id and hold the behaviour anyway.
    for node in &nodes {
        let path = &node.record.node_path;
        if cited_paths.contains(path) {
            continue;
        }
        let NodeSubject::Action {
            state,
            action,
            site,
        } = &node.subject
        else {
            continue;
        };

        let relation = if is_arming_send(action, &awaited_events) {
            Some(Relation::ArmingSend)
        } else if is_cancel_of(action, &arming_send_ids) {
            Some(Relation::CancelOfArmingSend)
        } else {
            match site {
                ActionSite::Entry if target_states.contains(state.id.as_str()) => {
                    Some(Relation::TargetEntry)
                }
                ActionSite::Exit if source_states.contains(state.id.as_str()) => {
                    Some(Relation::SourceExit)
                }
                ActionSite::Transition { index }
                    if cited_transitions.contains(&(state.id.as_str(), *index)) =>
                {
                    Some(Relation::TransitionAction)
                }
                _ => None,
            }
        };

        if let Some(relation) = relation {
            found.push(Dependency {
                node_path: path.clone(),
                relation,
            });
        }
    }

    found.sort();
    found.dedup();
    found
}

/// The whole report, as the page a person reads in one sitting.
///
/// RFC §7a's three blocks: A is one block per requirement, B is the
/// risk surface, C is what the source says that nothing here claims.
///
/// ⚠ When `sidecar` is given, the output carries sentences from someone
/// else's document. It says so on its own face, because the only way
/// the manifest/sidecar split fails is a helpful person checking the
/// rendered page into the repository.
///
/// `variant` is printed under the title because an acceptance is of one
/// variant (Requirement-closure RFC §5.2d): a page that did not say which
/// could be signed for the base build and read as covering every build
/// composed on top of it.
pub fn render(
    model: &SCXMLModel,
    manifest: &RequirementManifest,
    sidecar: Option<&RequirementSidecar>,
    variant: &str,
) -> String {
    let classification = classify(model, manifest);
    let shown = Shown::statechart(model);

    let mut out = String::new();
    out.push_str(&format!(
        "ACCEPTANCE REPORT  {}@{}  <->  {}\n",
        manifest.doc_id, manifest.rev, model.name
    ));
    out.push_str(&format!("variant: {variant}\n"));
    // Why the document is a statechart, when it says (docs/SCE_ACCEPTED_
    // SUBSET.md §2.10.1). Printed only when present, so a report for a
    // document without one is byte-identical to what it was.
    push_kind_basis(&mut out, model.kind_basis.as_ref());
    push_cautions(&mut out, sidecar.is_some(), &classification);

    out.push_str("\nA. REQUIREMENTS\n");
    for entry in &manifest.requirements {
        push_requirement(&mut out, entry, sidecar);
        push_evidence(&mut out, shown.evidence(&entry.id));
    }

    let open = crate::open_matters::of_statechart(model);
    push_attention(&mut out, &open, &classification, shown.unclaimed());
    push_coverage(&mut out, &classification);
    out
}

/// What one document puts on the page: the rows a requirement's evidence is
/// read from, read once for every requirement of the page.
enum Shown<'a> {
    /// A statechart's transition table, and the model its fragments are taken
    /// over.
    Statechart {
        model: &'a SCXMLModel,
        table: Vec<TransitionRow>,
    },
    /// The review-table rows of a document of another kind, its root's first.
    Forge {
        nodes: Vec<crate::forge::requirement_nodes::ForgeReqNode>,
    },
}

impl<'a> Shown<'a> {
    fn statechart(model: &'a SCXMLModel) -> Self {
        Shown::Statechart {
            model,
            table: transition_table(model),
        }
    }

    fn forge(parsed: &crate::forge::model::ParsedForge) -> Self {
        Shown::Forge {
            nodes: crate::forge::requirement_nodes::requirement_nodes(parsed),
        }
    }

    /// The lines of evidence the document shows for `id`, or `None` when
    /// nothing in it depends on the requirement.
    ///
    /// A statechart shows the rows of the requirement's [`fragment`], each
    /// with the relation that put it there; a document of another kind shows
    /// its own review-table line of each node that claims the requirement,
    /// which is everything that node decides (a forge node has no transition
    /// row and no dependency closure).
    fn evidence(&self, id: &str) -> Option<Vec<String>> {
        match self {
            Shown::Statechart { model, table } => {
                let deps = fragment(model, id);
                if deps.is_empty() {
                    return None;
                }
                let by_path: BTreeMap<&str, &TransitionRow> = table
                    .iter()
                    .map(|row| (row.node_path.as_str(), row))
                    .collect();
                Some(
                    deps.iter()
                        .filter_map(|dep| {
                            let row = by_path.get(dep.node_path.as_str())?;
                            Some(format!(
                                "      {:<18} {} | {} | {} | {} | {} | {}\n",
                                format!("{:?}", dep.relation),
                                row.from,
                                row.event,
                                row.guard,
                                row.after,
                                row.to,
                                row.action
                            ))
                        })
                        .collect(),
                )
            }
            Shown::Forge { nodes } => {
                let lines: Vec<String> = nodes
                    .iter()
                    .filter(|row| {
                        row.requirements
                            .iter()
                            .any(|claimed| claimed.as_str() == id)
                    })
                    .map(|row| {
                        format!(
                            "      {:<18} {:<24} {}\n",
                            row.node_type, row.node_path, row.detail
                        )
                    })
                    .collect();
                (!lines.is_empty()).then_some(lines)
            }
        }
    }

    /// How many of the document's rows claim no requirement.
    fn unclaimed(&self) -> usize {
        match self {
            Shown::Statechart { table, .. } => {
                table.iter().filter(|row| row.is_unclaimed()).count()
            }
            Shown::Forge { nodes } => nodes
                .iter()
                .filter(|row| row.requirements.is_empty())
                .count(),
        }
    }
}

/// The warnings under the title: the page carries the source's own words, or
/// the design cites another revision of the specification than the manifest.
fn push_cautions(out: &mut String, sidecar: bool, classification: &Classification) {
    if sidecar {
        out.push_str(
            "⚠ carries verbatim text from the source document. A local artefact \
             for one sitting: do not commit it.\n",
        );
    }
    if let Some(note) = &classification.revision_note {
        out.push_str(&format!("⚠ {note}\n"));
    }
}

/// A requirement's id, its section, and what the source says for it.
fn push_requirement(
    out: &mut String,
    entry: &RequirementEntry,
    sidecar: Option<&RequirementSidecar>,
) {
    out.push_str(&format!("\n  {}", entry.id));
    if let Some(section) = &entry.section {
        out.push_str(&format!("   section {section}"));
    }
    out.push('\n');

    match sidecar.and_then(|side| side.sentence(&entry.id)) {
        Some(sentence) => out.push_str(&format!("    says: {sentence}\n")),
        None if sidecar.is_some() => out.push_str(
            "    says: (no sentence for this id in the sidecar — check the \
             source before accepting)\n",
        ),
        None => out.push_str("    says: (no sidecar supplied)\n"),
    }
}

/// The evidence of one requirement in a page of one document.
fn push_evidence(out: &mut String, lines: Option<Vec<String>>) {
    let Some(lines) = lines else {
        out.push_str("    evidence: (this document annotates nothing for it)\n");
        return;
    };
    out.push_str("    evidence:\n");
    for line in lines {
        out.push_str(&line);
    }
}

/// Block B: what the design leaves to a person, every requirement that is not
/// `implemented`, and how many rows claim nothing.
fn push_attention(
    out: &mut String,
    open: &[crate::open_matters::OpenMatter],
    classification: &Classification,
    unclaimed: usize,
) {
    out.push_str("\nB. NEEDING ATTENTION\n");
    let mut flagged = push_open_matters(out, open);
    for outcome in &classification.outcomes {
        if outcome.outcome == Outcome::Implemented {
            continue;
        }
        flagged += 1;
        out.push_str(&format!(
            "  {:<14} {}\n",
            outcome.outcome.as_str(),
            outcome.id
        ));
    }
    if unclaimed > 0 {
        out.push_str(&format!(
            "  {:<14} {unclaimed} element(s) the source never asked for\n",
            "unclaimed"
        ));
    }
    if flagged == 0 && unclaimed == 0 {
        out.push_str("  (nothing)\n");
    }
}

/// Block C: how many requirements each section of the source holds, a
/// section that holds none marked for opening.
fn push_coverage(out: &mut String, classification: &Classification) {
    out.push_str("\nC. SOURCE COVERAGE\n");
    for (section, count) in &classification.section_counts {
        let marker = if *count == 0 { "   <- open this" } else { "" };
        out.push_str(&format!("  {section:<12} {count}{marker}\n"));
    }
}

/// What the design leaves to a person, as the first lines of block B, and
/// how many lines that was.
///
/// First, because block A is one paragraph per requirement and a person
/// signing the page reads B for what could still be wrong: a question the
/// specification never answered outranks a requirement that reads
/// `implemented`. Nothing is written for a design that leaves nothing, so
/// a report for a finished design is byte for byte what it was.
///
/// ⚠ The sentences are [`crate::open_matters`]'s, not this function's: the
/// manifest and the acceptance record say the same thing in the same words,
/// and a second author of them is a second answer.
fn push_open_matters(out: &mut String, open: &[crate::open_matters::OpenMatter]) -> usize {
    for matter in open {
        let label = match matter.kind {
            crate::open_matters::OpenKind::Question => "open question",
            crate::open_matters::OpenKind::Assumed => "assumed value",
            crate::open_matters::OpenKind::HouseRule => "house rule",
            crate::open_matters::OpenKind::Interface => "open interface",
            crate::open_matters::OpenKind::SelfDelivered => "caller sends",
            crate::open_matters::OpenKind::Parent => "needs a parent",
            crate::open_matters::OpenKind::HostProcessor => "host processor",
        };
        out.push_str(&format!("  {label:<14} {}\n", matter.message));
    }
    open.len()
}

/// Why the document is its kind, when it says (docs/SCE_ACCEPTED_SUBSET.md
/// §2.10.1). Written only when present, so a report for a document without
/// one is byte-identical to what it was.
fn push_kind_basis(out: &mut String, basis: Option<&crate::forge::kind_basis::KindBasis>) {
    let Some(basis) = basis else {
        return;
    };
    out.push_str("kind basis:\n");
    for evidence in &basis.evidence {
        out.push_str(&format!("  evidence  {}", evidence.text));
        if let Some(compact) = evidence.provenance.as_ref().and_then(|p| p.to_compact()) {
            out.push_str(&format!("   ({compact})"));
        }
        out.push('\n');
    }
    for rejected in &basis.rejected {
        out.push_str(&format!(
            "  rather than {}  {}\n",
            rejected.kind.as_attr(),
            rejected.because
        ));
    }
    for marker in &basis.unresolved {
        out.push_str(&format!("  ⚠ kind left open ({})", marker.id));
        if let Some(reason) = &marker.reason {
            out.push_str(&format!("  {reason}"));
        }
        if !marker.candidates.is_empty() {
            out.push_str(&format!("  candidates: {}", marker.candidates.join(" ")));
        }
        out.push('\n');
    }
}

/// The acceptance report for a document of a kind other than statechart —
/// the page the owner reads before accepting a transform, a lookup, a
/// procedure.
///
/// The same three blocks as [`render`], answered by the same classifier
/// ([`crate::requirement_manifest::classify_document`]): Requirement-closure
/// RFC §7.1, one coverage implementation. What differs is the evidence a
/// requirement's block shows. A forge node has no transition row and no
/// dependency closure; its evidence is the node's own line in the kind's
/// review table — `entries[key=3]  3 -> DRIVE` — which is everything that
/// node decides.
///
/// Every kind can claim a requirement on its root (docs/adr/0106, item
/// G3), so a requirement the document claims as a whole has the root's
/// row as its evidence, and one it claims nowhere reads `missing` because
/// the document does not claim it.
pub fn render_forge(
    parsed: &crate::forge::model::ParsedForge,
    manifest: &RequirementManifest,
    sidecar: Option<&RequirementSidecar>,
    variant: &str,
    open: &[crate::open_matters::OpenMatter],
) -> String {
    let doc = &parsed.document;
    let classification = crate::requirement_manifest::classify_document(parsed, manifest);
    let shown = Shown::forge(parsed);

    let mut out = String::new();
    out.push_str(&format!(
        "ACCEPTANCE REPORT  {}@{}  <->  {}\n",
        manifest.doc_id,
        manifest.rev,
        doc.name()
    ));
    out.push_str(&format!("variant: {variant}\n"));
    out.push_str(&format!("kind: {}\n", doc.kind().as_attr()));
    push_kind_basis(&mut out, parsed.kind_basis.as_ref());
    push_cautions(&mut out, sidecar.is_some(), &classification);

    out.push_str("\nA. REQUIREMENTS\n");
    for entry in &manifest.requirements {
        push_requirement(&mut out, entry, sidecar);
        push_evidence(&mut out, shown.evidence(&entry.id));
    }

    push_attention(&mut out, open, &classification, shown.unclaimed());
    push_coverage(&mut out, &classification);
    out
}

/// One document of the design a report is rendered for.
pub struct ReportMember<'a> {
    /// The document as the page names it: the path it was named by, or the
    /// path its import resolved to.
    pub label: &'a str,
    pub document: ReportDocument<'a>,
}

/// A document of a design, read through the door of its own pipeline.
#[derive(Clone, Copy)]
pub enum ReportDocument<'a> {
    Statechart(&'a SCXMLModel),
    /// A document of another kind, and what its markers leave open: read by
    /// the caller from the expanded text, where the markers are, as
    /// [`render_forge`] is handed them.
    Forge(
        &'a crate::forge::model::ParsedForge,
        &'a [crate::open_matters::OpenMatter],
    ),
}

/// The report for a design of one document or several
/// ([`crate::design_set`]): the page a person reads to accept the design once.
///
/// ⚠ One document is answered by [`render`] or [`render_forge`], so its page is
/// byte for byte the page it always was. Several are answered together, as
/// `requirements` answers them ([`crate::requirement_manifest::classify_documents`]):
/// a requirement is met when a node of any document claims it, and an id the
/// manifest does not hold is `dangling` wherever it is cited. Measured document
/// by document, a requirement claimed in an algorithm the statechart imports
/// read "this document annotates nothing for it" on the statechart's page,
/// which is true of the statechart and says nothing about the design.
///
/// Every requirement then shows its evidence document by document, each block
/// led by the document it is in, since `states.idle` is a place in every
/// statechart; block B leads each open matter with its document for the same
/// reason, and counts the rows that claim nothing over the whole design.
pub fn render_design(
    members: &[ReportMember<'_>],
    manifest: &RequirementManifest,
    sidecar: Option<&RequirementSidecar>,
    variant: &str,
) -> String {
    if let [only] = members {
        return match only.document {
            ReportDocument::Statechart(model) => render(model, manifest, sidecar, variant),
            ReportDocument::Forge(parsed, open) => {
                render_forge(parsed, manifest, sidecar, variant, open)
            }
        };
    }
    let citations: Vec<(String, Vec<crate::requirement_manifest::OwnedCitation>)> = members
        .iter()
        .map(|member| {
            let claims = match member.document {
                ReportDocument::Statechart(model) => {
                    crate::requirement_manifest::citations_of_model(model)
                }
                ReportDocument::Forge(parsed, _) => {
                    crate::requirement_manifest::citations_of_document(parsed)
                }
            };
            (member.label.to_string(), claims)
        })
        .collect();
    let classification = crate::requirement_manifest::classify_documents(&citations, manifest);
    let shown: Vec<Shown<'_>> = members
        .iter()
        .map(|member| match member.document {
            ReportDocument::Statechart(model) => Shown::statechart(model),
            ReportDocument::Forge(parsed, _) => Shown::forge(parsed),
        })
        .collect();

    let mut out = String::new();
    out.push_str(&format!(
        "ACCEPTANCE REPORT  {}@{}  <->  {} documents\n",
        manifest.doc_id,
        manifest.rev,
        members.len()
    ));
    out.push_str(&format!("variant: {variant}\n"));
    for member in members {
        let (kind, name, basis) = match member.document {
            ReportDocument::Statechart(model) => {
                ("statechart", model.name.as_str(), model.kind_basis.as_ref())
            }
            ReportDocument::Forge(parsed, _) => (
                parsed.document.kind().as_attr(),
                parsed.document.name(),
                parsed.kind_basis.as_ref(),
            ),
        };
        out.push_str(&format!("document: {}  ({kind} '{name}')\n", member.label));
        push_kind_basis(&mut out, basis);
    }
    push_cautions(&mut out, sidecar.is_some(), &classification);

    out.push_str("\nA. REQUIREMENTS\n");
    for entry in &manifest.requirements {
        push_requirement(&mut out, entry, sidecar);
        let mut evidenced = false;
        for (member, shown) in members.iter().zip(&shown) {
            let Some(lines) = shown.evidence(&entry.id) else {
                continue;
            };
            evidenced = true;
            out.push_str(&format!("    evidence in {}:\n", member.label));
            for line in lines {
                out.push_str(&line);
            }
        }
        if !evidenced {
            out.push_str("    evidence: (no document of this design annotates anything for it)\n");
        }
    }

    let open: Vec<crate::open_matters::OpenMatter> = members
        .iter()
        .flat_map(|member| {
            let own = match member.document {
                ReportDocument::Statechart(model) => crate::open_matters::of_statechart(model),
                ReportDocument::Forge(_, open) => open.to_vec(),
            };
            crate::open_matters::in_document(member.label, own)
        })
        .collect();
    let unclaimed: usize = shown.iter().map(Shown::unclaimed).sum();
    push_attention(&mut out, &open, &classification, unclaimed);
    push_coverage(&mut out, &classification);
    out
}

/// A `<send>` that raises one of the events a cited transition waits
/// for — the timer arming it, when the send carries a delay.
fn is_arming_send(action: &Action, awaited: &HashSet<&str>) -> bool {
    action.action_type == "send"
        && !action.event.is_empty()
        && awaited.contains(action.event.as_str())
}

/// A `<cancel>` naming one of those sends.
///
/// ⚠ `sendidexpr` is an expression, not a value: it cannot be resolved
/// at build time, so a cancel written that way is included whenever the
/// requirement has any arming send at all. Leaving it out would drop the
/// one node that can stop the timer.
fn is_cancel_of(action: &Action, arming_send_ids: &BTreeSet<&str>) -> bool {
    if action.action_type != "cancel" {
        return false;
    }
    if !action.sendidexpr.is_empty() {
        return !arming_send_ids.is_empty();
    }
    arming_send_ids.contains(action.sendid.as_str())
}
