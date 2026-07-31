# RFC — EventSchema `bytes`-field native transition guards (all-6 backends)

Status: DRAFT, 2026-06-03. Commits 0 LOC; implementation lands after §3 locks are confirmed.
Branch: `main` (HEAD `8e13729a1` at draft time).
Date: 2026-06-03.
Driver: a downstream switchboard example wires a `bytes`
EventSchema field into an observable transition guard
(`_event.data.raw === 'ack'`). Today that guard passes validation, is
selected as a native typed-payload guard, and the Rust emitter lowers it to
`ev.raw == "ack"` — which does **not compile** (`Vec<u8>: PartialEq<&str>`
does not exist). The same defect is latent on the other backends, and worse
on C11 where the `bytes` payload field has no representation at all.

## §1 Background

### §1.1 The reported defect

`_event.data.<bytes-field> === '<literal>'`:

| Stage | File:line | Behavior |
|---|---|---|
| receive-side typecheck | `event_schema_check.rs:897` | `String \| Bytes => String literal` — **accepts** the comparison |
| doc contract | `event_schema_check.rs:874-876` | promises "Bytes compares against string literals through the existing equality-as-bytes coercion at codegen" |
| codegen coercion | — | **does not exist** on any of the 6 emitters |
| operand typing | `expr.rs:1660-1665` → `types.rs:322` | comparison routes through `join_arith(Bytes, Str)` → `Unknown`; the literal never receives `expected = Bytes` |
| Rust emit | `expr.rs:2326` | `StringLit` unconditionally emits `"{value}"` → `ev.raw == "ack"` → rustc error |

The doc is an **unfulfilled contract**: the validator says "yes," codegen
emits garbage. The validation rule and its promised lowering are connected
only by a prose comment, never by code.

### §1.2 Why it is worse than "fix the Rust literal"

`==` on a byte sequence has a different shape per language. A
literal-only patch (emit `b"ack"`) is correct **only** for Rust/Python:

| Backend | payload field type | `raw == "ack"` today | textbook lowering |
|---|---|---|---|
| Rust | `Vec<u8>` | compile error | `ev.raw == b"ack"` (or numeric byte array) |
| Python | `bytes` | **compiles, always `False`** | `ev.raw == b"\x61\x63\x6b"` |
| C++ | `std::vector<uint8_t>` | compile error | `ev.raw == std::vector<uint8_t>{...}` |
| Go | `[]byte` | **compile error** (slice `==` illegal) | `bytes.Equal(ev.raw, []byte{...})` |
| Kotlin | `ByteArray` | always `false` (reference eq) | `ev.raw.contentEquals(byteArrayOf(...))` |
| **C11** | **`const uint8_t *` placeholder, no length** (`generator.rs:750`) | pointer compare / uncompilable | `len == N && memcmp(buf, lit, N) == 0` — **needs a length the field does not carry** |

The Python case is the most dangerous: it compiles and silently evaluates
to `False`, so a green build masks a wrong guard
([[feedback_green_tests_not_correct]], [[feedback_codegen_fixture_orthogonal_axes]]).

### §1.3 The SSOT-honesty problem

`guard_is_native_lowerable` (`event_schema_check.rs:262`) decides — for the
whole toolchain, language-neutrally — whether a guard lowers natively (no
script engine). It proxies "all six succeed" by checking **only Rust + Go**
(`:272-273`, documented assumption "success on those two implies success on
all six"). `bytes` **falsifies that assumption**: Rust/Go produce *some*
output (the broken `==`), so the guard is classed native, and then
`build_c11_event_payload` emits a length-less pointer field plus a broken
comparison. The verdict also feeds `native_typed_inject_events` — a
**language-neutral** set the transport switchboard keys off — so nativeness
cannot legitimately differ per backend without splitting that contract.

Consequence: this cannot be fixed "on Rust only." Either all six backends
represent and compare `bytes` natively, or none do. Backend-divergent
nativeness is rejected (it pollutes the switchboard SSOT).

### §1.4 Existing reusable infrastructure

The no-alloc bounded-`bytes` contract already exists for codec/procedure/
data slots (`rfc-forge-bytes-bounded.md`, LANDED 2026-04-30):

- `sce:max-size` annotation family; default `BYTES_DEFAULT_MAX = 256`
  (`limits.rs:18`, C mirror `sce-forge-runtime/c/include/sce/forge/limits.h`,
  lockstep test).
- `ForgeField.max_size: Option<u32>` already parsed.
- C storage shape `uint8_t <id>[CAP]; size_t <id>_len;` already emitted on
  the codec path (`generator.rs:3389/3418/3701`), `resolve_bytes_max`.
- `validation/bytes-max-size-violation` diagnostic already registered
  (`SCE_ACCEPTED_SUBSET.md:1027`).

This RFC **extends that contract to the EventSchema payload-struct surface**;
it invents no new no-alloc machinery.

## §2 Driver / non-goals

- **Correctness**: the reported guard must compile and evaluate correctly on
  every backend that claims it native ([[feedback_correctness_before_features]]).
- **North Star**: typed `_event.data.<field>` value path with no script
  engine on no_std MCU. A `bytes` guard is unusable on MCU until C11 carries
  the bytes natively — so C11 is in scope, not deferred
  ([[feedback_no_carveouts]], [[feedback_planned_not_yagni]]).
- **Non-goal**: ordering semantics on `bytes` (`<`,`>`); rejected (see B3).
- **Non-goal**: comparing a `bytes` field against a non-literal `string`
  expression (a datamodel variable). Stays on the existing typed-expression
  pipeline / script-engine path; out of scope.

## §3 Lock-in decisions

| # | Decision | Proposed answer | Rationale |
|---|---|---|---|
| **B1** | How is a `string` literal reinterpreted as `bytes`? | A new typed IR node **`ExprKind::BytesLit { bytes: Vec<u8> }`**, produced **once** in `infer_types` when a comparison has one `Bytes` operand and one `StringLit` operand. The bytes are decoded once (UTF-8) in shared code; emitters render the literal *byte values*, not a target string literal. | SSOT: the "this string is bytes" decision and the encoding live in one place, not re-derived in 6 emitters (the C `strcmp` precedent re-checks types per emitter only because operator *syntax* is irreducibly per-language; literal *reinterpretation* is not). Byte-identical comparison constant on every backend. Kotlin has no byte-string literal, so decode-once is mandatory regardless → it is the only cross-backend-consistent design. |
| **B2** | Literal content scope | **Printable-ASCII only** for the first land; escapes (`\n`, `\xNN`, `\u…`) and non-ASCII bytes are **rejected** with a clear diagnostic. `value.as_bytes()` is then exactly the decoded bytes. | Complete-not-partial ([[feedback_no_carveouts]] is about permanent W3C-conformance carve-outs; this is a *validated, forward-compatible* literal-syntax boundary — `BytesLit{Vec<u8>}` stays; only the decoder grows later). The lexer captures verbatim source (`expr.rs:723`), and emitters currently delegate escape decoding to the target compiler — a per-target divergence we must NOT inherit into byte constants. The real consumer (wz: `'ack'`, magic markers) is all printable ASCII. |
| **B3** | Operators permitted on `bytes` | **`===` / `!==` only.** Ordering (`<`,`>`,`<=`,`>=`) on a `bytes` operand is a validation error. | Lexicographic ordering of an opaque payload byte-blob is not a meaningful author intent; equality-as-bytes is. Rejecting is more textbook than silently defining an order that differs from the C `strcmp` lexicographic precedent. |
| **B4** | C11 / C storage for an EventSchema `bytes` payload field | **`uint8_t <id>[CAP]; size_t <id>_len;`**, `CAP = resolve_bytes_max(field.max_size)` — the exact codec-path shape (§1.4). Inject seam copies the struct **by value** (owned buffer), so the borrow survives event queuing. | Mirrors LANDED `rfc-forge-bytes-bounded.md` §6; no-alloc; owned-copy is safe across the C11 event queue (which copies `event_with_meta_t` by value). Borrowed `const uint8_t*`+len was considered and rejected: the queue copies the struct, not the pointee, so a borrow would dangle. |
| **B5** | Guard lowering per backend (equality) | C/C11: `<acc>_len == N && memcmp(<acc>, "<bytes>", N) == 0`; Rust: `<acc> == [<bytes>]` (`Vec<u8> == [u8; N]`); Go: `bytes.Equal(<acc>, []byte{<bytes>})`; Kotlin: `<acc>.contentEquals(byteArrayOf(<bytes>))`; C++: `<acc> == std::vector<uint8_t>{<bytes>}`; Python: `<acc> == bytes([<bytes>])`. `!==` negates. | Each backend's content-equality primitive. The `<acc>_len` field is the C-only length sibling from B4. Emitted from the **same** decoded `Vec<u8>` (B1) so the constant is byte-identical. |
| **B6** | The `guard_is_native_lowerable` proxy | **Verify all six `ExprTarget`s**, not just Rust+Go. Make the C/C++/Kotlin/Python emitters fallible for the `bytes` path (or add an explicit pre-check) so an unrepresentable `bytes` form fails the verdict instead of emitting silently. | Kills the §1.3 proxy lie permanently: nativeness can never again diverge from what a backend can actually emit. |
| **B7** | Diagnostic for B2/B3 violations | **Reuse `validation/cross-kind-type-mismatch`** for B2 (literal-not-representable) framed as a type-category failure; **new `validation/bytes-comparison-not-equality`** for B3 (ordering operator on bytes). | B2 is genuinely a literal-vs-field-type category issue (Item 4 reuse precedent). B3 is a distinct operator-domain rule with no existing code that fits without semantic stretch; one new `DiagnosticCode` variant is justified (full 11-place edit per [[diagnostic_code_edit_checklist]] + acceptance-doc appendix). **OPEN — confirm whether to instead overload `typed-cond-non-native`.** |

### §3.1 Honest interim gate (before B1–B6 land)

The first implementation commit makes a guard that compares a `bytes` field
**non-native** uniformly (a one-line predicate in the native-lowerable path),
turning today's silent miscompile into an honest "needs script engine"
classification on all backends. Hosted backends then route the guard through
the script engine (correct, if not yet native); MCU/C11 surfaces it as a
genuine "unsupported until native bytes" limitation rather than emitting a
length-less pointer. This commit is non-breaking and strictly better than
today, and is removed by the final commit once B1–B6 make `bytes` guards
genuinely native on all six.

## §4 Surface / accepted-subset changes

- `SCE_ACCEPTED_SUBSET.md` §EventSchema: document that `_event.data.<bytes>`
  supports `===`/`!==` against a printable-ASCII string literal, lowering to
  a native byte-equality on all six backends; `sce:max-size` bounds the C11
  fixed buffer (default 256). Note the B2/B3 rejections.
- No `schemas/sce-diagnostic.v1.schema.json` shape change. If B7 adds a
  variant, follow `SCE_ERROR_CONTRACT.md` §8.1 (`pre-release`) + the
  acceptance-doc appendix (`acceptance_doc_covers_every_code`).

## §5 Implementation plan (never-broken staging)

The pieces are interdependent (BytesLit is consumed by all emitters; C11
needs the storage change or it breaks), and nativeness is all-or-nothing
(§1.3), so the feature activates atomically — staged behind the §3.1 gate:

| Commit | Scope | Non-broken invariant |
|---|---|---|
| **1** | §3.1 honest gate: `bytes`-comparing guards → non-native uniformly. Regression fixture: the wz guard no longer emits broken Rust (routes to script engine). | Removes silent miscompile. All existing fixtures green (no schema has a bytes guard today). |
| **2** | B6 harden: `guard_is_native_lowerable` verifies all six targets; emitters fallible on unrepresentable forms. | Behavior-preserving (gate from commit 1 still suppresses bytes). |
| **3** | B1+B2: `ExprKind::BytesLit`, `infer_types` reinterpretation, decode-once, all-6 emitter `BytesLit` render + equality lowering (B5) for the 5 hosted backends. C still gated off. | Hosted emitters compile-correct under unit tests; gate still off end-to-end. |
| **4** | B4: C11/C EventSchema bytes payload storage (`uint8_t[CAP]+len`), inject seam owned-copy, C `memcmp` lowering (B5). | C unit tests green; gate still off. |
| **5** | B3+B7 validation: ordering-on-bytes rejection; B2 literal rejection diagnostic. | Validator tests; existing fixtures green. |
| **6** | **Flip the gate on** + cross-backend conformance fixture (match / non-match runtime branching + a rejection fixture). Update `SCE_ACCEPTED_SUBSET.md`. | All six native; runtime parity verified; 202/202 + forge gates green. |

## §6 Acceptance gates

- Per commit: `cargo test -p sce-build` green; `ctest` unaffected until commit 6.
- Commit 6 final:
  - A bytes-guard fixture compiles on all six backends with `-Werror`-grade
    strictness and **runs**: matching payload takes the guarded transition,
    non-matching does not (Python silent-`False` cannot pass this).
  - The decoded comparison constant is byte-identical across the six
    generated artifacts.
  - A rejection fixture: ordering-on-bytes and a non-ASCII literal each
    surface their B3/B2 diagnostic.
  - `guard_is_native_lowerable` verifies all six targets (B6).
  - C11 `-std=c11 -Wall -Wextra -Wpedantic -Werror` clean; no heap.

## §7 Risks

| Risk | Mitigation |
|---|---|
| C11 inject borrow dangles across queue | B4 owned fixed-buffer copy; no borrow held |
| Python `bytes == str` silent-`False` slips through | runtime match/non-match fixture (§6), not byte-golden only |
| B2 ASCII-only feels arbitrary | validated boundary with a clear diagnostic; `Vec<u8>` node is forward-compatible — decoder grows without architecture change |
| New diagnostic (B7) wire churn | reuse where defensible (B2); single justified variant for B3, full checklist |
| Gate flip (commit 6) regresses a hosted backend | commits 3–5 land each backend behind the gate with unit coverage before the atomic flip |
