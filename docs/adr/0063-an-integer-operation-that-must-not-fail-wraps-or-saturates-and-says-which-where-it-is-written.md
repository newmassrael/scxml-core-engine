# ADR 0063 — An integer operation that must not fail wraps or saturates, and says which where it is written

- Status: Accepted (2026-10-29, by the owner, with the order to proceed); being implemented in the order below.
  Requested by a downstream consumer whose supervision deadline and sampler period read a 32-bit tick counter that
  wraps; designed under the owner's rule: the most long-term correct structure, textbook in its foundations, cost
  ignored
- Date: 2026-10-29
- Scope: the forge expression language (`sce-build/src/forge/expr.rs`: a new node, its parse, typing and refusals);
  the interval analysis (`forge::int_ranges`); the build-time fold (`forge::const_fold`); the lowering on the seven
  generated backends and the Interpreter's ECMAScript; a row per backend in each forge runtime
  (`backends/*/forge-runtime`, `sce-build/src/forge/static_js/sce_static.js`); conformance fixtures under
  `tests/forge/`; SCE_FORGE.md §3.4.2 and docs/SCE_ACCEPTED_SUBSET.md §3.4.1. **Nothing an accepted document says
  today changes meaning**: `+ - *` and unary `-` keep the integer arithmetic contract of SCE_FORGE.md §3.4.1.
- Related: SCE_FORGE.md §3.4.1 (the integer arithmetic contract, `may-fail`), ADR 0047 (an enum's two conversions,
  the precedent for a vocabulary name with its own typing rule), docs/SCE_ACCEPTED_SUBSET.md §3.4.1 (the forge
  expression vocabulary) and §2.15 (a `sce-static` machine checks its integer operations)

## Context

The request, as made: two total functions over a 32-bit tick counter that wraps,

```
elapsed(now, last, timeout) -> bool   = now.wrapping_sub(last) >= timeout
until_due(now, last, period) -> u32   = period.saturating_sub(now.wrapping_sub(last))
```

and, for them, wrapping and saturating `add`, `sub` and `mul` on the unsigned widths, lowered to each backend's
equivalent, with a return type that is not a `Result` when the body cannot fail.

The second half already holds. SCE_FORGE.md §3.4.1 makes every integer `+ - * / %` and unary `-` a partial
function of the mathematical integers: a result outside the declared width is not a value, and an algorithm either
proves (by an interval analysis over its body) that none of its operations reaches one, or declares `may-fail` and
hands the failure to its caller. An algorithm that does not declare `may-fail` is generated with a plain return type
today (`stdlib/mesh/outbound_overflows.scxml` is `pub fn outbound_overflows(depth: u32, max_pending: u32) -> bool`).
No marker is needed: the analysis is the proof, and it runs once, before any backend renders.

The first half is missing, and no rewrite of the body supplies it:

- `now - last` is an overflow exactly when the counter has wrapped, which is the case the function exists for.
- A guard does not help. The analysis keeps one interval per value and no relation between two values, so
  `now - last` under `now >= last` is not proven (SCE_FORGE.md §3.4.1, "Enforced"); and the branch the guard leaves,
  `(MAX - last) + now + 1`, overflows where `now` is `MAX` and `last` is 0 unless written with a further guard.
- `may-fail` admits the subtraction and turns a predicate that cannot fail into `Result<bool, AlgorithmError>`,
  whose error arm is dead code every caller must still handle.
- Bitwise operations wrap at their width (§3.4.1, "At build time"), but a modular subtraction built from them is a
  ripple-carry adder written by hand.

The arithmetic the request needs is arithmetic modulo 2^32, and saturation at the bounds: two algebras other than
the one `-` denotes. The language has to be able to say them.

## Decision

### 1. Three algebras of one width

Let `T` be an integer type of width `W` with range `[m, M]`: `[0, 2^W − 1]` for an unsigned type, `[−2^(W−1),
2^(W−1) − 1]` for a signed one. For an operation `op` among `add`, `sub`, `mul` and `neg`, let `e` be its exact
result in ℤ. ℤ is closed under these four, so `e` always exists. Each operation then has one of three meanings,
and each meaning is a reduction `R` of `e` into `T`:

| Rule | `R(e)` | Defined for |
|---|---|---|
| checked (`+ - *`, unary `-`; SCE_FORGE.md §3.4.1) | `e` if `m ≤ e ≤ M`, otherwise no value: the failure `overflow` | every input, partial |
| wrapping | the unique `r ∈ [m, M]` with `r ≡ e (mod 2^W)` | every input, total |
| saturating | `max(m, min(M, e))` | every input, total |

Wrapping is the quotient map ℤ → ℤ/2^Wℤ, choosing as representative the element of `T`'s range. For a signed type
that representative is the two's-complement one. Because the map is a ring homomorphism, wrapping operations
compose: `wrappingAdd(wrappingAdd(a, b), c)` is `R(a + b + c)`, so a backend may compute in any wider type and reduce
once. Saturation is not a homomorphism and does not associate: `saturatingAdd(saturatingAdd(M, 1), −1)` is `M − 1`,
while `saturatingAdd(M, saturatingAdd(1, −1))` is `M`. **No pass may reassociate, fuse or fold a saturating
operation except by computing it as written**, one reduction per operation; the build-time fold and every backend
obey this.

Division and remainder are excluded on principle, not for lack of time. ℤ is not closed under them: `a / 0` has no
exact result to reduce, so `R ∘ div` is still partial, and a "wrapping division" would be a fourth algebra that can
fail, which is what the family exists to avoid. Their one overflow (signed `MIN / −1`) stays the checked one.

Shifts and bitwise operations keep their present meaning (they wrap at their width).

### 2. The surface: seven names of the expression vocabulary

| Name | Operands | Value |
|---|---|---|
| `wrappingAdd(a, b)` | two | `R_wrap(a + b)` |
| `wrappingSub(a, b)` | two | `R_wrap(a − b)` |
| `wrappingMul(a, b)` | two | `R_wrap(a × b)` |
| `wrappingNeg(a)` | one | `R_wrap(−a)` |
| `saturatingAdd(a, b)` | two | `R_sat(a + b)` |
| `saturatingSub(a, b)` | two | `R_sat(a − b)` |
| `saturatingMul(a, b)` | two | `R_sat(a × b)` |
| `saturatingNeg(a)` | one, **signed `T` only** | `R_sat(−a)` |

The forge expression vocabulary (docs/SCE_ACCEPTED_SUBSET.md §3.4.1) grows from four names to twelve. The names are
camelCase because the expression language is an ECMAScript subset whose own multi-word names are (`Math.imul`,
`toNumber`); the backends never see them.

`saturatingNeg` of an unsigned value is 0 for every input. A function that ignores its operand is an author's
mistake rather than an operation, so it is refused where it is written, as Rust's standard library omits it.

They are calls rather than operators (Zig's `+%`, `-|`, Swift's `&+`) because a document's expressions stay
ECMAScript: the Interpreter's script engine, the Lua translation seam, every tokenizer that reads an `expr=`, and an
author's editor all parse them today, and an operator the grammar does not have would break each of them.

The rule belongs to the operation, not to the type or the variable, because the meaning of an operation must be
readable where it is written. A modular *type* (Ada's `type Tick is mod 2**32`, Rust's `Wrapping<u32>`) changes what
every `+` on that variable means at a distance, and its order is the source of the classic wrap bug: `<` on the
representatives of ℤ/2^Wℤ is not an order of the ring (RFC 1982 defines serial-number comparison for exactly that
reason). An operation-level rule is the primitive a type-level one could be defined from later; the reverse is not
true.

### 3. Typing

The rules are judged once, in inference, before any backend renders, so every backend and `check` refuse the same
documents.

1. **Arity.** Two operands, or one for the negations: otherwise `expression/argument-count-mismatch`.
2. **Integers only.** Each operand is an integer. A real, a boolean, a string, a byte string, a value of an enum, a
   record, a list or an optional value not proven present is `expression/type-mismatch`.
3. **One type.** The operation's type `T` is its operands' type. Two operands of declared types must have the same
   type, the same width and the same signedness (`wrappingAdd` of a `uint8` and a `uint32` has no single modulus;
   the author stores one into a local of the other's type first, which the integer contract then checks).
   Otherwise `expression/type-mismatch`, naming both types.
4. **A literal takes `T`.** A literal operand takes the other operand's type and must fit it, as everywhere:
   `wrappingSub(x, 300)` on a `uint8` is `expression/literal-out-of-range`, and so is `wrappingAdd(u, −1)` on an
   unsigned `u`. The author writes `wrappingSub(u, 1)`.
5. **A width is declared, never inferred from the place.** An operation whose operands are all literals, or are
   all computed from literals only, has no `T`, and is `expression/type-mismatch`. The width is the modulus, part of
   the operation's meaning, and the integer contract holds an operation at its own width, not at the width of the
   place its value lands in. A modulus chosen by context would change the value of `wrappingAdd(200, 100)` with the
   variable that receives it.
6. **The result is a value of `T`.** It is stored, compared, returned and passed as any value of `T` is. A store
   into a narrower type is the integer contract's (proven, or checked in a `may-fail` body). In float context the
   value converts as an integer does; the operation itself is never made in reals.
7. **`saturatingNeg` takes a signed `T`** (section 2): an unsigned one is `expression/type-mismatch`.

Each refusal names the operation as the author spelled it.

### 4. Where the names are provided

In every forge expression (every `sce:kind`), and in every expression of a `datamodel="sce-static"` statechart,
because both are lowered by the one expression layer to the same emitters. In a `may-fail` algorithm and in a
`sce-static` machine the operation stays total: it is never rewritten to a checked one and never raises
`error.execution`. Its operands are what they are: `wrappingAdd(a + 1, b)` checks `a + 1` by the contract and wraps the
sum. A statechart with another data model is not affected.

A document may not declare a callable (an import alias, an `<sce:helper>`, a stateful import's method name) spelled
as a vocabulary name. Today the vocabulary wins silently over a declaration of the same name; the rule is held for
all twelve names, not only the new ones, and the implementation measures the present behaviour before choosing the
code it reports.

### 5. The interval analysis

`forge::int_ranges` gains a transfer function per rule, sound and exact on intervals (an abstract interpretation in
the sense of Cousot and Cousot, over the interval domain). With `[lo, hi]` the exact interval of `e` (the transfer
`+ − ×` the checked operation uses, `[−hi, −lo]` for a negation):

- **saturating**: `[max(m, min(M, lo)), max(m, min(M, hi))]`. Clamping is monotone, so this is the exact image.
- **wrapping**: with `k_lo = ⌊(lo − m) / 2^W⌋` and `k_hi = ⌊(hi − m) / 2^W⌋`, the interval `[lo − k·2^W, hi − k·2^W]`
  when `k_lo = k_hi = k` (the whole range is moved by one multiple of the modulus, and when `k = 0` nothing wraps),
  and `[m, M]` otherwise. This is the wrapped-interval treatment of Navas et al. (APLAS 2012) and Gange et al.
  (TOPLAS 2015) restricted to the domain the analysis keeps; it is exact for a range that does not cross a wrap
  point, and sound for one that does.

Neither rule ever records a hazard, since neither fails; a hazard inside an operand is still reported where the
operand is. The analysis computes in `i128`. An exact product of two 64-bit unsigned bounds exceeds `i128`, so the
transfer computes its corners saturating at `i128`'s bounds, which keeps the result sound because every use clamps
it to `T` or reports it. The implementation measures whether the checked `×` already overflows there and fixes it
in the same commit if so.

### 6. The build-time fold

`forge::const_fold` evaluates the seven names as their definitions say: exactly in `i128`, or in `u128` for an
unsigned 64-bit product, then reduced into `T`. A `<sce:const>` table may therefore be built with wrapping arithmetic
(a linear congruential generator's sequence, an FNV-1a basis), which today it can build only from shifts.
`algorithm/const-integer-failure` is never raised by one of them.

### 7. The node

The parser turns a call of one of the seven names into one node, `ExprKind::Total { op: TotalOp, rule: OverflowRule,
left, right }` (`TotalOp`: `Add`, `Sub`, `Mul`, `Neg`; `OverflowRule`: `Wrapping`, `Saturating`), rather than leaving a
`Call` that each pass recognises by name. Every walk of the tree matches every node kind (`TypedExpr::children`), so
a new kind is decided by every pass and every emitter or fails to compile. A call by name would reach, for example,
the build-time fold as a call, which the fold refuses, and the import resolver as a function the document does not
declare. The node is not `ExprKind::Checked` with a third rule: `Checked` is a rewrite the pipeline applies to the
author's operators in a `may-fail` body, just before emission, while `Total` is what the author wrote and must be
visible to inference, the analysis and the fold from the first pass on.

### 8. Lowering

One family per runtime, named beside the checked family it mirrors, with one function per width. Rust's standard
library is that runtime for Rust.

| Backend | Wrapping | Saturating | Why not the native operator |
|---|---|---|---|
| Rust | `u32::wrapping_sub(a, b)` (fully qualified, so a literal left operand is typed) | `u32::saturating_sub(a, b)` | — (they are the native operations, and `const fn`) |
| C11 | `sce_forge_wrapping_sub_u32(a, b)` | `sce_forge_saturating_sub_u32(a, b)` | C promotes `uint8_t`/`uint16_t` to `int`, so `uint16_t × uint16_t` can overflow `int` (undefined, C17 6.5p5); signed overflow is undefined; an out-of-range conversion to a signed type is implementation-defined (6.3.1.3p3). The helpers compute in the unsigned type of the width and map back without either |
| C++ | `SCE::Forge::Wrapping::sub(a, b)` | `SCE::Forge::Saturating::sub(a, b)` | as C; the conversion back is defined only from C++20 (P1236R1), and the interface compiles at C++17 (ADR 0059) |
| Go | `scealgorithm.WrappingSubUint32(a, b)` | `scealgorithm.SaturatingSubUint32(a, b)` | Go defines the wrap for both signednesses, but an expression of two constants is folded at compile time and refused when it overflows; one family keeps the emitter uniform with the checked one |
| Kotlin | `com.sce.forge.runtime.SceWrapping.sub(a, b)` | `com.sce.forge.runtime.SceSaturating.sub(a, b)` | `Byte`, `Short`, `UByte` and `UShort` arithmetic is made in `Int` and must be truncated back |
| Python | `sce_algorithm.U32.wrapping_sub(a, b)` | `sce_algorithm.U32.saturating_sub(a, b)` | Python's integers do not overflow |
| Swift | `Wrapping.sub(a, b)` (`&-` over `FixedWidthInteger`) | `Saturating.sub(a, b)` | the standard library has no saturating operation |
| Interpreter (ECMAScript) | `SceStatic.U32.wrappingSub(a, b)` | `SceStatic.U32.saturatingSub(a, b)` | every integer is a Number or a BigInt; the runtime computes exactly and reduces (`wrap` already reduces bitwise results) |

The helpers are the reviewed and unit-tested place for each width: every runtime's own test suite holds each
function at `m`, `M`, 0, 1 and −1 against the definition. Their correctness arguments are the standard ones (Warren,
*Hacker's Delight*, §2-13, for the overflow tests a saturating helper of the widest type needs; CERT INT30-C and
INT32-C, which the checked helpers already cite).

The Interpreter keeps its present limit for 64-bit values outside the safe-integer range: a BigInt is a value only
inside the expression that computes it. A wrapping result there is the same as a bitwise one is today, and is not a
new limit.

### 9. Conformance

- `algorithm_total_arith_{u8,u16,u32,u64,i8,i16,i32,i64}`: one fixture per type, `(a: T, b: T, op: uint8) -> T`, each
  `op` one of the seven operations (six for an unsigned `T`), with **no `may-fail`**, so each backend's plain return
  type is part of what passes. The cases are each type's `m`, `M`, 0, 1, −1 and their neighbours in every pair the
  operation makes distinct: the wrap point, the saturation point on each side, and, for `mul`, a product past `2^W` but
  within `2^(2W)`. That last case is the one that separates computing in the type from computing in `int` in C. The
  expected values are computed from the definition in section 1 by arbitrary-precision arithmetic, independently of
  every backend.
- `algorithm_tick_elapsed` and `algorithm_tick_until_due`: the two functions of the request, over the request's cases
  (a wrap of the counter, a timeout of 0 and of 2^31 − 1, saturation at 0).
- A `<sce:const>` table built by a wrapping linear congruential generator, read by an algorithm.
- A `sce-static` statechart whose `uint8` counter steps past 255 with `wrappingAdd` and down past 0 with
  `saturatingSub`, run on every engine and the Interpreter.
- Unit tests of the analysis (a wrapping subtraction of two parameters proven in `uint32`, the exact wrapped interval
  of a range that does not cross, the whole type for one that does, a saturating result's clamp) and of every
  refusal in section 3.

## Order of landing

1. This ADR.
2. The node, its parse, typing and refusals; the analysis and the fold; SCE_FORGE.md §3.4.2 and the subset's
   vocabulary. Every emitter refuses the node by name (`generate/unsupported-feature`) until its own step.
3. One commit per target, each with its runtime row and the runtime's own tests: Rust, C++, C11, Go, Kotlin, Python,
   Swift, the Interpreter.
4. The conformance fixtures of section 9, on every backend at once.
5. The reservation of the vocabulary's names (section 4), after measuring what a colliding declaration does today.

## Consequences

- A document can say modular and saturating arithmetic. A function that cannot fail keeps a plain return type, and
  the consumer's two functions are `-> bool` and `-> u32` on every backend.
- The forge vocabulary has twelve names. An author's callable named like one of them is refused rather than
  silently shadowed.
- The interval analysis is no weaker: the new transfers are exact where nothing wraps, and every old transfer is
  unchanged.
- Each runtime grows two families of one function per width. They are small, total and free of branches the language
  leaves undefined, and they are tested where they live.
- `+ - *` still mean what §3.4.1 says. Choosing wrapping is a visible act at each operation, and a reviewer reads it
  where it is written.

## Alternatives not taken

- **Modular and saturating types** (Ada modular types, Rust `Wrapping<T>` / `Saturating<T>`). Rejected for section
  2's reason: the meaning of an operator would depend on a declaration elsewhere, and the order on a modular type
  invites the comparison bug RFC 1982 exists to prevent. They remain definable over this primitive.
- **Operators** (Zig `+%` `-%` `*%` `+|` `-|` `*|`, Swift `&+` `&-` `&*`). Rejected: not ECMAScript, so every parser of
  a document's expressions would have to change, and the Interpreter's engine would refuse them.
- **A relational abstract domain** (difference-bound matrices, octagons; Miné 2006), so that `now − last` under
  `now >= last` is proven. Orthogonal and worth having, but it does not give modular arithmetic: the branch a guard
  leaves still has to compute the wrapped difference. Not decided here.
- **Proving totality and dropping `Result` by a marker.** Unnecessary: §3.4.1 already decides the return type from
  `may-fail`, and the analysis is the proof.
- **Wrapping and saturating division.** Rejected on section 1's principle: they remain partial.
- **Unsigned widths only**, as the request scoped it. The definition does not depend on signedness, the backends that
  need helpers need them most for signed types (C, C++), and a family that stops at unsigned would be extended later
  under a second review. Signed types are included; the consumer's unsigned use is a subset.

## References

- ISO/IEC 9899:2018 (C17) §6.2.5p9 (unsigned arithmetic is reduced modulo 2^W), §6.3.1.1 (integer promotions),
  §6.3.1.3p3 (conversion to a signed type), §6.5p5 (an exceptional condition is undefined); ISO/IEC 9899:2024 (C23)
  §7.20 `<stdckdint.h>` (`ckd_add`, `ckd_sub`, `ckd_mul`).
- ISO/IEC 14882:2020 [conv.integral] and P1236R1, *Alternative Wording for P0907R4 Signed Integers are Two's
  Complement*.
- Rust RFC 560, *Integer Overflow*; `core` `wrapping_*`, `saturating_*`, `core::num::Wrapping`, `core::num::Saturating`.
- The Go Programming Language Specification, "Integer overflow".
- The Swift Programming Language, "Advanced Operators: Overflow Operators".
- The Java Language Specification §15.17.1 and §15.18.2 (the JVM arithmetic Kotlin inherits).
- Ada 2012 Reference Manual §3.5.4, modular types.
- The Zig Language Reference, wrapping and saturating operators.
- LLVM Language Reference, `nuw`/`nsw` and `llvm.uadd.sat` / `llvm.sadd.sat` / `llvm.usub.sat` / `llvm.ssub.sat`.
- ECMA-262, `BigInt.asUintN`, `BigInt.asIntN`, `Math.imul`.
- R. Elz, R. Bush, RFC 1982, *Serial Number Arithmetic*, 1996.
- H. S. Warren, Jr., *Hacker's Delight*, 2nd ed., 2012, §2-13 "Overflow Detection".
- P. Cousot, R. Cousot, "Abstract Interpretation: A Unified Lattice Model for Static Analysis of Programs by
  Construction or Approximation of Fixpoints", POPL 1977.
- J. A. Navas, P. Schachte, H. Søndergaard, P. J. Stuckey, "Signedness-Agnostic Program Analysis: Precise Integer
  Bounds for Low-Level Code", APLAS 2012.
- G. Gange, J. A. Navas, P. Schachte, H. Søndergaard, P. J. Stuckey, "Interval Analysis and Machine Arithmetic: Why
  Signedness Ignorance Is Bliss", ACM TOPLAS 37(1), 2015.
- A. Miné, "The Octagon Abstract Domain", Higher-Order and Symbolic Computation 19(1), 2006.
- W. Dietz, P. Li, J. Regehr, V. Adve, "Understanding Integer Overflow in C/C++", ICSE 2012.
- ITU-T G.191, Software Tool Library, basic operators (`add`, `sub`, `L_mult` with saturation): the DSP precedent for
  saturation as an operation rather than a type.
- SEI CERT C Coding Standard, INT30-C and INT32-C.

## Questions left open for the owner

1. **A modular conversion.** `wrappingAdd` takes one type, so a signed delta added to an unsigned counter needs a
   conversion that wraps (`R_wrap` applied across types). The expression language has no syntax for a target type in
   a call. A conversion is the natural next member of the family; it is not decided here.
2. **A relational domain** for the analysis (Alternatives), which would let a guard prove `q − p` under `q >= p`.
