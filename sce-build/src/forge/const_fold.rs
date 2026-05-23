// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// RFC §5.F build-time const-fold — single-source host interpreter.
//
// Purpose: evaluate `<sce:fold>` bodies on the host at code-generation
// time, producing a `Vec<ConstValue>` that the §5.J.5 emitters
// serialize into per-language array literals. The interpreter is
// pure (no I/O, no allocation beyond the output vector) and
// budget-bounded (default 1_000_000 iterations across all folds in
// one document; CLI overridable via `--const-fold-budget=N`).
//
// Single-source means: one Rust evaluator drives the table values for
// every backend. Cross-language byte-equivalence on the underlying
// numeric data is therefore guaranteed by construction — only the
// per-language array-literal *syntax* differs.
//
// Statement vocabulary (RFC §5.A): Var / Assign / If / While /
// Foreach are supported inside fold bodies; Return / Call are
// rejected (a fold body produces an array element via `<sce:yield>`,
// not via early return, and cannot invoke other algorithms).
//
// Numeric model: every arithmetic step computes in widened domains
// (`i128` for integers, `f64` for floats). Storage into a typed slot
// (Var.init, Assign target, yield element) coerces back to the
// declared `SceType` with two-complement truncation for narrow
// integers — matching the wrapping semantics every target backend
// uses for fixed-width integer arithmetic. This keeps CRC-class
// algorithms (whose canonical implementation relies on truncating
// shifts) byte-equivalent to a hand-coded reference.

use std::collections::HashMap;

use crate::forge::error::{ExprError, GenerateError};
use crate::forge::expr::{self, BinOp, ExprKind, TypedExpr, UnaryOp};
use crate::forge::model::{AlgorithmStmt, FoldBody, SceType};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Internal error kind — boundary-lifted to typed `GenerateError`
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Context-free shape for the three RFC §5.F wire codes. Internal
/// helpers raise these without knowing which algorithm + const
/// declaration owns the failure; the public entry points
/// ([`evaluate_fold`] / [`evaluate_scalar_init`]) attach the locator
/// from a [`ConstSite`] when crossing into the typed `GenerateError`
/// surface, producing wire-stable codes:
///
/// | variant | wire code |
/// |--|--|
/// | `NotFoldable`        | `algorithm/const-not-foldable` |
/// | `BudgetExceeded`     | `algorithm/const-fold-budget-exceeded` |
/// | `YieldTypeMismatch`  | `algorithm/const-yield-type-mismatch` |
#[derive(Debug)]
enum ConstFoldKind {
    /// Body construct outside the foldable substrate. `detail` quotes
    /// the specific clause (member access, runtime ident, malformed
    /// literal, …) so consumers reading the message text retain the
    /// β-era diagnostic shape.
    NotFoldable(String),
    /// Iteration budget exhausted. `budget` is the *configured* maximum
    /// (not the remaining count), so the message can quote the policy
    /// the operator is hitting verbatim.
    BudgetExceeded { budget: u64 },
    /// Coercion to the declared scalar / element type rejected. Mirrors
    /// the β slug payload (`actual → expected`).
    YieldTypeMismatch { expected: SceType, actual: String },
}

impl ConstFoldKind {
    /// Boundary lift. Attaches `algorithm` + `const_name` to produce
    /// the typed [`GenerateError`] variant the diagnostic wire layer
    /// dispatches on. Called only from [`evaluate_fold`] /
    /// [`evaluate_scalar_init`] — internal helpers stay
    /// context-agnostic so a future caller (e.g. a unit test or a new
    /// fold-form site) can re-route the same logic with its own
    /// locator.
    fn into_generate_error(self, site: ConstSite<'_>) -> GenerateError {
        match self {
            Self::NotFoldable(detail) => GenerateError::ConstNotFoldable {
                algorithm: site.algorithm.to_string(),
                const_name: site.const_name.to_string(),
                detail,
            },
            Self::BudgetExceeded { budget } => GenerateError::ConstFoldBudgetExceeded {
                algorithm: site.algorithm.to_string(),
                const_name: Some(site.const_name.to_string()),
                budget,
            },
            Self::YieldTypeMismatch { expected, actual } => GenerateError::ConstYieldTypeMismatch {
                algorithm: site.algorithm.to_string(),
                const_name: site.const_name.to_string(),
                expected,
                actual,
            },
        }
    }
}

/// Locator threaded into every [`evaluate_fold`] /
/// [`evaluate_scalar_init`] call so error lifting can name the offending
/// declaration without re-parsing the message text.
#[derive(Clone, Copy)]
pub(crate) struct ConstSite<'a> {
    pub algorithm: &'a str,
    pub const_name: &'a str,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Public surface
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Iteration budget for the host interpreter.
///
/// RFC §5.F bound 1: "Total iterations across all folds ≤
/// configurable budget (default 1M)". The same counter is decremented
/// on every fold iteration and on every nested-loop tick (while body,
/// foreach body), so a malicious or misauthored body cannot turn
/// `sce-build` into a general-purpose compute platform. CLI knob lives
/// at [`crate::ForgeCompileOptions::const_fold_budget`]; the default
/// constant is the single source of truth.
///
/// Tracks both the configured maximum (`max`) and the running
/// `remaining` count so [`ConstFoldKind::BudgetExceeded`] can quote
/// the policy the operator is hitting.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    max: u64,
    remaining: u64,
}

impl Budget {
    /// RFC §5.F default budget (1_000_000 total iterations).
    pub const DEFAULT_MAX_ITERS: u64 = 1_000_000;

    pub fn new(max_iters: u64) -> Self {
        Self {
            max: max_iters,
            remaining: max_iters,
        }
    }

    fn consume(&mut self) -> Result<(), ConstFoldKind> {
        if self.remaining == 0 {
            return Err(ConstFoldKind::BudgetExceeded { budget: self.max });
        }
        self.remaining -= 1;
        Ok(())
    }
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_ITERS)
    }
}

/// One concrete numeric value the host interpreter produces.
///
/// Variants mirror every scalar `SceType` admitted by RFC §5.A
/// algorithm bodies. `Bytes` and `String` are excluded because RFC
/// §5.F's `array<elem, len>` outer type only admits scalar elements
/// (the parser enforces this). Each variant carries the same
/// fixed-width integer / float / bool that the eventual emitted
/// const slot will hold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Bool(bool),
}

impl ConstValue {
    fn declared_type(&self) -> SceType {
        match self {
            Self::U8(_) => SceType::Uint8,
            Self::U16(_) => SceType::Uint16,
            Self::U32(_) => SceType::Uint32,
            Self::U64(_) => SceType::Uint64,
            Self::I8(_) => SceType::Int8,
            Self::I16(_) => SceType::Int16,
            Self::I32(_) => SceType::Int32,
            Self::I64(_) => SceType::Int64,
            Self::F32(_) => SceType::Float32,
            Self::F64(_) => SceType::Float64,
            Self::Bool(_) => SceType::Bool,
        }
    }
}

/// Evaluate a `<sce:fold>` body to its element vector.
///
/// Iterates `iter_var` over `[fold.range_start, fold.range_end)`,
/// running `fold.body` against a fresh local scope per iteration and
/// evaluating `fold.yield_expr` to one [`ConstValue`] per iteration.
/// The returned vector has length `range_end - range_start` and
/// every element's type equals `fold.elem_type`.
///
/// `site` locates the offending declaration so the boundary lift to
/// the typed [`GenerateError`] can route through the wire codes
/// `algorithm/const-not-foldable`,
/// `algorithm/const-fold-budget-exceeded`, and
/// `algorithm/const-yield-type-mismatch`.
pub(crate) fn evaluate_fold(
    fold: &FoldBody,
    budget: &mut Budget,
    site: ConstSite<'_>,
) -> Result<Vec<ConstValue>, GenerateError> {
    evaluate_fold_inner(fold, budget).map_err(|k| k.into_generate_error(site))
}

fn evaluate_fold_inner(
    fold: &FoldBody,
    budget: &mut Budget,
) -> Result<Vec<ConstValue>, ConstFoldKind> {
    let len = (fold.range_end - fold.range_start) as usize;
    let mut out = Vec::with_capacity(len);

    for i in fold.range_start..fold.range_end {
        budget.consume()?;
        let mut scope = Scope::new();
        let iter_value = scalar_from_i128(i as i128, &iter_var_type(&fold.elem_type, i))?;
        scope.declare(&fold.iter_var, iter_value);

        eval_stmts(&fold.body, &mut scope, budget)?;

        let yield_value = eval_expr_typed(&fold.yield_expr, &scope, &fold.elem_type)?;
        out.push(yield_value);
    }
    Ok(out)
}

/// Lower a scalar `<sce:const init=...>` literal to a [`ConstValue`]
/// of the declared type. Used by the algorithm renderer to emit
/// `const NAME: T = lit;` style declarations alongside the fold-form
/// arrays. Re-uses the same expression evaluator with an empty scope.
pub(crate) fn evaluate_scalar_init(
    init_expr: &str,
    declared: &SceType,
    site: ConstSite<'_>,
) -> Result<ConstValue, GenerateError> {
    let scope = Scope::new();
    eval_expr_typed(init_expr, &scope, declared).map_err(|k| k.into_generate_error(site))
}

/// Iter-variable type. RFC §5.F worked example uses `i: u32` over
/// `0..256`, but the IR derives the iter var's storage type from the
/// fold's `elem-type`. Picking the elem type is sufficient for every
/// fixture today (CRC-table form treats `i` as the same-width
/// integer it shifts left into the seed). When `elem_type` is a
/// float the iter variable falls back to `Int32` since
/// `range_start`/`range_end` are u32 in the IR.
fn iter_var_type(elem_type: &SceType, _iter_value: u32) -> SceType {
    match elem_type {
        SceType::Float32 | SceType::Float64 | SceType::Bool | SceType::String | SceType::Bytes => {
            SceType::Int32
        }
        _ => elem_type.clone(),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Per-language array-literal serialisation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Render an evaluated `Vec<ConstValue>` as a per-language array
/// literal body — the contents inside the brackets/braces. Caller
/// wraps with `[N]T{...}` (Go), `[T; N]` (Rust), `std::array<T, N>{
/// ... }` (Cpp), etc., per RFC §5.J.5.
///
/// Element formatting:
/// - Integers emit decimal — readable for hand-authored tables and
///   identical across hex/decimal authoring of the source. Per-language
///   suffixes (`u`, `_u16`, `L`) are added only when the language
///   requires them to disambiguate the literal's width.
/// - Floats emit `Display` (`{}`) which produces the canonical
///   shortest-round-trip form. Sufficient for the float-table
///   fixtures the manifest exercises.
/// - Bools emit the language-native keyword.
pub(crate) fn serialize_array_literal_body(
    values: &[ConstValue],
    lang: crate::generator::Language,
) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for (i, v) in values.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{}", FormatValue { value: *v, lang });
    }
    out
}

struct FormatValue {
    value: ConstValue,
    lang: crate::generator::Language,
}

impl std::fmt::Display for FormatValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::generator::Language;
        match self.value {
            // Plain decimal integers in most backends; the surrounding
            // array type declaration disambiguates width, so no suffix
            // is required for byte-equivalence with hand-authored
            // tables. (CRC-class fixtures keep author intent visible
            // by emitting decimal too.)
            //
            // Kotlin is the exception: its `ubyteArrayOf` /
            // `ushortArrayOf` factories take `vararg UByte` /
            // `vararg UShort`, and Int → narrow-unsigned widening is
            // not implicit. Each element is wrapped in `(N).toU<T>()`
            // so the type matches the factory's vararg signature.
            // Signed `byteArrayOf` / `shortArrayOf` likewise need
            // `(N).toByte()` / `(N).toShort()` wrappers when an entry
            // exceeds the destination's positive range; we wrap
            // unconditionally for stability across input ranges.
            ConstValue::U8(v) => match self.lang {
                Language::Kotlin => write!(f, "({v}).toUByte()"),
                _ => write!(f, "{v}"),
            },
            ConstValue::U16(v) => match self.lang {
                Language::Kotlin => write!(f, "({v}).toUShort()"),
                _ => write!(f, "{v}"),
            },
            ConstValue::U32(v) => match self.lang {
                Language::Kotlin => write!(f, "{v}u"),
                _ => write!(f, "{v}"),
            },
            ConstValue::U64(v) => match self.lang {
                Language::Kotlin => write!(f, "{v}uL"),
                _ => write!(f, "{v}"),
            },
            ConstValue::I8(v) => match self.lang {
                Language::Kotlin => write!(f, "({v}).toByte()"),
                _ => write!(f, "{v}"),
            },
            ConstValue::I16(v) => match self.lang {
                Language::Kotlin => write!(f, "({v}).toShort()"),
                _ => write!(f, "{v}"),
            },
            ConstValue::I32(v) => write!(f, "{v}"),
            ConstValue::I64(v) => match self.lang {
                Language::Kotlin => write!(f, "{v}L"),
                _ => write!(f, "{v}"),
            },
            ConstValue::F32(v) => format_float_lit(f, v as f64, true, self.lang),
            ConstValue::F64(v) => format_float_lit(f, v, false, self.lang),
            ConstValue::Bool(v) => match self.lang {
                Language::Python => write!(f, "{}", if v { "True" } else { "False" }),
                _ => write!(f, "{v}"),
            },
        }
    }
}

fn format_float_lit(
    f: &mut std::fmt::Formatter<'_>,
    v: f64,
    is_f32: bool,
    lang: crate::generator::Language,
) -> std::fmt::Result {
    use crate::generator::Language;
    // `{:?}` always emits a `.` for floats (e.g. `1.0` rather than
    // `1`), guaranteeing the literal parses as a float in every
    // target — Rust would otherwise infer `1` as `i32`.
    let body = format!("{v:?}");
    match (is_f32, lang) {
        (true, Language::Rust) => write!(f, "{body}f32"),
        (false, Language::Rust) => write!(f, "{body}f64"),
        (true, Language::Cpp | Language::C11) => write!(f, "{body}f"),
        _ => write!(f, "{body}"),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Internal: scope, eval values, statement / expression walker
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Lexical scope of a fold-body iteration. RFC §5.F declares the
/// scope is per-iteration ("running `body` against a fresh local
/// scope per iteration") so each fold tick gets a brand-new
/// [`Scope`]; nested while/foreach bodies inherit the outer scope by
/// reference (mutating in place is fine — the structured-control
/// statements never escape their enclosing scope).
#[derive(Default)]
struct Scope {
    vars: HashMap<String, ConstValue>,
}

impl Scope {
    fn new() -> Self {
        Self::default()
    }

    fn declare(&mut self, name: &str, value: ConstValue) {
        self.vars.insert(name.to_string(), value);
    }

    fn assign(&mut self, name: &str, value: ConstValue) -> Result<(), ConstFoldKind> {
        if !self.vars.contains_key(name) {
            return Err(ConstFoldKind::NotFoldable(format!(
                "assignment target '{name}' is not in scope \
                 (fold-body assigns must reference an earlier <sce:var>)"
            )));
        }
        self.vars.insert(name.to_string(), value);
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<ConstValue> {
        self.vars.get(name).copied()
    }
}

/// Wide intermediate value used during arithmetic. Storage variables
/// live in [`Scope`] as typed [`ConstValue`]s; loading one widens to
/// `Int(i128)` (any int variant) or `Float(f64)`. Storing a result
/// back into a typed slot truncates with two-complement semantics
/// for narrow integers.
#[derive(Clone, Copy, Debug)]
enum EvalValue {
    Int(i128),
    Float(f64),
    Bool(bool),
}

impl EvalValue {
    fn from_const(c: ConstValue) -> Self {
        match c {
            ConstValue::U8(v) => Self::Int(v as i128),
            ConstValue::U16(v) => Self::Int(v as i128),
            ConstValue::U32(v) => Self::Int(v as i128),
            ConstValue::U64(v) => Self::Int(v as i128),
            ConstValue::I8(v) => Self::Int(v as i128),
            ConstValue::I16(v) => Self::Int(v as i128),
            ConstValue::I32(v) => Self::Int(v as i128),
            ConstValue::I64(v) => Self::Int(v as i128),
            ConstValue::F32(v) => Self::Float(v as f64),
            ConstValue::F64(v) => Self::Float(v),
            ConstValue::Bool(v) => Self::Bool(v),
        }
    }

    fn to_bool(self) -> Result<bool, ConstFoldKind> {
        match self {
            Self::Bool(b) => Ok(b),
            Self::Int(i) => Ok(i != 0),
            Self::Float(_) => Err(ConstFoldKind::NotFoldable(
                "cannot use float as boolean".to_string(),
            )),
        }
    }
}

fn eval_stmts(
    stmts: &[AlgorithmStmt],
    scope: &mut Scope,
    budget: &mut Budget,
) -> Result<(), ConstFoldKind> {
    for s in stmts {
        eval_stmt(s, scope, budget)?;
    }
    Ok(())
}

fn eval_stmt(
    s: &AlgorithmStmt,
    scope: &mut Scope,
    budget: &mut Budget,
) -> Result<(), ConstFoldKind> {
    match s {
        AlgorithmStmt::Var {
            name,
            sce_type,
            init,
        } => {
            let value = eval_expr_typed(init, scope, sce_type)?;
            scope.declare(name, value);
            Ok(())
        }
        AlgorithmStmt::Assign { target, expr } => {
            let target = target.trim();
            // Identifier-only LValues in fold bodies. Member/Index
            // assignments are out of scope for v1 — the fold's output
            // is the surrounding `array<elem, len>`, not an inner
            // mutable structure.
            if !is_simple_ident(target) {
                return Err(ConstFoldKind::NotFoldable(format!(
                    "assign target '{target}' is not a bare identifier \
                     (fold bodies cannot mutate members or indexed slots in v1)"
                )));
            }
            let prev = scope.lookup(target).ok_or_else(|| {
                ConstFoldKind::NotFoldable(format!("assign target '{target}' is not in scope"))
            })?;
            let new = eval_expr_typed(expr, scope, &prev.declared_type())?;
            scope.assign(target, new)?;
            Ok(())
        }
        AlgorithmStmt::If {
            cond,
            then_body,
            else_body,
        } => {
            let c = eval_expr(cond, scope)?.to_bool()?;
            if c {
                eval_stmts(then_body, scope, budget)?;
            } else if let Some(eb) = else_body {
                eval_stmts(eb, scope, budget)?;
            }
            Ok(())
        }
        AlgorithmStmt::While {
            cond,
            body,
            max_iter,
        } => {
            let cap = max_iter.unwrap_or(u32::MAX);
            let mut ticks = 0u32;
            while eval_expr(cond, scope)?.to_bool()? {
                if ticks >= cap {
                    return Err(ConstFoldKind::NotFoldable(format!(
                        "while loop exceeded max-iter={cap} \
                         (fold-body while loops must terminate within max-iter)"
                    )));
                }
                budget.consume()?;
                eval_stmts(body, scope, budget)?;
                ticks += 1;
            }
            Ok(())
        }
        AlgorithmStmt::Foreach { item, source, body } => {
            // Source must resolve to a `bytes`-typed binding. v1 only
            // supports iterating `bytes`, mirroring algorithm-body
            // semantics (lower_algorithm_stmt :: AlgorithmStmt::Foreach).
            // Bytes are not foldable values today (no `bytes` const
            // can appear inside a fold scope), so this is unreachable
            // for fixtures that respect the parser's element-type
            // constraint — but a clean diagnostic guards us if a
            // future fixture changes that.
            let _ = (item, source, body);
            Err(ConstFoldKind::NotFoldable(
                "<sce:foreach> not supported inside a fold body in v1 \
                 (fold scope contains only scalar locals; bytes-typed \
                 iteration would require a `bytes` source binding which \
                 is not constructible at fold time)"
                    .to_string(),
            ))
        }
        AlgorithmStmt::Return { .. } => Err(ConstFoldKind::NotFoldable(
            "<sce:return> is forbidden inside a fold body (RFC §5.F — \
             fold elements are produced by <sce:yield>, not by early return)"
                .to_string(),
        )),
        AlgorithmStmt::Call { target, .. } => Err(ConstFoldKind::NotFoldable(format!(
            "<sce:call target=\"{target}\"> is forbidden inside a fold body \
             (RFC §5.F bound 3 — host interpreter is pure; cross-algorithm \
             calls require runtime resolution)"
        ))),
    }
}

fn is_simple_ident(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// Evaluate an expression and coerce the result to `expected`.
fn eval_expr_typed(
    expr_text: &str,
    scope: &Scope,
    expected: &SceType,
) -> Result<ConstValue, ConstFoldKind> {
    let value = eval_expr(expr_text, scope)?;
    coerce_to_const(value, expected)
}

/// Evaluate an expression to a wide [`EvalValue`] without type
/// coercion. Used at every site that does not store the result
/// (If.cond, While.cond) and as the inner step of
/// [`eval_expr_typed`].
fn eval_expr(expr_text: &str, scope: &Scope) -> Result<EvalValue, ConstFoldKind> {
    let ast = expr::parse_to_ast(expr_text).map_err(map_expr_err)?;
    eval_node(&ast, scope)
}

fn eval_node(node: &TypedExpr, scope: &Scope) -> Result<EvalValue, ConstFoldKind> {
    match &node.kind {
        ExprKind::NumberLit(s) => parse_number_lit(s),
        ExprKind::BoolLit(b) => Ok(EvalValue::Bool(*b)),
        ExprKind::NullLit => Err(ConstFoldKind::NotFoldable(
            "`null` literal has no fold-time value".to_string(),
        )),
        ExprKind::StringLit { .. } => Err(ConstFoldKind::NotFoldable(
            "string literals are not foldable to a scalar array element".to_string(),
        )),
        ExprKind::Ident(name) => scope
            .lookup(name)
            .map(EvalValue::from_const)
            .ok_or_else(|| {
                ConstFoldKind::NotFoldable(format!(
                    "identifier '{name}' is not in fold scope \
                     (fold bodies see only the iter variable, locals declared \
                     by <sce:var>, and literals)"
                ))
            }),
        ExprKind::Raw(s) => Err(ConstFoldKind::NotFoldable(format!(
            "pre-rendered fragment '{s}' has no fold-time value"
        ))),
        ExprKind::Binary { op, left, right } => {
            let l = eval_node(left, scope)?;
            let r = eval_node(right, scope)?;
            eval_binop(*op, l, r)
        }
        ExprKind::Unary { op, operand } => {
            let v = eval_node(operand, scope)?;
            eval_unop(*op, v)
        }
        ExprKind::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            let c = eval_node(condition, scope)?.to_bool()?;
            if c {
                eval_node(consequent, scope)
            } else {
                eval_node(alternate, scope)
            }
        }
        ExprKind::Member { .. } => Err(ConstFoldKind::NotFoldable(
            "member access is not supported inside a fold body \
             (cross-record references require runtime resolution)"
                .to_string(),
        )),
        ExprKind::Index { .. } => Err(ConstFoldKind::NotFoldable(
            "indexed access is not supported inside a fold body \
             (v1 fold scope contains only scalar locals)"
                .to_string(),
        )),
        ExprKind::Call { .. } => Err(ConstFoldKind::NotFoldable(
            "function calls are not supported inside a fold body \
             (RFC §5.F bound 3 — host interpreter is pure)"
                .to_string(),
        )),
    }
}

fn eval_binop(op: BinOp, l: EvalValue, r: EvalValue) -> Result<EvalValue, ConstFoldKind> {
    use EvalValue as V;
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => match (l, r) {
            (V::Int(a), V::Int(b)) => Ok(V::Int(arith_int(op, a, b)?)),
            (V::Float(a), V::Float(b)) => Ok(V::Float(arith_float(op, a, b)?)),
            (V::Int(a), V::Float(b)) => Ok(V::Float(arith_float(op, a as f64, b)?)),
            (V::Float(a), V::Int(b)) => Ok(V::Float(arith_float(op, a, b as f64)?)),
            _ => Err(ConstFoldKind::NotFoldable(
                "arithmetic on non-numeric operand".to_string(),
            )),
        },
        BinOp::StrictEq => Ok(V::Bool(eq(l, r))),
        BinOp::StrictNeq => Ok(V::Bool(!eq(l, r))),
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => match (l, r) {
            (V::Int(a), V::Int(b)) => Ok(V::Bool(cmp_int(op, a, b))),
            (V::Float(a), V::Float(b)) => Ok(V::Bool(cmp_float(op, a, b))),
            (V::Int(a), V::Float(b)) => Ok(V::Bool(cmp_float(op, a as f64, b))),
            (V::Float(a), V::Int(b)) => Ok(V::Bool(cmp_float(op, a, b as f64))),
            _ => Err(ConstFoldKind::NotFoldable(
                "comparison on non-numeric operand".to_string(),
            )),
        },
        BinOp::And => Ok(V::Bool(l.to_bool()? && r.to_bool()?)),
        BinOp::Or => Ok(V::Bool(l.to_bool()? || r.to_bool()?)),
        BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr | BinOp::UShr => {
            let a = require_int(l)?;
            let b = require_int(r)?;
            Ok(V::Int(bitwise_int(op, a, b)?))
        }
    }
}

fn arith_int(op: BinOp, a: i128, b: i128) -> Result<i128, ConstFoldKind> {
    match op {
        // Wrapping at i128 width — this is wider than every target's
        // fixed-width int, so the eventual coerce-to-stored-type
        // truncation is what produces target-equivalent wrapping.
        BinOp::Add => Ok(a.wrapping_add(b)),
        BinOp::Sub => Ok(a.wrapping_sub(b)),
        BinOp::Mul => Ok(a.wrapping_mul(b)),
        BinOp::Div => {
            if b == 0 {
                return Err(ConstFoldKind::NotFoldable(
                    "integer division by zero".to_string(),
                ));
            }
            Ok(a.wrapping_div(b))
        }
        BinOp::Mod => {
            if b == 0 {
                return Err(ConstFoldKind::NotFoldable(
                    "integer modulo by zero".to_string(),
                ));
            }
            Ok(a.wrapping_rem(b))
        }
        _ => unreachable!("non-arith op routed to arith_int: {op:?}"),
    }
}

fn arith_float(op: BinOp, a: f64, b: f64) -> Result<f64, ConstFoldKind> {
    match op {
        BinOp::Add => Ok(a + b),
        BinOp::Sub => Ok(a - b),
        BinOp::Mul => Ok(a * b),
        BinOp::Div => Ok(a / b),
        BinOp::Mod => Ok(a % b),
        _ => unreachable!("non-arith op routed to arith_float: {op:?}"),
    }
}

fn cmp_int(op: BinOp, a: i128, b: i128) -> bool {
    match op {
        BinOp::Lt => a < b,
        BinOp::Gt => a > b,
        BinOp::LtEq => a <= b,
        BinOp::GtEq => a >= b,
        _ => unreachable!(),
    }
}

fn cmp_float(op: BinOp, a: f64, b: f64) -> bool {
    match op {
        BinOp::Lt => a < b,
        BinOp::Gt => a > b,
        BinOp::LtEq => a <= b,
        BinOp::GtEq => a >= b,
        _ => unreachable!(),
    }
}

fn bitwise_int(op: BinOp, a: i128, b: i128) -> Result<i128, ConstFoldKind> {
    match op {
        BinOp::BitAnd => Ok(a & b),
        BinOp::BitOr => Ok(a | b),
        BinOp::BitXor => Ok(a ^ b),
        BinOp::Shl => {
            if !(0..128).contains(&b) {
                return Err(ConstFoldKind::NotFoldable(format!(
                    "shift count {b} out of range"
                )));
            }
            Ok(a.wrapping_shl(b as u32))
        }
        BinOp::Shr | BinOp::UShr => {
            if !(0..128).contains(&b) {
                return Err(ConstFoldKind::NotFoldable(format!(
                    "shift count {b} out of range"
                )));
            }
            // RFC §5.F evaluator follows arithmetic-right-shift on
            // the wide i128 domain. Per-language `>>>` (unsigned)
            // semantics are reproduced at storage time: every
            // non-negative coerce-into-unsigned-narrow target masks
            // off high bits, matching `u8/u16/u32/u64`'s logical
            // shift behaviour. CRC-class fixtures stay in
            // non-negative u16/u32 territory throughout.
            Ok(a >> (b as u32))
        }
        _ => unreachable!("non-bitwise op routed to bitwise_int: {op:?}"),
    }
}

fn require_int(v: EvalValue) -> Result<i128, ConstFoldKind> {
    match v {
        EvalValue::Int(i) => Ok(i),
        EvalValue::Bool(b) => Ok(if b { 1 } else { 0 }),
        EvalValue::Float(_) => Err(ConstFoldKind::NotFoldable(
            "bitwise operation on float operand".to_string(),
        )),
    }
}

fn eq(a: EvalValue, b: EvalValue) -> bool {
    use EvalValue as V;
    match (a, b) {
        (V::Int(x), V::Int(y)) => x == y,
        (V::Float(x), V::Float(y)) => x == y,
        (V::Int(x), V::Float(y)) | (V::Float(y), V::Int(x)) => (x as f64) == y,
        (V::Bool(x), V::Bool(y)) => x == y,
        _ => false,
    }
}

fn eval_unop(op: UnaryOp, v: EvalValue) -> Result<EvalValue, ConstFoldKind> {
    use EvalValue as V;
    match op {
        UnaryOp::Pos => Ok(v),
        UnaryOp::Neg => match v {
            V::Int(i) => Ok(V::Int(i.wrapping_neg())),
            V::Float(f) => Ok(V::Float(-f)),
            V::Bool(_) => Err(ConstFoldKind::NotFoldable(
                "unary minus on bool".to_string(),
            )),
        },
        UnaryOp::Not => Ok(V::Bool(!v.to_bool()?)),
        UnaryOp::BitNot => match v {
            V::Int(i) => Ok(V::Int(!i)),
            _ => Err(ConstFoldKind::NotFoldable(
                "bitwise NOT on non-integer".to_string(),
            )),
        },
    }
}

fn parse_number_lit(s: &str) -> Result<EvalValue, ConstFoldKind> {
    let s = s.trim();
    let is_float = s.contains('.')
        || s.chars().any(|c| c == 'e' || c == 'E') && !s.starts_with("0x") && !s.starts_with("0X");
    if is_float {
        s.parse::<f64>()
            .map(EvalValue::Float)
            .map_err(|e| ConstFoldKind::NotFoldable(format!("malformed float literal '{s}': {e}")))
    } else if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i128::from_str_radix(rest, 16)
            .map(EvalValue::Int)
            .map_err(|e| ConstFoldKind::NotFoldable(format!("malformed hex literal '{s}': {e}")))
    } else if let Some(rest) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i128::from_str_radix(rest, 2)
            .map(EvalValue::Int)
            .map_err(|e| ConstFoldKind::NotFoldable(format!("malformed binary literal '{s}': {e}")))
    } else if let Some(rest) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        i128::from_str_radix(rest, 8)
            .map(EvalValue::Int)
            .map_err(|e| ConstFoldKind::NotFoldable(format!("malformed octal literal '{s}': {e}")))
    } else {
        s.parse::<i128>().map(EvalValue::Int).map_err(|e| {
            ConstFoldKind::NotFoldable(format!("malformed integer literal '{s}': {e}"))
        })
    }
}

fn coerce_to_const(value: EvalValue, ty: &SceType) -> Result<ConstValue, ConstFoldKind> {
    use SceType::*;
    match (value, ty) {
        (EvalValue::Bool(b), Bool) => Ok(ConstValue::Bool(b)),
        (EvalValue::Int(i), Bool) => Ok(ConstValue::Bool(i != 0)),
        (EvalValue::Bool(_), _) => Err(ConstFoldKind::YieldTypeMismatch {
            expected: ty.clone(),
            actual: "bool".to_string(),
        }),

        (EvalValue::Float(f), Float32) => Ok(ConstValue::F32(f as f32)),
        (EvalValue::Float(f), Float64) => Ok(ConstValue::F64(f)),
        (EvalValue::Int(i), Float32) => Ok(ConstValue::F32(i as f32)),
        (EvalValue::Int(i), Float64) => Ok(ConstValue::F64(i as f64)),

        (EvalValue::Float(_), _) => Err(ConstFoldKind::YieldTypeMismatch {
            expected: ty.clone(),
            actual: "float".to_string(),
        }),

        (EvalValue::Int(i), Uint8) => Ok(ConstValue::U8((i as u128 & 0xFF) as u8)),
        (EvalValue::Int(i), Uint16) => Ok(ConstValue::U16((i as u128 & 0xFFFF) as u16)),
        (EvalValue::Int(i), Uint32) => Ok(ConstValue::U32((i as u128 & 0xFFFF_FFFF) as u32)),
        (EvalValue::Int(i), Uint64) => {
            Ok(ConstValue::U64((i as u128 & 0xFFFF_FFFF_FFFF_FFFF) as u64))
        }
        (EvalValue::Int(i), Int8) => Ok(ConstValue::I8(i as i8)),
        (EvalValue::Int(i), Int16) => Ok(ConstValue::I16(i as i16)),
        (EvalValue::Int(i), Int32) => Ok(ConstValue::I32(i as i32)),
        (EvalValue::Int(i), Int64) => Ok(ConstValue::I64(i as i64)),

        // String / Bytes are not RFC §5.F element types — the parser
        // already rejects them on `array<elem>` so this arm is
        // unreachable for fixtures, but keeping the explicit error
        // closes the typed-coercion match exhaustively.
        (_, String | Bytes) => Err(ConstFoldKind::YieldTypeMismatch {
            expected: ty.clone(),
            actual: "scalar fold yield".to_string(),
        }),
        // NL→IR Item C1 Path A: enum-typed array elements would
        // require resolving the imported enum's variants — out of
        // scope for the scalar const-fold path. Authors using enum
        // element types reach this only via misconfiguration; the
        // explicit error keeps the match exhaustive.
        (_, Enum(_)) => Err(ConstFoldKind::YieldTypeMismatch {
            expected: ty.clone(),
            actual: "scalar fold yield".to_string(),
        }),
    }
}

fn scalar_from_i128(i: i128, ty: &SceType) -> Result<ConstValue, ConstFoldKind> {
    coerce_to_const(EvalValue::Int(i), ty)
}

fn map_expr_err(e: ExprError) -> ConstFoldKind {
    ConstFoldKind::NotFoldable(format!("failed to parse fold-body expression: {e}"))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Unit tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::{AlgorithmStmt, FoldBody, SceType};

    /// Test-only locator. Real call sites pass authoring identifiers;
    /// unit tests don't care about the wire-locator content.
    const TEST_SITE: ConstSite<'static> = ConstSite {
        algorithm: "test_alg",
        const_name: "test_const",
    };

    /// Smoke evaluator: `array<u16, 4> { i + i }` over `0..4`
    /// should produce `[0, 2, 4, 6]`.
    #[test]
    fn smoke_doubled_table() {
        let fold = FoldBody {
            range_start: 0,
            range_end: 4,
            iter_var: "i".into(),
            elem_type: SceType::Uint16,
            body: vec![AlgorithmStmt::Var {
                name: "doubled".into(),
                sce_type: SceType::Uint16,
                init: "i + i".into(),
            }],
            yield_expr: "doubled".into(),
        };
        let mut budget = Budget::default();
        let out = evaluate_fold(&fold, &mut budget, TEST_SITE).unwrap();
        assert_eq!(
            out,
            vec![
                ConstValue::U16(0),
                ConstValue::U16(2),
                ConstValue::U16(4),
                ConstValue::U16(6),
            ]
        );
    }

    /// CRC16-CCITT-FALSE table generation. Hand-computed reference
    /// values (rows 0, 1, 2, 0xFF) come from any canonical CRC16
    /// reference implementation (e.g. crccalc.com); identical
    /// across endianness because the table is byte-keyed.
    #[test]
    fn crc16_ccitt_false_table_matches_reference() {
        let fold = FoldBody {
            range_start: 0,
            range_end: 256,
            iter_var: "i".into(),
            elem_type: SceType::Uint16,
            body: vec![
                AlgorithmStmt::Var {
                    name: "c".into(),
                    sce_type: SceType::Uint16,
                    init: "i << 8".into(),
                },
                AlgorithmStmt::Var {
                    name: "bit".into(),
                    sce_type: SceType::Uint16,
                    init: "0".into(),
                },
                AlgorithmStmt::While {
                    cond: "bit < 8".into(),
                    max_iter: Some(8),
                    body: vec![
                        AlgorithmStmt::If {
                            cond: "(c & 0x8000) !== 0".into(),
                            then_body: vec![AlgorithmStmt::Assign {
                                target: "c".into(),
                                expr: "(c << 1) ^ 0x1021".into(),
                            }],
                            else_body: Some(vec![AlgorithmStmt::Assign {
                                target: "c".into(),
                                expr: "c << 1".into(),
                            }]),
                        },
                        AlgorithmStmt::Assign {
                            target: "bit".into(),
                            expr: "bit + 1".into(),
                        },
                    ],
                },
            ],
            yield_expr: "c".into(),
        };
        let mut budget = Budget::default();
        let out = evaluate_fold(&fold, &mut budget, TEST_SITE).expect("CRC16 fold must evaluate");
        // Reference values verified against the canonical CRC16-CCITT-FALSE
        // table (poly 0x1021, init 0xFFFF, no reflect, no xor-out — RFC 1662
        // / ISO/IEC 13239 / X.25). Spot-checks in each octet of the index
        // space prove every nested control-flow branch executes correctly.
        assert_eq!(out.len(), 256);
        assert_eq!(out[0x00], ConstValue::U16(0x0000));
        assert_eq!(out[0x01], ConstValue::U16(0x1021));
        assert_eq!(out[0x02], ConstValue::U16(0x2042));
        assert_eq!(out[0x08], ConstValue::U16(0x8108));
        assert_eq!(out[0x10], ConstValue::U16(0x1231));
        assert_eq!(out[0x80], ConstValue::U16(0x9188));
        assert_eq!(out[0xFF], ConstValue::U16(0x1EF0));
    }

    /// Budget = 3 must reject a 4-iteration fold.
    #[test]
    fn budget_exceeded_rejects_oversized_fold() {
        let fold = FoldBody {
            range_start: 0,
            range_end: 4,
            iter_var: "i".into(),
            elem_type: SceType::Uint16,
            body: vec![],
            yield_expr: "i".into(),
        };
        let mut budget = Budget::new(3);
        let err = evaluate_fold(&fold, &mut budget, TEST_SITE).unwrap_err();
        assert!(
            matches!(
                err,
                GenerateError::ConstFoldBudgetExceeded { budget: 3, .. }
            ),
            "budget-exceeded error must surface; got {err:?}"
        );
    }

    /// Return / Call statements inside a fold body must be rejected.
    #[test]
    fn fold_body_rejects_return_and_call() {
        let make_fold = |stmt: AlgorithmStmt| FoldBody {
            range_start: 0,
            range_end: 1,
            iter_var: "i".into(),
            elem_type: SceType::Uint16,
            body: vec![stmt],
            yield_expr: "i".into(),
        };

        let mut budget = Budget::default();
        let ret_err = evaluate_fold(
            &make_fold(AlgorithmStmt::Return {
                expr: Some("0".into()),
            }),
            &mut budget,
            TEST_SITE,
        )
        .unwrap_err();
        assert!(
            matches!(
                ret_err,
                GenerateError::ConstNotFoldable { ref detail, .. } if detail.contains("<sce:return>")
            ),
            "return inside fold must be rejected; got {ret_err:?}"
        );

        let mut budget = Budget::default();
        let call_err = evaluate_fold(
            &make_fold(AlgorithmStmt::Call {
                target: "other".into(),
                args: vec![],
            }),
            &mut budget,
            TEST_SITE,
        )
        .unwrap_err();
        assert!(
            matches!(
                call_err,
                GenerateError::ConstNotFoldable { ref detail, .. } if detail.contains("<sce:call")
            ),
            "call inside fold must be rejected; got {call_err:?}"
        );
    }

    /// While loops must terminate within their declared max-iter.
    #[test]
    fn while_max_iter_enforced() {
        let fold = FoldBody {
            range_start: 0,
            range_end: 1,
            iter_var: "i".into(),
            elem_type: SceType::Uint16,
            body: vec![AlgorithmStmt::While {
                cond: "i === i".into(), // always true
                max_iter: Some(4),
                body: vec![],
            }],
            yield_expr: "i".into(),
        };
        let mut budget = Budget::default();
        let err = evaluate_fold(&fold, &mut budget, TEST_SITE).unwrap_err();
        assert!(
            matches!(
                err,
                GenerateError::ConstNotFoldable { ref detail, .. } if detail.contains("max-iter")
            ),
            "while max-iter must be enforced; got {err:?}"
        );
    }

    /// Scalar init evaluator coerces hex literals to the declared
    /// scalar type, mirroring the typed-Var path.
    #[test]
    fn scalar_init_evaluates_hex_to_u16() {
        let v = evaluate_scalar_init("0xFFFF", &SceType::Uint16, TEST_SITE).unwrap();
        assert_eq!(v, ConstValue::U16(0xFFFF));
    }

    /// Per-language array-literal serialisation.
    #[test]
    fn serialize_decimal_integers() {
        use crate::generator::Language;
        let body = serialize_array_literal_body(
            &[
                ConstValue::U16(0),
                ConstValue::U16(2),
                ConstValue::U16(0x1021),
            ],
            Language::Rust,
        );
        assert_eq!(body, "0, 2, 4129");
        let cpp_body = serialize_array_literal_body(&[ConstValue::U16(0xFFFF)], Language::Cpp);
        assert_eq!(cpp_body, "65535");
    }
}
