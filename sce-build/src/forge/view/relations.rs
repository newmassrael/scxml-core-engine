// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A document of relations (docs/adr/0151, D5): who may read a row, derived
//! from relation tuples by Zanzibar's namespace rewrites, read as rules of
//! Datalog.
//!
//! A tuple says `object_type:object#relation@subject`, its subject a principal
//! (`subject_type:subject`) or a set of them (`subject_type:subject#subject_relation`).
//! Its record declares the six fields by those names: the object's and the
//! subject's identities of one type, never optional; `object_type`, `relation` and
//! `subject_type` each a text or a value of an enum; and `subject_relation` a text,
//! empty for a principal. A tuple is a key of a set, and no part of a key is an
//! optional value (docs/adr/0108, D1).
//!
//! A relation reads the relations its rewrite names: a `<sce:computed>` the
//! relation of the same object, a `<sce:through>` every relation of its `then` name,
//! a `<sce:direct>` each set of principals its `subjects` names, through which the
//! subject of a tuple is expanded. A set a direct does not name is not expanded,
//! so what a relation reads is known before any tuple is: the build can tell a
//! relation that depends on its own exclusion, whose rules have no one least
//! fixed point (Apt, Blair and Walker 1988), and refuses it. Every other cycle
//! is a least fixed point, and the relations are evaluated in strata, each after
//! every stratum it excludes from.

use std::collections::BTreeMap;

use super::plan::{PlanBound, RelationId, RelationRule, RelationsPlan, SizeTerm, TuplesTable};
use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{
    ForgeKind, RelationRewrite, RewriteTerm, SceType, ViewModel, ViewRelation, ViewTableSource,
};
use crate::forge::value_type::{TypeScope, ValueType};

/// The fields of a relation tuple's record (D5).
pub(super) const TUPLE_FIELDS: &[&str] = &[
    "object_type",
    "object",
    "relation",
    "subject_type",
    "subject",
    "subject_relation",
];

/// One dependency of a relation on another: whether it is read through an
/// exclusion.
struct Edge {
    from: usize,
    to: usize,
    excluded: bool,
}

/// The plan of the document of relations `view`, whose relations are
/// `relations` and whose imports' records `types` holds.
pub(super) fn plan(
    view: &ViewModel,
    relations: &[ViewRelation],
    types: &TypeScope,
    document: &str,
) -> Result<RelationsPlan, Located<ForgeError>> {
    let table = view
        .tables
        .first()
        .expect("the parser holds a document of relations to one table");
    let (record, bound) = match &table.source {
        ViewTableSource::Type(ValueType::Set { key, max }) => match key.as_ref() {
            ValueType::Record { alias } => (alias.clone(), max.clone()),
            _ => unreachable!("the parser holds the tuples to a set of records"),
        },
        _ => unreachable!("the parser holds the tuples to a set of records"),
    };
    if let Some(rule) = tuple_shape_refusal(types, &record) {
        return Err(Located::new(
            ValidationError::AttributeRuleViolated {
                element: "<sce:table>".into(),
                attr: "type".into(),
                value: format!("record:{record}"),
                rule,
            }
            .into(),
            document,
            table.line,
            None,
        ));
    }

    let ids: Vec<RelationId> = relations
        .iter()
        .map(|r| RelationId {
            object: r.object.clone(),
            name: r.name.clone(),
        })
        .collect();
    let mut edges: Vec<Edge> = Vec::new();
    for (from, relation) in relations.iter().enumerate() {
        let mut reads: Vec<(RelationId, bool)> = Vec::new();
        let mut tuplesets: Vec<String> = Vec::new();
        reads_of(
            &relation.rewrite,
            &relation.object,
            false,
            &mut reads,
            &mut tuplesets,
        );
        let available = || ids.iter().map(ToString::to_string).collect::<Vec<_>>();
        for tupleset in &tuplesets {
            if !ids
                .iter()
                .any(|id| id.object == relation.object && id.name == *tupleset)
            {
                return Err(unknown_relation(
                    format!("{}#{tupleset}", relation.object),
                    available(),
                    document,
                    relation,
                ));
            }
        }
        for (read, excluded) in reads {
            // A through's `then` names a relation of whatever object a tuple names,
            // so it reads every relation of that name.
            let targets: Vec<usize> = if read.object.is_empty() {
                ids.iter()
                    .enumerate()
                    .filter(|(_, id)| id.name == read.name)
                    .map(|(at, _)| at)
                    .collect()
            } else {
                ids.iter().position(|id| *id == read).into_iter().collect()
            };
            if targets.is_empty() {
                let named = if read.object.is_empty() {
                    read.name.clone()
                } else {
                    read.to_string()
                };
                return Err(unknown_relation(named, available(), document, relation));
            }
            for to in targets {
                edges.push(Edge { from, to, excluded });
            }
        }
    }

    let component = components(ids.len(), &edges);
    if let Some(edge) = edges
        .iter()
        .find(|edge| edge.excluded && component[edge.from] == component[edge.to])
    {
        let mut cycle: Vec<String> = vec![ids[edge.from].to_string()];
        cycle.extend(
            path_within(edge.to, edge.from, &edges, &component)
                .into_iter()
                .map(|at| ids[at].to_string()),
        );
        let relation = &relations[edge.from];
        return Err(Located::new(
            ValidationError::ViewRelationCycleThroughExclusion {
                view: view.name.clone(),
                relation: ids[edge.from].to_string(),
                cycle,
            }
            .into(),
            document,
            relation.line,
            None,
        ));
    }

    // A relation's stratum is the most, over what it reads, of that one's
    // stratum, one more through an exclusion. Components complete in an order
    // where every component one reads completes before it.
    let count = component.iter().copied().max().map_or(0, |most| most + 1);
    let mut stratum_of_component: Vec<usize> = Vec::with_capacity(count);
    for c in 0..count {
        let stratum = edges
            .iter()
            .filter(|e| component[e.from] == c && component[e.to] != c)
            .map(|e| stratum_of_component[component[e.to]] + usize::from(e.excluded))
            .max()
            .unwrap_or(0);
        stratum_of_component.push(stratum);
    }
    let mut strata: BTreeMap<usize, Vec<RelationId>> = BTreeMap::new();
    for (at, id) in ids.iter().enumerate() {
        strata
            .entry(stratum_of_component[component[at]])
            .or_default()
            .push(id.clone());
    }

    let tuples = SizeTerm::Bound(PlanBound::of(&bound));
    Ok(RelationsPlan {
        tuples: TuplesTable {
            name: table.name.clone(),
            record,
            size: tuples.clone(),
        },
        relations: relations
            .iter()
            .map(|r| RelationRule {
                object: r.object.clone(),
                name: r.name.clone(),
                rewrite: r.rewrite.clone(),
            })
            .collect(),
        strata: strata.into_values().collect(),
        // A derived relation holds at most every object a tuple names with
        // every subject a tuple names (D8).
        size: SizeTerm::Product(vec![tuples.clone(), tuples]),
    })
}

/// Why the record `alias` is no relation tuple, where it is not one.
fn tuple_shape_refusal(types: &TypeScope, alias: &str) -> Option<String> {
    let Some(record) = types.records.get(alias) else {
        return Some(format!(
            "the record '{alias}' is read from no imported event schema"
        ));
    };
    let field = |name: &str| {
        record
            .fields
            .iter()
            .find(|(id, _)| id == name)
            .map(|(_, ty)| ty)
    };
    let missing: Vec<&str> = TUPLE_FIELDS
        .iter()
        .copied()
        .filter(|name| field(name).is_none())
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "a relation tuple declares {}; '{alias}' declares no {}",
            TUPLE_FIELDS.join(", "),
            missing.join(", ")
        ));
    }
    let is_name = |ty: &ValueType| {
        matches!(
            ty,
            ValueType::Text { .. } | ValueType::Scalar(SceType::Enum(_))
        )
    };
    for name in ["object_type", "relation", "subject_type"] {
        if !field(name).is_some_and(is_name) {
            return Some(format!(
                "the field {name} of a relation tuple names a type or a relation: a text or a \
                 value of an enum"
            ));
        }
    }
    if !matches!(field("subject_relation"), Some(ValueType::Text { .. })) {
        return Some(
            "the field subject_relation of a relation tuple is a text, empty where the subject \
             is a principal: a set of relation tuples is a set of keys, and an optional value is \
             no part of a key (docs/adr/0108, D1)"
                .to_string(),
        );
    }
    let object = field("object")?;
    let subject = field("subject")?;
    if object != subject || matches!(object, ValueType::Optional { .. }) {
        return Some(
            "the fields object and subject of a relation tuple are of one type, never optional: \
             a set of principals is named by an object"
                .to_string(),
        );
    }
    None
}

/// The refusal of a relation `relation` names and the document does not declare.
fn unknown_relation(
    named: String,
    available: Vec<String>,
    document: &str,
    relation: &ViewRelation,
) -> Located<ForgeError> {
    Located::new(
        ValidationError::InvalidReference {
            kind: ForgeKind::View,
            name: named,
            what: "relation of this document".into(),
            available,
        }
        .into(),
        document,
        relation.line,
        None,
    )
}

/// The relations `rewrite` of a relation of `object` reads, each with whether it
/// is read through an exclusion; a relation of every object is named with an
/// empty object. `tuplesets` collects the relations of `object` a through reads
/// the tuples of.
fn reads_of(
    rewrite: &RelationRewrite,
    object: &str,
    excluded: bool,
    reads: &mut Vec<(RelationId, bool)>,
    tuplesets: &mut Vec<String>,
) {
    for term in &rewrite.terms {
        match term {
            RewriteTerm::Direct { subjects } => {
                for set in subjects {
                    reads.push((
                        RelationId {
                            object: set.object.clone(),
                            name: set.relation.clone(),
                        },
                        excluded,
                    ));
                }
            }
            RewriteTerm::Computed { relation } => reads.push((
                RelationId {
                    object: object.to_string(),
                    name: relation.clone(),
                },
                excluded,
            )),
            RewriteTerm::Through { relation, then } => {
                tuplesets.push(relation.clone());
                reads.push((
                    RelationId {
                        object: String::new(),
                        name: then.clone(),
                    },
                    excluded,
                ));
            }
            RewriteTerm::All(inner) => reads_of(inner, object, excluded, reads, tuplesets),
        }
    }
    if let Some(inner) = &rewrite.excluded {
        reads_of(inner, object, true, reads, tuplesets);
    }
}

/// The strongly connected component of each of `count` relations (Tarjan 1972),
/// numbered in the order they complete: a component every relation of another
/// reads completes before it.
fn components(count: usize, edges: &[Edge]) -> Vec<usize> {
    struct Walk<'e> {
        edges: &'e [Edge],
        index: Vec<Option<usize>>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next: usize,
        component: Vec<usize>,
        completed: usize,
    }
    impl Walk<'_> {
        fn visit(&mut self, at: usize) {
            self.index[at] = Some(self.next);
            self.low[at] = self.next;
            self.next += 1;
            self.stack.push(at);
            self.on_stack[at] = true;
            let targets: Vec<usize> = self
                .edges
                .iter()
                .filter(|e| e.from == at)
                .map(|e| e.to)
                .collect();
            for to in targets {
                match self.index[to] {
                    None => {
                        self.visit(to);
                        self.low[at] = self.low[at].min(self.low[to]);
                    }
                    Some(index) if self.on_stack[to] => self.low[at] = self.low[at].min(index),
                    Some(_) => {}
                }
            }
            if Some(self.low[at]) == self.index[at] {
                while let Some(member) = self.stack.pop() {
                    self.on_stack[member] = false;
                    self.component[member] = self.completed;
                    if member == at {
                        break;
                    }
                }
                self.completed += 1;
            }
        }
    }
    let mut walk = Walk {
        edges,
        index: vec![None; count],
        low: vec![0; count],
        on_stack: vec![false; count],
        stack: Vec::new(),
        next: 0,
        component: vec![0; count],
        completed: 0,
    };
    for at in 0..count {
        if walk.index[at].is_none() {
            walk.visit(at);
        }
    }
    walk.component
}

/// The relations a path from `from` to `to` passes, within their one component,
/// `from` first and `to` last.
fn path_within(from: usize, to: usize, edges: &[Edge], component: &[usize]) -> Vec<usize> {
    let mut came_from: BTreeMap<usize, usize> = BTreeMap::new();
    let mut frontier: Vec<usize> = vec![from];
    let mut seen: Vec<usize> = vec![from];
    while !frontier.is_empty() && !seen.contains(&to) {
        let mut next: Vec<usize> = Vec::new();
        for at in frontier {
            for edge in edges.iter().filter(|e| {
                e.from == at && component[e.to] == component[from] && !seen.contains(&e.to)
            }) {
                seen.push(edge.to);
                came_from.insert(edge.to, at);
                next.push(edge.to);
            }
        }
        frontier = next;
    }
    let mut path: Vec<usize> = vec![to];
    let mut at = to;
    while at != from {
        match came_from.get(&at) {
            Some(previous) => {
                at = *previous;
                path.push(at);
            }
            None => break,
        }
    }
    path.reverse();
    path
}
