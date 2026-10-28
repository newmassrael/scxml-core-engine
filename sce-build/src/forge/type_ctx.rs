// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::types::{
    CallCrossing, EnumScope, FuncSig, InferredType, RecordShape, SlotCrossing, TypeCtx,
};

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
            imp.host_only.as_deref(),
            imp.may_fail,
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
///
/// A `may-fail` callee is registered as one; the call is refused where the
/// failure cannot be received, which is everywhere but a `may-fail`
/// algorithm's body ([`TypeCtx::receives_failures`]).
pub(crate) fn imported_callee_sig(
    params: &[SceType],
    ret: Option<&SceType>,
    host_only: Option<&str>,
    may_fail: bool,
) -> Option<FuncSig> {
    if let Some(slot) = host_only {
        return Some(FuncSig::host_only(slot));
    }
    Some(FuncSig {
        may_fail,
        ..FuncSig::new(
            params.iter().map(InferredType::from_sce_type).collect(),
            InferredType::from_sce_type(ret?),
        )
    })
}

/// A forge document a `sce-static` statechart imports and calls as `Alias(args)`
/// (docs/adr/0048): an algorithm (`<sce:import kind="algorithm">`), or a transform,
/// condition, lookup, interpolation or validator, the kinds that are functions of
/// their inputs. Read where the document is parsed: its alias, the document it
/// names, what kind of callee it is, and its signature as the forge import pass
/// discovers it. Language-free — the name a backend calls it by is the renderer's.
#[derive(Debug, Clone)]
pub struct StaticCallee {
    pub alias: String,
    /// The imported document's own name, from which a backend derives the
    /// symbol it emits.
    pub document_name: String,
    /// What a target calls ([`CalleeKind`]).
    pub kind: CalleeKind,
    /// How the call's value is read from what the function answers: a
    /// validator's validity, the value a lookup whose miss is an error found.
    pub reading: crate::forge::types::CallReading,
    pub params: Vec<SceType>,
    pub ret: Option<SceType>,
    /// Why only a host may call it (a list or record slot), if so.
    pub host_only: Option<String>,
    /// It declares `may-fail` (SCE_FORGE.md §3.4.1); a statechart does not
    /// receive a failure, so its expressions may not call it.
    pub may_fail: bool,
    /// The `<sce:import>` element's row, for a refusal of the import itself.
    pub line: Option<u32>,
    /// What it returns when that is a list of numbers or bools and it takes
    /// scalars only: the list a statechart may take, whole, into a list
    /// variable of its own, and nothing else a statechart may do with it.
    pub list_return: Option<StaticListReturn>,
    /// Why a statechart does not take the list it returns whole, when it returns
    /// one and `list_return` is `None`: what a refusal of the assignment says.
    pub list_refusal: Option<String>,
    /// The signature when the algorithm takes or returns a record, a list or a
    /// value of an enum in a way a statechart can call: each record or list slot is
    /// a variable of the statechart's, named at the call, and an enum slot is a
    /// value of an enum the statechart imports (docs/SCE_ACCEPTED_SUBSET.md §2.15).
    /// Set instead of `host_only`; the list of numbers a statechart already takes
    /// whole (`list_return`) is not this.
    pub value: Option<StaticValueSignature>,
}

/// What a `sce-static` statechart's imported callee is, as a target spells a call
/// of it (docs/adr/0048).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalleeKind {
    /// A free function the document emits, called with the arguments as they are:
    /// an algorithm's, a transform's for its one output, a condition's, a lookup's
    /// or an interpolation's, named as a forge kind that imports the document names
    /// it.
    Function(crate::forge::generator::StatelessFunction),
    /// A validator that keeps nothing from one check to the next, built for the call
    /// and asked once ([`crate::forge::generator::validator_check_call`]); its
    /// validity is the value ([`crate::forge::types::CallReading::Validity`]).
    Validator,
}

impl CalleeKind {
    /// The kind of the document, as its import names it (`kind="lookup"`).
    pub fn forge_kind(&self) -> ForgeKind {
        match self {
            Self::Function(function) => function.kind(),
            Self::Validator => ForgeKind::Validator,
        }
    }

    /// Whether it is an algorithm: the only callee whose slots may hold a record, a
    /// list or a value of an enum, and the only one the Interpreter lowers.
    pub fn is_algorithm(&self) -> bool {
        matches!(
            self,
            Self::Function(crate::forge::generator::StatelessFunction::Algorithm { .. })
        )
    }
}

/// One slot of an imported algorithm's signature, as a `sce-static` statechart
/// hands it a value or takes one back (docs/SCE_ACCEPTED_SUBSET.md §2.15,
/// "State an algorithm takes and gives back").
///
/// A record and a list of records are named by the alias the STATECHART imports
/// their schema under, which is how its own variables are typed: the algorithm
/// names the schema by an alias of its own, and the two are one type only where
/// they come from the same document (SCE_FORGE.md §4.12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticSlot {
    /// A fixed-width number, a bool, a string or bytes — or `enum:<alias>`, a value
    /// of an enum named, as a record is, by the alias the STATECHART imports the
    /// enum document under ([`Self::enum_alias`]).
    Scalar(SceType),
    /// `record:<alias>`.
    Record(String),
    /// `list<T>` of numbers or bools.
    ScalarList(SceType),
    /// `list<record:<alias>>`.
    RecordList(String),
    /// `optional<T>` over a fixed-width number, a `bool` or a text (docs/adr/0055):
    /// a value the statechart hands over as it holds one — an optional variable,
    /// `null`, or a value of `T`, lifted — and takes back into an optional
    /// variable. The same type on both sides, so it crosses as it is.
    Optional(SceType),
    /// `optional<record:<alias>>` (docs/adr/0055, step 9), named by the alias the
    /// STATECHART imports the schema under: a record the statechart holds in a
    /// variable of its own, passed by the name of that variable — an optional
    /// record, or a record, which is lifted — or by `null`, and taken back whole
    /// into an optional record variable.
    OptionalRecord(String),
}

impl StaticSlot {
    /// Whether the slot holds a value the statechart keeps in a variable of its own
    /// (a record or a list) and so passes by the name of that variable. A value
    /// that may be absent is a value, passed by any expression of its type.
    pub fn is_state(&self) -> bool {
        !matches!(self, Self::Scalar(_) | Self::Optional(_))
    }

    /// The alias the statechart imports the enum under, for a slot that holds a
    /// value of one, or an optional value of one (docs/adr/0055): an argument or
    /// return that crosses by its declared name, which the machine's type and the
    /// algorithm's spell apart.
    pub fn enum_alias(&self) -> Option<&str> {
        match self {
            Self::Scalar(SceType::Enum(reference)) | Self::Optional(SceType::Enum(reference)) => {
                Some(reference.alias.as_str())
            }
            _ => None,
        }
    }
}

/// The signature of an imported algorithm that takes or returns a record, a list or
/// a value of an enum, as a `sce-static` statechart calls it: the slots in order,
/// and for a list returned the most it may hold.
#[derive(Debug, Clone)]
pub struct StaticValueSignature {
    pub params: Vec<StaticSlot>,
    pub ret: Option<StaticSlot>,
    /// `sce:returns-max-size`, when the algorithm returns a list.
    pub max_size: Option<u32>,
}

impl StaticValueSignature {
    /// Whether a call is made through the adapter a target declares for it: a
    /// parameter is a record or a list the statechart holds, or a value of an
    /// enum, which the machine's type and the algorithm's spell apart and the
    /// adapter maps by declared name, as it does what an enum is returned as.
    pub fn adapted(&self) -> bool {
        self.params
            .iter()
            .any(|slot| slot.is_state() || slot.enum_alias().is_some())
            || self
                .ret
                .as_ref()
                .is_some_and(|slot| slot.enum_alias().is_some())
    }
}

/// A list an imported algorithm returns, as a `sce-static` statechart takes it
/// whole ([`StaticCallee::list_return`]).
#[derive(Debug, Clone)]
pub struct StaticListReturn {
    /// The scalar slots it takes, in order.
    pub params: Vec<SceType>,
    /// The type of each element it returns.
    pub elem: SceType,
    /// The most elements it may return (`sce:returns-max-size`).
    pub max_size: u32,
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
                FuncSig::new(params, InferredType::from_sce_type(ret_ty)),
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
                carrier: imp
                    .enum_underlying
                    .as_ref()
                    .map_or(InferredType::Unknown, InferredType::from_sce_type),
                to_number: imp.enum_to_number.as_str(),
                from_number: imp.enum_from_number.as_str(),
                open: imp.enum_is_open,
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
        ctx.insert_func(h.name.as_str(), FuncSig::new(params, ret));
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
    /// The integer the enum is carried as ([`EnumScope::carrier`]).
    pub carrier: SceType,
    /// What `<alias>.toNumber` and `<alias>.fromNumber` are lowered to
    /// ([`EnumScope::to_number`], [`EnumScope::from_number`]).
    pub to_number: String,
    pub from_number: String,
    /// The functions a value of the enum crosses through into the type the enum
    /// document declares and back, where a function of another kind the statechart
    /// calls holds it (docs/adr/0051); empty for a target whose machine holds the
    /// document's own type, and for a check that lowers nothing.
    pub to_document: String,
    pub from_document: String,
}

/// How a backend spells an enum a statechart imports: the type it holds a value
/// in, and the functions its two conversions are lowered to
/// ([`StaticEnum::from_imports`]), and the two it crosses into the enum
/// document's own type by ([`StaticEnum::to_document`]).
#[derive(Debug, Clone)]
pub struct StaticEnumSpelling {
    pub qualified_type: String,
    pub to_number: String,
    pub from_number: String,
    pub to_document: String,
    pub from_document: String,
}

impl StaticEnumSpelling {
    /// The spelling of a check that lowers nothing: the alias, and its two
    /// conversions as the document writes them.
    pub fn unlowered(alias: &str) -> Self {
        Self {
            qualified_type: alias.to_string(),
            to_number: format!("{alias}.{}", crate::forge::types::ENUM_TO_NUMBER),
            from_number: format!("{alias}.{}", crate::forge::types::ENUM_FROM_NUMBER),
            to_document: String::new(),
            from_document: String::new(),
        }
    }
}

impl StaticEnum {
    /// One per imported enum, keyed by alias, spelled as the backend spells it
    /// ([`StaticEnumSpelling`]) — [`StaticEnumSpelling::unlowered`] for a check
    /// that lowers nothing.
    pub fn from_imports(
        enums: &std::collections::BTreeMap<String, EnumModel>,
        spelling: impl Fn(&str, &EnumModel) -> StaticEnumSpelling,
    ) -> Vec<Self> {
        enums
            .iter()
            .map(|(alias, model)| {
                let spelled = spelling(alias, model);
                StaticEnum {
                    alias: alias.clone(),
                    variants: model.variants.iter().map(|v| v.name.clone()).collect(),
                    source_name: model.name.clone(),
                    qualified_type: spelled.qualified_type,
                    carrier: model.underlying_type.clone(),
                    to_number: spelled.to_number,
                    from_number: spelled.from_number,
                    to_document: spelled.to_document,
                    from_document: spelled.from_document,
                }
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
        // An element of a `list<record:<alias>>` read by its index, `xs[i].f`, is
        // read through `<id>[].<field>`: the index is a value and the path is
        // not, and a number, a bool, a string, a byte string or an enum is what
        // the element's fields can be read as. An enum field is registered untyped,
        // as a record variable's is: the lattice names no type for an enum, and
        // [`crate::forge::static_enum`] answers which enum the path holds.
        if let Some(ListElemType::Record { alias }) = var
            .value_type
            .as_ref()
            .and_then(AlgorithmValueType::list_elem)
        {
            if let Some(schema) = records.get(alias) {
                for field in &schema.fields {
                    let ty = InferredType::of_field(&field.sce_type);
                    // An optional field is read as the optional value it is
                    // (docs/adr/0055), whatever its element.
                    let readable = match &field.sce_type {
                        FieldType::Scalar(SceType::Enum(_)) | FieldType::Optional { .. } => true,
                        FieldType::Scalar(_) => matches!(
                            ty,
                            InferredType::Int { .. }
                                | InferredType::Float { .. }
                                | InferredType::Bool
                                | InferredType::Str
                                | InferredType::Bytes
                        ),
                    };
                    if readable {
                        paths.push((format!("{}{INDEXED_ELEMENT}{}", var.id, field.id), ty));
                    }
                }
            }
            continue;
        }
        let Some((alias, holder)) = record_holder(var) else {
            continue;
        };
        let Some(schema) = records.get(alias) else {
            continue;
        };
        for field in &schema.fields {
            paths.push((
                format!("{holder}.{}", field.id),
                InferredType::of_field(&field.sce_type),
            ));
        }
    }
    paths
}

/// The alias of the schema a variable holds a record of, and the path the
/// fields of that record are read under: `<id>` for a record variable, and
/// `<id>.value` for an optional record, whose fields are those of its present
/// value (docs/adr/0055, step 9). `None` for any other variable.
pub(crate) fn record_holder(var: &crate::model::Variable) -> Option<(&str, String)> {
    match var.value_type.as_ref()? {
        AlgorithmValueType::Record { alias } => Some((alias.as_str(), var.id.clone())),
        AlgorithmValueType::Optional {
            elem: OptionalElemType::Record { alias },
        } => Some((
            alias.as_str(),
            crate::forge::static_enum::present_value_path(&var.id),
        )),
        _ => None,
    }
}

/// What stands between a list's name and a field's in the path an indexed read
/// of a list of records is typed and spelled by: `days[].dayOfMonth` is the
/// field of `days[i]`, whatever `i` is.
pub const INDEXED_ELEMENT: &str = "[].";

/// The bound, in UTF-8 bytes, of each string field of a `record:<alias>`
/// variable, under the dotted path an expression reads it by, `<id>.<field>`:
/// the `sce:max-size` its schema declares. What a value that joins such a field
/// is sized from, as it is from a string variable's `sce:capacity`
/// ([`TypeCtx::string_capacities`]).
fn static_record_string_bounds<'v>(
    variables: impl IntoIterator<Item = &'v crate::model::Variable>,
    records: &std::collections::BTreeMap<String, EventSchemaModel>,
) -> Vec<(String, usize)> {
    let mut bounds = Vec::new();
    for var in variables {
        // A string field of the element a list of records is indexed at is bounded
        // by the schema alike, under the path it is typed by (`labels[].label`).
        if let Some(ListElemType::Record { alias }) = var
            .value_type
            .as_ref()
            .and_then(AlgorithmValueType::list_elem)
        {
            if let Some(schema) = records.get(alias) {
                for field in &schema.fields {
                    if let Some(bound) = text_field_bound(field) {
                        bounds.push((format!("{}{INDEXED_ELEMENT}{}", var.id, field.id), bound));
                    }
                }
            }
            continue;
        }
        let Some((alias, holder)) = record_holder(var) else {
            continue;
        };
        let Some(schema) = records.get(alias) else {
            continue;
        };
        for field in &schema.fields {
            if let Some(bound) = text_field_bound(field) {
                bounds.push((format!("{holder}.{}", field.id), bound));
            }
        }
    }
    bounds
}

/// The bound, in UTF-8 bytes, of a string field of a schema: its
/// `sce:max-size`. An optional text is bounded as the value it holds when
/// present is (docs/adr/0055). `None` for a field that is no text, or a text
/// the schema bounds by nothing.
fn text_field_bound(field: &SchemaField) -> Option<usize> {
    match (field.sce_type.present_type(), field.max_size) {
        (SceType::String, Some(bound)) => Some(bound as usize),
        _ => None,
    }
}

/// What a `<foreach>` over a list variable binds in its body (§scxml-4.6):
/// `item`, each element — typed as the list's element, or, for a list of
/// records, a closed record whose fields are the schema's — and `index`, its
/// position, a `uint32` as `len` is. One definition for the pass that judges
/// the body and the pass that lowers it, so neither sees a scope the other
/// does not.
pub struct LoopVariables {
    item: String,
    /// The item's type for a scalar element; `None` for a record.
    item_ty: Option<InferredType>,
    index: Option<String>,
    /// The typed `<item>.<field>` paths of a record item.
    paths: Vec<(String, InferredType)>,
    /// The bound of each string field of a record item, under the same paths.
    string_bounds: Vec<(String, usize)>,
}

impl LoopVariables {
    /// The variables a loop over a list of `elem` binds. A record's fields are
    /// read from `schemas`, the document's imports by alias.
    pub fn new(
        item: &str,
        index: Option<&str>,
        elem: &ListElemType,
        schemas: &std::collections::BTreeMap<String, EventSchemaModel>,
    ) -> Self {
        let (item_ty, paths) = match elem {
            ListElemType::Scalar(ty) => (Some(InferredType::from_sce_type(ty)), Vec::new()),
            ListElemType::Record { alias } => (
                None,
                schemas
                    .get(alias)
                    .into_iter()
                    .flat_map(|schema| &schema.fields)
                    .map(|field| {
                        (
                            format!("{item}.{}", field.id),
                            InferredType::of_field(&field.sce_type),
                        )
                    })
                    .collect(),
            ),
        };
        let string_bounds = match elem {
            ListElemType::Scalar(_) => Vec::new(),
            ListElemType::Record { alias } => schemas
                .get(alias)
                .into_iter()
                .flat_map(|schema| &schema.fields)
                .filter_map(|field| {
                    text_field_bound(field).map(|bound| (format!("{item}.{}", field.id), bound))
                })
                .collect(),
        };
        Self {
            item: item.to_string(),
            item_ty,
            index: index.map(str::to_string),
            paths,
            string_bounds,
        }
    }

    /// `ctx` with the loop's variables added: the scope of its body.
    pub fn bind<'s, 'c: 's>(&'s self, ctx: &TypeCtx<'c>) -> TypeCtx<'s> {
        let mut inner: TypeCtx<'s> = ctx.clone();
        match self.item_ty {
            Some(ty) => inner.insert_var(self.item.as_str(), ty),
            None => inner.insert_record(self.item.as_str(), RecordShape::Closed),
        }
        for (path, ty) in &self.paths {
            inner.insert_var(path.as_str(), *ty);
        }
        for (path, bound) in &self.string_bounds {
            inner.insert_string_capacity(path.as_str(), *bound);
        }
        if let Some(index) = &self.index {
            inner.insert_var(
                index.as_str(),
                InferredType::from_sce_type(&SceType::Uint32),
            );
        }
        inner
    }

    /// The fields of a record item that are strings, each bounded by the
    /// `sce:max-size` its schema declares: what a target that reads such a field
    /// through the buffer that holds it names (`<item>.<field>`).
    pub fn string_fields(&self) -> impl Iterator<Item = &str> {
        self.string_bounds
            .iter()
            .filter_map(|(path, _)| path.split_once('.').map(|(_, field)| field))
    }
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
    /// The bound of each string field of those ([`static_record_string_bounds`]).
    record_string_bounds: Vec<(String, usize)>,
    /// `<id>.value` of each optional record variable (docs/adr/0055, step 9): the
    /// closed record its present value's fields are read under.
    optional_record_values: Vec<String>,
    /// Every imported callee, an algorithm or a function of another kind
    /// ([`StaticCallee`]), callable as `Alias(args)`.
    pub callees: Vec<StaticCallee>,
    /// Each name a call of an algorithm with state in a slot can take
    /// ([`StateCallName`]), registered with the signature the call is judged by.
    state_calls: Vec<StateCallName>,
    /// Each bound the statechart names ([`Self::limits`]).
    limits: Vec<(String, BuildLimit)>,
}

/// What a build gives a bound a document names (`sce:limit`): the number every
/// element that names it resolved to, or the first two numbers two of them resolved
/// to when they disagree, which is no one value a constant or a read could have.
pub type BuildLimit = Result<u32, (u32, u32)>;

/// The bounds `named` (each a `sce:limit` name with the number the build resolved
/// its element to) as one value per name, in the order of the names
/// ([`BuildLimit`]).
pub(crate) fn named_limits<'n>(
    named: impl IntoIterator<Item = (&'n str, u32)>,
) -> Vec<(String, BuildLimit)> {
    let mut limits: std::collections::BTreeMap<String, BuildLimit> = Default::default();
    for (name, value) in named {
        let entry = limits.entry(name.to_string()).or_insert(Ok(value));
        if let Ok(held) = *entry {
            if held != value {
                *entry = Err((held, value));
            }
        }
    }
    limits.into_iter().collect()
}

/// How a state argument — a record or a list a statechart hands an algorithm — is held
/// where the call is, which decides how the call spells it: `v` a variable of the
/// machine, `p` the payload of the event the transition is on (`_event.data`), `i` the
/// item of a `<foreach>` over records.
pub const STATE_CODES: [char; 3] = ['v', 'p', 'i'];

/// How an argument of an optional record slot can be held, beside
/// [`STATE_CODES`]: `n` the literal `null`, which holds no variable and no
/// value, and is spelled as the absent value the slot's type has (docs/adr/0055,
/// step 9).
pub const OPTIONAL_RECORD_CODES: [char; 4] = ['v', 'p', 'i', 'n'];

/// The names the arguments of a call of an algorithm with state in a slot take in the
/// expression a lowering reads instead of the one the author wrote (docs/SCE_ACCEPTED_SUBSET.md
/// §2.15). There are as many as a call has state arguments in one expression.
pub const STATE_PLACEHOLDERS: [&str; 8] = [
    "__st0", "__st1", "__st2", "__st3", "__st4", "__st5", "__st6", "__st7",
];

/// A call of an algorithm with state in a slot, by the name the rewritten expression
/// gives it: the algorithm's alias and one code per state slot that says how its
/// argument is held, so that each way an argument is held has the one adapter a
/// target writes it (`AddEntry__v`, `AmountOf__p`).
#[derive(Debug, Clone)]
pub struct StateCallName {
    pub name: String,
    pub alias: String,
    /// One code ([`STATE_CODES`]) per state parameter, in order.
    pub codes: Vec<char>,
}

/// One state argument of a call, as the rewrite names it.
#[derive(Debug, Clone)]
pub struct StateArg {
    /// The name the rewritten expression holds in its place ([`STATE_PLACEHOLDERS`]).
    pub placeholder: String,
    pub code: char,
    /// What the author wrote: a variable's or a loop item's name, or `_event.data`.
    pub written: String,
    pub slot: StaticSlot,
    /// The alias of the algorithm it is an argument of.
    pub callee: String,
}

/// An expression with its state calls rewritten ([`StaticScope::rewrite_state_calls`]).
#[derive(Debug, Clone, Default)]
pub struct StateRewrite {
    /// The expression a judge and a lowering read: each state argument replaced by its
    /// placeholder and each such callee by its [`StateCallName`].
    pub text: String,
    pub args: Vec<StateArg>,
    /// The [`StateCallName`]s the expression calls.
    pub calls: Vec<String>,
}

/// A call of an algorithm with state in a slot that the statechart cannot make, and why.
#[derive(Debug, Clone)]
pub struct StateCallRefusal {
    pub callee: String,
    pub reason: String,
}

/// Where a state argument may come from besides a variable of the machine.
pub struct StateContext<'a> {
    /// The record items of the `<foreach>`es the expression is inside, each with the
    /// alias of the schema it holds.
    pub loop_records: &'a [(String, String)],
    /// The alias of the schema the triggering event carries, if it carries one — what
    /// `_event.data` is a record of.
    pub payload_alias: Option<&'a str>,
    /// Whether that schema has a `string` field. A payload's text is not yet held to
    /// the bound its schema declares — the assignment that takes it into a variable
    /// is what holds it — so a payload that carries text is not handed to an
    /// algorithm whole (docs/SCE_ACCEPTED_SUBSET.md §2.15, "A record's string
    /// field").
    pub payload_holds_text: bool,
}

/// The signature a call of `callee` is judged by when its slots are scalars, a record or
/// a list: a state slot is typed `Unknown` (the argument is replaced by a placeholder
/// before the call is judged, and was checked against the slot when it was), a scalar
/// slot as the algorithm declares it, and the return as what the judge needs of it.
fn value_func_sig(callee: &StaticCallee) -> Option<FuncSig> {
    let value = callee.value.as_ref()?;
    let optional = |t: &SceType| InferredType::Optional(crate::forge::types::OptionalElem::of(t));
    let params = value
        .params
        .iter()
        .map(|slot| match slot {
            StaticSlot::Scalar(t) => InferredType::from_sce_type(t),
            StaticSlot::Optional(t) => optional(t),
            StaticSlot::Record(_)
            | StaticSlot::OptionalRecord(_)
            | StaticSlot::ScalarList(_)
            | StaticSlot::RecordList(_) => InferredType::Unknown,
        })
        .collect();
    let ret = match value.ret.as_ref()? {
        StaticSlot::Scalar(t) => InferredType::from_sce_type(t),
        StaticSlot::Optional(t) => optional(t),
        StaticSlot::Record(_) => InferredType::Unknown,
        StaticSlot::OptionalRecord(_) => {
            InferredType::Optional(crate::forge::types::OptionalElem::Record)
        }
        StaticSlot::ScalarList(t) => InferredType::List(crate::forge::types::ListElem::of(t)?),
        StaticSlot::RecordList(_) => InferredType::List(crate::forge::types::ListElem::Record),
    };
    Some(FuncSig {
        may_fail: callee.may_fail,
        ..FuncSig::new(params, ret)
    })
}

/// The name of a call of `alias` whose state arguments are held as `codes`.
pub fn state_call_name(alias: &str, codes: &[char]) -> String {
    format!("{alias}__{}", codes.iter().collect::<String>())
}

/// Every way the state parameters of `value` can be held: a record parameter by
/// a variable, a payload or a loop's item, a list parameter by a variable.
fn state_code_combinations(value: &StaticValueSignature) -> Vec<Vec<char>> {
    let mut combos: Vec<Vec<char>> = vec![Vec::new()];
    for slot in value.params.iter().filter(|s| s.is_state()) {
        let options: &[char] = match slot {
            StaticSlot::Record(_) => &STATE_CODES,
            StaticSlot::OptionalRecord(_) => &OPTIONAL_RECORD_CODES,
            _ => &['v'],
        };
        combos = combos
            .into_iter()
            .flat_map(|prefix| {
                options.iter().map(move |code| {
                    let mut next = prefix.clone();
                    next.push(*code);
                    next
                })
            })
            .collect();
    }
    combos
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
        let record_string_bounds =
            static_record_string_bounds(variables.iter(), &model.imported_records);
        let optional_record_values = variables
            .iter()
            .filter(|v| {
                v.value_type
                    .as_ref()
                    .and_then(AlgorithmValueType::optional_elem)
                    .is_some_and(|elem| elem.record_alias().is_some())
            })
            .map(|v| crate::forge::static_enum::present_value_path(&v.id))
            .collect();
        let limits = named_limits(
            variables
                .iter()
                .filter_map(|v| Some((v.limit.as_deref()?, v.capacity?)))
                .chain(model.imported_records.values().flat_map(|schema| {
                    schema
                        .fields
                        .iter()
                        .filter_map(|f| Some((f.limit.as_deref()?, f.max_size?)))
                })),
        );
        let callees = model.imported_callees.clone();
        let state_calls = callees
            .iter()
            .filter_map(|callee| Some((callee, callee.value.as_ref()?)))
            .filter(|(_, value)| value.adapted())
            .flat_map(|(callee, value)| {
                state_code_combinations(value)
                    .into_iter()
                    .map(move |codes| StateCallName {
                        name: state_call_name(&callee.alias, &codes),
                        alias: callee.alias.clone(),
                        codes,
                    })
            })
            .collect();
        Some(Self {
            variables,
            record_paths,
            record_string_bounds,
            optional_record_values,
            callees,
            state_calls,
            limits,
        })
    }

    /// Each bound the statechart names (`sce:limit`) with the number the build
    /// resolved it to, by name ([`named_limits`]): what a generated machine declares a
    /// constant for and what `_x.limits.<name>` reads (docs/adr/0050).
    pub fn limits(&self) -> &[(String, BuildLimit)] {
        &self.limits
    }

    /// The call `alias` names in this scope, when it is of an algorithm whose slots
    /// are scalars, a record or a list.
    pub fn value_callee(&self, alias: &str) -> Option<&StaticCallee> {
        self.callees
            .iter()
            .find(|c| c.alias == alias && c.value.is_some())
    }

    /// The imported callee `expr` is, taken whole, a call of — when its slots are
    /// scalars, a record or a list ([`StaticCallee::value`]).
    pub fn whole_value_call(&self, expr: &str) -> Option<&StaticCallee> {
        let (whole, _) = crate::forge::expr::called_names(expr).ok()?;
        self.value_callee(&whole?)
    }

    /// Whether `callee` returns a record or a list: a value that is the right side of
    /// an assignment to a variable of its type and nothing else. An optional record
    /// is one too (docs/adr/0055, step 9): taken whole into an optional record
    /// variable.
    pub fn returns_state(callee: &StaticCallee) -> bool {
        callee.value.as_ref().is_some_and(|v| {
            matches!(
                v.ret,
                Some(
                    StaticSlot::Record(_)
                        | StaticSlot::OptionalRecord(_)
                        | StaticSlot::ScalarList(_)
                        | StaticSlot::RecordList(_)
                )
            )
        })
    }

    /// The alias of a callee that returns a record or a list when `expr` calls one
    /// anywhere but as its whole: such a value is held by an assignment and read from
    /// the variable, never an operand, an argument or a condition.
    pub fn state_returning_call_used_as_value(&self, expr: &str) -> Option<String> {
        let (_, others) = crate::forge::expr::called_names(expr).ok()?;
        others
            .into_iter()
            .find(|name| self.value_callee(name).is_some_and(Self::returns_state))
    }

    /// The call of an algorithm with state in a slot that `name` ([`state_call_name`])
    /// stands for.
    pub fn state_call(&self, name: &str) -> Option<&StateCallName> {
        self.state_calls.iter().find(|c| c.name == name)
    }

    /// The enum each slot of the call `name` holds — `name` the callee's alias, or the
    /// name a rewrite gave its call ([`state_call_name`]) — what
    /// [`crate::forge::static_enum`] holds an argument and the value of the call to.
    /// `None` for a name that is no call of an imported algorithm this scope reads.
    pub(crate) fn call_enums(&self, name: &str) -> Option<crate::forge::static_enum::CallEnums> {
        let alias = self
            .state_call(name)
            .map_or(name, |call| call.alias.as_str());
        if let Some(value) = self.value_callee(alias).and_then(|c| c.value.as_ref()) {
            // A slot of a value of an enum, or of an optional value of one
            // (docs/adr/0055), which an argument and the call are held to.
            let held = |slot: &StaticSlot| {
                slot.enum_alias()
                    .map(|alias| crate::forge::static_enum::EnumSlot {
                        alias: alias.to_string(),
                        optional: matches!(slot, StaticSlot::Optional(_)),
                    })
            };
            return Some(crate::forge::static_enum::CallEnums {
                callee: alias.to_string(),
                params: value.params.iter().map(held).collect(),
                ret: value.ret.as_ref().and_then(held),
            });
        }
        // A function of another kind whose slot holds a value of an enum the
        // statechart imports (docs/adr/0051): its slots are the statechart's own.
        let callee = self
            .callees
            .iter()
            .find(|c| c.alias == alias && !c.kind.is_algorithm())?;
        let held = |ty: &SceType| match ty {
            SceType::Enum(reference) => Some(crate::forge::static_enum::EnumSlot::present(
                reference.alias.clone(),
            )),
            _ => None,
        };
        let params: Vec<Option<crate::forge::static_enum::EnumSlot>> =
            callee.params.iter().map(held).collect();
        let ret = callee.ret.as_ref().and_then(held);
        (ret.is_some() || params.iter().any(Option::is_some)).then(|| {
            crate::forge::static_enum::CallEnums {
                callee: alias.to_string(),
                params,
                ret,
            }
        })
    }

    /// How the argument `shape` of a call is held, when it is one the slot `slot` takes.
    fn classify_state_arg(
        &self,
        slot: &StaticSlot,
        shape: &crate::forge::expr::ArgShape,
        context: &StateContext<'_>,
    ) -> Result<char, String> {
        use crate::forge::expr::ArgShape;
        let variable = |name: &str| self.variables.iter().find(|v| v.id == name);
        let list_of = |name: &str| {
            variable(name)
                .and_then(|v| v.value_type.as_ref())
                .and_then(AlgorithmValueType::list_elem)
        };
        match (slot, shape) {
            // An optional record takes `null`, an optional record variable of its
            // schema, and a record of its schema as a present value lifted
            // (docs/adr/0055, step 9): the three ways a record is held that are not
            // a record slot's own are an optional variable and the literal.
            (StaticSlot::OptionalRecord(_), ArgShape::Null) => Ok('n'),
            (StaticSlot::OptionalRecord(alias), shape) => {
                if let ArgShape::Name(name) = shape {
                    let held = variable(name)
                        .and_then(|v| v.value_type.as_ref())
                        .and_then(AlgorithmValueType::optional_elem)
                        .and_then(OptionalElemType::record_alias);
                    match held {
                        Some(held) if held == alias => return Ok('v'),
                        Some(held) => {
                            return Err(format!(
                                "`{name}` is an optional record of `{held}`, and the algorithm \
                                 takes an optional record of `{alias}`"
                            ))
                        }
                        None => {}
                    }
                }
                self.classify_state_arg(&StaticSlot::Record(alias.clone()), shape, context)
            }
            (StaticSlot::Record(alias), ArgShape::Name(name)) => {
                if let Some(var) = variable(name) {
                    return match var
                        .value_type
                        .as_ref()
                        .and_then(AlgorithmValueType::record_alias)
                    {
                        Some(held) if held == alias => Ok('v'),
                        Some(held) => Err(format!(
                            "`{name}` is a record of `{held}`, and the algorithm takes a record \
                             of `{alias}`"
                        )),
                        None => Err(format!("`{name}` is not a record variable")),
                    };
                }
                match context.loop_records.iter().find(|(item, _)| item == name) {
                    Some((_, held)) if held == alias => Ok('i'),
                    Some((_, held)) => Err(format!(
                        "`{name}` is a record of `{held}`, and the algorithm takes a record of \
                         `{alias}`"
                    )),
                    None => Err(format!(
                        "`{name}` is neither a record variable nor the item of a <foreach> over \
                         records"
                    )),
                }
            }
            (StaticSlot::Record(_), ArgShape::EventData) if context.payload_holds_text => Err(
                "the payload of the event holds text, which is not held to the bound its \
                 schema declares until it is taken into a variable: assign `_event.data` to a \
                 record variable and hand the algorithm that"
                    .to_string(),
            ),
            (StaticSlot::Record(alias), ArgShape::EventData) => match context.payload_alias {
                Some(held) if held == alias => Ok('p'),
                Some(held) => Err(format!(
                    "the payload of the event is a record of `{held}`, and the algorithm takes a \
                     record of `{alias}`"
                )),
                None => Err("the event this transition is on carries no schema, so \
                             `_event.data` is no record"
                    .to_string()),
            },
            (StaticSlot::ScalarList(elem), ArgShape::Name(name)) => match list_of(name) {
                Some(ListElemType::Scalar(held)) if held == elem => Ok('v'),
                Some(ListElemType::Scalar(held)) => Err(format!(
                    "`{name}` is a list of {}, and the algorithm takes a list of {}",
                    held.as_attr(),
                    elem.as_attr()
                )),
                Some(ListElemType::Record { .. }) => Err(format!(
                    "`{name}` is a list of records, not of {}",
                    elem.as_attr()
                )),
                None => Err(format!("`{name}` is not a list variable")),
            },
            (StaticSlot::RecordList(alias), ArgShape::Name(name)) => match list_of(name) {
                Some(ListElemType::Record { alias: held }) if held == alias => Ok('v'),
                Some(ListElemType::Record { alias: held }) => Err(format!(
                    "`{name}` is a list of `{held}`, and the algorithm takes a list of `{alias}`"
                )),
                Some(ListElemType::Scalar(_)) => {
                    Err(format!("`{name}` is a list of numbers, not of records"))
                }
                None => Err(format!("`{name}` is not a list variable")),
            },
            _ => Err(
                "a record or a list is handed over by the name of the variable that holds \
                      it, or `_event.data` for the payload of the event, and not by an \
                      expression that computes one"
                    .to_string(),
            ),
        }
    }

    /// `expr` with each call of an algorithm that takes a record or a list rewritten
    /// into the call the judge and the lowering read: the argument in a state slot is
    /// replaced by a placeholder ([`STATE_PLACEHOLDERS`]), checked to be the name of a
    /// variable of the right type, and the callee takes the name that says how the
    /// argument is held ([`state_call_name`]). An expression that calls no such
    /// algorithm is returned as it is.
    pub fn rewrite_state_calls(
        &self,
        expr: &str,
        context: &StateContext<'_>,
    ) -> Result<StateRewrite, StateCallRefusal> {
        let wanted = |name: &str| {
            self.value_callee(name)
                .and_then(|c| c.value.as_ref())
                .is_some_and(StaticValueSignature::adapted)
        };
        // A parse error is the judge's to report, in its own words.
        let Ok(sites) = crate::forge::expr::call_sites(expr, &wanted) else {
            return Ok(StateRewrite {
                text: expr.to_string(),
                ..StateRewrite::default()
            });
        };
        let mut rewrite = StateRewrite::default();
        let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
        for site in sites {
            let callee = self
                .value_callee(&site.callee)
                .and_then(|c| c.value.as_ref())
                .expect("call_sites answers a callee `wanted` accepted");
            let refuse = |reason: String| StateCallRefusal {
                callee: site.callee.clone(),
                reason,
            };
            if site.args.len() != callee.params.len() {
                return Err(refuse(format!(
                    "it takes {} arguments, and the call gives {}",
                    callee.params.len(),
                    site.args.len()
                )));
            }
            let mut codes = Vec::new();
            for (slot, arg) in callee.params.iter().zip(&site.args) {
                if !slot.is_state() {
                    continue;
                }
                let code = self
                    .classify_state_arg(slot, &arg.shape, context)
                    .map_err(refuse)?;
                let Some(placeholder) = STATE_PLACEHOLDERS.get(rewrite.args.len()) else {
                    return Err(refuse(format!(
                        "an expression hands at most {} records or lists to algorithms",
                        STATE_PLACEHOLDERS.len()
                    )));
                };
                edits.push((arg.span.clone(), (*placeholder).to_string()));
                rewrite.args.push(StateArg {
                    placeholder: (*placeholder).to_string(),
                    code,
                    written: expr[arg.span.clone()].trim().to_string(),
                    slot: slot.clone(),
                    callee: site.callee.clone(),
                });
                codes.push(code);
            }
            let name = state_call_name(&site.callee, &codes);
            edits.push((site.callee_span.clone(), name.clone()));
            rewrite.calls.push(name);
        }
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.0.start));
        let mut text = expr.to_string();
        for (span, replacement) in edits {
            text.replace_range(span, &replacement);
        }
        rewrite.text = text;
        Ok(rewrite)
    }

    /// The `(variable, field)` of each string field of a record variable: the
    /// places a target that holds a string in a buffer of its own reads through it.
    pub fn record_string_fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.record_string_bounds
            .iter()
            .filter(|(path, _)| !path.contains(INDEXED_ELEMENT))
            // The field of an optional record's present value is read through a
            // path of three names, `<id>.value.<field>`, which no target holds in
            // a buffer of its own yet (docs/adr/0055, step 9).
            .filter(|(path, _)| path.matches('.').count() == 1)
            .filter_map(|(path, _)| path.split_once('.'))
    }

    /// Whether the field at `path`, as [`Self::indexed_record_fields`] gives it, is a
    /// string: the one a target that holds a string in a buffer of its own reads
    /// through that buffer.
    pub fn indexed_field_is_string(&self, path: &str) -> bool {
        self.record_string_bounds
            .iter()
            .any(|(held, _)| held == path)
    }

    /// Each field of the element a list of records is indexed at, as the path it
    /// is typed by (`days[].dayOfMonth`) and the field's own id — what a target
    /// spells with the name it gives a record's field.
    pub fn indexed_record_fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.record_paths.iter().filter_map(|(path, _)| {
            path.split_once(INDEXED_ELEMENT)
                .map(|(_, field)| (path.as_str(), field))
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

    /// The imported callee `expr` is, taken whole, a call of — and that returns a
    /// list a statechart may take into a list variable — or `None`.
    pub fn whole_list_call(&self, expr: &str) -> Option<&StaticCallee> {
        let (whole, _) = crate::forge::expr::called_names(expr).ok()?;
        let whole = whole?;
        self.callees
            .iter()
            .find(|c| c.alias == whole && c.list_return.is_some())
    }

    /// The imported callee `expr` is, taken whole, a call of that returns a list a
    /// statechart does not take — and why not, as [`StaticCallee::list_refusal`] words it.
    pub fn refused_list_call(&self, expr: &str) -> Option<(&StaticCallee, &str)> {
        let (whole, _) = crate::forge::expr::called_names(expr).ok()?;
        let whole = whole?;
        self.callees
            .iter()
            .find(|c| c.alias == whole)
            .and_then(|c| c.list_refusal.as_deref().map(|reason| (c, reason)))
    }

    /// The alias of a callee that returns a list when `expr` calls one anywhere
    /// but as its whole: a list a call returns is a value only as the right side
    /// of an assignment to a list variable, never an operand, an argument, a
    /// guard or an index's object — the rule a list variable is held to
    /// ([`Self::list_read_as_value`]).
    pub fn list_returning_call_used_as_value(&self, expr: &str) -> Option<String> {
        let (_, others) = crate::forge::expr::called_names(expr).ok()?;
        others.into_iter().find(|name| {
            self.callees
                .iter()
                .any(|c| &c.alias == name && c.list_return.is_some())
        })
    }

    /// The [`TypeCtx`] over this scope with `paths` ([`Self::paths`]) in it.
    pub fn ctx<'a>(
        &'a self,
        paths: &'a [(String, InferredType)],
        enums: &'a [StaticEnum],
    ) -> TypeCtx<'a> {
        let mut ctx = static_statechart(self.variables.iter(), paths, enums);
        for (path, bound) in &self.record_string_bounds {
            ctx.insert_string_capacity(path.as_str(), *bound);
        }
        // The present value of an optional record is a closed record of the
        // schema's fields (`paths` types each as `<id>.value.<field>`), so a
        // member it does not declare is `expression/unknown-member`.
        for name in &self.optional_record_values {
            ctx.insert_record(name.as_str(), RecordShape::Closed);
        }
        // What `_x.limits.<name>` reads (`_x` is the system variable a session
        // reserves for what a platform adds): the bounds the statechart names.
        for (name, value) in &self.limits {
            ctx.limits.insert(name.as_str(), *value);
        }
        // The names a record or a list handed to an algorithm takes in the expression
        // read instead of the author's ([`Self::rewrite_state_calls`]): the placeholders,
        // which stand for a value of the machine the call is lowered with, and each way
        // of calling an algorithm that takes one.
        for placeholder in STATE_PLACEHOLDERS {
            ctx.insert_var(placeholder, InferredType::Unknown);
        }
        for call in &self.state_calls {
            if let Some(sig) = self
                .callees
                .iter()
                .find(|c| c.alias == call.alias)
                .and_then(value_func_sig)
            {
                ctx.insert_func(call.name.as_str(), sig);
            }
        }
        for callee in &self.callees {
            // An algorithm that takes no record or list but returns one is called by
            // its own alias, and the whole of the call is the right side of an
            // assignment. One with a value of an enum in a slot is called through its
            // adapter, by the name its rewrite gives it, as one with state is.
            if callee.value.as_ref().is_some_and(|v| !v.adapted()) {
                if let Some(sig) = value_func_sig(callee) {
                    ctx.insert_func(callee.alias.as_str(), sig);
                }
            }
            // A validator's check and a lookup whose miss is an error answer
            // something other than the value: the call is read as the callee says
            // ([`crate::forge::types::CallReading`]).
            if let Some(sig) = imported_callee_sig(
                &callee.params,
                callee.ret.as_ref(),
                callee.host_only.as_deref(),
                callee.may_fail,
            ) {
                ctx.insert_func(
                    callee.alias.as_str(),
                    FuncSig {
                        reading: callee.reading,
                        ..sig
                    },
                );
            }
            // A callee returning a list a statechart takes whole is typed by what
            // it takes and returns, so the one place it may stand —
            // the right side of an assignment to a list variable — is judged and
            // lowered as any call is. Anywhere else the judge refuses it
            // ([`Self::list_returning_call_used_as_value`]).
            if let Some(returned) = &callee.list_return {
                if let Some(elem) = crate::forge::types::ListElem::of(&returned.elem) {
                    ctx.insert_func(
                        callee.alias.as_str(),
                        FuncSig {
                            // Only an algorithm that declares `may-fail` is taken
                            // ([`StatelessSignature::list_return`]), so the call hands
                            // its caller a failure to receive.
                            may_fail: true,
                            ..FuncSig::new(
                                returned
                                    .params
                                    .iter()
                                    .map(InferredType::from_sce_type)
                                    .collect(),
                                InferredType::List(elem),
                            )
                        },
                    );
                }
            }
        }
        // How each argument of a call of a function of another kind, and the value
        // it answers, cross between the machine and the function (docs/adr/0051): a
        // value of an enum through the functions the target declares for it, a text
        // as the target lends one.
        for callee in self.callees.iter().filter(|c| !c.kind.is_algorithm()) {
            let params: Vec<SlotCrossing<'a>> = callee
                .params
                .iter()
                .map(|ty| match ty {
                    SceType::Enum(reference) => {
                        enum_crossing_function(enums, &reference.alias, Crossing::ToDocument)
                            .map_or(SlotCrossing::AsIs, SlotCrossing::Enum)
                    }
                    SceType::String => SlotCrossing::Text,
                    _ => SlotCrossing::AsIs,
                })
                .collect();
            let ret = match callee.ret.as_ref() {
                Some(SceType::Enum(reference)) => {
                    enum_crossing_function(enums, &reference.alias, Crossing::FromDocument)
                }
                _ => None,
            };
            if ret.is_some() || params.iter().any(|p| *p != SlotCrossing::AsIs) {
                ctx.crossings
                    .insert(callee.alias.as_str(), CallCrossing { params, ret });
            }
        }
        // E12 D5: a machine receives the failures of what it runs — an
        // integer operation that overflows, a call of a callee that declares
        // `may-fail` — as `error.execution`, so every integer operation is
        // checked and such a call is admitted (SCE_FORGE.md §3.4.1).
        ctx.receives_failures = true;
        ctx
    }
}

/// Which way [`enum_crossing_function`] crosses.
#[derive(Debug, Clone, Copy)]
enum Crossing {
    ToDocument,
    FromDocument,
}

/// The function a value of the enum `alias` crosses through, into the type its
/// document declares or back, as the target spells it ([`StaticEnum::to_document`]);
/// `None` where the target names none, since its machine holds the document's own
/// type, or the scope does not import the enum.
fn enum_crossing_function<'e>(
    enums: &'e [StaticEnum],
    alias: &str,
    crossing: Crossing,
) -> Option<&'e str> {
    let held = enums.iter().find(|e| e.alias == alias)?;
    let function = match crossing {
        Crossing::ToDocument => held.to_document.as_str(),
        Crossing::FromDocument => held.from_document.as_str(),
    };
    (!function.is_empty()).then_some(function)
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
    // A statechart reads the text of a join where it computes it — a delay, an
    // event name, an id, a request's source — so a buffer sized for that one
    // evaluation is all it needs.
    ctx.joins_into_buffers = true;
    // A length is a `uint32` (SCE Accepted Subset §2.15), so arithmetic over
    // one is checked like any other integer operation.
    ctx.lengths_are_uint32 = true;
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
            // Typed as a list so `len(…)` measures it and `xs[i]` reads one
            // element, as a number of the element's own type; the static data
            // model's judge refuses it anywhere it would be read as a value.
            // A list of records has no operand element — `xs[i]` is refused
            // ([`crate::forge::expr`]'s unnamed-record-element rule).
            if let Some(list_elem) = crate::forge::types::ListElem::of_list(elem) {
                ctx.insert_var(var.id.as_str(), InferredType::List(list_elem));
            }
            if let Some(scalar) = elem.scalar() {
                ctx.insert_array_elem(var.id.as_str(), InferredType::from_sce_type(scalar));
            }
            continue;
        }
        // An optional variable is the optional value it holds (docs/adr/0055).
        let ty = match value_type.optional_elem() {
            Some(elem) => InferredType::Optional(crate::forge::types::OptionalElem::of_value(elem)),
            None => value_type
                .scalar()
                .map_or(InferredType::Unknown, InferredType::from_sce_type),
        };
        ctx.insert_var(var.id.as_str(), ty);
        // The bound a string is held to is what a value that joins it is sized
        // from, on a target that stores a string in a buffer of its own.
        if let (InferredType::Str, Some(capacity)) = (ty, var.capacity) {
            ctx.insert_string_capacity(var.id.as_str(), capacity as usize);
        }
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
                carrier: InferredType::from_sce_type(&e.carrier),
                to_number: e.to_number.as_str(),
                from_number: e.from_number.as_str(),
                // A statechart holds a closed enum only: an open one is refused
                // where a variable or a field declares it.
                open: false,
            },
        );
    }
    // `In(stateID)` is the one predicate every data model provides. Its
    // signature is declared here so the closed scope admits it; lowering it
    // is each backend's.
    ctx.insert_func(
        "In",
        FuncSig::new(vec![InferredType::Str], InferredType::Bool),
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
