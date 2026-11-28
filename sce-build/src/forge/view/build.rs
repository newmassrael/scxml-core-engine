// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The plan of a query (docs/adr/0151, D1, D2, D3, D6 and D8).
//!
//! The operators are planned in the order the document writes them. Each table
//! is read by a scan and then by its filter, the semi-join with the objects the
//! viewer may read its rows by, before any other operator meets its rows (D6).
//! The key of each operator's rows is inferred by the rules of functional
//! dependency (D2): a table's key; a join's pair of keys, or the left side's
//! alone where it joins on the right side's; a group's columns that give its
//! `by`. The order is completed by the result's key, so it is total, and the
//! result is cut by it at its bound. Each count of rows is a term of the bounds
//! the document names (D8).

use std::collections::BTreeMap;

use super::plan::{
    Operator, OperatorId, PlanAggregate, PlanBound, PlanColumn, PlanExpr, PlanField, PlanNest,
    PlanNode, PlanOrderKey, PlanResult, PlanTable, PlanVisibility, QueryPlan, RelationsPlan,
    SizeTerm, StatePart, TableSource,
};
use super::scope::{self, Judged, RowType, Scope};
use super::Read;
use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::expr::{self, BinOp, Expected, ExprKind, TypedExpr};
use crate::forge::model::{
    ForgeKind, ViewGroup, ViewGroupOutput, ViewJoinKind, ViewModel, ViewNest, ViewOrderKey,
    ViewOutputField, ViewQuery, ViewShape, ViewStep, ViewTable, ViewTableSource, ViewVisibility,
};
use crate::forge::types::InferredType;
use crate::forge::value_type::{ScopedRecord, TypeScope, ValueType};

/// What the rows of a table are, as the query reads them.
struct TableRows {
    key: Option<ValueType>,
    row: Option<RowType>,
    /// The names, relative to a row, that tell its rows apart: `key`, or the
    /// fields of another view's result its key names.
    key_columns: Vec<String>,
    /// How the key and the row are spelled as columns.
    key_spelled: Option<String>,
    row_spelled: Option<String>,
}

/// An alias the rows of a table are named by in the query.
struct Binding {
    alias: String,
    table: String,
    /// The right side of a left join: each of its names may be absent.
    optional: bool,
}

/// The operators of one query, as they are made.
struct Build<'a> {
    view: &'a ViewModel,
    reads: &'a [Read],
    types: &'a TypeScope,
    document: &'a str,
    tables: Vec<PlanTable>,
    rows: BTreeMap<String, TableRows>,
    operators: Vec<PlanNode>,
    bindings: Vec<Binding>,
    /// The equalities a join on the right side's key states: the right side's
    /// key, and the left side's value it equals.
    equal: Vec<(String, String)>,
    state: Vec<StatePart>,
}

/// The plan of `query`, the body of `view`, whose imports' records and enums
/// `types` holds and whose imported views `reads` holds planned.
pub(super) fn query(
    view: &ViewModel,
    query: &ViewQuery,
    reads: &[Read],
    types: &TypeScope,
    document: &str,
) -> Result<QueryPlan, Located<ForgeError>> {
    let mut build = Build {
        view,
        reads,
        types,
        document,
        tables: Vec::new(),
        rows: BTreeMap::new(),
        operators: Vec::new(),
        bindings: Vec::new(),
        equal: Vec::new(),
        state: Vec::new(),
    };
    for table in &view.tables {
        build.table(table)?;
    }

    let mut at = build.read(&query.from.table, &query.from.alias, false, query.from.line)?;
    for step in &query.steps {
        at = match step {
            ViewStep::Join(join) => build.join(at, join)?,
            ViewStep::Where(filter) => {
                let scope = build.rows_scope();
                let judged = build.judge(
                    &scope,
                    &filter.cond,
                    Expected::Slot(InferredType::Bool),
                    filter.line,
                )?;
                build.total(&judged, &filter.cond, filter.line)?;
                let node = &build.operators[at];
                let (columns, key, size) =
                    (node.columns.clone(), node.key.clone(), node.size.clone());
                build.push(
                    Operator::Where {
                        input: at,
                        cond: PlanExpr::new(&filter.cond),
                    },
                    columns,
                    key,
                    size,
                )
            }
        };
    }

    let Some(result) = types.records.get(&query.result.record) else {
        return Err(build.at(
            ValidationError::InvalidReference {
                kind: ForgeKind::View,
                name: query.result.record.clone(),
                what: "record of an event schema this document imports".into(),
                available: types.records.keys().cloned().collect(),
            },
            query.result.line,
        ));
    };
    let (projected, inferred) = match &query.shape {
        ViewShape::Group(group) => build.group(at, group, result)?,
        ViewShape::Select(select) => build.select(at, &select.fields, result)?,
    };
    // Every field of the result's record is given by a column, and every
    // column is a field of it.
    if let Some((missing, _)) = result.fields.iter().find(|(name, _)| {
        !build.operators[projected]
            .columns
            .iter()
            .any(|c| c.name == *name)
    }) {
        return Err(build.at(
            ValidationError::AttributeRuleViolated {
                element: "<sce:result>".into(),
                attr: "type".into(),
                value: format!("record:{}", query.result.record),
                rule: format!(
                    "the record declares the field '{missing}', which no column of the query \
                     gives: a row of the result is a record whose every field is a column"
                ),
            },
            query.result.line,
        ));
    }

    // The total order: the keys written, then the result's key (D2).
    let mut scope = Scope::new(types);
    for (name, ty) in &result.fields {
        scope.column(name, ty, &result.scope);
    }
    let mut by: Vec<PlanOrderKey> = Vec::new();
    for key in &query.order {
        build.order_key(&scope, key, "the order of the result by", query.order_line)?;
        by.push(PlanOrderKey {
            expr: PlanExpr::new(&key.expr),
            descending: key.descending,
            from_key: false,
        });
    }
    let columns: Vec<String> = build.operators[projected]
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect();
    for name in &query.result.key {
        if !columns.contains(name) {
            return Err(build.at(
                ValidationError::InvalidReference {
                    kind: ForgeKind::View,
                    name: name.clone(),
                    what: "column of the result".into(),
                    available: columns.clone(),
                },
                query.result.line,
            ));
        }
        if !by.iter().any(|key| key.expr.text == *name) {
            by.push(PlanOrderKey {
                expr: PlanExpr::new(name),
                descending: false,
                from_key: true,
            });
        }
    }
    let unique = !inferred.is_empty() && inferred.iter().all(|k| query.result.key.contains(k));
    if !unique {
        return Err(build.at(
            ValidationError::ViewResultKeyNotUnique {
                view: view.name.clone(),
                key: query.result.key.clone(),
                inferred: if inferred.is_empty() {
                    "no key: no column gives what tells the rows apart".to_string()
                } else {
                    format!("the key ({})", inferred.join(", "))
                },
            },
            query.result.line,
        ));
    }
    let size = build.operators[projected].size.clone();
    let bound = PlanBound::of(&query.result.bound);
    if query.order.is_empty() && u128::from(bound.max) < size.most() {
        return Err(build.at(
            ValidationError::ViewBoundWithoutOrder {
                view: view.name.clone(),
                what: "the result".into(),
                bound: bound.max,
                most: format!("{size} ({} rows)", size.most()),
            },
            query.result.line,
        ));
    }
    let projected_columns = build.operators[projected].columns.clone();
    let ordered = build.push(
        Operator::Order {
            input: projected,
            by,
        },
        projected_columns.clone(),
        query.result.key.clone(),
        size.clone(),
    );
    let cut_size = SizeTerm::Min(vec![SizeTerm::Bound(bound.clone()), size]);
    build.state.push(StatePart {
        part: "result".into(),
        size: cut_size.clone(),
    });
    let output = build.push(
        Operator::Cut {
            input: ordered,
            bound: bound.clone(),
        },
        projected_columns,
        query.result.key.clone(),
        cut_size,
    );
    Ok(QueryPlan {
        tables: build.tables,
        operators: build.operators,
        output,
        result: PlanResult {
            record: query.result.record.clone(),
            key: query.result.key.clone(),
            bound,
        },
        state: build.state,
    })
}

impl Build<'_> {
    /// `error` placed on the row `line` of the document.
    fn at(&self, error: impl Into<ForgeError>, line: Option<u32>) -> Located<ForgeError> {
        Located::new(error.into(), self.document, line, None)
    }

    /// Push an operator and hand back where it stands.
    fn push(
        &mut self,
        op: Operator,
        columns: Vec<PlanColumn>,
        key: Vec<String>,
        size: SizeTerm,
    ) -> OperatorId {
        self.operators.push(PlanNode {
            op,
            columns,
            key,
            size,
        });
        self.operators.len() - 1
    }

    /// The view read under `alias`, where the document imports one.
    fn read_view(&self, alias: &str) -> Option<&Read> {
        self.reads.iter().find(|read| read.alias == alias)
    }

    /// Declare `table`: what its rows are, who may read them, and its most rows.
    fn table(&mut self, table: &ViewTable) -> Result<(), Located<ForgeError>> {
        let (source, rows, size, passed) = match &table.source {
            ViewTableSource::Type(ty) => {
                let (source, rows, max) = match ty {
                    ValueType::Map { key, value, max } => (
                        TableSource::Map {
                            key: key.to_string(),
                            value: value.to_string(),
                        },
                        TableRows {
                            key: Some((**key).clone()),
                            row: Some(RowType::Value((**value).clone())),
                            key_columns: vec!["key".into()],
                            key_spelled: Some(key.to_string()),
                            row_spelled: Some(value.to_string()),
                        },
                        max,
                    ),
                    ValueType::Set { key, max } => (
                        TableSource::Set {
                            key: key.to_string(),
                        },
                        TableRows {
                            key: Some((**key).clone()),
                            row: None,
                            key_columns: vec!["key".into()],
                            key_spelled: Some(key.to_string()),
                            row_spelled: None,
                        },
                        max,
                    ),
                    _ => unreachable!("the parser holds a table to a map or a set"),
                };
                (source, rows, SizeTerm::Bound(PlanBound::of(max)), None)
            }
            ViewTableSource::View(alias) => {
                let Some(read) = self.read_view(alias) else {
                    return Err(self.at(
                        ValidationError::InvalidReference {
                            kind: ForgeKind::View,
                            name: alias.clone(),
                            what: "view this document imports".into(),
                            available: self.reads.iter().map(|r| r.alias.clone()).collect(),
                        },
                        table.line,
                    ));
                };
                let (Some(result), Some((record, key, bound))) =
                    (&read.result, read.query_result())
                else {
                    return Err(self.at(
                        ValidationError::AttributeRuleViolated {
                            element: "<sce:table>".into(),
                            attr: "view".into(),
                            value: alias.clone(),
                            rule: "a table of a view is the result of a query; a document of \
                                   relations is read through <sce:visible relations>"
                                .into(),
                        },
                        table.line,
                    ));
                };
                let passed = self.passed_from(table, alias, read)?;
                (
                    TableSource::View {
                        view: alias.clone(),
                        record: record.clone(),
                    },
                    TableRows {
                        key: None,
                        row: Some(RowType::Record(result.clone())),
                        key_columns: key.iter().map(|k| format!("row.{k}")).collect(),
                        key_spelled: None,
                        row_spelled: Some(format!("record:{record}")),
                    },
                    SizeTerm::Bound(bound),
                    Some(passed),
                )
            }
        };
        let visibility = match (&table.visibility, passed) {
            (Some(ViewVisibility::Relation { .. }), _) if self.view.viewer.is_none() => {
                return Err(self.at(
                    ValidationError::ViewViewerMismatch {
                        view: self.view.name.clone(),
                        table: table.name.clone(),
                        reason: "a view with no viewer names no relation a viewer stands in: its \
                                 value is one value for every reader, so it reads only what \
                                 every viewer may"
                            .into(),
                    },
                    table.line,
                ))
            }
            (
                Some(ViewVisibility::Relation {
                    relations,
                    relation,
                    object,
                }),
                _,
            ) => self.relation_filter(table, &rows, relations, relation, object)?,
            (Some(ViewVisibility::Public), None) => PlanVisibility::Public,
            (Some(ViewVisibility::Public), Some(_)) => {
                return Err(self.at(
                    ValidationError::AttributeRuleViolated {
                        element: "<sce:table>".into(),
                        attr: "public".into(),
                        value: "true".into(),
                        rule: "the rows of a view are read by that view's own viewer, passed on \
                               (docs/adr/0151, D3): a table of a view says nothing of who may \
                               read them"
                            .into(),
                    },
                    table.line,
                ))
            }
            (None, Some(passed)) => passed,
            (None, None) => {
                return Err(self.at(
                    ValidationError::ViewTableWithoutVisibility {
                        view: self.view.name.clone(),
                        table: table.name.clone(),
                    },
                    table.line,
                ))
            }
        };
        self.state.push(StatePart {
            part: format!("table {}", table.name),
            size: size.clone(),
        });
        self.tables.push(PlanTable {
            name: table.name.clone(),
            source,
            visibility,
            size,
        });
        self.rows.insert(table.name.clone(), rows);
        Ok(())
    }

    /// Who may read the rows of the view `read` a table of this one reads
    /// (docs/adr/0151, D3 and D6): a view with no viewer gives rows every viewer
    /// may read; one with a viewer gives rows of that viewer, which a view of
    /// the same viewer reads passed on, and no other view reads.
    fn passed_from(
        &self,
        table: &ViewTable,
        alias: &str,
        read: &Read,
    ) -> Result<PlanVisibility, Located<ForgeError>> {
        match (read.plan.viewer, self.view.viewer.as_ref().map(|v| v.of)) {
            (None, _) => Ok(PlanVisibility::Public),
            (Some(theirs), Some(ours)) if theirs == ours => Ok(PlanVisibility::PassedOn {
                view: alias.to_string(),
            }),
            (Some(theirs), Some(ours)) => Err(self.at(
                ValidationError::ViewViewerMismatch {
                    view: self.view.name.clone(),
                    table: table.name.clone(),
                    reason: format!(
                        "the view '{alias}' is a value per {}, and this one per {}: a view reads \
                         views of its own viewer, passed on, and views with none",
                        theirs.as_attr(),
                        ours.as_attr()
                    ),
                },
                table.line,
            )),
            (Some(theirs), None) => Err(self.at(
                ValidationError::ViewViewerMismatch {
                    view: self.view.name.clone(),
                    table: table.name.clone(),
                    reason: format!(
                        "the view '{alias}' is a value per {}, and a view with no viewer reads \
                         only views with none",
                        theirs.as_attr()
                    ),
                },
                table.line,
            )),
        }
    }

    /// The filter of a table whose rows a viewer reads by its relation to the
    /// object each names (D6): the relation, derived by the document of
    /// relations imported as `relations`, and the object, an expression over the
    /// row of the type of a tuple's object.
    fn relation_filter(
        &self,
        table: &ViewTable,
        rows: &TableRows,
        relations: &str,
        relation: &str,
        object: &str,
    ) -> Result<PlanVisibility, Located<ForgeError>> {
        let documents: Vec<String> = self
            .reads
            .iter()
            .filter(|read| read.relations().is_some())
            .map(|read| read.alias.clone())
            .collect();
        let Some((derived, tuple_object)) = self
            .read_view(relations)
            .and_then(|read| Some((read.relations()?, read.tuple_object.as_ref()?)))
        else {
            return Err(self.at(
                ValidationError::InvalidReference {
                    kind: ForgeKind::View,
                    name: relations.to_string(),
                    what: "document of relations this document imports".into(),
                    available: documents,
                },
                table.line,
            ));
        };
        let objects: Vec<&str> = derived
            .relations
            .iter()
            .filter(|rule| rule.name == relation)
            .map(|rule| rule.object.as_str())
            .collect();
        let object_type = match objects.as_slice() {
            [one] => one.to_string(),
            [] => {
                return Err(self.at(
                    ValidationError::InvalidReference {
                        kind: ForgeKind::View,
                        name: relation.to_string(),
                        what: format!("relation of '{relations}'"),
                        available: derived.relations.iter().map(|r| r.name.clone()).collect(),
                    },
                    table.line,
                ))
            }
            several => {
                return Err(self.at(
                    ValidationError::AttributeRuleViolated {
                        element: "<sce:visible>".into(),
                        attr: "relation".into(),
                        value: relation.to_string(),
                        rule: format!(
                            "'{relations}' derives a relation '{relation}' of each of {}: a row \
                             is filtered by one relation of one type of object",
                            several.join(", ")
                        ),
                    },
                    table.line,
                ))
            }
        };
        let mut scope = Scope::new(self.types);
        scope.bind_unaliased(rows.key.as_ref(), rows.row.as_ref(), self.types);
        let judged = self.judge(
            &scope,
            object,
            Expected::Hint(InferredType::Unknown),
            table.line,
        )?;
        self.total(&judged, object, table.line)?;
        let wanted = InferredType::of_value(tuple_object);
        if judged.tree.ty != wanted {
            return Err(self.at(
                ValidationError::AttributeRuleViolated {
                    element: "<sce:visible>".into(),
                    attr: "object".into(),
                    value: object.to_string(),
                    rule: format!(
                        "the object a row names is a value of the type of a tuple's object, \
                         {tuple_object}, and this is {}",
                        judged.tree.ty.describe()
                    ),
                },
                table.line,
            ));
        }
        Ok(PlanVisibility::Relation {
            relations: relations.to_string(),
            relation: relation.to_string(),
            object_type,
            object: PlanExpr::new(object),
        })
    }

    /// Read the table `table` under `alias`: its scan, then its filter (D6).
    fn read(
        &mut self,
        table: &str,
        alias: &str,
        optional: bool,
        line: Option<u32>,
    ) -> Result<OperatorId, Located<ForgeError>> {
        let Some(rows) = self.rows.get(table) else {
            return Err(self.at(
                ValidationError::InvalidReference {
                    kind: ForgeKind::View,
                    name: table.to_string(),
                    what: "table of this view".into(),
                    available: self.view.tables.iter().map(|t| t.name.clone()).collect(),
                },
                line,
            ));
        };
        let columns = alias_columns(alias, rows, false);
        let key: Vec<String> = rows
            .key_columns
            .iter()
            .map(|column| format!("{alias}.{column}"))
            .collect();
        let size = self
            .tables
            .iter()
            .find(|t| t.name == table)
            .map(|t| t.size.clone())
            .expect("every table of the view is declared before any is read");
        let scan = self.push(
            Operator::Scan {
                table: table.to_string(),
                alias: alias.to_string(),
            },
            columns.clone(),
            key.clone(),
            size.clone(),
        );
        let filter = self.push(
            Operator::Filter {
                input: scan,
                table: table.to_string(),
            },
            columns,
            key,
            size,
        );
        self.bindings.push(Binding {
            alias: alias.to_string(),
            table: table.to_string(),
            optional,
        });
        Ok(filter)
    }

    /// The scope of the rows at the operator last planned: every alias bound
    /// so far.
    fn rows_scope(&self) -> Scope {
        let mut scope = Scope::new(self.types);
        for binding in &self.bindings {
            let rows = &self.rows[&binding.table];
            scope.bind(
                &binding.alias,
                rows.key.as_ref(),
                rows.row.as_ref(),
                binding.optional,
                self.types,
            );
        }
        scope
    }

    /// `text` typed in `scope`, its refusal placed on `line`.
    fn judge(
        &self,
        scope: &Scope,
        text: &str,
        expected: Expected,
        line: Option<u32>,
    ) -> Result<Judged, Located<ForgeError>> {
        scope
            .judge(text, expected)
            .map_err(|refusal| self.at(ForgeError::from(refusal), line))
    }

    /// Refuse an expression the build does not prove total.
    fn total(
        &self,
        judged: &Judged,
        text: &str,
        line: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        match &judged.may_fail {
            None => Ok(()),
            Some(why) => Err(self.at(
                ValidationError::ViewExpressionMayFail {
                    view: self.view.name.clone(),
                    expr: text.trim().to_string(),
                    why: why.clone(),
                },
                line,
            )),
        }
    }

    /// Refuse a value stored in a place of `place` that may not hold it.
    fn stored(
        &self,
        judged: &Judged,
        text: &str,
        place: &ValueType,
        line: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        self.total(judged, text, line)?;
        match scope::store_failure(judged, place) {
            None => Ok(()),
            Some(why) => Err(self.at(
                ValidationError::ViewExpressionMayFail {
                    view: self.view.name.clone(),
                    expr: text.trim().to_string(),
                    why,
                },
                line,
            )),
        }
    }

    /// Plan a join of the rows at `left` with the table `join` names.
    fn join(
        &mut self,
        left: OperatorId,
        join: &crate::forge::model::ViewJoin,
    ) -> Result<OperatorId, Located<ForgeError>> {
        let right = self.read(&join.table, &join.alias, false, join.line)?;
        // `on` is read of a pair of rows, the right one present.
        let scope = self.rows_scope();
        let judged = self.judge(
            &scope,
            &join.on,
            Expected::Slot(InferredType::Bool),
            join.line,
        )?;
        self.total(&judged, &join.on, join.line)?;
        if join.kind == ViewJoinKind::Left {
            if let Some(binding) = self.bindings.last_mut() {
                binding.optional = true;
            }
        }
        let right_key = self.operators[right].key.clone();
        let left_node = &self.operators[left];
        let (left_key, left_size) = (left_node.key.clone(), left_node.size.clone());
        let right_size = self.operators[right].size.clone();
        let equated = match right_key.as_slice() {
            [key] => equated_with(&judged.tree, key, &join.alias),
            _ => None,
        };
        let by_right_key = equated.is_some();
        if let (Some(left_value), [key]) = (&equated, right_key.as_slice()) {
            if !left_value.is_empty() {
                self.equal.push((key.clone(), left_value.clone()));
            }
        }
        let key = if by_right_key {
            left_key
        } else {
            left_key.into_iter().chain(right_key).collect()
        };
        let size = match (by_right_key, join.kind) {
            (true, _) => left_size.clone(),
            (false, ViewJoinKind::Inner) => {
                SizeTerm::Product(vec![left_size.clone(), right_size.clone()])
            }
            (false, ViewJoinKind::Left) => SizeTerm::Product(vec![
                left_size.clone(),
                SizeTerm::Sum(vec![SizeTerm::one(), right_size.clone()]),
            ]),
        };
        self.state.push(StatePart {
            part: format!("join {}", join.alias),
            size: SizeTerm::Sum(vec![left_size, right_size]),
        });
        let columns = self.binding_columns();
        Ok(self.push(
            Operator::Join {
                left,
                right,
                kind: join.kind,
                on: PlanExpr::new(&join.on),
                by_right_key,
            },
            columns,
            key,
            size,
        ))
    }

    /// The columns of the rows of every alias bound so far.
    fn binding_columns(&self) -> Vec<PlanColumn> {
        self.bindings
            .iter()
            .flat_map(|binding| {
                alias_columns(&binding.alias, &self.rows[&binding.table], binding.optional)
            })
            .collect()
    }

    /// Whether `path`, a name the rows at a group are read by, is determined by
    /// the paths `known` holds: one of them or a part of one, or a name of an alias
    /// whose key they hold, or a name of no alias at all (an enum's value).
    fn determined(&self, path: &str, known: &[String]) -> bool {
        if covers(known, path) {
            return true;
        }
        let alias = path.split('.').next().unwrap_or(path);
        match self.bindings.iter().find(|b| b.alias == alias) {
            None => true,
            Some(binding) => self.rows[&binding.table]
                .key_columns
                .iter()
                .all(|column| covers(known, &format!("{alias}.{column}"))),
        }
    }

    /// Plan a group of the rows at `input` (D1): its columns, each held to the
    /// result's record, and the key its columns give (D2).
    fn group(
        &mut self,
        input: OperatorId,
        group: &ViewGroup,
        result: &ScopedRecord,
    ) -> Result<(OperatorId, Vec<String>), Located<ForgeError>> {
        let scope = self.rows_scope();
        let input_size = self.operators[input].size.clone();
        let input_key = self.operators[input].key.clone();
        let mut by_paths: Vec<Option<String>> = Vec::new();
        for by in &group.by {
            let judged = self.judge(
                &scope,
                by,
                Expected::Hint(InferredType::Unknown),
                group.line,
            )?;
            self.total(&judged, by, group.line)?;
            by_paths.push(expr::dotted_path(&judged.tree));
        }
        let known = closure(by_paths.iter().flatten().cloned().collect(), &self.equal);
        let mut outputs: Vec<PlanAggregate> = Vec::new();
        let mut columns: Vec<PlanColumn> = Vec::new();
        let mut key_for: Vec<Option<String>> = vec![None; group.by.len()];
        for output in &group.outputs {
            let name = output.name();
            let line = output.line();
            let Some(place) = field_type(result, name) else {
                return Err(self.at(
                    ValidationError::AttributeRuleViolated {
                        element: format!("<sce:{}>", output.element()),
                        attr: "name".into(),
                        value: name.to_string(),
                        rule: format!(
                            "a column of the query is a field of the result's record '{}', \
                             which declares {}",
                            result.name,
                            field_names(result)
                        ),
                    },
                    line,
                ));
            };
            let aggregate = match output {
                ViewGroupOutput::Field(field) => {
                    let judged = self.judge(
                        &scope,
                        &field.expr,
                        Expected::Slot(InferredType::of_value(place)),
                        line,
                    )?;
                    self.stored(&judged, &field.expr, place, line)?;
                    let paths = maximal_paths(&judged.tree);
                    if let Some(free) = paths.iter().find(|p| !self.determined(p, &known)) {
                        return Err(self.at(
                            ValidationError::AttributeRuleViolated {
                                element: "<sce:field>".into(),
                                attr: "expr".into(),
                                value: field.expr.clone(),
                                rule: format!(
                                    "a field of a group reads only what the group's by \
                                     determines, and {free} is not determined by {}: read a \
                                     value of the group's rows by <sce:min>, <sce:max> or \
                                     <sce:nest>",
                                    group.by.join(", ")
                                ),
                            },
                            line,
                        ));
                    }
                    if let Some(path) = expr::dotted_path(&judged.tree) {
                        let given = closure(vec![path], &self.equal);
                        for (at, by_path) in by_paths.iter().enumerate() {
                            if key_for[at].is_none()
                                && by_path.as_ref().is_some_and(|b| given.contains(b))
                            {
                                key_for[at] = Some(name.to_string());
                            }
                        }
                    }
                    for (at, by) in group.by.iter().enumerate() {
                        if key_for[at].is_none() && by.trim() == field.expr.trim() {
                            key_for[at] = Some(name.to_string());
                        }
                    }
                    PlanAggregate::Field(plan_field(field))
                }
                ViewGroupOutput::Count { .. } => {
                    self.held_count("count", name, place, &input_size, (1, 1), line)?;
                    PlanAggregate::Count {
                        name: name.to_string(),
                    }
                }
                ViewGroupOutput::Sum(field) => {
                    let judged = self.judge(
                        &scope,
                        &field.expr,
                        Expected::Hint(InferredType::Unknown),
                        line,
                    )?;
                    self.total(&judged, &field.expr, line)?;
                    if scope::floating(judged.tree.ty) {
                        return Err(self.floating("the sum of", &field.expr, line));
                    }
                    let Some(range) = scope::value_range(judged.tree.ty) else {
                        return Err(self.at(
                            ValidationError::AttributeRuleViolated {
                                element: "<sce:sum>".into(),
                                attr: "expr".into(),
                                value: field.expr.clone(),
                                rule: format!(
                                    "a sum is of integers, and this is {}",
                                    judged.tree.ty.describe()
                                ),
                            },
                            line,
                        ));
                    };
                    self.held_count("sum", name, place, &input_size, range, line)?;
                    PlanAggregate::Sum(plan_field(field))
                }
                ViewGroupOutput::Min(field) | ViewGroupOutput::Max(field) => {
                    let judged = self.judge(
                        &scope,
                        &field.expr,
                        Expected::Hint(InferredType::Unknown),
                        line,
                    )?;
                    let what = if matches!(output, ViewGroupOutput::Min(_)) {
                        "the minimum of"
                    } else {
                        "the maximum of"
                    };
                    if scope::floating(judged.tree.ty) {
                        return Err(self.floating(what, &field.expr, line));
                    }
                    self.orderable(&judged, &field.expr, line)?;
                    self.stored(&judged, &field.expr, place, line)?;
                    if matches!(output, ViewGroupOutput::Min(_)) {
                        PlanAggregate::Min(plan_field(field))
                    } else {
                        PlanAggregate::Max(plan_field(field))
                    }
                }
                ViewGroupOutput::Nest(nest) => {
                    PlanAggregate::Nest(self.nest(&scope, nest, place, &input_size, &input_key)?)
                }
            };
            outputs.push(aggregate);
            columns.push(PlanColumn {
                name: name.to_string(),
                ty: place.to_string(),
            });
        }
        let key: Vec<String> = if key_for.iter().all(Option::is_some) {
            key_for.into_iter().flatten().collect()
        } else {
            Vec::new()
        };
        self.state.push(StatePart {
            part: "group".into(),
            size: input_size.clone(),
        });
        let at = self.push(
            Operator::Group {
                input,
                by: group.by.iter().map(|b| PlanExpr::new(b.as_str())).collect(),
                outputs,
            },
            columns,
            key.clone(),
            input_size,
        );
        Ok((at, key))
    }

    /// Refuse a count or a sum held in `place` that may not hold it: up to
    /// `rows` rows, each of a value in `range` (a count's each 1).
    fn held_count(
        &self,
        aggregate: &str,
        column: &str,
        place: &ValueType,
        rows: &SizeTerm,
        (low, high): (i128, i128),
        line: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        let Some((place_low, place_high)) = scope::place_range(place) else {
            return Err(self.at(
                ValidationError::AttributeRuleViolated {
                    element: format!("<sce:{aggregate}>"),
                    attr: "name".into(),
                    value: column.to_string(),
                    rule: format!(
                        "a {aggregate} is held in a field of an integer type, and this one is \
                         {place}"
                    ),
                },
                line,
            ));
        };
        let most_rows = i128::try_from(rows.most()).unwrap_or(i128::MAX);
        let least = low.min(low.saturating_mul(most_rows));
        let most = high.max(high.saturating_mul(most_rows));
        if least < place_low || most > place_high {
            return Err(self.at(
                ValidationError::ViewAggregateMayOverflow {
                    view: self.view.name.clone(),
                    aggregate: aggregate.to_string(),
                    column: column.to_string(),
                    ty: place.to_string(),
                    most: if least < place_low {
                        format!("{least}, over {rows} rows")
                    } else {
                        format!("{most}, over {rows} rows")
                    },
                },
                line,
            ));
        }
        Ok(())
    }

    /// The refusal of an aggregate or an order over a floating type.
    fn floating(&self, what: &str, text: &str, line: Option<u32>) -> Located<ForgeError> {
        self.at(
            ValidationError::ViewFloatingAggregate {
                view: self.view.name.clone(),
                what: what.to_string(),
                expr: text.trim().to_string(),
            },
            line,
        )
    }

    /// Refuse a value no order is defined on: a list or a record.
    fn orderable(
        &self,
        judged: &Judged,
        text: &str,
        line: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        if matches!(
            judged.tree.ty,
            InferredType::List(_) | InferredType::ListBuffer(_) | InferredType::Record(_)
        ) {
            return Err(self.at(
                ValidationError::AttributeRuleViolated {
                    element: "<sce:query>".into(),
                    attr: "order".into(),
                    value: text.trim().to_string(),
                    rule: format!(
                        "an order and a minimum or a maximum are over a number, a text, a value of \
                         an enum or a boolean, and this is {}",
                        judged.tree.ty.describe()
                    ),
                },
                line,
            ));
        }
        Ok(())
    }

    /// One key of an order, typed in `scope` and held to D1 and D2.
    fn order_key(
        &self,
        scope: &Scope,
        key: &ViewOrderKey,
        what: &str,
        line: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        let judged = self.judge(
            scope,
            &key.expr,
            Expected::Hint(InferredType::Unknown),
            line,
        )?;
        self.total(&judged, &key.expr, line)?;
        if scope::floating(judged.tree.ty) {
            return Err(self.floating(what, &key.expr, line));
        }
        self.orderable(&judged, &key.expr, line)
    }

    /// Plan a nest of a group: its record's fields from the group's rows, in
    /// its order completed by the key of those rows, cut at its bound (D2).
    fn nest(
        &self,
        scope: &Scope,
        nest: &ViewNest,
        place: &ValueType,
        rows: &SizeTerm,
        rows_key: &[String],
    ) -> Result<PlanNest, Located<ForgeError>> {
        if *place != nest.ty {
            return Err(self.at(
                ValidationError::AttributeRuleViolated {
                    element: "<sce:nest>".into(),
                    attr: "type".into(),
                    value: nest.ty.to_string(),
                    rule: format!(
                        "the result's record declares '{}' as {place}, the type the nest is",
                        nest.name
                    ),
                },
                nest.line,
            ));
        }
        let ValueType::List { elem, max } = &nest.ty else {
            unreachable!("the parser holds a nest to a list of records");
        };
        let ValueType::Record { alias } = elem.as_ref() else {
            unreachable!("the parser holds a nest to a list of records");
        };
        let Some(record) = self.types.records.get(alias) else {
            return Err(self.at(
                ValidationError::InvalidReference {
                    kind: ForgeKind::View,
                    name: alias.clone(),
                    what: "record of an event schema this document imports".into(),
                    available: self.types.records.keys().cloned().collect(),
                },
                nest.line,
            ));
        };
        let mut fields: Vec<PlanField> = Vec::new();
        for field in &nest.fields {
            let Some(field_place) = field_type(record, &field.name) else {
                return Err(self.at(
                    ValidationError::AttributeRuleViolated {
                        element: "<sce:field>".into(),
                        attr: "name".into(),
                        value: field.name.clone(),
                        rule: format!(
                            "a field of a nest is a field of its record '{}', which declares {}",
                            record.name,
                            field_names(record)
                        ),
                    },
                    field.line,
                ));
            };
            let judged = self.judge(
                scope,
                &field.expr,
                Expected::Slot(InferredType::of_value(field_place)),
                field.line,
            )?;
            self.stored(&judged, &field.expr, field_place, field.line)?;
            fields.push(plan_field(field));
        }
        if let Some((missing, _)) = record
            .fields
            .iter()
            .find(|(name, _)| !nest.fields.iter().any(|f| f.name == *name))
        {
            return Err(self.at(
                ValidationError::AttributeRuleViolated {
                    element: "<sce:nest>".into(),
                    attr: "type".into(),
                    value: nest.ty.to_string(),
                    rule: format!(
                        "the record '{}' declares the field '{missing}', which no field of the \
                         nest gives",
                        record.name
                    ),
                },
                nest.line,
            ));
        }
        let what = format!("the order of the nest '{}' by", nest.name);
        let mut order: Vec<PlanOrderKey> = Vec::new();
        for key in &nest.order {
            self.order_key(scope, key, &what, nest.line)?;
            order.push(PlanOrderKey {
                expr: PlanExpr::new(&key.expr),
                descending: key.descending,
                from_key: false,
            });
        }
        for key in rows_key {
            order.push(PlanOrderKey {
                expr: PlanExpr::new(key),
                descending: false,
                from_key: true,
            });
        }
        let bound = PlanBound::of(max);
        if nest.order.is_empty() && u128::from(bound.max) < rows.most() {
            return Err(self.at(
                ValidationError::ViewBoundWithoutOrder {
                    view: self.view.name.clone(),
                    what: format!("the nest '{}'", nest.name),
                    bound: bound.max,
                    most: format!("{rows} ({} rows)", rows.most()),
                },
                nest.line,
            ));
        }
        Ok(PlanNest {
            name: nest.name.clone(),
            record: alias.clone(),
            fields,
            order,
            bound,
        })
    }

    /// Plan a select of the rows at `input` (D1): one row per row, and the key
    /// its fields give (D2).
    fn select(
        &mut self,
        input: OperatorId,
        selected: &[ViewOutputField],
        result: &ScopedRecord,
    ) -> Result<(OperatorId, Vec<String>), Located<ForgeError>> {
        let scope = self.rows_scope();
        let input_key = self.operators[input].key.clone();
        let size = self.operators[input].size.clone();
        let mut key_for: Vec<Option<String>> = vec![None; input_key.len()];
        let mut fields: Vec<PlanField> = Vec::new();
        let mut columns: Vec<PlanColumn> = Vec::new();
        for field in selected {
            let Some(place) = field_type(result, &field.name) else {
                return Err(self.at(
                    ValidationError::AttributeRuleViolated {
                        element: "<sce:field>".into(),
                        attr: "name".into(),
                        value: field.name.clone(),
                        rule: format!(
                            "a column of the query is a field of the result's record '{}', \
                             which declares {}",
                            result.name,
                            field_names(result)
                        ),
                    },
                    field.line,
                ));
            };
            let judged = self.judge(
                &scope,
                &field.expr,
                Expected::Slot(InferredType::of_value(place)),
                field.line,
            )?;
            self.stored(&judged, &field.expr, place, field.line)?;
            if let Some(path) = expr::dotted_path(&judged.tree) {
                let given = closure(vec![path], &self.equal);
                for (at, key) in input_key.iter().enumerate() {
                    if key_for[at].is_none() && given.contains(key) {
                        key_for[at] = Some(field.name.clone());
                    }
                }
            }
            fields.push(plan_field(field));
            columns.push(PlanColumn {
                name: field.name.clone(),
                ty: place.to_string(),
            });
        }
        let key: Vec<String> = if !input_key.is_empty() && key_for.iter().all(Option::is_some) {
            key_for.into_iter().flatten().collect()
        } else {
            Vec::new()
        };
        let at = self.push(
            Operator::Select { input, fields },
            columns,
            key.clone(),
            size,
        );
        Ok((at, key))
    }
}

/// The columns of the rows of `alias` over `rows`, optional where it is the right
/// side of a left join.
fn alias_columns(alias: &str, rows: &TableRows, optional: bool) -> Vec<PlanColumn> {
    let spell = |ty: &str| {
        if optional && !ty.starts_with("optional<") {
            format!("optional<{ty}>")
        } else {
            ty.to_string()
        }
    };
    let mut columns = Vec::new();
    if let Some(key) = &rows.key_spelled {
        columns.push(PlanColumn {
            name: format!("{alias}.key"),
            ty: spell(key.as_str()),
        });
    }
    if let Some(row) = &rows.row_spelled {
        columns.push(PlanColumn {
            name: format!("{alias}.row"),
            ty: spell(row.as_str()),
        });
    }
    columns
}

/// The type the field `name` of `record` is declared with.
fn field_type<'r>(record: &'r ScopedRecord, name: &str) -> Option<&'r ValueType> {
    record
        .fields
        .iter()
        .find(|(field, _)| field == name)
        .map(|(_, ty)| ty)
}

/// The fields `record` declares, as a refusal lists them.
fn field_names(record: &ScopedRecord) -> String {
    record
        .fields
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// A column of the document as the plan holds it.
fn plan_field(field: &ViewOutputField) -> PlanField {
    PlanField {
        name: field.name.clone(),
        expr: PlanExpr::new(&field.expr),
    }
}

/// Whether `known` holds `path` or a value `path` is a part of.
fn covers(known: &[String], path: &str) -> bool {
    known.iter().any(|held| {
        path == held
            || path
                .strip_prefix(held.as_str())
                .is_some_and(|rest| rest.starts_with('.'))
    })
}

/// `start` and every path the equalities `equal` give of one it covers.
fn closure(mut known: Vec<String>, equal: &[(String, String)]) -> Vec<String> {
    loop {
        let mut grew = false;
        for (a, b) in equal {
            if covers(&known, a) && !known.contains(b) {
                known.push(b.clone());
                grew = true;
            }
            if covers(&known, b) && !known.contains(a) {
                known.push(a.clone());
                grew = true;
            }
        }
        if !grew {
            return known;
        }
    }
}

/// The paths `tree` reads, each the longest a member chain spells.
fn maximal_paths(tree: &TypedExpr) -> Vec<String> {
    if let Some(path) = expr::dotted_path(tree) {
        return vec![path];
    }
    let mut paths = Vec::new();
    let children = match &tree.kind {
        // A call's callee is a function's name, not a value a row holds.
        ExprKind::Call { args, .. } => args.iter().collect(),
        _ => tree.children(),
    };
    for child in children {
        paths.extend(maximal_paths(child));
    }
    paths
}

/// The value of the left side `tree` equates `right_key` with, where `tree`
/// is that equality or a conjunction holding it, and its other side reads no
/// name of `right_alias`: a join on the right side's key, which a left row meets
/// once at most. The value is its path, or its text where it is no path.
fn equated_with(tree: &TypedExpr, right_key: &str, right_alias: &str) -> Option<String> {
    match &tree.kind {
        ExprKind::Binary {
            op: BinOp::StrictEq,
            left,
            right,
        } => {
            let (l, r) = (expr::dotted_path(left), expr::dotted_path(right));
            if r.as_deref() == Some(right_key) && !reads_alias(left, right_alias) {
                return Some(l.unwrap_or_default());
            }
            if l.as_deref() == Some(right_key) && !reads_alias(right, right_alias) {
                return Some(r.unwrap_or_default());
            }
            None
        }
        ExprKind::Binary {
            op: BinOp::And,
            left,
            right,
        } => equated_with(left, right_key, right_alias)
            .or_else(|| equated_with(right, right_key, right_alias)),
        _ => None,
    }
}

/// Whether `tree` reads a name of `alias`.
fn reads_alias(tree: &TypedExpr, alias: &str) -> bool {
    matches!(&tree.kind, ExprKind::Ident(name) if name == alias)
        || tree
            .children()
            .into_iter()
            .any(|child| reads_alias(child, alias))
}

/// The plan of a document of relations a read view is.
impl Read {
    fn relations(&self) -> Option<&RelationsPlan> {
        match &self.plan.body {
            super::plan::PlanBody::Relations(relations) => Some(relations),
            super::plan::PlanBody::Query(_) => None,
        }
    }

    /// The record, the key and the bound of the result a read query view has.
    fn query_result(&self) -> Option<(&String, &Vec<String>, PlanBound)> {
        match &self.plan.body {
            super::plan::PlanBody::Query(query) => Some((
                &query.result.record,
                &query.result.key,
                query.result.bound.clone(),
            )),
            super::plan::PlanBody::Relations(_) => None,
        }
    }
}
