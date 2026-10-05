// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The static data model's expressions, judged (docs/SCE_ACCEPTED_SUBSET.md
// §2.15).
//
// Under `datamodel="sce-static"` every expression is written in the forge
// expression language against a closed scope — the declared variables, the
// triggering event's typed payload, each record variable's fields, the
// imported enums, `In()` — gathered by [`crate::forge::type_ctx::StaticScope`]. This pass judges each one
// where it lands: a `<data>` initialiser and an `<assign>` against the
// variable's type, a condition as `bool`, a logged or sent value as whatever
// it is. A refusal is placed at the expression's own range.
//
// An expression attribute the model has no typed form for is refused rather
// than passed on: every backend's templates read one as script-engine text,
// so admitting it would evaluate part of a document in a language it never
// declared — the failure the `datamodel` attribute exists to prevent.

use crate::forge::error::{ForgeError, Located};
use crate::forge::expr::{judge_into, Expected};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::type_ctx::{StaticEnum, StaticScope};
use crate::forge::types::{InferredType, TypeCtx};
use crate::model::{Action, Datamodel, DoneDataParam, Invoke, Param, SCXMLModel, Variable};
use crate::scxml_semantic::ScxmlSemanticError;

/// A `<param>` whose value crosses as data — a `<send>`'s, a host-run
/// `<invoke>`'s, a `<donedata>`'s — as the judge and the lowering both read
/// it, so the three elements are held to one rule and lowered by one routine.
///
/// A `location` names a variable, and reading one is reading it as an
/// expression: `written` is whichever of the two the param has.
pub(crate) struct WireParam<'a> {
    pub name: &'a str,
    /// A string literal is folded at build time and crosses as written.
    pub is_static_literal: bool,
    /// The expression, or the location read as one; empty when the param has
    /// neither, which is a run-time error of its own.
    pub written: &'a str,
    pub spelling: Option<&'a crate::attribute_spelling::AttributeSpelling>,
    pub source_location: Option<&'a crate::forge::error::SourceLocation>,
}

impl<'a> WireParam<'a> {
    pub(crate) fn of_param(param: &'a Param) -> Self {
        let (written, spelling) = if param.expr.trim().is_empty() {
            (param.location.as_str(), param.location_spelling.as_ref())
        } else {
            (param.expr.as_str(), param.expr_spelling.as_ref())
        };
        Self {
            name: &param.name,
            is_static_literal: param.is_static_literal,
            written,
            spelling,
            source_location: param.source_location.as_ref(),
        }
    }

    /// A name of a `namelist`, of a `<send>` or of an `<invoke>` a host runs:
    /// the variable it names, read as the expression it is. The model records
    /// no position for a name, so a refusal is placed at the attribute when
    /// the element records one (`spelling`), and at the element when it does
    /// not (`at`).
    pub(crate) fn of_namelist_name(
        name: &'a str,
        spelling: Option<&'a crate::attribute_spelling::AttributeSpelling>,
        at: Option<&'a crate::forge::error::SourceLocation>,
    ) -> Self {
        Self {
            name,
            is_static_literal: false,
            written: name,
            spelling,
            source_location: at,
        }
    }

    /// §scxml-5.7 admits exactly one of `expr` and `location` on a donedata
    /// `<param>`; an `expr` that is empty is no expression, and falls to the
    /// `location` the same way [`Self::of_param`] does.
    pub(crate) fn of_done_param(param: &'a DoneDataParam) -> Self {
        let (written, spelling) = match (&param.expr, &param.location) {
            (Some(expr), _) if !expr.trim().is_empty() => {
                (expr.as_str(), param.expr_spelling.as_ref())
            }
            (_, Some(location)) => (location.as_str(), param.location_spelling.as_ref()),
            _ => ("", None),
        };
        Self {
            name: &param.name,
            is_static_literal: false,
            written,
            spelling,
            source_location: param.source_location.as_ref(),
        }
    }
}

/// The attributes of executable content that carry an expression this model
/// does not type, with the element each belongs to. Refused where written.
const UNTYPED_ACTION_ATTRIBUTES: &[(&str, &str)] = &[("send", "targetexpr"), ("send", "typeexpr")];

/// The most bytes of the id a machine generates for a `<send idlocation>`:
/// `_auto_send_` and the twenty digits of the largest `u64` count. The number
/// every runtime's `AUTO_SEND_ID_MAX_LEN` states
/// (`backends/rust/runtime/src/helpers/unique_id_generator.rs`), which a test
/// holds this one to.
pub const AUTO_SEND_ID_MAX_BYTES: u32 = 31;

/// Judge every expression of a `sce-static` document; a document under any
/// other data model is not this pass's to judge.
///
/// `enums` are the document's imported enums
/// ([`StaticEnum::from_imports`]); empty for a document read with no
/// directory to resolve its imports against.
pub fn check(
    model: &SCXMLModel,
    enums: &[StaticEnum],
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let Some(scope) = StaticScope::of(model) else {
        return Ok(());
    };
    let judge = Judge {
        scope: &scope,
        enums,
        schemas: &model.imported_records,
        loop_records: Default::default(),
        loop_names: Default::default(),
        enum_vars: enum_variables(&scope, &model.imported_records),
        loop_enum_paths: Default::default(),
        payload_enum_paths: Default::default(),
        payload_schema: Default::default(),
        diag_label,
        read: Default::default(),
    };
    // Every record variable's fields, readable everywhere; a transition adds
    // its payload to these.
    let record_paths = scope.paths(None);
    let plain = judge.ctx(&record_paths);

    for var in &scope.variables {
        if let Some(alias) = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            judge.record(&plain, var, alias, &model.imported_records)?;
            continue;
        }
        if let Some(alias) = judge.enum_vars.get(&var.id) {
            judge.enum_variable(&plain, var, alias, &model.imported_enums)?;
            continue;
        }
        if let Some(alias) = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::list_elem)
            .and_then(crate::forge::model::ListElemType::record_alias)
        {
            if let Some(schema) = model.imported_records.get(alias) {
                judge.schema_enums_imported(var, alias, schema)?;
            }
        }
        if var.expr.trim().is_empty() {
            continue;
        }
        let slot = variable_type(var);
        judge.expr(
            &plain,
            &var.expr,
            var.expr_spelling.as_ref(),
            Expected::Slot(slot),
        )?;
        judge.string_start(var)?;
    }
    judge.actions(&plain, &model.global_scripts, "")?;

    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        for block in state.on_entry_blocks.iter().chain(&state.on_exit_blocks) {
            judge.actions(&plain, block, &state.id)?;
        }
        judge.actions(&plain, &state.initial_transition_actions, &state.id)?;
        judge.actions(&plain, &state.initial_history_default_actions, &state.id)?;
        for transition in &state.transitions {
            // The payload is the triggering event's, so each transition
            // judges against its own.
            let schema = model.imported_event_schemas.get(&transition.event);
            let paths = scope.paths(schema);
            let ctx = judge.ctx(&paths);
            // An enum field of the payload is a value of the enum the schema
            // names, as a record's field is, and the payload taken whole is a
            // record of its schema.
            judge.open_payload(schema);
            if !transition.cond.trim().is_empty()
                && !transition.is_cpp_condition
                && !transition.is_kt_condition
            {
                judge.expr(
                    &ctx,
                    &transition.cond,
                    transition.cond_spelling.as_ref(),
                    Expected::Slot(InferredType::Bool),
                )?;
            }
            judge.actions(&ctx, &transition.actions, &state.id)?;
        }
        judge.open_payload(None);
        for invoke in &state.invokes {
            judge.invoke(&plain, invoke, &state.id)?;
        }
        if let Some(done) = &state.donedata {
            for param in &done.params {
                judge.wire_param(
                    &plain,
                    &WireParam::of_done_param(param),
                    "<donedata>",
                    &state.id,
                )?;
            }
            // The record it names crosses as the pairs of its fields, read when
            // the state is entered, where no event's payload is in scope.
            if let crate::model::DoneDataContent::Expression(expr) = &done.content {
                judge.content_record(
                    &plain,
                    "<donedata>",
                    expr,
                    done.content_spelling.as_ref(),
                    !done.params.is_empty(),
                    &state.id,
                )?;
            }
        }
    }
    // An imported algorithm is imported to be called. One nothing calls is
    // refused rather than dropped in silence — a statechart keeps no import
    // for later, and a stale one would outlive the call it was for.
    let read = judge.read.borrow();
    if let Some(unused) = scope.callees.iter().find(|c| !read.contains(&c.alias)) {
        return Err(Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct: format!("<sce:import kind=\"algorithm\" as=\"{}\">", unused.alias),
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: format!(
                    "an imported algorithm is called by the document, and nothing calls `{}`",
                    unused.alias
                ),
                state: String::new(),
                observed: Some(unused.alias.clone()),
            }
            .into(),
            diag_label,
            unused.line,
            None,
        ));
    }
    Ok(())
}

/// The refusal of a list variable read as a value
/// ([`StaticScope::list_read_as_value`]) — one wording wherever it is found.
pub(crate) fn list_read_refusal(list: &str) -> crate::forge::expr::Refusal {
    crate::forge::error::ExprError::UnsupportedConstruct {
        construct: format!(
            "reading the list `{list}` as a value (a list is filled by <sce:append>, \
             emptied by <sce:clear>, measured by len({list}), and read by the host \
             through the snapshot)"
        ),
        observed: Some(list.to_string()),
    }
    .at(None)
}

/// A variable's declared type; `Unknown` for an enum, whose width the
/// inference layer declines to claim ([`InferredType::from_sce_type`]).
fn variable_type(var: &Variable) -> InferredType {
    var.value_type
        .as_ref()
        .and_then(crate::forge::model::AlgorithmValueType::scalar)
        .map_or(InferredType::Unknown, InferredType::from_sce_type)
}

/// The type a value handed to the child's variable `var` is held to
/// (§scxml-6.4.1), or `None` for a variable no value can be handed to — a
/// list, a record, an enum or bytes. The one rule the judge and the lowering of
/// a child's arguments share.
pub(crate) fn seed_slot(var: &Variable) -> Option<InferredType> {
    match var.value_type.as_ref()?.scalar()? {
        crate::forge::model::SceType::Bytes | crate::forge::model::SceType::Enum(_) => None,
        scalar => Some(InferredType::from_sce_type(scalar)),
    }
}

/// The enum each variable declared `enum:<alias>` holds, by the variable's id,
/// and each enum field of a record variable, by its `<id>.<field>` path —
/// what [`crate::forge::static_enum`] asks to tell an enum value from a number
/// that happens to share its inferred type.
pub(crate) fn enum_variables(
    scope: &StaticScope,
    records: &std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
) -> std::collections::BTreeMap<String, String> {
    let mut held = std::collections::BTreeMap::new();
    for var in &scope.variables {
        let Some(value_type) = var.value_type.as_ref() else {
            continue;
        };
        if let Some(crate::forge::model::SceType::Enum(reference)) = value_type.scalar() {
            held.insert(var.id.clone(), reference.alias.clone());
        }
        if let Some(schema) = value_type.record_alias().and_then(|a| records.get(a)) {
            held.extend(enum_fields(&var.id, schema));
        }
    }
    held
}

/// Each enum field of a record of `schema` held under the name `holder`, by
/// its `<holder>.<field>` path and the alias of the enum it holds.
pub(crate) fn enum_fields(
    holder: &str,
    schema: &crate::forge::model::EventSchemaModel,
) -> Vec<(String, String)> {
    schema
        .fields
        .iter()
        .filter_map(|field| match &field.sce_type {
            crate::forge::model::SceType::Enum(reference) => {
                Some((format!("{holder}.{}", field.id), reference.alias.clone()))
            }
            _ => None,
        })
        .collect()
}

struct Judge<'a> {
    scope: &'a StaticScope,
    enums: &'a [StaticEnum],
    /// The schemas the document imports, by alias — the fields of a record a
    /// loop walks are read from them.
    schemas: &'a std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
    /// The record items of the `<foreach>`es the walk is inside, each with the
    /// alias of the schema it holds — a record an append may take by name.
    loop_records: std::cell::RefCell<Vec<(String, String)>>,
    /// The enum fields of the record items of the `<foreach>`es the walk is
    /// inside, by `<item>.<field>` path with the alias of the enum each holds.
    loop_enum_paths: std::cell::RefCell<Vec<(String, String)>>,
    /// The enum fields of the payload of the transition the walk is in, by
    /// `_event.data.<field>` path with the alias of the enum each holds.
    payload_enum_paths: std::cell::RefCell<Vec<(String, String)>>,
    /// The schema of the payload of the transition the walk is in, which
    /// `_event.data` taken whole is a record of.
    payload_schema: std::cell::RefCell<Option<crate::forge::model::EventSchemaModel>>,
    /// Every item and index of the `<foreach>`es the walk is inside: values
    /// the loop binds and the body reads, and does not write.
    loop_names: std::cell::RefCell<Vec<String>>,
    /// Each enum-typed variable's id and the alias of the enum it holds.
    enum_vars: std::collections::BTreeMap<String, String>,
    diag_label: &'a str,
    /// Every name an expression of the document reads or calls, so an
    /// imported algorithm nothing calls is found.
    read: std::cell::RefCell<std::collections::BTreeSet<String>>,
}

impl<'a> Judge<'a> {
    fn ctx<'c>(&self, payload: &'c [(String, InferredType)]) -> TypeCtx<'c>
    where
        'a: 'c,
    {
        self.scope.ctx(payload, self.enums)
    }

    /// `expr` judged against `expected`, refused at its own range.
    ///
    /// A `list<T>` variable is not a value an expression reads: it is filled
    /// by `<sce:append>`, emptied by `<sce:clear>`, measured by `len(…)`, and
    /// read by the host through the snapshot. Anything but the measure is
    /// refused here, the one place every expression of the document passes,
    /// before the scope — which types it as a list — is asked.
    fn expr(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        expected: Expected,
    ) -> Result<InferredType, Located<ForgeError>> {
        let place = |refusal: crate::forge::expr::Refusal| {
            Located::in_file(
                ExpressionSite::new(expr, spelling).place(refusal),
                self.diag_label,
            )
        };
        self.record_reads(expr);
        if let Some(list) = self.scope.list_read_as_value(expr) {
            return Err(place(list_read_refusal(&list)));
        }
        let ty = judge_into(expr, ctx, expected).map_err(place)?;
        // An enum value is typed `Unknown`, so the slot cannot refuse it a
        // number's place: say here where it may stand and where it may not.
        if let Some(alias) = self.value_enum(ctx, expr).map_err(place)? {
            if let Expected::Slot(slot) = expected {
                if !matches!(slot, InferredType::Unknown) {
                    return Err(place(
                        crate::forge::error::ExprError::UnsupportedConstruct {
                            construct: format!("a value of the enum `{alias}` where a number or a bool is expected"),
                            observed: Some(expr.trim().to_string()),
                        }
                        .at(None),
                    ));
                }
            }
        }
        Ok(ty)
    }

    /// The enum `expr` is a value of ([`crate::forge::static_enum`]), refused
    /// when it uses one for what an enum value is not for.
    fn value_enum(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
    ) -> Result<Option<String>, crate::forge::expr::Refusal> {
        crate::forge::static_enum::value_enum(expr, ctx, &|name| self.enum_of(name))
    }

    /// The alias of the enum the variable, record field, loop's record item's
    /// field or the payload's field at `path` holds, if it holds one.
    fn enum_of(&self, path: &str) -> Option<String> {
        let held_at = |paths: &std::cell::RefCell<Vec<(String, String)>>| {
            paths
                .borrow()
                .iter()
                .find(|(held, _)| held == path)
                .map(|(_, alias)| alias.clone())
        };
        self.enum_vars
            .get(path)
            .cloned()
            .or_else(|| held_at(&self.loop_enum_paths))
            .or_else(|| held_at(&self.payload_enum_paths))
    }

    /// The alias of the enum the variable, record field or payload field at
    /// `path` holds, for a value that crosses as data: [`Self::enum_of`] but
    /// for a record item of a loop, which has no wire spelling yet.
    fn enum_of_wire_value(&self, path: &str) -> Option<String> {
        self.enum_vars.get(path).cloned().or_else(|| {
            self.payload_enum_paths
                .borrow()
                .iter()
                .find(|(held, _)| held == path)
                .map(|(_, alias)| alias.clone())
        })
    }

    /// Opens the walk of a transition, whose event's payload `schema` is
    /// declared, to the payload: the enum fields it holds, each a value of the
    /// enum the schema names, and the schema itself, which `_event.data` taken
    /// whole is a record of. Whether the document imports that enum under the
    /// alias the schema writes is asked where the payload is read, which is
    /// where the lowering knows it is.
    fn open_payload(&self, schema: Option<&crate::forge::model::EventSchemaModel>) {
        *self.payload_enum_paths.borrow_mut() = schema
            .map(|schema| enum_fields(crate::forge::event_schema_check::EVENT_DATA_PATH, schema))
            .unwrap_or_default();
        *self.payload_schema.borrow_mut() = schema.cloned();
    }

    /// `expr`, which a variable declared `enum:<alias>` is to hold: a variant
    /// of that enum, another variable of it, or a conditional of the two.
    fn expr_of_enum(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        alias: &str,
    ) -> Result<(), Located<ForgeError>> {
        let place = |refusal: crate::forge::expr::Refusal| {
            Located::in_file(
                ExpressionSite::new(expr, spelling).place(refusal),
                self.diag_label,
            )
        };
        self.expr(ctx, expr, spelling, Expected::Hint(InferredType::Unknown))?;
        match self.value_enum(ctx, expr).map_err(place)? {
            Some(held) if held == alias => Ok(()),
            _ => Err(place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a value that is not of the enum `{alias}` for a variable declared \
                         enum:{alias} (write `{alias}.<variant>`, or another variable of that enum)"
                    ),
                    observed: Some(expr.trim().to_string()),
                }
                .at(None),
            )),
        }
    }

    /// A variable declared `enum:<alias>`: the enum is a closed set the
    /// document imports, and the variable starts at one of its variants.
    fn enum_variable(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        enums: &std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
    ) -> Result<(), Located<ForgeError>> {
        // An alias the parser admitted names an import; without sibling files
        // it has no enum here, and nothing to judge against.
        if let Some(model) = enums.get(alias) {
            // A machine holds a declared variant and nothing else; the open
            // set's value that no variant names has no type to be held in.
            if !model.strict_variants {
                let spelling = var.value_type_spelling.as_ref();
                return Err(self.rule_at(
                    format!("<data id=\"{}\" sce:type=\"enum:{alias}\">", var.id),
                    "a variable holds a closed enum: the enum declares \
                     sce:strict-variants=\"false\", which admits values no variant names",
                    spelling.map(|s| s.row()),
                    spelling.map(|s| s.col()),
                    "",
                    alias,
                ));
            }
        }
        // A `<data>` with no `expr` is refused before this pass is asked, so
        // the value is written.
        self.expr_of_enum(ctx, &var.expr, var.expr_spelling.as_ref(), alias)
    }

    /// Note every name `expr` reads or calls ([`Judge::read`]).
    fn record_reads(&self, expr: &str) {
        if let Ok(names) = crate::forge::expr::read_identifiers(expr) {
            self.read.borrow_mut().extend(names);
        }
    }

    /// A string variable starts at a string literal that fits its bound. The
    /// machine is built with no error to raise (§scxml-4.9: there is no block
    /// to end yet), so a value that could fail to fit is refused where it is
    /// written, not carried over from another variable at run time.
    fn string_start(&self, var: &Variable) -> Result<(), Located<ForgeError>> {
        let (Some(capacity), Some(crate::forge::model::SceType::String)) = (
            var.capacity,
            var.value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::scalar),
        ) else {
            return Ok(());
        };
        let literal = match crate::forge::expr::parse_to_ast(&var.expr).map(|ast| ast.kind) {
            Ok(crate::forge::expr::ExprKind::StringLit { value, .. }) => value,
            _ => {
                return Err(self.rule_at(
                    format!("<data id=\"{}\" expr=\"{}\">", var.id, var.expr),
                    "a string variable starts at a string literal: the machine is built with \
                     no error to raise for a value that does not fit its sce:capacity",
                    var.expr_spelling.as_ref().map(|s| s.row()),
                    var.expr_spelling.as_ref().map(|s| s.col()),
                    "",
                    &var.expr,
                ));
            }
        };
        if literal.len() > capacity as usize {
            return Err(self.rule_at(
                format!("<data id=\"{}\" expr=\"{}\">", var.id, var.expr),
                &format!(
                    "the initial value is {} UTF-8 bytes, past the sce:capacity of {capacity} \
                     the variable declares",
                    literal.len()
                ),
                var.expr_spelling.as_ref().map(|s| s.row()),
                var.expr_spelling.as_ref().map(|s| s.col()),
                "",
                &var.expr,
            ));
        }
        Ok(())
    }

    /// The `list<T>` variable `name` names, if it names one.
    fn list_var(&self, name: &str) -> Option<&Variable> {
        self.scope.variables.iter().find(|v| {
            v.id == name.trim()
                && v.value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem)
                    .is_some()
        })
    }

    /// The refusal of an `<sce:append>` / `<sce:clear>` whose `target` names
    /// no list variable, placed on `target` and naming the lists there are.
    fn not_a_list(&self, action: &Action, state: &str) -> Located<ForgeError> {
        let lists: Vec<&str> = self
            .scope
            .variables
            .iter()
            .filter(|v| {
                v.value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem)
                    .is_some()
            })
            .map(|v| v.id.as_str())
            .collect();
        let spelling = action.spellings.get("target");
        let element = action.action_type.trim_start_matches("sce_");
        Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct: format!("<sce:{element} target=\"{}\">", action.location),
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: if lists.is_empty() {
                    "target names a list variable, and this document declares none".to_string()
                } else {
                    format!("target names a list variable: one of {}", lists.join(", "))
                },
                state: state.to_string(),
                observed: (!action.location.is_empty()).then(|| action.location.clone()),
            }
            .into(),
            self.diag_label,
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
        )
    }

    /// A `record:<alias>` variable built whole: one `<sce:set>` per field
    /// of the schema `alias` names ([`crate::forge::generator::order_record_fields`],
    /// the rule an algorithm's record local keeps), each value judged against
    /// its field's type.
    fn record(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        records: &std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
    ) -> Result<(), Located<ForgeError>> {
        // An alias the parser admitted names an import; without sibling
        // files to read it has no schema here, and nothing to judge against.
        let Some(schema) = records.get(alias) else {
            return Ok(());
        };
        let declared: Vec<&str> = schema.fields.iter().map(|f| f.id.as_str()).collect();
        let ordered = crate::forge::generator::order_record_fields(
            &var.record_fields,
            &declared,
            alias,
            format!("<data id=\"{}\">", var.id),
            "sce:type",
            &var.value_type
                .as_ref()
                .map(|t| t.as_attr())
                .unwrap_or_default(),
            var.value_type_spelling.as_ref(),
        )
        .map_err(|error| Located::in_file(error, self.diag_label))?;
        self.schema_enums_imported(var, alias, schema)?;
        for (field, init) in schema.fields.iter().zip(ordered) {
            // A field of an enum starts at one of its variants, as a variable
            // of it does.
            if let crate::forge::model::SceType::Enum(reference) = &field.sce_type {
                self.expr_of_enum(
                    ctx,
                    &init.expr,
                    init.expr_spelling.as_ref(),
                    &reference.alias,
                )?;
                continue;
            }
            self.expr(
                ctx,
                &init.expr,
                init.expr_spelling.as_ref(),
                Expected::Slot(InferredType::from_sce_type(&field.sce_type)),
            )?;
        }
        Ok(())
    }

    /// A record whose schema has an enum field holds it in the machine's own
    /// type for that enum, so the document imports the enum under the alias the
    /// schema writes. Judged where the record is declared, in a variable or in
    /// the elements of a list, and only when the imports were read: without
    /// sibling files there is no schema either.
    fn schema_enums_imported(
        &self,
        var: &Variable,
        alias: &str,
        schema: &crate::forge::model::EventSchemaModel,
    ) -> Result<(), Located<ForgeError>> {
        for field in &schema.fields {
            let crate::forge::model::SceType::Enum(reference) = &field.sce_type else {
                continue;
            };
            if self.enums.iter().any(|e| e.alias == reference.alias) {
                continue;
            }
            let spelling = var.value_type_spelling.as_ref();
            return Err(self.rule_at(
                format!("<data id=\"{}\"> of record:{alias}", var.id),
                &format!(
                    "the field `{}` of record:{alias} holds the enum `{}`: the document \
                     imports it under that alias, <sce:import kind=\"enum\" as=\"{}\">",
                    field.id, reference.alias, reference.alias
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &reference.alias,
            ));
        }
        Ok(())
    }

    /// The refusal of an expression attribute this model has no typed form
    /// for, placed on the attribute.
    fn untyped(
        &self,
        construct: String,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        state: &str,
        value: &str,
    ) -> Located<ForgeError> {
        self.untyped_at(
            construct,
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
            state,
            value,
        )
    }

    /// [`Self::untyped`] placed at `line`/`col` — for a construct the model
    /// records by its element's position rather than by an attribute's.
    fn untyped_at(
        &self,
        construct: String,
        line: Option<u32>,
        col: Option<u32>,
        state: &str,
        value: &str,
    ) -> Located<ForgeError> {
        self.rule_at(
            construct,
            "this data model types the expressions of <data>, a condition, \
             <assign>, <log> and <param>; this attribute has no typed form",
            line,
            col,
            state,
            value,
        )
    }

    /// A refusal under the rule `rule`, placed at `line`/`col`: the one shape
    /// every refusal of this pass takes, whatever the rule says.
    fn rule_at(
        &self,
        construct: String,
        rule: &str,
        line: Option<u32>,
        col: Option<u32>,
        state: &str,
        value: &str,
    ) -> Located<ForgeError> {
        Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct,
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: rule.to_string(),
                state: state.to_string(),
                observed: (!value.is_empty()).then(|| value.to_string()),
            }
            .into(),
            self.diag_label,
            line,
            col,
        )
    }

    /// A `<send>`'s `delayexpr`: a string, the CSS2 time the delay is written
    /// in (`5s`, `100ms`), computed from the machine's fields when the send
    /// runs — `wait + 'ms'` for an integer `wait`. A number alone is no time and
    /// is refused where it is written, as the type of any other string slot is;
    /// the delay's own text is read as a time when the send runs, and one that
    /// is none is an argument that cannot be evaluated. The element takes one
    /// delay, a written one or an expression, so a `delay` beside it is refused.
    fn delay_expr(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("delayexpr");
        if !action.delay.trim().is_empty() {
            return Err(self.rule_at(
                format!("delayexpr=\"{}\"", action.delayexpr),
                "a <send> carries its delay as `delay` or as `delayexpr`, and never as both",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.delayexpr,
            ));
        }
        self.expr(
            ctx,
            &action.delayexpr,
            spelling,
            Expected::Slot(InferredType::Str),
        )?;
        Ok(())
    }

    /// A `<send>`'s `eventexpr`: a string, the name of the event the send
    /// delivers, computed from the machine's fields when the send runs. A name
    /// that is empty names no event and is an argument that cannot be
    /// evaluated, as it is under every data model. The element names its event
    /// one way, a written one or an expression, so an `event` beside it is
    /// refused.
    fn event_expr(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("eventexpr");
        if !action.event.trim().is_empty() {
            return Err(self.rule_at(
                format!("eventexpr=\"{}\"", action.eventexpr),
                "a <send> names its event as `event` or as `eventexpr`, and never as both",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.eventexpr,
            ));
        }
        self.expr(
            ctx,
            &action.eventexpr,
            spelling,
            Expected::Slot(InferredType::Str),
        )?;
        Ok(())
    }

    /// A `<cancel>`'s `sendidexpr`: a string, the id of the send to cancel,
    /// computed from the machine's fields when the cancel runs. An id no send
    /// holds cancels nothing, as it does under every data model. The element
    /// names the send one way, a written id or an expression, so a `sendid`
    /// beside it is refused.
    fn sendid_expr(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("sendidexpr");
        if !action.sendid.trim().is_empty() {
            return Err(self.rule_at(
                format!("sendidexpr=\"{}\"", action.sendidexpr),
                "a <cancel> names its send as `sendid` or as `sendidexpr`, and never as both",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.sendidexpr,
            ));
        }
        self.expr(
            ctx,
            &action.sendidexpr,
            spelling,
            Expected::Slot(InferredType::Str),
        )?;
        Ok(())
    }

    /// A `<send>`'s `idlocation`: the string variable the machine writes the id
    /// it generates for the send to, so that a later `<cancel>` can name it. The
    /// id is written before any other argument is read, and is the send's own,
    /// so the element takes one id, a written one or a generated one.
    ///
    /// The machine is built with no error to raise for a write that does not fit
    /// (§scxml-4.9), so a variable that declares a bound declares one the id
    /// always fits: [`AUTO_SEND_ID_MAX_BYTES`] plus the terminator a C buffer
    /// holds, 32. And the place is a variable the document declares — not a
    /// record's field, a list's element, or what a `<foreach>` binds, which
    /// would be a copy no one reads.
    fn send_idlocation(&self, action: &Action, state: &str) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("idlocation");
        let refuse = |rule: &str| {
            self.rule_at(
                format!("idlocation=\"{}\"", action.idlocation),
                rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.idlocation,
            )
        };
        if !action.id.trim().is_empty() {
            return Err(refuse(
                "a <send> carries its id as `id` or has the machine generate one for \
                 `idlocation`, and never both",
            ));
        }
        let location = action.idlocation.trim();
        let variable = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == location)
            .filter(|_| !self.loop_names.borrow().iter().any(|name| name == location));
        let is_string = variable
            .and_then(|v| v.value_type.as_ref())
            .and_then(crate::forge::model::AlgorithmValueType::scalar)
            == Some(&crate::forge::model::SceType::String);
        let Some(variable) = variable.filter(|_| is_string) else {
            return Err(refuse(
                "an idlocation names a string variable the document declares: the id is a \
                 string, and the place it is written to is one variable, not a record's \
                 field, a list's element or what a <foreach> binds",
            ));
        };
        match variable.capacity {
            Some(capacity) if capacity <= AUTO_SEND_ID_MAX_BYTES => Err(refuse(&format!(
                "the id a machine generates is up to {AUTO_SEND_ID_MAX_BYTES} UTF-8 bytes, \
                 and the variable it is written to declares an sce:capacity of {capacity}: \
                 a bound the id fits, with room for a C buffer's terminator, is at least {}",
                AUTO_SEND_ID_MAX_BYTES + 1
            ))),
            _ => Ok(()),
        }
    }

    /// The `<content expr>` of a `<send>` or of a `<final>`'s `<donedata>`
    /// (`element`): the one value this model has that is an object, a record,
    /// taken whole by name — a record variable, the item of a `<foreach>` over a
    /// list of records, or the payload of the event the transition is on
    /// (`_event.data`). It crosses as the pairs of its fields, each the
    /// `<param name="f" expr="record.f"/>` it abbreviates
    /// ([`crate::model::Action::fold_content_record_into_params`]) and held to
    /// the rule a param is. The element carries its data one way, as content or
    /// as pairs, so a `<param>` or a `namelist` `beside` it is refused.
    fn content_record(
        &self,
        ctx: &TypeCtx<'_>,
        element: &str,
        written: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        beside: bool,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let written = written.trim();
        let (line, col) = (spelling.map(|s| s.row()), spelling.map(|s| s.col()));
        let construct = format!("<content expr=\"{written}\">");
        if beside {
            return Err(self.rule_at(
                construct,
                &format!(
                    "a {element} carries its data as <content> or as <param>s (and a namelist), \
                     and never as both"
                ),
                line,
                col,
                state,
                written,
            ));
        }
        let Some(fields) = self.content_fields(written) else {
            return Err(self.rule_at(
                construct,
                "a <content expr> of this data model names a record, which crosses as the pairs \
                 of its fields: a record variable, or, in the content of a transition, the \
                 item of a <foreach> over a list of records or the payload of an event whose \
                 schema it is (`_event.data`)",
                line,
                col,
                state,
                written,
            ));
        };
        for field in &fields {
            let path = format!("{written}.{field}");
            let pair = WireParam {
                name: field,
                is_static_literal: false,
                written: &path,
                spelling,
                source_location: None,
            };
            self.wire_param(ctx, &pair, element, state)?;
        }
        Ok(())
    }

    /// The ids of the fields of the record `written` names, or `None` when it
    /// names no record: a record variable, a loop's record item, or the payload
    /// of the transition's event when that event declares a schema.
    fn content_fields(&self, written: &str) -> Option<Vec<String>> {
        let ids = |schema: &crate::forge::model::EventSchemaModel| {
            schema.fields.iter().map(|f| f.id.clone()).collect()
        };
        let by_alias = |alias: &str| self.schemas.get(alias).map(ids);
        if let Some(alias) = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == written)
            .and_then(|v| v.value_type.as_ref())
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            return by_alias(alias);
        }
        if let Some((_, alias)) = self
            .loop_records
            .borrow()
            .iter()
            .find(|(item, _)| item == written)
        {
            return by_alias(alias);
        }
        if written == crate::forge::event_schema_check::EVENT_DATA_PATH {
            return self.payload_schema.borrow().as_ref().map(ids);
        }
        None
    }

    /// A `<param>` of a `<send>`, of an `<invoke>` the host runs or of a
    /// `<donedata>`, judged as the value that crosses as data (SCE Accepted
    /// Subset §2.15): to the host, or on the event a `<final>` raises.
    ///
    /// Its expression is read from the machine's fields when the element runs,
    /// and lowered to native code that makes the value both the text the
    /// pair crosses as and the JSON value in the event's data. That holds for
    /// the values every backend spells alike
    /// ([`InferredType::wire_param_slot`]); any other is refused where it is
    /// written rather than dropped, or carried differently by two backends. A
    /// value read from the triggering event's payload is carried on as any
    /// other, where the walk is in a transition on an event that declares the
    /// schema the payload is read through: the transition's content then runs
    /// only for a delivery that carried one, as an `<assign>` that reads it
    /// does. Anywhere else (an entry, an exit, an invoke, a `<final>`, an event
    /// with no schema) no payload is in scope, and the read is refused.
    ///
    /// `element` names the element a refusal places the param in. A `location`
    /// is read as an expression naming a variable, which is what it is.
    fn wire_param(
        &self,
        ctx: &TypeCtx<'_>,
        param: &WireParam<'_>,
        element: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        if param.is_static_literal {
            return Ok(());
        }
        let (written, spelling) = (param.written, param.spelling);
        if written.trim().is_empty() {
            return Ok(());
        }
        // A param is placed at its own element, a `namelist` name (which has
        // none) at the attribute it is written in.
        let (line, col) = match (param.source_location, spelling) {
            (Some(at), _) => (at.line, at.col),
            (None, Some(spelling)) => (Some(spelling.row()), Some(spelling.col())),
            (None, None) => (None, None),
        };
        let construct = format!("<param name=\"{}\"> of {element}", param.name);
        if crate::forge::expr::references_event_data_lexically(written)
            && self.payload_schema.borrow().is_none()
        {
            return Err(self.rule_at(
                construct,
                "a <param> of this data model is read from the machine's fields when its \
                 element runs; the triggering event's payload is read only in a transition \
                 on an event that declares a schema, so copy the value into a variable \
                 there first",
                line,
                col,
                state,
                written,
            ));
        }
        let ty = self.expr(
            ctx,
            written,
            spelling,
            Expected::Hint(InferredType::Unknown),
        )?;
        // An enum value crosses as the name its enum declares for it, a string
        // as a saved state holds one, so every backend spells it alike. Only a
        // variable, a field of a record variable or a field of the payload is
        // read that way: a value of a loop's record item has no lowering here
        // yet.
        let held_by_a_variable = |name: &str| self.enum_of_wire_value(name);
        match crate::forge::static_enum::value_enum(written, ctx, &held_by_a_variable) {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {}
            Err(refusal) => {
                return Err(Located::in_file(
                    ExpressionSite::new(written, spelling).place(refusal),
                    self.diag_label,
                ))
            }
        }
        if ty.wire_param_slot().is_none() {
            return Err(self.rule_at(
                construct,
                "a <param> crosses as text and as a JSON value, which every \
                 backend spells alike for a bool, a string, an integer of at most 32 bits, \
                 a real, and an enum value held by a variable or by a field of a record \
                 variable (as the name its enum declares); a 64-bit integer (which a \
                 backend that reads numbers through a double would carry with its low bits \
                 wrong), bytes, a list, a record and an enum value of a loop's item have \
                 no such spelling yet",
                line,
                col,
                state,
                written,
            ));
        }
        Ok(())
    }

    fn actions(
        &self,
        ctx: &TypeCtx<'_>,
        actions: &[Action],
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        for action in actions {
            self.action(ctx, action, state)?;
        }
        Ok(())
    }

    fn action(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let kind = action.action_type.as_str();
        for (element, attr) in UNTYPED_ACTION_ATTRIBUTES {
            if kind != *element {
                continue;
            }
            let value = action_attribute(action, attr);
            if !value.is_empty() {
                return Err(self.untyped(
                    format!("{attr}=\"{value}\""),
                    action.spellings.get(attr),
                    state,
                    value,
                ));
            }
        }
        match kind {
            "assign" => {
                // A record is built whole and updated a field at a time
                // (SCE_FORGE.md §4.12): assigning one whole is refused, as it
                // is to an algorithm's record local.
                let location = action.location.trim();
                // A loop's item and index are bound by the loop: its item is
                // an element of the list as it was, and writing a field of a
                // record item would change a copy no one reads.
                let root = location.split('.').next().unwrap_or(location).trim();
                if self.loop_names.borrow().iter().any(|name| name == root) {
                    return Err(Located::in_file(
                        ExpressionSite::new(&action.location, action.spellings.get("location"))
                            .place(
                                crate::forge::error::ExprError::UnsupportedConstruct {
                                    construct: format!(
                                        "an assignment to `{location}`, which a <foreach> \
                                         binds (its item and index are read, not written)"
                                    ),
                                    observed: Some(location.to_string()),
                                }
                                .at(None),
                            ),
                        self.diag_label,
                    ));
                }
                // Nothing in an expression makes a record, so a whole record is
                // assigned from one that exists, by its name: a record
                // variable of its schema, or a loop's record item.
                if let Some(alias) = self
                    .scope
                    .variables
                    .iter()
                    .find(|v| v.id == location)
                    .and_then(|v| v.value_type.as_ref())
                    .and_then(crate::forge::model::AlgorithmValueType::record_alias)
                {
                    return self.record_of(
                        "assign",
                        &action.expr,
                        action.spellings.get("expr"),
                        alias,
                        state,
                    );
                }
                if self.list_var(location).is_some() {
                    return Err(Located::in_file(
                        ExpressionSite::new(&action.location, action.spellings.get("location"))
                            .place(
                                crate::forge::error::ExprError::UnsupportedConstruct {
                                    construct: format!(
                                        "an assignment to the whole list `{location}` (a list \
                                         is filled by <sce:append> and emptied by <sce:clear>)"
                                    ),
                                    observed: Some(location.to_string()),
                                }
                                .at(None),
                            ),
                        self.diag_label,
                    ));
                }
                // The location is a declared variable; its type is the slot.
                let slot = self.expr(
                    ctx,
                    &action.location,
                    action.spellings.get("location"),
                    Expected::Hint(InferredType::Unknown),
                )?;
                // An enum variable's slot is its enum, which no inferred type
                // names: the value must be one of that enum's.
                if let Some(alias) = self.enum_of(location) {
                    self.expr_of_enum(ctx, &action.expr, action.spellings.get("expr"), &alias)?;
                } else {
                    self.expr(
                        ctx,
                        &action.expr,
                        action.spellings.get("expr"),
                        Expected::Slot(slot),
                    )?;
                }
            }
            "if" if !action.is_cpp_condition && !action.is_kt_condition => {
                self.expr(
                    ctx,
                    &action.cond,
                    action.spellings.get("cond"),
                    Expected::Slot(InferredType::Bool),
                )?;
            }
            "log" if !action.expr.trim().is_empty() => {
                self.expr(
                    ctx,
                    &action.expr,
                    action.spellings.get("expr"),
                    Expected::Hint(InferredType::Unknown),
                )?;
            }
            "send" => {
                for param in &action.params {
                    self.wire_param(ctx, &WireParam::of_param(param), "<send>", state)?;
                }
                // A `namelist` name is the `<param name="x" expr="x"/>` it
                // abbreviates ([`Action::fold_namelist_into_params`]), held to
                // the rule a param is, and placed at the attribute.
                for name in action.namelist.split_whitespace() {
                    let namelist =
                        WireParam::of_namelist_name(name, action.spellings.get("namelist"), None);
                    self.wire_param(ctx, &namelist, "<send>", state)?;
                }
                if !action.eventexpr.is_empty() {
                    self.event_expr(ctx, action, state)?;
                }
                if !action.delayexpr.is_empty() {
                    self.delay_expr(ctx, action, state)?;
                }
                if !action.idlocation.is_empty() {
                    self.send_idlocation(action, state)?;
                }
                if !action.contentexpr.is_empty() {
                    self.content_record(
                        ctx,
                        "<send>",
                        &action.contentexpr,
                        action.contentexpr_spelling.as_ref(),
                        !action.params.is_empty() || !action.namelist.trim().is_empty(),
                        state,
                    )?;
                }
            }
            "cancel" => {
                if !action.sendidexpr.is_empty() {
                    self.sendid_expr(ctx, action, state)?;
                }
            }
            // SCE Accepted Subset §2.15: the value appended is judged
            // against the list's element, as a typed assignment is judged
            // against its variable — the rule E8 holds an algorithm's list to.
            "sce_append" => {
                let Some(list) = self.list_var(&action.location) else {
                    return Err(self.not_a_list(action, state));
                };
                match list
                    .value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem)
                {
                    // A record is appended whole, by the name of a record of
                    // the list's schema: nothing computes one in an expression.
                    Some(crate::forge::model::ListElemType::Record { alias }) => {
                        self.record_of(
                            "sce:append",
                            &action.expr,
                            action.spellings.get("expr"),
                            alias,
                            state,
                        )?;
                    }
                    elem => {
                        let elem = elem
                            .and_then(crate::forge::model::ListElemType::scalar)
                            .map_or(InferredType::Unknown, InferredType::from_sce_type);
                        self.expr(
                            ctx,
                            &action.expr,
                            action.spellings.get("expr"),
                            Expected::Slot(elem),
                        )?;
                    }
                }
            }
            "sce_clear" => {
                if self.list_var(&action.location).is_none() {
                    return Err(self.not_a_list(action, state));
                }
            }
            // A host action's arguments are judged by
            // `native_action::validate`; what they call still counts as a
            // call of an imported algorithm.
            "native_action" => {
                for arg in &action.params {
                    self.record_reads(&arg.expr);
                }
            }
            // The body is judged in the scope the loop's variables are added
            // to, so nothing below it is asked of this one.
            "foreach" => return self.foreach(ctx, action, state),
            _ => {}
        }
        // Everything nested inside, through the model's one definition of
        // what lies inside an action. An `<elseif>`'s condition travels with
        // its block.
        for block in action.nested_blocks() {
            if let Some(cond) = block.cond.filter(|_| !block.cond_is_native) {
                self.expr(
                    ctx,
                    cond,
                    block.cond_spelling,
                    Expected::Slot(InferredType::Bool),
                )?;
            }
            self.actions(ctx, block.actions, state)?;
        }
        Ok(())
    }

    /// A `<foreach>` over a list variable (§scxml-4.6): `item` is each element,
    /// typed as the list's element, and `index` — when written — its position,
    /// a `uint32` as `len` is. Both are local to the body, so each is a name
    /// nothing in scope already means, a name the generated code can declare in
    /// every backend (it is spelled as written), and not the other's.
    fn foreach(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let array = action.array.trim();
        let place = |attr: &str, rule: String, value: &str| {
            let spelling = action
                .spellings
                .get(attr)
                .or_else(|| action.spellings.get("array"));
            self.rule_at(
                format!("<foreach {attr}=\"{value}\">"),
                &rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                value,
            )
        };
        let Some(list) = self.list_var(array) else {
            return Err(place(
                "array",
                "a <foreach> of this data model walks a list variable: array names one of the \
                 lists the machine declares"
                    .to_string(),
                array,
            ));
        };
        let Some(elem) = list
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::list_elem)
        else {
            return Err(place(
                "array",
                "a <foreach> of this data model walks a list variable".to_string(),
                array,
            ));
        };
        let item = action.item.trim();
        let index = action.index.trim();
        if item.is_empty() {
            return Err(place(
                "item",
                "a <foreach> names the variable that holds each element".to_string(),
                item,
            ));
        }
        for (attr, name) in [("item", item), ("index", index)] {
            if name.is_empty() {
                continue;
            }
            let why = if !crate::scxml_identifier::is_code_identifier(name) {
                Some("a name the generated code can spell: a letter or `_`, then letters, digits or `_`")
            } else if name.starts_with("sce_") {
                Some(
                    "a name that does not begin `sce_`, which the generated code keeps for its own",
                )
            } else if crate::generator::Language::ALL
                .iter()
                .any(|l| crate::reader_names::is_reserved_word(*l, name))
            {
                Some("a name no backend reserves as a keyword")
            } else if ctx.declares(name) {
                Some(
                    "a name nothing in scope already means: a variable, an enum, an imported \
                     algorithm or an enclosing loop's variable, which this one would hide",
                )
            } else if attr == "index" && name == item {
                Some("a name other than the item's")
            } else {
                None
            };
            if let Some(why) = why {
                return Err(place(attr, format!("a <foreach> {attr} is {why}"), name));
            }
        }
        let loop_variables = crate::forge::type_ctx::LoopVariables::new(
            item,
            (!index.is_empty()).then_some(index),
            elem,
            self.schemas,
        );
        let inner = loop_variables.bind(ctx);
        // A record item is a record an append in the body may take by name.
        let record_item = match elem {
            crate::forge::model::ListElemType::Record { alias } => Some(alias.clone()),
            crate::forge::model::ListElemType::Scalar(_) => None,
        };
        let enum_paths = record_item
            .as_deref()
            .and_then(|alias| self.schemas.get(alias))
            .map(|schema| enum_fields(item, schema))
            .unwrap_or_default();
        let enum_paths_bound = enum_paths.len();
        if let Some(alias) = &record_item {
            self.loop_records
                .borrow_mut()
                .push((item.to_string(), alias.clone()));
        }
        self.loop_enum_paths.borrow_mut().extend(enum_paths);
        let bound = 1 + usize::from(!index.is_empty());
        {
            let mut names = self.loop_names.borrow_mut();
            names.push(item.to_string());
            if !index.is_empty() {
                names.push(index.to_string());
            }
        }
        let judged = self.actions(&inner, &action.actions, state);
        let kept = self.loop_names.borrow().len() - bound;
        self.loop_names.borrow_mut().truncate(kept);
        let kept = self.loop_enum_paths.borrow().len() - enum_paths_bound;
        self.loop_enum_paths.borrow_mut().truncate(kept);
        if record_item.is_some() {
            self.loop_records.borrow_mut().pop();
        }
        judged
    }

    /// `expr`, which an `<sce:append>` to a list of `record:<alias>` or an
    /// `<assign>` to a record variable of it takes (`element`), is the name of
    /// a record of that schema — a record variable declared `record:<alias>`,
    /// or the record item of a `<foreach>` over a list of it: a record is built
    /// by its `<sce:set>`s and updated a field at a time, so nothing in an
    /// expression makes one.
    fn record_of(
        &self,
        element: &str,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        alias: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let written = expr.trim();
        let names_one = self.scope.variables.iter().any(|v| {
            v.id == written
                && v.value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::record_alias)
                    == Some(alias)
        }) || self
            .loop_records
            .borrow()
            .iter()
            .any(|(item, held)| item == written && held == alias);
        if names_one {
            return Ok(());
        }
        // The payload of the event the transition is on is a record of its
        // schema, taken whole as `_event.data` — when that schema declares the
        // fields the record's does, each of the type it does.
        if written == crate::forge::event_schema_check::EVENT_DATA_PATH {
            let payload = self.payload_schema.borrow();
            let record = self.schemas.get(alias);
            if let (Some(payload), Some(record)) = (payload.as_ref(), record) {
                let shape = |schema: &crate::forge::model::EventSchemaModel| {
                    let mut fields: Vec<_> = schema
                        .fields
                        .iter()
                        .map(|f| (f.id.clone(), f.sce_type.clone()))
                        .collect();
                    fields.sort_by(|a, b| a.0.cmp(&b.0));
                    fields
                };
                if shape(payload) == shape(record) {
                    return Ok(());
                }
                return Err(self.rule_at(
                    format!("<{element} expr=\"{written}\">"),
                    &format!(
                        "the payload of the event is not a record of the schema {alias}: it \
                         declares other fields, or the same fields of other types"
                    ),
                    spelling.map(|s| s.row()),
                    spelling.map(|s| s.col()),
                    state,
                    written,
                ));
            }
        }
        Err(self.rule_at(
            format!("<{element} expr=\"{written}\">"),
            &format!(
                "a whole record of the schema {alias} is taken by name: a record variable \
                 declared record:{alias}, the item of a <foreach> over a list of it, or the \
                 payload of an event whose schema it is (`_event.data`)"
            ),
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
            state,
            written,
        ))
    }

    /// §scxml-6.4.1: each `<param>` and `namelist` name of an `<invoke
    /// type="scxml">` is bound, in the child session it starts, to the child's
    /// variable of the same name. Under this model that variable is a native
    /// field, which only the child's own code sets, so a value is accepted only
    /// where the child hands it one: the child must be a `sce-static` document
    /// this build read, must declare the name as a top-level `<data>`, and the
    /// value must be of that variable's type — the check an `<assign>` to it
    /// would get. A name the child does not declare would be dropped, as would
    /// a value handed to a list, a record, an enum or bytes, so each is refused
    /// where it is written rather than accepted and never delivered.
    fn child_arguments(
        &self,
        ctx: &TypeCtx<'_>,
        info: &crate::model::ScxmlInvokeInfo,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let base = &info.common.base;
        let element = format!("<invoke id=\"{}\">", base.invoke_id);
        let mut handed = std::collections::BTreeSet::new();
        for param in info.arguments() {
            let (written, spelling) = if param.expr.trim().is_empty() {
                (param.location.as_str(), param.location_spelling.as_ref())
            } else {
                (param.expr.as_str(), param.expr_spelling.as_ref())
            };
            let at = param.source_location.as_ref();
            let refuse = |rule: &str| {
                self.rule_at(
                    format!("<param name=\"{}\"> of {element}", param.name),
                    rule,
                    at.and_then(|l| l.line),
                    at.and_then(|l| l.col),
                    state,
                    written,
                )
            };
            if written.trim().is_empty() {
                return Err(refuse(
                    "a <param> hands the child a value, and this one names none: give it an \
                     expr or a location",
                ));
            }
            if !handed.insert(param.name.clone()) {
                return Err(refuse(
                    "the child is handed this name twice, and one value would hide the other: \
                     hand it once",
                ));
            }
            let Some(declared) = info.common.child_static_variables.as_deref() else {
                return Err(refuse(
                    "the child of this invoke is not a datamodel=\"sce-static\" document this \
                     build read, so the value has no typed variable to arrive in: write the \
                     child inline or beside this document, under the same data model",
                ));
            };
            let Some(variable) = declared.iter().find(|v| v.id == param.name) else {
                return Err(refuse(&format!(
                    "the child declares no top-level <data id=\"{}\">, so the value would be \
                     dropped: declare it in the child, or hand a name it does declare",
                    param.name
                )));
            };
            let Some(slot) = seed_slot(variable) else {
                return Err(refuse(&format!(
                    "the child's `{}` is a list, a record, an enum or bytes, and a value is \
                     handed only to a bool, a string, an integer or a real",
                    param.name
                )));
            };
            self.expr(ctx, written, spelling, Expected::Slot(slot))?;
        }
        Ok(())
    }

    fn invoke(
        &self,
        ctx: &TypeCtx<'_>,
        invoke: &Invoke,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        // A hybrid invoke names its child by an expression evaluated at run
        // time — its `srcexpr` or its `<content expr>`.
        if let Invoke::Hybrid(info) = invoke {
            let (attr, value, spelling) = if info.srcexpr.is_empty() {
                (
                    "contentexpr",
                    &info.contentexpr,
                    info.contentexpr_spelling.as_ref(),
                )
            } else {
                ("srcexpr", &info.srcexpr, info.srcexpr_spelling.as_ref())
            };
            return Err(self.untyped(format!("{attr}=\"{value}\""), spelling, state, value));
        }
        let base = invoke.base();
        let at = base.source_location.as_ref();
        let (line, col) = (at.and_then(|l| l.line), at.and_then(|l| l.col));
        // A mesh-rpc `srcexpr` names its peer by an expression, evaluated as
        // script-engine text. The `namelist` of a child session is judged with
        // its `<param>`s ([`Self::child_arguments`]), and the `namelist` and
        // `srcexpr` of a host-run invoke with the request it hands the host,
        // below.
        let srcexpr = match invoke {
            Invoke::MeshRpc(info) => match &info.target {
                crate::model::MeshRpcTarget::SrcExpr { srcexpr } => srcexpr.as_str(),
                _ => "",
            },
            _ => "",
        };
        // A host-run invoke's `<content expr>` is evaluated when it starts, as
        // its `srcexpr` is.
        let contentexpr = match invoke {
            Invoke::Unsupported(info) => info.contentexpr.as_str(),
            _ => "",
        };
        // An `idlocation` stores the id the build already wrote for the invoke —
        // its `id`, or `<state>.platform_N` — and this model reads no
        // `_event.invokeid`, only `_event.data`, so nothing could compare what it
        // stored. A document that must name its invocation writes `id`, which
        // `done.invoke.<id>` and `error.invoke.<id>` already match.
        if !base.idlocation.is_empty() {
            return Err(self.rule_at(
                format!("idlocation=\"{}\"", base.idlocation),
                "an <invoke idlocation> stores the id the build already wrote for the \
                 invocation, and this model reads no `_event.invokeid` to compare it with: \
                 write `id` where the document must name its invocation",
                line,
                col,
                state,
                &base.idlocation,
            ));
        }
        for (attr, value) in [("srcexpr", srcexpr), ("contentexpr", contentexpr)] {
            if !value.is_empty() {
                return Err(self.untyped_at(
                    format!("{attr}=\"{value}\""),
                    line,
                    col,
                    state,
                    value,
                ));
            }
        }
        // A host-run invoke's `<param>` is part of the request the host
        // receives, and is judged below.
        if let Invoke::Scxml(info) = invoke {
            self.child_arguments(ctx, info, state)?;
            // §scxml-6.5: a `<finalize>` runs in the invoking machine before an
            // event from the child is processed. The model keeps its body as
            // one script text, so no type rule reaches it, and the generated
            // code hands that text to a script engine this model never builds
            // (the Rust body is an empty block; Kotlin finds no engine). A body
            // written here would be accepted and never run. The model also
            // synthesizes a body for an empty `<finalize>` from `namelist` and
            // `<param>`, so a `<finalize/>` beside either reaches this refusal
            // as well. The model records the `<invoke>`, not the `<finalize>`,
            // so that is where the refusal sits.
            if !info.finalize_content.trim().is_empty() {
                return Err(self.rule_at(
                    format!("<finalize> of <invoke id=\"{}\">", base.invoke_id),
                    "a machine of this data model runs no <finalize>: its body is script \
                     text for an engine this model does not build, so it would be accepted \
                     and never run; take what the child sent in a transition of the \
                     invoking state instead",
                    line,
                    col,
                    state,
                    "",
                ));
            }
        }
        // A host-run invoke's `<param>`s are the request the host receives, and
        // are lowered to native code like a `<send>`'s, as are the names of its
        // `namelist`, which are the `<param name="x" expr="x"/>`s it abbreviates
        // ([`crate::model::UnsupportedInvokeInfo::fold_namelist_into_params`]).
        // One typed by `sce:request` holds the host to the whole record its
        // schema names and checks each value against its field, which the
        // lowering does not yet do — so such a request may carry literals and
        // nothing computed, rather than a `<param>` the host was promised and
        // is not given. It takes no `namelist` at all: the parser refuses one
        // beside a typed request, under every data model.
        if let Invoke::Unsupported(info) = invoke {
            let element = format!("<invoke id=\"{}\">", base.invoke_id);
            if !info.request_schema.is_empty() {
                if let Some(param) = base.params.iter().find(|p| {
                    !p.is_static_literal && !(p.expr.trim().is_empty() && p.location.is_empty())
                }) {
                    let at = param.source_location.as_ref();
                    return Err(self.rule_at(
                        format!("<param name=\"{}\"> of {element}", param.name),
                        "a request typed by sce:request is held to the record its schema \
                         names, and a value computed from this data model's variables is not \
                         yet checked against its field: write it as a literal",
                        at.and_then(|l| l.line).or(line),
                        at.and_then(|l| l.col).or(col),
                        state,
                        &param.expr,
                    ));
                }
            }
            // The `src` the host is handed is a string computed from the
            // machine's fields when the invocation starts. The element names its
            // source one way, written or computed, so a `src` beside it is
            // refused.
            if !info.srcexpr.is_empty() {
                let spelling = info.srcexpr_spelling.as_ref();
                if !info.src.trim().is_empty() {
                    return Err(self.rule_at(
                        format!("srcexpr=\"{}\"", info.srcexpr),
                        "an <invoke> names its source as `src` or as `srcexpr`, and never as both",
                        spelling.map(|s| s.row()).or(line),
                        spelling.map(|s| s.col()).or(col),
                        state,
                        &info.srcexpr,
                    ));
                }
                self.expr(
                    ctx,
                    &info.srcexpr,
                    spelling,
                    Expected::Slot(InferredType::Str),
                )?;
            }
            for param in &base.params {
                self.wire_param(ctx, &WireParam::of_param(param), &element, state)?;
            }
            for name in info.namelist.split_whitespace() {
                let pair = WireParam::of_namelist_name(name, None, at);
                self.wire_param(ctx, &pair, &element, state)?;
            }
            return Ok(());
        }
        for param in &base.params {
            if !param.expr.trim().is_empty() {
                self.expr(
                    ctx,
                    &param.expr,
                    param.expr_spelling.as_ref(),
                    Expected::Hint(InferredType::Unknown),
                )?;
            }
        }
        Ok(())
    }
}

/// The value of one of [`UNTYPED_ACTION_ATTRIBUTES`] on `action`.
fn action_attribute<'a>(action: &'a Action, attr: &str) -> &'a str {
    match attr {
        "targetexpr" => &action.targetexpr,
        "typeexpr" => &action.typeexpr,
        // An attribute listed in [`UNTYPED_ACTION_ATTRIBUTES`] and read nowhere
        // here would be refused never, which is the worst way to fail.
        other => unreachable!("`{other}` is read by no arm of action_attribute"),
    }
}
