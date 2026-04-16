// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge type system for expression transpilation.
//
// This module provides the abstract type lattice used by the expression
// transpiler to reason about operand and result types independently of the
// target language. Each language emitter consumes these types plus a small
// per-language coercion table to produce idiomatic output.
//
// Design rationale:
//
// * **Untyped numeric sentinels** — integer and float literals are assigned
//   `UntypedInt` / `UntypedFloat` rather than a fixed width. Their concrete
//   type is determined by context (bidirectional typing, as in Go "untyped
//   constants" and Haskell's `Num a => a`). Without this, an expression like
//   `celsius * 9 / 5 + 32` (celsius: f64) would be stuck with `9`/`5`/`32`
//   as i32, producing a type error in strict languages like Rust.
//
// * **No Coerce AST node** — coercion is performed at emission time by each
//   language emitter, comparing each child's natural type to the "expected"
//   type propagated down from the parent. This keeps the AST language-
//   agnostic and concentrates cast syntax in one place per language.
//
// * **Hex/binary/octal literals are NOT promotable to float** — an expression
//   like `x * 0xFF` (x: f64) is a compile error, not a silent `0xFF as f64`.
//   Users must rewrite to a decimal float literal. This matches textbook
//   strict type checking and prevents silent precision loss.

use crate::forge::model::SceType;
use std::collections::HashMap;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Inferred type lattice
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Abstract type of an expression node after inference.
///
/// The lattice has three families:
///
/// * **Untyped** (malleable) — `UntypedInt`, `UntypedFloat`: literal values
///   whose concrete type is determined by the surrounding context. An
///   untyped literal adopts the type of the context into which it flows.
///
/// * **Concrete** — `Int`, `Float`, `Bool`, `Str`, `Bytes`, `Null`: fully
///   determined types, usually flowing in from `TypeCtx::vars` lookups.
///
/// * **Unknown** — opaque: the node's type cannot be inferred from local
///   information (e.g., a member access on an external struct). Emitters
///   must refuse to insert coercion when either operand is `Unknown`; the
///   user is responsible for providing type-correct expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredType {
    /// Integer literal whose width/signedness is determined by context.
    /// Includes decimal, hex, binary, and octal forms. The emitter must
    /// inspect the original literal text to decide if promotion to float
    /// is legal (decimal: yes, hex/bin/oct: no).
    UntypedInt,

    /// Floating-point literal (contains `.`, `e`, or `E`) whose width
    /// (f32/f64) is determined by context. Defaults to f64 if unconstrained.
    UntypedFloat,

    /// Concrete integer with explicit sign and bit width.
    Int { signed: bool, bits: u8 },

    /// Concrete floating-point with explicit bit width (32 or 64).
    Float { bits: u8 },

    /// Boolean.
    Bool,

    /// String.
    Str,

    /// Byte array.
    Bytes,

    /// The `null` literal. Has no direct SCE type; mostly used for
    /// emitter-side null-checks and rejected in arithmetic contexts.
    Null,

    /// Opaque — type cannot be inferred from the expression alone.
    /// Produced by unresolved identifiers, unresolved member accesses,
    /// and calls to functions absent from the context's `funcs` table.
    Unknown,
}

impl InferredType {
    /// Is this type a concrete or untyped integer?
    pub fn is_integer_like(&self) -> bool {
        matches!(self, Self::UntypedInt | Self::Int { .. })
    }

    /// Is this type a concrete or untyped float?
    pub fn is_float_like(&self) -> bool {
        matches!(self, Self::UntypedFloat | Self::Float { .. })
    }

    /// Does any arithmetic operand of this type require floating-point
    /// arithmetic? True for any float-family type.
    pub fn is_arith_float(&self) -> bool {
        self.is_float_like()
    }

    /// Map an `SceType` from the model layer to an inferred concrete type.
    ///
    /// This is the single entry point from the generator-layer type system
    /// into the inference layer. All `TypeCtx::vars` and `FuncSig` entries
    /// produced by generator code flow through this converter.
    pub fn from_sce_type(ty: &SceType) -> Self {
        match ty {
            SceType::Uint8 => Self::Int { signed: false, bits: 8 },
            SceType::Uint16 => Self::Int { signed: false, bits: 16 },
            SceType::Uint32 => Self::Int { signed: false, bits: 32 },
            SceType::Uint64 => Self::Int { signed: false, bits: 64 },
            SceType::Int8 => Self::Int { signed: true, bits: 8 },
            SceType::Int16 => Self::Int { signed: true, bits: 16 },
            SceType::Int32 => Self::Int { signed: true, bits: 32 },
            SceType::Int64 => Self::Int { signed: true, bits: 64 },
            SceType::Float32 => Self::Float { bits: 32 },
            SceType::Float64 => Self::Float { bits: 64 },
            SceType::Bool => Self::Bool,
            SceType::String => Self::Str,
            SceType::Bytes => Self::Bytes,
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Lattice joins (used by inference for arithmetic / comparison / branch)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Arithmetic join: result type of `left op right` where `op` is any of
/// `+ - * / %` or the arithmetic-unifying context in comparisons and
/// conditional branches.
///
/// Rules (commutative):
/// * `Unknown ⊔ _` = `Unknown` (propagates the opacity)
/// * `UntypedInt ⊔ UntypedInt` = `UntypedInt`
/// * `UntypedFloat ⊔ UntypedFloat` = `UntypedFloat`
/// * `UntypedInt ⊔ UntypedFloat` = `UntypedFloat`
/// * `UntypedInt ⊔ Int{s,b}` = `Int{s,b}` (literal adopts concrete)
/// * `UntypedInt ⊔ Float{b}` = `Float{b}`
/// * `UntypedFloat ⊔ Int{_,b}` = `Float{max(32, b)}` — float wins
/// * `UntypedFloat ⊔ Float{b}` = `Float{b}`
/// * `Int{s1,b1} ⊔ Int{s2,b2}` = `Int{s1||s2, max(b1,b2)}` (widen, prefer signed)
/// * `Int{_,bi} ⊔ Float{bf}` = `Float{max(bf, 32)}`
/// * `Float{b1} ⊔ Float{b2}` = `Float{max(b1,b2)}`
/// * Everything else (Bool, Str, Bytes, Null) mixed with a numeric → `Unknown`
pub fn join_arith(left: InferredType, right: InferredType) -> InferredType {
    use InferredType::*;
    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,

        (UntypedInt, UntypedInt) => UntypedInt,
        (UntypedFloat, UntypedFloat) => UntypedFloat,
        (UntypedInt, UntypedFloat) | (UntypedFloat, UntypedInt) => UntypedFloat,

        (UntypedInt, Int { signed, bits }) | (Int { signed, bits }, UntypedInt) => {
            Int { signed, bits }
        }
        (UntypedInt, Float { bits }) | (Float { bits }, UntypedInt) => Float { bits },

        (UntypedFloat, Int { bits, .. }) | (Int { bits, .. }, UntypedFloat) => {
            Float { bits: bits.max(32) }
        }
        (UntypedFloat, Float { bits }) | (Float { bits }, UntypedFloat) => Float { bits },

        (Int { signed: s1, bits: b1 }, Int { signed: s2, bits: b2 }) => {
            Int { signed: s1 || s2, bits: b1.max(b2) }
        }
        (Int { bits: bi, .. }, Float { bits: bf })
        | (Float { bits: bf }, Int { bits: bi, .. }) => {
            let _ = bi; // integer width ignored when unified with float
            Float { bits: bf.max(32) }
        }
        (Float { bits: b1 }, Float { bits: b2 }) => Float { bits: b1.max(b2) },

        // Mixing non-numeric with numeric is opaque — the user has written
        // a semantically invalid expression; don't silently paper over it.
        _ => Unknown,
    }
}

/// Integer-only join, used for bitwise ops and shifts. Non-integer operands
/// poison the result to `Unknown` (bitwise on floats/bools/strings is nonsense).
pub fn join_int(left: InferredType, right: InferredType) -> InferredType {
    use InferredType::*;
    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,

        (UntypedInt, UntypedInt) => UntypedInt,

        (UntypedInt, Int { signed, bits }) | (Int { signed, bits }, UntypedInt) => {
            Int { signed, bits }
        }

        (Int { signed: s1, bits: b1 }, Int { signed: s2, bits: b2 }) => {
            Int { signed: s1 || s2, bits: b1.max(b2) }
        }

        _ => Unknown,
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Function signatures and type context
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Signature of a function that may appear in a SCXML expression as
/// `callee(args...)`. Used to type `Call` nodes where the callee is a
/// cross-file import or a built-in intrinsic.
#[derive(Debug, Clone)]
pub struct FuncSig {
    /// Parameter types, in positional order.
    pub params: Vec<InferredType>,
    /// Return type.
    pub ret: InferredType,
}

/// Type context for expression inference. Built by the generator from a
/// kind model (transform/condition/validator/procedure/etc.) and passed
/// into `expr::transpile_typed` at every call site.
///
/// The context has two namespaces:
/// * `vars` — identifier → type. Populated from the kind's input/internal/
///   output fields. For stateful imports, member-access paths like
///   `alias_.field` are also folded in as pre-composed keys (e.g., `alias_`
///   is a member whose fields appear via property access; see the per-kind
///   builders in `forge::type_ctx`).
/// * `funcs` — function name → signature. Populated from cross-file import
///   aliases for stateless kinds (Transform, Condition, Lookup).
#[derive(Debug, Clone, Default)]
pub struct TypeCtx<'a> {
    pub vars: HashMap<&'a str, InferredType>,
    pub funcs: HashMap<&'a str, FuncSig>,
}

impl<'a> TypeCtx<'a> {
    pub fn new() -> Self {
        Self { vars: HashMap::new(), funcs: HashMap::new() }
    }

    /// Look up an identifier's type. Returns `Unknown` if absent so that
    /// inference can still proceed on expressions mixing known and unknown
    /// identifiers.
    pub fn lookup_var(&self, name: &str) -> InferredType {
        self.vars.get(name).copied().unwrap_or(InferredType::Unknown)
    }

    /// Look up a function signature. Returns `None` if absent.
    pub fn lookup_func(&self, name: &str) -> Option<&FuncSig> {
        self.funcs.get(name)
    }

    /// Insert a typed identifier. Convenience for builders.
    pub fn insert_var(&mut self, name: &'a str, ty: InferredType) {
        self.vars.insert(name, ty);
    }

    /// Insert a function signature. Convenience for builders.
    pub fn insert_func(&mut self, name: &'a str, sig: FuncSig) {
        self.funcs.insert(name, sig);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Unit tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    fn int(signed: bool, bits: u8) -> InferredType {
        InferredType::Int { signed, bits }
    }
    fn float(bits: u8) -> InferredType {
        InferredType::Float { bits }
    }

    // ── from_sce_type ───────────────────────────────────────────

    #[test]
    fn from_sce_type_covers_all_variants() {
        assert_eq!(InferredType::from_sce_type(&SceType::Uint8), int(false, 8));
        assert_eq!(InferredType::from_sce_type(&SceType::Int64), int(true, 64));
        assert_eq!(InferredType::from_sce_type(&SceType::Float32), float(32));
        assert_eq!(InferredType::from_sce_type(&SceType::Float64), float(64));
        assert_eq!(InferredType::from_sce_type(&SceType::Bool), InferredType::Bool);
        assert_eq!(InferredType::from_sce_type(&SceType::String), InferredType::Str);
        assert_eq!(InferredType::from_sce_type(&SceType::Bytes), InferredType::Bytes);
    }

    // ── join_arith ──────────────────────────────────────────────

    #[test]
    fn arith_untyped_int_and_concrete_float_promotes_to_float() {
        assert_eq!(join_arith(InferredType::UntypedInt, float(64)), float(64));
        assert_eq!(join_arith(float(32), InferredType::UntypedInt), float(32));
    }

    #[test]
    fn arith_untyped_int_and_concrete_int_adopts_concrete() {
        assert_eq!(join_arith(InferredType::UntypedInt, int(true, 32)), int(true, 32));
        assert_eq!(join_arith(int(false, 16), InferredType::UntypedInt), int(false, 16));
    }

    #[test]
    fn arith_two_untyped_ints_stay_untyped() {
        assert_eq!(
            join_arith(InferredType::UntypedInt, InferredType::UntypedInt),
            InferredType::UntypedInt
        );
    }

    #[test]
    fn arith_untyped_float_and_int_widens_to_float() {
        // UntypedFloat ⊔ Int{u, 16} → Float{bits: max(32, 16)} = Float{32}
        assert_eq!(
            join_arith(InferredType::UntypedFloat, int(false, 16)),
            float(32)
        );
        // UntypedFloat ⊔ Int{i, 64} → Float{bits: max(32, 64)} = Float{64}
        assert_eq!(join_arith(int(true, 64), InferredType::UntypedFloat), float(64));
    }

    #[test]
    fn arith_int_and_float_concrete_yields_float_min_32() {
        assert_eq!(join_arith(int(false, 8), float(64)), float(64));
        assert_eq!(join_arith(float(32), int(true, 16)), float(32));
    }

    #[test]
    fn arith_int_widening_prefers_signed() {
        // u16 ⊔ i32 → i32 (widen to 32, prefer signed)
        assert_eq!(
            join_arith(int(false, 16), int(true, 32)),
            int(true, 32)
        );
        // u32 ⊔ u64 → u64
        assert_eq!(
            join_arith(int(false, 32), int(false, 64)),
            int(false, 64)
        );
    }

    #[test]
    fn arith_float_widening_picks_max_bits() {
        assert_eq!(join_arith(float(32), float(64)), float(64));
        assert_eq!(join_arith(float(64), float(32)), float(64));
    }

    #[test]
    fn arith_unknown_propagates() {
        assert_eq!(
            join_arith(InferredType::Unknown, float(64)),
            InferredType::Unknown
        );
        assert_eq!(
            join_arith(int(true, 32), InferredType::Unknown),
            InferredType::Unknown
        );
    }

    #[test]
    fn arith_nonnumeric_with_numeric_is_unknown() {
        assert_eq!(
            join_arith(InferredType::Bool, int(true, 32)),
            InferredType::Unknown
        );
        assert_eq!(
            join_arith(float(64), InferredType::Str),
            InferredType::Unknown
        );
    }

    // ── join_int ────────────────────────────────────────────────

    #[test]
    fn int_two_untyped_ints_stay_untyped() {
        assert_eq!(
            join_int(InferredType::UntypedInt, InferredType::UntypedInt),
            InferredType::UntypedInt
        );
    }

    #[test]
    fn int_untyped_adopts_concrete() {
        assert_eq!(
            join_int(InferredType::UntypedInt, int(false, 32)),
            int(false, 32)
        );
    }

    #[test]
    fn int_bitwise_with_float_is_unknown() {
        assert_eq!(join_int(int(true, 32), float(64)), InferredType::Unknown);
        assert_eq!(join_int(InferredType::UntypedFloat, int(false, 8)), InferredType::Unknown);
    }

    #[test]
    fn int_widening_prefers_signed() {
        assert_eq!(
            join_int(int(false, 16), int(true, 8)),
            int(true, 16)
        );
    }

    // ── TypeCtx ─────────────────────────────────────────────────

    #[test]
    fn ctx_unknown_for_missing_var() {
        let ctx = TypeCtx::new();
        assert_eq!(ctx.lookup_var("nope"), InferredType::Unknown);
    }

    #[test]
    fn ctx_insert_and_lookup_roundtrip() {
        let mut ctx = TypeCtx::new();
        ctx.insert_var("celsius", float(64));
        assert_eq!(ctx.lookup_var("celsius"), float(64));
    }

    #[test]
    fn ctx_func_lookup() {
        let mut ctx = TypeCtx::new();
        ctx.insert_func(
            "temp_xform",
            FuncSig { params: vec![int(false, 16)], ret: float(64) },
        );
        assert!(ctx.lookup_func("temp_xform").is_some());
        assert_eq!(ctx.lookup_func("missing").is_none(), true);
    }
}
