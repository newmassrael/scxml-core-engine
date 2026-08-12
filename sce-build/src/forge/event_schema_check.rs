// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// EventSchema receive-side + send-side typecheck + per-statechart
// schema visibility resolver.
//
// Resolves `_event.data.<field>` member-access expressions in transition
// `cond` attributes against the [`EventSchemaModel`] declared for the
// transition's `event`. Unknown fields surface as
// `validation/cross-kind-field-not-found` (with did-you-mean candidates
// drawn from the schema's declared field names). Type mismatches in
// comparisons surface as `validation/cross-kind-type-mismatch` (reused — no new wire code).
//
// Send-side: walks every `<send event="X">` / `<raise event="X">`
// executable-content action and verifies each `<param name="F"/>`
// against the schema's declared field surface. Unknown field surfaces
// `validation/event-payload-field-unknown` (FixCarriesCandidates over
// the schema's declared field set); literal-type mismatch surfaces
// `validation/cross-kind-type-mismatch` (reused — no new wire code).
//
// Receive-side + send-side check scope:
//   * Receive-side field-not-found rejection on every
//     `_event.data.<unknown>` read.
//   * Receive-side type-mismatch rejection on the immediate parent of
//     an `_event.data.<field>` member access when that parent is a
//     typed comparison against a primitive literal whose type does
//     not unify with the field's declared `sce_type`.
//   * Send-side field-unknown + literal-type-mismatch on `<param>` of
//     `<send>` / `<raise>` (transitions, on_entry/on_exit, initial-
//     transition + history-default, nested <if>/<foreach>).
//
// Width-narrowing scope:
//   * Cross-doc Enum underlying-type width narrowing — integer
//     literals compared against (receive-side) or assigned to
//     (send-side) an enum-typed field are now narrowed against the
//     enum's declared `underlying_type`. Overflows surface as
//     `validation/cross-kind-type-mismatch` (reused — no new
//     wire code).
//
// Strict-variant-membership scope:
//   * After width narrowing accepts a literal, the membership layer
//     verifies the value is one of the enum's declared
//     `<sce:variant value="…"/>` set. Closed-set vocabularies (the
//     default) reject `_event.data.status === 7` against an enum
//     declaring `{ok=0, error=1, timeout=2}`; open-set vocabularies
//     (`sce:strict-variants="false"` on the enum's `<scxml>` root —
//     e.g. UDS NRC, OEM-extensible response codes) silent-skip the
//     membership check while still enforcing width narrowing.
//   * Same `validation/cross-kind-type-mismatch` reuse (no new
//     wire code).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::forge::error::{ExprError, ForgeError, Located, SourceLocation, ValidationError};
use crate::forge::expr::{
    decode_bytes_literal, parse_to_ast, references_event_data_lexically, transpile_typed, BinOp,
    ExprKind, ExprTarget, TypedExpr, UnaryOp,
};
use crate::forge::model::{EnumModel, EventSchemaModel, ForgeField, ForgeKind, SceType};
use crate::forge::types::{InferredType, TypeCtx};
use crate::model::{Action, Param, SCXMLModel, Transition};

/// The W3C SCXML reserved system-variable path a typed guard reads its
/// payload through (§scxml-5.10) — the object [`lower_typed_guard`]
/// renames onto the generated payload binding, and the alias the
/// receive-side diagnostics name back to the author.
///
/// Whether a given `cond` *reaches for* that path is decided
/// structurally, never by matching this string against raw source: on the
/// AST by [`cond_references_event_data`], and on the token stream — for
/// expressions that do not parse — by
/// [`crate::forge::expr::references_event_data_lexically`].
const EVENT_DATA_PATH: &str = "_event.data";

/// Per-statechart schema-visibility resolution — resolve
/// a statechart's `<sce:import>` declarations into the per-statechart
/// `event_name → EventSchemaModel` map consumed by the receive-side
/// and send-side validators.
///
/// `event_schemas_by_doc_name` is the orchestrator's build-wide
/// EventSchema registry keyed by file stem (the unique doc name).
/// `scxml.forge_imports` is filtered to entries whose `kind` is
/// `ForgeKind::EventSchema`; each surviving import's `src` is resolved
/// to its file stem and matched against the registry, then keyed in
/// the returned map by the schema's declared `sce:event-name` so the
/// validators can look up by `<transition event="X">` or
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

/// Width-narrowing support — resolve the statechart's
/// `<sce:import kind="enum">` declarations into the per-statechart
/// `alias → EnumModel` map consumed by the literal-width narrowing
/// inside [`check_comparison_type`] and [`check_send_param`].
///
/// Shape mirrors [`resolve_imported_event_schemas`]: `enums_by_doc_name`
/// is the orchestrator's build-wide Enum registry keyed by file stem;
/// `scxml.forge_imports` is filtered to entries whose `kind` is
/// `ForgeKind::Enum`, each surviving import's `src` is resolved to its
/// file stem and matched against the registry, then keyed in the
/// returned map by the import's authored `alias` so the validators can
/// look up by `SceType::Enum(EnumRef { alias }).alias` directly.
///
/// Schema-of-record for the alias: each statechart's own imports table
/// is the alias surface for that statechart's enum-typed field
/// comparisons. A statechart whose schema's field declares
/// `sce:type="enum:Result"` must independently declare an
/// `<sce:import kind="enum" as="Result"/>` for the narrowing to fire;
/// otherwise the alias is opaque from the statechart's view and the
/// narrowing silent-skips (the underlying width was already opaque to
/// the field check, so silent-skip preserves the conservative-accept
/// default).
///
/// Returns empty when the statechart declares no Enum imports — the
/// non-narrowing baseline path remains zero-cost.
pub fn resolve_imported_enums(
    scxml: &SCXMLModel,
    enums_by_doc_name: &BTreeMap<String, EnumModel>,
) -> BTreeMap<String, EnumModel> {
    let mut resolved: BTreeMap<String, EnumModel> = BTreeMap::new();
    for import in &scxml.forge_imports {
        if !matches!(import.kind, ForgeKind::Enum) {
            continue;
        }
        let Some(stem) = Path::new(&import.src).file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(em) = enums_by_doc_name.get(stem) else {
            continue;
        };
        // First occurrence wins; per-statechart duplicate-alias is
        // already rejected upstream by `validate_and_enrich_imports`
        // as `validation/duplicate-alias` so the silent-skip here is
        // the conservative-defensive path (no diagnostic double-emit).
        resolved
            .entry(import.alias.clone())
            .or_insert_with(|| em.clone());
    }
    resolved
}

/// EventSchema MCU native lowering — the
/// single source of truth for "is this transition guard expressible
/// without a runtime script engine".
///
/// Lower a guard `cond` that reads `_event.data.<field>` (whose
/// triggering event carries `schema`) into a native `target` expression,
/// renaming the `_event.data` access object to `accessor` (the generated
/// state machine's bound typed-payload variable, e.g. the `if let`
/// binding of the payload-enum variant). Each schema field is registered
/// in the type context under its `_event.data.<field>` access path so the
/// generalized member-path inference in [`crate::forge::expr`] resolves
/// each leaf to its declared concrete type.
///
/// Returns the emitted native expression on success. `Err` means the
/// guard cannot be lowered for `target`: there is deliberately NO
/// allow-list of permitted AST nodes — "native-conforming" is defined
/// purely as "the typed-expression pipeline emits it". The engine-need
/// analyzer ([`crate::script_engine_analyzer`]) treats `Err` as "keep the
/// script engine", and codegen emits `Ok(s)` as the native guard.
///
/// `Err` on its own is therefore NOT a rejection — falling back to the
/// script engine is a supported outcome (`static_hybrid`), not a failure.
/// What is not supported is falling back *silently*, so the two ways a
/// typed guard can fail to lower are separated at their source:
///
///   * it does not parse as a Forge expression — rejected up front by
///     [`check`], which is the only place that failure can be seen (this
///     function has already discarded it by the time it returns `Err`);
///   * it parses but has no native form (a datamodel identifier or call
///     with no binding inside the payload `matches!`) — a legal document
///     that keeps the script engine, and reports the fact through the
///     engine-need analyzer's [`crate::script_engine_analyzer::NeedsScriptEngineCause`]
///     list, which the stdout manifest carries so the degradation is
///     visible without being fatal.
pub fn lower_typed_guard(
    cond: &str,
    schema: &EventSchemaModel,
    accessor: &str,
    target: ExprTarget,
) -> Result<String, ExprError> {
    // The dotted access-path keys must outlive `ctx` (whose `vars` map
    // borrows its keys), so they are owned here and borrowed in.
    let keys: Vec<String> = schema
        .fields
        .iter()
        .map(|f| format!("_event.data.{}", f.id))
        .collect();
    let mut ctx = TypeCtx::new();
    for (field, key) in schema.fields.iter().zip(keys.iter()) {
        ctx.insert_var(key.as_str(), InferredType::from_sce_type(&field.sce_type));
    }
    let mut renames: HashMap<&str, &str> = HashMap::new();
    renames.insert(EVENT_DATA_PATH, accessor);
    transpile_typed(cond, target, &ctx, &renames, InferredType::Unknown)
}

/// `true` iff `schema` can back a native typed-payload channel. The
/// generated payload struct emits every declared field, so the channel
/// is only eligible when all fields resolve to a self-contained native
/// type. One field type is excluded:
///
///   * Enum-typed fields (`SceType::Enum`) carry their concrete width
///     in a *separate* imported Enum document whose `use`/`include`
///     would have to be threaded into the state-machine unit — until
///     that wiring exists, an enum-typed schema keeps the dynamic
///     `_event.data` baseline (the cond stays on the script engine)
///     rather than emit a payload struct that names an out-of-scope
///     type.
///
/// `bytes` fields ARE eligible: every backend carries the payload field
/// natively (C11 as a no-alloc bounded buffer `uint8_t[CAP]; size_t
/// _len`, the hosted backends as their owned byte-sequence type) and a
/// `bytes` guard lowers to a byte-identical equality on all six.
/// Forms that have no
/// representable native lowering — an ordering operator or a
/// non-printable-ASCII literal on a `bytes` operand — are kept off the
/// native path by [`guard_is_native_lowerable`]'s per-guard
/// representability check, not by excluding the schema:
/// a schema's other guards stay native.
///
/// This is the single eligibility rule shared by the engine-need
/// analyzer and the codegen path so the two never disagree.
pub fn schema_is_native_payload_eligible(schema: &EventSchemaModel) -> bool {
    schema
        .fields
        .iter()
        .all(|f| !matches!(f.sce_type, SceType::Enum(_)))
}

/// `true` iff `cond` lowers natively on every backend SCE generates —
/// i.e. the document needs no runtime script engine to evaluate this
/// guard. The native form is a self-contained `matches!` over the
/// payload-enum variant, so all of the following must hold:
///
/// (a) the schema is payload-eligible
///     ([`schema_is_native_payload_eligible`]);
/// (b) the cond reads at least one typed `_event.data.<field>` (a guard
///     that touches no payload field is not a typed guard — it keeps the
///     normal native/script-engine path);
/// (c) the cond references *nothing else* —
///     [`cond_is_pure_typed_payload`]. A datamodel identifier, function
///     call, or `In()` predicate has no binding inside the payload
///     `matches!` guard; emitting it would produce a bare undefined
///     identifier (e.g. `retry_count` instead of `self.retry_count`).
///     Such mixed conds are legal — they keep the script-engine path, and
///     the engine-need analyzer records the fallback as a
///     [`crate::script_engine_analyzer::NeedsScriptEngineCause::TransitionGuard`]
///     that the stdout manifest carries, so the machine never loses its
///     native lowering without saying so;
/// (d) the expression lowers on *every* backend SCE generates —
///     [`ExprTarget::ALL`]. The verdict feeds the language-neutral
///     `native_typed_inject_events` switchboard SSOT, which is
///     all-or-nothing across backends: a guard one backend can emit but
///     another cannot must not be classed native, or the SSOT splits
///     ([`select_native_typed_guards`] doc, RFC §bytesguard-1.3). Earlier this
///     proxied all six by checking only Rust + Go on the documented
///     assumption that the other emitters never fail. Iterating
///     [`ExprTarget::ALL`] removes the proxy structurally — the verdict
///     can never again silently diverge from a backend it did not
///     consult, and a future fallible emitter path is covered the day it
///     lands. The accessor is irrelevant to lowerability (any valid
///     identifier renames cleanly), so a fixed placeholder is used.
///
/// (e) every comparison touching a `bytes` field is a
///     natively-representable form —
///     [`bytes_comparisons_are_native_representable`]. A `bytes` operand
///     under an ordering operator, or compared against a
///     non-printable-ASCII literal, has no byte-identical native
///     lowering (RFC §bytesguard-3 B2/B3); the hosted emitters would mangle it
///     silently (Go emits an illegal slice `<`; Python a `False`-always
///     `bytes < str`) rather than fail, so the verdict cannot rely on
///     the per-target lowering in (d) to reject it. This explicit
///     pre-check makes the verdict self-sufficient: the analyzer
///     (`compute_typed_inject_events`) runs it with no validation
///     barrier ahead of it, so a malformed `bytes` guard must be classed
///     non-native here, independent of the receive-side validator that
///     separately rejects the same forms in the forge codegen path.
pub fn guard_is_native_lowerable(cond: &str, schema: &EventSchemaModel) -> bool {
    if !schema_is_native_payload_eligible(schema) {
        return false;
    }
    let Ok(ast) = parse_to_ast(cond) else {
        return false;
    };
    if !cond_references_event_data(&ast) || !cond_is_pure_typed_payload(&ast) {
        return false;
    }
    if !bytes_comparisons_are_native_representable(&ast, schema) {
        return false;
    }
    ExprTarget::ALL
        .iter()
        .all(|&target| lower_typed_guard(cond, schema, "ev", target).is_ok())
}

/// `true` iff every comparison touching a `bytes`-typed `_event.data`
/// field in `ast` is a form with a byte-identical native lowering on all
/// six backends: an equality (`===` / `!==`) against a printable-ASCII
/// string literal. Ordering operators (`<`, `>`, `<=`, `>=`) on a
/// `bytes` operand and equality against a non-decodable literal have no
/// such lowering (RFC §bytesguard-3 B2/B3) and make the cond non-native. Each
/// comparison is classified by [`classify_bytes_comparison`] — the SAME
/// SSOT rule the receive-side validator renders as a diagnostic — so the
/// verdict and the diagnostic can never disagree about which forms are
/// representable. Non-comparison nodes and comparisons that touch no
/// `bytes` field are transparently recursed/accepted — the
/// pure-typed-payload check (c) has already excluded everything but typed
/// fields, literals, and operators.
fn bytes_comparisons_are_native_representable(ast: &TypedExpr, schema: &EventSchemaModel) -> bool {
    match &ast.kind {
        ExprKind::Binary { op, left, right } => {
            for (field_side, other) in [(left, right), (right, left)] {
                let Some(field) = extract_event_data_field(field_side, schema) else {
                    continue;
                };
                // The same SSOT rule the validator uses: a bytes field
                // touched by a non-representable comparison (B2/B3) is not
                // natively lowerable.
                if matches!(field.sce_type, SceType::Bytes)
                    && classify_bytes_comparison(*op, other) != BytesComparisonForm::Representable
                {
                    return false;
                }
            }
            bytes_comparisons_are_native_representable(left, schema)
                && bytes_comparisons_are_native_representable(right, schema)
        }
        ExprKind::Unary { operand, .. } => {
            bytes_comparisons_are_native_representable(operand, schema)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            bytes_comparisons_are_native_representable(condition, schema)
                && bytes_comparisons_are_native_representable(consequent, schema)
                && bytes_comparisons_are_native_representable(alternate, schema)
        }
        _ => true,
    }
}

/// One transition whose typed `_event.data` guard lowers to a native
/// payload comparison: its owning `state_id` plus per-source-state
/// `transition_index` (the two together are the machine-unique transition
/// identity — `transition_index` alone is NOT unique across states), the
/// schema-carrying event it matches, and the raw `cond`. The per-language
/// payload builders ([`crate::forge::generator::build_rust_event_payload`]
/// and its C11/Go/C++/Kotlin/Python twins) format the guard expression and
/// the payload type definitions from this shared selection, then attach the
/// rendered guard to the transition located by `(state_id, transition_index)`.
pub(crate) struct NativeTypedGuard {
    pub(crate) state_id: String,
    pub(crate) transition_index: usize,
    pub(crate) event: String,
    pub(crate) cond: String,
}

/// Select every transition whose typed `_event.data` guard lowers
/// natively (event carries an imported, payload-eligible schema + the
/// cond passes [`guard_is_native_lowerable`]).
///
/// This is the single SSOT every backend payload builder shares so the
/// six never disagree about which guards are native — gating on a
/// per-backend local predicate would let one backend emit a native guard
/// the other (and the engine-need analyzer) treated as script-engine.
/// Returns an empty vec for the schemaless baseline (no imported schema,
/// or no guard lowers). Lives here next to the eligibility predicate
/// rather than in the codegen module so the language-neutral analyzer can
/// derive [`native_typed_inject_events`] from the identical selection.
pub(crate) fn select_native_typed_guards(
    model: &crate::model::SCXMLModel,
) -> Vec<NativeTypedGuard> {
    let mut guarded = Vec::new();
    if model.imported_event_schemas.is_empty() {
        return guarded;
    }
    for (state_id, state) in &model.states {
        for trans in &state.transitions {
            if trans.cond.trim().is_empty() {
                continue;
            }
            let Some(schema) = model.imported_event_schemas.get(&trans.event) else {
                continue;
            };
            if !guard_is_native_lowerable(&trans.cond, schema) {
                continue;
            }
            guarded.push(NativeTypedGuard {
                state_id: state_id.clone(),
                transition_index: trans.transition_index,
                event: trans.event.clone(),
                cond: trans.cond.clone(),
            });
        }
    }
    guarded
}

/// The set of event descriptors for which a per-event typed-inject seam
/// (`<Machine>Inject::raise_<event>`) is generated — i.e. the events whose
/// transition guard lowers to a native typed-payload comparison
/// ([`select_native_typed_guards`]). Language-neutral SSOT a transport
/// switchboard keys off to decide, per value binding, whether an event has
/// a typed value path (the generated `raise_<event>` inject seam) or only
/// the schemaless signal path (`Engine::raise_external_by_name`), without
/// re-deriving the native-lowering eligibility rule. Surfaced on the
/// statechart AST arm as [`crate::model::SCXMLModel::typed_inject_events`].
pub(crate) fn native_typed_inject_events(
    model: &crate::model::SCXMLModel,
) -> std::collections::BTreeSet<String> {
    select_native_typed_guards(model)
        .into_iter()
        .map(|g| g.event)
        .collect()
}

/// `true` iff `expr` reads at least one `_event.data.<field>` member
/// access anywhere in the tree.
fn cond_references_event_data(expr: &TypedExpr) -> bool {
    match &expr.kind {
        ExprKind::Member { object, .. } if is_event_data_object(object) => true,
        ExprKind::Member { object, .. } => cond_references_event_data(object),
        ExprKind::Binary { left, right, .. } => {
            cond_references_event_data(left) || cond_references_event_data(right)
        }
        ExprKind::Unary { operand, .. } => cond_references_event_data(operand),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            cond_references_event_data(condition)
                || cond_references_event_data(consequent)
                || cond_references_event_data(alternate)
        }
        ExprKind::Index { object, index } => {
            cond_references_event_data(object) || cond_references_event_data(index)
        }
        ExprKind::Call { callee, args } => {
            cond_references_event_data(callee) || args.iter().any(cond_references_event_data)
        }
        // RFC c7-wildcard W-project: a `BytesView` projection is produced
        // only by the algorithm-kind transpile; it cannot reach an
        // EventSchema typed-guard AST. The transparent recursion keeps the
        // predicate exhaustive without asserting unreachability.
        ExprKind::BytesView { source, .. } => cond_references_event_data(source),
        ExprKind::Ident(_)
        | ExprKind::Raw(_)
        | ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit => false,
    }
}

/// `true` iff every reference in `expr` is a `_event.data.<field>` access
/// — i.e. the cond is built only from typed payload fields, literals, and
/// operators. A bare identifier (datamodel variable), function call,
/// `In()` predicate, index, or nested member (`_event.data.a.b`) makes it
/// `false`: those cannot be expressed inside the payload-variant
/// `matches!` guard, which binds only the flat payload fields.
fn cond_is_pure_typed_payload(expr: &TypedExpr) -> bool {
    match &expr.kind {
        // `_event.data.<field>` — the only permitted leaf reference. The
        // object is the validated `_event.data` access; do not recurse.
        ExprKind::Member { object, .. } if is_event_data_object(object) => true,
        ExprKind::Binary { left, right, .. } => {
            cond_is_pure_typed_payload(left) && cond_is_pure_typed_payload(right)
        }
        ExprKind::Unary { operand, .. } => cond_is_pure_typed_payload(operand),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            cond_is_pure_typed_payload(condition)
                && cond_is_pure_typed_payload(consequent)
                && cond_is_pure_typed_payload(alternate)
        }
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit => true,
        // RFC c7-wildcard W-project: a `BytesView` is algorithm-kind only
        // and cannot reach this statechart-side predicate; recurse
        // transparently to stay exhaustive.
        ExprKind::BytesView { source, .. } => cond_is_pure_typed_payload(source),
        // Any other Member shape (`frame.x`, `_event.data.a.b`), bare
        // Ident (datamodel var), Call, Index, or Raw fragment references
        // something the payload `matches!` cannot bind.
        ExprKind::Member { .. }
        | ExprKind::Ident(_)
        | ExprKind::Call { .. }
        | ExprKind::Index { .. }
        | ExprKind::Raw(_) => false,
    }
}

/// Per-statechart receive-side typecheck.
///
/// `imported_schemas` maps SCXML event names (e.g. `"job.completed"`) to
/// their parsed [`EventSchemaModel`]. The orchestrator builds this map
/// via [`resolve_imported_event_schemas`] from the per-statechart
/// `<sce:import>` declarations.
///
/// Returns `Ok(())` when every `_event.data.<field>` reference in every
/// transition `cond` resolves to a declared field whose type is
/// compatible with the comparison context. Returns the first failing
/// diagnostic at the offending transition.
///
/// A `cond` that reads `_event.data` on a schema-carrying event but does
/// not parse as a Forge expression is rejected here with the `ExprError`
/// the expression pipeline raised — `==` on a typed guard is
/// `expression/strict-equality`, the build-time error SCE Forge §3.4
/// mandates. This is the only stage that can report it: every other
/// consumer of the parse (`guard_is_native_lowerable`, and through it the
/// engine-need analyzer) reduces the same failure to "not natively
/// lowerable", which is indistinguishable from a guard that legitimately
/// keeps the script engine.
///
/// `diag_label` is the statechart's diagnostic-label string (typically
/// the basename of the `.scxml` file) — threaded through every emitted
/// `Located<ForgeError>` so consumers can route the rejection back to
/// the offending file.
pub fn check(
    scxml: &SCXMLModel,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    imported_enums: &BTreeMap<String, EnumModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if imported_schemas.is_empty() {
        // Schemaless fallback: no imported schemas means every
        // event keeps the dynamic `_event.data` baseline. Skip the
        // walk entirely so a statechart that opts not to declare
        // schemas pays no validator cost.
        return Ok(());
    }
    for state in scxml.states.values() {
        for transition in &state.transitions {
            check_transition(
                transition,
                imported_schemas,
                imported_enums,
                &scxml.name,
                diag_label,
            )?;
        }
    }
    Ok(())
}

fn check_transition(
    transition: &Transition,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    imported_enums: &BTreeMap<String, EnumModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    // Schemaless fallback: if the transition's event name
    // does not match any imported schema, leave the cond unvalidated.
    // The dynamic-payload baseline behavior of `_event.data` predates
    // EventSchema and remains the contract for un-schema'd events.
    let Some(schema) = imported_schemas.get(&transition.event) else {
        return Ok(());
    };

    let cond = transition.cond.trim();
    if cond.is_empty() {
        return Ok(());
    }

    // Does this `cond` reach for typed event data? The answer decides
    // whether a Forge parse failure is a rejection or none of Forge's
    // business, so it must be answered *before* parsing — and therefore on
    // the token stream, which survives a parse the expression fails
    // ([`references_event_data_lexically`]).
    //
    //   * No `_event.data` → plain ECMAScript bound for the script engine,
    //     even though this transition's event carries a schema. The Forge
    //     expression language is a strict subset of ECMAScript, so
    //     rejecting every cond it cannot parse would reject W3C-legal
    //     documents that never asked for the typed path (`retry == 1` is
    //     legal there and stays legal). Left unvalidated, as it always was.
    //
    //   * `_event.data` present → the typed path, so the cond IS an
    //     Extended SCXML expression and must parse as one. Nothing
    //     downstream will ever say so: `guard_is_native_lowerable` reduces
    //     the identical `Err` to a bare `false`, meaning "keep the script
    //     engine", which is indistinguishable from a guard that legitimately
    //     has no native form. This is the sole stage that can report it.
    if !references_event_data_lexically(cond) {
        return Ok(());
    }
    let ast =
        parse_to_ast(cond).map_err(|e| located_expr_on_transition(transition, diag_label, e))?;

    walk_for_event_data_refs(
        &ast,
        schema,
        imported_enums,
        transition,
        statechart_name,
        diag_label,
    )?;
    Ok(())
}

/// Recursive AST walk. For every `_event.data.<field>` member access we
/// encounter, verify that `<field>` exists on the schema. For every
/// comparison node whose immediate left/right operand is such a member
/// access, additionally verify primitive-literal type compatibility.
fn walk_for_event_data_refs(
    expr: &TypedExpr,
    schema: &EventSchemaModel,
    imported_enums: &BTreeMap<String, EnumModel>,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    match &expr.kind {
        ExprKind::Member { object, property } => {
            if is_event_data_object(object) {
                // `_event.data.<property>` — resolve `<property>` against
                // the schema. Field-not-found is the primary
                // receive-side diagnostic.
                resolve_field(schema, property, transition, statechart_name, diag_label)?;
            }
            // Recurse into the object (covers nested patterns such as
            // `_event.data.foo.bar` — the check treats the outer
            // `_event.data.foo` as a field reference; deeper paths are
            // currently unverified because schema fields are flat
            // typed primitives, not nested records).
            walk_for_event_data_refs(
                object,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
        }
        ExprKind::Binary { op, left, right } => {
            // Order: field-not-found checks first via recursive walk,
            // then comparison-type-mismatch layered on top.
            walk_for_event_data_refs(
                left,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            walk_for_event_data_refs(
                right,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            if is_comparison(*op) {
                // Each operand is checked against the schema with the
                // OTHER operand as its comparison partner. RFC §bytesguard-3 B2/B3
                // (non-representable bytes form) surfaces before the
                // primitive-literal type check: it is a structural
                // rejection the SSOT classifier decides, independent of
                // the partner's type.
                for (field_side, other) in [(left, right), (right, left)] {
                    if let Some(field) = extract_event_data_field(field_side, schema) {
                        reject_non_representable_bytes(
                            field,
                            *op,
                            other,
                            transition,
                            statechart_name,
                            diag_label,
                        )?;
                        check_comparison_type(
                            field,
                            other,
                            imported_enums,
                            transition,
                            statechart_name,
                            diag_label,
                        )?;
                    }
                }
            }
        }
        ExprKind::Unary { operand, .. } => {
            walk_for_event_data_refs(
                operand,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            walk_for_event_data_refs(
                condition,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            walk_for_event_data_refs(
                consequent,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            walk_for_event_data_refs(
                alternate,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
        }
        ExprKind::Call { callee, args } => {
            walk_for_event_data_refs(
                callee,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            for arg in args {
                walk_for_event_data_refs(
                    arg,
                    schema,
                    imported_enums,
                    transition,
                    statechart_name,
                    diag_label,
                )?;
            }
        }
        ExprKind::Index { object, index } => {
            walk_for_event_data_refs(
                object,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
            walk_for_event_data_refs(
                index,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
        }
        // RFC c7-wildcard W-project: algorithm-kind-only node; recurse
        // transparently to stay exhaustive (unreachable on this path).
        ExprKind::BytesView { source, .. } => {
            walk_for_event_data_refs(
                source,
                schema,
                imported_enums,
                transition,
                statechart_name,
                diag_label,
            )?;
        }
        // Leaf nodes — nothing to walk.
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
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
            // the schema. The receive-side wiring routes the
            // statechart's name as the importing-name; the alias
            // surfaces the authored token `_event.data` verbatim so
            // the user sees the exact site they wrote.
            importing_kind: ForgeKind::Statechart,
            importing_name: statechart_name.to_string(),
            alias: EVENT_DATA_PATH.to_string(),
            field: field_name.to_string(),
            imported_kind: ForgeKind::EventSchema,
            imported_name: schema.name.clone(),
            candidates,
        },
    ))
}

/// Comparison type-mismatch check. Layered atop
/// `walk_for_event_data_refs` — for an expression shape
/// `_event.data.<field> === <literal>` (or reversed), compare
/// `<literal>`'s lexically determinable type against `<field>`'s
/// declared `sce_type`. Mismatches raise
/// `validation/cross-kind-type-mismatch` (reused — no new wire code).
///
/// Scope: literal vs primitive type. The other operand may
/// also be an arbitrary expression (e.g. a variable reference), in
/// which case the inferred type is not statically determinable from
/// the local view; the check is skipped in that case (the
/// existing typed-expression pipeline catches deeper mismatches at
/// codegen).
fn check_comparison_type(
    field: &ForgeField,
    other_operand: &TypedExpr,
    imported_enums: &BTreeMap<String, EnumModel>,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let other_kind = match operand_literal_kind(other_operand) {
        Some(kind) => kind,
        None => return Ok(()),
    };
    if !literal_is_compatible_with(&field.sce_type, other_kind) {
        return Err(located_on_transition(
            transition,
            diag_label,
            ValidationError::CrossKindTypeMismatch {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                alias: EVENT_DATA_PATH.to_string(),
                field: field.id.clone(),
                actual: literal_kind_canonical(other_kind),
                expected: sce_type_canonical(&field.sce_type),
            },
        ));
    }
    // Note: the `bytes`-specific representability rules (RFC §bytesguard-3 B2/B3 —
    // non-ASCII literal, ordering operator) are NOT decided here. They
    // live in the SSOT [`classify_bytes_comparison`] and are surfaced as
    // diagnostics by [`reject_non_representable_bytes`], which the walk
    // runs alongside this category check. Keeping the bytes rule out of
    // this primitive-category helper is what lets the native-lowerability
    // verdict consult the identical rule without duplicating it.
    if let Some(overflow) = enum_underlying_overflow(&field.sce_type, other_operand, imported_enums)
    {
        return Err(located_on_transition(
            transition,
            diag_label,
            ValidationError::CrossKindTypeMismatch {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                alias: EVENT_DATA_PATH.to_string(),
                field: field.id.clone(),
                actual: overflow.actual,
                expected: overflow.expected,
            },
        ));
    }
    // Strict variant membership — ordering invariant: it runs
    // only after the width narrowing layer above returned
    // `None`. A literal that overflows the underlying carrier is the
    // more fundamental violation and must surface first; once the
    // value is known to fit, the membership check asks the orthogonal
    // question of whether the value was actually declared.
    if let Some(not_declared) =
        enum_variant_not_declared(&field.sce_type, other_operand, imported_enums)
    {
        return Err(located_on_transition(
            transition,
            diag_label,
            ValidationError::CrossKindTypeMismatch {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                alias: EVENT_DATA_PATH.to_string(),
                field: field.id.clone(),
                actual: not_declared.actual,
                expected: not_declared.expected,
            },
        ));
    }
    Ok(())
}

/// The native-lowering admissibility of ONE comparison that touches a
/// `bytes` payload field — the single rule shared by the receive-side
/// validator (which renders a violation as a diagnostic) and the
/// native-lowerability verdict (which renders it as a non-native
/// classification). Defining it once keeps "which `bytes` comparison
/// forms lower natively" in exactly one place (RFC §bytesguard-3 B2/B3): a third
/// forbidden form is added here and both consumers inherit it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BytesComparisonForm {
    /// `===` / `!==` against a printable-ASCII string literal (or a
    /// non-literal operand) — the one shape with a byte-identical native
    /// lowering on all six backends.
    Representable,
    /// B3 — an ordering operator (`<`, `>`, `<=`, `>=`) on the `bytes`
    /// operand. Lexicographic ordering of an opaque payload byte-blob is
    /// not a meaningful author intent.
    OrderingOperator,
    /// B2 — equality against a string literal carrying a backslash
    /// escape or a non-ASCII byte: no unambiguous cross-backend byte
    /// constant.
    NonAsciiLiteral,
}

/// Classify a comparison `op` between a `bytes`-typed `_event.data`
/// field and its partner operand `other`. The caller has established
/// that one operand is the `bytes` field. Built on the shared primitives
/// [`is_ordering_comparison`] and [`decode_bytes_literal`] (the latter
/// the same printable-ASCII rule the codegen string→bytes
/// reinterpretation consults), so validator, verdict, and emitter can
/// never disagree about which forms are representable.
fn classify_bytes_comparison(op: BinOp, other: &TypedExpr) -> BytesComparisonForm {
    if is_ordering_comparison(op) {
        return BytesComparisonForm::OrderingOperator;
    }
    if matches!(op, BinOp::StrictEq | BinOp::StrictNeq) {
        if let ExprKind::StringLit { value, .. } = &other.kind {
            if decode_bytes_literal(value).is_none() {
                return BytesComparisonForm::NonAsciiLiteral;
            }
        }
    }
    BytesComparisonForm::Representable
}

/// Render a non-representable `bytes` comparison as its receive-side
/// diagnostic (RFC §bytesguard-3 B2/B3). A no-op for a non-`bytes` field or a
/// representable form — [`classify_bytes_comparison`] is the SSOT that
/// decides; this only maps the verdict to a diagnostic. `op` and `other`
/// are the comparison operator and the partner operand of `field`.
fn reject_non_representable_bytes(
    field: &ForgeField,
    op: BinOp,
    other: &TypedExpr,
    transition: &Transition,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if !matches!(field.sce_type, SceType::Bytes) {
        return Ok(());
    }
    match classify_bytes_comparison(op, other) {
        BytesComparisonForm::Representable => Ok(()),
        BytesComparisonForm::OrderingOperator => Err(located_on_transition(
            transition,
            diag_label,
            ValidationError::BytesComparisonNotEquality {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                field: field.id.clone(),
                op: ordering_op_token(op).to_string(),
            },
        )),
        BytesComparisonForm::NonAsciiLiteral => {
            // The classifier returns this variant only for a StringLit
            // partner, so the literal's source text is recoverable for
            // the message.
            let ExprKind::StringLit { value, .. } = &other.kind else {
                unreachable!("NonAsciiLiteral is produced only for a StringLit operand");
            };
            Err(located_on_transition(
                transition,
                diag_label,
                ValidationError::CrossKindTypeMismatch {
                    importing_kind: ForgeKind::Statechart,
                    importing_name: statechart_name.to_string(),
                    alias: EVENT_DATA_PATH.to_string(),
                    field: field.id.clone(),
                    actual: format!("non-printable-ASCII bytes literal '{value}'"),
                    expected: sce_type_canonical(&field.sce_type),
                },
            ))
        }
    }
}

/// The ordering subset of the comparison operators — the operators
/// `reject_bytes_ordering` forbids on a `bytes` operand. Distinct from
/// [`is_comparison`], which also admits the equality operators that
/// `bytes` permits.
fn is_ordering_comparison(op: BinOp) -> bool {
    matches!(op, BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq)
}

/// Source-token rendering for the ordering operators, for the B3
/// diagnostic's `op` slot. Only ordering operators reach this helper
/// (callers gate on [`is_ordering_comparison`]); the catch-all is
/// unreachable in practice and returns an empty token rather than
/// panicking.
fn ordering_op_token(op: BinOp) -> &'static str {
    match op {
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        _ => "",
    }
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
///   * `SceType::Enum(EnumRef)` accepts `Int` literals (per the lattice
///     rule `Enum(E) ⊑ E.underlying_type` — the underlying type is a
///     fixed-width integer, signed or unsigned). Width narrowing
///     against the resolved `underlying_type` runs as a second layer in
///     [`enum_underlying_overflow`] — this helper decides category only.
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
        SceType::Enum(_) => matches!(literal_kind, LiteralKind::Int),
    }
}

/// Width-narrowing diagnostic payload returned by
/// [`enum_underlying_overflow`]. `expected` carries the canonical name
/// of the enum's declared `underlying_type` (e.g. `"uint8"`), `actual`
/// names the offending integer literal with its parsed value (e.g.
/// `"integer literal 131071 (0x1FFFF) overflows uint8 underlying type of enum 'Result'"`).
/// Keeping both halves text-only lets the existing
/// `CrossKindTypeMismatch` variant carry the narrowing diagnostic
/// without new wire codes.
struct EnumUnderlyingOverflow {
    actual: String,
    expected: String,
}

/// Cross-doc Enum width narrowing — layered
/// atop the category check in [`literal_is_compatible_with`]. Returns
/// `Some` when:
///
///   * `field_type` is `SceType::Enum(EnumRef { alias })`,
///   * the statechart declared an `<sce:import kind="enum" as="alias">`
///     resolvable via `imported_enums` (silent-skip otherwise — the
///     alias is opaque from the statechart's view and the
///     conservative-accept default applies),
///   * `operand` is an integer literal whose value falls outside the
///     enum's declared `underlying_type` range, in either direction.
///
/// Returns `None` when any of the above does not hold — the in-range
/// case + the silent-skip case both return `None` and let the caller
/// continue.
///
/// Hex / binary / octal literal forms (`0x`, `0b`, `0o` prefixes) are
/// parsed alongside decimal so authors can write `_event.data.code ===
/// 0xFF` against a `uint8` underlying without false rejections. A
/// leading unary minus is unwrapped by [`integer_literal_value`] rather
/// than skipped: enum carriers may be signed, so `=== -1` is an
/// ordinary comparison that must be narrowed like any other, and `-1`
/// against an unsigned carrier has to report here instead of falling
/// through to the typed-expression pipeline at codegen time.
fn enum_underlying_overflow(
    field_type: &SceType,
    operand: &TypedExpr,
    imported_enums: &BTreeMap<String, EnumModel>,
) -> Option<EnumUnderlyingOverflow> {
    let SceType::Enum(enum_ref) = field_type else {
        return None;
    };
    let enum_model = imported_enums.get(&enum_ref.alias)?;
    let value = integer_literal_value(operand)?;
    if value_fits_underlying(value, &enum_model.underlying_type) {
        return None;
    }
    Some(EnumUnderlyingOverflow {
        actual: format!(
            "integer literal {value} overflows {underlying} underlying type of enum '{alias}'",
            underlying = sce_type_canonical(&enum_model.underlying_type),
            alias = enum_ref.alias,
        ),
        expected: sce_type_canonical(&enum_model.underlying_type),
    })
}

/// Parse an integer-literal `TypedExpr` (decimal, hex, binary, or
/// octal — matches the lexer's [`crate::forge::expr`] numeric forms)
/// into its value. Returns `None` for non-integer literals and for
/// magnitudes that do not fit in `u64`.
///
/// A leading unary minus is unwrapped here rather than rejected: enum
/// carriers may be signed, so `_event.data.status === -1` is a
/// well-formed comparison whose width and membership must be checked
/// like any other. `i128` holds every legal carrier value plus the
/// negation of every `u64` magnitude, so the unwrap cannot itself
/// overflow.
fn integer_literal_value(operand: &TypedExpr) -> Option<i128> {
    match &operand.kind {
        ExprKind::NumberLit(text) => parse_int_literal_text(text).map(i128::from),
        // The lexer models `-1` as a unary minus over a NumberLit; the
        // sign never reaches the literal's own text.
        ExprKind::Unary { op, operand: inner } if *op == UnaryOp::Neg => {
            let ExprKind::NumberLit(text) = &inner.kind else {
                return None;
            };
            parse_int_literal_text(text).map(|v| -i128::from(v))
        }
        _ => None,
    }
}

/// Membership diagnostic payload returned by [`enum_variant_not_declared`].
/// Shape mirrors [`EnumUnderlyingOverflow`] so the membership check
/// can flow through the same `CrossKindTypeMismatch` reuse precedent
/// without a new wire code.
struct EnumVariantNotDeclared {
    actual: String,
    expected: String,
}

/// Strict variant membership — check layered atop [`enum_underlying_overflow`]. Returns `Some`
/// when:
///
///   * `field_type` is `SceType::Enum(EnumRef { alias })`,
///   * the statechart declared an `<sce:import kind="enum" as="alias">`
///     resolvable via `imported_enums` (silent-skip otherwise — same
///     conservative-accept default as the width narrowing layer),
///   * the imported enum's `strict_variants` flag is `true` (opt-out
///     is owned by the declaring vocabulary),
///   * `operand` is an integer literal whose value is not one of the
///     values declared on the enum's `variants`.
///
/// Ordering invariant (design lock #3): callers MUST invoke this
/// helper *after* [`enum_underlying_overflow`] returns `None` —
/// width overflow is the more fundamental violation (the literal is
/// not even representable in the carrier) and its diagnostic must
/// win when both conditions hold. The membership check assumes the
/// value already fits the underlying type.
///
/// The opt-out (`sce:strict-variants="false"` on the enum's `<scxml>`
/// root) is the escape hatch for open-set status vocabularies — UDS
/// negative-response codes, OEM-extensible status enums — where wire-
/// side values outside the declared set are legal. Width narrowing
/// (design lock #4) still runs on opt-out enums; only this membership
/// layer silent-skips.
fn enum_variant_not_declared(
    field_type: &SceType,
    operand: &TypedExpr,
    imported_enums: &BTreeMap<String, EnumModel>,
) -> Option<EnumVariantNotDeclared> {
    let SceType::Enum(enum_ref) = field_type else {
        return None;
    };
    let enum_model = imported_enums.get(&enum_ref.alias)?;
    if !enum_model.strict_variants {
        return None;
    }
    let value = integer_literal_value(operand)?;
    if enum_model.variants.iter().any(|v| v.value == value) {
        return None;
    }
    // Render the declared variants in declaration order — the IR
    // preserves `<sce:variant>` ordering and authors typically scan
    // for the value they meant by recalling the document's natural
    // order, not by sorting alphabetically.
    let declared = enum_model
        .variants
        .iter()
        .map(|v| format!("{name}={value}", name = v.name, value = v.value))
        .collect::<Vec<_>>()
        .join(", ");
    Some(EnumVariantNotDeclared {
        actual: format!(
            "integer literal {value} is not a declared variant of enum '{alias}' (declared: {declared})",
            alias = enum_ref.alias,
        ),
        expected: format!("enum:{alias}", alias = enum_ref.alias),
    })
}

/// Parse a numeric literal's verbatim text. Supports the lexer's hex
/// (`0x` / `0X`), binary (`0b` / `0B`), and octal (`0o` / `0O`)
/// prefixes plus plain decimal. Returns `None` on parse failure or
/// when the text encodes a non-integer (decimal point / exponent —
/// callers already gate on `number_text_looks_like_int` upstream, so
/// this is a defensive secondary check).
fn parse_int_literal_text(text: &str) -> Option<u64> {
    if !number_text_looks_like_int(text) {
        return None;
    }
    if let Some(rest) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(rest, 16).ok()
    } else if let Some(rest) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        u64::from_str_radix(rest, 2).ok()
    } else if let Some(rest) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        u64::from_str_radix(rest, 8).ok()
    } else {
        text.parse::<u64>().ok()
    }
}

/// Compare a literal `value` against an enum's declared
/// `underlying_type`. Returns `true` iff `value` fits without overflow
/// in either direction.
///
/// The range comes from [`SceType::int_value_range`], which is also what
/// `parser::parse_enum` admits variants against, so this layer cannot
/// call a value out of range that the parser already accepted. A
/// non-integer carrier is structurally unreachable for an enum
/// underlying (the parser rejects it with
/// `validation/enum-unsupported-underlying-type`), and the permissive
/// `true` keeps the narrowing layer silent there rather than producing
/// a contradictory diagnostic if the parser-side rule ever relaxes.
fn value_fits_underlying(value: i128, underlying: &SceType) -> bool {
    let Some((min, max)) = underlying.int_value_range() else {
        return true;
    };
    (min..=max).contains(&value)
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
        SceType::Enum(r) => format!("enum:{}", r.alias),
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

/// Anchor an expression-stage rejection on the transition whose `cond`
/// produced it. Sibling of [`located_on_transition`] for the
/// `expression/*` half of the code catalog: a typed guard that does not
/// parse as a Forge expression is rejected as the `ExprError` the lexer
/// or parser actually raised, so the author gets the real cause
/// (`expression/strict-equality` for `==`, carrying its
/// `replace_with: "==="` fix) rather than a generic "unlowerable guard".
fn located_expr_on_transition(
    transition: &Transition,
    diag_label: &str,
    err: ExprError,
) -> Located<ForgeError> {
    let (line, col) = match transition.source_location.as_ref() {
        Some(loc) => (loc.line, loc.col),
        None => (None, None),
    };
    Located::new(ForgeError::Expression(err), diag_label, line, col)
}

/// Per-statechart send-side typecheck.
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
/// are skipped per the schemaless fallback. Dynamic event names
/// (`<send eventexpr="…">` with no `event=` attribute) are also
/// skipped — the validator cannot resolve a schema without a stable
/// event name.
///
/// Returns `Ok(())` when every `<send>` / `<raise>` payload validates,
/// or the first failing diagnostic.
pub fn check_send_side(
    scxml: &SCXMLModel,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    imported_enums: &BTreeMap<String, EnumModel>,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if imported_schemas.is_empty() {
        // Schemaless fallback — no imported schemas means every event keeps
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
                imported_enums,
                &scxml.name,
                diag_label,
            )?;
        }
        // Onentry / onexit handler bodies — each block is a
        // document-ordered sequence of executable-content actions
        // (§scxml-3.8, §3.9).
        for block in &state.on_entry_blocks {
            walk_actions(
                block,
                imported_schemas,
                imported_enums,
                &scxml.name,
                diag_label,
            )?;
        }
        for block in &state.on_exit_blocks {
            walk_actions(
                block,
                imported_schemas,
                imported_enums,
                &scxml.name,
                diag_label,
            )?;
        }
        // Initial-transition + history-default action sequences (W3C
        // SCXML §3.11) carry the same executable-content shape as the
        // transition bodies above.
        walk_actions(
            &state.initial_transition_actions,
            imported_schemas,
            imported_enums,
            &scxml.name,
            diag_label,
        )?;
        walk_actions(
            &state.initial_history_default_actions,
            imported_schemas,
            imported_enums,
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
    imported_enums: &BTreeMap<String, EnumModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    for action in actions {
        check_send_action(
            action,
            imported_schemas,
            imported_enums,
            statechart_name,
            diag_label,
        )?;
        // Composite-action bodies. The shapes are mutually exclusive
        // by action_type (`<if>` populates then/elseif/else, `<foreach>`
        // populates the `actions` field) but the model serialises them
        // as parallel `Vec<Action>` fields, so we walk each
        // unconditionally — empty vecs cost nothing.
        walk_actions(
            &action.then_actions,
            imported_schemas,
            imported_enums,
            statechart_name,
            diag_label,
        )?;
        for branch in &action.elseif_branches {
            walk_actions(
                &branch.actions,
                imported_schemas,
                imported_enums,
                statechart_name,
                diag_label,
            )?;
        }
        walk_actions(
            &action.else_actions,
            imported_schemas,
            imported_enums,
            statechart_name,
            diag_label,
        )?;
        walk_actions(
            &action.actions,
            imported_schemas,
            imported_enums,
            statechart_name,
            diag_label,
        )?;
    }
    Ok(())
}

/// Per-`<send>` / `<raise>` validator. Returns `Ok(())` for any
/// action that is not a send/raise, has no statically-resolvable
/// event name, or whose event name does not resolve to an imported
/// schema (schemaless fallback).
fn check_send_action(
    action: &Action,
    imported_schemas: &BTreeMap<String, EventSchemaModel>,
    imported_enums: &BTreeMap<String, EnumModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    if action.action_type != "send" && action.action_type != "raise" {
        return Ok(());
    }
    // Dynamic event name (`<send eventexpr="…">`) — the event the
    // payload will carry is not statically resolvable. Validator
    // cannot pick a schema; skip per the schemaless-fallback shape.
    if action.event.is_empty() {
        return Ok(());
    }
    let Some(schema) = imported_schemas.get(&action.event) else {
        return Ok(());
    };
    for param in &action.params {
        check_send_param(
            param,
            action,
            schema,
            imported_enums,
            statechart_name,
            diag_label,
        )?;
    }
    Ok(())
}

/// Per-`<param>` validator. Two failure modes:
///
/// * `<param name="F">` with `F` not on the schema —
///   [`ValidationError::EventPayloadFieldUnknown`] with the schema's
///   sorted, deduplicated field surface as the closed candidate set.
/// * `<param expr="…">` whose expression is a primitive literal
///   incompatible with the field's declared type —
///   [`ValidationError::CrossKindTypeMismatch`] (reused per Item 4
///   precedent).
///
/// §scxml-6.2.4 mandates exactly one of `expr` / `location` on
/// every `<param>`; the location form (`<param name="X" location="Y"/>`,
/// data-model variable assignment) is not statically typeable
/// without the typed datamodel pipeline and is left for the existing
/// typed-expression machinery at codegen time.
fn check_send_param(
    param: &Param,
    action: &Action,
    schema: &EventSchemaModel,
    imported_enums: &BTreeMap<String, EnumModel>,
    statechart_name: &str,
    diag_label: &str,
) -> Result<(), Located<ForgeError>> {
    let Some(field) = schema.fields.iter().find(|f| f.id == param.name) else {
        let mut candidates: Vec<String> = schema.fields.iter().map(|f| f.id.clone()).collect();
        candidates.sort();
        candidates.dedup();
        return Err(located_on_param(
            param,
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
    if !literal_is_compatible_with(&field.sce_type, literal_kind) {
        return Err(located_on_param(
            param,
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
        ));
    }
    if let Some(overflow) = enum_underlying_overflow(&field.sce_type, &expr_ast, imported_enums) {
        return Err(located_on_param(
            param,
            action,
            diag_label,
            ValidationError::CrossKindTypeMismatch {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                alias: format!("<send event=\"{}\">", action.event),
                field: field.id.clone(),
                actual: overflow.actual,
                expected: overflow.expected,
            },
        ));
    }
    // Strict variant membership (send-side mirror of the receive-
    // side check in `check_comparison_type`). Same ordering invariant:
    // runs after width narrowing so a value that overflows the
    // underlying carrier surfaces the overflow diagnostic rather than
    // a membership one.
    if let Some(not_declared) =
        enum_variant_not_declared(&field.sce_type, &expr_ast, imported_enums)
    {
        return Err(located_on_param(
            param,
            action,
            diag_label,
            ValidationError::CrossKindTypeMismatch {
                importing_kind: ForgeKind::Statechart,
                importing_name: statechart_name.to_string(),
                alias: format!("<send event=\"{}\">", action.event),
                field: field.id.clone(),
                actual: not_declared.actual,
                expected: not_declared.expected,
            },
        ));
    }
    Ok(())
}

/// Anchor a [`ValidationError`] on the `<param>` that carries the
/// rejected value, not on the `<send>` that encloses it.
///
/// SCE_ERROR_CONTRACT.md §2.2 has the consumer open `location.file`
/// and edit at `location.line`; §3.1 calls the substitution variants
/// applicable "without further judgment". A param-level rejection
/// anchored on its `<send>` breaks both promises at once — the value
/// in `actual` is not on the line the record names, so the consumer
/// either edits the wrong token or finds nothing to edit.
///
/// The action location remains the fallback for models built outside
/// the parser (unit tests, in-memory construction), where the param
/// never passed a node to record.
fn located_on_param(
    param: &Param,
    action: &Action,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    located_at(
        param
            .source_location
            .as_ref()
            .or(action.source_location.as_ref()),
        diag_label,
        err,
    )
}

/// Shared line/col extraction. The `file` half never comes from the
/// recorded [`SourceLocation`]: that field carries the *artifact*
/// spelling (a basename, so a generated marker does not bake one
/// machine's checkout into the tree), while a diagnostic must name
/// the document the way the caller named it — `diag_label`. Same
/// document, two spellings, chosen by who reads them (§2.2).
fn located_at(
    at: Option<&SourceLocation>,
    diag_label: &str,
    err: ValidationError,
) -> Located<ForgeError> {
    let (line, col) = match at {
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
    use crate::forge::model::{Direction, EnumModel, EnumRef, EnumVariant, ForgeField, SceType};

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

    /// Width-axis test helper — produces a single-variant enum with
    /// `strict_variants = false` so the membership layer
    /// silent-skips. Name is explicit (`_open` suffix) so axis intent
    /// is visible at every call site: tests using this helper care
    /// about width narrowing only, not membership. The earlier
    /// unsuffixed `enum_model` name silently diverged from
    /// production's `strict_variants = true` default, creating a
    /// drift vector for any newly-authored test that read the helper
    /// as "production-default enum"; the `_open` suffix forces an
    /// explicit choice at the call site.
    ///
    /// Membership tests build their own models via
    /// `enum_model_strict_with_variants` /
    /// `enum_model_open_with_variants` (declared below) when they
    /// need a richer variant set.
    fn enum_model_open(name: &str, underlying: SceType) -> EnumModel {
        EnumModel {
            name: name.to_string(),
            underlying_type: underlying,
            variants: vec![EnumVariant {
                name: "ok".to_string(),
                value: 0,
                source_line: None,
            }],
            strict_variants: false,
            source_location: None,
        }
    }

    fn run(schema: EventSchemaModel, cond: &str) -> Result<(), Located<ForgeError>> {
        run_with_enums(schema, cond, BTreeMap::new())
    }

    fn run_with_enums(
        schema: EventSchemaModel,
        cond: &str,
        enums: BTreeMap<String, EnumModel>,
    ) -> Result<(), Located<ForgeError>> {
        let ast = parse_to_ast(cond).expect("valid cond");
        let transition = Transition {
            event: schema.event_name.clone(),
            cond: cond.to_string(),
            ..Default::default()
        };
        walk_for_event_data_refs(
            &ast,
            &schema,
            &enums,
            &transition,
            "test_statechart",
            "test_diag_label",
        )
    }

    /// Drive [`check_transition`] itself, rather than the AST walk beneath
    /// it. [`run`] hands in an already-parsed AST, so it cannot see how
    /// the validator treats a `cond` that does not parse — which is
    /// exactly where the typed path used to lose its diagnostic.
    fn run_check_transition(
        schema: EventSchemaModel,
        cond: &str,
    ) -> Result<(), Located<ForgeError>> {
        let transition = Transition {
            event: schema.event_name.clone(),
            cond: cond.to_string(),
            ..Default::default()
        };
        let mut schemas = BTreeMap::new();
        schemas.insert(schema.event_name.clone(), schema);
        check_transition(
            &transition,
            &schemas,
            &BTreeMap::new(),
            "test_statechart",
            "test_diag_label",
        )
    }

    /// Loose `==` on a typed guard is the build-time error SCE Forge §3.4
    /// mandates. The validator is the only stage that can raise it: the
    /// engine-need analyzer reduces the same parse failure to "not
    /// natively lowerable", which is indistinguishable from a guard that
    /// legitimately keeps the script engine.
    #[test]
    fn loose_equality_on_typed_guard_is_rejected() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        let err = run_check_transition(s, "_event.data.status == 0")
            .expect_err("loose == on a typed guard must be rejected");
        assert!(
            matches!(
                err.error,
                ForgeError::Expression(ExprError::StrictEquality { .. })
            ),
            "must surface the real cause (with its replace_with fix), not a \
             generic unlowerable-guard error: {:?}",
            err.error
        );
    }

    /// The rejection is scoped to the typed path. A `cond` that touches no
    /// `_event.data` is plain ECMAScript bound for the script engine, even
    /// when its transition's event carries a schema. The Forge expression
    /// language is a strict subset of ECMAScript, so rejecting every cond
    /// it cannot parse would reject W3C-legal documents that never asked
    /// for the typed path.
    #[test]
    fn unparseable_cond_off_the_typed_path_is_not_rejected() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        assert!(run_check_transition(s, "retryCount == 3").is_ok());
    }

    /// A guard that is unlowerable-but-legal (a datamodel identifier has
    /// no binding inside the payload `matches!`) is NOT a rejection — it
    /// keeps the script engine, and the engine-need cause list reports it.
    #[test]
    fn mixed_typed_guard_is_legal_and_not_rejected() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        assert!(run_check_transition(s, "_event.data.status === 0 && retryCount > 3").is_ok());
    }

    #[test]
    fn known_field_passes() {
        let s = schema("job.completed", vec![field("status", SceType::Uint8)]);
        assert!(run(s, "_event.data.status === 0").is_ok());
    }

    // EventSchema MCU native lowering —
    // the shared SSOT lowering helper emits a native typed-payload
    // comparison (rename `_event.data` → bound payload var, `===` → `==`,
    // leaf field type adopted) and the engine-need verdict agrees.
    #[test]
    fn primitive_guard_lowers_native_and_is_engine_free() {
        let s = schema("job.completed", vec![field("elapsed_ms", SceType::Uint32)]);
        assert_eq!(
            lower_typed_guard("_event.data.elapsed_ms === 0", &s, "ev", ExprTarget::Rust).unwrap(),
            "ev.elapsed_ms == 0"
        );
        assert!(schema_is_native_payload_eligible(&s));
        assert!(guard_is_native_lowerable(
            "_event.data.elapsed_ms === 0",
            &s
        ));
    }

    // RFC §bytesguard-3 B6 — the native-lowerability verdict is the conjunction over
    // every backend SCE generates (`ExprTarget::ALL`), not a Rust + Go
    // proxy. A guard the verdict classes native must lower on each target.
    #[test]
    fn native_verdict_requires_all_six_targets_lower() {
        let s = schema("job.completed", vec![field("elapsed_ms", SceType::Uint32)]);
        assert!(guard_is_native_lowerable(
            "_event.data.elapsed_ms === 0",
            &s
        ));
        for target in ExprTarget::ALL {
            assert!(
                lower_typed_guard("_event.data.elapsed_ms === 0", &s, "ev", target).is_ok(),
                "{target:?} must lower the native guard the verdict accepted"
            );
        }
    }

    // A guard that mixes a typed `_event.data` field with a datamodel
    // identifier (or any non-payload reference) is NOT natively lowerable:
    // the bare identifier has no binding inside the payload `matches!`
    // guard. Must report false so codegen keeps it off the native path —
    // emitting it would produce a reference to an undefined local. The
    // document stays legal (the script engine evaluates it) and the
    // fallback is reported through the engine-need cause list, not as a
    // rejection.
    #[test]
    fn mixed_typed_and_datamodel_guard_is_not_lowerable() {
        let s = schema("job.completed", vec![field("elapsed_ms", SceType::Uint32)]);
        assert!(!guard_is_native_lowerable(
            "_event.data.elapsed_ms === 0 && retryCount > 3",
            &s
        ));
        // A function call over a typed field is likewise non-pure.
        assert!(!guard_is_native_lowerable(
            "compute(_event.data.elapsed_ms) > 0",
            &s
        ));
        // Sanity: the pure single-field comparison still lowers.
        assert!(guard_is_native_lowerable(
            "_event.data.elapsed_ms === 0",
            &s
        ));
    }

    // An enum-typed field carries its width in a separate imported Enum
    // document not yet threaded into the state-machine unit, so the
    // schema is payload-ineligible and the guard keeps the script-engine
    // baseline — 2b and 2c agree via this single eligibility rule.
    #[test]
    fn enum_typed_schema_is_payload_ineligible() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        assert!(!schema_is_native_payload_eligible(&s));
        assert!(!guard_is_native_lowerable("_event.data.status === 0", &s));
    }

    // Bytes-guard contract — the gate is
    // flipped: a bytes field is payload-eligible (it carries natively as
    // a bounded buffer on C11 / owned byte sequence on the hosted
    // backends) and an equality guard against a printable-ASCII literal
    // lowers natively on all six.
    #[test]
    fn bytes_typed_schema_is_payload_eligible() {
        let s = schema("temp.update", vec![field("raw", SceType::Bytes)]);
        assert!(schema_is_native_payload_eligible(&s));
        assert!(guard_is_native_lowerable("_event.data.raw === 'ack'", &s));
        assert!(guard_is_native_lowerable("_event.data.raw !== 'ack'", &s));
    }

    // A schema mixing a numeric field and a bytes field is eligible as a
    // whole, and the numeric guard stays native (the payload struct
    // carries both fields).
    #[test]
    fn schema_with_bytes_and_numeric_fields_is_eligible() {
        let s = schema(
            "temp.update",
            vec![
                field("count", SceType::Uint32),
                field("raw", SceType::Bytes),
            ],
        );
        assert!(schema_is_native_payload_eligible(&s));
        assert!(guard_is_native_lowerable("_event.data.count === 0", &s));
        assert!(guard_is_native_lowerable("_event.data.raw === 'ack'", &s));
    }

    // RFC §bytesguard-3 B2/B3 — the per-guard representability pre-check keeps a
    // non-lowerable bytes form off the native path even though the schema
    // is eligible: an ordering operator (B3) or a non-printable-ASCII
    // literal (B2) on a bytes operand is classed non-native (the
    // receive-side validator separately rejects the same forms).
    #[test]
    fn non_representable_bytes_guard_is_not_native() {
        let s = schema("temp.update", vec![field("raw", SceType::Bytes)]);
        // B3: ordering on a bytes operand.
        for op in ["<", ">", "<=", ">="] {
            assert!(
                !guard_is_native_lowerable(&format!("_event.data.raw {op} 'ack'"), &s),
                "ordering '{op}' on bytes must be non-native"
            );
        }
        // B2: non-printable-ASCII literal.
        assert!(!guard_is_native_lowerable("_event.data.raw === 'café'", &s));
        // Control: the representable equality form stays native.
        assert!(guard_is_native_lowerable("_event.data.raw === 'ack'", &s));
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
                SceType::Enum(EnumRef {
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
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let err = run(s, "_event.data.status === 'ok'").expect_err("should reject");
        let msg = format!("{err}");
        assert!(msg.contains("string"), "{msg}");
        assert!(msg.contains("enum:Result"), "{msg}");
    }

    // ─── Bytes guards (representability) ──

    // B3 baseline: equality against a printable-ASCII literal is the
    // one comparison shape a bytes payload permits.
    #[test]
    fn bytes_equality_with_ascii_literal_passes() {
        let s = schema("job.completed", vec![field("raw", SceType::Bytes)]);
        assert!(run(s.clone(), "_event.data.raw === 'ack'").is_ok());
        assert!(run(s, "_event.data.raw !== 'ack'").is_ok());
    }

    // B3: an ordering operator on a bytes field is rejected with the
    // dedicated operator-domain code, naming the offending operator.
    #[test]
    fn bytes_ordering_rejects() {
        for op in ["<", ">", "<=", ">="] {
            let s = schema("job.completed", vec![field("raw", SceType::Bytes)]);
            let cond = format!("_event.data.raw {op} 'ack'");
            let err = run(s, &cond).expect_err("ordering on bytes should reject");
            let ForgeError::Validation(v) = &err.error else {
                panic!("expected Validation, got {err:?}");
            };
            assert!(
                matches!(**v, ValidationError::BytesComparisonNotEquality { .. }),
                "expected BytesComparisonNotEquality, got {v:?}"
            );
            let msg = format!("{err}");
            assert!(msg.contains("bytes"), "{msg}");
            assert!(msg.contains(op), "{msg}");
        }
    }

    // B3 is operand-order independent — the bytes field on the right of
    // the operator is rejected the same way.
    #[test]
    fn bytes_ordering_reversed_operand_rejects() {
        let s = schema("job.completed", vec![field("raw", SceType::Bytes)]);
        let err = run(s, "'ack' < _event.data.raw").expect_err("should reject");
        let ForgeError::Validation(v) = &err.error else {
            panic!("expected Validation, got {err:?}");
        };
        assert!(matches!(
            **v,
            ValidationError::BytesComparisonNotEquality { .. }
        ));
    }

    // B2: a non-printable-ASCII literal (here a non-ASCII byte) has no
    // unambiguous cross-backend byte constant, so even an equality
    // comparison is rejected — reusing the cross-kind type-mismatch code.
    #[test]
    fn bytes_non_ascii_literal_rejects() {
        let s = schema("job.completed", vec![field("raw", SceType::Bytes)]);
        let err = run(s, "_event.data.raw === 'café'").expect_err("should reject");
        let ForgeError::Validation(v) = &err.error else {
            panic!("expected Validation, got {err:?}");
        };
        assert!(
            matches!(**v, ValidationError::CrossKindTypeMismatch { .. }),
            "expected CrossKindTypeMismatch, got {v:?}"
        );
        let msg = format!("{err}");
        assert!(msg.contains("bytes"), "{msg}");
    }

    #[test]
    fn empty_schema_map_skips_check() {
        let scxml = SCXMLModel {
            name: "test".to_string(),
            ..Default::default()
        };
        let schemas = BTreeMap::new();
        let enums = BTreeMap::new();
        check(&scxml, &schemas, &enums, "diag").expect("empty map should pass trivially");
    }

    // ─── Cross-doc Enum literal width narrowing ───────────
    //
    // All tests in this section use `enum_model_open` so the
    // membership layer silent-skips and the assertions exercise the
    // width axis in isolation. Membership behavior has its own
    // dedicated test section further below.

    #[test]
    fn enum_int_literal_within_underlying_width_passes() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_open("Result", SceType::Uint8),
        );
        assert!(run_with_enums(s, "_event.data.status === 255", enums).is_ok());
    }

    #[test]
    fn enum_int_literal_overflows_underlying_rejects() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_open("Result", SceType::Uint8),
        );
        let err = run_with_enums(s, "_event.data.status === 256", enums)
            .expect_err("256 must overflow uint8 underlying");
        let msg = format!("{err}");
        assert!(msg.contains("overflows"), "{msg}");
        assert!(msg.contains("uint8"), "{msg}");
        assert!(msg.contains("Result"), "{msg}");
    }

    #[test]
    fn enum_hex_literal_overflows_underlying_rejects() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_open("Result", SceType::Uint8),
        );
        // 0x1FFFF = 131071 > 255
        let err = run_with_enums(s, "_event.data.status === 0x1FFFF", enums)
            .expect_err("0x1FFFF must overflow uint8 underlying");
        let msg = format!("{err}");
        assert!(msg.contains("131071"), "{msg}");
        assert!(msg.contains("uint8"), "{msg}");
    }

    #[test]
    fn enum_hex_literal_at_underlying_boundary_passes() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_open("Result", SceType::Uint8),
        );
        // 0xFF = 255, exact uint8 boundary.
        assert!(run_with_enums(s, "_event.data.status === 0xFF", enums).is_ok());
    }

    #[test]
    fn enum_int_literal_within_wider_underlying_passes() {
        let s = schema(
            "job.completed",
            vec![field(
                "code",
                SceType::Enum(EnumRef {
                    alias: "Code".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert("Code".to_string(), enum_model_open("Code", SceType::Uint16));
        // 65535 = u16::MAX, exact uint16 boundary.
        assert!(run_with_enums(s, "_event.data.code === 65535", enums).is_ok());
    }

    #[test]
    fn enum_int_literal_overflows_uint16_underlying_rejects() {
        let s = schema(
            "job.completed",
            vec![field(
                "code",
                SceType::Enum(EnumRef {
                    alias: "Code".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert("Code".to_string(), enum_model_open("Code", SceType::Uint16));
        // 65536 > u16::MAX.
        let err = run_with_enums(s, "_event.data.code === 65536", enums)
            .expect_err("65536 must overflow uint16 underlying");
        let msg = format!("{err}");
        assert!(msg.contains("65536"), "{msg}");
        assert!(msg.contains("uint16"), "{msg}");
    }

    #[test]
    fn enum_int_literal_unknown_alias_silent_skips() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        // No imported enums declared — narrowing silent-skips
        // (conservative-accept), but category check still requires Int.
        let enums = BTreeMap::new();
        assert!(run_with_enums(s, "_event.data.status === 999999", enums).is_ok());
    }

    // ─── Strict variant membership ─────────────────────────────

    /// Membership-axis test helper: strict enum with author-supplied
    /// declared variants. Use this when the test exercises membership
    /// behavior and needs the declared set to be a concrete, asserted
    /// surface (e.g. "value 7 is rejected because it's not declared
    /// among {ok=0, error=1, timeout=2}").
    fn enum_model_strict_with_variants(
        name: &str,
        underlying: SceType,
        variants: &[(&str, i128)],
    ) -> EnumModel {
        EnumModel {
            name: name.to_string(),
            underlying_type: underlying,
            variants: variants
                .iter()
                .map(|(n, v)| EnumVariant {
                    name: (*n).to_string(),
                    value: *v,
                    source_line: None,
                })
                .collect(),
            strict_variants: true,
            source_location: None,
        }
    }

    /// Membership opt-out test helper: same as
    /// [`enum_model_strict_with_variants`] but with the opt-out flag
    /// flipped. Use this when the test exercises the opt-out branch
    /// (open-set vocabularies where undeclared values are legal).
    fn enum_model_open_with_variants(
        name: &str,
        underlying: SceType,
        variants: &[(&str, i128)],
    ) -> EnumModel {
        let mut m = enum_model_strict_with_variants(name, underlying, variants);
        m.strict_variants = false;
        m
    }

    #[test]
    fn enum_declared_variant_value_passes_membership() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_strict_with_variants(
                "Result",
                SceType::Uint8,
                &[("ok", 0), ("error", 1), ("timeout", 2)],
            ),
        );
        assert!(run_with_enums(s, "_event.data.status === 1", enums).is_ok());
    }

    #[test]
    fn enum_undeclared_value_within_width_rejects() {
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_strict_with_variants(
                "Result",
                SceType::Uint8,
                &[("ok", 0), ("error", 1), ("timeout", 2)],
            ),
        );
        // 7 fits uint8 width (passes width narrowing) but is not declared.
        let err = run_with_enums(s, "_event.data.status === 7", enums)
            .expect_err("7 must be rejected by the strict variant-membership check");
        let msg = format!("{err}");
        assert!(
            msg.contains("not a declared variant") && msg.contains("Result"),
            "{msg}"
        );
        // Diagnostic must enumerate the declared set in declaration order.
        assert!(msg.contains("ok=0"), "{msg}");
        assert!(msg.contains("error=1"), "{msg}");
        assert!(msg.contains("timeout=2"), "{msg}");
    }

    #[test]
    fn enum_overflow_diagnostic_wins_over_membership() {
        // Design lock #3 ordering: 256 both overflows uint8 AND is
        // not a declared variant — the width diagnostic must surface,
        // not the membership one.
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_strict_with_variants(
                "Result",
                SceType::Uint8,
                &[("ok", 0), ("error", 1), ("timeout", 2)],
            ),
        );
        let err = run_with_enums(s, "_event.data.status === 256", enums)
            .expect_err("256 must overflow uint8 (width diagnostic wins)");
        let msg = format!("{err}");
        assert!(msg.contains("overflows"), "{msg}");
        assert!(!msg.contains("not a declared variant"), "{msg}");
    }

    #[test]
    fn enum_opt_out_silent_skips_membership_but_keeps_width() {
        // Design lock #4: opt-out skips membership but width still runs.
        let s = schema(
            "uds.response",
            vec![field(
                "code",
                SceType::Enum(EnumRef {
                    alias: "UdsNrc".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "UdsNrc".to_string(),
            enum_model_open_with_variants(
                "UdsNrc",
                SceType::Uint8,
                &[("generalReject", 0x10), ("serviceNotSupported", 0x11)],
            ),
        );
        // 0x21 (33) is undeclared but fits uint8 → opt-out accepts.
        assert!(run_with_enums(s.clone(), "_event.data.code === 0x21", enums.clone()).is_ok());
        // 0x1FF (511) overflows uint8 → width still rejects.
        let err = run_with_enums(s, "_event.data.code === 0x1FF", enums)
            .expect_err("width narrowing must still reject overflow on opt-out enum");
        let msg = format!("{err}");
        assert!(msg.contains("overflows"), "{msg}");
    }

    #[test]
    fn enum_hex_literal_membership_check_uses_decimal_value() {
        // Authors may write `0x10` for value 16; the diagnostic
        // canonicalizes to the decimal numeric value so the rendered
        // candidate list lines up with the source variants' declared
        // numbers.
        let s = schema(
            "job.completed",
            vec![field(
                "status",
                SceType::Enum(EnumRef {
                    alias: "Result".to_string(),
                }),
            )],
        );
        let mut enums = BTreeMap::new();
        enums.insert(
            "Result".to_string(),
            enum_model_strict_with_variants("Result", SceType::Uint8, &[("ok", 0), ("error", 1)]),
        );
        // 0x05 = 5, undeclared and fits uint8.
        let err = run_with_enums(s, "_event.data.status === 0x05", enums)
            .expect_err("0x05 must be rejected by membership check");
        let msg = format!("{err}");
        assert!(msg.contains("integer literal 5"), "{msg}");
    }
}
