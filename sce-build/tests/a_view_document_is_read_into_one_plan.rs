// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The `view` kind (docs/adr/0151, step 2 of its order of work): a view document
//! and the views it imports are read into one plan, each table read first by
//! the filter of its viewer's relation (D6), the plan lifted over every viewer
//! (D6), the key of each operator's rows inferred (D2), the order total by the
//! one comparison of D2 and completed by the result's key, each count of rows a
//! term of the named bounds (D8); each refusal of D12 the document alone
//! decides is refused by its code; a plan that reads a table before its filter,
//! and one that joins the rows of two viewers, are plans the build does not
//! make; and every backend refuses a valid view by name (D11).
//!
//! The documents are under `sce-build/tests/fixtures/view/`: the board of D1, its
//! relations, the records they read, and one document per refusal.

use std::path::{Path, PathBuf};

use sce_build::forge::diagnostic::{DiagnosticCode, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::forge::model::{ViewJoinKind, ViewerOf};
use sce_build::forge::parser::parse_forge_with_imports;
use sce_build::forge::view;
use sce_build::forge::view::plan::{
    Accumulator, Operator, PlanAggregate, PlanBody, PlanDefect, PlanOrderKey, PlanVisibility,
    QueryPlan, RelationId, SizeTerm, ViewPlan, VIEWER,
};
use sce_build::generator::Language;
use sce_build::{DocumentLabel, ForgeCompileOptions};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/view")
        .join(name)
}

/// The plan of the fixture `name`, read as the build reads it: parsed, then
/// planned with every view it imports, from its own directory.
fn read(name: &str) -> Result<ViewPlan, Located<ForgeError>> {
    let path = fixture(name);
    let text = std::fs::read_to_string(&path).expect("the fixture reads");
    read_text(&text, &path)
}

/// The plan of `text`, read as the document at `path` is: its imports from
/// the directory `path` is in.
fn read_text(text: &str, path: &Path) -> Result<ViewPlan, Located<ForgeError>> {
    let identifier = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("a fixture has a stem")
        .to_string();
    let document = path.to_string_lossy().into_owned();
    let label = DocumentLabel {
        identifier: &identifier,
        diagnostic_label: &document,
    };
    let parsed = parse_forge_with_imports(text, label)?.expect("a view is a forge document");
    let dir = path.parent().expect("a fixture is in a directory");
    view::plan(&parsed, dir, &document).map(|plan| plan.expect("a view document has a plan"))
}

fn planned(name: &str) -> ViewPlan {
    read(name).unwrap_or_else(|e| panic!("{name} is read into a plan: {e:?}"))
}

/// The order the output of `plan` cuts.
fn result_order(plan: &ViewPlan) -> &[PlanOrderKey] {
    let query = query(plan);
    match &query.operators[query.operators.len() - 2].op {
        Operator::Order { by, .. } => by,
        other => panic!("the output cuts an order, not {other:?}"),
    }
}

/// The code the fixture `name` is refused by, and the refusal.
fn refused(name: &str) -> (DiagnosticCode, Located<ForgeError>) {
    let refusal = match read(name) {
        Ok(plan) => panic!("{name} is planned, and a refusal was expected: {plan:?}"),
        Err(refusal) => refusal,
    };
    (refusal.error.to_diagnostics()[0].code, refusal)
}

/// The 1-based row of the fixture `name` the text `needle` is first on.
fn row(name: &str, needle: &str) -> u32 {
    let text = std::fs::read_to_string(fixture(name)).expect("the fixture reads");
    let at = text
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{name} holds no {needle}"));
    u32::try_from(at + 1).expect("a fixture is short")
}

fn query(plan: &ViewPlan) -> &QueryPlan {
    match &plan.body {
        PlanBody::Query(query) => query,
        PlanBody::Relations(_) => panic!("{} is a document of relations", plan.view),
    }
}

/// The operators of `query` that are joins, where they stand.
fn joins(query: &QueryPlan) -> Vec<usize> {
    query
        .operators
        .iter()
        .enumerate()
        .filter(|(_, node)| matches!(node.op, Operator::Join { .. }))
        .map(|(at, _)| at)
        .collect()
}

#[test]
fn the_board_of_d1_is_read_into_one_plan() {
    let plan = planned("board.scxml");
    assert_eq!(plan.view, "board");
    assert_eq!(plan.viewer, Some(ViewerOf::Person));
    assert_eq!(plan.verify(), Ok(()));
    let query = query(&plan);

    // The tables, each with who may read a row.
    let names: Vec<&str> = query.tables.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["tasks", "projects", "people"]);
    match &query.tables[0].visibility {
        PlanVisibility::Relation {
            relations,
            relation,
            object_type,
            object,
            at,
        } => {
            assert_eq!(relations, "Access");
            assert_eq!(relation, "viewer");
            assert_eq!(object_type, "project");
            assert_eq!(object.text, "row.project");
            assert_eq!(
                at.as_ref().map(|at| at.text.as_str()),
                Some("row.relations_at")
            );
        }
        other => panic!("tasks are read through the viewer's relation, not {other:?}"),
    }
    assert!(
        matches!(&query.tables[1].visibility, PlanVisibility::Relation { object, .. } if object.text == "key"),
        "{:?}",
        query.tables[1].visibility
    );
    assert_eq!(query.tables[2].visibility, PlanVisibility::Public);

    // The operators, in the order the document writes them.
    let shape: Vec<&str> = query
        .operators
        .iter()
        .map(|node| match &node.op {
            Operator::Scan { .. } => "scan",
            Operator::Filter { .. } => "filter",
            Operator::Join { .. } => "join",
            Operator::Where { .. } => "where",
            Operator::Group { .. } => "group",
            Operator::Select { .. } => "select",
            Operator::Order { .. } => "order",
            Operator::Cut { .. } => "cut",
        })
        .collect();
    assert_eq!(
        shape,
        [
            "scan", "filter", "scan", "filter", "join", "scan", "filter", "join", "where", "group",
            "order", "cut"
        ]
    );
    assert_eq!(query.output, query.operators.len() - 1);

    // The plan is lifted over its viewers: every operator from a filter on
    // carries the viewer (D6).
    assert!(query.lifted);
    for (at, node) in query.operators.iter().enumerate() {
        let scan = matches!(node.op, Operator::Scan { .. });
        assert_eq!(node.carries_viewer, !scan, "operator {at}: {:?}", node.op);
    }

    // Both joins are on the right side's key, so the rows keep the key of the
    // tasks, and the second keeps a task no person is assigned (D2); each joins
    // two relations that carry the viewer, so on equal viewers too (D6).
    let at = joins(query);
    for (join, kind) in at.iter().zip([ViewJoinKind::Inner, ViewJoinKind::Left]) {
        match &query.operators[*join].op {
            Operator::Join {
                kind: written,
                by_right_key,
                on_viewers,
                ..
            } => {
                assert_eq!(*written, kind);
                assert!(*by_right_key, "join {join} is on the right side's key");
                assert!(*on_viewers, "join {join} is on equal viewers");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(query.operators[*join].key, [VIEWER, "t.key"]);
    }

    // The group gives the result's key, the project, after the viewer, and
    // its columns.
    let group = &query.operators[9];
    assert_eq!(group.key, [VIEWER, "project"]);
    let Operator::Group { outputs, by, .. } = &group.op else {
        panic!("{:?}", group.op);
    };
    assert_eq!(by[0].text, "p.key");
    let columns: Vec<&str> = group.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(columns, [VIEWER, "project", "title", "open", "next"]);
    assert!(matches!(&outputs[2], PlanAggregate::Count { name } if name == "open"));
    assert_eq!(query.result.record, "BoardRow");
    assert_eq!(query.result.key, ["project"]);
    assert_eq!(query.operators[query.output].key, [VIEWER, "project"]);

    // A view reads the document of relations it filters by, planned.
    assert_eq!(plan.reads.len(), 1);
    assert_eq!(plan.reads[0].alias, "Access");
    assert!(matches!(plan.reads[0].plan.body, PlanBody::Relations(_)));
}

#[test]
fn every_table_is_read_first_through_its_filter() {
    let plan = planned("board.scxml");
    let query = query(&plan);
    let mut scans = 0;
    for (at, node) in query.operators.iter().enumerate() {
        let Operator::Scan { table, .. } = &node.op else {
            continue;
        };
        scans += 1;
        let readers: Vec<&Operator> = query
            .operators
            .iter()
            .map(|n| &n.op)
            .filter(|op| op.inputs().contains(&at))
            .collect();
        assert_eq!(readers.len(), 1, "the scan of {table} is read once");
        assert!(
            matches!(readers[0], Operator::Filter { table: filtered, input } if filtered == table && *input == at),
            "the scan of {table} is read by its filter alone: {readers:?}"
        );
    }
    assert_eq!(scans, 3);
}

#[test]
fn a_plan_that_reads_a_table_before_its_filter_is_not_one_the_build_makes() {
    let plan = planned("board.scxml");
    let mut moved = plan.clone();
    let PlanBody::Query(query) = &mut moved.body else {
        panic!("the board is a query");
    };
    // The filter of the tasks moved after the first join: the join reads the
    // scan of the tasks, and the filter is no longer the first operator over
    // them.
    let first_join = joins(query)[0];
    let Operator::Join { left: filter, .. } = query.operators[first_join].op else {
        panic!("a join");
    };
    let Operator::Filter { input: scan, .. } = query.operators[filter].op else {
        panic!("the left side of the first join is the filter of the tasks");
    };
    if let Operator::Join { left, .. } = &mut query.operators[first_join].op {
        *left = scan;
    }
    assert_eq!(
        moved.verify(),
        Err(PlanDefect::ScanReadOutsideItsFilter {
            table: "tasks".into(),
            at: first_join,
        })
    );
    assert_eq!(plan.verify(), Ok(()));
}

#[test]
fn a_plan_that_joins_the_rows_of_two_viewers_is_not_one_the_build_makes() {
    let plan = planned("board.scxml");
    let mut crossed = plan.clone();
    let PlanBody::Query(query) = &mut crossed.body else {
        panic!("the board is a query");
    };
    // The first join on its own columns alone: a task one viewer may read meets
    // a project another may (D6).
    let first_join = joins(query)[0];
    if let Operator::Join { on_viewers, .. } = &mut query.operators[first_join].op {
        *on_viewers = false;
    }
    assert_eq!(
        crossed.verify(),
        Err(PlanDefect::JoinAcrossViewers { at: first_join })
    );
    assert_eq!(plan.verify(), Ok(()));
}

#[test]
fn a_plan_that_groups_the_rows_of_every_viewer_together_is_not_one_the_build_makes() {
    let plan = planned("board.scxml");
    let mut merged = plan.clone();
    let PlanBody::Query(query) = &mut merged.body else {
        panic!("the board is a query");
    };
    // The group keyed by the project alone: one row of a project for every
    // viewer at once (D6).
    query.operators[9].key.retain(|column| column != VIEWER);
    assert_eq!(merged.verify(), Err(PlanDefect::KeyWithoutViewer { at: 9 }));
}

#[test]
fn the_order_is_total_and_a_nest_is_held_in_its_order_and_cut() {
    let plan = planned("board.scxml");
    let query = query(&plan);
    let Operator::Order { by, .. } = &query.operators[query.operators.len() - 2].op else {
        panic!("the output cuts an order");
    };
    let keys: Vec<(&str, bool, bool)> = by
        .iter()
        .map(|k| (k.expr.text.as_str(), k.descending, k.from_key))
        .collect();
    assert_eq!(
        keys,
        [
            (VIEWER, false, true),
            ("open", true, false),
            ("project", false, true)
        ]
    );

    let Operator::Group { outputs, .. } = &query.operators[9].op else {
        panic!("the group");
    };
    let Some(PlanAggregate::Nest(nest)) =
        outputs.iter().find(|o| matches!(o, PlanAggregate::Nest(_)))
    else {
        panic!("the group nests the next tasks");
    };
    let order: Vec<(&str, bool)> = nest
        .order
        .iter()
        .map(|k| (k.expr.text.as_str(), k.from_key))
        .collect();
    assert_eq!(
        order,
        [(VIEWER, true), ("t.row.due", false), ("t.key", true)]
    );
    assert_eq!(nest.bound.name.as_deref(), Some("per_project"));
    assert_eq!(nest.bound.max, 5);
    assert_eq!(nest.record, "TaskLine");
    // A nest's total counts a group's rows, at most the tasks' 10 000, and the
    // result's the groups', at most the same: each in 16 bits (D2).
    assert_eq!(nest.total_bits, 16);
    assert_eq!(query.result.total_bits, 16);
}

#[test]
fn every_count_of_rows_is_a_term_of_the_named_bounds() {
    let plan = planned("board.scxml");
    let query = query(&plan);
    assert_eq!(query.result.bound.name.as_deref(), Some("board_rows"));
    assert_eq!(query.result.bound.max, 200);
    let tasks = &query.tables[0].size;
    assert_eq!(tasks.to_string(), "tasks_max");
    assert_eq!(tasks.most(), 10_000);
    // The joins are on keys, so the rows are the tasks' at most, and the cut
    // holds the result's bound of them.
    assert_eq!(query.operators[9].size.most(), 10_000);
    assert_eq!(query.operators[query.output].size.most(), 200);
    let parts: Vec<&str> = query.state.iter().map(|p| p.part.as_str()).collect();
    // A table is the machine's, once; every other part is held for each viewer
    // kept (D8).
    for part in &query.state {
        assert_eq!(
            part.per_viewer,
            !part.part.starts_with("table "),
            "{}",
            part.part
        );
    }
    for part in [
        "table tasks",
        "table projects",
        "table people",
        "join p",
        "join a",
        "group",
        "result",
    ] {
        assert!(
            parts.contains(&part),
            "{part} is a part of the state: {parts:?}"
        );
    }
    let join = query
        .state
        .iter()
        .find(|p| p.part == "join p")
        .expect("the join of the projects holds its sides");
    assert_eq!(
        join.size,
        SizeTerm::Sum(vec![
            query.tables[0].size.clone(),
            query.tables[1].size.clone()
        ])
    );
}

#[test]
fn the_relations_are_evaluated_in_strata_after_what_they_exclude() {
    let plan = planned("access.scxml");
    assert_eq!(plan.viewer, None);
    let PlanBody::Relations(relations) = &plan.body else {
        panic!("access is a document of relations");
    };
    let id = |object: &str, name: &str| RelationId {
        object: object.into(),
        name: name.into(),
    };
    assert_eq!(relations.strata.len(), 2);
    assert_eq!(relations.strata[1], [id("project", "viewer")]);
    assert!(relations.strata[0].contains(&id("project", "banned")));
    assert!(relations.strata[0].contains(&id("group", "member")));
    assert!(relations.strata[0].contains(&id("project", "approver")));
    assert_eq!(relations.tuples.record, "Tuple");
    assert_eq!(relations.size.to_string(), "tuples_max * tuples_max");
}

#[test]
fn the_plan_is_plain_data_that_reads_back() {
    let plan = planned("board.scxml");
    let json = serde_json::to_value(&plan).expect("a plan serialises");
    assert_eq!(json["body"]["body"], "query");
    let back: ViewPlan = serde_json::from_value(json).expect("a plan reads back");
    assert_eq!(back, plan);
}

#[test]
fn a_result_keyed_by_a_column_the_build_does_not_infer_is_refused() {
    let (code, refusal) = refused("key_not_unique.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewResultKeyNotUnique),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row("key_not_unique.scxml", "<sce:result"))
    );
}

#[test]
fn a_cut_with_no_order_is_refused() {
    let (code, refusal) = refused("bound_without_order.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewBoundWithoutOrder),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row("bound_without_order.scxml", "<sce:result"))
    );
}

#[test]
fn a_cut_that_reaches_every_row_needs_no_order() {
    let plan = planned("bound_reaching_every_row.scxml");
    let query = query(&plan);
    let Operator::Order { by, .. } = &query.operators[query.operators.len() - 2].op else {
        panic!("the output cuts an order");
    };
    // The order is the key alone, which the build appends.
    assert_eq!(by.len(), 1);
    assert_eq!(by[0].expr.text, "task");
    assert!(by[0].from_key);
    assert_eq!(query.operators[query.output].key, ["task"]);
}

#[test]
fn a_cycle_of_views_is_refused_where_it_closes() {
    let (code, refusal) = refused("cycle_a.scxml");
    assert!(matches!(code, DiagnosticCode::ViewCycle), "{code:?}");
    assert!(
        refusal.location.file.ends_with("cycle_b.scxml"),
        "{}",
        refusal.location.file
    );
    assert_eq!(
        refusal.location.line,
        Some(row("cycle_b.scxml", "src=\"cycle_a.scxml\""))
    );
    assert!(
        refusal
            .error
            .to_string()
            .contains("cycle_a -> cycle_b -> cycle_a"),
        "{}",
        refusal.error
    );
}

#[test]
fn a_relation_that_depends_on_its_own_exclusion_is_refused() {
    let (code, refusal) = refused("access_excluding_itself.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewRelationCycleThroughExclusion),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row(
            "access_excluding_itself.scxml",
            "<sce:relation name=\"viewer\""
        ))
    );
}

#[test]
fn a_table_that_says_nobody_may_read_it_is_refused() {
    let (code, refusal) = refused("table_without_visibility.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewTableWithoutVisibility),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row(
            "table_without_visibility.scxml",
            "<sce:table name=\"tasks\""
        ))
    );
}

#[test]
fn a_view_with_no_viewer_names_no_relation_of_a_viewer() {
    let (code, _) = refused("viewer_mismatch_relation.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewViewerMismatch),
        "{code:?}"
    );
}

#[test]
fn a_view_reads_no_view_of_another_viewer() {
    let (code, refusal) = refused("viewer_mismatch_view.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewViewerMismatch),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row(
            "viewer_mismatch_view.scxml",
            "<sce:table name=\"boards\""
        ))
    );
}

#[test]
fn an_expression_the_build_does_not_prove_total_is_refused() {
    let (code, refusal) = refused("expression_may_fail.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewExpressionMayFail),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row("expression_may_fail.scxml", "<sce:where"))
    );
}

#[test]
fn a_count_its_field_may_not_hold_is_refused() {
    let (code, refusal) = refused("aggregate_may_overflow.scxml");
    assert!(
        matches!(code, DiagnosticCode::ViewAggregateMayOverflow),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row("aggregate_may_overflow.scxml", "<sce:count"))
    );
}

#[test]
fn a_floating_sum_is_kept_in_a_long_accumulator_and_floating_numbers_are_ordered() {
    let plan = planned("floating_hours.scxml");
    let query = query(&plan);
    assert!(!query.lifted);
    let group = query
        .operators
        .iter()
        .find_map(|node| match &node.op {
            Operator::Group { outputs, .. } => Some(outputs),
            _ => None,
        })
        .expect("the hours are grouped by project");
    let PlanAggregate::Sum(sum) = &group[1] else {
        panic!("the hours are summed: {:?}", group[1]);
    };
    assert_eq!(sum.name, "hours");
    // binary32: 277 bits from the least subnormal to the most of the largest
    // finite value, 14 more for the carries of 10 000 terms, and a sign; read
    // as the float64 of its field (D1).
    assert_eq!(
        sum.accumulator,
        Accumulator::Floating {
            bits: 292,
            format: 64
        }
    );
    assert!(matches!(&group[2], PlanAggregate::Min(field) if field.name == "fewest"));
    assert!(matches!(&group[3], PlanAggregate::Max(field) if field.name == "most"));
    let keys: Vec<(&str, bool, bool)> = result_order(&plan)
        .iter()
        .map(|k| (k.expr.text.as_str(), k.descending, k.from_key))
        .collect();
    assert_eq!(keys, [("hours", true, false), ("project", false, true)]);
}

#[test]
fn an_absent_value_is_first_under_asc_unless_the_order_says_otherwise() {
    let plan = planned("absent_last.scxml");
    let keys: Vec<(&str, bool, bool)> = result_order(&plan)
        .iter()
        .map(|k| (k.expr.text.as_str(), k.descending, k.absent_first))
        .collect();
    // `absent last` on the assignee; the key appended keeps the default of asc.
    assert_eq!(keys, [("assignee", false, false), ("task", false, true)]);

    // Unwritten, an absent value is first under asc and last under desc (D2).
    for (written, absent_first) in [("assignee asc", true), ("assignee desc", false)] {
        let text = std::fs::read_to_string(fixture("absent_last.scxml"))
            .expect("the fixture reads")
            .replace("assignee asc absent last", written);
        assert!(
            text.contains(&format!("by=\"{written}\"")),
            "the order was rewritten"
        );
        let plan = read_text(&text, &fixture("absent_last.scxml"))
            .unwrap_or_else(|e| panic!("{written} is read into a plan: {e:?}"));
        assert_eq!(
            result_order(&plan)[0].absent_first,
            absent_first,
            "{written}"
        );
    }
}

#[test]
fn an_absent_term_on_a_value_never_absent_is_refused() {
    let (code, refusal) = refused("absent_on_a_value_never_absent.scxml");
    assert!(
        matches!(code, DiagnosticCode::ValidationAttributeRuleViolated),
        "{code:?}"
    );
    assert_eq!(
        refusal.location.line,
        Some(row("absent_on_a_value_never_absent.scxml", "<sce:order"))
    );
}

/// A count is the rows of its group and reads no expression: the grammar
/// refuses one written on it rather than leave it unread.
#[test]
fn a_count_takes_no_expression() {
    let text = std::fs::read_to_string(fixture("aggregate_may_overflow.scxml"))
        .expect("the fixture reads")
        .replace(
            "<sce:count name=\"open\"/>",
            "<sce:count name=\"open\" expr=\"t.row.points\"/>",
        );
    assert!(
        text.contains("expr=\"t.row.points\""),
        "the count was rewritten"
    );
    let label = DocumentLabel {
        identifier: "count_with_an_expression",
        diagnostic_label: "count_with_an_expression.scxml",
    };
    match parse_forge_with_imports(&text, label) {
        Ok(_) => panic!("a count with an expression is read"),
        Err(refusal) => {
            let code = refusal.error.to_diagnostics()[0].code;
            assert!(
                matches!(code, DiagnosticCode::XmlSchemaValidation),
                "{code:?}"
            );
        }
    }
}

#[test]
fn every_backend_refuses_a_valid_view_by_name() {
    let options = ForgeCompileOptions {
        go_module_prefix: Some("example.com/generated".to_string()),
        ..Default::default()
    };
    for &language in Language::ALL {
        let refusal =
            match sce_build::compile_forge_file(&fixture("board.scxml"), language, &[], &options) {
                Ok(_) => panic!("a view generated on {language:?}"),
                Err(refusal) => refusal,
            };
        let code = refusal.to_diagnostics()[0].code;
        assert!(
            matches!(code, DiagnosticCode::ViewBackendUnsupported),
            "{language:?}: {code:?}: {}",
            refusal.error
        );
    }
}
