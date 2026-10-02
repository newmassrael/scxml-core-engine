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
use crate::model::{Action, Datamodel, Invoke, SCXMLModel, Variable};
use crate::scxml_semantic::ScxmlSemanticError;

/// The attributes of executable content that carry an expression this model
/// does not type, with the element each belongs to. Refused where written.
const UNTYPED_ACTION_ATTRIBUTES: &[(&str, &str)] = &[
    ("send", "eventexpr"),
    ("send", "targetexpr"),
    ("send", "delayexpr"),
    ("send", "typeexpr"),
    ("send", "idlocation"),
    ("send", "namelist"),
    ("cancel", "sendidexpr"),
];

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
        enum_vars: enum_variables(&scope),
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
            let paths = scope.paths(model.imported_event_schemas.get(&transition.event));
            let ctx = judge.ctx(&paths);
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
        for invoke in &state.invokes {
            judge.invoke(&plain, invoke, &state.id)?;
        }
        if let Some(done) = &state.donedata {
            for param in &done.params {
                if let Some(expr) = &param.expr {
                    judge.expr(
                        &plain,
                        expr,
                        param.expr_spelling.as_ref(),
                        Expected::Hint(InferredType::Unknown),
                    )?;
                }
            }
            if let crate::model::DoneDataContent::Expression(expr) = &done.content {
                return Err(judge.untyped(
                    format!("<content expr=\"{expr}\">"),
                    done.content_spelling.as_ref(),
                    &state.id,
                    expr,
                ));
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

/// The enum each variable declared `enum:<alias>` holds, by the variable's id
/// — what [`crate::forge::static_enum`] asks to tell an enum value from a
/// number that happens to share its inferred type.
fn enum_variables(scope: &StaticScope) -> std::collections::BTreeMap<String, String> {
    scope
        .variables
        .iter()
        .filter_map(|var| {
            let Some(crate::forge::model::SceType::Enum(reference)) = var
                .value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::scalar)
            else {
                return None;
            };
            Some((var.id.clone(), reference.alias.clone()))
        })
        .collect()
}

struct Judge<'a> {
    scope: &'a StaticScope,
    enums: &'a [StaticEnum],
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
        crate::forge::static_enum::value_enum(expr, ctx, &|name| self.enum_vars.get(name).cloned())
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
        for (field, init) in schema.fields.iter().zip(ordered) {
            self.expr(
                ctx,
                &init.expr,
                init.expr_spelling.as_ref(),
                Expected::Slot(InferredType::from_sce_type(&field.sce_type)),
            )?;
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

    /// A `<param>` of a `<send>` or of an `<invoke>` the host runs, judged as
    /// the value that crosses to the host (SCE Accepted Subset §2.15).
    ///
    /// Its expression is read from the machine's fields when the element runs,
    /// and lowered to native code that makes the value both the text the
    /// pair crosses as and the JSON value in the event's data. That holds for
    /// the values every backend spells alike
    /// ([`InferredType::wire_param_slot`]); any other is refused where it is
    /// written rather than dropped, or carried differently by two backends. A
    /// value read from the triggering event's payload is refused too: reading
    /// it needs the payload channel's guard around the whole element, which the
    /// lowering of a `<param>` does not yet put there.
    ///
    /// `element` names the element a refusal places the param in. A `location`
    /// is read as an expression naming a variable, which is what it is.
    fn wire_param(
        &self,
        ctx: &TypeCtx<'_>,
        param: &crate::model::Param,
        element: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        // A string literal is folded at build time and crosses as written.
        if param.is_static_literal {
            return Ok(());
        }
        let (written, spelling) = if param.expr.trim().is_empty() {
            (param.location.as_str(), param.location_spelling.as_ref())
        } else {
            (param.expr.as_str(), param.expr_spelling.as_ref())
        };
        if written.trim().is_empty() {
            return Ok(());
        }
        let ty = self.expr(
            ctx,
            written,
            spelling,
            Expected::Hint(InferredType::Unknown),
        )?;
        let at = param.source_location.as_ref();
        let (line, col) = (at.and_then(|l| l.line), at.and_then(|l| l.col));
        let construct = format!("<param name=\"{}\"> of {element}", param.name);
        if crate::forge::expr::references_event_data_lexically(written) {
            return Err(self.rule_at(
                construct,
                "a <param> of this data model is read from the machine's fields when its \
                 element runs; one that reads the triggering event's payload has no \
                 lowering yet, so copy the value into a variable first",
                line,
                col,
                state,
                written,
            ));
        }
        if ty.wire_param_slot().is_none() {
            return Err(self.rule_at(
                construct,
                "a <param> crosses to the host as text and as a JSON value, which every \
                 backend spells alike for a bool, a string, an integer of at most 32 bits \
                 and a real; a 64-bit integer (which a backend that reads numbers through \
                 a double would carry with its low bits wrong), bytes, a list, a record and \
                 an enum have no such spelling yet",
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
                if self.scope.variables.iter().any(|v| {
                    v.id == location
                        && v.value_type
                            .as_ref()
                            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
                            .is_some()
                }) {
                    return Err(Located::in_file(
                        ExpressionSite::new(&action.location, action.spellings.get("location"))
                            .place(
                                crate::forge::error::ExprError::UnsupportedConstruct {
                                    construct: format!(
                                        "an assignment to the whole record `{location}` \
                                         (a record is updated a field at a time)"
                                    ),
                                    observed: Some(location.to_string()),
                                }
                                .at(None),
                            ),
                        self.diag_label,
                    ));
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
                if let Some(alias) = self.enum_vars.get(location) {
                    self.expr_of_enum(ctx, &action.expr, action.spellings.get("expr"), alias)?;
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
                    self.wire_param(ctx, param, "<send>", state)?;
                }
                if !action.contentexpr.is_empty() {
                    return Err(self.untyped(
                        format!("<content expr=\"{}\">", action.contentexpr),
                        action.contentexpr_spelling.as_ref(),
                        state,
                        &action.contentexpr,
                    ));
                }
            }
            // SCE Accepted Subset §2.15: the value appended is judged
            // against the list's element, as a typed assignment is judged
            // against its variable — the rule E8 holds an algorithm's list to.
            "sce_append" => {
                let Some(list) = self.list_var(&action.location) else {
                    return Err(self.not_a_list(action, state));
                };
                let elem = list
                    .value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem)
                    // A machine list is of scalars: the parser refuses a list
                    // of records on a variable (`enforce_static_datamodel`).
                    .and_then(crate::forge::model::ListElemType::scalar)
                    .map_or(InferredType::Unknown, InferredType::from_sce_type);
                self.expr(
                    ctx,
                    &action.expr,
                    action.spellings.get("expr"),
                    Expected::Slot(elem),
                )?;
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
        // A machine list is of scalars: the parser refuses a list of records
        // on a variable (`enforce_static_datamodel`).
        let elem = list
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::list_elem)
            .and_then(crate::forge::model::ListElemType::scalar)
            .map_or(InferredType::Unknown, InferredType::from_sce_type);
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
        let mut inner: TypeCtx<'_> = ctx.clone();
        inner.insert_var(item, elem);
        if !index.is_empty() {
            inner.insert_var(
                index,
                InferredType::from_sce_type(&crate::forge::model::SceType::Uint32),
            );
        }
        self.actions(&inner, &action.actions, state)
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
        // A `namelist` reads datamodel variables by name at entry, and a
        // mesh-rpc `srcexpr` names its peer by an expression — both are
        // evaluated as script-engine text.
        let namelist = match invoke {
            Invoke::Scxml(info) => info.namelist.as_str(),
            Invoke::Unsupported(info) => info.namelist.as_str(),
            _ => "",
        };
        let srcexpr = match invoke {
            Invoke::MeshRpc(info) => match &info.target {
                crate::model::MeshRpcTarget::SrcExpr { srcexpr } => srcexpr.as_str(),
                _ => "",
            },
            Invoke::Unsupported(info) => info.srcexpr.as_str(),
            _ => "",
        };
        // A host-run invoke's `<content expr>` is evaluated when it starts, as
        // its `srcexpr` is.
        let contentexpr = match invoke {
            Invoke::Unsupported(info) => info.contentexpr.as_str(),
            _ => "",
        };
        for (attr, value) in [
            ("idlocation", base.idlocation.as_str()),
            ("namelist", namelist),
            ("srcexpr", srcexpr),
            ("contentexpr", contentexpr),
        ] {
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
        // §scxml-6.4.1: an `<invoke type="scxml">` hands each `<param>` to the
        // child session it starts, to be bound as a variable of the child's
        // datamodel. Under this model that datamodel is the child's own native
        // fields, which only the child's code sets, and no generated code
        // delivers a parent's `<param>` to one — so the value would be typed
        // here, accepted, and never arrive. Refused where it is written until
        // a value can land in a field. A host-run invoke's `<param>` is part of
        // the request the host receives, and is judged below.
        if let Invoke::Scxml(info) = invoke {
            if let Some(param) = base.params.first() {
                let at = param.source_location.as_ref();
                let written = if param.expr.trim().is_empty() {
                    &param.location
                } else {
                    &param.expr
                };
                return Err(self.rule_at(
                    format!(
                        "<param name=\"{}\"> of <invoke id=\"{}\">",
                        param.name, base.invoke_id
                    ),
                    "a child session of this data model has native fields of its own and \
                     is handed no <param>: the value would be dropped, so give the child \
                     the value in an event it takes",
                    at.and_then(|l| l.line).or(line),
                    at.and_then(|l| l.col).or(col),
                    state,
                    written,
                ));
            }
            // §scxml-6.5: a `<finalize>` runs in the invoking machine before an
            // event from the child is processed. The model keeps its body as
            // one script text, so no type rule reaches it, and the generated
            // code hands that text to a script engine this model never builds
            // (the Rust body is an empty block; Kotlin finds no engine). A body
            // written here would be accepted and never run. The model also
            // synthesizes a body for an empty `<finalize>` from `namelist` and
            // `<param>`, both refused before this point, so what is left is a
            // body the author wrote. The model records the `<invoke>`, not the
            // `<finalize>`, so that is where the refusal sits.
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
        // are lowered to native code like a `<send>`'s. One typed by
        // `sce:request` holds the host to the whole record its schema names
        // and checks each value against its field, which the lowering does not
        // yet do — so such a request may carry literals and nothing computed,
        // rather than a `<param>` the host was promised and is not given.
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
            for param in &base.params {
                self.wire_param(ctx, param, &element, state)?;
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
        "eventexpr" => &action.eventexpr,
        "targetexpr" => &action.targetexpr,
        "delayexpr" => &action.delayexpr,
        "typeexpr" => &action.typeexpr,
        "idlocation" => &action.idlocation,
        "namelist" => &action.namelist,
        "sendidexpr" => &action.sendidexpr,
        _ => "",
    }
}
