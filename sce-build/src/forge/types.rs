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
use crate::forge::quantity::{NumericBaseType, Quantity, Rational, UnitTag};
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

    /// Physical-quantity-annotated numeric (NL→IR Item 4).
    ///
    /// Layers a `physical = raw * scale + offset` linear conversion over
    /// the inner `NumericBaseType`. Two `Quantity` values combine
    /// arithmetically iff their unit tags compare equal; mixing
    /// different units yields `Unknown` so a post-pass can surface
    /// the incompatibility as `validation/cross-kind-type-mismatch`.
    ///
    /// `Quantity` stays `Copy` because every component is `Copy`:
    /// `NumericBaseType` is plain-old-data, `Rational` is two `i64`s,
    /// and `UnitTag` is a process-wide interned `u16`.
    Quantity {
        base: NumericBaseType,
        scale: Rational,
        offset: Rational,
        unit: UnitTag,
    },
}

impl InferredType {
    /// Is this type a concrete or untyped integer? Looks through a
    /// `Quantity` annotation so `Quantity { base: Int{..}, .. }` still
    /// reports as integer-like.
    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Self::UntypedInt
                | Self::Int { .. }
                | Self::Quantity {
                    base: NumericBaseType::Int { .. },
                    ..
                }
        )
    }

    /// Is this type a concrete or untyped float? Looks through a
    /// `Quantity` annotation so `Quantity { base: Float{..}, .. }` still
    /// reports as float-like.
    pub fn is_float_like(&self) -> bool {
        matches!(
            self,
            Self::UntypedFloat
                | Self::Float { .. }
                | Self::Quantity {
                    base: NumericBaseType::Float { .. },
                    ..
                }
        )
    }

    /// Does any arithmetic operand of this type require floating-point
    /// arithmetic? True for any float-family type.
    pub fn is_arith_float(&self) -> bool {
        self.is_float_like()
    }

    /// Strip any `Quantity` wrapper to the raw numeric `InferredType`.
    /// `Quantity { base: Int{s,b}, .. }` → `Int{s,b}`; everything else
    /// passes through unchanged. Used by codegen sites that need the
    /// raw wire-level type without the unit annotation.
    pub fn strip_quantity(self) -> Self {
        match self {
            Self::Quantity { base, .. } => match base {
                NumericBaseType::Int { signed, bits } => Self::Int { signed, bits },
                NumericBaseType::Float { bits } => Self::Float { bits },
            },
            other => other,
        }
    }

    /// If this is a `Quantity`, return the conversion descriptor.
    pub fn quantity(self) -> Option<Quantity> {
        match self {
            Self::Quantity {
                base: _,
                scale,
                offset,
                unit,
            } => Some(Quantity {
                scale,
                offset,
                unit,
            }),
            _ => None,
        }
    }

    /// `true` when this is a `Quantity` variant.
    pub fn is_quantity(&self) -> bool {
        matches!(self, Self::Quantity { .. })
    }

    /// Map an `SceType` from the model layer to an inferred concrete type.
    ///
    /// This is the single entry point from the generator-layer type system
    /// into the inference layer. All `TypeCtx::vars` and `FuncSig` entries
    /// produced by generator code flow through this converter.
    pub fn from_sce_type(ty: &SceType) -> Self {
        match ty {
            SceType::Uint8 => Self::Int {
                signed: false,
                bits: 8,
            },
            SceType::Uint16 => Self::Int {
                signed: false,
                bits: 16,
            },
            SceType::Uint32 => Self::Int {
                signed: false,
                bits: 32,
            },
            SceType::Uint64 => Self::Int {
                signed: false,
                bits: 64,
            },
            SceType::Int8 => Self::Int {
                signed: true,
                bits: 8,
            },
            SceType::Int16 => Self::Int {
                signed: true,
                bits: 16,
            },
            SceType::Int32 => Self::Int {
                signed: true,
                bits: 32,
            },
            SceType::Int64 => Self::Int {
                signed: true,
                bits: 64,
            },
            SceType::Float32 => Self::Float { bits: 32 },
            SceType::Float64 => Self::Float { bits: 64 },
            SceType::Bool => Self::Bool,
            SceType::String => Self::Str,
            SceType::Bytes => Self::Bytes,
            // NL→IR Item C1 Path A: an enum-typed value's concrete
            // integer width is determined by the imported enum
            // document's `sce:underlying-type` — cross-doc
            // information not visible at the model layer. Map to
            // `Unknown` so the inference layer takes the conservative
            // "opaque" path; the cross-kind binding pass and Atomic 5
            // literal-width narrowing resolve the imported enum and
            // perform explicit typecheck against the declared
            // underlying type, independent of this inference path.
            // `Unknown` here means "this layer declines to claim the
            // type", not "type mismatch".
            SceType::Enum(_) => Self::Unknown,
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
    // Quantity handling sits at the top so unit-mismatch detection
    // doesn't fall through to the unit-stripped numeric joins below.
    // The lattice rules for Quantity:
    //
    // * `Quantity ⊔ Quantity`   — unit-equal: keep one quantity, joined
    //   base. unit-mismatch:    yields `Unknown` so a post-pass can
    //   surface `validation/cross-kind-type-mismatch`.
    // * `Quantity ⊔ Untyped`    — literal adopts the quantity (acts as
    //   a dimensionless multiplier in `raw * literal`-style expressions);
    //   unit annotation stays sticky.
    // * `Quantity ⊔ Concrete`   — explicit raw numeric drops the unit
    //   annotation. The author authored a typed bare numeric, so the
    //   result is the joined raw base. (Use cases like ARXML COMPU-METHOD
    //   evaluation pass raw bytes into `phys = raw * scale + offset` —
    //   that emission goes through the codegen accessor, not direct
    //   arithmetic in author expressions.)
    if matches!(left, Quantity { .. }) || matches!(right, Quantity { .. }) {
        return join_arith_quantity(left, right);
    }

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

        (
            Int {
                signed: s1,
                bits: b1,
            },
            Int {
                signed: s2,
                bits: b2,
            },
        ) => Int {
            signed: s1 || s2,
            bits: b1.max(b2),
        },
        (Int { bits: bi, .. }, Float { bits: bf }) | (Float { bits: bf }, Int { bits: bi, .. }) => {
            let _ = bi; // integer width ignored when unified with float
            Float { bits: bf.max(32) }
        }
        (Float { bits: b1 }, Float { bits: b2 }) => Float { bits: b1.max(b2) },

        // Mixing non-numeric with numeric is opaque — the user has written
        // a semantically invalid expression; don't silently paper over it.
        _ => Unknown,
    }
}

/// Quantity-aware arithmetic join. Called from [`join_arith`] when at
/// least one operand is `InferredType::Quantity`. Mismatched units
/// collapse to `Unknown`; the dedicated post-typing checker
/// (`forge::quantity_check`) re-walks the AST and produces the typed
/// `validation/cross-kind-type-mismatch` diagnostic.
fn join_arith_quantity(left: InferredType, right: InferredType) -> InferredType {
    use InferredType::*;
    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,

        // Both quantities — unit equality is the gate.
        (
            Quantity {
                base: b1,
                scale: s1,
                offset: o1,
                unit: u1,
            },
            Quantity {
                base: b2,
                scale: _s2,
                offset: _o2,
                unit: u2,
            },
        ) => {
            if u1 != u2 {
                Unknown
            } else {
                // Keep the left operand's conversion factors. Two
                // operand-level Quantity declarations on the same unit
                // are expected to match (the parser canonicalises both),
                // but if they drift the left one is the natural anchor
                // — the right is normally an interior subexpression.
                Quantity {
                    base: join_numeric_base(b1, b2),
                    scale: s1,
                    offset: o1,
                    unit: u1,
                }
            }
        }

        // Quantity ⊔ untyped literal — literal adopts the quantity.
        (Quantity { .. }, UntypedInt) | (UntypedInt, Quantity { .. }) => {
            if matches!(left, Quantity { .. }) {
                left
            } else {
                right
            }
        }
        (Quantity { .. }, UntypedFloat) | (UntypedFloat, Quantity { .. }) => {
            // Untyped float against an integer-backed quantity widens
            // to a float-backed quantity in the same unit; against a
            // float-backed quantity, retain the float backing.
            let q = if matches!(left, Quantity { .. }) {
                left
            } else {
                right
            };
            if let Quantity {
                base,
                scale,
                offset,
                unit,
            } = q
            {
                let base = match base {
                    NumericBaseType::Int { bits, .. } => {
                        NumericBaseType::Float { bits: bits.max(32) }
                    }
                    NumericBaseType::Float { bits } => NumericBaseType::Float { bits },
                };
                Quantity {
                    base,
                    scale,
                    offset,
                    unit,
                }
            } else {
                Unknown
            }
        }

        // Quantity ⊔ concrete numeric — strip the unit; explicit
        // typed-bare-numeric authorship means the user opted out of
        // unit checking at this site.
        (Quantity { base: bq, .. }, Int { signed, bits })
        | (Int { signed, bits }, Quantity { base: bq, .. }) => {
            let raw_q = match bq {
                NumericBaseType::Int {
                    signed: sq,
                    bits: bsq,
                } => Int {
                    signed: sq,
                    bits: bsq,
                },
                NumericBaseType::Float { bits } => Float { bits },
            };
            join_arith(raw_q, Int { signed, bits })
        }
        (Quantity { base: bq, .. }, Float { bits })
        | (Float { bits }, Quantity { base: bq, .. }) => {
            let raw_q = match bq {
                NumericBaseType::Int {
                    signed: sq,
                    bits: bsq,
                } => Int {
                    signed: sq,
                    bits: bsq,
                },
                NumericBaseType::Float { bits: bq_bits } => Float { bits: bq_bits },
            };
            join_arith(raw_q, Float { bits })
        }

        // Any non-numeric combinations involving a Quantity are opaque.
        _ => Unknown,
    }
}

fn join_numeric_base(a: NumericBaseType, b: NumericBaseType) -> NumericBaseType {
    match (a, b) {
        (
            NumericBaseType::Int {
                signed: s1,
                bits: b1,
            },
            NumericBaseType::Int {
                signed: s2,
                bits: b2,
            },
        ) => NumericBaseType::Int {
            signed: s1 || s2,
            bits: b1.max(b2),
        },
        (NumericBaseType::Float { bits: f1 }, NumericBaseType::Float { bits: f2 }) => {
            NumericBaseType::Float { bits: f1.max(f2) }
        }
        (NumericBaseType::Int { bits: _bi, .. }, NumericBaseType::Float { bits: bf })
        | (NumericBaseType::Float { bits: bf }, NumericBaseType::Int { bits: _bi, .. }) => {
            NumericBaseType::Float { bits: bf.max(32) }
        }
    }
}

/// Integer-only join, used for bitwise ops and shifts. Non-integer operands
/// poison the result to `Unknown` (bitwise on floats/bools/strings is nonsense).
///
/// `Quantity` operands strip their unit annotation here: bit-twiddling on
/// a unit-annotated raw value carries no physical interpretation, so the
/// caller is implicitly working at the raw layer (typical for codec
/// flag-bit extraction). Two `Quantity` operands with **different** units
/// still collapse to `Unknown` so the post-pass can flag the mismatch.
pub fn join_int(left: InferredType, right: InferredType) -> InferredType {
    use InferredType::*;
    // Unit mismatch in bitwise context surfaces the same way as in
    // arith — `Unknown`, with the post-pass producing the diagnostic.
    if let (Quantity { unit: u1, .. }, Quantity { unit: u2, .. }) = (left, right) {
        if u1 != u2 {
            return Unknown;
        }
    }
    // Strip Quantity wrappers so the underlying int joins reach the
    // ordinary `Int` paths below. Bitwise ops carry no unit semantics.
    let left = left.strip_quantity();
    let right = right.strip_quantity();

    match (left, right) {
        (Unknown, _) | (_, Unknown) => Unknown,

        (UntypedInt, UntypedInt) => UntypedInt,

        (UntypedInt, Int { signed, bits }) | (Int { signed, bits }, UntypedInt) => {
            Int { signed, bits }
        }

        (
            Int {
                signed: s1,
                bits: b1,
            },
            Int {
                signed: s2,
                bits: b2,
            },
        ) => Int {
            signed: s1 || s2,
            bits: b1.max(b2),
        },

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
    /// Per-identifier element type for indexable containers — populated
    /// by RFC §5.A `<sce:const name=… type="array<elem, N>">` so that
    /// `CRC16_TABLE[idx]` can be inferred as `elem` instead of falling
    /// through to `Unknown`. Without this, Kotlin's narrow-unsigned
    /// arithmetic widening cannot insert `.toInt()` at the index access
    /// (the index node has no parent-type signal otherwise).
    pub array_elems: HashMap<&'a str, InferredType>,
    /// RFC c7-wildcard W-project: when `true`, [`infer_types`] projects a
    /// `Str` argument that flows into a `bytes` function parameter into a
    /// borrowed `bytes` view (an [`ExprKind::BytesView`] node), so a
    /// bounded-string element field (`entry.pattern`) lowers to each
    /// backend's byte-view idiom at the call site (Q-W-5 (a) lock). Set
    /// only by the algorithm renderer — the projection's per-backend emit
    /// assumes the algorithm-kind string representation (a codec
    /// `&str` / `std::string` / `char[N]+_len` field), so leaving it
    /// `false` everywhere else keeps codec / procedure expression
    /// inference byte-identical to pre-W-project.
    ///
    /// [`ExprKind::BytesView`]: crate::forge::expr::ExprKind::BytesView
    /// [`infer_types`]: crate::forge::expr::infer_types
    pub project_str_args_as_bytes_view: bool,
    /// RFC c7-wildcard W-project (§8 Smell A fix): qualified member path
    /// (`"entry.pattern"`) → its **length sibling member name**
    /// (`"pattern_len"`). Populated by the algorithm renderer from the
    /// element-type codec's `length_field` SSOT (or the auto `<field>_len`
    /// for a tail/fixed `bytes` field). The `BytesView` projection consults
    /// this so the C11 borrowed view references the actual length member
    /// instead of guessing `<field>_len`. Empty for every other kind and
    /// for algorithms with no byte-addressable element field.
    pub member_len_fields: HashMap<&'a str, &'a str>,
}

impl<'a> TypeCtx<'a> {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            funcs: HashMap::new(),
            array_elems: HashMap::new(),
            project_str_args_as_bytes_view: false,
            member_len_fields: HashMap::new(),
        }
    }

    /// Register a member path's C11 length sibling (W-project §8 Smell A).
    pub fn insert_member_len_field(&mut self, path: &'a str, len_member: &'a str) {
        self.member_len_fields.insert(path, len_member);
    }

    /// Look up a member path's C11 length sibling member name, if recorded.
    pub fn lookup_member_len_field(&self, path: &str) -> Option<&'a str> {
        self.member_len_fields.get(path).copied()
    }

    /// Look up an identifier's type. Returns `Unknown` if absent so that
    /// inference can still proceed on expressions mixing known and unknown
    /// identifiers.
    pub fn lookup_var(&self, name: &str) -> InferredType {
        self.vars
            .get(name)
            .copied()
            .unwrap_or(InferredType::Unknown)
    }

    /// Look up a function signature. Returns `None` if absent.
    pub fn lookup_func(&self, name: &str) -> Option<&FuncSig> {
        self.funcs.get(name)
    }

    /// Look up the element type of an indexable container by name.
    /// Returns `None` for unknown identifiers — caller falls through to
    /// `Unknown`-element semantics (mirrors the `lookup_var` contract).
    pub fn lookup_array_elem(&self, name: &str) -> Option<InferredType> {
        self.array_elems.get(name).copied()
    }

    /// Insert a typed identifier. Convenience for builders.
    pub fn insert_var(&mut self, name: &'a str, ty: InferredType) {
        self.vars.insert(name, ty);
    }

    /// Insert a function signature. Convenience for builders.
    pub fn insert_func(&mut self, name: &'a str, sig: FuncSig) {
        self.funcs.insert(name, sig);
    }

    /// Insert an indexable-container element type for `name`.
    /// Used by the algorithm renderer to register `<sce:const>` arrays
    /// so `name[idx]` gets typed as `elem`.
    pub fn insert_array_elem(&mut self, name: &'a str, elem: InferredType) {
        self.array_elems.insert(name, elem);
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
        assert_eq!(
            InferredType::from_sce_type(&SceType::Bool),
            InferredType::Bool
        );
        assert_eq!(
            InferredType::from_sce_type(&SceType::String),
            InferredType::Str
        );
        assert_eq!(
            InferredType::from_sce_type(&SceType::Bytes),
            InferredType::Bytes
        );
    }

    // ── join_arith ──────────────────────────────────────────────

    #[test]
    fn arith_untyped_int_and_concrete_float_promotes_to_float() {
        assert_eq!(join_arith(InferredType::UntypedInt, float(64)), float(64));
        assert_eq!(join_arith(float(32), InferredType::UntypedInt), float(32));
    }

    #[test]
    fn arith_untyped_int_and_concrete_int_adopts_concrete() {
        assert_eq!(
            join_arith(InferredType::UntypedInt, int(true, 32)),
            int(true, 32)
        );
        assert_eq!(
            join_arith(int(false, 16), InferredType::UntypedInt),
            int(false, 16)
        );
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
        assert_eq!(
            join_arith(int(true, 64), InferredType::UntypedFloat),
            float(64)
        );
    }

    #[test]
    fn arith_int_and_float_concrete_yields_float_min_32() {
        assert_eq!(join_arith(int(false, 8), float(64)), float(64));
        assert_eq!(join_arith(float(32), int(true, 16)), float(32));
    }

    #[test]
    fn arith_int_widening_prefers_signed() {
        // u16 ⊔ i32 → i32 (widen to 32, prefer signed)
        assert_eq!(join_arith(int(false, 16), int(true, 32)), int(true, 32));
        // u32 ⊔ u64 → u64
        assert_eq!(join_arith(int(false, 32), int(false, 64)), int(false, 64));
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
        assert_eq!(
            join_int(InferredType::UntypedFloat, int(false, 8)),
            InferredType::Unknown
        );
    }

    #[test]
    fn int_widening_prefers_signed() {
        assert_eq!(join_int(int(false, 16), int(true, 8)), int(true, 16));
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

    // ── Quantity (NL→IR Item 4) ─────────────────────────────────

    fn celsius_q_i8() -> InferredType {
        InferredType::Quantity {
            base: NumericBaseType::Int {
                signed: true,
                bits: 8,
            },
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::from_int(-40),
            unit: UnitTag::intern("celsius-types-test"),
        }
    }

    fn kelvin_q_i8() -> InferredType {
        InferredType::Quantity {
            base: NumericBaseType::Int {
                signed: true,
                bits: 8,
            },
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::zero(),
            unit: UnitTag::intern("kelvin-types-test"),
        }
    }

    #[test]
    fn quantity_is_integer_or_float_like_per_base() {
        let q_i = celsius_q_i8();
        assert!(q_i.is_integer_like());
        assert!(!q_i.is_float_like());

        let q_f = InferredType::Quantity {
            base: NumericBaseType::Float { bits: 32 },
            scale: Rational::one(),
            offset: Rational::zero(),
            unit: UnitTag::intern("hz-types-test"),
        };
        assert!(!q_i_or_f_swap_check(q_i, q_f));
        assert!(q_f.is_float_like());
        assert!(!q_f.is_integer_like());
    }

    fn q_i_or_f_swap_check(q_i: InferredType, q_f: InferredType) -> bool {
        q_i.is_float_like() && q_f.is_integer_like()
    }

    #[test]
    fn quantity_strip_returns_underlying_numeric_type() {
        let q = celsius_q_i8();
        assert_eq!(q.strip_quantity(), int(true, 8));

        let q_f = InferredType::Quantity {
            base: NumericBaseType::Float { bits: 64 },
            scale: Rational::one(),
            offset: Rational::zero(),
            unit: UnitTag::intern("strip-types-test"),
        };
        assert_eq!(q_f.strip_quantity(), float(64));

        // Non-quantity passes through.
        assert_eq!(int(false, 16).strip_quantity(), int(false, 16));
    }

    #[test]
    fn arith_quantity_same_unit_keeps_quantity() {
        let a = celsius_q_i8();
        let b = celsius_q_i8();
        match join_arith(a, b) {
            InferredType::Quantity { unit, .. } => {
                assert_eq!(unit.as_str(), "celsius-types-test");
            }
            other => panic!("expected Quantity, got {other:?}"),
        }
    }

    #[test]
    fn arith_quantity_different_units_collapses_to_unknown() {
        let a = celsius_q_i8();
        let b = kelvin_q_i8();
        assert_eq!(join_arith(a, b), InferredType::Unknown);
        assert_eq!(join_arith(b, a), InferredType::Unknown);
    }

    #[test]
    fn arith_quantity_with_untyped_int_keeps_quantity() {
        let q = celsius_q_i8();
        // celsius * 9 — literal adopts the quantity.
        let r = join_arith(q, InferredType::UntypedInt);
        assert!(matches!(r, InferredType::Quantity { .. }));
        let r = join_arith(InferredType::UntypedInt, q);
        assert!(matches!(r, InferredType::Quantity { .. }));
    }

    #[test]
    fn arith_quantity_with_untyped_float_promotes_base() {
        let q = celsius_q_i8();
        // celsius * 0.5 — int-backed quantity widens to float-backed.
        let r = join_arith(q, InferredType::UntypedFloat);
        match r {
            InferredType::Quantity { base, .. } => {
                assert_eq!(base, NumericBaseType::Float { bits: 32 });
            }
            other => panic!("expected float-backed Quantity, got {other:?}"),
        }
    }

    #[test]
    fn arith_quantity_with_concrete_int_strips_unit() {
        let q = celsius_q_i8();
        // celsius * (i32) — explicit raw int authorship drops the unit.
        let r = join_arith(q, int(true, 32));
        assert_eq!(r, int(true, 32));
    }

    #[test]
    fn arith_quantity_with_concrete_float_strips_unit() {
        let q = celsius_q_i8();
        let r = join_arith(q, float(64));
        assert_eq!(r, float(64));
    }

    #[test]
    fn int_join_quantity_unit_mismatch_is_unknown() {
        let a = celsius_q_i8();
        let b = kelvin_q_i8();
        assert_eq!(join_int(a, b), InferredType::Unknown);
    }

    #[test]
    fn int_join_quantity_same_unit_strips_to_int() {
        let a = celsius_q_i8();
        let b = celsius_q_i8();
        // Bitwise on unit-annotated raws strips the annotation; the
        // result is the underlying int join.
        assert_eq!(join_int(a, b), int(true, 8));
    }

    #[test]
    fn ctx_func_lookup() {
        let mut ctx = TypeCtx::new();
        ctx.insert_func(
            "temp_xform",
            FuncSig {
                params: vec![int(false, 16)],
                ret: float(64),
            },
        );
        assert!(ctx.lookup_func("temp_xform").is_some());
        assert!(ctx.lookup_func("missing").is_none());
    }
}
