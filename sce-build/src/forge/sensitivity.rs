// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The marks of an event schema's fields (`sce:sensitivity`, docs/adr/0109,
//! D14) wherever a statechart writes a value to a log line: a `<log>` of a
//! value read from a marked field writes the mark's marker in its place
//! (docs/adr/0109, Open question 9).
//!
//! A mark is declared once, on the schema, and the build follows it by type:
//!
//! - **Under `datamodel="sce-static"` the build knows the type of every place
//!   an expression reads** ([`crate::forge::types::TypeCtx::values`]). A
//!   `<log expr>` that reads a field a schema marks, at any depth, or a value
//!   that holds one (a record, a list of records, a map of them), writes the
//!   marker of the strongest mark it reads ([`mark_read`]): `sce-secret`, the
//!   text an audit entry holds in place of a secret, or `sce-personal` for a
//!   personal value. The whole value is replaced, not the read alone, since
//!   anything computed from a marked value (its length, a text joined to it, a
//!   comparison with it) says something of it. The replacement is made where
//!   the expression is lowered ([`crate::forge::static_lowering`]), once for
//!   every lane and the Interpreter's lowering, so no runtime differs.
//! - **The marked value is not computed for the log line.** The SCXML
//!   recommendation leaves the form of a logged value to the processor, which
//!   may convert it before it writes it; the conversion here writes the marker
//!   and computes nothing. Whether an operation on a marked value fails (a
//!   length less one, an overflow) says something of the value too, and an
//!   `error.execution` raised by it is an event the machine and its host see,
//!   so a log of a marked value neither writes it nor fails on it, and a
//!   `<log>` that reads only marked fields of the payload reads no payload.
//! - **A personal value is not pseudonymised in a log line.** The audit log
//!   writes it as its keyed digest so that equal values still correlate, under
//!   the log's pseudonymisation key in the host's key store (D7). A diagnostic
//!   log has no such key, no retention and no access rule of its own, and an
//!   unkeyed digest of an identifier is reversed by trying the identifiers, so
//!   a log line keeps no trace of the person: the marker alone.
//! - **Under `datamodel="ecmascript"` no value has a type**, so the build cannot
//!   tell a marked value from any other once it is read: it is assigned to a
//!   variable, handed to a `<script>`, or logged in a `<finalize>`, and which
//!   event a `<log>` of an `<onentry>` sees is not known before it runs. A
//!   statechart under any data model but `sce-static` that imports a schema
//!   marking a field, at any depth, is refused at the import
//!   ([`refuse_untyped`]) rather than run with a log that may write the value
//!   in the clear, as a schema that grew is refused there under the same data
//!   models: only the typed lift reads it as its schema says.
//!
//! A value carried out of a marked field into a place of no mark (a variable,
//! an unmarked field, a parameter of a child) is no longer marked, as an
//! audit entry's unmarked field is written as it stands: a document keeps a
//! secret in a field that marks it.

use std::collections::BTreeMap;

use crate::forge::error::{ForgeError, Located, ValidationError};
use crate::forge::model::{EventSchemaModel, ForgeKind, Sensitivity};
use crate::forge::types::TypeCtx;
use crate::forge::value_path::{Place, Step};
use crate::forge::value_type::{TypeScope, ValueType};
use crate::model::SCXMLModel;

/// The marks of a record's fields, by field id.
pub type FieldMarks = BTreeMap<String, Sensitivity>;

/// The marks of every record a statechart's types reach, keyed by the
/// record's identity ([`crate::forge::value_type::ScopedRecord::identity`]),
/// so a record a field holds at any depth is found by the document it is,
/// whatever alias names it where it is read. A record that marks no field is
/// not held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Marks {
    by_record: BTreeMap<String, FieldMarks>,
}

impl Marks {
    /// Hold the marks `schema` declares under `identity`; nothing for a schema
    /// that marks no field.
    pub fn note<T>(&mut self, identity: String, schema: &EventSchemaModel<T>) {
        let marks: FieldMarks = schema
            .fields
            .iter()
            .filter_map(|field| field.sensitivity.map(|mark| (field.id.clone(), mark)))
            .collect();
        if !marks.is_empty() {
            self.by_record.insert(identity, marks);
        }
    }

    /// The mark of the field `field` of the record `identity`, if it has one.
    pub fn field(&self, identity: &str, field: &str) -> Option<Sensitivity> {
        self.by_record.get(identity)?.get(field).copied()
    }

    /// Whether no record holds a mark.
    pub fn is_empty(&self) -> bool {
        self.by_record.is_empty()
    }

    /// The marks of every record a `sce-static` statechart's types reach: those
    /// its imports were read with ([`SCXMLModel::imported_marks`]), and the marks
    /// of each schema the model holds that the imports' scope does not, under the
    /// identity [`crate::forge::value_path::statechart_scope`] gives it there,
    /// its name.
    pub fn of_statechart(model: &SCXMLModel) -> Marks {
        let mut marks = model.imported_marks.clone();
        for (alias, schema) in &model.imported_records {
            if !model.imported_type_scope.records.contains_key(alias) {
                marks.note(schema.name.clone(), schema);
            }
        }
        marks
    }
}

/// The stronger of two marks either may lack.
fn stronger(a: Option<Sensitivity>, b: Option<Sensitivity>) -> Option<Sensitivity> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.stronger(b)),
        (a, b) => a.or(b),
    }
}

/// The strongest mark among the fields `expr` reads, in the places `ctx` types
/// ([`TypeCtx::values`]): a read of a marked field at any depth, and a read
/// of a value that holds one (a record, an element of a list of records, a map
/// of them). `None` for an expression that reads no marked field, for one
/// that does not parse (whose refusal is the lowering's), and wherever `marks`
/// holds none.
pub fn mark_read(expr: &str, ctx: &TypeCtx<'_>, marks: &Marks) -> Option<Sensitivity> {
    if marks.is_empty() {
        return None;
    }
    let ast = crate::forge::expr::parse_to_ast(expr.trim()).ok()?;
    let mut places = Vec::new();
    places_read(&ast, &mut places);
    places
        .iter()
        .map(|path| place_mark(path, ctx, marks))
        .fold(None, stronger)
}

/// The expression a `<log>` writes for `expr`: the marker of the strongest
/// mark `expr` reads ([`mark_read`]), as a string literal of the expression
/// language, or `None` when it reads no marked field and is written as it is.
pub fn logged_marker(expr: &str, ctx: &TypeCtx<'_>, marks: &Marks) -> Option<String> {
    mark_read(expr, ctx, marks).map(|mark| format!("'{}'", mark.log_marker()))
}

/// Every place `node` reads, as [`crate::forge::expr::place_path`] writes one:
/// a chain of member and index reads over a name is one place, read whole, and
/// the indices on the way are expressions of their own.
fn places_read(node: &crate::forge::expr::TypedExpr, out: &mut Vec<String>) {
    if let Some(path) = crate::forge::expr::place_path(node) {
        out.push(path);
        indices_read(node, out);
        return;
    }
    for child in node.children() {
        places_read(child, out);
    }
}

/// The places the indices of the chain `node` read.
fn indices_read(node: &crate::forge::expr::TypedExpr, out: &mut Vec<String>) {
    use crate::forge::expr::ExprKind;
    match &node.kind {
        ExprKind::Member { object, .. } => indices_read(object, out),
        ExprKind::Index { object, index } => {
            indices_read(object, out);
            places_read(index, out);
        }
        _ => {}
    }
}

/// The strongest mark the read of the place `path` meets: each field it steps
/// through, and every field of the value it reads whole. A path whose step the
/// value's type does not have (a method of a map, `m.has`) reads the value
/// before the step whole. `None` for a path no name of `ctx` starts.
fn place_mark(path: &str, ctx: &TypeCtx<'_>, marks: &Marks) -> Option<Sensitivity> {
    let base = ctx.value_scope;
    let path = path.trim();
    // The longest name at its start that holds a value: `_event.data` before
    // `_event`, as `crate::forge::value_path::resolve` finds it.
    let ends: Vec<usize> = path
        .match_indices(['.', '['])
        .map(|(at, _)| at)
        .chain(std::iter::once(path.len()))
        .collect();
    let (root, at) = ends
        .into_iter()
        .rev()
        .find_map(|at| ctx.values.get(&path[..at]).map(|root| (root, at)))?;
    let Some(steps) = crate::forge::value_path::steps(&path[at..]) else {
        return held_mark(base, root, marks);
    };
    let mut place = root.clone();
    let mut found = None;
    for step in &steps {
        if let Step::Field(name) = step {
            found = stronger(found, field_mark(base, &place, name, marks));
        }
        match crate::forge::value_path::walk(base, &place, std::slice::from_ref(step)) {
            Some(next) => place = next,
            None => return stronger(found, held_mark(base, &place, marks)),
        }
    }
    stronger(found, held_mark(base, &place, marks))
}

/// The mark of the field `name` of the record, or of any case of the union,
/// the place `at` holds.
fn field_mark(
    base: Option<&TypeScope>,
    at: &Place,
    name: &str,
    marks: &Marks,
) -> Option<Sensitivity> {
    let scope = crate::forge::value_path::scope_at(base?, &at.scope)?;
    match &at.ty {
        ValueType::Record { alias } => {
            let record = scope.records.get(alias)?;
            marks.field(&record.identity, name)
        }
        ValueType::Union { alias } => {
            let union = scope.unions.get(alias)?;
            union
                .cases
                .iter()
                .filter_map(|case| case.record.as_deref())
                .filter_map(|record| union.scope.records.get(record))
                .map(|record| marks.field(&record.identity, name))
                .fold(None, stronger)
        }
        _ => None,
    }
}

/// The strongest mark among the fields the value the place `at` holds reaches.
fn held_mark(base: Option<&TypeScope>, at: &Place, marks: &Marks) -> Option<Sensitivity> {
    let mut found = Vec::new();
    marked_fields(base, &at.ty, &at.scope, marks, "", &mut found);
    found
        .into_iter()
        .map(|(_, mark)| mark)
        .reduce(Sensitivity::stronger)
}

/// Every field a value of `ty`, read in the scope `chain` leads to from
/// `base`, reaches that a schema marks, as the JSON pointer of the field below
/// `prefix` (an element of a list, a value of a map, written `*`) with its
/// mark, in declaration order. A record already on the way down is not walked
/// again.
fn marked_fields(
    base: Option<&TypeScope>,
    ty: &ValueType,
    chain: &[String],
    marks: &Marks,
    prefix: &str,
    out: &mut Vec<(String, Sensitivity)>,
) {
    fn walk(
        base: Option<&TypeScope>,
        ty: &ValueType,
        chain: &[String],
        marks: &Marks,
        prefix: &str,
        seen: &mut Vec<String>,
        out: &mut Vec<(String, Sensitivity)>,
    ) {
        match ty {
            ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => {}
            ValueType::Optional { elem } => walk(base, elem, chain, marks, prefix, seen, out),
            ValueType::List { elem, .. } | ValueType::Set { key: elem, .. } => {
                walk(base, elem, chain, marks, &format!("{prefix}/*"), seen, out);
            }
            ValueType::Map { key, value, .. } => {
                walk(base, key, chain, marks, &format!("{prefix}/*"), seen, out);
                walk(base, value, chain, marks, &format!("{prefix}/*"), seen, out);
            }
            ValueType::Record { alias } => {
                let Some(record) = base
                    .and_then(|base| crate::forge::value_path::scope_at(base, chain))
                    .and_then(|scope| scope.records.get(alias))
                else {
                    return;
                };
                if seen.contains(&record.identity) {
                    return;
                }
                seen.push(record.identity.clone());
                let mut inner = chain.to_vec();
                inner.push(alias.clone());
                for (id, field) in &record.fields {
                    let pointer = format!("{prefix}/{id}");
                    if let Some(mark) = marks.field(&record.identity, id) {
                        out.push((pointer.clone(), mark));
                    }
                    walk(base, field, &inner, marks, &pointer, seen, out);
                }
                seen.pop();
            }
            ValueType::Union { alias } => {
                let Some(union) = base
                    .and_then(|base| crate::forge::value_path::scope_at(base, chain))
                    .and_then(|scope| scope.unions.get(alias))
                else {
                    return;
                };
                let mut inner = chain.to_vec();
                inner.push(alias.clone());
                for record in union.cases.iter().filter_map(|case| case.record.clone()) {
                    let case = ValueType::Record { alias: record };
                    walk(base, &case, &inner, marks, prefix, seen, out);
                }
            }
        }
    }
    walk(base, ty, chain, marks, prefix, &mut Vec::new(), out);
}

/// Refuse a statechart under any data model but `sce-static` that imports an
/// event schema, or a union, whose records mark a field at any depth: no value
/// of such a data model has a type a mark can follow (see the module's
/// documentation). Refused at the first such `<sce:import>`, naming the field it
/// marks and the mark, as `validation/attribute-rule-violated` on its `src`,
/// the code a schema that grew is refused by under the same data models
/// (`crate::parser`'s reading of the imported siblings). Run where the imports
/// are resolved, so a document is judged by every route that parses it.
pub fn refuse_untyped(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    if model.datamodel == crate::model::Datamodel::SceStatic {
        return Ok(());
    }
    let marks = Marks::of_statechart(model);
    if marks.is_empty() {
        return Ok(());
    }
    let scope = crate::forge::value_path::statechart_scope(model);
    for import in &model.forge_imports {
        let ty = match import.kind {
            ForgeKind::EventSchema => ValueType::Record {
                alias: import.alias.clone(),
            },
            ForgeKind::Union => ValueType::Union {
                alias: import.alias.clone(),
            },
            _ => continue,
        };
        let mut found = Vec::new();
        marked_fields(Some(&scope), &ty, &[], &marks, "", &mut found);
        if let Some((field, mark)) = found.first() {
            return Err(Located::new(
                ValidationError::AttributeRuleViolated {
                    element: format!("<sce:import kind=\"{}\">", import.kind),
                    attr: "src".into(),
                    value: import.src.clone(),
                    rule: untyped_rule(model.datamodel.as_str(), field, *mark),
                }
                .into(),
                diag_label,
                import.line,
                None,
            ));
        }
    }
    Ok(())
}

/// The rule [`refuse_untyped`] holds a statechart under `datamodel` to, for a
/// schema whose field at the pointer `field` carries `mark`.
fn untyped_rule(datamodel: &str, field: &str, mark: Sensitivity) -> String {
    format!(
        "a schema that marks no field under datamodel=\"{datamodel}\" — this one marks \
         `{field}` {} (sce:sensitivity), and only a datamodel=\"sce-static\" machine follows \
         a mark, writing its marker where a <log> reads a marked value; under any other data \
         model no value has a type, and a marked value could reach a log line in the clear \
         through a <log>, a <script> or a <finalize>",
        mark.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{Bound, ScopedCase, ScopedRecord, ScopedUnion, ScopedVariant};

    fn text(max: u32) -> ValueType {
        ValueType::Text {
            max: Bound::of(max),
        }
    }

    /// A statechart's scope that imports `SignIn` (`account` personal,
    /// `password` secret, `result` unmarked) and `Batch`, whose `attempts` is a
    /// list of `SignIn` records as its own document names them, and the marks.
    fn scope_and_marks() -> (TypeScope, Marks) {
        let sign_in = ScopedRecord {
            identity: "/here/schema_audit_sign_in.scxml".into(),
            name: "schema_audit_sign_in".into(),
            fields: vec![
                ("account".into(), text(64)),
                ("password".into(), text(32)),
                ("result".into(), text(16)),
            ],
            scope: TypeScope::default(),
            added: Default::default(),
            lineage: Default::default(),
        };
        let mut batch_scope = TypeScope::default();
        batch_scope
            .records
            .insert("Attempt".into(), sign_in.clone());
        let batch = ScopedRecord {
            identity: "/here/schema_batch.scxml".into(),
            name: "schema_batch".into(),
            fields: vec![
                (
                    "count".into(),
                    ValueType::Scalar(crate::forge::model::SceType::Uint32),
                ),
                (
                    "attempts".into(),
                    ValueType::List {
                        elem: Box::new(ValueType::Record {
                            alias: "Attempt".into(),
                        }),
                        max: Bound::of(4),
                    },
                ),
            ],
            scope: batch_scope,
            added: Default::default(),
            lineage: Default::default(),
        };
        let mut scope = TypeScope::default();
        scope.records.insert("SignIn".into(), sign_in);
        scope.records.insert("Batch".into(), batch);
        let mut marks = Marks::default();
        let mut by_field = FieldMarks::new();
        by_field.insert("account".into(), Sensitivity::Personal);
        by_field.insert("password".into(), Sensitivity::Secret);
        marks
            .by_record
            .insert("/here/schema_audit_sign_in.scxml".into(), by_field);
        (scope, marks)
    }

    fn record(alias: &str) -> Place {
        Place::of(ValueType::Record {
            alias: alias.into(),
        })
    }

    fn read(expr: &str) -> Option<Sensitivity> {
        let (scope, marks) = scope_and_marks();
        let mut ctx = TypeCtx::new();
        ctx.value_scope = Some(&scope);
        ctx.values.insert("attempt", record("SignIn"));
        ctx.values.insert("batch", record("Batch"));
        ctx.values.insert("_event.data", record("SignIn"));
        ctx.values.insert(
            "count",
            Place::of(ValueType::Scalar(crate::forge::model::SceType::Uint32)),
        );
        mark_read(expr, &ctx, &marks)
    }

    #[test]
    fn a_read_of_a_marked_field_carries_its_mark() {
        assert_eq!(read("attempt.password"), Some(Sensitivity::Secret));
        assert_eq!(read("attempt.account"), Some(Sensitivity::Personal));
        assert_eq!(read("_event.data.password"), Some(Sensitivity::Secret));
    }

    #[test]
    fn a_read_of_an_unmarked_field_carries_none() {
        assert_eq!(read("attempt.result"), None);
        assert_eq!(read("count + 1"), None);
        assert_eq!(read("'text'"), None);
        assert_eq!(read("batch.count"), None);
    }

    #[test]
    fn a_value_computed_from_a_marked_field_carries_its_mark() {
        assert_eq!(read("len(attempt.password)"), Some(Sensitivity::Secret));
        assert_eq!(
            read("'who: ' + attempt.account"),
            Some(Sensitivity::Personal)
        );
        assert_eq!(
            read("attempt.result + attempt.account + attempt.password"),
            Some(Sensitivity::Secret),
            "secret is the stronger mark"
        );
    }

    #[test]
    fn a_marked_field_of_a_record_a_list_holds_carries_its_mark() {
        assert_eq!(
            read("batch.attempts[0].password"),
            Some(Sensitivity::Secret)
        );
        assert_eq!(read("batch.attempts[count].result"), None);
        assert_eq!(read("len(batch.attempts)"), Some(Sensitivity::Secret));
    }

    #[test]
    fn a_marked_field_read_as_an_index_carries_its_mark() {
        assert_eq!(
            read("batch.attempts[len(attempt.password)].result"),
            Some(Sensitivity::Secret)
        );
    }

    #[test]
    fn a_record_read_whole_carries_the_strongest_mark_it_holds() {
        assert_eq!(read("attempt"), Some(Sensitivity::Secret));
        assert_eq!(read("_event.data"), Some(Sensitivity::Secret));
    }

    #[test]
    fn no_marks_mark_nothing() {
        let (scope, _) = scope_and_marks();
        let mut ctx = TypeCtx::new();
        ctx.value_scope = Some(&scope);
        ctx.values.insert("attempt", record("SignIn"));
        assert_eq!(mark_read("attempt.password", &ctx, &Marks::default()), None);
    }

    #[test]
    fn the_marker_is_a_string_literal_of_the_strongest_mark() {
        let (scope, marks) = scope_and_marks();
        let mut ctx = TypeCtx::new();
        ctx.value_scope = Some(&scope);
        ctx.values.insert("attempt", record("SignIn"));
        assert_eq!(
            logged_marker("attempt.password", &ctx, &marks).as_deref(),
            Some("'sce-secret'")
        );
        assert_eq!(
            logged_marker("attempt.account", &ctx, &marks).as_deref(),
            Some("'sce-personal'")
        );
        assert_eq!(logged_marker("attempt.result", &ctx, &marks), None);
    }

    #[test]
    fn a_union_case_field_carries_the_mark_any_case_gives_it() {
        let (mut scope, marks) = scope_and_marks();
        let sign_in = scope.records["SignIn"].clone();
        let mut union_scope = TypeScope::default();
        union_scope.records.insert("Attempt".into(), sign_in);
        scope.unions.insert(
            "Outcome".into(),
            ScopedUnion {
                identity: "/here/union_outcome.scxml".into(),
                name: "union_outcome".into(),
                tag: "kind".into(),
                tag_enum: "Kind".into(),
                cases: vec![
                    ScopedCase {
                        variant: ScopedVariant {
                            name: "failed".into(),
                            value: 0,
                        },
                        record: Some("Attempt".into()),
                    },
                    ScopedCase {
                        variant: ScopedVariant {
                            name: "none".into(),
                            value: 1,
                        },
                        record: None,
                    },
                ],
                scope: union_scope,
            },
        );
        let held = Place::of(ValueType::Union {
            alias: "Outcome".into(),
        });
        assert_eq!(
            field_mark(Some(&scope), &held, "password", &marks),
            Some(Sensitivity::Secret)
        );
        assert_eq!(field_mark(Some(&scope), &held, "result", &marks), None);
        let mut found = Vec::new();
        marked_fields(Some(&scope), &held.ty, &[], &marks, "", &mut found);
        assert_eq!(
            found,
            vec![
                ("/account".to_string(), Sensitivity::Personal),
                ("/password".to_string(), Sensitivity::Secret),
            ]
        );
    }

    #[test]
    fn a_nested_mark_is_named_by_its_pointer() {
        let (scope, marks) = scope_and_marks();
        let mut found = Vec::new();
        marked_fields(
            Some(&scope),
            &ValueType::Record {
                alias: "Batch".into(),
            },
            &[],
            &marks,
            "",
            &mut found,
        );
        assert_eq!(
            found,
            vec![
                ("/attempts/*/account".to_string(), Sensitivity::Personal),
                ("/attempts/*/password".to_string(), Sensitivity::Secret),
            ]
        );
    }

    const SIGN_IN_SCHEMA: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_sign_in"
       sce:event-name="sign_in.tried">
  <datamodel>
    <data id="account" sce:type="string[64]" sce:direction="in" sce:sensitivity="personal"/>
    <data id="password" sce:type="string[32]" sce:direction="in" sce:sensitivity="secret"/>
    <data id="result" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>"#;

    /// A machine under `datamodel` that logs the payload of `sign_in.tried`
    /// and, under `sce-static`, the fields of a record variable of its schema.
    fn logging(datamodel: &str) -> String {
        let record = if datamodel == "sce-static" {
            r#"<datamodel>
    <data id="attempt" sce:type="record:SignIn">
      <sce:set name="account" expr="'ada'"/>
      <sce:set name="password" expr="'hunter2'"/>
      <sce:set name="result" expr="'ok'"/>
    </data>
  </datamodel>
  <state id="idle">
    <transition event="go" type="internal">
      <log label="field_secret" expr="attempt.password"/>
      <log label="field_personal" expr="attempt.account"/>
      <log label="length_secret" expr="len(attempt.password)"/>
      <log label="field_plain" expr="attempt.result"/>
    </transition>"#
        } else {
            r#"<state id="idle">"#
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="{datamodel}" name="m">
  <sce:import kind="event-schema" src="schema_sign_in.scxml" as="SignIn"/>
  {record}
    <transition event="sign_in.tried" type="internal">
      <log label="payload_secret" expr="_event.data.password"/>
      <log label="payload_plain" expr="_event.data.result"/>
    </transition>
  </state>
</scxml>"#
        )
    }

    /// [`logging`] beside `schema`, parsed from their directory.
    fn parsed(
        datamodel: &str,
        schema: &str,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("schema_sign_in.scxml"), schema).expect("the schema");
        let path = dir.path().join("m.scxml");
        std::fs::write(&path, logging(datamodel)).expect("the statechart");
        let mut model =
            crate::parser::SCXMLParser::new().parse_file(path.to_str().expect("a UTF-8 path"))?;
        crate::analyzer::analyze(&mut model, "m.scxml");
        Ok(model)
    }

    /// What `lane` lowers each expression of [`logging`]'s `sce-static`
    /// machine to, by the expression as written.
    fn lowered(lane: &str) -> BTreeMap<String, String> {
        use crate::forge::static_lowering as lowering;
        let mut model = parsed("sce-static", SIGN_IN_SCHEMA).expect("the machine parses");
        let lowered = match lane {
            "rust" => lowering::lower_rust(&mut model, "m"),
            "kotlin" => lowering::lower_kotlin(&mut model, "m"),
            "cpp" => lowering::lower_cpp(&mut model, "m"),
            "go" => lowering::lower_go(&mut model, "m", None),
            "python" => lowering::lower_python(&mut model, "m"),
            "c11" => lowering::lower_c11(&mut model, ""),
            "swift" => lowering::lower_swift(&mut model, "m"),
            other => unreachable!("no lane {other}"),
        }
        .unwrap_or_else(|e| panic!("{lane} lowers the machine: {e:?}"));
        lowered
            .sites
            .into_iter()
            .map(|site| (site.source, site.text))
            .collect()
    }

    /// Pinned by the `<log>` arm of `crate::forge::static_lowering`'s walk,
    /// which lowers [`logged_marker`] in place of the expression.
    #[test]
    fn every_lane_lowers_a_log_of_a_marked_field_to_its_marker_alone() {
        for lane in ["rust", "kotlin", "cpp", "go", "python", "c11", "swift"] {
            let sites = lowered(lane);
            let site = |source: &str| {
                sites
                    .get(source)
                    .unwrap_or_else(|| panic!("{lane}: `{source}` is lowered"))
            };
            for (source, marker) in [
                ("_event.data.password", "sce-secret"),
                ("attempt.password", "sce-secret"),
                ("attempt.account", "sce-personal"),
                ("len(attempt.password)", "sce-secret"),
            ] {
                let text = site(source);
                assert!(
                    text.contains(marker)
                        && !text.contains("password")
                        && !text.contains("account"),
                    "{lane}: `{source}` is logged as `{marker}` alone, not as `{text}`"
                );
            }
            for source in ["_event.data.result", "attempt.result"] {
                let text = site(source);
                assert!(
                    text.contains("result") && !text.contains("sce-"),
                    "{lane}: `{source}` is logged as it is, not as `{text}`"
                );
            }
        }
    }

    /// Pinned by [`refuse_untyped`], run where the parser resolves the imports.
    #[test]
    fn an_ecmascript_machine_that_imports_a_marked_schema_is_refused_at_the_import() {
        let refusal = parsed("ecmascript", SIGN_IN_SCHEMA)
            .expect_err("a marked schema is not imported untyped")
            .to_string();
        assert!(
            refusal.contains("<sce:import kind=\"event-schema\">")
                && refusal.contains("'schema_sign_in.scxml'")
                && refusal.contains("`/account` personal"),
            "{refusal}"
        );
    }

    #[test]
    fn an_ecmascript_machine_that_imports_an_unmarked_schema_is_accepted() {
        let unmarked = SIGN_IN_SCHEMA
            .replace(r#" sce:sensitivity="personal""#, "")
            .replace(r#" sce:sensitivity="secret""#, "");
        let model = parsed("ecmascript", &unmarked).expect("an unmarked schema is imported");
        assert!(model.imported_marks.is_empty());
    }

    #[test]
    fn the_untyped_rule_names_the_data_model_the_field_and_the_mark() {
        let rule = untyped_rule("ecmascript", "/password", Sensitivity::Secret);
        assert!(
            rule.contains("datamodel=\"ecmascript\"")
                && rule.contains("`/password` secret")
                && rule.contains("datamodel=\"sce-static\""),
            "{rule}"
        );
    }

    #[test]
    fn the_parse_holds_the_marks_of_the_imported_schema() {
        let model = parsed("sce-static", SIGN_IN_SCHEMA).expect("the machine parses");
        let marks = Marks::of_statechart(&model);
        let identity = &model.imported_type_scope.records["SignIn"].identity;
        assert_eq!(marks.field(identity, "password"), Some(Sensitivity::Secret));
        assert_eq!(
            marks.field(identity, "account"),
            Some(Sensitivity::Personal)
        );
        assert_eq!(marks.field(identity, "result"), None);
    }

    #[test]
    fn secret_is_the_stronger_mark() {
        assert_eq!(
            stronger(Some(Sensitivity::Personal), Some(Sensitivity::Secret)),
            Some(Sensitivity::Secret)
        );
        assert_eq!(
            stronger(None, Some(Sensitivity::Personal)),
            Some(Sensitivity::Personal)
        );
        assert_eq!(stronger(None, None), None);
    }
}
