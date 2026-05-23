// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Item C1 — EventSchema receive-side typecheck.
//
// Resolves `_event.data.<field>` member-access expressions in transition
// `cond` attributes against the [`EventSchemaModel`] declared for the
// transition's `event`. Unknown fields surface as
// `validation/cross-kind-field-not-found` (with did-you-mean candidates
// drawn from the schema's declared field names). Type mismatches in
// comparisons surface as `validation/cross-kind-type-mismatch` (reused
// per Item 4 precedent — see `nl_to_ir_mapping_roadmap.md` Item 4 memo).
//
// Atomic A scope (design RFC §3 DL-5):
//   * Field-not-found rejection on every `_event.data.<unknown>` read.
//   * Type-mismatch rejection on the immediate parent of an
//     `_event.data.<field>` member access when that parent is a typed
//     comparison against a primitive literal whose type does not unify
//     with the field's declared `sce_type`.
//
// Out of scope at Atomic A (deferred to Atomic B):
//   * Send-side payload validation (`<send>/<param>` vs schema fields).
//   * Mesh cross-machine sender/receiver schema match.
//
// Out of scope at Atomic A (deferred to Atomic C):
//   * Cross-doc Enum/Lookup key-width resolution. EventSchema-declared
//     `SceType::Enum(LookupRef)` typechecks here use the *underlying
//     integer position* of the comparison — the Lookup's declared
//     `key_type` is treated as opaque at Atomic A. Atomic C lands the
//     enum-typed codegen and threads through the resolved key width so
//     the validator can reject `_event.data.<enum_field> === "ok"` while
//     accepting `_event.data.<enum_field> === 0x11` only when the
//     lookup's key_type is the matching unsigned width.

use std::collections::BTreeMap;
use std::path::Path;

use crate::forge::error::{ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::expr::{parse_to_ast, BinOp, ExprKind, TypedExpr};
use crate::forge::model::{EventSchemaModel, ForgeField, ForgeKind, SceType};
use crate::model::{Action, Param, SCXMLModel, Transition};

/// NL→IR Mapping Roadmap Item C1 (DL-7 prerequisite) — resolve a
/// statechart's `<sce:import>` declarations into the per-statechart
/// `event_name → EventSchemaModel` map consumed by the receive-side
/// and send-side validators.
///
/// `event_schemas_by_doc_name` is the orchestrator's build-wide
/// EventSchema registry keyed by file stem (the unique doc name).
/// `scxml.forge_imports` is filtered to entries whose `kind` is
/// `ForgeKind::EventSchema`; each surviving import's `src` is resolved
/// to its file stem and matched against the registry, then keyed
/// in the returned map by the schema's declared `sce:event-name` so
/// the validators can look up by `<transition event="X">` or
/// `<send event="X">` directly.
///
/// Two pre-build-time rejections happen here:
///
///   * An `<sce:import kind="event-schema" src="X.scxml">` whose
///     `X.scxml` does not appear in the build-wide registry —
///     surfaces as the existing `validation/cross-kind-circular-
///     dependency`-style import resolution failure earlier in the
///     pipeline (`validate_and_enrich_imports`); the resolver here
///     treats unresolvable srcs as silent skips since the pipeline
///     has already rejected.
///   * Two distinct EventSchema imports on the *same* statechart
///     that declare the *same* event name — receive-side validator
///     could not decide which field set to enforce on `_event.data`
///     for that event name. The orchestrator surfaces this via
///     `validation/incompatible-attributes` at the statechart
///     boundary; the resolver here records the first occurrence
///     and silently skips the second (the pipeline-level rejection
///     is the load-bearing diagnostic).
///
/// Returns the per-statechart `event_name → EventSchemaModel` view.
/// Empty when the statechart declares no event-schema imports (the
/// schemaless-fallback path keeps cost at zero).
pub fn resolve_imported_event_schemas(
    scxml: &SCXMLModel,
    event_schemas_by_doc_name: &BTreeMap<String, EventSchemaModel>,
) -> BTreeMap<String, EventSchemaModel> {
    let mut resolved: BTreeMap<String, EventSchemaModel> = BTreeMap::new();
    for import in &scxml.forge_imports {
        if !matches!(import.kind, ForgeKind::EventSchema) {
            continue;
        }
        let Some(stem) = Path::new(&import.src).file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(schema) = event_schemas_by_doc_name.get(stem) else {
            continue;
        };
        // First occurrence wins; the per-statechart duplicate-event-
        // name case is rejected upstream as `validation/incompatible-
        // attributes` so the resolver's silent skip here is the
        // conservative-defensive path (no diagnostic double-emit).
        // Owned clones (rather than references) keep the resolver's
        // output shape compatible with the existing `check` /
        // `check_send_side` signatures — schemas are author-bounded
        // small structs and the per-statechart resolution runs once
        // per statechart per build.
        resolved
            .entry(schema.event_name.clone())
            .or_insert_with(|| schema.clone());
    }
    resolved
}

/// Per-statechart receive-side typecheck.
///
/// `imported_schemas` maps SCXML event names (e.g. `"job.completed"`) to
/// their parsed [`EventSchemaModel`]. The orchestrator builds this map
/// from the resolved import table, filtering to documents whose
/// [`ForgeKind`] is [`ForgeKind::EventSchema`].
///
/// Returns `Ok(())` when every `_event.data.<field>` reference in every
/// transition `cond` resolves to a declared field whose type is
/// compatible with the comparison context. Returns the first failing
/// diagnostic at the offending transition.
///
/// `diag_label` is the statechart's diagnostic-label string (typically
/// the basename of the `.scxml` file) — threaded through every emitted
/// `Located<ForgeError>` so consumers can route the rejection back to
/// the offending file.
pub fn check(
    scxml: &SCXMLModel,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if imported_schemas.is_empty() {
        // Schemaless fallback (DL-9): no imported schemas means every
        // event keeps the dynamic `_event.data` baseline. Skip the
        // walk entirely so a statechart that opts not to declare
        // schemas pays no validator cost.
        return Ok(());
    }
    for state in scxml.states.values() {
        for transition in &state.transitions {
            check_transition(transition, imported_schemas, &scxml.name, diag_label)?;
        }
    }
    Ok(())
}

fn check_transition(
    transition: &Transition,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    // Schemaless fallback (DL-9): if the transition's event name does
    // not match any imported schema, leave the cond unvalidated. The
    // dynamic-payload baseline behavior of `_event.data` predates
    // EventSchema and remains the contract for un-schema'd events.
    let Some(schema) = imported_schemas.get(&transition.event) else {
        return Ok(());
    };

    let cond = transition.cond.trim();
    if cond.is_empty() {
        return Ok(());
    }

    // Expression parse failures themselves surface through the existing
    // typed-expression pipeline at codegen time; the receive-side
    // typecheck is opportunistic on top of that pipeline, so a parse
    // failure here means the cond is also un-typecheckable on the
    // generator side and the user will see the diagnostic there. Swallow
    // the error rather than duplicate it.
    let Ok(ast) = parse_to_ast(cond) else {
        return Ok(());
    };

    walk_for_event_data_refs(&ast, schema, transition, statechart_name, diag_label)?;
    Ok(())
}

/// Recursive AST walk. For every `_event.data.<field>` member access we
/// encounter, verify that `<field>` exists on the schema. For every
/// comparison node whose immediate left/right operand is such a member
/// access, additionally verify primitive-literal type compatibility.
fn walk_for_event_data_refs(
    expr: &TypedExpr,
    schema: &EventSchemaModel,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    match &expr.kind {
        ExprKind::Member { object, property } => {
            if is_event_data_object(object) {
                // `_event.data.<property>` — resolve `<property>` against
                // the schema. Field-not-found is the primary Atomic A
                // diagnostic.
                resolve_field(schema, property, transition, statechart_name, diag_label)?;
            }
            // Recurse into the object (covers nested patterns such as
            // `_event.data.foo.bar` — Atomic A treats the outer
            // `_event.data.foo` as a field reference; deeper paths are
            // currently unverified because schema fields are flat
            // typed primitives, not nested records).
            walk_for_event_data_refs(object, schema, transition, statechart_name, diag_label)?;
        }
        ExprKind::Binary { op, left, right } => {
            // Order: field-not-found checks first via recursive walk,
            // then comparison-type-mismatch layered on top.
            walk_for_event_data_refs(left, schema, transition, statechart_name, diag_label)?;
            walk_for_event_data_refs(right, schema, transition, statechart_name, diag_label)?;
            if is_comparison(*op) {
                if let Some(field) = extract_event_data_field(left, schema) {
                    check_comparison_type(field, right, transition, statechart_name, diag_label)?;
                }
                if let Some(field) = extract_event_data_field(right, schema) {
                    check_comparison_type(field, left, transition, statechart_name, diag_label)?;
                }
            }
        }
        ExprKind::Unary { operand, .. } => {
            walk_for_event_data_refs(operand, schema, transition, statechart_name, diag_label)?;
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            walk_for_event_data_refs(condition, schema, transition, statechart_name, diag_label)?;
            walk_for_event_data_refs(consequent, schema, transition, statechart_name, diag_label)?;
            walk_for_event_data_refs(alternate, schema, transition, statechart_name, diag_label)?;
        }
        ExprKind::Call { callee, args } => {
            walk_for_event_data_refs(callee, schema, transition, statechart_name, diag_label)?;
            for arg in args {
                walk_for_event_data_refs(arg, schema, transition, statechart_name, diag_label)?;
            }
        }
        ExprKind::Index { object, index } => {
            walk_for_event_data_refs(object, schema, transition, statechart_name, diag_label)?;
            walk_for_event_data_refs(index, schema, transition, statechart_name, diag_label)?;
        }
        // Leaf nodes — nothing to walk.
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit
        | ExprKind::Ident(_)
        | ExprKind::Raw(_) => {}
    }
    Ok(())
}

/// Test whether `expr` is the `_event.data` member access (the canonical
/// receiver-side payload accessor). Recognised shape:
/// `Member { object: Ident("_event"), property: "data" }`.
fn is_event_data_object(expr: &TypedExpr) -> bool {
    let ExprKind::Member { object, property } = &expr.kind else {
        return false;
    };
    if property != "data" {
        return false;
    }
    let ExprKind::Ident(name) = &object.kind else {
        return false;
    };
    name == "_event"
}

/// If `expr` is `_event.data.<field>` and `<field>` resolves on `schema`,
/// return a reference to the resolved field. Returns `None` for
/// non-`_event.data.<field>` shapes and for `_event.data.<unknown>`
/// (the field-not-found case is raised separately by the recursive
/// walker — this helper is purely for downstream typecheck dispatch).
fn extract_event_data_field<'s>(
    expr: &TypedExpr,
    schema: &'s EventSchemaModel,
) -> Option<&'s ForgeField> {
    let ExprKind::Member { object, property } = &expr.kind else {
        return None;
    };
    if !is_event_data_object(object) {
        return None;
    }
    schema.fields.iter().find(|f| f.id == *property)
}

/// Field-not-found check. Emits `validation/cross-kind-field-not-found`
/// (existing variant, reused per Item 4 precedent) with did-you-mean
/// candidates drawn from the schema's declared field names.
fn resolve_field(
    schema: &EventSchemaModel,
    field_name: &str,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if schema.fields.iter().any(|f| f.id == field_name) {
        return Ok(());
    }
    let mut candidates: Vec<String> = schema.fields.iter().map(|f| f.id.clone()).collect();
    candidates.sort();
    candidates.dedup();
    Err(located_on_transition(
        transition,
        diag_label,
        ValidationError::CrossKindFieldNotFound {
            // The importing surface is the SCXML statechart consuming
            // the schema. Atomic A's first wiring routes the
            // statechart's name as the importing-name; the alias
            // surfaces the authored token `_event.data` verbatim so
            // the user sees the exact site they wrote.
            importing_kind: ForgeKind::Statechart,
            importing_name: statechart_name.to_string(),
            alias: "_event.data".to_string(),
            field: field_name.to_string(),
            imported_kind: ForgeKind::EventSchema,
            imported_name: schema.name.clone(),
            candidates,
        },
    ))
}

/// Comparison type-mismatch check. Layered atop `walk_for_event_data_refs`
/// — for an expression shape `_event.data.<field> === <literal>` (or
/// reversed), compare `<literal>`'s lexically determinable type against
/// `<field>`'s declared `sce_type`. Mismatches raise
/// `validation/cross-kind-type-mismatch` (reused per Item 4 precedent).
///
/// Atomic A scope: literal vs primitive type. The other operand may
/// also be an arbitrary expression (e.g. a variable reference), in
/// which case the inferred type is not statically determinable from
/// the local view; Atomic A skips the check in that case (the existing
/// typed-expression pipeline catches deeper mismatches at codegen).
fn check_comparison_type(
    field: &ForgeField,
    other_operand: &TypedExpr,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let other_kind = match operand_literal_kind(other_operand) {
        Some(kind) => kind,
        None => return Ok(()),
    };
    if literal_is_compatible_with(&field.sce_type, other_kind) {
        return Ok(());
    }
    Err(located_on_transition(
        transition,
        diag_label,
        ValidationError::CrossKindTypeMismatch {
            importing_kind: ForgeKind::Statechart,
            importing_name: statechart_name.to_string(),
            alias: "_event.data".to_string(),
            field: field.id.clone(),
            actual: literal_kind_canonical(other_kind),
            expected: sce_type_canonical(&field.sce_type),
        },
    ))
}

/// The rough type of a comparison's non-`_event.data` operand. Restricted
/// to literal shapes whose category can be decided syntactically — the
/// receive-side validator declines to type-check operands that depend on
/// deeper inference (variables, nested computations, calls), leaving
/// those to the existing typed-expression pipeline at codegen time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiteralKind {
    Int,
    Float,
    Bool,
    String,
}

fn operand_literal_kind(expr: &TypedExpr) -> Option<LiteralKind> {
    match &expr.kind {
        // ExprKind::NumberLit holds the verbatim source text; the
        // syntactic int-vs-float distinction matches the rule used by
        // the per-language literal emitters: a number lit is an integer
        // iff it has no `.`, `e`, or `E`.
        ExprKind::NumberLit(text) => {
            if number_text_looks_like_int(text) {
                Some(LiteralKind::Int)
            } else {
                Some(LiteralKind::Float)
            }
        }
        ExprKind::BoolLit(_) => Some(LiteralKind::Bool),
        ExprKind::StringLit { .. } => Some(LiteralKind::String),
        // Unary minus on an integer literal is still an int comparison;
        // common on signed comparisons (`_event.data.delta === -1`).
        // Other unary operators (`!`, `~`) flip the category and are
        // not handled here — the existing typed pipeline catches those
        // at codegen.
        ExprKind::Unary { operand, .. } => operand_literal_kind(operand),
        _ => None,
    }
}

/// Mirror of `forge::generator::looks_like_int` for `NumberLit`'s
/// verbatim text. Re-implemented here rather than imported to keep
/// `event_schema_check` free of generator-internal helpers.
fn number_text_looks_like_int(n: &str) -> bool {
    !n.contains('.') && !n.contains('e') && !n.contains('E')
}

/// Decide whether a literal of `literal_kind` is a permissible operand
/// for a comparison against a value of `field_type`. The lattice rule:
///
///   * Unsigned-int / signed-int field types accept `Int` literals.
///   * Float field types accept `Int` and `Float` literals (decimal
///     promotion).
///   * Bool field types accept `Bool` literals.
///   * String / Bytes field types accept `String` literals (Bytes
///     compares against string literals through the existing
///     equality-as-bytes coercion at codegen).
///   * `SceType::Enum` accepts `Int` literals (per the lattice rule
///     `Enum(L) ⊑ <L's key_type>` — the key type is unsigned-int at
///     every declared lookup precedent). At Atomic A scope the
///     lookup's exact key width is treated as opaque; rejecting
///     non-int literals is the conservative correct check.
fn literal_is_compatible_with(field_type: &SceType, literal_kind: LiteralKind) -> bool {
    match field_type {
        SceType::Uint8
        | SceType::Uint16
        | SceType::Uint32
        | SceType::Uint64
        | SceType::Int8
        | SceType::Int16
        | SceType::Int32
        | SceType::Int64 => matches!(literal_kind, LiteralKind::Int),
        SceType::Float32 | SceType::Float64 => {
            matches!(literal_kind, LiteralKind::Int | LiteralKind::Float)
        }
        SceType::Bool => matches!(literal_kind, LiteralKind::Bool),
        SceType::String | SceType::Bytes => matches!(literal_kind, LiteralKind::String),
        // NL→IR Item C1: enum-typed field accepts integer literals
        // only (Atomic A treats the underlying key width as opaque;
        // Atomic C threads the resolved width through and may further
        // narrow `Int` to "integer fits in <key_width>"). String /
        // bool / float literals against an enum field are always
        // wrong.
        SceType::Enum(_) => matches!(literal_kind, LiteralKind::Int),
    }
}

fn literal_kind_canonical(kind: LiteralKind) -> String {
    match kind {
        LiteralKind::Int => "integer".to_string(),
        LiteralKind::Float => "float".to_string(),
        LiteralKind::Bool => "bool".to_string(),
        LiteralKind::String => "string".to_string(),
    }
}

fn sce_type_canonical(t: &SceType) -> String {
    match t {
        SceType::Uint8 => "uint8".to_string(),
        SceType::Uint16 => "uint16".to_string(),
        SceType::Uint32 => "uint32".to_string(),
        SceType::Uint64 => "uint64".to_string(),
        SceType::Int8 => "int8".to_string(),
        SceType::Int16 => "int16".to_string(),
        SceType::Int32 => "int32".to_string(),
        SceType::Int64 => "int64".to_string(),
        SceType::Float32 => "float32".to_string(),
        SceType::Float64 => "float64".to_string(),
        SceType::Bool => "bool".to_string(),
        SceType::String => "string".to_string(),
        SceType::Bytes => "bytes".to_string(),
        SceType::Enum(lookup_ref) => format!("enum:{}", lookup_ref.alias),
    }
}

fn is_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::StrictEq | BinOp::StrictNeq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq
    )
}

/// Anchor a [`ValidationError`] on a transition's recorded source
/// location, falling back to the statechart's `diag_label` when the
/// transition has no location (legacy fixture paths that predate the
/// `<transition>` source-position capture).
fn located_on_transition(
    transition: &Transition,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    let (line, col) = match transition.source_location.as_ref() {
        Some(loc) => (loc.line, loc.col),
        None => (None, None),
    };
    Located::new(ForgeError::Validation(Box::new(err)), diag_label, line, col)
}

/// Per-statechart send-side typecheck (DL-4).
///
/// Walks every `<send event="X">` / `<raise event="X">` executable-
/// content action reachable from the SCXML's state graph and verifies
/// each `<param name="F" expr="..."/>` against the imported
/// [`EventSchemaModel`] for `X`:
///
/// * `<param name="F">` whose `F` is not a declared field on the
///   schema → [`ValidationError::EventPayloadFieldUnknown`] with the
///   schema's field surface as a closed `Fix::ReplaceOneOf` candidate
///   set. Mirrors the receive-side
///   [`ValidationError::CrossKindFieldNotFound`] shape.
/// * `<param expr="…">` whose expression is a primitive literal whose
///   type does not unify with the field's declared `sce_type` →
///   [`ValidationError::CrossKindTypeMismatch`] (reused per Item 4
///   precedent — see [`check_comparison_type`] for the receive-side
///   mirror).
///
/// Non-literal `<param expr="…">` expressions (variable references,
/// nested computations, function calls) are deferred to the existing
/// typed-expression pipeline at codegen time, matching the receive-
/// side opportunistic-typecheck contract: the validator declines
/// operands whose category is not syntactically determinable.
///
/// Schemaless events (no imported schema for the send's event name)
/// are skipped per the [DL-9] fallback. Dynamic event names
/// (`<send eventexpr="…">` with no `event=` attribute) are also
/// skipped — the validator cannot resolve a schema without a stable
/// event name.
///
/// Returns `Ok(())` when every `<send>` / `<raise>` payload validates,
/// or the first failing diagnostic.
pub fn check_send_side(
    scxml: &SCXMLModel,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if imported_schemas.is_empty() {
        // DL-9 fallback — no imported schemas means every event keeps
        // the dynamic-payload baseline. Skip the walk entirely so
        // statecharts that opt not to declare schemas pay no
        // validator cost.
        return Ok(());
    }
    for state in scxml.states.values() {
        // Transition actions — `<send>` / `<raise>` inside transition
        // bodies are the primary site authors target.
        for transition in &state.transitions {
            walk_actions(
                &transition.actions,
                imported_schemas,
                &scxml.name,
                diag_label,
            )?;
        }
        // Onentry / onexit handler bodies — each block is a
        // document-ordered sequence of executable-content actions
        // (W3C SCXML §3.8, §3.9).
        for block in &state.on_entry_blocks {
            walk_actions(block, imported_schemas, &scxml.name, diag_label)?;
        }
        for block in &state.on_exit_blocks {
            walk_actions(block, imported_schemas, &scxml.name, diag_label)?;
        }
        // Initial-transition + history-default action sequences (W3C
        // SCXML §3.11) carry the same executable-content shape as the
        // transition bodies above.
        walk_actions(
            &state.initial_transition_actions,
            imported_schemas,
            &scxml.name,
            diag_label,
        )?;
        walk_actions(
            &state.initial_history_default_actions,
            imported_schemas,
            &scxml.name,
            diag_label,
        )?;
    }
    Ok(())
}

/// Recursive descent over a sequence of executable-content actions.
/// Drives into the `<if>` / `<elseif>` / `<else>` / `<foreach>`
/// composite shapes so a `<send>` nested two-or-more levels deep
/// still surfaces for typecheck.
fn walk_actions(
    actions: &[Action],
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    for action in actions {
        check_send_action(action, imported_schemas, statechart_name, diag_label)?;
        // Composite-action bodies. The shapes are mutually exclusive
        // by action_type (`<if>` populates then/elseif/else, `<foreach>`
        // populates the `actions` field) but the model serialises them
        // as parallel `Vec<Action>` fields, so we walk each
        // unconditionally — empty vecs cost nothing.
        walk_actions(
            &action.then_actions,
            imported_schemas,
            statechart_name,
            diag_label,
        )?;
        for branch in &action.elseif_branches {
            walk_actions(
                &branch.actions,
                imported_schemas,
                statechart_name,
                diag_label,
            )?;
        }
        walk_actions(
            &action.else_actions,
            imported_schemas,
            statechart_name,
            diag_label,
        )?;
        walk_actions(
            &action.actions,
            imported_schemas,
            statechart_name,
            diag_label,
        )?;
    }
    Ok(())
}

/// Per-`<send>` / `<raise>` validator. Returns `Ok(())` for any
/// action that is not a send/raise, has no statically-resolvable
/// event name, or whose event name does not resolve to an imported
/// schema (DL-9 schemaless fallback).
fn check_send_action(
    action: &Action,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if action.action_type != "send" && action.action_type != "raise" {
        return Ok(());
    }
    // Dynamic event name (`<send eventexpr="…">`) — the event the
    // payload will carry is not statically resolvable. Validator
    // cannot pick a schema; skip per DL-9 fallback shape.
    if action.event.is_empty() {
        return Ok(());
    }
    let Some(schema) = imported_schemas.get(&action.event) else {
        return Ok(());
    };
    for param in &action.params {
        check_send_param(param, action, schema, statechart_name, diag_label)?;
    }
    Ok(())
}

/// Per-`<param>` validator. Two failure modes per DL-4:
///
/// * `<param name="F">` with `F` not on the schema —
///   [`ValidationError::EventPayloadFieldUnknown`] with the schema's
///   sorted, deduplicated field surface as the closed candidate set.
/// * `<param expr="…">` whose expression is a primitive literal
///   incompatible with the field's declared type —
///   [`ValidationError::CrossKindTypeMismatch`] (reused per Item 4
///   precedent).
///
/// W3C SCXML 6.2.4 mandates exactly one of `expr` / `location` on
/// every `<param>`; the location form (`<param name="X" location="Y"/>`,
/// data-model variable assignment) is not statically typeable
/// without the typed datamodel pipeline and is left for the existing
/// typed-expression machinery at codegen time.
fn check_send_param(
    param: &Param,
    action: &Action,
    schema: &EventSchemaModel,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let Some(field) = schema.fields.iter().find(|f| f.id == param.name) else {
        let mut candidates: Vec<String> = schema.fields.iter().map(|f| f.id.clone()).collect();
        candidates.sort();
        candidates.dedup();
        return Err(located_on_action(
            action,
            diag_label,
            ValidationError::EventPayloadFieldUnknown {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                event_name: action.event.clone(),
                field: param.name.clone(),
                imported_kind: ForgeKind::EventSchema,
                imported_name: schema.name.clone(),
                candidates,
            },
        ));
    };
    // Literal-shape typecheck: only `<param expr="…">` whose `expr`
    // is a primitive literal participates here. `<param location="…">`
    // and non-literal expressions defer to the typed-expression
    // pipeline.
    let expr_text = param.expr.trim();
    if expr_text.is_empty() {
        return Ok(());
    }
    let Ok(expr_ast) = parse_to_ast(expr_text) else {
        return Ok(());
    };
    let Some(literal_kind) = operand_literal_kind(&expr_ast) else {
        return Ok(());
    };
    if literal_is_compatible_with(&field.sce_type, literal_kind) {
        return Ok(());
    }
    Err(located_on_action(
        action,
        diag_label,
        ValidationError::CrossKindTypeMismatch {
            importing_kind: ForgeKind::Statechart,
            importing_name: statechart_name.to_string(),
            alias: format!("<send event=\"{}\">", action.event),
            field: field.id.clone(),
            actual: literal_kind_canonical(literal_kind),
            expected: sce_type_canonical(&field.sce_type),
        },
    ))
}

/// Anchor a [`ValidationError`] on an action's recorded source
/// location, falling back to the statechart's `diag_label` when the
/// action has no captured location (legacy parse paths that predate
/// the per-executable-content source-position capture).
fn located_on_action(
    action: &Action,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    let (line, col) = match action.source_location.as_ref() {
        Some(SourceLocation {
            line: Some(l),
            col: Some(c),
            ..
        }) => (Some(*l), Some(*c)),
        Some(SourceLocation { line, col, .. }) => (*line, *col),
        None => (None, None),
    };
    Located::new(ForgeError::Validation(Box::new(err)), diag_label, line, col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{Direction, ForgeField, LookupRef, SceType};

    fn field(id: &str, ty: SceType) -> ForgeField {
        ForgeField {
            id: id.to_string(),
            sce_type: ty,
            direction: Direction::In,
            expr: None,
            quantity: None,
            max_size: None,
        }
    }

    fn schema(event: &str, fields: Vec<ForgeField>) -> EventSchemaModel {
        EventSchemaModel {
            name: format!("{event}_schema"),
            event_name: event.to_string(),
            fields,
            source_location: None,
        }
    }

    fn run(schema: EventSchemaModel, cond: &str) -> Result<(), Located<ForgeError>> {
        let ast = parse_to_ast(cond).expect("valid cond");
        let transition = Transition {
            event: schema.event_name.clone(),
            cond: cond.to_string(),
            ..Default::default()
        };
        walk_for_event_data_refs(
            &ast,
            &schema,
            &transition,
            "test_statechart",
            "test_diag_label",
        )
    }

    #[test]
    fn known_field_passes() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        assert!(run(s, "_event.data.status === 0").is_ok());
    }

    #[test]
    fn unknown_field_rejects() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        let err = run(s, "_event.data.missing === 0").expect_err("should reject");
        let msg = format!("{err}");
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("status"), "{msg}");
    }

    #[test]
    fn int_literal_against_uint32_passes() {
        let s = schema("job.completed", vec![field("count", SceType::Uint32)]);
        assert!(run(s, "_event.data.count === 42").is_ok());
    }

    #[test]
    fn string_literal_against_uint32_rejects() {
        let s = schema("job.completed", vec![field("count", SceType::Uint32)]);
        let err = run(s, "_event.data.count === 'forty-two'").expect_err("should reject");
        let msg = format!("{err}");
        assert!(msg.contains("string"), "{msg}");
        assert!(msg.contains("uint32"), "{msg}");
    }

    #[test]
    fn int_literal_against_enum_passes() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(LookupRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        assert!(run(s, "_event.data.status === 0").is_ok());
    }

    #[test]
    fn string_literal_against_enum_rejects() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(LookupRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let err = run(s, "_event.data.status === 'ok'").expect_err("should reject");
        let msg = format!("{err}");
        assert!(msg.contains("string"), "{msg}");
        assert!(msg.contains("enum:Result"), "{msg}");
    }

    #[test]
    fn empty_schema_map_skips_check() {
        let scxml = SCXMLModel {
            name: "test".to_string(),
            ..Default::default()
        };
        let schemas = BTreeMap::new();
        check(&scxml, &schemas, "diag").expect("empty map should pass trivially");
    }
}
