// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The print diagram's figure MODEL: which figures a statechart is drawn
//! as, what each one shows, and where each transition is described.
//!
//! # Why a model, before any geometry
//!
//! A statechart that goes into a printed specification is split into
//! figures by hierarchy: one figure per compound level, deeper compounds
//! drawn folded and opened in figures of their own. The two properties a
//! reviewer relies on — nothing is missing, and every transition is
//! described exactly once — are properties of that split, not of pixels.
//! So they are decided and tested HERE, on data, and every renderer (the
//! printed SVG, the GUI) draws from this model rather than re-deciding
//! it. A test on a rendered picture could not compare a figure byte for
//! byte; a test on this model can.
//!
//! The split rules come from a prototype built outside the product
//! (2026-09-28): a figure expands a compound down to `depth` levels; a
//! compound deeper than that is folded there and gets its own figure;
//! a transition is described in full only in its source's home figure,
//! and elsewhere appears as a brief arrow naming where it is described.
//!
//! # Determinism
//!
//! Figures, states and transitions are ordered by document order, with
//! the state id as the tie-break. Nothing here reads time, randomness or
//! the machine.

use crate::model::SCXMLModel;

pub mod metrics;

/// Which figure — named by the state it opens, never by a number, because a
/// number shifts when a state is added and a specification that says
/// "figure 4" would then point at the wrong picture.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(tag = "kind", content = "state", rename_all = "kebab-case")]
pub enum FigureName {
    /// The document's top level.
    Document,
    /// The inside of this compound or parallel state.
    Inside(String),
}

/// One transition of the document, as the model holds it: the state that
/// carries it, its position in that state's list, and one target (a
/// multi-target transition is one arrow per target).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct TransitionRef {
    pub source: String,
    pub index: usize,
    /// `None` for a targetless transition, which stays in its state and is
    /// described in that state's body rather than as an arrow.
    pub target: Option<String>,
}

/// One end of an arrow in a figure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", content = "state", rename_all = "kebab-case")]
pub enum End {
    /// A state drawn in this figure, as itself or folded.
    Shown(String),
    /// A state drawn in another figure, stood in for at the figure's edge.
    Elsewhere(String),
}

/// An arrow in a figure.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Arrow {
    pub from: End,
    pub to: End,
    /// The transitions this arrow stands for. One, when it is described
    /// here; possibly several, when it is a brief arrow for transitions
    /// described elsewhere that join the same two ends.
    pub transitions: Vec<TransitionRef>,
    /// Where each transition is described, in the order of `transitions`.
    /// Equal to this figure's name exactly when the arrow describes it.
    pub described_in: Vec<FigureName>,
    /// The real source, when the arrow leaves a folded box that contains
    /// it — "from unlocked" — so an arrow drawn from `released` does not
    /// read as leaving `released` itself.
    pub from_inside: Option<String>,
}

/// One figure of the split.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Figure {
    pub name: FigureName,
    /// States drawn as themselves here, in document order.
    pub states: Vec<String>,
    /// Compound states drawn folded here, each opened in its own figure.
    pub folded: Vec<String>,
    /// States of other figures an arrow here reaches, in order of first
    /// use — drawn in one column at the figure's edge.
    pub elsewhere: Vec<String>,
    pub arrows: Vec<Arrow>,
    /// Targetless transitions of the states drawn here, described in their
    /// state's body.
    pub in_place: Vec<TransitionRef>,
}

/// The whole split.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Diagram {
    pub figures: Vec<Figure>,
}

/// Split `model` into figures. Every figure is one CONTAINER — the document,
/// or a compound / parallel state — and what lies `depth` levels inside it;
/// a compound deeper than that is folded there and is the container of a
/// figure of its own (`depth >= 1`).
///
/// The document is a container like any state. So at depth 1 every figure
/// is flat — a container and its direct children, compound children folded
/// — and a layout never meets a box nested inside another box it must also
/// draw. The prototype expanded top-level compounds in the document's
/// figure, which needed Graphviz clusters to lay out.
pub fn split(model: &SCXMLModel, depth: usize) -> Diagram {
    let depth = depth.max(1);
    let tree = Tree::of(model);

    // Plan: which states each figure expands and folds, in document order.
    let mut plans: Vec<(FigureName, Vec<String>, Vec<String>)> = Vec::new();
    let mut queue: Vec<FigureName> = vec![FigureName::Document];
    while !queue.is_empty() {
        let name = queue.remove(0);
        let (mut expanded, mut folded) = (Vec::new(), Vec::new());
        let inside = match &name {
            FigureName::Document => tree.roots.clone(),
            FigureName::Inside(container) => {
                // The container is drawn as itself here: its frame, its body,
                // its own transitions.
                expanded.push(container.clone());
                tree.children.get(container).cloned().unwrap_or_default()
            }
        };
        for child in &inside {
            tree.plan(child, 0, depth, &mut expanded, &mut folded);
        }
        for f in &folded {
            queue.push(FigureName::Inside(f.clone()));
        }
        plans.push((name, expanded, folded));
    }

    // Home: the figure where a state is drawn as itself. A folded state is
    // drawn as itself in the figure that opens it, which the plan made the
    // first figure to expand it.
    let mut home: std::collections::BTreeMap<String, FigureName> = Default::default();
    for (name, expanded, _) in &plans {
        for sid in expanded {
            home.entry(sid.clone()).or_insert_with(|| name.clone());
        }
    }

    let figures = plans
        .iter()
        .map(|(name, expanded, folded)| tree.figure(model, name, expanded, folded, &home))
        .collect();
    Diagram { figures }
}

/// The state hierarchy in document order.
struct Tree {
    roots: Vec<String>,
    children: std::collections::BTreeMap<String, Vec<String>>,
    parent: std::collections::BTreeMap<String, String>,
}

impl Tree {
    fn of(model: &SCXMLModel) -> Self {
        let mut ordered: Vec<_> = model.states.values().collect();
        ordered.sort_by(|a, b| (a.document_order, &a.id).cmp(&(b.document_order, &b.id)));
        let mut tree = Tree {
            roots: Vec::new(),
            children: Default::default(),
            parent: Default::default(),
        };
        for s in &ordered {
            tree.children.entry(s.id.clone()).or_default();
            match &s.parent {
                Some(p) => {
                    tree.children
                        .entry(p.clone())
                        .or_default()
                        .push(s.id.clone());
                    tree.parent.insert(s.id.clone(), p.clone());
                }
                None => tree.roots.push(s.id.clone()),
            }
        }
        // §scxml-3.10: a `<history>` sits inside its parent state; it is a
        // transition target, so it is drawn where its parent is opened.
        let mut histories: Vec<_> = model.history_states.iter().collect();
        histories.sort_by(|a, b| a.0.cmp(b.0));
        for (id, h) in histories {
            tree.children
                .entry(h.parent.clone())
                .or_default()
                .push(id.clone());
            tree.children.entry(id.clone()).or_default();
            tree.parent.insert(id.clone(), h.parent.clone());
        }
        tree
    }

    fn has_children(&self, sid: &str) -> bool {
        self.children.get(sid).is_some_and(|c| !c.is_empty())
    }

    /// Place `sid`, which lies `level` levels inside its figure's container
    /// (0 = a direct child): folded when it is a compound at the figure's
    /// depth, otherwise drawn as itself with what lies inside it placed in
    /// turn.
    fn plan(
        &self,
        sid: &str,
        level: usize,
        depth: usize,
        expanded: &mut Vec<String>,
        folded: &mut Vec<String>,
    ) {
        if self.has_children(sid) && level + 1 >= depth {
            folded.push(sid.to_string());
            return;
        }
        expanded.push(sid.to_string());
        for child in self.children.get(sid).into_iter().flatten() {
            self.plan(child, level + 1, depth, expanded, folded);
        }
    }

    /// The nearest of `sid` and its ancestors that `shown` contains.
    fn visible(&self, sid: &str, shown: &std::collections::BTreeSet<&str>) -> Option<String> {
        let mut at = Some(sid.to_string());
        while let Some(s) = at {
            if shown.contains(s.as_str()) {
                return Some(s);
            }
            at = self.parent.get(&s).cloned();
        }
        None
    }

    fn figure(
        &self,
        model: &SCXMLModel,
        name: &FigureName,
        expanded: &[String],
        folded: &[String],
        home: &std::collections::BTreeMap<String, FigureName>,
    ) -> Figure {
        let shown: std::collections::BTreeSet<&str> =
            expanded.iter().chain(folded).map(String::as_str).collect();
        let folded_set: std::collections::BTreeSet<&str> =
            folded.iter().map(String::as_str).collect();
        let mut fig = Figure {
            name: name.clone(),
            states: expanded.to_vec(),
            folded: folded.to_vec(),
            elsewhere: Vec::new(),
            arrows: Vec::new(),
            in_place: Vec::new(),
        };
        let mut ordered: Vec<_> = model.states.values().collect();
        ordered.sort_by(|a, b| (a.document_order, &a.id).cmp(&(b.document_order, &b.id)));
        for s in ordered {
            let owner = &home[&s.id];
            for (index, t) in s.transitions.iter().enumerate() {
                if t.targets.is_empty() {
                    if owner == name {
                        fig.in_place.push(TransitionRef {
                            source: s.id.clone(),
                            index,
                            target: None,
                        });
                    }
                    continue;
                }
                for target in &t.targets {
                    let (src_v, tgt_v) =
                        (self.visible(&s.id, &shown), self.visible(target, &shown));
                    if src_v.is_none() && tgt_v.is_none() {
                        continue;
                    }
                    // Wholly inside one folded box: drawn in that box's figure.
                    if src_v.is_some()
                        && src_v == tgt_v
                        && folded_set.contains(src_v.as_deref().unwrap_or(""))
                    {
                        continue;
                    }
                    let end = |v: Option<String>, sid: &str, fig: &mut Figure| match v {
                        Some(v) => End::Shown(v),
                        None => {
                            if !fig.elsewhere.iter().any(|e| e == sid) {
                                fig.elsewhere.push(sid.to_string());
                            }
                            End::Elsewhere(sid.to_string())
                        }
                    };
                    let from_inside = src_v.as_ref().filter(|v| *v != &s.id).map(|_| s.id.clone());
                    let from = end(src_v, &s.id, &mut fig);
                    let to = end(tgt_v, target, &mut fig);
                    let r = TransitionRef {
                        source: s.id.clone(),
                        index,
                        target: Some(target.clone()),
                    };
                    // A brief arrow joins others described elsewhere between
                    // the same two ends; a described arrow is always its own.
                    if owner != name {
                        if let Some(a) = fig.arrows.iter_mut().find(|a| {
                            a.from == from && a.to == to && a.described_in.iter().all(|d| d != name)
                        }) {
                            a.transitions.push(r);
                            a.described_in.push(owner.clone());
                            continue;
                        }
                    }
                    fig.arrows.push(Arrow {
                        from,
                        to,
                        transitions: vec![r],
                        described_in: vec![owner.clone()],
                        from_inside,
                    });
                }
            }
        }
        fig
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn parse(body: &str) -> SCXMLModel {
        SCXMLParser::new()
            .parse_string(body, "diagram")
            .expect("parses")
    }

    const LOCK: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="released">
  <state id="released" initial="unlocked">
    <state id="unlocked">
      <transition event="lock.request" target="locked"/>
      <transition event="tick"/>
    </state>
    <state id="relocking" initial="waiting">
      <state id="waiting"><transition event="timer" target="armed"/></state>
      <state id="armed"><transition event="expire" target="locked"/></state>
    </state>
    <transition event="speed.high" target="relocking"/>
  </state>
  <state id="locked">
    <transition event="unlock.request" target="unlocked"/>
  </state>
</scxml>"#;

    /// Every transition — targetless included — is described in full in
    /// exactly one figure, and every state is drawn as itself in exactly
    /// one. Swept at every depth the document has, because the split
    /// changes with depth and a rule that holds at one depth can fail at
    /// another.
    #[test]
    fn nothing_is_missing_and_every_transition_is_described_once() {
        let m = parse(LOCK);
        let mut expected: Vec<TransitionRef> = Vec::new();
        for s in m.states.values() {
            for (index, t) in s.transitions.iter().enumerate() {
                if t.targets.is_empty() {
                    expected.push(TransitionRef {
                        source: s.id.clone(),
                        index,
                        target: None,
                    });
                }
                for target in &t.targets {
                    expected.push(TransitionRef {
                        source: s.id.clone(),
                        index,
                        target: Some(target.clone()),
                    });
                }
            }
        }
        expected.sort();
        assert!(expected.len() >= 6, "the fixture must exercise the rule");

        for depth in 1..=3 {
            let d = split(&m, depth);
            let mut described: Vec<TransitionRef> = Vec::new();
            let mut drawn: std::collections::BTreeMap<&str, usize> = Default::default();
            for fig in &d.figures {
                described.extend(fig.in_place.iter().cloned());
                for a in &fig.arrows {
                    for (t, at) in a.transitions.iter().zip(&a.described_in) {
                        if at == &fig.name {
                            described.push(t.clone());
                        }
                    }
                }
                for s in &fig.states {
                    *drawn.entry(s.as_str()).or_default() += 1;
                }
            }
            described.sort();
            assert_eq!(described, expected, "depth {depth}: {d:#?}");
            for s in m.states.keys() {
                assert!(
                    drawn.contains_key(s.as_str()),
                    "depth {depth}: {s} is in no figure"
                );
            }
        }
    }

    /// Depth 1 gives flat figures: the document holds `locked` and `released`
    /// folded; `released`'s figure holds `unlocked` and `relocking` folded.
    /// There, the arrow that leaves the `relocking` box for `locked` comes
    /// from `armed` inside it, and says so; it is described in
    /// `relocking`'s own figure.
    #[test]
    fn an_arrow_leaving_a_folded_box_names_its_real_source() {
        let d = split(&parse(LOCK), 1);
        let names: Vec<&FigureName> = d.figures.iter().map(|f| &f.name).collect();
        assert_eq!(
            names,
            [
                &FigureName::Document,
                &FigureName::Inside("released".into()),
                &FigureName::Inside("relocking".into()),
            ]
        );
        assert_eq!(d.figures[0].states, ["locked"]);
        assert_eq!(d.figures[0].folded, ["released"]);

        let released = &d.figures[1];
        assert_eq!(released.states, ["released", "unlocked"]);
        assert_eq!(released.folded, ["relocking"]);
        let leaving: Vec<&Arrow> = released
            .arrows
            .iter()
            .filter(|a| a.from == End::Shown("relocking".into()))
            .collect();
        assert_eq!(leaving.len(), 1, "{released:#?}");
        assert_eq!(leaving[0].from_inside.as_deref(), Some("armed"));
        assert_eq!(leaving[0].to, End::Elsewhere("locked".into()));
        assert_eq!(
            leaving[0].described_in,
            [FigureName::Inside("relocking".into())],
            "brief here, described where `armed` is drawn as itself"
        );

        // In `relocking`'s own figure both neighbours are elsewhere: the
        // `released` that enters it (`speed.high`) and the `locked` that
        // `armed` leaves for — gathered in order of first use.
        let inside = &d.figures[2];
        assert_eq!(inside.name, FigureName::Inside("relocking".into()));
        assert_eq!(inside.elsewhere, ["released", "locked"]);
    }

    /// The words a figure puts in a state's box are the page's words: the
    /// lines `action_lines` returns for an `<onentry>` appear, as written,
    /// on the statechart page a reviewer reads — in both lexicons.
    #[test]
    fn a_box_says_what_the_page_says() {
        use crate::forge::model::ForgeDocument;
        use crate::forge::page::{EN, KO};
        let m = parse(
            r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
                       initial="idle" datamodel="ecmascript">
  <state id="idle">
    <onentry>
      <send event="indicator.update" target="#_parent"/>
      <assign location="count" expr="count + 1"/>
    </onentry>
  </state>
</scxml>"##,
        );
        let block = &m.states["idle"].on_entry_blocks[0];
        for lexicon in [&EN, &KO] {
            let nodes = crate::forge::pseudo::render_nodes(
                &ForgeDocument::Statechart(Box::new(m.clone())),
                &crate::forge::pseudo::Deployment::default(),
            )
            .expect("the page renders");
            let page = crate::forge::page::write_page(&nodes, &crate::forge::page::Indent, lexicon)
                .expect("indent refuses nothing");
            let page_lines: Vec<&str> = page.lines().map(str::trim).collect();
            let lines = crate::forge::pseudo::action_lines(block, lexicon).expect("renders");
            assert!(lines.len() >= 2, "{lines:?}");
            for line in &lines {
                assert!(
                    page_lines.contains(&line.trim()),
                    "{line:?} is not on the page:\n{page}"
                );
            }
        }
    }

    /// Same input, same split — compared as serialized data, the form a
    /// renderer consumes.
    #[test]
    fn the_split_is_deterministic() {
        let m = parse(LOCK);
        let a = serde_json::to_string(&split(&m, 1)).expect("serializes");
        let b = serde_json::to_string(&split(&parse(LOCK), 1)).expect("serializes");
        assert_eq!(a, b);
    }
}
