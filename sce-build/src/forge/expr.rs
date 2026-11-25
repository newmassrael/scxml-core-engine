// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::ring_ops::{self, OverflowRule, RingOp};
use crate::forge::rust_optional;
use crate::forge::types::{
    join_arith, join_int, join_shift, CallReading, InferredType, OptionalElem, ShiftCount, TypeCtx,
};
use std::collections::HashMap;
use std::fmt;

/// A refusal of the expression the pipeline was handed, and the range of it
/// the refusal was raised at ([`Spanned`]) — what every entry point below
/// answers with, so the caller holding the attribute can place it.
pub type Refusal = Spanned<ExprError>;

pub(crate) mod rust_tree;
pub(crate) mod swift;

use rust_tree::{RustBinOp, RustExpr, RustStmt, RustUnOp};

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
    /// Swift. Every integer type is the language's own fixed-width one, a checked
    /// operation is `(try Checked.<op>(…))` of the forge runtime, which throws its
    /// failure (docs/adr/0099), and a string is compared over its UTF-8 bytes
    /// (`expr/swift.rs`).
    Swift,
    /// ECMAScript, spelled for the Interpreter's script engine: the lowering
    /// that lets a `datamodel="sce-static"` document run there
    /// (docs/SCE_ACCEPTED_SUBSET.md §2.15). It is NOT a code-generation
    /// backend — there is no `Language` for it, and it is deliberately absent
    /// from [`ExprTarget::ALL`], so a verdict of "lowers natively on all
    /// backends" does not start to require it.
    ///
    /// Every integer is a JavaScript Number. A checked operation is the
    /// runtime library's (`SceStatic.<type>.<helper>`), which computes exactly
    /// and hands back a Number only when it is a safe integer.
    Js,
}

impl ExprTarget {
    /// The code generator's name for this backend — what
    /// [`crate::forge::enum_naming`] spells a variant reference for — or
    /// `None` for the target that is not a code-generation backend
    /// ([`ExprTarget::Js`]).
    pub(crate) fn language(self) -> Option<crate::generator::Language> {
        use crate::generator::Language;
        match self {
            ExprTarget::Cpp => Some(Language::Cpp),
            ExprTarget::Kotlin => Some(Language::Kotlin),
            ExprTarget::Rust => Some(Language::Rust),
            ExprTarget::Go => Some(Language::Go),
            ExprTarget::Python => Some(Language::Python),
            ExprTarget::C => Some(Language::C11),
            ExprTarget::Swift => Some(Language::Swift),
            ExprTarget::Js => None,
        }
    }

    /// The target `language` lowers an expression for: the inverse of
    /// [`Self::language`], decided here once for every caller that holds a
    /// backend and asks for its target.
    pub(crate) fn of(language: crate::generator::Language) -> Self {
        use crate::generator::Language;
        match language {
            Language::Cpp => ExprTarget::Cpp,
            Language::Kotlin => ExprTarget::Kotlin,
            Language::Rust => ExprTarget::Rust,
            Language::Go => ExprTarget::Go,
            Language::Python => ExprTarget::Python,
            Language::C11 => ExprTarget::C,
            Language::Swift => ExprTarget::Swift,
        }
    }

    /// Whether this target's emitter spells an optional value (docs/adr/0055):
    /// `null`, a value of `T` lifted into `optional<T>`, a test of presence, a
    /// proven `x.value`, `x ?? d` and equality between optional values
    /// ([`OptionalForm`], [`InferredType::lifted_into`]). The one answer a
    /// target gives: its `sce-static` lowering asks it too
    /// ([`crate::forge::static_lowering::StaticTarget::lowers_optional`]), and a
    /// target that answers `false` is refused by name before its emitter is
    /// reached ([`reject_unlowered_optional`]). Each target answers `true` in
    /// the step that writes its spellings.
    pub(crate) fn lowers_optional(self) -> bool {
        match self {
            // Go: `optional.Of[T]` of the Go forge runtime ([`go_optional`]).
            ExprTarget::Go => true,
            // The Interpreter: `null`, and `SceStatic.otherwise` / `mapPresent`.
            ExprTarget::Js => true,
            // Python: `Optional[T]`, `None` absent ([`python_optional`]).
            ExprTarget::Python => true,
            // C++: `std::optional<T>` and the runtime's `sce/forge/optional.h`
            // ([`cpp_optional`]).
            ExprTarget::Cpp => true,
            // Kotlin: `T?` ([`kotlin_optional_form`]).
            ExprTarget::Kotlin => true,
            // Rust: `core`'s `Option<T>` ([`rust_optional`]).
            ExprTarget::Rust => true,
            // C11: `sce_optional_<elem>_t` of `sce/forge/optional.h` ([`c_optional`]).
            ExprTarget::C => true,
            // Swift: `T?` ([`swift::swift_optional_form`]).
            ExprTarget::Swift => true,
        }
    }

    /// Whether this target's emitter spells an optional record
    /// (`optional<record:X>`, docs/adr/0055, step 9): the test and the null
    /// comparison of the whole value, and `x.value.f`, a field of the present
    /// record where the build proved it present ([`OptionalForm::Proven`] of a
    /// record, over which the member is read). The one answer a target gives: its
    /// `sce-static` lowering asks it too
    /// ([`crate::forge::static_lowering::StaticTarget::lowers_optional_record`]),
    /// and a target that answers `false` is refused by name before its emitter is
    /// reached ([`reject_unlowered_optional`]). Each target answers `true` in the
    /// step that writes its spellings.
    pub(crate) fn lowers_optional_record(self) -> bool {
        match self {
            // The Interpreter: a record is a plain object, absent is `null`, and
            // `x.value.f` is the member read of the object `x` is.
            ExprTarget::Js => true,
            // Python: a record is a dataclass, absent is `None`, and `x.value.f`
            // is the member read of the record `x` is.
            ExprTarget::Python => true,
            // Kotlin: a record is an immutable data class, `T?` over it holds it
            // or `null`, and `x.value.f` is the member of the record
            // `SceOptional.proven(x)` reads ([`kotlin_optional_form`]).
            ExprTarget::Kotlin => true,
            // Rust: an `Option` of the record's own type; `x.value.f` is read
            // through the runtime's `proven_ref` and written through `proven_mut`,
            // so the record stays where it is held ([`rust_optional::proven_ref`]).
            ExprTarget::Rust => true,
            // C++: `std::optional<Record>`, whose present record `x.value` reads as
            // `(*x)` and over which the member is read ([`cpp_optional`]).
            ExprTarget::Cpp => true,
            // Go: `optional.Of[R]` over the record's own struct, and `x.value.f`
            // is the member `f` of what `x.Proven()` answers ([`go_optional`]).
            ExprTarget::Go => true,
            // C11: `sce_optional_record_<type>_t` of `sce/forge/optional.h`, whose
            // `.value` is the struct, and over which `.f` reads a field
            // ([`c_present_read`]).
            ExprTarget::C => true,
            // Swift: a record is a struct, `T?` over it holds it or `nil`, and
            // `x.value.f` is the member of the record `SceOptional.proven(x)` reads
            // ([`swift::swift_optional_form`]).
            ExprTarget::Swift => true,
        }
    }

    /// How this target spells a name the author gave a local — a parameter or a
    /// variable of an algorithm, the item or the index of a `<foreach>` — where
    /// it is declared and where an expression reads it. The emitter's `Ident`
    /// and the loop a `<foreach>` lowers to both read it, and a declaration a
    /// template writes reads `forge_local_id`, which it is, so a local is never
    /// bound under one spelling and read under another (a loop that bound
    /// `laterAt` and a condition that read `later_at`). The script engine's
    /// ECMAScript keeps the name as written.
    pub(crate) fn local_spelling(self, id: &str) -> String {
        match self.language() {
            Some(language) => crate::forge::generator::forge_local_id(id, language),
            None => id.to_string(),
        }
    }

    /// Every backend SCE generates code for. This is the canonical list a
    /// "lowers natively on all backends" verdict must exercise — checking
    /// a proper subset (historically just Rust + Go) is a proxy that lies
    /// the moment one backend's emitter can fail where another cannot
    /// (e.g. an unrepresentable `bytes` form). Kept exhaustive by
    /// `expr_target_all_contains_every_variant`.
    pub(crate) const ALL: [ExprTarget; 7] = [
        ExprTarget::Cpp,
        ExprTarget::Kotlin,
        ExprTarget::Rust,
        ExprTarget::Go,
        ExprTarget::Python,
        ExprTarget::C,
        ExprTarget::Swift,
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

/// The declared type of the place a value lands in — `None` for a hint,
/// where nothing is declared and so nothing can fail to fit.
fn slot_type(expected: Expected) -> Option<InferredType> {
    match expected {
        Expected::Slot(ty) => Some(ty),
        Expected::Hint(_) => None,
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
    /// Stored in a place that owns it — a statechart's variable, a record's
    /// field, a list's element. Told from [`Self::Returned`] by what a rename
    /// has made of a variable read: it is a `Raw` fragment, exactly as a call
    /// is, but a call's value is a fresh `String` and a variable read is the
    /// field itself, which moving out of would take it from the machine. So
    /// here nothing is trusted to be owned already.
    Owned,
    /// Copied into a place by the place's own constructor, which takes the
    /// value borrowed: a Rust machine's bounded text and byte string
    /// ([`transpile_into_copied`]). Told from [`Self::Owned`] so the value is
    /// not made owned only for the constructor to borrow it again.
    Copied,
}

fn transpile_at(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: Expected,
    position: Position,
) -> Result<String, Refusal> {
    lower_at(expr, target, ctx, renames, expected, position).map(|lowered| lowered.text)
}

/// An expression lowered where its failures are received
/// ([`TypeCtx::receives_failures`]), and whether the code can record one — a
/// checked integer operation, or a call of a callee that declares
/// `may-fail`.
pub struct Receiving {
    pub text: String,
    pub can_fail: bool,
}

/// [`transpile_into`] for a place that receives the failures of what it
/// runs (SCE_FORGE.md §3.4.1) and must know whether there are any: a
/// `sce-static` statechart's statement or guard turns one into
/// `error.execution`, and wraps only the expressions that can fail.
pub fn transpile_into_receiving(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: InferredType,
) -> Result<Receiving, Refusal> {
    lower_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Slot(expected),
        Position::Operand,
    )
    .map(|lowered| Receiving {
        text: lowered.text,
        can_fail: lowered.can_fail,
    })
}

/// [`transpile_into_receiving`] for a place that OWNS what it is given: a
/// variable, a record's field, a list's element. Rust holds a `string` there as
/// a `String`, and a string inside an expression is borrowed, so the value is
/// made owned on its way in ([`emit_rust_owned_str`]) — which a place that
/// only reads what it is handed, a host call's `&str` parameter or a guard's
/// operand, must NOT do. Every other target and type emits exactly what
/// [`transpile_into_receiving`] does.
pub fn transpile_into_owned(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: InferredType,
) -> Result<Receiving, Refusal> {
    lower_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Slot(expected),
        Position::Owned,
    )
    .map(|lowered| Receiving {
        text: lowered.text,
        can_fail: lowered.can_fail,
    })
}

/// [`transpile_into_owned`] for a text or a byte string written to a place
/// that copies it through its own checked constructor ([`Position::Copied`]):
/// a Rust machine's `SceString<N>` and `SceBytes<N>` (docs/adr/0086, section
/// 7.4). A literal lands in a byte-string slot as it does in an owned one, and
/// the value is borrowed whole, since the constructor makes the copy.
pub fn transpile_into_copied(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: InferredType,
) -> Result<Receiving, Refusal> {
    lower_at(
        expr,
        target,
        ctx,
        renames,
        Expected::Slot(expected),
        Position::Copied,
    )
    .map(|lowered| Receiving {
        text: lowered.text,
        can_fail: lowered.can_fail,
    })
}

/// What [`lower_at`] produced: the code, and whether it can fail.
struct Lowered {
    text: String,
    can_fail: bool,
}

/// Whether `node` or anything under it can record a failure: a checked
/// integer operation, or a call of a callee that declares `may-fail`. An
/// operation proven at build time ([`Discharge::AtBuild`]) records none.
fn can_fail(node: &TypedExpr) -> bool {
    matches!(
        node.kind,
        ExprKind::Checked {
            discharge: Discharge::AtRun,
            ..
        } | ExprKind::Call { fails: true, .. }
    ) || node.children().into_iter().any(can_fail)
}

fn lower_at(
    expr: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    expected: Expected,
    position: Position,
) -> Result<Lowered, Refusal> {
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
    judge_value(&mut ast, slot, expr)?;
    land_reals_in_slot(&mut ast, expected);
    if matches!(position, Position::Owned | Position::Copied) && expected == InferredType::Bytes {
        land_strings_in_bytes_slot(&mut ast, expr)?;
    }
    if matches!(target, ExprTarget::Js) && !ctx.exact_integers {
        bytes_literals_as_text(&mut ast);
    }
    // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is not
    // known below its width is the total shift, or a checked one. The
    // Interpreter's library settles every shift itself (`SceStatic.U8.shl`).
    if !matches!(target, ExprTarget::Js) {
        settle_shifts(&mut ast, ctx, expr);
    }
    if ctx.receives_failures {
        check_integer_arithmetic(&mut ast, expected, Discharge::AtRun);
        check_integer_indexing(&mut ast, Discharge::AtRun);
        check_integer_narrowing(&mut ast, slot_type(slot));
        if matches!(target, ExprTarget::C) {
            views_for_bytes_equality(&mut ast);
        }
    } else if ctx.integers_proven && matches!(target, ExprTarget::Rust) {
        prove_integer_operations(&mut ast, expected);
    }
    if ctx.rounds_to_single {
        round_to_single(&mut ast, target, expected);
    }

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
    let text = match target {
        ExprTarget::Cpp => emit_cpp(&ast, expected)?,
        ExprTarget::Kotlin => emit_kotlin(&ast, expected)?,
        ExprTarget::Rust if position == Position::Returned && expected == InferredType::Str => {
            emit_rust_returned_str(&ast)?.to_string()
        }
        ExprTarget::Rust
            if position == Position::Copied
                && matches!(expected, InferredType::Str | InferredType::Bytes) =>
        {
            emit_rust_copied(&ast, expected)?.to_string()
        }
        ExprTarget::Rust if position == Position::Owned && expected == InferredType::Str => {
            emit_rust_owned_str(&ast)?.to_string()
        }
        ExprTarget::Rust if position == Position::Owned && expected == InferredType::Bytes => {
            emit_rust_owned_bytes(&ast)?.to_string()
        }
        ExprTarget::Rust if position == Position::Owned && expected.is_optional() => {
            emit_rust_owned_optional(&ast, expected)?.to_string()
        }
        // A text or a byte string that may be absent, returned, leaves the function
        // as the owned value its signature declares, as a present one does: an
        // expression reads one borrowed (docs/adr/0055).
        ExprTarget::Rust
            if position == Position::Returned
                && matches!(
                    expected,
                    InferredType::Optional(OptionalElem::Str | OptionalElem::Bytes)
                ) =>
        {
            emit_rust_owned_optional(&ast, expected)?.to_string()
        }
        ExprTarget::Rust => emit_rust(&ast, expected)?.to_string(),
        ExprTarget::Go => emit_go(&ast, expected)?,
        ExprTarget::Swift => swift::emit(&ast, expected)?,
        ExprTarget::Python => emit_python(&ast, expected)?,
        ExprTarget::C if position == Position::Owned && expected == InferredType::Bytes => {
            emit_c_owned_bytes(&ast)?
        }
        ExprTarget::C => emit_c(&ast, expected)?,
        ExprTarget::Js => {
            let text = emit_js(&ast, expected)?;
            if !ctx.exact_integers && js_value_may_be_wide(&ast, expected) {
                format!("{}.out({text})", crate::forge::static_js::RUNTIME_GLOBAL)
            } else {
                text
            }
        }
    };
    Ok(Lowered {
        text,
        can_fail: can_fail(&ast),
    })
}

/// Whether a value this expression gives, written for the Interpreter, may hold
/// an integer a Number does not — a 64-bit integer, or a record or a list that
/// may carry one. Such a value is computed exactly while the expressions that
/// make it run, and has to pass the library's `out` before the Interpreter's
/// data model can take it, which loses a BigInt without a word.
fn js_value_may_be_wide(ast: &TypedExpr, expected: InferredType) -> bool {
    use crate::forge::quantity::NumericBaseType;
    match ast.ty {
        InferredType::Int { bits: 64, .. } | InferredType::Record(_) | InferredType::List(_) => {
            true
        }
        // An optional value is `null` or its present value (docs/adr/0055), so
        // one of a 64-bit integer, or of a record, may be wide when present.
        InferredType::Optional(
            OptionalElem::Number(NumericBaseType::Int { bits: 64, .. }) | OptionalElem::Record,
        ) => true,
        // A literal lands as the type of its place, or as the present value of
        // an optional place.
        InferredType::UntypedInt => matches!(
            expected.unwrap_optional(),
            InferredType::Int { bits: 64, .. }
        ),
        _ => false,
    }
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
    lower_build_limits(&mut ast, ctx)?;
    infer_types(&mut ast, ctx);
    Ok(ast.ty)
}

/// The two names a build's bounds are read under, `_x.limits.<name>` (docs/adr/0050):
/// `_x` is the system variable an SCXML session reserves for what a platform adds,
/// which no document declares, and `limits` the bounds `sce:limit` names.
const BUILD_LIMITS: (&str, &str) = ("_x", "limits");

/// The name `expr` reads a build's bound by, when it is `_x.limits.<name>`.
fn build_limit_name(expr: &TypedExpr) -> Option<&str> {
    let ExprKind::Member { object, property } = &expr.kind else {
        return None;
    };
    let ExprKind::Member {
        object: root,
        property: limits,
    } = &object.kind
    else {
        return None;
    };
    let ExprKind::Ident(x) = &root.kind else {
        return None;
    };
    (x == BUILD_LIMITS.0 && limits == BUILD_LIMITS.1).then_some(property.as_str())
}

/// Each `_x.limits.<name>` of `expr` written as the integer the build gave the
/// bound the name stands for ([`TypeCtx::limits`]): a number the build decided,
/// the same on every backend and in the Interpreter's lowering, which then stands
/// wherever an integer literal does and is held to the place it lands in as one
/// is. Done first, so every pass after it sees a literal.
///
/// Where the scope is closed, a name no element of the document's build names is
/// refused naming the ones it does, and a name two of its elements resolve to two
/// numbers is refused naming both: neither has one value to read. Anywhere else the
/// read is left as it is written.
fn lower_build_limits(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) -> Result<(), Refusal> {
    if let Some(name) = build_limit_name(expr).map(str::to_string) {
        return match ctx.limits.get(name.as_str()) {
            Some(Ok(value)) => {
                expr.kind = ExprKind::NumberLit(value.to_string());
                Ok(())
            }
            Some(Err((one, other))) => Err(ExprError::UnsupportedConstruct {
                construct: format!(
                    "a read of the bound `{name}`, which two elements of this build name and \
                     resolve to two numbers, {one} and {other}: one name is one bound, so write \
                     one number beside every `sce:limit=\"{name}\"`, or give the build one with \
                     `--limit {name}=N`"
                ),
                observed: Some(format!("{}.{}.{name}", BUILD_LIMITS.0, BUILD_LIMITS.1)),
            }
            .at(expr.span.clone())),
            None if ctx.reject_unknown_identifiers => {
                let mut declared: Vec<String> =
                    ctx.limits.keys().map(|name| name.to_string()).collect();
                declared.sort();
                Err(ExprError::UnknownMember {
                    record: format!("{}.{}", BUILD_LIMITS.0, BUILD_LIMITS.1),
                    member: name,
                    declared,
                }
                .at(expr.span.clone()))
            }
            None => Ok(()),
        };
    }
    for child in expr.children_mut() {
        lower_build_limits(child, ctx)?;
    }
    Ok(())
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
    judge_value(&mut ast, expected, expr)?;
    settle_shifts(&mut ast, ctx, expr);
    if ctx.receives_failures {
        check_integer_arithmetic(&mut ast, expected.ty(), Discharge::AtRun);
        check_integer_indexing(&mut ast, Discharge::AtRun);
        check_integer_narrowing(&mut ast, slot_type(expected));
    }
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
/// 4. An enum's conversions (`<alias>.toNumber`, `<alias>.fromNumber`) lowered
///    to the functions the context spells them with, each call whose function
///    answers something other than its value read as its signature says
///    ([`lower_call_readings`]), and then the enum variants to this backend's
///    spelling, once the names are known good; rename leaves the resulting
///    `Raw` alone.
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
    // Judged on the tree as the author wrote it: a rename makes a read a
    // fragment of no type.
    reject_unlowered_optional(ast, target)?;
    if !lowerings.is_empty() {
        lower_stateful_import_calls(ast, lowerings);
    }
    lower_enum_conversions(ast, ctx, target);
    // docs/adr/0092, "Decided while handing a union from one algorithm to
    // another": a union handed whole to another algorithm's parameter, judged
    // above, spelled as the target hands a value it keeps.
    lower_union_arguments(ast, ctx, target);
    lower_call_crossings(ast, ctx, target);
    append_map_seeds(ast, ctx);
    lower_call_readings(ast, ctx, target);
    lower_enum_variant_refs(ast, ctx, target)?;
    // docs/adr/0108, D3: a read of a map or a set, judged above, spelled on the
    // one lowering that holds one, while each map is still the name the author
    // wrote.
    if matches!(target, ExprTarget::Js) {
        lower_keyed_reads_for_js(ast, ctx)?;
    }
    if matches!(target, ExprTarget::Rust) {
        lower_keyed_reads_for_rust(ast, ctx);
    }
    if matches!(target, ExprTarget::C) {
        lower_keyed_reads_for_c(ast, ctx)?;
    }
    // docs/adr/0092, section 6.1: a read of a union's member, judged above
    // where its case is proven, spelled by the cases the facts leave, on the
    // backend whose machine holds a union.
    if matches!(
        target,
        ExprTarget::Rust
            | ExprTarget::Swift
            | ExprTarget::Kotlin
            | ExprTarget::Go
            | ExprTarget::Cpp
            | ExprTarget::Python
            | ExprTarget::C
    ) {
        lower_union_reads(ast, ctx, &ctx.proven, target)?;
    }
    // A C string is a bounded buffer: a join of strings the data model sizes is
    // written into one the lowering sizes, and one it cannot size is refused at
    // emission, by name.
    if matches!(target, ExprTarget::C) && ctx.joins_into_buffers {
        lower_string_concatenation(ast, ctx);
    }
    // A text a composed value holds is a buffer of its bound in C, read as the
    // buffer's text: after the joins, whose operands are read so too, and before
    // rename, while each place is still the path the author wrote.
    if matches!(target, ExprTarget::C) {
        read_composed_texts_through_buffers(ast, ctx, renames);
    }
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
///
/// The `eq` builtin leaves it as the comparison it means ([`lower_bytes_eq`]),
/// once its arguments are judged: no emitter has an `eq` to call.
fn resolve_names(ast: &mut TypedExpr, ctx: &TypeCtx<'_>, source: &str) -> Result<(), Refusal> {
    lower_build_limits(ast, ctx)?;
    lower_previous(ast, ctx);
    infer_types(ast, ctx);
    reject_unknown_callees(ast, ctx)?;
    reject_unknown_names(ast, ctx)?;
    reject_unnamed_record_elements(ast, ctx, source)?;
    reject_union_elements_as_parts(ast, ctx, source, true)?;
    reject_misdirected_indexing(ast, ctx, source)?;
    reject_misread_keys(ast, ctx, source)?;
    reject_unreadable_element_fields(ast, ctx, source)?;
    reject_records_as_operands(ast, ctx, source, RecordPlace::Whole)?;
    reject_record_text_misuse(ast, ctx, source, false)?;
    reject_buffers_as_values(ast, source)?;
    reject_optional_misuse(ast, &ctx.proven, source)?;
    reject_unproven_union_reads(ast, ctx, &ctx.proven, source)?;
    reject_call_argument_mismatches(ast, ctx, source)?;
    reject_ill_typed_total_operations(ast, source)?;
    reject_union_arguments(ast, ctx, source)?;
    reject_composed_operands(ast, ctx, source, ReadThrough::Whole)?;
    lower_bytes_eq(ast, ctx);
    Ok(())
}

/// Where a node stands for [`reject_composed_operands`]: whether a composed
/// value may stand there.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadThrough {
    /// The whole expression, which the statement that holds it judges: a
    /// `<foreach>`'s list, a value taken whole by an `<assign>`.
    Whole,
    /// A place a composed value is read through: the object of a field read
    /// or of an index, the argument of `len`, the subject of a test against
    /// `null`.
    Part,
    /// Anywhere else: an operand, an index, a call's argument, a branch.
    Operand,
}

/// Refuse a composed value — a list, a record, or an optional value of either,
/// at a place the scope types by the one value type ([`TypeCtx::value_of`]) —
/// as an operand, compared or combined (docs/adr/0086, section 6.1): no
/// backend compares or combines two lists or two records alike. Such a value is
/// read through its fields, its elements and its length, tested against `null`
/// where it may be absent, and taken whole only by the statement that holds the
/// expression ([`ReadThrough::Whole`]).
///
/// A scope that holds no value of the one value type (every kind but a
/// `sce-static` statechart) holds no such place, and nothing is refused.
fn reject_composed_operands(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
    stands: ReadThrough,
) -> Result<(), Refusal> {
    if stands == ReadThrough::Operand {
        let composed = place_path(expr)
            .and_then(|path| ctx.value_of(&path))
            .filter(crate::forge::value_path::Place::is_composed);
        if let Some(place) = composed {
            let observed = expr
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string);
            return Err(ExprError::UnsupportedConstruct {
                construct: format!(
                    "`{}`, a value of `{}`, as an operand (a list, a record or a union is read \
                     through its fields, its elements, its length and its tag, and is taken \
                     whole only into a variable whose type it fits or by a <foreach>)",
                    observed.as_deref().unwrap_or_default(),
                    place.ty
                ),
                observed,
            }
            .at(expr.span.clone()));
        }
    }
    let null_test = |subject: &TypedExpr, other: &TypedExpr| {
        matches!(other.kind, ExprKind::NullLit) && subject.ty.is_optional()
    };
    // An argument of a parameter that takes a union whole stands as the union
    // the caller hands by name, judged by the rule of a union argument
    // ([`reject_union_arguments`]) and by no rule of an operand.
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if let Some(unions) = union_parameters(callee, ctx) {
            reject_composed_operands(callee, ctx, source, ReadThrough::Operand)?;
            for (index, arg) in args.iter().enumerate() {
                if unions.get(index).is_some_and(Option::is_some) {
                    continue;
                }
                reject_composed_operands(arg, ctx, source, ReadThrough::Operand)?;
            }
            return Ok(());
        }
    }
    match &expr.kind {
        ExprKind::Member { object, .. } => {
            reject_composed_operands(object, ctx, source, ReadThrough::Part)
        }
        ExprKind::Index { object, index } => {
            reject_composed_operands(object, ctx, source, ReadThrough::Part)?;
            reject_composed_operands(index, ctx, source, key_stands(object, ctx))
        }
        ExprKind::Call { callee, args, .. } if is_len_builtin(callee, args) => {
            reject_composed_operands(&args[0], ctx, source, ReadThrough::Part)
        }
        // A map or a set is read through by `len` and `has`, and a key stands
        // where [`key_stands`] says (docs/adr/0108, D3).
        ExprKind::Call { callee, args, .. } if keyed_builtin(callee, args, ctx).is_some() => {
            reject_composed_operands(&args[0], ctx, source, ReadThrough::Part)?;
            let stands = key_stands(&args[0], ctx);
            args[1..]
                .iter()
                .try_for_each(|key| reject_composed_operands(key, ctx, source, stands))
        }
        ExprKind::Binary {
            op: BinOp::StrictEq | BinOp::StrictNeq,
            left,
            right,
        } if null_test(left, right) || null_test(right, left) => {
            let stands = |node: &TypedExpr| {
                if matches!(node.kind, ExprKind::NullLit) {
                    ReadThrough::Operand
                } else {
                    ReadThrough::Part
                }
            };
            reject_composed_operands(left, ctx, source, stands(left))?;
            reject_composed_operands(right, ctx, source, stands(right))
        }
        _ => expr.children().into_iter().try_for_each(|child| {
            reject_composed_operands(child, ctx, source, ReadThrough::Operand)
        }),
    }
}

/// The union each parameter of the function `callee` names takes whole
/// ([`crate::forge::types::FuncSig::unions`]), when `callee` is the name of a
/// function that takes one: an imported algorithm with a parameter of
/// `union:<alias>`
/// (docs/adr/0092, "Decided while handing a union from one algorithm to
/// another"). `None` for every other callee.
fn union_parameters<'c>(
    callee: &TypedExpr,
    ctx: &'c TypeCtx<'_>,
) -> Option<&'c [Option<crate::forge::types::UnionParam>]> {
    let ExprKind::Ident(name) = &callee.kind else {
        return None;
    };
    let signature = ctx.lookup_func(name)?;
    signature
        .unions
        .iter()
        .any(Option::is_some)
        .then_some(signature.unions.as_slice())
}

/// Refuse every argument handed to a parameter that takes a union whole which
/// is not the name of a union the caller holds of the parameter's document
/// ([`judge_union_argument`]), at any depth of `expr`. The one rule of a union
/// argument, read by the judge of an algorithm's body before any backend
/// ([`judge_narrowed_use`]) and by every lowering ([`resolve_names`]).
fn reject_union_arguments(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if let (ExprKind::Ident(name), Some(unions)) = (&callee.kind, union_parameters(callee, ctx))
        {
            for (arg, param) in args.iter().zip(unions) {
                if let Some(param) = param {
                    judge_union_argument(arg, name, param, ctx, source)?;
                }
            }
        }
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_union_arguments(child, ctx, source))
}

/// The name of the union `arg` hands to the parameter of `callee` that takes
/// the union `param` whole (docs/adr/0092, "Decided while handing a union from
/// one algorithm to another"), or its refusal. A union stands whole there by
/// the name of the parameter or the local of the caller that holds it, and only
/// a union of the parameter's own document, by its identity, as a record
/// argument is held to its schema's document (SCE_FORGE.md §4.12): a union of
/// another document, and anything that is not such a name (a call that answers
/// a union, a conditional, a number), is refused naming the document the
/// parameter takes.
fn judge_union_argument<'e>(
    arg: &'e TypedExpr,
    callee: &str,
    param: &crate::forge::types::UnionParam,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<&'e str, Refusal> {
    let observed = arg
        .span
        .clone()
        .and_then(|span| source.get(span))
        .unwrap_or(source)
        .trim()
        .to_string();
    let construct = match &arg.kind {
        ExprKind::Ident(name) => {
            let held = ctx.value_of(name).and_then(|place| {
                crate::forge::value_path::union_of(ctx.value_scope, &place)
                    .map(|(union, alias)| (union.identity.clone(), union.name.clone(), alias))
            });
            match held {
                Some((identity, _, _)) if identity == param.identity => return Ok(name.as_str()),
                Some((_, document, alias)) => format!(
                    "`{observed}`, a value of union:{alias} of the union document \
                     `{document}`, handed to `{callee}`, whose parameter takes a union of the \
                     document `{}` (a union is handed only to a parameter of its own document)",
                    param.document
                ),
                None => union_argument_unnamed(&observed, callee, param),
            }
        }
        _ => union_argument_unnamed(&observed, callee, param),
    };
    Err(ExprError::UnsupportedConstruct {
        construct,
        observed: Some(observed),
    }
    .at(arg.span.clone()))
}

/// What a refusal of `observed`, no name of a union, handed to the parameter of
/// `callee` that takes the union `param` whole, says ([`judge_union_argument`]).
fn union_argument_unnamed(
    observed: &str,
    callee: &str,
    param: &crate::forge::types::UnionParam,
) -> String {
    format!(
        "`{observed}` handed to `{callee}`, whose parameter takes a union of the document `{}` \
         whole (an algorithm hands a union by the name of the parameter or the local that \
         holds it)",
        param.document
    )
}

/// [`judge_union_argument`] of the argument written `written`, as a
/// `<sce:call args>` lists it, where `ctx` holds: what the judge of an
/// algorithm's body holds each argument of a `<sce:call>` to whose parameter
/// takes a union, before any backend.
pub(crate) fn judge_union_argument_written(
    written: &str,
    callee: &str,
    param: &crate::forge::types::UnionParam,
    ctx: &TypeCtx<'_>,
) -> Result<(), Refusal> {
    let written = written.trim();
    let ast = parse_to_ast(written)?;
    judge_union_argument(&ast, callee, param, ctx, written).map(|_| ())
}

/// The union an element of the list `ty` is, by the alias the scope names it
/// under, when `ty` is a `list<union:<alias>>` parameter or local of an
/// algorithm (docs/adr/0099, "Still open" item 2); `None` for every other type.
pub(crate) fn listed_union(ty: InferredType) -> Option<&'static str> {
    match ty {
        InferredType::List(elem) | InferredType::ListBuffer(elem) => elem.union_alias(),
        _ => None,
    }
}

/// Judge the `init` written `written` of a union local of `union:<alias>`
/// copied whole (docs/adr/0099, section 5.2): an element of a list of that
/// union read at an index, `xs[i]`, where `xs` is a `list<union:…>` parameter
/// or local whose union is the local's by its document's identity, as a union
/// argument is held to its parameter's ([`judge_union_argument`]). The element
/// is read by the backend's checked read, so an index outside the list fails
/// `out-of-range`; anything else (a union by its name, a call, a field) is
/// refused naming what a union local takes, since a union local is built from
/// its `<sce:set>`s or copied from an element, and nothing else.
pub(crate) fn judge_union_element_copy(
    written: &str,
    alias: &str,
    ctx: &TypeCtx<'_>,
) -> Result<(), Refusal> {
    let written = written.trim();
    let mut ast = parse_to_ast(written)?;
    infer_types(&mut ast, ctx);
    let identity = |of: &str| {
        ctx.value_scope
            .and_then(|scope| scope.unions.get(of))
            .map(|union| union.identity.clone())
    };
    let listed = match &ast.kind {
        ExprKind::Index { object, .. } => listed_union(object.ty),
        _ => None,
    };
    let construct = match listed {
        // The element is read by the checked read of an index, which fails
        // `out-of-range`: only a body that may fail has the channel for it.
        Some(_) if !ctx.receives_failures => format!(
            "`{written}`, an element copied into a union local of union:{alias} in an algorithm \
             that does not declare `may-fail` (the element is read by the checked read of its \
             index, which fails `out-of-range` where the index is outside the list)"
        ),
        Some(held) if identity(held).is_some() && identity(held) == identity(alias) => {
            return Ok(())
        }
        Some(held) => format!(
            "`{written}`, an element of a list of union:{held}, copied into a union local of \
             union:{alias} (a union local is copied from an element of a list of its own union)"
        ),
        None => format!(
            "`{written}` as the value of a union local of union:{alias} (a union local is built \
             from its <sce:set>s or copied whole from an element of a list of its union, \
             `xs[i]`)"
        ),
    };
    Err(ExprError::UnsupportedConstruct {
        construct,
        observed: Some(written.to_string()),
    }
    .at(ast.span.clone()))
}

/// The argument written `written`, as a `<sce:call args>` lists it, handed to
/// the parameter of `callee` that takes the union `param` whole, judged as an
/// expression's argument is ([`judge_union_argument`]) and spelled as `target`
/// hands a union it keeps ([`handed_union`]).
pub(crate) fn transpile_union_argument(
    written: &str,
    target: ExprTarget,
    ctx: &TypeCtx<'_>,
    callee: &str,
    param: &crate::forge::types::UnionParam,
) -> Result<String, Refusal> {
    let written = written.trim();
    let ast = parse_to_ast(written)?;
    let name = judge_union_argument(&ast, callee, param, ctx, written)?;
    Ok(handed_union(target, name))
}

/// How `target` hands the union a caller holds as `name` to a parameter of
/// another algorithm (docs/adr/0092, "Decided while handing a union from one
/// algorithm to another"). Every backend takes it by value, the type the
/// union's own unit declares on both sides. Rust's union is no `Copy` type and
/// the caller may read its own after the call, so it hands a clone; C, C++, Go
/// and Swift copy the value they pass, and Kotlin, Python and the Interpreter
/// hand the very object, which no algorithm writes ("Decided while landing step
/// 5" item 3), so each hands the name as it is.
pub(crate) fn handed_union(target: ExprTarget, name: &str) -> String {
    let held = target.local_spelling(name);
    match target {
        ExprTarget::Rust => format!("{held}.clone()"),
        ExprTarget::Cpp
        | ExprTarget::Kotlin
        | ExprTarget::Go
        | ExprTarget::Python
        | ExprTarget::C
        | ExprTarget::Swift
        | ExprTarget::Js => held,
    }
}

/// Each union an expression hands whole to a parameter that takes one
/// ([`reject_union_arguments`] admitted it: the name of a union the caller
/// holds), spelled as `target` hands it ([`handed_union`]).
fn lower_union_arguments(expr: &mut TypedExpr, ctx: &TypeCtx<'_>, target: ExprTarget) {
    for child in expr.children_mut() {
        lower_union_arguments(child, ctx, target);
    }
    let ExprKind::Call { callee, args, .. } = &mut expr.kind else {
        return;
    };
    let Some(unions) = union_parameters(callee, ctx) else {
        return;
    };
    for (arg, param) in args.iter_mut().zip(unions) {
        if param.is_none() {
            continue;
        }
        let handed = match &arg.kind {
            ExprKind::Ident(name) => handed_union(target, name),
            _ => continue,
        };
        arg.kind = ExprKind::Raw(handed);
    }
}

/// Refuse a field of the element a list of records is indexed at that the
/// element's schema does not declare: the place `<list>[].<field>` is typed by
/// walking the list's type ([`TypeCtx::value_of`]), at any depth, so a field the
/// schema does not have is no place, and would reach every backend as an
/// operand of no type.
fn reject_unreadable_element_fields(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    if let ExprKind::Member { object, property } = &expr.kind {
        // The value at a key of a map of records is read as an element of a
        // list of records is (docs/adr/0108, D3).
        let record_value = |map: &TypedExpr| {
            keyed_place(map, ctx)
                .and_then(|held| held.map_value())
                .is_some_and(|value| {
                    matches!(value.ty, crate::forge::value_type::ValueType::Record { .. })
                })
        };
        let indexed = match &object.kind {
            ExprKind::Index { object: list, .. }
                if matches!(list.ty, InferredType::List(elem) if elem.is_record())
                    || record_value(list) =>
            {
                place_path(list)
            }
            _ => None,
        };
        if let Some(list) = indexed {
            let path = format!(
                "{list}{}{property}",
                crate::forge::type_ctx::INDEXED_ELEMENT
            );
            if expr.ty == InferredType::Unknown && !ctx.declares(&path) {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!(
                        "`{list}[…].{property}`, a field the element's schema does not declare \
                         (a field the schema does not declare is read nowhere)"
                    ),
                    observed: expr
                        .span
                        .clone()
                        .and_then(|span| source.get(span))
                        .map(str::to_string),
                }
                .at(expr.span.clone()));
            }
        }
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_unreadable_element_fields(child, ctx, source))
}

/// The members of `union`: its tag, and every field a case's record declares,
/// each once, in the enum's order of the cases (docs/adr/0092, section 6.1).
fn union_members(union: &crate::forge::value_type::ScopedUnion) -> Vec<String> {
    let mut members = vec![union.tag.clone()];
    for case in &union.cases {
        let fields = case
            .record
            .as_deref()
            .and_then(|record| union.scope.records.get(record))
            .map(|record| record.fields.as_slice())
            .unwrap_or_default();
        for (id, _) in fields {
            if !members.contains(id) {
                members.push(id.clone());
            }
        }
    }
    members
}

/// Refuse a read of a union's field where the build has not proven the union
/// in cases that each declare it with one type (docs/adr/0092, section 6.1,
/// `expression/union-read-unproven`): the tag is read anywhere, and a case's
/// field only where a test of the tag that holds there (`facts`) leaves cases
/// that each declare it. A union read through an element at an index is no
/// subject a test names, so only its tag is read there, and a field no case
/// declares is no member of it.
///
/// Within the expression the facts grow as [`reject_optional_misuse`]'s do,
/// by one rule for both kinds of fact ([`crate::forge::narrowing::proves`]):
/// the right operand of `a && b` is read where `a` is true, of `a || b` where
/// `a` is false, and the branches of `c ? t : e` where `c` is true and false.
///
/// A scope that holds no value of the one value type holds no union, and
/// nothing is refused.
fn reject_unproven_union_reads(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    facts: &crate::forge::narrowing::Facts,
    source: &str,
) -> Result<(), Refusal> {
    use crate::forge::narrowing::proves;
    if let ExprKind::Member { object, property } = &expr.kind {
        let held = place_path(object).and_then(|path| ctx.value_of(&path));
        let union = held
            .as_ref()
            .and_then(|place| crate::forge::value_path::union_of(ctx.value_scope, place));
        if let Some((union, alias)) = union {
            let read = expr
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map_or_else(|| property.clone(), str::to_string);
            if *property != union.tag {
                let declared = union_members(union);
                if !declared.iter().any(|field| field == property) {
                    let mut declared = declared;
                    declared.sort_unstable();
                    return Err(ExprError::UnknownMember {
                        record: place_path(object).unwrap_or_default(),
                        member: property.clone(),
                        declared,
                    }
                    .at(expr.span.clone()));
                }
                let subject = dotted_path(object);
                if let Some(refusal) = crate::forge::value_path::union_read_refusal(
                    ctx.value_scope,
                    union,
                    &alias,
                    subject.as_deref(),
                    property,
                    Some(facts),
                ) {
                    return Err(ExprError::UnionReadUnproven(Box::new(
                        crate::forge::error::UnionReadUnproven {
                            read,
                            union: refusal.union,
                            declaring: refusal.declaring,
                            test: refusal.test,
                        },
                    ))
                    .at(expr.span.clone()));
                }
            }
        }
    }
    match &expr.kind {
        ExprKind::Binary {
            op: op @ (BinOp::And | BinOp::Or),
            left,
            right,
        } => {
            reject_unproven_union_reads(left, ctx, facts, source)?;
            let when = *op == BinOp::And;
            reject_unproven_union_reads(right, ctx, &facts.and(&proves(left, when)), source)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            reject_unproven_union_reads(condition, ctx, facts, source)?;
            reject_unproven_union_reads(
                consequent,
                ctx,
                &facts.and(&proves(condition, true)),
                source,
            )?;
            reject_unproven_union_reads(
                alternate,
                ctx,
                &facts.and(&proves(condition, false)),
                source,
            )
        }
        _ => expr
            .children()
            .into_iter()
            .try_for_each(|child| reject_unproven_union_reads(child, ctx, facts, source)),
    }
}

/// Spell each read of a union's member as the machine of `target` reads it
/// (docs/adr/0092, section 6.1 and section 7, the Rust, Swift, Kotlin, Go, C++,
/// Python and C11 rows): the member a read names after the union's value becomes
/// its tag's read, the field of the one case the facts leave read through that
/// case's proven read, or the accessor of a field the cases left share
/// ([`crate::forge::composed_lowering::rust_union_member`],
/// [`crate::forge::composed_lowering::swift_union_member`],
/// [`crate::forge::composed_lowering::kotlin_union_member`],
/// [`crate::forge::composed_lowering::go_union_member`],
/// [`crate::forge::composed_lowering::cpp_union_member`],
/// [`crate::forge::composed_lowering::python_union_member`],
/// [`crate::forge::composed_lowering::c11_union_member`]); C11, which has no
/// accessor of a field the cases left share, reads one by a test of the tag
/// over those cases ([`crate::forge::composed_lowering::c11_shared_union_read`]).
/// Each is a read
/// that cannot fail at run time: [`reject_unproven_union_reads`] admitted it
/// only where the facts prove its cases, and the facts grow here by the same
/// rule. A node is spelled before the nodes below it, so its object is still
/// the place the author wrote when it is typed.
///
/// A scope that holds no value of the one value type holds no union, and
/// nothing is spelled.
fn lower_union_reads(
    expr: &mut TypedExpr,
    ctx: &TypeCtx<'_>,
    facts: &crate::forge::narrowing::Facts,
    target: ExprTarget,
) -> Result<(), Refusal> {
    use crate::forge::narrowing::proves;
    let mut spelled = None;
    let mut whole = None;
    let mut shared: Option<Vec<(String, String)>> = None;
    if let ExprKind::Member { object, property } = &expr.kind {
        let place = place_path(object).and_then(|path| ctx.value_of(&path));
        let union = place
            .as_ref()
            .and_then(|place| crate::forge::value_path::union_of(ctx.value_scope, place));
        if let Some((union, alias)) = union {
            let subject = dotted_path(object);
            let left = crate::forge::value_path::cases_left(
                ctx.value_scope,
                union,
                subject.as_deref(),
                Some(facts),
            );
            let refused = |why: String| {
                ExprError::UnsupportedConstruct {
                    construct: why,
                    observed: None,
                }
                .at(expr.span.clone())
            };
            // An algorithm's parameter or local holds the union's own unit
            // (docs/adr/0092, Landing order step 5): the whole read is spelled
            // on that type, through the name that holds it, the only subject of
            // a union an algorithm has.
            if let Some(unit) = ctx.unit_unions.get(alias.as_str()) {
                let ExprKind::Ident(holder) = &object.kind else {
                    return Err(refused(format!(
                        "`{property}` of `union:{alias}` read through anything but the parameter \
                         or the local that holds it"
                    )));
                };
                let read = match target {
                    ExprTarget::Rust => crate::forge::composed_lowering::rust_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Rust,
                        ),
                        unit,
                        union,
                        &left,
                        property,
                    ),
                    ExprTarget::Kotlin => crate::forge::composed_lowering::kotlin_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Kotlin,
                        ),
                        unit,
                        union,
                        &left,
                        property,
                    ),
                    // Go spells the result of the function literal a field
                    // several cases share is read by, as its conditional does
                    // ([`go_conditional`]): the read's own type, where the
                    // expression names it.
                    ExprTarget::Go => crate::forge::composed_lowering::go_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Go,
                        ),
                        union,
                        &left,
                        property,
                        go_nameable_type(expr.ty).as_deref(),
                    ),
                    // C++ takes a field several cases share apart by the
                    // alternative of each, which names the unit's struct.
                    ExprTarget::Cpp => crate::forge::composed_lowering::cpp_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Cpp,
                        ),
                        unit,
                        union,
                        &left,
                        property,
                    ),
                    ExprTarget::Swift => crate::forge::composed_lowering::swift_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Swift,
                        ),
                        union,
                        &left,
                        property,
                    ),
                    // Python reads a field several cases share through the
                    // `value` each holds its record in, whatever its class.
                    ExprTarget::Python => crate::forge::composed_lowering::python_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::Python,
                        ),
                        union,
                        &left,
                        property,
                    ),
                    // C reads a field several cases share by a test of the tag
                    // over them, the holder a name read once per case.
                    ExprTarget::C => crate::forge::composed_lowering::c11_unit_union_read(
                        &crate::forge::generator::forge_local_id(
                            holder,
                            crate::generator::Language::C11,
                        ),
                        union,
                        &left,
                        property,
                    ),
                    // The Interpreter holds an algorithm's union as the wire's own
                    // object, read by its member's name, and names no unit's type,
                    // so a read on one reaching it is a fault of the pipeline.
                    ExprTarget::Js => Err(format!(
                        "`{holder}.{property}`, a read of an algorithm's `union:{alias}` on its \
                         unit's type, which the Interpreter does not hold (docs/adr/0092, Landing \
                         order step 5)"
                    )),
                };
                whole = Some(read.map_err(refused)?);
            } else if let Some(arms) = match target {
                // C has no member that names a field several cases share: it is
                // read by a test of the tag over the cases left, each tag the
                // constant the statechart's enum of it declares.
                ExprTarget::C => {
                    let tag_enum = ctx
                        .value_scope
                        .and_then(|base| crate::forge::value_path::tag_enum_alias(base, union));
                    let constant = |variant: &str| {
                        let held = ctx.enums.get(tag_enum.as_deref()?)?;
                        Some(crate::forge::enum_naming::variant_ident(
                            crate::generator::Language::C11,
                            held.source_name,
                            variant,
                        ))
                    };
                    crate::forge::composed_lowering::c11_shared_union_read(
                        union, &left, property, &constant,
                    )
                    .map_err(refused)?
                }
                _ => None,
            } {
                shared = Some(arms);
            } else {
                let member = match target {
                    ExprTarget::Swift => {
                        crate::forge::composed_lowering::swift_union_member(union, &left, property)
                    }
                    ExprTarget::Kotlin => {
                        crate::forge::composed_lowering::kotlin_union_member(union, &left, property)
                    }
                    ExprTarget::Go => {
                        crate::forge::composed_lowering::go_union_member(union, &left, property)
                    }
                    ExprTarget::Cpp => {
                        crate::forge::composed_lowering::cpp_union_member(union, &left, property)
                    }
                    ExprTarget::Python => {
                        crate::forge::composed_lowering::python_union_member(union, &left, property)
                    }
                    ExprTarget::C => {
                        crate::forge::composed_lowering::c11_union_member(union, &left, property)
                    }
                    _ => crate::forge::composed_lowering::rust_union_member(union, &left, property),
                }
                .map_err(refused)?;
                spelled = Some(member);
            }
        }
    }
    // A read spelled whole has no part left to lower: its object is the name
    // the read was spelled through.
    if let Some(read) = whole {
        expr.kind = ExprKind::Raw(read);
        return Ok(());
    }
    // A field several cases share, read on C by a test of the tag over the
    // cases left: the object is lowered first, then read once per case, the
    // last case untested, the narrowing having proved the value is in one of
    // them. The members read are no fields of the union, so nothing below the
    // reads is lowered again.
    if let Some(arms) = shared {
        let ExprKind::Member { object, .. } = &mut expr.kind else {
            unreachable!("a shared read is spelled for a member read alone")
        };
        lower_union_reads(object, ctx, facts, target)?;
        let object = object.clone();
        let (ty, span) = (expr.ty, expr.span.clone());
        let read = |property: String, ty: InferredType| TypedExpr {
            kind: ExprKind::Member {
                object: object.clone(),
                property,
            },
            ty,
            span: span.clone(),
        };
        let mut arms = arms.into_iter().rev();
        let Some((_, last)) = arms.next() else {
            unreachable!("a field several cases share is read under two cases or more")
        };
        let mut built = read(last, ty);
        for (tag, property) in arms {
            let condition = TypedExpr {
                kind: ExprKind::Binary {
                    op: BinOp::StrictEq,
                    left: Box::new(read(
                        crate::forge::composed_lowering::C11_UNION_TAG.to_string(),
                        InferredType::Unknown,
                    )),
                    right: Box::new(TypedExpr::new(ExprKind::Raw(tag))),
                },
                ty: InferredType::Bool,
                span: span.clone(),
            };
            built = TypedExpr {
                kind: ExprKind::Conditional {
                    condition: Box::new(condition),
                    consequent: Box::new(read(property, ty)),
                    alternate: Box::new(built),
                },
                ty,
                span: span.clone(),
            };
        }
        *expr = built;
        return Ok(());
    }
    if let (Some(member), ExprKind::Member { property, .. }) = (spelled, &mut expr.kind) {
        *property = member;
    }
    match &mut expr.kind {
        ExprKind::Binary {
            op: op @ (BinOp::And | BinOp::Or),
            left,
            right,
        } => {
            let right_facts = facts.and(&proves(left, *op == BinOp::And));
            lower_union_reads(left, ctx, facts, target)?;
            lower_union_reads(right, ctx, &right_facts, target)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let holds = facts.and(&proves(condition, true));
            let fails = facts.and(&proves(condition, false));
            lower_union_reads(condition, ctx, facts, target)?;
            lower_union_reads(consequent, ctx, &holds, target)?;
            lower_union_reads(alternate, ctx, &fails, target)
        }
        _ => expr
            .children_mut()
            .into_iter()
            .try_for_each(|child| lower_union_reads(child, ctx, facts, target)),
    }
}

/// Refuse an index read whose object is a value that holds no elements, or
/// whose index is not a whole number. Only a list, a byte string or a
/// `<sce:const>` table is indexed, and what is indexed by is an integer;
/// every other value reached the emitters typed `Unknown` under an index and
/// was left to each target's compiler to refuse in its own words — or not to
/// (`total[0]` over a `uint32` is a bit read in one language and a type error
/// in the next).
///
/// A type the lattice cannot name — an enum, a table's name — is `Unknown`
/// and is let through: the name checks and the target's compiler own it.
///
/// A map is read at a key, whatever the key's type is (docs/adr/0108, D3): its
/// key is judged against the map's ([`reject_misread_keys`]), and not as a
/// position.
fn reject_misdirected_indexing(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    let observed = || {
        expr.span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string)
    };
    if let ExprKind::Index { object, index } = &expr.kind {
        let holds_nothing = matches!(
            object.ty.strip_quantity(),
            InferredType::Int { .. }
                | InferredType::UntypedInt
                | InferredType::Float { .. }
                | InferredType::UntypedFloat
                | InferredType::Bool
                | InferredType::Str
                | InferredType::Null
                | InferredType::Record(_)
        );
        if holds_nothing {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!(
                    "an index read of {} (only a list, a byte string or a constant table is \
                     indexed)",
                    object.ty.describe()
                ),
                observed: observed(),
            }
            .at(expr.span.clone()));
        }
        let whole = matches!(
            index.ty.strip_quantity(),
            InferredType::Int { .. } | InferredType::UntypedInt | InferredType::Unknown
        );
        if !whole && keyed_place(object, ctx).is_none() {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!(
                    "an index of {} (an index is a whole number)",
                    index.ty.describe()
                ),
                observed: observed(),
            }
            .at(expr.span.clone()));
        }
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_misdirected_indexing(child, ctx, source))
}

/// Refuse an element of a list of records read by index. A record is read
/// through a name — a parameter, a local, a `<sce:foreach>` item — whose
/// fields the record rule types (SCE_FORGE.md §4.12); `xs[i]` has no name,
/// so `xs[i].wallTime` would reach every backend as an operand of no type.
fn reject_unnamed_record_elements(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    // `xs[i].f` is the one way an element of a list of records is read by its
    // index: the field is typed through `<xs>[].<f>`, and the element itself is
    // never a value. Its index and its list are walked as any expression is. It is
    // a way only where the scope types those fields (a static statechart's does,
    // an algorithm's does not): elsewhere the element is refused here, at `xs[i]`.
    if let ExprKind::Member { object, .. } = &expr.kind {
        if let ExprKind::Index {
            object: list,
            index,
        } = &object.kind
        {
            if place_path(list).is_some_and(|path| ctx.reads_elements_of(&path)) {
                return [list.as_ref(), index.as_ref()]
                    .into_iter()
                    .try_for_each(|child| reject_unnamed_record_elements(child, ctx, source));
            }
        }
    }
    // A list buffer of records an algorithm builds holds its elements the same
    // way (SCE_FORGE.md §4.12, v2): it is walked by a `<sce:foreach>`, never
    // indexed.
    if let ExprKind::Index { object, .. } = &expr.kind {
        if matches!(
            object.ty,
            InferredType::List(elem) | InferredType::ListBuffer(elem) if elem.is_record()
        ) {
            return Err(ExprError::UnsupportedConstruct {
                construct: "an element of a list of records read by index (a record element \
                            is read through a <sce:foreach> item)"
                    .to_string(),
                observed: expr
                    .span
                    .clone()
                    .and_then(|span| source.get(span))
                    .map(str::to_string),
            }
            .at(expr.span.clone()));
        }
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_unnamed_record_elements(child, ctx, source))
}

/// Refuse the element of a list of unions an algorithm holds read at an index
/// anywhere but as the whole expression (`whole`), which a union local copies
/// (docs/adr/0099, section 5.2): an element at an index is no narrowing
/// subject, so its fields are read nowhere and its tag is not tested there
/// (docs/adr/0092, section 6.1 item 2), and a union is no operand. The repair
/// is to bind the element: a union local copied from it, or a `<sce:foreach>`
/// item.
///
/// A statechart's list is a place of the one value type whose elements its scope
/// types ([`TypeCtx::reads_elements_of`]), read through its path as the
/// statechart's own rules say (docs/adr/0099, section 5.1); this is an
/// algorithm's rule, and passes such a list by.
fn reject_union_elements_as_parts(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
    whole: bool,
) -> Result<(), Refusal> {
    if let ExprKind::Index { object, .. } = &expr.kind {
        let algorithm_list = !place_path(object).is_some_and(|path| ctx.reads_elements_of(&path));
        if let (Some(alias), false, true) = (listed_union(object.ty), whole, algorithm_list) {
            let observed = expr
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string);
            return Err(ExprError::UnsupportedConstruct {
                construct: format!(
                    "`{}`, an element of a list of union:{alias} read at an index inside an \
                     expression (an element at an index is no subject a test of its tag \
                     narrows: copy it whole into a union local, \
                     <sce:var type=\"union:{alias}\" init=\"…[…]\"/>, or walk the list with a \
                     <sce:foreach>, and read the local or the item)",
                    observed.as_deref().unwrap_or_default()
                ),
                observed,
            }
            .at(expr.span.clone()));
        }
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_union_elements_as_parts(child, ctx, source, false))
}

/// Where a record-valued node stands, for [`reject_records_as_operands`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum RecordPlace {
    /// The whole expression: a call returning a record may stand here, as
    /// the value a record local is initialised from.
    Whole,
    /// A call's argument: a record may be passed here by name.
    Argument,
    /// The key a value of a map is read at, `m[k]` (docs/adr/0108, D3): a
    /// record key is handed to the lookup whole by its name, as an argument
    /// of `has(m, k)` is.
    Key,
    /// Anywhere else — an operand, an index, a member's object.
    Operand,
}

/// Refuse a record anywhere but where it is passed whole (SCE_FORGE.md
/// §4.12): a record NAME as a call's argument or as the key of a map's read
/// (docs/adr/0108, D3, [`keyed_place`]), and a CALL returning a record as the
/// whole expression a record local is initialised from. A record is no
/// operand — backends do not compare or combine structs alike — and its
/// fields are read by `r.<field>`, which types each field instead.
fn reject_records_as_operands(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
    place: RecordPlace,
) -> Result<(), Refusal> {
    if matches!(expr.ty, InferredType::Record(_)) {
        let admitted = matches!(
            (&expr.kind, place),
            (ExprKind::Ident(_), RecordPlace::Argument | RecordPlace::Key)
                | (ExprKind::Call { .. }, RecordPlace::Whole)
        );
        if !admitted {
            return Err(ExprError::UnsupportedConstruct {
                construct: "a record used as a value (a record is passed whole by its name, as \
                            an argument, or received from a call as a record local's init; \
                            its fields are read as r.<field>)"
                    .to_string(),
                observed: expr
                    .span
                    .clone()
                    .and_then(|span| source.get(span))
                    .map(str::to_string),
            }
            .at(expr.span.clone()));
        }
    }
    match &expr.kind {
        ExprKind::Call { args, .. } => args.iter().try_for_each(|arg| {
            reject_records_as_operands(arg, ctx, source, RecordPlace::Argument)
        }),
        // `r.<field>` reads a field of the record `r` names: the object is
        // the record's name, which is how a field is read.
        ExprKind::Member { object, .. } if matches!(object.kind, ExprKind::Ident(_)) => Ok(()),
        // `m[k]` of a map reads the value at the key `k`, which the lookup
        // takes whole; the key was held to the map's key type where it was
        // judged ([`reject_misread_keys`]).
        ExprKind::Index { object, index } if keyed_place(object, ctx).is_some() => {
            reject_records_as_operands(object, ctx, source, RecordPlace::Operand)?;
            reject_records_as_operands(index, ctx, source, RecordPlace::Key)
        }
        _ => expr.children().into_iter().try_for_each(|child| {
            reject_records_as_operands(child, ctx, source, RecordPlace::Operand)
        }),
    }
}

/// Refuse a record's text field anywhere but as an operand of `===` or `!==`
/// (SCE_FORGE.md §4.12). The text of a record is held in a buffer of the
/// schema's bound, which each backend spells in its own storage — an owned
/// bounded string, a fixed array, the language's string — so v1 admits only
/// the comparison, which every one of them lowers to the same answer. The one
/// other read, the whole field copied into another record's string field, is
/// judged where that write is lowered and never reaches an expression. A
/// concatenation, an index, a length, a condition or a return of the text is
/// refused here, at the field.
///
/// The check runs against [`TypeCtx::record_text`], which is empty for every
/// context but an algorithm whose records hold text.
fn reject_record_text_misuse(
    expr: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
    operand_of_comparison: bool,
) -> Result<(), Refusal> {
    if ctx.record_text.is_empty() {
        return Ok(());
    }
    let reads_text = matches!(expr.kind, ExprKind::Member { .. })
        && dotted_path(expr).is_some_and(|path| ctx.record_text.contains(path.as_str()));
    if reads_text {
        if operand_of_comparison {
            return Ok(());
        }
        return Err(ExprError::UnsupportedConstruct {
            construct: "a record's text field read as a value (v1 reads it only as an operand \
                        of === or !==, and copies it whole into another record's string field \
                        with <sce:set> or <sce:assign>)"
                .to_string(),
            observed: expr
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string),
        }
        .at(expr.span.clone()));
    }
    match &expr.kind {
        ExprKind::Binary {
            op: BinOp::StrictEq | BinOp::StrictNeq,
            left,
            right,
        } => {
            reject_record_text_misuse(left, ctx, source, true)?;
            reject_record_text_misuse(right, ctx, source, true)
        }
        _ => expr
            .children()
            .into_iter()
            .try_for_each(|child| reject_record_text_misuse(child, ctx, source, false)),
    }
}

/// Refuse a buffer the body is building anywhere but where it is read back:
/// as the collection of `len(out)` and of `out[i]` (SCE_FORGE.md §4.12). It
/// is the algorithm's own storage — a result struct in C11, a growable
/// buffer class in Kotlin — not the view a `bytes` or `list<T>` value is,
/// so no backend can pass, compare or combine it as one.
fn reject_buffers_as_values(expr: &TypedExpr, source: &str) -> Result<(), Refusal> {
    let is_buffer = |node: &TypedExpr| {
        matches!(
            node.ty,
            InferredType::BytesBuffer | InferredType::ListBuffer(_)
        ) && matches!(node.kind, ExprKind::Ident(_))
    };
    match &expr.kind {
        ExprKind::Index { object, index } if is_buffer(object) => {
            reject_buffers_as_values(index, source)
        }
        ExprKind::Call { callee, args, .. }
            if is_len_builtin(callee, args) && is_buffer(&args[0]) =>
        {
            Ok(())
        }
        _ if is_buffer(expr) => Err(ExprError::UnsupportedConstruct {
            construct: "a buffer the body is building, used as a value (it is read back \
                        by len(out) and out[i], and returned by name)"
                .to_string(),
            observed: expr
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string),
        }
        .at(expr.span.clone())),
        _ => expr
            .children()
            .into_iter()
            .try_for_each(|child| reject_buffers_as_values(child, source)),
    }
}

/// The member that reads the value an optional value holds, where the build has
/// proven it present: `x.value` (docs/adr/0055).
pub(crate) const OPTIONAL_VALUE: &str = "value";

/// The C++ view a list or a byte string is borrowed through: the forge
/// runtime's `sce/forge/span.h`, one type at C++17 and at C++20, never
/// `std::span` (docs/adr/0059, section 4). Its element type follows it in
/// angle brackets.
pub(crate) const CPP_SPAN: &str = "SCE::Forge::Span";

/// Refuse what an optional value (docs/adr/0055) is not for, and a read of one
/// the build has not proven present:
///
/// - `x.value` where `x` is no subject proven present where it is read
///   ([`ExprError::OptionalReadUnproven`]): `facts` hold the subjects proven
///   where the expression stands, and inside it the right operand of `a && b`
///   is read where `a` is true, of `a || b` where `a` is false, and the
///   branches of `c ? t : e` where `c` is true and false
///   ([`crate::forge::narrowing::proves`]);
/// - an optional value as an operand of an ordering, an arithmetic or a
///   bitwise operator or a text join, as a truth test (`!x`, `x && y`,
///   `x ? a : b`), as what `len` measures or what is indexed, or as an argument
///   of a parameter that is not optional — `expression/type-mismatch`, naming
///   `??` and a test;
/// - `null` beside an operand of `===` or `!==` that is never absent, and an
///   optional value compared with one its present value is never equal to;
/// - `??` on a value that is never absent, or with a default the present value's
///   type does not take.
///
/// An optional record (docs/adr/0055, step 9) is read through a field of its
/// present value (`pick.value.title`) where the build has proven it present, as
/// any `x.value` is; its value taken whole, a default given with `??` and a
/// comparison with anything but `null` are refused by name, a record being no
/// operand.
fn reject_optional_misuse(
    expr: &TypedExpr,
    facts: &crate::forge::narrowing::Facts,
    source: &str,
) -> Result<(), Refusal> {
    use crate::forge::narrowing::{proves, subject};
    let observed = |node: &TypedExpr| {
        node.span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string)
    };
    // An optional value where `expected` is: what an absent one would be there
    // has no answer.
    let misused = |node: &TypedExpr, expected: InferredType| -> Refusal {
        ExprError::TypeMismatch {
            expected: expected.describe(),
            got: described(node.ty, expected),
            observed: observed(node),
        }
        .at(node.span.clone())
    };
    let truth_test = |node: &TypedExpr| -> Result<(), Refusal> {
        if node.ty.is_optional() {
            return Err(misused(node, InferredType::Bool));
        }
        Ok(())
    };
    // `x.value` read where `x` is proven present, or the refusal at `read`.
    let proven_present = |optional: &TypedExpr, read: &TypedExpr| -> Result<(), Refusal> {
        let tested = subject(optional);
        if tested.as_deref().is_some_and(|s| facts.holds(s)) {
            return Ok(());
        }
        Err(ExprError::OptionalReadUnproven {
            optional: tested
                .or_else(|| observed(optional))
                .unwrap_or_else(|| optional.ty.describe()),
            read: observed(read).unwrap_or_default(),
            // A record has no default to read it with, so the repair named for one
            // is the test alone.
            record: optional.ty == InferredType::Optional(OptionalElem::Record),
        }
        .at(read.span.clone()))
    };
    match &expr.kind {
        ExprKind::Member { object, property } => {
            // `x.value.f`, a field of the present value of an optional record
            // (docs/adr/0055, step 9): the record is read through its field, and
            // only where the build has proven it present. Its value as a whole is
            // no expression: a record is no operand, and is passed by name.
            if let ExprKind::Member {
                object: holder,
                property: value,
            } = &object.kind
            {
                if value == OPTIONAL_VALUE
                    && holder.ty == InferredType::Optional(OptionalElem::Record)
                {
                    proven_present(holder, expr)?;
                    return reject_optional_misuse(holder, facts, source);
                }
            }
            if property == OPTIONAL_VALUE && object.ty.is_optional() {
                if object.ty == InferredType::Optional(OptionalElem::Record) {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: "the value of an optional record taken whole (a record is \
                                    no operand: read a field of it, `x.value.<field>`, where \
                                    a test proves it present)"
                            .to_string(),
                        observed: observed(expr),
                    }
                    .at(expr.span.clone()));
                }
                proven_present(object, expr)?;
                return reject_optional_misuse(object, facts, source);
            }
        }
        ExprKind::Binary {
            op: op @ (BinOp::And | BinOp::Or),
            left,
            right,
        } => {
            truth_test(left)?;
            truth_test(right)?;
            reject_optional_misuse(left, facts, source)?;
            let when = *op == BinOp::And;
            return reject_optional_misuse(right, &facts.and(&proves(left, when)), source);
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            truth_test(condition)?;
            reject_optional_misuse(condition, facts, source)?;
            reject_optional_misuse(consequent, &facts.and(&proves(condition, true)), source)?;
            return reject_optional_misuse(
                alternate,
                &facts.and(&proves(condition, false)),
                source,
            );
        }
        ExprKind::Binary {
            op: BinOp::Coalesce,
            left,
            right,
        } => match left.ty {
            // A record is no operand, so no default can be given for an absent
            // one: it is read through a conditional that tests it
            // (`x !== null ? x.value.f : d`, docs/adr/0055, step 9).
            InferredType::Optional(OptionalElem::Record) => {
                return Err(ExprError::UnsupportedConstruct {
                    construct: "a read of an optional record with a default (`??`): a record is \
                                no operand, so it has no default; read a field of it where a \
                                test proves it present, `x !== null ? x.value.<field> : <value>`"
                        .to_string(),
                    observed: observed(left),
                }
                .at(left.span.clone()))
            }
            InferredType::Optional(elem) => {
                let takes = matches!(right.ty, InferredType::Null)
                    || slot_admits(InferredType::Optional(elem), right.ty);
                if !takes {
                    return Err(type_mismatch(elem.present(), right, source));
                }
            }
            // A value whose type this layer does not know is not judged here.
            InferredType::Unknown => {}
            never_absent => {
                return Err(ExprError::TypeMismatch {
                    expected: "an optional value".to_string(),
                    got: format!("{}, which is never absent", never_absent.describe()),
                    observed: observed(left),
                }
                .at(left.span.clone()))
            }
        },
        ExprKind::Binary {
            op: BinOp::StrictEq | BinOp::StrictNeq,
            left,
            right,
        } => {
            for (side, other) in [(left, right), (right, left)] {
                let never_absent = !matches!(
                    other.ty,
                    InferredType::Optional(_) | InferredType::Null | InferredType::Unknown
                );
                if matches!(side.kind, ExprKind::NullLit) && never_absent {
                    return Err(type_mismatch(other.ty, side, source));
                }
            }
            // Equality is total over an optional value: two absent values are
            // equal, and an absent one equals no present one. The present values
            // are compared by the rule of their type.
            let optional = [left.ty, right.ty].into_iter().find_map(|ty| match ty {
                InferredType::Optional(elem) => Some(elem),
                _ => None,
            });
            if let Some(elem) = optional {
                for side in [left, right] {
                    if !slot_admits(InferredType::Optional(elem), side.ty) {
                        return Err(type_mismatch(InferredType::Optional(elem), side, source));
                    }
                }
                // A record is no operand, and no backend compares two alike: an
                // optional record is tested against `null`, and its fields are
                // compared where a test proves it present (docs/adr/0055,
                // step 9).
                let tests_null = [left, right]
                    .into_iter()
                    .any(|side| matches!(side.kind, ExprKind::NullLit));
                if elem == OptionalElem::Record && !tests_null {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: "a comparison of an optional record with a value other than \
                                    `null` (records are compared by no backend: test it with \
                                    `x !== null`, and compare its fields where it is proven \
                                    present)"
                            .to_string(),
                        observed: observed(expr),
                    }
                    .at(expr.span.clone()));
                }
            }
        }
        ExprKind::Binary { left, right, .. } => {
            for side in [left, right] {
                if let InferredType::Optional(elem) = side.ty {
                    return Err(misused(side, elem.present()));
                }
            }
        }
        ExprKind::Unary { op, operand } => {
            if let InferredType::Optional(elem) = operand.ty {
                let expected = match op {
                    UnaryOp::Not => InferredType::Bool,
                    UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => elem.present(),
                };
                return Err(misused(operand, expected));
            }
        }
        ExprKind::Index { object, .. } => {
            if let InferredType::Optional(elem) = object.ty {
                return Err(misused(object, elem.present()));
            }
        }
        ExprKind::Call { args, params, .. } => {
            for (index, arg) in args.iter().enumerate() {
                let InferredType::Optional(elem) = arg.ty else {
                    continue;
                };
                // A parameter that is not optional takes a present value, and so
                // does every builtin (`len`, `abs`, …), whose parameters the
                // context does not type. A parameter whose type is not known is
                // the judge of the call's own (an enum's conversion).
                let expected = match params.get(index) {
                    None => Some(elem.present()),
                    Some(param) if param.is_optional() || *param == InferredType::Unknown => None,
                    Some(param) => Some(*param),
                };
                if let Some(expected) = expected {
                    return Err(misused(arg, expected));
                }
            }
        }
        _ => {}
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_optional_misuse(child, facts, source))
}

/// What a value of `got` is called where `slot` is declared: its type, and —
/// for an optional value where a value that is never absent is declared — the
/// two reads that make one present (docs/adr/0055).
fn described(got: InferredType, slot: InferredType) -> String {
    if got.is_optional() && !slot.is_optional() && slot != InferredType::Unknown {
        format!(
            "{}, which may be absent (read it with `??` and a default, or with `.value` where \
             a `!== null` test proves it present)",
            got.describe()
        )
    } else {
        got.describe()
    }
}

/// Refuse an expression that holds an optional value (docs/adr/0055) — `x`,
/// `x.value`, `x ?? d`, `x === null` — on a `target` whose emitter does not
/// spell one yet ([`ExprTarget::lowers_optional`]), before the emitter is
/// reached, naming the construct ([`OptionalForm`]) and placed at it. A document
/// that holds one is refused by name where it is lowered for such a target, so
/// an expression reaches here with one only where no such refusal ran.
fn reject_unlowered_optional(expr: &TypedExpr, target: ExprTarget) -> Result<(), Refusal> {
    let spells_value = target.lowers_optional();
    if spells_value && target.lowers_optional_record() {
        return Ok(());
    }
    // An optional record (docs/adr/0055, step 9) is spelled apart from the
    // other optional values: a target that spells those and not it refuses a
    // node that holds one, and the construct that reads its present value.
    let is_record = expr.ty == InferredType::Optional(OptionalElem::Record)
        || matches!(
            &expr.kind,
            ExprKind::Member { object, property }
                if property == OPTIONAL_VALUE
                    && object.ty == InferredType::Optional(OptionalElem::Record)
        );
    // `null === null` reads no optional value, and is left as it was.
    let refused = if spells_value && !is_record {
        None
    } else {
        match optional_form(expr) {
            Some(OptionalForm::Constant(_)) => None,
            Some(form) => Some((form.describe(), form.span(expr))),
            None if expr.ty.is_optional() => {
                Some(("an optional value".to_string(), expr.span.clone()))
            }
            None => None,
        }
    };
    if let Some((what, at)) = refused {
        let lang = target
            .language()
            .map_or("ecmascript", crate::generator::Language::canonical_name);
        let noun = if is_record {
            "an optional record"
        } else {
            "an optional value"
        };
        return Err(ExprError::UnsupportedConstruct {
            construct: format!("{what}: {noun} has no {lang} lowering yet (docs/adr/0055)"),
            observed: None,
        }
        .at(at));
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_unlowered_optional(child, target))
}

/// What a node does with an optional value (docs/adr/0055, section 2), as
/// every emitter that lowers one reads it ([`optional_form`]): the one
/// classification of the constructs the expression surface admits, so the
/// emitters differ only in how each is spelled (ADR 0055, section 5).
///
/// A value of `T` that stands where an `optional<T>` is expected is no node of
/// its own: it is lifted where the emitter coerces it to the type its place
/// expects ([`InferredType::lifted_into`]).
/// `null` is no form either: it is the absent value of the optional value its
/// place expects, which the emitter is handed as the type it coerces to, and is
/// spelled as it was where the place expects none.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OptionalForm<'e> {
    /// `subject !== null` (`present`) or `subject === null`, either side of the
    /// operator, where `subject` is an optional value: whether it is present.
    Test {
        subject: &'e TypedExpr,
        present: bool,
    },
    /// `subject.value`, where the build has proven `subject` present
    /// ([`reject_optional_misuse`]): a read with no failure path.
    Proven { subject: &'e TypedExpr },
    /// `value ?? fallback`: the present value of `value`, or `fallback`, which
    /// is evaluated only when `value` is absent. `value` is evaluated once. The
    /// node is of `T`, or of `optional<T>` when `fallback` is optional itself.
    Otherwise {
        value: &'e TypedExpr,
        fallback: &'e TypedExpr,
    },
    /// `left === right` (`equal`) or `left !== right`, where either side is an
    /// optional value and neither is `null`: total equality, two absent values
    /// equal and an absent one equal to no present one. A side of `T` is lifted
    /// into the optional value the other side is.
    Equality {
        left: &'e TypedExpr,
        right: &'e TypedExpr,
        equal: bool,
    },
    /// `null === null` (`true`) or `null !== null`: a comparison of two absent
    /// values of no type, whose answer is known.
    Constant(bool),
}

impl OptionalForm<'_> {
    /// The construct, as a refusal names it.
    pub(crate) fn describe(&self) -> String {
        match self {
            Self::Test { present: true, .. } => {
                "a test that a value is present (`!== null`)".into()
            }
            Self::Test { present: false, .. } => {
                "a test that a value is absent (`=== null`)".into()
            }
            Self::Proven { .. } => "a read of a value proven present (`.value`)".to_string(),
            Self::Otherwise { .. } => "a read with a default (`??`)".to_string(),
            Self::Equality { equal: true, .. } => "`===` of an optional value".to_string(),
            Self::Equality { equal: false, .. } => "`!==` of an optional value".to_string(),
            Self::Constant(true) => "`null === null`".to_string(),
            Self::Constant(false) => "`null !== null`".to_string(),
        }
    }

    /// Where the construct is written in `whole`, the node it is the form of:
    /// the optional value it reads, or the range of its two operands.
    pub(crate) fn span(&self, whole: &TypedExpr) -> Option<std::ops::Range<usize>> {
        let between = |first: &TypedExpr, last: &TypedExpr| match (&first.span, &last.span) {
            (Some(first), Some(last)) => Some(first.start.min(last.start)..first.end.max(last.end)),
            _ => whole.span.clone(),
        };
        match self {
            Self::Constant(_) => whole.span.clone(),
            Self::Test { subject, .. } | Self::Proven { subject } => {
                subject.span.clone().or_else(|| whole.span.clone())
            }
            Self::Otherwise { value, fallback } => between(value, fallback),
            Self::Equality { left, right, .. } => between(left, right),
        }
    }
}

/// The [`OptionalForm`] of `expr`, or `None` for a node that is none of them —
/// one whose operands may hold an optional value all the same, each of which is
/// a node of its own.
pub(crate) fn optional_form(expr: &TypedExpr) -> Option<OptionalForm<'_>> {
    let is_null = |node: &TypedExpr| matches!(node.kind, ExprKind::NullLit);
    match &expr.kind {
        ExprKind::Member { object, property }
            if property == OPTIONAL_VALUE && object.ty.is_optional() =>
        {
            Some(OptionalForm::Proven { subject: object })
        }
        ExprKind::Binary {
            op: BinOp::Coalesce,
            left,
            right,
        } => Some(OptionalForm::Otherwise {
            value: left,
            fallback: right,
        }),
        ExprKind::Binary {
            op: op @ (BinOp::StrictEq | BinOp::StrictNeq),
            left,
            right,
        } => {
            let equal = *op == BinOp::StrictEq;
            // A test of a value that is not optional (one this layer does not
            // type, in a data model that is not `sce-static`) is left as it was.
            match (is_null(left), is_null(right)) {
                (true, true) => Some(OptionalForm::Constant(equal)),
                (true, false) => right.ty.is_optional().then_some(OptionalForm::Test {
                    subject: right,
                    present: !equal,
                }),
                (false, true) => left.ty.is_optional().then_some(OptionalForm::Test {
                    subject: left,
                    present: !equal,
                }),
                (false, false) if left.ty.is_optional() || right.ty.is_optional() => {
                    Some(OptionalForm::Equality { left, right, equal })
                }
                (false, false) => None,
            }
        }
        _ => None,
    }
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
///
/// `null` stands only where an optional value is declared (docs/adr/0055), so
/// a slot that declares any other type takes no `null`. Until 2026-10-21 it was
/// admitted as an unknown value is, and `<data sce:type="int64" expr="null"/>`
/// was judged fit.
///
/// A slot of `optional<T>` takes `null`, an optional value whose present value
/// the slot of `T` would take, and a value of `T` itself, which is present —
/// by the rule that admits it where `T` is. An optional value never stands
/// where `T` is declared: what an absent one would be there has no answer, so
/// it is read with `??` or under a test.
pub(crate) fn slot_admits(slot: InferredType, got: InferredType) -> bool {
    use InferredType as T;
    match (slot, got) {
        (T::Unknown | T::Null, _) | (_, T::Unknown) => true,
        (T::Optional(_), T::Null) => true,
        // A record is no operand, so an optional record's slot takes an optional
        // record, a record by name and `null`; and no other optional value or
        // record fits a slot of another optional value (docs/adr/0055, step 9).
        (T::Optional(OptionalElem::Record), got) => {
            matches!(got, T::Record(_) | T::Optional(OptionalElem::Record))
        }
        (T::Optional(_), T::Optional(OptionalElem::Record) | T::Record(_)) => false,
        (T::Optional(want), T::Optional(got)) => slot_admits(want.present(), got.present()),
        (T::Optional(want), got) => slot_admits(want.present(), got),
        (_, T::Null) => false,
        (_, T::Optional(_)) => false,
        (slot, got) if slot.is_integer_like() => got.is_integer_like(),
        (slot, got) if slot.is_numeric() => got.is_numeric(),
        (T::Bool, got) => got == T::Bool,
        (T::Str, got) => got == T::Str,
        (T::Bytes, got) => matches!(got, T::Bytes | T::Str),
        // A record stands only where a record of the same schema is
        // declared: two schemas are two struct types on every backend, even
        // when their fields agree (SCE_FORGE.md §4.12).
        (T::Record(want), got) => got == T::Record(want),
        (_, T::Record(_)) => false,
        _ => true,
    }
}

/// The refusal of a call to an imported algorithm whose signature has a
/// slot the caller cannot fill (`slot` names it: "returns list<int64>",
/// "takes record:Hlc `a`", "returns union:Shape"), under either form — an
/// expression's `days(n)` or a `<sce:call target="days">`, from where no such
/// call is admitted. A list crosses only the host boundary; a record the host
/// boundary and an algorithm's call of another algorithm (SCE_FORGE.md
/// §4.12); a union the host boundary and an algorithm's call that hands one
/// by name to a parameter, since no place of an algorithm takes whole a union
/// a call answers (docs/adr/0092, "Decided while handing a union from one
/// algorithm to another"). One text for both forms, so the two read alike.
pub(crate) fn host_only_call(target: &str, slot: &str, observed: Option<String>) -> ExprError {
    ExprError::UnsupportedConstruct {
        construct: format!(
            "a call to algorithm `{target}`, which {slot} \
             (a host calls such an algorithm, and another algorithm when no \
             slot of it is a list and its return is no union)"
        ),
        observed,
    }
}

/// The refusal of a call to an imported algorithm that declares `may-fail`
/// from a place that cannot receive its failure — anywhere but a `may-fail`
/// algorithm's body (SCE_FORGE.md §3.4.1) — under either form, an
/// expression's `tick(n)` or a `<sce:call target="tick">`. The failure has
/// nowhere to go there: a guard cannot bound what another algorithm does,
/// so no proof would admit the call either.
pub(crate) fn unreceived_failure_call(target: &str, observed: Option<String>) -> ExprError {
    ExprError::UnsupportedConstruct {
        construct: format!(
            "a call to algorithm `{target}`, which declares may-fail, where its failure \
             cannot be received (only a host, or an algorithm that declares may-fail itself, \
             receives one)"
        ),
        observed,
    }
}

/// `value`, of a kind `slot` does not admit, refused at its range — which
/// `source` is the text of, and the refusal reports as written.
fn type_mismatch(slot: InferredType, value: &TypedExpr, source: &str) -> Refusal {
    ExprError::TypeMismatch {
        expected: slot.describe(),
        got: described(value.ty, slot),
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
/// literal in it takes a type that cannot hold it, or a shift or a `~` of
/// literals takes none ([`land_integers`] — a hint's type is the one the
/// literal is emitted in, so it counts). Landing the value also types every
/// operation of literals in it, which is why the tree is mutable.
fn judge_value(ast: &mut TypedExpr, expected: Expected, source: &str) -> Result<(), Refusal> {
    if let Expected::Slot(slot) = expected {
        if !slot_admits(slot, ast.ty) {
            return Err(type_mismatch(slot, ast, source));
        }
    }
    // A literal where an optional value is expected is a present value, held to
    // the type it holds when present (docs/adr/0055).
    land_integers(
        ast,
        expected.ty().unwrap_optional(),
        Adopts::No,
        LiteralFit::Judged,
        source,
    )
}

/// [`land_integers`] for a tree whose literals are held to their types
/// elsewhere: a build-time fold, which computes each value exactly and refuses
/// one its place cannot hold where it stores it (`algorithm/const-integer-failure`),
/// and so needs the types the emitters give its operations and nothing more.
pub(crate) fn type_integers_in(
    ast: &mut TypedExpr,
    context: InferredType,
    source: &str,
) -> Result<(), Refusal> {
    land_integers(
        ast,
        context.unwrap_optional(),
        Adopts::No,
        LiteralFit::Unjudged,
        source,
    )
}

/// Whether [`land_integers`] holds each integer literal to the type it takes
/// ([`ExprError::LiteralOutOfRange`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiteralFit {
    Judged,
    Unjudged,
}

/// Whether the node [`land_integers`] is handed is an operand of an operation
/// that took its type from its context: a literal there takes that type too,
/// as the operation's other operands do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Adopts {
    Yes,
    No,
}

/// Land every integer of `ast` in the type it takes (SCE_FORGE.md §3.4,
/// "Typing convention"; §3.4.1, "The width of a shift"), once, before any
/// backend spells it. `context` is the type the value lands in.
///
/// A literal takes the type of the operand it meets in a binary operation, of
/// the parameter it is passed to, or of the place it lands in. An integer
/// operation none of whose operands has a declared type (`65536 * 65536`,
/// `~0`, a shift of a literal, which its count does not type) takes the type
/// of its context, and so do its literal operands: their node's type is set,
/// and every emitter spells such a literal in it ([`typed_integer_literal`]),
/// where its language would otherwise compute the operation in its own
/// default width (`1 << m` an `i32` in Rust, an `int` in C, an `Int` in
/// Kotlin). A literal that meets a declared operand keeps no type of its own:
/// every backend gives it its partner's.
///
/// A shift of a literal, and a `~` of one, have no width until their context
/// gives one, and their value depends on it; where it gives none — compared
/// with another literal, an index, a real, no declared place — they are
/// refused (`expression/type-mismatch`), at the operand that has no type.
///
/// With [`LiteralFit::Judged`], a literal its type cannot hold is refused
/// ([`ExprError::LiteralOutOfRange`]) — the type Rust and Go would infer for
/// it, and refuse it in.
fn land_integers(
    ast: &mut TypedExpr,
    context: InferredType,
    adopts: Adopts,
    fit: LiteralFit,
    source: &str,
) -> Result<(), Refusal> {
    let place = context.strip_quantity();
    let width = matches!(place, InferredType::Int { .. }).then_some(place);
    // The minus belongs to the literal: `-128` fits an `int8`.
    if let Some((text, negated)) = integer_literal_parts(ast) {
        if let (Adopts::Yes, Some(width)) = (adopts, width) {
            ast.ty = width;
        }
        return match fit {
            LiteralFit::Judged => literal_fits(ast, &text, negated, context, source),
            LiteralFit::Unjudged => Ok(()),
        };
    }
    let unwidened = |what: String, operand: &TypedExpr| {
        ExprError::TypeMismatch {
            expected: what,
            got: operand.ty.describe(),
            observed: operand
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string),
        }
        .at(operand.span.clone())
    };
    match &mut ast.kind {
        ExprKind::Unary { op, operand } => {
            let op = *op;
            let mut inner = (context, Adopts::No);
            if ast.ty == InferredType::UntypedInt && op != UnaryOp::Not {
                match width {
                    Some(width) => {
                        ast.ty = width;
                        inner = (width, Adopts::Yes);
                    }
                    None if op == UnaryOp::BitNot => {
                        return Err(unwidened(
                            "an operand of `~` of a declared integer type, or a declared \
                             integer place it is stored into or compared with, which gives \
                             it its width"
                                .to_string(),
                            operand,
                        ));
                    }
                    None => {}
                }
            }
            land_integers(operand, inner.0, inner.1, fit, source)
        }
        // `x ?? d`: the default takes the type `x` holds when present
        // (docs/adr/0055).
        ExprKind::Binary {
            op: BinOp::Coalesce,
            left,
            right,
        } => {
            land_integers(left, InferredType::Unknown, Adopts::No, fit, source)?;
            let held = left.ty.unwrap_optional();
            land_integers(right, held, Adopts::No, fit, source)
        }
        ExprKind::Binary { op, left, right } => {
            let op = *op;
            // An optional operand of `===` meets a literal as the value it
            // holds when present.
            let joined =
                binary_operand_type(op, left.ty.unwrap_optional(), right.ty.unwrap_optional());
            let operation = op.is_arith() || op.is_bitwise();
            let (operand, operands_adopt) = match joined {
                InferredType::UntypedInt if operation => match width {
                    Some(width) => {
                        ast.ty = width;
                        (width, Adopts::Yes)
                    }
                    None if op.is_shift() => {
                        return Err(unwidened(
                            format!(
                                "a left operand of `{}` of a declared integer type, or a \
                                 declared integer place the shift is stored into or compared \
                                 with, which gives it its width",
                                op.token()
                            ),
                            left,
                        ));
                    }
                    None => (context, Adopts::No),
                },
                joined => (joined, Adopts::No),
            };
            // The count of a shift is an integer of its own, which takes no type
            // from what it shifts.
            let (count, count_adopts) = if op.is_shift() {
                (InferredType::Unknown, Adopts::No)
            } else {
                (operand, operands_adopt)
            };
            land_integers(left, operand, operands_adopt, fit, source)?;
            land_integers(right, count, count_adopts, fit, source)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            land_integers(condition, InferredType::Bool, Adopts::No, fit, source)?;
            // A conditional of literals that is an operand of such an operation
            // is one of its operands: `(c ? 1 : 2) << m`.
            let mut branches = (context, Adopts::No);
            if let (Adopts::Yes, Some(width), InferredType::UntypedInt) = (adopts, width, ast.ty) {
                ast.ty = width;
                branches = (width, Adopts::Yes);
            }
            land_integers(consequent, branches.0, branches.1, fit, source)?;
            land_integers(alternate, branches.0, branches.1, fit, source)
        }
        ExprKind::Call { args, params, .. } => {
            for (i, arg) in args.iter_mut().enumerate() {
                let param = argument_type(params.as_slice(), i);
                land_integers(arg, param, Adopts::No, fit, source)?;
            }
            Ok(())
        }
        ExprKind::Index { object, index } => {
            land_integers(object, InferredType::Unknown, Adopts::No, fit, source)?;
            land_integers(index, InferredType::Unknown, Adopts::No, fit, source)
        }
        ExprKind::Member { object, .. } => {
            land_integers(object, InferredType::Unknown, Adopts::No, fit, source)
        }
        ExprKind::BytesView { source: of, len } => {
            land_integers(of, InferredType::Unknown, Adopts::No, fit, source)?;
            match len {
                Some(len) => land_integers(len, InferredType::Unknown, Adopts::No, fit, source),
                None => Ok(()),
            }
        }
        // Its operands are checked at the operation's own type; a total
        // operation's literal takes the type of its other operand, which is
        // the operation's.
        ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
            let own = ast.ty;
            land_integers(left, own, Adopts::No, fit, source)?;
            match right {
                Some(right) => land_integers(right, own, Adopts::No, fit, source),
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

/// `node` as an integer literal as written — its text, and whether a minus
/// leads it (SCE_FORGE.md §3.4: the minus is part of the literal) — or `None`
/// for anything else.
fn integer_literal_parts(node: &TypedExpr) -> Option<(String, bool)> {
    match &node.kind {
        ExprKind::NumberLit(text) if !is_float_literal_text(text) => Some((text.clone(), false)),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match &operand.kind {
            ExprKind::NumberLit(text) if !is_float_literal_text(text) => Some((text.clone(), true)),
            _ => None,
        },
        _ => None,
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
/// size passes `i64::MAX`. [`land_integers`] has held every
/// literal to its type, so there are exactly two — a `uint64` value above
/// `i64::MAX`, and `int64`'s minimum, the negation of `2^63`.
///
/// ⚠ Both were emitted bare until 2026-09-24: gcc and g++ refuse a decimal
/// constant that large under `-Werror` ("so large that it is unsigned"),
/// and kotlinc refuses it outright ("value out of range").
///
/// The same two values reach a backend a second way — folded into a
/// `<sce:const>` table's elements — and are spelled there by the same
/// [`wide_literal_spelling`], so an expression and a table cannot disagree
/// about how C, C++ or Kotlin writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WideLiteral {
    Unsigned,
    Int64Min,
}

const LONG_MAX: u64 = i64::MAX.unsigned_abs();

impl WideLiteral {
    /// An unsigned value as a [`WideLiteral`], when it passes `i64::MAX`.
    pub(crate) fn of_unsigned(value: u64) -> Option<Self> {
        (value > LONG_MAX).then_some(Self::Unsigned)
    }

    /// A signed value as a [`WideLiteral`], when it is `i64::MIN`.
    pub(crate) fn of_signed(value: i64) -> Option<Self> {
        (value == i64::MIN).then_some(Self::Int64Min)
    }
}

/// `node` as a [`WideLiteral`], when it is one.
fn wide_literal(node: &TypedExpr) -> Option<WideLiteral> {
    let magnitude = |kind: &ExprKind| match kind {
        ExprKind::NumberLit(text) if !is_float_literal_text(text) => integer_literal_value(text),
        _ => None,
    };
    match &node.kind {
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => (magnitude(&operand.kind) == Some(LONG_MAX + 1)).then_some(WideLiteral::Int64Min),
        kind => magnitude(kind).and_then(WideLiteral::of_unsigned),
    }
}

/// `INT64_MIN` in C and C++. `-9223372036854775808LL` negates a constant
/// `long long` cannot hold, so the value is spelled as an expression.
pub(crate) const C_INT64_MIN: &str = "(-9223372036854775807LL - 1)";

/// `Long`'s minimum in Kotlin, which has no literal for it.
const KOTLIN_LONG_MIN: &str = "Long.MIN_VALUE";

/// `wide`, whose unsigned value is written `raw`, as `lang` spells it; `None`
/// for a backend whose bare literal already carries it (Rust, Go, Python,
/// Swift: `let b: UInt64 = 18446744073709551615` and
/// `let a: Int64 = -9223372036854775808` both compile as written).
pub(crate) fn wide_literal_spelling(
    lang: crate::generator::Language,
    wide: WideLiteral,
    raw: &str,
) -> Option<String> {
    use crate::generator::Language;
    match (lang, wide) {
        (Language::C11 | Language::Cpp, WideLiteral::Unsigned) => Some(format!("{raw}ULL")),
        (Language::C11 | Language::Cpp, WideLiteral::Int64Min) => Some(C_INT64_MIN.to_string()),
        (Language::Kotlin, WideLiteral::Unsigned) => Some(format!("{raw}uL")),
        (Language::Kotlin, WideLiteral::Int64Min) => Some(KOTLIN_LONG_MIN.to_string()),
        (Language::Rust | Language::Go | Language::Python | Language::Swift, _) => None,
    }
}

/// `node`, of integer type `to`, as C and C++ spell it when it is a
/// [`WideLiteral`]; `None` for anything else, which stands as emitted.
fn c_family_wide_literal(raw: &str, to: InferredType, node: &TypedExpr) -> Option<String> {
    if !matches!(to.strip_quantity(), InferredType::Int { .. }) {
        return None;
    }
    // C and C++ spell both the same way; either names the family.
    wide_literal_spelling(crate::generator::Language::C11, wide_literal(node)?, raw)
}

/// `node`, an integer literal whose context gave it its type
/// ([`land_integers`]: an operand of an operation none of whose operands has a
/// declared type), written in that type for `target`: `1u64`, `UINT64_C(1)`,
/// `std::uint64_t{1}`, `uint64(1)`, `1uL`, `UInt64(1)`. `None` for any other
/// node, and for a target whose bare literal already has that type: Python's
/// integers have no width (its operations are reduced to theirs), and the
/// Interpreter's library takes the type from the operation it is called for.
///
/// A [`WideLiteral`] keeps its own spelling in C, C++ and Kotlin
/// ([`wide_literal_spelling`]), which their coercions write and which is
/// already of the 64-bit type.
///
/// ⚠ Until 2026-11-19 such a literal was written bare, so it was the
/// language's default integer: `prev | (1 << m)` with `prev` a `uint64` and
/// `m` a `uint8` typed the shift as `m`'s `uint8`, and Rust wrote
/// `(1 << m) as u64`, an `i32` shift that panics in a debug build from
/// `m = 32` and sets the wrong bits from `m = 31`; C shifted an `int`, Kotlin
/// an `Int`, Swift a literal its type checker typed, Python masked the shift
/// to 8 bits and the Interpreter wrapped it at 8.
fn typed_integer_literal(target: ExprTarget, node: &TypedExpr) -> Option<String> {
    let InferredType::Int { signed, bits } = node.ty else {
        return None;
    };
    let (text, negated) = integer_literal_parts(node)?;
    let digits = portable_number_literal(&text);
    let written = if negated {
        format!("-{digits}")
    } else {
        digits
    };
    let wide = wide_literal(node).is_some();
    Some(match target {
        ExprTarget::Rust => return rust_typed_literal(node).map(|literal| literal.to_string()),
        ExprTarget::Go => format!("{}({written})", go_int_type(signed, bits)),
        ExprTarget::Swift => format!("{}({written})", swift::int_type(signed, bits)),
        ExprTarget::C | ExprTarget::Cpp | ExprTarget::Kotlin if wide => return None,
        ExprTarget::C => format!("{}INT{bits}_C({written})", if signed { "" } else { "U" }),
        ExprTarget::Cpp => format!(
            "std::{}int{bits}_t{{{written}}}",
            if signed { "" } else { "u" }
        ),
        ExprTarget::Kotlin => kotlin_typed_literal(&written, negated, signed, bits),
        ExprTarget::Python | ExprTarget::Js => return None,
    })
}

/// [`typed_integer_literal`] for Kotlin: a suffix where the type has one
/// (`1L`, `1u`, `1uL`, and an `Int` bare), and the conversion of the `Int`
/// literal to a type that has none (`1.toUByte()`, `(-1).toShort()`).
fn kotlin_typed_literal(written: &str, negated: bool, signed: bool, bits: u8) -> String {
    match (signed, bits) {
        (true, 32) => written.to_string(),
        (true, 64) => format!("{written}L"),
        (false, 32) => format!("{written}u"),
        (false, 64) => format!("{written}uL"),
        _ => {
            let ctor = if signed {
                kotlin_signed_ctor(bits)
            } else {
                kotlin_unsigned_ctor(bits)
            };
            if negated {
                format!("({written}).{ctor}()")
            } else {
                format!("{written}.{ctor}()")
            }
        }
    }
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
            ExprKind::Ident(name) => ctx.lookup_func(name).map(|sig| (name.clone(), sig.clone())),
            ExprKind::Member { object, property } => match &object.kind {
                ExprKind::Ident(object) => ctx
                    .lookup_member_func(object, property)
                    .map(|sig| (format!("{object}.{property}"), sig)),
                _ => None,
            },
            _ => None,
        };
        if let Some((name, signature)) = registered {
            // Before arity: a host-only signature declares no parameters, and
            // "expects 0 arguments" would name a rule nobody wrote.
            let observed = || {
                callee
                    .span
                    .clone()
                    .and_then(|span| source.get(span))
                    .map(str::to_string)
            };
            if let Some(slot) = &signature.host_only {
                return Err(host_only_call(&name, slot, observed()).at(callee.span.clone()));
            }
            if signature.may_fail && !ctx.receives_failures {
                return Err(unreceived_failure_call(&name, observed()).at(callee.span.clone()));
            }
            judge_arguments(&name, &signature.params, callee, args, source)?;
            if let Some(arg) = string_for_a_bytes_parameter(&signature.params, args) {
                return Err(string_where_bytes_are_held(arg, source));
            }
        } else if is_bytes_eq_builtin(callee, ctx) {
            judge_arguments("eq", &BYTES_EQ_PARAMS, callee, args, source)?;
        }
    }
    for child in expr_children(expr) {
        reject_call_argument_mismatches(child, ctx, source)?;
    }
    Ok(())
}

/// A call's `args` against the parameters `params` of the function `name`
/// its `callee` names: as many as it takes, refused at the callee, each of
/// a kind its parameter admits ([`slot_admits`]), refused at that argument.
fn judge_arguments(
    name: &str,
    params: &[InferredType],
    callee: &TypedExpr,
    args: &[TypedExpr],
    source: &str,
) -> Result<(), Refusal> {
    if args.len() != params.len() {
        return Err(ExprError::ArgumentCount {
            callee: name.to_string(),
            expected: params.len(),
            actual: args.len(),
        }
        .at(callee.span.clone()));
    }
    match params
        .iter()
        .zip(args)
        .find(|(param, arg)| !slot_admits(**param, arg.ty))
    {
        Some((param, arg)) => Err(type_mismatch(*param, arg, source)),
        None => Ok(()),
    }
}

/// The first of a registered function's `args` that is a string where its
/// parameter is a byte string. [`slot_admits`] lets a string through where bytes
/// are declared, which a payload field and the `eq` builtin rely on, but a call
/// hands its argument as it is, and no backend takes a text for a byte string: it
/// was generated and refused by the target language's compiler. A text is not
/// converted here either, since a text's bytes are of an encoding and a byte
/// string's text is Latin-1 (docs/adr/0005, decision 2).
fn string_for_a_bytes_parameter<'a>(
    params: &[InferredType],
    args: &'a [TypedExpr],
) -> Option<&'a TypedExpr> {
    params
        .iter()
        .zip(args)
        .find(|(param, arg)| {
            let present = match **param {
                InferredType::Optional(elem) => elem.present(),
                param => param,
            };
            present == InferredType::Bytes && arg.ty == InferredType::Str
        })
        .map(|(_, arg)| arg)
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

/// [`resolve`], then judged against what `expected` says about the place
/// the value lands in, exactly as [`transpile_expecting`] judges it — for a
/// validator that refuses a document before any backend lowers it. Returns
/// the value's type, as inference gives it: landing the value types only the
/// operations among its literals ([`land_integers`]).
///
/// Emitter-specific refusals are not reached: they belong to the backend
/// that lowers the expression, and a backend that does not lower the
/// document says so itself.
pub(crate) fn judge_into(
    expr: &str,
    ctx: &TypeCtx<'_>,
    expected: Expected,
) -> Result<InferredType, Refusal> {
    let expr = expr.trim();
    let mut ast = resolve(expr, ctx)?;
    let ty = ast.ty;
    judge_value(&mut ast, expected, expr)?;
    Ok(ty)
}

/// [`judge_into`]'s tree rather than its type: resolved, judged, and typed
/// as the emitters lower it — an operation of literals at the type its
/// context gives it ([`land_integers`]) — for an analysis that has to judge
/// each operation at the width it is emitted at.
pub(crate) fn judged(
    expr: &str,
    ctx: &TypeCtx<'_>,
    expected: Expected,
) -> Result<TypedExpr, Refusal> {
    let expr = expr.trim();
    let mut ast = resolve(expr, ctx)?;
    judge_value(&mut ast, expected, expr)?;
    Ok(ast)
}

/// The rules of the narrowing alone, over `expr` read in `ctx` — whose
/// [`TypeCtx::proven`] holds what the tests that hold where it stands prove —
/// and landing where `expected` says: a `.value` read where nothing proves it
/// present, an optional value where it has no answer
/// ([`reject_optional_misuse`], docs/adr/0055), a case's field read where no
/// test proves its case ([`reject_unproven_union_reads`], docs/adr/0092,
/// section 6.1), an argument of a parameter that takes a union whole which is
/// no name of a union of its document ([`reject_union_arguments`]), and an
/// optional value or `null` where a place that is never absent is declared.
/// For a body judged before any backend renders it, whose other refusals are
/// the renderer's.
pub(crate) fn judge_narrowed_use(
    expr: &str,
    ctx: &TypeCtx<'_>,
    expected: Expected,
) -> Result<(), Refusal> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(());
    }
    let mut ast = parse_to_ast(expr)?;
    infer_types(&mut ast, ctx);
    reject_optional_misuse(&ast, &ctx.proven, expr)?;
    reject_union_elements_as_parts(&ast, ctx, expr, true)?;
    reject_unproven_union_reads(&ast, ctx, &ctx.proven, expr)?;
    reject_union_arguments(&ast, ctx, expr)?;
    if let Expected::Slot(slot) = expected {
        let may_be_absent = ast.ty.is_optional() || ast.ty == InferredType::Null;
        if may_be_absent && !slot_admits(slot, ast.ty) {
            return Err(type_mismatch(slot, &ast, expr));
        }
    }
    Ok(())
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
            ..
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
                                // A stateful import's method returns a value.
                                fails: false,
                                into_destination: false,
                                storage: None,
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
        ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
            lower_stateful_import_calls(left, lowerings);
            if let Some(right) = right {
                lower_stateful_import_calls(right, lowerings);
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
    reject_unlowered_optional(&ast, target)?;
    if !renames.is_empty() {
        rename_identifiers(&mut ast, renames);
    }

    // LHS has no top-down coercion context — it is a storage location, not a
    // value being shoved into an expected type. Pass `Unknown` so the
    // per-language emitter emits the identifier/member path verbatim.
    let emitted = match target {
        ExprTarget::Cpp => emit_cpp(&ast, InferredType::Unknown)?,
        ExprTarget::Kotlin => emit_kotlin(&ast, InferredType::Unknown)?,
        // A place that holds an optional text or byte string is read borrowed
        // ([`rust_optional::view`]); the place assigned to is the storage itself.
        ExprTarget::Rust
            if matches!(ty, InferredType::Optional(_))
                && matches!(
                    ast.kind,
                    ExprKind::Ident(_) | ExprKind::Raw(_) | ExprKind::Member { .. }
                ) =>
        {
            rust_emit_value(&ast)?.to_string()
        }
        ExprTarget::Rust => emit_rust(&ast, InferredType::Unknown)?.to_string(),
        ExprTarget::Go => emit_go(&ast, InferredType::Unknown)?,
        ExprTarget::Swift => swift::emit(&ast, InferredType::Unknown)?,
        ExprTarget::Python => emit_python(&ast, InferredType::Unknown)?,
        ExprTarget::C => emit_c(&ast, InferredType::Unknown)?,
        ExprTarget::Js => emit_js(&ast, InferredType::Unknown)?,
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

/// `true` iff `expr` names `_event` itself — any member of it, or the event
/// whole — decided on the token stream for the reasons
/// [`references_event_data_lexically`] gives: a `_event` inside a string
/// literal is no read of it, and a member of another value that happens to be
/// spelt `_event` (`x._event`) is none either.
///
/// What a test component asks of every expression it holds
/// (docs/SCE_TEST_PROFILE.md §4.4): the received message is read only in the
/// executable content of a receive branch. An expression that does not
/// tokenize reads as `false`, and is refused where it is typed.
pub(crate) fn references_event_lexically(expr: &str) -> bool {
    let Ok(tokens) = tokenize(expr.trim()) else {
        return false;
    };
    tokens.iter().enumerate().any(|(at, token)| {
        matches!(token, Token::Ident(s) if s == "_event")
            && (at == 0 || tokens[at - 1] != Token::Dot)
    })
}

/// Whether `location` is the shape of an assignment target — a variable, or
/// a field of one — before any type is asked of it
/// ([`transpile_lvalue`] holds a location to the same rule when it lowers
/// one). A judge that reads a location as an expression to type it needs this
/// first: an index read is an expression, and an element of a list is read by
/// its index and never written.
pub fn judge_lvalue_shape(location: &str) -> Result<(), Refusal> {
    let trimmed = location.trim();
    let ast = parse_to_ast(trimmed)?;
    validate_lvalue_shape(&ast.kind, trimmed).map_err(|refusal| refusal.at(ast.span.clone()))
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
        ExprKind::Checked { .. } => "checked integer operation",
        ExprKind::Total { .. } => "wrapping or saturating integer operation",
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

impl TypedExpr {
    /// A node no source text spells.
    fn new(kind: ExprKind) -> Self {
        Self {
            kind,
            ty: InferredType::Unknown,
            span: None,
        }
    }

    /// Every expression directly inside this one, in the order it is
    /// written — the one walk every pass over the tree takes, so a node kind
    /// added later is either walked everywhere or fails to compile here.
    pub(crate) fn children(&self) -> Vec<&TypedExpr> {
        match &self.kind {
            ExprKind::NumberLit(_)
            | ExprKind::StringLit { .. }
            | ExprKind::BytesLit { .. }
            | ExprKind::BoolLit(_)
            | ExprKind::NullLit
            | ExprKind::Ident(_)
            | ExprKind::Raw(_) => Vec::new(),
            ExprKind::Binary { left, right, .. } => vec![left, right],
            ExprKind::Unary { operand, .. } => vec![operand],
            ExprKind::Conditional {
                condition,
                consequent,
                alternate,
            } => vec![condition, consequent, alternate],
            ExprKind::Member { object, .. } => vec![object],
            ExprKind::Index { object, index } => vec![object, index],
            ExprKind::Call { callee, args, .. } => {
                std::iter::once(&**callee).chain(args.iter()).collect()
            }
            ExprKind::BytesView { source, len } => {
                std::iter::once(&**source).chain(len.as_deref()).collect()
            }
            ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
                std::iter::once(&**left).chain(right.as_deref()).collect()
            }
        }
    }

    /// [`children`](Self::children), for a pass that rewrites them.
    pub(crate) fn children_mut(&mut self) -> Vec<&mut TypedExpr> {
        match &mut self.kind {
            ExprKind::NumberLit(_)
            | ExprKind::StringLit { .. }
            | ExprKind::BytesLit { .. }
            | ExprKind::BoolLit(_)
            | ExprKind::NullLit
            | ExprKind::Ident(_)
            | ExprKind::Raw(_) => Vec::new(),
            ExprKind::Binary { left, right, .. } => vec![left, right],
            ExprKind::Unary { operand, .. } => vec![operand],
            ExprKind::Conditional {
                condition,
                consequent,
                alternate,
            } => vec![condition, consequent, alternate],
            ExprKind::Member { object, .. } => vec![object],
            ExprKind::Index { object, index } => vec![object, index],
            ExprKind::Call { callee, args, .. } => std::iter::once(&mut **callee)
                .chain(args.iter_mut())
                .collect(),
            ExprKind::BytesView { source, len } => std::iter::once(&mut **source)
                .chain(len.as_deref_mut())
                .collect(),
            ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
                std::iter::once(&mut **left)
                    .chain(right.as_deref_mut())
                    .collect()
            }
        }
    }
}

/// Structural equality: two trees are the same expression whatever text
/// they were read from, so `span` — where, not what — takes no part.
impl PartialEq for TypedExpr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.ty == other.ty
    }
}

/// The structural content of an expression node. Does not carry type
/// information on its own — that lives in the enclosing [`TypedExpr`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExprKind {
    NumberLit(String),
    /// A string literal, carrying the text it denotes: its escapes were read
    /// by the lexer ([`decode_string_literal`]), so `value` is characters and
    /// not the author's spelling of them, and which quote the author wrote it
    /// in is not part of what it is. Every emitter writes `value` through its
    /// own language's escaper, and every measure of it (a byte count, a
    /// comparison, the bytes a byte string stands for) is a measure of the
    /// text itself.
    StringLit {
        value: String,
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
        /// The function the call resolved to declares `may-fail`
        /// (SCE_FORGE.md §3.4.1), so the call hands a failure in place of a
        /// value, which each backend passes on ([`pass_failure_on`]).
        /// Written by [`infer_types`] with `params`; a call it is `true` for
        /// is refused anywhere but a `may-fail` algorithm's body.
        fails: bool,
        /// The function the call resolved to writes the list it returns into a
        /// destination the call hands it ([`CallReading::IntoDestination`],
        /// docs/adr/0082), which Rust and C11 spell. Written by [`infer_types`]
        /// with `fails`.
        into_destination: bool,
        /// The storage the caller holds for the call and hands it after the
        /// arguments it is written with ([`FuncSig::storage`], docs/adr/0082), as
        /// the backend spells a pointer to it. Written by [`infer_types`] with
        /// `fails`.
        storage: Option<String>,
    },
    /// A borrowed `bytes`-view projection of a `Str`-typed source. Not
    /// produced by the parser — [`infer_types`] wraps a call argument in
    /// this node when the argument is `Str`-typed and the resolved callee
    /// parameter is `Bytes` (and only when the TypeCtx enables
    /// `project_str_args_as_bytes_view` — the algorithm kind). It lowers a
    /// bounded-string element field (`entry.pattern`) to each backend's
    /// borrowed byte-view idiom at the call site: Rust `.as_bytes()`, C11
    /// a `sce_forge_bytes_view_t` over `{src, src_len}`, Cpp a
    /// `SCE::Forge::Span<const std::uint8_t>` over the string's data/size, Go
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
    /// An integer operation that can fail, in an algorithm that declares
    /// `may-fail` (SCE_FORGE.md §3.4.1, the integer arithmetic contract):
    /// an overflow of the node's type, a division by zero, or a signed
    /// `MIN / -1` hands a failure to the caller instead of a value.
    ///
    /// A rewrite of a `Binary` or `Unary` node ([`check_integer_arithmetic`])
    /// run once the tree is judged and renamed, just before emission, so
    /// every pass that reads arithmetic — inference, the range analysis in
    /// [`crate::forge::int_ranges`], the literal checks — sees the operators
    /// the author wrote. The node's `ty` is the integer type the operation
    /// is checked at; `right` is `None` for a negation.
    ///
    /// The same rewrite marks the operations of an algorithm that does not
    /// declare `may-fail` for the Rust emitter, whose interval analysis proved
    /// that none fails ([`Discharge::AtBuild`], [`prove_integer_operations`]).
    Checked {
        op: CheckedOp,
        left: Box<TypedExpr>,
        right: Option<Box<TypedExpr>>,
        discharge: Discharge,
    },
    /// A ring operation the author wrote under a rule that makes it total —
    /// `wrappingSub(now, last)`, `saturatingAdd(level, 3)` (docs/adr/0063):
    /// the exact result reduced into the node's type, the type both operands
    /// share, modulo 2^W or held to the type's bounds. It never fails, in a
    /// `may-fail` body or anywhere else.
    ///
    /// The parser builds it from a call of a vocabulary name
    /// ([`crate::forge::ring_ops::VOCABULARY`]), so every pass reads the
    /// operation the author named rather than recognising a call by its
    /// callee. Unlike [`ExprKind::Checked`], which the pipeline writes over
    /// the author's operators just before emission, it is in the tree from
    /// the first pass. `right` is `None` for a negation.
    Total {
        op: RingOp,
        rule: OverflowRule,
        left: Box<TypedExpr>,
        right: Option<Box<TypedExpr>>,
    },
}

/// The integer operations [`ExprKind::Checked`] carries — exactly those the
/// integer arithmetic contract names (SCE_FORGE.md §3.4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckedOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    /// A value stored where a narrower integer type is declared — the node's
    /// `ty` is that type and `left` the value, at its own type
    /// ([`check_integer_narrowing`]).
    Narrow,
    /// A read of `left[right]` — `left` the collection, `right` the index,
    /// the node's `ty` the element's — which fails out of range rather than
    /// reading past the end ([`check_integer_indexing`]).
    Index,
    /// `left << right` where the count `right` is not known to be at least 0
    /// and below the width of `left`, the node's `ty` ([`settle_shifts`]): the
    /// exact result reduced to that width for a count at least 0, a count at
    /// or past the width included, and an out-of-range failure for one below 0
    /// (SCE_FORGE.md §3.4.1, "The width of a shift"). The node's discharge says
    /// which: [`Discharge::AtRun`] checks the count when it runs;
    /// [`Discharge::Total`] is the total shift, whose count the build knows
    /// is at least 0, or, outside the bodies the contract covers, leaves to the
    /// caller as it leaves a divisor of 0; [`Discharge::AtBuild`] is one whose
    /// count is proven below the width, which Rust alone is handed.
    Shl,
    /// `left >> right`, the arithmetic shift of a signed `left`, settled as
    /// [`CheckedOp::Shl`] is.
    Shr,
    /// `left >>> right`, the logical shift of `left`'s bits at its width,
    /// settled as [`CheckedOp::Shl`] is; also one whose count is below the
    /// width, where `left` is signed, since no C-family, Rust, Go or Python
    /// operator shifts a signed value's bits logically.
    UShr,
}

impl CheckedOp {
    fn of_binary(op: BinOp) -> Option<Self> {
        Some(match op {
            BinOp::Add => Self::Add,
            BinOp::Sub => Self::Sub,
            BinOp::Mul => Self::Mul,
            BinOp::Div => Self::Div,
            BinOp::Mod => Self::Rem,
            _ => return None,
        })
    }

    /// The runtime helper's name — one spelling every backend's checked
    /// arithmetic helpers share.
    pub(crate) fn helper(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Div => "div",
            Self::Rem => "rem",
            Self::Neg => "neg",
            Self::Narrow => "narrow",
            Self::Index => "at",
            Self::Shl => "shl",
            Self::Shr => "shr",
            Self::UShr => "ushr",
        }
    }

    /// The shift a shift operator is, or `None` for any other operator.
    fn of_shift(op: BinOp) -> Option<Self> {
        Some(match op {
            BinOp::Shl => Self::Shl,
            BinOp::Shr => Self::Shr,
            BinOp::UShr => Self::UShr,
            _ => return None,
        })
    }

    /// The shift operator this operation is, or `None` for one that is not a
    /// shift.
    pub(crate) fn shift(self) -> Option<BinOp> {
        match self {
            Self::Shl => Some(BinOp::Shl),
            Self::Shr => Some(BinOp::Shr),
            Self::UShr => Some(BinOp::UShr),
            _ => None,
        }
    }
}

/// When the integer arithmetic contract (SCE_FORGE.md §3.4.1) is discharged
/// for an [`ExprKind::Checked`] operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Discharge {
    /// When the operation runs, in an algorithm that declares `may-fail`: a
    /// failure is handed to the caller in place of a value.
    AtRun,
    /// When the document is built: the interval analysis proved that the
    /// operation, in an algorithm that does not declare `may-fail`, never
    /// fails, and the algorithm was refused had it not
    /// (`algorithm/undeclared-integer-failure`). Rust prints it through the
    /// forge runtime's `algorithm::proven`, so the crate a generated file is
    /// compiled in holds no operator and no index of its own; every other
    /// backend prints the operator, and is never handed this node.
    ///
    /// A shift ([`CheckedOp::Shl`], [`CheckedOp::Shr`], [`CheckedOp::UShr`]) is
    /// marked so only where its count is proven at least 0 and below the width
    /// and is not a literal ([`prove_integer_operations`]): Rust prints it through
    /// `algorithm::proven::shl` or `shr`.
    AtBuild,
    /// Never: the operation has a value for every input it can be handed. A
    /// shift ([`CheckedOp::Shl`], [`CheckedOp::Shr`], [`CheckedOp::UShr`]) whose
    /// count is known at least 0 but not known below the width, or one outside
    /// the bodies the contract covers ([`settle_shifts`]): its exact result
    /// reduced to the width, a count at or past the width included, which every
    /// backend prints as its total shift. No operation of the contract is ever
    /// marked so.
    Total,
}

/// The refusal of a backend other than Rust handed an operation proven at build
/// time ([`Discharge::AtBuild`]), which only the Rust emitter prints apart from
/// the operator it stands for ([`prove_integer_operations`]).
fn proven_operation_outside_rust() -> ExprError {
    ExprError::UnsupportedConstruct {
        construct: "an integer operation proven at build time, which only Rust prints \
                    apart from its operator"
            .to_string(),
        observed: None,
    }
}

/// Rewrite every index read of `node` into a [`CheckedOp::Index`] node
/// (SCE_FORGE.md §3.4.1): in a `may-fail` body an index below 0 or not below
/// its collection's length is a failure, as an overflow is, and every
/// backend spells the read so it cannot run past the end.
pub(crate) fn check_integer_indexing(node: &mut TypedExpr, discharge: Discharge) {
    for child in node.children_mut() {
        check_integer_indexing(child, discharge);
    }
    if !matches!(node.kind, ExprKind::Index { .. }) {
        return;
    }
    let placeholder = ExprKind::NullLit;
    if let ExprKind::Index { object, index } = std::mem::replace(&mut node.kind, placeholder) {
        node.kind = ExprKind::Checked {
            op: CheckedOp::Index,
            left: object,
            right: Some(index),
            discharge,
        };
    }
}

/// Rewrite every integer value of `node` that lands where a narrower integer
/// type is declared into a [`CheckedOp::Narrow`] node (SCE_FORGE.md §3.4.1):
/// `node` itself when `place` is that type, and each argument of a call in
/// its parameter's type.
///
/// Only a value whose type can hold something `place` cannot is rewritten —
/// a `uint8` stored in an `int32` never fails. A literal is not: it is the
/// author's own value, and one its place cannot hold is refused at build
/// time (`expression/literal-out-of-range`). Runs after
/// [`check_integer_arithmetic`], so a narrowed operation is checked at its
/// own width first.
pub(crate) fn check_integer_narrowing(node: &mut TypedExpr, place: Option<InferredType>) {
    match &mut node.kind {
        ExprKind::Call { args, params, .. } => {
            let params = params.clone();
            for (i, arg) in args.iter_mut().enumerate() {
                check_integer_narrowing(arg, params.get(i).copied());
            }
        }
        _ => {
            for child in node.children_mut() {
                check_integer_narrowing(child, None);
            }
        }
    }
    let Some(place) = place else {
        return;
    };
    if matches!(node.kind, ExprKind::NumberLit(_)) || !integer_can_leave(node.ty, place) {
        return;
    }
    let value = std::mem::replace(node, TypedExpr::new(ExprKind::NullLit));
    let span = value.span.clone();
    *node = TypedExpr {
        kind: ExprKind::Checked {
            op: CheckedOp::Narrow,
            left: Box::new(value),
            right: None,
            discharge: Discharge::AtRun,
        },
        ty: place,
        span,
    };
}

/// Whether a value of integer type `from` can be one integer type `to`
/// cannot hold.
fn integer_can_leave(from: InferredType, to: InferredType) -> bool {
    match (from.int_bounds(), to.int_bounds()) {
        (Some((flo, fhi)), Some((tlo, thi))) => flo < tlo || fhi > thi,
        _ => false,
    }
}

/// Rewrite every integer `+ - * / %` and unary `-` of `node` into an
/// [`ExprKind::Checked`] node (SCE_FORGE.md §3.4.1).
///
/// Only an operation whose type is a declared integer width is rewritten:
/// an operation on two literals has no width until it lands, and a real or
/// a string operation has no failure to check. Nor is one in float context
/// — `expected` a real, pushed down through arithmetic as every emitter
/// pushes it — which divides and multiplies as reals (§3.4.1's table).
pub(crate) fn check_integer_arithmetic(
    node: &mut TypedExpr,
    expected: InferredType,
    discharge: Discharge,
) {
    let float_context = matches!(expected, InferredType::Float { .. });
    match &mut node.kind {
        ExprKind::Binary { op, left, right } => {
            let inner = if float_context && op.is_arith() {
                expected
            } else {
                InferredType::Unknown
            };
            check_integer_arithmetic(left, inner, discharge);
            check_integer_arithmetic(right, inner, discharge);
        }
        ExprKind::Unary { op, operand } => {
            let inner = if float_context && matches!(op, UnaryOp::Neg | UnaryOp::Pos) {
                expected
            } else {
                InferredType::Unknown
            };
            check_integer_arithmetic(operand, inner, discharge);
        }
        // Every other child is emitted at its own type, whatever the
        // enclosing context expects.
        ExprKind::Conditional { .. }
        | ExprKind::Member { .. }
        | ExprKind::Index { .. }
        | ExprKind::Call { .. }
        | ExprKind::BytesView { .. }
        | ExprKind::Checked { .. }
        | ExprKind::Total { .. }
        | ExprKind::NumberLit(_)
        | ExprKind::StringLit { .. }
        | ExprKind::BytesLit { .. }
        | ExprKind::BoolLit(_)
        | ExprKind::NullLit
        | ExprKind::Ident(_)
        | ExprKind::Raw(_) => {
            for child in node.children_mut() {
                check_integer_arithmetic(child, InferredType::Unknown, discharge);
            }
        }
    }
    // A negative literal is a value, not an operation (SCE_FORGE.md §3.4: the
    // minus is part of the literal), though it has a type where its context
    // gave it one ([`land_integers`]).
    if float_context
        || !matches!(node.ty, InferredType::Int { .. })
        || integer_literal_parts(node).is_some()
    {
        return;
    }
    let placeholder = ExprKind::NullLit;
    node.kind = match std::mem::replace(&mut node.kind, placeholder) {
        ExprKind::Binary { op, left, right } => match CheckedOp::of_binary(op) {
            Some(op) => ExprKind::Checked {
                op,
                left,
                right: Some(right),
                discharge,
            },
            None => ExprKind::Binary { op, left, right },
        },
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => ExprKind::Checked {
            op: CheckedOp::Neg,
            left: operand,
            right: None,
            discharge,
        },
        other => other,
    };
}

/// Mark every integer `+ - * / %`, unary `-` and index read of `node` as an
/// operation the interval analysis proved never fails ([`Discharge::AtBuild`]),
/// for the Rust emitter: the body of an algorithm that does not declare
/// `may-fail` ([`TypeCtx::integers_proven`]), which the analysis refuses
/// unless every such operation is proven (SCE_FORGE.md §3.4.1, "Enforced").
///
/// The rewrite is [`check_integer_arithmetic`]'s and [`check_integer_indexing`]'s,
/// so the operations marked are exactly those a `may-fail` body checks. A store
/// into a narrower type is not marked: the analysis proved it fits, and Rust
/// stores it with a cast, which neither fails nor panics.
///
/// [`TypeCtx::integers_proven`]: crate::forge::types::TypeCtx::integers_proven
pub(crate) fn prove_integer_operations(node: &mut TypedExpr, expected: InferredType) {
    check_integer_arithmetic(node, expected, Discharge::AtBuild);
    check_integer_indexing(node, Discharge::AtBuild);
    prove_shifts(node);
}

/// Mark every shift of `node` that [`settle_shifts`] left the operator it is,
/// whose count is therefore proven at least 0 and below the width, as proven
/// ([`Discharge::AtBuild`]), for the Rust emitter: `clippy::arithmetic_side_effects`
/// reads a shift by a count that is not a literal as one that can overflow, so
/// it is printed through the runtime's `algorithm::proven` as the other proven
/// operations are. A literal count is left the operator, which the lint passes.
fn prove_shifts(node: &mut TypedExpr) {
    for child in node.children_mut() {
        prove_shifts(child);
    }
    let ExprKind::Binary { op, right, .. } = &node.kind else {
        return;
    };
    let Some(shift) = CheckedOp::of_shift(*op) else {
        return;
    };
    if !matches!(node.ty, InferredType::Int { .. }) || integer_literal_parts(right).is_some() {
        return;
    }
    let placeholder = ExprKind::NullLit;
    if let ExprKind::Binary { left, right, .. } = std::mem::replace(&mut node.kind, placeholder) {
        node.kind = ExprKind::Checked {
            op: shift,
            left,
            right: Some(right),
            discharge: Discharge::AtBuild,
        };
    }
}

/// Settle every integer shift of `node`, read from `source`, by what the build
/// knows of its count (SCE_FORGE.md §3.4.1, "The width of a shift"). A shift is
/// its exact result reduced to the width `W` of what it shifts for every count
/// at least 0, and has no value for a count below 0, which fails `out-of-range`:
///
/// - a count known to be at least 0 and below `W` is left the operator it is,
///   which every language computes there; but for a `>>>` of a signed value,
///   which no C-family, Rust, Go or Python operator shifts logically;
/// - a count known to be at least 0, and every count outside the bodies the
///   contract covers, is the total shift ([`Discharge::Total`]): the value
///   for a count at or past `W`, and nothing checked, as a divisor of 0 is
///   not checked there (SCE_FORGE.md §6.5, "Error Model");
/// - any other count, in a body that receives failures (`may-fail`, or a
///   `sce-static` statechart), is checked when it runs ([`Discharge::AtRun`]).
///
/// What is known of a count is its spelling (a literal), its type (an unsigned
/// count is at least 0), and, in the body of an algorithm that does not declare
/// `may-fail`, what the interval analysis proved of it
/// ([`TypeCtx::shift_counts`]); that analysis refused the algorithm unless
/// every count is at least 0 ([`TypeCtx::integers_proven`]). A `may-fail`
/// body checks every count its type does not prove, as it checks every
/// operation of the contract and not only those the analysis flags.
///
/// [`TypeCtx::shift_counts`]: crate::forge::types::TypeCtx::shift_counts
/// [`TypeCtx::integers_proven`]: crate::forge::types::TypeCtx::integers_proven
pub(crate) fn settle_shifts(node: &mut TypedExpr, ctx: &TypeCtx<'_>, source: &str) {
    for child in node.children_mut() {
        settle_shifts(child, ctx, source);
    }
    let ExprKind::Binary { op, right, .. } = &node.kind else {
        return;
    };
    let Some(shift) = CheckedOp::of_shift(*op) else {
        return;
    };
    let InferredType::Int { signed, bits } = node.ty else {
        return;
    };
    let proved = node.span.as_ref().and_then(|span| {
        ctx.shift_counts
            .get(&(source.to_string(), span.start, span.end))
            .copied()
    });
    let mut known =
        shift_count_written(right, bits).max(proved.unwrap_or(ShiftCount::MayBeNegative));
    if ctx.integers_proven {
        known = known.max(ShiftCount::NonNegative);
    }
    let logical_of_signed = shift == CheckedOp::UShr && signed;
    if known == ShiftCount::BelowWidth && !logical_of_signed {
        return;
    }
    let discharge = if known == ShiftCount::MayBeNegative && ctx.receives_failures {
        Discharge::AtRun
    } else {
        Discharge::Total
    };
    let placeholder = ExprKind::NullLit;
    if let ExprKind::Binary { left, right, .. } = std::mem::replace(&mut node.kind, placeholder) {
        node.kind = ExprKind::Checked {
            op: shift,
            left,
            right: Some(right),
            discharge,
        };
    }
}

/// What a shift's count `count` is known to be against the width `bits` from
/// how it is written alone: a literal by its value, a value of an unsigned
/// type as at least 0, and anything else as a count that may be below 0.
fn shift_count_written(count: &TypedExpr, bits: u8) -> ShiftCount {
    if let Some((text, negated)) = integer_literal_parts(count) {
        return match integer_literal_value(&text) {
            Some(0) => ShiftCount::BelowWidth,
            Some(_) if negated => ShiftCount::MayBeNegative,
            Some(value) if value < u64::from(bits) => ShiftCount::BelowWidth,
            Some(_) => ShiftCount::NonNegative,
            None => ShiftCount::MayBeNegative,
        };
    }
    match count.ty.strip_quantity() {
        InferredType::Int { signed: false, .. } => ShiftCount::NonNegative,
        _ => ShiftCount::MayBeNegative,
    }
}

/// The sign and width of a shift the build settled ([`settle_shifts`]): its
/// node's type, which is the type of what it shifts.
fn settled_shift_width(expr: &TypedExpr) -> Result<(bool, u8), ExprError> {
    match expr.ty {
        InferredType::Int { signed, bits } => Ok((signed, bits)),
        other => Err(ExprError::UnsupportedConstruct {
            construct: format!("a shift of type {other:?}"),
            observed: None,
        }),
    }
}

/// `call` as the operation it names when its callee is a name of the
/// vocabulary's total arithmetic (docs/adr/0063, section 2), and `call`
/// unchanged otherwise. The operation takes its own number of operands, so a
/// call with another number is refused here, at the callee, where the call
/// is read.
fn total_operation(call: TypedExpr) -> Result<TypedExpr, Refusal> {
    let named = match &call.kind {
        ExprKind::Call { callee, args, .. } => match &callee.kind {
            ExprKind::Ident(name) => {
                ring_ops::lookup(name).map(|(op, rule)| (op, rule, args.len(), callee.span.clone()))
            }
            _ => None,
        },
        _ => None,
    };
    let Some((op, rule, count, at)) = named else {
        return Ok(call);
    };
    if count != op.arity() {
        return Err(ExprError::ArgumentCount {
            callee: ring_ops::name(op, rule).to_string(),
            expected: op.arity(),
            actual: count,
        }
        .at(at));
    }
    let TypedExpr {
        kind: ExprKind::Call { args, .. },
        ty,
        span,
    } = call
    else {
        unreachable!("matched as a call above");
    };
    let mut operands = args.into_iter().map(Box::new);
    let left = operands
        .next()
        .expect("the operation's arity is at least one");
    Ok(TypedExpr {
        kind: ExprKind::Total {
            op,
            rule,
            left,
            right: operands.next(),
        },
        ty,
        span,
    })
}

/// The refusal of a total operation by an emitter that does not lower it yet:
/// named as the author wrote it, so a document that uses it is refused by
/// that backend rather than generated wrong.
fn total_unlowered(op: RingOp, rule: OverflowRule, backend: &str) -> ExprError {
    ExprError::UnsupportedConstruct {
        construct: format!(
            "{} on the {backend} backend (docs/adr/0063)",
            ring_ops::name(op, rule)
        ),
        observed: None,
    }
}

/// The runtime helper of `op` under `rule` in the spelling of a runtime whose
/// members are camelCase: `wrappingSub`, `saturatingAdd` (the Interpreter's
/// library). The other runtimes join the two names in their own case.
fn camel_helper(op: RingOp, rule: OverflowRule) -> String {
    format!(
        "{}{}",
        rule.helper(),
        crate::filters::capitalize_first(op.helper())
    )
}

/// A total operation's width and its operands, each spelled by `emit` at the
/// operation's type (docs/adr/0063): the parts every backend's call of the
/// runtime is written from. `backend` names the emitter in the refusal of a
/// node typing gave no integer type, which the judge refuses before any
/// emitter is reached.
fn total_parts<T, E>(
    expr: &TypedExpr,
    backend: &str,
    mut emit: impl FnMut(&TypedExpr, InferredType) -> Result<T, E>,
) -> Result<(bool, u8, Vec<T>), E>
where
    E: From<ExprError>,
{
    let ExprKind::Total {
        op,
        rule,
        left,
        right,
    } = &expr.kind
    else {
        unreachable!("total_parts is called on a total operation");
    };
    let InferredType::Int { signed, bits } = expr.ty else {
        return Err(total_unlowered(*op, *rule, backend).into());
    };
    let mut operands = vec![emit(left, expr.ty)?];
    if let Some(right) = right {
        operands.push(emit(right, expr.ty)?);
    }
    Ok((signed, bits, operands))
}

/// The type of a total operation over operands of types `left` and `right`
/// (docs/adr/0063, section 3): the type its declared operands share, a
/// literal taking it. `None` where the operation has none, which
/// [`reject_ill_typed_total_operations`] says why.
fn total_operand_type(left: InferredType, right: Option<InferredType>) -> Option<InferredType> {
    use InferredType::{Int, UntypedInt};
    match (left, right) {
        (t @ Int { .. }, None) => Some(t),
        (t @ Int { .. }, Some(UntypedInt)) | (UntypedInt, Some(t @ Int { .. })) => Some(t),
        (l @ Int { .. }, Some(r)) if l == r => Some(l),
        _ => None,
    }
}

/// Refuse every total operation of `ast` that has no type, or whose type
/// does not admit it (docs/adr/0063, section 3): an operand that is not an
/// integer, two declared operands of different types, operands that are
/// literals alone, and the saturating negation of an unsigned value. Each
/// refusal names the operation as the author spelled it and stands at the
/// operand it is about.
pub(crate) fn reject_ill_typed_total_operations(
    ast: &TypedExpr,
    source: &str,
) -> Result<(), Refusal> {
    for child in ast.children() {
        reject_ill_typed_total_operations(child, source)?;
    }
    let ExprKind::Total {
        op,
        rule,
        left,
        right,
    } = &ast.kind
    else {
        return Ok(());
    };
    let name = ring_ops::name(*op, *rule);
    let refused = |expected: String, operand: &TypedExpr| {
        ExprError::TypeMismatch {
            expected,
            got: operand.ty.describe(),
            observed: operand
                .span
                .clone()
                .and_then(|span| source.get(span))
                .map(str::to_string),
        }
        .at(operand.span.clone())
    };
    let integer =
        |t: InferredType| matches!(t, InferredType::Int { .. } | InferredType::UntypedInt);
    for operand in std::iter::once(&**left).chain(right.as_deref()) {
        if !integer(operand.ty) {
            return Err(refused(format!("an integer operand of {name}"), operand));
        }
    }
    let Some(ty) = total_operand_type(left.ty, right.as_ref().map(|r| r.ty)) else {
        return Err(match right {
            Some(right) if left.ty != InferredType::UntypedInt => refused(
                format!("{} like the other operand of {name}", left.ty.describe()),
                right,
            ),
            _ => refused(
                format!(
                    "an operand of {name} of a declared integer type, whose width is the modulus"
                ),
                left,
            ),
        });
    };
    let (lo, _) = ty.int_bounds().expect("the operation's type is an integer");
    if !ring_ops::admits(*op, *rule, lo) {
        return Err(refused(
            format!("a signed integer ({name} of an unsigned value is always 0)"),
            left,
        ));
    }
    Ok(())
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
    /// `x ?? d` — the value of `x`, an optional value, when it is present, and
    /// `d` when it is absent (docs/adr/0055). `d` is evaluated only then, and
    /// `x` once.
    Coalesce,
}

impl BinOp {
    /// The operator as the expression language spells it — what a refusal
    /// quotes, not any backend's spelling of it.
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Coalesce => "??",
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::StrictEq => "===",
            Self::StrictNeq => "!==",
            Self::Lt => "<",
            Self::Gt => ">",
            Self::LtEq => "<=",
            Self::GtEq => ">=",
            Self::And => "&&",
            Self::Or => "||",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Shl => "<<",
            Self::Shr => ">>",
            Self::UShr => ">>>",
        }
    }

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
    /// `<<`, `>>` and `>>>`: typed by the left operand alone ([`join_shift`]),
    /// the count an integer of its own.
    pub(crate) fn is_shift(self) -> bool {
        matches!(self, Self::Shl | Self::Shr | Self::UShr)
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
        if let ExprKind::Ident(name) = &node.kind {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        for child in node.children() {
            walk(child, names);
        }
    }
    let mut names = Vec::new();
    // Trimmed, as every entry point hands the parser its text, so a refusal
    // raised here indexes the same string as one raised by a lowering.
    walk(&parse_to_ast(raw_expr.trim())?, &mut names);
    Ok(names)
}

/// [`read_identifiers`] less the names only measured or indexed: an
/// identifier that is the sole argument of the `len(…)` builtin is asked for
/// its length, and one that is the object of an index is asked for one
/// element (the index itself is still walked) — neither reads it as a value.
/// What a `sce-static` list variable may appear as (SCE Accepted Subset
/// §2.15) — anywhere else, it is a value it is not. The first argument of
/// `has(…)` is asked whether it holds a key (the key itself is still walked),
/// which a map or a set may be asked and is no read of it as a value either
/// (docs/adr/0108, D3).
pub fn identifiers_read_as_values(raw_expr: &str) -> Result<Vec<String>, Refusal> {
    fn walk(node: &TypedExpr, names: &mut Vec<String>) {
        if let ExprKind::Call { callee, args, .. } = &node.kind {
            let is_len =
                matches!(&callee.kind, ExprKind::Ident(n) | ExprKind::Raw(n) if n == "len");
            if is_len && args.len() == 1 && matches!(args[0].kind, ExprKind::Ident(_)) {
                return;
            }
            let is_has = matches!(&callee.kind, ExprKind::Ident(n) if n == HAS_BUILTIN);
            if is_has
                && args
                    .first()
                    .is_some_and(|map| matches!(map.kind, ExprKind::Ident(_)))
            {
                for key in &args[1..] {
                    walk(key, names);
                }
                return;
            }
        }
        if let ExprKind::Index { object, index } = &node.kind {
            if matches!(object.kind, ExprKind::Ident(_)) {
                walk(index, names);
                return;
            }
        }
        if let ExprKind::Ident(name) = &node.kind {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        for child in node.children() {
            walk(child, names);
        }
    }
    let mut names = Vec::new();
    walk(&parse_to_ast(raw_expr.trim())?, &mut names);
    Ok(names)
}

/// The callees `raw_expr` calls by a bare name: the callee of the call that is
/// the whole expression, if it is one, and the callee of every other call in it,
/// each in the order met. What a `sce-static` statechart asks to tell a list an
/// algorithm returns taken whole into a list variable from one used as a value
/// (SCE Accepted Subset §2.15).
pub fn called_names(raw_expr: &str) -> Result<(Option<String>, Vec<String>), Refusal> {
    fn walk(node: &TypedExpr, is_root: bool, others: &mut Vec<String>) {
        if let ExprKind::Call { callee, .. } = &node.kind {
            if !is_root {
                if let ExprKind::Ident(name) = &callee.kind {
                    others.push(name.clone());
                }
            }
        }
        for child in node.children() {
            walk(child, false, others);
        }
    }
    let ast = parse_to_ast(raw_expr.trim())?;
    let whole = match &ast.kind {
        ExprKind::Call { callee, .. } => match &callee.kind {
            ExprKind::Ident(name) => Some(name.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut others = Vec::new();
    walk(&ast, true, &mut others);
    Ok((whole, others))
}

/// What one argument of a call is, as far as a `sce-static` statechart's call of an
/// algorithm that takes a record or a list needs to tell: such an argument is the
/// NAME of a variable of the machine (or of a loop's item), or the payload of the
/// event the transition is on, and never an expression that computes one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgShape {
    /// A bare name: a variable, or a loop's item.
    Name(String),
    /// `_event.data`, taken whole.
    EventData,
    /// The literal `null`: the absent value of the optional record a call's
    /// parameter is (docs/adr/0055, step 9).
    Null,
    /// Anything else.
    Other,
}

/// One argument of a call found by [`call_sites`]: the bytes of the expression it is
/// written in, and what it is.
#[derive(Debug, Clone)]
pub struct ArgSite {
    pub span: std::ops::Range<usize>,
    pub shape: ArgShape,
}

/// One call by a bare name found by [`call_sites`].
#[derive(Debug, Clone)]
pub struct CallSite {
    /// The callee's name, and the bytes it is written in.
    pub callee: String,
    pub callee_span: std::ops::Range<usize>,
    pub args: Vec<ArgSite>,
}

/// Every call in `raw_expr` of a callee `wanted` accepts, outermost first, with the
/// bytes of the callee and of each argument — what a statechart that calls an
/// algorithm with state in a slot rewrites the call from (SCE Accepted Subset §2.15).
/// The spans index `raw_expr` itself, whatever space leads it.
pub fn call_sites(raw_expr: &str, wanted: &dyn Fn(&str) -> bool) -> Result<Vec<CallSite>, Refusal> {
    fn walk(
        node: &TypedExpr,
        wanted: &dyn Fn(&str) -> bool,
        offset: usize,
        found: &mut Vec<CallSite>,
    ) {
        if let ExprKind::Call { callee, args, .. } = &node.kind {
            if let ExprKind::Ident(name) = &callee.kind {
                if wanted(name) {
                    let shift =
                        |span: &std::ops::Range<usize>| span.start + offset..span.end + offset;
                    let shape = |arg: &TypedExpr| match &arg.kind {
                        ExprKind::Ident(name) => ArgShape::Name(name.clone()),
                        ExprKind::NullLit => ArgShape::Null,
                        ExprKind::Member { object, property }
                            if property == "data"
                                && matches!(&object.kind, ExprKind::Ident(o) if o == "_event") =>
                        {
                            ArgShape::EventData
                        }
                        _ => ArgShape::Other,
                    };
                    if let (Some(callee_span), true) =
                        (callee.span.as_ref(), args.iter().all(|a| a.span.is_some()))
                    {
                        found.push(CallSite {
                            callee: name.clone(),
                            callee_span: shift(callee_span),
                            args: args
                                .iter()
                                .map(|a| ArgSite {
                                    span: shift(a.span.as_ref().expect("checked above")),
                                    shape: shape(a),
                                })
                                .collect(),
                        });
                    }
                }
            }
        }
        for child in node.children() {
            walk(child, wanted, offset, found);
        }
    }
    let offset = raw_expr.len() - raw_expr.trim_start().len();
    let ast = parse_to_ast(raw_expr.trim())?;
    let mut found = Vec::new();
    walk(&ast, wanted, offset, &mut found);
    Ok(found)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tokens
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Number(String),
    /// A string literal's value, its escapes already read
    /// ([`decode_string_literal`]).
    String {
        value: String,
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
    /// `??`, the read of an optional value with a default (docs/adr/0055).
    /// Produced only under [`LexMode::Forge`]: the ECMAScript datamodel keeps
    /// reading it as a construct outside its subset.
    QuestionQuestion,
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
            Token::String { value } => {
                write!(f, "{}", crate::filters::ecmascript_string_literal(value))
            }
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
            Token::QuestionQuestion => write!(f, "??"),
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

/// Why [`decode_string_literal`] refuses an escape: the reason an
/// [`ExprError::UnsupportedStringEscape`] carries.
const ESCAPE_HEX_DIGITS: &str = "`\\x` is followed by exactly two hexadecimal digits";
const ESCAPE_UNIT_DIGITS: &str =
    "`\\u` is followed by exactly four hexadecimal digits or by a code point in braces";
const ESCAPE_CODE_POINT: &str =
    "a code point in braces is one or more hexadecimal digits naming at most 10FFFF";
const ESCAPE_LONE_SURROGATE: &str = "a surrogate that is not one half of a pair names no \
     character, and no backend's string holds one";
const ESCAPE_LEGACY_OCTAL: &str = "a digit after a backslash is a legacy octal escape, which a \
     forge expression does not read; write `\\0` for U+0000, or `\\xHH` or `\\u{H}` for any \
     other character";
const ESCAPE_IDENTITY: &str = "a backslash before a character that has no escape meaning is \
     not read in a forge expression; write the character itself, or `\\\\` for a backslash";
const ESCAPE_TRAILING: &str = "a backslash ends the literal and escapes nothing";

/// The value a string literal denotes: `raw`, the source spelling between its
/// quotes, with every escape read once, here, by the rules of the dialect it
/// is written in. `at` is where `raw` starts in the expression, so a refusal
/// is placed on the escape it names.
///
/// Both dialects read ECMA-262's escapes (12.9.4): `\\`, `\'`, `\"`, `\b`,
/// `\f`, `\n`, `\r`, `\t`, `\v`; `\0` before anything but a decimal digit;
/// `\xHH`; `\uHHHH`; `\u{H…}` naming at most 10FFFF; and a backslash before a
/// line break (LF, CR, CR LF, U+2028, U+2029), which continues the literal
/// and denotes nothing. Two escapes that are the halves of a surrogate pair
/// denote the one character the pair makes; a half alone is refused, since a
/// Rust, Go, C or Swift string holds characters, not UTF-16 code units. A
/// malformed `\x`, `\u` or `\u{…}` is refused too, as ECMA-262 refuses it.
///
/// The dialects part at the escapes ECMAScript keeps for compatibility. The
/// W3C ECMAScript datamodel reads them as ECMAScript does: a backslash before
/// any other character is that character, and a digit after one is Annex B's
/// legacy octal (`\8` and `\9` the digits themselves). A forge expression
/// refuses both: `'c\d'` denotes `cd` in ECMAScript, which is almost never
/// what an author who wrote it meant, and a legacy octal is a second spelling
/// of what `\x` already spells.
///
/// A line break written raw between the quotes is kept as the character it
/// is. ECMA-262 ends the literal there; an XML attribute cannot carry one
/// unless the author wrote a character reference for it, and the documents
/// that do are read as holding it.
fn decode_string_literal(raw: &str, at: usize, mode: LexMode) -> Result<String, Refusal> {
    /// What one escape contributes to the value.
    enum Escaped {
        /// A character, named outright.
        Char(char),
        /// A code point, or one UTF-16 code unit that may be half of a pair.
        Unit(u32),
        /// Nothing: a line continuation.
        Nothing,
    }
    /// `text`, when it is there and is hexadecimal digits only.
    fn hex_run(text: Option<&str>) -> Option<&str> {
        text.filter(|digits| digits.bytes().all(|b| b.is_ascii_hexdigit()))
    }
    let refuse = |from: usize, to: usize, reason: &'static str| -> Refusal {
        ExprError::UnsupportedStringEscape {
            position: at + from,
            escape: raw[from..to].to_string(),
            reason,
        }
        .at(Some(at + from..at + to))
    };
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    // The high half of a surrogate pair an escape named, waiting for the low
    // half: its code unit and where its escape was written.
    let mut high: Option<(u32, usize, usize)> = None;
    let mut i = 0;
    while i < raw.len() {
        if bytes[i] != b'\\' {
            let ch = raw[i..]
                .chars()
                .next()
                .expect("a byte other than a backslash starts a character here");
            if let Some((_, from, to)) = high.take() {
                return Err(refuse(from, to, ESCAPE_LONE_SURROGATE));
            }
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }
        let from = i;
        let Some(escape) = raw[i + 1..].chars().next() else {
            return Err(refuse(from, raw.len(), ESCAPE_TRAILING));
        };
        i += 1 + escape.len_utf8();
        let escaped = match escape {
            '\n' | '\u{2028}' | '\u{2029}' => Escaped::Nothing,
            '\r' => {
                if bytes.get(i) == Some(&b'\n') {
                    i += 1;
                }
                Escaped::Nothing
            }
            '\\' | '\'' | '"' => Escaped::Char(escape),
            'b' => Escaped::Char('\u{8}'),
            'f' => Escaped::Char('\u{c}'),
            'n' => Escaped::Char('\n'),
            'r' => Escaped::Char('\r'),
            't' => Escaped::Char('\t'),
            'v' => Escaped::Char('\u{b}'),
            '0' if !bytes.get(i).is_some_and(|next| next.is_ascii_digit()) => Escaped::Char('\0'),
            'x' => {
                let Some(digits) = hex_run(raw.get(i..i + 2)) else {
                    return Err(refuse(from, i, ESCAPE_HEX_DIGITS));
                };
                i += 2;
                Escaped::Unit(u32::from_str_radix(digits, 16).expect("two hexadecimal digits"))
            }
            'u' if bytes.get(i) == Some(&b'{') => {
                let Some(length) = raw[i + 1..].find('}') else {
                    return Err(refuse(from, i + 1, ESCAPE_CODE_POINT));
                };
                let end = i + 1 + length + 1;
                let value = hex_run(raw.get(i + 1..i + 1 + length))
                    .filter(|digits| !digits.is_empty())
                    .and_then(|digits| u32::from_str_radix(digits, 16).ok())
                    .filter(|value| *value <= 0x10_FFFF);
                let Some(value) = value else {
                    return Err(refuse(from, end, ESCAPE_CODE_POINT));
                };
                i = end;
                Escaped::Unit(value)
            }
            'u' => {
                let Some(digits) = hex_run(raw.get(i..i + 4)) else {
                    return Err(refuse(from, i, ESCAPE_UNIT_DIGITS));
                };
                i += 4;
                Escaped::Unit(u32::from_str_radix(digits, 16).expect("four hexadecimal digits"))
            }
            '0'..='9' if mode == LexMode::Forge => {
                return Err(refuse(from, i, ESCAPE_LEGACY_OCTAL));
            }
            // ECMA-262 Annex B: a NonOctalDecimalEscapeSequence is the digit,
            // and a LegacyOctalEscapeSequence takes up to three octal digits,
            // the first of them at most 3 when there are three.
            '8' | '9' => Escaped::Char(escape),
            '0'..='7' => {
                let mut value = escape.to_digit(8).expect("an octal digit");
                let more = if value <= 3 { 2 } else { 1 };
                for _ in 0..more {
                    match bytes.get(i).copied() {
                        Some(digit @ b'0'..=b'7') => {
                            value = value * 8 + u32::from(digit - b'0');
                            i += 1;
                        }
                        _ => break,
                    }
                }
                Escaped::Unit(value)
            }
            _ if mode == LexMode::Forge => return Err(refuse(from, i, ESCAPE_IDENTITY)),
            other => Escaped::Char(other),
        };
        match escaped {
            Escaped::Nothing => {}
            Escaped::Char(ch) => {
                if let Some((_, from, to)) = high.take() {
                    return Err(refuse(from, to, ESCAPE_LONE_SURROGATE));
                }
                out.push(ch);
            }
            Escaped::Unit(unit) => match (high.take(), unit) {
                (Some((first, ..)), 0xDC00..=0xDFFF) => {
                    let pair = 0x1_0000 + ((first - 0xD800) << 10) + (unit - 0xDC00);
                    out.push(char::from_u32(pair).expect("a surrogate pair names a character"));
                }
                (Some((_, first_from, first_to)), _) => {
                    return Err(refuse(first_from, first_to, ESCAPE_LONE_SURROGATE));
                }
                (None, 0xD800..=0xDBFF) => high = Some((unit, from, i)),
                (None, 0xDC00..=0xDFFF) => return Err(refuse(from, i, ESCAPE_LONE_SURROGATE)),
                (None, _) => out.push(
                    char::from_u32(unit)
                        .expect("a code point that is no surrogate, at most 10FFFF"),
                ),
            },
        }
    }
    if let Some((_, from, to)) = high {
        return Err(refuse(from, to, ESCAPE_LONE_SURROGATE));
    }
    Ok(out)
}

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
pub(crate) type SpannedTokens = (Vec<Token>, Vec<std::ops::Range<usize>>);

/// [`tokenize_as`], keeping where each token was read from — and where a
/// refusal was raised, in the same byte ranges. Both dialects' parsers read
/// it, so both place a refusal against the same ranges.
pub(crate) fn tokenize_spanned(input: &str, mode: LexMode) -> Result<SpannedTokens, Refusal> {
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

        // String literals. The token carries the value the literal denotes,
        // its escapes read once, here ([`decode_string_literal`]): every
        // reader after this one sees characters, and every emitter writes them
        // through its own language's escaper.
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            i += 1;
            let start = i;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    // A backslash before CR LF continues the literal past
                    // both, as one line break.
                    if bytes.get(i + 1) == Some(&b'\r') && bytes.get(i + 2) == Some(&b'\n') {
                        i += 1;
                    }
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
            let value = decode_string_literal(&input[start..i], start, mode)?;
            i += 1;
            tokens.push(Token::String { value });
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
        // `??` is the forge expression language's read of an optional value
        // with a default (docs/adr/0055); the ECMAScript datamodel's subset
        // still leaves it out.
        if bytes[i..].starts_with(b"??") {
            tokens.push(if ecma {
                Token::Unsupported {
                    construct: "nullish coalescing (??)",
                    spelling: "??",
                }
            } else {
                Token::QuestionQuestion
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
        let expr = self.parse_short_circuit()?;
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

    /// ECMAScript's ShortCircuitExpression: a chain of `??` (docs/adr/0055)
    /// over operands that are no bare `&&` or `||`, or a logical-OR
    /// expression. The two are never mixed without parentheses, which
    /// ECMAScript refuses too: which of them is read first is then the
    /// author's to write.
    fn parse_short_circuit(&mut self) -> Result<TypedExpr, Refusal> {
        let start = self.start();
        let first = self.parse_bitwise_or()?;
        if *self.peek() != Token::QuestionQuestion {
            let logical = self.logical_or_from(start, first)?;
            if *self.peek() == Token::QuestionQuestion {
                return Err(self.mixed_coalescing());
            }
            return Ok(logical);
        }
        let mut left = first;
        while *self.peek() == Token::QuestionQuestion {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = self.node(
                start,
                ExprKind::Binary {
                    op: BinOp::Coalesce,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            );
        }
        if matches!(self.peek(), Token::AmpAmp | Token::PipePipe) {
            return Err(self.mixed_coalescing());
        }
        Ok(left)
    }

    /// The refusal of `??` written beside a bare `&&` or `||`, placed at the
    /// operator that mixes them.
    fn mixed_coalescing(&self) -> Refusal {
        ExprError::UnsupportedConstruct {
            construct: "`??` beside `&&` or `||` with no parentheses to say which is read \
                        first (ECMAScript refuses the mix; parenthesise the one meant first)"
                .to_string(),
            observed: Some(self.peek().to_string()),
        }
        .at(self.span_of(self.pos))
    }

    /// The `||` chain whose first `&&` operand, `first`, is already read from
    /// `start`.
    fn logical_or_from(&mut self, start: usize, first: TypedExpr) -> Result<TypedExpr, Refusal> {
        let mut left = self.logical_and_from(start, first)?;
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
        let first = self.parse_bitwise_or()?;
        self.logical_and_from(start, first)
    }

    /// The `&&` chain whose first operand, `first`, is already read from
    /// `start`.
    fn logical_and_from(&mut self, start: usize, first: TypedExpr) -> Result<TypedExpr, Refusal> {
        let mut left = first;
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
                    expr = total_operation(self.node(
                        start,
                        ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                            params: Vec::new(),
                            fails: false,
                            into_destination: false,
                            storage: None,
                        },
                    ))?;
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
            Token::String { value } => Ok(self.node(start, ExprKind::StringLit { value })),
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

/// The names of a chain of member reads over an identifier, joined by dots —
/// `_event.data` or `_event.data.layout` — or `None` when the node is anything
/// else (a call, an index, a literal).
pub(crate) fn dotted_path(node: &TypedExpr) -> Option<String> {
    match &node.kind {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Member { object, property } => {
            Some(format!("{}.{property}", dotted_path(object)?))
        }
        _ => None,
    }
}

/// Apply an identifier rename map to the AST.
///
/// Handles two cases:
/// 1. Bare `Ident` nodes: renamed via the `renames` map
///    (e.g., `retryCount` → `retryCount_`).
/// 2. `Member` chains over an identifier (`x.prop`, `x.a.b`): if the full
///    dotted path is in `renames`, the entire Member node collapses to a
///    single `Raw` (e.g., `_event.data` → `pendingEventData_`).
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
            // The whole chain of names the member reads, `a.b` or `a.b.c`: a
            // longer path wins over the shorter one it begins with, so the
            // outer member is asked before the object it reads through.
            if let Some(object_path) = dotted_path(object) {
                let full_path = format!("{object_path}.{property}");
                if let Some(renamed) = renames.get(full_path.as_str()) {
                    // The type stays: inference ran before rename
                    // (`resolve_then_rename`), and the emitter still needs it
                    // to spell what the member meets. It was reset to
                    // `Unknown` when rename ran first and nothing could be
                    // looked up under the new spelling.
                    ast.kind = ExprKind::Raw(renamed.to_string());
                    return;
                }
            }
            // A field of the element a list of records is indexed at is spelled
            // as the backend spells a record's field, `<list>[].<field>`.
            if let Some(list) = indexed_list_name(object) {
                let key = format!(
                    "{list}{}{property}",
                    crate::forge::type_ctx::INDEXED_ELEMENT
                );
                if let Some(renamed) = renames.get(key.as_str()) {
                    *property = renamed.to_string();
                }
            }
            rename_identifiers(object, renames);
        }
        ExprKind::Index { object, index } => {
            rename_identifiers(object, renames);
            rename_identifiers(index, renames);
        }
        ExprKind::Call { callee, args, .. } => {
            // A call of a builtin is the vocabulary's whatever the document names
            // ([`vocabulary_names`]), so its callee is no name of the author's and no
            // rename of one reaches it: an author's local `len`, which a backend may
            // spell otherwise (C11's destination `(*len)`, docs/adr/0082), leaves
            // `len(len)` the length of that local.
            let builtin = matches!(&callee.kind, ExprKind::Ident(name) if EXPR_BUILTINS.contains(&name.as_str()));
            if !builtin {
                rename_identifiers(callee, renames);
            }
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
        ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
            rename_identifiers(left, renames);
            if let Some(right) = right {
                rename_identifiers(right, renames);
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
            // The `.value` of an optional value (docs/adr/0055) is no field of
            // a struct but the proven read the emitter spells; it keeps its name.
            if !(property == OPTIONAL_VALUE && object.ty.is_optional()) {
                *property = crate::filters::to_pascal_case(property.clone());
            }
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
        ExprKind::Checked { left, right, .. } | ExprKind::Total { left, right, .. } => {
            export_go_member_properties(left);
            if let Some(right) = right {
                export_go_member_properties(right);
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
/// The name of the list `expr` reads one element of by an index — `days` in
/// `days[i]` — when `expr` is exactly that read and the list holds records.
fn indexed_list_name(expr: &TypedExpr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Index { object, .. }
        | ExprKind::Checked {
            op: CheckedOp::Index,
            left: object,
            ..
        } if matches!(object.ty, InferredType::List(elem) if elem.is_record()) => {
            match &object.kind {
                ExprKind::Ident(name) => Some(name.as_str()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The place `node` reads, as [`crate::forge::value_path`] writes one: a name,
/// each member read after it as `.<member>`, and each index read as
/// [`crate::forge::value_path::ELEMENT`] — `_event.data.items[].tags` for
/// `_event.data.items[i].tags`, whatever `i` is. `None` for any other node (a
/// call, a literal, an operation): it reads no place.
pub(crate) fn place_path(node: &TypedExpr) -> Option<String> {
    match &node.kind {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::Member { object, property } => {
            Some(format!("{}.{property}", place_path(object)?))
        }
        ExprKind::Index { object, .. } => Some(format!(
            "{}{}",
            place_path(object)?,
            crate::forge::value_path::ELEMENT
        )),
        _ => None,
    }
}

/// The place the expression `text` reads ([`place_path`]), when it reads one:
/// `_event.data.items[].tags` for `_event.data.items[0].tags`, every index read
/// as an element whatever it is. `None` for a text that does not parse or reads
/// no place. What a statement that names a place in an attribute (a
/// `<foreach>`'s `array`, the value an `<assign>` takes whole) is typed by.
pub(crate) fn written_place(text: &str) -> Option<String> {
    place_path(&parse_to_ast(text.trim()).ok()?)
}

/// One step of a place written in a statement's attribute, after the name it
/// starts at ([`written_steps`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WrittenStep {
    /// `[index]`: the text of the index, as written.
    Index(String),
    /// `.field`: the name of the field.
    Field(String),
}

/// The name the place `text` starts at and each step after it, in order:
/// `("m", [Index("k"), Field("tags")])` for `m[k].tags`. What a statement
/// that writes through a nested place reads its `target` by (docs/adr/0108,
/// D12 rule 1). `None` for a text that does not parse, and for one that reads a
/// part of no place (a call, a literal, an operation).
pub(crate) fn written_steps(text: &str) -> Option<(String, Vec<WrittenStep>)> {
    fn walk(node: &TypedExpr, text: &str) -> Option<(String, Vec<WrittenStep>)> {
        match &node.kind {
            ExprKind::Ident(name) => Some((name.clone(), Vec::new())),
            ExprKind::Member { object, property } => {
                let (root, mut steps) = walk(object, text)?;
                steps.push(WrittenStep::Field(property.clone()));
                Some((root, steps))
            }
            ExprKind::Index { object, index } => {
                let (root, mut steps) = walk(object, text)?;
                steps.push(WrittenStep::Index(
                    text.get(index.span.clone()?)?.trim().to_string(),
                ));
                Some((root, steps))
            }
            _ => None,
        }
    }
    let text = text.trim();
    walk(&parse_to_ast(text).ok()?, text)
}

/// One read of an element on the way down to the element an expression reads
/// whole ([`written_elements`]): the text of its index, and the fields read
/// after it (`.shapes`, or nothing), which reach the next list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElementStep {
    pub index: String,
    pub fields: String,
}

/// The place the expression `text` starts at and each element it reads on the
/// way down, when the whole of `text` is the read of an element: `("uss",
/// [i, j])` for `uss[i][j]`, and `("_event.data.groups", [(g, ".shapes"), (s,
/// "")])` for `_event.data.groups[g].shapes[s]`, the place a path of fields with
/// no element, each step's index as the text writes it, and the last step's
/// fields empty. `None` for a text that does not parse, reads something after
/// its last element (`xs[i].kind`), or reads a part of no place (a call, a
/// literal). What an `<assign>` that takes an element whole reads the element by,
/// through each list on the way in turn (docs/adr/0099, section 5.3 and "Still
/// open (2026-11-16)" item 3).
pub(crate) fn written_elements(text: &str) -> Option<(String, Vec<ElementStep>)> {
    fn walk(node: &TypedExpr, text: &str) -> Option<(String, Vec<ElementStep>)> {
        match &node.kind {
            ExprKind::Ident(name) => Some((name.clone(), Vec::new())),
            ExprKind::Member { object, property } => {
                let (mut root, mut steps) = walk(object, text)?;
                match steps.last_mut() {
                    Some(step) => step.fields = format!("{}.{property}", step.fields),
                    None => root = format!("{root}.{property}"),
                }
                Some((root, steps))
            }
            ExprKind::Index { object, index } => {
                let (root, mut steps) = walk(object, text)?;
                steps.push(ElementStep {
                    index: text.get(index.span.clone()?)?.trim().to_string(),
                    fields: String::new(),
                });
                Some((root, steps))
            }
            _ => None,
        }
    }
    let text = text.trim();
    let ast = parse_to_ast(text).ok()?;
    if !matches!(ast.kind, ExprKind::Index { .. }) {
        return None;
    }
    walk(&ast, text)
}

/// The member of a C11 text buffer (`sce_static_string_<N>_t`, a text of at
/// most `N` UTF-8 bytes and its terminator) that holds its text, which is what
/// a C11 expression reads a text as: a `const char *`.
pub(crate) const C_BUFFER_TEXT: &str = "data";

/// Each text a composed value holds, read as a value, read through the buffer
/// that holds it, for C (docs/adr/0086, sections 6 and 7.1, the C11 row). A
/// statechart reads a composed payload as it reads a variable of the same type
/// (section 6), and C11 holds a text of a composed value, at any depth, in the
/// buffer of its bound, as it holds a text variable and a record's text field:
/// an expression reads each as the buffer's text ([`C_BUFFER_TEXT`]).
///
/// A member read names the buffer's member in its own name (`note.data`), as
/// the field of an element a list variable of records is indexed at is renamed
/// to (`rename_identifiers`), so a checked index read reads the text inside its
/// conditional and gives the empty text where it fails. Any other place, the
/// item of a loop and an element read by its index, is read through a member of
/// its own. Left as they are: a place the lowering spells for itself
/// (`renames`), which it spells through the buffer already; a present value of
/// an optional text, which [`c_present_read`] reads through its buffer; and a
/// byte string, which a composed value holds in the `{data, len}` struct every
/// C11 byte-string expression reads.
fn read_composed_texts_through_buffers(
    node: &mut TypedExpr,
    ctx: &TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
) {
    for child in node.children_mut() {
        read_composed_texts_through_buffers(child, ctx, renames);
    }
    if node.ty != InferredType::Str
        || matches!(optional_form(node), Some(OptionalForm::Proven { .. }))
    {
        return;
    }
    let held = place_path(node).is_some_and(|path| held_by_a_composed_value(&path, ctx, renames));
    if !held {
        return;
    }
    if let ExprKind::Member { property, .. } = &mut node.kind {
        property.push('.');
        property.push_str(C_BUFFER_TEXT);
        return;
    }
    let buffer = std::mem::replace(node, TypedExpr::new(ExprKind::NullLit));
    *node = TypedExpr {
        span: buffer.span.clone(),
        ty: InferredType::Str,
        kind: ExprKind::Member {
            object: Box::new(buffer),
            property: C_BUFFER_TEXT.to_string(),
        },
    };
}

/// Whether the place `path` names is held by a composed value: a place below
/// the payload with a composed value on the way to it, or the item of a loop
/// over a list a place holds ([`TypeCtx::place_items`]) or a place below it,
/// which no rename spells, or a place below a variable of a union, whose case's
/// record the walks declare, or below a variable of a record that holds a
/// composition, or of one that may be absent, which the walks declare as they
/// declare a union's (docs/adr/0087, "Landing order" item 8), or an element of
/// a list of texts, each held in the buffer of its bound (docs/adr/0086,
/// section 7.3). No other variable holds one: a record of entries' texts are
/// spelled by the renames.
fn held_by_a_composed_value(path: &str, ctx: &TypeCtx<'_>, renames: &HashMap<&str, &str>) -> bool {
    if renames.contains_key(path) {
        return false;
    }
    let payload = crate::forge::event_schema_check::EVENT_DATA_PATH;
    let element = crate::forge::value_path::ELEMENT;
    match path.strip_prefix(payload) {
        Some(rest) if rest.is_empty() || rest.starts_with('.') || rest.starts_with(element) => {
            // Every place from the payload to the one read, that one left out:
            // each step opens with a `.` or an element's `[]`.
            std::iter::once(0)
                .chain(
                    rest.char_indices()
                        .filter(|(at, c)| *c == '.' || rest[*at..].starts_with(element))
                        .map(|(at, _)| at),
                )
                .map(|at| &path[..payload.len() + at])
                .any(|holder| {
                    ctx.value_of(holder).is_some_and(|place| {
                        crate::forge::value_path::holds_composition(ctx.value_scope, &place)
                    })
                })
        }
        _ => {
            let root = path.split(['.', '[']).next().unwrap_or(path);
            // A variable of a union, or of one that may be absent, holds its
            // case's record as the walks over a composed value declare it
            // (docs/adr/0092, section 7), a text of it in the buffer of its bound.
            // A variable of a record that holds a composition, or of one that
            // may be absent, is the machine's own record the walks declare,
            // each text of it at any depth in the buffer of its bound.
            let holds_a_composed_value = || {
                ctx.value_of(root).is_some_and(|place| {
                    let held = match place.ty {
                        crate::forge::value_type::ValueType::Optional { elem } => *elem,
                        other => other,
                    };
                    match held {
                        crate::forge::value_type::ValueType::Union { .. } => true,
                        crate::forge::value_type::ValueType::List { elem, .. } => {
                            matches!(*elem, crate::forge::value_type::ValueType::Text { .. })
                        }
                        record @ crate::forge::value_type::ValueType::Record { .. } => {
                            crate::forge::value_path::holds_composition(
                                ctx.value_scope,
                                &crate::forge::value_path::Place {
                                    ty: record,
                                    scope: place.scope,
                                },
                            )
                        }
                        _ => false,
                    }
                })
            };
            ctx.place_items.contains(root) || (path != root && holds_a_composed_value())
        }
    }
}

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
/// The byte sequence a string literal's `value` stands for in a
/// `Bytes`-typed comparison (RFC §bytesguard-3 B2). `value` is the text the
/// literal denotes, its escapes already read by the lexer. Conservative:
/// only printable ASCII with no backslash (and no non-ASCII character) is
/// accepted — the byte sequence is then exactly `value.as_bytes()`,
/// byte-identical on every backend. Anything else returns `None`; the caller then leaves the
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
    string_literal_as_bytes(lit_node);
}

/// Rewrite `lit_node`, a `StringLit` of printable ASCII, in place into the
/// `BytesLit` of the same bytes, and say whether it was one. Nothing else is
/// touched: a literal [`decode_bytes_literal`] declines stays a string.
fn string_literal_as_bytes(lit_node: &mut TypedExpr) -> bool {
    let ExprKind::StringLit { value, .. } = &lit_node.kind else {
        return false;
    };
    let Some(bytes) = decode_bytes_literal(value) else {
        return false;
    };
    lit_node.kind = ExprKind::BytesLit { bytes };
    lit_node.ty = InferredType::Bytes;
    true
}

/// A value that lands in a place that owns a byte string is bytes, and a string
/// stands for them only as the literal of printable ASCII the one reading every
/// backend shares ([`decode_bytes_literal`]) gives bytes to: `slot_admits` lets a
/// string through where bytes are declared, which a payload field relies on, and
/// a `String` from a variable or a concatenation would reach a `Vec<u8>` as a
/// type no backend takes. The arms of a conditional land one by one, as they do
/// for a real ([`land_reals_in_slot`]).
fn land_strings_in_bytes_slot(expr: &mut TypedExpr, source: &str) -> Result<(), Refusal> {
    if let ExprKind::Conditional {
        consequent,
        alternate,
        ..
    } = &mut expr.kind
    {
        land_strings_in_bytes_slot(consequent, source)?;
        land_strings_in_bytes_slot(alternate, source)?;
        expr.ty = InferredType::Bytes;
        return Ok(());
    }
    if expr.ty != InferredType::Str || string_literal_as_bytes(expr) {
        return Ok(());
    }
    Err(string_where_bytes_are_held(expr, source))
}

/// The refusal of `expr`, a string that stands where a byte string does.
fn string_where_bytes_are_held(expr: &TypedExpr, source: &str) -> Refusal {
    ExprError::UnsupportedConstruct {
        construct: "a string where a byte string is held (only a literal of printable ASCII \
                    with no backslash stands for bytes)"
            .to_string(),
        observed: expr
            .span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string),
    }
    .at(expr.span.clone())
}

/// The text of a byte string, each byte the character of that code point
/// (docs/adr/0005, decision 2): the value a target that spells bytes as text
/// writes through its own string-literal escaper.
fn latin1(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
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

/// Whether `left op right` is a string concatenation.
///
/// ECMA-262 13.15.3 makes `+` a concatenation as soon as one operand is a
/// string, whatever the other turns out to be: `'E' + (7300 + n)` is a string
/// and never a sum. The numeric lattice ([`join_arith`]) has no place for a
/// string, so without this the sum was `Unknown` and every emitter wrote the
/// operator between its operands as written -- a pointer addition in C and
/// C++, a type error in Rust and Go, a `TypeError` in Python, and a
/// concatenation only in Kotlin, where `String.plus` takes anything.
///
/// Decided here, once, so the emitters read one answer (the result is `Str`)
/// instead of each re-deriving it from its operands. Constant folding and
/// integer range analysis already stand aside for a non-numeric operand or a
/// non-numeric result, so a `Str` sum is never read as a number.
pub(crate) fn is_string_concatenation(op: BinOp, left: InferredType, right: InferredType) -> bool {
    matches!(op, BinOp::Add)
        && (matches!(left, InferredType::Str) || matches!(right, InferredType::Str))
}

/// How one operand of a string concatenation reaches the string.
///
/// ECMA-262 7.1.17 turns any value into its string form, and for most of them
/// that form is a property of the language: `1.5`, `1e21`, `true`, `null` and a
/// byte array each print one way in ECMAScript and another, or no way, in a
/// target language. A document must mean the same in every backend, so the
/// ones whose text this generator cannot make the same everywhere are refused
/// by name, never left to whatever the target language does with them. That
/// includes a type the generator could not infer (`Unknown`): passing it on
/// is how the bug this classification exists for was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConcatOperand {
    /// Already a string.
    Text,
    /// An integer: its string form is its decimal digits, with a leading `-`
    /// for a negative, in every language.
    Decimal,
    /// A value whose string form differs between languages, or is unknown.
    Refused(&'static str),
}

fn concat_operand(ty: InferredType) -> ConcatOperand {
    match ty {
        InferredType::Str => ConcatOperand::Text,
        InferredType::UntypedInt | InferredType::Int { .. } => ConcatOperand::Decimal,
        InferredType::UntypedFloat | InferredType::Float { .. } => {
            ConcatOperand::Refused("a floating-point value")
        }
        InferredType::Bool => ConcatOperand::Refused("a boolean"),
        InferredType::Null => ConcatOperand::Refused("null"),
        InferredType::Optional(_) => ConcatOperand::Refused("an optional value"),
        InferredType::Bytes | InferredType::BytesBuffer => ConcatOperand::Refused("a byte array"),
        InferredType::List(_) | InferredType::ListBuffer(_) => ConcatOperand::Refused("a list"),
        InferredType::Record(_) => ConcatOperand::Refused("a record"),
        InferredType::Quantity { .. } => ConcatOperand::Refused("a quantity"),
        InferredType::Unknown => ConcatOperand::Refused("a value whose type is not known here"),
    }
}

/// The refusal for a concatenation operand [`concat_operand`] will not carry.
fn concat_refusal(what: &'static str, observed: String) -> ExprError {
    ExprError::UnsupportedConstruct {
        construct: format!(
            "string concatenation with {what} (the text of {what} is not the same in every backend; \
             write the conversion explicitly)"
        ),
        observed: Some(observed),
    }
}

/// The type of `c ? a : b`.
///
/// Two numeric branches join by the arithmetic lattice, as a sum of them
/// would. Two branches of the SAME other type -- both strings, both booleans --
/// are that type: the lattice has no place for them and answered `Unknown`, so
/// `(flag ? 'A' : 'B') + n` was not seen to be a string at all and a conditional
/// that returns text was opaque wherever it stood. Branches of different
/// non-numeric types stay `Unknown`, as a numeric one mixed with a non-numeric
/// one does: that is an expression the author has not made well typed, and it
/// is not covered over here.
///
/// A branch that may be absent makes the conditional an optional value
/// (docs/adr/0055): branches of `optional<T>` and `T`, or of `null` and `T`,
/// are of `optional<T>`. Two `null`s are `null`, which has no type until the
/// place it lands in gives it one.
fn join_branches(consequent: InferredType, alternate: InferredType) -> InferredType {
    use InferredType as T;
    match (consequent, alternate) {
        (T::Str, T::Str) => T::Str,
        (T::Bool, T::Bool) => T::Bool,
        (T::Null, T::Null) => T::Null,
        (T::Optional(elem), T::Null) | (T::Null, T::Optional(elem)) => T::Optional(elem),
        (T::Optional(one), T::Optional(other)) => {
            if slot_admits(one.present(), other.present()) {
                T::Optional(one)
            } else {
                T::Unknown
            }
        }
        (T::Optional(elem), present) | (present, T::Optional(elem)) => {
            if slot_admits(elem.present(), present) {
                T::Optional(elem)
            } else {
                T::Unknown
            }
        }
        (T::Null, present) | (present, T::Null) => {
            OptionalElem::lifting(present).map_or(T::Unknown, T::Optional)
        }
        _ => join_arith(consequent, alternate),
    }
}

/// Whether `node`, already typed, is a value of an enum the typer can tell by its
/// form: a variant `Alias.v` of an enum `ctx` holds, the proven present value of
/// an optional enum (`status.value`), or a conditional of two of them. A variable
/// of an enum is typed `Unknown` as every enum value is
/// ([`InferredType::from_sce_type`]), and is not told here.
fn is_enum_value(node: &TypedExpr, ctx: &TypeCtx<'_>) -> bool {
    match &node.kind {
        ExprKind::Member { object, property } => {
            matches!(&object.kind, ExprKind::Ident(alias) if ctx.lookup_enum(alias).is_some())
                || (property == OPTIONAL_VALUE
                    && object.ty == InferredType::Optional(OptionalElem::Enum))
        }
        ExprKind::Conditional {
            consequent,
            alternate,
            ..
        } => is_enum_value(consequent, ctx) && is_enum_value(alternate, ctx),
        _ => false,
    }
}

/// The type of `x ?? d` (docs/adr/0055): the present value of `x` joined with
/// `d` as a conditional's branches are, or an optional value of it where `d`
/// may be absent too. `Unknown` for an `x` that is no optional value, which
/// [`reject_optional_misuse`] refuses.
fn coalesced(value: InferredType, fallback: InferredType) -> InferredType {
    match value {
        InferredType::Optional(elem) => match fallback {
            InferredType::Optional(_) | InferredType::Null => InferredType::Optional(elem),
            present => join_branches(elem.present(), present),
        },
        _ => InferredType::Unknown,
    }
}

/// Whether the text of a string operand needs parentheses to stand beside a
/// `+`. A binary child follows the backend's own precedence table; a
/// conditional never stands bare, in the languages that spell it as an
/// expression that reaches rightwards: Kotlin's `if (c) a else b + "E"` and
/// Python's `a if c else b + "E"` both read the `+` into the else branch.
fn concat_text_needs_parens(side: &TypedExpr, is_left: bool, precedence: fn(BinOp) -> u8) -> bool {
    matches!(side.kind, ExprKind::Conditional { .. })
        || child_needs_parens(side, BinOp::Add, is_left, precedence)
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
            if *op == BinOp::Coalesce {
                coalesced(left.ty, right.ty)
            } else if is_string_concatenation(*op, left.ty, right.ty) {
                InferredType::Str
            } else if op.is_arith() {
                join_arith(left.ty, right.ty)
            } else if op.is_comparison() || op.is_logical() {
                InferredType::Bool
            } else if op.is_shift() {
                join_shift(left.ty, right.ty)
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
            // `c ? null : Alias.v` is an optional value of the enum
            // (docs/adr/0055, section 2), though the lattice names no type for
            // the enum value itself.
            let null_beside_enum = |one: &TypedExpr, other: &TypedExpr| {
                matches!(one.kind, ExprKind::NullLit) && is_enum_value(other, ctx)
            };
            if null_beside_enum(consequent, alternate) || null_beside_enum(alternate, consequent) {
                InferredType::Optional(OptionalElem::Enum)
            } else {
                join_branches(consequent.ty, alternate.ty)
            }
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
            //
            // `x.value` of an optional value reads what it holds when present
            // (docs/adr/0055); whether it may be read where it stands is
            // [`reject_optional_misuse`]'s. A member is judged by the type of
            // what it is read from, so a record's field named `value` stays
            // that field.
            if let (InferredType::Optional(elem), OPTIONAL_VALUE) = (object.ty, property.as_str()) {
                elem.present()
            } else {
                // A field of the element a list is indexed at is the field of
                // `<list>[]`, whatever the index is, and a place a value holds is
                // typed by walking its type at any depth ([`TypeCtx::value_of`]).
                match place_path(object) {
                    Some(base) => ctx.lookup_var(&format!("{base}.{property}")),
                    None => InferredType::Unknown,
                }
            }
        }
        ExprKind::Index { object, index } => {
            infer_types(object, ctx);
            infer_types(index, ctx);
            match object.ty {
                InferredType::Bytes | InferredType::BytesBuffer => InferredType::Int {
                    signed: false,
                    bits: 8,
                },
                _ => {
                    // RFC §synth-5-A: `<sce:const name="X" type="array<elem, N>">`
                    // registers `X` in `ctx.array_elems`. Recover the
                    // element type so per-language emitters (Kotlin's
                    // narrow-unsigned widening, in particular) can wrap
                    // the index result with the right cast.
                    let table = match &object.kind {
                        ExprKind::Ident(name) => ctx.lookup_array_elem(name.as_str()),
                        _ => None,
                    };
                    // An element of a list a value holds, at any depth, is the
                    // place `<list>[]` ([`TypeCtx::value_of`]).
                    let list = place_path(object);
                    table.unwrap_or_else(|| {
                        list.map_or(InferredType::Unknown, |list| {
                            ctx.lookup_var(&format!("{list}{}", crate::forge::value_path::ELEMENT))
                        })
                    })
                }
            }
        }
        ExprKind::Call {
            callee,
            args,
            params: call_params,
            fails: call_fails,
            into_destination,
            storage,
        } => {
            infer_types(callee, ctx);
            for a in args.iter_mut() {
                infer_types(a, ctx);
            }
            // Resolve the callee's signature. A bare identifier names a
            // stateless import (cross-algorithm dispatch, transform,
            // condition, lookup); a `obj.method` member call names a
            // stateful import method (registered as `"{obj}.{method}"` by
            // `insert_stateful_imports`). Capture `(ret, params, may_fail)`
            // by value so the argument-projection pass below can borrow
            // `args` mutably after the immutable `ctx` lookup ends.
            // A member call is a stateful import's method or a conversion of an
            // imported enum (`<alias>.toNumber`, `<alias>.fromNumber`), both
            // answered by `lookup_member_func`.
            let resolved: Option<(InferredType, Vec<InferredType>, bool)> = match &callee.kind {
                ExprKind::Ident(name) => ctx
                    .lookup_func(name.as_str())
                    .map(|s| (s.ret, s.params.clone(), s.may_fail)),
                ExprKind::Member { object, property } => {
                    if let ExprKind::Ident(obj_name) = &object.kind {
                        ctx.lookup_member_func(obj_name, property)
                            .map(|s| (s.ret, s.params, s.may_fail))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            *call_fails = resolved.as_ref().is_some_and(|(_, _, may_fail)| *may_fail);
            let named = match &callee.kind {
                ExprKind::Ident(name) => ctx.lookup_func(name.as_str()),
                _ => None,
            };
            *into_destination = named.is_some_and(|s| s.reading == CallReading::IntoDestination);
            *storage = named.and_then(|s| s.storage.clone());
            match resolved {
                Some((ret, params, _)) => {
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
                //
                // A map's or a set's length is a `uint32`, as a statechart's
                // list's is, and `has` answers whether it holds a key
                // (docs/adr/0108, D3).
                None if keyed_builtin(callee, args, ctx)
                    .is_some_and(|(builtin, _)| builtin == KeyedBuiltin::Has) =>
                {
                    InferredType::Bool
                }
                None if keyed_builtin(callee, args, ctx).is_some() => InferredType::Int {
                    signed: false,
                    bits: 32,
                },
                None if is_len_builtin(callee, args) && ctx.lengths_are_uint32 => {
                    InferredType::Int {
                        signed: false,
                        bits: 32,
                    }
                }
                None if is_len_builtin(callee, args) => InferredType::UntypedInt,
                // `round` and `floor` both yield a whole number, and the
                // notation they lower from says so outright: with no digit
                // comment attached, the result is an integer. `UntypedInt`
                // rather than a fixed width for the same reason `len` uses it —
                // the surrounding context decides how wide, so a linear unit
                // conversion adopts its declared output width instead of
                // forcing one.
                None if real_to_int_builtin(callee, args).is_some() => InferredType::UntypedInt,
                // `eq(a, b)` answers whether two byte sequences are equal. Its
                // parameters are recorded so the argument check judges them
                // as it judges any declared signature's.
                None if is_bytes_eq_builtin(callee, ctx) => {
                    *call_params = BYTES_EQ_PARAMS.to_vec();
                    InferredType::Bool
                }
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
        // Written by the rewrite at the type it checks, which inference
        // cannot recover from the operator alone.
        ExprKind::Checked { left, right, .. } => {
            infer_types(left, ctx);
            if let Some(right) = right {
                infer_types(right, ctx);
            }
            expr.ty
        }
        // The type its declared operands share (docs/adr/0063, section 3);
        // none where they share none, which the refusal of the operation
        // says.
        ExprKind::Total { left, right, .. } => {
            infer_types(left, ctx);
            if let Some(right) = right {
                infer_types(right, ctx);
            }
            total_operand_type(left.ty, right.as_ref().map(|r| r.ty))
                .unwrap_or(InferredType::Unknown)
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
    // A hex literal's digits include `e`/`E` (`0xFE`, `0xcc9e2d51`), which
    // are digits there and not an exponent; `0x`, `0o` and `0b` literals are
    // integers whatever digits they hold. Reading the `e` of `0xcc9e2d51` as
    // an exponent typed a uint64 product as a real, and Go then refused
    // `&` on the float64 it emitted (sce:std/hash/murmur3_32).
    if n.len() > 1
        && n.starts_with('0')
        && matches!(n.as_bytes()[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B')
    {
        return false;
    }
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

/// A number literal as every backend reads it. The lexer takes the
/// ECMAScript notations — decimal, `0x`, `0o` and `0b`, each prefix in
/// either case — and no backend reads all of them: C has neither `0o` nor
/// `0b`, C++ and Kotlin have no `0o`, and Rust takes only lowercase
/// prefixes. All six read decimal and lowercase `0x`, so an octal or binary
/// literal is written in hex and an uppercase hex prefix is lowered; a
/// decimal, a lowercase hex or a real literal stands as the author wrote
/// it. Hex keeps the literal non-decimal: a float context still refuses it
/// rather than promote it, since [`is_decimal_integer_literal`] reads the
/// author's text.
///
/// ⚠ Every emitter wrote the author's text until 2026-09-24: `flags & 0o17`
/// generated on all six backends and gcc, g++ and kotlinc refused it, and
/// rustc refused `0XFF`, `0B1010` and `0O17` ("invalid base prefix").
fn portable_number_literal(text: &str) -> String {
    match text.get(..2) {
        Some("0X") => format!("0x{}", &text[2..]),
        Some("0o" | "0O" | "0b" | "0B") => match integer_literal_value(text) {
            Some(value) => format!("0x{value:X}"),
            None => text.to_string(),
        },
        _ => text.to_string(),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Operator precedence (per target language) + paren insertion
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The precedence of `??` on every target: below every other binary operator,
/// as ECMAScript places it (it is never written beside `&&` or `||` without
/// parentheses, so the two never share a level in a parsed tree).
const COALESCE_PRECEDENCE: u8 = 0;

/// Why no emitter's operator table spells `??`: an expression that holds an
/// optional value is refused before it reaches an emitter
/// ([`reject_unlowered_optional`]), on every target until its step of
/// docs/adr/0055 lowers one.
const COALESCE_NOT_SPELLED: &str =
    "`??` is refused before an emitter spells an operator (reject_unlowered_optional)";

fn ecma_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Coalesce => COALESCE_PRECEDENCE,
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
        // Kotlin's elvis, which `??` is spelled as, binds tighter than a
        // comparison and looser than an infix function (`and`, `shl`): a
        // comparison under it is parenthesised, and it stands bare under one.
        BinOp::Coalesce => 6,
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
        BinOp::Coalesce => COALESCE_PRECEDENCE,
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

fn python_precedence(op: BinOp) -> u8 {
    match op {
        BinOp::Coalesce => COALESCE_PRECEDENCE,
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
        // A comparison that is an operand of a comparison is parenthesised
        // whatever the table says, on either side: `(a === b) === c` is a
        // test of a truth value against `c`, and no backend but ECMAScript
        // and Go reads `a == b == c` that way. Rust and Swift refuse it
        // ("comparison operators cannot be chained"), Python reads it as
        // `a == b and b == c`, Kotlin will not chain `<`, and GCC flags it
        // under `-Wparentheses`. Where it does associate, the parentheses
        // change nothing.
        //
        // ⚠ Until 2026-11-23 a left operand at the same level was left bare,
        // as an associative operator's is: Rust refused the generated file,
        // and Python ran it and answered something else than the document.
        ExprKind::Binary { op: child_op, .. }
            if child_op.is_comparison() && parent_op.is_comparison() =>
        {
            true
        }
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

/// An operand GCC's `-Wparentheses` flags — `&&` nested inside `||` first
/// among them — parenthesised for the C family even though precedence does
/// not require it.
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
///
/// The same warning has more shapes than `&&` within `||`, each as fatal
/// under `-Werror`, and the rule covers every one GCC names:
/// - `+`/`-`, or a comparison, as an operand of `&`, `|` or `^`
///   (`(h * 5 + c) & m` emitted bare as `h * 5 + c & m` stopped the C11
///   conformance build of `sce:std/hash/murmur3_32`);
/// - `&` or `^` within `|`, and `&` within `^`;
/// - `+`/`-` within `<<` or `>>`.
fn c_family_clarity_parens(child: &TypedExpr, parent_op: BinOp) -> bool {
    use BinOp::*;
    let ExprKind::Binary { op: child_op, .. } = &child.kind else {
        return false;
    };
    let additive = matches!(child_op, Add | Sub);
    match parent_op {
        Or => matches!(child_op, And),
        BitAnd => additive || child_op.is_comparison(),
        BitXor => additive || child_op.is_comparison() || matches!(child_op, BitAnd),
        BitOr => additive || child_op.is_comparison() || matches!(child_op, BitAnd | BitXor),
        Shl | Shr | UShr => additive,
        _ => false,
    }
}

/// Which of the two C-family emitters a shared spelling is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CFamily {
    C,
    Cpp,
}

impl CFamily {
    /// `text` converted to the integer type of `signed` and `bits`.
    fn cast(self, signed: bool, bits: u8, text: &str) -> String {
        let unsigned = if signed { "" } else { "u" };
        match self {
            Self::C => format!("({unsigned}int{bits}_t)({text})"),
            Self::Cpp => format!("static_cast<std::{unsigned}int{bits}_t>({text})"),
        }
    }
}

/// `left op right`, the operands as the operator takes them, of the node type
/// `ty`, as C and C++ write it.
///
/// A left shift wraps at the width of its type (SCE_FORGE.md §3.4.1, "The
/// width of a shift"), and neither language makes it there by itself: an
/// operand narrower than `int` is promoted to `int`, so a `uint8` or `int16`
/// shift keeps the bits it shifts out until its value is converted back to its
/// type, and a left shift of a negative signed value is undefined. So a narrow
/// unsigned shift is converted back to its type, and a signed one is made in
/// the unsigned type of its width and converted back, which is its
/// two's-complement value. Every other operator is written as it is.
///
/// ⚠ Until 2026-11-19 a `uint8` `x << 4` stored in a `uint32` kept 12 bits in
/// C, C++ and Kotlin and 8 in every other backend.
fn c_family_binary(
    family: CFamily,
    op: BinOp,
    ty: InferredType,
    left: &str,
    right: &str,
) -> String {
    let written = format!("{left} {} {right}", cpp_binop(op));
    if op != BinOp::Shl {
        return written;
    }
    match ty.strip_quantity() {
        InferredType::Int { signed: true, bits } => {
            let unsigned = family.cast(false, bits, left);
            family.cast(true, bits, &format!("{unsigned} << {right}"))
        }
        InferredType::Int {
            signed: false,
            bits,
        } if bits < 32 => family.cast(false, bits, &written),
        _ => written,
    }
}

/// `op` over `operand`, written `inner`, of the node type `ty`, as C and C++
/// write it. A `~` of an operand narrower than `int` is converted back to its
/// type, as a left shift is ([`c_family_binary`]): `~x` of a `uint8` is an
/// `int` below 0 until it is.
fn c_family_unary(
    family: CFamily,
    op: UnaryOp,
    ty: InferredType,
    operand: &TypedExpr,
    inner: &str,
) -> String {
    let wrap = matches!(
        &operand.kind,
        ExprKind::Binary { .. } | ExprKind::Conditional { .. }
    );
    let written = if wrap {
        format!("{}({inner})", cpp_unary(op))
    } else {
        format!("{}{inner}", cpp_unary(op))
    };
    match (op, ty.strip_quantity()) {
        (UnaryOp::BitNot, InferredType::Int { signed, bits }) if bits < 32 => {
            family.cast(signed, bits, &written)
        }
        _ => written,
    }
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
///
/// For a shift it is the type of the left operand ([`join_shift`]), which only
/// the left operand is coerced to: the count is an integer of its own, and
/// each emitter writes it at its own type ([`right_operand_type`]).
fn binary_operand_type(op: BinOp, left: InferredType, right: InferredType) -> InferredType {
    if op.is_arith() {
        join_arith(left, right)
    } else if op.is_comparison() {
        // Comparison returns Bool but operands must share a numeric type.
        join_arith(left, right)
    } else if op.is_logical() {
        InferredType::Bool
    } else if op.is_shift() {
        join_shift(left, right)
    } else if op.is_bitwise() {
        join_int(left, right)
    } else {
        InferredType::Unknown
    }
}

/// The type the right operand of a binary node is written at, given the
/// `operand_ty` both operands share ([`binary_operand_type`]): that type, but
/// for the count of a shift, which is written at its own. Converting a count
/// to the type of what it shifts makes no shift right, and could only turn a
/// count past the width into one inside it. Kotlin, whose shifts take an
/// `Int` count, converts the count to that instead.
fn right_operand_type(op: BinOp, operand_ty: InferredType) -> InferredType {
    if op.is_shift() {
        InferredType::Unknown
    } else {
        operand_ty
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

/// Recognizes the `len(<bytes|str|list>)` builtin — the length accessor the
/// two-cursor keyexpr matcher uses to bound `pi < len(pattern)`, and the one
/// read a `sce-static` list variable admits. Each
/// emitter lowers it to its native length idiom (C11 `.len`, Rust
/// `.len()`, Cpp `.size()`, Kotlin `.size`, Go/Python `len(x)`) rather
/// than the generic `len(args)` call. Item C7 wildcard-keyexpr lowering.
fn is_len_builtin(callee: &TypedExpr, args: &[TypedExpr]) -> bool {
    args.len() == 1
        && matches!(
            args[0].ty,
            InferredType::Bytes
                | InferredType::BytesBuffer
                | InferredType::Str
                | InferredType::List(_)
                | InferredType::ListBuffer(_)
        )
        && matches!(&callee.kind, ExprKind::Ident(n) | ExprKind::Raw(n) if n == "len")
}

/// The builtin that asks a map or a set whether it holds a key, `has(m, k)`
/// (docs/adr/0108, D3). It is the vocabulary's only where its first argument
/// names a map or a set the scope holds and no callable of the name is
/// registered ([`keyed_builtin`]), so every kind that holds no map keeps the
/// name for its own.
const HAS_BUILTIN: &str = "has";

/// A read of a map or a set through a builtin (docs/adr/0108, D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyedBuiltin {
    /// `len(m)`: how many entries it holds, a `uint32` as a list's length is.
    Len,
    /// `has(m, k)`: whether `k` is one of its keys.
    Has,
}

/// The map or the set `node` names, when it is a place the scope holds one in
/// (docs/adr/0108): a statechart's map or set variable. `None` for any other
/// expression, and in every scope that holds no value of the one value type.
pub(crate) fn keyed_place(
    node: &TypedExpr,
    ctx: &TypeCtx<'_>,
) -> Option<crate::forge::value_path::Place> {
    place_path(node)
        .and_then(|path| ctx.value_of(&path))
        .filter(crate::forge::value_path::Place::is_keyed)
}

/// Which read of a map or a set the call of `callee` with `args` is, with the
/// map or the set it reads: `len` of one argument, or `has` of any number,
/// whose first argument names a map or a set ([`keyed_place`]) and whose name
/// no callable of the scope takes. `None` for any other call.
fn keyed_builtin(
    callee: &TypedExpr,
    args: &[TypedExpr],
    ctx: &TypeCtx<'_>,
) -> Option<(KeyedBuiltin, crate::forge::value_path::Place)> {
    let ExprKind::Ident(name) = &callee.kind else {
        return None;
    };
    let builtin = match name.as_str() {
        "len" if args.len() == 1 => KeyedBuiltin::Len,
        HAS_BUILTIN if ctx.lookup_func(HAS_BUILTIN).is_none() => KeyedBuiltin::Has,
        _ => return None,
    };
    Some((builtin, keyed_place(args.first()?, ctx)?))
}

/// Each read of a map or a set of `expr` (docs/adr/0108, D3), spelled for the
/// Interpreter's library, which holds a map as the array of its entries
/// ([`crate::forge::static_js::map_descriptor`]): `has(m, k)` is a call of its
/// `mapHas`, and `m[k]` of its `mapGet`, which fails `out-of-range` for a key
/// that is absent, as an index outside a list does, and so is a call that can
/// fail. `len(m)` is the `length` of the array, as a list's is: written as that
/// read here, since the emitter spells `len` as a length only of a value whose
/// type it knows as measured ([`is_len_builtin`]), and a map is typed by no
/// lattice type. Run after the reads are judged and before any rename, while
/// each map is the place the author named; a scope that holds no map holds no
/// such read.
///
/// Which read a node is, and of which map, is asked of the node before its
/// children are lowered: a map held at a key of another (`m[k][k2]`,
/// `has(m[k], k2)`, `len(m[k])`, docs/adr/0108, D12) is known by the place
/// the author wrote, which the lowering of the child that reads it turns into
/// a call of `mapGet`, a value no place names. Asked after, the outer read
/// would be left an index of the array of entries by a key, which reads no
/// entry.
fn lower_keyed_reads_for_js(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) -> Result<(), Refusal> {
    let measures_a_map = matches!(
        &expr.kind,
        ExprKind::Call { callee, args, .. }
            if matches!(keyed_builtin(callee, args, ctx), Some((KeyedBuiltin::Len, _)))
    );
    let keyed = match &expr.kind {
        ExprKind::Index { object, .. } => {
            keyed_place(object, ctx).map(|held| ("mapGet", held, true))
        }
        ExprKind::Call { callee, args, .. } => match keyed_builtin(callee, args, ctx) {
            Some((KeyedBuiltin::Has, held)) => Some(("mapHas", held, false)),
            _ => None,
        },
        _ => None,
    };
    for child in expr.children_mut() {
        lower_keyed_reads_for_js(child, ctx)?;
    }
    if measures_a_map {
        if let ExprKind::Call { mut args, .. } =
            std::mem::replace(&mut expr.kind, ExprKind::NullLit)
        {
            if let Some(map) = args.pop() {
                expr.kind = ExprKind::Member {
                    object: Box::new(map),
                    property: "length".to_string(),
                };
            }
        }
        return Ok(());
    }
    let Some((helper, held, fails)) = keyed else {
        return Ok(());
    };
    let unspelled = || {
        ExprError::UnsupportedConstruct {
            construct: format!(
                "a read of the map or set of type `{}`, whose key has no ecmascript spelling",
                held.ty
            ),
            observed: None,
        }
        .at(expr.span.clone())
    };
    let Some(descriptor) = crate::forge::static_js::map_descriptor(&held, ctx.value_scope) else {
        return Err(unspelled());
    };
    let (map, key) = match std::mem::replace(&mut expr.kind, ExprKind::NullLit) {
        ExprKind::Index { object, index } => (*object, *index),
        ExprKind::Call { args, .. } => {
            let mut args = args.into_iter();
            match (args.next(), args.next()) {
                (Some(map), Some(key)) => (map, key),
                _ => return Err(unspelled()),
            }
        }
        other => {
            expr.kind = other;
            return Ok(());
        }
    };
    expr.kind = ExprKind::Call {
        callee: Box::new(TypedExpr::new(ExprKind::Raw(format!(
            "{}.{helper}",
            crate::forge::static_js::RUNTIME_GLOBAL
        )))),
        args: vec![map, TypedExpr::new(ExprKind::Raw(descriptor)), key],
        params: Vec::new(),
        fails,
        into_destination: false,
        storage: None,
    };
    Ok(())
}

/// The runtime module whose functions the reads of a Rust machine's map or set
/// lower to (docs/adr/0108, D3): `count`, `holds`, `value` and
/// `value_cloned`, each handed the map and the key lent.
pub(crate) const RUST_MAP_READS: &str = "::sce_rust_runtime::portable::map";

/// Each read of a map or a set of `expr` (docs/adr/0108, D3), spelled for a
/// Rust machine, which holds one in the runtime's `BoundedMap` or `BoundedSet`
/// and reads it through [`RUST_MAP_READS`]'s functions, each handed the map and
/// the key lent: `len(m)` is its `count`, a `uint32` as a list's length is;
/// `has(m, k)` its `holds`; `m[k]` its `value_cloned`, the copy the place it
/// is written to takes; and `m[k].f` a field of its `value`, lent where the map
/// holds it, so a read copies no more than the field. A value read at a key
/// that is absent is none, which `found` fails `out-of-range` as an index
/// outside a list fails, and so is a call that can fail. A key is looked up by
/// its canonical bytes (the runtime's `MapKey`), so a value of another width, a
/// text slice and a payload taken whole each find the key they write the bytes
/// of; an integer literal is written at the key's own type, which its value is
/// held to where it is judged. Run after the reads are judged and before any
/// rename, while each map is the place the author named.
///
/// A statechart's map holds a record and a value of an enum in the type its
/// unit declares (docs/adr/0108, D6 item 4, [`TypeCtx::map_units`]): a field of
/// a record entry is read by the name the unit gives it, and a value of an enum
/// read out of one, an entry or a field of one, crosses into the machine's own
/// type where it is read. A record entry taken whole crosses where the
/// statement that takes it is lowered.
///
/// An algorithm calls the same functions of the leaf crate its map type is
/// declared in ([`TypeCtx::rust_algorithm_map_reads`]). In one that does not
/// declare `may-fail`, which has no failure to hand on, a value at a key is
/// read where the build proved the key held (D6 item 2; any other read is
/// refused as `algorithm/undeclared-integer-failure`), by `held` and
/// `held_cloned`, which read it as a proven index of a list is read.
fn lower_keyed_reads_for_rust(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    let reads = ctx.rust_algorithm_map_reads().unwrap_or(RUST_MAP_READS);
    // `m[k].f`: the field of the value the map lends.
    if let ExprKind::Member { object, property } = &mut expr.kind {
        let entry = keyed_entry(object, ctx);
        if rust_keyed_value(object, ctx, "value") {
            let crossing = entry.and_then(|entry| rust_unit_field(&entry, property, ctx));
            if let Some(crossing) = crossing {
                crossing.apply(expr);
            }
            return;
        }
    }
    let entry = keyed_entry(expr, ctx);
    if rust_keyed_value(expr, ctx, "value_cloned") {
        if let Some(crossing) = entry.and_then(|entry| rust_unit_enum(&entry, ctx)) {
            crossing.apply(expr);
        }
        return;
    }
    let builtin = match &expr.kind {
        ExprKind::Call { callee, args, .. } => keyed_builtin(callee, args, ctx),
        _ => None,
    };
    // `len(m[k])`, `has(m[k], k2)`: a map held at a key is read where the map
    // that holds it lends it (docs/adr/0108, D12), a reference already.
    let lent_map = matches!(
        &expr.kind,
        ExprKind::Call { args, .. } if args.first().is_some_and(|map| rust_lends_keyed(map, ctx))
    );
    for child in expr.children_mut() {
        lower_keyed_reads_for_rust(child, ctx);
    }
    let Some((builtin, held)) = builtin else {
        return;
    };
    let ExprKind::Call { callee, args, .. } = &mut expr.kind else {
        return;
    };
    let function = match builtin {
        KeyedBuiltin::Len => "count",
        KeyedBuiltin::Has => "holds",
    };
    **callee = TypedExpr::new(ExprKind::Raw(format!("{reads}::{function}")));
    for (position, arg) in args.iter_mut().enumerate() {
        if position == 1 {
            rust_typed_key_literal(arg, &held);
        }
        if position == 0 && lent_map {
            continue;
        }
        let ty = arg.ty;
        wrap_in_call(arg, "&", ty);
    }
}

/// Whether `node` reads a map, a set or a list held at a key of a map
/// (`m[k]`, docs/adr/0108, D12), which a Rust machine reads where the map that
/// holds it lends it ([`rust_keyed_value`]): a container is no operand (rule
/// 6), so it is read only through a further step, a builtin, a loop, or a copy
/// the statement that takes it whole makes.
fn rust_lends_keyed(node: &TypedExpr, ctx: &TypeCtx<'_>) -> bool {
    keyed_entry(node, ctx).is_some_and(|value| value.ty.is_container())
}

/// The place of the value `node` reads at a key of a map (`m[k]`,
/// [`keyed_place`]); `None` for any other node.
fn keyed_entry(node: &TypedExpr, ctx: &TypeCtx<'_>) -> Option<crate::forge::value_path::Place> {
    let ExprKind::Index { object, .. } = &node.kind else {
        return None;
    };
    keyed_place(object, ctx)?.map_value()
}

/// How a value read out of a map's entry crosses from the type its unit
/// declares into the machine's own (docs/adr/0108, D6 item 4): through `from`,
/// the function the machine declares for its enum, applied to the value, or to
/// the present value of an optional one.
struct RustEntryCrossing {
    from: String,
    optional: bool,
}

impl RustEntryCrossing {
    /// `node`, the read, made the machine's value it crosses into.
    fn apply(self, node: &mut TypedExpr) {
        let ty = node.ty;
        if !self.optional {
            wrap_in_call(node, &self.from, ty);
            return;
        }
        let inner = std::mem::replace(node, TypedExpr::new(ExprKind::Raw(String::new())));
        *node = TypedExpr {
            kind: ExprKind::Call {
                callee: Box::new(TypedExpr::new(ExprKind::Raw(
                    "::core::option::Option::map".to_string(),
                ))),
                args: vec![inner, TypedExpr::new(ExprKind::Raw(self.from))],
                params: Vec::new(),
                fails: false,
                into_destination: false,
                storage: None,
            },
            ty,
            span: None,
        };
    }
}

/// The crossing of a value of an enum at `at`, or of an optional value of one,
/// that a statechart's map holds in its unit's type ([`TypeCtx::map_units`]),
/// by the alias the statechart holds the enum by
/// ([`crate::forge::value_path::enum_alias`]); `None` for a place that holds
/// no value of an enum, and for an enum no map holds in its unit's type.
fn rust_unit_enum(
    at: &crate::forge::value_path::Place,
    ctx: &TypeCtx<'_>,
) -> Option<RustEntryCrossing> {
    use crate::forge::model::SceType;
    use crate::forge::value_type::ValueType;
    if ctx.map_units.enums.is_empty() {
        return None;
    }
    let (elem, optional) = match &at.ty {
        ValueType::Optional { elem } => (elem.as_ref(), true),
        other => (other, false),
    };
    let ValueType::Scalar(SceType::Enum(reference)) = elem else {
        return None;
    };
    let present = crate::forge::value_path::Place {
        ty: elem.clone(),
        scope: at.scope.clone(),
    };
    let alias = ctx
        .value_scope
        .and_then(|base| crate::forge::value_path::enum_alias(base, &present))
        .unwrap_or_else(|| reference.alias.clone());
    let unit = ctx.map_units.enums.get(&alias)?;
    Some(RustEntryCrossing {
        from: unit.back.clone(),
        optional,
    })
}

/// The field `property` of a record entry of a statechart's map, `entry`,
/// which the map holds in its unit's type ([`TypeCtx::map_units`]): renamed to
/// the name the unit gives it, with the crossing of the value it reads where
/// that is a value of an enum ([`rust_unit_enum`]). Nothing for an entry that
/// is no record, and for a record no map holds in its unit's type.
fn rust_unit_field(
    entry: &crate::forge::value_path::Place,
    property: &mut String,
    ctx: &TypeCtx<'_>,
) -> Option<RustEntryCrossing> {
    use crate::forge::value_type::ValueType;
    let ValueType::Record { alias } = &entry.ty else {
        return None;
    };
    if !ctx.map_units.records.contains_key(alias) {
        return None;
    }
    let field = crate::forge::value_path::walk(
        ctx.value_scope,
        entry,
        &[crate::forge::value_path::Step::Field(property.as_str())],
    );
    *property = crate::forge::generator::event_schema_field_ident(
        property,
        crate::generator::Language::Rust,
    );
    rust_unit_enum(&field?, ctx)
}

/// `node`, when it reads the value at a key of a map (`m[k]`), as the read of
/// it through `function` of [`RUST_MAP_READS`] that `found` fails where the
/// key is absent ([`lower_keyed_reads_for_rust`]), its key's own reads lowered
/// first; `false`, and `node` as it was, for any other node.
///
/// A map, a set or a list held at the key (docs/adr/0108, D12) is read through
/// `value`, lent where the map holds it, whatever `function` asks: a container
/// is read only through a further step, a builtin, a loop, or a copy the
/// statement that takes it whole makes (rule 6), so no read copies it. A map
/// read at a key of one held at a key (`m[k][k2]`) is read through the one the
/// outer map lends, never through a copy of the outer map.
fn rust_keyed_value(node: &mut TypedExpr, ctx: &TypeCtx<'_>, function: &str) -> bool {
    let held = match &node.kind {
        ExprKind::Index { object, .. } => keyed_place(object, ctx),
        _ => None,
    };
    let Some(held) = held else {
        return false;
    };
    let function = if held
        .map_value()
        .is_some_and(|value| value.ty.is_container())
    {
        "value"
    } else {
        function
    };
    let ExprKind::Index { object, index } = std::mem::replace(&mut node.kind, ExprKind::NullLit)
    else {
        return false;
    };
    let (mut map, mut key) = (*object, *index);
    lower_keyed_reads_for_rust(&mut key, ctx);
    rust_typed_key_literal(&mut key, &held);
    let (map_ty, key_ty) = (map.ty, key.ty);
    // A map held at a key of another is the reference its map lends.
    if !rust_keyed_value(&mut map, ctx, "value") {
        wrap_in_call(&mut map, "&", map_ty);
    }
    wrap_in_call(&mut key, "&", key_ty);
    let algorithm_reads = ctx.rust_algorithm_map_reads();
    let reads = algorithm_reads.unwrap_or(RUST_MAP_READS);
    // An algorithm's body that hands on no failure reads the key the build
    // proved held ([`lower_keyed_reads_for_rust`]); a machine receives every
    // failure it runs into.
    if algorithm_reads.is_some() && !ctx.receives_failures {
        let proven = match function {
            "value" => "held",
            _ => "held_cloned",
        };
        node.kind = ExprKind::Call {
            callee: Box::new(TypedExpr::new(ExprKind::Raw(format!("{reads}::{proven}")))),
            args: vec![map, key],
            params: Vec::new(),
            fails: false,
            into_destination: false,
            storage: None,
        };
        return true;
    }
    let lookup = TypedExpr::new(ExprKind::Call {
        callee: Box::new(TypedExpr::new(ExprKind::Raw(format!(
            "{reads}::{function}"
        )))),
        args: vec![map, key],
        params: Vec::new(),
        fails: false,
        into_destination: false,
        storage: None,
    });
    let found = found_wrapper(ExprTarget::Rust).unwrap_or("sce_forge_runtime::algorithm::found");
    node.kind = ExprKind::Call {
        callee: Box::new(TypedExpr::new(ExprKind::Raw(found.to_string()))),
        args: vec![lookup],
        params: vec![InferredType::Unknown],
        fails: true,
        into_destination: false,
        storage: None,
    };
    true
}

/// An integer literal `key` of the map or the set `held` written at the
/// type of its key (`7u32`, `-2i32`): a key is looked up by any value whose
/// canonical bytes are a key's, and a bare literal would take the type Rust
/// gives one it cannot infer, which may not hold it. Any other key is left as
/// it is.
fn rust_typed_key_literal(key: &mut TypedExpr, held: &crate::forge::value_path::Place) {
    use crate::forge::model::SceType;
    use crate::forge::value_type::ValueType;
    let Some(ValueType::Scalar(scalar)) = held.map_key().map(|key| key.ty) else {
        return;
    };
    if !matches!(
        scalar,
        SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Uint64
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32
            | SceType::Int64
    ) {
        return;
    }
    let suffix = crate::forge::generator::rust_type(&scalar);
    let typed = match &key.kind {
        ExprKind::NumberLit(text) if !is_float_literal_text(text) => {
            format!("{}{suffix}", portable_number_literal(text))
        }
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match &operand.kind {
            ExprKind::NumberLit(text) if !is_float_literal_text(text) => {
                format!("-{}{suffix}", portable_number_literal(text))
            }
            _ => return,
        },
        _ => return,
    };
    key.kind = ExprKind::Raw(typed);
}

/// What `len(m)` of a C11 machine's map or set calls (`sce/forge/map.h`).
const C_MAP_COUNT: &str = "sce_map_count";
/// What `has(m, k)` calls.
const C_MAP_HOLDS: &str = "sce_map_holds";
/// What finds the value `m[k]` reads, a call that may fail, whose failure its
/// `_take` records ([`pass_failure_on`]).
const C_MAP_LOOKUP: &str = "sce_map_lookup";
/// What takes the value a lookup found where the read is proven to find its
/// key, in a body that receives no failure.
const C_MAP_FOUND: &str = "sce_map_lookup_found";
/// What reads the value the lookup found as the map holds its values.
const C_MAP_VALUE_OF: &str = "SCE_MAP_VALUE_OF";
/// What reads it as a value of the type it is handed, for a map lent by the
/// address of its storage ([`TypeCtx::c11_algorithm_map_value`]).
const C_MAP_VALUE_AS: &str = "SCE_MAP_VALUE_AS";

/// Each read of a map or a set of `expr` (docs/adr/0108, D3), spelled for a
/// C11 machine, which holds one in the type `sce/forge/map.h` declares for its
/// parts and its bound and reads it through the header's functions over the
/// map's address: `len(m)` is its `sce_map_count`, a `uint32` as a list's
/// length is; `has(m, k)` its `sce_map_holds`; and `m[k]` the value its
/// `sce_map_lookup` finds, read where the map holds it as the map holds its
/// values (`SCE_MAP_VALUE_OF`), a call that may fail: a key that is absent
/// fails `out-of-range`, as an index outside a list does, and gives the map's
/// zero value. A text value is read as its buffer's text, and so is a text
/// field of a record value (`m[k].f`).
///
/// An algorithm reads its map parameter the same way, through the address of
/// the caller's storage, which names no type of its bound: the value is read
/// as the type the algorithm names for it (`SCE_MAP_VALUE_AS`,
/// [`TypeCtx::c11_algorithm_map_value`]), and in a body that does not declare
/// `may-fail`, whose reads at a key the analysis proved present, as the value
/// the lookup found with no failure to record (`sce_map_lookup_found`).
///
/// A record the map holds is of the type its unit declares (D6 item 4,
/// [`TypeCtx::map_units`]), which names every field as the machine's record
/// does: a field of it, at a key or as the item or the key of a walk
/// ([`TypeCtx::unit_held_names`]), is read where it stands. A record entry
/// taken whole crosses where the statement that takes it is lowered.
///
/// A key is handed as a probe and the function that writes the probe's
/// canonical bytes ([`c_map_probe`]), so a value of another width, a text
/// pointer and the payload taken whole each find the key whose bytes they
/// write. Run after the reads are judged and before any rename, while each map
/// is the place the author named; a scope that holds no map holds no such
/// read.
fn lower_keyed_reads_for_c(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) -> Result<(), Refusal> {
    let ty = expr.ty;
    if let ExprKind::Member { object, property } = &mut expr.kind {
        // `m[k].f`: the field of the value the lookup found.
        if c_keyed_value(object, ctx)? {
            if ty == InferredType::Str {
                property.push('.');
                property.push_str(C_BUFFER_TEXT);
            }
            return Ok(());
        }
    }
    if c_keyed_value(expr, ctx)? {
        if ty == InferredType::Str {
            let value = std::mem::replace(expr, TypedExpr::new(ExprKind::NullLit));
            *expr = TypedExpr {
                span: value.span.clone(),
                ty,
                kind: ExprKind::Member {
                    object: Box::new(value),
                    property: C_BUFFER_TEXT.to_string(),
                },
            };
        }
        return Ok(());
    }
    let builtin = match &expr.kind {
        ExprKind::Call { callee, args, .. } => keyed_builtin(callee, args, ctx),
        _ => None,
    };
    // The key of a `has` is lowered as the probe it is handed as
    // ([`c_map_probe`]), and its map where the call is lowered below.
    if !matches!(builtin, Some((KeyedBuiltin::Has, _))) {
        for child in expr.children_mut() {
            lower_keyed_reads_for_c(child, ctx)?;
        }
    }
    let Some((builtin, held)) = builtin else {
        return Ok(());
    };
    let span = expr.span.clone();
    let ExprKind::Call {
        callee,
        args,
        params,
        ..
    } = &mut expr.kind
    else {
        return Ok(());
    };
    params.clear();
    match builtin {
        KeyedBuiltin::Len => {
            **callee = TypedExpr::new(ExprKind::Raw(C_MAP_COUNT.to_string()));
            if let Some(map) = args.first_mut() {
                let map_ty = map.ty;
                wrap_in_call(map, "&", map_ty);
            }
        }
        KeyedBuiltin::Has => {
            let mut taken = std::mem::take(args).into_iter();
            let (Some(mut map), Some(key)) = (taken.next(), taken.next()) else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: "a `has` of a map or a set that names no key".to_string(),
                    observed: None,
                }
                .at(span));
            };
            let (probe, feed) = c_map_probe(key, &held, ctx)?;
            // A map variable is a place no read lowers; a map held at a key of
            // another (`has(m[k], k2)`, docs/adr/0108, D12) is read first.
            lower_keyed_reads_for_c(&mut map, ctx)?;
            let map_ty = map.ty;
            wrap_in_call(&mut map, "&", map_ty);
            **callee = TypedExpr::new(ExprKind::Raw(C_MAP_HOLDS.to_string()));
            *args = vec![map, probe, feed];
        }
    }
    Ok(())
}

/// How `node` is held where it reads a record a map holds in the type the
/// record's own unit declares (docs/adr/0108, D6 item 4,
/// [`TypeCtx::map_units`]): the value at a key of a map of such records
/// (`m[k]`), or the item or the key a walk of one binds
/// ([`TypeCtx::unit_held_names`]). `None` for any other node, and in every
/// scope whose maps hold no record in its unit's type.
fn c_unit_record<'c>(
    node: &TypedExpr,
    ctx: &'c TypeCtx<'_>,
) -> Option<&'c crate::forge::types::MapUnit> {
    if ctx.map_units.records.is_empty() {
        return None;
    }
    match &node.kind {
        ExprKind::Ident(name) => ctx.unit_held_name(name),
        ExprKind::Index { object, .. } => match keyed_place(object, ctx)?.map_value()?.ty {
            crate::forge::value_type::ValueType::Record { alias } => {
                ctx.map_units.records.get(&alias)
            }
            _ => None,
        },
        _ => None,
    }
}

/// `node`, when it reads the value at a key of a map (`m[k]`), as the read of
/// the value [`C_MAP_LOOKUP`] finds there, its key lowered as the probe it is
/// handed as ([`c_map_probe`]); `false`, and `node` as it was, for any other
/// node.
fn c_keyed_value(node: &mut TypedExpr, ctx: &TypeCtx<'_>) -> Result<bool, Refusal> {
    let held = match &node.kind {
        ExprKind::Index { object, .. } => keyed_place(object, ctx),
        _ => None,
    };
    let Some(held) = held else {
        return Ok(false);
    };
    let ExprKind::Index { object, index } = std::mem::replace(&mut node.kind, ExprKind::NullLit)
    else {
        return Ok(false);
    };
    let (mut map, key) = (*object, *index);
    // A map an algorithm is lent by the address of its storage names no type
    // its value could be read as, so the read names the value's own
    // ([`TypeCtx::c11_algorithm_map_value`]).
    let lent_value = match &map.kind {
        ExprKind::Ident(name) => ctx.c11_algorithm_map_value(name).map(str::to_string),
        _ => None,
    };
    // A map held at a key of another (`m[k][k2]`, docs/adr/0108, D12) is
    // itself the value a read finds: it is read first, where the map that
    // holds it holds it, and its lookup's failure is the read's.
    lower_keyed_reads_for_c(&mut map, ctx)?;
    let (probe, feed) = c_map_probe(key, &held, ctx)?;
    let mut address = map.clone();
    let address_ty = address.ty;
    wrap_in_call(&mut address, "&", address_ty);
    // A body that receives no failure is one whose every read at a key the
    // analysis proved present (`algorithm/undeclared-integer-failure`), so the
    // value is the one the lookup found, read with no failure to record.
    let lookup = TypedExpr::new(ExprKind::Call {
        callee: Box::new(TypedExpr::new(ExprKind::Raw(C_MAP_LOOKUP.to_string()))),
        args: vec![address, probe, feed],
        params: Vec::new(),
        fails: ctx.receives_failures,
        into_destination: false,
        storage: None,
    });
    let found = if ctx.receives_failures {
        lookup
    } else {
        TypedExpr::new(ExprKind::Call {
            callee: Box::new(TypedExpr::new(ExprKind::Raw(C_MAP_FOUND.to_string()))),
            args: vec![lookup],
            params: Vec::new(),
            fails: false,
            into_destination: false,
            storage: None,
        })
    };
    // A byte string an algorithm holds is a view of its bytes, so one read out
    // of a map lent to it is read as the view of the buffer the map holds.
    let lent_bytes = match held.map_value().map(|value| value.ty) {
        Some(crate::forge::value_type::ValueType::Bytes { max }) if lent_value.is_some() => {
            Some(crate::forge::static_lowering::c_bytes_storage_view(max.max))
        }
        _ => None,
    };
    let (read, args) = match (lent_bytes, lent_value) {
        (Some(view), _) => (view, vec![found]),
        (None, Some(value)) => (
            C_MAP_VALUE_AS.to_string(),
            vec![TypedExpr::new(ExprKind::Raw(value)), found],
        ),
        (None, None) => (C_MAP_VALUE_OF.to_string(), vec![map, found]),
    };
    node.kind = ExprKind::Call {
        callee: Box::new(TypedExpr::new(ExprKind::Raw(read))),
        args,
        params: Vec::new(),
        fails: false,
        into_destination: false,
        storage: None,
    };
    Ok(true)
}

/// The probe of the key `key` of the map or the set `held` a C11 read hands the
/// lookup, its own reads lowered first ([`lower_keyed_reads_for_c`]), and the
/// function that writes the probe's canonical bytes (`sce/forge/map.h`,
/// docs/adr/0108, D1): an integer as a 64-bit integer of its sign, the key's
/// where the literal states none, a value of an enum as the number its constant
/// carries, a `bool`, a text and a byte string each by the header's probe of
/// it, and a record where it stands: one a map holds in the type its unit
/// declares, the map's own key type, by the map's own feed (`NULL`), read where
/// that map holds it rather than built into the machine's record; one the
/// machine holds in its own record type by that record's feed
/// ([`crate::forge::types::MapUnit::key_feed`]), which writes the same bytes;
/// and the payload taken whole by the feed its key declaration names
/// ([`TypeCtx::payload_key_feed`]).
fn c_map_probe(
    mut key: TypedExpr,
    held: &crate::forge::value_path::Place,
    ctx: &TypeCtx<'_>,
) -> Result<(TypedExpr, TypedExpr), Refusal> {
    use crate::forge::model::SceType;
    use crate::forge::value_type::ValueType;
    let raw = |text: &str| TypedExpr::new(ExprKind::Raw(text.to_string()));
    let probe = |made_by: &str, mut key: TypedExpr| {
        let key_ty = key.ty;
        wrap_in_call(&mut key, made_by, key_ty);
        key
    };
    let unspelled = |span: Option<std::ops::Range<usize>>, what: &str| {
        ExprError::UnsupportedConstruct {
            construct: format!("{what}, which a C11 map looks up by no key"),
            observed: None,
        }
        .at(span)
    };
    let span = key.span.clone();
    let Some(key_place) = held.map_key() else {
        return Err(unspelled(span, "a read of a value that is no map"));
    };
    let unit_held = c_unit_record(&key, ctx).is_some();
    lower_keyed_reads_for_c(&mut key, ctx)?;
    Ok(match key_place.ty {
        ValueType::Record { alias } => {
            let payload = place_path(&key).as_deref()
                == Some(crate::forge::event_schema_check::EVENT_DATA_PATH);
            let feed = if payload {
                ctx.payload_key_feed
                    .clone()
                    .ok_or_else(|| unspelled(span, "a payload taken whole as a record key"))?
            } else if unit_held {
                "NULL".to_string()
            } else {
                ctx.map_units
                    .records
                    .get(&alias)
                    .and_then(|unit| unit.key_feed.clone())
                    .unwrap_or_else(|| "NULL".to_string())
            };
            (probe("&", key), raw(&feed))
        }
        ValueType::Text { .. } => (probe("SCE_MAP_KEY_TEXT", key), raw("sce_map_key_text_at")),
        ValueType::Bytes { .. } => (
            probe("SCE_MAP_KEY_BYTES", key),
            raw("sce_map_key_bytes_view"),
        ),
        ValueType::Scalar(SceType::Bool) => {
            (probe("SCE_MAP_KEY_BOOL", key), raw("sce_map_key_bool"))
        }
        ValueType::Scalar(SceType::Enum(_)) => {
            (probe("SCE_MAP_KEY_UINT", key), raw("sce_map_key_uint64"))
        }
        ValueType::Scalar(scalar) => {
            let signed = match key.ty {
                InferredType::Int { signed, .. } => signed,
                _ => scalar.is_signed(),
            };
            if signed {
                (probe("SCE_MAP_KEY_INT", key), raw("sce_map_key_int64"))
            } else {
                (probe("SCE_MAP_KEY_UINT", key), raw("sce_map_key_uint64"))
            }
        }
        other => return Err(unspelled(span, &format!("a key of type `{other}`"))),
    })
}

/// Where the key of a read at a key of the map `map` names stands
/// (docs/adr/0108, D3), for [`reject_composed_operands`]: whole where the
/// map's key is a record, which a key names whole, and as an operand
/// everywhere else, a number, a bool, an enum, a text or a byte string being
/// read as a value there.
fn key_stands(map: &TypedExpr, ctx: &TypeCtx<'_>) -> ReadThrough {
    let record_key = keyed_place(map, ctx)
        .and_then(|held| held.map_key())
        .is_some_and(|key| matches!(key.ty, crate::forge::value_type::ValueType::Record { .. }));
    if record_key {
        ReadThrough::Whole
    } else {
        ReadThrough::Operand
    }
}

/// The enum `node` is a value of, as the scope's types resolve it: a variant
/// `Alias.v` of an enum the scope imports, or a place that holds a value of an
/// enum, read in the scope of the document that declares it. `None` for any
/// other expression, and in a scope with no types of the one value type.
fn enum_held_by<'a>(
    node: &TypedExpr,
    ctx: &TypeCtx<'a>,
) -> Option<&'a crate::forge::value_type::ScopedEnum> {
    let base = ctx.value_scope?;
    if let ExprKind::Member { object, .. } = &node.kind {
        if let ExprKind::Ident(alias) = &object.kind {
            if ctx.lookup_enum(alias).is_some() {
                return base.enums.get(alias.as_str());
            }
        }
    }
    let place = ctx.value_of(&place_path(node)?)?;
    let crate::forge::value_type::ValueType::Scalar(crate::forge::model::SceType::Enum(reference)) =
        &place.ty
    else {
        return None;
    };
    crate::forge::value_path::scope_at(base, &place.scope)?
        .enums
        .get(&reference.alias)
}

/// Refuse a read of a map or a set (docs/adr/0108, D3) that is no read of it:
/// the value at a key of a set, whose entries carry none; a `has` of other
/// than two arguments; and a key that does not fit the map's key
/// ([`judge_key`]). A scope that holds no map or set holds no such read.
fn reject_misread_keys(expr: &TypedExpr, ctx: &TypeCtx<'_>, source: &str) -> Result<(), Refusal> {
    let written = |node: &TypedExpr| {
        node.span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string)
    };
    match &expr.kind {
        ExprKind::Index { object, index } => {
            if let Some(held) = keyed_place(object, ctx) {
                if held.is_set() {
                    let set = written(object).unwrap_or_default();
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!(
                            "`{}`, a value read at a key of the set `{set}` (a set holds keys \
                             that carry no value: `{HAS_BUILTIN}({set}, k)` asks whether `k` is \
                             one of them)",
                            written(expr).unwrap_or_default()
                        ),
                        observed: written(expr),
                    }
                    .at(expr.span.clone()));
                }
                judge_key(index, &held, object, ctx, source)?;
            }
        }
        ExprKind::Call { callee, args, .. } => {
            if let Some((KeyedBuiltin::Has, held)) = keyed_builtin(callee, args, ctx) {
                if args.len() != 2 {
                    return Err(ExprError::ArgumentCount {
                        callee: HAS_BUILTIN.to_string(),
                        expected: 2,
                        actual: args.len(),
                    }
                    .at(callee.span.clone()));
                }
                judge_key(&args[1], &held, &args[0], ctx, source)?;
            }
        }
        _ => {}
    }
    expr.children()
        .into_iter()
        .try_for_each(|child| reject_misread_keys(child, ctx, source))
}

/// `key`, read at a key of the map or the set `held` that `map` names, judged
/// against the map's key (docs/adr/0108, D1 and D3) as an assignment to a place
/// of that type is judged: a value of the key's own kind for a number, a bool,
/// a text or a byte string, and never a value of an enum there; a variant or a
/// place of the key's enum for an enum key; and, for a record key, the name of
/// a record of the key's schema (a record variable, the item of a `<foreach>`,
/// the payload or a field that holds one), since nothing in an expression makes
/// a record.
fn judge_key(
    key: &TypedExpr,
    held: &crate::forge::value_path::Place,
    map: &TypedExpr,
    ctx: &TypeCtx<'_>,
    source: &str,
) -> Result<(), Refusal> {
    use crate::forge::value_type::ValueType;
    let Some(want) = held.map_key() else {
        return Ok(());
    };
    let written = |node: &TypedExpr| {
        node.span
            .clone()
            .and_then(|span| source.get(span))
            .map(str::to_string)
    };
    let unfit = |why: &str| {
        ExprError::UnsupportedConstruct {
            construct: format!(
                "`{}` as a key of `{}`, whose key is `{}`: {why}",
                written(key).unwrap_or_default(),
                written(map).unwrap_or_default(),
                want.ty
            ),
            observed: written(key),
        }
        .at(key.span.clone())
    };
    let base = ctx.value_scope;
    match &want.ty {
        ValueType::Record { .. } => {
            let given = place_path(key).and_then(|path| ctx.value_of(&path));
            let same = given
                .as_ref()
                .and_then(|given| crate::forge::value_path::record_identity(base, given))
                .zip(crate::forge::value_path::record_identity(base, &want))
                .is_some_and(|(given, wanted)| given == wanted);
            if !same {
                return Err(unfit(
                    "a record key is named whole, by a record of its schema (a record \
                     variable, the item of a <foreach>, the payload or a field that holds one)",
                ));
            }
        }
        ValueType::Scalar(crate::forge::model::SceType::Enum(reference)) => {
            let wanted = base
                .and_then(|base| crate::forge::value_path::scope_at(base, &want.scope))
                .and_then(|scope| scope.enums.get(&reference.alias));
            let fits = match (enum_held_by(key, ctx), wanted) {
                (Some(given), Some(wanted)) => given.holds_the_values_of(wanted),
                (_, None) => true,
                (None, Some(_)) => false,
            };
            if !fits {
                return Err(unfit(&format!(
                    "a key of an enum is a value of it, one of its variants \
                     (`{}.<variant>`) or a place that holds one",
                    reference.alias
                )));
            }
        }
        other => {
            if enum_held_by(key, ctx).is_some() || is_enum_value(key, ctx) {
                return Err(unfit(
                    "a value of an enum is no number, bool, text or byte string \
                     (`<alias>.toNumber(value)` is the number its variant declares)",
                ));
            }
            let slot = InferredType::of_value(other);
            if !slot_admits(slot, key.ty) {
                return Err(type_mismatch(slot, key, source));
            }
        }
    }
    Ok(())
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
/// method — so `ctx.funcs` is the rest of the vocabulary. The eight wrapping
/// and saturating operations (docs/adr/0063) are the vocabulary too, but are
/// never calls past the parser ([`total_operation`]).
const EXPR_BUILTINS: [&str; 4] = ["len", "eq", "round", "floor"];

/// Every name the forge expression vocabulary provides without the document
/// declaring it (docs/SCE_ACCEPTED_SUBSET.md §3.4.1): the four builtins and
/// the eight wrapping and saturating operations. A call of one of them is the
/// vocabulary's, whatever the document declares, so a callable the document
/// declares under one of these names could never be called, and is refused
/// where it is declared ([`crate::forge::namespace`]).
pub(crate) fn vocabulary_names() -> impl Iterator<Item = &'static str> {
    EXPR_BUILTINS
        .into_iter()
        .chain(ring_ops::VOCABULARY.iter().map(|(name, ..)| *name))
}

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
            // A conversion of an imported enum is a member every enum carries
            // without registering it.
            let known = is_enum_conversion(callee, ctx)
                || ctx.lookup_func(&name).is_some()
                || EXPR_BUILTINS.contains(&name.as_str())
                || is_len_builtin(callee, args)
                || keyed_builtin(callee, args, ctx).is_some();
            // `has` asks a map or a set (docs/adr/0108, D3): asked of anything
            // else it names no read, and the refusal says what it reads.
            if !known && name == HAS_BUILTIN {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a call of `{HAS_BUILTIN}` whose first argument names no map or set \
                         variable (`{HAS_BUILTIN}(m, k)` asks the map or the set `m` whether \
                         `k` is one of its keys)"
                    ),
                    observed: Some(HAS_BUILTIN.to_string()),
                }
                .at(callee.span.clone()));
            }
            if !known {
                let mut available: Vec<String> =
                    ctx.funcs.keys().map(|k| (*k).to_string()).collect();
                available.extend(EXPR_BUILTINS.iter().map(|b| (*b).to_string()));
                // The parser has already turned a call of these into the
                // operation it names, so only a misspelling reaches here.
                available.extend(
                    ring_ops::VOCABULARY
                        .iter()
                        .map(|(name, ..)| (*name).to_string()),
                );
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
        ExprKind::Call { callee, args, .. }
            if matches!(callee.kind, ExprKind::Ident(_)) || is_enum_conversion(callee, ctx) =>
        {
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
    // A place a value holds is asked through its type (docs/adr/0086, section
    // 6.2): a record has the fields its schema declares, an optional value its
    // `value`, and any other value no member.
    if let Some(place) = ctx.value_of(&base) {
        return match &place.ty {
            crate::forge::value_type::ValueType::Optional { .. } if member == OPTIONAL_VALUE => {
                Ok(())
            }
            crate::forge::value_type::ValueType::Record { .. } => {
                match crate::forge::value_path::record_fields(ctx.value_scope, &place) {
                    Some(fields) if fields.iter().any(|field| field == member) => Ok(()),
                    Some(mut declared) => {
                        declared.sort_unstable();
                        Err(ExprError::UnknownMember {
                            record: base,
                            member: member.to_string(),
                            declared,
                        })
                    }
                    // A record whose schema the build did not read: nothing
                    // here knows its fields.
                    None => Ok(()),
                }
            }
            // A union has its tag and the fields its cases' records declare
            // (docs/adr/0092, section 6.1); where a case's field is read is
            // judged apart ([`reject_unproven_union_reads`]).
            crate::forge::value_type::ValueType::Union { .. } => {
                match crate::forge::value_path::union_of(ctx.value_scope, &place) {
                    Some((union, _)) => {
                        let mut declared = union_members(union);
                        if declared.iter().any(|field| field == member) {
                            Ok(())
                        } else {
                            declared.sort_unstable();
                            Err(ExprError::UnknownMember {
                                record: base,
                                member: member.to_string(),
                                declared,
                            })
                        }
                    }
                    None => Ok(()),
                }
            }
            other => Err(ExprError::MemberOfNonRecord {
                name: base,
                member: member.to_string(),
                ty: InferredType::of_value(other).declared_spelling(),
            }),
        };
    }
    let Some(&base_ty) = ctx.vars.get(base.as_str()) else {
        return Ok(());
    };
    // An optional value's one member is the read of what it holds, judged
    // where it stands ([`reject_optional_misuse`], docs/adr/0055).
    if base_ty.is_optional() && member == OPTIONAL_VALUE {
        return Ok(());
    }
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

/// Whether `callee` is a conversion of an imported enum: `<alias>.toNumber` or
/// `<alias>.fromNumber` ([`crate::forge::types::EnumScope::conversion`]), a member
/// every imported enum carries and no document declares. A variant of the same
/// name stays a variant where it is read and not called.
pub(crate) fn is_enum_conversion(callee: &TypedExpr, ctx: &TypeCtx<'_>) -> bool {
    let ExprKind::Member { object, property } = &callee.kind else {
        return false;
    };
    let ExprKind::Ident(alias) = &object.kind else {
        return false;
    };
    ctx.lookup_enum(alias)
        .is_some_and(|scope| scope.conversion(property).is_some())
}

/// What turns the answer of a reading that may find nothing — an enum's
/// `fromNumber` function, the variant a number declares or that none does, and a
/// lookup whose miss is an error, the value its table holds for a key or that it
/// holds none, each in the backend's own optional shape — into the shape a call
/// that may fail answers on `target` ([`pass_failure_on`]), when the two differ: a
/// `Result` in Rust, an `AlgorithmResult` in Kotlin and C++, an `Outcome` in Go, a
/// value or a raised failure in Python. `None` for C, whose readings answer the
/// result their own `_take` reads ([`crate::forge::generator::c11_enum_from_number`],
/// [`crate::forge::generator::c11_lookup_found`]), and for the Interpreter, whose
/// library throws the failure itself.
pub(crate) fn found_wrapper(target: ExprTarget) -> Option<&'static str> {
    match target {
        ExprTarget::Rust => Some("sce_forge_runtime::algorithm::found"),
        ExprTarget::Kotlin => Some("com.sce.forge.runtime.SceChecked.found"),
        ExprTarget::Cpp => Some("SCE::Forge::Checked::found"),
        ExprTarget::Go => Some(GO_FOUND),
        ExprTarget::Python => Some("sce_algorithm.found"),
        // The enum's `fromUnderlying` answers an `Optional`, which `Checked.found`
        // throws the failure of as an out-of-range read, for the call that may fail
        // to pass on ([`pass_failure_on`]).
        ExprTarget::Swift => Some("Checked.found"),
        ExprTarget::C | ExprTarget::Js => None,
    }
}

/// The Go runtime's reading of the optional value a reading that may not have
/// found anything answers (`optional.Of`, docs/adr/0055 section 5), as the
/// `Outcome` a call that may fail answers ([`found_wrapper`]): what an enum's
/// `FromUnderlying` and a lookup whose miss is an error both answer.
const GO_FOUND: &str = "scealgorithm.Found";

/// Lower each conversion of an imported enum (`<alias>.toNumber(value)`,
/// `<alias>.fromNumber(number)`) to the functions the context spells the enum's
/// conversions with ([`crate::forge::types::EnumScope::to_number`],
/// [`crate::forge::types::EnumScope::from_number`]): a statechart's own type for
/// the enum, an algorithm's import of the enum document. A `fromNumber` that may
/// fail is wrapped where the backend's optional answer is not the shape a failing
/// call answers ([`found_wrapper`]): the call becomes the wrapper's
/// call of the conversion, which the failure channel then receives as any call
/// that may fail ([`pass_failure_on`]), and the number is still held to the
/// enum's carrier where it is handed over.
///
/// Runs before [`lower_enum_variant_refs`], which would otherwise read the callee
/// as a reference to a variant named `toNumber`.
fn lower_enum_conversions(expr: &mut TypedExpr, ctx: &TypeCtx<'_>, target: ExprTarget) {
    for child in expr.children_mut() {
        lower_enum_conversions(child, ctx, target);
    }
    let ExprKind::Call { callee, fails, .. } = &mut expr.kind else {
        return;
    };
    let (alias, method) = match &callee.kind {
        ExprKind::Member { object, property } => match &object.kind {
            ExprKind::Ident(alias) => (alias.clone(), property.clone()),
            _ => return,
        },
        _ => return,
    };
    let Some(scope) = ctx.lookup_enum(&alias) else {
        return;
    };
    let wrapper = match method.as_str() {
        crate::forge::types::ENUM_TO_NUMBER => {
            callee.kind = ExprKind::Raw(scope.to_number.to_string());
            None
        }
        crate::forge::types::ENUM_FROM_NUMBER => {
            callee.kind = ExprKind::Raw(scope.from_number.to_string());
            found_wrapper(target).filter(|_| *fails)
        }
        _ => None,
    };
    if let Some(wrapper) = wrapper {
        wrap_in_found(expr, wrapper);
    }
}

/// `expr`, a call of a function that answers a value or that it found none, in the
/// backend's own optional shape, as the call of `wrapper` ([`found_wrapper`]) with
/// it: the wrapper's call is the one that may fail, which the failure channel
/// receives as any call that may fail ([`pass_failure_on`]), and the function's own
/// call keeps its arguments and the types they are held to where they are handed
/// over. Anything but a call is left as it is.
fn wrap_in_found(expr: &mut TypedExpr, wrapper: &str) {
    let span = expr.span.clone();
    let ExprKind::Call {
        callee,
        args,
        params,
        fails,
        ..
    } = &mut expr.kind
    else {
        return;
    };
    let reading = TypedExpr {
        kind: ExprKind::Call {
            callee: std::mem::replace(
                callee,
                Box::new(TypedExpr::new(ExprKind::Raw(wrapper.to_string()))),
            ),
            args: std::mem::take(args),
            params: std::mem::take(params),
            fails: false,
            into_destination: false,
            storage: None,
        },
        ty: InferredType::Unknown,
        span,
    };
    *args = vec![reading];
    *params = vec![InferredType::Unknown];
    *fails = true;
}

/// Each call of a function whose arguments or value cross between the caller and
/// the function ([`TypeCtx::crossings`], docs/adr/0051), with them crossed: a value
/// of an enum taken into the type the enum document declares through the function
/// the target names for it, and the value an enum is answered in taken back the same
/// way; a text lent where the target's machine owns its strings and the function
/// borrows one (Rust). Runs before the call is read as its signature says
/// ([`lower_call_readings`]), so a reading that may find nothing is read inside the
/// crossing back: the value crosses once it is found.
/// Each call of `expr` of a function that builds a map local with no map to
/// draw its seed from, handed the seed its caller spells for it as its last
/// argument (docs/adr/0108, D8, [`TypeCtx::seeded_calls`]). Run after the
/// calls are judged, so the argument the author did not write is no argument
/// the judge counts, and before any rename, while each callee is the name the
/// author wrote or the rewrite of a call gave it.
fn append_map_seeds(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    if ctx.seeded_calls.is_empty() {
        return;
    }
    for child in expr.children_mut() {
        append_map_seeds(child, ctx);
    }
    let seed = match &expr.kind {
        ExprKind::Call { callee, .. } => match &callee.kind {
            ExprKind::Ident(name) => ctx.seeded_calls.get(name.as_str()).cloned(),
            _ => None,
        },
        _ => None,
    };
    if let (Some(seed), ExprKind::Call { args, .. }) = (seed, &mut expr.kind) {
        args.push(TypedExpr::new(ExprKind::Raw(seed)));
    }
}

fn lower_call_crossings(expr: &mut TypedExpr, ctx: &TypeCtx<'_>, target: ExprTarget) {
    for child in expr.children_mut() {
        lower_call_crossings(child, ctx, target);
    }
    let crossing = match &expr.kind {
        ExprKind::Call { callee, .. } => match &callee.kind {
            ExprKind::Ident(name) => ctx.crossings.get(name.as_str()).cloned(),
            _ => None,
        },
        _ => None,
    };
    let Some(crossing) = crossing else {
        return;
    };
    if let ExprKind::Call { args, .. } = &mut expr.kind {
        for (arg, slot) in args.iter_mut().zip(&crossing.params) {
            // A crossing keeps the type the value had: the enum's conversion
            // answers the same value under the other document's type, and Rust's
            // `&` lends the text it is given.
            let ty = arg.ty;
            match slot {
                crate::forge::types::SlotCrossing::AsIs => {}
                crate::forge::types::SlotCrossing::Enum(function) => {
                    wrap_in_call(arg, function, ty)
                }
                crate::forge::types::SlotCrossing::Text => {
                    if target == ExprTarget::Rust {
                        wrap_in_call(arg, "&", ty);
                    }
                }
            }
        }
    }
    if let Some(function) = crossing.ret {
        let ty = expr.ty;
        wrap_in_call(expr, function, ty);
    }
}

/// The field of a validator's `ValidationResult` that says whether its inputs are
/// valid, as `target` spells it: `Valid` on Go, whose exported fields are
/// capitalised, and `valid` everywhere else — the field each backend's validator
/// declares.
pub(crate) fn validation_result_valid(target: ExprTarget) -> &'static str {
    match target {
        ExprTarget::Go => "Valid",
        ExprTarget::Rust
        | ExprTarget::Kotlin
        | ExprTarget::Cpp
        | ExprTarget::Python
        | ExprTarget::C
        | ExprTarget::Swift
        | ExprTarget::Js => "valid",
    }
}

/// Read each call whose function answers something other than the value of the
/// call as its signature says ([`CallReading`], docs/adr/0048), before the callee is
/// renamed to the function a backend calls:
///
/// - a lookup whose miss is an error ([`CallReading::Found`]) answers the value or
///   that it found none, in the backend's optional shape, and is read through the
///   runtime's `found` where the call's failure is received — its failure is a key
///   the table does not hold — as the variant a number declares is
///   ([`lower_enum_conversions`]). C's reading is the `<function>_found` family the
///   machine declares beside the call, which answers the result its `_take` reads
///   ([`crate::forge::generator::c11_lookup_found`]); the callee renames to it.
/// - a validator's check ([`CallReading::Validity`]) answers its `ValidationResult`,
///   and the call's value is the result's validity ([`validation_result_valid`]).
fn lower_call_readings(expr: &mut TypedExpr, ctx: &TypeCtx<'_>, target: ExprTarget) {
    for child in expr.children_mut() {
        lower_call_readings(child, ctx, target);
    }
    let (reading, fails) = match &expr.kind {
        ExprKind::Call { callee, fails, .. } => match &callee.kind {
            ExprKind::Ident(name) => match ctx.lookup_func(name) {
                Some(signature) => (signature.reading, *fails),
                None => return,
            },
            _ => return,
        },
        _ => return,
    };
    match reading {
        // Spelled by the emitters of the backends that hand a destination in
        // (`ExprKind::Call::into_destination`), and the list itself elsewhere.
        CallReading::Value | CallReading::IntoDestination => {}
        CallReading::Found => {
            if let Some(wrapper) = found_wrapper(target).filter(|_| fails) {
                wrap_in_found(expr, wrapper);
            }
        }
        CallReading::Validity => {
            let span = expr.span.clone();
            let check = std::mem::replace(&mut expr.kind, ExprKind::Raw(String::new()));
            expr.kind = ExprKind::Member {
                object: Box::new(TypedExpr {
                    kind: check,
                    ty: InferredType::Unknown,
                    span,
                }),
                property: validation_result_valid(target).to_string(),
            };
        }
    }
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
///
/// A target with no `Language` ([`ExprTarget::language`]) has no declaration
/// for the variant to be spelled against, and its data model holds no types:
/// the Interpreter's enum value is the variant's declared name, a string, which
/// is also the form a saved state holds it in, so a reference is that string
/// rather than the author's `Alias.variant`, which names nothing in the script
/// engine.
fn lower_enum_variant_refs(
    expr: &mut TypedExpr,
    ctx: &TypeCtx<'_>,
    target: ExprTarget,
) -> Result<(), Refusal> {
    if let ExprKind::Member { object, property } = &expr.kind {
        if let ExprKind::Ident(alias) = &object.kind {
            if let Some(scope) = ctx.lookup_enum(alias) {
                expr.kind = match target.language() {
                    Some(language) => ExprKind::Raw(crate::forge::enum_naming::variant_ref(
                        language,
                        scope.qualified_type,
                        scope.source_name,
                        property,
                    )),
                    None => ExprKind::StringLit {
                        value: property.clone(),
                    },
                };
                return Ok(());
            }
        }
    }
    for child in expr_children_mut(expr) {
        lower_enum_variant_refs(child, ctx, target)?;
    }
    Ok(())
}

/// A string concatenation of operands the data model sizes, as the call that
/// writes the joined text into a buffer of exactly their sum — for C11, whose
/// string is a bounded buffer and has nowhere to put the text of `wait + 'ms'`.
///
/// Every operand of a `sce-static` statechart's expression is sized by the
/// document: a string variable by the capacity it declares, a literal by its
/// text, an integer by the digits of its type (the only operands a
/// concatenation carries, [`concat_operand`]). The sum of those bounds, and one
/// byte for the terminator, is a buffer that holds every text the expression can
/// compute, so the join is a compound literal of that size which
/// `SCE_FORGE_CONCAT` writes into: an expression that needs no statement and
/// whose storage lasts as long as the block that reads it. A chain `a + b + c`
/// is one join, not a join of a join.
///
/// Left as written, for the emitter to refuse by name, when an operand has no
/// declared size, and everywhere a context does not read the joined text where
/// it computes it ([`TypeCtx::joins_into_buffers`]): a forge kind's expression
/// may return the text or keep it, and a buffer sized for one evaluation does
/// not outlive the block that made it.
fn lower_string_concatenation(expr: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if is_string_concatenation(*op, left.ty, right.ty) {
            let mut spine = Vec::new();
            concat_spine(expr, &mut spine);
            let mut parts: Vec<TypedExpr> = spine.into_iter().cloned().collect();
            let bounds: Option<Vec<usize>> = parts
                .iter()
                .map(|part| concat_part_bound(part, ctx))
                .collect();
            if let Some(bounds) = bounds {
                for part in &mut parts {
                    lower_string_concatenation(part, ctx);
                }
                let capacity = bounds.iter().sum::<usize>() + 1;
                let raw = |text: String| TypedExpr::new(ExprKind::Raw(text));
                let wire_value = |part: TypedExpr| {
                    let constructor = match part.ty {
                        InferredType::Int { signed: false, .. } => "sce_forge_wire_uint",
                        InferredType::Int { .. } | InferredType::UntypedInt => "sce_forge_wire_int",
                        _ => "sce_forge_wire_string",
                    };
                    TypedExpr::new(ExprKind::Call {
                        callee: Box::new(raw(constructor.to_string())),
                        args: vec![part],
                        params: Vec::new(),
                        fails: false,
                        into_destination: false,
                        storage: None,
                    })
                };
                let mut args = vec![
                    raw(format!("(char[{capacity}]){{0}}")),
                    raw(capacity.to_string()),
                ];
                args.extend(parts.into_iter().map(wire_value));
                expr.kind = ExprKind::Call {
                    callee: Box::new(raw("SCE_FORGE_CONCAT".to_string())),
                    args,
                    params: Vec::new(),
                    fails: false,
                    into_destination: false,
                    storage: None,
                };
                return;
            }
        }
    }
    for child in expr_children_mut(expr) {
        lower_string_concatenation(child, ctx);
    }
}

/// An operation on 32-bit reals, rounded to binary32 where it is made, for the
/// targets that hold a real in a double (Python's `float`, ECMAScript's
/// `Number`): the languages that have a single of their own — Rust, Go, Kotlin,
/// C and C++ — compute it as one, and a document must mean one thing on every
/// engine.
///
/// An arithmetic operation is made at the precision of the place its value
/// lands: the single a `float32` slot holds, the double of a `float64` one — an
/// operand narrower than that is widened first, which is exact — and, where no
/// real is expected (a comparison's operand, a condition), at the width of the
/// wider operand, a literal taking its partner's. This is how every emitter
/// pushes a real context down through arithmetic, so the languages with a
/// single compute exactly this, and the two that hold every real in a double
/// have to round to a single where the precision is one.
///
/// `a + b` made as a single is the single nearest to the exact sum. Computed
/// in a double it is the same number, because a double holds the exact sum of
/// two singles' worth of bits with room to spare, so rounding the double result
/// to a single once gives the single nearest to the exact sum — but only if each
/// operand was a single first. A literal is the double nearest to what is
/// written until it is made one, a 64-bit real is wider, and an integer may be
/// more than a single holds, so each of them is rounded to a single where it
/// meets an operation made as one, and the result of the operation is rounded
/// again. A comparison over singles compares the singles, so its operands are
/// rounded and its result is not.
///
/// The same is done where a value flows into a slot that is a 32-bit real
/// (`expected`): a literal, a 64-bit real or an integer is rounded to the single
/// it becomes there.
fn round_to_single(expr: &mut TypedExpr, target: ExprTarget, expected: InferredType) {
    let callee = match target {
        ExprTarget::Python => "sce_algorithm.to_f32",
        ExprTarget::Js => "Math.fround",
        _ => return,
    };
    round_singles_in(expr, expected, callee);
    if matches!(expected, InferredType::Float { bits: 32 })
        && !matches!(expr.ty, InferredType::Float { bits: 32 })
        && matches!(
            expr.ty,
            InferredType::Float { .. }
                | InferredType::UntypedFloat
                | InferredType::UntypedInt
                | InferredType::Int { .. }
        )
    {
        wrap_in_call(expr, callee, InferredType::Float { bits: 32 });
    }
}

/// A conditional that lands in a real slot is made at the slot's precision.
///
/// ⚠ The accepted subset says an operation is made at the precision of the place its value
/// lands (docs/SCE_ACCEPTED_SUBSET.md, "A 32-bit real"): a double in a `float64` slot, a
/// single in a `float32` one. An arithmetic node already took the slot's type from the context
/// each emitter pushes into it. A conditional took its own, the join of its branches, so
/// `c ? n * 0.1 : 0.0` over a `uint32` `n` was a `float`, and its value reached a `float64` slot
/// as `-12.300000190734863` where the double is `-12.3` (measured 2026-10-06: four cases of one
/// component, on every language that holds a single of its own). Its branches take the slot's
/// type here, the way an operand of arithmetic does, so every emitter reads the same tree.
fn land_reals_in_slot(expr: &mut TypedExpr, expected: InferredType) {
    if !matches!(expected, InferredType::Float { .. }) {
        return;
    }
    let real =
        |ty: InferredType| matches!(ty, InferredType::Float { .. } | InferredType::UntypedFloat);
    match &mut expr.kind {
        ExprKind::Conditional {
            consequent,
            alternate,
            ..
        } if real(expr.ty) => {
            expr.ty = expected;
            land_reals_in_slot(consequent, expected);
            land_reals_in_slot(alternate, expected);
        }
        // A negation is exact in any width, and an operand of arithmetic is made where the
        // operation is.
        ExprKind::Unary {
            op: UnaryOp::Neg | UnaryOp::Pos,
            operand,
        } => land_reals_in_slot(operand, expected),
        ExprKind::Binary { op, left, right }
            if op.is_arith() && !is_string_concatenation(*op, left.ty, right.ty) =>
        {
            land_reals_in_slot(left, expected);
            land_reals_in_slot(right, expected);
        }
        _ => {}
    }
}

/// `expected` is the type the context pushes into `expr`: a real where the
/// value lands in one, and anything else where it does not.
fn round_singles_in(expr: &mut TypedExpr, expected: InferredType, callee: &str) {
    let single = InferredType::Float { bits: 32 };
    let real = |ty: InferredType| {
        if matches!(ty, InferredType::Float { .. }) {
            ty
        } else {
            InferredType::Unknown
        }
    };
    match &mut expr.kind {
        ExprKind::Binary { op, left, right } => {
            let arithmetic = op.is_arith() && !is_string_concatenation(*op, left.ty, right.ty);
            let operands = binary_operand_type(*op, left.ty, right.ty);
            if arithmetic {
                // At the precision of the place the value lands, or at the
                // operands' own where it lands in no real.
                let at = if matches!(expected, InferredType::Float { .. }) {
                    expected
                } else {
                    operands
                };
                round_singles_in(left, real(at), callee);
                round_singles_in(right, real(at), callee);
                if at == single {
                    for side in [&mut **left, &mut **right] {
                        if side.ty != single {
                            wrap_in_call(side, callee, single);
                        }
                    }
                    wrap_in_call(expr, callee, single);
                }
            } else if op.is_comparison() {
                round_singles_in(left, real(operands), callee);
                round_singles_in(right, real(operands), callee);
                if operands == single {
                    for side in [&mut **left, &mut **right] {
                        if side.ty != single {
                            wrap_in_call(side, callee, single);
                        }
                    }
                }
            } else {
                round_singles_in(left, InferredType::Unknown, callee);
                round_singles_in(right, InferredType::Unknown, callee);
            }
        }
        // A negation is exact in any width: the precision passes through it.
        ExprKind::Unary {
            op: UnaryOp::Neg | UnaryOp::Pos,
            operand,
        } => round_singles_in(operand, real(expected), callee),
        // The branches of a conditional are made where the conditional lands; its condition
        // lands in no real.
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            round_singles_in(condition, InferredType::Unknown, callee);
            round_singles_in(consequent, real(expected), callee);
            round_singles_in(alternate, real(expected), callee);
        }
        _ => {
            for child in expr_children_mut(expr) {
                round_singles_in(child, InferredType::Unknown, callee);
            }
        }
    }
}

/// `node` as the argument of a call of `callee`, typed `ty`.
fn wrap_in_call(node: &mut TypedExpr, callee: &str, ty: InferredType) {
    let inner = std::mem::replace(node, TypedExpr::new(ExprKind::Raw(String::new())));
    *node = TypedExpr {
        kind: ExprKind::Call {
            callee: Box::new(TypedExpr::new(ExprKind::Raw(callee.to_string()))),
            args: vec![inner],
            params: Vec::new(),
            fails: false,
            into_destination: false,
            storage: None,
        },
        ty,
        span: None,
    };
}

/// The operands of the string concatenation `expr` stands for, left to right:
/// the chain `a + b + c` is read along its left spine, so a sum of numbers that
/// begins it (`1 + 2 + 'x'`, which is `'3x'`) is the first operand.
fn concat_spine<'e>(expr: &'e TypedExpr, into: &mut Vec<&'e TypedExpr>) {
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if is_string_concatenation(*op, left.ty, right.ty) {
            concat_spine(left, into);
            into.push(right);
            return;
        }
    }
    into.push(expr);
}

/// The most bytes the text of one operand of a concatenation can take, or `None`
/// when the operand is one a concatenation does not carry ([`concat_operand`])
/// or the data model declares no size for it: the bound of its text, by the one
/// analysis of text bounds ([`crate::forge::text_bound`]).
fn concat_part_bound(part: &TypedExpr, ctx: &TypeCtx<'_>) -> Option<usize> {
    match concat_operand(part.ty) {
        ConcatOperand::Text | ConcatOperand::Decimal => {
            crate::forge::text_bound::text_bound(part, ctx)
                .bounded()
                .and_then(|bytes| usize::try_from(bytes).ok())
        }
        ConcatOperand::Refused(_) => None,
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

/// The parameters of the `eq(a, b)` builtin: two byte sequences.
/// `docs/SCE_ACCEPTED_SUBSET.md` §3.4.1 names it SCE's bytes comparison.
const BYTES_EQ_PARAMS: [InferredType; 2] = [InferredType::Bytes, InferredType::Bytes];

/// Whether `callee` is the `eq` builtin — not a function the document
/// provides under that name, an import aliased `eq`, which is the
/// document's to type and to call.
///
/// ⚠ `eq` was listed as a builtin and lowered by no emitter until
/// 2026-09-24: `expr="eq(frame, expect)"` generated on all six backends as
/// a call to an `eq` none of them has, so the target compiler was the one
/// to refuse it. It now has a signature, so its arguments are judged
/// ([`reject_call_argument_mismatches`]), and it is lowered to the bytes
/// comparison every emitter already renders ([`lower_bytes_eq`]).
fn is_bytes_eq_builtin(callee: &TypedExpr, ctx: &TypeCtx<'_>) -> bool {
    matches!(&callee.kind, ExprKind::Ident(name) if name == "eq") && ctx.lookup_func("eq").is_none()
}

/// Every call of the `eq` builtin rewritten to the `===` of its two
/// arguments, then typed again: a comparison is where a string literal
/// against bytes becomes the bytes it spells ([`infer_types`]), and that
/// rewrite has to see the comparison, not the call.
///
/// Runs after the arguments are judged, so a call it meets has the two
/// arguments its signature declares.
fn lower_bytes_eq(ast: &mut TypedExpr, ctx: &TypeCtx<'_>) {
    for child in expr_children_mut(ast) {
        lower_bytes_eq(child, ctx);
    }
    let ExprKind::Call { callee, args, .. } = &mut ast.kind else {
        return;
    };
    if !is_bytes_eq_builtin(callee, ctx) || args.len() != BYTES_EQ_PARAMS.len() {
        return;
    }
    let right = args.pop().expect("two arguments");
    let left = args.pop().expect("two arguments");
    ast.kind = ExprKind::Binary {
        op: BinOp::StrictEq,
        left: Box::new(left),
        right: Box::new(right),
    };
    infer_types(ast, ctx);
}

/// A byte string is held by a statechart's data model, which holds no types, as
/// the text of its bytes, each the character of that code point (docs/adr/0005,
/// decision 2) — the spelling it takes on the wire. So each constant is the
/// string literal of that text, which the emitter escapes as it does any other.
/// An algorithm lowered for the Interpreter holds bytes as an array of numbers
/// instead (`exact_integers` is how the two are told apart), and keeps refusing a
/// constant.
fn bytes_literals_as_text(expr: &mut TypedExpr) {
    for child in expr_children_mut(expr) {
        bytes_literals_as_text(child);
    }
    if let ExprKind::BytesLit { bytes } = &expr.kind {
        expr.kind = ExprKind::StringLit {
            value: latin1(bytes),
        };
    }
}

/// Where a failure can be received — an algorithm's body, a `sce-static`
/// machine's statements — a byte string is held as the `{data, len}` struct C
/// gives a view or a buffer, and not as the array and `_len` sibling of an event
/// payload's field. So each side of a bytes equality that is not a constant is
/// given to the C emitter as the pointer and the length it is made of: a
/// [`ExprKind::BytesView`] of `<held>.data` and `<held>.len`, whichever the
/// expression `<held>` is. A guard over a payload receives no failure, keeps its
/// operand as it was, and is read as the sibling it names.
fn views_for_bytes_equality(expr: &mut TypedExpr) {
    for child in expr_children_mut(expr) {
        views_for_bytes_equality(child);
    }
    let ExprKind::Binary {
        op: BinOp::StrictEq | BinOp::StrictNeq,
        left,
        right,
    } = &mut expr.kind
    else {
        return;
    };
    if !(matches!(left.ty, InferredType::Bytes) || matches!(right.ty, InferredType::Bytes)) {
        return;
    }
    for side in [left, right] {
        if matches!(
            side.kind,
            ExprKind::BytesLit { .. } | ExprKind::BytesView { .. }
        ) {
            continue;
        }
        let held = (**side).clone();
        let part = |property: &str| {
            Box::new(TypedExpr::new(ExprKind::Member {
                object: Box::new(held.clone()),
                property: property.to_string(),
            }))
        };
        **side = TypedExpr {
            kind: ExprKind::BytesView {
                source: part("data"),
                len: Some(part("len")),
            },
            ty: InferredType::Bytes,
            span: held.span.clone(),
        };
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
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
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
    // Push-down: `len(x)` is a `size_t`, so against a signed integer it is a sign
    // comparison the build refuses under `-Werror` (`i < len(xs)` with `i: int64`).
    // In a signed context the length is cast to that width where the context is
    // known, as the Rust, Go and C pushes-down of the same builtin do.
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if is_len_builtin(callee, args) && matches!(expr.ty, InferredType::UntypedInt) {
            if let InferredType::Int { signed: true, bits } = expected {
                let size = format!("({}).size()", emit_cpp(&args[0], InferredType::Unknown)?);
                return Ok(format!("static_cast<std::int{bits}_t>({size})"));
            }
        }
    }
    // `null` where an optional value is expected is the absent one
    // (docs/adr/0055), which `std::optional` is built from and compared with.
    if matches!(expr.kind, ExprKind::NullLit) && expected.is_optional() {
        return Ok("std::nullopt".to_string());
    }
    let raw = match typed_integer_literal(ExprTarget::Cpp, expr) {
        Some(literal) => literal,
        None => cpp_emit_node(expr)?,
    };
    Ok(cpp_coerce(raw, expr.ty, expected, expr))
}

/// The `std::optional` a value of `elem` is held in, for a place that has to
/// name it — a branch of a conditional whose other branch is `null`, where C++
/// finds no type the two share. A number, a `bool`, a text and a byte string
/// name their own; an enum or a record is named by `present`, the branch that
/// holds a present value, whose type the emitter does not spell.
fn cpp_optional_type(elem: OptionalElem, present: &str) -> String {
    use crate::forge::quantity::NumericBaseType;
    let held = match elem {
        OptionalElem::Number(NumericBaseType::Int { signed, bits }) => {
            format!("std::{}int{bits}_t", if signed { "" } else { "u" })
        }
        OptionalElem::Number(NumericBaseType::Float { bits }) => {
            c_family_float_name(InferredType::Float { bits }).to_string()
        }
        OptionalElem::Bool => "bool".to_string(),
        OptionalElem::Str => "std::string".to_string(),
        OptionalElem::Bytes => "std::vector<uint8_t>".to_string(),
        OptionalElem::Enum | OptionalElem::Record | OptionalElem::List(_) => {
            format!("std::decay_t<decltype({present})>")
        }
    };
    format!("std::optional<{held}>")
}

/// The C++ spelling of each construct of an optional value (docs/adr/0055,
/// section 5) over `std::optional<T>` and the runtime's `sce/forge/optional.h`:
///
/// - a test: `x.has_value()`, or its negation;
/// - a read the build has proven present: `(*x)`, which has no failure to record;
/// - `x ?? d`: `SCE::Forge::otherwise(x, [&] { return d; })`, which reads `x`
///   once and calls the lambda only when `x` is absent, so a failure `d`
///   records in the statement's `sce_failure_` is recorded only then;
/// - `null === null`: its answer.
///
/// `None` for equality: `x === y` of two optional values, or of one and a value
/// of `T`, is the `==` C++ defines over `std::optional`, which lifts the side of
/// `T` itself, and the generic arm writes it with the operands' parentheses.
fn cpp_optional(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<Option<String>, ExprError> {
    let receiver = |node: &TypedExpr| -> Result<String, ExprError> {
        Ok(wrap_postfix(node, emit_cpp(node, InferredType::Unknown)?))
    };
    Ok(Some(match form {
        OptionalForm::Test { subject, present } => {
            let test = format!("{}.has_value()", receiver(subject)?);
            if present {
                test
            } else {
                format!("!{test}")
            }
        }
        OptionalForm::Proven { subject } => format!("(*{})", receiver(subject)?),
        OptionalForm::Otherwise { value, fallback } => format!(
            "SCE::Forge::otherwise({}, [&] {{ return {}; }})",
            emit_cpp(value, InferredType::Unknown)?,
            emit_cpp(fallback, whole.ty)?
        ),
        OptionalForm::Equality { .. } => return Ok(None),
        OptionalForm::Constant(answer) => answer.to_string(),
    }))
}

/// A conditional of an optional value one of whose branches is `null` and the
/// other a value of `T` (docs/adr/0055, section 2), in C++: both branches as
/// the `std::optional` of the present one ([`cpp_optional_type`]), since C++
/// finds no type `std::nullopt` and a `T` share. `None` for any other node; a
/// conditional of an optional value and `null`, or of an optional value and a
/// value of `T`, has the optional value's type, which the other branch converts
/// to as it is.
fn cpp_optional_conditional(expr: &TypedExpr) -> Result<Option<String>, ExprError> {
    let (
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        },
        InferredType::Optional(elem),
    ) = (&expr.kind, expr.ty)
    else {
        return Ok(None);
    };
    let (absent_first, present) = match (&consequent.kind, &alternate.kind) {
        (ExprKind::NullLit, _) if !alternate.ty.is_optional() => (true, alternate),
        (_, ExprKind::NullLit) if !consequent.ty.is_optional() => (false, consequent),
        _ => return Ok(None),
    };
    let value = emit_cpp(present, elem.present())?;
    let ty = cpp_optional_type(elem, &value);
    let cond = emit_cpp(condition, InferredType::Bool)?;
    let cond = if matches!(&condition.kind, ExprKind::Conditional { .. }) {
        format!("({cond})")
    } else {
        cond
    };
    let (absent, held) = (format!("{ty}()"), format!("{ty}({value})"));
    Ok(Some(if absent_first {
        format!("{cond} ? {absent} : {held}")
    } else {
        format!("{cond} ? {held} : {absent}")
    }))
}

fn cpp_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    if let Some(form) = optional_form(expr) {
        if let Some(spelled) = cpp_optional(form, expr)? {
            return Ok(spelled);
        }
    }
    if let Some(spelled) = cpp_optional_conditional(expr)? {
        return Ok(spelled);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's function for the operation's own
        // width. The width is explicit, as the checked one's is, so an operand
        // converts to it rather than choosing the instantiation.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "C++", emit_cpp)?;
            format!(
                "SCE::Forge::{}::{}<std::{}int{bits}_t>({})",
                crate::filters::capitalize_first(rule.helper()),
                op.helper(),
                if signed { "" } else { "u" },
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => portable_number_literal(n),
        // A literal while one holds the text, a `std::string` of its bytes
        // where it is too long for one or holds a U+0000 a literal would end
        // at.
        ExprKind::StringLit { value } => crate::filters::cpp_text_expression(value),
        // `std::vector<uint8_t>` has `operator==`, so the default
        // `==`/`!=` path compares length+content directly against this
        // value-constructed temporary. No byte-string literal in C++, so
        // a hex initializer list. RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => {
            format!("std::vector<uint8_t>{{{}}}", bytes_as_hex_list(bytes))
        }
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "nullptr".to_string(),
        // The one spelling every C++ local takes, declaration and read alike
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::Cpp.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // `"E" + n` is pointer arithmetic in C++, so each side is made a
            // `std::string` first: a string is wrapped (a literal is a
            // `const char *`), an integer becomes its decimal digits. The
            // left operand being a `std::string` is what makes every later
            // `+` in a chain a concatenation (ECMA-262 13.15.3).
            let operand = |side: &TypedExpr| -> Result<String, ExprError> {
                match concat_operand(side.ty) {
                    ConcatOperand::Text => Ok(format!("std::string({})", emit_cpp(side, side.ty)?)),
                    ConcatOperand::Decimal => {
                        Ok(format!("std::to_string({})", emit_cpp(side, side.ty)?))
                    }
                    ConcatOperand::Refused(what) => {
                        Err(concat_refusal(what, emit_cpp(side, side.ty)?))
                    }
                }
            };
            format!("{} + {}", operand(left)?, operand(right)?)
        }
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_cpp(left, operand_ty)?;
            let r_raw = emit_cpp(right, right_operand_type(*op, operand_ty))?;
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
            c_family_binary(CFamily::Cpp, *op, expr.ty, &l, &r)
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_cpp(operand, expr.ty)?;
            c_family_unary(CFamily::Cpp, *op, expr.ty, operand, &inner)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            // The condition of a conditional that is itself a conditional is bracketed:
            // `g ? a : b ? 1 : 0` reads `g ? a : (b ? 1 : 0)`. The consequent and the
            // alternate keep the spelling they have, which reads as written.
            let cond = emit_cpp(condition, InferredType::Bool)?;
            let cond = if matches!(&condition.kind, ExprKind::Conditional { .. }) {
                format!("({cond})")
            } else {
                cond
            };
            format!(
                "{cond} ? {} : {}",
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
            fails,
            ..
        } => {
            if is_len_builtin(callee, args) {
                let size = format!("({}).size()", emit_cpp(&args[0], InferredType::Unknown)?);
                // A length typed as an integer width (`TypeCtx::lengths_are_uint32`)
                // is that width: the cast is spelled here, where `size_t` meets it.
                return Ok(match expr.ty {
                    InferredType::Int { signed, bits } => format!(
                        "static_cast<std::{}int{bits}_t>({size})",
                        if signed { "" } else { "u" }
                    ),
                    _ => size,
                });
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
            let symbol = wrap_postfix(callee, emit_cpp(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Cpp, &symbol, &call, expr.ty)?
            } else {
                call
            }
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: a bounded-string field projected to
            // the algorithm `bytes` param type — `SCE::Forge::Span<const
            // std::uint8_t>` over the string's data()/size().
            let s = emit_cpp(source, InferredType::Unknown)?;
            format!(
                "{CPP_SPAN}<const std::uint8_t>(reinterpret_cast<const std::uint8_t*>({s}.data()), {s}.size())"
            )
        }
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => cpp_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: an index read that records an out-of-range
        // failure instead of reading past the end.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust()),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => format!(
            "SCE::Forge::Checked::at(sce_failure_, {}, {})",
            emit_cpp(left, InferredType::Unknown)?,
            emit_cpp(index, InferredType::Unknown)?,
        ),
        // SCE_FORGE.md §3.4.1: the runtime's helper at the operation's own
        // width, which records a failure in the body's `sce_failure_` and
        // yields 0; the statement around it returns that failure. The width
        // is explicit so an operand of another type converts to it rather
        // than choosing the instantiation.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                });
            };
            if *op == CheckedOp::Narrow {
                // The value at its own type; the helper names both.
                let InferredType::Int {
                    signed: from_signed,
                    bits: from_bits,
                } = left.ty
                else {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("a checked narrowing from {:?}", left.ty),
                        observed: None,
                    });
                };
                return Ok(format!(
                    "SCE::Forge::Checked::narrow<std::{}int{bits}_t, std::{}int{from_bits}_t>(sce_failure_, {})",
                    if signed { "" } else { "u" },
                    if from_signed { "" } else { "u" },
                    emit_cpp(left, left.ty)?
                ));
            }
            let mut operands = vec!["sce_failure_".to_string(), emit_cpp(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit_cpp(right, expr.ty)?);
            }
            format!(
                "SCE::Forge::Checked::{}<std::{}int{bits}_t>({})",
                op.helper(),
                if signed { "" } else { "u" },
                operands.join(", ")
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in C++: the runtime's total
/// shift (`SCE::Forge::Wrapping::shl<std::uint32_t>(x, n)`), which is the exact
/// result reduced to the width for every count at least 0, or its checked one,
/// which records an out-of-range failure in the body's `sce_failure_` for a
/// count below 0 and yields 0. The language's own operator is undefined for a
/// count at or past the width. The width is explicit, as a checked operation's
/// is, so a literal operand converts to it; the count is taken at its own type.
fn cpp_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<String, ExprError> {
    let (signed, bits) = settled_shift_width(expr)?;
    let ty = format!("std::{}int{bits}_t", if signed { "" } else { "u" });
    let value = emit_cpp(left, expr.ty)?;
    let places = emit_cpp(count, InferredType::Unknown)?;
    Ok(match discharge {
        Discharge::AtBuild | Discharge::Total => format!(
            "SCE::Forge::Wrapping::{}<{ty}>({value}, {places})",
            op.helper()
        ),
        Discharge::AtRun => format!(
            "SCE::Forge::Checked::{}<{ty}>(sce_failure_, {value}, {places})",
            op.helper()
        ),
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
    // A value of `T` where an optional value of `T` is expected (docs/adr/0055,
    // section 2) is lifted by `std::optional`'s own conversion, and spelled as
    // the value of its element it is: a wide literal or an unsigned value meets
    // the element's type. Asked before the early return below: an enum's
    // variant is a value this layer does not type. An optional value, or `null`,
    // where one is expected is written as it is.
    if let Some(elem) = from.lifted_into(to) {
        return cpp_coerce(raw, from, elem.present(), node);
    }
    if to.is_optional() {
        return raw;
    }
    if let Some(spelled) = c_family_wide_literal(&raw, to, node) {
        return spelled;
    }
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return raw;
    }
    // Promote untyped decimal integer literals in float context to prevent
    // C++ integer division: `9 / 5` → `9.0 / 5.0`.
    if let (UntypedInt, Float { bits }) = (from, to) {
        if let ExprKind::NumberLit(text) = &node.kind {
            if is_decimal_integer_literal(text) {
                return format!("{raw}.0{}", single_suffix(bits));
            }
        }
    }
    // A real literal that lands in a 32-bit real is the single nearest to it:
    // `0.2f`, and not the double `0.2`, which would make the sum around it a
    // double's and round it to a single only once it is stored.
    if let (UntypedFloat, Float { bits: 32 }) = (from, to) {
        if matches!(node.kind, ExprKind::NumberLit(_)) {
            return format!("{raw}f");
        }
    }
    // An unsigned operand where the join of the two is signed.
    if let Some(bits) = c_family_unsigned_to_signed(from, to) {
        return format!("static_cast<std::int{bits}_t>({raw})");
    }
    raw
}

/// The width of the signed type an unsigned operand is converted to where the join
/// of it and another operand is signed (`join_arith`), or `None` when `from` and
/// `to` are not that pair. The usual arithmetic conversions of C and C++ would do
/// the opposite — turn the signed operand unsigned, so `cursor < len` with a
/// negative `cursor` compares as a huge number — and `-Wsign-compare` refuses the
/// mix outright; the language's rule is the join's, so the cast is spelled.
///
/// Only where the unsigned operand is at least as wide as the signed type: a
/// narrower one (`uint8` into an `int32`) is promoted to it by C itself, and a
/// cast there would only change what an algorithm's output already says.
fn c_family_unsigned_to_signed(from: InferredType, to: InferredType) -> Option<u8> {
    match (from, to) {
        (
            InferredType::Int {
                signed: false,
                bits: from_bits,
            },
            InferredType::Int { signed: true, bits },
        ) if from_bits >= bits => Some(bits),
        _ => None,
    }
}

/// The suffix of a C-family real literal of `bits` bits: `f` for a single.
fn single_suffix(bits: u8) -> &'static str {
    if bits == 32 {
        "f"
    } else {
        ""
    }
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
        BinOp::Coalesce => unreachable!("{COALESCE_NOT_SPELLED}"),
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

/// The `SceListBuf` reader that returns a list buffer's element as the type
/// it was appended as. The buffer keeps every element in one 64-bit slot,
/// so the reader is chosen by the element, not by an overload on the
/// buffer. An index reads an element through it, and so does a loop over the
/// buffer (`forge::generator`).
pub(crate) fn kotlin_buffer_reader(
    elem: crate::forge::types::ListElem,
) -> Result<&'static str, ExprError> {
    Ok(match elem.element_type() {
        InferredType::Int { signed, bits } => match (signed, bits) {
            (true, 8) => "getByte",
            (true, 16) => "getShort",
            (true, 32) => "getInt",
            (true, _) => "getLong",
            (false, 8) => "getUByte",
            (false, 16) => "getUShort",
            (false, 32) => "getUInt",
            (false, _) => "getULong",
        },
        InferredType::Float { bits: 32 } => "getFloat",
        InferredType::Float { .. } => "getDouble",
        InferredType::Bool => "getBoolean",
        // A buffer holds a fixed-width number or `bool` in its 64-bit slot; a
        // buffer of records is an `ArrayList` whose element is never read by
        // index (refused where the index is judged), and no other element is
        // held by one.
        _ => {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!("an index into a list of {}", elem.value()),
                observed: None,
            })
        }
    })
}

fn emit_kotlin(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // A value of `T` where an `optional<T>` is expected is lifted
    // (docs/adr/0055, section 2, [`InferredType::lifted_into`]). Kotlin's `T?`
    // takes a `T` as it is, so it is spelled as the value of `T` it is, with
    // every conversion and push-down a place of `T` makes, and an untyped
    // integer in `T` (`7L`): `Long?` takes no `Int`, and a branch of a
    // conditional beside `null` is typed by nothing else. Asked first: an
    // enum's variant is a value this layer does not type, and is lifted too.
    if let Some(elem) = expr.ty.lifted_into(expected) {
        let present = elem.present();
        return Ok(kotlin_equality_operand(
            emit_kotlin(expr, present)?,
            expr,
            present,
        ));
    }
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
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
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
        // Kotlin's narrow unsigned types (UByte/UShort) define no operator:
        // the operation runs in the type `kotlin_narrow_unsigned_operation`
        // names, and the result is coerced from it to the caller-requested
        // type. The outer kotlin_coerce handles the reverse conversion via
        // `.toUByte()` / `.toUShort()`.
        if let Some(widened) = kotlin_narrow_unsigned_operation(expr) {
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
            // A left shift made in `Int` wraps at the width of what it shifts
            // (SCE_FORGE.md §3.4.1, "The width of a shift"), as it does on
            // every backend: converted to its own type first, and only then
            // to the type the caller asks for, so `x shl 4` of a `UByte`
            // keeps 8 bits where it lands in a `UInt`.
            if kotlin_wraps_at_its_width(expr) {
                let own = kotlin_coerce(inner, widened, expr.ty, expr);
                return Ok(kotlin_coerce(own, expr.ty, expected, expr));
            }
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
    let raw = match typed_integer_literal(ExprTarget::Kotlin, expr) {
        Some(literal) => literal,
        None => kotlin_emit_node(expr)?,
    };
    Ok(kotlin_coerce(raw, expr.ty, expected, expr))
}

/// The Kotlin type `expr` is computed in when it is a bitwise or arithmetic
/// operation with a UByte/UShort operand, which Kotlin defines no operator on;
/// `None` for any other expression, whose text has the type it is inferred to.
///
/// Bitwise and shift operations run in `Int`: `UByte shr Int` does not resolve.
/// Arithmetic runs in `UInt`: `UByte + UByte` is defined as
/// `this.toUInt() + other.toUInt(): UInt`. Either widens to the joined type
/// when that is 64-bit (`kotlin_narrow_unsigned_domain`).
///
/// The emitted text of such an operation has this type and not the inferred
/// one until it is coerced, so a reader that converts the text itself, as an
/// index does, asks here rather than reading `expr.ty`: an `Int` converted to
/// `Int` again is a redundant call kotlinc `-Werror` refuses.
fn kotlin_narrow_unsigned_operation(expr: &TypedExpr) -> Option<InferredType> {
    let ExprKind::Binary { op, left, right } = &expr.kind else {
        return None;
    };
    // A shift's count is an `Int` wherever it is written, so only what it
    // shifts decides; and `Byte` and `Short` define no shift either.
    let narrow = if op.is_shift() {
        is_kotlin_narrow(left.ty)
    } else {
        is_narrow_unsigned(left.ty) || is_narrow_unsigned(right.ty)
    };
    if !narrow {
        return None;
    }
    if op.is_bitwise() {
        Some(kotlin_narrow_unsigned_domain(
            binary_operand_type(*op, left.ty, right.ty),
            InferredType::Int {
                signed: true,
                bits: 32,
            },
        ))
    } else if op.is_arith() {
        Some(kotlin_narrow_unsigned_domain(
            join_arith(left.ty, right.ty),
            InferredType::Int {
                signed: false,
                bits: 32,
            },
        ))
    } else {
        None
    }
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

/// An integer Kotlin defines no shift on, whatever its sign: `Byte`, `Short`,
/// `UByte`, `UShort` (8 or 16 bits). Its shift is made in `Int`.
fn is_kotlin_narrow(ty: InferredType) -> bool {
    matches!(ty, InferredType::Int { bits: 8 | 16, .. })
}

/// Whether the Kotlin text of `expr`, made in the type
/// [`kotlin_narrow_unsigned_operation`] names, is converted back to `expr`'s
/// own type before anything else reads it: a left shift, which wraps at the
/// width of what it shifts.
fn kotlin_wraps_at_its_width(expr: &TypedExpr) -> bool {
    matches!(&expr.kind, ExprKind::Binary { op: BinOp::Shl, .. })
}

/// The type of the text [`emit_kotlin`] writes for `expr` before it is
/// converted to what its caller asks: the type its operation is made in, or,
/// for one [`kotlin_wraps_at_its_width`], its own.
fn kotlin_written_type(expr: &TypedExpr) -> InferredType {
    match kotlin_narrow_unsigned_operation(expr) {
        Some(_) if kotlin_wraps_at_its_width(expr) => expr.ty,
        Some(made_in) => made_in,
        None => expr.ty,
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
    if let Some(form) = optional_form(expr) {
        return kotlin_optional_form(form, expr);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's overload for the operation's own width,
        // each operand spelled at that width so the overload resolves, as the
        // checked one's are. `Byte`, `Short`, `UByte` and `UShort` arithmetic
        // is made in `Int`, so the runtime truncates it back.
        ExprKind::Total { op, rule, .. } => {
            let (_, _, operands) = total_parts(expr, "Kotlin", emit_kotlin)?;
            format!(
                "com.sce.forge.runtime.Sce{}.{}({})",
                crate::filters::capitalize_first(rule.helper()),
                op.helper(),
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => portable_number_literal(n),
        ExprKind::StringLit { value } => crate::filters::kotlin_text_expression(value),
        // `"ack".toByteArray()` is byte-identical to the ASCII literal
        // (UTF-8 default). Used as the `contentEquals` argument in the
        // bytes-equality Binary branch below. RFC §bytesguard-3 B5. Escaped
        // as any Kotlin literal is, since a `$` in the bytes would otherwise
        // open a template expression.
        ExprKind::BytesLit { bytes } => {
            format!(
                "{}.toByteArray()",
                crate::filters::kotlin_text_expression(&latin1(bytes))
            )
        }
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "null".to_string(),
        // The one spelling the declaration of the name takes
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::Kotlin.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // `String.plus` takes `Any`, so Kotlin concatenated this by
            // accident while the other backends did not; the digits are made
            // explicit so that it is the same decision here as everywhere
            // (ECMA-262 13.15.3), not a property of Kotlin's overload.
            let operand = |side: &TypedExpr, is_left: bool| -> Result<String, ExprError> {
                let text = emit_kotlin(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text => Ok(
                        if concat_text_needs_parens(side, is_left, kotlin_precedence) {
                            format!("({text})")
                        } else {
                            text
                        },
                    ),
                    ConcatOperand::Decimal => {
                        Ok(format!("{}.toString()", wrap_postfix(side, text)))
                    }
                    ConcatOperand::Refused(what) => Err(concat_refusal(what, text)),
                }
            };
            format!("{} + {}", operand(left, true)?, operand(right, false)?)
        }
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
            let equality = matches!(op, BinOp::StrictEq | BinOp::StrictNeq);
            // A shift count is an `Int` whatever is shifted: `UInt.shl`,
            // `ULong.shr` and the rest all take `bitCount: Int`, so a count
            // converted to the shifted type (`k shl 15.toULong()`) does not
            // compile. The narrow-unsigned path above already does this.
            let count_ty = if matches!(op, BinOp::Shl | BinOp::Shr | BinOp::UShr) {
                InferredType::Int {
                    signed: true,
                    bits: 32,
                }
            } else {
                operand_ty
            };
            let mut l_raw = emit_kotlin(left, operand_ty)?;
            let mut r_raw = emit_kotlin(right, count_ty)?;
            if equality {
                l_raw = kotlin_equality_operand(l_raw, left, operand_ty);
                r_raw = kotlin_equality_operand(r_raw, right, operand_ty);
            }
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
            // `UInt` and `ULong` have no `ushr`: their `shr` is the logical
            // shift `>>>` is (SCE_FORGE.md §3.4.1, "The width of a shift").
            let spelled = match (op, operand_ty) {
                (BinOp::UShr, InferredType::Int { signed: false, .. }) => "shr",
                _ => kotlin_binop(*op),
            };
            format!("{l} {spelled} {r}")
        }
        // `Byte` and `Short` have no `inv()` of their own (only
        // `kotlin.experimental`'s), so their complement is made in `Int` and
        // converted back to the operand's width, which is its value.
        ExprKind::Unary {
            op: UnaryOp::BitNot,
            operand,
        } if matches!(
            expr.ty,
            InferredType::Int {
                signed: true,
                bits: 8 | 16
            }
        ) =>
        {
            let int = InferredType::Int {
                signed: true,
                bits: 32,
            };
            let complement = format!(
                "{}.inv()",
                wrap_postfix(operand, emit_kotlin(operand, int)?)
            );
            kotlin_coerce(complement, int, expr.ty, expr)
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
            // An operation on a narrow unsigned index is emitted in the type
            // Kotlin computes it in, which may be `Int` already.
            let idx_raw = emit_kotlin(index, InferredType::Unknown)?;
            let idx_emit = match kotlin_written_type(index) {
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
            // A list buffer is a `SceListBuf`, read by the element's own
            // typed reader; a bytes buffer a `SceByteBuf`, read as a byte.
            let reader = match object.ty {
                InferredType::ListBuffer(elem) => Some(kotlin_buffer_reader(elem)?),
                InferredType::BytesBuffer => Some("getUByte"),
                _ => None,
            };
            if let Some(reader) = reader {
                return Ok(format!(
                    "{}.{reader}({idx_emit})",
                    wrap_postfix(object, emit_kotlin(object, InferredType::Unknown)?),
                ));
            }
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
            fails,
            ..
        } => {
            if is_len_builtin(callee, args) {
                let measured = emit_kotlin(&args[0], InferredType::Unknown)?;
                // A string is measured in the UTF-8 bytes its bound counts: a Kotlin
                // `String` has no `size`, and its `length` counts UTF-16 units. A list
                // and a byte array by their `size`.
                let size = if args[0].ty == InferredType::Str {
                    format!("({measured}).encodeToByteArray().size")
                } else {
                    format!("({measured}).size")
                };
                // `Int` is Kotlin's length; one typed as an integer width
                // (`TypeCtx::lengths_are_uint32`) is that width.
                return Ok(match expr.ty {
                    InferredType::Int { signed, bits } => format!(
                        "{size}.{}()",
                        if signed {
                            kotlin_signed_ctor(bits)
                        } else {
                            kotlin_unsigned_ctor(bits)
                        }
                    ),
                    _ => size,
                });
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
            let symbol = wrap_postfix(callee, emit_kotlin(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Kotlin, &symbol, &call, expr.ty)?
            } else {
                call
            }
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field → `ByteArray`
            // (the algorithm `bytes` param type), UTF-8 encoded.
            format!(
                "{}.toByteArray(Charsets.UTF_8)",
                emit_kotlin(source, InferredType::Unknown)?
            )
        }
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => kotlin_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: the runtime's overload for the operation's own
        // width, each operand spelled at that width so the overload resolves
        // (Kotlin gives a bare literal no unsigned or narrow type). A failure
        // is thrown and turned into `AlgorithmResult.Failed` at the
        // algorithm's boundary.
        //
        // An index read goes through the overload for the array's type, the
        // index widened to `Long`; a `bytes` element is normalized to `UByte`
        // as the unchecked read's is.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust()),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => {
            // A buffer checks the index against its size, then reads with
            // its reader.
            // A buffer of unions is the `ArrayList` a record buffer is, read by
            // the overload of any `List` below (docs/adr/0099, section 5.2).
            let reader = match left.ty {
                InferredType::ListBuffer(elem) if elem.union_alias().is_none() => {
                    Some(kotlin_buffer_reader(elem)?)
                }
                InferredType::BytesBuffer => Some("getUByte"),
                _ => None,
            };
            if let Some(reader) = reader {
                let buffer = wrap_postfix(left, emit_kotlin(left, InferredType::Unknown)?);
                return Ok(format!(
                    "{buffer}.{reader}(com.sce.forge.runtime.SceChecked.index(({}).toLong(), {buffer}.size))",
                    emit_kotlin(index, InferredType::Unknown)?,
                ));
            }
            let norm = if matches!(left.ty, InferredType::Bytes) {
                ".toUByte()"
            } else {
                ""
            };
            format!(
                "com.sce.forge.runtime.SceChecked.at({}, ({}).toLong()){norm}",
                emit_kotlin(left, InferredType::Unknown)?,
                emit_kotlin(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Checked {
            op: CheckedOp::Narrow,
            left,
            ..
        } => {
            // The value widened, exactly, to `Long` or `ULong` by its own
            // signedness; one overload per target takes either.
            let (
                InferredType::Int { signed, bits },
                InferredType::Int {
                    signed: from_signed,
                    bits: from_bits,
                },
            ) = (expr.ty, left.ty)
            else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked narrowing from {:?} to {:?}", left.ty, expr.ty),
                    observed: None,
                });
            };
            let value = emit_kotlin(left, left.ty)?;
            let widened = match (from_signed, from_bits) {
                (true, 64) | (false, 64) => value,
                (true, _) => format!("({value}).toLong()"),
                (false, _) => format!("({value}).toULong()"),
            };
            let target = match (signed, bits) {
                (true, 8) => "Byte",
                (true, 16) => "Short",
                (true, 32) => "Int",
                (true, _) => "Long",
                (false, 8) => "UByte",
                (false, 16) => "UShort",
                (false, 32) => "UInt",
                (false, _) => "ULong",
            };
            format!("com.sce.forge.runtime.SceChecked.narrowTo{target}({widened})")
        }
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let mut operands = vec![emit_kotlin(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit_kotlin(right, expr.ty)?);
            }
            format!(
                "com.sce.forge.runtime.SceChecked.{}({})",
                op.helper(),
                operands.join(", ")
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in Kotlin: the runtime's
/// total shift (`SceWrapping.shl(x, n)`), which is the exact result reduced to
/// the width for every count at least 0, or its checked one (`SceChecked.shl`),
/// which throws an out-of-range failure for a count below 0. Kotlin's own `shl`,
/// `shr` and `ushr` take the count modulo the width, so `x shl 33` of an `Int`
/// is `x shl 1`. One overload per width, picked by what is shifted, spelled at
/// its own type; the count is written as the `ULong` the total shift takes, or
/// the `Long` the checked one does.
fn kotlin_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<String, ExprError> {
    settled_shift_width(expr)?;
    let value = emit_kotlin(left, expr.ty)?;
    let (family, count_ty) = match discharge {
        Discharge::AtBuild | Discharge::Total => (
            "SceWrapping",
            InferredType::Int {
                signed: false,
                bits: 64,
            },
        ),
        Discharge::AtRun => (
            "SceChecked",
            InferredType::Int {
                signed: true,
                bits: 64,
            },
        ),
    };
    let places = emit_kotlin(count, count_ty)?;
    Ok(format!(
        "com.sce.forge.runtime.{family}.{}({value}, {places})",
        op.helper()
    ))
}

/// The Kotlin spelling of each construct of an optional value (docs/adr/0055,
/// section 5) over `T?`:
///
/// - a test is `x != null` / `x == null`;
/// - a proven read is the forge runtime's `SceOptional.proven(x)` rather than
///   `x!!`: a test of a stable value (a field of a `<foreach>`'s item, a `val`
///   of a data class) smart-casts it to `T`, and `!!` on a `T` is a warning the
///   generated code is compiled to refuse;
/// - `x ?? d` is the elvis `x ?: d`, which evaluates `x` once and `d` only when
///   `x` is absent, as ECMAScript's `??` does; `d` is spelled in the type the
///   whole read is of (`Long? ?: Int` is no `Long`);
/// - equality is `==`, which is total over `T?`: true of two absent values,
///   false of an absent and a present one, `T`'s own of two present ones. Both
///   sides are spelled as the optional value of the wider one, so a literal
///   beside a `Long?` is a `Long` and a `UShort?` beside a `UInt?` is widened;
///   a byte string is compared by its contents through the standard library's
///   `ByteArray?.contentEquals(ByteArray?)`, which is true of two absent ones.
fn kotlin_optional_form(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<String, ExprError> {
    // `side` spelled as an operand of `op`, parenthesised where Kotlin would
    // otherwise bind it to something else.
    let operand = |side: &TypedExpr, op: BinOp, is_left: bool, text: String| {
        if child_needs_parens(side, op, is_left, kotlin_precedence) {
            format!("({text})")
        } else {
            text
        }
    };
    Ok(match form {
        OptionalForm::Test { subject, present } => {
            let (op, spelled) = if present {
                (BinOp::StrictNeq, "!=")
            } else {
                (BinOp::StrictEq, "==")
            };
            let tested = emit_kotlin(subject, InferredType::Unknown)?;
            format!("{} {spelled} null", operand(subject, op, true, tested))
        }
        OptionalForm::Proven { subject } => format!(
            "{KOTLIN_PROVEN}({})",
            emit_kotlin(subject, InferredType::Unknown)?
        ),
        OptionalForm::Otherwise { value, fallback } => {
            let present = match whole.ty {
                InferredType::Optional(elem) => elem.present(),
                present => present,
            };
            let read = emit_kotlin(value, InferredType::Unknown)?;
            let default =
                kotlin_equality_operand(emit_kotlin(fallback, whole.ty)?, fallback, present);
            format!(
                "{} {} {}",
                operand(value, BinOp::Coalesce, true, read),
                kotlin_binop(BinOp::Coalesce),
                operand(fallback, BinOp::Coalesce, false, default)
            )
        }
        OptionalForm::Equality { left, right, equal } => {
            let present_of = |ty: InferredType| match ty {
                InferredType::Optional(elem) => elem.present(),
                other => other,
            };
            let first = [left.ty, right.ty]
                .into_iter()
                .find_map(|ty| match ty {
                    InferredType::Optional(elem) => Some(elem),
                    _ => None,
                })
                .expect("an equality of an optional value has an optional side");
            let (l, r) = (present_of(left.ty), present_of(right.ty));
            let elem = if l.is_numeric() && r.is_numeric() {
                OptionalElem::lifting(join_arith(l, r)).unwrap_or(first)
            } else {
                first
            };
            let optional = InferredType::Optional(elem);
            let l_text = emit_kotlin(left, optional)?;
            let r_text = emit_kotlin(right, optional)?;
            if elem == OptionalElem::Bytes {
                let call = format!("{}.contentEquals({r_text})", wrap_postfix(left, l_text));
                if equal {
                    call
                } else {
                    format!("!{call}")
                }
            } else {
                let op = if equal {
                    BinOp::StrictEq
                } else {
                    BinOp::StrictNeq
                };
                format!(
                    "{} {} {}",
                    operand(left, op, true, l_text),
                    kotlin_binop(op),
                    operand(right, op, false, r_text)
                )
            }
        }
        OptionalForm::Constant(answer) => answer.to_string(),
    })
}

/// Spell an untyped integer operand of `==`/`!=` in the operand type when
/// Kotlin would not.
///
/// `kotlin_coerce` leaves an untyped literal bare in a signed context and
/// lets Kotlin adapt it to the type it flows into — which an assignment, an
/// argument, a return and `<` (`Long.compareTo(Int)` exists) all supply.
/// Equality supplies none: `week % interval == 0` with `week: Long` is
/// `Operator '==' cannot be applied to 'Long' and 'Int'`, and a `Byte` or
/// `Short` operand fails the same way. `Int` needs nothing, and unsigned
/// operands already carry their constructor (`kotlin_coerce`'s
/// `.toUInt()` arm).
///
/// An optional value supplies none either (docs/adr/0055): a `Long?` takes no
/// `Int`, so the default of `x ?? d` and a present value lifted into an
/// optional place are spelled the same way.
fn kotlin_equality_operand(raw: String, node: &TypedExpr, operand_ty: InferredType) -> String {
    if node.ty != InferredType::UntypedInt {
        return raw;
    }
    match operand_ty {
        InferredType::Int {
            signed: true,
            bits: 64,
        } => match &node.kind {
            ExprKind::NumberLit(text) if is_decimal_integer_literal(text) => format!("{raw}L"),
            _ => wrap_dotcall(raw, node, "toLong"),
        },
        InferredType::Int {
            signed: true,
            bits: 16,
        } => wrap_dotcall(raw, node, "toShort"),
        InferredType::Int {
            signed: true,
            bits: 8,
        } => wrap_dotcall(raw, node, "toByte"),
        _ => raw,
    }
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
        BinOp::Coalesce => "?:",
    }
}

/// The runtime function a proven read of an optional value is spelled with
/// (docs/adr/0055, section 5): the value, which the build proved present.
pub(crate) const KOTLIN_PROVEN: &str = "com.sce.forge.runtime.SceOptional.proven";

/// Apply language-specific coercion from a child's natural type to the
/// expected parent type.
fn kotlin_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    // An optional number where an optional number of another width is
    // expected: a present value is converted as a value of `T` is, and an
    // absent one stays absent (docs/adr/0055). Kotlin's `UInt?` takes no
    // `UShort?`.
    if let (Optional(have), Optional(want)) = (from, to) {
        let ctor = match (have.present(), want.present()) {
            (have, want) if have == want => None,
            (Int { .. } | Float { .. }, Int { signed, bits }) => Some(if signed {
                kotlin_signed_ctor(bits)
            } else {
                kotlin_unsigned_ctor(bits)
            }),
            (Int { .. } | Float { .. }, Float { bits: 32 }) => Some("toFloat"),
            (Int { .. } | Float { .. }, Float { .. }) => Some("toDouble"),
            _ => None,
        };
        return match ctor {
            Some(ctor) => format!("{}?.{ctor}()", wrap_postfix(node, raw)),
            None => raw,
        };
    }
    // Any other value that may be absent converts to nothing: it is compared
    // as it is, and read as `T` only through `??` or a proven read.
    if matches!(from, Optional(_) | Null) {
        return raw;
    }
    // Kotlin has no bare literal past `Long`: the unsigned value takes the
    // `uL` suffix, and `Long`'s minimum is its named constant.
    if matches!(to.strip_quantity(), Int { .. }) {
        if let Some(spelled) = wide_literal(node)
            .and_then(|wide| wide_literal_spelling(crate::generator::Language::Kotlin, wide, &raw))
        {
            return spelled;
        }
    }
    if from == to || matches!(to, Unknown) {
        return raw;
    }
    match (from, to) {
        // Literal promotion for untyped integers into float context. A Kotlin
        // real literal is a `Double` unless it ends in `f`, so one that lands in
        // a 32-bit real is written as the single it is: `1.0f` is a `Float`,
        // `1.0` does not assign to one.
        (UntypedInt, Float { .. }) | (UntypedInt, UntypedFloat) => {
            let single = matches!(to, Float { bits: 32 });
            if let ExprKind::NumberLit(text) = &node.kind {
                if is_decimal_integer_literal(text) {
                    return format!("{raw}.0{}", if single { "f" } else { "" });
                }
            }
            // Computed subtree or hex/bin/oct literal — explicit cast.
            format!("({raw}).{}()", if single { "toFloat" } else { "toDouble" })
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
            // Literal adopting unsigned concrete type. An untyped integer
            // is not always a lone literal: `c ? 1 : 0` is one too, and a
            // bare suffix bound to its last branch alone
            // (`if (c) 1 else 0.toUByte()`), which Kotlin types as
            // `Comparable<*>`. Wrapped like every other conversion here.
            let suffix = kotlin_unsigned_ctor(bits);
            wrap_dotcall(raw, node, suffix)
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
        // Untyped float → a 64-bit real: the text is a `Double` already.
        (UntypedFloat, Float { bits: 64 }) => raw,
        // Untyped float → a 32-bit real: the literal is the single nearest to
        // it, written with the suffix that makes it one; a computed subtree is
        // narrowed.
        (UntypedFloat, Float { .. }) => match &node.kind {
            ExprKind::NumberLit(_) => format!("{raw}f"),
            _ => wrap_dotcall(raw, node, "toFloat"),
        },
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
// * The expression is built as a tree of the Rust expressions it is made of
//   ([`RustExpr`], `expr/rust_tree.rs`), a cast a node of its own wherever
//   it comes from, and printed once, where it leaves this layer: every
//   parenthesis is the printer's, decided by the kinds of the nodes around it
//   and the precedence the Rust Reference gives them, and no rule here groups
//   anything or reads back what it has written.

/// The Rust expression `expr` is where a value of `expected` is wanted.
fn emit_rust(expr: &TypedExpr, expected: InferredType) -> Result<RustExpr, Refusal> {
    // The context a quantity-annotated output asks for is its numeric base
    // (see `rust_coerce`), or the push-downs below never see a float.
    let expected = expected.strip_quantity();
    // Push-down: for arithmetic binary ops with a concrete-float expected
    // type, emit operands at the expected type directly. Without this, a
    // mixed expression like `raw * 0.1` (raw: UInt16, expected: Float64)
    // infers operand_ty = Float32 from join_arith, producing
    // `(raw as f32 * 0.1) as f64` instead of `raw as f64 * 0.1`.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
            return Ok(RustExpr::binary(
                rust_binop(*op),
                emit_rust(left, expected)?,
                emit_rust(right, expected)?,
            ));
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
            // `+x` is `x`.
            return Ok(if *op == UnaryOp::Neg {
                inner.prefixed(RustUnOp::Neg)
            } else {
                inner
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
                return Ok(rust_length(&args[0])?.cast(rust_int_type(signed, bits)));
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
                return Ok(rust_real_to_int(op, &args[0])?.cast(rust_int_type(signed, bits)));
            }
        }
    }
    let raw = match rust_typed_literal(expr) {
        Some(literal) => literal,
        None => rust_emit_node(expr)?,
    };
    rust_coerce(raw, expr.ty, expected, expr)
}

/// `len(value)`: Rust's `len`, a `usize`, which counts a text in its UTF-8
/// bytes.
fn rust_length(value: &TypedExpr) -> Result<RustExpr, Refusal> {
    Ok(emit_rust(value, InferredType::Unknown)?.method("len", Vec::new()))
}

/// The real `round(value)` or `floor(value)` is before it is made an integer:
/// `f64::round` is already half away from zero, and `f64::floor` already goes
/// toward −∞.
fn rust_real_to_int(op: RealToInt, value: &TypedExpr) -> Result<RustExpr, Refusal> {
    let method = match op {
        RealToInt::Round => "round",
        RealToInt::Floor => "floor",
    };
    Ok(emit_rust(value, InferredType::Float { bits: 64 })?.method(method, Vec::new()))
}

/// An integer literal that has taken the type of its place
/// ([`typed_integer_literal`]), in Rust: its digits with the type's suffix,
/// under the `-` the author wrote, which stays the prefix operator it is.
fn rust_typed_literal(node: &TypedExpr) -> Option<RustExpr> {
    let InferredType::Int { signed, bits } = node.ty else {
        return None;
    };
    let (text, negated) = integer_literal_parts(node)?;
    let literal = RustExpr::atom(format!(
        "{}{}",
        portable_number_literal(&text),
        rust_int_type(signed, bits)
    ));
    Some(if negated {
        literal.prefixed(RustUnOp::Neg)
    } else {
        literal
    })
}

/// How a Rust index reads an element that is no plain value (docs/adr/0086):
/// `at` copies the element out, which only a `Copy` element allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustElement {
    /// A list, read by reference: the operand of a further read, its length,
    /// an index into it, or a field of its element.
    Operand,
    /// A text, a byte string, or an optional value of either or of a list:
    /// the owned copy the place it is written to takes.
    Owned,
}

/// How a Rust index reads an element of type `ty`; `None` for a plain value,
/// which `at` copies out.
fn rust_element_by_reference(ty: InferredType) -> Option<RustElement> {
    match ty {
        InferredType::List(_) => Some(RustElement::Operand),
        InferredType::Str | InferredType::Bytes => Some(RustElement::Owned),
        InferredType::Optional(elem)
            if matches!(
                elem.present(),
                InferredType::Str | InferredType::Bytes | InferredType::List(_)
            ) =>
        {
            Some(RustElement::Owned)
        }
        _ => None,
    }
}

/// The list `left` an index reads, as the reference `at` and `at_ref` take: a
/// borrow of the list, or, for a list that is itself an element read by
/// reference ([`RustElement::Operand`]), that reference.
fn rust_list_operand(left: &TypedExpr) -> Result<RustExpr, Refusal> {
    let emitted = emit_rust(left, InferredType::Unknown)?;
    let by_reference = matches!(
        &left.kind,
        ExprKind::Checked {
            op: CheckedOp::Index,
            right: Some(_),
            ..
        }
    ) && rust_element_by_reference(left.ty) == Some(RustElement::Operand);
    Ok(if by_reference {
        emitted
    } else {
        emitted.prefixed(RustUnOp::Ref)
    })
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
fn emit_rust_returned_str(expr: &TypedExpr) -> Result<RustExpr, Refusal> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(RustExpr::if_else(
            emit_rust(condition, InferredType::Bool)?,
            emit_rust_returned_str(consequent)?,
            emit_rust_returned_str(alternate)?,
        )),
        ExprKind::Call { .. } | ExprKind::Raw(_) => emit_rust(expr, InferredType::Str),
        _ => Ok(emit_rust(expr, InferredType::Str)?.method("to_string", Vec::new())),
    }
}

/// A `string` value made owned for a place that holds a `String`
/// ([`Position::Owned`]).
///
/// [`emit_rust_returned_str`]'s rule, with one change: a `Raw` fragment is NOT
/// taken to be owned already. After the rename pass a call is `Raw`, and so is
/// the read of a variable — which is the machine's own field, and assigning it
/// somewhere else would move it out from behind `&mut self`. A call's fresh
/// `String` copied once more costs an allocation and is correct; the field
/// moved out is not.
fn emit_rust_owned_str(expr: &TypedExpr) -> Result<RustExpr, Refusal> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(RustExpr::if_else(
            emit_rust(condition, InferredType::Bool)?,
            emit_rust_owned_str(consequent)?,
            emit_rust_owned_str(alternate)?,
        )),
        _ => Ok(emit_rust(expr, InferredType::Str)?.method("to_string", Vec::new())),
    }
}

/// A `bytes` value as the `sce_forge_bytes_view_t` a place that owns a byte string
/// is written from ([`Position::Owned`]): a constant is the view of its literal, a
/// read of a byte string the machine holds the view of its `{data, len}`, and the
/// arms of a conditional one by one, since both arms of a C `?:` must be one type.
/// Whoever writes the value copies from the view, so a variable assigned from
/// another is two buffers and not one.
fn emit_c_owned_bytes(expr: &TypedExpr) -> Result<String, ExprError> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(format!(
            "({} ? {} : {})",
            emit_c(condition, InferredType::Bool)?,
            emit_c_owned_bytes(consequent)?,
            emit_c_owned_bytes(alternate)?,
        )),
        ExprKind::BytesLit { bytes } => Ok(format!(
            "(sce_forge_bytes_view_t){{ (const uint8_t *){}, {} }}",
            c_bytes_literal(bytes),
            bytes.len()
        )),
        _ => {
            let held = wrap_postfix(expr, emit_c(expr, InferredType::Unknown)?);
            Ok(format!(
                "(sce_forge_bytes_view_t){{ ({held}).data, ({held}).len }}"
            ))
        }
    }
}

/// A `bytes` value made owned for a place that holds a `Vec<u8>`
/// ([`Position::Owned`]), as [`emit_rust_owned_str`] does a string: a constant
/// is a `&[u8; N]` and a variable read is the machine's own field, so each leaf
/// is copied, and the arms of a conditional one by one, since both arms of a
/// Rust `if` must be one type.
fn emit_rust_owned_bytes(expr: &TypedExpr) -> Result<RustExpr, Refusal> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(RustExpr::if_else(
            emit_rust(condition, InferredType::Bool)?,
            emit_rust_owned_bytes(consequent)?,
            emit_rust_owned_bytes(alternate)?,
        )),
        _ => Ok(emit_rust(expr, InferredType::Bytes)?.method("to_vec", Vec::new())),
    }
}

/// The Rust literal `written` is, when it is one string literal: the text
/// literal as the Rust emitter spells it, or for a place of bytes
/// (`bytes`) the byte-string literal of the printable ASCII it spells, as a
/// literal lands in a byte-string slot ([`string_literal_as_bytes`]). `None`
/// for any other expression, and for a text no byte-string literal spells.
pub(crate) fn rust_literal(written: &str, bytes: bool) -> Option<String> {
    let mut ast = parse_to_ast(written.trim()).ok()?;
    if !matches!(ast.kind, ExprKind::StringLit { .. }) {
        return None;
    }
    if bytes && !string_literal_as_bytes(&mut ast) {
        return None;
    }
    rust_emit_value(&ast)
        .ok()
        .map(|literal| literal.to_string())
}

/// `text` as what the C++ runtime's `OwnedText<N>::literal` is handed: the
/// string literal of its bytes while the text fits one and holds no U+0000,
/// and otherwise the braced character constants of its bytes and the zero
/// that ends them, an array of the same type and length, which holds a zero
/// byte as any other and is held to no length.
pub(crate) fn cpp_text_literal(text: &str) -> String {
    if text.len() <= crate::filters::CPP_STRING_LITERAL_MAX_BYTES && !text.contains('\0') {
        return format!("\"{}\"", crate::filters::escape_cpp(text.to_string()));
    }
    let mut constants = crate::filters::c_char_constants(text);
    constants.push("'\\0'".to_string());
    format!("{{{}}}", constants.join(", "))
}

/// What the C++ runtime's `literal` of a text or a byte-string place is handed
/// for `written`, when it is one string literal: a text as the C++ string
/// literal of its bytes, or, where the text is too long for one or holds a
/// U+0000, the braced character constants of its bytes and the zero that ends
/// them, an array of the same type and length; for a place of bytes (`bytes`)
/// the braced hex bytes of the printable ASCII it spells, as a literal lands in
/// a byte-string slot ([`string_literal_as_bytes`]). `Some(None)` for a byte
/// string of no bytes, which no array holds and which the place's default is.
/// `None` for any other expression, and for a text no byte string spells.
pub(crate) fn cpp_literal_elements(written: &str, bytes: bool) -> Option<Option<String>> {
    let mut ast = parse_to_ast(written.trim()).ok()?;
    let ExprKind::StringLit { value } = &ast.kind else {
        return None;
    };
    if !bytes {
        return Some(Some(cpp_text_literal(value)));
    }
    if !string_literal_as_bytes(&mut ast) {
        return None;
    }
    let ExprKind::BytesLit { bytes } = &ast.kind else {
        return None;
    };
    Some((!bytes.is_empty()).then(|| format!("{{{}}}", bytes_as_hex_list(bytes))))
}

/// A text or a byte string for a place that copies it through its own
/// checked constructor ([`Position::Copied`]): borrowed whole, as the `&str`
/// or `&[u8]` the constructor takes, and the arms of a conditional one by one,
/// since both arms of a Rust `if` must be one type. A text literal is a `&str`
/// already; every other value is sliced whole, a variable read included, so a
/// field of the machine is lent and never moved out of it.
fn emit_rust_copied(expr: &TypedExpr, expected: InferredType) -> Result<RustExpr, Refusal> {
    match &expr.kind {
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => Ok(RustExpr::if_else(
            emit_rust(condition, InferredType::Bool)?,
            emit_rust_copied(consequent, expected)?,
            emit_rust_copied(alternate, expected)?,
        )),
        ExprKind::StringLit { .. } if expected == InferredType::Str => emit_rust(expr, expected),
        _ => Ok(emit_rust(expr, expected)?.borrowed_whole()),
    }
}

/// An optional value made owned for a place that holds an `Option<T>`
/// ([`Position::Owned`]), as [`emit_rust_owned_str`] does a present text: an
/// expression reads an optional text or byte string borrowed
/// ([`rust_optional::view`]), so its present value is copied into the place's
/// own; any other element is `Copy`, and the value is taken as it is.
fn emit_rust_owned_optional(expr: &TypedExpr, expected: InferredType) -> Result<RustExpr, Refusal> {
    let view = emit_rust(expr, expected)?;
    Ok(match expected {
        // The absent value is owned as it is: there is no borrowed text to copy.
        InferredType::Optional(elem @ (OptionalElem::Str | OptionalElem::Bytes))
            if !matches!(expr.kind, ExprKind::NullLit) =>
        {
            rust_optional::owned(view, elem)
        }
        // An optional record that a variable, a parameter or a local holds is
        // copied where another place takes it whole, as a record is
        // (docs/adr/0055, step 9): the place keeps its own, and the body may read
        // it again. A value a call answers is owned already.
        InferredType::Optional(OptionalElem::Record)
            if expr.ty == InferredType::Optional(OptionalElem::Record)
                && matches!(
                    expr.kind,
                    ExprKind::Ident(_) | ExprKind::Raw(_) | ExprKind::Member { .. }
                ) =>
        {
            view.method("clone", Vec::new())
        }
        _ => view,
    })
}

/// A Rust expression node. A construct of an optional value (docs/adr/0055)
/// is its own form ([`rust_optional_form`]), and a place that holds an optional
/// text or byte string is read borrowed ([`rust_optional::view`]), as a place
/// that holds a present one is.
fn rust_emit_node(expr: &TypedExpr) -> Result<RustExpr, Refusal> {
    if let Some(form) = optional_form(expr) {
        return rust_optional_form(form, expr);
    }
    let value = rust_emit_value(expr)?;
    let place = matches!(
        expr.kind,
        ExprKind::Ident(_) | ExprKind::Raw(_) | ExprKind::Member { .. }
    );
    Ok(match expr.ty {
        InferredType::Optional(elem) if place => rust_optional::view(value, elem),
        _ => value,
    })
}

/// The Rust spelling of each construct of an optional value (docs/adr/0055,
/// section 5) over `core`'s `Option<T>` ([`rust_optional`]): a test is
/// `is_some()` or `is_none()`, a proven read goes through the runtime's
/// `proven`, `x ?? d` is a `match` whose absent arm alone evaluates `d`, and
/// equality is `Option`'s own, a side of `T` lifted into the optional value of
/// the other ([`InferredType::lifted_into`], [`rust_coerce`]).
fn rust_optional_form(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<RustExpr, Refusal> {
    let read = |node: &TypedExpr| emit_rust(node, InferredType::Unknown);
    Ok(match form {
        OptionalForm::Test { subject, present } => {
            let view = read(subject)?;
            if present {
                rust_optional::is_present(view)
            } else {
                rust_optional::is_absent(view)
            }
        }
        // A record is read in place, through a borrow, and the member `.f` of the
        // field read is taken of it: the record that owns text is not `Copy`, and
        // `x.value` taken whole is no expression (docs/adr/0055, step 9).
        OptionalForm::Proven { subject }
            if subject.ty == InferredType::Optional(OptionalElem::Record) =>
        {
            rust_optional::proven_ref_expr(read(subject)?)
        }
        OptionalForm::Proven { subject } => rust_optional::proven(read(subject)?),
        OptionalForm::Otherwise { value, fallback } => {
            // Both arms of the `match` are one type: a present text is read
            // borrowed, so a default of text is borrowed too.
            let default = emit_rust(fallback, whole.ty)?;
            let default = match whole.ty {
                InferredType::Optional(_) => default,
                ty => match OptionalElem::lifting(ty) {
                    Some(elem) => rust_borrowed(fallback, default, elem),
                    None => default,
                },
            };
            rust_optional::coalesce(read(value)?, default, whole.ty.is_optional())
        }
        OptionalForm::Equality { left, right, equal } => {
            let optional = [left.ty, right.ty]
                .into_iter()
                .find(|ty| ty.is_optional())
                .unwrap_or(InferredType::Unknown);
            rust_optional::equality(
                emit_rust(left, optional)?,
                emit_rust(right, optional)?,
                equal,
            )
        }
        OptionalForm::Constant(answer) => RustExpr::atom(answer.to_string()),
    })
}

/// `value`, the Rust value of `node`, borrowed as an expression reads the
/// present value of an optional text or byte string of the element `elem`
/// (`&str`, `&[u8]`): a string literal is one already, and any other text — an
/// owned `String` or `Vec<u8>`, a byte-string literal's array — is sliced whole.
/// A value of any other element is `Copy`, and is `value` itself.
fn rust_borrowed(node: &TypedExpr, value: RustExpr, elem: OptionalElem) -> RustExpr {
    match (elem, &node.kind) {
        (OptionalElem::Str, ExprKind::StringLit { .. }) => value,
        (OptionalElem::Str | OptionalElem::Bytes, _) => value.borrowed_whole(),
        _ => value,
    }
}

fn rust_emit_value(expr: &TypedExpr) -> Result<RustExpr, Refusal> {
    Ok(match &expr.kind {
        // docs/adr/0063: the standard library's own operation at the node's
        // type, a `const fn` that never panics. Fully qualified, so a literal
        // left operand takes the type too, where a method call on the literal
        // would leave it ambiguous.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "Rust", emit_rust)?;
            RustExpr::path_call(
                format!(
                    "{}::{}_{}",
                    rust_int_type(signed, bits),
                    rule.helper(),
                    op.helper()
                ),
                operands,
            )
        }
        ExprKind::NumberLit(n) => RustExpr::atom(portable_number_literal(n)),
        ExprKind::StringLit { value } => RustExpr::atom(format!(
            "\"{}\"",
            crate::filters::escape_rust(value.clone())
        )),
        // `Vec<u8> == &[u8; N]` (and `== Vec<u8>`) compare length+content,
        // so the default `==`/`!=` Binary path needs no special-casing —
        // only this constant rendering. ASCII-only by contract so `b"…"`
        // is valid.
        ExprKind::BytesLit { bytes } => {
            RustExpr::atom(format!("b\"{}\"", bytes_as_quoted_ascii(bytes)))
        }
        ExprKind::BoolLit(b) => RustExpr::atom(if *b { "true" } else { "false" }),
        ExprKind::NullLit => RustExpr::atom("None"),
        // The one spelling the declaration of the name takes
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => RustExpr::atom(ExprTarget::Rust.local_spelling(s)),
        // A fragment another pass wrote whole, which stands as one operand.
        ExprKind::Raw(s) => RustExpr::atom(s.clone()),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // `&str + i64` does not compile in Rust, and `&str + &str` does
            // not either. `format!` takes a string slice, an owned `String`
            // and an integer alike, and prints an integer as its decimal
            // digits -- the one text ECMA-262 13.15.3 gives it everywhere.
            let operand = |side: &TypedExpr| -> Result<RustExpr, Refusal> {
                let value = emit_rust(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text | ConcatOperand::Decimal => Ok(value),
                    ConcatOperand::Refused(what) => {
                        Err(concat_refusal(what, value.to_string()).into())
                    }
                }
            };
            RustExpr::macro_call(
                "format",
                vec![RustExpr::atom("\"{}{}\""), operand(left)?, operand(right)?],
            )
        }
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            RustExpr::binary(
                rust_binop(*op),
                emit_rust(left, operand_ty)?,
                emit_rust(right, right_operand_type(*op, operand_ty))?,
            )
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_rust(operand, expr.ty)?;
            match op {
                UnaryOp::Neg => inner.prefixed(RustUnOp::Neg),
                // `+x` is `x`.
                UnaryOp::Pos => inner,
                UnaryOp::Not | UnaryOp::BitNot => inner.prefixed(RustUnOp::Not),
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => RustExpr::if_else(
            emit_rust(condition, InferredType::Bool)?,
            emit_rust(consequent, expr.ty)?,
            emit_rust(alternate, expr.ty)?,
        ),
        // A field of the element a list of records is indexed at is read through
        // the element by reference: a record that owns text is not `Copy`, and the
        // list keeps its element.
        ExprKind::Member { object, property }
            if matches!(
                &object.kind,
                ExprKind::Checked {
                    op: CheckedOp::Index,
                    right: Some(_),
                    ..
                }
            ) =>
        {
            let ExprKind::Checked {
                left,
                right: Some(index),
                discharge,
                ..
            } = &object.kind
            else {
                unreachable!("guarded by the match above")
            };
            rust_contract(
                *discharge,
                "at_ref",
                vec![
                    rust_list_operand(left)?,
                    emit_rust(index, InferredType::Unknown)?,
                ],
            )
            .field(property.as_str())
        }
        ExprKind::Member { object, property } => {
            emit_rust(object, InferredType::Unknown)?.field(property.as_str())
        }
        ExprKind::Index { object, index } => {
            // Rust slice/array indexing requires `usize`. Concrete
            // integer-typed indices (declared via `<sce:var name=...
            // type="u16">`, parameters, member fields) need an
            // explicit `as usize` cast or rustc's strict typecheck
            // refuses the access. `UntypedInt` literals (e.g.
            // `arr[0]`) are accepted by rustc directly and stay
            // un-wrapped to keep prior goldens byte-stable.
            let position = emit_rust(index, InferredType::Unknown)?;
            let position = match index.ty {
                InferredType::Int { .. } => position.cast("usize"),
                _ => position,
            };
            emit_rust(object, InferredType::Unknown)?.indexed(position)
        }
        ExprKind::Call {
            callee,
            args,
            params,
            fails,
            into_destination,
            ..
        } => {
            if is_len_builtin(callee, args) {
                return rust_length(&args[0]);
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                return Ok(rust_real_to_int(op, &args[0])?.cast("i64"));
            }
            let mut emitted_args = Vec::with_capacity(args.len());
            for (i, a) in args.iter().enumerate() {
                emitted_args.push(emit_rust(a, argument_type(params, i))?);
            }
            let callee = emit_rust(callee, InferredType::Unknown)?;
            // A generated algorithm writes the list it returns into a destination
            // the call holds (docs/adr/0082), and the call's value is that list. A
            // list is taken only from an algorithm that declares `may-fail`, whose
            // failure `?` hands on.
            if *into_destination && *fails {
                emitted_args.push(RustExpr::atom(RUST_DESTINATION).prefixed(RustUnOp::RefMut));
                return Ok(RustExpr::block(
                    vec![
                        RustStmt::Let {
                            pattern: format!("mut {RUST_DESTINATION}"),
                            init: RustExpr::atom(RUST_LIST_DESTINATION),
                        },
                        RustStmt::Expr(RustExpr::call(callee, emitted_args).tried()),
                    ],
                    RustExpr::atom(RUST_DESTINATION),
                ));
            }
            let call = RustExpr::call(callee, emitted_args);
            // A call of a `may-fail` function hands its failure on by `?`, as
            // [`pass_failure_on`] spells it for Rust.
            if *fails {
                call.tried()
            } else {
                call
            }
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`&str`) → the
            // algorithm `bytes` param type (`&[u8]`) via `.as_bytes()`.
            emit_rust(source, InferredType::Unknown)?.method("as_bytes", Vec::new())
        }
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => rust_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: an index read through the runtime's `at`,
        // which takes the index at its own type and fails out of range
        // instead of panicking.
        //
        // An element that is no plain value (a list, a text, a byte string, an
        // optional one of either, docs/adr/0086) is read by reference through
        // `at_ref`: a list is the operand of a further read, and a text or a
        // byte string is the owned copy a place takes.
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            discharge,
        } => {
            let list = rust_list_operand(left)?;
            let index = emit_rust(index, InferredType::Unknown)?;
            // A union, typed by no operand type, is no `Copy` value either: the
            // element a union local copies is read by reference and cloned out
            // of the list, which keeps its own (docs/adr/0099, section 5.2).
            let element = rust_element_by_reference(expr.ty)
                .or_else(|| listed_union(left.ty).map(|_| RustElement::Owned));
            match element {
                None => rust_contract(*discharge, "at", vec![list, index]),
                Some(RustElement::Operand) => {
                    rust_contract(*discharge, "at_ref", vec![list, index])
                }
                Some(RustElement::Owned) => rust_contract(*discharge, "at_ref", vec![list, index])
                    .method("clone", Vec::new()),
            }
        }
        // SCE_FORGE.md §3.4.1: the runtime's checked helper at the operation's
        // own width, its failure returned through the algorithm's `Result`
        // by `?`. The width rides as a turbofish so a literal operand takes
        // it too, where a method call on the literal would be ambiguous.
        ExprKind::Checked {
            op,
            left,
            right,
            discharge,
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                }
                .at(expr.span.clone()));
            };
            if *op == CheckedOp::Narrow {
                // The value at its own type; the helper names both.
                let InferredType::Int {
                    signed: from_signed,
                    bits: from_bits,
                } = left.ty
                else {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("a checked narrowing from {:?}", left.ty),
                        observed: None,
                    }
                    .at(expr.span.clone()));
                };
                return Ok(RustExpr::path_call(
                    format!(
                        "sce_forge_runtime::algorithm::narrow::<{}, {}>",
                        rust_int_type(signed, bits),
                        rust_int_type(from_signed, from_bits)
                    ),
                    vec![emit_rust(left, left.ty)?],
                )
                .tried());
            }
            let mut operands = vec![emit_rust(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit_rust(right, expr.ty)?);
            }
            rust_contract(
                *discharge,
                &format!("{}::<{}>", op.helper(), rust_int_type(signed, bits)),
                operands,
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in Rust. Rust's own `<<` and
/// `>>` panic in a build that checks overflow at a count at or past the width,
/// and mask the count in one that does not, so neither is written there:
///
/// - the total shift is the standard library's own `checked_shl` and
///   `checked_shr` at the shift's type, the count as the `u32` they take, and
///   the value for a count at or past the width where they answer none: 0, or
///   for a signed `>>` the sign, which `wrapping_shr` by a count held at
///   `W - 1` leaves. A count past `u32` is past every width. A `>>>` of a
///   signed value is made in the unsigned type of its width. None panics; each
///   is fully qualified, as a wrapping operation is, so a literal operand takes
///   the type; and none names the runtime, so a kind that depends on none still
///   depends on none;
/// - the checked shift is the forge runtime's `algorithm::shl`, `shr` and
///   `ushr`, whose out-of-range failure for a count below 0 `?` hands on, the
///   count taken at its own type.
fn rust_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<RustExpr, Refusal> {
    let (signed, bits) = settled_shift_width(expr).map_err(|e| e.at(expr.span.clone()))?;
    let ty = rust_int_type(signed, bits);
    let value = emit_rust(left, expr.ty)?;
    let written = emit_rust(count, InferredType::Unknown)?;
    match discharge {
        Discharge::AtRun => {
            return Ok(RustExpr::path_call(
                format!("sce_forge_runtime::algorithm::{}::<{ty}, _>", op.helper()),
                vec![value, written],
            )
            .tried())
        }
        // Proven below the width ([`prove_integer_operations`]): the operator
        // itself, in the runtime, so the crate the file is compiled in holds no
        // shift operator of its own. A `>>>` proven so is of an unsigned value,
        // where it is `>>`.
        Discharge::AtBuild => {
            let helper = if op == CheckedOp::Shl { "shl" } else { "shr" };
            return Ok(RustExpr::path_call(
                format!("sce_forge_runtime::algorithm::proven::{helper}::<{ty}, _>"),
                vec![value, written],
            ));
        }
        Discharge::Total => {}
    }
    let places = rust_shift_places(count, written);
    // The value for a count at or past the width, where the standard library's
    // checked shift answers none.
    let or_zero = |call: RustExpr| call.method("unwrap_or", vec![RustExpr::atom("0")]);
    Ok(match op {
        CheckedOp::Shl => or_zero(RustExpr::path_call(
            format!("{ty}::checked_shl"),
            vec![value, places],
        )),
        CheckedOp::Shr if signed => RustExpr::path_call(
            format!("{ty}::wrapping_shr"),
            vec![
                value,
                RustExpr::path_call(
                    "u32::min",
                    vec![places, RustExpr::atom((bits - 1).to_string())],
                ),
            ],
        ),
        // Made on the bits, in the unsigned type of the width, and cast back: a
        // cast node, which the printer groups where the grammar needs it and
        // leaves bare where it is assigned, as `unused_parens` asks
        // (`algorithm_shift_count`).
        CheckedOp::UShr if signed => {
            let unsigned = rust_int_type(false, bits);
            or_zero(RustExpr::path_call(
                format!("{unsigned}::checked_shr"),
                vec![value.cast(unsigned), places],
            ))
            .cast(ty)
        }
        _ => or_zero(RustExpr::path_call(
            format!("{ty}::checked_shr"),
            vec![value, places],
        )),
    })
}

/// A shift's count `count`, written `written`, as the `u32` number of places the
/// standard library's shifts take: a literal as its value, an unsigned count of
/// at most 32 bits widened, and any other held to `u32::MAX` where `u32` cannot
/// hold it, which is past every width. A count below 0 reaches a total shift
/// only outside the bodies the contract covers, where it is the caller's to rule
/// out (SCE_FORGE.md §3.4.1), and is past the width there too.
fn rust_shift_places(count: &TypedExpr, written: RustExpr) -> RustExpr {
    if let Some((text, false)) = integer_literal_parts(count) {
        if let Some(value) = integer_literal_value(&text) {
            return RustExpr::atom(
                u32::try_from(value).map_or_else(|_| "u32::MAX".to_string(), |v| v.to_string()),
            );
        }
    }
    match count.ty.strip_quantity() {
        InferredType::Int {
            signed: false,
            bits: 32,
        } => written,
        InferredType::Int {
            signed: false,
            bits,
        } if bits < 32 => RustExpr::path_call("u32::from", vec![written]),
        _ => RustExpr::path_call("u32::try_from", vec![written])
            .method("unwrap_or", vec![RustExpr::atom("u32::MAX")]),
    }
}

/// A Rust integer operation of the contract (SCE_FORGE.md §3.4.1), the call of
/// `helper` with `args`: the runtime's checked helper, its failure handed to
/// the caller of a `may-fail` body by `?`, or the runtime's `proven` function,
/// for an operation the interval analysis proved ([`Discharge::AtBuild`]).
/// Either way the operation is a call, so the crate the file is compiled in
/// holds no operator and no index of its own for it.
fn rust_contract(discharge: Discharge, helper: &str, args: Vec<RustExpr>) -> RustExpr {
    match discharge {
        Discharge::AtRun => {
            RustExpr::path_call(format!("sce_forge_runtime::algorithm::{helper}"), args).tried()
        }
        // A total operation is a shift's alone, which [`rust_settled_shift`]
        // prints; no operation of the contract reaches here so marked.
        Discharge::AtBuild | Discharge::Total => RustExpr::path_call(
            format!("sce_forge_runtime::algorithm::proven::{helper}"),
            args,
        ),
    }
}

/// The local a call that writes the list it returns into a destination is
/// handed (docs/adr/0082), whose value the call is.
const RUST_DESTINATION: &str = "sce_out";

/// The Rust operator `op` is written as.
fn rust_binop(op: BinOp) -> RustBinOp {
    match op {
        BinOp::Add => RustBinOp::Add,
        BinOp::Sub => RustBinOp::Sub,
        BinOp::Mul => RustBinOp::Mul,
        BinOp::Div => RustBinOp::Div,
        BinOp::Mod => RustBinOp::Rem,
        BinOp::StrictEq => RustBinOp::Eq,
        BinOp::StrictNeq => RustBinOp::Ne,
        BinOp::Lt => RustBinOp::Lt,
        BinOp::Gt => RustBinOp::Gt,
        BinOp::LtEq => RustBinOp::Le,
        BinOp::GtEq => RustBinOp::Ge,
        BinOp::And => RustBinOp::And,
        BinOp::Or => RustBinOp::Or,
        BinOp::BitAnd => RustBinOp::BitAnd,
        BinOp::BitOr => RustBinOp::BitOr,
        BinOp::BitXor => RustBinOp::BitXor,
        BinOp::Shl => RustBinOp::Shl,
        BinOp::Shr | BinOp::UShr => RustBinOp::Shr,
        BinOp::Coalesce => unreachable!("{COALESCE_NOT_SPELLED}"),
    }
}

fn rust_coerce(
    raw: RustExpr,
    from: InferredType,
    to: InferredType,
    node: &TypedExpr,
) -> Result<RustExpr, Refusal> {
    use InferredType::*;
    // A value of `T` where an optional value of `T` is expected is a present one
    // (docs/adr/0055, section 2): converted to the element's type, borrowed as an
    // expression reads an optional text ([`rust_borrowed`]), and lifted. Asked
    // before the early return below: an enum's variant is a value this layer
    // does not type.
    if let Some(elem) = from.lifted_into(to) {
        let value = rust_coerce(raw, from, elem.present(), node)?;
        return Ok(rust_optional::present_expr(rust_borrowed(
            node, value, elem,
        )));
    }
    // A quantity is emitted as its numeric base: the unit is a check the
    // document passed, not a type Rust has. Left on, `raw_temp * 0.5` with
    // `raw_temp: i8` carrying `sce:quantity` fell through to the final arm and
    // lost the `as f64` that the same expression without the annotation has.
    let (from, to) = (from.strip_quantity(), to.strip_quantity());
    if from == to || matches!(to, Unknown) || matches!(from, Unknown) {
        return Ok(raw);
    }
    match (from, to) {
        // Literal promotion for untyped decimal integers into float context.
        (UntypedInt, Float { .. }) | (UntypedInt, UntypedFloat) => {
            if let ExprKind::NumberLit(text) = &node.kind {
                if is_decimal_integer_literal(text) {
                    // The literal itself, written as a real one.
                    return Ok(RustExpr::atom(format!(
                        "{}.0",
                        portable_number_literal(text)
                    )));
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
            Ok(raw.cast(target))
        }
        // Concrete int → float: explicit cast.
        (Int { .. }, Float { bits: 64 }) => Ok(raw.cast("f64")),
        (Int { .. }, Float { bits: 32 }) => Ok(raw.cast("f32")),
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
        ) if (s1, b1) != (s2, b2) => Ok(raw.cast(rust_int_type(s2, b2))),
        // Float widening or narrowing.
        (Float { bits: b1 }, Float { bits: b2 }) if b1 != b2 => {
            Ok(raw.cast(if b2 == 64 { "f64" } else { "f32" }))
        }
        _ => Ok(raw),
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
    // The context a quantity-annotated output asks for is its numeric base
    // (see `rust_coerce`), or the push-down below never sees a float.
    let expected = expected.strip_quantity();
    // `null` where an optional value is expected is its absent value, which Go
    // spells with the element's type (docs/adr/0055).
    if let (ExprKind::NullLit, InferredType::Optional(elem)) = (&expr.kind, expected) {
        return go_absent(elem).map_err(|e| e.at(expr.span.clone()));
    }
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
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
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
    let raw = match typed_integer_literal(ExprTarget::Go, expr) {
        Some(literal) => literal,
        None => go_emit_node(expr)?,
    };
    Ok(go_coerce(raw, expr.ty, expected, expr))
}

fn go_emit_node(expr: &TypedExpr) -> Result<String, Refusal> {
    if let Some(form) = optional_form(expr) {
        return go_optional(form, expr);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's function for the operation's own width.
        // Go defines the wrap of a typed operation, but folds an expression of
        // two constants at compile time and refuses one that overflows, so
        // the operation is a call, as the checked one is.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "Go", emit_go)?;
            format!(
                "scealgorithm.{}{}{}{bits}({})",
                crate::filters::capitalize_first(rule.helper()),
                crate::filters::capitalize_first(op.helper()),
                if signed { "Int" } else { "Uint" },
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => portable_number_literal(n),
        ExprKind::StringLit { value } => {
            format!("\"{}\"", crate::filters::escape_go(value.clone()))
        }
        // Standalone fallback only — the bytes-equality Binary branch
        // renders operands itself (as `string(x)` / a string literal) so
        // it needs no `bytes` import. RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => format!("[]byte{{{}}}", bytes_as_hex_list(bytes)),
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "nil".to_string(),
        // The spelling the function DECLARED the name with: one function spells
        // the parameter, the variable and every read, so a name that hides a
        // builtin is escaped in the read as it was in the declaration
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::Go.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // Go refuses `string + int64`, and `string(n)` is not the digits
            // of `n` but the code point `n`. `strconv` is the digits; the
            // file that holds this text imports it because the render door
            // (`LangCtx::render`) reads the package off the text.
            let operand = |side: &TypedExpr, is_left: bool| -> Result<String, Refusal> {
                let text = emit_go(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text => {
                        Ok(if concat_text_needs_parens(side, is_left, go_precedence) {
                            format!("({text})")
                        } else {
                            text
                        })
                    }
                    ConcatOperand::Decimal => Ok(match side.ty {
                        InferredType::Int { signed: false, .. } => {
                            format!("strconv.FormatUint(uint64({text}), 10)")
                        }
                        _ => format!("strconv.FormatInt(int64({text}), 10)"),
                    }),
                    ConcatOperand::Refused(what) => Err(concat_refusal(what, text).into()),
                }
            };
            format!("{} + {}", operand(left, true)?, operand(right, false)?)
        }
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
            let r_raw = emit_go(right, right_operand_type(*op, operand_ty))?;
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
            fails,
            ..
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
            let symbol = wrap_postfix(callee, emit_go(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Go, &symbol, &call, expr.ty)
                    .map_err(|e| e.at(expr.span.clone()))?
            } else {
                call
            }
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`string`) →
            // the algorithm `bytes` param type (`[]byte`).
            format!("[]byte({})", emit_go(source, InferredType::Unknown)?)
        }
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => go_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: an index read through `At` (a signed index,
        // widened to `int64`) or `AtU` (an unsigned one, to `uint64`), over
        // the collection's slice so a fixed array passes too.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust().at(expr.span.clone())),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => {
            let (helper, wide) = match index.ty {
                InferredType::Int { signed: false, .. } => ("AtU", "uint64"),
                _ => ("At", "int64"),
            };
            format!(
                "scealgorithm.{helper}(&sceFailure, {}[:], {wide}({}))",
                wrap_postfix(left, emit_go(left, InferredType::Unknown)?),
                emit_go(index, InferredType::Unknown)?,
            )
        }
        // SCE_FORGE.md §3.4.1: the runtime's helper for the operation's own
        // width, which records a failure in the body's `sceFailure` and
        // yields 0; the statement around it returns that failure. Go has no
        // overloading, so the width is in the helper's name, and each operand
        // is spelled at that width so the call type-checks.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                }
                .at(expr.span.clone()));
            };
            if *op == CheckedOp::Narrow {
                // The value converted, exactly, to `int64` or `uint64` by its
                // own signedness; one helper per target takes either.
                let InferredType::Int {
                    signed: from_signed,
                    ..
                } = left.ty
                else {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("a checked narrowing from {:?}", left.ty),
                        observed: None,
                    }
                    .at(expr.span.clone()));
                };
                let wide = if from_signed { "int64" } else { "uint64" };
                return Ok(format!(
                    "scealgorithm.Narrow{}{bits}From{}(&sceFailure, {wide}({}))",
                    if signed { "Int" } else { "Uint" },
                    if from_signed { "Int" } else { "Uint" },
                    emit_go(left, left.ty)?
                ));
            }
            let mut operands = vec!["&sceFailure".to_string(), emit_go(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit_go(right, expr.ty)?);
            }
            let helper = op.helper();
            format!(
                "scealgorithm.{}{}{}{bits}({})",
                helper[..1].to_ascii_uppercase(),
                &helper[1..],
                if signed { "Int" } else { "Uint" },
                operands.join(", ")
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in Go.
///
/// Go's own `<<` and `>>` are the total shift already: a count of any integer
/// type at or past the width shifts every bit out, leaving 0, or the sign for a
/// signed `>>` (The Go Programming Language Specification, "Arithmetic
/// operators"), so the total shift is the operator, parenthesised so it binds as
/// one operand, and a `>>>` of a signed value is made in the unsigned type of
/// its width. Go panics on a count below 0, which reaches a total shift only
/// outside the bodies the contract covers, where it is the caller's to rule out,
/// as a divisor of 0 is. A literal count at or past the width goes through the
/// runtime's `WrappingShl` and `WrappingShr` instead, which compute the same:
/// written as the operator, `go vet` reports the shift as too large for its type.
///
/// The checked shift is the runtime's helper for the width, which records an
/// out-of-range failure in the body's `sceFailure` for a count below 0 and
/// yields 0; the count is widened to the `int64` it takes.
fn go_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<String, Refusal> {
    let (signed, bits) = settled_shift_width(expr).map_err(|e| e.at(expr.span.clone()))?;
    let Some(operator) = op.shift() else {
        unreachable!("only a shift is settled as one");
    };
    let value = emit_go(left, expr.ty)?;
    let written = emit_go(count, InferredType::Unknown)?;
    let helper = op.helper();
    if discharge == Discharge::AtRun {
        return Ok(format!(
            "scealgorithm.{}{}{}{bits}(&sceFailure, {value}, int64({written}))",
            helper[..1].to_ascii_uppercase(),
            &helper[1..],
            if signed { "Int" } else { "Uint" },
        ));
    }
    // Only a literal count at or past the width is one `go vet` reports; a
    // negative literal never reaches a total shift (the analysis refuses it).
    let vet_refuses = integer_literal_parts(count).is_some_and(|(text, negated)| {
        !negated && integer_literal_value(&text).is_none_or(|n| n >= u64::from(bits))
    });
    let logical_of_signed = op == CheckedOp::UShr && signed;
    if logical_of_signed {
        let shifted = if vet_refuses {
            format!("scealgorithm.WrappingShr(uint{bits}({value}), {written})")
        } else {
            format!(
                "uint{bits}({value}) >> {}",
                go_operand(count, operator, false, written)
            )
        };
        return Ok(format!("int{bits}({shifted})"));
    }
    if vet_refuses {
        let family = if op == CheckedOp::Shl {
            "WrappingShl"
        } else {
            "WrappingShr"
        };
        return Ok(format!("scealgorithm.{family}({value}, {written})"));
    }
    Ok(format!(
        "({} {} {})",
        go_operand(left, operator, true, value),
        go_binop(operator),
        go_operand(count, operator, false, written)
    ))
}

/// `text`, the Go spelling of `child`, an operand of `op`, parenthesised where Go
/// would bind it to something else than the document does.
fn go_operand(child: &TypedExpr, op: BinOp, is_left: bool, text: String) -> String {
    if child_needs_parens(child, op, is_left, go_precedence) {
        format!("({text})")
    } else {
        text
    }
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
        BinOp::Coalesce => unreachable!("{COALESCE_NOT_SPELLED}"),
    }
}

/// The Go spelling of each construct of an optional value (docs/adr/0055,
/// section 5) over `optional.Of[T]` of the Go forge runtime
/// (`github.com/newmassrael/sce-forge-runtime/optional`, imported where the
/// text names it): `x.IsPresent()`, `x.Proven()`, `x.Equal(y)`, and `x ?? d` as
/// `optional.Otherwise(x, func() T { return d })`, which evaluates `x` once and
/// calls the function literal only when `x` is absent, so a failure `d` records
/// in the statement's `sceFailure` is recorded only then.
fn go_optional(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<String, Refusal> {
    let receiver = |node: &TypedExpr| -> Result<String, Refusal> {
        Ok(wrap_postfix(node, emit_go(node, InferredType::Unknown)?))
    };
    Ok(match form {
        OptionalForm::Test { subject, present } => {
            let test = format!("{}.IsPresent()", receiver(subject)?);
            if present {
                test
            } else {
                format!("!{test}")
            }
        }
        OptionalForm::Proven { subject } => format!("{}.Proven()", receiver(subject)?),
        OptionalForm::Otherwise { value, fallback } => {
            // The function literal names the type it answers, the type of the
            // whole read: an optional value where the default may be absent.
            let result = whole.ty;
            let helper = if result.is_optional() {
                "OtherwiseOptional"
            } else {
                "Otherwise"
            };
            let Some(type_name) = go_nameable_type(result) else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a read with a default (`??`) of {}, whose type Go cannot name here: \
                         test it with `!== null` and read `.value` in its branch",
                        value.ty.describe()
                    ),
                    observed: None,
                }
                .at(whole.span.clone()));
            };
            format!(
                "optional.{helper}({}, func() {type_name} {{ return {} }})",
                emit_go(value, InferredType::Unknown)?,
                emit_go(fallback, result)?
            )
        }
        OptionalForm::Equality { left, right, equal } => {
            // Each side as the optional value of the one that is: a side of `T`
            // is lifted ([`InferredType::lifted_into`]).
            let optional = [left.ty, right.ty]
                .into_iter()
                .find(|ty| ty.is_optional())
                .unwrap_or(InferredType::Unknown);
            let equality = format!(
                "{}.Equal({})",
                wrap_postfix(left, emit_go(left, optional)?),
                emit_go(right, optional)?
            );
            if equal {
                equality
            } else {
                format!("!{equality}")
            }
        }
        OptionalForm::Constant(answer) => answer.to_string(),
    })
}

/// The absent value of an optional value of `elem` in Go:
/// `optional.None[T]()`, which has to name `T`. An enum's type is the machine's
/// own, which the lattice does not carry: a `null` of one is written whole where
/// a variable or a field takes it, where the lowering names the type
/// ([`crate::forge::static_lowering::StaticTarget::optional_none`]).
fn go_absent(elem: OptionalElem) -> Result<String, ExprError> {
    match go_nameable_type(elem.present()) {
        Some(ty) => Ok(crate::forge::generator::go_optional_none(&ty)),
        None => Err(ExprError::UnsupportedConstruct {
            construct: "`null` of an optional value of an enum or a record inside an \
                        expression, whose type Go cannot name here: write it as the whole \
                        value of the variable, the field, the local or the return that takes \
                        it"
            .to_string(),
            observed: None,
        }),
    }
}

fn go_coerce(raw: String, from: InferredType, to: InferredType, node: &TypedExpr) -> String {
    use InferredType::*;
    // A value of `T` where an optional value of `T` is expected is lifted
    // (docs/adr/0055, section 2), converted to the element's type first. Asked
    // before the early return below: an enum's variant is a value this layer
    // does not type.
    if let Some(elem) = from.lifted_into(to) {
        let present = elem.present();
        let value = go_coerce(raw, from, present, node);
        return match go_nameable_type(present) {
            Some(ty) => format!("optional.Some[{ty}]({value})"),
            None => format!("optional.Some({value})"),
        };
    }
    // A quantity is emitted as its numeric base (see `rust_coerce`): Go has no
    // implicit int-to-float conversion, so `raw_temp * 0.5` with an annotated
    // `int8` input was `0.5 truncated to int8`.
    let (from, to) = (from.strip_quantity(), to.strip_quantity());
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
    let Some((result, type_name)) = [expected, conditional.ty]
        .into_iter()
        .find_map(|ty| go_nameable_type(ty).map(|name| (ty, name)))
    else {
        // No type to spell, and two branches that are each a pre-rendered
        // reference — a variant of an enum, which carries its type as a named
        // constant, or a read of a field. Go infers the type from the values,
        // and evaluating both costs nothing: neither can fail.
        if is_pure_reference(consequent) && is_pure_reference(alternate) {
            return Ok(format!(
                "scealgorithm.Choose({}, {}, {})",
                emit_go(condition, InferredType::Bool)?,
                emit_go(consequent, InferredType::Unknown)?,
                emit_go(alternate, InferredType::Unknown)?,
            ));
        }
        return Err(ExprError::GoTernary.at(conditional.span.clone()));
    };
    Ok(format!(
        "func() {type_name} {{ if {} {{ return {} }}; return {} }}()",
        emit_go(condition, InferredType::Bool)?,
        emit_go(consequent, result)?,
        emit_go(alternate, result)?,
    ))
}

/// Whether `expr` is a pre-rendered reference — a name, an enum variant — or a
/// conditional of nothing else: a value read, never computed, so evaluating it
/// when its branch is not taken can neither fail nor be seen.
fn is_pure_reference(expr: &TypedExpr) -> bool {
    match &expr.kind {
        ExprKind::Raw(_) => true,
        ExprKind::Conditional {
            consequent,
            alternate,
            ..
        } => is_pure_reference(consequent) && is_pure_reference(alternate),
        _ => false,
    }
}

/// What the name of the adapter a Go statechart declares for a call that hands an
/// algorithm a record or a list begins with, so that [`pass_failure_on`] knows the
/// callee answers an `Outcome` and not the `(value, error)` of a generated algorithm
/// (SCE_FORGE.md §4.12). No algorithm's call begins with it: a package is imported as
/// `sce_<snake_case document name>`, and this has a capital after `sce`.
pub(crate) const STATE_ADAPTER_PREFIX: &str = "sceAdapt_";

/// What the name of the adapter a Swift statechart declares for such a call begins
/// with ([`STATE_ADAPTER_PREFIX`] is Go's). An adapter is a `private` function of the
/// machine's file, called by its bare name, and a generated algorithm is called
/// through the enum that holds it (`Pascal.camel(…)`), so a bare name that begins with
/// this is an adapter, whose answer is a type the lattice does not name.
pub(crate) const SWIFT_STATE_ADAPTER_PREFIX: &str = "call_";

/// The empty destination a Rust caller hands an algorithm that returns a list
/// (docs/adr/0082). Its element and bound are the algorithm's parameter's, which the
/// call infers.
pub(crate) const RUST_LIST_DESTINATION: &str = "sce_portable_bytes::SceOwnedList::new()";

/// `call` — a call to a `may-fail` algorithm, spelled `symbol(args)` in
/// `target` — as the calling `may-fail` body passes its failure on and keeps
/// its value of type `ty` (SCE_FORGE.md §3.4.1). ONE spelling for the
/// expression form and the `<sce:call>` statement.
///
/// The body's failure channel is the one its own checked arithmetic writes:
/// Rust's `?`, C++'s and C11's recorded `sce_failure_` (the statement after
/// returns it, and a condition is computed before its branch), Go's
/// `sceFailure`, Kotlin's `AlgorithmFailure` thrown to the body's own
/// boundary, and Python's exception, which passes itself on.
pub(crate) fn pass_failure_on(
    target: ExprTarget,
    symbol: &str,
    call: &str,
    ty: InferredType,
) -> Result<String, ExprError> {
    Ok(match target {
        ExprTarget::Rust => format!("{call}?"),
        ExprTarget::Kotlin => format!("com.sce.forge.runtime.SceChecked.take({call})"),
        ExprTarget::Cpp => format!("SCE::Forge::Checked::take(sce_failure_, {call})"),
        // The callee's header declares the unwrapping of its own result type.
        ExprTarget::C => format!("{symbol}_take(&sce_failure_, {call})"),
        // A call of an adapter a statechart declares for an algorithm that takes or
        // returns a record or a list answers in the statechart's own types, which
        // this layer cannot spell (the lattice does not carry which record a list
        // holds): it answers an `Outcome`, whose type `TakeOutcome` infers.
        // ... and so does the runtime's reading of a value that may not have been
        // found ([`found_wrapper`]), the variant a number declares among them,
        // whose type is the enum's, which the lattice does not carry either.
        ExprTarget::Go if symbol.starts_with(STATE_ADAPTER_PREFIX) || symbol == GO_FOUND => {
            format!("scealgorithm.TakeOutcome(&sceFailure, {call})")
        }
        ExprTarget::Go => {
            let Some(value) = go_nameable_type(ty.strip_quantity()) else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a may-fail call returning {}", ty.describe()),
                    observed: None,
                });
            };
            format!("scealgorithm.Take[{value}](&sceFailure)({call})")
        }
        // The callee throws its failure, which is the caller's by `try` alone: no
        // value of any type stands where it failed (docs/adr/0099), so a call that
        // answers a record, a list, an enum or a union is spelled as one that
        // answers a number.
        ExprTarget::Swift => swift::throwing(call),
        ExprTarget::Python => call.to_string(),
        // A failure is a throw, which passes itself on as Python's does: the
        // expression that called the algorithm fails as an overflow of its own
        // does, and the Interpreter answers that with `error.execution`.
        ExprTarget::Js => call.to_string(),
    })
}

/// The Go spelling of a type a function literal can return, or `None` for a
/// type that has no single spelling here (untyped literals, opaque values).
/// A record is its schema's payload struct, spelled by the one rule the
/// struct was emitted by.
fn go_nameable_type(ty: InferredType) -> Option<String> {
    Some(match ty {
        InferredType::Int { signed, bits } => go_int_type(signed, bits).to_string(),
        InferredType::Float { bits: 32 } => "float32".to_string(),
        InferredType::Float { .. } => "float64".to_string(),
        InferredType::Bool => "bool".to_string(),
        InferredType::Str => "string".to_string(),
        InferredType::Bytes => "[]byte".to_string(),
        // The slice an algorithm's list return is, of the element it names.
        InferredType::List(elem) => format!("[]{}", go_nameable_type(elem.element_type())?),
        InferredType::Record(id) => crate::forge::generator::event_schema_payload_type(
            id.schema_name(),
            crate::generator::Language::Go,
        ),
        // The backend's one spelling of an optional value (docs/adr/0055) over
        // its element's, where the lattice names the element.
        InferredType::Optional(elem) => crate::forge::generator::optional_of(
            crate::generator::Language::Go,
            &go_nameable_type(elem.present())?,
        )?,
        _ => return None,
    })
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
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
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
    if let Some(form) = optional_form(expr) {
        return python_optional(form, expr);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's method for the operation's own width,
        // which computes exactly and reduces the result: Python's integers
        // never overflow, so this is where its `uint32` wraps.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "Python", emit_python)?;
            format!(
                "sce_algorithm.{}{bits}.{}_{}({})",
                if signed { "I" } else { "U" },
                rule.helper(),
                op.helper(),
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => portable_number_literal(n),
        ExprKind::StringLit { value } => crate::filters::py_string_literal(value.clone()),
        // `bytes == bytes` compares content, so the default `==`/`!=`
        // path needs no special-casing — only this constant rendering.
        // A `b"…"` literal (double-quoted so a `'` byte needs no escape).
        // RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => format!("b\"{}\"", bytes_as_quoted_ascii(bytes)),
        ExprKind::BoolLit(b) => if *b { "True" } else { "False" }.to_string(),
        ExprKind::NullLit => "None".to_string(),
        // The spelling the function DECLARED the name with: the same function
        // spells its parameter and its variable, so a name that hides a
        // builtin is escaped in the read as it was in the declaration
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::Python.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // `'E' + 5` is a `TypeError` in Python; `str(int)` is the
            // decimal digits, the one text ECMA-262 13.15.3 gives an integer.
            let operand = |side: &TypedExpr, is_left: bool| -> Result<String, ExprError> {
                let text = emit_python(side, side.ty)?;
                match concat_operand(side.ty) {
                    ConcatOperand::Text => Ok(
                        if concat_text_needs_parens(side, is_left, python_precedence) {
                            format!("({text})")
                        } else {
                            text
                        },
                    ),
                    ConcatOperand::Decimal => Ok(format!("str({text})")),
                    ConcatOperand::Refused(what) => Err(concat_refusal(what, text)),
                }
            };
            format!("{} + {}", operand(left, true)?, operand(right, false)?)
        }
        ExprKind::Binary { op, left, right } => {
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_python(left, operand_ty)?;
            let r_raw = emit_python(right, right_operand_type(*op, operand_ty))?;
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
            //
            // A left shift wraps at its width whatever the width and sign
            // (SCE_FORGE.md §3.4.1, "The width of a shift"): until 2026-11-19
            // only an unsigned one of at most 32 bits was reduced, so a
            // `uint64` or a signed shift kept every bit it shifted out.
            if *op == BinOp::Shl {
                if let InferredType::Int { signed, bits } = expr.ty {
                    return Ok(python_reduce(raw, signed, bits));
                }
            }
            if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) {
                if let InferredType::Int {
                    signed: false,
                    bits,
                } = expr.ty
                {
                    if bits <= 32 {
                        return Ok(python_reduce(raw, false, bits));
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
            let spelled = if wrap {
                format!("{prefix}({inner})")
            } else {
                format!("{prefix}{inner}")
            };
            // Python's `~x` is `-x - 1`, a negative number for every `x` at
            // least 0, where an unsigned type holds the complement of `x`'s
            // bits: it is reduced into the type, as one operand.
            match (op, expr.ty) {
                (
                    UnaryOp::BitNot,
                    InferredType::Int {
                        signed: false,
                        bits,
                    },
                ) => python_reduce(spelled, false, bits),
                _ => spelled,
            }
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            // Python's conditional expression is `A if C else B`, and only
            // the ELSE branch may hold another one unparenthesised -- it
            // associates to the right. A conditional in the consequent or in
            // the CONDITION must be wrapped: `x if a if g else b else 0` is a
            // SyntaxError, which a document reading `(g ? a : b) ? x : 0`
            // produced (2026-09-28) while every other target compiled.
            let wrap = |e: &TypedExpr, s: String| {
                if matches!(&e.kind, ExprKind::Conditional { .. }) {
                    format!("({s})")
                } else {
                    s
                }
            };
            let cons = wrap(consequent, emit_python(consequent, expr.ty)?);
            let cond = wrap(condition, emit_python(condition, InferredType::Bool)?);
            format!("{cons} if {cond} else {}", emit_python(alternate, expr.ty)?,)
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
            fails,
            ..
        } => {
            if is_len_builtin(callee, args) {
                let measured = emit_python(&args[0], InferredType::Unknown)?;
                // A string is measured in the UTF-8 bytes its bound counts, which
                // `len` of a `str` (code points) is not; a list and `bytes` by `len`.
                return Ok(if args[0].ty == InferredType::Str {
                    format!("len(({measured}).encode(\"utf-8\"))")
                } else {
                    format!("len({measured})")
                });
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
            let symbol = wrap_postfix(callee, emit_python(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Python, &symbol, &call, expr.ty)?
            } else {
                call
            }
        }
        ExprKind::BytesView { source, .. } => {
            // RFC c7-wildcard W-project: bounded-string field (`str`) → the
            // algorithm `bytes` param type (`bytes`), UTF-8 encoded.
            format!(
                "{}.encode(\"utf-8\")",
                emit_python(source, InferredType::Unknown)?
            )
        }
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => python_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: an index read through the runtime's `at`, which
        // refuses a negative index Python would otherwise read from the end.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust()),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => format!(
            "sce_algorithm.at({}, {})",
            emit_python(left, InferredType::Unknown)?,
            emit_python(index, InferredType::Unknown)?,
        ),
        // SCE_FORGE.md §3.4.1: the runtime's method for the operation's own
        // width, which computes exactly and raises `AlgorithmFailure` for a
        // result the width does not hold. Python's integers never overflow,
        // so this is the one place its `int32` is held to 32 bits.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                });
            };
            // A narrowing holds the value, at its own type, to the target's
            // width; the operations compute at the width they are checked at.
            let operand_ty = if *op == CheckedOp::Narrow {
                left.ty
            } else {
                expr.ty
            };
            let mut operands = vec![emit_python(left, operand_ty)?];
            if let Some(right) = right {
                operands.push(emit_python(right, expr.ty)?);
            }
            format!(
                "sce_algorithm.{}{bits}.{}({})",
                if signed { "I" } else { "U" },
                op.helper(),
                operands.join(", ")
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in Python.
///
/// Python's own `>>` is the total right shift already: `x >> n` is the floor of
/// `x / 2**n` for every `n` at least 0, so a count at or past the width leaves 0,
/// or -1 for a negative value, and the value of a width is held reduced to it.
/// It is written as the operator, parenthesised so it binds as one operand, for
/// a `>>` and for a `>>>` of an unsigned value. A left shift is the runtime's
/// `wrapping_shl` of the width, which is 0 for a count at or past the width: the
/// operator would compute `x * 2**n` whole before reducing it, a number of `n`
/// bits for a count as large as an unsigned type holds. A `>>>` of a signed
/// value is the runtime's `wrapping_ushr`, which shifts the bits of its width.
/// Python raises `ValueError` on a count below 0, which reaches a total shift
/// only outside the bodies the contract covers, where it is the caller's to rule
/// out, as a divisor of 0 is.
///
/// The checked shift is the runtime's `shl`, `shr` or `ushr` of the width,
/// which raises `AlgorithmFailure` with `out-of-range` for a count below 0.
fn python_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<String, ExprError> {
    let (signed, bits) = settled_shift_width(expr)?;
    let width = format!("sce_algorithm.{}{bits}", if signed { "I" } else { "U" });
    let value = emit_python(left, expr.ty)?;
    let places = emit_python(count, InferredType::Unknown)?;
    let native = op == CheckedOp::Shr || (op == CheckedOp::UShr && !signed);
    Ok(match discharge {
        Discharge::AtRun => format!("{width}.{}({value}, {places})", op.helper()),
        Discharge::AtBuild | Discharge::Total if native => {
            let parenthesised = |child: &TypedExpr, is_left: bool, text: String| {
                if child_needs_parens(child, BinOp::Shr, is_left, python_precedence) {
                    format!("({text})")
                } else {
                    text
                }
            };
            format!(
                "({} >> {})",
                parenthesised(left, true, value),
                parenthesised(count, false, places)
            )
        }
        Discharge::AtBuild | Discharge::Total => {
            format!("{width}.wrapping_{}({value}, {places})", op.helper())
        }
    })
}

/// The Python spelling of each construct of an optional value (docs/adr/0055,
/// section 5), held as `Optional[T]`: `None` absent and the value itself
/// present. Presence is an identity test against `None` and never a truth test,
/// since a present `0`, `''` or `False` is falsy: `x is not None`, a proven
/// `x.value` is `x`, and `x ?? d` is the runtime's
/// `sce_algorithm.otherwise(x, lambda: d)`, which reads `x` once as its argument
/// and calls the `lambda` only when `x` is absent, so a failure of `d` is raised
/// only then. Equality is `==`, which is total over `None`.
fn python_optional(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<String, ExprError> {
    // An operand of a comparison, bracketed where its own operator binds less
    // tightly than one.
    let operand =
        |node: &TypedExpr, expected: InferredType, is_left: bool| -> Result<String, ExprError> {
            let text = emit_python(node, expected)?;
            Ok(
                if child_needs_parens(node, BinOp::StrictEq, is_left, python_precedence) {
                    format!("({text})")
                } else {
                    text
                },
            )
        };
    Ok(match form {
        OptionalForm::Test { subject, present } => {
            let test = if present { "is not None" } else { "is None" };
            format!("{} {test}", operand(subject, subject.ty, true)?)
        }
        OptionalForm::Proven { subject } => emit_python(subject, InferredType::Unknown)?,
        OptionalForm::Otherwise { value, fallback } => format!(
            "sce_algorithm.otherwise({}, lambda: {})",
            emit_python(value, value.ty)?,
            emit_python(fallback, whole.ty)?
        ),
        OptionalForm::Equality { left, right, equal } => {
            // Each side as the optional value of the one that is: a side of `T`
            // is lifted ([`InferredType::lifted_into`]).
            let optional = [left.ty, right.ty]
                .into_iter()
                .find(|ty| ty.is_optional())
                .unwrap_or(InferredType::Unknown);
            let op = if equal { "==" } else { "!=" };
            format!(
                "{} {op} {}",
                operand(left, optional, true)?,
                operand(right, optional, false)?
            )
        }
        OptionalForm::Constant(answer) => if answer { "True" } else { "False" }.to_string(),
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
    // A value of `T` where an optional value of `T` is expected is lifted
    // (docs/adr/0055, section 2): Python holds a present value as the value
    // itself, so the lift is the conversion to the element's type alone. Asked
    // before the early return below: an enum's variant is a value this layer
    // does not type.
    if let Some(elem) = from.lifted_into(to) {
        return python_coerce(raw, from, elem.present());
    }
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
    python_reduce(raw, signed, bits)
}

/// `raw`, any Python integer, reduced modulo 2^bits into the integer type of
/// that width and sign — its two's-complement value for a signed one.
///
/// The reduction is one parenthesised operand, so it binds as the value it is
/// under any operator its node stands beside: the node is still the operation
/// or the read it reduces, and the parentheses a parent gives a child are
/// decided by that node, not by the `&` and `-` written around it.
///
/// ⚠ Until 2026-11-19 it was written bare, `(x) & M`: under a `>>` it read
/// `(x) & (M >> n)`, under a `+` `(x) & (M + n)`, and under a comparison
/// `(x) & (M == n)`, since Python's `&` binds looser than all three.
fn python_reduce(raw: String, signed: bool, bits: u8) -> String {
    let mask: u128 = (1u128 << bits) - 1;
    if signed {
        let half: u128 = 1u128 << (bits - 1);
        format!("(((({raw}) + 0x{half:X}) & 0x{mask:X}) - 0x{half:X})")
    } else {
        format!("(({raw}) & 0x{mask:X})")
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
        BinOp::Coalesce => unreachable!("{PYTHON_COALESCE_IS_A_CALL}"),
    }
}

/// Why Python's operator table does not spell `??`: [`python_optional`]
/// writes it as a call of the runtime's `otherwise` before any operator is
/// looked up, because no Python operator evaluates its right side only when
/// its left one is `None`.
const PYTHON_COALESCE_IS_A_CALL: &str =
    "`??` is written as `sce_algorithm.otherwise` before an operator is looked up";

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Emitter — ECMAScript (the Interpreter's script engine)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//
// The Interpreter hands every expression to its script engine, which reads
// ECMAScript. A `sce-static` document is ECMAScript with types the engine does
// not hold, so this emitter writes what the document means in ECMAScript's own
// terms:
//
// * Every integer is a Number while a Number holds it exactly, and a BigInt
//   past that. A checked operation is the runtime library's
//   (`SceStatic.<I|U><bits>.<helper>`), which computes exactly and hands back a
//   Number where it is a safe integer, a BigInt where it is not, and throws
//   where the type does not hold it — the failure the generated backends record
//   (docs/SCE_ACCEPTED_SUBSET.md §2.15). One value has one form, so `===`
//   between two integers is the comparison the document wrote, and an
//   operation at 64 bits is the library's whether or not a proof says it cannot
//   fail: a bare `+` of a BigInt and a Number throws a TypeError.
// * Names stand as authored: the document's variables are the script engine's
//   globals under the ids the document declares.
// * A construct whose ECMAScript meaning is not the document's is REFUSED, not
//   passed through. The bitwise operators are written as the library's, at
//   their width: a Number's own are 32-bit and signed, so `x & 0xFFFFFFFF` would
//   read as -1 where every backend reads 4294967295.

/// The largest integer a Number holds exactly, 2^53 - 1 — the largest an integer
/// literal is written as a Number, and the largest the runtime library hands
/// back as one.
const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

fn js_binop(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::StrictEq => "===",
        BinOp::StrictNeq => "!==",
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
        BinOp::UShr => ">>>",
        BinOp::Coalesce => unreachable!("{COALESCE_NOT_SPELLED}"),
    }
}

/// A number literal as ECMAScript holds it.
///
/// An integer is written as its decimal value — never as the author's text,
/// because a decimal with a leading zero (`017`) reads as octal in sloppy-mode
/// ECMAScript while [`integer_literal_value`], which every range check reads,
/// takes it as seventeen. One a Number holds exactly is a Number, and one it
/// does not is a BigInt literal (`9223372036854775807n`): rounded to a Number it
/// would be a different integer, and the generated backends keep it.
fn js_number_literal(text: &str) -> Result<String, ExprError> {
    let is_integer = matches!(text.get(..2), Some("0x" | "0X" | "0b" | "0B" | "0o" | "0O"))
        || text.bytes().all(|b| b.is_ascii_digit());
    if !is_integer {
        return Ok(text.to_string());
    }
    match integer_literal_value(text) {
        Some(value) if value <= JS_MAX_SAFE_INTEGER => Ok(value.to_string()),
        Some(value) => Ok(format!("{value}n")),
        None => Err(ExprError::UnsupportedConstruct {
            construct: format!(
                "the integer literal {text} in an ecmascript lowering: it is past `u64`, \
                 the widest integer any backend declares"
            ),
            observed: Some(text.to_string()),
        }),
    }
}

/// Whether `node` is an integer: typed as one, or a literal with no type of its
/// own.
fn js_is_integer(node: &TypedExpr) -> bool {
    matches!(node.ty, InferredType::Int { .. } | InferredType::UntypedInt)
}

/// Whether the integer `node` may be a BigInt: a 64-bit one, or a literal a
/// Number cannot hold.
fn js_may_be_big(node: &TypedExpr) -> bool {
    match &node.kind {
        ExprKind::NumberLit(text) => {
            integer_literal_value(text).is_some_and(|value| value > JS_MAX_SAFE_INTEGER)
        }
        _ => matches!(node.ty, InferredType::Int { bits: 64, .. }),
    }
}

/// The library's operation for a bitwise operator, and the width it is held to.
fn js_bitwise_helper(op: BinOp) -> &'static str {
    match op {
        BinOp::BitAnd => "and",
        BinOp::BitOr => "or",
        BinOp::BitXor => "xor",
        BinOp::Shl => "shl",
        BinOp::Shr => "shr",
        BinOp::UShr => "ushr",
        _ => unreachable!("only a bitwise operator is asked for"),
    }
}

/// The library's namespace for an integer type — `U16`, `I64` — or the refusal
/// for one it has no namespace for. An integer with no type of its own (two
/// literals) is a 64-bit signed one: the widest that holds both.
fn js_integer_namespace(ty: InferredType, what: &str) -> Result<String, ExprError> {
    use crate::forge::static_js::{INTEGER_WIDTHS, RUNTIME_GLOBAL};
    let (signed, bits) = match ty {
        InferredType::Int { signed, bits } => (signed, bits),
        InferredType::UntypedInt => (true, 64),
        other => {
            return Err(ExprError::UnsupportedConstruct {
                construct: format!("{what} on {} in an ecmascript lowering", other.describe()),
                observed: None,
            })
        }
    };
    if !INTEGER_WIDTHS.contains(&bits) {
        return Err(ExprError::UnsupportedConstruct {
            construct: format!(
                "{what} on a {bits}-bit integer in an ecmascript lowering: the library \
                 implements {INTEGER_WIDTHS:?}"
            ),
            observed: None,
        });
    }
    Ok(format!(
        "{RUNTIME_GLOBAL}.{}{bits}",
        if signed { "I" } else { "U" }
    ))
}

fn emit_js(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    use crate::forge::static_js::{INTEGER_WIDTHS, PAYLOAD_ACCESSOR, RUNTIME_GLOBAL};
    // An optional value (docs/adr/0055) is `null` where it is absent and the
    // value itself where it is present, so `null` is the literal, a value of `T`
    // lifted into `optional<T>` is the value, and a test, an equality and
    // `null === null` are ECMAScript's own `===` and `!==` (an integer has one
    // form, so equal values are the same kind of value), written below as any
    // comparison is.
    match optional_form(expr) {
        // A read where the build has proven the value present is the value as
        // it stands.
        Some(OptionalForm::Proven { subject }) => return emit_js(subject, expected),
        // `x ?? d` is the library's `otherwise`, handed `d` as the function it is
        // written in: `x` is evaluated once, as the argument, and `d` only when
        // `x` is absent, so a default that fails fails only then. Not the
        // engine's own `??`, which the lowered document would then need an
        // engine of that edition to read.
        Some(OptionalForm::Otherwise { value, fallback }) => {
            return Ok(format!(
                "{RUNTIME_GLOBAL}.otherwise({}, function () {{ return {}; }})",
                emit_js(value, value.ty)?,
                emit_js(fallback, expr.ty.unwrap_optional())?
            ))
        }
        Some(
            OptionalForm::Test { .. } | OptionalForm::Equality { .. } | OptionalForm::Constant(_),
        )
        | None => {}
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the library's method for the operation's own width,
        // which computes exactly and reduces the result into the width, so it
        // never throws.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "Interpreter", emit_js)?;
            if !INTEGER_WIDTHS.contains(&bits) {
                return Err(total_unlowered(*op, *rule, "Interpreter"));
            }
            format!(
                "{RUNTIME_GLOBAL}.{}{bits}.{}({})",
                if signed { "I" } else { "U" },
                camel_helper(*op, *rule),
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => js_number_literal(n)?,
        // The value, written as an ECMAScript literal again: the lexer read the
        // author's escapes, so the spelling is the one escaper's, not the
        // author's.
        ExprKind::StringLit { value } => crate::filters::ecmascript_string_literal(value),
        // An algorithm holds bytes as an array of numbers, which no literal spells
        // ([`bytes_literals_as_text`] makes a statechart's data model's constants
        // text before they reach here).
        ExprKind::BytesLit { .. } | ExprKind::BytesView { .. } => {
            return Err(ExprError::UnsupportedConstruct {
                construct: "a `bytes` value in an ecmascript lowering".to_string(),
                observed: None,
            })
        }
        ExprKind::BoolLit(b) => b.to_string(),
        ExprKind::NullLit => "null".to_string(),
        ExprKind::Ident(s) | ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } => {
            // Bitwise operations and shifts are outside the integer contract and
            // wrap at their width (SCE_FORGE.md §3.4.1), which is the library's
            // method of the node's own type.
            if op.is_bitwise() {
                let namespace = js_integer_namespace(
                    expr.ty,
                    &format!("the bitwise operator `{}`", js_binop(*op)),
                )?;
                return Ok(format!(
                    "{namespace}.{}({}, {})",
                    js_bitwise_helper(*op),
                    emit_js(left, expr.ty)?,
                    emit_js(right, expr.ty)?
                ));
            }
            // A real division is `/`; between two integers it is the truncating
            // pair the generated backends compute (SCE_FORGE.md §3.4.1), which
            // is not what `/` gives a Number.
            let operand_ty = if op.is_arith() && matches!(expected, InferredType::Float { .. }) {
                expected
            } else {
                binary_operand_type(*op, left.ty, right.ty)
            };
            // An integer operation at 64 bits is the library's even where
            // nothing checks it: either operand may be a BigInt, and a bare
            // operator over one throws. The library's result is the same
            // value, held to the same type.
            if op.is_arith()
                && matches!(operand_ty, InferredType::Int { bits: 64, .. })
                && !matches!(left.ty, InferredType::Float { .. })
                && !matches!(right.ty, InferredType::Float { .. })
            {
                let namespace = js_integer_namespace(operand_ty, "an integer operation")?;
                let helper = CheckedOp::of_binary(*op)
                    .expect("an arithmetic operator has a checked form")
                    .helper();
                return Ok(format!(
                    "{namespace}.{helper}({}, {})",
                    emit_js(left, operand_ty)?,
                    emit_js(right, operand_ty)?
                ));
            }
            // Two integers ordered by `<` where one may be a BigInt are ordered
            // as BigInts. The engine orders a BigInt against a Number by
            // comparing the magnitudes of the two, and for two negative values
            // that is the wrong way round (QuickJS, `js_bigint_float64_cmp`:
            // `-2^62 > -1790503200000` is true there). Equality needs no such
            // care: an integer has one form, so equal integers are the same
            // kind of value.
            let ordered_as_big = matches!(op, BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq)
                && js_is_integer(left)
                && js_is_integer(right)
                && (js_may_be_big(left) || js_may_be_big(right));
            // A 64-bit integer that meets a real is the real nearest to it, as
            // on every backend (`Number(big)` rounds to nearest, ties to even),
            // and a bare operator over a BigInt and a real throws a TypeError.
            let as_real = |child: &TypedExpr, text: String| {
                if matches!(operand_ty, InferredType::Float { .. })
                    && matches!(child.ty, InferredType::Int { bits: 64, .. })
                {
                    format!("Number({text})")
                } else {
                    text
                }
            };
            let operand = |child: &TypedExpr, left_side: bool| -> Result<String, ExprError> {
                let text = emit_js(child, operand_ty)?;
                if ordered_as_big {
                    return Ok(format!("BigInt({text})"));
                }
                let converted = as_real(child, text.clone());
                if converted != text {
                    return Ok(converted);
                }
                Ok(
                    if child_needs_parens(child, *op, left_side, ecma_precedence) {
                        format!("({text})")
                    } else {
                        text
                    },
                )
            };
            let l = operand(left, true)?;
            let r = operand(right, false)?;
            let integer_operands = matches!(
                operand_ty,
                InferredType::Int { .. } | InferredType::UntypedInt
            );
            if *op == BinOp::Div && integer_operands {
                format!("Math.trunc({l} / {r})")
            } else {
                format!("{l} {} {r}", js_binop(*op))
            }
        }
        ExprKind::Unary { op, operand } => {
            let prefix = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Pos => "+",
                UnaryOp::Not => "!",
                UnaryOp::BitNot => {
                    let namespace = js_integer_namespace(expr.ty, "the bitwise operator `~`")?;
                    return Ok(format!("{namespace}.not({})", emit_js(operand, expr.ty)?));
                }
            };
            let inner = emit_js(operand, expr.ty)?;
            // `- -x` is one token, `--x`, in ECMAScript: a sign in front of a
            // sign is wrapped as an operand of an operator is.
            let sign = matches!(op, UnaryOp::Neg | UnaryOp::Pos);
            let wrap = matches!(
                &operand.kind,
                ExprKind::Binary { .. } | ExprKind::Conditional { .. }
            ) || (sign && matches!(&operand.kind, ExprKind::Unary { .. }));
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
            let wrap = |e: &TypedExpr, s: String| {
                if matches!(&e.kind, ExprKind::Conditional { .. }) {
                    format!("({s})")
                } else {
                    s
                }
            };
            format!(
                "{} ? {} : {}",
                wrap(condition, emit_js(condition, InferredType::Bool)?),
                wrap(consequent, emit_js(consequent, expr.ty)?),
                emit_js(alternate, expr.ty)?,
            )
        }
        // A field of the event's data. Under `sce-static` it is typed by the
        // event's schema, and an event's data reaches the Interpreter as
        // untyped JSON, so the read goes through the library's `field`, which
        // refuses what a generated machine's lift of the payload refuses. An
        // optional field is read by the walk's rename of its path instead
        // (`JsTarget::payload_optional_read`), as an enum field is.
        ExprKind::Member { object, property } if matches!(&object.kind, ExprKind::Raw(accessor) if accessor == PAYLOAD_ACCESSOR) =>
        {
            let Some(kind) = expr.ty.declared_spelling() else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a read of the payload field `{property}`, of a type an ecmascript \
                         lowering has no reader for ({})",
                        expr.ty.describe()
                    ),
                    observed: Some(format!("{PAYLOAD_ACCESSOR}.{property}")),
                });
            };
            format!("{RUNTIME_GLOBAL}.field({PAYLOAD_ACCESSOR}, '{property}', '{kind}')")
        }
        ExprKind::Member { object, property } => format!(
            "{}.{property}",
            wrap_postfix(object, emit_js(object, InferredType::Unknown)?)
        ),
        ExprKind::Index { object, index } => format!(
            "{}[{}]",
            wrap_postfix(object, emit_js(object, InferredType::Unknown)?),
            emit_js(index, InferredType::Unknown)?,
        ),
        ExprKind::Call {
            callee,
            args,
            params,
            fails,
            ..
        } => {
            if is_len_builtin(callee, args) {
                let measured = emit_js(&args[0], InferredType::Unknown)?;
                // A string is measured in the UTF-8 bytes its bound counts, which
                // `length` (UTF-16 units) is not; a list and a byte string (one
                // character to a byte) by their `length`.
                return Ok(if args[0].ty == InferredType::Str {
                    format!("{RUNTIME_GLOBAL}.utf8Length({measured})")
                } else {
                    format!("{}.length", wrap_postfix(&args[0], measured))
                });
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // ⚠ NOT `Math.round`, which rounds half toward +∞: `Math.round(-0.5)`
                // is -0 and `Math.round(-2.5)` is -2, where the generated backends
                // round half away from zero. The library's `round` is that rule;
                // `Math.floor` goes toward -∞ everywhere and needs no rewrite.
                let inner = emit_js(&args[0], InferredType::Float { bits: 64 })?;
                // The result is an integer, so it has an integer's one form: a
                // real past 2^53 is a whole number a Number holds, and a BigInt
                // is how the rest of the lowering holds it (`fromReal`).
                return Ok(match op {
                    RealToInt::Round => format!("{RUNTIME_GLOBAL}.round({inner})"),
                    RealToInt::Floor => {
                        format!("{RUNTIME_GLOBAL}.fromReal(Math.floor({inner}))")
                    }
                });
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_js(arg, argument_type(params, i))?);
            }
            let symbol = wrap_postfix(callee, emit_js(callee, InferredType::Unknown)?);
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::Js, &symbol, &call, expr.ty)?
            } else {
                call
            }
        }
        // SCE_FORGE.md §3.4.1: an index read through the library's `at`, which
        // refuses an index outside the collection where a bare read would give
        // `undefined`.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust()),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => format!(
            "{RUNTIME_GLOBAL}.at({}, {})",
            emit_js(left, InferredType::Unknown)?,
            emit_js(index, InferredType::Unknown)?,
        ),
        // SCE_FORGE.md §3.4.1: the library's method for the operation's own
        // width, which computes exactly and throws for a result the width does
        // not hold.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                });
            };
            if !INTEGER_WIDTHS.contains(&bits) {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!(
                        "a checked operation on a {bits}-bit integer in an ecmascript \
                         lowering: the library implements {INTEGER_WIDTHS:?}"
                    ),
                    observed: None,
                });
            }
            let operand_ty = if *op == CheckedOp::Narrow {
                left.ty
            } else {
                expr.ty
            };
            let mut operands = vec![emit_js(left, operand_ty)?];
            if let Some(right) = right {
                operands.push(emit_js(right, expr.ty)?);
            }
            format!(
                "{RUNTIME_GLOBAL}.{}{bits}.{}({})",
                if signed { "I" } else { "U" },
                op.helper(),
                operands.join(", ")
            )
        }
    })
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

/// The member a C11 index reads a collection's elements through: a `bytes`
/// value or a `list<T>` parameter is a view (`{const T *data; size_t len}`),
/// a buffer the algorithm is building is its own result struct
/// (`{T items[N]; size_t len; …}` for a list, `{uint8_t bytes[N]; size_t
/// len; …}` for bytes), and a build-time array is indexed as it is. Every
/// named member carries the length as `len`.
fn c_element_accessor(collection: InferredType) -> &'static str {
    match collection {
        InferredType::Bytes | InferredType::List(_) => ".data",
        InferredType::ListBuffer(_) => ".items",
        InferredType::BytesBuffer => ".bytes",
        _ => "",
    }
}

fn emit_c(expr: &TypedExpr, expected: InferredType) -> Result<String, ExprError> {
    // `null` where an optional value is expected is its absent value, and a value
    // of `T` there is lifted into it (docs/adr/0055, section 2), each by the
    // constructor of the element's type. Asked before anything coerces the value:
    // an enum's variant is a value this layer does not type.
    if let (ExprKind::NullLit, InferredType::Optional(elem)) = (&expr.kind, expected) {
        return c_optional_constructor(elem, None);
    }
    if let Some(elem) = expr.ty.lifted_into(expected) {
        let present = emit_c(expr, elem.present())?;
        return c_optional_constructor(elem, Some(&present));
    }
    // Push-down: arithmetic + Float expectation propagates into operands so
    // decimal-integer literals pick up `.0` and avoid integer division.
    if let ExprKind::Binary { op, left, right } = &expr.kind {
        if op.is_arith()
            && !is_string_concatenation(*op, left.ty, right.ty)
            && matches!(expected, InferredType::Float { .. })
        {
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
    // Push-down: `len(x)` is a `size_t`, so against a signed integer it is a sign
    // comparison C refuses under `-Werror` (`i < len(xs)` with `i: int64`). In a
    // signed context the length is cast to that width where the context is known,
    // as the Rust and Go push-downs of the same builtin do; an unsigned one meets
    // `size_t` without a diagnostic and is left as it is written.
    if let ExprKind::Call { callee, args, .. } = &expr.kind {
        if is_len_builtin(callee, args) && matches!(expr.ty, InferredType::UntypedInt) {
            if let InferredType::Int { signed: true, bits } = expected {
                return Ok(format!("(int{bits}_t){}", c_length(&args[0])?));
            }
        }
    }
    let raw = match typed_integer_literal(ExprTarget::C, expr) {
        Some(literal) => literal,
        None => c_emit_node(expr)?,
    };
    Ok(c_coerce(raw, expr.ty, expected, expr))
}

/// What a checked index read in C gives when it records a failure. The statement that
/// holds the read is skipped on a failure, so the value is never used; it only has to be
/// something the conditional's two arms can share and a callee can be handed.
#[derive(Clone, Copy)]
enum CFailedRead {
    /// 0: a number, a `bool`, an enum.
    Zero,
    /// The empty string: a function that takes a string must never be handed a null pointer.
    Text,
    /// A zeroed value of the field's own type: a byte string is a struct, and a conditional
    /// whose arms are a struct and 0 does not compile. So is a list, an element of a list of
    /// lists a composed value holds (docs/adr/0086, section 6.1), which reads as empty.
    Aggregate,
    /// The absent value of an element that may be absent, a struct of its flag and its value
    /// (docs/adr/0055, section 5).
    Absent,
    /// The language's own zero of a union's struct, `{0}`, whose first member is the tag:
    /// the value beside a recorded failure that no statement commits (docs/adr/0099,
    /// section 2.2).
    Whole,
}

/// What a checked index read in C gives when it records a failure, by the type of what it
/// reads: a zeroed struct of a byte string or a list, the absent value of a value that may be
/// absent, and 0 for every other.
fn c_failed_part(read: InferredType) -> CFailedRead {
    match read {
        InferredType::Bytes | InferredType::List(_) => CFailedRead::Aggregate,
        InferredType::Optional(_) => CFailedRead::Absent,
        _ => CFailedRead::Zero,
    }
}

/// A checked index read in C (SCE_FORGE.md §3.4.1), with `field` — empty, or
/// `.name` — read from the element inside the same conditional: the failure is
/// recorded and the read gives `failed` when the index is outside the collection, and
/// an index that is outside is never read, the collection's emptiness included.
/// A `bytes` view and a `list<T>` view carry their length; a build-time array's is
/// its `sizeof`.
fn c_checked_index(
    left: &TypedExpr,
    index: &TypedExpr,
    field: &str,
    failed: CFailedRead,
) -> Result<String, ExprError> {
    let object = wrap_postfix(left, emit_c(left, InferredType::Unknown)?);
    let idx = emit_c(index, InferredType::Unknown)?;
    let accessor = c_element_accessor(left.ty);
    let len = if accessor.is_empty() {
        format!("(sizeof({object}) / sizeof(({object})[0]))")
    } else {
        format!("({object}).len")
    };
    let from = match index.ty {
        InferredType::Int { signed: false, .. } => "u",
        _ => "i",
    };
    let read = format!("{object}{accessor}[{idx}]{field}");
    let failed = match failed {
        CFailedRead::Zero => "0".to_string(),
        CFailedRead::Text => "\"\"".to_string(),
        // A designated initializer: `{0}` over a struct whose first member is an array is
        // `-Wmissing-braces`, and the rest of the value is zeroed all the same.
        CFailedRead::Aggregate => format!("(__typeof__({read})){{ .len = 0 }}"),
        CFailedRead::Absent => format!("(__typeof__({read})){{ .present = false }}"),
        CFailedRead::Whole => format!("(__typeof__({read})){{0}}"),
    };
    Ok(format!(
        "(sce_forge_checked_index_from_{from}(&sce_failure_, {idx}, {len}) ? {read} : {failed})"
    ))
}

/// What `len(arg)` reads in C: a string's UTF-8 bytes, which `sce_text_length`
/// counts over the NUL-terminated text every C string is read as (from
/// `sce/text.h`, which the lowering of a string comparison already needs; a
/// machine's core calls no hosted library, docs/adr/0110), and the `len` member
/// of a list's or a byte string's view.
fn c_length(arg: &TypedExpr) -> Result<String, ExprError> {
    let measured = emit_c(arg, InferredType::Unknown)?;
    Ok(if arg.ty == InferredType::Str {
        format!("sce_text_length({measured})")
    } else {
        format!("({measured}).len")
    })
}

/// A string literal's value as a C11 expression of type `const char *`
/// ([`crate::filters::c_text_expression`]). A C string ends at its first zero
/// byte, so a value holding U+0000 has no C11 string to be: it is refused by
/// name rather than cut short where the zero stands, as a byte string holding
/// one is where a request carries it as text.
fn c_text_literal(value: &str) -> Result<String, ExprError> {
    if value.contains('\0') {
        return Err(ExprError::UnsupportedConstruct {
            construct: "a string holding U+0000 in C11 (a C string ends at its first zero \
                        byte, so the text after it would be lost; generate for another \
                        backend, or hold the bytes as a byte string)"
                .to_string(),
            observed: Some(crate::filters::ecmascript_string_literal(value)),
        });
    }
    Ok(crate::filters::c_text_expression(value))
}

/// A byte-string constant as the C string literal of its bytes, escaped as
/// any C literal is. [`decode_bytes_literal`] holds the bytes to printable
/// ASCII, so the literal never reaches a zero byte or a length C refuses.
fn c_bytes_literal(bytes: &[u8]) -> String {
    format!("\"{}\"", crate::filters::escape_c(latin1(bytes)))
}

fn c_emit_node(expr: &TypedExpr) -> Result<String, ExprError> {
    if let Some(form) = optional_form(expr) {
        return c_optional(form, expr);
    }
    Ok(match &expr.kind {
        // docs/adr/0063: the runtime's helper for the operation's own width.
        // C promotes a narrow operand to `int`, where a product can overflow,
        // and leaves a signed overflow undefined; the helper computes in the
        // unsigned type of the width. C has no overloading, so the width is
        // in the name.
        ExprKind::Total { op, rule, .. } => {
            let (signed, bits, operands) = total_parts(expr, "C11", emit_c)?;
            format!(
                "sce_forge_{}_{}_{}{bits}({})",
                rule.helper(),
                op.helper(),
                if signed { "i" } else { "u" },
                operands.join(", ")
            )
        }
        ExprKind::NumberLit(n) => portable_number_literal(n),
        ExprKind::StringLit { value } => c_text_literal(value)?,
        // Standalone fallback — the bytes-equality Binary branch renders
        // the literal and its byte count itself (a `bytes` value has no
        // length without its `_len` sibling). RFC §bytesguard-3 B5.
        ExprKind::BytesLit { bytes } => c_bytes_literal(bytes),
        ExprKind::BoolLit(b) => if *b { "true" } else { "false" }.to_string(),
        ExprKind::NullLit => "NULL".to_string(),
        // The one spelling every C local takes, declaration and read alike
        // ([`ExprTarget::local_spelling`]).
        ExprKind::Ident(s) => ExprTarget::C.local_spelling(s),
        ExprKind::Raw(s) => s.clone(),
        ExprKind::Binary { op, left, right } if is_string_concatenation(*op, left.ty, right.ty) => {
            // A C string is a `const char *`: there is nowhere for the joined
            // text to live, and a returned pointer to a local buffer is a
            // dangling one. Written as `"E" + n` -- the shape this branch
            // replaces -- it compiled and read another string's bytes, which
            // is the worst of the answers. A join whose operands the data model
            // sizes was written into a buffer of the sum of their bounds before
            // emission ([`lower_string_concatenation`]), so what reaches here is
            // one that has no such buffer: an operand whose text is not the same
            // in every backend is refused as every backend refuses it, and one
            // with no declared size is refused, by name, at the construct.
            for side in [left, right] {
                if let ConcatOperand::Refused(what) = concat_operand(side.ty) {
                    return Err(concat_refusal(what, emit_c(side, side.ty)?));
                }
            }
            let (l, r) = (emit_c(left, left.ty)?, emit_c(right, right.ty)?);
            return Err(ExprError::UnsupportedConstruct {
                construct: "string concatenation in C (a C string is a bounded buffer, and a \
                            value whose size the data model does not declare has no buffer \
                            to be joined into; declare `sce:capacity` on the variable, hold \
                            the pieces in separate outputs, or generate for another backend)"
                    .to_string(),
                observed: Some(format!("{l} + {r}")),
            });
        }
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
                            let v = c_bytes_literal(bytes);
                            Ok((v, bytes.len().to_string(), Some(bytes.len())))
                        } else if let ExprKind::BytesView {
                            source,
                            len: Some(len),
                        } = &operand.kind
                        {
                            // A byte string held as `{data, len}`, whose pointer
                            // and length are two expressions of their own
                            // ([`views_for_bytes_equality`]).
                            Ok((
                                emit_c(source, InferredType::Unknown)?,
                                emit_c(len, InferredType::Unknown)?,
                                None,
                            ))
                        } else {
                            // A payload's field: the array and its `_len` sibling.
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
                    // Parenthesised, since a `||` read in a larger condition
                    // would otherwise bind to whatever stands beside it.
                    format!("({ll} != {rl} || memcmp({lv}, {rv}, {cmp_len}) != 0)")
                });
            }
            // String comparison lowering. C lacks operator
            // overloading; `a == b` on `const char *` is pointer equality, not
            // content equality. Lower any string-typed comparison (==, !=, <,
            // >, <=, >=) to `sce_text_compare(a, b) <op> 0`, which is the
            // lexicographic semantic the other backends already provide natively
            // (cpp via std::string operator==, Rust via PartialOrd, Python lex
            // order), and `strcmp`'s, from the runtime's own `sce/text.h`
            // rather than the hosted library (docs/adr/0110, D1). The units
            // whose expressions this lowers include that header.
            if op.is_comparison()
                && matches!(left.ty, InferredType::Str)
                && matches!(right.ty, InferredType::Str)
            {
                let l_raw = emit_c(left, InferredType::Str)?;
                let r_raw = emit_c(right, InferredType::Str)?;
                return Ok(format!(
                    "sce_text_compare({l_raw}, {r_raw}) {} 0",
                    cpp_binop(*op)
                ));
            }
            let operand_ty = binary_operand_type(*op, left.ty, right.ty);
            let l_raw = emit_c(left, operand_ty)?;
            let r_raw = emit_c(right, right_operand_type(*op, operand_ty))?;
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
            c_family_binary(CFamily::C, *op, expr.ty, &l, &r)
        }
        ExprKind::Unary { op, operand } => {
            let inner = emit_c(operand, expr.ty)?;
            c_family_unary(CFamily::C, *op, expr.ty, operand, &inner)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            // See `cpp_emit_node`: the condition that is a conditional is bracketed.
            let cond = emit_c(condition, InferredType::Bool)?;
            let cond = if matches!(&condition.kind, ExprKind::Conditional { .. }) {
                format!("({cond})")
            } else {
                cond
            };
            format!(
                "{cond} ? {} : {}",
                emit_c(consequent, expr.ty)?,
                emit_c(alternate, expr.ty)?,
            )
        }
        // A field of the element a list of records is indexed at: the field is read
        // inside the checked conditional, which otherwise would be a struct against
        // the `0` of a failed read.
        ExprKind::Member { object, property }
            if matches!(
                &object.kind,
                ExprKind::Checked {
                    op: CheckedOp::Index,
                    right: Some(_),
                    discharge: Discharge::AtRun,
                    ..
                }
            ) =>
        {
            let ExprKind::Checked {
                left,
                right: Some(index),
                ..
            } = &object.kind
            else {
                unreachable!("guarded by the match above")
            };
            let failed = match expr.ty {
                InferredType::Str => CFailedRead::Text,
                other => c_failed_part(other),
            };
            c_checked_index(left, index, &format!(".{property}"), failed)?
        }
        ExprKind::Member { object, property } => {
            format!(
                "{}.{property}",
                wrap_postfix(object, emit_c(object, InferredType::Unknown)?)
            )
        }
        ExprKind::Index { object, index } => {
            // A `bytes` operand lowers to `sce_forge_bytes_view_t`
            // (`{const uint8_t *data; size_t len}`), and a `list<T>`
            // parameter to its element's `sce_forge_<T>_view_t` of the same
            // shape, so a random read projects through `.data` — mirroring
            // the foreach arm's `src.data[__i]`. Item C7 wildcard-keyexpr
            // lowering.
            format!(
                "{}{}[{}]",
                wrap_postfix(object, emit_c(object, InferredType::Unknown)?),
                c_element_accessor(object.ty),
                emit_c(index, InferredType::Unknown)?,
            )
        }
        ExprKind::Call {
            callee,
            args,
            params,
            fails,
            into_destination,
            storage,
        } => {
            if is_len_builtin(callee, args) {
                let len = c_length(&args[0])?;
                // A length typed as an integer width (`TypeCtx::lengths_are_uint32`)
                // is that width: the cast is spelled here, where `size_t` meets it.
                return Ok(match expr.ty {
                    InferredType::Int { signed, bits } => {
                        format!("({}int{bits}_t){len}", if signed { "" } else { "u" })
                    }
                    _ => len,
                });
            }
            if let Some(op) = real_to_int_builtin(callee, args) {
                // The forge runtime's own `round` (half away from zero, as C99
                // `llround` and C++) and `floor`, not `<math.h>`'s: a machine's
                // core and the kinds it calls link no mathematics library
                // (docs/adr/0110, D1). Both answer an `int64_t`, and the name
                // makes the kind's template include `sce/forge/algorithm.h`
                // (`std_imports::c_uses_forge_algorithm`).
                let inner = emit_c(&args[0], InferredType::Float { bits: 64 })?;
                return Ok(match op {
                    RealToInt::Round => format!("sce_forge_real_round({inner})"),
                    RealToInt::Floor => format!("sce_forge_real_floor({inner})"),
                });
            }
            let mut a = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                a.push(emit_c(arg, argument_type(params, i))?);
            }
            let symbol = wrap_postfix(callee, emit_c(callee, InferredType::Unknown)?);
            // The storage the caller holds for the call follows the arguments it is
            // written with (docs/adr/0082): a machine's member of its `scratch`.
            a.extend(storage.iter().cloned());
            // A generated algorithm writes the list it returns into the destination
            // the caller holds, which is that storage, and records its failure in the
            // statement's `sce_failure_`: the call is whether it wrote. A list is taken
            // only from an algorithm that declares `may-fail`.
            if *into_destination && *fails {
                if storage.is_none() {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: "a call that writes a list, where its caller holds no \
                                    storage for it"
                            .to_string(),
                        observed: None,
                    });
                }
                a.push("&sce_failure_".to_string());
                return Ok(format!("{symbol}({})", a.join(", ")));
            }
            let call = format!("{symbol}({})", a.join(", "));
            if *fails {
                pass_failure_on(ExprTarget::C, &symbol, &call, expr.ty)?
            } else {
                call
            }
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
        // SCE_FORGE.md §3.4.1, "The width of a shift": a shift whose count is
        // not known below its width ([`settle_shifts`]).
        ExprKind::Checked {
            op,
            left,
            right: Some(count),
            discharge,
        } if op.shift().is_some() => c_settled_shift(expr, *op, left, count, *discharge)?,
        // SCE_FORGE.md §3.4.1: an index read behind a range test, which
        // records an out-of-range failure and yields 0 instead of reading:
        // `(inside ? xs[i] : 0)`. The index is written twice — algorithm
        // expressions have no side effects — so an index outside is never
        // read, an empty collection's included. A `bytes` view and a
        // `list<T>` parameter's view carry their length; a build-time
        // array's is its `sizeof`.
        // An element of a list of unions, which a union local copies whole
        // (docs/adr/0099, section 5.2), is a struct the expression types by no
        // operand type: its filler is the struct's own zero.
        ExprKind::Checked {
            discharge: Discharge::AtBuild,
            ..
        } => return Err(proven_operation_outside_rust()),
        ExprKind::Checked {
            op: CheckedOp::Index,
            left,
            right: Some(index),
            ..
        } => {
            let failed = if listed_union(left.ty).is_some() {
                CFailedRead::Whole
            } else {
                c_failed_part(expr.ty)
            };
            c_checked_index(left, index, "", failed)?
        }
        // SCE_FORGE.md §3.4.1: the runtime's helper for the operation's own
        // width, which records a failure in the body's `sce_failure_` and
        // yields 0; the statement around it returns that failure. C has no
        // overloading, so the width is in the helper's name.
        ExprKind::Checked {
            op, left, right, ..
        } => {
            let InferredType::Int { signed, bits } = expr.ty else {
                return Err(ExprError::UnsupportedConstruct {
                    construct: format!("a checked operation of type {:?}", expr.ty),
                    observed: None,
                });
            };
            if *op == CheckedOp::Narrow {
                // The value arrives as `int64_t` or `uint64_t` by its own
                // signedness, which holds it exactly; one helper per target
                // takes either.
                let InferredType::Int {
                    signed: from_signed,
                    ..
                } = left.ty
                else {
                    return Err(ExprError::UnsupportedConstruct {
                        construct: format!("a checked narrowing from {:?}", left.ty),
                        observed: None,
                    });
                };
                return Ok(format!(
                    "sce_forge_checked_narrow_{}{bits}_from_{}(&sce_failure_, {})",
                    if signed { "i" } else { "u" },
                    if from_signed { "i" } else { "u" },
                    emit_c(left, left.ty)?
                ));
            }
            let mut operands = vec!["&sce_failure_".to_string(), emit_c(left, expr.ty)?];
            if let Some(right) = right {
                operands.push(emit_c(right, expr.ty)?);
            }
            format!(
                "sce_forge_checked_{}_{}{bits}({})",
                op.helper(),
                if signed { "i" } else { "u" },
                operands.join(", ")
            )
        }
    })
}

/// A shift the build settled ([`settle_shifts`]), in C: the runtime's total
/// shift of the width (`sce_forge_wrapping_shl_u32(x, n)`), which is the exact
/// result reduced to the width for every count at least 0, or its checked one
/// (`sce_forge_checked_shl_u32(&sce_failure_, x, n)`), which records an
/// out-of-range failure for a count below 0 and yields 0. C's own operator is
/// undefined for a count at or past the width, and for a left shift of a
/// negative value. C has no overloading, so the width is in the name, and the
/// count is converted to the `uint64_t` the total shift takes, or the `int64_t`
/// the checked one does, which holds every count either is handed.
fn c_settled_shift(
    expr: &TypedExpr,
    op: CheckedOp,
    left: &TypedExpr,
    count: &TypedExpr,
    discharge: Discharge,
) -> Result<String, ExprError> {
    let (signed, bits) = settled_shift_width(expr)?;
    let width = format!("{}{bits}", if signed { "i" } else { "u" });
    let value = emit_c(left, expr.ty)?;
    let places = emit_c(count, InferredType::Unknown)?;
    Ok(match discharge {
        Discharge::AtBuild | Discharge::Total => format!(
            "sce_forge_wrapping_{}_{width}({value}, (uint64_t)({places}))",
            op.helper()
        ),
        Discharge::AtRun => format!(
            "sce_forge_checked_{}_{width}(&sce_failure_, {value}, (int64_t)({places}))",
            op.helper()
        ),
    })
}

/// The name `sce/forge/optional.h` declares the optional value of `elem` under
/// (`sce_optional_<name>_t`) where the lattice names the element whole: a number
/// or a `bool`, by the type the document writes (`int64`, `float64`, `bool`).
/// `None` for a text, a byte string, an enum and a record, whose optional type
/// carries a bound or a document the expression does not name (docs/adr/0055,
/// section 5).
fn c_optional_scalar_name(elem: OptionalElem) -> Option<String> {
    use crate::forge::model::SceType;
    elem.scalar()
        .filter(|ty| !matches!(ty, SceType::String | SceType::Bytes | SceType::Enum(_)))
        .map(|ty| ty.as_attr())
}

/// An optional value of `elem` built inside an expression: the absent one for
/// `None` (what `null` is where the place expects an optional value), and
/// `present`, a value of the element, lifted for `Some` (docs/adr/0055,
/// section 2). An optional value whose type the expression cannot name — of a
/// text, a byte string, an enum or a record — is refused by name: the walk
/// writes such a value whole where a variable or a field takes it, where the
/// place names the type
/// ([`crate::forge::static_lowering::StaticTarget::optional_none`]).
fn c_optional_constructor(elem: OptionalElem, present: Option<&str>) -> Result<String, ExprError> {
    let Some(name) = c_optional_scalar_name(elem) else {
        return Err(ExprError::UnsupportedConstruct {
            construct: format!(
                "{} of an optional text, byte string, enum or record inside an expression, \
                 whose C type carries a bound or a document the expression does not name: write \
                 it as the whole value of the variable or the field that takes it",
                if present.is_some() {
                    "a present value lifted into one"
                } else {
                    "`null`"
                }
            ),
            observed: None,
        });
    };
    Ok(match present {
        Some(value) => format!("sce_optional_{name}_some({value})"),
        None => format!("sce_optional_{name}_none()"),
    })
}

/// The present value `subject`, an optional value of type `ty`, holds, read as
/// an expression reads a value of its element: a text as the text of its buffer,
/// a byte string as the view of its buffer and length, and any other as the
/// value itself. Read only where the build proved it present (docs/adr/0055,
/// section 3), or under a test of its flag: the C11 lowering's map and wire
/// value of an optional value read it the same way.
pub(crate) fn c_present_read(subject: &str, ty: InferredType) -> String {
    match ty {
        InferredType::Optional(OptionalElem::Str) => {
            format!("{subject}.{OPTIONAL_VALUE}.{C_BUFFER_TEXT}")
        }
        InferredType::Optional(OptionalElem::Bytes) => {
            format!("(sce_forge_bytes_view_t){{ {subject}.value.data, {subject}.value.len }}")
        }
        _ => format!("{subject}.value"),
    }
}

/// The C11 spelling of each construct of an optional value (docs/adr/0055,
/// section 5) over `sce_optional_<elem>_t` of `sce/forge/optional.h`: a test of
/// `.present`, a proven read of `.value`, `x ?? d` as the conditional over the
/// flag the runtime's `SCE_OPTIONAL_*_OR` spells, which evaluates `d` only when
/// `x` is absent, and total equality through `SCE_OPTIONAL_SAME` of two optional
/// values or `SCE_OPTIONAL_HOLDS` of one and a present value, which name no type.
fn c_optional(form: OptionalForm<'_>, whole: &TypedExpr) -> Result<String, ExprError> {
    let subject = |node: &TypedExpr| -> Result<String, ExprError> {
        Ok(wrap_postfix(node, emit_c(node, InferredType::Unknown)?))
    };
    Ok(match form {
        OptionalForm::Test {
            subject: s,
            present,
        } => {
            let test = format!("{}.present", subject(s)?);
            if present {
                test
            } else {
                format!("!{test}")
            }
        }
        OptionalForm::Proven { subject: s } => c_present_read(&subject(s)?, s.ty),
        OptionalForm::Otherwise { value, fallback } => {
            let read = if whole.ty.is_optional() {
                "SCE_OPTIONAL_EITHER"
            } else {
                match value.ty {
                    InferredType::Optional(OptionalElem::Str) => "SCE_OPTIONAL_TEXT_OR",
                    InferredType::Optional(OptionalElem::Bytes) => "SCE_OPTIONAL_BYTES_OR",
                    _ => "SCE_OPTIONAL_VALUE_OR",
                }
            };
            format!(
                "{read}({}, {})",
                emit_c(value, InferredType::Unknown)?,
                emit_c(fallback, whole.ty)?
            )
        }
        OptionalForm::Equality { left, right, equal } => {
            // The optional side, and the other: an optional value too, or a
            // present value of the element, which needs no lift to be compared.
            let (optional, other) = if left.ty.is_optional() {
                (left, right)
            } else {
                (right, left)
            };
            let InferredType::Optional(elem) = optional.ty else {
                unreachable!("an equality of an optional value has an optional side")
            };
            let held = emit_c(optional, InferredType::Unknown)?;
            let comparison = if other.ty.is_optional() {
                let same = match elem {
                    OptionalElem::Str => "SCE_OPTIONAL_SAME_STRING",
                    OptionalElem::Bytes => "SCE_OPTIONAL_SAME_BYTES",
                    _ => "SCE_OPTIONAL_SAME_VALUE",
                };
                format!(
                    "SCE_OPTIONAL_SAME({held}, {}, {same})",
                    emit_c(other, InferredType::Unknown)?
                )
            } else {
                let value = emit_c(other, elem.present())?;
                match elem {
                    OptionalElem::Str => format!("SCE_OPTIONAL_HOLDS_TEXT({held}, {value})"),
                    OptionalElem::Bytes => {
                        return Err(ExprError::UnsupportedConstruct {
                            construct: "`===` of an optional byte string and a present one in \
                                        C: test it with `!== null` and compare `.value` in its \
                                        branch"
                                .to_string(),
                            observed: None,
                        })
                    }
                    _ => format!("SCE_OPTIONAL_HOLDS({held}, {value})"),
                }
            };
            if equal {
                comparison
            } else {
                format!("!{comparison}")
            }
        }
        OptionalForm::Constant(answer) => answer.to_string(),
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
    if let (UntypedInt, Float { bits }) = (from, to) {
        if let ExprKind::NumberLit(text) = &node.kind {
            if is_decimal_integer_literal(text) {
                return format!("{raw}.0{}", single_suffix(bits));
            }
        }
    }
    // A real literal that lands in a 32-bit real is the single nearest to it,
    // as it is in C++ (`cpp_coerce`).
    if let (UntypedFloat, Float { bits: 32 }) = (from, to) {
        if matches!(node.kind, ExprKind::NumberLit(_)) {
            return format!("{raw}f");
        }
    }
    // An unsigned operand where the join of the two is signed, as in C++.
    if let Some(bits) = c_family_unsigned_to_signed(from, to) {
        return format!("(int{bits}_t)({raw})");
    }
    raw
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod string_literal_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::types::{FuncSig, InferredType, RecordShape};

    /// An element read whole is read through each list on the way down
    /// (docs/adr/0099, "Still open (2026-11-16)" item 3): the place a path of
    /// fields starts at, each index as written, and the fields after each that
    /// reach the next list; a text that reads past its last element, or reads
    /// no place, reads no element whole.
    #[test]
    fn an_element_read_whole_is_read_through_each_list_on_the_way() {
        let step = |index: &str, fields: &str| ElementStep {
            index: index.to_string(),
            fields: fields.to_string(),
        };
        assert_eq!(
            written_elements("_event.data.many[_event.data.at]"),
            Some((
                "_event.data.many".to_string(),
                vec![step("_event.data.at", "")]
            ))
        );
        assert_eq!(
            written_elements(" _event.data.groups[g + 1].shapes[s] "),
            Some((
                "_event.data.groups".to_string(),
                vec![step("g + 1", ".shapes"), step("s", "")]
            ))
        );
        assert_eq!(
            written_elements("uss[i][j]"),
            Some(("uss".to_string(), vec![step("i", ""), step("j", "")]))
        );
        assert_eq!(written_elements("us[i].kind"), None);
        assert_eq!(written_elements("us"), None);
        assert_eq!(written_elements("f(x)[0]"), None);
    }

    // ── Helpers ─────────────────────────────────────────────────

    /// Forcing function for [`ExprTarget::ALL`]: a new `ExprTarget`
    /// variant breaks the exhaustive `match` below at compile time,
    /// pointing the author at the array (and at every all-backend verdict
    /// that iterates it — e.g. `guard_is_native_lowerable`). The
    /// `contains` check additionally proves the array is not missing any
    /// existing variant — and that the one target that is not a
    /// code-generation backend, [`ExprTarget::Js`], is not in it.
    /// [`ExprTarget::of`] is the inverse of [`ExprTarget::language`] on every
    /// backend.
    #[test]
    fn the_target_of_a_backend_names_that_backend() {
        for &language in crate::generator::Language::ALL {
            assert_eq!(ExprTarget::of(language).language(), Some(language));
        }
    }

    #[test]
    fn expr_target_all_contains_every_variant() {
        fn is_backend(t: ExprTarget) -> bool {
            match t {
                ExprTarget::Cpp
                | ExprTarget::Kotlin
                | ExprTarget::Rust
                | ExprTarget::Go
                | ExprTarget::Python
                | ExprTarget::C
                | ExprTarget::Swift => true,
                // The ecmascript lowering runs a `sce-static` document in the
                // Interpreter; it has no `Language`, and listing it in `ALL`
                // would make every "lowers natively on all backends" verdict
                // require a lowering no forge kind has.
                ExprTarget::Js => false,
            }
        }
        let every = [
            ExprTarget::Cpp,
            ExprTarget::Kotlin,
            ExprTarget::Rust,
            ExprTarget::Go,
            ExprTarget::Python,
            ExprTarget::C,
            ExprTarget::Swift,
            ExprTarget::Js,
        ];
        for t in every {
            assert_eq!(
                ExprTarget::ALL.contains(&t),
                is_backend(t),
                "{t:?}: membership of ExprTarget::ALL disagrees with being a backend"
            );
            assert_eq!(
                t.language().is_some(),
                is_backend(t),
                "{t:?}: a backend has a Language and the ecmascript target has none"
            );
        }
        assert_eq!(ExprTarget::ALL.len(), 7);
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

    /// `a as u32 < b` does not parse in Rust: the `<` after a type opens its
    /// generic arguments. A widened `len` before `<` is the shape
    /// `sce:std/mesh/dedup_admit` wrote; `<=` parses and leaves the cast bare.
    /// The length is read off its operand as it is, with no pair of its own.
    #[test]
    fn rust_cast_before_less_than_is_parenthesised() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("buf", InferredType::Bytes);
        ctx.insert_var("n", int(false, 32));
        ctx.insert_var("x", float(64));
        ctx.insert_var("m", int(true, 64));
        assert_eq!(
            tp_with("len(buf) < n", ExprTarget::Rust, &ctx),
            "(buf.len() as u32) < n"
        );
        assert_eq!(
            tp_with("len(buf) <= n", ExprTarget::Rust, &ctx),
            "buf.len() as u32 <= n"
        );
        assert_eq!(
            tp_with("round(x) < m", ExprTarget::Rust, &ctx),
            "(x.round() as i64) < m"
        );
        assert_eq!(
            tp_with("n > len(buf)", ExprTarget::Rust, &ctx),
            "n > buf.len() as u32"
        );
    }

    #[test]
    fn rust_ternary_if_else() {
        assert_eq!(
            tp("status === 'OK' ? 1 : 0", ExprTarget::Rust),
            "if status == \"OK\" { 1 } else { 0 }"
        );
    }

    #[test]
    fn python_wraps_a_conditional_used_as_a_condition() {
        // `x if a if g else b else 0` does not parse: only the else branch
        // may nest unparenthesised.
        assert_eq!(
            tp("(g ? a : b) ? 1 : 0", ExprTarget::Python),
            "1 if (a if g else b) else 0"
        );
        assert_eq!(
            tp("c ? (g ? 1 : 2) : (h ? 3 : 4)", ExprTarget::Python),
            "(1 if g else 2) if c else 3 if h else 4"
        );
    }

    #[test]
    fn c_and_cpp_wrap_a_conditional_used_as_a_condition() {
        // `g ? a : b ? 1 : 0` is `g ? a : (b ? 1 : 0)`: the condition `g ? a : b` of the outer
        // conditional was emitted without its brackets, which changes what it says and, when
        // the branches differ in type (a boolean against a string), is not a C++ expression at
        // all ("operands to ?: have different types"). A document reading
        // `(c ? x : y) ? 'P' : 'Q'` produced it (2026-10-10); Python and the Interpreter's
        // script had been given the brackets on 2026-09-28 and these two had not.
        for target in [ExprTarget::Cpp, ExprTarget::C] {
            assert_eq!(
                tp("(g ? a : b) ? 1 : 0", target),
                "(g ? a : b) ? 1 : 0",
                "{target:?}"
            );
        }
        // The branches keep their spelling: a conditional there is read as it was written.
        for target in [ExprTarget::Cpp, ExprTarget::C] {
            assert_eq!(
                tp("c ? (g ? 1 : 2) : (h ? 3 : 4)", target),
                "c ? g ? 1 : 2 : h ? 3 : 4",
                "{target:?}"
            );
        }
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

    /// What Python makes of `expr`, an expression that holds an optional value
    /// (docs/adr/0055), lowered into a place of `expected` against `ctx`, its
    /// integer operations checked where `ctx` receives failures.
    fn python_of(expr: &str, ctx: &TypeCtx<'_>, expected: InferredType) -> String {
        transpile_into(expr, ExprTarget::Python, ctx, &empty_renames(), expected)
            .unwrap_or_else(|e| panic!("{expr} lowers for Python: {e:?}"))
    }

    /// Two optional days, an optional text, a day that is never absent and a
    /// count.
    fn python_optional_ctx() -> TypeCtx<'static> {
        let day = InferredType::Optional(OptionalElem::of(&crate::forge::model::SceType::Int64));
        let mut ctx = TypeCtx::new();
        ctx.insert_var("due", day);
        ctx.insert_var("later", day);
        ctx.insert_var("label", InferredType::Optional(OptionalElem::Str));
        ctx.insert_var("huge", int(true, 64));
        ctx.insert_var("count", int(false, 32));
        ctx
    }

    /// A test of presence is an identity test against `None`, whichever side
    /// `null` is written on: `==` would do for `None` itself, but a truth test
    /// would read a present `0`, `''` or `False` as absent.
    #[test]
    fn python_tests_an_optional_value_against_none_by_identity() {
        let ctx = python_optional_ctx();
        let bool_slot = InferredType::Bool;
        assert_eq!(python_of("due === null", &ctx, bool_slot), "due is None");
        assert_eq!(
            python_of("due !== null", &ctx, bool_slot),
            "due is not None"
        );
        assert_eq!(python_of("null === due", &ctx, bool_slot), "due is None");
        assert_eq!(
            python_of("null !== label", &ctx, bool_slot),
            "label is not None"
        );
    }

    /// `x.value` where a test proves `x` present is `x` itself, in each place
    /// a test proves it: the right operand of `&&` and of `||`, the branches
    /// of a conditional, one under `!`, and a block the facts of an `<if>`
    /// reach.
    #[test]
    fn python_reads_a_proven_optional_value_as_the_value_itself() {
        let ctx = python_optional_ctx();
        let day = int(true, 64);
        assert_eq!(
            python_of("due !== null ? due.value : 0", &ctx, day),
            "due if due is not None else 0"
        );
        assert_eq!(
            python_of("due !== null && due.value > 10", &ctx, InferredType::Bool),
            "due is not None and due > 10"
        );
        assert_eq!(
            python_of("due === null || due.value < 3", &ctx, InferredType::Bool),
            "due is None or due < 3"
        );
        assert_eq!(
            python_of("!(due === null) ? due.value + 1 : 0", &ctx, day),
            "due + 1 if not (due is None) else 0"
        );
        let proven = ctx.proving(crate::forge::narrowing::proves_written(
            "label !== null",
            true,
        ));
        assert_eq!(
            python_of("label.value", &proven, InferredType::Str),
            "label"
        );
    }

    /// `x ?? d` hands the runtime's `otherwise` the value and the default as a
    /// `lambda`, so the default — here a product that overflows an int64 — is
    /// computed only when the value is absent, and a present `0` is not read as
    /// absent, as Python's `or` would read it.
    #[test]
    fn python_defers_the_default_of_an_absent_value_to_a_lambda() {
        let mut ctx = python_optional_ctx();
        assert_eq!(
            python_of("label ?? 'none'", &ctx, InferredType::Str),
            "sce_algorithm.otherwise(label, lambda: \"none\")"
        );
        ctx.receives_failures = true;
        assert_eq!(
            python_of("due ?? (huge * huge)", &ctx, int(true, 64)),
            "sce_algorithm.otherwise(due, lambda: sce_algorithm.I64.mul(huge, huge))"
        );
    }

    /// Equality between optional values is Python's `==`, which is total over
    /// `None`: two absent values are equal and an absent one equals no present
    /// one.
    #[test]
    fn python_compares_optional_values_with_equality() {
        let ctx = python_optional_ctx();
        assert_eq!(
            python_of("due === later", &ctx, InferredType::Bool),
            "due == later"
        );
        assert_eq!(
            python_of("due !== later", &ctx, InferredType::Bool),
            "due != later"
        );
        // A side of `T` is the present value it is, compared as one.
        assert_eq!(python_of("due !== 3", &ctx, InferredType::Bool), "due != 3");
        assert_eq!(python_of("null === null", &ctx, InferredType::Bool), "True");
    }

    /// `null` where an optional value is expected is `None`, and a value of `T`
    /// there is the value itself, the lift being no conversion in Python; a
    /// conditional of the two is one of them.
    #[test]
    fn python_spells_null_and_a_lifted_value_where_an_optional_one_is_expected() {
        let ctx = python_optional_ctx();
        let day = InferredType::Optional(OptionalElem::of(&crate::forge::model::SceType::Int64));
        assert_eq!(python_of("null", &ctx, day), "None");
        assert_eq!(python_of("3", &ctx, day), "3");
        assert_eq!(python_of("later", &ctx, day), "later");
        assert_eq!(
            python_of("count === 1 ? null : 5", &ctx, day),
            "None if count == 1 else 5"
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

    // ── C11 V3: string compare → the runtime's text comparison ──

    #[test]
    fn c_string_eq_lowers_to_a_text_compare() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("engineState", InferredType::Str);
        assert_eq!(
            tp_with("engineState === 'STOP'", ExprTarget::C, &ctx),
            "sce_text_compare(engine_state, \"STOP\") == 0"
        );
    }

    #[test]
    fn c_string_neq_lowers_to_a_text_compare() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("engineState", InferredType::Str);
        assert_eq!(
            tp_with("engineState !== 'STOP'", ExprTarget::C, &ctx),
            "sce_text_compare(engine_state, \"STOP\") != 0"
        );
    }

    #[test]
    fn c_string_lex_compare_lowers_to_a_text_compare() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("a", InferredType::Str);
        ctx.insert_var("b", InferredType::Str);
        assert_eq!(
            tp_with("a < b", ExprTarget::C, &ctx),
            "sce_text_compare(a, b) < 0"
        );
        assert_eq!(
            tp_with("a > b", ExprTarget::C, &ctx),
            "sce_text_compare(a, b) > 0"
        );
        assert_eq!(
            tp_with("a <= b", ExprTarget::C, &ctx),
            "sce_text_compare(a, b) <= 0"
        );
        assert_eq!(
            tp_with("a >= b", ExprTarget::C, &ctx),
            "sce_text_compare(a, b) >= 0"
        );
    }

    /// A context that reads a join where it computes it, over a `uint32`
    /// `wait` and a string `name` the data model bounds to eight bytes.
    fn sized_join_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var(
            "wait",
            InferredType::Int {
                signed: false,
                bits: 32,
            },
        );
        ctx.insert_var(
            "level",
            InferredType::Int {
                signed: true,
                bits: 8,
            },
        );
        ctx.insert_var("name", InferredType::Str);
        ctx.insert_string_capacity("name", 8);
        ctx.insert_var("loose", InferredType::Str);
        ctx.joins_into_buffers = true;
        ctx
    }

    #[test]
    fn c_join_of_sized_operands_is_written_into_a_buffer_of_their_sum() {
        let ctx = sized_join_ctx();
        // The ten digits of a `uint32`, the two bytes of the unit, and the
        // terminator.
        assert_eq!(
            tp_with("wait + 'ms'", ExprTarget::C, &ctx),
            "SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_uint(wait), sce_forge_wire_string(\"ms\"))"
        );
        // A string variable by the capacity the data model declares, a signed
        // `int8` by its three digits and its sign.
        assert_eq!(
            tp_with("name + level", ExprTarget::C, &ctx),
            "SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_string(name), sce_forge_wire_int(level))"
        );
    }

    #[test]
    fn c_chain_of_joins_is_one_buffer() {
        let ctx = sized_join_ctx();
        // `'a' + wait + 'b'` is one join of three parts: 1 + 10 + 1 + 1.
        assert_eq!(
            tp_with("'a' + wait + 'b'", ExprTarget::C, &ctx),
            "SCE_FORGE_CONCAT((char[13]){0}, 13, sce_forge_wire_string(\"a\"), sce_forge_wire_uint(wait), sce_forge_wire_string(\"b\"))"
        );
        // A sum that begins the chain is a number: `1 + 2 + 'x'` is `'3x'`, and
        // the sum is one part of ten digits at most.
        let text = tp_with("wait + wait + 'x'", ExprTarget::C, &ctx);
        assert!(
            text.starts_with(
                "SCE_FORGE_CONCAT((char[12]){0}, 12, sce_forge_wire_uint(wait + wait)"
            ),
            "{text}"
        );
    }

    #[test]
    fn c_join_of_a_conditional_is_sized_by_its_larger_branch() {
        let ctx = sized_join_ctx();
        let text = tp_with("(wait > 3 ? name : 'ab') + 'x'", ExprTarget::C, &ctx);
        assert!(
            text.starts_with("SCE_FORGE_CONCAT((char[10]){0}, 10, "),
            "8 + 1 + 1 bytes: {text}"
        );
    }

    /// A context of a `sce-static` statechart over the 32-bit real `tenth`, the
    /// 64-bit real `wide` and the `uint32` `n`.
    fn single_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("tenth", InferredType::Float { bits: 32 });
        ctx.insert_var("wide", InferredType::Float { bits: 64 });
        ctx.insert_var(
            "n",
            InferredType::Int {
                signed: false,
                bits: 32,
            },
        );
        ctx.rounds_to_single = true;
        ctx
    }

    fn single_slot(expr: &str, target: ExprTarget, ctx: &TypeCtx<'_>) -> String {
        transpile_typed(
            expr,
            target,
            ctx,
            &empty_renames(),
            InferredType::Float { bits: 32 },
        )
        .unwrap()
    }

    fn double_slot(expr: &str, target: ExprTarget, ctx: &TypeCtx<'_>) -> String {
        transpile_typed(
            expr,
            target,
            ctx,
            &empty_renames(),
            InferredType::Float { bits: 64 },
        )
        .unwrap()
    }

    /// The text of a single in any of the languages that have one of their own.
    fn spells_a_single(text: &str) -> bool {
        ["0.1f", "f32", "float32", "to_f32", "toFloat", "(float)"]
            .iter()
            .any(|marker| text.contains(marker))
    }

    #[test]
    fn a_conditional_that_lands_in_a_double_is_made_as_a_double_on_every_language() {
        // `n` is a `uint32`, so `n * 0.1` joins to a single of its own; the slot is a double,
        // and an operation is made at the precision of the place its value lands.
        let ctx = single_ctx();
        for target in [
            ExprTarget::Cpp,
            ExprTarget::C,
            ExprTarget::Kotlin,
            ExprTarget::Rust,
            ExprTarget::Go,
            ExprTarget::Python,
        ] {
            for expr in [
                "n > 3 ? n * 0.1 : 0.0",
                "n > 3 ? 0.0 - n * 0.1 : 0.0",
                "n > 3 ? (n > 5 ? n * 0.1 : 1.5) : 0.0",
                "wide + (n > 3 ? n * 0.1 : 0.0)",
            ] {
                let text = double_slot(expr, target, &ctx);
                assert!(
                    !spells_a_single(&text),
                    "{target:?}: `{expr}` into a double was made as a single: {text}"
                );
            }
        }
    }

    #[test]
    fn a_conditional_that_lands_in_a_single_is_still_made_as_a_single() {
        // The other half: the precision is the slot's, so a single's slot keeps its single.
        let ctx = single_ctx();
        let text = single_slot("n > 3 ? n * 0.1 : 0.0", ExprTarget::Python, &ctx);
        assert!(text.contains("to_f32"), "{text}");
        let text = single_slot("n > 3 ? n * 0.1 : 0.0", ExprTarget::Cpp, &ctx);
        assert!(text.contains("0.1f"), "{text}");
    }

    #[test]
    fn a_sum_of_singles_is_rounded_to_a_single_where_python_and_ecmascript_make_it() {
        let ctx = single_ctx();
        // The literal is the single nearest to it before it is added, and the
        // sum is rounded again.
        assert_eq!(
            tp_with("tenth + 0.2", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(tenth + sce_algorithm.to_f32(0.2))"
        );
        assert_eq!(
            tp_with("tenth + 0.2", ExprTarget::Js, &ctx),
            "Math.fround(tenth + Math.fround(0.2))"
        );
        // A comparison compares singles: its operands are rounded, and its
        // result is a truth value.
        assert_eq!(
            tp_with("tenth > 0.1", ExprTarget::Python, &ctx),
            "tenth > sce_algorithm.to_f32(0.1)"
        );
    }

    #[test]
    fn a_value_that_lands_in_a_single_is_rounded_to_one_for_python_and_ecmascript() {
        let ctx = single_ctx();
        assert_eq!(
            single_slot("wide", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(wide)"
        );
        assert_eq!(single_slot("0.1", ExprTarget::Js, &ctx), "Math.fround(0.1)");
        assert_eq!(
            single_slot("n", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(n)"
        );
        // A single is already one.
        assert_eq!(single_slot("tenth", ExprTarget::Python, &ctx), "tenth");
    }

    #[test]
    fn an_operation_is_made_at_the_precision_of_the_place_its_value_lands() {
        let ctx = single_ctx();
        // In a 32-bit real's slot a sum is a single's: its operands are rounded
        // to singles and its result is rounded again.
        assert_eq!(
            single_slot("wide + 0.5", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(sce_algorithm.to_f32(wide) + sce_algorithm.to_f32(0.5))"
        );
        // In a 64-bit real's slot it is a double's, though one operand is a
        // single: the single is widened, which is exact, and nothing is
        // rounded.
        let into_a_double = |target| {
            transpile_typed(
                "tenth + 0.2",
                target,
                &ctx,
                &empty_renames(),
                InferredType::Float { bits: 64 },
            )
            .unwrap()
        };
        assert_eq!(into_a_double(ExprTarget::Python), "tenth + 0.2");
        assert_eq!(into_a_double(ExprTarget::Js), "tenth + 0.2");
    }

    #[test]
    fn a_negation_passes_the_precision_of_its_place_to_what_it_negates() {
        let ctx = single_ctx();
        // Negating is exact in any width, so the sum under it is the one made
        // as a single: rounded in its operands and in its result, as it is
        // without the minus.
        assert_eq!(
            single_slot("-(wide + 0.5)", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(-sce_algorithm.to_f32(sce_algorithm.to_f32(wide) + sce_algorithm.to_f32(0.5)))"
        );
    }

    #[test]
    fn what_is_not_an_operation_on_singles_is_left_as_it_is_for_python() {
        let ctx = single_ctx();
        assert_eq!(
            tp_with("wide + 0.5", ExprTarget::Python, &ctx),
            "wide + 0.5"
        );
        // The opt-out is the language's own arithmetic.
        let mut unrounded = single_ctx();
        unrounded.rounds_to_single = false;
        assert_eq!(
            tp_with("tenth + 0.2", ExprTarget::Python, &unrounded),
            "tenth + 0.2"
        );
    }

    #[test]
    fn a_context_that_says_nothing_rounds_a_single_for_every_kind() {
        // A forge kind's context is built with no word about rounding: it is on
        // by default, since every kind's `float32` is one answer on every engine.
        let mut ctx = TypeCtx::new();
        ctx.insert_var("tenth", InferredType::Float { bits: 32 });
        assert_eq!(
            tp_with("tenth + 0.2", ExprTarget::Python, &ctx),
            "sce_algorithm.to_f32(tenth + sce_algorithm.to_f32(0.2))"
        );
        assert_eq!(
            tp_with("tenth + 0.2", ExprTarget::Js, &ctx),
            "Math.fround(tenth + Math.fround(0.2))"
        );
    }

    #[test]
    fn a_literal_beside_a_single_is_written_as_a_single_for_kotlin_cpp_and_c() {
        let ctx = single_ctx();
        for target in [ExprTarget::Kotlin, ExprTarget::Cpp, ExprTarget::C] {
            assert_eq!(
                tp_with("tenth + 0.2", target, &ctx),
                "tenth + 0.2f",
                "{target:?}: a real literal beside a single is one"
            );
            assert_eq!(
                tp_with("tenth + 1", target, &ctx),
                "tenth + 1.0f",
                "{target:?}: so is an integer literal"
            );
        }
        // Beside a 64-bit real it stays a double.
        for target in [ExprTarget::Kotlin, ExprTarget::Cpp, ExprTarget::C] {
            assert_eq!(
                tp_with("wide + 0.2", target, &ctx),
                "wide + 0.2",
                "{target:?}"
            );
        }
    }

    #[test]
    fn c_join_with_an_operand_the_model_does_not_size_is_refused_by_name() {
        let ctx = sized_join_ctx();
        // `loose` is a string with no declared bound: there is no buffer to
        // size, and the refusal names the construct.
        let refusal = transpile_typed(
            "loose + 'ms'",
            ExprTarget::C,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect_err("a join with no declared size");
        assert!(
            format!("{refusal:?}").contains("string concatenation in C"),
            "{refusal:?}"
        );
    }

    #[test]
    fn c_join_outside_a_statechart_keeps_its_refusal() {
        // A forge kind's string may be returned or kept, so a buffer sized for
        // one evaluation is not one it can use, even for a literal and a
        // number.
        let mut ctx = sized_join_ctx();
        ctx.joins_into_buffers = false;
        let refusal = transpile_typed(
            "'v=' + wait",
            ExprTarget::C,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect_err("no buffer outside a statechart");
        assert!(
            format!("{refusal:?}").contains("string concatenation in C"),
            "{refusal:?}"
        );
    }

    #[test]
    fn other_backends_are_unchanged_by_a_sized_join() {
        let ctx = sized_join_ctx();
        assert!(
            !tp_with("wait + 'ms'", ExprTarget::Rust, &ctx).contains("SCE_FORGE_CONCAT"),
            "only C11 writes a join into a buffer"
        );
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
            FuncSig::new(
                vec![InferredType::Bytes, InferredType::Bytes],
                InferredType::Bool,
            ),
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
        // Outside the algorithm kind the flag is off → no projection, and the
        // text is refused where the parameter is a byte string: emitted as it
        // was, it reached a backend's compiler.
        let mut ctx = projection_ctx();
        ctx.project_str_args_as_bytes_view = false;
        ctx.insert_member_len_field("entry.pattern", "klen");
        let refusal = transpile_typed(
            "eq(entry.pattern, target)",
            ExprTarget::C,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect_err("a text where a byte string is taken");
        assert!(
            refusal
                .to_string()
                .contains("a string where a byte string is held"),
            "{refusal}"
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

    /// `eq(a, b)` is the bytes comparison it is documented as, on every
    /// backend — the same text `a === b` lowers to, not a call to an `eq`
    /// none of them has.
    #[test]
    fn the_eq_builtin_lowers_as_the_bytes_comparison() {
        let mut ctx = bytes_ctx();
        ctx.insert_var("expect", InferredType::Bytes);
        for target in [
            ExprTarget::Rust,
            ExprTarget::Cpp,
            ExprTarget::Go,
            ExprTarget::Python,
            ExprTarget::Kotlin,
            ExprTarget::C,
        ] {
            for (called, compared) in [
                ("eq(raw, 'ack')", "raw === 'ack'"),
                ("eq(raw, expect)", "raw === expect"),
                ("!eq(raw, expect)", "!(raw === expect)"),
            ] {
                assert_eq!(
                    tp_with(called, target, &ctx),
                    tp_with(compared, target, &ctx),
                    "{target:?}: {called}"
                );
            }
        }
    }

    /// Its signature is judged like a declared one: two arguments, each of
    /// a kind bytes admits.
    #[test]
    fn the_eq_builtin_refuses_what_its_signature_does_not_admit() {
        let mut ctx = bytes_ctx();
        ctx.insert_var("n", int(true, 32));
        let refused = |expr: &str| {
            transpile_typed(
                expr,
                ExprTarget::Rust,
                &ctx,
                &empty_renames(),
                InferredType::Unknown,
            )
            .expect_err(expr)
            .error
        };
        assert!(
            matches!(
                refused("eq(raw)"),
                ExprError::ArgumentCount { ref callee, expected: 2, actual: 1 } if callee == "eq"
            ),
            "{:?}",
            refused("eq(raw)")
        );
        assert!(
            matches!(
                refused("eq(raw, n)"),
                ExprError::TypeMismatch { observed: Some(ref written), .. } if written == "n"
            ),
            "{:?}",
            refused("eq(raw, n)")
        );
    }

    /// A function the document provides under the name `eq` — an import
    /// aliased `eq` — is that function, called as declared, not the builtin.
    /// It takes two arguments, as the builtin does, so only the question of
    /// WHICH `eq` separates the call from a comparison.
    #[test]
    fn an_eq_the_document_provides_is_called_not_lowered() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("n", int(true, 32));
        ctx.insert_var("m", int(true, 32));
        ctx.insert_func(
            "eq",
            FuncSig::new(vec![int(true, 32), int(true, 32)], InferredType::Bool),
        );
        assert_eq!(tp_with("eq(n, m)", ExprTarget::Rust, &ctx), "eq(n, m)");
    }

    // Where a failure can be received a byte string is the `{data, len}` struct C
    // gives a view or a buffer, so its two sides of an equality are the pointer and
    // the length of each, whichever expression holds it; a literal fixes the count.
    #[test]
    fn bytes_equality_where_failures_are_received_reads_the_data_and_len_of_a_struct() {
        let mut ctx = bytes_ctx();
        ctx.receives_failures = true;
        ctx.insert_var("other", InferredType::Bytes);
        let eq = |expr: &str| tp_with(expr, ExprTarget::C, &ctx);
        assert_eq!(
            eq("raw === 'ack'"),
            "raw.len == 3 && memcmp(raw.data, \"ack\", 3) == 0"
        );
        assert_eq!(
            eq("raw !== 'ack'"),
            "(raw.len != 3 || memcmp(raw.data, \"ack\", 3) != 0)"
        );
        assert_eq!(
            eq("raw === other"),
            "raw.len == other.len && memcmp(raw.data, other.data, raw.len) == 0"
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
            "(raw_len != 3 || memcmp(raw, \"ack\", 3) != 0)"
        );
    }

    // A non-ASCII / escape-bearing literal is NOT reinterpreted (RFC §bytesguard-3
    // B2): the node stays a `StringLit`, so the comparison does not lower
    // to a byte constant. The receive-side validator rejects it later;
    // here we only assert the reinterpretation declined.
    #[test]
    fn bytes_non_ascii_literal_not_reinterpreted() {
        let ctx = bytes_ctx();
        // The lexer reads `\t` into a tab, which is not printable ASCII, so
        // the literal is declined; the node stays a StringLit, so no
        // `BytesLit` byte constant (`== b"…"`) is emitted. The gate /
        // validator handles the residual mismatch, not codegen.
        let out = tp_with("raw === 'a\\tb'", ExprTarget::Rust, &ctx);
        assert_eq!(out, "raw == \"a\\tb\"");
        assert!(
            !out.contains("== b\""),
            "must not reinterpret as bytes: {out}"
        );
    }

    /// A value that lands in a place that owns a byte string is bytes
    /// ([`land_strings_in_bytes_slot`]): a literal of printable ASCII lands as the
    /// constant of its bytes, the arms of a conditional one by one, and any other
    /// string is refused there by name, on every backend. Through the generated
    /// Rust machines a literal left a string is never a test's verdict, since
    /// `"xy".to_vec()` is refused by rustc, so the landing is judged here.
    #[test]
    fn a_string_lands_where_a_byte_string_is_held_only_as_a_printable_ascii_literal() {
        let mut ctx = bytes_ctx();
        ctx.insert_var("title", InferredType::Str);
        ctx.insert_var("flag", InferredType::Bool);
        let owned = |expr: &str, target: ExprTarget| {
            transpile_into_owned(expr, target, &ctx, &empty_renames(), InferredType::Bytes)
        };
        let rust = |expr: &str| match owned(expr, ExprTarget::Rust) {
            Ok(lowered) => lowered.text,
            Err(e) => panic!("{expr} lowers owned for Rust: {e:?}"),
        };
        assert_eq!(rust("'xy'"), "b\"xy\".to_vec()");
        assert_eq!(
            rust("flag ? 'xy' : 'zw'"),
            "if flag { b\"xy\".to_vec() } else { b\"zw\".to_vec() }"
        );
        for target in ExprTarget::ALL {
            for text in ["title", "'x' + 'y'", "'\u{e9}'", "flag ? 'xy' : title"] {
                match owned(text, target) {
                    Ok(lowered) => panic!(
                        "{target:?}, `{text}` is a text, not bytes, yet lowered to {}",
                        lowered.text
                    ),
                    Err(e) => assert!(
                        e.to_string()
                            .contains("a string where a byte string is held"),
                        "{target:?}, `{text}`: expected the refusal naming the string: {e}"
                    ),
                }
            }
        }
    }

    /// A call hands its argument as it is, so a string is refused where the
    /// parameter is a byte string ([`string_for_a_bytes_parameter`]), a text of a
    /// variable, a concatenation and a literal alike, on every backend; the bytes
    /// it takes and the `eq` builtin, which converts a literal, are not.
    #[test]
    fn a_string_is_refused_where_a_call_takes_a_byte_string() {
        let mut ctx = bytes_ctx();
        ctx.insert_var("title", InferredType::Str);
        ctx.insert_var("flag", InferredType::Bool);
        ctx.insert_func(
            "Length",
            FuncSig::new(
                vec![InferredType::Bytes],
                InferredType::Int {
                    signed: false,
                    bits: 32,
                },
            ),
        );
        for target in ExprTarget::ALL {
            for text in ["title", "title + 'x'", "'x'", "flag ? title : 'x'"] {
                let call = format!("Length({text})");
                match transpile_into(&call, target, &ctx, &empty_renames(), InferredType::Unknown) {
                    Ok(lowered) => panic!(
                        "{target:?}, `{call}` hands a text to bytes, yet lowered to {lowered}"
                    ),
                    Err(e) => assert!(
                        e.to_string()
                            .contains("a string where a byte string is held"),
                        "{target:?}, `{call}`: expected the refusal naming the string: {e}"
                    ),
                }
            }
        }
        assert_eq!(
            tp_with("Length(raw)", ExprTarget::Rust, &ctx),
            "length(raw)"
        );
        assert_eq!(
            tp_with("eq(raw, 'ack')", ExprTarget::Rust, &ctx),
            "raw == b\"ack\""
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
            "rpm == 0 || sce_text_compare(engine_state, \"STOP\") != 0"
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

    /// An index is read as the `Int` Kotlin's arrays take, converted once from
    /// the type its text has. An operation on a narrow unsigned value is
    /// computed in `Int` (bitwise) or `UInt` (arithmetic), not in the type it
    /// is inferred to, and an `Int` converted to `Int` again is a redundant
    /// call kotlinc `-Werror` refuses: `algorithm_lcg_table`'s `LCG_TABLE[n & 15]`.
    #[test]
    fn kotlin_index_is_converted_to_int_once() {
        let mut ctx = TypeCtx::new();
        ctx.insert_array_elem("table", int(false, 32));
        ctx.insert_var("n", int(false, 8));
        assert_eq!(
            tp_with("table[n & 15]", ExprTarget::Kotlin, &ctx),
            "table[n.toInt() and 15]"
        );
        assert_eq!(
            tp_with("table[n + 1]", ExprTarget::Kotlin, &ctx),
            "table[(n.toUInt() + 1.toUInt()).toInt()]"
        );
        assert_eq!(
            tp_with("table[n]", ExprTarget::Kotlin, &ctx),
            "table[(n).toInt()]"
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

    /// Kotlin's `shl`/`shr`/`ushr` take `bitCount: Int` on every receiver,
    /// so a count is an `Int` whatever it shifts — a 64-bit or 32-bit
    /// unsigned value included, where the count used to be converted to the
    /// shifted type and kotlinc refused it (`k shl 15.toULong()`, in
    /// sce:std/hash/murmur3_32 and sce:std/merge/hlc_text). `UInt` and `ULong`
    /// have no `ushr`: their `shr` is the logical shift. The operator is written
    /// where the count is known below the width, here proven so
    /// ([`TypeCtx::shift_counts`]).
    #[test]
    fn kotlin_shift_count_is_an_int_whatever_it_shifts() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("k", int(false, 64));
        ctx.insert_var("c", int(false, 32));
        ctx.insert_var("s", int(false, 32));
        for source in ["k >> s", "c >>> s"] {
            ctx.shift_counts.insert(
                (source.to_string(), 0, source.len()),
                ShiftCount::BelowWidth,
            );
        }
        assert_eq!(tp_with("k << 15", ExprTarget::Kotlin, &ctx), "k shl 15");
        assert_eq!(
            tp_with("k >> s", ExprTarget::Kotlin, &ctx),
            "k shr s.toInt()"
        );
        assert_eq!(
            tp_with("c >>> s", ExprTarget::Kotlin, &ctx),
            "c shr s.toInt()"
        );
    }

    /// The places the literal-width tests below read: a `uint64` `prev`, a
    /// `uint8` count `m`, an `int32` `s` and an `int16` `h`.
    fn literal_width_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("prev", int(false, 64));
        ctx.insert_var("m", int(false, 8));
        ctx.insert_var("s", int(true, 32));
        ctx.insert_var("h", int(true, 16));
        ctx
    }

    /// `expr` lowered for `target` into a place declared `slot`.
    fn landed(expr: &str, target: ExprTarget, slot: InferredType) -> String {
        transpile_into(expr, target, &literal_width_ctx(), &empty_renames(), slot)
            .unwrap_or_else(|e| panic!("{expr} lowers for {target:?}: {e:?}"))
    }

    /// SCE_FORGE.md §3.4.1, "The width of a shift": the shift is as wide as
    /// its left operand, and a literal there takes the `uint64` of the place
    /// the value lands in, which every backend writes (`land_integers`,
    /// `typed_integer_literal`). The count `m`, a `uint8`, types nothing.
    ///
    /// Before the rule the shift took `m`'s `uint8`, and Rust wrote
    /// `prev | (1 << m) as u64`, an `i32` shift.
    ///
    /// The count `m` is known at least 0 by its type and not below the width,
    /// so the shift is the total one: the operator where the language's own is
    /// that (Go, Swift), and elsewhere the standard library's or the runtime's.
    #[test]
    fn a_shifted_literal_is_written_in_the_type_its_context_gives_it() {
        let u64t = int(false, 64);
        let source = "prev | (1 << m)";
        assert_eq!(
            landed(source, ExprTarget::Rust, u64t),
            "prev | u64::checked_shl(1u64, u32::from(m)).unwrap_or(0)"
        );
        assert_eq!(
            landed(source, ExprTarget::C, u64t),
            "prev | sce_forge_wrapping_shl_u64(UINT64_C(1), (uint64_t)(m))"
        );
        assert_eq!(
            landed(source, ExprTarget::Cpp, u64t),
            "prev | SCE::Forge::Wrapping::shl<std::uint64_t>(std::uint64_t{1}, m)"
        );
        assert_eq!(
            landed(source, ExprTarget::Go, u64t),
            "prev | (uint64(1) << m)"
        );
        assert_eq!(
            landed(source, ExprTarget::Kotlin, u64t),
            "prev or com.sce.forge.runtime.SceWrapping.shl(1uL, m.toULong())"
        );
        assert_eq!(
            landed(source, ExprTarget::Swift, u64t),
            "prev | (UInt64(1) << m)"
        );
        // Python's left shift would make `1 * 2**m` whole before it is reduced.
        assert_eq!(
            landed(source, ExprTarget::Python, u64t),
            "prev | sce_algorithm.U64.wrapping_shl(1, m)"
        );
        // The Interpreter's library takes the width from the operation.
        let js = landed(source, ExprTarget::Js, u64t);
        assert!(
            js.contains("SceStatic.U64.or(prev, SceStatic.U64.shl(1, m))"),
            "{js}"
        );
    }

    /// A shift with a declared left operand is that operand's width, and its
    /// count is written at its own type: converting it to the type it shifts
    /// (`prev >> m as u64`, `uint64(m)`) makes no shift right. A count proven
    /// below the width ([`TypeCtx::shift_counts`]) is the language's own
    /// operator.
    #[test]
    fn a_shift_count_is_written_at_its_own_type() {
        let u64t = int(false, 64);
        let mut ctx = literal_width_ctx();
        ctx.shift_counts
            .insert(("prev >> m".to_string(), 0, 9), ShiftCount::BelowWidth);
        let proven = |target| {
            transpile_into("prev >> m", target, &ctx, &empty_renames(), u64t)
                .unwrap_or_else(|e| panic!("prev >> m lowers for {target:?}: {e:?}"))
        };
        assert_eq!(proven(ExprTarget::Rust), "prev >> m");
        assert_eq!(proven(ExprTarget::Go), "prev >> m");
        assert_eq!(proven(ExprTarget::C), "prev >> m");
        assert_eq!(proven(ExprTarget::Kotlin), "prev shr m.toInt()");
    }

    /// SCE_FORGE.md §3.4.1, "The width of a shift": a count at least 0 but not
    /// known below the width is the total shift, its exact result reduced to the
    /// width — 0, or the sign of a signed `>>` — written as the language's own
    /// operator only where that operator is it (Go, Swift, Python's `>>`).
    #[test]
    fn a_count_not_known_below_the_width_is_the_total_shift() {
        let u64t = int(false, 64);
        let i16t = int(true, 16);
        for (target, source, slot, want) in [
            (
                ExprTarget::Rust,
                "prev >> m",
                u64t,
                "u64::checked_shr(prev, u32::from(m)).unwrap_or(0)",
            ),
            (
                ExprTarget::Rust,
                "h >> m",
                i16t,
                "i16::wrapping_shr(h, u32::min(u32::from(m), 15))",
            ),
            (
                ExprTarget::Rust,
                "h >>> m",
                i16t,
                "u16::checked_shr(h as u16, u32::from(m)).unwrap_or(0) as i16",
            ),
            (
                ExprTarget::Rust,
                "h << 20",
                i16t,
                "i16::checked_shl(h, 20).unwrap_or(0)",
            ),
            (
                ExprTarget::C,
                "h >> m",
                i16t,
                "sce_forge_wrapping_shr_i16(h, (uint64_t)(m))",
            ),
            (
                ExprTarget::Cpp,
                "h >>> m",
                i16t,
                "SCE::Forge::Wrapping::ushr<std::int16_t>(h, m)",
            ),
            (ExprTarget::Go, "prev >> m", u64t, "(prev >> m)"),
            (ExprTarget::Go, "h >>> m", i16t, "int16(uint16(h) >> m)"),
            (
                ExprTarget::Go,
                "h << 20",
                i16t,
                "scealgorithm.WrappingShl(h, 20)",
            ),
            (
                ExprTarget::Kotlin,
                "h >> m",
                i16t,
                "com.sce.forge.runtime.SceWrapping.shr(h, m.toULong())",
            ),
            (ExprTarget::Python, "h >> m", i16t, "(h >> m)"),
            (
                ExprTarget::Python,
                "h >>> m",
                i16t,
                "sce_algorithm.I16.wrapping_ushr(h, m)",
            ),
            (
                ExprTarget::Python,
                "prev << m",
                u64t,
                "sce_algorithm.U64.wrapping_shl(prev, m)",
            ),
            (ExprTarget::Swift, "h >> m", i16t, "(h >> m)"),
        ] {
            assert_eq!(landed(source, target, slot), want, "{target:?}: {source}");
        }
    }

    /// A `>>>` of a signed value shifts its bits at its width logically, which no
    /// C-family, Rust, Go or Python operator does, so even a count known below the
    /// width is written as the total shift there.
    #[test]
    fn a_logical_shift_of_a_signed_value_is_made_on_its_bits() {
        let i32t = int(true, 32);
        assert_eq!(
            landed("s >>> 3", ExprTarget::C, i32t),
            "sce_forge_wrapping_ushr_i32(s, (uint64_t)(3))"
        );
        assert_eq!(
            landed("s >>> 3", ExprTarget::Rust, i32t),
            "u32::checked_shr(s as u32, 3).unwrap_or(0) as i32"
        );
        assert_eq!(
            landed("s >>> 3", ExprTarget::Go, i32t),
            "int32(uint32(s) >> 3)"
        );
        assert_eq!(
            landed("s >>> 3", ExprTarget::Kotlin, i32t),
            "com.sce.forge.runtime.SceWrapping.ushr(s, 3.toULong())"
        );
    }

    /// A count that may be below 0, in a body that receives failures, is checked
    /// when it runs: below 0 it fails `out-of-range` (SCE_FORGE.md §3.4.1).
    /// Outside such a body it is the total shift, the count unchecked, as a
    /// divisor of 0 is unchecked there (SCE_FORGE.md §6.5).
    #[test]
    fn a_count_that_may_be_below_zero_is_checked_where_failures_are_received() {
        let i32t = int(true, 32);
        let mut ctx = literal_width_ctx();
        ctx.receives_failures = true;
        let checked = |target| {
            transpile_into("s << s", target, &ctx, &empty_renames(), i32t)
                .unwrap_or_else(|e| panic!("s << s lowers for {target:?}: {e:?}"))
        };
        assert_eq!(
            checked(ExprTarget::Rust),
            "sce_forge_runtime::algorithm::shl::<i32, _>(s, s)?"
        );
        assert_eq!(
            checked(ExprTarget::C),
            "sce_forge_checked_shl_i32(&sce_failure_, s, (int64_t)(s))"
        );
        assert_eq!(
            checked(ExprTarget::Cpp),
            "SCE::Forge::Checked::shl<std::int32_t>(sce_failure_, s, s)"
        );
        assert_eq!(
            checked(ExprTarget::Go),
            "scealgorithm.ShlInt32(&sceFailure, s, int64(s))"
        );
        assert_eq!(
            checked(ExprTarget::Kotlin),
            "com.sce.forge.runtime.SceChecked.shl(s, s.toLong())"
        );
        assert_eq!(checked(ExprTarget::Python), "sce_algorithm.I32.shl(s, s)");
        assert_eq!(checked(ExprTarget::Swift), "(try Checked.shl(s, s))");
        assert_eq!(
            landed("s << s", ExprTarget::C, i32t),
            "sce_forge_wrapping_shl_i32(s, (uint64_t)(s))"
        );
        // An unsigned count is at least 0 by its type, and never checked.
        assert_eq!(
            transpile_into("s << m", ExprTarget::Rust, &ctx, &empty_renames(), i32t).unwrap(),
            "i32::checked_shl(s, u32::from(m)).unwrap_or(0)"
        );
    }

    /// In the body of an algorithm that does not declare `may-fail`, the
    /// analysis refused every count that may be below 0, so a signed count is
    /// the total shift there, never the checked one.
    #[test]
    fn a_proven_body_never_checks_a_count() {
        let i32t = int(true, 32);
        let mut ctx = literal_width_ctx();
        ctx.integers_proven = true;
        assert_eq!(
            transpile_into("s >> s", ExprTarget::Go, &ctx, &empty_renames(), i32t).unwrap(),
            "(s >> s)"
        );
        assert_eq!(
            transpile_into("s >> s", ExprTarget::C, &ctx, &empty_renames(), i32t).unwrap(),
            "sce_forge_wrapping_shr_i32(s, (uint64_t)(s))"
        );
    }

    /// An operation of literals alone takes the type of the place it lands in,
    /// and its literals are written in it, so `65536 * 65536` is `2^32` and
    /// not a 32-bit product that overflows.
    #[test]
    fn an_operation_of_literals_alone_takes_the_type_of_its_place() {
        let u64t = int(false, 64);
        let source = "prev + 65536 * 65536";
        assert_eq!(
            landed(source, ExprTarget::Rust, u64t),
            "prev + 65536u64 * 65536u64"
        );
        assert_eq!(
            landed(source, ExprTarget::C, u64t),
            "prev + UINT64_C(65536) * UINT64_C(65536)"
        );
        assert_eq!(
            landed(source, ExprTarget::Kotlin, u64t),
            "prev + 65536uL * 65536uL"
        );
        assert_eq!(
            landed(source, ExprTarget::Swift, u64t),
            "prev + UInt64(65536) * UInt64(65536)"
        );
        // A literal that meets a declared operand keeps no type of its own.
        assert_eq!(landed("prev + 1", ExprTarget::Rust, u64t), "prev + 1");
    }

    /// Python's integers have no width: a left shift is reduced to its type at
    /// every width and sign, and a `~` of an unsigned value to the complement
    /// of its bits, not the negative number Python's `~` makes.
    #[test]
    fn python_reduces_a_shift_and_a_complement_to_their_width() {
        assert_eq!(
            landed("s << 3", ExprTarget::Python, int(true, 32)),
            "((((s << 3) + 0x80000000) & 0xFFFFFFFF) - 0x80000000)"
        );
        assert_eq!(
            landed("~(1 << m)", ExprTarget::Python, int(false, 64)),
            "((~sce_algorithm.U64.wrapping_shl(1, m)) & 0xFFFFFFFFFFFFFFFF)"
        );
    }

    /// A reduction is one operand under any operator around it. Written bare,
    /// `(m << 4) & 0xFF >> 4` is `(m << 4) & 0x0F` in Python, whose `&` binds
    /// looser than `>>`, `+` and `==`; parenthesised, each line below is the
    /// wrapped value: `((1 << 63) & M) >> 1` is `2^62`, `((m << 4) & 0xFF) >> 4`
    /// is the low four bits of `m`, and `((m << 4) & 0xFF) == 16` holds for
    /// `m = 17`.
    #[test]
    fn a_python_reduction_binds_as_one_operand() {
        assert_eq!(
            landed("((1 << m) >> 1) + 1", ExprTarget::Python, int(false, 64)),
            "(sce_algorithm.U64.wrapping_shl(1, m) >> 1) + 1"
        );
        assert_eq!(
            landed("(m << 4) >> 4", ExprTarget::Python, int(false, 8)),
            "((m << 4) & 0xFF) >> 4"
        );
        assert_eq!(
            landed("(m << 4) === 16", ExprTarget::Python, InferredType::Bool),
            "((m << 4) & 0xFF) == 16"
        );
    }

    /// A shift of an operand narrower than `int` wraps at its own width in C,
    /// C++ and Kotlin as in every other backend (SCE_FORGE.md §3.4.1, "The
    /// width of a shift"), and so does its `~`: each promotes the operand to a
    /// 32-bit integer, so the shift is converted back to its type where it is
    /// made. A signed one is made in the unsigned type of its width, since a
    /// left shift of a negative value is undefined in C and C++.
    #[test]
    fn a_narrow_shift_wraps_at_its_width_in_the_c_family_and_kotlin() {
        let u32t = int(false, 32);
        let i32t = int(true, 32);
        assert_eq!(landed("m << 4", ExprTarget::C, u32t), "(uint8_t)(m << 4)");
        assert_eq!(
            landed("(m << 4) >> 4", ExprTarget::C, int(false, 8)),
            "(uint8_t)(m << 4) >> 4"
        );
        assert_eq!(
            landed("h << 8", ExprTarget::C, i32t),
            "(int16_t)((uint16_t)(h) << 8)"
        );
        assert_eq!(landed("~m", ExprTarget::C, u32t), "(uint8_t)(~m)");
        assert_eq!(
            landed("m << 4", ExprTarget::Cpp, u32t),
            "static_cast<std::uint8_t>(m << 4)"
        );
        assert_eq!(
            landed("h << 8", ExprTarget::Cpp, i32t),
            "static_cast<std::int16_t>(static_cast<std::uint16_t>(h) << 8)"
        );
        assert_eq!(
            landed("~m", ExprTarget::Cpp, u32t),
            "static_cast<std::uint8_t>(~m)"
        );
        // A shift of a 64-bit unsigned value by a count not known below the
        // width is the runtime's total shift, which is made at its width.
        assert_eq!(
            landed("prev << m", ExprTarget::C, int(false, 64)),
            "sce_forge_wrapping_shl_u64(prev, (uint64_t)(m))"
        );
        assert_eq!(
            landed("m << 4", ExprTarget::Kotlin, u32t),
            "((m.toInt() shl 4).toUByte()).toUInt()"
        );
        assert_eq!(
            landed("h << 8", ExprTarget::Kotlin, i32t),
            "((h.toInt() shl 8).toShort()).toInt()"
        );
        assert_eq!(
            landed("~h", ExprTarget::Kotlin, i32t),
            "((h.toInt().inv()).toShort()).toInt()"
        );
    }

    /// A shift or a `~` of a literal that nothing gives a width is refused at
    /// the literal, naming what would give it one: compared with another
    /// literal, or lowered where no type is declared.
    #[test]
    fn a_shifted_literal_with_no_width_is_refused() {
        let ctx = literal_width_ctx();
        for (source, slot) in [
            ("(1 << m) === 0", InferredType::Bool),
            ("1 << m", InferredType::Unknown),
            ("~0", InferredType::Unknown),
        ] {
            let refusal = transpile_typed(source, ExprTarget::Rust, &ctx, &empty_renames(), slot)
                .expect_err(source);
            assert!(
                matches!(
                    &refusal.error,
                    ExprError::TypeMismatch { got, observed: Some(o), .. }
                        if got == "integer literal" && (o == "1" || o == "0")
                ),
                "{source}: {refusal:?}"
            );
        }
        // Compared with a declared value, the shift takes its type.
        assert_eq!(
            landed("prev === (1 << m)", ExprTarget::Rust, InferredType::Bool),
            "prev == u64::checked_shl(1u64, u32::from(m)).unwrap_or(0)"
        );
    }

    /// `==` gives a Kotlin literal no type to adapt to, so an untyped operand
    /// compared with a `Long`/`Short`/`Byte` is spelled in that type —
    /// `Long == Int` does not compile. `Int` and ordering comparisons, which
    /// Kotlin already accepts, are left as written.
    #[test]
    fn kotlin_equality_spells_the_literal_in_the_operand_type() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("week", int(true, 64));
        ctx.insert_var("s", int(true, 16));
        ctx.insert_var("b", int(true, 8));
        ctx.insert_var("n", int(true, 32));
        let kt = |src: &str| {
            transpile_typed(
                src,
                ExprTarget::Kotlin,
                &ctx,
                &HashMap::new(),
                InferredType::Unknown,
            )
            .expect("lowers")
        };
        assert_eq!(kt("week % 7 === 0"), "week % 7 == 0L");
        assert_eq!(kt("0 !== week"), "0L != week");
        // An operation of literals takes the type it is compared with, so its
        // literals are written in it (`land_integers`).
        assert_eq!(kt("week === 1 + 2"), "week == 1L + 2L");
        assert_eq!(kt("s === 3"), "s == 3.toShort()");
        assert_eq!(kt("b === 3"), "b == 3.toByte()");
        assert_eq!(kt("n === 3"), "n == 3");
        assert_eq!(kt("week < 3"), "week < 3");
    }

    /// The optional values the Kotlin cases below read (docs/adr/0055): `due`
    /// and `other` of `optional<int64>`, `count` of `optional<uint32>`, `small`
    /// of `optional<uint16>`, `title` of `optional<string>`, `blob` and `seal`
    /// of `optional<bytes>`, beside an `int64` `huge`; `due` and `count` are
    /// proven present, as a `!== null` test around the read proves them.
    fn kotlin_optional_ctx() -> TypeCtx<'static> {
        use crate::forge::quantity::NumericBaseType;
        let number = |signed: bool, bits: u8| {
            InferredType::Optional(OptionalElem::Number(NumericBaseType::Int { signed, bits }))
        };
        let mut ctx = TypeCtx::new();
        ctx.insert_var("due", number(true, 64));
        ctx.insert_var("other", number(true, 64));
        ctx.insert_var("count", number(false, 32));
        ctx.insert_var("small", number(false, 16));
        ctx.insert_var("title", InferredType::Optional(OptionalElem::Str));
        ctx.insert_var("blob", InferredType::Optional(OptionalElem::Bytes));
        ctx.insert_var("seal", InferredType::Optional(OptionalElem::Bytes));
        ctx.insert_var("huge", int(true, 64));
        ctx.proven =
            crate::forge::narrowing::proves_written("due !== null && count !== null", true);
        ctx
    }

    /// `src` lowered for Kotlin into a place of `expected`, judged against `ctx`.
    fn kotlin_optional(src: &str, ctx: &TypeCtx<'_>, expected: InferredType) -> String {
        transpile_into(src, ExprTarget::Kotlin, ctx, &empty_renames(), expected)
            .unwrap_or_else(|e| panic!("{src} lowers for Kotlin: {e:?}"))
    }

    /// docs/adr/0055, section 5, the Kotlin row: absent is `null`, a test is
    /// `!= null`, a proven read is the runtime's `SceOptional.proven` (never
    /// `!!`, which a smart-cast value warns on), `??` is the elvis and `===`
    /// is `==`.
    #[test]
    fn kotlin_spells_each_read_of_an_optional_value() {
        let ctx = kotlin_optional_ctx();
        let unknown = InferredType::Unknown;
        assert_eq!(
            kotlin_optional("due !== null ? due.value : 0", &ctx, int(true, 64)),
            "if (due != null) com.sce.forge.runtime.SceOptional.proven(due) else 0"
        );
        assert_eq!(
            kotlin_optional("due.value + 1", &ctx, int(true, 64)),
            "com.sce.forge.runtime.SceOptional.proven(due) + 1"
        );
        assert_eq!(
            kotlin_optional("count.value", &ctx, int(false, 32)),
            "com.sce.forge.runtime.SceOptional.proven(count)"
        );
        assert_eq!(
            kotlin_optional("due === null", &ctx, unknown),
            "due == null"
        );
        // A test of presence names its subject first, whichever side `null`
        // was written on (`OptionalForm::Test`).
        assert_eq!(
            kotlin_optional("null !== due", &ctx, unknown),
            "due != null"
        );
        assert_eq!(
            kotlin_optional("due === other", &ctx, unknown),
            "due == other"
        );
    }

    /// A literal beside an optional number is spelled in the number's own type
    /// (`Long? == Int` does not compile); an optional byte string is compared by
    /// its contents, and against `null` by `==`.
    #[test]
    fn kotlin_compares_an_optional_value_by_its_present_type() {
        let ctx = kotlin_optional_ctx();
        let unknown = InferredType::Unknown;
        assert_eq!(kotlin_optional("due === 5", &ctx, unknown), "due == 5L");
        assert_eq!(
            kotlin_optional("count !== 5", &ctx, unknown),
            "count != 5.toUInt()"
        );
        assert_eq!(
            kotlin_optional("blob === seal", &ctx, unknown),
            "blob.contentEquals(seal)"
        );
        assert_eq!(
            kotlin_optional("blob !== seal", &ctx, unknown),
            "!blob.contentEquals(seal)"
        );
        assert_eq!(
            kotlin_optional("blob === null", &ctx, unknown),
            "blob == null"
        );
    }

    /// `x ?? d` is the elvis, `d` spelled in the type the read is of; the elvis
    /// binds tighter than `==` and looser than `+` in Kotlin, so it is
    /// parenthesised under the one and not under the other.
    #[test]
    fn kotlin_spells_a_read_with_a_default_as_the_elvis() {
        let ctx = kotlin_optional_ctx();
        assert_eq!(
            kotlin_optional("due ?? 0", &ctx, int(true, 64)),
            "due ?: 0L"
        );
        assert_eq!(
            kotlin_optional("due ?? huge * huge", &ctx, int(true, 64)),
            "due ?: huge * huge"
        );
        assert_eq!(
            kotlin_optional("count ?? 0", &ctx, int(false, 32)),
            "count ?: 0.toUInt()"
        );
        assert_eq!(
            kotlin_optional("title ?? 'none'", &ctx, InferredType::Str),
            "title ?: \"none\""
        );
        assert_eq!(
            kotlin_optional("(due ?? 0) + 1", &ctx, int(true, 64)),
            "(due ?: 0L) + 1"
        );
        assert_eq!(
            kotlin_optional("(due ?? 0) === 3", &ctx, InferredType::Unknown),
            "due ?: 0L == 3L"
        );
    }

    /// A value of `T` where an `optional<T>` is expected is present, spelled as
    /// the `T` it is; an optional number of another width is converted where it
    /// is present and stays `null` where it is absent.
    #[test]
    fn kotlin_lifts_a_present_value_into_an_optional_place() {
        use crate::forge::quantity::NumericBaseType;
        let ctx = kotlin_optional_ctx();
        let optional = |signed: bool, bits: u8| {
            InferredType::Optional(OptionalElem::Number(NumericBaseType::Int { signed, bits }))
        };
        assert_eq!(kotlin_optional("7", &ctx, optional(true, 64)), "7L");
        assert_eq!(
            kotlin_optional("7", &ctx, optional(false, 32)),
            "7.toUInt()"
        );
        assert_eq!(kotlin_optional("null", &ctx, optional(true, 64)), "null");
        assert_eq!(kotlin_optional("other", &ctx, optional(true, 64)), "other");
        assert_eq!(
            kotlin_optional("small", &ctx, optional(false, 32)),
            "small?.toUInt()"
        );
        assert_eq!(
            kotlin_optional("due !== null ? 1 : null", &ctx, optional(true, 64)),
            "if (due != null) 1L else null"
        );
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

    /// A comparison that is an operand of a comparison keeps its parentheses
    /// on every backend. Found by a consumer whose
    /// `(o.kind === WhoKind.of_department) === p.ofDepartment` passed `check`
    /// and was generated as `a == b == c`, which rustc refuses; Python ran the
    /// same text as a chained comparison and answered something else.
    #[test]
    fn a_comparison_under_a_comparison_keeps_its_parentheses() {
        for target in ExprTarget::ALL {
            assert_eq!(tp("(a === b) === c", target), "(a == b) == c", "{target:?}");
            assert_eq!(tp("a === (b === c)", target), "a == (b == c)", "{target:?}");
            assert_eq!(
                tp("(a < b) === (c > d)", target),
                "(a < b) == (c > d)",
                "{target:?}"
            );
        }
    }

    #[test]
    fn cpp_precedence_bitwise_vs_comparison() {
        // `===` binds tighter than `&` in both languages, so the parens change
        // nothing about the meaning; they are there because GCC refuses a bare
        // comparison as an operand of `&` under `-Werror=parentheses`.
        assert_eq!(tp("a & 0xFF === b", ExprTarget::Cpp), "a & (0xFF == b)");
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

    /// Every other operand shape GCC's `-Wparentheses` names, parenthesised
    /// in the C family and left bare elsewhere.
    ///
    /// ⚠ THE WITNESS IS A BUILD FAILURE. `sce:std/hash/murmur3_32` emitted
    /// `h = (h & 0xFFFFFFFF) * 5 + 0xe6546b64 & 0xFFFFFFFF;` for C11 and the
    /// conformance build stopped on
    ///
    ///     error: suggest parentheses around '+' in operand of '&'
    ///            [-Werror=parentheses]
    #[test]
    fn c_family_parenthesises_every_wparentheses_operand() {
        let cases = [
            ("h * 5 + c & m", "(h * 5 + c) & m"),
            ("a - b | c", "(a - b) | c"),
            ("a + b ^ c", "(a + b) ^ c"),
            ("a & b | c", "(a & b) | c"),
            ("a ^ b | c", "(a ^ b) | c"),
            ("a & b ^ c", "(a & b) ^ c"),
            ("a << b + c", "a << (b + c)"),
            ("a + b >> c", "(a + b) >> c"),
            ("a < b | c", "(a < b) | c"),
        ];
        for target in [ExprTarget::Cpp, ExprTarget::C] {
            for (src, want) in cases {
                assert_eq!(tp(src, target), want, "{target:?} left `{src}` bare");
            }
            // shapes GCC does not warn about stay bare
            assert_eq!(tp("a * b & c", target), "a * b & c");
            assert_eq!(tp("a & b & c", target), "a & b & c");
            assert_eq!(tp("a | b | c", target), "a | b | c");
        }
        assert_eq!(tp("h * 5 + c & m", ExprTarget::Rust), "h * 5 + c & m");
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

    /// `??` reads an optional value with a default (docs/adr/0055): the forge
    /// language parses it, and an emitter refuses it until its backend's step
    /// lowers an optional value. A backend that lowers one is judged by its
    /// own spelling tests instead.
    #[test]
    fn coalescing_is_parsed_and_refused_where_no_backend_lowers_it() {
        assert!(parse_to_ast("a ?? b").is_ok());
        for target in ExprTarget::ALL
            .into_iter()
            .chain([ExprTarget::Js])
            .filter(|target| !target.lowers_optional())
        {
            let e = tp_err("a ?? b", target);
            assert!(
                e.contains("an optional value has no") && e.contains("lowering yet"),
                "{target:?}: {e}"
            );
        }
    }

    /// ECMAScript refuses `??` written beside a bare `&&` or `||`, and so does
    /// the forge language: which is read first is the author's to write.
    #[test]
    fn coalescing_beside_a_bare_logical_operator_is_refused() {
        for mixed in ["a ?? b || c", "a || b ?? c", "a && b ?? c", "a ?? b && c"] {
            let e = parse_to_ast(mixed)
                .map(|_| ())
                .unwrap_err()
                .error
                .to_string();
            assert!(e.contains("`??` beside `&&` or `||`"), "{mixed}: {e}");
        }
        for grouped in [
            "(a ?? b) || c",
            "a ?? (b || c)",
            "(a && b) ?? c",
            "a ?? b ?? c",
        ] {
            assert!(parse_to_ast(grouped).is_ok(), "{grouped} parses");
        }
    }

    /// The ECMAScript datamodel's dialect still leaves `??` out.
    #[test]
    fn the_ecmascript_dialect_still_refuses_coalescing() {
        let tokens = tokenize_as("a ?? b", LexMode::EcmaScript).expect("tokenizes");
        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::Unsupported { spelling: "??", .. })));
        let tokens = tokenize_as("a ?? b", LexMode::Forge).expect("tokenizes");
        assert!(tokens.contains(&Token::QuestionQuestion));
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

    // ── An optional value in C++ (docs/adr/0055) ────────────────

    /// The element of an optional `int64`.
    fn optional_int64() -> InferredType {
        InferredType::Optional(OptionalElem::Number(
            crate::forge::quantity::NumericBaseType::Int {
                signed: true,
                bits: 64,
            },
        ))
    }

    /// A machine's variables as the optional scenario declares them: `due` and
    /// `other` optional days, `title` an optional text, `status` an optional
    /// value of an enum, and the present `huge`, `count` and `flag`.
    fn cpp_optional_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("due", optional_int64());
        ctx.insert_var("other", optional_int64());
        ctx.insert_var("title", InferredType::Optional(OptionalElem::Str));
        ctx.insert_var("status", InferredType::Optional(OptionalElem::Enum));
        ctx.insert_var("huge", int(true, 64));
        ctx.insert_var("count", int(false, 64));
        ctx.insert_var("flag", InferredType::Bool);
        ctx
    }

    /// Each variable renamed to the member that holds it, as the lowering of a
    /// statechart renames it.
    fn cpp_optional_renames() -> HashMap<&'static str, &'static str> {
        [
            ("due", "v_due"),
            ("other", "v_other"),
            ("title", "v_title"),
            ("status", "v_status"),
            ("huge", "v_huge"),
            ("count", "v_count"),
            ("flag", "v_flag"),
        ]
        .into_iter()
        .collect()
    }

    /// `expr` typed against [`cpp_optional_ctx`], renamed by
    /// [`cpp_optional_renames`] and spelled by the C++ emitter into a place of
    /// `expected`: what the emitter writes of each construct of an optional
    /// value, after the renames a statechart's lowering makes, which the types
    /// survive.
    fn cpp_spelled(expr: &str, expected: InferredType) -> String {
        let mut ast = parse_to_ast(expr).unwrap();
        infer_types(&mut ast, &cpp_optional_ctx());
        rename_identifiers(&mut ast, &cpp_optional_renames());
        emit_cpp(&ast, expected).unwrap()
    }

    /// The whole pipeline lowers an optional value for C++, the judge's passes
    /// included: a test, a read its own conditional proves, and a read under a
    /// test the context already holds.
    #[test]
    fn cpp_lowers_an_optional_value_through_the_whole_pipeline() {
        let ctx = optional_ctx();
        let cpp = |expr: &str, expected: InferredType| {
            transpile_into(expr, ExprTarget::Cpp, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers for C++: {e:?}"))
        };
        assert_eq!(cpp("due !== null", InferredType::Bool), "due.has_value()");
        assert_eq!(
            cpp("other !== null ? other.value : 0", int(true, 64)),
            "other.has_value() ? (*other) : 0"
        );
        assert_eq!(cpp("due.value", int(true, 64)), "(*due)");
        assert_eq!(cpp("null === null", InferredType::Bool), "true");
    }

    #[test]
    fn cpp_tests_presence_with_has_value() {
        assert_eq!(
            cpp_spelled("due === null", InferredType::Bool),
            "!v_due.has_value()"
        );
        assert_eq!(
            cpp_spelled("null !== due", InferredType::Bool),
            "v_due.has_value()"
        );
    }

    #[test]
    fn cpp_reads_a_proven_value_through_the_optional() {
        assert_eq!(
            cpp_spelled("due !== null ? due.value : 0", int(true, 64)),
            "v_due.has_value() ? (*v_due) : 0"
        );
    }

    #[test]
    fn cpp_hands_the_default_of_a_coalesce_over_as_a_lambda() {
        assert_eq!(
            cpp_spelled("due ?? (huge * huge)", int(true, 64)),
            "SCE::Forge::otherwise(v_due, [&] { return v_huge * v_huge; })"
        );
        assert_eq!(
            cpp_spelled("title ?? 'none'", InferredType::Str),
            "SCE::Forge::otherwise(v_title, [&] { return \"none\"; })"
        );
    }

    #[test]
    fn cpp_compares_optional_values_with_the_equality_of_optional() {
        assert_eq!(
            cpp_spelled("due === other", InferredType::Bool),
            "v_due == v_other"
        );
        assert_eq!(
            cpp_spelled("status !== status", InferredType::Bool),
            "v_status != v_status"
        );
    }

    #[test]
    fn cpp_spells_null_in_an_optional_place_as_nullopt() {
        assert_eq!(cpp_spelled("null", optional_int64()), "std::nullopt");
    }

    #[test]
    fn cpp_lifts_a_present_value_as_its_element() {
        assert_eq!(cpp_spelled("7", optional_int64()), "7");
        assert_eq!(
            cpp_spelled("count", optional_int64()),
            "static_cast<std::int64_t>(v_count)"
        );
    }

    #[test]
    fn cpp_gives_a_conditional_of_null_the_optional_of_its_value() {
        assert_eq!(
            cpp_spelled("flag ? null : 3", optional_int64()),
            "v_flag ? std::optional<std::int64_t>() : std::optional<std::int64_t>(3)"
        );
        assert_eq!(
            cpp_spelled(
                "flag ? 'x' : null",
                InferredType::Optional(OptionalElem::Str)
            ),
            "v_flag ? std::optional<std::string>(\"x\") : std::optional<std::string>()"
        );
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
    fn a_literal_is_written_in_a_notation_every_backend_reads() {
        let ctx = TypeCtx::new();
        for target in ExprTarget::ALL {
            let emit = |expr: &str| {
                transpile_typed(expr, target, &ctx, &empty_renames(), InferredType::Unknown)
                    .unwrap()
            };
            // Octal and binary become hex, and hex's prefix is lowered.
            assert_eq!(emit("0o17"), "0xF", "{target:?}");
            assert_eq!(emit("0O17"), "0xF", "{target:?}");
            assert_eq!(emit("0b1010"), "0xA", "{target:?}");
            assert_eq!(emit("0B1010"), "0xA", "{target:?}");
            assert_eq!(emit("0XFF"), "0xFF", "{target:?}");
            // What every backend already reads stands as written.
            assert_eq!(emit("0x0F"), "0x0F", "{target:?}");
            assert_eq!(emit("15"), "15", "{target:?}");
        }
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

    #[test]
    fn a_hex_digit_e_is_not_an_exponent() {
        // `0xcc9e2d51` holds the digit `e`; read as an exponent it typed the
        // product a real, and Go refused `&` on the float64 it emitted.
        let ctx = ctx_with_uint("k", 64);
        let uint64 = InferredType::Int {
            signed: false,
            bits: 64,
        };
        for target in [ExprTarget::Go, ExprTarget::Rust, ExprTarget::Cpp] {
            let out = transpile_typed(
                "(k * 0xcc9e2d51) & 0xFFFFFFFE",
                target,
                &ctx,
                &empty_renames(),
                uint64,
            )
            .unwrap();
            assert!(
                !out.contains("float") && !out.contains("f64") && !out.contains(".0"),
                "{out}"
            );
        }
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
        assert_eq!(py("wide", int(false, 8)), "((wide) & 0xFF)");
        assert_eq!(py("signed_wide", int(false, 8)), "((signed_wide) & 0xFF)");
        assert_eq!(
            py("signed_wide", int(false, 64)),
            "((signed_wide) & 0xFFFFFFFFFFFFFFFF)"
        );
        assert_eq!(
            py("big", int(true, 32)),
            "((((big) + 0x80000000) & 0xFFFFFFFF) - 0x80000000)"
        );
        assert_eq!(
            py("wide", int(true, 32)),
            "((((wide) + 0x80000000) & 0xFFFFFFFF) - 0x80000000)"
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
        ctx.insert_func("temp_xform", FuncSig::new(vec![int(false, 16)], float(64)));
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
        ctx.insert_func("frame.encode", FuncSig::new(vec![], InferredType::Bytes));
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

    /// An author's local named as a builtin keeps the builtin a builtin: the rename
    /// of the local (C11 reaches an algorithm's buffer through its destination,
    /// `(*len)`, docs/adr/0082) reaches the argument and not the callee, so
    /// `len(len)` is the length of the local and not a call of it.
    #[test]
    fn a_rename_of_a_local_named_as_a_builtin_leaves_the_builtin() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("len", InferredType::BytesBuffer);
        let mut renames = HashMap::new();
        renames.insert("len", "(*len)");
        let out = transpile_typed(
            "len(len)",
            ExprTarget::C,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert!(
            !out.contains("(*len)("),
            "the builtin is called as the local: {out}"
        );
        assert!(out.contains("(*len)"), "the local is renamed: {out}");
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

    /// A renamed member keeps the type inference gave it. Rename runs after
    /// inference (`resolve_then_rename`), so the member's type is known when
    /// its spelling changes, and the emitter still needs it: a Kotlin
    /// `UInt` member plus a literal must spell the literal `UInt`, since
    /// `UInt + Int` does not compile. The member arm used to reset the type
    /// to `Unknown` — a leftover from when rename ran before inference — and
    /// the literal went out bare.
    #[test]
    fn a_renamed_member_keeps_its_inferred_type() {
        let mut ctx = TypeCtx::new();
        ctx.insert_record("frame", RecordShape::Closed);
        ctx.insert_var("frame.counter", int(false, 32));
        let mut renames = HashMap::new();
        renames.insert("frame.counter", "self.frame.counter");
        let out = transpile_typed(
            "frame.counter + 1",
            ExprTarget::Kotlin,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(out, "self.frame.counter + 1.toUInt()");
    }

    /// A conditional of two integer literals is itself an untyped integer,
    /// and its conversion to an unsigned output has to wrap the whole
    /// `if` — a bare suffix binds to the last branch alone
    /// (`if (c) 1 else 0.toUByte()`), which Kotlin types as
    /// `Comparable<*>` and refuses as a `UByte` return.
    #[test]
    fn a_literal_conditional_narrows_whole_to_an_unsigned_output() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("a", int(true, 64));
        ctx.insert_var("b", int(true, 64));
        let out = transpile_typed(
            "a > b ? 1 : 0",
            ExprTarget::Kotlin,
            &ctx,
            &HashMap::new(),
            int(false, 8),
        )
        .unwrap();
        assert_eq!(out, "(if (a > b) 1 else 0).toUByte()");
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
            FuncSig::new(Vec::new(), InferredType::Bytes),
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

    // ── Wrapping and saturating operations (docs/adr/0063) ──────

    fn total_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("now", int(false, 32));
        ctx.insert_var("last", int(false, 32));
        ctx.insert_var("small", int(false, 8));
        ctx.insert_var("delta", int(true, 32));
        ctx.insert_var("ratio", float(64));
        ctx.insert_var("ready", InferredType::Bool);
        ctx
    }

    fn total(expr: &str, target: ExprTarget) -> Result<String, Refusal> {
        transpile_typed(
            expr,
            target,
            &total_ctx(),
            &empty_renames(),
            InferredType::Unknown,
        )
    }

    /// A vocabulary name is the operation it names from the parse on, so no
    /// pass reads it as a call of a function the document must declare.
    #[test]
    fn a_vocabulary_name_parses_to_the_operation_it_names() {
        let ast = parse_to_ast("wrappingSub(now, last)").unwrap();
        assert!(matches!(
            ast.kind,
            ExprKind::Total {
                op: RingOp::Sub,
                rule: OverflowRule::Wrapping,
                right: Some(_),
                ..
            }
        ));
        let ast = parse_to_ast("saturatingNeg(delta)").unwrap();
        assert!(matches!(
            ast.kind,
            ExprKind::Total {
                op: RingOp::Neg,
                rule: OverflowRule::Saturating,
                right: None,
                ..
            }
        ));
        // Any other name is still a call.
        let ast = parse_to_ast("wrappingDiv(now, last)").unwrap();
        assert!(matches!(ast.kind, ExprKind::Call { .. }));
    }

    #[test]
    fn a_total_operation_with_the_wrong_number_of_operands_is_refused_at_its_name() {
        let refusal = parse_to_ast("wrappingAdd(now)").unwrap_err();
        assert!(matches!(
            refusal.error,
            ExprError::ArgumentCount { ref callee, expected: 2, actual: 1 } if callee == "wrappingAdd"
        ));
        assert_eq!(refusal.span, Some(0..11));
        let refusal = parse_to_ast("saturatingNeg(now, last)").unwrap_err();
        assert!(matches!(
            refusal.error,
            ExprError::ArgumentCount {
                expected: 1,
                actual: 2,
                ..
            }
        ));
    }

    /// The operation's type is its operands' one type; a literal takes it.
    #[test]
    fn a_total_operation_is_of_its_operands_type() {
        let mut ctx_ast = parse_to_ast("wrappingSub(now, 1)").unwrap();
        infer_types(&mut ctx_ast, &total_ctx());
        assert_eq!(ctx_ast.ty, int(false, 32));
        let mut ast = parse_to_ast("saturatingAdd(7, delta)").unwrap();
        infer_types(&mut ast, &total_ctx());
        assert_eq!(ast.ty, int(true, 32));
    }

    #[test]
    fn rust_lowers_a_total_operation_to_the_standard_librarys() {
        assert_eq!(
            total("wrappingSub(now, last)", ExprTarget::Rust).unwrap(),
            "u32::wrapping_sub(now, last)"
        );
        assert_eq!(
            total(
                "saturatingSub(1000, wrappingSub(now, last))",
                ExprTarget::Rust
            )
            .unwrap(),
            "u32::saturating_sub(1000, u32::wrapping_sub(now, last))"
        );
        assert_eq!(
            total("wrappingNeg(small)", ExprTarget::Rust).unwrap(),
            "u8::wrapping_neg(small)"
        );
        assert_eq!(
            total("saturatingMul(delta, -3)", ExprTarget::Rust).unwrap(),
            "i32::saturating_mul(delta, -3)"
        );
    }

    /// Each backend calls the row of its runtime named beside the checked one
    /// (docs/adr/0063, section 8), at the operation's own width.
    #[test]
    fn every_backend_spells_a_total_operation_as_its_runtime_row() {
        let cases = [
            (
                ExprTarget::Cpp,
                "SCE::Forge::Wrapping::sub<std::uint32_t>(now, last)",
                "SCE::Forge::Saturating::neg<std::int32_t>(delta)",
            ),
            (
                ExprTarget::C,
                "sce_forge_wrapping_sub_u32(now, last)",
                "sce_forge_saturating_neg_i32(delta)",
            ),
            (
                ExprTarget::Go,
                "scealgorithm.WrappingSubUint32(now, last)",
                "scealgorithm.SaturatingNegInt32(delta)",
            ),
            (
                ExprTarget::Kotlin,
                "com.sce.forge.runtime.SceWrapping.sub(now, last)",
                "com.sce.forge.runtime.SceSaturating.neg(delta)",
            ),
            (
                ExprTarget::Python,
                "sce_algorithm.U32.wrapping_sub(now, last)",
                "sce_algorithm.I32.saturating_neg(delta)",
            ),
            (
                ExprTarget::Swift,
                "Wrapping.sub(now, last)",
                "Saturating.neg(delta)",
            ),
            (
                ExprTarget::Js,
                "SceStatic.U32.wrappingSub(now, last)",
                "SceStatic.I32.saturatingNeg(delta)",
            ),
            (
                ExprTarget::Rust,
                "u32::wrapping_sub(now, last)",
                "i32::saturating_neg(delta)",
            ),
        ];
        for (target, wrapping, saturating) in cases {
            assert_eq!(
                total("wrappingSub(now, last)", target).unwrap(),
                wrapping,
                "{target:?}"
            );
            assert_eq!(
                total("saturatingNeg(delta)", target).unwrap(),
                saturating,
                "{target:?}"
            );
        }
    }

    fn refused(expr: &str) -> ExprError {
        total(expr, ExprTarget::Rust).unwrap_err().error
    }

    #[test]
    fn a_total_operation_of_two_widths_is_refused_naming_both() {
        let ExprError::TypeMismatch {
            expected,
            got,
            observed,
        } = refused("wrappingAdd(now, small)")
        else {
            panic!("not a type mismatch");
        };
        assert_eq!(got, "uint8");
        assert!(
            expected.contains("uint32") && expected.contains("wrappingAdd"),
            "{expected}"
        );
        assert_eq!(observed.as_deref(), Some("small"));
    }

    #[test]
    fn a_total_operation_of_another_signedness_is_refused() {
        assert!(matches!(
            refused("saturatingSub(now, delta)"),
            ExprError::TypeMismatch { ref got, .. } if got == "int32"
        ));
    }

    #[test]
    fn a_total_operation_on_anything_but_integers_is_refused() {
        for (expr, got) in [
            ("wrappingAdd(ratio, 1)", "float64"),
            ("saturatingSub(ready, 1)", "bool"),
            ("wrappingMul(now, 1.5)", "float literal"),
        ] {
            assert!(
                matches!(refused(expr), ExprError::TypeMismatch { got: ref g, .. } if g == got),
                "{expr}"
            );
        }
    }

    /// The width is the modulus: an operation of literals alone has none.
    #[test]
    fn a_total_operation_of_literals_alone_is_refused() {
        for expr in [
            "wrappingAdd(200, 100)",
            "wrappingNeg(5)",
            "saturatingSub(1 + 2, 3)",
        ] {
            assert!(
                matches!(refused(expr), ExprError::TypeMismatch { got: ref g, .. } if g == "integer literal"),
                "{expr}"
            );
        }
    }

    #[test]
    fn a_literal_operand_must_fit_the_operations_type() {
        assert!(matches!(
            refused("wrappingSub(small, 300)"),
            ExprError::LiteralOutOfRange { ref literal, ref ty, .. } if literal == "300" && ty == "uint8"
        ));
        assert!(matches!(
            refused("wrappingAdd(now, -1)"),
            ExprError::LiteralOutOfRange { ref literal, .. } if literal == "-1"
        ));
    }

    #[test]
    fn the_saturating_negation_of_an_unsigned_value_is_refused() {
        assert!(matches!(
            refused("saturatingNeg(now)"),
            ExprError::TypeMismatch { ref expected, ref got, .. }
                if got == "uint32" && expected.contains("signed")
        ));
        assert!(total("saturatingNeg(delta)", ExprTarget::Rust).is_ok());
        assert!(total("wrappingNeg(now)", ExprTarget::Rust).is_ok());
    }

    /// A `may-fail` body checks the operators the author wrote and leaves a
    /// total operation total: only its operand is checked.
    #[test]
    fn a_may_fail_body_checks_the_operand_and_not_the_total_operation() {
        let mut ctx = total_ctx();
        ctx.receives_failures = true;
        let text = transpile_typed(
            "wrappingAdd(now + 1, last)",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(
            text,
            "u32::wrapping_add(sce_forge_runtime::algorithm::add::<u32>(now, 1)?, last)"
        );
    }

    // ── ECMAScript (the Interpreter's script engine) ────────────

    /// A `sce-static` machine's context: the failures of every integer
    /// operation are received, so each one is a library call.
    fn js_ctx() -> TypeCtx<'static> {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("level", int(false, 8));
        ctx.insert_var("delta", int(true, 32));
        ctx.insert_var("wide", int(false, 64));
        ctx.insert_var("ratio", float(64));
        ctx.insert_var("ready", InferredType::Bool);
        ctx.insert_var("label", InferredType::Str);
        ctx.receives_failures = true;
        ctx
    }

    fn js(expr: &str) -> String {
        transpile_typed(
            expr,
            ExprTarget::Js,
            &js_ctx(),
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap()
    }

    /// The one property the lowering exists for: an integer operation is the
    /// library's, at the width the operation is checked at, so a `uint8` that
    /// would pass 255 throws where a bare `+` would write 256.
    #[test]
    fn js_checks_an_integer_operation_at_its_own_width() {
        assert_eq!(js("level + 3"), "SceStatic.U8.add(level, 3)");
        assert_eq!(js("delta - 1"), "SceStatic.I32.sub(delta, 1)");
        assert_eq!(js("wide * 2"), "SceStatic.out(SceStatic.U64.mul(wide, 2))");
        assert_eq!(js("delta / 2"), "SceStatic.I32.div(delta, 2)");
        assert_eq!(js("delta % 2"), "SceStatic.I32.rem(delta, 2)");
        assert_eq!(js("-delta"), "SceStatic.I32.neg(delta)");
    }

    /// A width the library does not implement is refused at build time; the
    /// call it would otherwise write names a member that is not there and
    /// fails only when the machine reaches it.
    #[test]
    fn js_refuses_a_width_the_library_lacks() {
        let mut ctx = js_ctx();
        ctx.insert_var("odd", int(false, 24));
        let refusal = transpile_typed(
            "odd + 1",
            ExprTarget::Js,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap_err()
        .to_string();
        assert!(refusal.contains("24-bit"), "{refusal}");
    }

    /// A nested operation is checked at each node, innermost first, so the
    /// failure is the first overflow and not the last.
    #[test]
    fn js_checks_a_nested_operation_at_every_node() {
        assert_eq!(
            js("(level + 1) * 2"),
            "SceStatic.U8.mul(SceStatic.U8.add(level, 1), 2)"
        );
    }

    /// A comparison, a logical operator and a conditional are ECMAScript's own.
    #[test]
    fn js_writes_comparison_and_logic_as_ecmascript() {
        assert_eq!(js("level === 3 && ready"), "level === 3 && ready");
        assert_eq!(js("level !== 3 || !ready"), "level !== 3 || !ready");
        assert_eq!(js("ready ? level : 0"), "ready ? level : 0");
        assert_eq!(js("label === 'idle'"), "label === 'idle'");
        // The literal is its value, written again by the one ECMAScript
        // quoter, whichever quote the author chose.
        assert_eq!(js("label === \"idle\""), "label === 'idle'");
    }

    #[test]
    fn js_parenthesises_by_ecmascript_precedence() {
        assert_eq!(js("(ready || false) && ready"), "(ready || false) && ready");
        assert_eq!(js("ready || (false && ready)"), "ready || false && ready");
        assert_eq!(
            js("ready ? (ready ? 1 : 2) : 3"),
            "ready ? (ready ? 1 : 2) : 3"
        );
        assert_eq!(js("!(level === 1)"), "!(level === 1)");
        // A comparison under a comparison keeps its parentheses here too,
        // where they change nothing (`child_needs_parens`).
        assert_eq!(js("(level === 1) === ready"), "(level === 1) === ready");
    }

    /// A sign in front of a sign is one token, `--`, in ECMAScript.
    #[test]
    fn js_does_not_glue_two_signs_into_a_decrement() {
        let text = js("-(-ratio)");
        assert!(!text.contains("--"), "{text}");
    }

    /// An integer literal is its decimal value — `017` reads as octal in
    /// sloppy-mode ECMAScript — and one a Number cannot hold is a BigInt literal,
    /// which is how the lowering holds such an integer; past `u64`, the widest
    /// any backend declares, it is refused.
    #[test]
    fn js_writes_an_integer_literal_as_its_value_and_a_wide_one_as_a_bigint() {
        assert_eq!(js_number_literal("017").unwrap(), "17");
        assert_eq!(js_number_literal("0xFF").unwrap(), "255");
        assert_eq!(js_number_literal("0b101").unwrap(), "5");
        assert_eq!(js_number_literal("1.5").unwrap(), "1.5");
        assert_eq!(
            js_number_literal("9007199254740991").unwrap(),
            "9007199254740991"
        );
        assert_eq!(
            js_number_literal("9007199254740992").unwrap(),
            "9007199254740992n"
        );
        assert_eq!(
            js_number_literal("18446744073709551615").unwrap(),
            "18446744073709551615n"
        );
        let refusal = js_number_literal("0xFFFFFFFFFFFFFFFF0")
            .unwrap_err()
            .to_string();
        assert!(refusal.contains("u64"), "{refusal}");
    }

    /// `round` is half away from zero, which `Math.round` is not
    /// (`Math.round(-2.5)` is -2), so it is the library's; `floor` is
    /// `Math.floor`. Both give an integer, which has the one form the rest of
    /// the lowering holds one in, so a whole number past 2^53 comes back a
    /// BigInt (`fromReal`).
    #[test]
    fn js_rounds_half_away_from_zero_through_the_library() {
        assert_eq!(js("round(ratio)"), "SceStatic.round(ratio)");
        assert_eq!(js("floor(ratio)"), "SceStatic.fromReal(Math.floor(ratio))");
    }

    /// The length of a list or a string is `.length`.
    /// `_x.limits.<name>` reads the number the build gave the bound, as an integer
    /// literal every backend and the Interpreter write alike; in a closed scope a name
    /// the build does not name, and one it gives two numbers, are refused by name.
    #[test]
    fn a_build_limit_is_read_as_the_number_the_build_gave_it() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("n", int(false, 32));
        ctx.limits.insert("room", Ok(4096));
        ctx.limits.insert("twice", Err((8, 16)));
        for target in [
            ExprTarget::Rust,
            ExprTarget::Cpp,
            ExprTarget::Go,
            ExprTarget::Kotlin,
            ExprTarget::Python,
            ExprTarget::C,
        ] {
            let text = tp_with("n < _x.limits.room", target, &ctx);
            assert!(
                text.contains("4096") && !text.contains("_x") && !text.contains("limits"),
                "{target:?}: {text}"
            );
        }
        ctx.reject_unknown_identifiers = true;
        let unknown = transpile_typed(
            "n < _x.limits.rooms",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap_err()
        .to_string();
        assert!(
            unknown.contains("rooms") && unknown.contains("room") && unknown.contains("twice"),
            "the refusal names the read and what the build names: {unknown}"
        );
        let twice = transpile_typed(
            "n < _x.limits.twice",
            ExprTarget::Rust,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap_err()
        .to_string();
        assert!(twice.contains("8 and 16"), "{twice}");
    }

    /// `len` of a string is the UTF-8 bytes it holds on every backend, the unit a
    /// string's bound counts: Rust's `len`, C++'s `size` and Go's `len` count bytes
    /// already, and Kotlin (whose `String` has no `size`, and whose `length` counts
    /// UTF-16 units), Python (code points), C (a text) and the Interpreter (UTF-16
    /// units) measure its encoding. Measured 2026-10-21: Kotlin wrote `(title).size`,
    /// which does not compile on a `String`.
    #[test]
    fn the_length_of_a_string_is_its_utf8_bytes_on_every_backend() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("label", InferredType::Str);
        for (target, expected) in [
            (ExprTarget::Rust, "label.len()"),
            (ExprTarget::Cpp, "(label).size()"),
            (ExprTarget::Go, "len(label)"),
            (ExprTarget::Kotlin, "(label).encodeToByteArray().size"),
            (ExprTarget::Python, "len((label).encode(\"utf-8\"))"),
            (ExprTarget::C, "sce_text_length(label)"),
        ] {
            assert_eq!(tp_with("len(label)", target, &ctx), expected, "{target:?}");
        }
        assert_eq!(
            transpile_typed(
                "len(label)",
                ExprTarget::Js,
                &js_ctx(),
                &empty_renames(),
                InferredType::Unknown,
            )
            .unwrap(),
            "SceStatic.utf8Length(label)"
        );
        // A byte string keeps the member every backend measures it by.
        let mut bytes = TypeCtx::new();
        bytes.insert_var("frame", InferredType::Bytes);
        assert_eq!(
            tp_with("len(frame)", ExprTarget::Kotlin, &bytes),
            "(frame).size"
        );
        assert_eq!(tp_with("len(frame)", ExprTarget::C, &bytes), "(frame).len");
    }

    #[test]
    fn js_reads_a_length_as_the_length_property() {
        let mut ctx = js_ctx();
        ctx.insert_var(
            "history",
            InferredType::List(
                crate::forge::types::ListElem::of(&crate::forge::model::SceType::Uint8)
                    .expect("a list holds a uint8"),
            ),
        );
        let text = transpile_typed(
            "len(history)",
            ExprTarget::Js,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(text, "history.length");
    }

    /// An operator whose ECMAScript meaning is not the document's is not written
    /// as it would be for a Number — a Number's own are 32-bit and signed — but
    /// as the library's, at the width of the operation, which wraps there.
    #[test]
    fn js_writes_the_bitwise_operators_as_the_librarys() {
        for (expr, written) in [
            ("level & 3", "SceStatic.U8.and(level, 3)"),
            ("level | 3", "SceStatic.U8.or(level, 3)"),
            ("level ^ 3", "SceStatic.U8.xor(level, 3)"),
            ("level << 1", "SceStatic.U8.shl(level, 1)"),
            ("level >> 1", "SceStatic.U8.shr(level, 1)"),
            ("level >>> 1", "SceStatic.U8.ushr(level, 1)"),
            ("~level", "SceStatic.U8.not(level)"),
            ("delta & 255", "SceStatic.I32.and(delta, 255)"),
        ] {
            assert_eq!(js(expr), written, "{expr}");
        }
    }

    /// A 64-bit integer that leaves the expressions for the Interpreter's data
    /// goes through `out`, which refuses what the data model would lose; a
    /// narrower one is always a Number and does not.
    #[test]
    fn js_hands_a_64_bit_integer_to_the_machine_through_out() {
        assert_eq!(js("wide * 2"), "SceStatic.out(SceStatic.U64.mul(wide, 2))");
        assert_eq!(js("level + 3"), "SceStatic.U8.add(level, 3)");
    }

    /// Two integers ordered where one may be a BigInt are ordered as BigInts
    /// (the engine orders a BigInt against a Number wrongly for two negative
    /// values), and one that meets a real is converted to it.
    #[test]
    fn js_orders_a_possible_bigint_as_a_bigint_and_converts_it_for_a_real() {
        assert_eq!(js("wide < delta"), "BigInt(wide) < BigInt(delta)");
        assert_eq!(js("level < 3"), "level < 3");
        assert_eq!(js("wide === 3"), "wide === 3");
        assert_eq!(js("wide * ratio"), "Number(wide) * ratio");
    }

    /// An enum variant reference has no ECMAScript declaration to be spelled
    /// against: the Interpreter holds the variant's declared name, a string,
    /// and not the author's `Alias.variant`, which names nothing there.
    #[test]
    fn js_lowers_an_enum_variant_reference_to_its_declared_name() {
        let variants = ["Idle".to_string(), "Run".to_string()];
        let mut ctx = TypeCtx::new();
        ctx.insert_enum(
            "Mode",
            crate::forge::types::EnumScope {
                variants: &variants,
                qualified_type: "Mode",
                source_name: "Mode",
                carrier: InferredType::Int {
                    signed: false,
                    bits: 8,
                },
                to_number: "Mode.toNumber",
                from_number: "Mode.fromNumber",
                open: false,
            },
        );
        let lowered = transpile_typed(
            "Mode.Idle",
            ExprTarget::Js,
            &ctx,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect("an enum variant lowers for the Interpreter");
        assert_eq!(lowered, "'Idle'");
    }

    /// A context holding two optional `int64`s, `due` and `other`, a `uint32`
    /// `count`, and `due` proven present.
    fn optional_ctx() -> TypeCtx<'static> {
        let int64 = OptionalElem::Number(crate::forge::quantity::NumericBaseType::Int {
            signed: true,
            bits: 64,
        });
        let mut ctx = TypeCtx::new();
        ctx.insert_var("due", InferredType::Optional(int64));
        ctx.insert_var("other", InferredType::Optional(int64));
        ctx.insert_var("count", int(false, 32));
        ctx.proving(crate::forge::narrowing::proves_written(
            "due !== null",
            true,
        ))
    }

    /// Each construct of an optional value is the one form every emitter spells
    /// (docs/adr/0055, section 2), whichever side of the operator `null` is on;
    /// a test of `null` beside a value that is not optional is none.
    #[test]
    fn each_construct_of_an_optional_value_is_one_form() {
        let ctx = optional_ctx();
        let form = |expr: &str| {
            let ast = resolve(expr, &ctx).unwrap_or_else(|e| panic!("{expr} resolves: {e:?}"));
            optional_form(&ast).map(|form| form.describe())
        };
        let present = Some("a test that a value is present (`!== null`)".to_string());
        let absent = Some("a test that a value is absent (`=== null`)".to_string());
        assert_eq!(form("due !== null"), present);
        assert_eq!(form("null !== due"), present);
        assert_eq!(form("due === null"), absent);
        assert_eq!(form("null === due"), absent);
        assert_eq!(
            form("due.value"),
            Some("a read of a value proven present (`.value`)".to_string())
        );
        assert_eq!(
            form("due ?? 0"),
            Some("a read with a default (`??`)".to_string())
        );
        assert_eq!(
            form("due === other"),
            Some("`===` of an optional value".to_string())
        );
        assert_eq!(
            form("due !== 3"),
            Some("`!==` of an optional value".to_string())
        );
        assert_eq!(form("count === 3"), None);
        assert_eq!(form("count + 1"), None);
    }

    /// A target whose emitter spells no optional value yet refuses each
    /// construct by name before the emitter is reached, placed at it.
    #[test]
    fn a_target_that_lowers_no_optional_value_names_the_construct_it_refuses() {
        let ctx = optional_ctx();
        for target in ExprTarget::ALL
            .into_iter()
            .chain([ExprTarget::Js])
            .filter(|t| !t.lowers_optional())
        {
            let refusal = transpile_typed(
                "(due ?? 0) === 5",
                target,
                &ctx,
                &empty_renames(),
                InferredType::Unknown,
            )
            .expect_err("an optional value is refused where it is not lowered");
            let text = refusal.error.to_string();
            assert!(
                text.contains("a read with a default (`??`)")
                    && text.contains("an optional value has no")
                    && text.contains("lowering yet"),
                "{target:?}: {text}"
            );
            assert_eq!(refusal.span, Some(1..9), "{target:?}: placed at `due ?? 0`");
        }
    }

    /// Go spells each construct of an optional value over `optional.Of[T]`
    /// (docs/adr/0055, section 5): a test, a proven read, a read with a default
    /// whose default is a function literal called only when the value is absent,
    /// total equality with a value of `T` lifted, `null` and a lifted value where
    /// an optional value is expected, and a conditional of the two.
    #[test]
    fn go_spells_each_construct_of_an_optional_value() {
        let ctx = optional_ctx();
        let int64 = OptionalElem::Number(crate::forge::quantity::NumericBaseType::Int {
            signed: true,
            bits: 64,
        });
        let go = |expr: &str, expected: InferredType| {
            transpile_into(expr, ExprTarget::Go, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers for Go: {e:?}"))
        };
        let optional = InferredType::Optional(int64);
        assert_eq!(go("due !== null", InferredType::Bool), "due.IsPresent()");
        assert_eq!(go("null === due", InferredType::Bool), "!due.IsPresent()");
        assert_eq!(go("due.value", int(true, 64)), "due.Proven()");
        assert_eq!(
            go("due ?? 0", int(true, 64)),
            "optional.Otherwise(due, func() int64 { return 0 })"
        );
        assert_eq!(go("due === other", InferredType::Bool), "due.Equal(other)");
        assert_eq!(
            go("due !== 3", InferredType::Bool),
            "!due.Equal(optional.Some[int64](3))"
        );
        assert_eq!(go("null === null", InferredType::Bool), "true");
        assert_eq!(go("null", optional), "optional.None[int64]()");
        assert_eq!(go("3", optional), "optional.Some[int64](3)");
        assert_eq!(go("other", optional), "other");
        assert_eq!(
            go("count === 1 ? null : 5", optional),
            "func() optional.Of[int64] { if count == 1 { return optional.None[int64]() }; \
             return optional.Some[int64](5) }()"
        );
    }

    /// Rust spells each construct of an optional value over `Option<T>`
    /// (docs/adr/0055, section 5): a test, a proven read through the runtime, a
    /// read with a default whose default is evaluated in the absent arm alone,
    /// total equality with a value of `T` lifted, `null` and a lifted value where
    /// an optional value is expected, and a conditional of the two. A text is
    /// read borrowed, and made owned where a place holds it.
    #[test]
    fn rust_spells_each_construct_of_an_optional_value() {
        let mut ctx = optional_ctx();
        ctx.insert_var("title", InferredType::Optional(OptionalElem::Str));
        let int64 = OptionalElem::Number(crate::forge::quantity::NumericBaseType::Int {
            signed: true,
            bits: 64,
        });
        let optional = InferredType::Optional(int64);
        let text = InferredType::Optional(OptionalElem::Str);
        let rust = |expr: &str, expected: InferredType| {
            transpile_into(expr, ExprTarget::Rust, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers for Rust: {e:?}"))
        };
        assert_eq!(rust("due !== null", InferredType::Bool), "due.is_some()");
        assert_eq!(rust("null === due", InferredType::Bool), "due.is_none()");
        assert_eq!(
            rust("due.value", int(true, 64)),
            "sce_forge_runtime::optional::proven(due)"
        );
        assert_eq!(
            rust("due ?? 0", int(true, 64)),
            "match due { Some(sce_present) => sce_present, None => 0 }"
        );
        assert_eq!(rust("due === other", InferredType::Bool), "due == other");
        assert_eq!(rust("due !== 3", InferredType::Bool), "due != Some(3)");
        assert_eq!(rust("null === null", InferredType::Bool), "true");
        assert_eq!(rust("null", optional), "None");
        assert_eq!(rust("3", optional), "Some(3)");
        assert_eq!(rust("other", optional), "other");
        assert_eq!(
            rust("count === 1 ? null : 5", optional),
            "if count == 1 { None } else { Some(5) }"
        );
        assert_eq!(
            rust("title !== null", InferredType::Bool),
            "title.as_deref().is_some()"
        );
        assert_eq!(
            rust("title === 'soon'", InferredType::Bool),
            "title.as_deref() == Some(\"soon\")"
        );
        assert_eq!(
            rust("title ?? 'none'", InferredType::Str),
            "match title.as_deref() { Some(sce_present) => sce_present, None => \"none\" }"
        );
        let owned = |expr: &str, expected: InferredType| {
            transpile_into_owned(expr, ExprTarget::Rust, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers owned for Rust: {e:?}"))
                .text
        };
        assert_eq!(owned("title", text), "title.as_deref().map(str::to_string)");
        assert_eq!(owned("due", optional), "due");
        // The absent value is owned as it is, and not mapped over.
        assert_eq!(owned("null", text), "None");
    }

    /// Rust reads the present record of an optional record through a borrow of the
    /// optional where the build proved it present, so the record that owns text is
    /// not moved out of the place that holds it; takes a record a place holds whole
    /// as a copy; and returns an optional text as the owned value its signature
    /// declares, an expression reading one borrowed (docs/adr/0055, step 9).
    #[test]
    fn rust_reads_an_optional_record_through_a_borrow_and_owns_what_it_copies_and_returns() {
        let mut ctx = TypeCtx::new();
        let record = InferredType::Optional(OptionalElem::Record);
        let text = InferredType::Optional(OptionalElem::Str);
        ctx.insert_var("pick", record);
        ctx.insert_var("other", record);
        ctx.insert_var("title", text);
        ctx.insert_record("pick.value", RecordShape::Closed);
        ctx.insert_var("pick.value.count", int(true, 32));
        let ctx = ctx.proving(crate::forge::narrowing::proves_written(
            "pick !== null",
            true,
        ));
        let rust = |expr: &str, expected: InferredType| {
            transpile_into(expr, ExprTarget::Rust, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers for Rust: {e:?}"))
        };
        assert_eq!(rust("pick !== null", InferredType::Bool), "pick.is_some()");
        assert_eq!(rust("null === pick", InferredType::Bool), "pick.is_none()");
        assert_eq!(
            rust("pick.value.count", int(true, 32)),
            "sce_forge_runtime::optional::proven_ref(&pick).count"
        );
        // An optional text is read borrowed, and a return owns it, as a present one
        // does ([`transpile_into_owned`]).
        assert_eq!(rust("title", text), "title.as_deref()");
        assert_eq!(rust("null", text), "None");
        let owned = |expr: &str, expected: InferredType| {
            transpile_into_owned(expr, ExprTarget::Rust, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers owned for Rust: {e:?}"))
                .text
        };
        assert_eq!(owned("title", text), "title.as_deref().map(str::to_string)");
        // A place that holds an optional record keeps its own: another takes a copy.
        assert_eq!(owned("other", record), "other.clone()");
        assert_eq!(owned("null", record), "None");
    }

    /// C11 spells each construct of an optional value over `sce_optional_<elem>_t`
    /// of `sce/forge/optional.h` (docs/adr/0055, section 5): a test of the flag, a
    /// proven read of `.value`, a read with a default through the runtime's
    /// conditional over the flag, total equality with no type named, and `null`
    /// and a lifted value by the element's constructors where an optional value is
    /// expected.
    #[test]
    fn c_spells_each_construct_of_an_optional_value() {
        let mut ctx = optional_ctx();
        ctx.insert_var("title", InferredType::Optional(OptionalElem::Str));
        let int64 = OptionalElem::Number(crate::forge::quantity::NumericBaseType::Int {
            signed: true,
            bits: 64,
        });
        let c = |expr: &str, expected: InferredType| {
            transpile_into(expr, ExprTarget::C, &ctx, &empty_renames(), expected)
                .unwrap_or_else(|e| panic!("{expr} lowers for C11: {e:?}"))
        };
        let optional = InferredType::Optional(int64);
        assert_eq!(c("due !== null", InferredType::Bool), "due.present");
        assert_eq!(c("null === due", InferredType::Bool), "!due.present");
        assert_eq!(c("due.value", int(true, 64)), "due.value");
        assert_eq!(
            c("due ?? 0", int(true, 64)),
            "SCE_OPTIONAL_VALUE_OR(due, 0)"
        );
        assert_eq!(
            c("title ?? 'none'", InferredType::Str),
            "SCE_OPTIONAL_TEXT_OR(title, \"none\")"
        );
        assert_eq!(
            c("due === other", InferredType::Bool),
            "SCE_OPTIONAL_SAME(due, other, SCE_OPTIONAL_SAME_VALUE)"
        );
        assert_eq!(
            c("due !== 3", InferredType::Bool),
            "!SCE_OPTIONAL_HOLDS(due, 3)"
        );
        assert_eq!(
            c("title === 'soon'", InferredType::Bool),
            "SCE_OPTIONAL_HOLDS_TEXT(title, \"soon\")"
        );
        assert_eq!(c("null === null", InferredType::Bool), "true");
        assert_eq!(c("null", optional), "sce_optional_int64_none()");
        assert_eq!(c("3", optional), "sce_optional_int64_some(3)");
        assert_eq!(c("other", optional), "other");
        let conditional = c("count === 1 ? null : 5", optional);
        assert!(
            conditional.contains("? sce_optional_int64_none() : sce_optional_int64_some(5)"),
            "{conditional}"
        );
    }

    /// An optional value whose C type carries a bound or a document — a text, a
    /// byte string, an enum — cannot be built inside an expression, which names
    /// neither: `null` of one is refused by name there, and written whole where
    /// a variable or a field takes it.
    #[test]
    fn c_refuses_an_optional_value_it_cannot_name_inside_an_expression() {
        let ctx = optional_ctx();
        for elem in [OptionalElem::Str, OptionalElem::Bytes, OptionalElem::Enum] {
            let refusal = transpile_into(
                "null",
                ExprTarget::C,
                &ctx,
                &empty_renames(),
                InferredType::Optional(elem),
            )
            .expect_err("a `null` of this element has no type here");
            assert!(
                refusal
                    .error
                    .to_string()
                    .contains("whose C type carries a bound or a document"),
                "{elem:?}: {}",
                refusal.error
            );
        }
    }

    /// Go names the type of a `null` and of a read with a default; where the
    /// lattice does not name it — an optional value of an enum inside an
    /// expression — the expression is refused by name rather than spelled with
    /// no type.
    #[test]
    fn go_refuses_an_optional_enum_it_cannot_name_inside_an_expression() {
        let variants = ["Idle".to_string(), "Run".to_string()];
        let mut ctx = conversion_ctx(&variants, false);
        ctx.insert_var("mode", InferredType::Optional(OptionalElem::Enum));
        let refusal = transpile_into(
            "null",
            ExprTarget::Go,
            &ctx,
            &empty_renames(),
            InferredType::Optional(OptionalElem::Enum),
        )
        .expect_err("a `null` of an enum has no type here");
        assert!(
            refusal
                .error
                .to_string()
                .contains("whose type Go cannot name here"),
            "{}",
            refusal.error
        );
    }

    /// A conditional of `null` and a variant of an enum is an optional value of
    /// the enum (docs/adr/0055, section 2), whichever branch is `null`, as one of
    /// `null` and a number is an optional number; a conditional of a variant and
    /// a value the typer cannot tell is no optional value.
    #[test]
    fn null_beside_a_variant_is_an_optional_value_of_the_enum() {
        let variants = ["Idle".to_string(), "Run".to_string()];
        let mut ctx = conversion_ctx(&variants, false);
        ctx.insert_var("flag", InferredType::Bool);
        let typed = |expr: &str| {
            infer_expr_type(expr, &ctx).unwrap_or_else(|e| panic!("{expr} types: {e:?}"))
        };
        let optional_enum = InferredType::Optional(OptionalElem::Enum);
        assert_eq!(typed("flag ? null : Mode.Idle"), optional_enum);
        assert_eq!(typed("flag ? Mode.Run : null"), optional_enum);
        assert_eq!(
            typed("flag ? null : (flag ? Mode.Idle : Mode.Run)"),
            optional_enum
        );
        assert_eq!(
            typed("flag ? null : n"),
            InferredType::Optional(OptionalElem::Number(
                crate::forge::quantity::NumericBaseType::Int {
                    signed: false,
                    bits: 8
                }
            ))
        );
        assert_eq!(typed("flag ? m : null"), InferredType::Unknown);
    }

    /// A context whose enum `Mode` is carried as a `uint8` and spells its two
    /// conversions `mode_to` and `mode_from` (docs/adr/0047), `open` saying whether
    /// every number is a value of it, with a value `m` of it and a `uint8` `n`.
    fn conversion_ctx<'a>(variants: &'a [String], open: bool) -> TypeCtx<'a> {
        let mut ctx = TypeCtx::new();
        ctx.insert_enum(
            "Mode",
            crate::forge::types::EnumScope {
                variants,
                qualified_type: "Mode",
                source_name: "Mode",
                carrier: int(false, 8),
                to_number: "mode_to",
                from_number: "mode_from",
                open,
            },
        );
        ctx.insert_var("m", InferredType::Unknown);
        ctx.insert_var("n", int(false, 8));
        ctx
    }

    /// `<alias>.toNumber` is the function the context spells it with, and
    /// `<alias>.fromNumber` of a closed enum the backend's wrapper of the function,
    /// which turns a number no variant declares into the failure every failing call
    /// hands its statement.
    #[test]
    fn an_enum_conversion_lowers_to_the_functions_its_context_spells() {
        let variants = ["Idle".to_string(), "Run".to_string()];
        let mut ctx = conversion_ctx(&variants, false);
        ctx.receives_failures = true;
        let lowered = |expr: &str, target: ExprTarget| {
            transpile_typed(expr, target, &ctx, &empty_renames(), InferredType::Unknown)
                .unwrap_or_else(|e| panic!("{expr} lowers for {target:?}: {e:?}"))
        };
        assert_eq!(lowered("Mode.toNumber(m)", ExprTarget::Rust), "mode_to(m)");
        for (target, fragment) in [
            (
                ExprTarget::Rust,
                "sce_forge_runtime::algorithm::found(mode_from(n))?",
            ),
            (
                ExprTarget::Kotlin,
                "com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(mode_from(",
            ),
            (
                ExprTarget::Cpp,
                "SCE::Forge::Checked::take(sce_failure_, SCE::Forge::Checked::found(mode_from(",
            ),
            (
                ExprTarget::Go,
                "scealgorithm.TakeOutcome(&sceFailure, scealgorithm.Found(mode_from(",
            ),
            (ExprTarget::Python, "sce_algorithm.found(mode_from("),
            (ExprTarget::C, "mode_from_take(&sce_failure_, mode_from("),
        ] {
            let out = lowered("Mode.fromNumber(n)", target);
            assert!(out.contains(fragment), "{target:?}: {out}");
        }
    }

    /// A number no variant of a closed enum declares is a failure, received only
    /// where failures are; an open enum holds every number of its carrier, so its
    /// reading cannot fail and is the function itself.
    #[test]
    fn from_number_may_fail_only_for_a_closed_enum() {
        let variants = ["Idle".to_string(), "Run".to_string()];
        let closed = conversion_ctx(&variants, false);
        let refusal = transpile_typed(
            "Mode.fromNumber(n)",
            ExprTarget::Rust,
            &closed,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect_err("a reading that may fail is refused where no failure is received");
        assert!(
            format!("{:?}", refusal.error).contains("may-fail"),
            "{refusal:?}"
        );
        let open = conversion_ctx(&variants, true);
        let lowered = transpile_typed(
            "Mode.fromNumber(n)",
            ExprTarget::Rust,
            &open,
            &empty_renames(),
            InferredType::Unknown,
        )
        .expect("an open enum's reading cannot fail");
        assert_eq!(lowered, "mode_from(n)");
    }

    /// A context whose `Check` is a validator's check and `Find` a lookup whose miss
    /// is an error (docs/adr/0048), each of an `int32`, with an `int32` `n`, renamed
    /// to the functions a backend calls.
    fn reading_ctx() -> TypeCtx<'static> {
        use crate::forge::types::FuncSig;
        let mut ctx = TypeCtx::new();
        ctx.insert_var("n", int(true, 32));
        ctx.insert_func(
            "Check",
            FuncSig {
                reading: CallReading::Validity,
                ..FuncSig::new(vec![int(true, 32)], InferredType::Bool)
            },
        );
        ctx.insert_func(
            "Find",
            FuncSig {
                may_fail: true,
                reading: CallReading::Found,
                ..FuncSig::new(vec![int(true, 32)], int(true, 32))
            },
        );
        ctx.receives_failures = true;
        ctx
    }

    /// A validator's check is read as the validity of what it answers, and a lookup
    /// whose miss is an error through the runtime's `found`, which turns a miss into
    /// the failure the statement receives; C's reading is the one the machine
    /// declares, which answers the result its `_take` reads.
    #[test]
    fn a_call_is_read_as_its_signature_says() {
        let ctx = reading_ctx();
        let lowered = |expr: &str, target: ExprTarget, renames: &[(&'static str, &'static str)]| {
            let renames: HashMap<&str, &str> = renames.iter().copied().collect();
            transpile_typed(expr, target, &ctx, &renames, InferredType::Unknown)
                .unwrap_or_else(|e| panic!("{expr} lowers for {target:?}: {e:?}"))
        };
        for (target, check, valid) in [
            (ExprTarget::Rust, "v::V::new().validate", ".valid"),
            (ExprTarget::Kotlin, "V().validate", ".valid"),
            (ExprTarget::Cpp, "v::V{}.validate", ".valid"),
            (ExprTarget::Go, "v.NewV().Validate", ".Valid"),
            (ExprTarget::Python, "v.V().validate", ".valid"),
            (ExprTarget::C, "v_validate", ".valid"),
        ] {
            let out = lowered("Check(n)", target, &[("Check", check)]);
            assert!(
                out.contains(&format!("{check}(n){valid}")),
                "{target:?}: {out}"
            );
        }
        for (target, fragment) in [
            (ExprTarget::Rust, "sce_forge_runtime::algorithm::found(t::find(n))?"),
            (
                ExprTarget::Kotlin,
                "com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(t_find(",
            ),
            (
                ExprTarget::Cpp,
                "SCE::Forge::Checked::take(sce_failure_, SCE::Forge::Checked::found(t_find(",
            ),
            (
                ExprTarget::Go,
                "scealgorithm.TakeOutcome(&sceFailure, scealgorithm.Found(t_find(",
            ),
            (ExprTarget::Python, "sce_algorithm.found(t_find("),
        ] {
            let callee = if target == ExprTarget::Rust {
                "t::find"
            } else {
                "t_find"
            };
            let out = lowered("Find(n)", target, &[("Find", callee)]);
            assert!(out.contains(fragment), "{target:?}: {out}");
        }
        let c = lowered("Find(n)", ExprTarget::C, &[("Find", "t_find_found")]);
        assert!(
            c.contains("t_find_found_take(&sce_failure_, t_find_found("),
            "C reads the lookup through the reading the machine declares: {c}"
        );
    }

    /// A call whose signature says nothing of how it is read is the function's own
    /// answer.
    #[test]
    fn a_call_of_a_value_is_left_as_it_is() {
        use crate::forge::types::FuncSig;
        let mut ctx = TypeCtx::new();
        ctx.insert_var("n", int(true, 32));
        ctx.insert_func("Twice", FuncSig::new(vec![int(true, 32)], int(true, 32)));
        let renames: HashMap<&str, &str> = [("Twice", "t::twice")].into_iter().collect();
        let out = transpile_typed(
            "Twice(n)",
            ExprTarget::Rust,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .expect("a call of a value lowers");
        assert_eq!(out, "t::twice(n)");
    }

    /// A field of the event's data reaches the Interpreter as untyped JSON, so
    /// its read is the library's, at the type the event's schema declares —
    /// and an arithmetic operation over it is checked at that type.
    #[test]
    fn js_reads_a_payload_field_through_the_library() {
        let mut ctx = js_ctx();
        ctx.insert_var("_event.data.dayOfMonth", int(false, 8));
        ctx.insert_var("_event.data.late", InferredType::Bool);
        ctx.insert_var("_event.data.title", InferredType::Str);
        let renames: HashMap<&str, &str> = HashMap::from([("_event.data", "_event.data")]);
        let lower = |expr: &str| {
            transpile_typed(expr, ExprTarget::Js, &ctx, &renames, InferredType::Unknown).unwrap()
        };
        assert_eq!(
            lower("_event.data.dayOfMonth"),
            "SceStatic.field(_event.data, 'dayOfMonth', 'uint8')"
        );
        assert_eq!(
            lower("_event.data.late"),
            "SceStatic.field(_event.data, 'late', 'bool')"
        );
        assert_eq!(
            lower("_event.data.title"),
            "SceStatic.field(_event.data, 'title', 'string')"
        );
        assert_eq!(
            lower("_event.data.dayOfMonth + 1"),
            "SceStatic.U8.add(SceStatic.field(_event.data, 'dayOfMonth', 'uint8'), 1)"
        );
    }

    /// A rename may name a whole path, not only an identifier and one property:
    /// the payload's enum field is read through a call that holds it to the
    /// enum's variants, and that call replaces the three names `_event.data.layout`,
    /// the longer path winning over the `_event.data` it begins with.
    #[test]
    fn a_rename_may_name_a_path_of_three_names() {
        let mut ctx = js_ctx();
        ctx.insert_var("_event.data.layout", InferredType::Unknown);
        ctx.insert_var("_event.data.zoom", int(false, 8));
        let renames: HashMap<&str, &str> = HashMap::from([
            ("_event.data", "_event.data"),
            (
                "_event.data.layout",
                "SceStatic.field(_event.data, 'layout', ['month', 'week'])",
            ),
        ]);
        let lower = |expr: &str| {
            transpile_typed(expr, ExprTarget::Js, &ctx, &renames, InferredType::Unknown).unwrap()
        };
        assert_eq!(
            lower("_event.data.layout"),
            "SceStatic.field(_event.data, 'layout', ['month', 'week'])"
        );
        // A sibling the rename does not name is read as it was.
        assert_eq!(
            lower("_event.data.zoom"),
            "SceStatic.field(_event.data, 'zoom', 'uint8')"
        );
    }

    /// A payload field of a type the library has no reader for is refused at
    /// build time, naming the field, and not read as an untyped value.
    #[test]
    fn js_refuses_a_payload_field_it_cannot_read() {
        let mut ctx = js_ctx();
        ctx.insert_var("_event.data.raw", InferredType::UntypedInt);
        let renames: HashMap<&str, &str> = HashMap::from([("_event.data", "_event.data")]);
        let refusal = transpile_typed(
            "_event.data.raw",
            ExprTarget::Js,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .unwrap_err()
        .to_string();
        assert!(refusal.contains("`raw`"), "{refusal}");
    }

    /// A payload's byte-string field is the text of its bytes, one character to a
    /// byte, and is read through the library as every other field is, which refuses
    /// a value that is not that text.
    #[test]
    fn js_reads_a_payload_byte_string_through_the_library() {
        let mut ctx = js_ctx();
        ctx.insert_var("_event.data.raw", InferredType::Bytes);
        let renames: HashMap<&str, &str> = HashMap::from([("_event.data", "_event.data")]);
        let text = transpile_typed(
            "_event.data.raw",
            ExprTarget::Js,
            &ctx,
            &renames,
            InferredType::Unknown,
        )
        .unwrap();
        assert_eq!(text, "SceStatic.field(_event.data, 'raw', 'bytes')");
    }

    /// The context of the optional values the Interpreter's emitter is held to:
    /// two optional `int64`s, an optional text, a plain `int64`, and the
    /// payload's optional `due`.
    fn js_optional_ctx() -> TypeCtx<'static> {
        let mut ctx = js_ctx();
        ctx.insert_var("due", optional_int64());
        ctx.insert_var("other", optional_int64());
        ctx.insert_var(
            "title",
            InferredType::Optional(crate::forge::types::OptionalElem::Str),
        );
        ctx.insert_var("huge", int(true, 64));
        ctx.insert_var("_event.data.due", optional_int64());
        ctx
    }

    /// The read of the payload's optional `due` the walk renames its path to
    /// (`JsTarget::payload_optional_read`).
    const JS_OPTIONAL_DUE_READ: &str = "SceStatic.field(_event.data, 'due', {optional: 'int64'})";

    /// `expr` lowered for the Interpreter in [`js_optional_ctx`], with the
    /// payload's `due` renamed to its read as the walk renames it.
    fn js_optional(expr: &str) -> String {
        let renames: HashMap<&str, &str> = HashMap::from([
            ("_event.data", "_event.data"),
            ("_event.data.due", JS_OPTIONAL_DUE_READ),
        ]);
        transpile_typed(
            expr,
            ExprTarget::Js,
            &js_optional_ctx(),
            &renames,
            InferredType::Unknown,
        )
        .unwrap_or_else(|e| panic!("{expr} lowers for the Interpreter: {e:?}"))
    }

    /// Absent is `null` and a present value is the value itself (docs/adr/0055),
    /// so a test is ECMAScript's own `===` against `null`, equality between two
    /// optional values is `===` (total: two `null`s are equal, and `null` equals
    /// no value), and a read where a test proves the value present is the
    /// optional value as it stands.
    #[test]
    fn js_writes_an_optional_value_as_null_or_its_value() {
        assert_eq!(js_optional("due === null"), "due === null");
        assert_eq!(js_optional("null !== due"), "null !== due");
        assert_eq!(js_optional("due === other"), "due === other");
        // A 64-bit integer leaves the expressions through `out`.
        assert_eq!(
            js_optional("due !== null ? due.value : 0"),
            "SceStatic.out(due !== null ? due : 0)"
        );
        assert_eq!(
            js_optional("due !== null ? due.value + 1 : 0"),
            "SceStatic.out(due !== null ? SceStatic.I64.add(due, 1) : 0)"
        );
        assert_eq!(
            js_optional("due !== null && due.value > 10"),
            "due !== null && BigInt(due) > BigInt(10)"
        );
        assert_eq!(
            js_optional("title === null || title.value === 'soon'"),
            "title === null || title === 'soon'"
        );
    }

    /// `x ?? d` is the library's `otherwise`, handed `d` as a function: `x` is
    /// evaluated once and `d` only when `x` is absent, so a default that would
    /// overflow fails only then.
    #[test]
    fn js_reads_an_optional_value_with_a_default_through_the_library() {
        assert_eq!(
            js_optional("title ?? 'none'"),
            "SceStatic.otherwise(title, function () { return 'none'; })"
        );
        assert_eq!(
            js_optional("due ?? (huge * huge)"),
            "SceStatic.out(SceStatic.otherwise(due, function () { return \
             SceStatic.I64.mul(huge, huge); }))"
        );
    }

    /// A read of the payload's optional field where a test proves it present is
    /// the read itself, and an optional 64-bit integer may be a BigInt when
    /// present, so it leaves the expressions through `out` as a plain one does;
    /// an optional text never is.
    #[test]
    fn js_hands_an_optional_64_bit_integer_to_the_machine_through_out() {
        let read = JS_OPTIONAL_DUE_READ;
        assert_eq!(
            js_optional("_event.data.due"),
            format!("SceStatic.out({read})")
        );
        assert_eq!(
            js_optional("_event.data.due !== null ? _event.data.due.value : 0"),
            format!("SceStatic.out({read} !== null ? {read} : 0)")
        );
        assert_eq!(js_optional("title"), "title");
    }

    /// The same document lowers on the six backends and on ECMAScript from one
    /// walk, and a difference between them is the emitters' alone: an
    /// operation the backends check, ECMAScript checks through the library.
    #[test]
    fn js_is_the_only_target_outside_the_all_list() {
        let text = js("level + 3");
        assert!(text.starts_with("SceStatic."), "{text}");
        assert!(!ExprTarget::ALL.contains(&ExprTarget::Js));
    }
}
