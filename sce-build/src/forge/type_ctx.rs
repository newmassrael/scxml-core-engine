// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge per-kind TypeCtx builders.
//
// Each Forge kind (transform, condition, validator, procedure, filter,
// observer, codec, lookup, interpolation, timer) exposes a different set
// of identifiers to its user-written expressions. This module builds the
// correct [`TypeCtx`] for each kind, including cross-file import propagation.
//
// The generator layer calls one of these functions at every expression
// transpilation site, passing the result straight into
// [`crate::forge::expr::transpile_typed`]. There is no path that skips the
// typed context — that would re-introduce the architectural debt this
// refactor is removing.

use crate::forge::generator::ImportContext;
use crate::forge::model::*;
use crate::forge::quantity::{NumericBaseType, Quantity};
use crate::forge::types::{EnumScope, FuncSig, InferredType, RecordShape, TypeCtx};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Low-level helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Populate `ctx.vars` with every field from the slice, keyed by the
/// field's `id` and typed via [`InferredType::from_sce_type`].
/// Physical-quantity surface: fields carrying a `quantity` annotation
/// surface in the context as `InferredType::Quantity { base, scale,
/// offset, unit }` so unit propagates through arithmetic inference and
/// unit mismatches collapse to `Unknown` for the post-pass diagnostic.
fn insert_fields<'a>(ctx: &mut TypeCtx<'a>, fields: &'a [ForgeField]) {
    for f in fields {
        ctx.insert_var(f.id.as_str(), forge_field_type(f));
    }
}

/// Map a `ForgeField` to its `InferredType`, wrapping in `Quantity`
/// when the field carries an `sce:quantity=…` annotation.
pub(crate) fn forge_field_type(f: &ForgeField) -> InferredType {
    quantified_type(&f.sce_type, f.quantity)
}

/// The one mapping from a declared `sce:type` plus an optional
/// `sce:quantity` to an inferred type, for every field shape that carries
/// the two. A non-numeric base keeps its raw type: a unit means nothing
/// on it, and saying so is validation's job, not inference's.
///
/// ⚠ ONE body. `forge_field_type` and `codec_field_type` each carried a
/// copy of it, identical but for the field type they read.
fn quantified_type(sce_type: &SceType, quantity: Option<Quantity>) -> InferredType {
    let base_ty = InferredType::from_sce_type(sce_type);
    let Some(q) = quantity else {
        return base_ty;
    };
    match base_ty {
        InferredType::Int { signed, bits } => InferredType::Quantity {
            base: NumericBaseType::Int { signed, bits },
            scale: q.scale,
            offset: q.offset,
            unit: q.unit,
        },
        InferredType::Float { bits } => InferredType::Quantity {
            base: NumericBaseType::Float { bits },
            scale: q.scale,
            offset: q.offset,
            unit: q.unit,
        },
        other => other,
    }
}

/// Populate `ctx.funcs` with each cross-file **stateless** import's
/// signature, keyed by the import's user-visible alias. Stateless kinds
/// (Transform, Condition, Lookup, Interpolation) are exposed to expressions
/// as `alias(arg, arg, ...)` calls that desugar to the imported function.
fn insert_stateless_imports<'a>(ctx: &mut TypeCtx<'a>, imports: &'a [ImportContext]) {
    for imp in imports {
        if imp.is_stateful {
            continue;
        }
        if let Some(sig) = imported_callee_sig(
            &imp.param_types,
            imp.ret_type.as_ref(),
            imp.list_slot.as_deref(),
        ) {
            ctx.insert_func(imp.alias.as_str(), sig);
        }
    }
}

/// The signature an imported stateless callee registers under its alias —
/// the one rule a forge kind and a `sce-static` statechart
/// ([`StaticScope`]) both call an import by.
///
/// An algorithm with a `list<T>` or `record:` slot has no signature to
/// register, and leaving it out would leave its calls unjudged: it is
/// registered as callable only by a host, which is what refuses the call.
/// A callee with no return is not registered — it is no value an expression
/// can take, and the closed scope refuses a call to it by name.
pub(crate) fn imported_callee_sig(
    params: &[SceType],
    ret: Option<&SceType>,
    host_only: Option<&str>,
) -> Option<FuncSig> {
    if let Some(slot) = host_only {
        return Some(FuncSig::host_only(slot));
    }
    Some(FuncSig {
        params: params.iter().map(InferredType::from_sce_type).collect(),
        ret: InferredType::from_sce_type(ret?),
        host_only: None,
    })
}

/// An algorithm a `sce-static` statechart imports (`<sce:import
/// kind="algorithm">`), read where the document is parsed: its alias, the
/// document it names, and its signature as the forge import pass discovers
/// it. Language-free — the name a backend calls it by is the renderer's.
#[derive(Debug, Clone, Default)]
pub struct StaticCallee {
    pub alias: String,
    /// The imported document's own name, from which a backend derives the
    /// symbol it emits.
    pub document_name: String,
    pub params: Vec<SceType>,
    pub ret: Option<SceType>,
    /// Why only a host may call it (a list or record slot), if so.
    pub host_only: Option<String>,
    /// The `<sce:import>` element's row, for a refusal of the import itself.
    pub line: Option<u32>,
}

/// Populate `ctx.vars` with every **stateful** import's alias as an opaque
/// struct-typed variable, and every publicly accessible member field of
/// that alias under the qualified key `"{alias}.{field}"`.
///
/// **Key namespace invariant**: member field keys MUST be qualified with the
/// alias prefix. An unqualified bare key would collide with any same-named
/// field in the enclosing kind (e.g. a codec named `frame` with field
/// `payload` imported into a procedure whose own input is also called
/// `payload`), silently yielding wrong type inference. The qualification
/// happens at enrichment time (`lib.rs::validate_and_enrich_imports`), so
/// this function simply trusts `imp.member_field_types` keys are already in
/// `"{alias}.{field}"` form.
///
/// The alias itself maps to `Unknown` because our type lattice cannot
/// represent a genuine struct type; downstream emitters must consult the
/// rename map to turn the alias Ident into a target-language member access
/// (e.g. `self.frame_`, `p.Frame`, `frame_` — one per language).
///
/// The inference pass in [`crate::forge::expr::infer_types`] looks up these
/// qualified keys via its Member branch: when it sees
/// `Member{object: Ident(obj), property: prop}` it forms `"{obj}.{prop}"`
/// and calls `ctx.lookup_var`, which returns the field's concrete type if
/// registered here.
fn insert_stateful_imports<'a>(ctx: &mut TypeCtx<'a>, imports: &'a [ImportContext]) {
    for imp in imports {
        if !imp.is_stateful {
            continue;
        }
        // The import's fields and methods are the whole of what its alias
        // may be asked for, and both are registered just below.
        ctx.insert_record(imp.alias.as_str(), RecordShape::Closed);
        for (qualified_key, fty) in &imp.member_field_types {
            ctx.insert_var(qualified_key.as_str(), InferredType::from_sce_type(fty));
        }
        for (qualified_key, param_tys, ret_ty) in &imp.member_method_sigs {
            let params = param_tys.iter().map(InferredType::from_sce_type).collect();
            ctx.insert_func(
                qualified_key.as_str(),
                FuncSig {
                    params,
                    ret: InferredType::from_sce_type(ret_ty),
                    host_only: None,
                },
            );
        }
    }
}

/// Register every imported enum under its alias, so an expression's
/// `<alias>.<variant>` can be checked against the declared variants and
/// lowered to this backend's spelling.
///
/// ⚠ Every builder calls this, not only the ones whose kind "uses" enums:
/// which kinds use them is the author's call, and the parser accepts
/// `enum:<alias>` on every field it reads.
///
/// `pub(crate)` for the one kind that builds its context outside this
/// module — the algorithm renderer — so it registers enums through the
/// same code rather than a copy of it.
pub(crate) fn insert_enum_imports<'a>(ctx: &mut TypeCtx<'a>, imports: &'a [ImportContext]) {
    for imp in imports {
        if imp.kind != "enum" {
            continue;
        }
        ctx.insert_enum(
            imp.alias.as_str(),
            EnumScope {
                variants: &imp.enum_variants,
                qualified_type: imp.enum_qualified_type.as_str(),
                source_name: imp.enum_source_name.as_str(),
            },
        );
    }
}

/// Everything a forge kind's expression may name comes from its own
/// declarations and its imports — there is no host behind it. So every
/// forge builder refuses the two mistakes a name can be: a call to
/// something unprovided, and a read of something undeclared.
fn close_the_scope(ctx: &mut TypeCtx<'_>) {
    ctx.reject_unknown_callees = true;
    ctx.reject_unknown_identifiers = true;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Per-kind builders
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// TypeCtx for a **Transform** kind: inputs are readable (parameters),
/// and so is a SIBLING OUTPUT — the generator lowers such a read to a
/// call of that output's own function. Each output's own type still
/// drives the `expected` parameter passed to `transpile_typed`.
/// Cross-file imports add function signatures.
///
/// ⚠ THIS USED TO SAY "outputs are not visible to expressions". That was
/// the stated rule and NOTHING ENFORCED IT: measured 2026-09-18, a
/// document whose output read a sibling generated with exit 0 and emitted
/// a body naming an identifier the signature never bound. A rule that is
/// only written down is not a rule. The resolution was to make the read
/// legal and lower it, because prose specifications genuinely name
/// intermediate values that several outputs consume — see
/// [`crate::forge::transform_dep_check`], which refuses the one shape
/// lowering cannot serve.
///
/// `cells` are the fields read through `previous()`
/// ([`crate::forge::previous_value::cells`]): each one's parameter is
/// readable like an input — it IS one, to the function that reads it — and
/// `previous(<field>)` is lowered to it before anything else runs.
pub fn transform<'a>(
    m: &'a TransformModel,
    cells: &'a [crate::forge::previous_value::Cell],
    imports: &'a [ImportContext],
) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    for cell in cells {
        ctx.insert_var(cell.param.id.as_str(), forge_field_type(&cell.param));
        ctx.previous_cells
            .insert(cell.of.as_str(), cell.param.id.as_str());
    }
    // Outputs AFTER inputs: an id declared on both sides is the author's
    // own collision, and the input spelling is the one a reader expects
    // to win because it is what the signature binds.
    insert_fields(&mut ctx, &m.outputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    // ⚠ A forge kind has no host to call. Everything callable from here was
    // registered three lines up, so a name that is not is a mistake — and
    // before this flag existed it was an ACCEPTED mistake: `expr="round(v)"`
    // generated with exit 0 and emitted C++ that did not compile, as did
    // `expr="totallyMadeUpFn(v)"`. See `TypeCtx::reject_unknown_callees` for
    // why the statechart path must keep the opposite default.
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for a **Condition** kind: inputs are the identifiers visible
/// inside the boolean expression. The overall expected type is
/// [`InferredType::Bool`].
pub fn condition<'a>(m: &'a ConditionModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for a **Validator** kind's plausibility expression. The expression
/// sees the inputs plus any rate-of-change helpers as opaque state, so we
/// register only the inputs explicitly.
pub fn validator<'a>(m: &'a ValidatorModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for a **Lookup** kind: single-input function.
pub fn lookup<'a>(m: &'a LookupModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    ctx.insert_var(
        m.input.id.as_str(),
        InferredType::from_sce_type(&m.input.sce_type),
    );
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for a **Filter** kind: the filter expression sees the input
/// field. The output is not visible on the right-hand side.
pub fn filter<'a>(m: &'a FilterModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    ctx.insert_var(
        m.input.id.as_str(),
        InferredType::from_sce_type(&m.input.sce_type),
    );
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for an **Observer** kind's monitor enter/leave expressions. The
/// monitor expressions reference the observer's input fields directly.
pub fn observer<'a>(m: &'a ObserverModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// Populate `ctx.funcs` with each declared `<sce:helper>` signature, keyed
/// by the user-visible name. Expressions referencing a declared helper get
/// typed inference on both the argument slots and the return value — which
/// means `computeKey(seed)` in an `sce:payload` attribute can propagate its
/// declared `returns="bytes"` type up through enclosing arithmetic / member
/// access, instead of the old "unknown → emit verbatim" fallback.
fn insert_procedure_helpers<'a>(ctx: &mut TypeCtx<'a>, helpers: &'a [ProcedureHelper]) {
    for h in helpers {
        let params: Vec<InferredType> = h.args.iter().map(InferredType::from_sce_type).collect();
        let ret = InferredType::from_sce_type(&h.returns);
        ctx.insert_func(
            h.name.as_str(),
            FuncSig {
                params,
                ret,
                host_only: None,
            },
        );
    }
}

/// TypeCtx for a **Procedure** kind. The procedure model exposes inputs
/// and internal state fields to expressions (guards, assigns, sends). The
/// `_event.data` rename that the generator applies before transpile is
/// transparent to the TypeCtx — the caller is responsible for registering
/// the target rename key (e.g. `pendingEventData_`) with the right type
/// via [`TypeCtx::insert_var`] after calling this builder, if that type
/// is statically known. `<sce:helper>` declarations seed `ctx.funcs` via
/// [`insert_procedure_helpers`] so helper call sites are fully typed.
pub fn procedure<'a>(m: &'a ProcedureModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_fields(&mut ctx, &m.internals);
    insert_procedure_helpers(&mut ctx, &m.helpers);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    // §scxml-5.10: `_event` is a system variable bound in every data
    // model, and a procedure's transitions read the reply that triggered
    // them through it (`<assign expr="_event.data"/>`). The generator
    // rewrites `_event.data` to the procedure's own payload member AFTER
    // names are checked, so the name the check sees is the author's.
    // A record: its shape is the triggering event's, not this document's,
    // so its members are not this context's to judge.
    ctx.insert_record("_event", RecordShape::Open);
    close_the_scope(&mut ctx);
    ctx
}

/// TypeCtx for a **Codec** kind. Codec expressions are encode/decode bit
/// manipulations that reference byte-level fields. Each field of the codec
/// is exposed by its `id`. Physical-quantity surface: codec fields
/// carrying `sce:quantity=…` surface as `InferredType::Quantity` so
/// downstream expressions (e.g., codec predicate guards referencing a
/// physically-tagged raw field) get the same unit-aware inference as
/// Transform / Condition.
pub fn codec<'a>(m: &'a CodecModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    for f in &m.fields {
        ctx.insert_var(f.id.as_str(), codec_field_type(f));
    }
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    insert_enum_imports(&mut ctx, imports);
    close_the_scope(&mut ctx);
    ctx
}

/// Map a `CodecField` to its `InferredType`, wrapping in `Quantity`
/// when the field carries an `sce:quantity=…` annotation.
pub(crate) fn codec_field_type(f: &CodecField) -> InferredType {
    quantified_type(&f.sce_type, f.quantity)
}

/// An enum a statechart imports, as the static data model's scope names it:
/// the import's alias, its variants in the enum document's spelling, the
/// enum document's name, and the backend type an emitter spells it as.
/// Owned, because a [`TypeCtx`] borrows what it registers and the
/// statechart holds its enums as models, not as [`ImportContext`]s.
#[derive(Debug, Clone)]
pub struct StaticEnum {
    pub alias: String,
    pub variants: Vec<String>,
    pub source_name: String,
    pub qualified_type: String,
}

impl StaticEnum {
    /// One per imported enum, keyed by alias, with `qualified_type` as the
    /// backend spells it — the alias itself for a check that lowers
    /// nothing.
    pub fn from_imports(
        enums: &std::collections::BTreeMap<String, EnumModel>,
        qualified_type: impl Fn(&str, &EnumModel) -> String,
    ) -> Vec<Self> {
        enums
            .iter()
            .map(|(alias, model)| StaticEnum {
                alias: alias.clone(),
                variants: model.variants.iter().map(|v| v.name.clone()).collect(),
                source_name: model.name.clone(),
                qualified_type: qualified_type(alias, model),
            })
            .collect()
    }
}

/// Each `record:<alias>` variable's fields as the dotted path an expression
/// reads them by, `<id>.<field>`, typed by the schema `alias` names in
/// `records` (the statechart's imports keyed by alias). A variable whose
/// alias names no resolved schema contributes nothing — the parser has
/// already judged the alias against the imports.
fn static_record_paths<'v>(
    variables: impl IntoIterator<Item = &'v crate::model::Variable>,
    records: &std::collections::BTreeMap<String, EventSchemaModel>,
) -> Vec<(String, InferredType)> {
    let mut paths = Vec::new();
    for var in variables {
        let Some(schema) = var
            .value_type
            .as_ref()
            .and_then(AlgorithmValueType::record_alias)
            .and_then(|alias| records.get(alias))
        else {
            continue;
        };
        for field in &schema.fields {
            paths.push((
                format!("{}.{}", var.id, field.id),
                InferredType::from_sce_type(&field.sce_type),
            ));
        }
    }
    paths
}

/// A `sce-static` statechart's typed scope (docs/SCE_ACCEPTED_SUBSET.md
/// §2.15), gathered once for every pass that judges or lowers one of its
/// expressions — validation, the host-action signature check, the Kotlin
/// lowering and the host-action call it renders — so that none of them can
/// see a scope the others do not.
///
/// Owned rather than borrowed from the model: the lowering passes rewrite
/// the model while they read this.
pub struct StaticScope {
    /// Every variable, the document's and each state's.
    pub variables: Vec<crate::model::Variable>,
    /// Each record variable's `<id>.<field>` ([`static_record_paths`]).
    record_paths: Vec<(String, InferredType)>,
    /// Every imported algorithm, callable as `Alias(args)`.
    pub callees: Vec<StaticCallee>,
}

impl StaticScope {
    /// The scope of `model`; `None` for a document under any other data model.
    pub fn of(model: &crate::model::SCXMLModel) -> Option<Self> {
        if model.datamodel != crate::model::Datamodel::SceStatic {
            return None;
        }
        let variables: Vec<crate::model::Variable> = model
            .variables
            .iter()
            .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
            .cloned()
            .collect();
        let record_paths = static_record_paths(variables.iter(), &model.imported_records);
        Some(Self {
            variables,
            record_paths,
            callees: model.imported_algorithms.clone(),
        })
    }

    /// The dotted paths an expression may read: every record variable's
    /// fields, and `payload`'s `_event.data.<field>` when the expression sits
    /// on a transition whose event carries one.
    pub fn paths(&self, payload: Option<&EventSchemaModel>) -> Vec<(String, InferredType)> {
        let mut paths = self.record_paths.clone();
        if let Some(schema) = payload {
            paths.extend(crate::forge::event_schema_check::event_payload_paths(
                schema,
            ));
        }
        paths
    }

    /// The first list variable `expr` reads as a value — anywhere but as
    /// `len(…)`'s argument — or `None`. The one test every pass that judges a
    /// `sce-static` expression applies, so a list is refused alike in a
    /// guard, an assignment and a host action's argument.
    pub fn list_read_as_value(&self, expr: &str) -> Option<String> {
        crate::forge::expr::identifiers_read_as_values(expr)
            .ok()?
            .into_iter()
            .find(|name| {
                self.variables.iter().any(|v| {
                    v.id == *name
                        && v.value_type
                            .as_ref()
                            .and_then(AlgorithmValueType::list_elem)
                            .is_some()
                })
            })
    }

    /// The [`TypeCtx`] over this scope with `paths` ([`Self::paths`]) in it.
    pub fn ctx<'a>(
        &'a self,
        paths: &'a [(String, InferredType)],
        enums: &'a [StaticEnum],
    ) -> TypeCtx<'a> {
        let mut ctx = static_statechart(self.variables.iter(), paths, enums);
        for callee in &self.callees {
            if let Some(sig) = imported_callee_sig(
                &callee.params,
                callee.ret.as_ref(),
                callee.host_only.as_deref(),
            ) {
                ctx.insert_func(callee.alias.as_str(), sig);
            }
        }
        ctx
    }
}

/// TypeCtx for a statechart's expression under `datamodel="sce-static"`
/// (docs/SCE_ACCEPTED_SUBSET.md §2.15): every declared variable at its
/// `sce:type` — a `record:<alias>` one as a closed record — the typed dotted
/// `paths` in scope, the imported enums, and `In(<state id>)`.
///
/// `paths` are every member an expression may read through a dot, owned by
/// the caller because a [`TypeCtx`] borrows its keys: the triggering
/// event's `_event.data.<field>` when its event carries a schema
/// ([`crate::forge::event_schema_check::event_payload_paths`]) and each
/// record variable's `<id>.<field>` ([`static_record_paths`]).
///
/// The scope is closed, as every forge kind's is: the model is defined so
/// that a document needs no script engine, so there is no host behind a
/// name nothing declares. `_event` is an open record for the reason the
/// procedure kind gives — its members beyond the typed payload are the
/// triggering event's, not this document's to judge.
fn static_statechart<'a>(
    variables: impl IntoIterator<Item = &'a crate::model::Variable>,
    paths: &'a [(String, InferredType)],
    enums: &'a [StaticEnum],
) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    for var in variables {
        let Some(value_type) = var.value_type.as_ref() else {
            ctx.insert_var(var.id.as_str(), InferredType::Unknown);
            continue;
        };
        if value_type.record_alias().is_some() {
            // Its fields are `paths`, and they are the whole of what it may
            // be asked for — the rule an algorithm's record local keeps.
            ctx.insert_record(var.id.as_str(), RecordShape::Closed);
            continue;
        }
        if let Some(elem) = value_type.list_elem() {
            // Typed as a list so `len(…)` measures it; the static data
            // model's judge refuses it anywhere it would be read as a value.
            if let Some(elem) = crate::forge::types::ListElem::of(elem) {
                ctx.insert_var(var.id.as_str(), InferredType::List(elem));
            }
            continue;
        }
        let ty = value_type
            .scalar()
            .map_or(InferredType::Unknown, InferredType::from_sce_type);
        ctx.insert_var(var.id.as_str(), ty);
    }
    for (path, ty) in paths {
        ctx.insert_var(path.as_str(), *ty);
    }
    for e in enums {
        ctx.insert_enum(
            e.alias.as_str(),
            EnumScope {
                variants: &e.variants,
                qualified_type: e.qualified_type.as_str(),
                source_name: e.source_name.as_str(),
            },
        );
    }
    // `In(stateID)` is the one predicate every data model provides. Its
    // signature is declared here so the closed scope admits it; lowering it
    // is each backend's.
    ctx.insert_func(
        "In",
        FuncSig {
            params: vec![InferredType::Str],
            ret: InferredType::Bool,
            host_only: None,
        },
    );
    ctx.insert_record("_event", RecordShape::Open);
    close_the_scope(&mut ctx);
    ctx
}

/// Empty context — no variables, no functions. Used for expressions that
/// have no access to named identifiers (e.g. default-value initializers
/// for internal fields that are pure literal constants).
pub fn empty() -> TypeCtx<'static> {
    TypeCtx::new()
}
