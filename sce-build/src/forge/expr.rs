// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge expression transpiler — ECMAScript subset → target language.
//
// Architecture:
//
//   source text
//     │
//     ▼
//   tokenize  (`tokenize`)
//     │
//     ▼
//   parse     (`Parser::parse_expression`)
//     │   produces a `TypedExpr` whose every node has `ty: Unknown`
//     ▼
//   infer     (`infer_types`)
//     │   bottom-up annotation using `TypeCtx` (variable types + function sigs)
//     │   and the lattice in `forge::types::{join_arith, join_int}`
//     ▼
//   rename    (`rename_identifiers`, optional)
//     │   applies user-supplied ident → ident map (e.g. `_event.data` collapse,
//     │   datamodel → struct field rename, cross-file alias → qualified call).
//     │   Runs *after* `infer_types` so each leaf can still bind its type from
//     │   the TypeCtx using its original name; the rename only changes node
//     │   `kind`s, never `ty` slots.
//     ▼
//   emit      (`emit_cpp` / `emit_rust` / `emit_go` / `emit_kotlin` / `emit_python`)
//         each emitter consumes the typed AST plus an `expected: InferredType`
//         context propagated top-down, and inserts language-specific coercions
//         at the points where `child.ty != expected` or where operand types in
//         a binary operation differ from the computed result type
//
// Design principles:
//
// * **No post-emit regex hacking**: all type-aware behavior lives in this file.
//   The generator passes a `TypeCtx` and an expected output type; the emitter
//   handles every coercion, cast, and literal promotion inside its own arm.
//
// * **Untyped literal polymorphism**: decimal integer literals are `UntypedInt`,
//   decimal floats are `UntypedFloat`. They adopt the context type. Example:
//   `celsius * 9 / 5 + 32` with `celsius: f64` emits `celsius * 9.0 / 5.0 + 32.0`
//   in Rust because the untyped literals flow into a float context.
//
// * **Hex/binary/octal literals reject float promotion**: `x * 0xFF` with
//   `x: f64` is an error (`cannot coerce hex/oct/bin literal '0xFF' to float`),
//   not a silent cast. Users must use a decimal float literal explicitly.
//
// * **Unknown is contagious but non-fatal**: missing identifiers, opaque
//   member accesses, and unresolved function calls produce `Unknown`, which
//   propagates through `join_arith` / `join_int`. Emitters emit the ident
//   verbatim (no cast) when operand type is `Unknown` — the generated code
//   relies on the target language to reject invalid expressions. This matches
//   how the old untyped transpiler worked, so pre-typed use cases (e.g.
//   built-in `computeKey(seed)` helpers) still compile.
//
// * **`>>>` unsigned right shift**: Rust/C++/Go have no unsigned-shift operator,
//   so `>>>` maps to `>>`. SCXML authors must ensure operands are unsigned or
//   use explicit masking. Kotlin has `ushr` and Python promotes to big-int.

use crate::forge::error::{ExprError, Spanned};
use crate::forge::types::{join_arith, join_int, InferredType, TypeCtx};
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

/// A refusal of the expression the pipeline was handed, and the range of it
/// the refusal was raised at ([`Spanned`]) — what every entry point below
/// answers with, so the caller holding the attribute can place it.
pub type Refusal = Spanned<ExprError>;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Public API
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Target language for expression transpilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprTarget {
    Cpp,
    Kotlin,
    Rust,
    Go,
    Python,
    /// C11 backend. Identifier emission is snake-case (matching
    /// the C convention and Rust/Python's ident handling); coercion rules
    /// mirror C++ (implicit widening, decimal-integer-to-float `.0`
    /// promotion under push-down). The emitter is restricted to operators
    /// the transform fixtures exercise.
    C,
}

impl ExprTarget {
    /// The code generator's name for this backend — what
    /// [`crate::forge::enum_naming`] spells a variant reference for.
    pub(crate) fn language(self) -> crate::generator::Language {
        use crate::generator::Language;
        match self {
            ExprTarget::Cpp => Language::Cpp,
            ExprTarget::Kotlin => Language::Kotlin,
            ExprTarget::Rust => Language::Rust,
            ExprTarget::Go => Language::Go,
            ExprTarget::Python => Language::Python,
            ExprTarget::C => Language::C11,
        }
    }

    /// Every backend SCE generates code for. This is the canonical list a
    /// "lowers natively on all backends" verdict must exercise — checking
    /// a proper subset (historically just Rust + Go) is a proxy that lies
    /// the moment one backend's emitter can fail where another cannot
    /// (e.g. an unrepresentable `bytes` form). Kept exhaustive by
    /// `expr_target_all_contains_every_variant`.
    pub(crate) const ALL: [ExprTarget; 6] = [
        ExprTarget::Cpp,
        ExprTarget::Kotlin,
        ExprTarget::Rust,
        ExprTarget::Go,
        ExprTarget::Python,
        ExprTarget::C,
    ];
}

/// Transpile an ECMAScript expression to the target language with full
/// type-aware coercion.
///
/// * `expr` — source expression in the ECMAScript subset permitted by
///   Extended SCXML (no `new`, `var`, `let`, arrow functions, template
///   literals, spread, optional chaining, nullish coalescing; strict
///   equality required).
/// * `target` — the language to emit.
/// * `ctx` — typed context: `vars` maps identifier names to their
///   [`InferredType`]; `funcs` maps function names (typically cross-file
///   imports) to their signatures.
/// * `renames` — identifier renaming applied before inference. This lets
///   callers map e.g. `_event.data` → `pendingEventData_` or camelCase
///   datamodel names → member-field names. Both plain idents and
///   `object.property` paths (as a single key like `"_event.data"`) are
///   recognized; see [`rename_identifiers`] for the full contract.
/// * `expected` — the type the enclosing context expects this whole
///   expression to produce. Drives top-level literal promotion and
///   back-conversion. Pass [`InferredType::Unknown`] if the caller does not
///   have a firm expectation (no top-level coercion will be applied).
pub fn transpile_typed(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: InferredType,
) -> Result<String, Refusal> {
    transpile_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Hint(expected),
        Position::Operand,
    )
}

/// [`transpile_typed`] for a value that lands in a place DECLARED to hold
/// `slot` — an output, a local, a returned value, a condition, a parameter.
/// A value of another kind is refused before any backend emits
/// ([`slot_admits`]), so every backend refuses the same documents; `slot` is
/// also what the emitters coerce toward.
pub fn transpile_into(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    slot: InferredType,
) -> Result<String, Refusal> {
    transpile_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Slot(slot),
        Position::Operand,
    )
}

/// [`transpile_typed`] or [`transpile_into`], as `expected` says — for a
/// caller that forwards an expectation it was handed.
pub fn transpile_expecting(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: Expected,
) -> Result<String, Refusal> {
    transpile_at(expr, target, ctx, renames, expected, Position::Operand)
}

/// What the type a caller hands the pipeline asks of the value.
///
/// ⚠ One parameter served both, and a caller could not say which it meant:
/// C11 passes `string` for a `<donedata>` param and `bytes` for a payload,
/// where every other backend passes nothing — an emission choice, not a
/// declaration. Judging every expectation would have refused on C11 alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The declared type of the place the value lands in: a value of
    /// another kind is refused, and the emitters coerce toward it.
    Slot(InferredType),
    /// Only what the emitters coerce toward. Nothing is declared there, so
    /// nothing is judged.
    Hint(InferredType),
}

impl Expected {
    /// The type the emitters coerce toward, either way.
    pub fn ty(self) -> InferredType {
        match self {
            Self::Slot(ty) | Self::Hint(ty) => ty,
        }
    }
}

/// [`transpile_typed`] for the value a function RETURNS in the type its
/// signature declares — a transform output's body. It differs in one
/// place: Rust declares a `string` result `String`, and a string inside an
/// expression is borrowed, so the value is made owned (see
/// [`emit_rust_returned_str`]). Every other target and type emits exactly
/// what [`transpile_typed`] does.
pub fn transpile_returned(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: InferredType,
) -> Result<String, Refusal> {
    transpile_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Slot(expected),
        Position::Returned,
    )
}

/// Where the emitted value goes, for the one target that cares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    /// Anywhere a borrowed value serves — an operand, an argument, a
    /// guard.
    Operand,
    /// Returned from a function, in the type its signature declares.
    Returned,
}

fn transpile_at(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: Expected,
    position: Position,
) -> Result<String, Refusal> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(ExprError::Empty { what: "expression" }.at(None));
    }
    let slot = expected;
    let expected = expected.ty();

    let mut ast = parse_to_ast(expr)?;
    // Inference must run BEFORE rename so Call callees, Idents, and Members
    // are still in their pre-rename form. The TypeCtx is keyed by the
    // user-visible names (`tempConvert`, datamodel ids, …) — once rename
    // collapses an Ident or Member into a `Raw` fragment we lose the ability
    // to look it up. Inferring first lets each leaf bind its type from the
    // context; the rename pass then changes only the syntactic form,
    // leaving each TypedExpr's `ty` slot intact (which is exactly what the
    // `Raw` arm of `infer_types` already documents).
    resolve_then_rename(&mut ast, ctx, renames, target, &[], expr)?;
    judge_value(&ast, slot, expr)?;

    // RFC c7-wildcard W-project: Go exports struct fields in PascalCase
    // (the `codec_field_id` SSOT). Inside an algorithm body every member
    // access is a read of a codec element field (the foreach item's
    // fields), so the access must use the exported spelling to bind
    // against the generated Go struct — `entry.pattern` → `entry.Pattern`.
    // Gated to the algorithm kind via `project_str_args_as_bytes_view`
    // (the same flag that enables bytes-view projection); statechart guard
    // paths, whose internal `_event.data` payload struct is unexported by
    // design, never set it and keep their verbatim leaf-property emit.
    if matches!(target, ExprTarget::Go) && ctx.project_str_args_as_bytes_view {
        export_go_member_properties(&mut ast);
    }

    // The Rust and Go emitters refuse at a node; the others refuse nothing of
    // their own, so a refusal they pass on carries no range.
    Ok(match target {
        ExprTarget::Cpp => emit_cpp(&ast, expected)?,
        ExprTarget::Kotlin => emit_kotlin(&ast, expected)?,
        ExprTarget::Rust if position == Position::Returned && expected == InferredType::Str => {
            emit_rust_returned_str(&ast)?
        }
        ExprTarget::Rust => emit_rust(&ast, expected)?,
        ExprTarget::Go => emit_go(&ast, expected)?,
        ExprTarget::Python => emit_python(&ast, expected)?,
        ExprTarget::C => emit_c(&ast, expected)?,
    })
}

/// Infer the static [`InferredType`] of an expression without emitting any
/// target code. The algorithm kind's `<sce:append target expr>` lowering
/// dispatches on this: a `bytes` value extends the buffer, any other
/// (integer) value pushes a single byte (SCE byte-buffer-build,
/// SCE_FORGE.md §4.12). Runs the same tokenize → parse → [`infer_types`]
/// pipeline as [`transpile_typed`] but stops at the typed-AST root instead
/// of emitting, so the dispatch sees exactly the type the emitter would.
pub fn infer_expr_type(expr: &str, ctx: &TypeCtx<'_>) -> Result<InferredType, Refusal> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(ExprError::Empty { what: "expression" }.at(None));
    }
    let mut ast = parse_to_ast(expr)?;
    infer_types(&mut ast, ctx);
    Ok(ast.ty)
}

/// Per-import lowering descriptor consumed by [`transpile_typed_with_import_lowering`].
///
/// The C11 codec template emits its API as a typedef'd struct + free
/// functions taking an explicit `const struct_t *self` first argument
/// (`tools/codegen/templates/forge/c/codec.h.jinja2:42`). When a procedure
/// embeds the codec by-value, a method-style call like `frame.encode(args)`
/// must lower to `<free_fn>(<prepended_arg>, args...)` where `<free_fn>` is
/// looked up per method in [`ImportLowering::methods`]. The textual rename
/// map cannot express the prepended-argument injection on its own, so the
/// C11 transpile entry runs an AST rewrite keyed by this descriptor once the
/// tree is resolved and its calls judged, before rename and emission
/// ([`resolve_then_rename`] step 3).
///
/// **Per-method routing**: each `(method_name, free_fn)` entry maps the
/// SCXML-source method spelling (`"encode"`, `"update"`, ...) to the actual
/// C11 free-function symbol the lowering should emit. This split lets the
/// codec lowering point at a per-procedure wrapper that converts the
/// codec's writer-based encode output to `sce_forge_bytes_t` while the filter
/// lowering points directly at the kind's `<snake>_update` free function
/// (filter returns a primitive — no wrapper conversion needed). Calls
/// whose `(alias, method)` pair is not registered here fall through to the
/// normal pipeline, which surfaces them as a renamed-Member miss in the
/// rename pass — exactly the diagnostic the caller wants for typos.
#[derive(Debug, Clone)]
pub(crate) struct ImportLowering {
    /// SCXML `<sce:import as="...">` alias the procedure addresses the
    /// import by (e.g. `frame`). Matched against the bare-Ident object of
    /// every `Call{Member{Ident(alias), method}, args}` site.
    pub alias: String,
    /// Pre-rendered first-argument expression (e.g. `&_st->frame_`). The
    /// rewrite installs it as a `Raw` node at index 0 of the rewritten
    /// Call's args, so the existing emit_c walker stamps it out verbatim
    /// without further coercion. Identical for every method on a given
    /// import — the prepended `self` reference does not vary by method.
    pub prepended_arg: String,
    /// Per-method routing: `(method_name, free_fn)` pairs. The rewrite
    /// looks up the source-level method name and rewrites the Call's
    /// callee to the matching free-function symbol verbatim. Unmatched
    /// methods are left untouched — see the type-level doc for rationale.
    pub methods: Vec<(String, String)>,
}

/// Transpile a procedure expression for the C11 backend with stateful
/// import method-call lowering.
///
/// Same pipeline as [`transpile_typed`] (target = `ExprTarget::C`,
/// expected ECMAScript subset), plus one C11-specific AST rewrite once the
/// tree is resolved: every `Call{Member{Ident(alias), method}, args}` whose
/// `alias` matches a [`ImportLowering`] entry is rewritten to the
/// equivalent free-function call form documented on [`ImportLowering`].
/// Sites whose alias does not match are left untouched, falling through to
/// the normal pipeline (where they would surface as an unknown-identifier
/// rename — exactly the diagnostic the caller wants for typos in
/// `<sce:import as="...">` references).
pub(crate) fn transpile_typed_with_import_lowering(
    expr: &str,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: Expected,
    lowerings: &[ImportLowering],
) -> Result<String, Refusal> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(ExprError::Empty { what: "expression" }.at(None));
    }

    let mut ast = parse_to_ast(expr)?;
    resolve_then_rename(&mut ast, ctx, renames, ExprTarget::C, lowerings, expr)?;
    judge_value(&ast, expected, expr)?;
    Ok(emit_c(&ast, expected.ty())?)
}

/// The passes between parsing and emission that every typed transpile
/// runs, in the one order that is correct — ONE copy.
///
/// ⚠ There were two, one per entry point, and they had already drifted:
/// the name checks and the enum lowering went into [`transpile_typed`] and
/// not into [`transpile_typed_with_import_lowering`], so a C11 procedure
/// with a stateful import would have skipped both — found while adding
/// them, 2026-09-21. The order:
///
/// 0. `previous(<field>)` lowered to its cell's parameter, FIRST, so every
///    pass after it sees an ordinary identifier the context binds.
/// 1. [`infer_types`] BEFORE anything renames, because `ctx` is keyed by
///    the names the author wrote and a renamed node is a `Raw` it can no
///    longer look up (see [`transpile_typed`]).
/// 2. The two name checks and the call checks, still before rename, for
///    the same reason and because the diagnostic must name back what the
///    author wrote.
/// 3. C11's stateful import calls lowered to free functions (`lowerings`,
///    empty everywhere else), once the calls are judged: the lowered call
///    is a `Raw` symbol that `ctx` holds under no spelling, so no check
///    could judge it afterwards.
/// 4. Enum variants lowered to this backend's spelling once the names are
///    known good; rename leaves the resulting `Raw` alone.
/// 5. Rename.
///
/// ⚠ Step 3 ran first, before inference, until 2026-09-24. Every check then
/// saw a `Raw` callee and skipped it, so a stateful import's method called
/// with the wrong number of arguments, or into a place of another kind, was
/// refused on five backends and generated on C11 — measured with a filter's
/// `update` called with two arguments.
fn resolve_then_rename(
    ast: &mut TypedExpr,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    target: ExprTarget,
    lowerings: &[ImportLowering],
    source: &str,
) -> Result<(), Refusal> {
    resolve_names(ast, ctx, source)?;
    if !lowerings.is_empty() {
        lower_stateful_import_calls(ast, lowerings);
    }
    lower_enum_variant_refs(ast, ctx, target);
    if !renames.is_empty() {
        rename_identifiers(ast, renames);
    }
    Ok(())
}

/// Steps 0–2 of [`resolve_then_rename`]: the tree typed against `ctx`, the
/// names it reads that `ctx` does not carry refused, and every call of a
/// function `ctx` registers held to its signature. Everything a lowering
/// needs to know about the expression and nothing it does to it. `source`
/// is the text the tree was parsed from, which a refusal of an argument
/// reports.
fn resolve_names(ast: &mut TypedExpr, ctx: &TypeCtx<'_>, source: &str) -> Result<(), Refusal> {
    lower_previous(ast, ctx);
    infer_types(ast, ctx);
    reject_unknown_callees(ast, ctx)?;
    reject_unknown_names(ast, ctx)?;
    reject_call_argument_mismatches(ast, ctx, source)
}

/// Whether a value of type `got` may stand where `slot` is declared.
///
/// Extended SCXML is typed and admits no implicit coercion, so a value
/// stands only in a place of its own kind: a number where a number is
/// declared, a `bool` where a `bool` is, a string where a string is. A
/// string also stands as bytes: a bytes comparison reads a string literal
/// as its bytes, and a string field is projected to a byte view. A type the
/// pipeline could not name — an operand it does not know, `null` — is not
/// judged here.
///
/// A number stands where another number type is declared when every
/// backend makes the same value of it: an integer as any integer, wrapped
/// to the declared width as fixed-width arithmetic wraps, or as a real; a
/// real as a real of either width. A real where an integer is declared is
/// refused. The backends do not agree on rounding it or on what an
/// out-of-range value becomes — C leaves that undefined, Rust saturates —
/// and `round` and `floor` say which one the document means.
///
/// ⚠ Nothing refused a real there before 2026-09-24: a real assigned to a
/// `uint16` local generated on every backend, and Rust then assigned an
/// `f32` to a `u16`, which rustc refuses.
pub(crate) fn slot_admits(slot: InferredType, got: InferredType) -> bool {
    use InferredType as T;
    match (slot, got) {
        (T::Unknown | T::Null, _) | (_, T::Unknown | T::Null) => true,
        (slot, got) if slot.is_integer_like() => got.is_integer_like(),
        (slot, got) if slot.is_numeric() => got.is_numeric(),
        (T::Bool, got) => got == T::Bool,
        (T::Str, got) => got == T::Str,
        (T::Bytes, got) => matches!(got, T::Bytes | T::Str),
        _ => true,
    }
}

/// `value`, of a kind `slot` does not admit, refused at its range — which
/// `source` is the text of, and the refusal reports as written.
fn type_mismatch(slot: InferredType, value: &TypedExpr, source: &str) -> Refusal {
    ExprError::TypeMismatch {
        expected: slot.describe(),
        got: value.ty.describe(),
        observed: value
            .span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string),
    }
    .at(value.span.clone())
}

/// The whole expression judged against what `expected` says about the place
/// it lands in: refused when a declared slot does not admit its kind
/// ([`slot_admits`] — a hint declares nothing), and wherever an integer
/// literal in it takes a type that cannot hold it
/// ([`reject_out_of_range_literals`] — a hint's type is the one the literal
/// is emitted in, so it counts).
fn judge_value(ast: &TypedExpr, expected: Expected, source: &str) -> Result<(), Refusal> {
    if let Expected::Slot(slot) = expected {
        if !slot_admits(slot, ast.ty) {
            return Err(type_mismatch(slot, ast, source));
        }
    }
    reject_out_of_range_literals(ast, expected.ty(), source)
}

/// Refuse an integer literal the type it takes cannot hold
/// ([`ExprError::LiteralOutOfRange`]). `context` is the type the value lands
/// in. A literal takes the type the emitters give it: its partner's in a
/// binary operation (the context's when both are literals), its parameter's
/// as an argument, the place's otherwise — which is the type Rust and Go
/// infer for it, and refuse it in.
fn reject_out_of_range_literals(
    ast: &TypedExpr,
    context: InferredType,
    source: &str,
) -> Result<(), Refusal> {
    match &ast.kind {
        ExprKind::NumberLit(text) if !is_float_literal_text(text) => {
            literal_fits(ast, text, false, context, source)
        }
        // The minus belongs to the literal: `-128` fits an `int8`.
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match &operand.kind {
            ExprKind::NumberLit(text) if !is_float_literal_text(text) => {
                literal_fits(ast, text, true, context, source)
            }
            _ => reject_out_of_range_literals(operand, context, source),
        },
        ExprKind::Unary { operand, .. } => reject_out_of_range_literals(operand, context, source),
        ExprKind::Binary { op, left, right } => {
            let operand = match binary_operand_type(*op, left.ty, right.ty) {
                InferredType::UntypedInt if op.is_arith() || op.is_bitwise() => context,
                joined => joined,
            };
            reject_out_of_range_literals(left, operand, source)?;
            reject_out_of_range_literals(right, operand, source)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            reject_out_of_range_literals(condition, InferredType::Bool, source)?;
            reject_out_of_range_literals(consequent, context, source)?;
            reject_out_of_range_literals(alternate, context, source)
        }
        ExprKind::Call { args, params, .. } => {
            for (i, arg) in args.iter().enumerate() {
                reject_out_of_range_literals(arg, argument_type(params, i), source)?;
            }
            Ok(())
        }
        ExprKind::Index { object, index } => {
            reject_out_of_range_literals(object, InferredType::Unknown, source)?;
            reject_out_of_range_literals(index, InferredType::Unknown, source)
        }
        ExprKind::Member { object, .. } => {
            reject_out_of_range_literals(object, InferredType::Unknown, source)
        }
        ExprKind::BytesView { source: of, len } => {
            reject_out_of_range_literals(of, InferredType::Unknown, source)?;
            match len {
                Some(len) => reject_out_of_range_literals(len, InferredType::Unknown, source),
                None => Ok(()),
            }
        }
        ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit
        | ExprKind::Ident(_)
        | ExprKind::Raw(_) => Ok(()),
    }
}

/// `node`, the integer literal `text` (negated when `negated`), refused when
/// `context` is an integer type that cannot hold its value.
fn literal_fits(
    node: &TypedExpr,
    text: &str,
    negated: bool,
    context: InferredType,
    source: &str,
) -> Result<(), Refusal> {
    let InferredType::Int { signed, bits } = context.strip_quantity() else {
        return Ok(());
    };
    let (min, max): (i128, i128) = if signed {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    };
    // `None` is a literal wider than any backend's integer, which no
    // declared type holds.
    let fits = integer_literal_value(text).is_some_and(|magnitude| {
        let value = i128::from(magnitude);
        (min..=max).contains(&if negated { -value } else { value })
    });
    if fits {
        return Ok(());
    }
    Err(ExprError::LiteralOutOfRange {
        literal: node
            .span
            .clone()
            .and_then(|span| source.get(span))
            .unwrap_or(text)
            .to_string(),
        ty: context.strip_quantity().describe(),
        min: min.to_string(),
        max: max.to_string(),
    }
    .at(node.span.clone()))
}

/// The value of an integer literal as the lexer spells it — decimal, or
/// `0x`/`0b`/`0o` prefixed — or `None` past `u64`, the widest integer any
/// backend declares.
fn integer_literal_value(text: &str) -> Option<u64> {
    let (digits, radix) = match text.get(..2) {
        Some("0x" | "0X") => (&text[2..], 16),
        Some("0b" | "0B") => (&text[2..], 2),
        Some("0o" | "0O") => (&text[2..], 8),
        _ => (text, 10),
    };
    u64::from_str_radix(digits, radix).ok()
}

/// An integer literal no bare spelling carries into C, C++ or Kotlin: its
/// size passes `i64::MAX`. [`reject_out_of_range_literals`] has held every
/// literal to its type, so there are exactly two — a `uint64` value above
/// `i64::MAX`, and `int64`'s minimum, the negation of `2^63`.
///
/// ⚠ Both were emitted bare until 2026-09-24: gcc and g++ refuse a decimal
/// constant that large under `-Werror` ("so large that it is unsigned"),
/// and kotlinc refuses it outright ("value out of range").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WideLiteral {
    Unsigned,
    Int64Min,
}

/// `node` as a [`WideLiteral`], when it is one.
fn wide_literal(node: &TypedExpr) -> Option<WideLiteral> {
    const LONG_MAX: u64 = i64::MAX.unsigned_abs();
    let magnitude = |kind: &ExprKind| match kind {
        ExprKind::NumberLit(text) if !is_float_literal_text(text) => integer_literal_value(text),
        _ => None,
    };
    match &node.kind {
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => (magnitude(&operand.kind) == Some(LONG_MAX + 1)).then_some(WideLiteral::Int64Min),
        kind => magnitude(kind)
            .filter(|value| *value > LONG_MAX)
            .map(|_| WideLiteral::Unsigned),
    }
}

/// `INT64_MIN` in C and C++. `-9223372036854775808LL` negates a constant
/// `long long` cannot hold, so the value is spelled as an expression.
pub(crate) const C_INT64_MIN: &str = "(-9223372036854775807LL - 1)";

/// `node`, of integer type `to`, as C and C++ spell it when it is a
/// [`WideLiteral`]; `None` for anything else, which stands as emitted.
fn c_family_wide_literal(raw: &str, to: InferredType, node: &TypedExpr) -> Option<String> {
    if !matches!(to.strip_quantity(), InferredType::Int { .. }) {
        return None;
    }
    Some(match wide_literal(node)? {
        WideLiteral::Unsigned => format!("{raw}ULL"),
        WideLiteral::Int64Min => C_INT64_MIN.to_string(),
    })
}

/// Refuse a call of a function `ctx` registers — an imported function, a
/// `<sce:helper>`, a stateful import's method — whose arguments do not fit
/// its signature: more or fewer than it takes, refused at the callee, or
/// one whose kind its parameter does not admit ([`slot_admits`]), refused
/// at that argument. A callee the context does not register — a builtin, a
/// `Raw` symbol this pipeline lowered a call to — is not judged here.
///
/// ⚠ Nothing compared a call with its signature before 2026-09-24: an
/// imported two-parameter algorithm called with one argument, or with its
/// arguments' kinds swapped, generated on every backend and was refused
/// only by the target language's compiler.
fn reject_call_argument_mismatches(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        // The same two spellings `infer_types` resolves a signature by: a
        // bare name, and `obj.method` registered as `"{obj}.{method}"`.
        let registered = match &callee.kind {
            ExprKind::Ident(name) => ctx.lookup_func(name).map(|sig| (name.clone(), sig)),
            ExprKind::Member { object, property } => match &object.kind {
                ExprKind::Ident(object) => {
                    let name = format!("{object}.{property}");
                    ctx.lookup_func(&name).map(|sig| (name, sig))
                }
                _ => None,
            },
            _ => None,
        };
        if let Some((name, signature)) = registered {
            if args.len() != signature.params.len() {
                return Err(ExprError::ArgumentCount {
                    callee: name,
                    expected: signature.params.len(),
                    actual: args.len(),
                }
                .at(callee.span.clone()));
            }
            if let Some((param, arg)) = signature
                .params
                .iter()
                .zip(args)
                .find(|(param, arg)| !slot_admits(**param, arg.ty))
            {
                return Err(type_mismatch(*param, arg, source));
            }
        }
    }
    for child in expr_children(expr) {
        reject_call_argument_mismatches(child, ctx, source)?;
    }
    Ok(())
}

/// `expr` parsed and resolved against `ctx` — typed, and refused for a name
/// `ctx` does not carry — without lowering it to any backend.
///
/// For a check that decides on the expression's TYPE, which is only a
/// question once its names are known: an undeclared name infers as no
/// type at all, and a check that read that as the answer would report the
/// type while the name is what is wrong.
///
/// ⚠ That is what happened. `validate::address_form` parsed and inferred
/// on its own, so `sce:addr="ecuAdr"` beside `ecuAddr` was refused as an
/// address "of no type this document establishes" — no mention of the
/// name, and no near miss offered (measured 2026-09-23).
pub(crate) fn resolve(expr: &str, ctx: &TypeCtx<'_>) -> Result<TypedExpr, Refusal> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(ExprError::Empty { what: "expression" }.at(None));
    }
    let mut ast = parse_to_ast(expr)?;
    resolve_names(&mut ast, ctx, expr)?;
    Ok(ast)
}

/// Walk a TypedExpr in place and rewrite every `Call{Member{Ident(alias),
/// method}, args}` site whose `alias` matches one of `lowerings` into a
/// free-function form per the [`ImportLowering`] contract.
///
/// Member nodes that are NOT inside a Call (e.g. `frame.msgId` on the LHS
/// of `<assign location="...">`) are left untouched — those flow through
/// the standard rename map's qualified-key collapse path
/// (`stateful_import_field_renames` for C11 emits `_st->{member}.{field}`),
/// which the rename pass handles after this rewrite returns. Splitting the
/// two cases at AST shape — Call vs. bare Member — keeps each pass's
/// responsibility one-thing-only.
///
/// Runs on a RESOLVED tree ([`resolve_then_rename`] step 3): the call keeps
/// the type and the parameter types inference gave the method it names,
/// and the prepended state argument takes no parameter type of its own.
fn lower_stateful_import_calls(ast: &mut TypedExpr, lowerings: &[ImportLowering]) {
    match &mut ast.kind {
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            // Try the lowering match first so a successful rewrite does not
            // double-walk the (now-replaced) callee through the recursive
            // arms below.
            if let ExprKind::Member { object, property } = &callee.kind {
                if let ExprKind::Ident(alias) = &object.kind {
                    if let Some(lowering) = lowerings.iter().find(|l| l.alias == *alias) {
                        if let Some((_, free_fn)) =
                            lowering.methods.iter().find(|(name, _)| name == property)
                        {
                            let prepended =
                                TypedExpr::new(ExprKind::Raw(lowering.prepended_arg.clone()));
                            let new_callee =
                                Box::new(TypedExpr::new(ExprKind::Raw(free_fn.clone())));
                            let mut new_args = Vec::with_capacity(args.len() + 1);
                            new_args.push(prepended);
                            new_args.append(args);
                            // Original args may contain nested Call{Member{...}}
                            // sites (e.g. one codec.encode() passed into another
                            // codec's method). Recurse over the rewritten args
                            // before returning so every inner site lowers too.
                            for arg in new_args.iter_mut().skip(1) {
                                lower_stateful_import_calls(arg, lowerings);
                            }
                            let mut new_params = Vec::with_capacity(params.len() + 1);
                            new_params.push(InferredType::Unknown);
                            new_params.append(params);
                            ast.kind = ExprKind::Call {
                                callee: new_callee,
                                args: new_args,
                                params: new_params,
                            };
                            return;
                        }
                    }
                }
            }
            // Fall-through: not a stateful-import call site, recurse normally.
            lower_stateful_import_calls(callee, lowerings);
            for arg in args {
                lower_stateful_import_calls(arg, lowerings);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            lower_stateful_import_calls(left, lowerings);
            lower_stateful_import_calls(right, lowerings);
        }
        ExprKind::Unary { operand, .. } => {
            lower_stateful_import_calls(operand, lowerings);
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            lower_stateful_import_calls(condition, lowerings);
            lower_stateful_import_calls(consequent, lowerings);
            lower_stateful_import_calls(alternate, lowerings);
        }
        ExprKind::Member { object, .. } => {
            lower_stateful_import_calls(object, lowerings);
        }
        ExprKind::Index { object, index } => {
            lower_stateful_import_calls(object, lowerings);
            lower_stateful_import_calls(index, lowerings);
        }
        ExprKind::BytesView { source, len } => {
            lower_stateful_import_calls(source, lowerings);
            if let Some(len) = len {
                lower_stateful_import_calls(len, lowerings);
            }
        }
        ExprKind::Raw(_)
        | ExprKind::Ident(_)
        | ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit => {}
    }
}

/// Transpile an assignment *left-hand side* (an SCXML `<assign location="…"/>`
/// attribute) to the target language, using the same pipeline as
/// [`transpile_typed`] for right-hand-side expressions.
///
/// The point of this function is symmetry: LHS and RHS share a single
/// `tokenize → parse → infer → rename → emit` path. A previous implementation
/// fed `location` through ad-hoc per-language string surgery (snake-casing,
/// prepending `self.`, appending `_`, …) which drifted out of step with the
/// RHS emitter the moment grammar extended beyond bare identifiers. This
/// helper forces every future grammar extension — dotted codec field access,
/// indexed writes, whatever — to flow through one pipeline.
///
/// Shape is restricted to the subset permitted as an SCXML assign location:
///
/// * `Ident(name)` — bare datamodel variable
/// * `Member { object: Ident(alias), property: field }` — one level of
///   member access, typically a stateful imported codec's field
///
/// Everything else (`Call`, `Index`, `Binary`, nested `Member`, literals, …)
/// is rejected with a descriptive error — writing to a function result or a
/// computed index makes no sense as an lvalue and we do not want to silently
/// emit broken code.
///
/// Returns `(emitted, inferred_type)`. The inferred type flows back to the
/// caller so it can drive the RHS's `expected` parameter (enabling type-aware
/// coercion around the assignment) and make type-dependent post-processing
/// decisions such as bytes-wrapping.
pub fn transpile_lvalue(
    location: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
) -> Result<(String, InferredType), Refusal> {
    let trimmed = location.trim();
    if trimmed.is_empty() {
        return Err(ExprError::Empty {
            what: "assign location",
        }
        .at(None));
    }

    let mut ast = parse_to_ast(trimmed)?;
    validate_lvalue_shape(&ast.kind, trimmed).map_err(|refusal| refusal.at(ast.span.clone()))?;

    // Same ordering as `transpile_typed`: infer first so Ident/Member leaves
    // can still bind their type from the user-visible name before rename
    // collapses them into `Raw` fragments.
    infer_types(&mut ast, ctx);
    let ty = ast.ty;
    if !renames.is_empty() {
        rename_identifiers(&mut ast, renames);
    }

    // LHS has no top-down coercion context — it is a storage location, not a
    // value being shoved into an expected type. Pass `Unknown` so the
    // per-language emitter emits the identifier/member path verbatim.
    let emitted = match target {
        ExprTarget::Cpp => emit_cpp(&ast, InferredType::Unknown)?,
        ExprTarget::Kotlin => emit_kotlin(&ast, InferredType::Unknown)?,
        ExprTarget::Rust => emit_rust(&ast, InferredType::Unknown)?,
        ExprTarget::Go => emit_go(&ast, InferredType::Unknown)?,
        ExprTarget::Python => emit_python(&ast, InferredType::Unknown)?,
        ExprTarget::C => emit_c(&ast, InferredType::Unknown)?,
    };
    Ok((emitted, ty))
}

/// Parse an expression string into the typed AST without running
/// type inference or any rename pass.
///
/// Used by [`crate::forge::const_fold`] — the RFC §synth-5-F build-time
/// host interpreter walks the raw AST shape and evaluates leaves
/// against typed `ConstValue` storage. The inference layer is
/// irrelevant there because the evaluator carries `SceType`
/// annotations on every storage site (Var/Assign/yield) instead.
pub(crate) fn parse_to_ast(expr: &str) -> Result<TypedExpr, Refusal> {
    if expr.trim().is_empty() {
        return Err(ExprError::Empty { what: "expression" }.at(None));
    }
    let (tokens, spans) = admitted_tokens(expr)?;
    Parser::new(&tokens, &spans).parse_expression()
}

/// Where each argument of `list` is written, for `list` a call's argument
/// list without its parentheses — what `<sce:call args>` holds.
///
/// SCE Forge: the arguments of `<sce:call>` are read by the rule that reads
/// a call's arguments between parentheses, so a comma inside a nested call
/// or a string literal separates nothing, and an empty argument (`a,,b`, a
/// comma leading or trailing the list) is refused at the comma that leaves
/// it empty. A list that is empty, or only whitespace, holds no arguments.
///
/// The ranges index `list` as the caller holds it, as [`parse_to_ast`]'s do.
pub(crate) fn argument_ranges(list: &str) -> Result<Vec<std::ops::Range<usize>>, Refusal> {
    let (tokens, spans) = admitted_tokens(list)?;
    let arguments = Parser::new(&tokens, &spans).parse_argument_list()?;
    Ok(arguments
        .into_iter()
        .map(|argument| {
            argument
                .span
                .expect("the parser spans every node it builds")
        })
        .collect())
}

/// `text`'s tokens once the Forge admission rules have passed them, with
/// their ranges in `text` as the caller holds it.
fn admitted_tokens(text: &str) -> Result<SpannedTokens, Refusal> {
    // Ranges into `text` as the caller holds it, not into its trimmed copy —
    // the tokens' and a refusal's alike, so every range this returns reads
    // against the one string.
    let lead = text.len() - text.trim_start().len();
    let shift = |span: std::ops::Range<usize>| span.start + lead..span.end + lead;
    let (tokens, mut spans) =
        tokenize_spanned(text.trim(), LexMode::Forge).map_err(|refusal| Spanned {
            span: refusal.span.map(shift),
            error: refusal.error,
        })?;
    for span in &mut spans {
        *span = shift(span.clone());
    }
    reject_policy_violating_tokens(&tokens, &spans)?;
    Ok((tokens, spans))
}

/// The Forge expression language's admission rules, as a single pass over
/// a token stream — the one place that decides what Extended SCXML
/// forbids, independent of where the tokens came from.
///
/// SCE Forge §3.4: Extended SCXML is typed and admits no implicit
/// coercion, so loose equality — the operator whose ECMAScript meaning
/// *is* coercion — has no interpretation, and the constructs outside the
/// typed subset have none either.
///
/// This runs *after* tokenizing and *before* parsing, so it fires on a
/// violating token in any position (`== 1` rejects as loose equality, not
/// as an unexpected token) and the parser below it never has to know the
/// rule. Every caller of [`parse_to_ast`] therefore keeps the exact
/// diagnostics it had when the tokenizer raised them itself — same
/// `ExprError`, same code, same fix.
///
/// `spans` are the tokens' ranges, one per token, and the refusal is raised
/// at the token that violates the rule.
fn reject_policy_violating_tokens(
    tokens: &[Token],
    spans: &[std::ops::Range<usize>],
) -> Result<(), Refusal> {
    for (token, span) in tokens.iter().zip(spans) {
        let refusal = match token {
            Token::LooseEq => ExprError::StrictEquality {
                operator: "==",
                strict: "===",
            },
            Token::LooseNeq => ExprError::StrictEquality {
                operator: "!=",
                strict: "!==",
            },
            Token::Unsupported {
                construct,
                spelling,
            } => ExprError::UnsupportedConstruct {
                construct: (*construct).to_string(),
                observed: Some((*spelling).to_string()),
            },
            _ => continue,
        };
        return Err(refusal.at(Some(span.clone())));
    }
    Ok(())
}

/// `true` iff `expr` reads the W3C reserved `_event.data` path — decided
/// on the token stream, so it holds for expressions the Forge *parser*
/// rejects.
///
/// This exists for exactly one question: when a `cond` on a
/// schema-carrying transition fails to parse as a Forge expression, was
/// the author reaching for typed event data (making the failure a
/// rejection) or writing plain ECMAScript for the script engine (making
/// it none of Forge's business)? An AST cannot answer it — the parse is
/// what failed.
///
/// It is the lexical shadow of the AST-level
/// [`crate::forge::event_schema_check::cond_references_event_data`], and
/// the two cannot disagree on any expression that parses: a
/// `_event.data` member access in the AST implies these tokens, and these
/// tokens in a parseable expression imply that access. Matching on tokens
/// rather than text is what makes it whitespace-insensitive
/// (`_event . data . flag`) and string-literal-safe (a literal is a single
/// token, so `x === '_event.data'` is not a hit) — the two ways a raw
/// substring search gets the answer wrong.
///
/// An expression that does not even tokenize (an unknown character, an
/// unterminated string, a reserved keyword like `function`) is not
/// lexically ECMAScript and is not a guard; it reads as `false`, keeping
/// Forge out of expressions it has no claim on.
pub(crate) fn references_event_data_lexically(expr: &str) -> bool {
    let Ok(tokens) = tokenize(expr.trim()) else {
        return false;
    };
    tokens.windows(3).any(|w| {
        matches!(&w[0], Token::Ident(s) if s == "_event")
            && w[1] == Token::Dot
            && matches!(&w[2], Token::Ident(s) if s == "data")
    })
}

/// Validate that a parsed expression's shape is a legal assignment target.
/// See [`transpile_lvalue`] for the full rule set.
fn validate_lvalue_shape(kind: &ExprKind, location: &str) -> Result<(), ExprError> {
    let detail = match kind {
        ExprKind::Ident(_) => return Ok(()),
        ExprKind::Member { object, .. } => match &object.kind {
            ExprKind::Ident(_) => return Ok(()),
            other => format!(
                "member access object must be a bare identifier, got: {}",
                shape_name(other),
            ),
        },
        other => format!(
            "must be an identifier or one-level member access, got: {}",
            shape_name(other),
        ),
    };
    Err(ExprError::InvalidLvalue {
        location: location.to_string(),
        detail,
    })
}

/// Human-readable shape name for diagnostic messages.
fn shape_name(kind: &ExprKind) -> &'static str {
    match kind {
        ExprKind::NumberLit(_) => "number literal",
        ExprKind::StringLit { .. } => "string literal",
        ExprKind::BytesLit { .. } => "bytes literal",
        ExprKind::BoolLit(_) => "boolean literal",
        ExprKind::NullLit => "null literal",
        ExprKind::Ident(_) => "identifier",
        ExprKind::Raw(_) => "pre-rendered fragment",
        ExprKind::Binary { .. } => "binary operation",
        ExprKind::Unary { .. } => "unary operation",
        ExprKind::Conditional { .. } => "conditional",
        ExprKind::Member { .. } => "member access",
        ExprKind::Index { .. } => "index expression",
        ExprKind::Call { .. } => "call expression",
        ExprKind::BytesView { .. } => "bytes-view projection",
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Typed AST
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A node in the expression AST, paired with its inferred type.
///
/// Every node carries exactly one `ty` slot. The parser initializes
/// `ty = InferredType::Unknown`, and [`infer_types`] overwrites it in a
/// bottom-up pass. Emitters then consume the annotated tree.
///
/// `span` is the byte range of the text the node was parsed from, in the
/// string given to [`parse_to_ast`] — so a refusal can report what the
/// author wrote (SCE_ERROR_CONTRACT §3.1.1) rather than a description or
/// a re-rendering of it. `None` for a node a rewrite synthesised, which
/// no source text spells.
///
/// ⚠ That string is an attribute value as the XML reader DECODED it, not
/// as the document spells it: `&lt;` is `<` there, and a value continued
/// over several rows is one row. A range of it is put back where the
/// author wrote it by [`crate::attribute_spelling::AttributeSpelling`].
#[derive(Debug, Clone)]
pub(crate) struct TypedExpr {
    pub kind: ExprKind,
    pub ty: InferredType,
    pub span: Option<std::ops::Range<usize>>,
}

/// Structural equality: two trees are the same expression whatever text
/// they were read from, so `span` — where, not what — takes no part.
impl PartialEq for TypedExpr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.ty == other.ty
    }
}

impl TypedExpr {
    /// A node no source text spells.
    fn new(kind: ExprKind) -> Self {
        Self {
            kind,
            ty: InferredType::Unknown,
            span: None,
        }
    }
}

/// The structural content of an expression node. Does not carry type
/// information on its own — that lives in the enclosing [`TypedExpr`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprKind {
    NumberLit(String),
    StringLit {
        value: String,
        quote: char,
    },
    /// A byte-sequence literal carrying its fully decoded bytes. Not
    /// produced by the parser — [`infer_types`] rewrites a `StringLit`
    /// into a `BytesLit` when it is compared against a `Bytes`-typed
    /// operand (e.g. `_event.data.raw === 'ack'`). The decode-once-here
    /// design keeps the "this string is bytes" decision and its encoding
    /// (UTF-8) in a single place, so every backend renders byte-identical
    /// comparison constants instead of re-deriving the reinterpretation
    /// per emitter.
    BytesLit {
        bytes: Vec<u8>,
    },
    BoolLit(bool),
    NullLit,
    /// A lexer-produced bare identifier. Represents a name the user wrote in
    /// SCXML source; per-language emitters apply case conversion
    /// (`to_snake_case`, `to_pascal_case`, etc.) to produce the target-language
    /// spelling.
    Ident(String),
    /// A pre-resolved, target-language-native expression fragment. Produced by
    /// [`rename_identifiers`] when it collapses a Member node or substitutes an
    /// alias with its fully-formatted call string (e.g. `self.ecu_addr`,
    /// `transform_temperature::compute_temperature`). Every emitter emits the
    /// string **verbatim** — no case conversion, no parenthesisation, no
    /// qualification. This variant exists to keep the type-level distinction
    /// "needs formatting" vs "already formatted" explicit; the earlier approach
    /// of stuffing both into `Ident(String)` forced emitters to guess via
    /// string-content heuristics, which broke as soon as the rename format
    /// diverged from the lexer's grammar for bare identifiers.
    Raw(String),
    Binary {
        op: BinOp,
        left: Box<TypedExpr>,
        right: Box<TypedExpr>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<TypedExpr>,
    },
    Conditional {
        condition: Box<TypedExpr>,
        consequent: Box<TypedExpr>,
        alternate: Box<TypedExpr>,
    },
    Member {
        object: Box<TypedExpr>,
        property: String,
    },
    Index {
        object: Box<TypedExpr>,
        index: Box<TypedExpr>,
    },
    Call {
        callee: Box<TypedExpr>,
        args: Vec<TypedExpr>,
        /// The parameter types of the function the call resolved to, one per
        /// argument in position order — what each argument converts to, the
        /// way a value converts to the place it lands in. Empty until
        /// [`infer_types`] resolves the callee, and for a callee no context
        /// registers (a builtin, a symbol this pipeline lowered a call to),
        /// whose arguments are emitted as they are.
        params: Vec<InferredType>,
    },
    /// A borrowed `bytes`-view projection of a `Str`-typed source. Not
    /// produced by the parser — [`infer_types`] wraps a call argument in
    /// this node when the argument is `Str`-typed and the resolved callee
    /// parameter is `Bytes` (and only when the TypeCtx enables
    /// `project_str_args_as_bytes_view` — the algorithm kind). It lowers a
    /// bounded-string element field (`entry.pattern`) to each backend's
    /// borrowed byte-view idiom at the call site: Rust `.as_bytes()`, C11
    /// a `sce_forge_bytes_view_t` over `{src, src_len}`, Cpp a
    /// `std::span<const std::uint8_t>` over the string's data/size, Go
    /// `[]byte(src)`, Python `src.encode("utf-8")`, Kotlin
    /// `src.toByteArray(Charsets.UTF_8)`. The decode-once-here design
    /// keeps the "this Str is passed as bytes" decision in a single place
    /// (`infer_types`) so emitters only render the resulting node — the
    /// same shape as the [`BytesLit`] string→bytes literal rewrite.
    ///
    /// [`BytesLit`]: ExprKind::BytesLit
    BytesView {
        source: Box<TypedExpr>,
        /// The C11 length sibling expression (`entry.pattern_len`), resolved
        /// at projection time from the codec's `length_field` SSOT — never a
        /// `<field>_len` guess. Only the C11 emit
        /// consumes it (a `char[N]` array carries no inherent length); every
        /// other backend reads the length from the string type itself
        /// (`.as_bytes()` / `.size()` / `len(...)`). `None` when the source
        /// is not a recognised element-field member — the C11 emit then
        /// falls back to the `<src>_len` sibling convention.
        len: Option<Box<TypedExpr>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    StrictEq,
    StrictNeq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    UShr,
}

impl BinOp {
    fn is_arith(self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::Mul | Self::Div | Self::Mod
        )
    }
    fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::StrictEq | Self::StrictNeq | Self::Lt | Self::Gt | Self::LtEq | Self::GtEq
        )
    }
    fn is_logical(self) -> bool {
        matches!(self, Self::And | Self::Or)
    }
    fn is_bitwise(self) -> bool {
        matches!(
            self,
            Self::BitAnd | Self::BitOr | Self::BitXor | Self::Shl | Self::Shr | Self::UShr
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Neg,
    Pos,
    Not,
    BitNot,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Public helper: string-literal stripping (used by generator-side
// tooling that pre-scans SCXML expressions without full parsing)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Extract unique free identifier names from a raw ECMAScript expression.
/// Used by inline kind rendering to build member-access renames for languages
/// that require explicit `self.` / `p.` prefixes (Rust, Go).
pub fn extract_free_idents(raw_expr: &str) -> Result<Vec<String>, Refusal> {
    let (tokens, spans) = tokenize_spanned(raw_expr.trim(), LexMode::Forge)?;
    // Extended SCXML expression, so the same admission rules apply. The
    // tokenizer no longer enforces them, so every entry point that treats
    // its input as Forge source states so explicitly.
    reject_policy_violating_tokens(&tokens, &spans)?;
    let mut idents = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for token in &tokens {
        if let Token::Ident(s) = token {
            // Skip boolean/null literals — they are parsed as Ident by the tokenizer
            // but are language keywords, not member variable references.
            if s != "true" && s != "false" && s != "null" && seen.insert(s.clone()) {
                idents.push(s.clone());
            }
        }
    }
    Ok(idents)
}

/// The names `raw_expr` reads as values — its identifier nodes, in the
/// order first read, each once. Unlike [`extract_free_idents`], which works
/// on tokens and so also yields the member after a `.`, this reads the
/// parsed tree: in `frame.len` it answers `frame`, never `len`.
pub fn read_identifiers(raw_expr: &str) -> Result<Vec<String>, Refusal> {
    fn walk(node: &TypedExpr, names: &mut Vec<String>) {
        match &node.kind {
            ExprKind::Ident(name) => {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
            ExprKind::Binary { left, right, .. } => {
                walk(left, names);
                walk(right, names);
            }
            ExprKind::Unary { operand, .. } => walk(operand, names),
            ExprKind::Conditional {
                condition,
                consequent,
                alternate,
            } => {
                walk(condition, names);
                walk(consequent, names);
                walk(alternate, names);
            }
            ExprKind::Member { object, .. } => walk(object, names),
            ExprKind::Index { object, index } => {
                walk(object, names);
                walk(index, names);
            }
            ExprKind::Call { callee, args, .. } => {
                walk(callee, names);
                for arg in args {
                    walk(arg, names);
                }
            }
            ExprKind::BytesView { source, len } => {
                walk(source, names);
                if let Some(len) = len {
                    walk(len, names);
                }
            }
            ExprKind::NumberLit(_)
            | ExprKind::StringLit { .. }
            | ExprKind::BytesLit { .. }
            | ExprKind::BoolLit(_)
            | ExprKind::NullLit
            | ExprKind::Raw(_) => {}
        }
    }
    let mut names = Vec::new();
    // Trimmed, as every entry point hands the parser its text, so a refusal
    // raised here indexes the same string as one raised by a lowering.
    walk(&parse_to_ast(raw_expr.trim())?, &mut names);
    Ok(names)
}

/// Replace the contents of every string literal with spaces of equal length,
/// preserving expression length and non-string token positions.
pub fn strip_string_literals(expr: &str) -> String {
    static RE_STR: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"'[^']*'|"[^"]*""#).unwrap());
    RE_STR
        .replace_all(expr, |caps: &regex::Captures| " ".repeat(caps[0].len()))
        .to_string()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tokens
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Number(String),
    String {
        value: String,
        quote: char,
    },
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StrictEq,
    StrictNeq,
    /// Loose equality `==` / `!=`. Lexically valid ECMAScript, so the
    /// tokenizer emits them; Extended SCXML forbids them, and that is a
    /// *policy* decision made by [`reject_policy_violating_tokens`], not a
    /// lexical one.
    ///
    /// The distinction is load-bearing. Whether `==` is permitted depends
    /// on context the tokenizer does not have: it is illegal in an
    /// Extended SCXML typed expression and perfectly legal in a plain
    /// ECMAScript `cond` bound for the script engine. A tokenizer that
    /// rejected it outright could not produce a token stream for the
    /// second case at all — which is exactly what forced callers to answer
    /// "does this expression read typed event data?" with a substring
    /// search over the raw text.
    LooseEq,
    LooseNeq,
    /// A construct that is lexically well-formed ECMAScript but outside
    /// the Forge subset (`=>`, `??`, `?.`, `...`, a template literal).
    /// Carries its author-facing description and the spelling the author
    /// wrote; rejected by [`reject_policy_violating_tokens`] as
    /// [`ExprError::UnsupportedConstruct`], the same diagnostic the
    /// tokenizer used to raise, one layer up.
    Unsupported {
        construct: &'static str,
        spelling: &'static str,
    },
    Lt,
    Gt,
    LtEq,
    GtEq,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Shl,
    Shr,
    UShr,
    Question,
    Colon,
    Dot,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    /// Statement and object-literal punctuation, plus the assignment and
    /// update operators. Produced only under [`LexMode::EcmaScript`] — the
    /// Forge subset is a single expression with no statements, no object
    /// literals and no assignment, so under [`LexMode::Forge`] these
    /// characters stay the lexical error they have always been rather than
    /// becoming a token the Forge parser would have to reject one layer
    /// later with a worse message.
    LBrace,
    RBrace,
    Semi,
    Assign,
    PlusPlus,
    MinusMinus,
    /// Compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`), carrying the
    /// arithmetic operator it folds in.
    OpAssign(ArithOp),
    Eof,
}

/// Which language the token stream is being read as.
///
/// The two dialects SCE accepts share this lexer and nothing else: Forge's
/// Extended SCXML is a typed single expression, and the W3C ECMAScript
/// datamodel (§B.2) is untyped and admits statements. Sharing the lexer is
/// what keeps the two from disagreeing about the things that are purely
/// lexical — string escapes, numeric forms, where an identifier ends — which
/// is where the string-rewriting transformer this replaced kept its bugs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum LexMode {
    /// Extended SCXML. Reserved words that have no typed meaning (`new`,
    /// `typeof`, `function`, …) are rejected at the point they are read.
    Forge,
    /// The W3C SCXML ECMAScript datamodel. Every reserved word tokenizes as an
    /// identifier for the parser to interpret, comments are skipped, and
    /// the statement/assignment punctuation above is produced.
    EcmaScript,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Number(n) => write!(f, "{n}"),
            Token::String { value, .. } => write!(f, "'{value}'"),
            Token::Ident(s) => write!(f, "{s}"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::StrictEq => write!(f, "==="),
            Token::StrictNeq => write!(f, "!=="),
            Token::LooseEq => write!(f, "=="),
            Token::LooseNeq => write!(f, "!="),
            Token::Unsupported { construct, .. } => write!(f, "{construct}"),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::AmpAmp => write!(f, "&&"),
            Token::PipePipe => write!(f, "||"),
            Token::Amp => write!(f, "&"),
            Token::Pipe => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::Bang => write!(f, "!"),
            Token::Shl => write!(f, "<<"),
            Token::Shr => write!(f, ">>"),
            Token::UShr => write!(f, ">>>"),
            Token::Question => write!(f, "?"),
            Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Semi => write!(f, ";"),
            Token::Assign => write!(f, "="),
            Token::PlusPlus => write!(f, "++"),
            Token::MinusMinus => write!(f, "--"),
            Token::OpAssign(op) => write!(f, "{}=", op.text()),
            Token::Eof => write!(f, "EOF"),
        }
    }
}

/// The arithmetic operators a compound assignment can fold in — the only
/// ones the lexer reads before `=`.
///
/// Its own type rather than a [`BinOp`], which let `Token::OpAssign` carry
/// `Shl` or `StrictEq`: no input produced one, yet every consumer had to
/// answer for it, and the answers were a debug-formatted refusal and a
/// silent `+`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl ArithOp {
    /// The ECMAScript spelling, without the `=`.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tokenizer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn tokenize(input: &str) -> Result<Vec<Token>, ExprError> {
    tokenize_as(input, LexMode::Forge)
}

/// The tokens of `input`, without where they were read from — and so a
/// refusal without where it was raised either, since its caller holds no
/// range to read one against.
pub(crate) fn tokenize_as(input: &str, mode: LexMode) -> Result<Vec<Token>, ExprError> {
    tokenize_spanned(input, mode)
        .map(|(tokens, _)| tokens)
        .map_err(|refusal| refusal.error)
}

/// A token stream with the byte range each token was read from: one range
/// per token, `Eof` taking the empty range at the end of `input`.
type SpannedTokens = (Vec<Token>, Vec<std::ops::Range<usize>>);

/// [`tokenize_as`], keeping where each token was read from — and where a
/// refusal was raised, in the same byte ranges.
fn tokenize_spanned(input: &str, mode: LexMode) -> Result<SpannedTokens, Refusal> {
    let mut tokens = Vec::new();
    let mut spans = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let ecma = mode == LexMode::EcmaScript;
    let mut token_start = 0;

    while i < len {
        // Every pass below reads at most one token, so the token the last
        // pass read — if it read one — ends where this pass begins.
        close_span(&tokens, &mut spans, token_start..i);
        token_start = i;
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Comments. Only ECMAScript source carries them: a Forge expression
        // is one expression written in an XML attribute, and treating `//`
        // there as a comment would silently swallow the rest of a `cond`
        // instead of reporting the division-by-division the author wrote.
        if ecma && bytes[i] == b'/' && i + 1 < len {
            if bytes[i + 1] == b'/' {
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if bytes[i + 1] == b'*' {
                let close = input[i + 2..].find("*/");
                i = match close {
                    Some(offset) => i + 2 + offset + 2,
                    None => len,
                };
                continue;
            }
        }

        // String literals
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i] as char;
            i += 1;
            let start = i;
            while i < len && bytes[i] != quote as u8 {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i >= len {
                // Raised at the quote that opened it: the literal has no end
                // to point at.
                return Err(ExprError::Lex {
                    position: start,
                    detail: "unterminated string literal".to_string(),
                }
                .at(Some(start - 1..start)));
            }
            let value = input[start..i].to_string();
            i += 1;
            tokens.push(Token::String { value, quote });
            continue;
        }

        // Numbers: decimal, hex, binary, octal, float
        if bytes[i].is_ascii_digit()
            || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
        {
            let start = i;
            let starts_with_dot = bytes[i] == b'.';
            if bytes[i] == b'0' && i + 1 < len {
                match bytes[i + 1] {
                    b'x' | b'X' => {
                        i += 2;
                        while i < len && bytes[i].is_ascii_hexdigit() {
                            i += 1;
                        }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    b'b' | b'B' => {
                        i += 2;
                        while i < len && (bytes[i] == b'0' || bytes[i] == b'1') {
                            i += 1;
                        }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    b'o' | b'O' => {
                        i += 2;
                        while i < len && bytes[i] >= b'0' && bytes[i] <= b'7' {
                            i += 1;
                        }
                        tokens.push(Token::Number(input[start..i].to_string()));
                        continue;
                    }
                    _ => {}
                }
            }
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let mut num = input[start..i].to_string();
            // Normalize `.5` -> `0.5` (not valid in Rust/Kotlin)
            if starts_with_dot {
                num.insert(0, '0');
            }
            tokens.push(Token::Number(num));
            continue;
        }

        // Identifiers and keywords
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'$' {
            let start = i;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }
            let word = &input[start..i];
            if !ecma {
                validate_keyword(word).map_err(|refusal| refusal.at(Some(start..i)))?;
            }
            tokens.push(Token::Ident(word.to_string()));
            continue;
        }

        // Constructs outside the Forge subset. They are lexically
        // well-formed ECMAScript, so they tokenize; the rejection is a
        // policy decision and belongs to `reject_policy_violating_tokens`,
        // which raises the identical `UnsupportedConstruct` diagnostic.
        // Tokenizing them keeps the token stream total over anything an
        // author could plausibly write in a guard, which is what lets a
        // caller ask "does this expression read typed event data?" of an
        // expression the Forge *parser* will go on to reject.
        //
        // Each multi-byte needle below is matched against `bytes`, never
        // against a `&input[i..i + n]` slice. `i` is always on a character
        // boundary, but `i + n` need not be: one non-ASCII character in the
        // expression put the end of the slice inside it, and slicing a `&str`
        // there panics. The lexer already has the right answer for a
        // character it does not know — the `ExprError::Lex` below — and the
        // panic was reaching the author *instead of* that answer, as an
        // abort with no diagnostic at all. Byte comparison is total over any
        // input and identical for these ASCII needles, so the unknown
        // character now walks to the arm that names it.
        if bytes[i..].starts_with(b"...") {
            tokens.push(Token::Unsupported {
                construct: "spread/rest (...)",
                spelling: "...",
            });
            i += 3;
            continue;
        }
        if bytes[i..].starts_with(b"=>") {
            tokens.push(Token::Unsupported {
                construct: "arrow function (=>)",
                spelling: "=>",
            });
            i += 2;
            continue;
        }
        if bytes[i..].starts_with(b"??") {
            tokens.push(Token::Unsupported {
                construct: "nullish coalescing (??)",
                spelling: "??",
            });
            i += 2;
            continue;
        }
        if bytes[i..].starts_with(b"?.") {
            tokens.push(Token::Unsupported {
                construct: "optional chaining (?.)",
                spelling: "?.",
            });
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            // Lex the whole literal as one opaque token, up to its closing
            // backtick (or end of input if unterminated). Its interior —
            // `${…}` interpolation, embedded quotes — is not Forge syntax
            // and must not be fed to the operator scanner below; emitting a
            // bare marker and lexing on would turn a template literal into
            // a misleading `expression/lex` on whatever character came
            // next. Consuming it whole keeps the diagnostic the author gets
            // the one that names what they actually wrote.
            let close = input[i + 1..].find('`');
            i = match close {
                Some(offset) => i + 1 + offset + 1,
                None => len,
            };
            tokens.push(Token::Unsupported {
                construct: "template literal (`)",
                spelling: "`",
            });
            continue;
        }

        // Multi-char operators (longest match first), on bytes for the
        // reason given above the Forge-subset needles.
        if bytes[i..].starts_with(b">>>") {
            tokens.push(Token::UShr);
            i += 3;
            continue;
        }
        if bytes[i..].starts_with(b"===") {
            tokens.push(Token::StrictEq);
            i += 3;
            continue;
        }
        if bytes[i..].starts_with(b"!==") {
            tokens.push(Token::StrictNeq);
            i += 3;
            continue;
        }
        if i + 1 < len {
            let two = [bytes[i], bytes[i + 1]];
            let tok = match &two {
                b"==" => Some(Token::LooseEq),
                b"!=" => Some(Token::LooseNeq),
                b"&&" => Some(Token::AmpAmp),
                b"||" => Some(Token::PipePipe),
                b"<<" => Some(Token::Shl),
                b">>" => Some(Token::Shr),
                b"<=" => Some(Token::LtEq),
                b">=" => Some(Token::GtEq),
                b"++" if ecma => Some(Token::PlusPlus),
                b"--" if ecma => Some(Token::MinusMinus),
                b"+=" if ecma => Some(Token::OpAssign(ArithOp::Add)),
                b"-=" if ecma => Some(Token::OpAssign(ArithOp::Sub)),
                b"*=" if ecma => Some(Token::OpAssign(ArithOp::Mul)),
                b"/=" if ecma => Some(Token::OpAssign(ArithOp::Div)),
                b"%=" if ecma => Some(Token::OpAssign(ArithOp::Mod)),
                _ => None,
            };
            if let Some(t) = tok {
                tokens.push(t);
                i += 2;
                continue;
            }
        }

        // Single-char operators
        let tok = match bytes[i] {
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'%' => Token::Percent,
            b'<' => Token::Lt,
            b'>' => Token::Gt,
            b'&' => Token::Amp,
            b'|' => Token::Pipe,
            b'^' => Token::Caret,
            b'~' => Token::Tilde,
            b'!' => Token::Bang,
            b'?' => Token::Question,
            b':' => Token::Colon,
            b'.' => Token::Dot,
            b',' => Token::Comma,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b'{' if ecma => Token::LBrace,
            b'}' if ecma => Token::RBrace,
            b';' if ecma => Token::Semi,
            b'=' if ecma => Token::Assign,
            ch => {
                // `ch` is one byte, and the character that stopped the lexer
                // may be several: `ch as char` reads a UTF-8 lead byte as the
                // Latin-1 codepoint of that byte, so the author would be
                // shown a character that is nowhere in their document and
                // cannot be searched for. Decode the character actually at
                // `i` instead; the byte is the fallback only for input this
                // arm cannot reach, since `i` is always a character boundary.
                let shown = input[i..].chars().next().unwrap_or(ch as char);
                return Err(ExprError::Lex {
                    position: i,
                    detail: format!("unexpected character: '{shown}'"),
                }
                .at(Some(i..i + shown.len_utf8())));
            }
        };
        tokens.push(tok);
        i += 1;
    }

    close_span(&tokens, &mut spans, token_start..i);
    tokens.push(Token::Eof);
    spans.push(len..len);
    Ok((tokens, spans))
}

/// Give the token a lexer pass just read — if it read one — its range.
fn close_span(
    tokens: &[Token],
    spans: &mut Vec<std::ops::Range<usize>>,
    range: std::ops::Range<usize>,
) {
    debug_assert!(
        tokens.len() <= spans.len() + 1,
        "a lexer pass read more than one token, so their ranges cannot be told apart"
    );
    if spans.len() < tokens.len() {
        spans.push(range);
    }
}

fn validate_keyword(word: &str) -> Result<(), ExprError> {
    const REJECTED: &[(&str, &str)] = &[
        ("new", "new"),
        ("delete", "delete"),
        ("typeof", "typeof"),
        ("instanceof", "instanceof"),
        ("this", "this"),
        ("eval", "eval()"),
        ("async", "async"),
        ("await", "await"),
        ("yield", "yield"),
        ("function", "function declaration"),
        ("class", "class declaration"),
        ("var", "var declaration"),
        ("let", "let declaration"),
        ("const", "const declaration"),
    ];
    for &(kw, desc) in REJECTED {
        if word == kw {
            return Err(ExprError::UnsupportedConstruct {
                construct: desc.to_string(),
                observed: Some(kw.to_string()),
            });
        }
    }
    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Recursive descent parser — produces a `TypedExpr` whose every
// node has `ty: InferredType::Unknown`. `infer_types` fills those in.
//
// Precedence (lowest → highest, matching ECMAScript):
//   1. Conditional     ( ? : )
//   2. Logical OR      ( || )
//   3. Logical AND     ( && )
//   4. Bitwise OR      ( | )
//   5. Bitwise XOR     ( ^ )
//   6. Bitwise AND     ( & )
//   7. Equality        ( === !== )
//   8. Relational      ( < > <= >= )
//   9. Shift           ( << >> >>> )
//  10. Additive        ( + - )
//  11. Multiplicative  ( * / % )
//  12. Unary           ( - + ! ~ )
//  13. Postfix         ( . [] () )
//  14. Primary         ( literals, identifiers, grouping )
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct Parser<'a> {
    tokens: &'a [Token],
    /// One byte range per token, from [`tokenize_spanned`].
    spans: &'a [std::ops::Range<usize>],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], spans: &'a [std::ops::Range<usize>]) -> Self {
        debug_assert_eq!(tokens.len(), spans.len(), "one range per token");
        Self {
            tokens,
            spans,
            pos: 0,
        }
    }

    /// Where the token at `index` was read from — the range a refusal of
    /// it is raised at. Past the end, the end: the `Eof` a parser runs
    /// into has the empty range there, and so does anything beyond it.
    fn span_of(&self, index: usize) -> Option<std::ops::Range<usize>> {
        self.spans.get(index).or(self.spans.last()).cloned()
    }

    /// Where the token last consumed was read from.
    fn consumed_span(&self) -> Option<std::ops::Range<usize>> {
        self.span_of(self.pos.saturating_sub(1))
    }

    /// Where the construct about to be parsed begins.
    fn start(&self) -> usize {
        self.spans
            .get(self.pos)
            .or(self.spans.last())
            .map_or(0, |span| span.start)
    }

    /// A node for the construct that began at `start` and ends with the
    /// last token consumed.
    fn node(&self, start: usize, kind: ExprKind) -> TypedExpr {
        let end = self
            .pos
            .checked_sub(1)
            .and_then(|last| self.spans.get(last))
            .map_or(start, |span| span.end.max(start));
        TypedExpr {
            kind,
            ty: InferredType::Unknown,
            span: Some(start..end),
        }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = self.tokens.get(self.pos).unwrap_or(&Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), Refusal> {
        let tok = self.advance().clone();
        if &tok == expected {
            Ok(())
        } else {
            Err(ExprError::ParseMismatch {
                expected: format!("'{expected}'"),
                got: tok.to_string(),
            }
            .at(self.consumed_span()))
        }
    }

    fn parse_expression(&mut self) -> Result<TypedExpr, Refusal> {
        let expr = self.parse_conditional()?;
        self.expect_end()?;
        Ok(expr)
    }

    /// A text that is an argument list and nothing more — see
    /// [`argument_ranges`].
    fn parse_argument_list(&mut self) -> Result<Vec<TypedExpr>, Refusal> {
        let arguments = self.parse_arguments(&Token::Eof)?;
        self.expect_end()?;
        Ok(arguments)
    }

    /// A call's arguments: none when `closer` closes the list at once, else
    /// `conditional (',' conditional)*`. Stops at the first token that does
    /// not continue the list; the caller expects whatever closes it.
    fn parse_arguments(&mut self, closer: &Token) -> Result<Vec<TypedExpr>, Refusal> {
        let mut arguments = Vec::new();
        if self.peek() != closer {
            arguments.push(self.parse_conditional()?);
            while *self.peek() == Token::Comma {
                self.advance();
                arguments.push(self.parse_conditional()?);
            }
        }
        Ok(arguments)
    }

    /// Nothing may follow what was parsed.
    fn expect_end(&self) -> Result<(), Refusal> {
        if *self.peek() == Token::Eof {
            return Ok(());
        }
        Err(ExprError::UnexpectedToken {
            token: self.peek().to_string(),
        }
        .at(self.span_of(self.pos)))
    }

    fn parse_conditional(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let expr = self.parse_logical_or()?;
        if *self.peek() == Token::Question {
            self.advance();
            let consequent = self.parse_conditional()?;
            self.expect(&Token::Colon)?;
            let alternate = self.parse_conditional()?;
            return Ok(self.node(
                start,
                ExprKind::Conditional {
                    condition: Box::new(expr),
                    consequent: Box::new(consequent),
                    alternate: Box::new(alternate),
                },
            ));
        }
        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_logical_and()?;
        while *self.peek() == Token::PipePipe {
            self.advance();
            let right = self.parse_logical_and()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_bitwise_or()?;
        while *self.peek() == Token::AmpAmp {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_bitwise_xor()?;
        while *self.peek() == Token::Pipe {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::BitOr,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_bitwise_and()?;
        while *self.peek() == Token::Caret {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::BitXor,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_equality()?;
        while *self.peek() == Token::Amp {
            self.advance();
            let right = self.parse_equality()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::BitAnd,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek() {
                Token::StrictEq => BinOp::StrictEq,
                Token::StrictNeq => BinOp::StrictNeq,
                _ => break,
            };
            self.advance();
            let right = self.parse_relational()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_shift()?;
        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_shift()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Shl => BinOp::Shl,
                Token::Shr => BinOp::Shr,
                Token::UShr => BinOp::UShr,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let op = match self.peek() {
            Token::Minus => Some(UnaryOp::Neg),
            Token::Plus => Some(UnaryOp::Pos),
            Token::Bang => Some(UnaryOp::Not),
            Token::Tilde => Some(UnaryOp::BitNot),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(self.node(
                start,
                ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
            ));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                Token::Dot => {
                    self.advance();
                    let prop = match self.advance().clone() {
                        Token::Ident(s) => s,
                        other => {
                            return Err(ExprError::ParseMismatch {
                                expected: "property name after '.'".into(),
                                got: other.to_string(),
                            }
                            .at(self.consumed_span()))
                        }
                    };
                    expr = self.node(
                        start,
                        ExprKind::Member {
                            object: Box::new(expr),
                            property: prop,
                        },
                    );
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_conditional()?;
                    self.expect(&Token::RBracket)?;
                    expr = self.node(
                        start,
                        ExprKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                    );
                }
                Token::LParen => {
                    self.advance();
                    let args = self.parse_arguments(&Token::RParen)?;
                    self.expect(&Token::RParen)?;
                    expr = self.node(
                        start,
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                            params: Vec::new(),
                        },
                    );
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        match self.advance().clone() {
            Token::Number(n) => Ok(self.node(start, ExprKind::NumberLit(n))),
            Token::String { value, quote } => {
                Ok(self.node(start, ExprKind::StringLit { value, quote }))
            }
            Token::Ident(s) if s == "true" => Ok(self.node(start, ExprKind::BoolLit(true))),
            Token::Ident(s) if s == "false" => Ok(self.node(start, ExprKind::BoolLit(false))),
            Token::Ident(s) if s == "null" => Ok(self.node(start, ExprKind::NullLit)),
            Token::Ident(s) => Ok(self.node(start, ExprKind::Ident(s))),
            Token::LParen => {
                let inner = self.parse_conditional()?;
                self.expect(&Token::RParen)?;
                // The operand as written, parentheses and all.
                Ok(TypedExpr {
                    span: self.node(start, ExprKind::NullLit).span,
                    ..inner
                })
            }
            other => Err(ExprError::UnexpectedToken {
                token: other.to_string(),
            }
            .at(self.consumed_span())),
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Rename pass — applied before inference
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Apply an identifier rename map to the AST.
///
/// Handles two cases:
/// 1. Bare `Ident` nodes: renamed via the `renames` map
///    (e.g., `retryCount` → `retryCount_`).
/// 2. `Member{Ident(x), prop}` patterns: if the full `x.prop` path is in
///    `renames`, the entire Member node collapses to a single `Ident`
///    (e.g., `_event.data` → `pendingEventData_`).
///
/// Property names in Member access are NOT renamed — they represent struct
/// fields. Function call names (`Call.callee`) ARE subject to renaming if
/// they match a key; datamodel variable names and function names occupy
/// different identifier spaces in SCXML, so collisions are rare in practice.
fn rename_identifiers(ast: &mut TypedExpr, renames: &HashMap<&str, &str>) {
    match &mut ast.kind {
        ExprKind::Ident(name) => {
            if let Some(renamed) = renames.get(name.as_str()) {
                // Produce a Raw node — the rename map value is the
                // target-language-native fragment, and downstream emitters
                // must not apply further case conversion to it.
                ast.kind = ExprKind::Raw(renamed.to_string());
            }
        }
        ExprKind::Binary { left, right, .. } => {
            rename_identifiers(left, renames);
            rename_identifiers(right, renames);
        }
        ExprKind::Unary { operand, .. } => {
            rename_identifiers(operand, renames);
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            rename_identifiers(condition, renames);
            rename_identifiers(consequent, renames);
            rename_identifiers(alternate, renames);
        }
        ExprKind::Member { object, property } => {
            if let ExprKind::Ident(obj_name) = &object.kind {
                let full_path = format!("{}.{}", obj_name, property);
                if let Some(renamed) = renames.get(full_path.as_str()) {
                    ast.kind = ExprKind::Raw(renamed.to_string());
                    ast.ty = InferredType::Unknown;
                    return;
                }
            }
            rename_identifiers(object, renames);
        }
        ExprKind::Index { object, index } => {
            rename_identifiers(object, renames);
            rename_identifiers(index, renames);
        }
        ExprKind::Call { callee, args, .. } => {
            rename_identifiers(callee, renames);
            for arg in args {
                rename_identifiers(arg, renames);
            }
        }
        ExprKind::BytesView { source, len } => {
            rename_identifiers(source, renames);
            if let Some(len) = len {
                rename_identifiers(len, renames);
            }
        }
        ExprKind::Raw(_)
        | ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit => {}
    }
}

/// PascalCase every `Member` access property so Go member reads bind
/// against the exported codec struct fields (`entry.pattern` →
/// `entry.Pattern`). Uses the same `to_pascal_case` transform as the Go
/// arm of `codec_field_id`, the single source of truth for Go struct
/// field casing. Runs only for the algorithm kind (see the gate in
/// [`transpile_typed`]); within that kind every member access is a codec
/// element-field read, so PascalCasing the leaf is always correct.
fn export_go_member_properties(ast: &mut TypedExpr) {
    match &mut ast.kind {
        ExprKind::Member { object, property } => {
            *property = crate::filters::to_pascal_case(property.clone());
            export_go_member_properties(object);
        }
        ExprKind::Binary { left, right, .. } => {
            export_go_member_properties(left);
            export_go_member_properties(right);
        }
        ExprKind::Unary { operand, .. } => export_go_member_properties(operand),
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            export_go_member_properties(condition);
            export_go_member_properties(consequent);
            export_go_member_properties(alternate);
        }
        ExprKind::Index { object, index } => {
            export_go_member_properties(object);
            export_go_member_properties(index);
        }
        ExprKind::Call { callee, args, .. } => {
            export_go_member_properties(callee);
            for arg in args {
                export_go_member_properties(arg);
            }
        }
        ExprKind::BytesView { source, len } => {
            export_go_member_properties(source);
            if let Some(len) = len {
                export_go_member_properties(len);
            }
        }
        ExprKind::Raw(_)
        | ExprKind::Ident(_)
        | ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit => {}
    }
}

/// Flatten a Member/Ident object chain into its dotted source path —
/// `Some("_event.data")` for `Member{Member{Ident(_event), data}}`,
/// `Some("frame")` for `Ident(frame)`. Returns `None` if any segment is
/// not a plain `Ident` or `Member` of such (e.g. a `Call` or `Index`),
/// because those carry no qualified-key meaning for `ctx.vars`.
///
/// This is the single path-construction point shared by the inference
/// and rename passes' qualified-key handling — generalizing the former
/// bare-`Ident`-only lookup to arbitrary depth without changing the
/// resolution semantics for any already-registered key shape.
fn flatten_member_path(expr: &TypedExpr) -> Option<String> {
    match &expr.kind {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Member { object, property } => {
            Some(format!("{}.{property}", flatten_member_path(object)?))
        }
        _ => None,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Inference pass — annotates every node's `ty` bottom-up
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Recursively infer the natural type of every node in the AST using the
/// provided context.
///
/// This is a simple bottom-up pass: each node's type is computed from its
/// children's types plus the context (for identifiers and calls). The pass
/// does NOT propagate an "expected" type top-down — that is handled at
/// emission time, where each language emitter decides how to coerce an
/// operand's natural type to the surrounding context.
///
/// Rationale for separating inference and coercion: the natural type of an
/// expression is language-agnostic (governed by the lattice in `forge::types`),
/// but the syntax of a coercion (e.g., `x as f64` vs `float64(x)` vs
/// `x.toDouble()`) is language-specific. Emitters keep their knowledge
/// localized by consulting the tree's natural types.
/// Decode a string literal's verbatim source `value` into its byte
/// sequence for a `Bytes`-typed comparison (RFC §bytesguard-3 B2). Conservative:
/// only printable ASCII (no backslash escape, no non-ASCII byte) is
/// accepted — the byte sequence is then exactly `value.as_bytes()`,
/// byte-identical on every backend with no target-compiler escape
/// involvement. Anything else returns `None`; the caller then leaves the
/// node a `StringLit`, and the receive-side validator rejects the
/// comparison with a clear diagnostic rather than emitting an ambiguous
/// byte constant. The `BytesLit` carrier is forward-compatible: a future
/// escape/UTF-8 decoder widens this helper without touching the IR or
/// the emitters.
///
/// Exposed `pub(crate)` so the receive-side validator
/// (`event_schema_check`) shares the exact same printable-ASCII
/// admission rule (RFC §bytesguard-3 B2) instead of re-deriving it: a literal this
/// helper declines is rejected at validation time with a clear
/// diagnostic rather than slipping through to an ambiguous byte
/// constant at codegen.
pub(crate) fn decode_bytes_literal(value: &str) -> Option<Vec<u8>> {
    if value
        .bytes()
        .all(|b| (b.is_ascii_graphic() && b != b'\\') || b == b' ')
    {
        Some(value.as_bytes().to_vec())
    } else {
        None
    }
}

/// If `lit_node` is a `StringLit` and `other` is `Bytes`-typed, rewrite
/// `lit_node` in place into a `BytesLit` carrying the decoded bytes and
/// retype it `Bytes`. No-op for every other shape (including a string
/// literal whose content [`decode_bytes_literal`] declines). This is the
/// single site that performs the string→bytes reinterpretation; emitters
/// only render the resulting `BytesLit`. See RFC §bytesguard-3 B1.
fn reinterpret_string_as_bytes(lit_node: &mut TypedExpr, other: &TypedExpr) {
    if !matches!(other.ty, InferredType::Bytes) {
        return;
    }
    let ExprKind::StringLit { value, .. } = &lit_node.kind else {
        return;
    };
    let Some(bytes) = decode_bytes_literal(value) else {
        return;
    };
    lit_node.kind = ExprKind::BytesLit { bytes };
    lit_node.ty = InferredType::Bytes;
}

/// Render a decoded byte sequence as the content of a double-quoted
/// target string literal. [`decode_bytes_literal`] guarantees printable
/// ASCII with no backslash, so the only character that needs escaping is
/// the double quote itself. Shared by the byte-equality emitters that
/// wrap the bytes in a target string literal (Rust `b"…"`, Python `b"…"`,
/// Go `"…"`, Kotlin `"…"`, C `"…"`).
fn bytes_as_quoted_ascii(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        if b == b'"' {
            s.push('\\');
        }
        s.push(b as char);
    }
    s
}

/// Render a decoded byte sequence as a comma-separated list of hex byte
/// values (`0x61, 0x63, 0x6b`). Used by the C++ vector-literal and the
/// Go/Kotlin standalone-fallback renderings, which have no byte-string
/// literal syntax.
fn bytes_as_hex_list(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("0x{b:02x}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn infer_types(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    expr.ty = match &mut expr.kind {
        ExprKind::NumberLit(n) => {
            if is_float_literal_text(n) {
                InferredType::UntypedFloat
            } else {
                InferredType::UntypedInt
            }
        }
        ExprKind::StringLit { .. } => InferredType::Str,
        ExprKind::BytesLit { .. } => InferredType::Bytes,
        ExprKind::BoolLit(_) => InferredType::Bool,
        ExprKind::NullLit => InferredType::Null,
        ExprKind::Ident(name) => ctx.lookup_var(name.as_str()),
        // Raw fragments are opaque — they came from a rename map whose key was
        // looked up in the same TypeCtx before renaming happened, so the
        // surrounding node already has the correct type information bound via
        // `rename_identifiers` (which leaves `ast.ty` untouched for Ident→Raw
        // and resets it to Unknown for Member→Raw). Treating them as Unknown
        // here is correct: no local type information to recover.
        ExprKind::Raw(_) => InferredType::Unknown,
        ExprKind::Binary { op, left, right } => {
            infer_types(left, ctx);
            infer_types(right, ctx);
            // Bytes reinterpretation (RFC §bytesguard-3 B1): a string literal compared
            // against a `Bytes`-typed operand denotes the literal's bytes,
            // not a target-language string. Rewrite the `StringLit` child
            // into a `BytesLit` carrying the decoded bytes so every emitter
            // renders the same byte sequence through its content-equality
            // primitive. Done once, here, rather than re-derived per emitter.
            if op.is_comparison() {
                reinterpret_string_as_bytes(left, right);
                reinterpret_string_as_bytes(right, left);
            }
            if op.is_arith() {
                join_arith(left.ty, right.ty)
            } else if op.is_comparison() || op.is_logical() {
                InferredType::Bool
            } else if op.is_bitwise() {
                join_int(left.ty, right.ty)
            } else {
                InferredType::Unknown
            }
        }
        ExprKind::Unary { op, operand } => {
            infer_types(operand, ctx);
            match op {
                UnaryOp::Neg | UnaryOp::Pos => operand.ty,
                UnaryOp::Not => InferredType::Bool,
                UnaryOp::BitNot => operand.ty,
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            infer_types(condition, ctx);
            infer_types(consequent, ctx);
            infer_types(alternate, ctx);
            join_arith(consequent.ty, alternate.ty)
        }
        ExprKind::Member { object, property } => {
            infer_types(object, ctx);
            // Qualified-key lookup: form the dotted source path of the
            // Member's object chain and consult `ctx.vars`. Stateful
            // import aliases register their fields under the depth-2
            // shape `"frame.payload"` (see
            // `forge::type_ctx::insert_stateful_imports`); EventSchema
            // typed-guard lowering registers `"_event.data.<field>"` at
            // depth 3 (`forge::event_schema_check::lower_typed_guard`).
            // [`flatten_member_path`] handles both — and any deeper
            // all-Ident/Member chain — uniformly. A chain containing a
            // non-path segment (Call, Index, …) yields `None` and falls
            // through to `Unknown`, exactly as the prior bare-`Ident`-only
            // guard did. `lookup_var` returns `Unknown` for an absent
            // key, so chains whose full path is not registered stay
            // opaque — no behaviour change for existing callers.
            //
            // Inference must run BEFORE rename (see top-of-file rationale)
            // so the Member node still carries its `Ident`/`Member` form
            // here; after rename the object may become `Raw(...)` and the
            // qualified-key lookup would no longer resolve.
            match flatten_member_path(object) {
                Some(base) => ctx.lookup_var(&format!("{base}.{property}")),
                None => InferredType::Unknown,
            }
        }
        ExprKind::Index { object, index } => {
            infer_types(object, ctx);
            infer_types(index, ctx);
            match object.ty {
                InferredType::Bytes => InferredType::Int {
                    signed: false,
                    bits: 8,
                },
                _ => {
                    // RFC §synth-5-A: `<sce:const name="X" type="array<elem, N>">`
                    // registers `X` in `ctx.array_elems`. Recover the
                    // element type so per-language emitters (Kotlin's
                    // narrow-unsigned widening, in particular) can wrap
                    // the index result with the right cast.
                    if let ExprKind::Ident(name) = &object.kind {
                        if let Some(elem) = ctx.lookup_array_elem(name.as_str()) {
                            elem
                        } else {
                            InferredType::Unknown
                        }
                    } else {
                        InferredType::Unknown
                    }
                }
            }
        }
        ExprKind::Call {
            callee,
            args,
            params: call_params,
        } => {
            infer_types(callee, ctx);
            for a in args.iter_mut() {
                infer_types(a, ctx);
            }
            // Resolve the callee's signature. A bare identifier names a
            // stateless import (cross-algorithm dispatch, transform,
            // condition, lookup); a `obj.method` member call names a
            // stateful import method (registered as `"{obj}.{method}"` by
            // `insert_stateful_imports`). Capture `(ret, params)` by value
            // so the argument-projection pass below can borrow `args`
            // mutably after the immutable `ctx` lookup ends.
            let resolved: Option<(InferredType, Vec<InferredType>)> = match &callee.kind {
                ExprKind::Ident(name) => ctx
                    .lookup_func(name.as_str())
                    .map(|s| (s.ret, s.params.clone())),
                ExprKind::Member { object, property } => {
                    if let ExprKind::Ident(obj_name) = &object.kind {
                        let qualified = format!("{}.{}", obj_name, property);
                        ctx.lookup_func(&qualified)
                            .map(|s| (s.ret, s.params.clone()))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            match resolved {
                Some((ret, params)) => {
                    // Wildcard-keyexpr Str-argument projection: a `Str`
                    // argument flowing into a `bytes` parameter is a
                    // bounded-string field used as bytes — project it to a
                    // borrowed `bytes` view so the call site emits each
                    // backend's byte-view idiom. A `Bytes` argument (an
                    // upstream view param) passes through unprojected.
                    // Gated to the algorithm kind via the TypeCtx flag.
                    if ctx.project_str_args_as_bytes_view {
                        for (i, a) in args.iter_mut().enumerate() {
                            if matches!(params.get(i), Some(InferredType::Bytes))
                                && matches!(a.ty, InferredType::Str)
                            {
                                project_str_as_bytes_view(a, ctx);
                            }
                        }
                    }
                    // Every other argument converts to its parameter's type
                    // when emitted, as a value converts to the place it
                    // lands in; one of a kind its parameter does not admit
                    // is refused before that (`reject_call_argument_mismatches`).
                    //
                    // ⚠ Until 2026-09-24 the arguments were emitted as they
                    // were: a `uint8` passed to a `uint16` parameter reached
                    // Rust as a `u8`, which rustc refused, while the
                    // `<sce:call>` statement form converted it.
                    *call_params = params;
                    ret
                }
                // Item C7 wildcard-keyexpr lowering: the `len(<bytes|str>)` builtin is
                // not a registered signature, but its result is an integer
                // length, not `Unknown`. Typing it `UntypedInt` lets the
                // surrounding comparison adopt the *other* operand's concrete
                // width (`join_arith(Int{32}, UntypedInt) = Int{32}`) instead
                // of being poisoned to `Unknown`, so a context like
                // `i < len(a)` (`i: u32`) can width-coerce the always-wider
                // host length type (`usize` / `size_t`) at emit time.
                None if is_len_builtin(callee, args) => InferredType::UntypedInt,
                // `round` and `floor` both yield a whole number, and the
                // notation they lower from says so outright: with no digit
                // comment attached, the result is an integer. `UntypedInt`
                // rather than a fixed width for the same reason `len` uses it —
                // the surrounding context decides how wide, so a linear unit
                // conversion adopts its declared output width instead of
                // forcing one.
                None if real_to_int_builtin(callee, args).is_some() => InferredType::UntypedInt,
                None => InferredType::Unknown,
            }
        }
        ExprKind::BytesView { source, len } => {
            infer_types(source, ctx);
            if let Some(len) = len {
                infer_types(len, ctx);
            }
            InferredType::Bytes
        }
    };
}

/// RFC c7-wildcard W-project: rewrite `node` in place into a
/// [`ExprKind::BytesView`] carrying the original `Str`-typed node as its
/// source, typing the result `Bytes`. Mirrors the in-place node rewrite
/// of [`reinterpret_string_as_bytes`]. Called from the `Call` inference
/// arm for a `Str` argument that flows into a `bytes` parameter.
///
/// When the source is an element-field member (`entry.pattern`) whose C11
/// length sibling is registered in `ctx` (from the codec `length_field`
/// SSOT — §8 Smell A fix), the rewrite attaches an explicit `len` node
/// `entry.<len_member>` so the C11 emit pairs the *actual* length member
/// rather than guessing `<src>_len`. `len` is `None` for any other source
/// shape; only the C11 emit reads it.
fn project_str_as_bytes_view(node: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    // Resolve the length sibling before moving the source out of `node`.
    let len = if let ExprKind::Member { object, property } = &node.kind {
        flatten_member_path(object)
            .map(|base| format!("{base}.{property}"))
            .and_then(|path| ctx.lookup_member_len_field(&path))
            .map(|len_member| {
                // The length sibling is a small unsigned count; its
                // precise width does not affect the C11 emit (it
                // initialises a `size_t`), so `Unknown` is faithful.
                Box::new(TypedExpr::new(ExprKind::Member {
                    object: object.clone(),
                    property: len_member.to_string(),
                }))
            })
    } else {
        None
    };
    let source = std::mem::replace(node, TypedExpr::new(ExprKind::NullLit));
    // The view is read from the same text its source was.
    node.span = source.span.clone();
    node.kind = ExprKind::BytesView {
        source: Box::new(source),
        len,
    };
    node.ty = InferredType::Bytes;
}

/// True if a numeric literal's text shape is float-like (has `.`, `e`, `E`).
fn is_float_literal_text(n: &str) -> bool {
    n.contains('.') || n.contains('e') || n.contains('E')
}

/// True if an integer literal string is a decimal form that can be safely
/// rewritten as a float literal by appending `.0`. Hex/binary/octal literals
/// must NOT be float-promoted — the resulting text would be invalid in every
/// target language.
fn is_decimal_integer_literal(n: &str) -> bool {
    !n.is_empty()
        && !is_float_literal_text(n)
        && !n.starts_with("0x")
        && !n.starts_with("0X")
        && !n.starts_with("0b")
        && !n.starts_with("0B")
        && !n.starts_with("0o")
        && !n.starts_with("0O")
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Operator precedence (per target language) + paren insertion
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn ecma_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::BitOr => 3,
        BinOp::BitXor => 4,
        BinOp::BitAnd => 5,
        BinOp::StrictEq | BinOp::StrictNeq => 6,
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

fn kotlin_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq => 3,
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr | BinOp::UShr => 7,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

fn go_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 3,
        BinOp::Add | BinOp::Sub | BinOp::BitOr | BinOp::BitXor => 4,
        BinOp::Mul
        | BinOp::Div
        | BinOp::Mod
        | BinOp::Shl
        | BinOp::Shr
        | BinOp::UShr
        | BinOp::BitAnd => 5,
    }
}

fn rust_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitOr => 5,
        BinOp::BitXor => 6,
        BinOp::BitAnd => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

fn python_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Or => 1,
        BinOp::And => 2,
        BinOp::StrictEq | BinOp::StrictNeq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => 4,
        BinOp::BitOr => 5,
        BinOp::BitXor => 6,
        BinOp::BitAnd => 7,
        BinOp::Shl | BinOp::Shr | BinOp::UShr => 8,
        BinOp::Add | BinOp::Sub => 9,
        BinOp::Mul | BinOp::Div | BinOp::Mod => 10,
    }
}

fn child_needs_parens(
    child: &TypedExpr,
    parent_op: BinOp,
    is_left: bool,
    prec: fn(BinOp) -> u8,
) -> bool {
    match &child.kind {
        ExprKind::Binary { op: child_op, .. } => {
            let cp = prec(*child_op);
            let pp = prec(parent_op);
            if is_left {
                cp < pp
            } else {
                cp <= pp
            }
        }
        ExprKind::Conditional { .. } => true,
        _ => false,
    }
}

/// `&&` nested inside `||` — parenthesise it for the C family even though
/// precedence does not require it.
///
/// ⚠ Precedence-correct is not the same as compilable. `a || b && c` binds the
/// way the document means, so `child_needs_parens` leaves it bare — and then
/// GCC and Clang emit `-Wparentheses` ("suggest parentheses around '&&' within
/// '||'"). A target that builds with `-Wall -Werror`, which a real downstream
/// build does, turns that into a hard error, so the generated file does not
/// compile at all. Found by generating a component and feeding it to such a
/// build: an expression of the shape `a == 2 || b >= 1 && b <= 4` stopped it.
///
/// This is deliberately C-family only. Rust, Go, Kotlin and Python have no
/// such diagnostic, and widening the rule would re-pin every committed tree in
/// those languages to fix a warning they do not have.
fn c_family_clarity_parens(child: &TypedExpr, parent_op: BinOp) -> bool {
    matches!(parent_op, BinOp::Or) && matches!(&child.kind, ExprKind::Binary { op: BinOp::And, .. })
}

/// Wrap an emitted sub-expression in parens when it is used as the base of a
/// postfix access (`.`, `[]`, `()`) and the underlying AST shape would
/// otherwise bind differently than intended.
fn wrap_postfix(expr: &TypedExpr, emitted: String) -> String {
    if matches!(
        &expr.kind,
        ExprKind::Binary { .. } | ExprKind::Conditional { .. } | ExprKind::Unary { .. }
    ) {
        format!("({emitted})")
    } else {
        emitted
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emission helpers — per-Binary operand type computation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Compute the type both operands of a binary node should be coerced to
/// before the operator executes. For arithmetic and comparison this is the
/// lattice join; for bitwise it is the int join; for logical it is Bool.
fn binary_operand_type(op: BinOp, left: InferredType, right: InferredType) -> InferredType {
    if op.is_arith() {
        join_arith(left, right)
    } else if op.is_comparison() {
        // Comparison returns Bool but operands must share a numeric type.
        join_arith(left, right)
    } else if op.is_logical() {
        InferredType::Bool
    } else if op.is_bitwise() {
        join_int(left, right)
    } else {
        InferredType::Unknown
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — C++
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// C++ has implicit numeric conversions covering every case we care about:
// integer ↔ float, narrower ↔ wider, signed ↔ unsigned. The emitter does
// not insert `static_cast` — the compiler warns on narrowing conversions
// but accepts them. Untyped float literals are emitted verbatim.
//
// Untyped decimal integer literals in a float context get a `.0` suffix to
// prevent C++ integer division: `9 / 5` yields 1 without promotion.  The
// push-down block below propagates the Float expectation into arithmetic
// sub-trees so leaf literals reach cpp_coerce with the correct target type.
//
// The only other text transformation C++ does is mapping quote characters in
// string literals (single → double) and emitting `nullptr` for null.

/// Recognizes the `len(<bytes|str>)` builtin — the length accessor the
/// two-cursor keyexpr matcher uses to bound `pi < len(pattern)`. Each
/// emitter lowers it to its native length idiom (C11 `.len`, Rust
/// `.len()`, Cpp `.size()`, Kotlin `.size`, Go/Python `len(x)`) rather
/// than the generic `len(args)` call. Item C7 wildcard-keyexpr lowering.
fn is_len_builtin(callee: &TypedExpr, args: &[TypedExpr]) -> bool {
    args.len() == 1
        && matches!(args[0].ty, InferredType::Bytes | InferredType::Str)
        && matches!(&callee.kind, ExprKind::Ident(n) | ExprKind::Raw(n) if n == "len")
}

/// The type every emitter converts a call's argument `index` to: its
/// parameter's, from the call's resolved `params`, or none at all for a
/// callee no context registers.
fn argument_type(params: &[InferredType], index: usize) -> InferredType {
    params.get(index).copied().unwrap_or(InferredType::Unknown)
}

/// The unary builtins that consume a real number and produce an integer:
/// `round(x)` and `floor(x)`.
///
/// They share every structural property — one argument, a `Float{64}`
/// expectation pushed onto that argument so an integer sub-expression widens
/// before the conversion, `UntypedInt` inference so the surrounding context
/// picks the width, and a width-aware plus a width-free emitter arm in each
/// backend. Only the native call differs. Keeping them ONE family is what
/// stops the nine emitter arms from becoming eighteen.
///
/// ⚠ WHY THEY ARE BUILTINS AND NOT IMPORTS. The surveyed source of this
/// work — an automotive cluster specification — attaches an approximation to
/// computed values, and it has TWO halves rather than one: round, and round
/// down. Setting aside the piecewise-interpolation cases, which
/// `sce:kind="interpolation"` already covers, what is left is LINEAR UNIT
/// CONVERSIONS: temperature, km↔mile, displayed speed, the odometer,
/// state-of-charge, accumulated time. The arithmetic already lowered; the
/// approximation was the only thing missing, so these two names unlock the
/// largest arithmetic cluster in that material.
///
/// ⚠⚠ `round` IS HALF AWAY FROM ZERO, DECIDED HERE RATHER THAN INHERITED. The
/// backends do NOT agree on what rounding does at `.5`:
///
///   C/C++ `std::round`, Rust `f64::round`, Go `math.Round` — half away from
///   zero (0.5 → 1, −0.5 → −1)
///   Python `round`, Kotlin `kotlin.math.round` — half to EVEN (0.5 → 0)
///
/// Taking each language's default would make Python and Kotlin disagree with
/// the other four on exactly the `.5` inputs — a divergence invisible to any
/// fixture that does not feed a `.5`, which is why `transform_rounding`
/// carries them.
///
/// ⚠ THE JUSTIFICATION THIS COMMENT ONCE GAVE WAS WRONG, and the correction is
/// the load-bearing part. It said the choice followed "half-up by convention".
/// Half-up and half away from zero are the SAME for positives and DIFFER for
/// negatives — half-up sends −2.5 to −2, half away from zero sends it to −3 —
/// so that sentence named a rule this code does not implement, and a reader who
/// believed it could "simplify" the Python/Kotlin lowering to `floor(x + 0.5)`
/// unconditionally and break every negative input. Worked examples in the
/// source material settle it the other way at every negative `.5` they cross.
/// A word's connotation is not evidence about a boundary.
///
/// ⚠⚠⚠ `floor` IS TOWARD −∞, AND THAT TOO WAS MEASURED RATHER THAN READ OFF A
/// WORD. The notation's own name reads as "round down", while the word beside
/// it reads as "discard the digits" — and those two diverge on negatives, where
/// discarding gives −2 for −2.7 and rounding down gives −3. The tie is broken
/// by the shipped implementation this generator has to agree with: it uses
/// floor throughout and truncation nowhere, including at both of the places
/// whose operand can actually go negative. Every OTHER site operates on a
/// non-negative quantity, so the two are indistinguishable there; the choice is
/// still made here, because the six emitters must agree with each other
/// whatever any one document exercises. `transform_floor` is what holds it.
///
/// ⚠⚠⚠⚠ NEITHER TAKES A DIGIT COUNT, because neither needs one. "Round down to
/// two decimals" is `floor(x * 100) / 100`, and a quantise-to-a-grid is
/// `floor(x / step) * step`; both compose from these names plus arithmetic that
/// already lowers, and the implementation writes them exactly that way. A
/// digit-taking overload would be a second spelling of something the vocabulary
/// can already say, and a second place for the boundary rule to drift.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RealToInt {
    Round,
    Floor,
}

fn real_to_int_builtin(callee: &TypedExpr, args: &[TypedExpr]) -> Option<RealToInt> {
    if args.len() != 1 {
        return None;
    }
    match &callee.kind {
        ExprKind::Ident(n) | ExprKind::Raw(n) => match n.as_str() {
            "round" => Some(RealToInt::Round),
            "floor" => Some(RealToInt::Floor),
            _ => None,
        },
        _ => None,
    }
}

/// Names a forge expression may call without the context registering them.
///
/// `len` is the length builtin (`is_len_builtin` types it) and `eq` the
/// bytes-comparison builtin; both lower to each backend's native idiom. Every
/// other callable reaches a forge expression by being REGISTERED — a stateless
/// cross-file import, an `<sce:helper>` declaration, or a stateful import's
/// method — so `ctx.funcs` is the rest of the vocabulary.
const EXPR_BUILTINS: [&str; 4] = ["len", "eq", "round", "floor"];

/// Refuse a call to a name nothing provides.
///
/// ⚠ WHY THIS EXISTS. `infer_types` types an unresolved callee `Unknown` and
/// the emitters then print it VERBATIM, so until this check every one of
/// these was accepted with exit 0:
///
/// ```text
/// expr="totallyMadeUpFn(v)"   ->  return totallyMadeUpFn(v);
/// expr="Math.tanh(v)"         ->  return Math.tanh(v);
/// ```
///
/// Neither compiles, which is the only reason the hole was survivable — a
/// typo in a helper's name took the same path and was caught by the target
/// compiler rather than by SCE. Measured 2026-09-17 while looking for a form
/// for the specification's `Rounds off`.
///
/// ⚠⚠ THE FENCE IS `text` DELIBERATELY. This block was written indented, and
/// rustdoc reads an indented block as a Rust doctest — so `cargo test -p
/// sce-build` tried to COMPILE `expr="…"` as Rust and failed, while
/// `cargo test --lib` (which does not run doctests) stayed green over it.
/// Measured 2026-09-18.
///
/// ⚠⚠⚠ The example used to be `round(v * 100.0)`, said to be unprovided. It
/// is provided now — see `real_to_int_builtin` — so the example moved to names
/// that are still absent. An example of a refusal has to be a name the
/// vocabulary really lacks, or it teaches the opposite of what it says.
///
/// ⚠⚠ The check runs AFTER `infer_types` on purpose: inference is what binds
/// each `Call` to a signature, so asking before it would refuse everything,
/// and asking here means `ctx` already carries every import, helper and
/// stateful method the document declared.
/// ⚠⚠⚠ Gated on `ctx.reject_unknown_callees`, and the gate is not a
/// convenience. A STATECHART guard may call what the host provides and is
/// emitted verbatim by design; a FORGE KIND has no host to call. Only the
/// per-kind builders in [`crate::forge::type_ctx`] turn this on — the first
/// version of this check had no gate and refused three legitimate emitter
/// behaviours that the suite asserts.
fn reject_unknown_callees(expr: &TypedExpr, ctx: &TypeCtx<'_>) -> Result<(), Refusal> {
    if !ctx.reject_unknown_callees {
        return Ok(());
    }
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        let name = match &callee.kind {
            // ⚠ `Ident` ONLY, never `Raw`. A `Raw` callee is not something an
            // author wrote — it is this pipeline's own product, emitted by
            // `lower_stateful_import_calls` (C11 rewrites `smoother.update(x)`
            // into the C symbol `filter_low_pass_update(...)`) and by
            // `rename_identifiers`. Those names are not in `ctx.funcs` and
            // never will be: the context is keyed by the USER-VISIBLE name,
            // which is exactly what this check has to speak about. Both run
            // after this check ([`resolve_then_rename`]), so it reads the
            // call as the author wrote it on every backend.
            //
            // ⚠⚠ The first version read `Raw` too, with a comment asserting
            // that a lowered call "is a bare name the context registered".
            // That was wrong and the C11 arm said so —
            // `filter_low_pass_update is not provided by SCE's ECMAScript
            // datamodel. Available: eq, len, smoother.update` — refusing a
            // document the other five arms accept. The registered key was
            // `smoother.update`; the lowered symbol is a different string.
            ExprKind::Ident(n) => Some(n.clone()),
            // A stateful import's method is registered as `"{obj}.{method}"`;
            // anything else in callee position (an index, a nested call) is
            // not a name this check can speak about, so it is left alone.
            ExprKind::Member { object, property } => match &object.kind {
                ExprKind::Ident(obj) => Some(format!("{obj}.{property}")),
                _ => None,
            },
            _ => None,
        };
        if let Some(name) = name {
            let known = ctx.lookup_func(&name).is_some()
                || EXPR_BUILTINS.contains(&name.as_str())
                || is_len_builtin(callee, args);
            if !known {
                let mut available: Vec<String> =
                    ctx.funcs.keys().map(|k| (*k).to_string()).collect();
                available.extend(EXPR_BUILTINS.iter().map(|b| (*b).to_string()));
                available.sort();
                // Raised at the callee, not the call: the callee is the name
                // `actual` reports, and the arguments are not the mistake.
                return Err(ExprError::UnsupportedBuiltin {
                    name,
                    // ⚠ NOT the ECMAScript datamodel. A `sce:kind` document's
                    // `expr=` is evaluated by this layer, whose vocabulary is
                    // `EXPR_BUILTINS` plus the document's own imported
                    // functions — two names, not the seventeen of `Math`.
                    vocabulary: "SCE's forge expression layer".to_string(),
                    available,
                }
                .at(callee.span.clone()));
            }
        }
    }
    for child in expr_children(expr) {
        reject_unknown_callees(child, ctx)?;
    }
    Ok(())
}

/// Refuse a name the context does not carry: an operand nothing declares,
/// or an `<alias>.<name>` on an imported enum that declares no such variant.
///
/// ⚠ WHY THIS EXISTS. `infer_types` types an unresolved name `Unknown` and
/// the emitters print it verbatim, so until this check both of these
/// generated with exit 0 on all six backends (measured 2026-09-21):
///
/// ```text
/// expr="conut + 1"                  (beside <data id="count">)
/// expr="w ? Mode.RUN : Mode.STOP"    (the enum declares RUN_BATCH, not RUN)
/// ```
///
/// The first named an identifier nothing bound — a compile error in five
/// backends and a `NameError` in Python the first time the line ran. The
/// second names an undeclared enum variant. This invented mode example
/// exercises the same name-resolution rule: a declared variant must be
/// named explicitly rather than inferred from a shorter spelling.
///
/// ⚠⚠ The CALLEE of a call is not judged here — [`reject_unknown_callees`]
/// owns that, with the builtins and method keys it alone knows. Only the
/// arguments are walked.
///
/// ⚠⚠ A member is judged only where this layer knows the member set. An
/// enum alias's is closed and carried in `ctx.enums`. A declared VALUE's is
/// empty, and a closed record's is its registered fields and methods — both
/// enforced by [`reject_undeclared_member`]. An open record's belongs to
/// another pass — the event schema's for `_event` — so `_event.foo` is read
/// here as `_event` alone.
///
/// ⚠⚠⚠ `Ident` only, never `Raw` — a `Raw` is this pipeline's own product,
/// for the reason [`reject_unknown_callees`] gives.
fn reject_unknown_names(expr: &TypedExpr, ctx: &TypeCtx<'_>) -> Result<(), Refusal> {
    match &expr.kind {
        ExprKind::Member { object, property } => {
            if let ExprKind::Ident(alias) = &object.kind {
                if let Some(scope) = ctx.lookup_enum(alias) {
                    if scope.variants.iter().any(|v| v == property) {
                        return Ok(());
                    }
                    return Err(ExprError::UnknownEnumVariant {
                        alias: alias.clone(),
                        name: property.clone(),
                        declared: scope.variants.to_vec(),
                    }
                    .at(expr.span.clone()));
                }
            }
            if ctx.reject_unknown_identifiers {
                // The whole access is what either refusal names as `actual`.
                reject_undeclared_member(object, property, ctx)
                    .map_err(|refusal| refusal.at(expr.span.clone()))?;
            }
        }
        ExprKind::Call { callee, args, .. } if matches!(callee.kind, ExprKind::Ident(_)) => {
            for arg in args {
                reject_unknown_names(arg, ctx)?;
            }
            return Ok(());
        }
        ExprKind::Ident(name)
            if ctx.reject_unknown_identifiers && !ctx.vars.contains_key(name.as_str()) =>
        {
            return Err(ExprError::UnknownIdentifier {
                name: name.clone(),
                // A qualified `alias.field` key is not something a bare
                // name could have meant.
                candidates: crate::near_miss::near_misses(
                    name,
                    ctx.vars.keys().copied().filter(|k| !k.contains('.')),
                ),
            }
            .at(expr.span.clone()));
        }
        _ => {}
    }
    for child in expr_children(expr) {
        reject_unknown_names(child, ctx)?;
    }
    Ok(())
}

/// Refuse `<base>.<member>` unless `<base>` declares `<member>`: a declared
/// VALUE has no members at all, and a record whose members are known
/// ([`crate::forge::types::RecordShape::Closed`]) has exactly those.
///
/// ⚠ `<base>` is the whole dotted path, not only its head, so a member of a
/// record's scalar field is judged too — `frame.msg_id.foo` asks a `uint32`
/// for a member. A path the context does not carry (`_event.data`, whose
/// fields the event schema owns) is not declared, so it is left to the pass
/// that knows it; so is a base that is no name at all (a call, an index),
/// and so is a member of an open record.
///
/// A member registered under its full path — a field in `vars`, a method in
/// `funcs` — is declared and passes on any base: the registration IS the
/// declaration.
fn reject_undeclared_member(
    object: &TypedExpr,
    member: &str,
    ctx: &TypeCtx<'_>,
) -> Result<(), ExprError> {
    let Some(base) = flatten_member_path(object) else {
        return Ok(());
    };
    let Some(&base_ty) = ctx.vars.get(base.as_str()) else {
        return Ok(());
    };
    let path = format!("{base}.{member}");
    if ctx.vars.contains_key(path.as_str()) || ctx.funcs.contains_key(path.as_str()) {
        return Ok(());
    }
    if ctx.is_record(&base) {
        let Some(declared) = ctx.closed_record_members(&base) else {
            return Ok(());
        };
        return Err(ExprError::UnknownMember {
            record: base,
            member: member.to_string(),
            declared: declared.into_iter().map(str::to_string).collect(),
        });
    }
    Err(ExprError::MemberOfNonRecord {
        name: base,
        member: member.to_string(),
        ty: base_ty.declared_spelling(),
    })
}

/// Replace every `<alias>.<variant>` on an imported enum with this backend's
/// reference to that variant, as a pre-resolved [`ExprKind::Raw`].
///
/// ⚠ ONE place for every kind. The spelling was a rename table built by the
/// transform renderer alone, so a condition, validator, observer, procedure
/// or algorithm comparing against a variant emitted `Mode.RUN_BATCH` verbatim
/// into C++, Rust and C, where it names nothing. The spelling itself is
/// [`crate::forge::enum_naming::variant_ref`]'s — it has to match the
/// declaration the enum kind emits, and that module is what keeps the two
/// from drifting.
///
/// Runs after [`reject_unknown_names`], so every enum member reaching here
/// is a declared variant.
fn lower_enum_variant_refs(expr: &mut TypedExpr, ctx: &TypeCtx<'_>, target: ExprTarget) {
    if let ExprKind::Member { object, property } = &expr.kind {
        if let ExprKind::Ident(alias) = &object.kind {
            if let Some(scope) = ctx.lookup_enum(alias) {
                let reference = crate::forge::enum_naming::variant_ref(
                    target.language(),
                    scope.qualified_type,
                    scope.source_name,
                    property,
                );
                expr.kind = ExprKind::Raw(reference);
                return;
            }
        }
    }
    for child in expr_children_mut(expr) {
        lower_enum_variant_refs(child, ctx, target);
    }
}

/// Every sub-expression of `expr`, in source order, mutably — the twin of
/// [`expr_children`] for passes that rewrite the tree.
fn expr_children_mut(expr: &mut TypedExpr) -> Vec<&mut TypedExpr> {
    match &mut expr.kind {
        ExprKind::Binary { left, right, .. } => vec![left, right],
        ExprKind::Unary { operand, .. } => vec![operand],
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => vec![condition, consequent, alternate],
        ExprKind::Call { callee, args, .. } => {
            let mut v = vec![&mut **callee];
            v.extend(args.iter_mut());
            v
        }
        ExprKind::Index { object, index } => vec![object, index],
        ExprKind::Member { object, .. } => vec![&mut **object],
        ExprKind::BytesView { source, len } => match len {
            Some(l) => vec![source, l],
            None => vec![&mut **source],
        },
        _ => Vec::new(),
    }
}

/// `previous(<field>)` → the identifier of that field's cell, for every
/// field `ctx` holds a cell for (`TypeCtx::previous_cells`).
///
/// ⚠ ON THE TREE, not the text. `<sce:cycle>` calls are expanded as text
/// before rendering, which is sound there because they are replaced by
/// conditionals the author never sees an error in; a `previous()` read sits
/// inside the author's own expression, and a textual rename of a different
/// length would put every later diagnostic in that expression at a column
/// the author did not write. Rewriting the NODE keeps its span, so a type
/// error in `previous(x) + "s"` still points at `previous(x)`.
///
/// A `previous(…)` this does not rewrite — no cell for its argument, or no
/// cells at all, as in every context but a transform's — is left as the
/// call it is, and the callee check refuses it by that name.
pub(crate) fn lower_previous(ast: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    if ctx.previous_cells.is_empty() {
        return;
    }
    let cell = match &ast.kind {
        ExprKind::Call { callee, args, .. } => match (&callee.kind, args.as_slice()) {
            (ExprKind::Ident(name), [arg]) if name == crate::forge::previous_value::PREVIOUS => {
                match &arg.kind {
                    ExprKind::Ident(field) => ctx.previous_cells.get(field.as_str()).copied(),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    };
    if let Some(param) = cell {
        ast.kind = ExprKind::Ident(param.to_string());
        return;
    }
    for child in expr_children_mut(ast) {
        lower_previous(child, ctx);
    }
}

/// Every sub-expression of `expr`, in source order.
pub(crate) fn expr_children(expr: &TypedExpr) -> Vec<&TypedExpr> {
    match &expr.kind {
        ExprKind::Binary { left, right, .. } => vec![left, right],
        ExprKind::Unary { operand, .. } => vec![operand],
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => vec![condition, consequent, alternate],
        ExprKind::Call { callee, args, .. } => {
            let mut v = vec![&**callee];
            v.extend(args.iter());
            v
        }
        ExprKind::Index { object, index } => vec![object, index],
        ExprKind::Member { object, .. } => vec![&**object],
        ExprKind::BytesView { source, len } => match len {
            Some(l) => vec![source, l],
            None => vec![&**source],
        },
        _ => Vec::new(),
    }
}

fn emit_cpp(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // Push-down: propagate Float expectation into arithmetic sub-trees so
    // cpp_coerce appends `.0` to integer literals, preventing C++ integer
    // division (e.g. `9 / 5` → `9.0 / 5.0`).
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_cpp(left, expected)?;
            let r_raw = emit_cpp(right, expected)?;
            let l = if c_family_divides_integers(*op, left, right) {
                format!("static_cast<{}>({l_raw})", c_family_float_name(expected))
            } else if child_needs_parens(left, *op, true, ecma_precedence)
                || c_family_clarity_parens(left, *op)
            {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, ecma_precedence)
                || c_family_clarity_parens(right, *op)
            {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", cpp_binop(*op)));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context — without this, `-40` in a
    // float comparison falls into `cpp_emit_node`'s Unary arm which passes
    // `expr.ty` (the unary's own UntypedInt type) instead of the caller's
    // float `expected`, so the inner literal stays `40` and the result is
    // `-40` instead of the symmetric `-40.0`. Mirrors the Binary push-down
    // above.
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_cpp(operand, expected)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{}({inner})", cpp_unary(*op))
            } else {
                format!("{}{inner}", cpp_unary(*op))
            });
        }
    }
    let raw = cpp_emit_node(expr)?;
    Ok(cpp_coerce(raw, expr.ty, expected, expr))
}

fn cpp_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        // `std::vector<uint8_t>` has `operator==`, so the default
        // `==`/`!=` path compares length+content directly against this
        // value-constructed temporary. No byte-string literal in C++, so
        // a hex initializer list. RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => {
            format!("std::vector<uint8_t>{{{}}}", bytes_as_hex_list(bytes))
        }
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "nullptr".to_string(),
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_cpp(left, operand_ty)?;
            let r_raw = emit_cpp(right, operand_ty)?;
            let l = if child_needs_parens(left, *op, true, ecma_precedence)
                || c_family_clarity_parens(left, *op)
            {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, ecma_precedence)
                || c_family_clarity_parens(right, *op)
            {
                format!("({r_raw})")
            } else {
                r_raw
            };
            format!("{l} {} {r}", cpp_binop(*op))
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_cpp(operand, expr.ty)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            if wrap {
                format!("{}({inner})", cpp_unary(*op))
            } else {
                format!("{}{inner}", cpp_unary(*op))
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            format!(
                "{} ? {} : {}",
                emit_cpp(condition, InferredType::Bool)?,
                emit_cpp(consequent, expr.ty)?,
                emit_cpp(alternate, expr.ty)?,
            )
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_cpp(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            format!(
                "{}[{}]",
                wrap_postfix(object, emit_cpp(object, InferredType::Unknown)?),
                emit_cpp(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "({}).size()",
                    emit_cpp(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // `std::llround` is half away from zero and returns an
                // integer, which is what the notation asks for. `std::floor`
                // returns an integral-VALUED double, so the cast that follows
                // it is exact rather than a second rounding. The argument is
                // emitted with a Float expectation so an integer-typed
                // sub-expression widens rather than truncating before the
                // conversion happens.
                let inner = emit_cpp(&args[0], InferredType::Float { bits: 64 })?;
                return Ok(match op {
                    RealToInt::Round => format!("std::llround({inner})"),
                    RealToInt::Floor => format!("static_cast<long long>(std::floor({inner}))"),
                });
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_cpp(arg, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_cpp(callee, InferredType::Unknown)?),
                a.join(", "),
            )
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: a bounded-string field projected to
            // the algorithm `bytes` param type — `std::span<const
            // std::uint8_t>` over the string's data()/size().
            let s = emit_cpp(source, InferredType::Unknown)?;
            format!(
                "std::span<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>({s}.data()), {s}.size())"
            )
        }
    })
}

/// Whether a `/` evaluated in a float context would still divide as integers
/// in C or C++, and so needs its left operand widened first.
///
/// SCE_FORGE.md §3.4.1: `/` is real division whenever the result is real.
/// Rust, Kotlin and Go widen every operand in a float context and Python's
/// `/` is true division, so those four already agree. The C family widens
/// only through [`cpp_coerce`]'s `.0` on a decimal literal, which leaves the
/// case where BOTH operands stay integral — two integer variables, or a
/// hexadecimal literal that cannot take `.0` — dividing as integers before
/// the result is converted: `a / b` with `a = 7, b = 2` returned 3.0 into a
/// `double` output while the other four returned 3.5.
///
/// Widening the left operand alone is enough: the usual arithmetic
/// conversions then carry the right one.
fn c_family_divides_integers(op: BinOp, left: &TypedExpr, right: &TypedExpr) -> bool {
    op == BinOp::Div
        && stays_integral_in_float_context(left)
        && stays_integral_in_float_context(right)
}

/// An operand the C family still holds as an integer after float push-down.
/// A decimal integer literal is not one — [`cpp_coerce`] gives it `.0`.
fn stays_integral_in_float_context(operand: &TypedExpr) -> bool {
    match operand.ty {
        InferredType::Int { .. } => true,
        InferredType::UntypedInt => match &operand.kind {
            ExprKind::NumberLit(text) => !is_decimal_integer_literal(text),
            _ => true,
        },
        _ => false,
    }
}

fn c_family_float_name(expected: InferredType) -> &'static str {
    match expected {
        InferredType::Float { bits: 32 } => "float",
        _ => "double",
    }
}

fn cpp_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    if let Some(spelled) = c_family_wide_literal(&raw, to, node) {
        return spelled;
    }
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    // Promote untyped decimal integer literals in float context to prevent
    // C++ integer division: `9 / 5` → `9.0 / 5.0`.
    if let (UntypedInt, Float { .. }) = (from, to) {
        if let ExprKind::NumberLit(text) = &node.kind {
            if is_decimal_integer_literal(text) {
                return format!("{raw}.0");
            }
        }
    }
    raw
}

fn cpp_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>",
    }
}

fn cpp_unary(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Neg => "-",
        UnaryOp::Pos => "+",
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — Kotlin
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Kotlin is the strictest target: no implicit numeric conversions at all.
// Every type change requires an explicit `.toX()` call.
//
// * `Int{u,..} + Int{i,..}` — Kotlin does not permit mixing UInt/Int, so
//   unsigned operands get `.toInt()` / `.toLong()` in signed context.
// * `Int{..} op Float{..}` — int side needs `.toDouble()` / `.toFloat()`.
// * Untyped integer literals inside a float context — append `.0` to the
//   literal text so the emitted source is a `Double` literal.
// * Unsigned output coercion at the top level — when the caller's expected
//   type is `Int{u, bits}`, the emitted expression (which may have computed
//   in signed domain to satisfy Kotlin arithmetic) is wrapped with
//   `.toUByte()` / `.toUShort()` / `.toUInt()` / `.toULong()`.

fn emit_kotlin(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // Width-aware `round`/`floor`: same rule as Rust and Go — the declared
    // output type decides the integer, not a constant. See
    // `real_to_int_builtin`.
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if let Some(op) = real_to_int_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                let inner = emit_kotlin(&args[0], InferredType::Float { bits: 64 })?;
                let ctor = if signed {
                    kotlin_signed_ctor(bits)
                } else {
                    kotlin_unsigned_ctor(bits)
                };
                // ⚠ `floor` needs no sign branch the way `round` does:
                // `kotlin.math.floor` already goes toward −∞, and it is
                // `kotlin.math.round` alone that disagrees with the other five
                // backends.
                let call = match op {
                    RealToInt::Round => kotlin_round_half_away(inner),
                    RealToInt::Floor => format!("kotlin.math.floor({inner})"),
                };
                return Ok(format!("{call}.{ctor}()"));
            }
        }
    }
    // Push-down: see emit_rust for rationale.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_kotlin(left, expected)?;
            let r_raw = emit_kotlin(right, expected)?;
            let l = if child_needs_parens(left, *op, true, kotlin_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, kotlin_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", kotlin_binop(*op)));
        }
        // Kotlin's narrow unsigned types (UByte/UShort) do not support
        // bitwise/shift operations directly — `UByte shr Int` does not
        // resolve, and `UByte.toUByte` is a method reference, not a call.
        // Widen to signed Int32 for the operation — or to the joined type
        // when that is 64-bit, see `kotlin_narrow_unsigned_domain` — then
        // coerce back to the caller-requested type. The outer kotlin_coerce
        // handles the Int32 → UByte/UShort reverse conversion via `.toUByte()`.
        if op.is_bitwise() && (is_narrow_unsigned(left.ty) || is_narrow_unsigned(right.ty)) {
            let widened = kotlin_narrow_unsigned_domain(
                join_int(left.ty, right.ty),
                InferredType::Int {
                    signed: true,
                    bits: 32,
                },
            );
            // A shift count is an `Int` whatever is being shifted:
            // `Long.shl` takes `bitCount: Int`, so a 64-bit domain applies
            // to the left operand only.
            let count_ty = if matches!(op, BinOp::Shl | BinOp::Shr | BinOp::UShr) {
                InferredType::Int {
                    signed: true,
                    bits: 32,
                }
            } else {
                widened
            };
            let l_raw = emit_kotlin(left, widened)?;
            let r_raw = emit_kotlin(right, count_ty)?;
            let l = if child_needs_parens(left, *op, true, kotlin_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, kotlin_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            let inner = format!("{l} {} {r}", kotlin_binop(*op));
            return Ok(kotlin_coerce(inner, widened, expected, expr));
        }
        // Arithmetic on narrow unsigned in Kotlin: `UByte + UByte` is
        // defined as `this.toUInt() + other.toUInt(): UInt`, so storing
        // the result back into a `UByte`/`UShort` lvalue triggers an
        // assignment-type-mismatch in kotlinc. Mirror the bitwise path
        // above by widening to `UInt`, then coercing the result back to
        // the caller-requested narrow unsigned via the outer
        // `kotlin_coerce`'s `.toUByte()` / `.toUShort()` arm.
        if op.is_arith() && (is_narrow_unsigned(left.ty) || is_narrow_unsigned(right.ty)) {
            let widened = kotlin_narrow_unsigned_domain(
                join_arith(left.ty, right.ty),
                InferredType::Int {
                    signed: false,
                    bits: 32,
                },
            );
            let l_raw = emit_kotlin(left, widened)?;
            let r_raw = emit_kotlin(right, widened)?;
            let l = if child_needs_parens(left, *op, true, kotlin_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, kotlin_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            let inner = format!("{l} {} {r}", kotlin_binop(*op));
            return Ok(kotlin_coerce(inner, widened, expected, expr));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context — without this the inner
    // literal stays UntypedInt and the outer `kotlin_coerce` falls back to
    // `(-40).toDouble()`. Pushing the Float expectation into the operand
    // lets it pick up the `.0` literal-rewrite path and emit `-40.0`.
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_kotlin(operand, expected)?;
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                _ => unreachable!("guarded by outer match"),
            };
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            });
        }
    }
    let raw = kotlin_emit_node(expr)?;
    Ok(kotlin_coerce(raw, expr.ty, expected, expr))
}

/// The type Kotlin computes an operation in when one operand is UByte/UShort,
/// which Kotlin defines no arithmetic or bitwise operators on.
///
/// `narrow` is the 32-bit domain each path has always used — `UInt` for
/// arithmetic, `Int` for bitwise. It is right whenever the operands join to 32
/// bits or fewer: two's complement wraps the 32-bit result back to the value
/// the joined type would have held. It is wrong when the other operand is
/// 64-bit, because `.toUInt()` / `.toInt()` on a `Long` discards the upper
/// half and the sign before the operation runs (`-5L + 3u` gave 4294967294,
/// `(1L shl 32) or 1` gave 1). A 64-bit join is therefore computed in the
/// joined type itself — `Long` or `ULong` — which Kotlin does define every
/// operator on.
fn kotlin_narrow_unsigned_domain(joined: InferredType, narrow: InferredType) -> InferredType {
    match joined {
        InferredType::Int { bits: 64, .. } => joined,
        _ => narrow,
    }
}

/// A narrow unsigned integer — UByte (8) or UShort (16). Kotlin stdlib does
/// not define bitwise/shift ops on these; widen to signed Int32 for ops.
fn is_narrow_unsigned(ty: InferredType) -> bool {
    matches!(
        ty,
        InferredType::Int {
            signed: false,
            bits: 8 | 16
        }
    )
}

fn kotlin_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        // `"ack".toByteArray()` is byte-identical to the ASCII literal
        // (UTF-8 default). Used as the `contentEquals` argument in the
        // bytes-equality Binary branch below. RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => {
            format!("\"{}\".toByteArray()", bytes_as_quoted_ascii(bytes))
        }
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "null".to_string(),
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            // Bytes equality: Kotlin `==` on `ByteArray` is reference
            // equality, so content comparison must use `contentEquals`.
            // RFC §bytesguard-3 B3/B5 — only `===`/`!==` reach here for bytes.
            if matches!(op, BinOp::StrictEq | BinOp::StrictNeq)
                && (matches!(left.ty, InferredType::Bytes)
                    || matches!(right.ty, InferredType::Bytes))
            {
                let l = emit_kotlin(left, InferredType::Unknown)?;
                let r = emit_kotlin(right, InferredType::Unknown)?;
                let call = format!("{}.contentEquals({r})", wrap_postfix(left, l));
                return Ok(if matches!(op, BinOp::StrictNeq) {
                    format!("!{call}")
                } else {
                    call
                });
            }
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_kotlin(left, operand_ty)?;
            let r_raw = emit_kotlin(right, operand_ty)?;
            let l = if child_needs_parens(left, *op, true, kotlin_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, kotlin_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            format!("{l} {} {r}", kotlin_binop(*op))
        }
        ExprKind::Unary {
            op: UnaryOp::BitNot,
            operand,
        } => {
            format!(
                "{}.inv()",
                wrap_postfix(operand, emit_kotlin(operand, expr.ty)?)
            )
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_kotlin(operand, expr.ty)?;
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                UnaryOp::Not => "!",
                UnaryOp::BitNot => unreachable!(),
            };
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            format!(
                "if ({}) {} else {}",
                emit_kotlin(condition, InferredType::Bool)?,
                emit_kotlin(consequent, expr.ty)?,
                emit_kotlin(alternate, expr.ty)?,
            )
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_kotlin(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            // Kotlin's `Array.get(index: Int)` and the unboxed
            // `XArray.get(Int)` overloads only accept `Int` for the
            // index. Concrete-typed indices (`<sce:var type="u16">`)
            // need explicit `.toInt()` or kotlinc rejects with
            // "argument type mismatch". Mirrors the Rust emitter's
            // `as usize` insertion for non-`UntypedInt` indices.
            let idx_raw = emit_kotlin(index, InferredType::Unknown)?;
            let idx_emit = match index.ty {
                InferredType::Int {
                    signed: true,
                    bits: 32,
                } => idx_raw,
                // Parenthesize before `.toInt()`: a compound index
                // (`data[i + run]`) emits as `i.toUInt() + run.toUInt()`, so a
                // bare `.toInt()` would bind to the trailing operand only
                // (`... + run.toUInt().toInt()` = `UInt + Int`, a type error).
                // The wrap is harmless on a single-token index.
                InferredType::Int { .. } => format!("({idx_raw}).toInt()"),
                _ => idx_raw,
            };
            // A `bytes` operand is a `ByteArray`; `[i]` yields a signed
            // `Byte`, so normalize to `UByte` exactly as the foreach arm
            // does. Item C7 wildcard-keyexpr lowering.
            let norm = if matches!(object.ty, InferredType::Bytes) {
                ".toUByte()"
            } else {
                ""
            };
            format!(
                "{}[{idx_emit}]{norm}",
                wrap_postfix(object, emit_kotlin(object, InferredType::Unknown)?),
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "({}).size",
                    emit_kotlin(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // ⚠ NOT `kotlin.math.round`, which is half to EVEN. `floor(x
                // + 0.5)` is half away from zero for non-negative input and
                // `ceil(x - 0.5)` for negative, so the sign is branched on
                // rather than inherited — the other five backends round
                // 0.5 → 1 and −0.5 → −1, and this must agree.
                //
                // ⚠ `kotlin.math.floor` needs no such treatment: it goes
                // toward −∞ like every other backend's floor, so only `round`
                // is the odd one out here.
                //
                // ⚠⚠ `.toLong()` here is the WIDTH-FREE fallback. When the
                // context declares an integer width, `emit_kotlin` intercepts
                // this call above and converts to that type instead; without
                // that arm a `sce:type="int32"` output returned `Long` from an
                // `Int` function, which Kotlin refuses.
                let inner = emit_kotlin(&args[0], InferredType::Float { bits: 64 })?;
                let call = match op {
                    RealToInt::Round => kotlin_round_half_away(inner),
                    RealToInt::Floor => format!("kotlin.math.floor({inner})"),
                };
                return Ok(format!("{call}.toLong()"));
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_kotlin(arg, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_kotlin(callee, InferredType::Unknown)?),
                a.join(", "),
            )
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field → `ByteArray`
            // (the algorithm `bytes` param type), UTF-8 encoded.
            format!(
                "{}.toByteArray(Charsets.UTF_8)",
                emit_kotlin(source, InferredType::Unknown)?
            )
        }
    })
}

fn kotlin_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "and",
        BinOp::BitOr => "or",
        BinOp::BitXor => "xor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        BinOp::UShr => "ushr",
    }
}

/// Apply language-specific coercion from a child's natural type to the
/// expected parent type.
fn kotlin_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    // Kotlin has no bare literal past `Long`: the unsigned value takes the
    // `uL` suffix, and `Long`'s minimum is its named constant.
    if matches!(to.strip_quantity(), Int { .. }) {
        match wide_literal(node) {
            Some(WideLiteral::Unsigned) => return format!("{raw}uL"),
            Some(WideLiteral::Int64Min) => return "Long.MIN_VALUE".to_string(),
            None => {}
        }
    }
    if from == to || matches!(to, Unknown) {
        return raw;
    }
    match (from, to) {
        // Literal promotion for untyped integers into float context.
        (UntypedInt, Float { .. }) | (UntypedInt, UntypedFloat) => {
            if let ExprKind::NumberLit(text) = &node.kind {
                if is_decimal_integer_literal(text) {
                    return format!("{raw}.0");
                }
            }
            // Computed subtree or hex/bin/oct literal — explicit cast.
            format!("({raw}).toDouble()")
        }
        // Concrete int → float: explicit `.toDouble()` / `.toFloat()`.
        (Int { .. }, Float { bits: 64 }) => wrap_dotcall(raw, node, "toDouble"),
        (Int { .. }, Float { bits: 32 }) => wrap_dotcall(raw, node, "toFloat"),
        (
            UntypedInt,
            Int {
                signed: false,
                bits,
            },
        ) => {
            // Literal adopting unsigned concrete type.
            let suffix = kotlin_unsigned_ctor(bits);
            format!("{raw}.{suffix}()")
        }
        (UntypedInt, Int { signed: true, .. }) => raw,
        // Unsigned → signed: required for mixed-sign arithmetic in Kotlin.
        (
            Int {
                signed: false,
                bits: bfrom,
            },
            Int {
                signed: true,
                bits: bto,
            },
        ) => {
            let ctor = kotlin_signed_ctor(bfrom.max(bto));
            wrap_dotcall(raw, node, ctor)
        }
        // Signed → unsigned: result suffix for unsigned outputs.
        (
            Int { signed: true, .. },
            Int {
                signed: false,
                bits: bto,
            },
        ) => {
            let ctor = kotlin_unsigned_ctor(bto);
            wrap_dotcall(raw, node, ctor)
        }
        // Int widening among same signedness.
        (
            Int {
                signed: s1,
                bits: b1,
            },
            Int {
                signed: s2,
                bits: b2,
            },
        ) if s1 == s2 && b1 != b2 => {
            let ctor = if s1 {
                kotlin_signed_ctor(b2)
            } else {
                kotlin_unsigned_ctor(b2)
            };
            wrap_dotcall(raw, node, ctor)
        }
        // Float widening.
        (Float { bits: 32 }, Float { bits: 64 }) => wrap_dotcall(raw, node, "toDouble"),
        (Float { bits: 64 }, Float { bits: 32 }) => wrap_dotcall(raw, node, "toFloat"),
        // Untyped float → concrete float: leave text alone (compiler accepts).
        (UntypedFloat, Float { .. }) => raw,
        // Unknown on either side → no coercion.
        (Unknown, _) | (_, Unknown) => raw,
        // Anything else — no coercion (would be a semantic error in a textbook
        // implementation, but we follow C++ precedent of emitting verbatim to
        // let the target compiler reject it with its own diagnostic).
        _ => raw,
    }
}

/// Kotlin's half-away-from-zero rounding, as a `Double`-valued expression.
///
/// Factored out because two call sites need the same arithmetic and only
/// differ in what they convert the result to — the width-aware arm in
/// `emit_kotlin` and the width-free fallback in `kotlin_emit_node`. Writing it
/// twice is how the two would drift apart at `.5`, which is the one input that
/// distinguishes this from the language's own `round`.
fn kotlin_round_half_away(inner: String) -> String {
    format!(
        "(({inner}).let {{ v -> if (v < 0.0) kotlin.math.ceil(v - 0.5) \
         else kotlin.math.floor(v + 0.5) }})"
    )
}

fn kotlin_signed_ctor(bits: u8) -> &'static str {
    match bits {
        8 => "toByte",
        16 => "toShort",
        32 => "toInt",
        64 => "toLong",
        _ => "toInt",
    }
}

fn kotlin_unsigned_ctor(bits: u8) -> &'static str {
    match bits {
        8 => "toUByte",
        16 => "toUShort",
        32 => "toUInt",
        64 => "toULong",
        _ => "toUInt",
    }
}

/// Wrap a raw emitted string with `.call()` suffix, adding parens around the
/// target if its AST shape would otherwise bind the call incorrectly.
fn wrap_dotcall(raw: String, node: &TypedExpr, method: &str) -> String {
    if matches!(
        &node.kind,
        ExprKind::Binary { .. } | ExprKind::Conditional { .. } | ExprKind::Unary { .. }
    ) {
        format!("({raw}).{method}()")
    } else {
        format!("{raw}.{method}()")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — Rust
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Rust rules:
//
// * Bare identifiers are normalized to `snake_case` to match Rust naming
//   convention. Member property names are emitted verbatim.
// * Integer literal promotion to float: append `.0` for decimal literals.
// * Concrete integer ident → float: `ident as f64` (or f32).
// * Concrete float{32} ↔ float{64}: `x as f64` / `x as f32`.
// * Integer widening is left implicit (Rust rejects mismatched integer
//   widths at the type system level; the generator is expected to pass a
//   well-typed context to begin with). For robustness we still emit
//   `expr as iN` when widening is necessary.
// * Hex/bin/oct literal in float context: hard error.

fn emit_rust(expr: &TypedExpr, expected: InferredType) -> Result<String, Refusal> {
    // Push-down: for arithmetic binary ops with a concrete-float expected
    // type, emit operands at the expected type directly. Without this, a
    // mixed expression like `raw * 0.1` (raw: UInt16, expected: Float64)
    // infers operand_ty = Float32 from join_arith, producing
    // `(raw as f32 * 0.1) as f64` instead of `raw as f64 * 0.1`.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_rust(left, expected)?;
            let r_raw = emit_rust(right, expected)?;
            let l = if child_needs_parens(left, *op, true, rust_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, rust_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", rust_binop(*op)));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context. Without this, `-40` in a
    // `> -40` comparison falls into `rust_emit_node`'s Unary arm which
    // recurses with `expr.ty` (the unary's UntypedInt) instead of the
    // caller's float `expected`, so the inner literal stays `40` and
    // `rust_coerce` then takes the "Computed subtree" branch on the outer
    // Unary, emitting the ugly `-40 as f64` instead of the symmetric
    // `-40.0`. Mirrors the Binary push-down above.
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_rust(operand, expected)?;
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "",
                _ => unreachable!("guarded by outer match"),
            };
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            });
        }
    }
    // Push-down: `len(x)` lowers to Rust's `.len()`, which is always
    // `usize`. In a concrete-integer context (e.g. `i < len(a)` where
    // `i: u32`, or `len(a) !== len(b)` unified to a sized int), emit the
    // width-coercing `as` so the comparison typechecks. `rust_emit_node`
    // is width-blind and `rust_coerce` trusts the node's inferred type —
    // which the always-`usize` `.len()` does not actually carry, so the
    // cast must be injected here where the `expected` width is known.
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if is_len_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                let inner = emit_rust(&args[0], InferredType::Unknown)?;
                return Ok(format!(
                    "({inner}).len() as {}",
                    rust_int_type(signed, bits)
                ));
            }
        }
        // ⚠ THE WIDTH COMES FROM THE CONTEXT, NOT FROM A CONSTANT. The first
        // version of `round` emitted `as i64` everywhere, and a `transform`
        // declaring `sce:type="int32"` then returned `i64` from an `i32`
        // function — a compile error in Rust and Go, a silent narrowing in
        // C++. `len` already solved this by widening to the EXPECTED integer
        // type; `round` does the same.
        if let Some(op) = real_to_int_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                let inner = emit_rust(&args[0], InferredType::Float { bits: 64 })?;
                let m = match op {
                    RealToInt::Round => "round",
                    RealToInt::Floor => "floor",
                };
                return Ok(format!(
                    "({inner}).{m}() as {}",
                    rust_int_type(signed, bits)
                ));
            }
        }
    }
    let raw = rust_emit_node(expr)?;
    rust_coerce(raw, expr.ty, expected, expr)
}

/// A `string` value a Rust function returns, in the `String` its signature
/// declares.
///
/// Inside an expression a string is borrowed: a parameter is a `&str`, a
/// literal a `&'static str`. Returned as they are, from a function declared
/// `-> String`, neither compiles — measured 2026-09-22, every transform with
/// a `string` output failed to build on Rust. So the value is made owned
/// where it is borrowed, leaf by leaf:
///
/// * a conditional lowers each arm on its own, because both arms of a Rust
///   `if` must be ONE type, and an arm that is already owned next to one
///   that is not would still disagree if only the whole were converted;
/// * a call is already owned — a sibling output's own function (the rename
///   pass has lowered it to a `Raw` call by now) returns `String`, as does
///   an imported transform's;
/// * anything else is borrowed, and takes `.to_string()`.
fn emit_rust_returned_str(expr: &TypedExpr) -> Result<String, Refusal> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(format!(
            "if {} {{ {} }} else {{ {} }}",
            emit_rust(condition, InferredType::Bool)?,
            emit_rust_returned_str(consequent)?,
            emit_rust_returned_str(alternate)?,
        )),
        ExprKind::Call { .. } | ExprKind::Raw(_) => emit_rust(expr, InferredType::Str),
        _ => {
            let borrowed = emit_rust(expr, InferredType::Str)?;
            Ok(format!("{}.to_string()", wrap_postfix(expr, borrowed)))
        }
    }
}

fn rust_emit_node(expr: &TypedExpr) -> Result<String, Refusal> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        // `Vec<u8> == &[u8; N]` (and `== Vec<u8>`) compare length+content,
        // so the default `==`/`!=` Binary path needs no special-casing —
        // only this constant rendering. ASCII-only by contract so `b"…"`
        // is valid.
        ExprKind::BytesLit { bytes } => format!("b\"{}\"", bytes_as_quoted_ascii(bytes)),
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "None".to_string(),
        ExprKind::Ident(s) => crate::filters::to_snake_case(s.clone()),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_rust(left, operand_ty)?;
            let r_raw = emit_rust(right, operand_ty)?;
            let l = if child_needs_parens(left, *op, true, rust_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, rust_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            format!("{l} {} {r}", rust_binop(*op))
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_rust(operand, expr.ty)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "",
                UnaryOp::Not | UnaryOp::BitNot => "!",
            };
            if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            format!(
                "if {} {{ {} }} else {{ {} }}",
                emit_rust(condition, InferredType::Bool)?,
                emit_rust(consequent, expr.ty)?,
                emit_rust(alternate, expr.ty)?,
            )
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_rust(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            // Rust slice/array indexing requires `usize`. Concrete
            // integer-typed indices (declared via `<sce:var name=...
            // type="u16">`, parameters, member fields) need an
            // explicit `as usize` cast or rustc's strict typecheck
            // refuses the access. `UntypedInt` literals (e.g.
            // `arr[0]`) are accepted by rustc directly and stay
            // un-wrapped to keep prior goldens byte-stable.
            let idx_raw = emit_rust(index, InferredType::Unknown)?;
            let idx_emit = match index.ty {
                InferredType::Int { .. } => format!("({idx_raw}) as usize"),
                _ => idx_raw,
            };
            format!(
                "{}[{idx_emit}]",
                wrap_postfix(object, emit_rust(object, InferredType::Unknown)?),
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "({}).len()",
                    emit_rust(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // `f64::round` is already half away from zero, and `f64::floor`
                // already goes toward −∞.
                let inner = emit_rust(&args[0], InferredType::Float { bits: 64 })?;
                let m = match op {
                    RealToInt::Round => "round",
                    RealToInt::Floor => "floor",
                };
                return Ok(format!("(({inner}).{m}() as i64)"));
            }
            let mut emitted_args = Vec::with_capacity(args.len());
            for (i, a) in args.iter().enumerate() {
                emitted_args.push(emit_rust(a, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_rust(callee, InferredType::Unknown)?),
                emitted_args.join(", "),
            )
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`&str`) → the
            // algorithm `bytes` param type (`&[u8]`) via `.as_bytes()`.
            format!("{}.as_bytes()", emit_rust(source, InferredType::Unknown)?)
        }
    })
}

fn rust_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>",
    }
}

fn rust_coerce(
    raw: String,
    from: InferredType,
    to: InferredType,
    node: &TypedExpr,
) -> Result<String, Refusal> {
    use InferredType::*;
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return Ok(raw);
    }
    match (from, to) {
        // Literal promotion for untyped decimal integers into float context.
        (UntypedInt, Float { .. }) | (UntypedInt, UntypedFloat) => {
            if let ExprKind::NumberLit(text) = &node.kind {
                if is_decimal_integer_literal(text) {
                    return Ok(format!("{raw}.0"));
                }
                // Hex/bin/oct literal in float context — textbook strict: error.
                return Err(ExprError::TypeCoercion {
                    lang: "Rust",
                    detail: format!(
                        "cannot coerce integer literal '{text}' to float: \
                         hex/binary/octal literals are not promotable. \
                         Use a decimal float literal (e.g. `1.0`, `255.0`) instead."
                    ),
                    observed: Some(text.clone()),
                }
                .at(node.span.clone()));
            }
            // Computed subtree: explicit cast.
            let target = match to {
                Float { bits: 32 } => "f32",
                _ => "f64",
            };
            Ok(format!("{raw} as {target}"))
        }
        // Concrete int → float: explicit cast.
        (Int { .. }, Float { bits: 64 }) => Ok(rust_cast(raw, node, "f64")),
        (Int { .. }, Float { bits: 32 }) => Ok(rust_cast(raw, node, "f32")),
        // Untyped int adopts concrete int — no cast needed at source level.
        (UntypedInt, Int { .. }) => Ok(raw),
        // Untyped float → concrete float: emit as-is; Rust infers.
        (UntypedFloat, Float { .. }) => Ok(raw),
        // Concrete int widening.
        (
            Int {
                signed: s1,
                bits: b1,
            },
            Int {
                signed: s2,
                bits: b2,
            },
        ) if (s1, b1) != (s2, b2) => {
            let target = rust_int_type(s2, b2);
            Ok(rust_cast(raw, node, target))
        }
        // Float widening or narrowing.
        (Float { bits: b1 }, Float { bits: b2 }) if b1 != b2 => {
            let target = if b2 == 64 { "f64" } else { "f32" };
            Ok(rust_cast(raw, node, target))
        }
        _ => Ok(raw),
    }
}

fn rust_cast(raw: String, node: &TypedExpr, target: &str) -> String {
    if matches!(
        &node.kind,
        ExprKind::Binary { .. } | ExprKind::Conditional { .. } | ExprKind::Unary { .. }
    ) {
        format!("({raw}) as {target}")
    } else {
        format!("{raw} as {target}")
    }
}

fn rust_int_type(signed: bool, bits: u8) -> &'static str {
    match (signed, bits) {
        (true, 8) => "i8",
        (true, 16) => "i16",
        (true, 32) => "i32",
        (true, 64) => "i64",
        (false, 8) => "u8",
        (false, 16) => "u16",
        (false, 32) => "u32",
        (false, 64) => "u64",
        _ => "i64",
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — Go
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Go has untyped constants: `9`, `5`, `32` used in a `float64` expression
// auto-convert at compile time. So Go's emitter leaves untyped literals
// alone — no `.0` suffix needed.
//
// Concrete integer variables DO need explicit conversion: `float64(raw)`.
// Integer widening: `int64(x)`. Go has no conditional expression; see
// `go_conditional` for how one is lowered.

fn emit_go(expr: &TypedExpr, expected: InferredType) -> Result<String, Refusal> {
    if let ExprKind::Conditional {
        condition,
        consequent,
        alternate,
    } = &expr.kind
    {
        return go_conditional(expr, condition, consequent, alternate, expected);
    }
    // Push-down: see emit_rust for rationale.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_go(left, expected)?;
            let r_raw = emit_go(right, expected)?;
            let l = if child_needs_parens(left, *op, true, go_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, go_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", go_binop(*op)));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context — propagate Float so
    // go_coerce appends `.0` to the inner literal (e.g. `-40` → `-40.0`).
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_go(operand, expected)?;
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                _ => unreachable!("guarded by outer match"),
            };
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            });
        }
    }
    // Push-down: `len(x)` lowers to Go's `len(...)`, which returns a concrete
    // `int` — NOT an untyped constant, so there is no implicit int→uintN
    // conversion (`i < len(a)` with `i: uint32` is a Go type error). In a
    // sized-integer context wrap the builtin in the target type. `go_coerce`
    // cannot do this: it types `len(x)` as `UntypedInt` (the infer lattice's
    // always-wider stand-in) and short-circuits `UntypedInt → _` to a no-op
    // on the untyped-constant assumption. Mirrors the Rust `.len() as uN`
    // push-down (item C7 wildcard-keyexpr lowering).
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if is_len_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                let inner = emit_go(&args[0], InferredType::Unknown)?;
                return Ok(format!("{}(len({inner}))", go_int_type(signed, bits)));
            }
        }
        // Same width rule as Rust above — the context decides, not a constant.
        if let Some(op) = real_to_int_builtin(callee, args) {
            if let InferredType::Int { signed, bits } = expected {
                let inner = emit_go(&args[0], InferredType::Float { bits: 64 })?;
                let f = match op {
                    RealToInt::Round => "math.Round",
                    RealToInt::Floor => "math.Floor",
                };
                return Ok(format!("{}({f}({inner}))", go_int_type(signed, bits)));
            }
        }
    }
    let raw = go_emit_node(expr)?;
    Ok(go_coerce(raw, expr.ty, expected, expr))
}

fn go_emit_node(expr: &TypedExpr) -> Result<String, Refusal> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        // Standalone fallback only — the bytes-equality Binary branch
        // renders operands itself (as `string(x)` / a string literal) so
        // it needs no `bytes` import. RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => format!("[]byte{{{}}}", bytes_as_hex_list(bytes)),
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "nil".to_string(),
        ExprKind::Ident(s) => s.clone(),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            // Bytes equality: Go forbids `==` on `[]byte` slices. Compare
            // via `string(slice)` conversion (no `bytes` import needed);
            // a `BytesLit` operand renders directly as a Go string literal.
            // RFC §bytesguard-3 B3/B5 — only `===`/`!==` reach here for bytes.
            if matches!(op, BinOp::StrictEq | BinOp::StrictNeq)
                && (matches!(left.ty, InferredType::Bytes)
                    || matches!(right.ty, InferredType::Bytes))
            {
                let render = |operand: &TypedExpr| -> Result<String, Refusal> {
                    if let ExprKind::BytesLit { bytes } = &operand.kind {
                        Ok(format!("\"{}\"", bytes_as_quoted_ascii(bytes)))
                    } else {
                        Ok(format!(
                            "string({})",
                            emit_go(operand, InferredType::Unknown)?
                        ))
                    }
                };
                let l = render(left)?;
                let r = render(right)?;
                return Ok(format!("{l} {} {r}", go_binop(*op)));
            }
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_go(left, operand_ty)?;
            let r_raw = emit_go(right, operand_ty)?;
            let l = if child_needs_parens(left, *op, true, go_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, go_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            format!("{l} {} {r}", go_binop(*op))
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_go(operand, expr.ty)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                UnaryOp::Not => "!",
                UnaryOp::BitNot => "^",
            };
            if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            }
        }
        ExprKind::Conditional { .. } => {
            unreachable!("emit_go lowers Conditional before go_emit_node")
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_go(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            format!(
                "{}[{}]",
                wrap_postfix(object, emit_go(object, InferredType::Unknown)?),
                emit_go(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "len({})",
                    emit_go(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // `math.Round` is already half away from zero, and `math.Floor`
                // already goes toward −∞.
                let inner = emit_go(&args[0], InferredType::Float { bits: 64 })?;
                let f = match op {
                    RealToInt::Round => "math.Round",
                    RealToInt::Floor => "math.Floor",
                };
                return Ok(format!("int64({f}({inner}))"));
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_go(arg, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_go(callee, InferredType::Unknown)?),
                a.join(", "),
            )
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`string`) →
            // the algorithm `bytes` param type (`[]byte`).
            format!("[]byte({})", emit_go(source, InferredType::Unknown)?)
        }
    })
}

fn go_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>",
    }
}

fn go_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    match (from, to) {
        // Promote untyped decimal integer literals in float context to prevent
        // Go integer division: `9 / 5` → `9.0 / 5.0`.
        (UntypedInt, Float { .. }) => {
            if let ExprKind::NumberLit(text) = &node.kind {
                if is_decimal_integer_literal(text) {
                    return format!("{raw}.0");
                }
            }
            raw
        }
        // Other untyped literals — Go's untyped constants auto-convert; no-op.
        (UntypedInt, _) | (UntypedFloat, _) => raw,
        // Concrete int → float: explicit `float64(x)` / `float32(x)`.
        (Int { .. }, Float { bits: 64 }) => format!("float64({raw})"),
        (Int { .. }, Float { bits: 32 }) => format!("float32({raw})"),
        // Float widening.
        (Float { bits: 32 }, Float { bits: 64 }) => format!("float64({raw})"),
        (Float { bits: 64 }, Float { bits: 32 }) => format!("float32({raw})"),
        // Integer conversions.
        (
            Int {
                signed: s1,
                bits: b1,
            },
            Int {
                signed: s2,
                bits: b2,
            },
        ) if (s1, b1) != (s2, b2) => {
            let target = go_int_type(s2, b2);
            format!("{target}({raw})")
        }
        _ => raw,
    }
}

fn go_int_type(signed: bool, bits: u8) -> &'static str {
    match (signed, bits) {
        (true, 8) => "int8",
        (true, 16) => "int16",
        (true, 32) => "int32",
        (true, 64) => "int64",
        (false, 8) => "uint8",
        (false, 16) => "uint16",
        (false, 32) => "uint32",
        (false, 64) => "uint64",
        _ => "int64",
    }
}

/// Lower `c ? a : b` to an immediately invoked function literal:
/// `func() T { if c { return a }; return b }()`.
///
/// Go has no conditional expression, and every other spelling is wrong in a
/// way that matters. A generic `pick(c, a, b)` helper evaluates BOTH branches
/// before choosing, so `d !== 0 ? n / d : 0` panics on the branch the document
/// said not to take; ECMAScript evaluates only the chosen one, and so do the
/// other five backends. Hoisting into statements needs a statement position,
/// which `transform`, `condition` and `lookup` documents do not have — the
/// advice this emitter used to give ("restructure with if/else") could not be
/// followed in them at all.
///
/// A function literal needs its result type spelled out, which is the one
/// thing an untyped expression cannot supply. The type the value flows into
/// wins; otherwise the conditional's own inferred type; when neither is a
/// type Go can name — an opaque member access, or two untyped literals flowing
/// into an untyped slot — the expression is refused at the conditional's own
/// span, and the refusal says why.
fn go_conditional(
    conditional: &TypedExpr,
    condition: &TypedExpr,
    consequent: &TypedExpr,
    alternate: &TypedExpr,
    expected: InferredType,
) -> Result<String, Refusal> {
    let (result, type_name) = [expected, conditional.ty]
        .into_iter()
        .find_map(|ty| go_nameable_type(ty).map(|name| (ty, name)))
        .ok_or_else(|| ExprError::GoTernary.at(conditional.span.clone()))?;
    Ok(format!(
        "func() {type_name} {{ if {} {{ return {} }}; return {} }}()",
        emit_go(condition, InferredType::Bool)?,
        emit_go(consequent, result)?,
        emit_go(alternate, result)?,
    ))
}

/// The Go spelling of a type a function literal can return, or `None` for a
/// type that has no single spelling here (untyped literals, opaque values).
fn go_nameable_type(ty: InferredType) -> Option<&'static str> {
    match ty {
        InferredType::Int { signed, bits } => Some(go_int_type(signed, bits)),
        InferredType::Float { bits: 32 } => Some("float32"),
        InferredType::Float { .. } => Some("float64"),
        InferredType::Bool => Some("bool"),
        InferredType::Str => Some("string"),
        InferredType::Bytes => Some("[]byte"),
        _ => None,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — Python
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// Python duck-types numeric values: `int * 0.1` auto-promotes, no explicit
// cast is ever required. The emitter's responsibilities are:
//
// * Rename identifiers to `snake_case` (Python convention).
// * Translate operators (`===` → `==`, `&&` → `and`, `||` → `or`, etc.).
// * Translate literals (`true`/`false` → `True`/`False`, `null` → `None`).
// * Translate ternary: `X ? Y : Z` → `Y if X else Z`.
// * Single-quoted string literals (PEP 8 idiomatic style).
// * No float-literal promotion needed: Python's `/` is always true division
//   (PEP 238), so `9 / 5` yields 1.8 without an explicit `.0` suffix.

fn emit_python(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // Push-down: propagate Float expectation into arithmetic sub-trees.
    // Python's `/` is true division so literal promotion is not required,
    // but the push-down is kept for structural symmetry with the other four
    // emitters (and python_coerce is a no-op for UntypedInt→Float).
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_python(left, expected)?;
            let r_raw = emit_python(right, expected)?;
            let l = if child_needs_parens(left, *op, true, python_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, python_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", python_binop(*op)));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context — structural symmetry
    // with the other four emitters.  python_coerce is a no-op for
    // UntypedInt→Float, so `-40` stays as `-40` (Python's dynamic typing
    // and true division make explicit promotion unnecessary).
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_python(operand, expected)?;
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                _ => unreachable!("guarded by outer match"),
            };
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            });
        }
    }
    let raw = python_emit_node(expr)?;
    Ok(python_coerce(raw, expr.ty, expected))
}

fn python_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("'{value}'"),
        // `bytes == bytes` compares content, so the default `==`/`!=`
        // path needs no special-casing — only this constant rendering.
        // A `b"…"` literal (double-quoted so a `'` byte needs no escape).
        // RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => format!("b\"{}\"", bytes_as_quoted_ascii(bytes)),
        ExprKind::BoolLit(b) => if *b { "True" } else { "False" }.to_string(),
        ExprKind::NullLit => "None".to_string(),
        ExprKind::Ident(s) => crate::filters::to_snake_case(s.clone()),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_python(left, operand_ty)?;
            let r_raw = emit_python(right, operand_ty)?;
            if let Some(lowered) = python_integer_div_rem(*op, operand_ty, &l_raw, &r_raw) {
                return Ok(lowered);
            }
            let l = if child_needs_parens(left, *op, true, python_precedence) {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, python_precedence) {
                format!("({r_raw})")
            } else {
                r_raw
            };
            let raw = format!("{l} {} {r}", python_binop(*op));
            // RFC §synth-5-A: Python's `int` is arbitrary-precision, so an
            // operation that would truncate on a fixed-width unsigned
            // type in C/Rust (e.g. `crc << 1` where crc is u16, then
            // `^ 0x1021`) keeps growing. To preserve byte-equivalence
            // with the other five backends, mask the result back to
            // the declared bit-width — but only for ops that can
            // produce values exceeding the operand range:
            //   * `+ - *` carry a value beyond the inputs' max.
            //   * `<<` shifts bits past the high end.
            // `^ | &` stay within `max(left, right)`; comparison /
            // logic produce booleans; division shrinks. Skipping
            // those ops avoids redundant masks like `byte & 0x0F &
            // 0xFF` on a body whose authors already wrote a
            // bit-mask, keeping the Python emit close to the
            // hand-authored shape on existing fixtures.
            let needs_mask = matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl);
            if needs_mask {
                if let InferredType::Int {
                    signed: false,
                    bits,
                } = expr.ty
                {
                    if bits <= 32 {
                        let mask: u64 = (1u64 << bits) - 1;
                        return Ok(format!("({raw}) & 0x{mask:X}"));
                    }
                }
            }
            raw
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_python(operand, expr.ty)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                UnaryOp::Not => "not ",
                UnaryOp::BitNot => "~",
            };
            if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let cons = emit_python(consequent, expr.ty)?;
            let cons = if matches!(&consequent.kind, ExprKind::Conditional { .. }) {
                format!("({cons})")
            } else {
                cons
            };
            format!(
                "{cons} if {} else {}",
                emit_python(condition, InferredType::Bool)?,
                emit_python(alternate, expr.ty)?,
            )
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_python(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            format!(
                "{}[{}]",
                wrap_postfix(object, emit_python(object, InferredType::Unknown)?),
                emit_python(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "len({})",
                    emit_python(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // ⚠ NOT the builtin `round`, which is banker's rounding:
                // `round(0.5)` is 0 in Python and 1 everywhere else. `floor(x
                // + 0.5)` / `ceil(x - 0.5)` by sign gives half away from zero,
                // matching the other five backends.
                //
                // ⚠ `math.floor` needs no such rewrite — it goes toward −∞ and
                // returns an `int`, so only `round` is the odd one out.
                let inner = emit_python(&args[0], InferredType::Float { bits: 64 })?;
                return Ok(match op {
                    RealToInt::Round => format!(
                        "int(__import__('math').floor(({inner}) + 0.5) \
                         if ({inner}) >= 0 else __import__('math').ceil(({inner}) - 0.5))"
                    ),
                    RealToInt::Floor => format!("__import__('math').floor({inner})"),
                });
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_python(arg, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_python(callee, InferredType::Unknown)?),
                a.join(", "),
            )
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`str`) → the
            // algorithm `bytes` param type (`bytes`), UTF-8 encoded.
            format!(
                "{}.encode(\"utf-8\")",
                emit_python(source, InferredType::Unknown)?
            )
        }
    })
}

/// Lower `/` and `%` between two integer operands to the truncating pair the
/// other five backends compute (SCE_FORGE.md §3.4.1).
///
/// ⚠ Neither Python operator is that pair. `/` is true division (PEP 238), so
/// an `int` output received a float; `//` and `%` both round toward −∞, so
/// they agree with truncation only while the operands share a sign — `-7 // 2`
/// is −4 where C, C++, Rust, Go and Kotlin give −3, and `-7 % 3` is 2 where
/// they give −1. Replacing `/` with `//` would have traded one wrong answer
/// for another on exactly the negative inputs a date calculation reaches.
///
/// The operands are bound once through a lambda rather than spliced in twice:
/// a spliced form re-evaluates the dividend in each sign branch, and a nested
/// division multiplies that at every level. `int(a / b)` is not an option —
/// it rounds through a float and is wrong past 2**53, inside the int64 range.
///
/// Only called outside a float context: [`emit_python`] returns before
/// reaching here when the expected type is `Float`, which is where Python's
/// true division is the right answer.
fn python_integer_div_rem(op: BinOp, operand_ty: InferredType, l: &str, r: &str) -> Option<String> {
    if !matches!(
        operand_ty,
        InferredType::Int { .. } | InferredType::UntypedInt
    ) {
        return None;
    }
    match op {
        BinOp::Div => Some(format!(
            "(lambda n, d: n // d if (n >= 0) == (d > 0) else -(-n // d))({l}, {r})"
        )),
        BinOp::Mod => Some(format!(
            "(lambda n, d: n % abs(d) if n >= 0 else -(-n % abs(d)))({l}, {r})"
        )),
        _ => None,
    }
}

fn python_coerce(raw: String, from: InferredType, to: InferredType) -> String {
    use InferredType::*;
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    if let (
        Int {
            signed: from_signed,
            bits: from_bits,
        },
        Int { signed, bits },
    ) = (from.strip_quantity(), to.strip_quantity())
    {
        return python_wrap_int(raw, (from_signed, from_bits), (signed, bits));
    }
    // Python's `/` is true division (PEP 238) and its dynamic typing
    // handles int→float implicitly.  No `.0` suffix needed — unlike
    // C++/Go/Kotlin/Rust where integer division would produce wrong results.
    raw
}

/// `raw`, an integer of type `from`, as the integer type `to` holds it: its
/// value modulo 2^bits, read in `to`'s sign — what Rust's `as`, Go's and
/// C's conversions and Kotlin's `toUByte()`/`toInt()` compute. Python's
/// `int` has no width, so the conversion is spelled out; a `to` that holds
/// every value of `from` leaves `raw` alone.
///
/// ⚠ Python kept the whole value until 2026-09-24: a `uint32` of 300
/// assigned to a `uint8` stayed 300 where the other five backends gave 44.
fn python_wrap_int(raw: String, from: (bool, u8), to: (bool, u8)) -> String {
    let ((from_signed, from_bits), (signed, bits)) = (from, to);
    let holds_every_value = match (from_signed, signed) {
        (false, false) | (true, true) => from_bits <= bits,
        (false, true) => from_bits < bits,
        (true, false) => false,
    };
    if holds_every_value {
        return raw;
    }
    let mask: u128 = (1u128 << bits) - 1;
    if signed {
        let half: u128 = 1u128 << (bits - 1);
        format!("((({raw}) + 0x{half:X}) & 0x{mask:X}) - 0x{half:X}")
    } else {
        format!("({raw}) & 0x{mask:X}")
    }
}

fn python_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "==",
        BinOp::StrictNeq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "and",
        BinOp::Or => "or",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>",
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — C11
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// C and C++ share enough that emit_c reuses the cpp_binop / cpp_unary
// operator strings and the same arithmetic-conversion rules (integer
// promotion, decimal-integer-to-float `.0` promotion in float context).
// The differences vs emit_cpp:
//
// * Identifiers go through `to_snake_case` (matches the C convention and
//   the parameter names emitted by `LangCtx::format_param` for
//   `Language::C11`). Cpp leaves identifiers verbatim.
// * Null literal becomes `NULL` (from `<stddef.h>`) instead of `nullptr`.
// * No other differences are exercised by the transform fixture
//   set; broader operator and type coverage waits until a consumer
//   needs it.

fn emit_c(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // Push-down: arithmetic + Float expectation propagates into operands so
    // decimal-integer literals pick up `.0` and avoid integer division.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
            let l_raw = emit_c(left, expected)?;
            let r_raw = emit_c(right, expected)?;
            let l = if c_family_divides_integers(*op, left, right) {
                format!("({})({l_raw})", c_family_float_name(expected))
            } else if child_needs_parens(left, *op, true, ecma_precedence)
                || c_family_clarity_parens(left, *op)
            {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, ecma_precedence)
                || c_family_clarity_parens(right, *op)
            {
                format!("({r_raw})")
            } else {
                r_raw
            };
            return Ok(format!("{l} {} {r}", cpp_binop(*op)));
        }
    }
    // Push-down: Unary{Neg|Pos} in float context — same rationale as emit_cpp.
    if let ExprKind::Unary {
        op: op @ (UnaryOp::Neg | UnaryOp::Pos),
        operand,
    } = &expr.kind
    {
        if matches!(expected, InferredType::Float { .. }) {
            let inner = emit_c(operand, expected)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            return Ok(if wrap {
                format!("{}({inner})", cpp_unary(*op))
            } else {
                format!("{}{inner}", cpp_unary(*op))
            });
        }
    }
    let raw = c_emit_node(expr)?;
    Ok(c_coerce(raw, expr.ty, expected, expr))
}

fn c_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    Ok(match &expr.kind {
        ExprKind::NumberLit(n) => n.clone(),
        ExprKind::StringLit { value, .. } => format!("\"{value}\""),
        // Standalone fallback — the bytes-equality Binary branch renders
        // the literal and its byte count itself (a `bytes` value has no
        // length without its `_len` sibling). RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => format!("\"{}\"", bytes_as_quoted_ascii(bytes)),
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "NULL".to_string(),
        ExprKind::Ident(s) => crate::filters::to_snake_case(s.clone()),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            // Bytes equality: C has no slice equality. The payload struct
            // stores a bytes field as `uint8_t <id>[CAP]; size_t <id>_len;`
            // (the EventSchema-bytes representation reuses the codec
            // bounded-buffer shape), so content equality is a length guard
            // plus `memcmp` over the asserted-equal byte count. A literal
            // operand fixes the count; the validator template's <string.h>
            // include (already added for the strcmp lowering below) also
            // covers memcmp. RFC §bytesguard-3 B3/B5 — only `===`/`!==` reach here.
            if matches!(op, BinOp::StrictEq | BinOp::StrictNeq)
                && (matches!(left.ty, InferredType::Bytes)
                    || matches!(right.ty, InferredType::Bytes))
            {
                let render =
                    |operand: &TypedExpr| -> Result<(String, String, Option<usize>), ExprError> {
                        if let ExprKind::BytesLit { bytes } = &operand.kind {
                            let v = format!("\"{}\"", bytes_as_quoted_ascii(bytes));
                            Ok((v, bytes.len().to_string(), Some(bytes.len())))
                        } else {
                            let v = emit_c(operand, InferredType::Unknown)?;
                            let len = format!("{v}_len");
                            Ok((v, len, None))
                        }
                    };
                let (lv, ll, l_lit) = render(left)?;
                let (rv, rl, r_lit) = render(right)?;
                let cmp_len = l_lit
                    .or(r_lit)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| ll.clone());
                return Ok(if matches!(op, BinOp::StrictEq) {
                    format!("{ll} == {rl} && memcmp({lv}, {rv}, {cmp_len}) == 0")
                } else {
                    format!("{ll} != {rl} || memcmp({lv}, {rv}, {cmp_len}) != 0")
                });
            }
            // String comparison lowering. C lacks operator
            // overloading; `a == b` on `const char *` is pointer equality, not
            // content equality. Lower any string-typed comparison (==, !=, <,
            // >, <=, >=) to `strcmp(a, b) <op> 0`, which is the lexicographic
            // semantic the other backends already provide natively (cpp via
            // std::string operator==, Rust via PartialOrd, Python lex order).
            // The validator template adds <string.h> when this lowering can
            // fire; non-validator kinds that import the result get the
            // include via the same mechanism if they ever exercise string
            // comparison in their expressions.
            if op.is_comparison()
                && matches!(left.ty, InferredType::Str)
                && matches!(right.ty, InferredType::Str)
            {
                let l_raw = emit_c(left, InferredType::Str)?;
                let r_raw = emit_c(right, InferredType::Str)?;
                return Ok(format!("strcmp({l_raw}, {r_raw}) {} 0", cpp_binop(*op)));
            }
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_c(left, operand_ty)?;
            let r_raw = emit_c(right, operand_ty)?;
            let l = if child_needs_parens(left, *op, true, ecma_precedence)
                || c_family_clarity_parens(left, *op)
            {
                format!("({l_raw})")
            } else {
                l_raw
            };
            let r = if child_needs_parens(right, *op, false, ecma_precedence)
                || c_family_clarity_parens(right, *op)
            {
                format!("({r_raw})")
            } else {
                r_raw
            };
            format!("{l} {} {r}", cpp_binop(*op))
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_c(operand, expr.ty)?;
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            );
            if wrap {
                format!("{}({inner})", cpp_unary(*op))
            } else {
                format!("{}{inner}", cpp_unary(*op))
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            format!(
                "{} ? {} : {}",
                emit_c(condition, InferredType::Bool)?,
                emit_c(consequent, expr.ty)?,
                emit_c(alternate, expr.ty)?,
            )
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_c(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            // A `bytes` operand lowers to `sce_forge_bytes_view_t`
            // (`{const uint8_t *data; size_t len}`), so a random byte
            // read projects through `.data` — mirroring the foreach
            // arm's `src.data[__i]`. Item C7 wildcard-keyexpr lowering.
            let accessor = if matches!(object.ty, InferredType::Bytes) {
                ".data"
            } else {
                ""
            };
            format!(
                "{}{accessor}[{}]",
                wrap_postfix(object, emit_c(object, InferredType::Unknown)?),
                emit_c(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
        } => {
            if is_len_builtin(callee, args) {
                return Ok(format!(
                    "({}).len",
                    emit_c(&args[0], InferredType::Unknown)?
                ));
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // C99 `llround` is half away from zero, same as C++. C99
                // `floor` returns an integral-VALUED double, so the cast is
                // exact rather than a second conversion.
                let inner = emit_c(&args[0], InferredType::Float { bits: 64 })?;
                return Ok(match op {
                    RealToInt::Round => format!("llround({inner})"),
                    RealToInt::Floor => format!("(long long)floor({inner})"),
                });
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_c(arg, argument_type(params, i))?);
            }
            format!(
                "{}({})",
                wrap_postfix(callee, emit_c(callee, InferredType::Unknown)?),
                a.join(", "),
            )
        }
        ExprKind::BytesView { source, len } => {
            // RFC c7-wildcard W-project: a bounded-string field projects to
            // the algorithm `bytes` param type `sce_forge_bytes_view_t`
            // (`{const uint8_t *data; size_t len}`). The C11 string field
            // lowers to `char <f>[N]` plus a length sibling whose member
            // name comes from the codec `length_field` SSOT (carried on the
            // node's `len` — §8 Smell A fix); the view is
            // `{ (const uint8_t *)<f>, <len> }`. `len` is `None` only for an
            // unrecognised source shape, where the `<f>_len` sibling
            // convention is the faithful fallback.
            let s = emit_c(source, InferredType::Unknown)?;
            let len_expr = match len {
                Some(l) => emit_c(l, InferredType::Unknown)?,
                None => format!("{s}_len"),
            };
            format!("(sce_forge_bytes_view_t){{ (const uint8_t *){s}, {len_expr} }}")
        }
    })
}

fn c_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    if let Some(spelled) = c_family_wide_literal(&raw, to, node) {
        return spelled;
    }
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    if let (UntypedInt, Float { .. }) = (from, to) {
        if let ExprKind::NumberLit(text) = &node.kind {
            if is_decimal_integer_literal(text) {
                return format!("{raw}.0");
            }
        }
    }
    raw
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::types::{FuncSig, InferredType, RecordShape};

    // ── Helpers ─────────────────────────────────────────────────

    /// Forcing function for [`ExprTarget::ALL`]: a new `ExprTarget`
    /// variant breaks the exhaustive `match` below at compile time,
    /// pointing the author at the array (and at every all-backend verdict
    /// that iterates it — e.g. `guard_is_native_lowerable`). The
    /// `contains` check additionally proves the array is not missing any
    /// existing variant.
    #[test]
    fn expr_target_all_contains_every_variant() {
        fn assert_listed(t: ExprTarget) {
            match t {
                ExprTarget::Cpp
                | ExprTarget::Kotlin
                | ExprTarget::Rust
                | ExprTarget::Go
                | ExprTarget::Python
                | ExprTarget::C => {}
            }
            assert!(
                ExprTarget::ALL.contains(&t),
                "{t:?} missing from ExprTarget::ALL"
            );
        }
        for t in ExprTarget::ALL {
            assert_listed(t);
        }
        assert_eq!(ExprTarget::ALL.len(), 6);
    }

    fn empty_ctx() -> TypeCtx<'static> {
        TypeCtx::new()
    }
    fn empty_renames() -> HashMap<&'static str, &'static str> {
        HashMap::new()
    }

    fn tp(expr: &str, target: ExprTarget) -> String {
        transpile_typed(
            expr,
            target,
            &empty_ctx(),
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap()
    }

    fn tp_err(expr: &str, target: ExprTarget) -> String {
        transpile_typed(
            expr,
            target,
            &empty_ctx(),
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap_err()
        .to_string()
    }

    fn float(bits: u8) -> InferredType {
        InferredType::Float { bits }
    }
    fn int(signed: bool, bits: u8) -> InferredType {
        InferredType::Int { signed, bits }
    }

    fn tp_with(expr: &str, target: ExprTarget, ctx: &TypeCtx<'_>) -> String {
        transpile_typed(expr, target, ctx, &empty_renames(), InferredType::Unknown).unwrap()
    }

    // ── Arithmetic (untyped contexts, verbatim) ─────────────────

    #[test]
    fn cpp_arithmetic_verbatim() {
        assert_eq!(tp("raw * 0.1 - 40.0", ExprTarget::Cpp), "raw * 0.1 - 40.0");
    }

    #[test]
    fn cpp_strict_equality_maps_to_double_equals() {
        assert_eq!(tp("status === 'OK'", ExprTarget::Cpp), "status == \"OK\"");
    }

    #[test]
    fn cpp_logical_verbatim() {
        assert_eq!(
            tp("engineStop && ignOn", ExprTarget::Cpp),
            "engineStop && ignOn"
        );
    }

    #[test]
    fn rust_comparison_verbatim() {
        assert_eq!(tp("rpm > 8000", ExprTarget::Rust), "rpm > 8000");
    }

    #[test]
    fn rust_ternary_if_else() {
        assert_eq!(
            tp("status === 'OK' ? 1 : 0", ExprTarget::Rust),
            "if status == \"OK\" { 1 } else { 0 }"
        );
    }

    #[test]
    fn python_logical_with_snake_case() {
        // Python ident convention is snake_case — emitter normalizes.
        assert_eq!(
            tp("engineStop && ignOn", ExprTarget::Python),
            "engine_stop and ign_on"
        );
    }

    #[test]
    fn python_booleans_with_snake_case() {
        assert_eq!(
            tp(
                "ignition === true && engineStop === false",
                ExprTarget::Python
            ),
            "ignition == True and engine_stop == False"
        );
    }

    // ── Rejection rules ──────────────────────────────────────────

    #[test]
    fn reject_arrow_function() {
        assert!(transpile_typed(
            "() => x + 1",
            ExprTarget::Cpp,
            &empty_ctx(),
            &empty_renames(),
            InferredType::Unknown
        )
        .is_err());
    }

    #[test]
    fn reject_new_keyword() {
        let e = tp_err("new Map()", ExprTarget::Cpp);
        assert!(e.contains("new"));
    }

    #[test]
    fn cpp_bitwise() {
        assert_eq!(tp("raw & 0x0F", ExprTarget::Cpp), "raw & 0x0F");
    }

    #[test]
    fn cpp_shift_and_mask() {
        assert_eq!(
            tp("(raw[1] >> 4) & 0x0F", ExprTarget::Cpp),
            "raw[1] >> 4 & 0x0F"
        );
    }

    #[test]
    fn cpp_string_literal_preserved() {
        assert_eq!(tp("status === 'new'", ExprTarget::Cpp), "status == \"new\"");
    }

    #[test]
    fn cpp_string_literal_contents_preserved() {
        assert_eq!(tp("x === 'a === b'", ExprTarget::Cpp), "x == \"a === b\"");
    }

    // ── C11 V3: string compare → strcmp lowering ────────────────

    #[test]
    fn c_string_eq_lowers_to_strcmp() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("engineState", InferredType::Str);
        assert_eq!(
            tp_with("engineState === 'STOP'", ExprTarget::C, &ctx),
            "strcmp(engine_state, \"STOP\") == 0"
        );
    }

    #[test]
    fn c_string_neq_lowers_to_strcmp() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("engineState", InferredType::Str);
        assert_eq!(
            tp_with("engineState !== 'STOP'", ExprTarget::C, &ctx),
            "strcmp(engine_state, \"STOP\") != 0"
        );
    }

    #[test]
    fn c_string_lex_compare_lowers_to_strcmp() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("a", InferredType::Str);
        ctx.insert_var("b", InferredType::Str);
        assert_eq!(tp_with("a < b", ExprTarget::C, &ctx), "strcmp(a, b) < 0");
        assert_eq!(tp_with("a > b", ExprTarget::C, &ctx), "strcmp(a, b) > 0");
        assert_eq!(tp_with("a <= b", ExprTarget::C, &ctx), "strcmp(a, b) <= 0");
        assert_eq!(tp_with("a >= b", ExprTarget::C, &ctx), "strcmp(a, b) >= 0");
    }

    // ── W-project: Str-arg → borrowed bytes-view projection ─────
    // A `Str` argument flowing into a `bytes` parameter is projected to a
    // borrowed view at the call site. The C11 view's length must
    // come from the codec `length_field` SSOT registered in the TypeCtx
    // (length-field SSOT rule), never a `<field>_len` guess.
    fn projection_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.project_str_args_as_bytes_view = true;
        ctx.insert_func(
            "eq",
            FuncSig {
                params: vec![InferredType::Bytes, InferredType::Bytes],
                ret: InferredType::Bool,
            },
        );
        ctx.insert_var("entry.pattern", InferredType::Str);
        ctx.insert_var("target", InferredType::Bytes);
        ctx
    }

    #[test]
    fn c_bytes_view_projection_uses_codec_length_field_ssot() {
        // The length sibling name deliberately does NOT match the `_len`
        // convention — a guessing emit would wrongly produce
        // `entry.pattern_len`. The SSOT-driven emit must use `entry.klen`.
        let mut ctx = projection_ctx();
        ctx.insert_member_len_field("entry.pattern", "klen");
        assert_eq!(
            tp_with("eq(entry.pattern, target)", ExprTarget::C, &ctx),
            "eq((sce_forge_bytes_view_t){ (const uint8_t *)entry.pattern, entry.klen }, target)"
        );
    }

    #[test]
    fn c_bytes_view_projection_passes_through_existing_bytes_arg() {
        // `target` is already a `bytes` view param — it is NOT re-wrapped;
        // only the `Str` field argument is projected.
        let mut ctx = projection_ctx();
        ctx.insert_member_len_field("entry.pattern", "klen");
        let out = tp_with("eq(target, entry.pattern)", ExprTarget::C, &ctx);
        assert_eq!(
            out,
            "eq(target, (sce_forge_bytes_view_t){ (const uint8_t *)entry.pattern, entry.klen })"
        );
    }

    #[test]
    fn c_bytes_view_projection_falls_back_to_len_sibling_when_unregistered() {
        // No length sibling registered: the C11 emit falls back to the
        // `<src>_len` sibling convention (faithful for an auto-`_len`
        // bytes field, which is the only shape that reaches the fallback).
        let ctx = projection_ctx();
        assert_eq!(
            tp_with("eq(entry.pattern, target)", ExprTarget::C, &ctx),
            "eq((sce_forge_bytes_view_t){ (const uint8_t *)entry.pattern, entry.pattern_len }, target)"
        );
    }

    #[test]
    fn rust_bytes_view_projection_uses_as_bytes() {
        // Non-C11 backends ignore the C11 length node and read the length
        // from the string type itself.
        let mut ctx = projection_ctx();
        ctx.insert_member_len_field("entry.pattern", "klen");
        assert_eq!(
            tp_with("eq(entry.pattern, target)", ExprTarget::Rust, &ctx),
            "eq(entry.pattern.as_bytes(), target)"
        );
    }

    #[test]
    fn projection_does_not_fire_without_algorithm_flag() {
        // Outside the algorithm kind the flag is off → no projection, the
        // arg is emitted verbatim (pre-W-project behaviour preserved).
        let mut ctx = projection_ctx();
        ctx.project_str_args_as_bytes_view = false;
        ctx.insert_member_len_field("entry.pattern", "klen");
        assert_eq!(
            tp_with("eq(entry.pattern, target)", ExprTarget::C, &ctx),
            "eq(entry.pattern, target)"
        );
    }

    #[test]
    fn go_member_access_exports_struct_field_in_algorithm() {
        // RFC c7-wildcard W-project: Go exports struct fields in PascalCase
        // (the `codec_field_id` SSOT). In the algorithm kind the element-
        // field read must bind against `entry.Pattern`, not `entry.pattern`,
        // and the projected `bytes`-view wraps the exported spelling.
        let ctx = projection_ctx();
        assert_eq!(
            tp_with("eq(entry.pattern, target)", ExprTarget::Go, &ctx),
            "eq([]byte(entry.Pattern), target)"
        );
    }

    #[test]
    fn go_member_access_stays_verbatim_without_algorithm_flag() {
        // The PascalCase export is gated to the algorithm kind. A statechart
        // guard path (flag off) keeps its verbatim leaf-property emit so it
        // binds against the unexported `_event.data` payload struct — the
        // `entry.Pattern` rewrite must never leak here.
        let mut renames = HashMap::new();
        renames.insert("_event.data", "ev");
        let out = transpile_typed(
            "_event.data.elapsed_ms",
            ExprTarget::Go,
            &empty_ctx(),
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "ev.elapsed_ms");
    }

    // ── Bytes equality (RFC §bytesguard-3 B1/B5) ───────────────────────────
    // A string literal compared against a `Bytes`-typed operand is
    // reinterpreted as a byte sequence (`BytesLit`) and lowered to each
    // backend's content-equality primitive. The decoded bytes are
    // identical on every backend; only the surface syntax differs.
    fn bytes_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("raw", InferredType::Bytes);
        ctx
    }

    #[test]
    fn bytes_eq_lowers_per_backend() {
        let ctx = bytes_ctx();
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::Rust, &ctx),
            "raw == b\"ack\""
        );
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::Python, &ctx),
            "raw == b\"ack\""
        );
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::Cpp, &ctx),
            "raw == std::vector<uint8_t>{0x61, 0x63, 0x6b}"
        );
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::Go, &ctx),
            "string(raw) == \"ack\""
        );
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::Kotlin, &ctx),
            "raw.contentEquals(\"ack\".toByteArray())"
        );
        assert_eq!(
            tp_with("raw === 'ack'", ExprTarget::C, &ctx),
            "raw_len == 3 && memcmp(raw, \"ack\", 3) == 0"
        );
    }

    #[test]
    fn bytes_neq_lowers_per_backend() {
        let ctx = bytes_ctx();
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::Rust, &ctx),
            "raw != b\"ack\""
        );
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::Python, &ctx),
            "raw != b\"ack\""
        );
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::Cpp, &ctx),
            "raw != std::vector<uint8_t>{0x61, 0x63, 0x6b}"
        );
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::Go, &ctx),
            "string(raw) != \"ack\""
        );
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::Kotlin, &ctx),
            "!raw.contentEquals(\"ack\".toByteArray())"
        );
        assert_eq!(
            tp_with("raw !== 'ack'", ExprTarget::C, &ctx),
            "raw_len != 3 || memcmp(raw, \"ack\", 3) != 0"
        );
    }

    // A non-ASCII / escape-bearing literal is NOT reinterpreted (RFC §bytesguard-3
    // B2): the node stays a `StringLit`, so the comparison does not lower
    // to a byte constant. The receive-side validator rejects it later;
    // here we only assert the reinterpretation declined.
    #[test]
    fn bytes_non_ascii_literal_not_reinterpreted() {
        let ctx = bytes_ctx();
        // A backslash escape is declined — Rust would otherwise need a
        // decoder; the node stays a StringLit, so no `BytesLit` byte
        // constant (`== b"…"`) is emitted. The gate / validator handles
        // the residual mismatch, not codegen.
        let out = tp_with("raw === 'a\\tb'", ExprTarget::Rust, &ctx);
        assert_eq!(out, "raw == \"a\\tb\"");
        assert!(
            !out.contains("== b\""),
            "must not reinterpret as bytes: {out}"
        );
    }

    #[test]
    fn c_numeric_compare_unchanged() {
        // Sanity: numeric comparison is NOT routed through strcmp.
        let mut ctx = TypeCtx::new();
        ctx.insert_var(
            "rpm",
            InferredType::Int {
                signed: false,
                bits: 16,
            },
        );
        assert_eq!(tp_with("rpm > 8000", ExprTarget::C, &ctx), "rpm > 8000");
    }

    #[test]
    fn c_string_compare_within_logical_combo() {
        // rpm_validator real expression: numeric == on rpm, string !== on engineState.
        let mut ctx = TypeCtx::new();
        ctx.insert_var(
            "rpm",
            InferredType::Int {
                signed: false,
                bits: 16,
            },
        );
        ctx.insert_var("engineState", InferredType::Str);
        assert_eq!(
            tp_with("rpm === 0 || engineState !== 'STOP'", ExprTarget::C, &ctx),
            "rpm == 0 || strcmp(engine_state, \"STOP\") != 0"
        );
    }

    #[test]
    fn go_rejects_ternary_with_message() {
        let e = tp_err("x > 0 ? 1 : 0", ExprTarget::Go);
        assert!(e.contains("ternary") || e.contains("conditional"));
    }

    #[test]
    fn kotlin_bitwise_shift_infix() {
        assert_eq!(
            tp("(byte >> 4) & 0x0F", ExprTarget::Kotlin),
            "byte shr 4 and 0x0F"
        );
    }

    #[test]
    fn kotlin_logical_preserved() {
        assert_eq!(tp("a && b || c", ExprTarget::Kotlin), "a && b || c");
    }

    #[test]
    fn kotlin_bitwise_mixed_with_logical() {
        assert_eq!(
            tp("(x >> 4) & 0x0F && y === true", ExprTarget::Kotlin),
            "x shr 4 and 0x0F && y == true"
        );
    }

    #[test]
    fn kotlin_left_shift() {
        assert_eq!(tp("x << 8", ExprTarget::Kotlin), "x shl 8");
    }

    #[test]
    fn kotlin_unsigned_shift() {
        assert_eq!(tp("x >>> 4", ExprTarget::Kotlin), "x ushr 4");
    }

    #[test]
    fn kotlin_xor() {
        assert_eq!(tp("a ^ b", ExprTarget::Kotlin), "a xor b");
    }

    #[test]
    fn kotlin_bitwise_not() {
        assert_eq!(tp("~mask", ExprTarget::Kotlin), "mask.inv()");
    }

    #[test]
    fn rust_nested_ternary() {
        assert_eq!(
            tp("a > 0 ? (b > 1 ? 2 : 3) : 4", ExprTarget::Rust),
            "if a > 0 { if b > 1 { 2 } else { 3 } } else { 4 }"
        );
    }

    #[test]
    fn kotlin_nested_bitwise_not() {
        assert_eq!(
            tp("~(a & (b | c))", ExprTarget::Kotlin),
            "(a and (b or c)).inv()"
        );
    }

    #[test]
    fn cpp_chained_member_access() {
        assert_eq!(
            tp("_event.data.payload", ExprTarget::Cpp),
            "_event.data.payload"
        );
    }

    #[test]
    fn cpp_function_call_multi_args() {
        assert_eq!(
            tp("computeKey(seed, 0x01)", ExprTarget::Cpp),
            "computeKey(seed, 0x01)"
        );
    }

    #[test]
    fn cpp_method_call_on_member() {
        assert_eq!(
            tp("securityResponse.decode(_event.data)", ExprTarget::Cpp),
            "securityResponse.decode(_event.data)"
        );
    }

    #[test]
    fn cpp_complex_codec_expression() {
        assert_eq!(
            tp("(raw[2] << 16) | (raw[3] << 8) | raw[4]", ExprTarget::Cpp),
            "raw[2] << 16 | raw[3] << 8 | raw[4]"
        );
    }

    #[test]
    fn kotlin_complex_codec_expression() {
        assert_eq!(
            tp(
                "(raw[2] << 16) | (raw[3] << 8) | raw[4]",
                ExprTarget::Kotlin
            ),
            "raw[2] shl 16 or (raw[3] shl 8) or raw[4]"
        );
    }

    #[test]
    fn cpp_precedence_bitwise_vs_comparison() {
        assert_eq!(tp("a & 0xFF === b", ExprTarget::Cpp), "a & 0xFF == b");
    }

    /// `&&` inside `||` must carry parens in the C family, although precedence
    /// does not require them.
    ///
    /// ⚠ THE WITNESS IS A BUILD FAILURE, NOT A STYLE PREFERENCE. Emitting
    /// `a == 2 || b >= 1 && b <= 4` is precedence-correct and GCC still refuses
    /// it under `-Wall -Werror`:
    ///
    ///     error: suggest parentheses around '&&' within '||'
    ///            [-Werror=parentheses]
    ///
    /// A real downstream build uses exactly those flags, so a generated file of
    /// this shape does not compile at all. Found by converting a component and
    /// feeding the result to that build.
    #[test]
    fn c_family_parenthesises_and_within_or() {
        for target in [ExprTarget::Cpp, ExprTarget::C] {
            assert_eq!(
                tp("a === 2 || b >= 1 && b <= 4", target),
                "a == 2 || (b >= 1 && b <= 4)",
                "{target:?} left the && bare inside ||"
            );
            // the other operand order warns the same way
            assert_eq!(
                tp("b >= 1 && b <= 4 || a === 2", target),
                "(b >= 1 && b <= 4) || a == 2",
                "{target:?} left the leading && bare"
            );
        }
        // ⚠ Not widened past the C family: no other target has this
        // diagnostic, and adding parens there would re-pin committed trees to
        // silence a warning that does not exist.
        assert_eq!(
            tp("a === 2 || b >= 1 && b <= 4", ExprTarget::Rust),
            "a == 2 || b >= 1 && b <= 4"
        );
    }

    #[test]
    fn rust_nested_function_calls_rendered_snake_case() {
        // Rust ident convention: identifiers normalized to snake_case by emitter.
        // Member/property/function call names defined in user space follow the
        // same rule because SCXML authors write camelCase but Rust idiom is
        // snake_case functions.
        assert_eq!(
            tp("encode(computeKey(seed), 0x02)", ExprTarget::Rust),
            "encode(compute_key(seed), 0x02)"
        );
    }

    #[test]
    fn cpp_unary_in_binary() {
        assert_eq!(tp("-a + b", ExprTarget::Cpp), "-a + b");
    }

    #[test]
    fn reject_loose_equality() {
        let e = tp_err("x == y", ExprTarget::Cpp);
        assert!(e.contains("loose =="));
    }

    #[test]
    fn reject_loose_inequality() {
        let e = tp_err("x != y", ExprTarget::Cpp);
        assert!(e.contains("loose !="));
    }

    #[test]
    fn reject_optional_chaining() {
        let e = tp_err("a?.b", ExprTarget::Cpp);
        assert!(e.contains("optional chaining"));
    }

    #[test]
    fn reject_spread() {
        let e = tp_err("f(...args)", ExprTarget::Cpp);
        assert!(e.contains("spread/rest"));
    }

    #[test]
    fn reject_template_literal() {
        let e = tp_err("`hello ${x}`", ExprTarget::Cpp);
        assert!(e.contains("template literal"));
    }

    #[test]
    fn reject_nullish_coalescing() {
        let e = tp_err("a ?? b", ExprTarget::Cpp);
        assert!(e.contains("nullish"));
    }

    #[test]
    fn double_bitnot() {
        assert_eq!(tp("~~mask", ExprTarget::Cpp), "~~mask");
        assert_eq!(tp("~~mask", ExprTarget::Kotlin), "(mask.inv()).inv()");
    }

    #[test]
    fn leading_dot_float_literal_normalized() {
        assert_eq!(tp(".5 + x", ExprTarget::Cpp), "0.5 + x");
    }

    #[test]
    fn scientific_notation_preserved() {
        assert_eq!(tp("1.5e10 * factor", ExprTarget::Cpp), "1.5e10 * factor");
    }

    #[test]
    fn null_literal_by_language() {
        assert_eq!(tp("x === null", ExprTarget::Cpp), "x == nullptr");
        assert_eq!(tp("x === null", ExprTarget::Kotlin), "x == null");
        assert_eq!(tp("x === null", ExprTarget::Python), "x == None");
        assert_eq!(tp("x === null", ExprTarget::Go), "x == nil");
    }

    // ── Type-aware coercion: Rust ───────────────────────────────

    fn ctx_with_float(name: &'static str) -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var(name, float(64));
        ctx
    }

    fn ctx_with_uint(name: &'static str, bits: u8) -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var(name, int(false, bits));
        ctx
    }

    #[test]
    fn rust_promotes_decimal_literal_in_float_binary() {
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "celsius * 9 / 5 + 32",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "celsius * 9.0 / 5.0 + 32.0");
    }

    #[test]
    fn rust_keeps_float_literals_unchanged() {
        let ctx = ctx_with_uint("raw", 16);
        let out = transpile_typed(
            "raw * 0.1 - 40.0",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        // Concrete int `raw` coerced to f64, float literals untouched.
        assert_eq!(out, "raw as f64 * 0.1 - 40.0");
    }

    #[test]
    fn rust_rejects_hex_literal_in_float_context() {
        let ctx = ctx_with_float("x");
        let err = transpile_typed(
            "x * 0xFF",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("hex/binary/octal"), "error: {err}");
    }

    #[test]
    fn an_integer_literal_must_fit_the_type_it_takes() {
        let ctx = TypeCtx::new();
        let judged = |expr: &str, to: InferredType| {
            transpile_typed(expr, ExprTarget::Rust, &ctx, &empty_renames(), to)
                .map(|_| ())
                .map_err(|refusal| refusal.to_string())
        };
        // Each type's edges fit, whatever the radix, and the minus counts.
        for (expr, to) in [
            ("255", int(false, 8)),
            ("0xFF", int(false, 8)),
            ("0b11111111", int(false, 8)),
            ("0o377", int(false, 8)),
            ("-128", int(true, 8)),
            ("127", int(true, 8)),
            ("0", int(false, 8)),
            ("18446744073709551615", int(false, 64)),
            ("-9223372036854775808", int(true, 64)),
        ] {
            assert_eq!(judged(expr, to), Ok(()), "{expr} as {}", to.describe());
        }
        // One past each edge does not, nor does a literal wider than 64 bits.
        for (expr, to) in [
            ("256", int(false, 8)),
            ("0x100", int(false, 8)),
            ("-129", int(true, 8)),
            ("128", int(true, 8)),
            ("-1", int(false, 8)),
            ("18446744073709551616", int(false, 64)),
            ("99999999999999999999999", int(true, 64)),
        ] {
            let refusal = judged(expr, to).expect_err(expr);
            assert!(refusal.contains("does not fit in"), "{expr}: {refusal}");
        }
        // A real context, or none, holds any integer literal.
        assert_eq!(judged("300", float(64)), Ok(()));
        assert_eq!(judged("300", InferredType::Unknown), Ok(()));
    }

    #[test]
    fn a_literal_past_long_is_spelled_as_each_backend_reads_it() {
        let ctx = TypeCtx::new();
        let emit = |expr: &str, target: ExprTarget, to: InferredType| {
            transpile_typed(expr, target, &ctx, &empty_renames(), to).unwrap()
        };
        let (unsigned64, signed64) = (int(false, 64), int(true, 64));
        for (target, max, min) in [
            (
                ExprTarget::C,
                "18446744073709551615ULL",
                "(-9223372036854775807LL - 1)",
            ),
            (
                ExprTarget::Cpp,
                "18446744073709551615ULL",
                "(-9223372036854775807LL - 1)",
            ),
            (
                ExprTarget::Kotlin,
                "18446744073709551615uL",
                "Long.MIN_VALUE",
            ),
            (
                ExprTarget::Rust,
                "18446744073709551615",
                "-9223372036854775808",
            ),
            (
                ExprTarget::Go,
                "18446744073709551615",
                "-9223372036854775808",
            ),
            (
                ExprTarget::Python,
                "18446744073709551615",
                "-9223372036854775808",
            ),
        ] {
            assert_eq!(
                emit("18446744073709551615", target, unsigned64),
                max,
                "{target:?}"
            );
            assert_eq!(
                emit("-9223372036854775808", target, signed64),
                min,
                "{target:?}"
            );
        }
        // Within `Long` the literal stands as it always did.
        assert_eq!(
            emit("9223372036854775807", ExprTarget::C, unsigned64),
            "9223372036854775807"
        );
        assert_eq!(
            emit("-9223372036854775807", ExprTarget::Kotlin, signed64),
            "-9223372036854775807"
        );
    }

    #[test]
    fn rust_condition_with_float_field_promotes_literal() {
        let ctx = ctx_with_float("temperature");
        let out = transpile_typed(
            "temperature > 100 && temperature < 200",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            InferredType::Bool,
        )
        .unwrap();
        assert_eq!(out, "temperature > 100.0 && temperature < 200.0");
    }

    #[test]
    fn rust_integer_only_expression_untouched() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("counter", int(true, 32));
        let out = transpile_typed(
            "counter + 1",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            int(true, 32),
        )
        .unwrap();
        assert_eq!(out, "counter + 1");
    }

    #[test]
    fn rust_top_level_bare_integer_literal_promotes() {
        let ctx = empty_ctx();
        let out =
            transpile_typed("42", ExprTarget::Rust, &ctx, &empty_renames(), float(64)).unwrap();
        assert_eq!(out, "42.0");
    }

    // ── Type-aware coercion: Go ─────────────────────────────────

    #[test]
    fn go_wraps_concrete_int_ident_with_float64() {
        let ctx = ctx_with_uint("raw", 16);
        let out = transpile_typed(
            "raw * 0.1",
            ExprTarget::Go,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        // Go untyped literal auto-converts, concrete ident needs wrap.
        assert_eq!(out, "float64(raw) * 0.1");
    }

    // ── Language-conditional literal promotion ─────────────────
    //
    // C++/Go promote integer literals to `.0` in float context to prevent
    // integer division (`9 / 5` → 1 without promotion). Python's `/` is
    // true division (PEP 238), so no promotion needed. Kotlin/Rust require
    // promotion for type-system reasons.

    #[test]
    fn go_promotes_literal_in_float_context() {
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "celsius * 9 / 5 + 32",
            ExprTarget::Go,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "celsius * 9.0 / 5.0 + 32.0");
    }

    #[test]
    fn go_integer_division_prevented_by_promotion() {
        // Without promotion, `9 / 5 + celsius` would integer-divide to 1.
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "9 / 5 + celsius",
            ExprTarget::Go,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "9.0 / 5.0 + celsius");
    }

    // ── Type-aware coercion: Kotlin ─────────────────────────────

    #[test]
    fn kotlin_promotes_decimal_literal_to_double_in_float_context() {
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "celsius * 9 / 5 + 32",
            ExprTarget::Kotlin,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "celsius * 9.0 / 5.0 + 32.0");
    }

    #[test]
    fn kotlin_wraps_concrete_int_with_to_double() {
        let ctx = ctx_with_uint("raw", 16);
        let out = transpile_typed(
            "raw * 0.1",
            ExprTarget::Kotlin,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "raw.toDouble() * 0.1");
    }

    // ── Type-aware coercion: C++ / Python ───────────────────────

    #[test]
    fn cpp_promotes_literal_in_float_context() {
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "celsius * 9 / 5 + 32",
            ExprTarget::Cpp,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "celsius * 9.0 / 5.0 + 32.0");
    }

    #[test]
    fn cpp_integer_division_prevented_by_promotion() {
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "9 / 5 + celsius",
            ExprTarget::Cpp,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "9.0 / 5.0 + celsius");
    }

    #[test]
    fn python_float_context_leaves_literal_alone() {
        // Python `/` is true division — no promotion needed.
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "celsius * 9 / 5 + 32",
            ExprTarget::Python,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "celsius * 9 / 5 + 32");
    }

    #[test]
    fn python_integer_division_safe_without_promotion() {
        // Python `/` is true division (PEP 238): `9 / 5` yields 1.8.
        let ctx = ctx_with_float("celsius");
        let out = transpile_typed(
            "9 / 5 + celsius",
            ExprTarget::Python,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "9 / 5 + celsius");
    }

    #[test]
    fn python_wraps_an_integer_to_the_type_it_is_converted_to() {
        // Rust `as`, Go and C conversions and Kotlin `toUByte()` keep the low
        // bits; Python's `int` keeps all of them unless told otherwise.
        let mut ctx = TypeCtx::new();
        ctx.insert_var("wide", int(false, 32));
        ctx.insert_var("signed_wide", int(true, 32));
        ctx.insert_var("big", int(true, 64));
        let py = |expr: &str, to: InferredType| {
            transpile_typed(expr, ExprTarget::Python, &ctx, &empty_renames(), to).unwrap()
        };
        assert_eq!(py("wide", int(false, 8)), "(wide) & 0xFF");
        assert_eq!(py("signed_wide", int(false, 8)), "(signed_wide) & 0xFF");
        assert_eq!(
            py("signed_wide", int(false, 64)),
            "(signed_wide) & 0xFFFFFFFFFFFFFFFF"
        );
        assert_eq!(
            py("big", int(true, 32)),
            "(((big) + 0x80000000) & 0xFFFFFFFF) - 0x80000000"
        );
        assert_eq!(
            py("wide", int(true, 32)),
            "(((wide) + 0x80000000) & 0xFFFFFFFF) - 0x80000000"
        );
        // A type that holds every value of the source changes nothing.
        assert_eq!(py("wide", int(false, 64)), "wide");
        assert_eq!(py("wide", int(true, 64)), "wide");
        assert_eq!(py("signed_wide", int(true, 64)), "signed_wide");
    }

    // ── Function signature lookup ───────────────────────────────

    #[test]
    fn rust_call_with_known_float_return_propagates_type() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("raw", int(false, 16));
        ctx.insert_func(
            "temp_xform",
            FuncSig {
                params: vec![int(false, 16)],
                ret: float(64),
            },
        );
        let out = transpile_typed(
            "temp_xform(raw) * 2 + 1",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            float(64),
        )
        .unwrap();
        assert_eq!(out, "temp_xform(raw) * 2.0 + 1.0");
    }

    #[test]
    fn member_call_return_type_propagates_bytes() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("frame", InferredType::Unknown);
        ctx.insert_func(
            "frame.encode",
            FuncSig {
                params: vec![],
                ret: InferredType::Bytes,
            },
        );
        // frame.encode()[0] should infer Index on Bytes → u8
        let mut ast = parse_to_ast("frame.encode()[0]").unwrap();
        infer_types(&mut ast, &ctx);
        assert_eq!(ast.ty, int(false, 8));
    }

    #[test]
    fn member_call_unknown_when_not_registered() {
        let ctx = TypeCtx::new();
        let mut ast = parse_to_ast("frame.encode()").unwrap();
        infer_types(&mut ast, &ctx);
        assert_eq!(ast.ty, InferredType::Unknown);
    }

    /// Every node the parser builds names the text it was read from, in
    /// the string the caller passed — surrounding whitespace, a
    /// non-ASCII literal and a parenthesised operand included — so a
    /// refusal can quote what the author wrote.
    #[test]
    fn a_parsed_node_names_the_text_it_was_read_from() {
        let source = "  _event.data.raw === ('café') && -n.len() > 0x1F ";
        let spelled = |node: &TypedExpr| node.span.clone().and_then(|range| source.get(range));
        let ast = parse_to_ast(source).unwrap();
        assert_eq!(spelled(&ast), Some(source.trim()));
        let ExprKind::Binary { left, right, .. } = &ast.kind else {
            panic!("expected `&&` at the top: {ast:?}");
        };
        let ExprKind::Binary {
            left: data,
            right: literal,
            ..
        } = &left.kind
        else {
            panic!("expected `===` on the left: {left:?}");
        };
        assert_eq!(spelled(data), Some("_event.data.raw"));
        assert_eq!(spelled(literal), Some("('café')"));
        let ExprKind::Binary {
            left: negated,
            right: hex,
            ..
        } = &right.kind
        else {
            panic!("expected `>` on the right: {right:?}");
        };
        assert_eq!(spelled(negated), Some("-n.len()"));
        assert_eq!(spelled(hex), Some("0x1F"));
    }

    /// The lexer is total: it answers for any `&str`, including one holding
    /// a character it does not know.
    ///
    /// It used to abort instead. The multi-character needles were compared
    /// as `&input[i..i + n]`, and while `i` is always on a character
    /// boundary `i + n` is not — one non-ASCII character put the end of the
    /// slice inside it and slicing a `&str` there panics. The reachable
    /// shapes are two, and only the first is the one a reader predicts:
    ///
    ///   * the unknown character *starts* a token, so `i` is on it;
    ///   * the unknown character *follows* a one-byte operator that has a
    ///     longer form — `?`, `!`, `>`, `<`, `&`, `|`, `.`, `=` — so `i` is
    ///     on ASCII and only `i + 1` or `i + 2` lands inside the character.
    ///
    /// Inserting at every byte offset covers both without naming them, and
    /// is what keeps this honest if a needle is added later at a position
    /// nobody thought to enumerate. A panic here is not a worse diagnostic
    /// than a rejection — it is *no* diagnostic: `sce-codegen` aborts, so
    /// the author gets an exit status and an empty `--error-format=json`
    /// stream where the `expression/lex` record belongs.
    #[test]
    fn the_lexer_answers_for_a_character_it_does_not_know() {
        // Every operator whose longer form the lexer looks ahead for.
        let host = "a ? b . c ! d = e > f < g & h | i / j * k";
        // Chosen for UTF-8 *width*, not for what they mean: two, three and
        // four bytes, so the end of a two- or three-byte needle can land on
        // the character's every interior offset. Any character of that width
        // would do; these are not sampled from a document.
        for unknown in ["\u{00e9}", "\u{21d2}", "\u{ac00}", "\u{10348}"] {
            assert!(
                matches!(unknown.len(), 2..=4),
                "a one-byte probe cannot reach the defect"
            );
            for at in 0..=host.len() {
                if !host.is_char_boundary(at) {
                    continue;
                }
                let mut probe = String::with_capacity(host.len() + unknown.len());
                probe.push_str(&host[..at]);
                probe.push_str(unknown);
                probe.push_str(&host[at..]);
                for mode in [LexMode::Forge, LexMode::EcmaScript] {
                    // The verdict is only that it *returns*. Whether the
                    // character is refused or swallowed by a literal is the
                    // business of the arms above, not of this claim.
                    let answer = tokenize_as(&probe, mode);
                    if let Err(ExprError::Lex { detail, .. }) = &answer {
                        // A lead byte read as Latin-1 would name a character
                        // the author never wrote and cannot search for.
                        assert!(
                            !detail.contains('\u{fffd}'),
                            "the refusal names a replacement character: {detail}"
                        );
                    }
                }
            }
        }
    }

    /// The character in the refusal is the one in the document.
    ///
    /// Splitting this from the totality claim above is deliberate: making
    /// the lexer return is one fix, and making it name the right character
    /// is another. A test that only asserted "it returns" would have passed
    /// on a version that reported `'\u{e2}'` — the first byte of the
    /// character below, read as a codepoint — which is a character the
    /// author never wrote and cannot find by searching their document.
    #[test]
    fn the_refusal_names_the_character_the_author_wrote() {
        const UNKNOWN: char = '\u{21d2}';
        let source = format!("a + {UNKNOWN}");
        let err = tokenize(&source).expect_err("an arrow is not Forge syntax");
        let ExprError::Lex { detail, position } = err else {
            panic!("expected a lex error, got {err:?}");
        };
        assert!(
            detail.contains(UNKNOWN),
            "the refusal must carry the character itself: {detail}"
        );
        assert!(
            !detail.contains(source.as_bytes()[position] as char),
            "the refusal names the lead byte rather than the character: {detail}"
        );
        // A byte offset, so the caller can slice the author's own text.
        assert!(
            source.is_char_boundary(position),
            "position {position} is not a character boundary"
        );
    }

    // ── Rename map ──────────────────────────────────────────────

    #[test]
    fn rename_event_data_to_member_field() {
        let mut renames = HashMap::new();
        renames.insert("_event.data", "pendingEventData_");
        let out = transpile_typed(
            "_event.data + 1",
            ExprTarget::Cpp,
            &empty_ctx(),
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "pendingEventData_ + 1");
    }

    #[test]
    fn rename_camel_case_to_member() {
        let mut renames = HashMap::new();
        renames.insert("retryCount", "retryCount_");
        let out = transpile_typed(
            "retryCount + 1",
            ExprTarget::Cpp,
            &empty_ctx(),
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "retryCount_ + 1");
    }

    // EventSchema MCU native lowering:
    // the depth-3 `_event.data.<field>` access path is the exact shape
    // `event_schema_check::lower_typed_guard` feeds in — the field's
    // concrete type is registered under the full dotted key, the inner
    // `_event.data` Member is renamed to the bound payload variable, and
    // the leaf `.field` is preserved. Proves the generalized member-path
    // inference resolves the leaf type (so the literal adopts the field
    // width) and that the `===` guard lowers to native Rust `==`.
    #[test]
    fn event_data_typed_guard_lowers_to_native_rust() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("_event.data.elapsed_ms", int(false, 32));
        let mut renames = HashMap::new();
        renames.insert("_event.data", "ev");
        let out = transpile_typed(
            "_event.data.elapsed_ms === 0",
            ExprTarget::Rust,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "ev.elapsed_ms == 0");
    }

    // The generalized member-path lookup is additive: a depth-3 chain
    // whose full dotted key is NOT registered still infers `Unknown` and
    // emits verbatim, exactly as before the generalization — so no
    // existing caller's output shifts.
    #[test]
    fn unregistered_member_path_stays_opaque() {
        let mut renames = HashMap::new();
        renames.insert("_event.data", "ev");
        let out = transpile_typed(
            "_event.data.elapsed_ms === 0",
            ExprTarget::Cpp,
            &empty_ctx(),
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "ev.elapsed_ms == 0");
    }

    // ── members: only a record has them ───────────────────────

    /// A closed forge scope: a `uint8` input, an enum-typed input (which
    /// inference types `Unknown`), a stateful import `frame` with one field
    /// and one method, and `_event`, whose members the event schema owns.
    fn member_scope() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("x", int(false, 8));
        ctx.insert_var("tone", InferredType::Unknown);
        ctx.insert_record("frame", RecordShape::Closed);
        ctx.insert_var("frame.msg_id", int(false, 32));
        ctx.insert_func(
            "frame.encode",
            FuncSig {
                params: Vec::new(),
                ret: InferredType::Bytes,
            },
        );
        ctx.insert_record("_event", RecordShape::Open);
        ctx.reject_unknown_identifiers = true;
        ctx
    }

    /// The refusal itself: where it was raised is
    /// [`a_refusal_is_raised_at_the_text_it_refuses`]'s question.
    fn member_check(src: &str, ctx: &TypeCtx<'_>) -> Result<String, ExprError> {
        transpile_typed(
            src,
            ExprTarget::Cpp,
            ctx,
            &HashMap::new(),
            InferredType::Unknown,
        )
        .map_err(|refusal| refusal.error)
    }

    /// A refusal is raised at the text it refuses, as a range of the string
    /// the pipeline was handed — the range a placement reads the row, the
    /// column and the author's spelling off. One case per kind of raise
    /// site: the lexer, the token rule, the parser, the name checks, and
    /// each backend's emitter.
    #[test]
    fn a_refusal_is_raised_at_the_text_it_refuses() {
        let mut ctx = member_scope();
        ctx.reject_unknown_callees = true;
        let cases: [(&str, ExprTarget, InferredType, &str); 9] = [
            ("x + conut", ExprTarget::Cpp, InferredType::Unknown, "conut"),
            ("x.foo + 1", ExprTarget::Cpp, InferredType::Unknown, "x.foo"),
            (
                "frame.msg_idd === 1",
                ExprTarget::Cpp,
                InferredType::Unknown,
                "frame.msg_idd",
            ),
            (
                "nope(x) + 1",
                ExprTarget::Cpp,
                InferredType::Unknown,
                "nope",
            ),
            ("x == 1", ExprTarget::Cpp, InferredType::Unknown, "=="),
            ("x + @", ExprTarget::Cpp, InferredType::Unknown, "@"),
            ("x + + )", ExprTarget::Cpp, InferredType::Unknown, ")"),
            (
                "x > 1 ? 1 : 2",
                ExprTarget::Go,
                InferredType::Unknown,
                "x > 1 ? 1 : 2",
            ),
            (
                "0x1F",
                ExprTarget::Rust,
                InferredType::Float { bits: 64 },
                "0x1F",
            ),
        ];
        for (src, target, expected, refused) in cases {
            let Spanned {
                error: refusal,
                span,
            } = transpile_typed(src, target, &ctx, &HashMap::new(), expected).expect_err(src);
            let span = span.unwrap_or_else(|| panic!("{src}: {refusal} carries no span"));
            assert_eq!(&src[span], refused, "{src}: {refusal}");
        }

        // The end of the expression is where a missing token is refused, and
        // nothing is written there — an empty range, not the last token.
        let unclosed = transpile_typed(
            "(x + 1",
            ExprTarget::Cpp,
            &ctx,
            &HashMap::new(),
            InferredType::Unknown,
        )
        .expect_err("an unclosed parenthesis");
        assert_eq!(unclosed.span, Some(6..6));
    }

    /// `<sce:call args>` is split where a call's parentheses would split it:
    /// not inside a nested call or a string, and never around an argument
    /// that is not there.
    #[test]
    fn an_argument_list_splits_only_at_its_own_commas() {
        let list = " clamp(reading, 1), 'a,b' ,x ";
        let arguments: Vec<&str> = argument_ranges(list)
            .expect("three arguments")
            .into_iter()
            .map(|range| &list[range])
            .collect();
        assert_eq!(arguments, ["clamp(reading, 1)", "'a,b'", "x"]);
        assert!(argument_ranges(" ").expect("no arguments").is_empty());

        // An empty argument is refused at the comma that leaves it empty, and
        // a trailing one at the end of the list, where nothing is written.
        for (list, at) in [("a,,b", 2..3), (", a", 0..1), ("a, b,", 5..5)] {
            let refusal = argument_ranges(list).expect_err(list);
            assert_eq!(refusal.span, Some(at), "{list}: {}", refusal.error);
        }
        // Two expressions with no comma between them are not one argument.
        let refusal = argument_ranges("a b").expect_err("a missing comma");
        assert_eq!(refusal.span, Some(2..3), "{}", refusal.error);
    }

    #[test]
    fn a_member_of_a_scalar_is_refused_with_its_declared_type() {
        let err = member_check("x.foo + 1", &member_scope()).unwrap_err();
        assert!(
            matches!(
                &err,
                ExprError::MemberOfNonRecord { name, member, ty: Some("uint8") }
                    if name == "x" && member == "foo"
            ),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            "x has no members (it is declared uint8), so x.foo names nothing"
        );
    }

    #[test]
    fn a_member_of_a_value_inference_cannot_type_is_refused_without_one() {
        let err = member_check("tone.level", &member_scope()).unwrap_err();
        assert!(
            matches!(
                &err,
                ExprError::MemberOfNonRecord { name, member, ty: None }
                    if name == "tone" && member == "level"
            ),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            "tone has no members, so tone.level names nothing"
        );
    }

    /// The base is the whole path: a record's scalar field is a value too.
    #[test]
    fn a_member_of_a_records_scalar_field_is_refused() {
        let err = member_check("frame.msg_id.foo", &member_scope()).unwrap_err();
        assert!(
            matches!(
                &err,
                ExprError::MemberOfNonRecord { name, member, ty: Some("uint32") }
                    if name == "frame.msg_id" && member == "foo"
            ),
            "{err:?}"
        );
    }

    /// A closed record's field and its method are both members.
    #[test]
    fn a_member_a_closed_record_declares_passes() {
        let ctx = member_scope();
        assert!(member_check("frame.msg_id === 1", &ctx).is_ok());
        assert!(member_check("len(frame.encode()) === 4", &ctx).is_ok());
    }

    /// Anything else is refused with the record's whole member set.
    #[test]
    fn a_member_a_closed_record_does_not_declare_is_refused_with_its_members() {
        let err = member_check("frame.msg_idd === 1", &member_scope()).unwrap_err();
        assert!(
            matches!(
                &err,
                ExprError::UnknownMember { record, member, declared }
                    if record == "frame" && member == "msg_idd" && declared == &["encode", "msg_id"]
            ),
            "{err:?}"
        );
        assert_eq!(
            err.to_string(),
            "frame.msg_idd is not a member of frame (declared: encode, msg_id). \
             Did you mean: msg_id?"
        );
    }

    /// An open record's members belong to a pass that knows them.
    #[test]
    fn an_open_records_members_are_not_judged() {
        assert!(member_check("_event.anything === 1", &member_scope()).is_ok());
    }

    /// A statechart guard's scope is the host's: the gate that keeps an
    /// unknown operand legal there keeps this legal too.
    #[test]
    fn an_open_scope_does_not_judge_members() {
        let mut ctx = member_scope();
        ctx.reject_unknown_identifiers = false;
        assert!(member_check("x.foo", &ctx).is_ok());
    }

    // ── transpile_lvalue ───────────────────────────────────────

    #[test]
    fn lvalue_bare_ident_cpp() {
        let mut renames = HashMap::new();
        renames.insert("retryCount", "retryCount_");
        let mut ctx = TypeCtx::new();
        ctx.insert_var("retryCount", int(true, 32));
        let (emitted, ty) =
            transpile_lvalue("retryCount", ExprTarget::Cpp, &ctx, &renames).unwrap();
        assert_eq!(emitted, "retryCount_");
        assert_eq!(ty, int(true, 32));
    }

    #[test]
    fn lvalue_bare_ident_rust() {
        let mut renames = HashMap::new();
        renames.insert("seed", "self.seed");
        let mut ctx = TypeCtx::new();
        ctx.insert_var("seed", InferredType::Bytes);
        let (emitted, ty) = transpile_lvalue("seed", ExprTarget::Rust, &ctx, &renames).unwrap();
        assert_eq!(emitted, "self.seed");
        assert_eq!(ty, InferredType::Bytes);
    }

    #[test]
    fn lvalue_member_access_with_rename() {
        let mut renames = HashMap::new();
        renames.insert("frame.msgId", "self.frame.msg_id");
        let mut ctx = TypeCtx::new();
        ctx.insert_var("frame", InferredType::Unknown);
        ctx.insert_var("frame.msgId", int(false, 32));
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Rust, &ctx, &renames).unwrap();
        assert_eq!(emitted, "self.frame.msg_id");
        assert_eq!(ty, int(false, 32));
    }

    // ── field_renames expansion per language ─────────────────
    //
    // These verify the rename map entries that stateful_import_field_renames
    // produces for each language. The function itself lives in generator.rs
    // but its output feeds into transpile_lvalue via the rename map — so the
    // end-to-end proof that "frame.msgId" emits the correct target-language
    // member path goes through this pipeline. Without these tests, the 5
    // language branches of stateful_import_field_renames have zero coverage
    // (no fixture currently accesses codec fields directly).

    /// Helper: build a TypeCtx + rename map that simulates a stateful codec
    /// import with alias "frame" and field "msgId: uint32", using the rename
    /// expansion that stateful_import_field_renames would produce for the
    /// given language.
    fn codec_field_ctx_and_renames<'a>(
        alias_rename: &'a str,
        field_rename: &'a str,
    ) -> (TypeCtx<'static>, HashMap<&'a str, &'a str>) {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("frame", InferredType::Unknown);
        ctx.insert_var("frame.msgId", int(false, 32));
        let mut renames = HashMap::new();
        renames.insert("frame", alias_rename);
        renames.insert("frame.msgId", field_rename);
        (ctx, renames)
    }

    #[test]
    fn lvalue_field_rename_cpp() {
        // C++: member_name = "frame_", field verbatim
        let (ctx, renames) = codec_field_ctx_and_renames("frame_", "frame_.msgId");
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Cpp, &ctx, &renames).unwrap();
        assert_eq!(emitted, "frame_.msgId");
        assert_eq!(ty, int(false, 32));
    }

    #[test]
    fn lvalue_field_rename_kotlin() {
        // Kotlin: member_name = "frame", field verbatim
        let (ctx, renames) = codec_field_ctx_and_renames("frame", "frame.msgId");
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Kotlin, &ctx, &renames).unwrap();
        assert_eq!(emitted, "frame.msgId");
        assert_eq!(ty, int(false, 32));
    }

    #[test]
    fn lvalue_field_rename_rust() {
        // Rust: "self." + member_name + snake_case field
        let (ctx, renames) = codec_field_ctx_and_renames("self.frame", "self.frame.msg_id");
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Rust, &ctx, &renames).unwrap();
        assert_eq!(emitted, "self.frame.msg_id");
        assert_eq!(ty, int(false, 32));
    }

    #[test]
    fn lvalue_field_rename_go() {
        // Go: "p." + PascalCase member + PascalCase field
        let (ctx, renames) = codec_field_ctx_and_renames("p.Frame", "p.Frame.MsgId");
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Go, &ctx, &renames).unwrap();
        assert_eq!(emitted, "p.Frame.MsgId");
        assert_eq!(ty, int(false, 32));
    }

    #[test]
    fn lvalue_field_rename_python() {
        // Python: "self." + member_name + snake_case field
        let (ctx, renames) = codec_field_ctx_and_renames("self.frame", "self.frame.msg_id");
        let (emitted, ty) =
            transpile_lvalue("frame.msgId", ExprTarget::Python, &ctx, &renames).unwrap();
        assert_eq!(emitted, "self.frame.msg_id");
        assert_eq!(ty, int(false, 32));
    }

    #[test]
    fn lvalue_rejects_call() {
        let err = transpile_lvalue("foo()", ExprTarget::Cpp, &empty_ctx(), &empty_renames());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("call expression"));
    }

    #[test]
    fn lvalue_rejects_binary() {
        let err = transpile_lvalue("a + b", ExprTarget::Cpp, &empty_ctx(), &empty_renames());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("binary operation"));
    }

    #[test]
    fn lvalue_rejects_index() {
        let err = transpile_lvalue("arr[0]", ExprTarget::Cpp, &empty_ctx(), &empty_renames());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("index expression"));
    }

    #[test]
    fn lvalue_rejects_nested_member() {
        let err = transpile_lvalue("a.b.c", ExprTarget::Cpp, &empty_ctx(), &empty_renames());
        assert!(err.is_err());
        assert!(err
            .unwrap_err()
            .to_string()
            .contains("must be a bare identifier"));
    }

    #[test]
    fn lvalue_rejects_empty() {
        let err = transpile_lvalue("", ExprTarget::Cpp, &empty_ctx(), &empty_renames());
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("empty"));
    }
}
