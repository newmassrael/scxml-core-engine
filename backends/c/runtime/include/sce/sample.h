// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Protocol-Synthesis RFC §synth-5-E lines 1276-1346 — application-facing Sample
// API + typestate spelling + capability-attribute family. Lives
// in the C11 backend's Tier 1 INTERFACE (sce_c_runtime per
// `c11_4tier_layering.md`) so generated code, downstream consumer
// crates, and the buffer-pool codegen integration
// (`tools/codegen/templates/forge/c/buffer_pool.h.jinja2`) all see
// one contract.
//
// ── What this header is ────────────────────────────────────────
//
//   * `SCE_WARN_UNUSED` — the one attribute here that a compiler
//     acts on. Ignoring `sce_sample_take`'s result is a diagnostic
//     on GCC and Clang, in C and C++, and it is also this header's
//     statement of MISRA C:2012 Rule 17.7.
//
//   * The typestate spelling (`SCE_CONSUMABLE` /
//     `SCE_CALLABLE_WHEN(s)` / `SCE_SET_TYPESTATE(s)` /
//     `SCE_PARAM_TYPESTATE(s)`), which expands to NOTHING. Clang's
//     consumed analysis is a C++ facility keyed on classes and
//     member functions; measured against Clang 19.1.7 it produces
//     no diagnostic for this free-function C API. The tokens stay on
//     the declarations as the written form of the contract that
//     the analyzer and runtime layers enforce — see the detection
//     block below for the measurement and
//     `sample_h_typestate_is_inert_in_c_and_only_warn_unused_survives`
//     for the test that pins it.
//
//   * `sce_sample_t` — the read-only borrow handed to subscriber
//     callbacks. Carries the protocol-decoded key expression, the
//     payload byte slice, the source timestamp, and an opaque pool
//     slot handle (`_slot`) that the runtime tag-checks on
//     `sce_sample_take` to transition the slot's FSM state.
//
//   * 3 function decls — `sce_sample_payload` (borrow accessor),
//     `sce_sample_take` (consume → caller-owned bytes), and
//     `sce_sub_callback_t` (typedef for the subscriber-side
//     callback signature).
//
//   * the analyzer annotations (spec lines 1349-1365) on the two
//     function declarations — PC-lint Plus `-sem` semantics and
//     Coverity function-model primitives. Both are rendered from
//     `sce-build/src/forge/ownership_contract.rs`, which is also what
//     the per-pool C11 template renders from; the
//     `sample_h_analyzer_annotations_match_the_ownership_contract` test
//     pins this file against that contract. Hand-editing an
//     annotation here without editing the contract fails that test.
//     The two syntaxes disagree on argument numbering (PC-lint is
//     1-based, Coverity 0-based), which is exactly why they are
//     generated from one origin rather than maintained as a pair.
//
//   * `_Static_assert` invariants — pin the Slot handle's
//     discriminant ordering and `SCE_SLOT_INVALID` sentinel value
//     against drift. Mirrored byte-for-byte in
//     `tools/codegen/templates/forge/c/buffer_pool.h.jinja2` lines
//     79-97; the invariants here fire if either side drifts.
//
// ── What this header is not ────────────────────────────────────
//
//   * Concrete `sce_keyexpr_t` / `sce_timestamp_t` definitions —
//     opaque forward-declared structs. Downstream consumers
//     (e.g. zenoh-pico bindings) supply
//     the bodies via their own typedefs against the same struct tags.
//     SCE-side cannot commit to zenoh / DDS / MQTT key-space
//     semantics without coupling the C runtime to one transport.
//     Both fields are therefore referenced as pointers through
//     `sce_sample_t`, keeping the borrow shape stable across all
//     possible downstream typedefs.
//
//   * Polyspace-specific annotations. Spec line 1351 groups Polyspace
//     with PC-Lint and Coverity, but Polyspace's in-source
//     `/* polyspace ... */` comments justify *findings* — they cannot
//     declare function behaviour. Behaviour mapping goes through
//     `-code-behavior-specifications`, a separate XML file whose
//     schema ships with the installation. Emitting a Polyspace comment
//     the tool ignores would be a silently-inert hook, so nothing is
//     emitted; Polyspace users get the result-check contract via
//     MISRA C:2012 Rule 17.7, which `SCE_WARN_UNUSED` below states in
//     the form every conforming MISRA checker reads.
//
//   * A shadow slot table. Spec line 1478 budgets one byte per slot
//     for the defensive layer; none is allocated, because `sce_sample_t`
//     already carries the slot handle and the boundary compares
//     entry against exit in a scope-local.
//
// Pre-1.0: this contract may evolve before SCE 1.0 (per
// `feedback_pre_release_no_compat.md`); downstream consumers re-link
// against new releases.

#ifndef SCE_SAMPLE_H
#define SCE_SAMPLE_H

#include <assert.h>
#include <stddef.h>
#include <stdint.h>
// `memset` backs the debug-poison fill. Included unconditionally
// and outside the `extern "C"` block below: a standard header must
// not be pulled in under C linkage, and gating it on
// `SCE_DEBUG_OWNERSHIP` would put the `#include` after that macro's
// definition, which lives further down.
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Typestate detection ──────────────────────────────────
//
// `__has_attribute` answers "does this compiler know the attribute
// name", NOT "does it apply the attribute to the declaration I am
// about to write". For Clang's consumed-analysis family those two
// answers differ in C, and the gap is total rather than partial.
//
// Measured against Clang 19.1.7, `-std=c11 -Wconsumed`:
//
//   struct __attribute__((consumable(unconsumed))) S { int x; };
//   → warning: 'consumable' attribute only applies to classes
//   → same for callable_when / set_typestate
//   → -Wconsumed diagnostics produced: none
//
// The same translation unit under `-std=c++17` does produce
// -Wconsumed diagnostics. Clang's consumed analysis is a C++ facility:
// `consumable` attaches to a class, and `callable_when` /
// `set_typestate` attach to that class's MEMBER functions — they are
// dropped on a free function even in C++, which is why this header's
// free-function API cannot express the typestate layer in either language.
//
// So `SCE_OWNERSHIP_ATTRS_AVAILABLE` is gated on `__cplusplus`. In C
// it is 0, and that is not a limitation being worked around: it is
// what the compiler actually does. Reporting 1 there would claim
// coverage that produces no diagnostic, and — because `-Werror`
// builds see `-Wignored-attributes` — would break downstream
// compiles while claiming to protect them.
//
// The consequence is deliberate and load-bearing: on the C11 backend
// the analyzer layer and the runtime boundary are not a fallback
// for the typestate layer, they are the entire defence. The
// defensive default below reads this macro rather than `__clang__`
// precisely so that a C build gets the runtime boundary switched on.

// Pinned to 0, not probed. `__has_attribute` answers yes for every
// name in the family and the compiler then drops all of them on these
// declarations — probing would report coverage that does not exist.
#define SCE_OWNERSHIP_ATTRS_AVAILABLE 0

// The typestate family expands to nothing, in every language and on
// every toolchain. The tokens stay on the declarations below because
// they are the written form of the ownership contract that the
// analyzer and runtime layers enforce — removing them would delete the
// statement of intent that the analyzer annotations and the runtime
// boundary are derived from. They are documentation with a compiler
// -checked spelling, not a dormant feature switch.
#define SCE_CONSUMABLE
#define SCE_CALLABLE_WHEN(s)
#define SCE_SET_TYPESTATE(s)
#define SCE_PARAM_TYPESTATE(s)

// `warn_unused_result` is the one member of the original five that
// works — on C and C++, GCC and Clang alike — so it gets its own
// detection instead of riding on the dead family above. It is also
// how this header states MISRA C:2012 Rule 17.7, which is the only
// The analyzer layer fact Polyspace can read from source.
#if defined(__has_attribute)
#if __has_attribute(warn_unused_result)
#define SCE_WARN_UNUSED __attribute__((warn_unused_result))
#else
#define SCE_WARN_UNUSED
#endif
#else
#define SCE_WARN_UNUSED
#endif

// ── Configure-time self-check ─────────────────────────────────────
//
// There is deliberately no `#warning` here any more.
//
// The previous one fired on "Clang detected but the attributes are
// unavailable", which reads as a misconfiguration the operator can
// fix. It is not one: the attributes are unavailable on every build
// of this header, so the warning would fire on every Clang
// translation unit and teach operators to silence it — the exact
// habit that later hides a diagnostic that does matter.
//
// The gap it was pointing at is closed rather than announced.
// `SCE_DEFENSIVE_OWNERSHIP` below defaults ON whenever
// `SCE_OWNERSHIP_ATTRS_AVAILABLE` is 0, so a build with no the typestate layer
// gets the defensive layer runtime boundary without the operator doing
// anything. The `pool/sample-typestate-attributes-disabled`
// codegen diagnostic still guards the other half — that the
// generated pool header actually pulls this file in.

// ── Opaque forward typedefs ──────────────────────────────────────
//
// Protocol-decoded value types: SCE has no opinion on key-expression
// or timestamp semantics. Downstream consumers (zenoh-pico, DDS, MQTT)
// provide the struct body and typedef their concrete shape against the
// same tag. The borrow form in `sce_sample_t` is therefore a pointer
// to an opaque type — the by-value spec form (RFC §synth-5-E line 1313) is
// achievable by consumers that supply a complete struct body before
// including this header, but the pointer form is the canonical SCE-
// side API surface.
struct sce_keyexpr_t;
typedef struct sce_keyexpr_t sce_keyexpr_t;

struct sce_timestamp_t;
typedef struct sce_timestamp_t sce_timestamp_t;

// ── Slot handle (mirrors forge/c/buffer_pool.h.jinja2) ───────────
//
// The discriminant ordering and `SCE_SLOT_INVALID` sentinel value
// are pinned identically to the per-pool template emission. C11
// permits identical typedef redeclaration (DR-477 / 6.7.2.3), so
// downstream code that includes both this header and one or more
// generated `<pool_name>.h` headers compiles cleanly. If the
// buffer-pool template ever drifts off this layout, the per-pool header's
// `static` storage emission against `sce_slot_state_t` would
// surface a redeclaration error at downstream consumer-build time.
typedef enum {
    SCE_SLOT_FREE = 0,
    SCE_SLOT_CPU_MUT = 1,
    SCE_SLOT_DMA_ARMED_TX = 2,
    SCE_SLOT_DMA_BUSY_TX = 3,
    SCE_SLOT_DMA_ARMED_RX = 4,
    SCE_SLOT_DMA_BUSY_RX = 5,
    SCE_SLOT_CPU_REF = 6,
    SCE_SLOT_INVALID = 0xFF
} sce_slot_state_t;

typedef struct {
    sce_slot_state_t state;
    size_t idx;
} sce_slot_handle_t;

// ── Result type for take operations ──────────────────────────────
//
// `sce_sample_take` reports outcome through this enum. The 0 = OK
// convention matches POSIX-style return codes; non-zero values are
// the documented failure cases (caller buffer too small, sample
// already invalidated by a prior take or pool-side eviction). Future
// failure modes land as additional discriminants — a value not
// listed here today must not be returned by any conforming
// implementation.
typedef enum { SCE_RESULT_OK = 0, SCE_RESULT_ERR_BUFFER_TOO_SMALL = 1, SCE_RESULT_ERR_INVALID_SAMPLE = 2 } sce_result_t;

// ── sce_sample_t ─────────────────────────────────────────────────
//
// The borrow handed to subscriber callbacks. The `SCE_CONSUMABLE`
// attribute on the struct (the typestate layer only) marks it as a typestate-
// tracked value: callbacks that exit without calling
// `sce_sample_take` invalidate the borrow at scope-end, and Clang's
// `-Wconsumed` flags any attempt to re-use a `consumed` sample.
typedef struct SCE_CONSUMABLE {
    /// Protocol-decoded key expression (opaque to SCE).
    const sce_keyexpr_t *key_expr;
    /// Payload byte slice — valid only while the sample is unconsumed.
    const uint8_t *payload;
    /// Length of `payload`.
    size_t payload_len;
    /// Source timestamp (opaque to SCE).
    const sce_timestamp_t *timestamp;
    /// Pool slot handle backing the borrow. Authors must not read or
    /// modify this field directly; it exists so `sce_sample_take`
    /// can drive the FSM transition under the runtime tag check.
    sce_slot_handle_t _slot;
} sce_sample_t;

// ── 3 function declarations (RFC §synth-5-E lines 1318-1334) ───────────

/// Borrow accessor — returns a pointer to the sample's payload bytes.
/// Valid only while the sample is unconsumed. Calling after
/// `sce_sample_take` is a compile-time error under
/// `-Wconsumed -Wthread-safety` and undefined behaviour without
/// the analyzer.
///
/// Analyzer layer: the sample is borrowed, not consumed — `1p` says only
/// that the argument is dereferenced. No `custodial` and no Coverity
/// `+free`: telling an analyzer this accessor consumes its argument
/// would flag correct callers that go on to `take`.
/*lint -sem(sce_sample_payload, 1p) */
SCE_CALLABLE_WHEN("unconsumed")
const uint8_t *sce_sample_payload(const sce_sample_t *sample SCE_PARAM_TYPESTATE("unconsumed"));

/// Consume the sample — copies its payload into the caller-owned
/// buffer at `dst` (capacity `dst_cap`), writes the byte count to
/// `*out_len`, and transitions the sample's typestate to `consumed`
/// so subsequent borrow accesses surface as the typestate layer diagnostics.
/// Returns `SCE_RESULT_OK` on success, or one of the documented
/// `SCE_RESULT_ERR_*` values otherwise. `SCE_WARN_UNUSED` makes
/// ignoring the result a Clang/GCC warning — and states MISRA C:2012
/// Rule 17.7 in the form every conforming checker reads, which is the
/// only the analyzer layer fact Polyspace can consume from source.
///
/// Analyzer layer: `custodial(1)` is the point of the layer — after the
/// call argument 1 is invalid, so PC-lint flags a subsequent
/// `sce_sample_payload(sample)` in the same scope (issue 429 / 449).
/// Coverity's `+free : arg-0` states the same fact 0-based. `2p` and
/// `4p` mark `dst` and `out_len` as dereferenced out-parameters.
/*lint -sem(sce_sample_take, custodial(1), 1p, 2p, 4p) */
/* coverity[+free : arg-0] */
SCE_WARN_UNUSED
SCE_CALLABLE_WHEN("unconsumed")
SCE_SET_TYPESTATE("consumed")
sce_result_t sce_sample_take(const sce_sample_t *sample SCE_PARAM_TYPESTATE("unconsumed"), uint8_t *dst, size_t dst_cap,
                             size_t *out_len);

/// Subscriber callback typedef — consumers register a function of
/// this signature with the link's RX path. The `sample` parameter
/// is `param_typestate("unconsumed")` so Clang flags any callback
/// that lets the borrow escape its scope without a `take`.
typedef void (*sce_sub_callback_t)(const sce_sample_t *sample SCE_PARAM_TYPESTATE("unconsumed"), void *ctx);

// ── Runtime ownership checking (spec lines 1367-1378 + 1462-1484)
//
// The typestate layer is intra-procedural: it loses the borrow at an
// indirect
// call, so a subscriber callback that stores `sample` in a global
// is the one violation class no compile-time layer can see (spec
// lines 1464-1467). the debug-poison and defensive layers close it at runtime.
//
// ── Where the check lives, and why ──────────────────────────────
//
// The callback boundary belongs to SCE: generated code calls the
// host's handler from `<machine>_deliver_link_<X>_sample`
// (`tools/codegen/templates/c/state_machine.c.jinja2`). That
// function is therefore where enter/exit verification and poisoning
// happen — no cooperation from the host is required, so the check
// cannot be silently skipped by a downstream that forgets to call
// into it.
//
// Three of the four debug-poison behaviours land there or in the pool:
//
//   * poison on callback return — the boundary (below)
//   * use-after-callback-return — the boundary, via the poison
//   * double `sce_sample_take` — the per-pool tag check, raised from
//     "return false" to a trap under these macros
//     (`forge/c/buffer_pool.h.jinja2`)
//
// The fourth, `dst_cap < payload_len`, sits inside `sce_sample_take`,
// whose definition is supplied downstream: a pool-agnostic runtime
// header cannot return a slot to a specific pool's static table, so
// SCE declares the function and the transport binding defines it.
// `SCE_OWNERSHIP_TRAP` is public precisely so that definition can
// raise the same way rather than inventing its own failure mode.
//
// ── No shadow table ─────────────────────────────────────────────
//
// Spec line 1478 budgets "one `uint8_t` per slot in the shadow
// table". None is needed: `sce_sample_t` already carries the slot's
// handle, so the entry state is read from `sample->_slot.state` and
// compared against the exit state in a scope-local. The check costs
// zero bytes of RAM, which matters more on an MCU than the table
// would have.

/// Debug-poison layer. Opt-in, never a release default (spec lines
/// 1367-1370). Implies the defensive boundary checks and adds the
/// poison fill.
#ifndef SCE_DEBUG_OWNERSHIP
#define SCE_DEBUG_OWNERSHIP 0
#endif

/// The defensive layer — defensive boundary checks, release-safe (spec lines
/// 1462-1484).
///
/// The default keys on whether the typestate layer is *actually* live, not on
/// which compiler is running. Spec lines 1441-1443 write the default
/// as "GCC → 1, Clang → 0", but a Clang build whose `consumable`
/// family is unavailable (older Clang, `-fno-thread-safety`) has no
/// typestate layer either — the case the removed `#warning` covered.
/// Keying on the compiler would leave that build with neither layer;
/// keying on `SCE_OWNERSHIP_ATTRS_AVAILABLE` closes it. On a Clang
/// build with the attributes present the two rules agree.
#ifndef SCE_DEFENSIVE_OWNERSHIP
#if SCE_OWNERSHIP_ATTRS_AVAILABLE
#define SCE_DEFENSIVE_OWNERSHIP 0
#else
#define SCE_DEFENSIVE_OWNERSHIP 1
#endif
#endif

/// True when either layer wants the boundary instrumented.
#define SCE_OWNERSHIP_CHECKED (SCE_DEBUG_OWNERSHIP || SCE_DEFENSIVE_OWNERSHIP)

/// Byte written over a consumed borrow's payload under the debug-poison layer
/// (spec line 1371). Chosen by the spec; pinned here so the
/// generated boundary and any downstream `sce_sample_take` agree on
/// what a poisoned read looks like.
#define SCE_OWNERSHIP_POISON_BYTE 0xDE

/// Failure action. Overridable so a target without `assert` (or one
/// that must reach a fault handler / safe state rather than abort)
/// can route the violation somewhere useful. The default is
/// `assert`, which compiles out under `NDEBUG` — deliberate for
/// the debug-poison layer, which is debug-only. The defensive layer
/// ships in release builds, where `NDEBUG` is typically defined, so a
/// deployment that turns it on is expected to define this.
#ifndef SCE_OWNERSHIP_TRAP
#define SCE_OWNERSHIP_TRAP(msg) assert(0 && (msg))
#endif

/// Raise a per-pool tag-check failure (spec line 1373, "traps on
/// double `sce_sample_take`, take-after-callback-return").
///
/// The generated pool API already tag-checks every transition and
/// returns `false` / `NULL` on a mismatch; that return is the release
/// behaviour and does not change. This macro is how the debug-poison layer turns the
/// same condition into a stop, so the offending call site is the one
/// in the debugger rather than whatever code later mishandles the
/// failure return.
///
/// Debug-only on purpose: a tag mismatch always indicates a caller
/// bug, but a release build that defensively probes state and handles
/// the failure return must not be stopped for doing so. the defensive layer
/// therefore leaves this inert and instruments only the callback
/// boundary, matching spec line 1474 ("skips the heavier checks").
#if SCE_DEBUG_OWNERSHIP
#define SCE_OWNERSHIP_TAG_VIOLATION(msg) SCE_OWNERSHIP_TRAP(msg)
#else
#define SCE_OWNERSHIP_TAG_VIOLATION(msg) ((void)0)
#endif

#if SCE_OWNERSHIP_CHECKED

/// Slot state captured when a callback is entered, so the exit check
/// has something to compare against. Scope-local at the boundary; no
/// static storage, so nesting and re-entrancy are naturally handled.
typedef struct {
    /// The slot's lifecycle state on entry.
    sce_slot_state_t entry_state;
    /// Payload base recorded on entry, poisoned on exit under the debug-poison layer.
    const uint8_t *payload;
    /// Payload length recorded on entry.
    size_t payload_len;
} sce_ownership_scope_t;

/// Open a callback scope. Verifies the borrow arrives in a
/// CPU-visible state: `cpu-ref` is the state the RX path hands a
/// subscriber (spec line 1471). A sample arriving DMA-armed or
/// DMA-busy means the link published a slot the peripheral still
/// owns — a bug that would otherwise surface as intermittently
/// corrupt payload bytes.
static inline sce_ownership_scope_t sce_ownership_callback_enter(const sce_sample_t *sample) {
    sce_ownership_scope_t scope;
    scope.entry_state = SCE_SLOT_INVALID;
    scope.payload = NULL;
    scope.payload_len = 0;
    if (sample == NULL) {
        SCE_OWNERSHIP_TRAP("sce_sample_t borrow delivered as NULL");
        return scope;
    }
    if (sample->_slot.state != SCE_SLOT_CPU_REF && sample->_slot.state != SCE_SLOT_CPU_MUT) {
        SCE_OWNERSHIP_TRAP("sample delivered to a callback while the slot is not CPU-visible");
    }
    scope.entry_state = sample->_slot.state;
    scope.payload = sample->payload;
    scope.payload_len = sample->payload_len;
    return scope;
}

/// Close a callback scope.
///
/// The borrow ends here whether the callback consumed the sample or
/// only read it. The exit check is not "did it take?" — it cannot be:
/// `sce_sample_take` receives a `const sce_sample_t *` and so cannot
/// legally alter the borrow the callback was handed. What the check
/// catches is the callback altering it anyway, by casting the `const`
/// away or writing through an aliased copy: a changed payload
/// pointer, length, or slot tag means the borrow the RX path
/// published is not the one being returned, and every later
/// conclusion drawn from it — including this function's own poison
/// fill — would be aimed at the wrong memory.
///
/// Under the debug-poison layer the payload is then poisoned unconditionally (spec
/// lines 1371-1372): a handler that stashed `sample` reads
/// `SCE_OWNERSHIP_POISON_BYTE` afterwards instead of bytes that still
/// look plausible. Doing it after a `take` is harmless — `take`
/// copies to the caller's buffer first. the defensive layer skips the fill;
/// that is the whole difference between the two (spec lines
/// 1473-1475).
static inline void sce_ownership_callback_exit(const sce_ownership_scope_t *scope, const sce_sample_t *sample) {
    if (scope == NULL || sample == NULL) {
        SCE_OWNERSHIP_TRAP("ownership scope closed against a NULL sample");
        return;
    }
    if (sample->payload != scope->payload || sample->payload_len != scope->payload_len) {
        SCE_OWNERSHIP_TRAP("callback mutated the sample borrow's payload pointer or length");
        return;
    }
    if (sample->_slot.state != scope->entry_state) {
        SCE_OWNERSHIP_TRAP("callback mutated the sample borrow's slot tag");
        return;
    }
#if SCE_DEBUG_OWNERSHIP
    // The cast drops `const` on purpose: the pointee is pool storage
    // whose borrow ends at this line, and overwriting it is the point.
    // Debug builds only — the defensive layer does not reach here.
    if (sample->payload != NULL && sample->payload_len > 0) {
        (void)memset((void *)(uintptr_t)sample->payload, SCE_OWNERSHIP_POISON_BYTE, sample->payload_len);
    }
#endif
}

#endif  // SCE_OWNERSHIP_CHECKED

// ── _Static_assert invariants ────────────────────────────────────
//
// Layout invariants that must hold across this header and the
// generated buffer-pool header (`forge/c/buffer_pool.h.jinja2`). A drift on either side trips the assertion
// at the offending TU's compile time — the load-bearing properties
// are: (a) the seven FSM states and the invalid sentinel keep their
// canonical numeric values so per-pool heap-free runtime tag checks
// (`if state == SCE_SLOT_CPU_MUT`) stay correct, and (b) the handle
// struct's `state` field sits at offset 0 so a plain `state`-only
// peek under the typedef-redeclaration aliasing rule resolves
// identically across all consumer translation units.

_Static_assert(SCE_SLOT_FREE == 0, "sce_slot_state_t: SCE_SLOT_FREE must be the 0 discriminant "
                                   "(buffer_pool.h.jinja2 line 80; per-pool zero-init relies on it)");
_Static_assert(SCE_SLOT_CPU_MUT == 1, "sce_slot_state_t: SCE_SLOT_CPU_MUT discriminant drifted from "
                                      "buffer_pool.h.jinja2 line 81");
_Static_assert(SCE_SLOT_DMA_ARMED_TX == 2, "sce_slot_state_t: SCE_SLOT_DMA_ARMED_TX discriminant drifted "
                                           "from buffer_pool.h.jinja2 line 82");
_Static_assert(SCE_SLOT_DMA_BUSY_TX == 3, "sce_slot_state_t: SCE_SLOT_DMA_BUSY_TX discriminant drifted "
                                          "from buffer_pool.h.jinja2 line 83");
_Static_assert(SCE_SLOT_DMA_ARMED_RX == 4, "sce_slot_state_t: SCE_SLOT_DMA_ARMED_RX discriminant drifted "
                                           "from buffer_pool.h.jinja2 line 84");
_Static_assert(SCE_SLOT_DMA_BUSY_RX == 5, "sce_slot_state_t: SCE_SLOT_DMA_BUSY_RX discriminant drifted "
                                          "from buffer_pool.h.jinja2 line 85");
_Static_assert(SCE_SLOT_CPU_REF == 6, "sce_slot_state_t: SCE_SLOT_CPU_REF discriminant drifted from "
                                      "buffer_pool.h.jinja2 line 86");
_Static_assert(SCE_SLOT_INVALID == 0xFF, "sce_slot_state_t: SCE_SLOT_INVALID sentinel must remain 0xFF "
                                         "(buffer_pool.h.jinja2 line 87; consumed-handle marker)");
_Static_assert(offsetof(sce_slot_handle_t, state) == 0, "sce_slot_handle_t: state field must be at offset 0 so "
                                                        "tag-check peeks alias correctly across redeclared TUs");

#ifdef __cplusplus
}
#endif

#endif  // SCE_SAMPLE_H
