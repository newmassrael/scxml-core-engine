// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The plan of a view (docs/adr/0151, D2, D5, D6 and D8): plain data, made by
//! the build where the document is read and handed whole to whatever evaluates
//! it, the reference evaluator, the translator of the model and the lane that
//! prints the maintenance circuit, none of which reads the document again.
//!
//! A plan of a query is a list of operators, each reading only operators
//! before it, the last a cut of a total order. Each knows the key of the rows
//! it gives, inferred by the rules of functional dependency, and the most rows
//! it gives, a term of the bounds the tables and the result are declared with.
//! The one rule every plan keeps is the filter's (D6): a table is read by a
//! scan, a scan is read by the filter of its table alone, and the filter is
//! the semi-join of the scan's rows with the objects the viewer may read them
//! by, so no operator reads a row its viewer may not.
//!
//! A plan of a view with a viewer is lifted over every viewer at once (D6): the
//! filter joins each table's rows with the table of viewers, so every operator
//! after it carries the viewer as the column [`VIEWER`]. A join of two
//! relations that carry it joins on equal viewers as well as on its own columns,
//! and every group, order, nest and the result have it as the first column of
//! their key: no operator meets the rows one viewer may read with the rows
//! another may. Every size of an operator is a term per viewer; the part it is of
//! the state is that term times `max_viewers` (D8).
//!
//! The order is the one total comparison of D2 for every type: an integer by its
//! number, `false` before `true`, a floating number by IEEE 754-2019 totalOrder,
//! a text by its code points and bytes as unsigned bytes (a prefix first), an
//! enum by its declared order, a record by its fields in order, a list by its
//! elements, and an absent value where each key's `absent_first` puts it.
//!
//! [`ViewPlan::verify`] holds a plan to the filter's rule and the viewer's, a plan
//! made by the build and one read from anywhere else alike.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::forge::model::{RelationRewrite, ViewJoinKind, ViewerOf};

/// The position of an operator in [`QueryPlan::operators`].
pub type OperatorId = usize;

/// The column a lifted plan carries the viewer in (D6). A name no author writes:
/// a system name begins with `_`, as `_event` and `_x` do.
pub const VIEWER: &str = "_viewer";

/// The plan of one view document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewPlan {
    /// The view's name.
    pub view: String,
    /// Whose value it is: a value per viewer, or one value (D6).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewer: Option<ViewerOf>,
    /// Every record a table, the result or a nest is of, by the alias the
    /// document imports it under, each with its fields and their types as the
    /// type grammar spells them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub records: BTreeMap<String, Vec<PlanColumn>>,
    /// The plans of the views it reads, by the alias it imports each under: a
    /// table of another view's result (D3), and a document of relations a
    /// filter reads (D5).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<ReadView>,
    pub body: PlanBody,
}

/// A view a plan reads, by the alias it is imported under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadView {
    pub alias: String,
    pub plan: ViewPlan,
}

/// A query's plan, or a document of relations'.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "body", rename_all = "kebab-case")]
pub enum PlanBody {
    Query(Box<QueryPlan>),
    Relations(RelationsPlan),
}

/// The operators of a query, the last of them its value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Whether it is lifted over a table of viewers: the view has a viewer
    /// (D6).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lifted: bool,
    pub tables: Vec<PlanTable>,
    /// Each reads only operators before it.
    pub operators: Vec<PlanNode>,
    /// The operator whose rows are the value: the cut of the total order.
    pub output: OperatorId,
    pub result: PlanResult,
    /// What the view holds between two changes, part by part, each a term of
    /// the named bounds (D8; docs/adr/0118, D3).
    pub state: Vec<StatePart>,
}

/// An input table of a query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTable {
    pub name: String,
    pub source: TableSource,
    pub visibility: PlanVisibility,
    /// The most rows it holds.
    pub size: SizeTerm,
}

/// What a table's rows are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "kebab-case")]
pub enum TableSource {
    /// A map of the machine: a row is a key and its value.
    Map { key: String, value: String },
    /// A set of the machine: a row is a key.
    Set { key: String },
    /// The result of the view read under the alias `view`: a row is a record of
    /// it, keyed by that view's key.
    View { view: String, record: String },
}

/// Who may read a row of a table: what its filter joins it with (D6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "visible", rename_all = "kebab-case")]
pub enum PlanVisibility {
    /// Every viewer may read every row; the filter passes each row.
    Public,
    /// The rows are another view's of the same viewer, computed from what that
    /// viewer may read, passed on (D3).
    PassedOn { view: String },
    /// A row may be read by the viewers that stand in `relation` to the object of
    /// the type `object_type` that `object` names, the relation derived by the
    /// document of relations read under the alias `relations`. `object` reads
    /// the row's `key` and `row`.
    /// `at`, where the table names one, is the position of the relations'
    /// stream the row's writer read: a row whose `at` is above the position the
    /// relations were followed to is visible to no viewer (D6, D9).
    Relation {
        relations: String,
        relation: String,
        object_type: String,
        object: PlanExpr,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        at: Option<PlanExpr>,
    },
}

/// One operator, the rows it gives and what the build knows of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNode {
    pub op: Operator,
    /// The names its rows are read by and their types: an alias's `key` and
    /// `row` before a projection, the columns after it.
    pub columns: Vec<PlanColumn>,
    /// The columns that tell its rows apart, inferred by functional
    /// dependency (D2); empty where the build infers none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key: Vec<String>,
    /// The most rows it gives, for one viewer where the plan is lifted.
    pub size: SizeTerm,
    /// Whether its rows carry the viewer, as [`VIEWER`] (D6): every operator
    /// from a filter on, where the plan is lifted.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub carries_viewer: bool,
}

/// A column: a name and its type, as the type grammar spells it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanColumn {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

/// An expression of SCE's expression language over the names in scope, as the
/// document writes it; the build has typed it and proved it total.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlanExpr {
    pub text: String,
}

impl PlanExpr {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.trim().to_string(),
        }
    }
}

/// The operators of a query (D1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum Operator {
    /// The rows of a table, named by `alias`. Read by its filter alone.
    Scan { table: String, alias: String },
    /// The semi-join of a scan's rows with the objects the viewer may read them
    /// by (D6): the first operator over every table.
    Filter { input: OperatorId, table: String },
    /// The pairs of rows `on` holds of; a left join keeps a left row no right
    /// row matches, its right side absent. `by_right_key` says `on` equates the
    /// right side's key with a value of the left side, so a left row meets at
    /// most one right row. `on_viewers` says it joins on equal viewers as well,
    /// which a join of two relations that carry the viewer does (D6).
    Join {
        left: OperatorId,
        right: OperatorId,
        kind: ViewJoinKind,
        on: PlanExpr,
        by_right_key: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        on_viewers: bool,
    },
    /// The rows `cond` holds of.
    Where { input: OperatorId, cond: PlanExpr },
    /// One row per value of `by`, of the columns `outputs` give.
    Group {
        input: OperatorId,
        by: Vec<PlanExpr>,
        outputs: Vec<PlanAggregate>,
    },
    /// One row per row, of the columns `fields` give.
    Select {
        input: OperatorId,
        fields: Vec<PlanField>,
    },
    /// The rows in a total order: the keys written, then the result's key.
    Order {
        input: OperatorId,
        by: Vec<PlanOrderKey>,
    },
    /// The first rows of the order, at most `bound`, and the count of all of
    /// them (D2).
    Cut { input: OperatorId, bound: PlanBound },
}

impl Operator {
    /// The operators it reads.
    pub fn inputs(&self) -> Vec<OperatorId> {
        match self {
            Self::Scan { .. } => Vec::new(),
            Self::Filter { input, .. }
            | Self::Where { input, .. }
            | Self::Group { input, .. }
            | Self::Select { input, .. }
            | Self::Order { input, .. }
            | Self::Cut { input, .. } => vec![*input],
            Self::Join { left, right, .. } => vec![*left, *right],
        }
    }
}

/// A named column and the expression it is computed by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanField {
    pub name: String,
    pub expr: PlanExpr,
}

/// A column a group gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "aggregate", rename_all = "kebab-case")]
pub enum PlanAggregate {
    /// A value the group's `by` determines.
    Field(PlanField),
    /// The rows of the group.
    Count {
        name: String,
    },
    Sum(PlanSum),
    Min(PlanField),
    Max(PlanField),
    /// The group's rows, each as a record, in their total order, cut at the
    /// bound; the group's rows are its total.
    Nest(PlanNest),
}

/// A sum of a group, and what it is kept in (D1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSum {
    pub name: String,
    pub expr: PlanExpr,
    pub accumulator: Accumulator,
}

/// What a sum is kept in between two changes, so that it is a function of the
/// group's rows, maintained or recomputed (docs/adr/0151, D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "accumulator", rename_all = "kebab-case")]
pub enum Accumulator {
    /// An integer of `bits` bits, signed or not, that holds every sum the group's
    /// largest count of the column's values reaches.
    Integer { signed: bool, bits: u32 },
    /// A long accumulator (Kulisch and Miranker 1981): a fixed-point register of
    /// `bits` bits that holds every sum of the group's largest count of finite
    /// values of the column's IEEE 754 binary format exactly, its infinities and
    /// NaNs counted beside it, and rounded once, to nearest with ties to even,
    /// into a floating number of `format` bits when it is read.
    Floating { bits: u32, format: u8 },
}

/// The rows of a group, as the record the build generates for a nest (D2):
/// `rows`, the first of them in the order, at most the bound, each a record of
/// `record`, and `total`, the count of them all, an unsigned integer of
/// `total_bits` bits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanNest {
    pub name: String,
    /// The alias of the record each row is.
    pub record: String,
    pub fields: Vec<PlanField>,
    /// The viewer first where the plan is lifted, then the order written, then
    /// the key of the rows the group holds.
    pub order: Vec<PlanOrderKey>,
    pub bound: PlanBound,
    pub total_bits: u8,
}

/// One key of a total order, by the comparison of D2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOrderKey {
    pub expr: PlanExpr,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub descending: bool,
    /// Whether an absent value is before every present one in the order as it
    /// is read, `desc` applied: `absent first` or `absent last` where written,
    /// and otherwise first under `asc` and last under `desc` (D2).
    pub absent_first: bool,
    /// Appended by the build from the key, so no two rows are tied (D2).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_key: bool,
}

/// A bound by name (D8): the number the build gives it, and the name a
/// deployment moves it by, where it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanBound {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub max: u32,
}

impl PlanBound {
    /// The bound of a type, as the plan names it.
    pub fn of(bound: &crate::forge::value_type::Bound) -> Self {
        Self {
            name: bound.limit.clone(),
            max: bound.max,
        }
    }
}

/// The result of a query: its record, its key as the document names it, its
/// bound, and the width of its `total`, an unsigned integer of `total_bits`
/// bits that holds the largest count the plan's terms allow (D2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanResult {
    pub record: String,
    pub key: Vec<String>,
    pub bound: PlanBound,
    pub total_bits: u8,
}

/// A part of what a view holds between two changes, and its most rows: `size`,
/// or `size` for each viewer kept, at most `max_viewers` of them, where
/// `per_viewer` says so (D8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePart {
    pub part: String,
    pub size: SizeTerm,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub per_viewer: bool,
}

/// The fewest bits of an unsigned integer of 8, 16, 32 or 64 bits that holds
/// `most`: the width of a count the build proves (D2).
pub fn count_bits(most: u128) -> u8 {
    match most {
        0..=0xff => 8,
        0x100..=0xffff => 16,
        0x1_0000..=0xffff_ffff => 32,
        _ => 64,
    }
}

/// A count of rows as a term of the named bounds (D8): what a deployment that
/// moves a bound by its name moves with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeTerm {
    Bound(PlanBound),
    Product(Vec<SizeTerm>),
    Sum(Vec<SizeTerm>),
    Min(Vec<SizeTerm>),
}

impl SizeTerm {
    /// The term of one row.
    pub fn one() -> Self {
        Self::Bound(PlanBound { name: None, max: 1 })
    }

    /// The term's value at the numbers the build gives its bounds.
    pub fn most(&self) -> u128 {
        match self {
            Self::Bound(bound) => u128::from(bound.max),
            Self::Product(terms) => terms
                .iter()
                .fold(1u128, |product, term| product.saturating_mul(term.most())),
            Self::Sum(terms) => terms
                .iter()
                .fold(0u128, |sum, term| sum.saturating_add(term.most())),
            Self::Min(terms) => terms.iter().map(Self::most).min().unwrap_or(0),
        }
    }
}

impl std::fmt::Display for SizeTerm {
    /// The term as its names spell it: `tasks_max * projects_max`, a literal bound
    /// as its number.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bound(PlanBound {
                name: Some(name), ..
            }) => write!(f, "{name}"),
            Self::Bound(PlanBound { name: None, max }) => write!(f, "{max}"),
            Self::Product(terms) => joined(f, terms, " * "),
            Self::Sum(terms) => joined(f, terms, " + "),
            Self::Min(terms) => {
                write!(f, "min(")?;
                joined(f, terms, ", ")?;
                write!(f, ")")
            }
        }
    }
}

/// `terms` written one after another with `between` between each two, a
/// sum inside parentheses.
fn joined(f: &mut std::fmt::Formatter<'_>, terms: &[SizeTerm], between: &str) -> std::fmt::Result {
    for (at, term) in terms.iter().enumerate() {
        if at > 0 {
            write!(f, "{between}")?;
        }
        match term {
            SizeTerm::Sum(_) => write!(f, "({term})")?,
            _ => write!(f, "{term}")?,
        }
    }
    Ok(())
}

/// The plan of a document of relations (D5): its tuples, its rules, and the
/// strata its rules are evaluated in, each a least fixed point over the strata
/// before it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationsPlan {
    pub tuples: TuplesTable,
    pub relations: Vec<RelationRule>,
    /// Every relation once, in an order where a relation is in a stratum after
    /// every relation it excludes, and in one no earlier than every relation it
    /// reads.
    pub strata: Vec<Vec<RelationId>>,
    /// The most tuples a derived relation holds.
    pub size: SizeTerm,
}

/// The table of relation tuples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuplesTable {
    pub name: String,
    pub record: String,
    pub size: SizeTerm,
}

/// One relation of an object type and its rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationRule {
    pub object: String,
    pub name: String,
    pub rewrite: RelationRewrite,
}

/// A relation, by its object type and its name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RelationId {
    pub object: String,
    pub name: String,
}

impl std::fmt::Display for RelationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}#{}", self.object, self.name)
    }
}

/// What makes a plan one the build would not make.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanDefect {
    #[error("operator {at} reads operator {input}, which does not come before it")]
    ForwardRead { at: OperatorId, input: OperatorId },
    #[error("operator {at} scans the table '{table}', which the plan does not declare")]
    UnknownTable { at: OperatorId, table: String },
    #[error(
        "the table '{table}' is read by operator {at}, which is not its filter: every operator \
         over a table reads its rows through the viewer's filter (docs/adr/0151, D6)"
    )]
    ScanReadOutsideItsFilter { table: String, at: OperatorId },
    #[error("the scan of the table '{table}' at operator {at} is read by no filter")]
    ScanNotFiltered { table: String, at: OperatorId },
    #[error("the filter of the table '{table}' at operator {at} reads no scan of it")]
    FilterNotOverItsScan { table: String, at: OperatorId },
    #[error("the output, operator {at}, is no cut of an order")]
    OutputNotACutOfAnOrder { at: OperatorId },
    #[error("the order the output cuts does not end with the result's key column '{column}'")]
    OrderNotTotal { column: String },
    #[error(
        "operator {at} gives rows after a filter and does not carry the viewer, in a plan \
         lifted over its viewers (docs/adr/0151, D6)"
    )]
    ViewerNotCarried { at: OperatorId },
    #[error("operator {at} carries a viewer in a plan of a view with none")]
    ViewerInAnUnliftedPlan { at: OperatorId },
    #[error(
        "operator {at} joins two relations that carry a viewer on its own columns alone: it \
         meets the rows one viewer may read with the rows another may (docs/adr/0151, D6)"
    )]
    JoinAcrossViewers { at: OperatorId },
    #[error(
        "operator {at} groups, orders, nests or cuts rows that carry a viewer with no viewer as \
         the first column of its key (docs/adr/0151, D6)"
    )]
    KeyWithoutViewer { at: OperatorId },
    #[error("the plan of the view read as '{alias}': {defect}")]
    InRead {
        alias: String,
        defect: Box<PlanDefect>,
    },
}

impl ViewPlan {
    /// Hold the plan, and every plan it reads, to the rules every plan the
    /// build makes keeps: each operator reads only operators before it; each
    /// scan is read by the filter of its table and by nothing else, and each
    /// filter reads a scan of its table (D6); where the plan is lifted, every
    /// operator from a filter on carries the viewer, every join of two that carry
    /// it is on equal viewers, and every group, order, nest and cut has it first in
    /// its key (D6); and the output is the cut of an order that holds the result's
    /// key (D2).
    pub fn verify(&self) -> Result<(), PlanDefect> {
        for read in &self.reads {
            read.plan.verify().map_err(|defect| PlanDefect::InRead {
                alias: read.alias.clone(),
                defect: Box::new(defect),
            })?;
        }
        match &self.body {
            PlanBody::Query(query) => query.verify(),
            PlanBody::Relations(_) => Ok(()),
        }
    }
}

impl QueryPlan {
    /// [`ViewPlan::verify`] of one query.
    pub fn verify(&self) -> Result<(), PlanDefect> {
        let mut readers: Vec<Vec<OperatorId>> = vec![Vec::new(); self.operators.len()];
        for (at, node) in self.operators.iter().enumerate() {
            for input in node.op.inputs() {
                if input >= at {
                    return Err(PlanDefect::ForwardRead { at, input });
                }
                readers[input].push(at);
            }
        }
        for (at, node) in self.operators.iter().enumerate() {
            match &node.op {
                Operator::Scan { table, .. } => {
                    if !self.tables.iter().any(|t| &t.name == table) {
                        return Err(PlanDefect::UnknownTable {
                            at,
                            table: table.clone(),
                        });
                    }
                    match readers[at].as_slice() {
                        [] => {
                            return Err(PlanDefect::ScanNotFiltered {
                                table: table.clone(),
                                at,
                            })
                        }
                        [reader] => {
                            let filtered = matches!(
                                &self.operators[*reader].op,
                                Operator::Filter { table: filtered, .. } if filtered == table
                            );
                            if !filtered {
                                return Err(PlanDefect::ScanReadOutsideItsFilter {
                                    table: table.clone(),
                                    at: *reader,
                                });
                            }
                        }
                        [first, second, ..] => {
                            let outside =
                                if matches!(&self.operators[*first].op, Operator::Filter { .. }) {
                                    *second
                                } else {
                                    *first
                                };
                            return Err(PlanDefect::ScanReadOutsideItsFilter {
                                table: table.clone(),
                                at: outside,
                            });
                        }
                    }
                }
                Operator::Filter { input, table } => {
                    let over_its_scan = matches!(
                        &self.operators[*input].op,
                        Operator::Scan { table: scanned, .. } if scanned == table
                    );
                    if !over_its_scan {
                        return Err(PlanDefect::FilterNotOverItsScan {
                            table: table.clone(),
                            at,
                        });
                    }
                }
                _ => {}
            }
        }
        self.verify_viewers()?;
        let order = match self.operators.get(self.output).map(|node| &node.op) {
            Some(Operator::Cut { input, .. }) => match &self.operators[*input].op {
                Operator::Order { by, .. } => by,
                _ => return Err(PlanDefect::OutputNotACutOfAnOrder { at: self.output }),
            },
            _ => return Err(PlanDefect::OutputNotACutOfAnOrder { at: self.output }),
        };
        if let Some(column) = self
            .result
            .key
            .iter()
            .find(|column| !order.iter().any(|key| key.expr.text == **column))
        {
            return Err(PlanDefect::OrderNotTotal {
                column: column.clone(),
            });
        }
        Ok(())
    }

    /// The viewer's rules of a lifted plan (D6), and that a plan of a view with
    /// no viewer carries none.
    fn verify_viewers(&self) -> Result<(), PlanDefect> {
        fn first_is_viewer(key: &[String]) -> bool {
            key.first().is_some_and(|k| k == VIEWER)
        }
        fn ordered_by_viewer(by: &[PlanOrderKey]) -> bool {
            by.first().is_some_and(|k| k.expr.text == VIEWER)
        }
        for (at, node) in self.operators.iter().enumerate() {
            let scan = matches!(node.op, Operator::Scan { .. });
            if !self.lifted {
                if node.carries_viewer {
                    return Err(PlanDefect::ViewerInAnUnliftedPlan { at });
                }
                continue;
            }
            if !scan && !node.carries_viewer {
                return Err(PlanDefect::ViewerNotCarried { at });
            }
            let keyed = match &node.op {
                Operator::Join {
                    left,
                    right,
                    on_viewers,
                    ..
                } => {
                    let both = self.operators[*left].carries_viewer
                        && self.operators[*right].carries_viewer;
                    if both && !*on_viewers {
                        return Err(PlanDefect::JoinAcrossViewers { at });
                    }
                    true
                }
                Operator::Group { outputs, .. } => {
                    let nests_by_viewer = outputs.iter().all(|output| match output {
                        PlanAggregate::Nest(nest) => ordered_by_viewer(&nest.order),
                        _ => true,
                    });
                    first_is_viewer(&node.key) && nests_by_viewer
                }
                Operator::Order { by, .. } => first_is_viewer(&node.key) && ordered_by_viewer(by),
                Operator::Cut { .. } => first_is_viewer(&node.key),
                _ => true,
            };
            if !keyed {
                return Err(PlanDefect::KeyWithoutViewer { at });
            }
        }
        Ok(())
    }
}
