// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::narrowing;
use crate::forge::type_ctx::{StaticEnum, StaticScope};
use crate::forge::types::{InferredType, TypeCtx};
use crate::model::{Action, Datamodel, DoneDataParam, Invoke, Param, SCXMLModel, Variable};
use crate::scxml_semantic::ScxmlSemanticError;

/// `ctx` with `facts` the subjects proven present where an expression of it is
/// read (docs/adr/0055): `ctx` itself when they are the ones it holds, so a
/// document that tests no optional value judges in the context it was given.
/// The lowering reads expressions in the same contexts
/// ([`crate::forge::static_lowering`]).
pub(crate) fn proving<'r, 'c>(
    ctx: &'r TypeCtx<'c>,
    facts: &narrowing::Facts,
) -> std::borrow::Cow<'r, TypeCtx<'c>> {
    if ctx.proven == *facts {
        std::borrow::Cow::Borrowed(ctx)
    } else {
        std::borrow::Cow::Owned(ctx.proving(facts.clone()))
    }
}

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

    /// The `<content expr>` of a `<send>` that names a single value: the one
    /// value the event carries, read as a `<param>` of no name is, so it is held
    /// to the rule a param's value is. The attribute it is written in places a
    /// refusal.
    pub(crate) fn of_content(
        written: &'a str,
        spelling: Option<&'a crate::attribute_spelling::AttributeSpelling>,
    ) -> Self {
        Self {
            name: "",
            is_static_literal: false,
            written,
            spelling,
            source_location: None,
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

/// A `<content expr>` as the judge reads it: where it is written, and what may
/// stand beside it or be named by it there.
#[derive(Clone, Copy)]
struct ContentExpr<'a> {
    /// The element it is written in (`<send>`, `<donedata>`), which a refusal
    /// names.
    element: &'a str,
    /// The expression, as written.
    written: &'a str,
    spelling: Option<&'a crate::attribute_spelling::AttributeSpelling>,
    /// `<param>`s or a `namelist` stand beside it, which an element that carries
    /// its data one way refuses.
    beside: bool,
    /// How the element carries the value ([`Carrier`]).
    carrier: Carrier,
}

/// The most bytes of the id a machine generates for a `<send idlocation>`:
/// `_auto_send_` and the twenty digits of the largest `u64` count. The number
/// every runtime's `AUTO_SEND_ID_MAX_LEN` states
/// (`backends/rust/runtime/src/helpers/unique_id_generator.rs`), which a test
/// holds this one to.
pub const AUTO_SEND_ID_MAX_BYTES: u32 = 31;

/// The standard enum an `<sce:put>` writes its outcome as (docs/adr/0108, D4),
/// by the identity every reader of a document names it with
/// ([`crate::forge::stdlib::identity`]): `inserted`, `replaced` or `full`.
pub const PUT_OUTCOME_DOCUMENT: &str = "sce:std/collection/put_outcome.scxml";

/// The standard enum an `<sce:remove>` writes its outcome as (docs/adr/0108,
/// D4): `removed` or `absent`.
pub const REMOVE_OUTCOME_DOCUMENT: &str = "sce:std/collection/remove_outcome.scxml";

/// Why an `idlocation` that names no string variable the document declares is
/// refused, a `<send>`'s or an `<invoke>`'s.
const IDLOCATION_NAMES_A_STRING_VARIABLE: &str =
    "an idlocation names a string variable the document declares: the id is a string, and \
     the place it is written to is one variable, not a record's field, a list's element or \
     what a <foreach> binds";

/// The most bytes of the id a start of the child session the `<invoke>`
/// `invoke_id` of state `state` starts is started under: `<state>.<start>.<id>`
/// (docs/adr/0045), the start a `u64` count of up to twenty digits.
pub fn invoke_id_max_bytes(state: &str, invoke_id: &str) -> u32 {
    let start_digits = u64::MAX.to_string().len();
    u32::try_from(state.len() + 1 + start_digits + 1 + invoke_id.len()).unwrap_or(u32::MAX)
}

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
    // docs/adr/0114, D1: a value held to the journal's facts is written only
    // where a fact is applied.
    crate::forge::journal::refuse_journal_values_written_elsewhere(model, diag_label)?;
    let held_enums = model.held_enums();
    // The schemas read one entry per field: every use of one that holds a
    // composition was refused where the document was parsed
    // ([`crate::forge::event_schema_check::check_composed_uses`]), but a
    // variable of the record itself, which is judged from the imports
    // ([`Judge::composed_record`]).
    let records = model.entry_records();
    let event_schemas = model.entry_event_schemas();
    let received = model.received_events();
    let judge = Judge {
        scope: &scope,
        enums,
        held_enums: &held_enums,
        schemas: &records,
        loop_records: Default::default(),
        loop_names: Default::default(),
        enum_vars: enum_variables(&scope, &model.imported_records),
        loop_enum_paths: Default::default(),
        payload_enum_paths: Default::default(),
        payload_schema: Default::default(),
        payload_declared: Default::default(),
        diag_label,
        read: Default::default(),
        whole_state_call_ok: Default::default(),
        in_place_call_ok: Default::default(),
        walked_maps: Default::default(),
        walk_state: Default::default(),
        receivers: Receivers::of(model, &held_enums, &event_schemas),
        lift_schemas: &model.imported_records,
        type_scope: crate::forge::value_path::statechart_scope(model),
        text_templates: &model.imported_text_templates,
        enum_documents: &model.enum_documents.by_alias,
    };
    // Every record variable's fields, readable everywhere; a transition adds
    // its payload to these.
    let record_paths = scope.paths(None);
    let plain = judge.ctx(&record_paths);

    for var in &scope.variables {
        // A map or a set (docs/adr/0108) starts empty and takes no value where
        // it is declared: its type is judged, and its reads, statements and
        // loop where each is written.
        if let Some(ty) = var.value_type.as_ref() {
            if judge.map_variable(var, ty, &records)? && !var.from_deployment {
                continue;
            }
        }
        // A value the deployment supplies has no initial value of the
        // document's to judge (docs/adr/0109, D12): its type is judged as the
        // one its reader reads, which a map or a set, its key judged above,
        // has none of.
        if var.from_deployment {
            judge.deployment_value(var)?;
            continue;
        }
        if let Some(alias) = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            judge.record(&plain, var, alias, &records)?;
            continue;
        }
        if let Some(alias) = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            judge.optional_record(&plain, var, alias, &records)?;
            continue;
        }
        // A union is built at its declaration, or starts absent (docs/adr/0092,
        // amendment of 2026-11-07).
        match var.value_type.as_ref() {
            Some(crate::forge::model::AlgorithmValueType::Union { alias }) => {
                judge.union_variable(&plain, var, alias, &records)?;
                continue;
            }
            Some(ty)
                if matches!(
                    ty.optional_elem(),
                    Some(crate::forge::model::AlgorithmValueType::Union { .. })
                ) =>
            {
                judge.optional_union_variable(&plain, var)?;
                continue;
            }
            _ => {}
        }
        if let Some(alias) = judge.enum_vars.get(&var.id) {
            judge.enum_variable(
                &plain,
                var,
                alias,
                &model.imported_enums,
                EnumHolding::Present,
            )?;
            continue;
        }
        if let Some(alias) = judge.optional_enum_of(&plain, &var.id) {
            judge.enum_variable(
                &plain,
                var,
                &alias,
                &model.imported_enums,
                EnumHolding::Optional,
            )?;
            continue;
        }
        if let Some(alias) = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::list_elem)
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            // A list starts empty, so it names no variant of an enum field of its
            // records, and holds one the document does not import (docs/adr/0054);
            // the enum it holds is closed all the same.
            if let Some(schema) = records.get(alias) {
                judge.schema_enums_closed(var, alias, schema)?;
                judge.schema_strings_bounded(var, alias, schema)?;
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
        *judge.walk_state.borrow_mut() = state.id.clone();
        for block in state.on_entry_blocks.iter().chain(&state.on_exit_blocks) {
            judge.actions(&plain, block, &state.id)?;
        }
        judge.actions(&plain, &state.initial_transition_actions, &state.id)?;
        judge.actions(&plain, &state.initial_history_default_actions, &state.id)?;
        for transition in &state.transitions {
            // The payload is the triggering event's, so each transition
            // judges against its own: the event it names, or the message a
            // receive branch of a test component takes.
            let trigger = transition.payload_event(&received);
            let schema = event_schemas.get(trigger);
            // `_event.data` is a record of the schema whole, whatever its
            // fields hold, and is read at any depth (docs/adr/0086, section 6).
            let whole = model.imported_event_schemas.get(trigger);
            let paths = scope.paths(whole.map(|schema| schema.name.as_str()));
            let ctx = judge.ctx(&paths);
            // An enum field of the payload is a value of the enum the schema
            // names, as a record's field is, and the payload taken whole is a
            // record of its schema.
            judge.open_payload(schema, whole);
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
            // A test of a field of the payload in the `cond` proves it in the
            // transition's content, which reads the same payload; a test of a
            // variable proves nothing there (docs/adr/0055, section 3).
            let guarded = if transition.is_cpp_condition || transition.is_kt_condition {
                narrowing::Facts::none()
            } else {
                narrowing::proves_written(&transition.cond, true).of_payload()
            };
            judge.actions(&proving(&ctx, &guarded), &transition.actions, &state.id)?;
        }
        judge.open_payload(None, None);
        for invoke in &state.invokes {
            judge.invoke(&plain, invoke, &state.id)?;
        }
        if let Some(done) = &state.donedata {
            for param in &done.params {
                judge.wire_param(
                    &plain,
                    &WireParam::of_done_param(param),
                    "<donedata>",
                    Carrier::Json,
                    &state.id,
                )?;
            }
            // The record it names crosses as the pairs of its fields, and any
            // other expression as the one value it computes, read when the state
            // is entered, where no event's payload is in scope.
            if let crate::model::DoneDataContent::Expression(expr) = &done.content {
                judge.content_record(
                    &plain,
                    &ContentExpr {
                        element: "<donedata>",
                        written: expr,
                        spelling: done.content_spelling.as_ref(),
                        beside: !done.params.is_empty(),
                        carrier: Carrier::Json,
                    },
                    &state.id,
                )?;
            }
        }
    }
    // An imported algorithm — or a transform, condition, lookup, interpolation or
    // validator (docs/adr/0048) — is imported to be called. One nothing calls is
    // refused rather than dropped in silence — a statechart keeps no import
    // for later, and a stale one would outlive the call it was for.
    //
    // A `<sce:convert>` of a declared migration calls the algorithm it names, on
    // the value a state an earlier version saved holds (docs/adr/0109, D11): the
    // document's call of it, though no expression spells it.
    judge.read.borrow_mut().extend(
        model
            .migrations
            .iter()
            .flat_map(|migration| &migration.changes)
            .filter_map(|change| match &change.op {
                crate::forge::saved_migration::MigrationOp::Convert { algorithm, .. } => {
                    Some(algorithm.clone())
                }
                _ => None,
            }),
    );
    let read = judge.read.borrow();
    if let Some(unused) = scope.callees.iter().find(|c| !read.contains(&c.alias)) {
        let kind = unused.kind.forge_kind();
        return Err(Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct: format!("<sce:import kind=\"{kind}\" as=\"{}\">", unused.alias),
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: format!(
                    "an imported {kind} is called by the document, and nothing calls `{}`",
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

/// The refusal of a map or set variable read as a value
/// ([`StaticScope::map_read_as_value`], docs/adr/0108, D2 and D3) — one wording
/// wherever it is found.
pub(crate) fn map_read_refusal(map: &str) -> crate::forge::expr::Refusal {
    crate::forge::error::ExprError::UnsupportedConstruct {
        construct: format!(
            "reading the map or set `{map}` as a value (a map is written by <sce:put>, \
             <sce:remove> and <sce:clear>, read by len({map}), has({map}, k) and {map}[k], \
             walked by a <foreach>, and taken whole only by an <assign> to a map variable \
             whose type it fits)"
        ),
        observed: Some(map.to_string()),
    }
    .at(None)
}

/// `days[0]` or `days[len(days) - 1]`: the name of a list and the text of the index
/// an element of it is read at, whole — nothing follows the bracket that closes the
/// index, so `days[0].frame` is a field's read and not this. `None` for any other
/// spelling.
pub(crate) fn indexed_element(written: &str) -> Option<(&str, &str)> {
    let text = written.trim();
    let open = text.find('[')?;
    let list = text[..open].trim();
    let is_name = !list.is_empty()
        && !list.starts_with(|c: char| c.is_ascii_digit())
        && list.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !is_name {
        return None;
    }
    let mut depth = 0usize;
    for (at, c) in text[open..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    // The bracket opened first closes last, or the text goes on past it.
                    let close = open + at;
                    let index = text[open + 1..close].trim();
                    return (close + 1 == text.len() && !index.is_empty()).then_some((list, index));
                }
            }
            _ => {}
        }
    }
    None
}

/// The place `written` reads when it is a field of a record, at any depth, that
/// holds a whole record or a whole optional record (`_event.data.payload`,
/// `_event.data.kept`), with the path it is read at
/// ([`crate::forge::expr::written_place`]): every place it is read through on the
/// way is a record (the payload, a record variable, a field of one), so no element
/// and no present value is read on the way, and the place holds `record:R` or
/// `optional<record:R>`. Taken whole into a record variable or an optional record
/// variable as a list a place holds is taken into a list variable (docs/adr/0086,
/// section 6.1). `None` for anything else, `_event.data` itself among them, which
/// is a record named whole.
pub(crate) fn record_field_place(
    ctx: &TypeCtx<'_>,
    written: &str,
) -> Option<(String, crate::forge::value_path::Place)> {
    use crate::forge::value_type::ValueType;
    let path = crate::forge::expr::written_place(written)?;
    if path.contains(crate::forge::value_path::ELEMENT) {
        return None;
    }
    // Each place on the way that holds a value is a record, from the first one
    // (a variable, or `_event.data`, whose own name holds a dot) to the holder
    // of the field.
    let mut through_records = false;
    for (at, _) in path.match_indices('.') {
        match ctx.value_of(&path[..at]) {
            Some(held) if matches!(held.ty, ValueType::Record { .. }) => through_records = true,
            Some(_) => return None,
            None if through_records => return None,
            None => {}
        }
    }
    if !through_records {
        return None;
    }
    let place = ctx.value_of(&path)?;
    let holds_a_record = match &place.ty {
        ValueType::Record { .. } => true,
        ValueType::Optional { elem } => matches!(**elem, ValueType::Record { .. }),
        _ => false,
    };
    holds_a_record.then_some((path, place))
}

/// Each place `action` writes itself, as the element and the attribute that
/// names it, and the place as written: what an `<assign>`, an `<sce:append>`,
/// an `<sce:clear>`, an `<sce:render>` and an `<sce:lift>` replace, the id a
/// `<send>` writes, the item and index a `<foreach>` binds, and the target and
/// outcome of a map's `<sce:put>` and `<sce:remove>`. The one list the rules of
/// what a statement may write read: a value the deployment supplies
/// (docs/adr/0109, D12) and a value held to the journal's facts
/// (docs/adr/0114, D1).
pub(crate) fn places_written(action: &Action) -> Vec<(&'static str, &'static str, &str)> {
    match action.action_type.as_str() {
        "assign" => vec![("<assign>", "location", action.location.as_str())],
        "sce_append" => vec![("<sce:append>", "target", action.location.as_str())],
        "sce_clear" => vec![("<sce:clear>", "target", action.location.as_str())],
        "sce_render" => vec![("<sce:render>", "location", action.location.as_str())],
        "sce_lift" => vec![
            ("<sce:lift>", "location", action.location.as_str()),
            ("<sce:lift>", "refusal", action.lift_refusal.as_str()),
        ],
        "send" => vec![("<send>", "idlocation", action.idlocation.as_str())],
        "foreach" => vec![
            ("<foreach>", "item", action.item.as_str()),
            ("<foreach>", "index", action.index.as_str()),
        ],
        // docs/adr/0108, D3 and D4: a map write, into its target, and the
        // outcome it writes.
        "sce_put" => vec![
            ("<sce:put>", "target", action.location.as_str()),
            ("<sce:put>", "outcome", action.outcome.as_str()),
        ],
        "sce_remove" => vec![
            ("<sce:remove>", "target", action.location.as_str()),
            ("<sce:remove>", "outcome", action.outcome.as_str()),
        ],
        _ => Vec::new(),
    }
}

/// The conditions of `action`'s `<if>` chain in order — its own, then each
/// `<elseif>`'s — as [`narrowing::Chain`] reads them; a native condition, which
/// proves nothing, as empty text. Empty for any other action. The lowering reads
/// the same conditions ([`crate::forge::static_lowering`]).
pub(crate) fn chain_conds(action: &Action) -> Vec<&str> {
    if action.action_type != "if" {
        return Vec::new();
    }
    let own = if action.is_cpp_condition || action.is_kt_condition {
        ""
    } else {
        action.cond.as_str()
    };
    std::iter::once(own)
        .chain(
            action
                .nested_blocks()
                .into_iter()
                .filter(|block| block.role == crate::model::BlockRole::ElseIf)
                .map(|block| {
                    if block.cond_is_native {
                        ""
                    } else {
                        block.cond.unwrap_or_default()
                    }
                }),
        )
        .collect()
}

/// Whether `written` is the literal `null`, an absent optional value
/// (docs/adr/0055), however it is parenthesised: what the lowering gives a
/// place of a known optional type whole as its absent value
/// ([`crate::forge::static_lowering::StaticTarget::optional_none`]).
pub(crate) fn is_null_literal(written: &str) -> bool {
    crate::forge::expr::parse_to_ast(written.trim())
        .is_ok_and(|ast| matches!(ast.kind, crate::forge::expr::ExprKind::NullLit))
}

/// Whether `name` is one of the identifiers `text` is made of — `last` is in
/// `last.n + 1` and `len(last)`, and is not in `lasting`.
fn names_identifier(text: &str, name: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|word| word == name)
}

/// A variable's declared type; `Unknown` for an enum, whose width the
/// inference layer declines to claim ([`InferredType::from_sce_type`]), and an
/// optional value of its element for an optional variable (docs/adr/0055).
fn variable_type(var: &Variable) -> InferredType {
    let Some(value_type) = var.value_type.as_ref() else {
        return InferredType::Unknown;
    };
    if let Some(elem) = value_type.optional_elem() {
        return InferredType::Optional(crate::forge::types::OptionalElem::of_value(elem));
    }
    value_type
        .scalar()
        .map_or(InferredType::Unknown, InferredType::from_sce_type)
}

/// Whether every backend writes a value of `ty` as the text a `<log>` shows:
/// a bool, a string, an integer, a real, a value of an enum (`enum_value`,
/// which the lattice types `Unknown`), and an optional value of one of those,
/// shown as `null` where it is absent. A byte string and a record have no
/// text a C or C++ machine writes — the C++ log writes a value through its
/// `operator<<` and the C one through a `printf` conversion — and the other
/// backends print each its own way, so a value of either, a value of no
/// type, and `null` itself are not among them. A list is refused before its
/// type is asked ([`list_read_refusal`]).
fn logged_alike(ty: InferredType, enum_value: bool) -> bool {
    use crate::forge::types::OptionalElem;
    match ty.strip_quantity() {
        InferredType::Bool
        | InferredType::Str
        | InferredType::Int { .. }
        | InferredType::Float { .. }
        | InferredType::UntypedInt
        | InferredType::UntypedFloat => true,
        InferredType::Optional(elem) => !matches!(
            elem,
            OptionalElem::Bytes | OptionalElem::Record | OptionalElem::List(_)
        ),
        InferredType::Unknown => enum_value,
        InferredType::Bytes
        | InferredType::BytesBuffer
        | InferredType::List(_)
        | InferredType::ListBuffer(_)
        | InferredType::Record(_)
        | InferredType::Null
        | InferredType::Quantity { .. } => false,
    }
}

/// A `<param>` of a host-run invoke that is a record the invoke names the
/// schema of, rather than a text the request's field types.
struct RecordParam<'a> {
    /// The alias of the record's schema.
    alias: &'a str,
    /// The attribute that names it.
    attribute: &'static str,
    /// What the param is, as a refusal opens.
    what: &'static str,
    /// The form the record crosses to the host in.
    form: &'static str,
}

/// The record `param` is, if it is one: the `entry` of an audit append that
/// names its entry's schema (docs/adr/0109, D14), which crosses as its JSON
/// payload, or the `payload` of a signing or the `plaintext` of a seal that
/// names its record (docs/adr/0109, D9), which crosses as its deterministic
/// CBOR.
fn record_param_of<'a>(
    info: &'a crate::model::UnsupportedInvokeInfo,
    param: &Param,
) -> Option<RecordParam<'a>> {
    if info.invoke_type == crate::forge::audit_entry::AUDIT_APPEND
        && !info.entry_schema.is_empty()
        && param.name == crate::forge::audit_entry::ENTRY_FIELD
    {
        return Some(RecordParam {
            alias: &info.entry_schema,
            attribute: "sce:entry",
            what: "the entry of an audit append",
            form: "JSON payload",
        });
    }
    let op = crate::forge::payload_record::operation(&info.invoke_type)?;
    (op.side == crate::forge::payload_record::Side::Request
        && !info.payload_schema.is_empty()
        && param.name == op.field)
        .then_some(RecordParam {
            alias: &info.payload_schema,
            attribute: "sce:payload",
            what: if op.field == "plaintext" {
                "the plaintext of a seal"
            } else {
                "the payload of a signing"
            },
            form: "deterministic CBOR (docs/adr/0101, section 2)",
        })
}

/// Whether a value of `ty` is a whole union, present or optional
/// (docs/adr/0092): what no carrier of a statechart's values takes yet.
pub(crate) fn holds_whole_union(ty: &crate::forge::value_type::ValueType) -> bool {
    use crate::forge::value_type::ValueType;
    match ty {
        ValueType::Union { .. } => true,
        ValueType::Optional { elem } => matches!(**elem, ValueType::Union { .. }),
        _ => false,
    }
}

/// The top-level `<data>` of a hybrid `<invoke>`'s `candidate` named `name`,
/// when the candidate is a `sce-static` document this build read and declares
/// one.
pub(crate) fn candidate_variable<'a>(
    candidate: &'a crate::model::InvokeCandidate,
    name: &str,
) -> Option<&'a Variable> {
    candidate
        .child_static_variables
        .as_deref()?
        .iter()
        .find(|v| v.id == name)
}

/// The expression an argument of an `<invoke>` hands its child, and where it is
/// written: its `expr`, or, failing that, the `location` read as one.
pub(crate) fn argument_written(
    param: &crate::model::Param,
) -> (&str, Option<&crate::attribute_spelling::AttributeSpelling>) {
    if param.expr.trim().is_empty() {
        (param.location.as_str(), param.location_spelling.as_ref())
    } else {
        (param.expr.as_str(), param.expr_spelling.as_ref())
    }
}

/// The type a value handed to the child's variable `var` is held to
/// (§scxml-6.4.1), or `None` for a variable no value can be handed to — a
/// list, a record, an enum or bytes, and an optional value of an enum, bytes or
/// a record. A variable of `optional<T>` over a bool, a number or a text is
/// handed an `optional<T>` (docs/adr/0055, step 10): a value of `T` is present,
/// `null` is absent, and an optional value of the invoking machine is handed as
/// it is. The one rule the judge and the lowering of a child's arguments share.
pub(crate) fn seed_slot(var: &Variable) -> Option<InferredType> {
    let value_type = var.value_type.as_ref()?;
    let optional = value_type.optional_elem();
    let held = match optional {
        Some(elem) => elem.scalar()?,
        None => value_type.scalar()?,
    };
    if matches!(
        held,
        crate::forge::model::SceType::Bytes | crate::forge::model::SceType::Enum(_)
    ) {
        return None;
    }
    Some(match optional {
        Some(elem) => InferredType::Optional(crate::forge::types::OptionalElem::of_value(elem)),
        None => InferredType::from_sce_type(held),
    })
}

/// The refusal of a `<param>` handed to the child's variable `var`, which
/// [`seed_slot`] answers no type for, `whose` naming the child as a refusal does
/// (`the child's`).
fn unseedable_variable(var: &Variable, whose: &str) -> String {
    let declared = var
        .value_type
        .as_ref()
        .map(crate::forge::model::AlgorithmValueType::as_attr)
        .unwrap_or_default();
    format!(
        "{whose} `{}` is {declared}, and a value is handed only to a bool, a string, an \
         integer or a real, or to an optional value of one of them: a list, a record, a \
         union, an enum, bytes and an optional value of an enum, bytes, a record or a \
         union are not seeded",
        var.id
    )
}

/// Whether the child's variable `var` starts absent, `expr="null"`: the state a
/// variable of `optional<T>` that is not handed a value is in
/// (docs/adr/0055, section 2).
fn starts_absent(var: &Variable) -> bool {
    is_null_literal(&var.expr)
}

/// The rule that refuses a `<param>` whose value may be absent (of type `ty`),
/// handed to the child's optional variable `var` when `var` starts at a value
/// (docs/adr/0055, step 10); `None` for any other variable or value. What a
/// child is started with spells a variable not handed as absent, so an absent
/// value stands for no value handed, and the child keeps the one its `<data>`
/// gave it: the same state as "handed absent" only where the variable starts
/// absent. Where it starts at a value, the value handed must be one that is
/// never absent, and the refusal names the two ways to write it.
fn absent_handed_to_a_present_start(
    var: &Variable,
    ty: InferredType,
    whose: &str,
) -> Option<String> {
    let value_type = var.value_type.as_ref()?;
    value_type.optional_elem()?;
    if starts_absent(var) || !(ty.is_optional() || ty == InferredType::Null) {
        return None;
    }
    Some(format!(
        "{whose} `{}` is {} and starts at `{}`, and a <param> hands a name not handed as \
         absent: a value that may be absent would leave the child that start, which is no \
         value handed. Start it at `null`, or hand it a value that is never absent (a value \
         of the element, or `<value> ?? <default>`)",
        var.id,
        value_type.as_attr(),
        var.expr.trim()
    ))
}

/// The enum each variable declared `enum:<alias>` holds, by the variable's id,
/// and each enum field of a record variable, by its `<id>.<field>` path —
/// what [`crate::forge::static_enum`] asks to tell an enum value from a number
/// that happens to share its inferred type.
///
/// `records` are the imported records whole
/// ([`crate::model::SCXMLModel::imported_records`]), a record that holds a
/// list, a record or a union included: an enum field of one is a value of its
/// enum whatever its sibling fields hold ([`enum_field_aliases`]).
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
        // An optional enum variable's present value is a value of the enum
        // (docs/adr/0055): `status.value`, and not `status`, holds it.
        if let Some(crate::forge::model::SceType::Enum(reference)) = value_type
            .optional_elem()
            .and_then(crate::forge::model::AlgorithmValueType::scalar)
        {
            held.insert(
                crate::forge::static_enum::present_value_path(&var.id),
                reference.alias.clone(),
            );
        }
        if let Some(schema) = value_type.record_alias().and_then(|a| records.get(a)) {
            held.extend(enum_fields(&var.id, schema));
        }
        // The enum field of an optional record is read under its present value,
        // `<id>.value.<field>` (docs/adr/0055, step 9).
        if let Some(schema) = value_type
            .optional_elem()
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
            .and_then(|a| records.get(a))
        {
            held.extend(enum_fields(
                &crate::forge::static_enum::present_value_path(&var.id),
                schema,
            ));
        }
        // The enum field of the element a list of records is indexed at, held
        // under `<list>[].<field>` whatever the index is.
        if let Some(alias) = value_type
            .list_elem()
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
        {
            if let Some(schema) = records.get(alias) {
                held.extend(enum_field_aliases(schema).map(|(field, alias)| {
                    (
                        format!(
                            "{}{}{field}",
                            var.id,
                            crate::forge::type_ctx::INDEXED_ELEMENT
                        ),
                        alias.to_string(),
                    )
                }));
            }
        }
    }
    held
}

/// Each enum field of a record of `schema`, by the path of the field's value
/// below the record and the alias of the enum it holds: the field's id, and for
/// an optional enum field the path of its present value (`status.value`), since
/// a value of it may be absent and its present value is the value of the enum
/// (docs/adr/0055, [`crate::forge::static_enum::present_value_path`]).
///
/// `schema` is read whole, each field by its own type: an enum field is the
/// same value of its enum beside a field that holds a list, a record or a
/// union (docs/adr/0086, section 1) as beside fields of one entry each, so a
/// schema that holds a composition keeps its enum fields. The one place both
/// a record variable's fields ([`enum_variables`]) and the payload's
/// ([`Judge::open_payload`], and the lowering's) are answered from, with the
/// alias the statechart names each field's enum by
/// ([`crate::forge::event_schema_check::name_schema_enums`]).
fn enum_field_aliases(
    schema: &crate::forge::model::EventSchemaModel,
) -> impl Iterator<Item = (String, &str)> {
    use crate::forge::value_type::ValueType;
    /// The alias of the enum `ty` is, where it is one.
    fn enum_alias(ty: &crate::forge::value_type::ValueType) -> Option<&str> {
        match ty {
            crate::forge::value_type::ValueType::Scalar(crate::forge::model::SceType::Enum(
                reference,
            )) => Some(reference.alias.as_str()),
            _ => None,
        }
    }
    schema
        .fields
        .iter()
        .filter_map(|field| match &field.sce_type {
            ValueType::Optional { elem } => enum_alias(elem).map(|alias| {
                (
                    crate::forge::static_enum::present_value_path(&field.id),
                    alias,
                )
            }),
            ty => enum_alias(ty).map(|alias| (field.id.clone(), alias)),
        })
}

/// Each enum field of a record of `schema` held under the name `holder`, by
/// its `<holder>.<field>` path and the alias of the enum it holds.
pub(crate) fn enum_fields(
    holder: &str,
    schema: &crate::forge::model::EventSchemaModel,
) -> Vec<(String, String)> {
    enum_field_aliases(schema)
        .map(|(field, alias)| (format!("{holder}.{field}"), alias.to_string()))
        .collect()
}

/// The alias the statechart names the enum of the value at `path` by, where
/// `path` is a place the walk of the one value type reaches (docs/adr/0086,
/// section 6.2): an enum field of a record, of an element of a list or of a
/// union's case at any depth (`_event.data.lines[].color`), and a union's tag
/// (`when.kind`, `_event.data.shapes[].kind`, docs/adr/0092, section 6.1), each
/// a value of its enum wherever it is, compared with a variant of it and
/// assigned to a variable of it as a value of the enum is. The places a
/// variable's record of entries and a loop's record item hold are answered
/// before this by the paths [`enum_variables`] and the loop register. `None`
/// for a path no place starts, one that holds no value of an enum, or one
/// whose enum the statechart does not import.
fn place_enum(ctx: &TypeCtx<'_>, path: &str) -> Option<String> {
    let place = ctx.value_of(path)?;
    crate::forge::value_path::enum_alias(ctx.value_scope?, &place)
}

/// The names one statechart gives the records and enums it imports: two are
/// one type when the statechart names them by one alias, since each alias
/// names one document (docs/adr/0086, section 5.1).
struct StatechartNames;

impl crate::forge::value_type::TypeIdentity for StatechartNames {
    fn same_record(&self, from: &str, to: &str) -> bool {
        from == to
    }
    fn same_enum(&self, from: &str, to: &str) -> bool {
        from == to
    }
    fn same_union(&self, from: &str, to: &str) -> bool {
        from == to
    }
}

/// The names a value read through a record of another document gives the
/// records and enums it holds, beside the statechart's own (docs/adr/0086,
/// section 5.1): `from` is the scope the value's aliases are read in
/// ([`crate::forge::value_path::Place::scope`]), `to` the statechart's. Two
/// records are one type when their aliases resolve to one record document; two
/// enums when theirs resolve to enums of the same variants, numbers and
/// underlying type, or, where neither scope reads the enum, when the aliases
/// are one. A test template's positions are told apart by the same reading
/// (`crate::forge::test_template_check`).
pub(crate) struct ScopedNames<'s> {
    pub(crate) from: Option<&'s crate::forge::value_type::TypeScope>,
    pub(crate) to: Option<&'s crate::forge::value_type::TypeScope>,
}

impl crate::forge::value_type::TypeIdentity for ScopedNames<'_> {
    fn same_record(&self, from: &str, to: &str) -> bool {
        let identity = |scope: Option<&crate::forge::value_type::TypeScope>, alias: &str| {
            scope
                .and_then(|scope| scope.records.get(alias))
                .map(|record| record.identity.clone())
        };
        matches!(
            (identity(self.from, from), identity(self.to, to)),
            (Some(a), Some(b)) if a == b
        )
    }
    fn same_enum(&self, from: &str, to: &str) -> bool {
        let held = |scope: Option<&crate::forge::value_type::TypeScope>, alias: &str| {
            scope.and_then(|scope| scope.enums.get(alias)).cloned()
        };
        match (held(self.from, from), held(self.to, to)) {
            (Some(a), Some(b)) => a.holds_the_values_of(&b),
            (None, None) => from == to,
            _ => false,
        }
    }
    // Two unions are one type when their aliases resolve to one union
    // document, as two records are (docs/adr/0092, section 6.1, item 7).
    fn same_union(&self, from: &str, to: &str) -> bool {
        let identity = |scope: Option<&crate::forge::value_type::TypeScope>, alias: &str| {
            scope
                .and_then(|scope| scope.unions.get(alias))
                .map(|union| union.identity.clone())
        };
        matches!(
            (identity(self.from, from), identity(self.to, to)),
            (Some(a), Some(b)) if a == b
        )
    }
}

/// Whether a list of the type `returned` may be taken whole into the list
/// variable `target`: the relation "fits" (docs/adr/0086, section 5.1), its
/// element the variable's own and its bound at most the variable's, each record
/// and enum told apart by `names`. A variable whose type states no bound holds
/// nothing: it is judged as a bound of 0, so its element is still judged first.
fn list_fits(
    returned: &crate::forge::value_type::ValueType,
    target: &Variable,
    names: &dyn crate::forge::value_type::TypeIdentity,
) -> Result<(), crate::forge::value_type::Misfit> {
    let held = target.value_type.clone().map(|ty| {
        if ty.stated_outer_bound().is_some() {
            ty
        } else {
            ty.clone()
                .with_outer_bound(crate::forge::value_type::Bound::of(0))
                .unwrap_or(ty)
        }
    });
    match held.map(|ty| ty.bounded()) {
        Some(Ok(to)) => crate::forge::value_type::fits(returned, &to, names),
        Some(Err(_)) | None => Err(crate::forge::value_type::Misfit {
            path: String::new(),
            from: returned.to_string(),
            to: target
                .value_type
                .as_ref()
                .map_or_else(String::new, ToString::to_string),
            why: crate::forge::value_type::MisfitReason::Bound,
        }),
    }
}

/// How a refusal of a string's (or a byte string's) initial value names what
/// holds it: a variable by its `sce:capacity`, a record's field by the
/// `sce:max-size` its schema declares.
struct StringHolder<'a> {
    holder: &'a str,
    bound: &'a str,
    declarer: &'a str,
    /// What the bound counts: `UTF-8 bytes` of a string, `bytes` of a byte string.
    unit: &'a str,
    /// A byte string starts at printable ASCII, the one literal every engine
    /// spells the same bytes for ([`crate::forge::expr::decode_bytes_literal`]).
    printable_ascii: bool,
}

impl<'a> StringHolder<'a> {
    /// A string variable, bounded by its `sce:capacity`.
    const STRING_VARIABLE: Self = Self {
        holder: "a string variable",
        bound: "sce:capacity",
        declarer: "the variable",
        unit: "UTF-8 bytes",
        printable_ascii: false,
    };
    /// A byte string variable, bounded by its `sce:capacity`.
    const BYTES_VARIABLE: Self = Self {
        holder: "a bytes variable",
        bound: "sce:capacity",
        declarer: "the variable",
        unit: "bytes",
        printable_ascii: true,
    };
    /// A record's string field, bounded by the `sce:max-size` its schema declares.
    const STRING_FIELD: Self = Self {
        holder: "a record's string field",
        bound: "sce:max-size",
        declarer: "its schema",
        unit: "UTF-8 bytes",
        printable_ascii: false,
    };
    /// A record's byte-string field, bounded likewise.
    const BYTES_FIELD: Self = Self {
        holder: "a record's bytes field",
        bound: "sce:max-size",
        declarer: "its schema",
        unit: "bytes",
        printable_ascii: true,
    };
}

/// How the value of a `<param>` crosses (docs/adr/0055, section 4): as JSON,
/// the event data a `<send>`, a `<donedata>` and a Mesh request carry, where an
/// absent value is `null`; or as text alone, a BasicHTTP form and the `params`
/// of an `<invoke>` the host runs, which has no spelling for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Carrier {
    Json,
    Text,
}

impl Carrier {
    /// The carrier of a `<send>`'s data: text alone when the send may go
    /// through the BasicHTTP processor, which posts a form, and JSON otherwise.
    fn of_send(action: &Action) -> Self {
        let basic_http = crate::host_processor_analyzer::BASIC_HTTP_EVENT_PROCESSOR_TYPE;
        if action.send_type == basic_http || action.types.iter().any(|t| t == basic_http) {
            Self::Text
        } else {
            Self::Json
        }
    }
}

/// Whether a variable of an enum holds a value of it always, or an optional
/// value of it (docs/adr/0055).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnumHolding {
    Present,
    Optional,
}

struct Judge<'a> {
    scope: &'a StaticScope,
    enums: &'a [StaticEnum],
    /// Every enum the document holds a value of, by the name it holds each by
    /// ([`SCXMLModel::held_enums`]): the ones it imports, which `enums` holds as
    /// its text names them, and the ones a field of its schemas holds that it
    /// does not import (docs/adr/0054).
    held_enums: &'a std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
    /// The schemas the document imports, by alias — the fields of a record a
    /// loop walks are read from them.
    schemas: &'a std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
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
    payload_schema: std::cell::RefCell<Option<crate::forge::model::EntrySchema>>,
    /// Whether the event of the transition the walk is in declares a schema
    /// at all, whatever its fields hold: what puts its payload in scope for a
    /// value a `<param>` reads ([`Self::wire_param`]), as for an `<assign>` or
    /// an `<if>`. [`Self::payload_schema`] is the schema seen as one entry per
    /// field, which a schema that holds a composed field has none of, though a
    /// scalar field of it is a typed scalar like any other.
    payload_declared: std::cell::Cell<bool>,
    /// Every item and index of the `<foreach>`es the walk is inside: values
    /// the loop binds and the body reads, and does not write.
    loop_names: std::cell::RefCell<Vec<String>>,
    /// Each enum-typed variable's id and the alias of the enum it holds.
    enum_vars: std::collections::BTreeMap<String, String>,
    diag_label: &'a str,
    /// Every name an expression of the document reads or calls, so an
    /// imported algorithm nothing calls is found.
    read: std::cell::RefCell<std::collections::BTreeSet<String>>,
    /// Set by the `<assign>` that takes what an algorithm returns as a record or a
    /// list, around the judging of its right side: a call that returns one is a value
    /// there and nowhere else.
    whole_state_call_ok: std::cell::Cell<bool>,
    /// Set by a `<sce:call>` around the judging of its `expr`: a call of an
    /// algorithm that writes a map in place is that statement's whole call and
    /// no other expression's value (docs/adr/0108, D6 item 2).
    in_place_call_ok: std::cell::Cell<bool>,
    /// The map and set variables the `<foreach>`es the walk is inside walk:
    /// maps no `<sce:call>` in their bodies hands an algorithm to write in
    /// place (docs/adr/0108, D6 item 2).
    walked_maps: std::cell::RefCell<Vec<String>>,
    /// The id of the state whose content the walk is in, empty at document
    /// scope: where a refusal of an expression by a rule of the data model is
    /// placed, for an expression judged without its action.
    walk_state: std::cell::RefCell<String>,
    /// Who reads the payload of a `<send>`, where the build knows ([`Receivers`]).
    receivers: Receivers<'a>,
    /// Every record schema the document imports, by alias, whole: what a
    /// `<sce:lift>` reads a text as (docs/adr/0087), a schema that holds a list
    /// or a record included, which [`Self::schemas`] leaves out.
    lift_schemas: &'a std::collections::BTreeMap<String, crate::forge::model::EventSchemaModel>,
    /// The scope the document names its types in, each record with the
    /// document it is: what tells a lift's refusal record from another, and
    /// what the depth and the longest pointer of a record are folded in.
    type_scope: crate::forge::value_type::TypeScope,
    /// The text templates the document imports, by alias: what a
    /// `<sce:render>` renders (docs/adr/0087, section 3.1, item 6).
    text_templates:
        &'a std::collections::BTreeMap<String, crate::forge::text_template::ImportedTextTemplate>,
    /// The enum documents the document imports, by alias: what tells a value of
    /// an enum hole's enum from a value of another document's.
    enum_documents:
        &'a std::collections::BTreeMap<String, crate::forge::schema_revision::EnumDocument>,
}

/// The documents that read the payload of a `<send>` of this document, where the
/// build knows them: the document itself, which reads an event by the schema it
/// imports for it (a send to `#_internal`, and the schema a send declares its
/// payload by whatever its target), and each child session it starts, by the id
/// a `<send target="#_<id>">` names it by, which reads an event by the schema it
/// declares for it (docs/SCE_ACCEPTED_SUBSET.md §2.15, "Child sessions").
struct Receivers<'a> {
    /// The schemas this document imports, by the event each declares.
    own_schemas: &'a std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    /// The enums this document holds values of, by the name it holds each by
    /// ([`SCXMLModel::held_enums`]): what an enum field of one of its own schemas
    /// holds, and what an enum value of it is.
    own_enums: &'a std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
    /// Each child session's id, with each document it may be (one, or a hybrid
    /// invoke's candidates) by name and what that document reads.
    children: std::collections::BTreeMap<String, Vec<(&'a str, &'a crate::model::ChildReceives)>>,
}

impl<'a> Receivers<'a> {
    fn of(
        model: &'a SCXMLModel,
        held_enums: &'a std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
        own_schemas: &'a std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    ) -> Self {
        let mut children: std::collections::BTreeMap<String, Vec<_>> = Default::default();
        for invoke in model.states.values().flat_map(|s| s.invokes.iter()) {
            match invoke {
                Invoke::Scxml(info) => {
                    if let Some(receives) = &info.common.child_receives {
                        children
                            .entry(info.common.base.invoke_id.clone())
                            .or_default()
                            .push((info.common.child_name.as_str(), receives));
                    }
                }
                Invoke::Hybrid(info) => {
                    for candidate in &info.candidates {
                        if let Some(receives) = &candidate.child_receives {
                            children
                                .entry(info.common.base.invoke_id.clone())
                                .or_default()
                                .push((candidate.stem.as_str(), receives));
                        }
                    }
                }
                Invoke::MeshRpc(_) | Invoke::Unsupported(_) => {}
            }
        }
        Self {
            own_schemas,
            own_enums: held_enums,
            children,
        }
    }

    /// The child sessions a `<send>` to `target` (as written) is delivered to: a
    /// `#_<id>` that names the id of an invoke of this document. `#_internal`,
    /// `#_parent` and `#_scxml_<session>` name no child.
    fn children_of(&self, target: &str) -> &[(&'a str, &'a crate::model::ChildReceives)] {
        let id = target.trim().strip_prefix("#_").unwrap_or_default();
        if id.is_empty() || id == "internal" || id == "parent" || id.starts_with("scxml_") {
            return &[];
        }
        self.children.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// A value a `<send>` carries, as a field that reads it holds it to: a value of an
/// enum, by the enum the sender holds it in, or a value of another type, with the
/// most bytes a text or a byte string can hold where a declaration states it
/// ([`Judge::text_bound`]).
enum SentValue<'m> {
    Enum {
        alias: String,
        model: Option<&'m crate::forge::model::EnumModel>,
        /// An optional value of the enum (docs/adr/0055), which may be absent.
        optional: bool,
    },
    Typed {
        ty: InferredType,
        bound: Option<u32>,
    },
}

/// Why a field cannot hold a value a `<send>` carries ([`SentValue::misfit`]).
enum Misfit {
    /// The value is of a kind the field never holds: no delivery of it fits.
    Kind(String),
    /// The value is a text or a byte string whose declared bound is past the field's:
    /// a delivery past the field's bound is refused where it arrives.
    Bound(String),
}

impl SentValue<'_> {
    /// Why `field`, whose enum (when it holds one) is `field_enum`, cannot hold this
    /// value as it crosses — as text, a number or a truth value, an enum's value as
    /// the name its variant declares — or `None` when it can. The width of a number
    /// is held when the delivery is read, as every payload is; the kind of a value
    /// is the sender's to get right, since no delivery of it ever fits. So is the
    /// bound of a text or a byte string whose own bound is declared: a source that
    /// may hold more than the field is a send that is refused whenever it does, and
    /// the narrowing is the sender's to write (docs/adr/0050).
    fn misfit(
        &self,
        field: &crate::forge::model::EntryField,
        field_enum: Option<&crate::forge::model::EnumModel>,
    ) -> Option<Misfit> {
        use crate::forge::model::SceType;
        let declared = field.sce_type.as_attr();
        // A value of `T` sent to a field of `optional<T>` is present
        // (docs/adr/0055): it fits the field as it fits a field of `T`. An
        // optional value fits a field of `optional<T>` as its present value
        // fits one of `T`, and no field of `T`, which refuses an absent one.
        let present = field.sce_type.present_type();
        let sent_optional = match self {
            SentValue::Enum { optional, .. } => *optional,
            SentValue::Typed { ty, .. } => ty.is_optional(),
        };
        if sent_optional && field.sce_type.optional_elem().is_none() {
            let sent = match self {
                SentValue::Enum { alias, .. } => format!("an optional value of the enum `{alias}`"),
                SentValue::Typed { ty, .. } => ty.describe(),
            };
            return Some(Misfit::Kind(format!(
                "the value is {sent}, which may be absent, and the field is {declared}, which \
                 holds no absent value (send it with a default, `?? <value>`, or send its \
                 `.value` where a `!== null` test proves it present)"
            )));
        }
        match (self, present) {
            (SentValue::Enum { model, .. }, SceType::Enum(_)) => {
                let (Some(sent), Some(read)) = (model, field_enum) else {
                    return None;
                };
                crate::forge::schema_revision::enum_divergence(sent, read).map(|_| {
                    Misfit::Kind(format!(
                        "the value is of the enum `{}`, and the field holds `{}`, which does not \
                         declare the same variants with the same values (a value crosses as the \
                         name its variant declares, and a name the field's enum does not declare \
                         is refused when it arrives)",
                        sent.name, read.name
                    ))
                })
            }
            (SentValue::Enum { alias, .. }, _) => Some(Misfit::Kind(format!(
                "the value is of the enum `{alias}`, which crosses as the name its variant \
                 declares, and the field is {declared} (`{alias}.toNumber(value)` is the number \
                 its variant declares)"
            ))),
            (SentValue::Typed { ty, bound }, _) => {
                let ty = &ty.unwrap_optional();
                let want = InferredType::from_sce_type(present);
                // A text or a byte string the sender declares a bound for, past the
                // bound the field declares: refused unless the sender narrows it.
                if let (InferredType::Str | InferredType::Bytes, Some(held), Some(max)) =
                    (want, *bound, field.max_size)
                {
                    if *ty == want && held > max {
                        let unit = if want == InferredType::Str {
                            "UTF-8 bytes"
                        } else {
                            "bytes"
                        };
                        return Some(Misfit::Bound(format!(
                            "the value may hold {held} {unit}, the bound its declaration states, \
                             and the field holds at most {max} (its sce:max-size)"
                        )));
                    }
                }
                let fits = match (want, *ty) {
                    (_, InferredType::Unknown) => true,
                    // An enum field reads the name of a variant, which no value of
                    // another type is.
                    (InferredType::Unknown, _) => !matches!(present, SceType::Enum(_)),
                    (want, got) if want.is_integer_like() => got.is_integer_like(),
                    (want, got) if want.is_numeric() => got.is_numeric(),
                    (InferredType::Bool, got) => got == InferredType::Bool,
                    (InferredType::Str, got) => got == InferredType::Str,
                    (InferredType::Bytes, got) => got == InferredType::Bytes,
                    _ => true,
                };
                (!fits).then(|| {
                    Misfit::Kind(format!(
                        "the value is {}, and the field is {declared}",
                        ty.describe()
                    ))
                })
            }
        }
    }
}

impl<'a> Judge<'a> {
    /// The alias this document imports the schema of the event the transition is on
    /// under, which `_event.data` taken whole is a record of.
    fn payload_alias(&self) -> Option<String> {
        let payload = self.payload_schema.borrow();
        let payload = payload.as_ref()?;
        self.schemas
            .iter()
            .find(|(_, held)| held.name == payload.name)
            .map(|(alias, _)| alias.clone())
    }

    /// `expr` with each call of an algorithm that takes a record or a list rewritten
    /// into the call the rest of the judge reads ([`StaticScope::rewrite_state_calls`]).
    fn rewrite_state(
        &self,
        expr: &str,
    ) -> Result<crate::forge::type_ctx::StateRewrite, crate::forge::type_ctx::StateCallRefusal>
    {
        let loops = self.loop_records.borrow();
        let payload = self.payload_alias();
        let payload_holds_text = self.payload_schema.borrow().as_ref().is_some_and(|schema| {
            schema.fields.iter().any(|f| {
                matches!(
                    f.sce_type.present_type(),
                    crate::forge::model::SceType::String
                )
            })
        });
        self.scope.rewrite_state_calls(
            expr,
            &crate::forge::type_ctx::StateContext {
                loop_records: &loops,
                payload_alias: payload.as_deref(),
                payload_holds_text,
            },
        )
    }

    /// The alias of an imported algorithm that writes a map in place
    /// ([`crate::forge::type_ctx::StaticValueSignature::writes_in_place`]) that
    /// `expr` calls where no call of one stands: anywhere inside it, and as the
    /// whole of it but where a `<sce:call>` judges its `expr`
    /// ([`Self::in_place_call_ok`]). `None` for an expression that calls none,
    /// and for one that does not parse, which the judge refuses in its own
    /// words.
    fn in_place_call_in(&self, expr: &str) -> Option<String> {
        let (whole, others) = crate::forge::expr::called_names(expr).ok()?;
        let writes = |name: &str| {
            self.scope
                .value_callee(name)
                .and_then(|callee| callee.value.as_ref())
                .is_some_and(crate::forge::type_ctx::StaticValueSignature::writes_in_place)
        };
        others
            .into_iter()
            .find(|name| writes(name.as_str()))
            .or_else(|| whole.filter(|name| !self.in_place_call_ok.get() && writes(name.as_str())))
    }

    fn ctx<'c>(&self, payload: &'c [(String, crate::forge::value_path::Place)]) -> TypeCtx<'c>
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
        self.expr_held(ctx, expr, spelling, expected)
            .map(|(ty, _)| ty)
    }

    /// [`Self::expr`], with the enum `expr` is a present value of, when it is
    /// one ([`Self::value_enum`]): the lattice types an enum value `Unknown`,
    /// so a place that tells it from a value of no type reads it here, off the
    /// expression as it was judged.
    fn expr_held(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        expected: Expected,
    ) -> Result<(InferredType, Option<String>), Located<ForgeError>> {
        let place = |refusal: crate::forge::expr::Refusal| {
            Located::in_file(
                ExpressionSite::new(expr, spelling).place(refusal),
                self.diag_label,
            )
        };
        self.record_reads(expr);
        // `_event.data` is the payload of the event the transition is on, read
        // through the schema that event declares. Everywhere else it names no
        // value of the machine: the data of an event without a schema (the
        // failure code of an `error.*` event, a refusal's reason) is text this
        // model has no type for, and an entry or an exit has no event. Such a
        // read was judged a value of no type, which fits every place, and
        // written into the generated machine as the author wrote it.
        if !self.payload_declared.get() && crate::forge::expr::references_event_data_lexically(expr)
        {
            return Err(self.rule_at(
                format!(
                    "a read of `_event.data` in the expression `{}`",
                    expr.trim()
                ),
                "no payload is in scope: `_event.data` is the payload of the event a transition \
                 is on, read there through the schema an import declares for that event, and an \
                 entry, an exit or a transition on an event no imported schema declares has none. \
                 Import a schema whose sce:event-name is the event's, or branch on the event's \
                 name",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                &self.walk_state.borrow(),
                expr.trim(),
            ));
        }
        // A call of an algorithm that writes a map in place is the statement
        // `<sce:call>`, the whole of its `expr`, and no value of any other
        // expression, which has no effect (docs/adr/0108, D6 item 2).
        if let Some(callee) = self.in_place_call_in(expr) {
            return Err(self.rule_at(
                format!("a call of `{callee}` in the expression `{}`", expr.trim()),
                &format!(
                    "`{callee}` writes a map in place, and an algorithm that does is called by \
                     the statement <sce:call expr=\"{callee}(…)\"/>, the whole of its expr, never \
                     in an expression, which has no effect"
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                &self.walk_state.borrow(),
                &callee,
            ));
        }
        // A record or a list a statechart hands an algorithm is named at the call, and
        // the call is judged with each such name replaced by a placeholder, once the
        // name is known to be a variable of the type the algorithm takes. What is
        // judged below is that rewritten expression; the author's is the one a refusal
        // is placed in.
        let state = self.rewrite_state(expr).map_err(|refusal| {
            place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!("a call of `{}`: {}", refusal.callee, refusal.reason),
                    observed: Some(refusal.callee),
                }
                .at(None),
            )
        })?;
        // A record or a list an algorithm returns is the right side of the assignment
        // that holds it, and is no value anywhere else.
        let unfit = |callee: &str| {
            place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "the record or list `{callee}` returns, used as a value (it is taken \
                         whole by an <assign> to a variable of its type)"
                    ),
                    observed: Some(callee.to_string()),
                }
                .at(None),
            )
        };
        if let Some(callee) = self.scope.state_returning_call_used_as_value(expr) {
            return Err(unfit(&callee));
        }
        if !self.whole_state_call_ok.get() {
            if let Some(callee) = self
                .scope
                .whole_value_call(expr)
                .filter(|c| StaticScope::returns_state(c))
            {
                return Err(unfit(&callee.alias));
            }
        }
        let expr = state.text.as_str();
        if let Some(list) = self.scope.list_read_as_value(expr) {
            return Err(place(list_read_refusal(&list)));
        }
        if let Some(map) = self.scope.map_read_as_value(expr) {
            return Err(place(map_read_refusal(&map)));
        }
        // A list an imported algorithm returns is taken whole into a list
        // variable and is no value anywhere else.
        if let Some(callee) = self.scope.list_returning_call_used_as_value(expr) {
            return Err(place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "the list `{callee}` returns, used as a value (it is taken whole by an \
                         <assign> to a list variable)"
                    ),
                    observed: Some(callee),
                }
                .at(None),
            ));
        }
        let ty = judge_into(expr, ctx, expected).map_err(place)?;
        // A whole union is typed `Unknown` too, so the slot cannot refuse it a
        // value's place either (docs/adr/0092, section 6.1): it is read through
        // its tag and the fields its tests prove, and taken whole only into a
        // variable of its union, which no place of a value type is.
        if let Expected::Slot(slot) = expected {
            let union = self
                .whole_place(ctx, expr)
                .filter(|held| holds_whole_union(&held.ty));
            if let Some(union) = union.filter(|_| slot != InferredType::Unknown) {
                return Err(place(
                    crate::forge::error::ExprError::UnsupportedConstruct {
                        construct: format!(
                            "`{}`, a value of `{}`, where a value of {} is expected (a union is \
                             read through its tag and the fields its tests prove, and is taken \
                             whole only into a variable of its union)",
                            expr.trim(),
                            union.ty,
                            slot.describe()
                        ),
                        observed: Some(expr.trim().to_string()),
                    }
                    .at(None),
                ));
            }
        }
        // An enum value is typed `Unknown`, so the slot cannot refuse it a
        // number's place: say here where it may stand and where it may not.
        let held = self.value_enum(ctx, expr).map_err(place)?;
        if let Some(alias) = &held {
            if let Expected::Slot(slot) = expected {
                // A place of an optional enum takes a value of the enum, which
                // [`Self::expr_of_optional_enum`] holds to the one it declares.
                if !matches!(
                    slot,
                    InferredType::Unknown
                        | InferredType::Optional(crate::forge::types::OptionalElem::Enum)
                ) {
                    return Err(place(
                        crate::forge::error::ExprError::UnsupportedConstruct {
                            construct: format!(
                                "a value of the enum `{alias}` where a number or a bool is expected \
                                 (`{alias}.toNumber(value)` is the number its variant declares)"
                            ),
                            observed: Some(expr.trim().to_string()),
                        }
                        .at(None),
                    ));
                }
            }
        }
        Ok((ty, held))
    }

    /// The enum `expr` is a value of ([`crate::forge::static_enum`]), refused
    /// when it uses one for what an enum value is not for.
    fn value_enum(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
    ) -> Result<Option<String>, crate::forge::expr::Refusal> {
        Ok(self.value_held(ctx, expr)?.present_enum())
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

    /// The alias of the enum the optional variable, record field, loop's record
    /// item's field or payload field at `path` holds an optional value of
    /// (docs/adr/0055): the enum its present value, `<path>.value`, is a value
    /// of, for a path `ctx` types optional.
    fn optional_enum_of(&self, ctx: &TypeCtx<'_>, path: &str) -> Option<String> {
        if !ctx.lookup_var(path).is_optional() {
            return None;
        }
        self.enum_of(&crate::forge::static_enum::present_value_path(path))
    }

    /// What `expr` is as far as an enum goes ([`crate::forge::static_enum::Held`]),
    /// refused when it uses an enum value for what an enum value is not for.
    fn value_held(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
    ) -> Result<crate::forge::static_enum::Held, crate::forge::expr::Refusal> {
        crate::forge::static_enum::value_held(
            expr,
            ctx,
            &crate::forge::static_enum::EnumReading {
                var_alias: &|name| self.enum_of(name).or_else(|| place_enum(ctx, name)),
                calls: &|name| self.scope.call_enums(name),
            },
        )
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
    /// whole is a record of. Each field names its enum as this document holds
    /// the enum document the schema imports (docs/adr/0054), so a value of it is
    /// compared and assigned with a value of that same document's enum alone.
    /// `whole` is the schema the event declares, if it declares one, whatever
    /// its fields hold, which the enum fields are read from
    /// ([`enum_field_aliases`]); `schema` is its view of one entry per field,
    /// absent for a schema that holds a composed field.
    fn open_payload(
        &self,
        schema: Option<&crate::forge::model::EntrySchema>,
        whole: Option<&crate::forge::model::EventSchemaModel>,
    ) {
        *self.payload_enum_paths.borrow_mut() = whole
            .map(|whole| enum_fields(crate::forge::event_schema_check::EVENT_DATA_PATH, whole))
            .unwrap_or_default();
        *self.payload_schema.borrow_mut() = schema.cloned();
        self.payload_declared.set(whole.is_some());
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
        use crate::forge::static_enum::Held;
        self.expr(ctx, expr, spelling, Expected::Hint(InferredType::Unknown))?;
        let observed = Some(expr.trim().to_string());
        match self.value_held(ctx, expr).map_err(place)? {
            Held::Enum(held) if held == alias => Ok(()),
            // A value that may be absent, or none, where one that never is is
            // declared (docs/adr/0055): what the place would hold has no answer.
            Held::Optional(held) => Err(place(
                crate::forge::error::ExprError::TypeMismatch {
                    expected: format!("a value of the enum `{alias}`"),
                    got: format!(
                        "an optional value of the enum `{held}`, which may be absent (read it \
                         with `??` and a default, or with `.value` where a `!== null` test \
                         proves it present)"
                    ),
                    observed,
                }
                .at(None),
            )),
            Held::Null => Err(place(
                crate::forge::error::ExprError::TypeMismatch {
                    expected: format!("a value of the enum `{alias}`"),
                    got: InferredType::Null.describe(),
                    observed,
                }
                .at(None),
            )),
            Held::Enum(_) | Held::Neither => Err(place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a value that is not of the enum `{alias}` for a variable declared \
                         enum:{alias} (write `{alias}.<variant>`, or another variable of that enum)"
                    ),
                    observed,
                }
                .at(None),
            )),
        }
    }

    /// `expr`, which a place declared `optional<enum:<alias>>` is to hold
    /// (docs/adr/0055): `null`, a value of that enum, which is present, or an
    /// optional value of it.
    fn expr_of_optional_enum(
        &self,
        ctx: &TypeCtx<'_>,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        alias: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::static_enum::Held;
        let place = |refusal: crate::forge::expr::Refusal| {
            Located::in_file(
                ExpressionSite::new(expr, spelling).place(refusal),
                self.diag_label,
            )
        };
        self.expr(ctx, expr, spelling, Expected::Hint(InferredType::Unknown))?;
        match self.value_held(ctx, expr).map_err(place)? {
            Held::Null => Ok(()),
            Held::Enum(held) | Held::Optional(held) if held == alias => Ok(()),
            Held::Enum(_) | Held::Optional(_) | Held::Neither => Err(place(
                crate::forge::error::ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a value that is neither `null` nor of the enum `{alias}` for a place \
                         declared optional<enum:{alias}> (write `{alias}.<variant>`, `null`, or \
                         another value of that enum)"
                    ),
                    observed: Some(expr.trim().to_string()),
                }
                .at(None),
            )),
        }
    }

    /// A variable declared `enum:<alias>`, or `optional<enum:<alias>>` as
    /// `holding` says: the enum is a closed set the document imports, and the
    /// variable starts at one of its variants, or absent where it is optional.
    fn enum_variable(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        enums: &std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
        holding: EnumHolding,
    ) -> Result<(), Located<ForgeError>> {
        // An alias the parser admitted names an import; without sibling files
        // it has no enum here, and nothing to judge against.
        if let Some(model) = enums.get(alias) {
            // A machine holds a declared variant and nothing else; the open
            // set's value that no variant names has no type to be held in.
            if !model.strict_variants {
                let spelling = var.value_type_spelling.as_ref();
                let declared = var
                    .value_type
                    .as_ref()
                    .map_or_else(|| format!("enum:{alias}"), |t| t.as_attr());
                return Err(self.rule_at(
                    format!("<data id=\"{}\" sce:type=\"{declared}\">", var.id),
                    "a variable holds a closed enum: the enum declares \
                     sce:strict-variants=\"false\", which admits values no variant names",
                    spelling.map(|s| s.row()),
                    spelling.map(|s| s.col()),
                    "",
                    alias,
                ));
            }
        }
        // The parser refuses a `<data>` with no `expr` but for a test
        // component's module parameter, whose value is the suite's
        // (docs/SCE_TEST_PROFILE.md §11.5, §11.8): it starts at no value of the
        // document's, so there is no start to judge, as for a variable of any
        // other type; the enum it holds is held closed above all the same.
        if var.expr.trim().is_empty() {
            return Ok(());
        }
        let (expr, spelling) = (&var.expr, var.expr_spelling.as_ref());
        match holding {
            EnumHolding::Present => self.expr_of_enum(ctx, expr, spelling, alias),
            EnumHolding::Optional => self.expr_of_optional_enum(ctx, expr, spelling, alias),
        }
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
        let Some(capacity) = var.capacity() else {
            return Ok(());
        };
        let Some(value_type) = var.value_type.as_ref() else {
            return Ok(());
        };
        // An optional text starts absent, or present at a literal that fits, as
        // a text does (docs/adr/0055).
        let present = match value_type.optional_elem() {
            Some(_) if is_null_literal(&var.expr) => return Ok(()),
            Some(elem) => elem.scalar(),
            None => value_type.scalar(),
        };
        let holder = match present {
            Some(crate::forge::model::SceType::String) => StringHolder::STRING_VARIABLE,
            // A byte string starts the same way, at a literal that fits.
            Some(crate::forge::model::SceType::Bytes) => StringHolder::BYTES_VARIABLE,
            _ => return Ok(()),
        };
        self.string_literal_fits(
            format!("<data id=\"{}\" expr=\"{}\">", var.id, var.expr),
            &var.expr,
            var.expr_spelling.as_ref(),
            capacity,
            holder,
        )
    }

    /// A record's string (or byte-string) field starts at a literal that fits the
    /// `sce:max-size` its schema declares, for the reason a variable's does
    /// ([`Self::string_start`]).
    fn string_field_start(
        &self,
        construct: String,
        init: &crate::forge::model::RecordFieldInit,
        max_size: u32,
        holder: StringHolder<'_>,
    ) -> Result<(), Located<ForgeError>> {
        self.string_literal_fits(
            construct,
            &init.expr,
            init.expr_spelling.as_ref(),
            max_size,
            holder,
        )
    }

    /// `expr` is a string literal of at most `bound` UTF-8 bytes: the value a
    /// machine is built with, before there is a block to end or an error to
    /// raise for a value that does not fit.
    fn string_literal_fits(
        &self,
        construct: String,
        expr: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        bound: u32,
        holder: StringHolder<'_>,
    ) -> Result<(), Located<ForgeError>> {
        let literal = match crate::forge::expr::parse_to_ast(expr).map(|ast| ast.kind) {
            Ok(crate::forge::expr::ExprKind::StringLit { value, .. }) => value,
            _ => {
                return Err(self.rule_at(
                    construct,
                    &format!(
                        "{} starts at a string literal: the machine is built with no error to \
                         raise for a value that does not fit its {}",
                        holder.holder, holder.bound
                    ),
                    spelling.map(|s| s.row()),
                    spelling.map(|s| s.col()),
                    "",
                    expr,
                ));
            }
        };
        // A byte string starts at printable ASCII, so that its bytes are the
        // literal's bytes on every engine.
        if holder.printable_ascii && crate::forge::expr::decode_bytes_literal(&literal).is_none() {
            return Err(self.rule_at(
                construct,
                &format!(
                    "{} starts at a literal of printable ASCII with no backslash, the one \
                     literal whose bytes every engine spells alike",
                    holder.holder
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                expr,
            ));
        }
        if literal.len() > bound as usize {
            return Err(self.rule_at(
                construct,
                &format!(
                    "the initial value is {} {}, past the {} of {bound} {} declares",
                    literal.len(),
                    holder.unit,
                    holder.bound,
                    holder.declarer
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                expr,
            ));
        }
        Ok(())
    }

    /// A record whose schema has a `string` or a `bytes` field holds it within the
    /// `sce:max-size` the schema writes, as a variable is held within its
    /// `sce:capacity` (docs/adr/0005, decisions 1 and 2); a field that declares
    /// none has no bound for the machine to keep, and no default stands in for
    /// one. Judged where the record is declared, in a variable or in the elements
    /// of a list.
    fn schema_strings_bounded(
        &self,
        var: &Variable,
        alias: &str,
        schema: &crate::forge::model::EntrySchema,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::model::SceType;
        // An optional text is bounded as the value it holds when present is
        // (docs/adr/0055).
        let Some(field) = schema.fields.iter().find(|f| {
            matches!(f.sce_type.present_type(), SceType::String | SceType::Bytes)
                && f.max_size.is_none()
        }) else {
            return Ok(());
        };
        let kind = match field.sce_type.present_type() {
            SceType::Bytes => "a byte string",
            _ => "a string",
        };
        let spelling = var.value_type_spelling.as_ref();
        Err(self.rule_at(
            format!("<data id=\"{}\"> of record:{alias}", var.id),
            &format!(
                "the field `{}` of record:{alias} is {kind} its schema does not bound: a \
                 record holds it within a bound, written as sce:max-size=\"N\" on the field",
                field.id
            ),
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
            "",
            &field.id,
        ))
    }

    /// An `<assign>` of `callee`'s list to the list variable `list`: the call's
    /// arguments are judged against the scalars it takes, its element is the
    /// list's own, and the list can hold as many as it may return — a machine
    /// holds the same list wherever it runs, so a bound that could be passed is
    /// refused when the document is built and not by an overflow later.
    fn list_from_call(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        list: &Variable,
        callee: &crate::forge::type_ctx::StaticCallee,
    ) -> Result<(), Located<ForgeError>> {
        let returned = callee
            .list_return
            .as_ref()
            .expect("whole_list_call answers a callee that returns a list");
        let place = |construct: String| {
            Located::in_file(
                ExpressionSite::new(&action.expr, action.spellings.get("expr")).place(
                    crate::forge::error::ExprError::UnsupportedConstruct {
                        construct,
                        observed: Some(action.expr.trim().to_string()),
                    }
                    .at(None),
                ),
                self.diag_label,
            )
        };
        // The list is taken whole, so the type the call returns fits the
        // variable's (docs/adr/0086, section 5.1): its element is the list's own,
        // and the list can hold as many as it may return.
        let returned_ty = crate::forge::value_type::ValueType::List {
            elem: Box::new(crate::forge::value_type::ValueType::Scalar(
                returned.elem.clone(),
            )),
            max: crate::forge::value_type::Bound::of(returned.max_size),
        };
        match list_fits(&returned_ty, list, &StatechartNames) {
            Ok(()) => {}
            Err(misfit) if misfit.why == crate::forge::value_type::MisfitReason::Bound => {
                return Err(place(format!(
                    "`{}` may return at most {} of {}, and `{}` holds at most {} (its \
                     sce:capacity)",
                    callee.alias,
                    returned.max_size,
                    returned.elem.as_attr(),
                    list.id,
                    list.capacity().unwrap_or(0)
                )));
            }
            Err(_) => {
                let held = list
                    .value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem);
                return Err(place(match held {
                    Some(crate::forge::model::AlgorithmValueType::Scalar(held)) => format!(
                        "`{}` returns a list of {}, and `{}` holds {}",
                        callee.alias,
                        returned.elem.as_attr(),
                        list.id,
                        held.as_attr()
                    ),
                    _ => format!(
                        "`{}` returns a list of {}, and `{}` holds records",
                        callee.alias,
                        returned.elem.as_attr(),
                        list.id
                    ),
                }));
            }
        }
        let Some(elem) = crate::forge::types::ListElem::of(&returned.elem) else {
            return Err(place(format!(
                "`{}` returns a list of {}, which a list variable does not hold",
                callee.alias,
                returned.elem.as_attr()
            )));
        };
        self.expr(
            ctx,
            &action.expr,
            action.spellings.get("expr"),
            Expected::Slot(InferredType::List(elem)),
        )?;
        Ok(())
    }

    /// An `<assign>` of what `callee` returns, a record, a list or a map, to the
    /// variable `target` that holds a value of that type. The call is the whole of the right
    /// side; its arguments are judged by the rewrite every expression passes through
    /// ([`Self::expr`]), and the type returned has to be the variable's own.
    fn state_from_call(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        target: &Variable,
        callee: &crate::forge::type_ctx::StaticCallee,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::model::AlgorithmValueType;
        use crate::forge::type_ctx::StaticSlot;
        let value = callee
            .value
            .as_ref()
            .expect("whole_value_call answers a callee with a value signature");
        let place = |construct: String| {
            Located::in_file(
                ExpressionSite::new(&action.expr, action.spellings.get("expr")).place(
                    crate::forge::error::ExprError::UnsupportedConstruct {
                        construct,
                        observed: Some(action.expr.trim().to_string()),
                    }
                    .at(None),
                ),
                self.diag_label,
            )
        };
        let held = target.value_type.as_ref();
        let held_record = held.and_then(AlgorithmValueType::record_alias);
        let held_optional_record = held
            .and_then(AlgorithmValueType::optional_elem)
            .and_then(AlgorithmValueType::record_alias);
        let held_list = held.and_then(AlgorithmValueType::list_elem);
        let held_union = held.and_then(AlgorithmValueType::union_alias);
        let expected = match &value.ret {
            Some(StaticSlot::Record(returned)) if held_record == Some(returned.as_str()) => {
                InferredType::Unknown
            }
            // A union a call answers is taken whole into a union variable of the
            // same document (docs/adr/0092, section 6.1 item 7).
            Some(StaticSlot::Union(returned)) if held_union == Some(returned.alias.as_str()) => {
                InferredType::Unknown
            }
            // A map or a set a call answers is taken whole into a map variable
            // whose entries are its own (docs/adr/0108, D5); its bound is judged
            // below.
            Some(StaticSlot::Map(returned))
                if self.scope.map_argument(returned, &target.id).is_ok() =>
            {
                InferredType::Unknown
            }
            // An optional record a call answers is taken whole into an optional
            // record variable of its schema, and a record into one as a present
            // value (docs/adr/0055, step 9).
            Some(StaticSlot::OptionalRecord(returned))
                if held_optional_record == Some(returned.as_str()) =>
            {
                InferredType::Optional(crate::forge::types::OptionalElem::Record)
            }
            Some(StaticSlot::Record(returned))
                if held_optional_record == Some(returned.as_str()) =>
            {
                InferredType::Unknown
            }
            Some(StaticSlot::ScalarList(returned)) if matches!(held_list, Some(AlgorithmValueType::Scalar(elem)) if elem == returned) => {
                match crate::forge::types::ListElem::of(returned) {
                    Some(elem) => InferredType::List(elem),
                    None => {
                        return Err(place(format!(
                            "`{}` returns a list of {}, which a list variable does not hold",
                            callee.alias,
                            returned.as_attr()
                        )))
                    }
                }
            }
            Some(StaticSlot::RecordList(returned))
                if held_list.and_then(AlgorithmValueType::record_alias)
                    == Some(returned.as_str()) =>
            {
                InferredType::List(crate::forge::types::ListElem::of_value(
                    &crate::forge::value_type::ValueType::Record {
                        alias: returned.clone(),
                    },
                ))
            }
            Some(slot) => {
                return Err(place(format!(
                    "`{}` returns {}, which `{}` does not hold",
                    callee.alias,
                    match slot {
                        StaticSlot::Record(alias) => format!("a record of `{alias}`"),
                        StaticSlot::OptionalRecord(alias) => {
                            format!("an optional record of `{alias}`")
                        }
                        StaticSlot::ScalarList(t) => format!("a list of {}", t.as_attr()),
                        StaticSlot::RecordList(alias) => format!("a list of `{alias}`"),
                        StaticSlot::Scalar(t) => format!("a {}", t.as_attr()),
                        StaticSlot::Optional(t) => {
                            format!("a value of optional<{}>", t.as_attr())
                        }
                        StaticSlot::Union(union) => format!("a union of `{}`", union.alias),
                        StaticSlot::Map(map) => format!("a {}", map.ty.as_attr()),
                    },
                    target.id
                )))
            }
            None => {
                return Err(place(format!("`{}` returns nothing", callee.alias)));
            }
        };
        // A machine holds the same list wherever it runs: a bound it could pass is
        // refused when the document is built and not by an overflow later.
        // The list the call returns is judged as the type it is, in the
        // statechart's names, against the variable's (docs/adr/0086, section 5.1);
        // its element was judged above, so what is left to refuse is its bound. A
        // bound is declared only beside a list return (`StaticValueSignature`).
        let returned = match (&value.ret, value.max_size) {
            (Some(StaticSlot::ScalarList(t)), Some(max)) => {
                Some((crate::forge::value_type::ValueType::Scalar(t.clone()), max))
            }
            (Some(StaticSlot::RecordList(alias)), Some(max)) => Some((
                crate::forge::value_type::ValueType::Record {
                    alias: alias.clone(),
                },
                max,
            )),
            _ => None,
        };
        if let Some((elem, max)) = returned {
            let returned = crate::forge::value_type::ValueType::List {
                elem: Box::new(elem),
                max: crate::forge::value_type::Bound::of(max),
            };
            if list_fits(&returned, target, &StatechartNames).is_err() {
                return Err(place(format!(
                    "`{}` may return at most {max}, and `{}` holds at most {} (its \
                     sce:capacity)",
                    callee.alias,
                    target.id,
                    target.capacity().unwrap_or(0)
                )));
            }
        }
        // ... and a map's the same way: the variable holds every entry the map
        // returned may hold, so a bound it could pass is refused here
        // (docs/adr/0108, D5).
        if let Some(StaticSlot::Map(returned)) = &value.ret {
            let holds = target.capacity().unwrap_or(0);
            if let Some(max) = returned.bound().filter(|max| *max > holds) {
                return Err(place(format!(
                    "`{}` may return a map of {max} entries, and `{}` holds at most {holds} \
                     (its bound): declare it at least {max}",
                    callee.alias, target.id
                )));
            }
        }
        self.whole_state_call_ok.set(true);
        let judged = self.expr(
            ctx,
            &action.expr,
            action.spellings.get("expr"),
            Expected::Hint(expected),
        );
        self.whole_state_call_ok.set(false);
        judged.map(|_| ())
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

    /// The refusal of an `<sce:append>` whose `target` names no list variable,
    /// placed on `target` and naming the lists there are.
    fn not_a_list(&self, action: &Action, state: &str) -> Located<ForgeError> {
        self.no_target(
            action,
            state,
            "a list variable",
            &|ty: &crate::forge::model::AlgorithmValueType| ty.list_elem().is_some(),
        )
    }

    /// An `<sce:append>` whose `target` is no list variable: a list held at a
    /// key of a map, or in a field of a record held there (docs/adr/0108, D12
    /// rule 1, [`Self::written_through`]), `m[k]` and `m[k].tags`, takes the
    /// value appended as a list variable does: a record whole by the name of a
    /// record of its schema, and any other element as a scalar expression of its
    /// type. A `target` that names no list at all is refused naming the list
    /// variables there are.
    fn nested_append(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::value_type::ValueType;
        let place = match self.written_through(ctx, action, state) {
            Some(place) => place?,
            None => return Err(self.not_a_list(action, state)),
        };
        let element = match place.place.element() {
            Some(element) if !place.steps.is_empty() => element,
            _ => return Err(self.not_a_list(action, state)),
        };
        let spelling = action.spellings.get("expr");
        match &element.ty {
            ValueType::Record { alias } => {
                self.record_of("sce:append", &action.expr, spelling, alias, state)
            }
            other => {
                let slot = other
                    .scalar()
                    .map_or(InferredType::Unknown, InferredType::from_sce_type);
                self.expr(ctx, &action.expr, spelling, Expected::Slot(slot))
                    .map(|_| ())
            }
        }
    }

    /// The refusal of an `<sce:put>` or an `<sce:remove>` whose `target` names
    /// no map or set variable (docs/adr/0108, D11).
    fn not_a_map(&self, action: &Action, state: &str) -> Located<ForgeError> {
        self.no_target(
            action,
            state,
            "a map or a set variable",
            &|ty: &crate::forge::model::AlgorithmValueType| ty.map_key().is_some(),
        )
    }

    /// The refusal of a statement whose `target` names no variable `holds`
    /// takes, `what` (`a list variable`), placed on `target` and naming the
    /// variables there are that it takes: an `<sce:append>`'s list, an
    /// `<sce:clear>`'s list, map or set, an `<sce:put>`'s or an
    /// `<sce:remove>`'s map or set (docs/adr/0108, D3).
    fn no_target(
        &self,
        action: &Action,
        state: &str,
        what: &str,
        holds: &dyn Fn(&crate::forge::model::AlgorithmValueType) -> bool,
    ) -> Located<ForgeError> {
        let held: Vec<&str> = self
            .scope
            .variables
            .iter()
            .filter(|v| v.value_type.as_ref().is_some_and(holds))
            .map(|v| v.id.as_str())
            .collect();
        let spelling = action.spellings.get("target");
        let element = action.action_type.trim_start_matches("sce_");
        Located::new(
            ScxmlSemanticError::StaticDatamodelRule {
                construct: format!("<sce:{element} target=\"{}\">", action.location),
                datamodel: Datamodel::SceStatic.as_str().to_string(),
                rule: if held.is_empty() {
                    format!("target names {what}, and this document declares none")
                } else {
                    format!("target names {what}: one of {}", held.join(", "))
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

    /// An `<assign>` whose location is a map or set variable or a part of one
    /// (docs/adr/0108, D2 and D3); `None` for any other. An entry is written by
    /// `<sce:put>`, so a location that reads one (`faces[k]`,
    /// `faces[k].open`) is refused, naming the statement. The whole map takes
    /// only the whole of another map or set variable whose type it fits, a
    /// copy that cannot fail ([`list_fits`], the relation "fits" of
    /// docs/adr/0086, section 5.1), or the map an imported algorithm returns,
    /// the call the whole of the right side, when its entries are the
    /// variable's and the variable holds its bound (D5,
    /// [`Self::state_from_call`]); anything else is refused.
    fn map_assign(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
    ) -> Option<Result<(), Located<ForgeError>>> {
        let location = action.location.trim();
        let root = location.split(['.', '[']).next().unwrap_or(location).trim();
        let (map, _) = self.scope.map_variable(root)?;
        let refuse = |attr: &str, construct: String, observed: &str| {
            let text = if attr == "location" {
                &action.location
            } else {
                &action.expr
            };
            Located::in_file(
                ExpressionSite::new(text, action.spellings.get(attr)).place(
                    crate::forge::error::ExprError::UnsupportedConstruct {
                        construct,
                        observed: Some(observed.to_string()),
                    }
                    .at(None),
                ),
                self.diag_label,
            )
        };
        if root != location {
            return Some(Err(refuse(
                "location",
                format!(
                    "an assignment to `{location}`, a part of an entry of the map `{root}` (an \
                     entry is written whole by <sce:put> and removed by <sce:remove>)"
                ),
                location,
            )));
        }
        let written = action.expr.trim();
        let whole = |why: String| {
            refuse(
                "expr",
                format!(
                    "an assignment to the whole map `{location}` from `{written}`{why} (a map \
                     takes whole only another map variable whose type it fits, and is written \
                     by <sce:put> and <sce:remove> and emptied by <sce:clear>)"
                ),
                written,
            )
        };
        if let Some(callee) = self
            .scope
            .whole_value_call(written)
            .filter(|c| StaticScope::returns_state(c))
        {
            return Some(self.state_from_call(ctx, action, map, callee));
        }
        let Some(held) = self
            .scope
            .map_variable(written)
            .and_then(|(source, _)| crate::forge::value_path::variable_place(source))
        else {
            // A map or a set held at a key (`m[k]`), or bound by a loop over a
            // map of them, is taken whole only into a variable of exactly its
            // type (docs/adr/0108, D12 rule 6): widening a nested value between
            // two bounds is not decided.
            let Some(at) = self
                .whole_place(ctx, written)
                .filter(crate::forge::value_path::Place::is_keyed)
            else {
                return Some(Err(whole(String::new())));
            };
            let exact =
                crate::forge::value_path::variable_place(map).is_some_and(|to| to.ty == at.ty);
            if !exact {
                return Some(Err(whole(format!(
                    ": `{written}` is a value of `{}`, and a map or a set held at a key is taken \
                     whole only into a variable of exactly its type (docs/adr/0108, D12 rule 6)",
                    at.ty
                ))));
            }
            return Some(
                self.expr(
                    ctx,
                    written,
                    action.spellings.get("expr"),
                    Expected::Hint(InferredType::Unknown),
                )
                .map(|_| ()),
            );
        };
        Some(
            list_fits(&held.ty, map, &StatechartNames)
                .map_err(|misfit| whole(format!(": {}", misfit.sentence(written)))),
        )
    }

    /// A `<sce:call>` (docs/adr/0108, D6 item 2): `expr` is one call, written
    /// whole, of an imported algorithm that writes a map in place; each argument
    /// it writes in place is the name of a map or a set variable whose entries
    /// are the parameter's ([`StaticScope::map_argument`]), and not the map a
    /// `<foreach>` around the call walks; no map is named twice in the call; and
    /// `location`, when written, takes what the algorithm returns by the rule an
    /// `<assign>` of that value follows, judged by the `<assign>`'s own judge. A
    /// `location` on an algorithm that returns nothing is refused, and a call
    /// with none drops the value. Each refusal here is
    /// `scxml/static-datamodel-rule`; what the `<assign>` judge and the
    /// expression judge refuse is refused in their words.
    fn call(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::expr::ArgShape;
        use crate::forge::type_ctx::StaticSlot;
        let written = action.expr.trim();
        let location = action.location.trim();
        let refused = |attr: &str, rule: &str, observed: &str| {
            let spelling = action
                .spellings
                .get(attr)
                .or_else(|| action.spellings.get("expr"));
            self.rule_at(
                format!("<sce:call expr=\"{written}\">"),
                rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                observed,
            )
        };
        let one_call = "a <sce:call> calls one algorithm the document imports, and its expr is \
                        that call written whole: `Algorithm(a, b)`";
        let Ok((Some(alias), _)) = crate::forge::expr::called_names(written) else {
            return Err(refused("expr", one_call, written));
        };
        let Some(callee) = self
            .scope
            .callees
            .iter()
            .find(|c| c.alias == alias && c.kind.is_algorithm())
        else {
            return Err(refused("expr", one_call, &alias));
        };
        let Some(value) = callee.value.as_ref().filter(|v| v.writes_in_place()) else {
            let why = match &callee.host_only {
                Some(why) => format!("`{alias}` is no algorithm a statechart calls: {why}"),
                None => format!(
                    "a <sce:call> calls an algorithm that writes a map in place (an inout \
                     parameter), and `{alias}` writes none: call it in an expression, as the \
                     value of an <assign> or a guard"
                ),
            };
            return Err(refused("expr", &why, &alias));
        };
        // A call of the wrong number of arguments is refused by the expression
        // judge below, in its own words.
        let site = crate::forge::expr::call_sites(written, &|name| name == alias)
            .ok()
            .and_then(|sites| sites.into_iter().next())
            .filter(|site| site.args.len() == value.params.len());
        if let Some(site) = site {
            let walked = self.walked_maps.borrow();
            let mut maps_named: Vec<&str> = Vec::new();
            for (slot, arg) in value.params.iter().zip(&site.args) {
                let name = match &arg.shape {
                    ArgShape::Name(name) => Some(name.as_str()),
                    _ => None,
                };
                if let StaticSlot::Map(map) = slot {
                    if map.inout {
                        let Some(name) = name else {
                            let argument = written[arg.span.clone()].trim();
                            return Err(refused(
                                "expr",
                                &format!(
                                    "each argument `{alias}` writes in place is the name of a map \
                                     or a set variable, and `{argument}` is not one"
                                ),
                                argument,
                            ));
                        };
                        self.scope.map_argument(map, name).map_err(|why| {
                            refused(
                                "expr",
                                &format!(
                                    "each argument `{alias}` writes in place is a map or a set \
                                     variable whose key and value are the parameter's: {why}"
                                ),
                                name,
                            )
                        })?;
                        if walked.iter().any(|walked_map| walked_map == name) {
                            return Err(refused(
                                "expr",
                                &format!(
                                    "a <foreach> around the call walks `{name}` as it was when \
                                     the loop began, and a call that wrote it in place would \
                                     change it under the walk: hand the algorithm a map no loop \
                                     around the call walks"
                                ),
                                name,
                            ));
                        }
                    }
                }
                if let Some(name) = name.filter(|name| self.scope.map_variable(name).is_some()) {
                    if maps_named.contains(&name) {
                        return Err(refused(
                            "expr",
                            &format!(
                                "no map or set is named twice in one call, since a map the \
                                 algorithm writes in place is no view another of its arguments \
                                 may hold: `{name}` is named twice"
                            ),
                            name,
                        ));
                    }
                    maps_named.push(name);
                }
            }
        }
        // What the call returns is taken as an `<assign>` takes that value, or
        // dropped; either way the call is the one expression that may call it.
        self.in_place_call_ok.set(true);
        let judged = if location.is_empty() {
            // An algorithm that returns nothing is no value of an expression,
            // so the call is judged where each way of making it answers what
            // the statement drops ([`crate::forge::type_ctx::statement_sig`]).
            let mut statement = ctx.clone();
            if value.ret.is_none() {
                if let Some(sig) = crate::forge::type_ctx::statement_sig(callee) {
                    for made in self
                        .scope
                        .state_calls()
                        .iter()
                        .filter(|made| made.alias == alias)
                    {
                        statement.insert_func(made.name.as_str(), sig.clone());
                    }
                }
            }
            self.whole_state_call_ok.set(true);
            let judged = self
                .expr(
                    &statement,
                    written,
                    action.spellings.get("expr"),
                    Expected::Hint(InferredType::Unknown),
                )
                .map(|_| ());
            self.whole_state_call_ok.set(false);
            judged
        } else if value.ret.is_none() {
            Err(refused(
                "location",
                &format!(
                    "location takes what the algorithm returns, and `{alias}` returns nothing: \
                     write the call with no location"
                ),
                location,
            ))
        } else {
            let assign = Action {
                action_type: "assign".to_string(),
                ..action.clone()
            };
            self.action(ctx, &assign, state)
        };
        self.in_place_call_ok.set(false);
        judged
    }

    /// An `<sce:put>` (docs/adr/0108, D3, D4 and D12): `target` a map or set
    /// variable, or a map or a set held at a key of one
    /// ([`Self::written_map`]); `key` a key of it ([`Self::key_of`]); `expr`
    /// the value put at the key, required of a map of a value of one entry and
    /// refused on a set, whose entries carry none: a scalar expression for a
    /// value of a number, a bool, a text or a byte string, a value of the enum
    /// for an enum, and the name of a record of the schema for a record, as
    /// `<sce:append>` takes a record. A value that is a map, a set or a list
    /// (D12 rule 4) is the empty one where `expr` is left out, which replaces
    /// the value at a key that is present, and otherwise the variable `expr`
    /// names, of exactly the value's type, copied whole. `outcome`, when
    /// written, is a variable of the standard put outcome
    /// ([`PUT_OUTCOME_DOCUMENT`]), what the inner map did for a put through a
    /// nested place (D12 rule 5).
    fn put(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::value_type::ValueType;
        let place = self.written_map(ctx, action, state)?;
        let target = action.location.trim();
        let spelling = action.spellings.get("expr");
        let written = action.expr.trim();
        let rule = |rule: String| {
            self.rule_at(
                format!("<sce:put target=\"{target}\">"),
                &rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            )
        };
        match place.place.map_value() {
            Some(value) if value.ty.is_container() => {
                let exact = written.is_empty()
                    || self
                        .scope
                        .variables
                        .iter()
                        .find(|v| v.id == written)
                        .and_then(crate::forge::value_path::variable_place)
                        .is_some_and(|held| held.ty == value.ty);
                if !exact {
                    return Err(rule(format!(
                        "a put into `{target}`, whose value is `{ty}`, names no expr to put the \
                         empty one, or a variable of exactly `{ty}`, copied whole \
                         (docs/adr/0108, D12 rule 4)",
                        ty = value.ty
                    )));
                }
            }
            Some(value) => {
                if written.is_empty() {
                    return Err(rule(format!(
                        "a put into the map `{target}` names the value it puts at the key: expr, \
                         a value of `{}`",
                        value.ty
                    )));
                }
                match &value.ty {
                    ValueType::Record { alias } => {
                        self.record_named("sce:put", "expr", written, spelling, alias, state)?;
                    }
                    ValueType::Scalar(crate::forge::model::SceType::Enum(reference)) => {
                        self.expr_of_enum(ctx, written, spelling, &reference.alias)?;
                    }
                    other => {
                        let slot = other
                            .scalar()
                            .map_or(InferredType::Unknown, InferredType::from_sce_type);
                        self.expr(ctx, written, spelling, Expected::Slot(slot))?;
                    }
                }
            }
            None if !written.is_empty() => {
                return Err(rule(format!(
                    "the set `{target}` holds keys that carry no value, so a put into it names the \
                     key alone and takes no expr"
                )));
            }
            None => {}
        }
        self.map_outcome(
            action,
            PUT_OUTCOME_DOCUMENT,
            "inserted, replaced, full",
            state,
        )
    }

    /// An `<sce:remove>` (docs/adr/0108, D3, D4 and D12): `target` a map or set
    /// variable, or a map or a set held at a key of one
    /// ([`Self::written_map`]), `key` a key of it ([`Self::key_of`]), no
    /// value, and `outcome`, when written, a variable of the standard remove
    /// outcome ([`REMOVE_OUTCOME_DOCUMENT`]).
    fn remove(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        self.written_map(ctx, action, state)?;
        let target = action.location.trim();
        let written = action.expr.trim();
        if !written.is_empty() {
            let spelling = action.spellings.get("expr");
            return Err(self.rule_at(
                format!("<sce:remove target=\"{target}\">"),
                &format!(
                    "a remove from `{target}` names the key of the entry it removes, and takes no \
                     expr"
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            ));
        }
        self.map_outcome(action, REMOVE_OUTCOME_DOCUMENT, "removed, absent", state)
    }

    /// The map or the set an `<sce:put>` or an `<sce:remove>` writes
    /// ([`Self::written_through`]), and its `key` judged as a key of it
    /// ([`Self::key_of`]). A `target` that starts at no map or set variable is
    /// refused naming the variables there are ([`Self::not_a_map`]), and one that
    /// reaches a value that is no map or set, naming that value's type.
    fn written_map(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<crate::forge::type_ctx::MapPlace<'_>, Located<ForgeError>> {
        let Some(place) = self.written_through(ctx, action, state) else {
            return Err(self.not_a_map(action, state));
        };
        let place = place?;
        let Some(key) = place.place.map_key() else {
            let element = action.action_type.trim_start_matches("sce_");
            let target = action.location.trim();
            let at = action.spellings.get("target");
            return Err(self.rule_at(
                format!("<sce:{element} target=\"{target}\">"),
                &format!(
                    "target names a map or a set, and `{target}` is a value of `{}`, which holds \
                     no entry at a key",
                    place.place.ty
                ),
                at.map(|s| s.row()),
                at.map(|s| s.col()),
                state,
                target,
            ));
        };
        self.key_of(
            ctx,
            action,
            &action.key,
            action.spellings.get("key"),
            &key,
            state,
        )?;
        Ok(place)
    }

    /// The place a statement's `target` writes through a map or set variable
    /// (docs/adr/0108, D12 rule 1, [`StaticScope::map_place`]): the variable
    /// itself, or a value held at a key of one and a field of a record held
    /// there, `m[k]`, `m[k][k2]`, `m[k].tags`. Each key on the way is judged
    /// as a key of the map it keys ([`Self::key_of`]), and none may read the
    /// variable the statement writes through: each key is evaluated once,
    /// before anything is written, as the key of a record's copy may not name
    /// the record being written. A step that reaches nothing a statement writes
    /// through (an index of a list, a key of a set) is refused, saying which.
    /// `None` for a `target` that starts at no map or set variable.
    fn written_through(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Option<Result<crate::forge::type_ctx::MapPlace<'_>, Located<ForgeError>>> {
        let element = action.action_type.trim_start_matches("sce_");
        let target = action.location.trim();
        let spelling = action.spellings.get("target");
        let refuse = |rule: &str| {
            self.rule_at(
                format!("<sce:{element} target=\"{target}\">"),
                rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                target,
            )
        };
        let place = match self.scope.map_place(target)? {
            Ok(place) => place,
            Err(why) => return Some(Err(refuse(&why))),
        };
        let root = place.variable.id.as_str();
        for step in &place.steps {
            let crate::forge::type_ctx::PlaceStep::Key { key, map } = step else {
                continue;
            };
            if names_identifier(key, root) {
                return Some(Err(refuse(&format!(
                    "the key `{key}` reads `{root}`, which the statement writes through: each \
                     key of a target is evaluated once, before anything is written, and reads \
                     no part of the variable written (docs/adr/0108, D12 rule 1): take the key \
                     into a variable first"
                ))));
            }
            let Some(keyed) = map.map_key() else {
                continue;
            };
            if let Err(refusal) = self.key_of(ctx, action, key, spelling, &keyed, state) {
                return Some(Err(refusal));
            }
        }
        Some(Ok(place))
    }

    /// The key `written` of a statement's entry, or of a step of its target,
    /// judged as a value of the key `key` of a map or a set (docs/adr/0108, D3)
    /// is, as an assignment to a place of the key's type is: a scalar
    /// expression for a number, a bool, a text or a byte string, a value of the
    /// enum for an enum, and the name of a record of the schema (a record
    /// variable, the item or key of a `<foreach>`, the payload) for a record,
    /// which nothing in an expression makes. A statement that names no key is
    /// refused at its `target`.
    fn key_of(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        written: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        key: &crate::forge::value_path::Place,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::value_type::ValueType;
        let element = format!("sce:{}", action.action_type.trim_start_matches("sce_"));
        let written = written.trim();
        if written.is_empty() {
            let target = action.location.trim();
            let at = action.spellings.get("target");
            return Err(self.rule_at(
                format!("<{element} target=\"{target}\">"),
                &format!(
                    "a <{element}> names the key of the entry it writes: key, a value of `{}`",
                    key.ty
                ),
                at.map(|s| s.row()),
                at.map(|s| s.col()),
                state,
                target,
            ));
        }
        match &key.ty {
            ValueType::Record { alias } => {
                self.record_named(&element, "key", written, spelling, alias, state)
            }
            ValueType::Scalar(crate::forge::model::SceType::Enum(reference)) => {
                self.expr_of_enum(ctx, written, spelling, &reference.alias)
            }
            other => {
                let slot = other
                    .scalar()
                    .map_or(InferredType::Unknown, InferredType::from_sce_type);
                self.expr(ctx, written, spelling, Expected::Slot(slot))
                    .map(|_| ())
            }
        }
    }

    /// The `outcome` of an `<sce:put>` or an `<sce:remove>` (docs/adr/0108,
    /// D4): none, or the name of a variable of the standard enum `document`,
    /// imported as any enum is, whose variants are `variants`. The statement
    /// writes what it did there, a full map among it, and raises nothing.
    fn map_outcome(
        &self,
        action: &Action,
        document: &str,
        variants: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let written = action.outcome.trim();
        if written.is_empty() {
            return Ok(());
        }
        let alias = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == written)
            .and_then(|v| v.value_type.as_ref())
            .and_then(crate::forge::model::AlgorithmValueType::scalar)
            .and_then(|t| match t {
                crate::forge::model::SceType::Enum(reference) => Some(reference.alias.as_str()),
                _ => None,
            });
        let standard = alias
            .and_then(|alias| self.enum_documents.get(alias))
            .is_some_and(|held| held.key == document);
        if standard {
            return Ok(());
        }
        let element = format!("sce:{}", action.action_type.trim_start_matches("sce_"));
        let named = match self.scope.variables.iter().find(|v| v.id == written) {
            Some(var) => format!(
                "`{written}` is a variable of `{}`",
                var.value_type
                    .as_ref()
                    .map(crate::forge::model::AlgorithmValueType::as_attr)
                    .unwrap_or_default()
            ),
            None => format!("`{written}` names no variable of the document"),
        };
        let spelling = action.spellings.get("outcome");
        Err(self.rule_at(
            format!("<{element} outcome=\"{written}\">"),
            &format!(
                "{named}, and outcome names a variable of the standard enum {document} \
                 ({variants}), imported as any enum is (<sce:import kind=\"enum\" \
                 src=\"{document}\" as=\"…\"/>) and declared sce:type=\"enum:…\""
            ),
            spelling.map(|s| s.row()),
            spelling.map(|s| s.col()),
            state,
            written,
        ))
    }

    /// A `<sce:lift>` (docs/adr/0087, section 2.1): `schema` an imported event
    /// schema whose fields are each one entry; `location` a variable of
    /// `record:R` or `optional<record:R>`, `R` the document `schema` names;
    /// `refusal` another variable, of `optional<record:L>`, `L` the standard
    /// refusal document; `text` the name of a bounded text, present where it is
    /// read; and the longest pointer into `R` no longer than the refusal's
    /// `path` holds. Each refusal is placed on the attribute that breaks it.
    fn lift(&self, ctx: &TypeCtx<'_>, action: &Action) -> Result<(), Located<ForgeError>> {
        use crate::forge::error::ValidationError;
        let located = |error: ForgeError, attr: &str| {
            let spelling = action.spellings.get(attr);
            Located::new(
                error,
                self.diag_label,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
            )
        };
        let variable = |name: &str| self.scope.variables.iter().find(|v| v.id == name);
        let type_of = |var: &Variable| {
            var.value_type
                .as_ref()
                .map(|t| t.as_attr())
                .unwrap_or_default()
        };

        // The schema: an event schema the document imports, or a union it
        // imports, whose value a lift puts into a union variable whole
        // (docs/adr/0092, section 6.1 item 5). A backend that holds no union
        // variable refuses the location's variable by its own name.
        let schema = action.lift_schema.trim();
        let union = self.type_scope.unions.contains_key(schema);
        if !union && !self.lift_schemas.contains_key(schema) {
            return Err(located(
                ValidationError::InvalidAttribute {
                    element: "<sce:lift>".to_string(),
                    attr: "schema".to_string(),
                    value: schema.to_string(),
                    allowed: self
                        .lift_schemas
                        .keys()
                        .chain(self.type_scope.unions.keys())
                        .cloned()
                        .collect(),
                }
                .into(),
                "schema",
            ));
        }
        // A schema one of whose fields holds a list or a record is read as any
        // other is (docs/adr/0087, "Landing order" item 8): its depth and its
        // longest pointer are the type's, below. Whether a backend holds the
        // variable it is lifted into is the one table of what each lowers
        // ([`crate::forge::event_schema_check::ComposedLowering::records`]),
        // which refuses it by name where it does not.

        // The location: a variable of the record or the union, or of an
        // optional one.
        let location = action.location.trim();
        let kind = if union { "union" } else { "record" };
        let wanted = format!("{kind}:{schema} or optional<{kind}:{schema}>");
        let union_alias = |t: &crate::forge::model::AlgorithmValueType| -> Option<String> {
            match t {
                crate::forge::model::AlgorithmValueType::Union { alias } => Some(alias.clone()),
                _ => None,
            }
        };
        let location_fits = variable(location).is_some_and(|var| {
            let Some(t) = var.value_type.as_ref() else {
                return false;
            };
            if union {
                union_alias(t)
                    .or_else(|| t.optional_elem().and_then(union_alias))
                    .is_some_and(|alias| {
                        crate::forge::lift::same_union(&alias, schema, &self.type_scope)
                    })
            } else {
                t.record_alias()
                    .or_else(|| t.optional_elem().and_then(|elem| elem.record_alias()))
                    .is_some_and(|alias| {
                        crate::forge::lift::same_record(alias, schema, &self.type_scope)
                    })
            }
        });
        if !location_fits {
            let detail = match variable(location) {
                Some(var) => format!(
                    "names a variable of {}, and a lift replaces a variable of {wanted} whole",
                    type_of(var)
                ),
                None => format!(
                    "names no variable of the document, and a lift replaces a variable of \
                     {wanted} whole"
                ),
            };
            return Err(located(
                ValidationError::LiftLocation {
                    attr: "location".to_string(),
                    value: location.to_string(),
                    detail,
                }
                .into(),
                "location",
            ));
        }

        // The refusal: another variable, of the optional refusal record.
        let refusal = action.lift_refusal.trim();
        let refusal_wanted = format!(
            "optional<record:L>, L the document {}",
            crate::forge::lift::REFUSAL_DOCUMENT
        );
        let refusal_alias = variable(refusal)
            .and_then(|var| var.value_type.as_ref())
            .and_then(|t| t.optional_elem())
            .and_then(|elem| elem.record_alias())
            .filter(|alias| crate::forge::lift::is_refusal_record(alias, &self.type_scope));
        let refusal_detail = if refusal == location {
            Some(
                "names the variable `location` names too, and a lift writes the record and the \
                 refusal to two variables, one of them absent after it"
                    .to_string(),
            )
        } else if refusal_alias.is_some() {
            None
        } else {
            Some(match variable(refusal) {
                Some(var) => format!(
                    "names a variable of {}, and a lift keeps its refusal in a variable of \
                     {refusal_wanted}",
                    type_of(var)
                ),
                None => format!(
                    "names no variable of the document, and a lift keeps its refusal in a \
                     variable of {refusal_wanted}"
                ),
            })
        };
        if let Some(detail) = refusal_detail {
            return Err(located(
                ValidationError::LiftLocation {
                    attr: "refusal".to_string(),
                    value: refusal.to_string(),
                    detail,
                }
                .into(),
                "refusal",
            ));
        }
        let refusal_alias = refusal_alias.expect("the refusal was held to its record above");

        // The text: the name of a bounded text, present where it is read.
        let text = action.lift_text.trim();
        let lift_text = |detail: String| {
            located(
                ValidationError::LiftText {
                    text: text.to_string(),
                    detail,
                }
                .into(),
                "text",
            )
        };
        let named = crate::forge::expr::parse_to_ast(text)
            .ok()
            .and_then(|ast| crate::forge::expr::dotted_path(&ast));
        if named.is_none() {
            return Err(lift_text(
                "is not a name, and a lift reads the text a variable, a field of a record \
                 variable or a field of the event's payload holds, in place"
                    .to_string(),
            ));
        }
        let ty = self.expr(
            ctx,
            text,
            action.spellings.get("text"),
            Expected::Hint(InferredType::Unknown),
        )?;
        match ty {
            InferredType::Str => {}
            InferredType::Optional(crate::forge::types::OptionalElem::Str) => {
                return Err(lift_text(format!(
                    "names a text that may be absent: test it first (`{text} !== null`) and \
                     lift its present value (`{text}.value`), or lift a text that is always \
                     present"
                )));
            }
            other => {
                return Err(lift_text(format!(
                    "names a value of {}, and a lift reads a text: a value of string[N]",
                    self.spelled_type(ctx, text, other)
                )));
            }
        }
        if self.text_bound(text).is_none() {
            return Err(lift_text(
                "names a text with no bound, and a lift reads a text of string[N]".to_string(),
            ));
        }

        // Every refusal names a place the schema declares, so the refusal's path
        // holds each one whole when it holds the longest.
        let longest = crate::forge::lift::longest_pointer(schema, &self.type_scope);
        let bound = crate::forge::lift::path_bound(refusal_alias, &self.type_scope);
        if let (Ok((pointer, length)), Some(bound)) = (longest, bound) {
            if length > u64::from(bound) {
                return Err(located(
                    ValidationError::LiftPathBound {
                        schema: schema.to_string(),
                        pointer,
                        length,
                        bound,
                    }
                    .into(),
                    "schema",
                ));
            }
        }
        Ok(())
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
        records: &std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    ) -> Result<(), Located<ForgeError>> {
        // A record that holds a composition is read whole from the imports, as
        // no record of entries describes it. An alias the parser admitted names
        // an import; without sibling files to read it has no schema here, and
        // nothing to judge against.
        let Some(schema) = records.get(alias) else {
            return match self.lift_schemas.get(alias) {
                Some(composed) => self.composed_record(ctx, var, alias, composed),
                None => Ok(()),
            };
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
        self.schema_enums_closed(var, alias, schema)?;
        self.schema_enums_named(var, alias, schema)?;
        self.schema_strings_bounded(var, alias, schema)?;
        self.record_starts(ctx, schema, ordered)
    }

    /// A `record:<alias>` variable whose record holds a composition, built where
    /// it is declared by definite assignment (docs/adr/0087, "Decided while
    /// landing item 8", decision 10): each field of one entry given by one
    /// `<sce:set>`, judged as a field of a record of entries is
    /// ([`Self::record_starts`]), and each composed field starting at the one
    /// value its type has without a writer ([`SchemaField::start`]): absent for
    /// an optional one, empty for a list of no lower count. A `<sce:set>` of a
    /// composed field is refused by name, since no expression of the language
    /// builds a list, a union or a record, and so is a field with no such
    /// value, naming the repair: `optional<record:R>`, which starts absent and
    /// a lift writes whole.
    ///
    /// [`SchemaField::start`]: crate::forge::model::SchemaField::start
    fn composed_record(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        schema: &crate::forge::model::EventSchemaModel,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::model::FieldStart;
        let written_type = var
            .value_type
            .as_ref()
            .map(|t| t.as_attr())
            .unwrap_or_default();
        let element = format!("<data id=\"{}\" sce:type=\"{written_type}\">", var.id);
        // A `<sce:set>` that names a composed field first: it names no field a
        // `<sce:set>` gives, and the reason is the language's, not a typo. One
        // of a field with no start is refused below with the variable, which
        // names the repair.
        for init in &var.record_fields {
            let Some(field) = schema.fields.iter().find(|f| f.id == init.name) else {
                continue;
            };
            let starts = match field.start() {
                FieldStart::Given | FieldStart::Unstarted(_) => continue,
                FieldStart::Absent => "absent",
                FieldStart::Empty => "empty",
            };
            let spelling = init.name_spelling.as_ref();
            return Err(self.rule_at(
                format!("<sce:set name=\"{}\" expr=\"{}\">", init.name, init.expr),
                &format!(
                    "the field `{}` of record:{alias} holds `{}`, and no expression of the \
                     language builds a list, a union or a record: it starts {starts} where \
                     the record is declared, and a lift writes the record whole",
                    field.id, field.sce_type
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &init.name,
            ));
        }
        // A composed field with no value without a writer leaves the record
        // nothing to start at: no type has a zero (docs/adr/0099).
        if let Some((field, what)) = schema.fields.iter().find_map(|field| match field.start() {
            FieldStart::Unstarted(what) => Some((field, what)),
            FieldStart::Given | FieldStart::Absent | FieldStart::Empty => None,
        }) {
            let spelling = var.value_type_spelling.as_ref();
            return Err(self.rule_at(
                element,
                &format!(
                    "the field `{}` of record:{alias} holds `{}`, {what}, which has no value \
                     before something writes one, and no <sce:set> builds one: a record is \
                     built where it is declared only when each of its composed fields is \
                     optional or a list of no lower count. Declare \
                     optional<record:{alias}>, which starts absent and a lift writes whole",
                    field.id, field.sce_type
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &written_type,
            ));
        }
        // Every other field holds one entry, and is given as a field of a
        // record of entries is.
        let given = schema.given_fields();
        let declared: Vec<&str> = given.fields.iter().map(|f| f.id.as_str()).collect();
        let ordered = crate::forge::generator::order_record_fields(
            &var.record_fields,
            &declared,
            alias,
            format!("<data id=\"{}\">", var.id),
            "sce:type",
            &written_type,
            var.value_type_spelling.as_ref(),
        )
        .map_err(|error| Located::in_file(error, self.diag_label))?;
        self.schema_enums_closed(var, alias, &given)?;
        self.schema_enums_named(var, alias, &given)?;
        self.schema_strings_bounded(var, alias, &given)?;
        self.record_starts(ctx, &given, ordered)
    }

    /// Each field of a record built whole starts at the value its `<sce:set>`
    /// writes (`ordered`, in the schema's order), judged against the field's
    /// type: a record variable's, and the record of the case a union variable
    /// is built in.
    fn record_starts(
        &self,
        ctx: &TypeCtx<'_>,
        schema: &crate::forge::model::EntrySchema,
        ordered: Vec<&crate::forge::model::RecordFieldInit>,
    ) -> Result<(), Located<ForgeError>> {
        for (field, init) in schema.fields.iter().zip(ordered) {
            use crate::forge::model::{EntryType, SceType};
            // A field of an enum starts at one of its variants, as a variable
            // of it does.
            match &field.sce_type {
                EntryType::Scalar(SceType::Enum(reference)) => {
                    self.expr_of_enum(
                        ctx,
                        &init.expr,
                        init.expr_spelling.as_ref(),
                        &reference.alias,
                    )?;
                    continue;
                }
                EntryType::Optional {
                    elem: SceType::Enum(reference),
                } => {
                    self.expr_of_optional_enum(
                        ctx,
                        &init.expr,
                        init.expr_spelling.as_ref(),
                        &reference.alias,
                    )?;
                    continue;
                }
                EntryType::Scalar(_) | EntryType::Optional { .. } => {}
            }
            self.expr(
                ctx,
                &init.expr,
                init.expr_spelling.as_ref(),
                Expected::Slot(InferredType::of_field(&field.sce_type)),
            )?;
            // A string field starts at a literal that fits its bound; one
            // that declares none was refused above. An optional one starts
            // absent, or at such a literal (docs/adr/0055).
            let holder = match &field.sce_type {
                EntryType::Scalar(SceType::String) => Some(StringHolder::STRING_FIELD),
                EntryType::Scalar(SceType::Bytes) => Some(StringHolder::BYTES_FIELD),
                EntryType::Optional {
                    elem: SceType::String,
                } if !is_null_literal(&init.expr) => Some(StringHolder::STRING_FIELD),
                EntryType::Optional {
                    elem: SceType::Bytes,
                } if !is_null_literal(&init.expr) => Some(StringHolder::BYTES_FIELD),
                EntryType::Scalar(_) | EntryType::Optional { .. } => None,
            };
            if let (Some(holder), Some(max_size)) = (holder, field.max_size) {
                self.string_field_start(
                    format!("<sce:set name=\"{}\" expr=\"{}\">", field.id, init.expr),
                    init,
                    max_size,
                    holder,
                )?;
            }
        }
        Ok(())
    }

    /// An `optional<record:<alias>>` variable (docs/adr/0055, step 9). It starts
    /// absent: a record is built whole from one `<sce:set>` per field, which a
    /// variable that may hold none has no use for, so its `expr` is `null` and a
    /// record is put into it by name where the machine runs. What its schema holds
    /// is held as a record variable's is: each enum field of a closed enum, each
    /// text field bounded.
    fn optional_record(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        records: &std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = var.expr_spelling.as_ref();
        if !is_null_literal(&var.expr) {
            return Err(self.rule_at(
                format!(
                    "<data id=\"{}\" sce:type=\"{}\" expr=\"{}\">",
                    var.id,
                    var.value_type
                        .as_ref()
                        .map(|t| t.as_attr())
                        .unwrap_or_default(),
                    var.expr
                ),
                "an optional record starts absent, so its expr is `null`: a record is built \
                 whole from one <sce:set> per field, which a variable that may hold none has \
                 no use for, and a record is put into it by name where the machine runs",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                var.expr.trim(),
            ));
        }
        if let Some(schema) = records.get(alias) {
            self.schema_enums_closed(var, alias, schema)?;
            self.schema_strings_bounded(var, alias, schema)?;
        }
        self.expr(
            ctx,
            &var.expr,
            var.expr_spelling.as_ref(),
            Expected::Slot(variable_type(var)),
        )?;
        Ok(())
    }

    /// A variable whose type holds a map or a set at any depth
    /// (docs/adr/0108): every key is judged against the records the document
    /// imports, through every record a field names, and one that is no key is
    /// refused on the type (`validation/map-key-type`, D1), what the reader of
    /// the type could not ask without the records.
    ///
    /// A variable that is itself a map or a set, starting empty (D3), is then
    /// accepted, its reads, statements and loop judged where they are written,
    /// when its value is one an entry's read reads: a number, a bool, an enum,
    /// a text, a byte string or a record, and (D12) a list of numbers, bools or
    /// records, a set, or a map of such a value
    /// ([`crate::forge::model::AlgorithmValueType::held_at_a_key`]), each record
    /// of its keys and values closed and bounded as a list's records are. A map
    /// held inside another type (a list's element, an optional value) and a map
    /// of any other value are the language's, and no judgment reads them yet:
    /// each is refused by name (`generate/unsupported-feature`, D10). Whether a
    /// lane generates a map is that lane's to say
    /// ([`crate::forge::event_schema_check::ComposedLowering::maps`]), and a map
    /// of a nested value
    /// ([`crate::forge::event_schema_check::ComposedLowering::map_values`]).
    ///
    /// `Ok(true)` for a map or set variable accepted here, `Ok(false)` for a
    /// type that holds none.
    fn map_variable(
        &self,
        var: &Variable,
        ty: &crate::forge::model::AlgorithmValueType,
        records: &std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    ) -> Result<bool, Located<ForgeError>> {
        let Some(keyed) = ty.keyed() else {
            return Ok(false);
        };
        let spelling = var.value_type_spelling.as_ref();
        let (line, col) = (spelling.map(|s| s.row()), spelling.map(|s| s.col()));
        let element = format!("<data id=\"{}\">", var.id);
        if let Some(refusal) = crate::forge::value_type::map_key_refusal(ty, &self.type_scope) {
            // The type as the document spells it, the record's `actual`.
            let value = spelling
                .and_then(|s| s.value())
                .map_or_else(|| ty.as_attr(), |written| written.text.to_string());
            return Err(Located::new(
                crate::forge::error::ValidationError::MapKeyType {
                    element,
                    attr: "sce:type".into(),
                    value,
                    rule: refusal.rule(),
                }
                .into(),
                self.diag_label,
                line,
                col,
            ));
        }
        let unread = |sentence: String| {
            Located::new(
                crate::forge::error::GenerateError::unsupported(format!("{element}: {sentence}"))
                    .into(),
                self.diag_label,
                line,
                col,
            )
        };
        // A map inside another type is no variable a statement names.
        let (key, value) = match ty {
            crate::forge::model::AlgorithmValueType::Map { key, value, .. } => {
                (&**key, Some(&**value))
            }
            crate::forge::model::AlgorithmValueType::Set { key, .. } => (&**key, None),
            _ => {
                return Err(unread(crate::forge::value_type::map_unlowered(
                    &keyed.to_string(),
                )))
            }
        };
        if let Some(value) = value.filter(|v| !v.held_at_a_key()) {
            return Err(unread(format!(
                "`{value}` as the value of `{ty}`: a map's entry is read here when its value is \
                 a number, a bool, an enum, a text, a byte string, a record, a list of numbers, \
                 bools or records, a set, or a map of such a value ({}), and a map of any other \
                 value is a type of the language no judgment or lowering holds yet ({})",
                crate::forge::value_type::MAP_VALUE_LANDING,
                crate::forge::value_type::MAP_LANDING
            )));
        }
        // A record key or value is held to what a list's record element is:
        // the enums it holds closed and the texts it holds bounded, at every
        // depth of a map held at a key (docs/adr/0108, D12).
        let mut aliases = vec![key.record_alias()];
        let mut held = value;
        while let Some(at) = held {
            aliases.push(at.record_alias());
            aliases.push(at.list_elem().and_then(|elem| elem.record_alias()));
            aliases.push(at.map_key().and_then(|inner| inner.record_alias()));
            held = at.map_value();
        }
        for alias in aliases.into_iter().flatten() {
            if let Some(schema) = records.get(alias) {
                self.schema_enums_closed(var, alias, schema)?;
                self.schema_strings_bounded(var, alias, schema)?;
            }
        }
        Ok(true)
    }

    /// A value the deployment supplies (docs/adr/0109, D12), read when the
    /// machine starts from the JSON its deployment's configuration is written
    /// in, by the reader of its type: so its type states every bound, and is
    /// one a payload schema states (docs/adr/0087, section 1), which is what
    /// the deployment's value is checked by where the deployment is read. A
    /// map or a set, which has no wire form, was refused before this.
    fn deployment_value(&self, var: &Variable) -> Result<(), Located<ForgeError>> {
        let spelling = var.value_type_spelling.as_ref();
        let (line, col) = (spelling.map(|s| s.row()), spelling.map(|s| s.col()));
        let written = var
            .value_type
            .as_ref()
            .map(|t| t.as_attr())
            .unwrap_or_default();
        let refuse = |rule: String| {
            self.rule_at(
                format!("<data id=\"{}\" sce:from=\"deployment\">", var.id),
                &rule,
                line,
                col,
                "",
                &written,
            )
        };
        let Some(ty) = var.value_type.clone().and_then(|t| t.bounded().ok()) else {
            return Err(refuse(
                "a value the deployment supplies states every bound of its type, which its \
                 reader holds the deployment's value to"
                    .to_string(),
            ));
        };
        let mut defs = crate::forge::value_type::SchemaDefs::default();
        crate::forge::value_type::json_schema_of(&ty, &self.type_scope, &mut defs)
            .map(|_| ())
            .map_err(|why| {
                refuse(format!(
                    "a value the deployment supplies is read from JSON by the reader of its \
                     type and checked by the type's payload schema, and `{written}` has none: \
                     {why}"
                ))
            })
    }

    /// docs/adr/0109, D12: the refusal of a statement that writes `location`,
    /// the attribute `attr` of `element`, written at `line` and `col`, where it
    /// names a value the deployment supplies or a place inside one, which the
    /// machine reads and never writes. `None` for a location rooted at any
    /// other name.
    fn deployment_value_written(
        &self,
        element: &str,
        attr: &str,
        location: &str,
        line: Option<u32>,
        col: Option<u32>,
    ) -> Option<Located<ForgeError>> {
        let location = location.trim();
        let root = location.split(['.', '[']).next().unwrap_or("").trim();
        let value = self
            .scope
            .variables
            .iter()
            .find(|v| v.from_deployment && !root.is_empty() && v.id == root)?;
        Some(Located::new(
            crate::forge::error::ValidationError::DeploymentValueAssigned {
                element: element.to_string(),
                attr: attr.to_string(),
                location: location.to_string(),
                value: value.id.clone(),
            }
            .into(),
            self.diag_label,
            line,
            col,
        ))
    }

    /// Each place `action` writes itself, as the attribute that names it,
    /// held to [`Self::deployment_value_written`]: what an `<assign>`, an
    /// `<sce:append>`, an `<sce:clear>`, an `<sce:render>` and an
    /// `<sce:lift>` replace, the id a `<send>` writes, and the item and index
    /// a `<foreach>` binds.
    fn deployment_values_unwritten(&self, action: &Action) -> Result<(), Located<ForgeError>> {
        let places = places_written(action);
        for (element, attr, location) in places {
            let spelling = action.spellings.get(attr);
            if let Some(refusal) = self.deployment_value_written(
                element,
                attr,
                location,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
            ) {
                return Err(refusal);
            }
        }
        Ok(())
    }

    /// A `union:<alias>` variable (docs/adr/0092, section 6.1 item 5, and its
    /// amendment of 2026-11-07): a sum type has no default value, so the
    /// variable is built at its declaration, from one `<sce:set>` naming the
    /// tag with a variant literal of the tag's enum and one naming each field
    /// of that case's record, exactly once, and takes no `expr`. One with no
    /// `<sce:set>` is refused on its `sce:type`, naming both repairs: build a
    /// case, or declare `optional<union:<alias>>`, which starts absent. The
    /// case's record is held to what a record variable of it is.
    fn union_variable(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
        alias: &str,
        records: &std::collections::BTreeMap<String, crate::forge::model::EntrySchema>,
    ) -> Result<(), Located<ForgeError>> {
        // An alias the parser admitted names an import; without sibling files
        // to read it has no document here, and nothing to judge against.
        let Some(union) = self.type_scope.unions.get(alias) else {
            return Ok(());
        };
        let written_type = var
            .value_type
            .as_ref()
            .map(|t| t.as_attr())
            .unwrap_or_default();
        let element = format!("<data id=\"{}\" sce:type=\"{written_type}\">", var.id);
        let type_spelling = var.value_type_spelling.as_ref();
        if !var.expr.trim().is_empty() {
            let spelling = var.expr_spelling.as_ref();
            return Err(self.rule_at(
                element,
                "a union variable is built whole from <sce:set> children, one naming the tag \
                 and one naming each field of its case, and takes no expr: no expression \
                 makes a union",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                var.expr.trim(),
            ));
        }
        let tag_alias = crate::forge::value_path::tag_enum_alias(&self.type_scope, union);
        let literal = format!(
            "{}.<variant>",
            tag_alias.as_deref().unwrap_or(union.tag_enum.as_str())
        );
        if var.record_fields.is_empty() {
            return Err(self.rule_at(
                element,
                &format!(
                    "a union has no default value: no case is the zero of the others. Build \
                     one case where it is declared (<sce:set name=\"{}\" expr=\"{literal}\"/> \
                     and one <sce:set> per field of that case), or declare \
                     optional<union:{alias}>, which starts absent",
                    union.tag
                ),
                type_spelling.map(|s| s.row()),
                type_spelling.map(|s| s.col()),
                "",
                &written_type,
            ));
        }
        let mut tags = var.record_fields.iter().filter(|f| f.name == union.tag);
        let Some(tag) = tags.next() else {
            return Err(self.rule_at(
                element,
                &format!(
                    "a union is built from one <sce:set name=\"{}\" expr=\"{literal}\"/> that \
                     names its case, and none of its <sce:set> names the tag `{}`",
                    union.tag, union.tag
                ),
                type_spelling.map(|s| s.row()),
                type_spelling.map(|s| s.col()),
                "",
                &written_type,
            ));
        };
        if let Some(again) = tags.next() {
            let spelling = again.name_spelling.as_ref();
            return Err(self.rule_at(
                format!("<sce:set name=\"{}\" expr=\"{}\">", again.name, again.expr),
                &format!(
                    "the tag `{}` is given once: a union holds one case",
                    union.tag
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &again.name,
            ));
        }
        let Some(case) =
            crate::forge::value_path::case_of_tag_literal(&self.type_scope, union, &tag.expr)
        else {
            let spelling = tag.expr_spelling.as_ref();
            let rule = match &tag_alias {
                Some(_) => format!(
                    "the tag of a union is a variant of its enum written as a literal \
                     ({literal}, one of {}): the build must know the case to know which \
                     fields follow",
                    union
                        .cases
                        .iter()
                        .map(|case| case.variant.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                None => format!(
                    "the tag of a union is a variant of its enum written as a literal, and \
                     the statechart imports no enum of the tag's: import the enum the union \
                     `{}` tags its cases with",
                    union.name
                ),
            };
            return Err(self.rule_at(
                format!("<sce:set name=\"{}\" expr=\"{}\">", tag.name, tag.expr),
                &rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                tag.expr.trim(),
            ));
        };
        let fields: Vec<crate::forge::model::RecordFieldInit> = var
            .record_fields
            .iter()
            .filter(|f| f.name != union.tag)
            .cloned()
            .collect();
        if case.record.is_none() {
            return match fields.first() {
                None => Ok(()),
                Some(stray) => {
                    let spelling = stray.name_spelling.as_ref();
                    Err(self.rule_at(
                        format!("<sce:set name=\"{}\" expr=\"{}\">", stray.name, stray.expr),
                        &format!(
                            "the case `{}` of union:{alias} is empty, its tag alone: no field \
                             follows it",
                            case.variant.name
                        ),
                        spelling.map(|s| s.row()),
                        spelling.map(|s| s.col()),
                        "",
                        &stray.name,
                    ))
                }
            };
        }
        let held = crate::forge::value_path::case_record_alias(&self.type_scope, union, case);
        let Some((record, schema)) = held
            .as_deref()
            .and_then(|record| records.get_key_value(record))
        else {
            let spelling = tag.expr_spelling.as_ref();
            return Err(self.rule_at(
                format!("<sce:set name=\"{}\" expr=\"{}\">", tag.name, tag.expr),
                &format!(
                    "the case `{}` holds a record the statechart builds as a record variable \
                     of it is built: import its event schema, one whose every field is one \
                     entry, as a record variable's is",
                    case.variant.name
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                tag.expr.trim(),
            ));
        };
        let declared: Vec<&str> = schema.fields.iter().map(|f| f.id.as_str()).collect();
        let ordered = crate::forge::generator::order_record_fields(
            &fields,
            &declared,
            record,
            element,
            "sce:type",
            &written_type,
            type_spelling,
        )
        .map_err(|error| Located::in_file(error, self.diag_label))?;
        self.schema_enums_closed(var, record, schema)?;
        self.schema_enums_named(var, record, schema)?;
        self.schema_strings_bounded(var, record, schema)?;
        self.record_starts(ctx, schema, ordered)
    }

    /// An `<assign>` that writes a union whole or writes its tag
    /// (docs/adr/0092, section 6.1 items 5 and 6); `None` for any other, a
    /// write of a case's field among them, which is judged as a write of a
    /// field is, where the read of it must stand.
    ///
    /// The tag alone is refused: it would leave the fields of the old case
    /// under the new tag, and the case changes by a whole assignment. A union
    /// variable takes `null` where it may be absent, and otherwise a union a
    /// place holds by its name (a variable, a payload field, a `<foreach>`
    /// item, the present value of an optional one) of the same union, or, into
    /// an optional one, an optional value of it; nothing in an expression
    /// makes a union.
    fn union_assign(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Option<Result<(), Located<ForgeError>>> {
        use crate::forge::value_type::ValueType;
        let location = action.location.trim();
        let at_location = action.spellings.get("location");
        if let Some((holder, member)) = location.rsplit_once('.') {
            let place = ctx.value_of(holder)?;
            let (union, _) = crate::forge::value_path::union_of(ctx.value_scope, &place)?;
            if member.trim() != union.tag {
                return None;
            }
            return Some(Err(self.rule_at(
                format!("<assign location=\"{location}\">"),
                &format!(
                    "the tag of a union is never written alone: it would leave the fields of \
                     the old case under the new tag. Change the case by a whole assignment of \
                     `{holder}`"
                ),
                at_location.map(|s| s.row()),
                at_location.map(|s| s.col()),
                state,
                location,
            )));
        }
        let var = self.scope.variables.iter().find(|v| v.id == location)?;
        let (alias, optional) = match var.value_type.as_ref()? {
            crate::forge::model::AlgorithmValueType::Union { alias } => (alias, false),
            crate::forge::model::AlgorithmValueType::Optional { elem } => match &**elem {
                crate::forge::model::AlgorithmValueType::Union { alias } => (alias, true),
                _ => return None,
            },
            _ => return None,
        };
        let at_expr = action.spellings.get("expr");
        let written = action.expr.trim();
        let refused = |rule: &str| {
            self.rule_at(
                format!("<assign location=\"{location}\" expr=\"{written}\">"),
                rule,
                at_expr.map(|s| s.row()),
                at_expr.map(|s| s.col()),
                state,
                written,
            )
        };
        if is_null_literal(written) {
            return Some(if optional {
                Ok(())
            } else {
                Err(refused(&format!(
                    "`{location}` holds union:{alias}, which is never absent: declare it \
                     optional<union:{alias}> to hold `null`"
                )))
            });
        }
        // ... or what an imported algorithm returns as a union of the same
        // document, the call being the whole of the right side (section 6.1
        // item 7), as a record variable takes a record one returns.
        if let Some(callee) = self
            .scope
            .whole_value_call(written)
            .filter(|c| StaticScope::returns_state(c))
        {
            return Some(self.state_from_call(ctx, action, var, callee));
        }
        if let Err(refusal) =
            self.expr(ctx, written, at_expr, Expected::Hint(InferredType::Unknown))
        {
            return Some(Err(refusal));
        }
        let takes = |held: &crate::forge::value_path::Place| -> bool {
            let from = match (&held.ty, optional) {
                (ValueType::Union { alias }, _) => alias,
                (ValueType::Optional { elem }, true) => match &**elem {
                    ValueType::Union { alias } => alias,
                    _ => return false,
                },
                _ => return false,
            };
            let base = &self.scope.type_scope;
            let names = ScopedNames {
                from: crate::forge::value_path::scope_at(base, &held.scope),
                to: Some(base),
            };
            crate::forge::value_type::TypeIdentity::same_union(&names, from, alias)
        };
        Some(match self.whole_place(ctx, written) {
            Some(held) if takes(&held) => Ok(()),
            _ => Err(refused(&format!(
                "a union is put into one whole from another value of union:{alias} by its \
                 name (a variable, a payload field, a <foreach> item, the present value of an \
                 optional one{}); nothing in an expression makes a union",
                if optional { ", or `null`" } else { "" }
            ))),
        })
    }

    /// An `optional<union:<alias>>` variable (docs/adr/0092, amendment of
    /// 2026-11-07): it starts absent, as an optional record does, so its `expr`
    /// is `null`, and a union is put into it whole where the machine runs.
    fn optional_union_variable(
        &self,
        ctx: &TypeCtx<'_>,
        var: &Variable,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = var.expr_spelling.as_ref();
        if !is_null_literal(&var.expr) {
            return Err(self.rule_at(
                format!(
                    "<data id=\"{}\" sce:type=\"{}\" expr=\"{}\">",
                    var.id,
                    var.value_type
                        .as_ref()
                        .map(|t| t.as_attr())
                        .unwrap_or_default(),
                    var.expr
                ),
                "an optional union starts absent, so its expr is `null`: a union is put into \
                 it whole where the machine runs",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                var.expr.trim(),
            ));
        }
        self.expr(
            ctx,
            &var.expr,
            var.expr_spelling.as_ref(),
            Expected::Slot(variable_type(var)),
        )?;
        Ok(())
    }

    /// The alias of the schema the optional record variable `name` is of, if
    /// `name` is one (docs/adr/0055, step 9).
    fn optional_record_alias(&self, name: &str) -> Option<&str> {
        self.scope
            .variables
            .iter()
            .find(|v| v.id == name.trim())
            .and_then(|v| v.value_type.as_ref())
            .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
    }

    /// An `<assign>` of a whole value to the optional record variable at the
    /// action's location, of the schema `alias` (docs/adr/0055, step 9): `null`
    /// makes it absent, and a record of its schema makes it present — by the name
    /// of a record variable or of an optional record of the same schema, a loop's
    /// record item, `_event.data` when the event's schema is the record's, the
    /// element of a list of its schema at an index, or what an imported algorithm
    /// returns as a record or an optional record of it, or a field of a record that
    /// holds a record or an optional record of it ([`Self::record_field_copy`]).
    /// Nothing in an expression makes a record, so any other value is refused,
    /// naming what is taken.
    fn optional_record_assign(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        alias: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        if is_null_literal(&action.expr) {
            return Ok(());
        }
        let location = action.location.trim();
        if let Some(callee) = self
            .scope
            .whole_value_call(&action.expr)
            .filter(|c| StaticScope::returns_state(c))
        {
            let target = self
                .scope
                .variables
                .iter()
                .find(|v| v.id == location)
                .expect("the location names an optional record variable");
            return self.state_from_call(ctx, action, target, callee);
        }
        if let Some((list, index)) = indexed_element(&action.expr) {
            return self.element_copy(ctx, action, alias, list, index, state);
        }
        let written = action.expr.trim();
        if let Some(held) = self.optional_record_alias(written) {
            if held == alias {
                return Ok(());
            }
            let spelling = action.spellings.get("expr");
            return Err(self.rule_at(
                "<assign>".to_string(),
                &format!(
                    "`{written}` is an optional record of the schema {held}, and `{location}` \
                     holds one of the schema {alias}"
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            ));
        }
        if let Some(judged) = self.record_field_copy(ctx, action, alias, true, state) {
            return judged;
        }
        self.record_of(
            "assign",
            &action.expr,
            action.spellings.get("expr"),
            alias,
            state,
        )
    }

    /// `<assign location="note" expr="_event.data.payload"/>`: the record variable,
    /// or the optional record variable (`optional`), of the schema `alias` at the
    /// action's location takes whole what a field of a record holds
    /// ([`record_field_place`]), copied as a list a place holds is taken whole
    /// (docs/adr/0086, section 6.1): the completion of a verification or an opening
    /// that names a record among them, whose `payload` (`plaintext`) is an
    /// `optional<record:R>` of the record `sce:payload` names (docs/adr/0109, D9).
    /// The field holds a record of the variable's schema, the same record document
    /// whichever alias names it where the field is declared; a field that may be
    /// absent goes only into an optional record, since a record variable always
    /// holds one. `None` for a value that is no such field, which the rule of a
    /// record named whole judges ([`Self::record_of`]).
    fn record_field_copy(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        alias: &str,
        optional: bool,
        state: &str,
    ) -> Option<Result<(), Located<ForgeError>>> {
        use crate::forge::value_type::ValueType;
        let written = action.expr.trim();
        let (_, held) = record_field_place(ctx, written)?;
        let spelling = action.spellings.get("expr");
        // Its names are judged as any expression's are.
        if let Err(refusal) = self.expr(
            ctx,
            written,
            spelling,
            Expected::Hint(InferredType::Unknown),
        ) {
            return Some(Err(refusal));
        }
        let location = action.location.trim();
        let refused = |rule: String| {
            self.rule_at(
                "<assign>".to_string(),
                &rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            )
        };
        let (ty, may_be_absent) = match held.ty {
            ValueType::Optional { elem } => (*elem, true),
            present => (present, false),
        };
        let record = crate::forge::value_path::Place {
            ty,
            scope: held.scope,
        };
        let base = &self.scope.type_scope;
        let same_record = crate::forge::value_path::record_identity(Some(base), &record)
            .is_some_and(|identity| {
                base.records
                    .get(alias)
                    .is_some_and(|wanted| wanted.identity == identity)
            });
        if !same_record {
            let named = self
                .statechart_record_alias(&record)
                .unwrap_or_else(|| record.ty.to_string());
            return Some(Err(refused(format!(
                "`{written}` holds a record of the schema {named}, and `{location}` holds one \
                 of the schema {alias}"
            ))));
        }
        if may_be_absent && !optional {
            return Some(Err(refused(format!(
                "`{written}` holds optional<record:{alias}>, which may be absent, and \
                 `{location}` holds record:{alias}, which always holds one: declare \
                 `{location}` optional<record:{alias}> to take it"
            ))));
        }
        Some(Ok(()))
    }

    /// A record of `schema` — a variable, or the element of a list — holds each enum
    /// field in the machine's own type for the enum document the schema imports,
    /// whether or not the document imports it (docs/adr/0054), and that type is a
    /// closed set: an enum that declares `sce:strict-variants="false"` admits values
    /// no variant names, which it has no member for and which no declared name
    /// crosses to an algorithm by. Refused where the record is declared, as a
    /// variable of such an enum is ([`Self::enum_variable`]).
    fn schema_enums_closed(
        &self,
        var: &Variable,
        alias: &str,
        schema: &crate::forge::model::EntrySchema,
    ) -> Result<(), Located<ForgeError>> {
        for field in &schema.fields {
            let crate::forge::model::SceType::Enum(reference) = field.sce_type.present_type()
            else {
                continue;
            };
            let Some(held) = self.held_enums.get(&reference.alias) else {
                continue;
            };
            if held.strict_variants {
                continue;
            }
            let spelling = var.value_type_spelling.as_ref();
            return Err(self.rule_at(
                format!("<data id=\"{}\"> of record:{alias}", var.id),
                &format!(
                    "the field `{}` of record:{alias} holds the enum `{}`, which declares \
                     sce:strict-variants=\"false\": a statechart holds a closed enum, and a \
                     value no variant names has no type to be held in",
                    field.id, held.name
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &reference.alias,
            ));
        }
        Ok(())
    }

    /// A record variable starts each field at the value its `<sce:set>` writes,
    /// which for an enum field is a variant, and a variant is named through an
    /// enum the document imports. The field holds the enum document its schema
    /// imports whether or not the document imports it too (docs/adr/0054), so a
    /// list of the records, or the payload, needs no import; a record variable
    /// does, and is refused here, before its `<sce:set>` is read as a value of an
    /// enum no name of the document's text stands for. Judged only when the
    /// imports were read: without sibling files there is no schema either.
    fn schema_enums_named(
        &self,
        var: &Variable,
        alias: &str,
        schema: &crate::forge::model::EntrySchema,
    ) -> Result<(), Located<ForgeError>> {
        for field in &schema.fields {
            let crate::forge::model::SceType::Enum(reference) = field.sce_type.present_type()
            else {
                continue;
            };
            if self.enums.iter().any(|e| e.alias == reference.alias) {
                continue;
            }
            let spelling = var.value_type_spelling.as_ref();
            return Err(self.rule_at(
                format!("<data id=\"{}\"> of record:{alias}", var.id),
                &format!(
                    "the field `{}` of record:{alias} holds the enum `{}`, which this document \
                     does not import: a record variable starts the field at a variant, and a \
                     variant is named through the document's own import of the enum document \
                     the schema imports (<sce:import kind=\"enum\">, under any alias). An enum \
                     is the document it is declared in, not the alias another document imports \
                     it under",
                    field.id, reference.alias
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                "",
                &reference.alias,
            ));
        }
        Ok(())
    }

    /// A `<log expr>`: its value is judged as whatever it is, since nothing is
    /// declared where it lands, and is then held to the values every backend
    /// writes as text ([`logged_alike`]). A value of any other type is refused
    /// for every target, where the value is written: the document is compiled
    /// once per target (docs/adr/0003), so a value one backend cannot log is
    /// not accepted for the rest.
    fn log_value(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("expr");
        let (ty, held_enum) = self.expr_held(
            ctx,
            &action.expr,
            spelling,
            Expected::Hint(InferredType::Unknown),
        )?;
        if logged_alike(ty, held_enum.is_some()) {
            return Ok(());
        }
        let written = action.expr.trim();
        // At the value as the document spells it, which stands on the
        // `<log>`'s own row; at the element where no spelling was recorded.
        let (line, col, observed) = match spelling.and_then(|s| s.value()) {
            Some(value) => (
                Some(value.row),
                Some(value.col),
                value.on_one_row().map(str::to_string),
            ),
            None => match action.source_location.as_ref() {
                Some(at) => (at.line, at.col, None),
                None => (None, None, None),
            },
        };
        Err(Located::new(
            ScxmlSemanticError::StaticLogUnloggableValue {
                expr: written.to_string(),
                ty: self.spelled_type(ctx, written, ty),
                state: state.to_string(),
                observed,
            }
            .into(),
            self.diag_label,
            line,
            col,
        ))
    }

    /// The type of the value `written` computes, as a `sce:type` spells it,
    /// every bound in its bracket (docs/adr/0086, section 1.2): the type the
    /// variable it names declares, the record a loop's item or the payload taken
    /// whole is of, the type of the field, element or present value a place
    /// holds at the path it writes, and the type it was judged to otherwise.
    fn spelled_type(&self, ctx: &TypeCtx<'_>, written: &str, ty: InferredType) -> String {
        if let Some(value_type) = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == written)
            .and_then(|v| v.value_type.as_ref())
        {
            return value_type.as_attr();
        }
        let record = |alias: &str| {
            format!(
                "{}{alias}",
                crate::forge::model::AlgorithmValueType::RECORD_PREFIX
            )
        };
        if let Some((_, alias)) = self
            .loop_records
            .borrow()
            .iter()
            .find(|(item, _)| item == written)
        {
            return record(alias);
        }
        if written == crate::forge::event_schema_check::EVENT_DATA_PATH {
            if let Some(alias) = self.payload_alias() {
                return record(&alias);
            }
        }
        // A field of a record, at any depth: the judged type has no bound, and
        // the place's type states every one its schema declares.
        if let Some(place) = self.whole_place(ctx, written) {
            return match self.statechart_record_alias(&place) {
                Some(alias) => record(&alias),
                None => place.ty.to_string(),
            };
        }
        ty.describe()
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
        if let Some(why) = unbounded_delay(&action.delayexpr, ctx) {
            return Err(self.rule_at(
                format!("delayexpr=\"{}\"", action.delayexpr),
                &format!(
                    "a `delayexpr` is a text the data model bounds, as every text a \
                     machine holds is, so an engine with no heap can hold it \
                     (docs/adr/0134): {why}"
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.delayexpr,
            ));
        }
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

    /// A `<send>`'s `targetexpr`: a string, the target the send is routed to,
    /// computed from the machine's fields when the send runs and chosen among
    /// the values the document declares as `sce:targets`
    /// (docs/adr/0005, decision 3) — a generated machine has no script engine to
    /// learn where it can send, so the routes it can ever use are written down. An
    /// expression with no declared set is refused where it is written, naming the
    /// attribute to add. The element takes one target, a written one or an
    /// expression, so a `target` beside it is refused.
    fn target_expr(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("targetexpr");
        if !action.target.trim().is_empty() {
            return Err(self.rule_at(
                format!("targetexpr=\"{}\"", action.targetexpr),
                "a <send> names its target as `target` or as `targetexpr`, and never as both",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.targetexpr,
            ));
        }
        if action.targets.is_empty() {
            return Err(self.rule_at(
                format!("targetexpr=\"{}\"", action.targetexpr),
                "a computed target is chosen among the routes the document declares, since \
                 this data model has no engine to learn them when the send runs: add \
                 `sce:targets` naming each value the expression can take",
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.targetexpr,
            ));
        }
        self.expr(
            ctx,
            &action.targetexpr,
            spelling,
            Expected::Slot(InferredType::Str),
        )?;
        Ok(())
    }

    /// A `<send>`'s `typeexpr`: a string, the Event I/O Processor the send is
    /// delivered through, computed from the machine's fields when the send runs
    /// and chosen among the types the document declares as `sce:types`
    /// (docs/adr/0005, decision 3). An expression with no declared set is refused
    /// where it is written, naming the attribute to add. The element takes one
    /// type, a written one or an expression, so a `type` beside it is refused; and
    /// a computed target beside a computed type is refused, since a target chosen
    /// among `#_` locations is no address for the HTTP processor the type may name.
    fn type_expr(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("typeexpr");
        let refuse = |rule: &str| {
            self.rule_at(
                format!("typeexpr=\"{}\"", action.typeexpr),
                rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                &action.typeexpr,
            )
        };
        if !action.send_type.trim().is_empty() {
            return Err(refuse(
                "a <send> names its type as `type` or as `typeexpr`, and never as both",
            ));
        }
        if !action.targetexpr.trim().is_empty() {
            return Err(refuse(
                "a <send> computes its type or its target, not both: a target chosen among \
                 `#_` locations is no address for the HTTP processor the type may name",
            ));
        }
        if action.types.is_empty() {
            return Err(refuse(
                "a computed type is chosen among the processors the document declares, since \
                 this data model has no engine to learn them when the send runs: add \
                 `sce:types` naming each value the expression can take",
            ));
        }
        self.expr(
            ctx,
            &action.typeexpr,
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
        let Some(variable) = self.idlocation_variable(&action.idlocation) else {
            return Err(refuse(IDLOCATION_NAMES_A_STRING_VARIABLE));
        };
        match variable.capacity() {
            Some(capacity) if capacity <= AUTO_SEND_ID_MAX_BYTES => Err(refuse(&format!(
                "the id a machine generates is up to {AUTO_SEND_ID_MAX_BYTES} UTF-8 bytes, \
                 and the variable it is written to declares an sce:capacity of {capacity}: \
                 a bound the id fits, with room for a C buffer's terminator, is at least {}",
                AUTO_SEND_ID_MAX_BYTES + 1
            ))),
            _ => Ok(()),
        }
    }

    /// The variable an `idlocation` (`location`, as written) names, when it is
    /// one the place an id is written to can be: a string variable the document
    /// declares, and not a record's field, a list's element, or what a
    /// `<foreach>` binds, which would be a copy no one reads. `None` otherwise,
    /// refused with [`IDLOCATION_NAMES_A_STRING_VARIABLE`]. One answer for a
    /// `<send>`'s and an `<invoke>`'s.
    fn idlocation_variable(&self, location: &str) -> Option<&crate::model::Variable> {
        let location = location.trim();
        let variable = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == location)
            .filter(|_| !self.loop_names.borrow().iter().any(|name| name == location));
        // An `optional<string>` takes the id as a string, which is present
        // (docs/adr/0055).
        let is_string = variable.and_then(|v| v.value_type.as_ref()).and_then(|t| {
            t.scalar()
                .or_else(|| t.optional_elem().and_then(|elem| elem.scalar()))
        }) == Some(&crate::forge::model::SceType::String);
        variable.filter(|_| is_string)
    }

    /// An `<invoke>`'s `idlocation`: the string variable the machine writes the
    /// id each start of a child session is started under to, `<state>.<start>.<id>`
    /// (docs/adr/0045), the id the child's events carry as `_event.invokeid`
    /// and a saved tree holds as the child's `invokeid`. Only a child session's
    /// start generates an id; a host-run invocation and a Mesh request are
    /// started under the one the build wrote, which `id` names.
    ///
    /// The variable is one the document declares, a string, and of a bound the
    /// longest id fits: the start count is a `u64`, up to twenty digits, and a
    /// C buffer holds a terminator beside it. The write then has nothing to
    /// fail at, so the machine needs no error to raise for it.
    fn invoke_idlocation(
        &self,
        invoke: &Invoke,
        state: &str,
        line: Option<u32>,
        col: Option<u32>,
    ) -> Result<(), Located<ForgeError>> {
        let base = invoke.base();
        let refuse = |rule: &str| {
            self.rule_at(
                format!("idlocation=\"{}\"", base.idlocation),
                rule,
                line,
                col,
                state,
                &base.idlocation,
            )
        };
        if !matches!(invoke, Invoke::Scxml(_) | Invoke::Hybrid(_)) {
            return Err(refuse(
                "an idlocation stores the id a start of a child session generates; a \
                 host-run invocation or a Mesh request is started under the id the build \
                 wrote, so write `id` where the document must name it",
            ));
        }
        // docs/adr/0109, D12: the id is written, and a value the deployment
        // supplies is never written.
        if let Some(refusal) =
            self.deployment_value_written("<invoke>", "idlocation", &base.idlocation, line, col)
        {
            return Err(refusal);
        }
        let Some(variable) = self.idlocation_variable(&base.idlocation) else {
            return Err(refuse(IDLOCATION_NAMES_A_STRING_VARIABLE));
        };
        let longest = invoke_id_max_bytes(state, &base.invoke_id);
        match variable.capacity() {
            Some(capacity) if capacity <= longest => Err(refuse(&format!(
                "the id a start of this invoke is started under is up to {longest} UTF-8 \
                 bytes (`{state}.<start>.{}`, the start a count of up to twenty digits), \
                 and the variable it is written to declares an sce:capacity of {capacity}: \
                 a bound the id fits, with room for a C buffer's terminator, is at least {}",
                base.invoke_id,
                longest + 1
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
    ///
    /// An expression that names no record is the one value the event carries,
    /// and is judged as a param's value is ([`WireParam::of_content`]).
    fn content_record(
        &self,
        ctx: &TypeCtx<'_>,
        content: &ContentExpr<'_>,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let ContentExpr {
            element,
            written,
            spelling,
            beside,
            carrier,
        } = *content;
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
            // Anything else is the one value the element carries: what the
            // expression computes is the event's data as it is, held to the
            // rule a param's value is.
            return self.wire_param(
                ctx,
                &WireParam::of_content(written, spelling),
                element,
                carrier,
                state,
            );
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
            self.wire_param(ctx, &pair, element, carrier, state)?;
        }
        Ok(())
    }

    /// The ids of the fields of the record `written` names, or `None` when it
    /// names no record: a record variable, a loop's record item, or the payload
    /// of the transition's event when that event declares a schema.
    fn content_fields(&self, written: &str) -> Option<Vec<String>> {
        let ids = |schema: &crate::forge::model::EntrySchema| {
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
        carrier: Carrier,
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
        // A value of no name is the `<content expr>` a `<send>` carries whole.
        let (construct, what) = if param.name.is_empty() {
            (
                format!("<content expr=\"{written}\"> of {element}"),
                "a <content expr>",
            )
        } else {
            (
                format!("<param name=\"{}\"> of {element}", param.name),
                "a <param>",
            )
        };
        if crate::forge::expr::references_event_data_lexically(written)
            && !self.payload_declared.get()
        {
            return Err(self.rule_at(
                construct,
                &format!(
                    "{what} of this data model is read from the machine's fields when its \
                     element runs; the triggering event's payload is read only in a transition \
                     on an event that declares a schema, so copy the value into a variable \
                     there first"
                ),
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
        // A whole union crosses only as a field of an event schema's union, by
        // its name (docs/adr/0092, section 6.1 item 7), which no lowering
        // spells yet: one carried here is refused by its type, for every
        // backend, rather than as a value of no wire spelling.
        if let Some(held) = self
            .whole_place(ctx, written)
            .filter(|place| holds_whole_union(&place.ty))
        {
            return Err(self.rule_at(
                construct,
                &format!(
                    "`{}` holds `{}`, and a whole union crosses only as a field of an event \
                     schema's union, which no backend lowers yet (docs/adr/0092, section 6.1 \
                     item 7): {what} carries its tag or a field its tests prove instead",
                    written.trim(),
                    held.ty
                ),
                line,
                col,
                state,
                written,
            ));
        }
        match self.wire_value_enum(ctx, written) {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {}
            Err(refusal) => {
                return Err(Located::in_file(
                    ExpressionSite::new(written, spelling).place(refusal),
                    self.diag_label,
                ))
            }
        }
        // An optional value crosses as JSON, `null` when absent and its value
        // when present; a carrier that is text alone has no spelling for an
        // absent one (docs/adr/0055, section 4).
        if let InferredType::Optional(elem) = ty {
            let rule = match carrier {
                Carrier::Text => Some(format!(
                    "{what} of {element} crosses as text alone, which has no spelling for an \
                     absent value, and `{}` is {}: read it with a default (`{} ?? <value>`), or \
                     carry its `.value` where a `!== null` test proves it present",
                    written.trim(),
                    ty.describe(),
                    written.trim()
                )),
                Carrier::Json => {
                    let crosses = match elem {
                        crate::forge::types::OptionalElem::Enum => matches!(
                            self.wire_value_held(ctx, written),
                            Ok(crate::forge::static_enum::Held::Optional(_))
                        ),
                        crate::forge::types::OptionalElem::Record => false,
                        present => present.present().wire_param_slot().is_some(),
                    };
                    (!crosses).then(|| {
                        format!(
                            "{what} crosses as a JSON value, `null` when absent and the value \
                             when present, and {} has no such spelling yet: an optional record, \
                             and an optional enum value of a loop's item, cross no value",
                            ty.describe()
                        )
                    })
                }
            };
            return match rule {
                None => Ok(()),
                Some(rule) => Err(self.rule_at(construct, &rule, line, col, state, written)),
            };
        }
        if ty.wire_param_slot().is_none() {
            return Err(self.rule_at(
                construct,
                &format!(
                    "{what} crosses as text and as a JSON value, which every \
                     backend spells alike for a bool, a string, a byte string (as its \
                     Latin-1 text), an integer (a 64-bit one as the text of its digits), a \
                     real, and an enum value held by a variable or by a field of a record \
                     variable (as the name its enum declares); a list, a record and an enum \
                     value of a loop's item have no such spelling yet"
                ),
                line,
                col,
                state,
                written,
            ));
        }
        Ok(())
    }

    /// The enum `written`, a value that crosses as data, is a value of.
    ///
    /// An enum value crosses as the name its enum declares for it, a string as a
    /// saved state holds one, so every backend spells it alike. Only a variable, a
    /// field of a record variable or a field of the payload is read that way: a
    /// value of a loop's record item has no lowering here yet.
    fn wire_value_enum(
        &self,
        ctx: &TypeCtx<'_>,
        written: &str,
    ) -> Result<Option<String>, crate::forge::expr::Refusal> {
        Ok(self.wire_value_held(ctx, written)?.present_enum())
    }

    /// [`Self::wire_value_enum`]'s reading of `written` as an enum value,
    /// present or optional ([`crate::forge::static_enum::Held`]).
    fn wire_value_held(
        &self,
        ctx: &TypeCtx<'_>,
        written: &str,
    ) -> Result<crate::forge::static_enum::Held, crate::forge::expr::Refusal> {
        let held_by_a_variable = |name: &str| self.enum_of_wire_value(name);
        let reading = crate::forge::static_enum::EnumReading {
            var_alias: &held_by_a_variable,
            calls: &|name| self.scope.call_enums(name),
        };
        crate::forge::static_enum::value_held(written, ctx, &reading)
    }

    /// A `<sce:render>` (docs/adr/0087, section 3.1, item 6): `template` a text
    /// template the document imports; `location` a `string[N]` variable whose
    /// bound is at least the template's `MAX_LEN`; and one `<sce:set>` for each
    /// hole and no other, each value fitting its hole (docs/adr/0086, section
    /// 5.1): a text of a bound at most the hole's, an integer of the hole's own
    /// type, a value of the enum document the hole's enum is. Each refusal is
    /// placed on the attribute that breaks it.
    fn render(&self, ctx: &TypeCtx<'_>, action: &Action) -> Result<(), Located<ForgeError>> {
        use crate::forge::error::ValidationError;
        use crate::forge::model::{HoleKind, SceType};
        let at =
            |error: ForgeError, spelling: Option<&crate::attribute_spelling::AttributeSpelling>| {
                Located::new(
                    error,
                    self.diag_label,
                    spelling.map(|s| s.row()),
                    spelling.map(|s| s.col()),
                )
            };
        let alias = action.render_template.trim();
        let Some(template) = self.text_templates.get(alias) else {
            return Err(at(
                ValidationError::InvalidAttribute {
                    element: "<sce:render>".to_string(),
                    attr: "template".to_string(),
                    value: alias.to_string(),
                    allowed: self.text_templates.keys().cloned().collect(),
                }
                .into(),
                action.spellings.get("template"),
            ));
        };

        // The location: a string variable, whose bound holds every rendering.
        let location = action.location.trim();
        let is_text = |var: &&Variable| {
            var.value_type
                .as_ref()
                .and_then(|t| t.scalar())
                .is_some_and(|s| matches!(s, SceType::String))
        };
        let bound = self
            .scope
            .variables
            .iter()
            .filter(is_text)
            .find(|v| v.id == location)
            .and_then(Variable::capacity);
        let Some(bound) = bound else {
            return Err(at(
                ValidationError::InvalidAttribute {
                    element: "<sce:render>".to_string(),
                    attr: "location".to_string(),
                    value: location.to_string(),
                    allowed: self
                        .scope
                        .variables
                        .iter()
                        .filter(is_text)
                        .map(|v| v.id.clone())
                        .collect(),
                }
                .into(),
                action.spellings.get("location"),
            ));
        };
        if u64::from(bound) < template.max_len {
            return Err(at(
                ValidationError::RenderBound {
                    template: alias.to_string(),
                    location: location.to_string(),
                    bound,
                    max_len: template.max_len,
                }
                .into(),
                action.spellings.get("location"),
            ));
        }

        // The holes: each given once, each value fitting its hole.
        let holes =
            |hole: &str,
             detail: String,
             spelling: Option<&crate::attribute_spelling::AttributeSpelling>| {
                at(
                    ValidationError::RenderHoles {
                        template: alias.to_string(),
                        hole: hole.to_string(),
                        detail,
                    }
                    .into(),
                    spelling,
                )
            };
        let mut given: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for set in &action.render_sets {
            let name = set.name.trim();
            let spelling = set.expr_spelling.as_ref();
            let Some(hole) = template.model.hole(name) else {
                let declared: Vec<&str> =
                    template.model.holes.iter().map(|h| h.id.as_str()).collect();
                return Err(holes(
                    name,
                    format!(
                        "is given by a <sce:set> and is no hole of the template: its holes are {}",
                        declared.join(", ")
                    ),
                    spelling,
                ));
            };
            if !given.insert(name) {
                return Err(holes(
                    name,
                    "is given twice: a <sce:render> gives each hole of its template once"
                        .to_string(),
                    spelling,
                ));
            }
            match &hole.kind {
                HoleKind::Text { max, .. } => {
                    let ty =
                        self.expr(ctx, &set.expr, spelling, Expected::Hint(InferredType::Str))?;
                    if ty != InferredType::Str {
                        return Err(holes(
                            name,
                            format!(
                                "is a text of string[{}], and is given a value of {}",
                                max.max,
                                self.spelled_type(ctx, &set.expr, ty)
                            ),
                            spelling,
                        ));
                    }
                    match self.text_bound(&set.expr) {
                        Some(given_bound) if given_bound <= max.max => {}
                        Some(given_bound) => {
                            return Err(holes(
                                name,
                                format!(
                                    "is a text of string[{}], and is given a text of up to \
                                     {given_bound} bytes: a value fits a hole whose bound is at \
                                     least its own (docs/adr/0086, section 5.1)",
                                    max.max
                                ),
                                spelling,
                            ));
                        }
                        None => {
                            return Err(holes(
                                name,
                                format!(
                                    "is a text of string[{}], and is given a text whose bound \
                                     nothing declares: give a variable, a field or a literal of \
                                     at most {} bytes",
                                    max.max, max.max
                                ),
                                spelling,
                            ));
                        }
                    }
                }
                HoleKind::Integer(integer) => {
                    let slot = InferredType::from_sce_type(integer);
                    let ty = self.expr(ctx, &set.expr, spelling, Expected::Hint(slot))?;
                    // A literal takes the hole's type where it holds the value,
                    // which the expression's own judge says of it in that slot.
                    if ty == InferredType::UntypedInt {
                        self.expr(ctx, &set.expr, spelling, Expected::Slot(slot))?;
                    } else if ty != slot {
                        return Err(holes(
                            name,
                            format!(
                                "is a {}, and is given a value of {}: a number fits only its own \
                                 type (docs/adr/0086, section 5.1)",
                                integer.as_attr(),
                                self.spelled_type(ctx, &set.expr, ty)
                            ),
                            spelling,
                        ));
                    }
                }
                HoleKind::Enum(reference) => {
                    let (_, held) = self.expr_held(
                        ctx,
                        &set.expr,
                        spelling,
                        Expected::Hint(InferredType::Unknown),
                    )?;
                    let same_document = held
                        .as_deref()
                        .and_then(|a| self.enum_documents.get(a))
                        .zip(template.enums.get(&hole.id))
                        .is_some_and(|(value, hole)| value.key == hole.key);
                    if !same_document {
                        let got = match &held {
                            Some(alias) => format!(
                                "a value of enum:{alias}, which is not the enum document the \
                                 template's is"
                            ),
                            None => "a value of no enum this document imports".to_string(),
                        };
                        return Err(holes(
                            name,
                            format!(
                                "is a value of enum:{} (as the template imports it), and is \
                                 given {got}",
                                reference.alias
                            ),
                            spelling,
                        ));
                    }
                }
            }
        }
        if let Some(missing) = template
            .model
            .holes
            .iter()
            .find(|h| !given.contains(h.id.as_str()))
        {
            return Err(holes(
                &missing.id,
                "is not given: a <sce:render> gives each hole of its template once, with one \
                 <sce:set>"
                    .to_string(),
                action.spellings.get("template"),
            ));
        }
        Ok(())
    }

    /// The most bytes the text or byte string `written` can hold, where a declaration
    /// states it: a literal's own length; a string or byte-string variable's
    /// `sce:capacity`; a field of a record variable, of the element a list of records
    /// is indexed at, of a `<foreach>`'s record item or of the payload, by the
    /// `sce:max-size` its schema declares; the larger of a conditional's two values;
    /// and the sum of a join's two. `None` for any other value (a call, a value whose
    /// length nothing declares), which a field holds to its bound where it arrives.
    fn text_bound(&self, written: &str) -> Option<u32> {
        let ast = crate::forge::expr::parse_to_ast(written).ok()?;
        self.node_text_bound(&ast)
    }

    /// [`Self::text_bound`] of one node of a parsed value.
    fn node_text_bound(&self, node: &crate::forge::expr::TypedExpr) -> Option<u32> {
        use crate::forge::expr::{BinOp, ExprKind};
        use crate::forge::model::SceType;
        let schema_field = |alias: &str, field: &str| -> Option<u32> {
            self.schemas
                .get(alias)?
                .fields
                .iter()
                .find(|f| f.id == field)?
                .max_size
        };
        let variable = |name: &str| self.scope.variables.iter().find(|v| v.id == name);
        match &node.kind {
            ExprKind::StringLit { value, .. } => u32::try_from(value.len()).ok(),
            ExprKind::Ident(name) => {
                let var = variable(name)?;
                // An optional text is bounded as its present value is
                // (docs/adr/0055).
                let value_type = var.value_type.as_ref()?;
                let present = value_type
                    .scalar()
                    .or_else(|| value_type.optional_elem().and_then(|elem| elem.scalar()));
                match present? {
                    SceType::String | SceType::Bytes => var.capacity(),
                    _ => None,
                }
            }
            ExprKind::Member { object, property } => {
                if let Some(path) = crate::forge::expr::dotted_path(node) {
                    if let Some(field) = path.strip_prefix("_event.data.") {
                        // The present value of an optional text of the payload is
                        // bounded as the field is (docs/adr/0055).
                        let field = field
                            .strip_suffix(&format!(".{}", crate::forge::expr::OPTIONAL_VALUE))
                            .unwrap_or(field);
                        if field.contains('.') {
                            return None;
                        }
                        let payload = self.payload_schema.borrow();
                        return payload
                            .as_ref()?
                            .fields
                            .iter()
                            .find(|f| f.id == field)?
                            .max_size;
                    }
                }
                match &object.kind {
                    // The present value of an optional text variable, `note.value`,
                    // is bounded as the variable is (docs/adr/0055).
                    ExprKind::Ident(holder) if property == crate::forge::expr::OPTIONAL_VALUE => {
                        let var = variable(holder)?;
                        match var.value_type.as_ref()?.optional_elem()?.scalar()? {
                            SceType::String | SceType::Bytes => var.capacity(),
                            _ => None,
                        }
                    }
                    ExprKind::Ident(holder) => {
                        if let Some(alias) = variable(holder)
                            .and_then(|v| v.value_type.as_ref())
                            .and_then(|t| t.record_alias())
                        {
                            return schema_field(alias, property);
                        }
                        let alias = self
                            .loop_records
                            .borrow()
                            .iter()
                            .find(|(item, _)| item == holder)
                            .map(|(_, alias)| alias.clone())?;
                        schema_field(&alias, property)
                    }
                    // A field of the present value of an optional record,
                    // `pick.value.title`, is bounded by its schema as a record
                    // variable's field is (docs/adr/0055, step 9).
                    ExprKind::Member {
                        object: record,
                        property: value,
                    } if value == crate::forge::expr::OPTIONAL_VALUE => {
                        let ExprKind::Ident(holder) = &record.kind else {
                            return None;
                        };
                        let alias = variable(holder)
                            .and_then(|v| v.value_type.as_ref())
                            .and_then(|t| t.optional_elem())
                            .and_then(|elem| elem.record_alias())?;
                        schema_field(alias, property)
                    }
                    ExprKind::Index { object: list, .. } => {
                        let ExprKind::Ident(list) = &list.kind else {
                            return None;
                        };
                        let alias = variable(list)?
                            .value_type
                            .as_ref()?
                            .list_elem()?
                            .record_alias()?;
                        schema_field(alias, property)
                    }
                    _ => None,
                }
            }
            ExprKind::Conditional {
                consequent,
                alternate,
                ..
            } => Some(
                self.node_text_bound(consequent)?
                    .max(self.node_text_bound(alternate)?),
            ),
            ExprKind::Binary {
                op: BinOp::Add,
                left,
                right,
            } => self
                .node_text_bound(left)?
                .checked_add(self.node_text_bound(right)?),
            _ => None,
        }
    }

    /// A `<param>` of a `<send>` that gives a field of a composed type, a list,
    /// a record or a union or an optional one, of the schema this document
    /// imports for the event (docs/adr/0086, section 5.2): it names a place
    /// that holds the value whole (a variable, a field of a record variable, or
    /// a part of the payload of the event the transition is on, forwarded),
    /// and is accepted when that place's type fits the field's (section 5.1),
    /// so the send checks nothing of it when it runs, or fits it but for the
    /// lower count of the list itself, which a list filled by appends cannot
    /// prove and the send holds when it runs (section 15.6,
    /// [`crate::forge::value_type::fits_but_for_count`]). A value that does not
    /// fit is refused naming the field, the schema that reads it and the first
    /// place, by path, where it may not fit; a value that names no place is
    /// judged as the expression it is, and refused when it is one.
    ///
    /// `false` for a param that gives no such field, which the rules of a
    /// scalar take ([`Self::wire_param`], [`Self::received`]).
    fn composed_param(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        param: &WireParam<'_>,
        state: &str,
    ) -> Result<bool, Located<ForgeError>> {
        use crate::forge::value_path::{scope_at, walk, Place, Step};
        let written = param.written;
        if action.event.is_empty()
            || param.is_static_literal
            || param.name.is_empty()
            || written.trim().is_empty()
        {
            return Ok(false);
        }
        let base = &self.scope.type_scope;
        // The field as the schema declares it, typed in the scope of the
        // schema's own document.
        let Some((schema, field)) = self
            .lift_schemas
            .iter()
            .find(|(_, schema)| schema.event_name == action.event)
            .and_then(|(alias, schema)| {
                let root = Place::of(crate::forge::value_type::ValueType::Record {
                    alias: alias.clone(),
                });
                walk(Some(base), &root, &[Step::Field(param.name)]).map(|field| (schema, field))
            })
            .filter(|(_, field)| field.is_composed())
        else {
            return Ok(false);
        };
        let (line, col) = match (param.source_location, param.spelling) {
            (Some(at), _) => (at.line, at.col),
            (None, Some(spelling)) => (Some(spelling.row()), Some(spelling.col())),
            (None, None) => (None, None),
        };
        let construct = format!(
            "<param name=\"{}\"> of <send event=\"{}\">",
            param.name, action.event
        );
        let reader = format!(
            "the schema `{}` this document declares `{}` by reads it as its field `{}`",
            schema.name, action.event, param.name
        );
        // A list variable is no value an expression reads, so its name alone
        // is the place; anything else is judged as the expression it is
        // first, its names and indices as any expression's are.
        if self.scope.list_read_as_value(written).is_none() {
            self.expr(
                ctx,
                written,
                param.spelling,
                Expected::Hint(InferredType::Unknown),
            )?;
        }
        let Some(place) =
            crate::forge::expr::written_place(written).and_then(|path| ctx.value_of(&path))
        else {
            return Err(self.rule_at(
                construct,
                &format!(
                    "{reader}, a `{}`: a <send> carries a composed value by the name of the \
                     place that holds it whole (a variable, a field of a record variable, or a \
                     part of the payload of the event the transition is on), and `{}` names no \
                     such place",
                    field.ty,
                    written.trim()
                ),
                line,
                col,
                state,
                written,
            ));
        };
        let names = ScopedNames {
            from: scope_at(base, &place.scope),
            to: scope_at(base, &field.scope),
        };
        // A list filled by appends fits but for its own lower count, which no
        // variable states (docs/adr/0086, sections 15.6 and 15.7): the send
        // holds that count when it runs, and the lowering writes the check
        // (`composed_wire_value`). Everything else is proven here or refused.
        crate::forge::value_type::fits_but_for_count(&place.ty, &field.ty, &names).map_err(
            |misfit| {
                self.rule_at(
                    construct,
                    &format!(
                        "{reader}, and {}: a delivery of it may not fit that schema, so the send \
                     is refused here rather than where it arrives",
                        misfit.sentence(written.trim())
                    ),
                    line,
                    col,
                    state,
                    written,
                )
            },
        )?;
        Ok(true)
    }

    /// A value a `<send>` carries under the name `param.name`, held to each field
    /// that reads it where the build knows the reader ([`Receivers`]): the field
    /// of the schema this document imports for the event, and the field of the
    /// schema each child the send is delivered to declares for it. A value of a
    /// kind the field cannot hold — a value of an enum, which crosses as its
    /// variant's declared name, where the field is a number — never fits the
    /// schema, and is refused here rather than each time it arrives.
    fn received(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        param: &WireParam<'_>,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::model::{EntryField, EnumModel, SceType};
        let written = param.written;
        if action.event.is_empty() || param.name.is_empty() || written.trim().is_empty() {
            return Ok(());
        }
        let mut readers: Vec<(String, &EntryField, Option<&EnumModel>)> = Vec::new();
        if let Some(schema) = self.receivers.own_schemas.get(&action.event) {
            if let Some(field) = schema.fields.iter().find(|f| f.id == param.name) {
                let held = match field.sce_type.present_type() {
                    SceType::Enum(reference) => self.receivers.own_enums.get(&reference.alias),
                    _ => None,
                };
                readers.push((
                    format!(
                        "the schema `{}` this document declares `{}` by",
                        schema.name, action.event
                    ),
                    field,
                    held,
                ));
            }
        }
        for (child, receives) in self.receivers.children_of(&action.target) {
            let Some(schema) = receives.schemas.get(&action.event) else {
                continue;
            };
            let Some(field) = schema.fields.iter().find(|f| f.id == param.name) else {
                continue;
            };
            let held = receives
                .enums
                .get(&action.event)
                .and_then(|fields| fields.get(&field.id));
            readers.push((
                format!(
                    "the schema `{}` the child `{child}` reads `{}` by",
                    schema.name, action.event
                ),
                field,
                held,
            ));
        }
        if readers.is_empty() {
            return Ok(());
        }
        // The value as it crosses: an enum value by the enum the sender holds it
        // in, anything else by its type. A value [`Self::wire_param`] refuses was
        // refused before this is asked.
        let value = match self.wire_value_held(ctx, written) {
            Ok(crate::forge::static_enum::Held::Enum(alias)) => SentValue::Enum {
                model: self.receivers.own_enums.get(&alias),
                alias,
                optional: false,
            },
            Ok(crate::forge::static_enum::Held::Optional(alias)) => SentValue::Enum {
                model: self.receivers.own_enums.get(&alias),
                alias,
                optional: true,
            },
            Ok(
                crate::forge::static_enum::Held::Neither | crate::forge::static_enum::Held::Null,
            ) => {
                let ty = self.expr(
                    ctx,
                    written,
                    param.spelling,
                    Expected::Hint(InferredType::Unknown),
                )?;
                let bound = if matches!(
                    ty.unwrap_optional(),
                    InferredType::Str | InferredType::Bytes
                ) {
                    self.text_bound(written)
                } else {
                    None
                };
                SentValue::Typed { ty, bound }
            }
            Err(_) => return Ok(()),
        };
        let (line, col) = match (param.source_location, param.spelling) {
            (Some(at), _) => (at.line, at.col),
            (None, Some(spelling)) => (Some(spelling.row()), Some(spelling.col())),
            (None, None) => (None, None),
        };
        for (reader, field, held) in readers {
            if let Some(misfit) = value.misfit(field, held) {
                let consequence = match misfit {
                    Misfit::Kind(why) => format!(
                        "{why}: no delivery of it fits that schema, so each would be refused \
                         where it arrives"
                    ),
                    Misfit::Bound(why) => format!(
                        "{why}: a delivery past it is refused where it arrives, so the narrowing \
                         is the sender's (assign the value to a variable declared with \
                         sce:capacity of {} or less, where an assignment past it fails as \
                         error.execution, and send that variable)",
                        field.max_size.unwrap_or_default()
                    ),
                };
                return Err(self.rule_at(
                    format!(
                        "<param name=\"{}\"> of <send event=\"{}\">",
                        param.name, action.event
                    ),
                    &format!(
                        "{reader} reads it as its field `{}`, and {consequence}",
                        field.id
                    ),
                    line,
                    col,
                    state,
                    written,
                ));
            }
        }
        Ok(())
    }

    /// A `<param>` of an `<invoke>` typed by `sce:request` (SCE Accepted Subset
    /// §2.12), held to the field of the record it supplies.
    ///
    /// The host was promised the whole record, so a value the field cannot hold
    /// is not left out as an untyped `<param>`'s is: it is refused where it is
    /// written. This model knows the type of every value before it runs, so the
    /// check is made here and the invocation has nothing to check when it
    /// starts:
    ///
    /// - an integer of a width the field's range holds whole (the same
    ///   signedness and no wider, or an unsigned value into a wider signed
    ///   field), never narrowed; a literal takes the field's type, and is
    ///   refused where the field's range does not hold it;
    /// - a real no wider than the field's;
    /// - a bool into a `bool` field;
    /// - a string or byte string held by a variable whose `sce:capacity` is no
    ///   more than the field's `sce:max-size`, which is the one place this model
    ///   knows the most bytes a value ever holds;
    /// - a value of the field's own enum, a variant or a variable of it, which
    ///   crosses as the name the enum declares for it
    ///   ([`Self::request_enum_value`]).
    ///
    /// A literal was held to its field at parse time, and an empty `<param>` is
    /// judged where it is lowered.
    fn request_field_value(
        &self,
        ctx: &TypeCtx<'_>,
        param: &Param,
        request: &crate::forge::model::EntrySchema,
        element: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let wire = WireParam::of_param(param);
        if wire.is_static_literal || wire.written.trim().is_empty() {
            return Ok(());
        }
        let Some(field) = request.fields.iter().find(|f| f.id == param.name) else {
            return Ok(());
        };
        let written = wire.written;
        let verdict =
            if let crate::forge::model::SceType::Enum(reference) = field.sce_type.present_type() {
                self.request_enum_value(ctx, written, wire.spelling, &reference.alias)?
            } else {
                // A typed request holds no optional field (`typed_invoke::validate`
                // refuses one by name, docs/adr/0055), so the field is read as it is.
                let want = InferredType::of_field(&field.sce_type);
                // The field is the place the value lands in, so a literal in it takes
                // the field's type, as a literal takes the type of the variable an
                // `<assign>` writes, and one the field's type cannot hold is refused
                // where it is written (`expression/literal-out-of-range`).
                let ty = self.expr(ctx, written, wire.spelling, Expected::Hint(want))?;
                let given = match (ty, want) {
                    (
                        InferredType::UntypedInt,
                        InferredType::Int { .. } | InferredType::Float { .. },
                    )
                    | (InferredType::UntypedFloat, InferredType::Float { .. }) => want,
                    _ => ty.wire_param_slot().unwrap_or(ty),
                };
                self.request_scalar_value(written, field.max_size, want, given)
            };
        match verdict {
            Ok(()) => Ok(()),
            Err(why) => {
                let at = wire.source_location;
                Err(self.rule_at(
                    format!("<param name=\"{}\"> of {element}", param.name),
                    &format!(
                        "a request typed by sce:request is held to the record its schema names, \
                         and {why}: a field takes a value its type holds, never a narrowed one"
                    ),
                    at.and_then(|l| l.line),
                    at.and_then(|l| l.col),
                    state,
                    written,
                ))
            }
        }
    }

    /// A `<param>` that is a record the invoke names its schema of
    /// ([`RecordParam`]): a variable, or a field of a record variable, of the
    /// very record `alias`, which crosses to the host in the form the
    /// attribute that names it gives. Anything else (a literal, a text, a
    /// record of another schema) is refused where it is written.
    fn record_param_value(
        &self,
        ctx: &TypeCtx<'_>,
        param: &Param,
        named: &RecordParam<'_>,
        element: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        use crate::forge::value_type::ValueType;
        let alias = named.alias;
        let wire = WireParam::of_param(param);
        let written = wire.written;
        let held = crate::forge::expr::written_place(written)
            .filter(|_| !wire.is_static_literal)
            .and_then(|path| ctx.value_of(&path));
        let holds_the_record = held.is_some_and(|place| {
            place.scope.is_empty()
                && matches!(&place.ty, ValueType::Record { alias: of } if of == alias)
        });
        if holds_the_record {
            return Ok(());
        }
        let at = wire.source_location;
        Err(self.rule_at(
            format!("<param name=\"{}\"> of {element}", param.name),
            &format!(
                "{} is a variable, or a field of a record variable, of the record {} names \
                 (`record:{alias}`), which crosses as its {}, and `{}` is no such place",
                named.what,
                named.attribute,
                named.form,
                written.trim()
            ),
            at.and_then(|l| l.line),
            at.and_then(|l| l.col),
            state,
            written,
        ))
    }

    /// [`Self::request_field_value`]'s judgement of a value `given` for a field
    /// whose type is the scalar `want` and whose bound is `max_size`: why the
    /// field cannot hold it, if it cannot.
    fn request_scalar_value(
        &self,
        written: &str,
        max_size: Option<u32>,
        want: InferredType,
        given: InferredType,
    ) -> Result<(), String> {
        match (want, given) {
            (
                InferredType::Int {
                    signed: want_signed,
                    bits: want_bits,
                },
                InferredType::Int {
                    signed: given_signed,
                    bits: given_bits,
                },
            ) => {
                let holds = (want_signed == given_signed && given_bits <= want_bits)
                    || (want_signed && !given_signed && given_bits < want_bits);
                holds.then_some(()).ok_or_else(|| {
                    format!(
                        "the value is {}, which the field's {} does not hold whole",
                        given.describe(),
                        want.describe()
                    )
                })
            }
            (InferredType::Float { bits: want_bits }, InferredType::Float { bits: given_bits }) => {
                (given_bits <= want_bits).then_some(()).ok_or_else(|| {
                    format!(
                        "the value is {}, which the field's {} does not hold whole",
                        given.describe(),
                        want.describe()
                    )
                })
            }
            (InferredType::Bool, InferredType::Bool) => Ok(()),
            (InferredType::Str, InferredType::Str) | (InferredType::Bytes, InferredType::Bytes) => {
                self.request_text_within(written, max_size)
            }
            _ => Err(format!(
                "the value is {}, and the field is {}",
                given.describe(),
                want.describe()
            )),
        }
    }

    /// [`Self::request_field_value`]'s judgement of `written` for a field that
    /// holds a value of the enum `alias` (docs/adr/0109, step 4 of its order of
    /// work): a value of that very enum, a variant or a variable of it, which
    /// crosses to the host as the name the enum declares for it and is read
    /// back into the enum by that name. Why the field cannot hold it, if it
    /// cannot; a use of an enum value the expression rules refuse is refused as
    /// they refuse it.
    fn request_enum_value(
        &self,
        ctx: &TypeCtx<'_>,
        written: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        alias: &str,
    ) -> Result<Result<(), String>, Located<ForgeError>> {
        use crate::forge::static_enum::Held;
        self.expr(
            ctx,
            written,
            spelling,
            Expected::Hint(InferredType::Unknown),
        )?;
        // The field holds the enum document the record imports whether or not
        // this document imports it (docs/adr/0054); a value of it is named
        // through the document's own import, so without one there is none.
        if !self.enums.iter().any(|e| e.alias == alias) {
            let document = self
                .held_enums
                .get(alias)
                .map_or(alias, |model| model.name.as_str());
            return Ok(Err(format!(
                "the field holds a value of the enum `{document}` the record imports, which this \
                 document does not import: import it (<sce:import kind=\"enum\">, under any \
                 alias) and write one of its variants"
            )));
        }
        let held = self.wire_value_held(ctx, written).map_err(|refusal| {
            Located::in_file(
                ExpressionSite::new(written, spelling).place(refusal),
                self.diag_label,
            )
        })?;
        Ok(match held {
            Held::Enum(of) if of == alias => Ok(()),
            Held::Enum(of) => Err(format!(
                "the value is of the enum `{of}`, and the field holds a value of the enum \
                 `{alias}`"
            )),
            Held::Optional(of) => Err(format!(
                "the value is an optional value of the enum `{of}`, which may be absent, and a \
                 request has no spelling for an absent value (read it with `??` and a default of \
                 the enum `{alias}`)"
            )),
            Held::Null => Err(format!(
                "the value is `null`, and the field holds a value of the enum `{alias}`"
            )),
            Held::Neither => Err(format!(
                "the value is no value of an enum, and the field holds a value of the enum \
                 `{alias}` (write `{alias}.<variant>`, or a variable of that enum)"
            )),
        })
    }

    /// Whether the text `written` is held by a variable whose declared bound is
    /// within `max_size`: a string's length is a run-time fact except where a
    /// declaration states its most, so only a bare variable name is shown to fit.
    fn request_text_within(&self, written: &str, max_size: Option<u32>) -> Result<(), String> {
        let Some(max) = max_size else {
            return Ok(());
        };
        let name = written.trim();
        let held = self
            .scope
            .variables
            .iter()
            .find(|v| v.id == name)
            .and_then(|v| v.capacity());
        match held {
            Some(capacity) if capacity <= max => Ok(()),
            Some(capacity) => Err(format!(
                "`{name}` holds up to {capacity} bytes, past the {max} the field holds"
            )),
            None => Err(format!(
                "the value is not a variable, so no declaration says how long it is, and the \
                 field holds {max} bytes at most: take it into a variable declared with \
                 sce:capacity of {max} or less"
            )),
        }
    }

    /// A block of executable content, each action judged with the subjects
    /// proven present where it stands (docs/adr/0055, section 3): those the
    /// block was entered with, less each one an action before it may write
    /// ([`narrowing::Facts::after`]). Nothing in a statechart's content proves
    /// a fact for the actions after it: an `<if>` proves its facts inside its
    /// own blocks only.
    fn actions(
        &self,
        ctx: &TypeCtx<'_>,
        actions: &[Action],
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let mut proven = ctx.proven.clone();
        for action in actions {
            self.action(&proving(ctx, &proven), action, state)?;
            proven = proven.after(&narrowing::Writes::of(action));
        }
        Ok(())
    }

    fn action(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        // docs/adr/0109, D12: a value the deployment supplies is read and
        // never written, whatever statement would write it.
        self.deployment_values_unwritten(action)?;
        let kind = action.action_type.as_str();
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
                // A map is written by its statements, and taken whole only from
                // another map (docs/adr/0108, D2 and D3).
                if let Some(judged) = self.map_assign(ctx, action) {
                    return judged;
                }
                // A union is put into one whole, and its tag is never written
                // alone (docs/adr/0092, section 6.1 items 5 and 6); a field of
                // one is a write judged below, where its case must be proven.
                if let Some(judged) = self.union_assign(ctx, action, state) {
                    return judged;
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
                    // ... or from what an imported algorithm returns as a record of its
                    // schema, the call being the whole of the right side.
                    if let Some(callee) = self
                        .scope
                        .whole_value_call(&action.expr)
                        .filter(|c| StaticScope::returns_state(c))
                    {
                        let target = self
                            .scope
                            .variables
                            .iter()
                            .find(|v| v.id == location)
                            .expect("the location names a record variable");
                        return self.state_from_call(ctx, action, target, callee);
                    }
                    // ... or from the element of a list of its schema, by an index:
                    // the element is copied field by field, as a field of it is read.
                    if let Some((list, index)) = indexed_element(&action.expr) {
                        return self.element_copy(ctx, action, alias, list, index, state);
                    }
                    // ... or from a field of a record that holds a record of its
                    // schema, copied whole.
                    if let Some(judged) = self.record_field_copy(ctx, action, alias, false, state) {
                        return judged;
                    }
                    return self.record_of(
                        "assign",
                        &action.expr,
                        action.spellings.get("expr"),
                        alias,
                        state,
                    );
                }
                // An optional record takes `null`, or a whole record by the same
                // rule a record variable takes one (docs/adr/0055, step 9).
                if let Some(alias) = self.optional_record_alias(location) {
                    return self.optional_record_assign(ctx, action, alias, state);
                }
                if let Some(list) = self.list_var(location) {
                    // The lists a statechart takes whole: what an imported
                    // algorithm returns, when the variable can hold all it may.
                    if let Some(callee) = self.scope.whole_list_call(&action.expr) {
                        return self.list_from_call(ctx, action, list, callee);
                    }
                    if let Some(callee) = self
                        .scope
                        .whole_value_call(&action.expr)
                        .filter(|c| StaticScope::returns_state(c))
                    {
                        return self.state_from_call(ctx, action, list, callee);
                    }
                    // A list held at a key of a map is taken whole only into a
                    // variable of exactly its type (docs/adr/0108, D12 rule 6).
                    if let Some(Ok(at)) = self.scope.map_place(&action.expr) {
                        let exact = crate::forge::value_path::variable_place(list)
                            .is_some_and(|to| to.ty == at.place.ty);
                        if !at.steps.is_empty() && !exact {
                            let written = action.expr.trim();
                            return Err(Located::in_file(
                                ExpressionSite::new(&action.expr, action.spellings.get("expr"))
                                    .place(
                                        crate::forge::error::ExprError::UnsupportedConstruct {
                                            construct: format!(
                                                "an assignment to the whole list `{location}` of \
                                                 `{written}`, a value of `{}` held at a key of \
                                                 the map `{}` (a value held at a key is taken \
                                                 whole only into a variable of exactly its type, \
                                                 docs/adr/0108, D12 rule 6)",
                                                at.place.ty, at.variable.id
                                            ),
                                            observed: Some(written.to_string()),
                                        }
                                        .at(None),
                                    ),
                                self.diag_label,
                            ));
                        }
                    }
                    // A list a place holds, at any depth, taken whole: a copy that
                    // cannot fail when its type fits the variable's
                    // (docs/adr/0086, section 6.1).
                    if let Some(held) = self.whole_place(ctx, &action.expr) {
                        return self.list_copy(ctx, action, list, &held, state);
                    }
                    // A call of an algorithm that does return a list, refused for a
                    // reason the author can act on: say which, rather than that a list
                    // "takes what an imported algorithm returns" when this one does not.
                    let construct = match self.scope.refused_list_call(&action.expr) {
                        Some((callee, reason)) => format!(
                            "an assignment to the whole list `{location}` from the algorithm \
                             `{}`, which a statechart cannot take a list from: {reason}",
                            callee.alias
                        ),
                        None => format!(
                            "an assignment to the whole list `{location}` (a list \
                             is filled by <sce:append> and emptied by <sce:clear>, \
                             or takes what an imported algorithm returns as a list)"
                        ),
                    };
                    return Err(Located::in_file(
                        ExpressionSite::new(&action.location, action.spellings.get("location"))
                            .place(
                                crate::forge::error::ExprError::UnsupportedConstruct {
                                    construct,
                                    observed: Some(location.to_string()),
                                }
                                .at(None),
                            ),
                        self.diag_label,
                    ));
                }
                // A map, a set or a list held at a key of a map is no value of
                // one entry (docs/adr/0108, D12 rule 6): it is taken whole only
                // into a variable of exactly its type, which the branches above
                // take, and read through `len`, `has`, a further key and a
                // <foreach>.
                if let Some(Ok(held)) = self.scope.map_place(&action.expr) {
                    if !held.steps.is_empty() && held.place.ty.is_container() {
                        let written = action.expr.trim();
                        return Err(Located::in_file(
                            ExpressionSite::new(&action.expr, action.spellings.get("expr")).place(
                                crate::forge::error::ExprError::UnsupportedConstruct {
                                    construct: format!(
                                        "an assignment to `{location}` of `{written}`, a value of \
                                         `{}` held at a key of the map `{}` (a map, a set or a \
                                         list held at a key is read through len, has, a further \
                                         key and a <foreach>, and is taken whole only into a \
                                         variable of exactly its type, docs/adr/0108, D12 rule 6)",
                                        held.place.ty, held.variable.id
                                    ),
                                    observed: Some(written.to_string()),
                                }
                                .at(None),
                            ),
                            self.diag_label,
                        ));
                    }
                }
                // The location is a declared variable; its type is the slot.
                // Typing it reads it as an expression, which an index read
                // is, so its shape is held first: a variable, or a field of
                // one — an element of a list is never written.
                //
                // A field of the present value of an optional record,
                // `pick.value.count`, is the one place a path of three names is
                // written (docs/adr/0055, step 9): where a test proves the record
                // present, and nowhere else, since an absent record has no field
                // to write and no write makes one.
                let present_field = narrowing::present_record_of(location)
                    .filter(|holder| self.optional_record_alias(holder).is_some());
                if let Some(holder) = present_field {
                    if !ctx.proven.holds(holder) {
                        return Err(Located::in_file(
                            ExpressionSite::new(&action.location, action.spellings.get("location"))
                                .place(
                                    crate::forge::error::ExprError::UnsupportedConstruct {
                                        construct: format!(
                                            "a write to `{location}`, a field of the value of the \
                                         optional record `{holder}`, where no test proves \
                                         `{holder}` present (test it first: `{holder} !== \
                                         null` around the write)"
                                        ),
                                        observed: Some(location.to_string()),
                                    }
                                    .at(None),
                                ),
                            self.diag_label,
                        ));
                    }
                } else {
                    crate::forge::expr::judge_lvalue_shape(&action.location).map_err(
                        |refusal| {
                            Located::in_file(
                                ExpressionSite::new(
                                    &action.location,
                                    action.spellings.get("location"),
                                )
                                .place(refusal),
                                self.diag_label,
                            )
                        },
                    )?;
                    // An optional value is written whole: `null` makes it absent
                    // and a value present (docs/adr/0055). Its `.value` is a read.
                    if let Some(holder) = crate::forge::static_enum::present_value_of(location)
                        .filter(|holder| ctx.lookup_var(holder.trim()).is_optional())
                    {
                        return Err(Located::in_file(
                            ExpressionSite::new(&action.location, action.spellings.get("location"))
                                .place(
                                    crate::forge::error::ExprError::UnsupportedConstruct {
                                        construct: format!(
                                            "an assignment to `{location}`, the value of an \
                                         optional value (an optional value is assigned whole: \
                                         <assign location=\"{}\"> writes `null` or a value)",
                                            holder.trim()
                                        ),
                                        observed: Some(location.to_string()),
                                    }
                                    .at(None),
                                ),
                            self.diag_label,
                        ));
                    }
                }
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
                } else if let Some(alias) = self.optional_enum_of(ctx, location) {
                    self.expr_of_optional_enum(
                        ctx,
                        &action.expr,
                        action.spellings.get("expr"),
                        &alias,
                    )?;
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
                self.log_value(ctx, action, state)?;
            }
            "send" => {
                let carrier = Carrier::of_send(action);
                for param in &action.params {
                    let wire = WireParam::of_param(param);
                    if self.composed_param(ctx, action, &wire, state)? {
                        continue;
                    }
                    self.wire_param(ctx, &wire, "<send>", carrier, state)?;
                    self.received(ctx, action, &wire, state)?;
                }
                // A `namelist` name is the `<param name="x" expr="x"/>` it
                // abbreviates ([`Action::fold_namelist_into_params`]), held to
                // the rule a param is, and placed at the attribute.
                for name in action.namelist.split_whitespace() {
                    let namelist =
                        WireParam::of_namelist_name(name, action.spellings.get("namelist"), None);
                    if self.composed_param(ctx, action, &namelist, state)? {
                        continue;
                    }
                    self.wire_param(ctx, &namelist, "<send>", carrier, state)?;
                    self.received(ctx, action, &namelist, state)?;
                }
                if !action.eventexpr.is_empty() {
                    self.event_expr(ctx, action, state)?;
                }
                if !action.targetexpr.is_empty() {
                    self.target_expr(ctx, action, state)?;
                }
                if !action.typeexpr.is_empty() {
                    self.type_expr(ctx, action, state)?;
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
                        &ContentExpr {
                            element: "<send>",
                            written: &action.contentexpr,
                            spelling: action.contentexpr_spelling.as_ref(),
                            beside: !action.params.is_empty() || !action.namelist.trim().is_empty(),
                            carrier,
                        },
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
                // A list held at a key of a map, or in a field of a record held
                // there, is appended to through its place (docs/adr/0108, D12
                // rule 1).
                let Some(list) = self.list_var(&action.location) else {
                    return self.nested_append(ctx, action, state);
                };
                match list
                    .value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem)
                {
                    // A record is appended whole, by the name of a record of
                    // the list's schema: nothing computes one in an expression.
                    Some(crate::forge::model::AlgorithmValueType::Record { alias }) => {
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
                            .and_then(crate::forge::model::AlgorithmValueType::scalar)
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
            "sce_render" => self.render(ctx, action)?,
            // docs/SCE_TEST_PROFILE.md §11.5: the `reason` of a test
            // component's `<sce:setverdict>` is an expression of this data
            // model of type string, read where the statement runs.
            "sce_setverdict" => {
                if let Some(crate::test_component::TestStatement::Setverdict {
                    reason: Some(reason),
                    ..
                }) = &action.test_statement
                {
                    self.expr(
                        ctx,
                        reason,
                        action.spellings.get("reason"),
                        Expected::Slot(InferredType::Str),
                    )?;
                }
            }
            // A list, a map or a set is emptied whole (docs/adr/0108, D3).
            "sce_clear" => {
                // ... and so is a list, a map or a set held at a key of a map
                // (docs/adr/0108, D12 rule 1).
                let names_one = self.list_var(&action.location).is_some()
                    || self.scope.map_variable(&action.location).is_some()
                    || match self.written_through(ctx, action, state) {
                        Some(Ok(place)) => place.place.ty.is_container(),
                        Some(Err(refusal)) => return Err(refusal),
                        None => false,
                    };
                if !names_one {
                    return Err(self.no_target(
                        action,
                        state,
                        "a list, a map or a set variable",
                        &|ty: &crate::forge::model::AlgorithmValueType| {
                            ty.list_elem().is_some() || ty.map_key().is_some()
                        },
                    ));
                }
            }
            "sce_put" => self.put(ctx, action, state)?,
            "sce_remove" => self.remove(ctx, action, state)?,
            "sce_call" => self.call(ctx, action, state)?,
            "sce_lift" => self.lift(ctx, action)?,
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
        // its block. Each condition of an `<if>` chain is read where every
        // earlier one is false, each block where its own is true besides, and
        // the `<else>` where all are false ([`narrowing::Chain`]).
        let chain = narrowing::Chain::of(&ctx.proven, &chain_conds(action));
        let mut elseif = 0;
        for block in action.nested_blocks() {
            let (cond_facts, block_facts) = match block.role {
                crate::model::BlockRole::Body => (ctx.proven.clone(), ctx.proven.clone()),
                crate::model::BlockRole::Then => (chain.cond(0), chain.block(0)),
                crate::model::BlockRole::ElseIf => {
                    elseif += 1;
                    (chain.cond(elseif), chain.block(elseif))
                }
                crate::model::BlockRole::Else => (chain.otherwise(), chain.otherwise()),
            };
            if let Some(cond) = block.cond.filter(|_| !block.cond_is_native) {
                self.expr(
                    &proving(ctx, &cond_facts),
                    cond,
                    block.cond_spelling,
                    Expected::Slot(InferredType::Bool),
                )?;
            }
            self.actions(&proving(ctx, &block_facts), block.actions, state)?;
        }
        Ok(())
    }

    /// A `<foreach>` over a list (§scxml-4.6) a variable, the payload or an
    /// enclosing loop's item holds: `item` is each element, a value of the
    /// list's element type, and `index` — when written — its position,
    /// a `uint32` as `len` is. Both are local to the body, so each is a name
    /// nothing in scope already means, a name the generated code can declare in
    /// every backend (it is spelled as written), and not the other's.
    ///
    /// A `<foreach>` over a map or a set variable (docs/adr/0108, D5) walks its
    /// entries in the order their keys were first put: `item` is the value of
    /// each, `sce:key` (which may be left out) its key and `index` its position;
    /// over a set `item` is each key, and `sce:key` is refused. The key is
    /// held to the rules the item and the index are, read and never written.
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
        // The list is a place a value holds, at any depth: a list variable, a
        // list the payload holds, or one an enclosing loop's item holds
        // (docs/adr/0086, section 6.1), written as an expression reads it, an
        // index into a list on the way included (`_event.data.items[0].tags`).
        let held = crate::forge::expr::written_place(array).and_then(|path| ctx.value_of(&path));
        // A map or a set variable is walked in the order its keys were first
        // put (docs/adr/0108, D2 and D5): the item is the value of each entry
        // and the key, `sce:key`, its key; a set's item is the key, and it
        // binds no other.
        // ... and so is a map or a set held at a key of one, `m[k]`
        // (docs/adr/0108, D12 rule 8), the variable it is held in the one a
        // body that writes it along any path writes.
        let walked_root = match self.scope.map_place(array) {
            Some(Ok(place)) => Some((place.variable.id.clone(), !place.steps.is_empty())),
            Some(Err(_)) | None => None,
        };
        let map = walked_root
            .as_ref()
            .and(held.as_ref())
            .filter(|place| place.is_keyed());
        let nested = walked_root.as_ref().is_some_and(|(_, nested)| *nested);
        let (element, key_place) = match map {
            Some(map) => match map.map_value() {
                Some(value) => (Some(value), map.map_key()),
                None => (map.map_key(), None),
            },
            None => (held.as_ref().and_then(|list| list.element()), None),
        };
        let Some(element) = element else {
            return Err(place(
                "array",
                "a <foreach> of this data model walks a list variable, a map or a set variable, \
                 or a list the event's payload or an enclosing loop's item holds: array names \
                 one of them"
                    .to_string(),
                array,
            ));
        };
        // A place past a name is an expression: its names and its indices are
        // judged as any expression's are. A list, a map or a set variable is no
        // value an expression reads ([`StaticScope::list_read_as_value`],
        // [`StaticScope::map_read_as_value`]), and holds none.
        if self.list_var(array).is_none() && (map.is_none() || nested) {
            self.expr(
                ctx,
                array,
                action.spellings.get("array"),
                Expected::Hint(InferredType::Unknown),
            )?;
        }
        let item = action.item.trim();
        let index = action.index.trim();
        let key = action.key.trim();
        if item.is_empty() {
            return Err(place(
                "item",
                "a <foreach> names the variable that holds each element".to_string(),
                item,
            ));
        }
        // Only a map's entry has a key apart from its item.
        if !key.is_empty() && key_place.is_none() {
            let walks = match map {
                Some(_) => format!(
                    "the set `{array}` holds keys that carry no value: its item is each key, \
                     and sce:key names nothing more"
                ),
                None => "a <foreach> over a list binds each element and its index: sce:key \
                         names the key of a map's entry, in a <foreach> over a map"
                    .to_string(),
            };
            return Err(place("sce:key", walks, key));
        }
        for (attr, name) in [("item", item), ("sce:key", key), ("index", index)] {
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
            } else if attr != "item" && name == item {
                Some("a name other than the item's")
            } else if attr == "index" && name == key {
                Some("a name other than the key's")
            } else {
                None
            };
            if let Some(why) = why {
                return Err(place(attr, format!("a <foreach> {attr} is {why}"), name));
            }
        }
        // A record item is a record an append in the body may take by name,
        // under the alias the statechart names its schema by; so is a record
        // key of a map's entry (docs/adr/0108, D5).
        let record_item = self.statechart_record_alias(&element);
        let key_place = key_place.filter(|_| !key.is_empty());
        let record_key = key_place
            .as_ref()
            .and_then(|at| self.statechart_record_alias(at));
        let loop_variables = crate::forge::type_ctx::LoopVariables::of_place(
            item,
            (!index.is_empty()).then_some(index),
            Some(element),
            record_item
                .as_deref()
                .and_then(|alias| self.schemas.get(alias)),
        );
        let loop_variables = match key_place {
            Some(at) => loop_variables.with_key(crate::forge::type_ctx::LoopVariables::of_place(
                key,
                None,
                Some(at),
                record_key
                    .as_deref()
                    .and_then(|alias| self.schemas.get(alias)),
            )),
            None => loop_variables,
        };
        let mut inner = loop_variables.bind(ctx);
        // A fact proven before the loop holds in its body only if nothing in
        // the body ends it: a turn follows the end of the one before
        // (docs/adr/0055, section 3).
        inner.proven = narrowing::loop_entry(&ctx.proven, &action.actions);
        // The item's and the key's enum fields are read from their records
        // whole, as a record variable's are ([`enum_field_aliases`]).
        let mut enum_paths = record_item
            .as_deref()
            .and_then(|alias| self.lift_schemas.get(alias))
            .map(|schema| enum_fields(item, schema))
            .unwrap_or_default();
        if let Some(schema) = record_key
            .as_deref()
            .and_then(|alias| self.lift_schemas.get(alias))
        {
            enum_paths.extend(enum_fields(key, schema));
        }
        let enum_paths_bound = enum_paths.len();
        let mut records_bound = 0;
        for (name, alias) in [(item, &record_item), (key, &record_key)] {
            if let Some(alias) = alias {
                self.loop_records
                    .borrow_mut()
                    .push((name.to_string(), alias.clone()));
                records_bound += 1;
            }
        }
        self.loop_enum_paths.borrow_mut().extend(enum_paths);
        let names_bound: Vec<&str> = [item, key, index]
            .into_iter()
            .filter(|name| !name.is_empty())
            .collect();
        self.loop_names
            .borrow_mut()
            .extend(names_bound.iter().map(|name| name.to_string()));
        // The map the body walks is no map a `<sce:call>` in it writes in place
        // ([`Self::call`]).
        let walks_a_map = map.is_some();
        if let Some((root, _)) = walked_root.as_ref().filter(|_| walks_a_map) {
            self.walked_maps.borrow_mut().push(root.clone());
        }
        let judged = self.actions(&inner, &action.actions, state);
        if walks_a_map {
            self.walked_maps.borrow_mut().pop();
        }
        let kept = self.loop_names.borrow().len() - names_bound.len();
        self.loop_names.borrow_mut().truncate(kept);
        let kept = self.loop_enum_paths.borrow().len() - enum_paths_bound;
        self.loop_enum_paths.borrow_mut().truncate(kept);
        let kept = self.loop_records.borrow().len() - records_bound;
        self.loop_records.borrow_mut().truncate(kept);
        judged
    }

    /// The alias this statechart names the record `place` holds by: the alias
    /// the place names it by where the place is read in the statechart's own
    /// scope, and otherwise the alias of the statechart's import of the same
    /// record document. `None` for a value that is no record, or a record the
    /// statechart does not import.
    fn statechart_record_alias(&self, place: &crate::forge::value_path::Place) -> Option<String> {
        let crate::forge::value_type::ValueType::Record { alias } = &place.ty else {
            return None;
        };
        if place.scope.is_empty() {
            return Some(alias.clone());
        }
        let base = &self.scope.type_scope;
        let identity = crate::forge::value_path::record_identity(Some(base), place)?;
        base.records
            .iter()
            .find(|(_, record)| record.identity == identity)
            .map(|(alias, _)| alias.clone())
    }

    /// The place `written` names, when it is one taken whole: a name a value
    /// is held under and the fields, elements and present values read after it
    /// ([`TypeCtx::value_of`]). `None` for an expression that is no place, and
    /// for a list variable, which is no value an expression reads
    /// ([`StaticScope::list_read_as_value`]).
    fn whole_place(
        &self,
        ctx: &TypeCtx<'_>,
        written: &str,
    ) -> Option<crate::forge::value_path::Place> {
        if self.scope.list_read_as_value(written).is_some() {
            return None;
        }
        ctx.value_of(&crate::forge::expr::written_place(written)?)
    }

    /// `<assign location="kept" expr="_event.data.items"/>`: the list variable
    /// `list` takes the value `held` holds whole (docs/adr/0086, section 6.1),
    /// a copy that cannot fail where the value's type fits the variable's
    /// (section 5.1). One that may not fit is refused, naming the first place
    /// where it may not and the narrowing that takes it: an `<sce:append>` of
    /// each element in a `<foreach>` over it.
    fn list_copy(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        list: &Variable,
        held: &crate::forge::value_path::Place,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("expr");
        // Its names and indices are judged as any expression's are.
        self.expr(
            ctx,
            &action.expr,
            spelling,
            Expected::Hint(InferredType::Unknown),
        )?;
        let base = &self.scope.type_scope;
        let names = ScopedNames {
            from: crate::forge::value_path::scope_at(base, &held.scope),
            to: Some(base),
        };
        list_fits(&held.ty, list, &names).map_err(|misfit| {
            let written = action.expr.trim();
            self.rule_at(
                format!("<assign location=\"{}\" expr=\"{written}\">", list.id),
                &format!(
                    "{}: `{}` takes it only element by element, an <sce:append> of each in a \
                     <foreach> over `{written}`",
                    misfit.sentence(written),
                    list.id
                ),
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            )
        })
    }

    /// A record variable of the schema `alias` takes the element of a list of that
    /// schema at `index` whole (`<assign location="last" expr="days[0]"/>`).
    ///
    /// Nothing in an expression makes a record, so no backend reads an element
    /// whole: the copy is the element's fields read one by one, each as
    /// `days[0].<field>` is, and each field is judged here as that read is. An index
    /// outside the list fails the first read, writes nothing and ends the block
    /// (§scxml-4.9), as it does for a field. The index is read once per field, so
    /// it may not name the record being written: its own fields change under it.
    ///
    /// The value at a key of a map of records is taken whole the same way
    /// (`<assign location="last" expr="faces[peer]"/>`, docs/adr/0108, D3): a
    /// key that is absent fails the first read as an index outside a list does,
    /// and the key, read once per field, may not name the record being written.
    fn element_copy(
        &self,
        ctx: &TypeCtx<'_>,
        action: &Action,
        alias: &str,
        list: &str,
        index: &str,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let spelling = action.spellings.get("expr");
        let location = action.location.trim();
        let written = action.expr.trim();
        let refuse = |rule: String| {
            self.rule_at(
                "<assign>".to_string(),
                &rule,
                spelling.map(|s| s.row()),
                spelling.map(|s| s.col()),
                state,
                written,
            )
        };
        let map_value = self.scope.map_variable(list).and_then(|(_, ty)| match ty {
            crate::forge::model::AlgorithmValueType::Map { value, .. } => Some(&**value),
            _ => None,
        });
        let (held, part, at) = match map_value {
            Some(value) => (Some(value), format!("a value of the map `{list}`"), "key"),
            None => (
                self.list_var(list)
                    .and_then(|v| v.value_type.as_ref())
                    .and_then(crate::forge::model::AlgorithmValueType::list_elem),
                format!("an element of the list `{list}`"),
                "index",
            ),
        };
        match held {
            Some(crate::forge::model::AlgorithmValueType::Record { alias: held })
                if held == alias => {}
            Some(crate::forge::model::AlgorithmValueType::Record { alias: held }) => {
                return Err(refuse(format!(
                    "{part} is a record of the schema {held}, and `{location}` holds one of the \
                     schema {alias}"
                )));
            }
            _ => return self.record_of("assign", written, spelling, alias, state),
        }
        if names_identifier(index, location) {
            return Err(refuse(format!(
                "the {at} of the {} reads `{location}`, which the copy writes a field at a time, \
                 so the {at} would change under it: take the {at} into a variable first",
                if map_value.is_some() {
                    "entry"
                } else {
                    "element"
                }
            )));
        }
        let Some(schema) = self.schemas.get(alias) else {
            return self.record_of("assign", written, spelling, alias, state);
        };
        for field in &schema.fields {
            // An enum is untyped to an expression: the schema holds it to the enum.
            let slot = match &field.sce_type {
                crate::forge::model::EntryType::Scalar(crate::forge::model::SceType::Enum(_)) => {
                    InferredType::Unknown
                }
                declared => InferredType::of_field(declared),
            };
            self.expr(
                ctx,
                &format!("{list}[{index}].{}", field.id),
                spelling,
                Expected::Slot(slot),
            )?;
        }
        Ok(())
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
        self.record_named(element, "expr", expr, spelling, alias, state)
    }

    /// [`Self::record_of`] for the attribute `attr` of `element`: a record
    /// named whole where a record of the schema `alias` is taken, a map's
    /// record key among them (docs/adr/0108, D3).
    fn record_named(
        &self,
        element: &str,
        attr: &str,
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
                let shape = |schema: &crate::forge::model::EntrySchema| {
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
                    format!("<{element} {attr}=\"{written}\">"),
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
            format!("<{element} {attr}=\"{written}\">"),
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
    /// where it is written rather than accepted and never delivered. A variable
    /// of `optional<T>` over a bool, a number or a text is handed an optional
    /// value ([`seed_slot`]); one that starts at a value takes no value that may
    /// be absent ([`absent_handed_to_a_present_start`]).
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
            let (written, spelling) = argument_written(&param);
            let refuse = |rule: &str| self.argument_rule(&element, &param, rule, state);
            self.argument_is_given_once(&param, &mut handed, &refuse)?;
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
                return Err(refuse(&unseedable_variable(variable, "the child's")));
            };
            let ty = self.expr(ctx, written, spelling, Expected::Slot(slot))?;
            if let Some(rule) = absent_handed_to_a_present_start(variable, ty, "the child's") {
                return Err(refuse(&rule));
            }
        }
        Ok(())
    }

    /// The refusal of one argument an `<invoke>` hands a child, placed at the
    /// `<param>` — or, for a `namelist` name, at the `<invoke>`, the only
    /// position the model records for it — and saying `rule`.
    fn argument_rule(
        &self,
        element: &str,
        param: &crate::model::Param,
        rule: &str,
        state: &str,
    ) -> Located<ForgeError> {
        let (written, _) = argument_written(param);
        let at = param.source_location.as_ref();
        self.rule_at(
            format!("<param name=\"{}\"> of {element}", param.name),
            rule,
            at.and_then(|l| l.line),
            at.and_then(|l| l.col),
            state,
            written,
        )
    }

    /// The two rules every argument of an `<invoke>` is held to whichever
    /// child it is handed to: it names a value, and the name is handed once.
    fn argument_is_given_once(
        &self,
        param: &crate::model::Param,
        handed: &mut std::collections::BTreeSet<String>,
        refuse: &dyn Fn(&str) -> Located<ForgeError>,
    ) -> Result<(), Located<ForgeError>> {
        let (written, _) = argument_written(param);
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
        Ok(())
    }

    /// §scxml-6.4: a hybrid `<invoke>` names its child by the `srcexpr` it
    /// computes when it starts, and under this model the child is one of the
    /// documents `sce:candidates` declares: the value names one by its document
    /// stem, and the build has generated each. So the `srcexpr` is a string the
    /// machine computes from its fields, every candidate is a
    /// `datamodel="sce-static"` document this build read, and a `<content expr>`,
    /// which PRODUCES the document, has no finite set to lower.
    ///
    /// The invoke's arguments are the same for whichever child is chosen, but
    /// what each candidate KEEPS of them is its own (§scxml-6.4.3): a name it
    /// declares is typed against that variable, and one it does not is
    /// evaluated and left out. A name no candidate declares would be dropped by
    /// every one, which is refused as a static invoke's is.
    fn hybrid_invoke(
        &self,
        ctx: &TypeCtx<'_>,
        info: &crate::model::HybridInvokeInfo,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let base = &info.common.base;
        let element = format!("<invoke id=\"{}\">", base.invoke_id);
        let at = base.source_location.as_ref();
        let (line, col) = (at.and_then(|l| l.line), at.and_then(|l| l.col));
        if !info.contentexpr.is_empty() {
            let spelling = info.contentexpr_spelling.as_ref();
            return Err(self.rule_at(
                format!("contentexpr=\"{}\"", info.contentexpr),
                "a hybrid <invoke> whose <content expr> produces the document its child runs \
                 has no finite set of documents to lower: name the child by `srcexpr` and \
                 declare the documents it may start in `sce:candidates`",
                spelling.map(|s| s.row()).or(line),
                spelling.map(|s| s.col()).or(col),
                state,
                &info.contentexpr,
            ));
        }
        let spelling = info.srcexpr_spelling.as_ref();
        if info.candidates.is_empty() {
            return Err(self.rule_at(
                format!("srcexpr=\"{}\"", info.srcexpr),
                "a hybrid <invoke> of this data model starts the child its `srcexpr` names \
                 among the documents `sce:candidates` declares: with none declared there is no \
                 child to start, so declare them",
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
        for candidate in &info.candidates {
            if candidate.child_static_variables.is_none() {
                return Err(self.rule_at(
                    format!("sce:candidates=\"{}\"", candidate.path),
                    "the candidate is not a datamodel=\"sce-static\" document this build read, \
                     so a value has no typed variable to arrive in: write the candidate under \
                     the same data model, beside this document",
                    line,
                    col,
                    state,
                    &candidate.path,
                ));
            }
        }
        let mut handed = std::collections::BTreeSet::new();
        for param in info.arguments() {
            let (written, spelling) = argument_written(&param);
            let refuse = |rule: &str| self.argument_rule(&element, &param, rule, state);
            self.argument_is_given_once(&param, &mut handed, &refuse)?;
            if info
                .candidates
                .iter()
                .all(|candidate| candidate_variable(candidate, &param.name).is_none())
            {
                return Err(refuse(&format!(
                    "no candidate declares a top-level <data id=\"{}\">, so every one would \
                     drop the value: declare it in a candidate, or hand a name one declares",
                    param.name
                )));
            }
            for candidate in &info.candidates {
                let Some(variable) = candidate_variable(candidate, &param.name) else {
                    continue;
                };
                let whose = format!("the candidate `{}`'s", candidate.stem);
                let Some(slot) = seed_slot(variable) else {
                    return Err(refuse(&unseedable_variable(variable, &whose)));
                };
                let ty = self.expr(ctx, written, spelling, Expected::Slot(slot))?;
                if let Some(rule) = absent_handed_to_a_present_start(variable, ty, &whose) {
                    return Err(refuse(&rule));
                }
            }
            // A candidate that declares no such variable evaluates the value
            // and leaves it out, so it must be one that can be evaluated to a
            // value every backend spells alike.
            if info
                .candidates
                .iter()
                .any(|candidate| candidate_variable(candidate, &param.name).is_none())
            {
                self.wire_param(
                    ctx,
                    &WireParam::of_param(&param),
                    &element,
                    Carrier::Json,
                    state,
                )?;
            }
        }
        Ok(())
    }

    fn invoke(
        &self,
        ctx: &TypeCtx<'_>,
        invoke: &Invoke,
        state: &str,
    ) -> Result<(), Located<ForgeError>> {
        let base = invoke.base();
        let at = base.source_location.as_ref();
        let (line, col) = (at.and_then(|l| l.line), at.and_then(|l| l.col));
        // The `namelist` of a child session is judged with its `<param>`s
        // ([`Self::child_arguments`]), and the `namelist` and `srcexpr` of a
        // host-run invoke, and the `<param>`s and `srcexpr` of a Mesh request,
        // with the request they hand, below.
        if !base.idlocation.is_empty() {
            self.invoke_idlocation(invoke, state, line, col)?;
        }
        // docs/adr/0109, D12: a value the deployment supplies is read when the
        // host starts the machine, from the configuration of the machine the
        // deployment names; a child session is started by its parent, which
        // has no such value to give it.
        let children: Vec<&[Variable]> = match invoke {
            Invoke::Scxml(info) => info
                .common
                .child_static_variables
                .iter()
                .map(Vec::as_slice)
                .collect(),
            Invoke::Hybrid(info) => info
                .candidates
                .iter()
                .filter_map(|candidate| candidate.child_static_variables.as_deref())
                .collect(),
            _ => Vec::new(),
        };
        if let Some(value) = children
            .into_iter()
            .flatten()
            .find(|variable| variable.from_deployment)
        {
            return Err(self.rule_at(
                format!("<invoke id=\"{}\">", base.invoke_id),
                &format!(
                    "the child declares `{}`, a value its deployment supplies, and a child \
                     session is started by its parent, which has no deployment value to give \
                     it: hand the value as a <param> to a variable of the child's own, or \
                     start the machine from the host",
                    value.id
                ),
                line,
                col,
                state,
                &value.id,
            ));
        }
        // A hybrid invoke names its child by an expression evaluated when it
        // starts, among the documents it declares.
        if let Invoke::Hybrid(info) = invoke {
            return self.hybrid_invoke(ctx, info, state);
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
        // schema names, so each value is held to the field it supplies: a
        // literal where it is written (`validation/typed-invoke-request`), and
        // a value the machine computes by its type, below — this model knows
        // every type before it runs, so there is nothing left to check when the
        // invocation starts. It takes no `namelist` at all: the parser refuses
        // one beside a typed request, under every data model.
        if let Invoke::Unsupported(info) = invoke {
            let element = format!("<invoke id=\"{}\">", base.invoke_id);
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
            // The `content` the host is handed — the body the service runs — is a
            // string computed from the machine's fields when the invocation
            // starts, as the `src` is: a value the host reads as text, not event
            // data, so it is held to a string and no other type.
            if !info.contentexpr.is_empty() {
                self.expr(
                    ctx,
                    &info.contentexpr,
                    info.contentexpr_spelling.as_ref(),
                    Expected::Slot(InferredType::Str),
                )?;
            }
            // The host is handed the pairs as text, with no spelling for an
            // absent value (docs/adr/0055). The entry of an audit append is a
            // record, which crosses as its JSON payload (docs/adr/0109, D14),
            // and the payload of a signing and the plaintext of a seal that
            // name their record are one, which crosses as its deterministic
            // CBOR (docs/adr/0109, D9).
            for param in &base.params {
                if let Some(named) = record_param_of(info, param) {
                    self.record_param_value(ctx, param, &named, &element, state)?;
                    continue;
                }
                self.wire_param(
                    ctx,
                    &WireParam::of_param(param),
                    &element,
                    Carrier::Text,
                    state,
                )?;
            }
            for name in info.namelist.split_whitespace() {
                let pair = WireParam::of_namelist_name(name, None, at);
                self.wire_param(ctx, &pair, &element, Carrier::Text, state)?;
            }
            if let Some(request) = self.schemas.get(&info.request_schema) {
                for param in base
                    .params
                    .iter()
                    .filter(|p| record_param_of(info, p).is_none())
                {
                    self.request_field_value(ctx, param, request, &element, state)?;
                }
            }
            return Ok(());
        }
        // A Mesh request's `<param>`s are the request its router is handed, read
        // from the machine's fields when the invocation starts and lowered to
        // native code like a host-run invoke's (docs/adr/0005, decision 5). Its
        // event name and deadline are constants the parser has already taken out.
        // Its peer is a string computed the same way when the invocation starts
        // (docs/adr/0005, decision 7), among the bindings the deployment declares:
        // a name that matches none is `error.execution` when the request is made.
        if let Invoke::MeshRpc(info) = invoke {
            if let crate::model::MeshRpcTarget::SrcExpr { srcexpr } = &info.target {
                self.expr(ctx, srcexpr, None, Expected::Slot(InferredType::Str))?;
            }
            let element = format!("<invoke id=\"{}\">", base.invoke_id);
            for param in &base.params {
                self.wire_param(
                    ctx,
                    &WireParam::of_param(param),
                    &element,
                    Carrier::Json,
                    state,
                )?;
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

/// Why the text of the `delayexpr` `written` has no bound the data model
/// declares, naming the subexpression that has none; `None` for a text it
/// bounds (docs/adr/0134, decision 2). A text with a bound is the one a machine
/// with no heap writes into a buffer of that bound, as every text it holds is.
fn unbounded_delay(written: &str, ctx: &TypeCtx<'_>) -> Option<String> {
    match crate::forge::text_bound::text_bound_of(written, ctx) {
        Ok(crate::forge::text_bound::TextBound::Unbounded(why)) => {
            Some(why.describe(written.trim()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{AlgorithmValueType, SceType};
    use crate::forge::types::OptionalElem;

    /// docs/adr/0134, decision 2: a delay whose text the data model bounds is
    /// accepted, and one that reads a call whose callee declares no bound on
    /// what it returns is refused, naming that call.
    #[test]
    fn a_delay_whose_text_has_no_bound_is_refused() {
        use crate::forge::types::FuncSig;
        let mut ctx = TypeCtx::new();
        ctx.insert_var(
            "wait",
            InferredType::Int {
                signed: false,
                bits: 32,
            },
        );
        ctx.insert_var("name", InferredType::Str);
        ctx.insert_string_capacity("name", 8);
        ctx.insert_func("Free", FuncSig::new(Vec::new(), InferredType::Str));
        for bounded in [
            "wait + 'ms'",
            "name + 's'",
            "'5s'",
            "wait > 1 ? '1s' : name",
        ] {
            assert_eq!(unbounded_delay(bounded, &ctx), None, "{bounded}");
        }
        let why = unbounded_delay("Free() + 'ms'", &ctx).expect("a call with no bound has none");
        assert!(why.starts_with("`Free()`"), "{why}");
    }

    /// The type of one value of the scalar `t`.
    fn scalar(t: SceType) -> AlgorithmValueType {
        AlgorithmValueType::of_scalar(t)
    }

    /// A child's top-level `<data>` of `value_type`, starting at `expr`.
    fn variable(value_type: AlgorithmValueType, expr: &str) -> Variable {
        Variable {
            id: "x".to_string(),
            expr: expr.to_string(),
            value_type: Some(value_type),
            ..Variable::default()
        }
    }

    /// A list a call returns is taken whole into a list variable exactly when
    /// its element is the variable's and its bound at most the variable's; the
    /// refusal names the element first and the bound after, as "fits" judges
    /// them (docs/adr/0086, section 5.1). Every element against every element,
    /// every returned bound against every variable bound and none.
    #[test]
    fn a_returned_list_fits_a_variable_by_its_element_and_its_bound() {
        use crate::forge::value_type::{Bound, MisfitReason, ValueType};
        let elements = ["uint8", "int64", "record:Item"];
        let elem_of = |text: &str| match text.strip_prefix("record:") {
            Some(alias) => ValueType::Record {
                alias: alias.to_string(),
            },
            None => ValueType::Scalar(SceType::from_attr(text).expect("a scalar")),
        };
        for from_elem in elements {
            for to_elem in elements {
                for returned_max in [2u32, 4] {
                    for held_max in [Some(2u32), Some(4), None] {
                        let returned = ValueType::List {
                            elem: Box::new(elem_of(from_elem)),
                            max: Bound::of(returned_max),
                        };
                        let to = AlgorithmValueType::List {
                            elem: Box::new(elem_of(to_elem).written()),
                            max: held_max.map(Bound::of),
                        };
                        let target = variable(to, "");
                        let got = list_fits(&returned, &target, &StatechartNames)
                            .map_err(|m| (m.why, m.path));
                        let expected = if from_elem != to_elem {
                            Err((MisfitReason::Constructor, "/*".to_string()))
                        } else if held_max.is_some_and(|held| returned_max <= held) {
                            Ok(())
                        } else {
                            Err((MisfitReason::Bound, String::new()))
                        };
                        assert_eq!(
                            got, expected,
                            "list<{from_elem}>[{returned_max}] into list<{to_elem}>[{held_max:?}]"
                        );
                    }
                }
            }
        }
    }

    fn optional(elem: AlgorithmValueType) -> AlgorithmValueType {
        AlgorithmValueType::Optional {
            elem: Box::new(elem),
        }
    }

    /// A value is handed to a bool, a number or a text, and to an optional value of
    /// one of them, which takes the optional value (docs/adr/0055, step 10); a byte
    /// string, an enum, a record and a list take none, nor does an optional one of
    /// the first three.
    #[test]
    fn a_child_variable_is_handed_a_value_of_its_own_optional_type() {
        let slot = |value_type| seed_slot(&variable(value_type, "null"));
        assert_eq!(
            slot(optional(scalar(SceType::Int64))),
            Some(InferredType::Optional(OptionalElem::of(&SceType::Int64)))
        );
        assert_eq!(
            slot(optional(scalar(SceType::String))),
            Some(InferredType::Optional(OptionalElem::of(&SceType::String)))
        );
        assert_eq!(
            slot(optional(scalar(SceType::Bool))),
            Some(InferredType::Optional(OptionalElem::of(&SceType::Bool)))
        );
        assert_eq!(
            slot(scalar(SceType::Int64)),
            Some(InferredType::from_sce_type(&SceType::Int64))
        );
        assert_eq!(
            slot(scalar(SceType::String)),
            Some(InferredType::from_sce_type(&SceType::String))
        );
        assert_eq!(slot(optional(scalar(SceType::Bytes))), None);
        assert_eq!(
            slot(optional(AlgorithmValueType::Record {
                alias: "Item".to_string()
            })),
            None
        );
        assert_eq!(slot(scalar(SceType::Bytes)), None);
    }

    /// docs/adr/0108, D12 rule 1: a statement's target that starts at a map or a
    /// set variable is read step by step, each key of a map keeping its text and
    /// the map it keys, the place reached typed by walking the variable's type;
    /// a step that reaches nothing a statement writes through is refused saying
    /// which (an index of a list, a key of a set or of a value of one entry, a
    /// field of a value that is no record); and a target that starts at no map
    /// or set variable is none of these places.
    #[test]
    fn a_target_is_read_step_by_step_from_the_map_it_starts_at() {
        use crate::forge::type_ctx::PlaceStep;
        let declared = |id: &str, ty: &str| Variable {
            id: id.to_string(),
            value_type: Some(AlgorithmValueType::read(ty).expect("a type")),
            ..Variable::default()
        };
        let model = SCXMLModel {
            datamodel: Datamodel::SceStatic,
            variables: vec![
                declared("m", "map<uint32, map<uint32, uint32>[2]>[3]"),
                declared("lists", "map<uint32, list<uint8>[2]>[2]"),
                declared("seen", "set<uint32>[2]"),
                declared("total", "uint32"),
                declared("order", "list<uint32>[4]"),
            ],
            ..SCXMLModel::default()
        };
        let scope = StaticScope::of(&model).expect("a sce-static scope");
        let reached = |target: &str| {
            scope
                .map_place(target)
                .unwrap_or_else(|| panic!("`{target}` starts at a map"))
                .unwrap_or_else(|why| panic!("`{target}` is a place: {why}"))
        };
        let refused = |target: &str| {
            scope
                .map_place(target)
                .unwrap_or_else(|| panic!("`{target}` starts at a map"))
                .expect_err("a step reaches nothing written through")
        };

        // The variable itself, of no step.
        let whole = reached("m");
        assert_eq!(whole.variable.id, "m");
        assert!(whole.steps.is_empty());
        assert_eq!(
            whole.place.ty.to_string(),
            "map<uint32, map<uint32, uint32>[2]>[3]"
        );
        // The map held at a key of it: the key as written, and the map it keys.
        let inner = reached("m[k]");
        assert_eq!(inner.variable.id, "m");
        assert_eq!(
            inner.steps,
            [PlaceStep::Key {
                key: "k".to_string(),
                map: whole.place.clone(),
            }]
        );
        assert_eq!(inner.place.ty.to_string(), "map<uint32, uint32>[2]");
        // Two keys reach a value of one entry, each key in the order written.
        let deep = reached("m[k][j]");
        assert_eq!(deep.keys().collect::<Vec<_>>(), ["k", "j"]);
        assert_eq!(deep.place.ty.to_string(), "uint32");
        // A list held at a key.
        assert_eq!(reached("lists[3]").place.ty.to_string(), "list<uint8>[2]");

        // A list has no index step in a target, so a map in a list stays refused.
        let why = refused("lists[3][0]");
        assert!(
            why.contains("`lists[3]` is a list, which has no index step in a target"),
            "{why}"
        );
        let why = refused("seen[3]");
        assert!(why.contains("`seen` is a set"), "{why}");
        let why = refused("m[k][j][i]");
        assert!(
            why.contains("`m[k][j]` is a value of `uint32`, which holds no entry at a key"),
            "{why}"
        );
        let why = refused("m[k].tags");
        assert!(
            why.contains(
                "`m[k]` is a value of `map<uint32, uint32>[2]`, which has no field `tags`"
            ),
            "{why}"
        );

        // A target that starts at no map or set variable is no such place.
        for target in ["total", "order", "order[0]", "len(m)", "nothing[1]"] {
            assert!(scope.map_place(target).is_none(), "`{target}`");
        }
    }

    /// A value that may be absent stands for no value handed, so a child whose
    /// optional variable starts at a value is refused it, and one that starts at
    /// `null` takes it; a value that is never absent is taken by both.
    #[test]
    fn an_absent_value_is_handed_only_to_a_variable_that_starts_absent() {
        let int64 = || optional(scalar(SceType::Int64));
        let maybe = InferredType::Optional(OptionalElem::of(&SceType::Int64));
        let rule = |var: &Variable, ty| absent_handed_to_a_present_start(var, ty, "the child's");
        let present_start = variable(int64(), "3");
        let absent_start = variable(int64(), "null");
        let refusal = rule(&present_start, maybe).expect("an optional value is refused");
        assert!(refusal.contains("starts at `3`"), "{refusal}");
        assert!(
            rule(&present_start, InferredType::Null).is_some(),
            "`null` is an absent value"
        );
        assert!(rule(&absent_start, maybe).is_none());
        assert!(rule(&absent_start, InferredType::Null).is_none());
        assert!(rule(&present_start, InferredType::from_sce_type(&SceType::Int64)).is_none());
        // A variable that is not optional has no start to keep apart from absence.
        let plain = variable(scalar(SceType::Int64), "3");
        assert!(rule(&plain, maybe).is_none());
    }
}
