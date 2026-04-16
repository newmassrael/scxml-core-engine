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
use crate::forge::types::{FuncSig, InferredType, TypeCtx};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Low-level helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Populate `ctx.vars` with every field from the slice, keyed by the
/// field's `id` and typed via [`InferredType::from_sce_type`].
fn insert_fields<'a>(ctx: &mut TypeCtx<'a>, fields: &'a [ForgeField]) {
    for f in fields {
        ctx.insert_var(f.id.as_str(), InferredType::from_sce_type(&f.sce_type));
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
        let Some(ret_ty) = imp.ret_type.as_ref() else { continue };
        let params: Vec<InferredType> = imp
            .param_types
            .iter()
            .map(InferredType::from_sce_type)
            .collect();
        let ret = InferredType::from_sce_type(ret_ty);
        ctx.insert_func(imp.alias.as_str(), FuncSig { params, ret });
    }
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
fn insert_stateful_imports<'a>(
    ctx: &mut TypeCtx<'a>,
    imports: &'a [ImportContext],
) {
    for imp in imports {
        if !imp.is_stateful {
            continue;
        }
        ctx.insert_var(imp.alias.as_str(), InferredType::Unknown);
        for (qualified_key, fty) in &imp.member_field_types {
            ctx.insert_var(qualified_key.as_str(), InferredType::from_sce_type(fty));
        }
        for (qualified_key, param_tys, ret_ty) in &imp.member_method_sigs {
            let params = param_tys.iter().map(InferredType::from_sce_type).collect();
            ctx.insert_func(
                qualified_key.as_str(),
                FuncSig { params, ret: InferredType::from_sce_type(ret_ty) },
            );
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Per-kind builders
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// TypeCtx for a **Transform** kind: inputs are readable (parameters),
/// outputs are not visible to expressions but their types drive the
/// `expected` output parameter passed to `transpile_typed` at each call.
/// Cross-file imports add function signatures.
pub fn transform<'a>(m: &'a TransformModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
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
    ctx
}

/// TypeCtx for a **Lookup** kind: single-input function.
pub fn lookup<'a>(m: &'a LookupModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    ctx.insert_var(m.input.id.as_str(), InferredType::from_sce_type(&m.input.sce_type));
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    ctx
}

/// TypeCtx for a **Filter** kind: the filter expression sees the input
/// field. The output is not visible on the right-hand side.
pub fn filter<'a>(m: &'a FilterModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    ctx.insert_var(m.input.id.as_str(), InferredType::from_sce_type(&m.input.sce_type));
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    ctx
}

/// TypeCtx for an **Observer** kind's monitor enter/leave expressions. The
/// monitor expressions reference the observer's input fields directly.
pub fn observer<'a>(m: &'a ObserverModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    insert_fields(&mut ctx, &m.inputs);
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
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
        ctx.insert_func(h.name.as_str(), FuncSig { params, ret });
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
    ctx
}

/// TypeCtx for a **Codec** kind. Codec expressions are encode/decode bit
/// manipulations that reference byte-level fields. Each field of the codec
/// is exposed by its `id`.
pub fn codec<'a>(m: &'a CodecModel, imports: &'a [ImportContext]) -> TypeCtx<'a> {
    let mut ctx = TypeCtx::new();
    for f in &m.fields {
        ctx.insert_var(f.id.as_str(), InferredType::from_sce_type(&f.sce_type));
    }
    insert_stateless_imports(&mut ctx, imports);
    insert_stateful_imports(&mut ctx, imports);
    ctx
}

/// Empty context — no variables, no functions. Used for expressions that
/// have no access to named identifiers (e.g. default-value initializers
/// for internal fields that are pure literal constants).
pub fn empty() -> TypeCtx<'static> {
    TypeCtx::new()
}
