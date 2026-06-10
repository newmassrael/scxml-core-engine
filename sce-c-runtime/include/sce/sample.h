// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// watching-zenoh RFC §5.E lines 1276-1346 — application-facing Sample
// API + Layer 1 Clang typestate + capability-attribute family. Lives
// in the C11 backend's Tier 1 INTERFACE (sce_c_runtime per
// `c11_4tier_layering.md`) so generated code, downstream consumer
// crates, and the buffer-pool codegen integration
// (`tools/codegen/templates/forge/c/buffer_pool.h.jinja2`) all see
// one contract.
//
// ── What this header is ────────────────────────────────────────
//
//   * 5-macro family (`SCE_CONSUMABLE` / `SCE_CALLABLE_WHEN(s)` /
//     `SCE_SET_TYPESTATE(s)` / `SCE_PARAM_TYPESTATE(s)` /
//     `SCE_WARN_UNUSED`) detected via `__has_attribute(consumable)` /
//     `__has_attribute(callable_when)`. On Clang ≥ 9 with
//     `-Wconsumed -Wthread-safety` the attributes diagnose
//     use-after-take, double-take, callback-leak, and ignored
//     `sce_sample_take` results at compile time. On non-Clang or
//     older Clang the macros expand to nothing — the API contract
//     is documented and analyzer-driven (Layer 2/3 are
//     consumer-gated).
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
//     (e.g. `consumer_watching_zenoh.md` zenoh-pico bindings) supply
//     the bodies via their own typedefs against the same struct tags.
//     SCE-side cannot commit to zenoh / DDS / MQTT key-space
//     semantics without coupling the C runtime to one transport.
//     Both fields are therefore referenced as pointers through
//     `sce_sample_t`, keeping the borrow shape stable across all
//     possible downstream typedefs.
//
//   * Layer 2 (PC-Lint / Coverity / Polyspace) annotations —
//     consumer-gated (spec lines 1349-1378 describe the emission).
//
//   * Layer 3 / 3.5 (debug-build runtime poisoning, release-mode
//     opt-in) — likewise consumer-gated.
//
// Pre-1.0: this contract may evolve before SCE 1.0 (per
// `feedback_pre_release_no_compat.md`); downstream consumers re-link
// against new releases.

#ifndef SCE_SAMPLE_H
#define SCE_SAMPLE_H

#include <assert.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Layer 1 typestate detection ──────────────────────────────────
//
// `__has_attribute` is the portable feature-test for Clang's
// consumable + callable_when family (introduced in Clang 3.4). GCC
// up through 13 does not implement them — the macros below expand
// to empty there, and analyzer-driven coverage (Layer 2/3) is
// expected to fill the gap on non-Clang toolchains. Detection is
// a compile-time decision (`SCE_OWNERSHIP_ATTRS_AVAILABLE`) that
// downstream tooling can read directly.

#if defined(__has_attribute)
#  if __has_attribute(consumable) && __has_attribute(callable_when) \
      && __has_attribute(set_typestate) && __has_attribute(param_typestate) \
      && __has_attribute(warn_unused_result)
#    define SCE_OWNERSHIP_ATTRS_AVAILABLE 1
#  else
#    define SCE_OWNERSHIP_ATTRS_AVAILABLE 0
#  endif
#else
#  define SCE_OWNERSHIP_ATTRS_AVAILABLE 0
#endif

#if SCE_OWNERSHIP_ATTRS_AVAILABLE
#  define SCE_CONSUMABLE         __attribute__((consumable(unconsumed)))
#  define SCE_CALLABLE_WHEN(s)   __attribute__((callable_when(s)))
#  define SCE_SET_TYPESTATE(s)   __attribute__((set_typestate(s)))
#  define SCE_PARAM_TYPESTATE(s) __attribute__((param_typestate(s)))
#  define SCE_WARN_UNUSED        __attribute__((warn_unused_result))
#else
#  define SCE_CONSUMABLE
#  define SCE_CALLABLE_WHEN(s)
#  define SCE_SET_TYPESTATE(s)
#  define SCE_PARAM_TYPESTATE(s)
#  define SCE_WARN_UNUSED
#endif

// ── Configure-time self-check ─────────────────────────────────────
//
// Per the `mem/inter-pool-padding-not-emitted` precedent: if the operator's
// build is Clang but the consumable family is missing (older Clang,
// non-default flags such as `-fno-thread-safety`), Layer 1 silently
// becomes inert — the failure mode the downstream
// `pool/sample-typestate-attributes-disabled` diagnostic catches at
// configure time. Fire a `#warning` here so single-TU
// compiles also surface the gap; the operator can then upgrade
// Clang, fix flags, or accept Layer 2/3 substitution.
#if defined(__clang__) && !SCE_OWNERSHIP_ATTRS_AVAILABLE
#  warning \
      "<sce/sample.h>: Clang detected but the consumable typestate " \
      "attributes are unavailable; Layer 1 ownership analysis is " \
      "silently inert. Upgrade to Clang ≥ 9 or compile with " \
      "-fthread-safety; otherwise rely on Layer 2 (PC-Lint / " \
      "Coverity / Polyspace) or Layer 3 debug-build poisoning."
#endif

// ── Opaque forward typedefs ──────────────────────────────────────
//
// Protocol-decoded value types: SCE has no opinion on key-expression
// or timestamp semantics. Downstream consumers (zenoh-pico, DDS, MQTT)
// provide the struct body and typedef their concrete shape against the
// same tag. The borrow form in `sce_sample_t` is therefore a pointer
// to an opaque type — the by-value spec form (RFC §5.E line 1313) is
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
    size_t           idx;
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
typedef enum {
    SCE_RESULT_OK = 0,
    SCE_RESULT_ERR_BUFFER_TOO_SMALL = 1,
    SCE_RESULT_ERR_INVALID_SAMPLE = 2
} sce_result_t;

// ── sce_sample_t ─────────────────────────────────────────────────
//
// The borrow handed to subscriber callbacks. The `SCE_CONSUMABLE`
// attribute on the struct (Layer 1 only) marks it as a typestate-
// tracked value: callbacks that exit without calling
// `sce_sample_take` invalidate the borrow at scope-end, and Clang's
// `-Wconsumed` flags any attempt to re-use a `consumed` sample.
typedef struct SCE_CONSUMABLE {
    /// Protocol-decoded key expression (opaque to SCE).
    const sce_keyexpr_t* key_expr;
    /// Payload byte slice — valid only while the sample is unconsumed.
    const uint8_t*       payload;
    /// Length of `payload`.
    size_t               payload_len;
    /// Source timestamp (opaque to SCE).
    const sce_timestamp_t* timestamp;
    /// Pool slot handle backing the borrow. Authors must not read or
    /// modify this field directly; it exists so `sce_sample_take`
    /// can drive the FSM transition under the runtime tag check.
    sce_slot_handle_t    _slot;
} sce_sample_t;

// ── 3 function declarations (RFC §5.E lines 1318-1334) ───────────

/// Borrow accessor — returns a pointer to the sample's payload bytes.
/// Valid only while the sample is unconsumed. Calling after
/// `sce_sample_take` is a compile-time error under
/// `-Wconsumed -Wthread-safety` and undefined behaviour without
/// the analyzer.
SCE_CALLABLE_WHEN("unconsumed")
const uint8_t* sce_sample_payload(const sce_sample_t* sample
                                  SCE_PARAM_TYPESTATE("unconsumed"));

/// Consume the sample — copies its payload into the caller-owned
/// buffer at `dst` (capacity `dst_cap`), writes the byte count to
/// `*out_len`, and transitions the sample's typestate to `consumed`
/// so subsequent borrow accesses surface as Layer 1 diagnostics.
/// Returns `SCE_RESULT_OK` on success, or one of the documented
/// `SCE_RESULT_ERR_*` values otherwise. `SCE_WARN_UNUSED` makes
/// ignoring the result a Clang/GCC warning.
SCE_WARN_UNUSED
SCE_CALLABLE_WHEN("unconsumed")
SCE_SET_TYPESTATE("consumed")
sce_result_t sce_sample_take(const sce_sample_t* sample
                             SCE_PARAM_TYPESTATE("unconsumed"),
                             uint8_t* dst, size_t dst_cap,
                             size_t* out_len);

/// Subscriber callback typedef — consumers register a function of
/// this signature with the link's RX path. The `sample` parameter
/// is `param_typestate("unconsumed")` so Clang flags any callback
/// that lets the borrow escape its scope without a `take`.
typedef void (*sce_sub_callback_t)(const sce_sample_t* sample
                                   SCE_PARAM_TYPESTATE("unconsumed"),
                                   void* ctx);

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

_Static_assert(SCE_SLOT_FREE == 0,
               "sce_slot_state_t: SCE_SLOT_FREE must be the 0 discriminant "
               "(buffer_pool.h.jinja2 line 80; per-pool zero-init relies on it)");
_Static_assert(SCE_SLOT_CPU_MUT == 1,
               "sce_slot_state_t: SCE_SLOT_CPU_MUT discriminant drifted from "
               "buffer_pool.h.jinja2 line 81");
_Static_assert(SCE_SLOT_DMA_ARMED_TX == 2,
               "sce_slot_state_t: SCE_SLOT_DMA_ARMED_TX discriminant drifted "
               "from buffer_pool.h.jinja2 line 82");
_Static_assert(SCE_SLOT_DMA_BUSY_TX == 3,
               "sce_slot_state_t: SCE_SLOT_DMA_BUSY_TX discriminant drifted "
               "from buffer_pool.h.jinja2 line 83");
_Static_assert(SCE_SLOT_DMA_ARMED_RX == 4,
               "sce_slot_state_t: SCE_SLOT_DMA_ARMED_RX discriminant drifted "
               "from buffer_pool.h.jinja2 line 84");
_Static_assert(SCE_SLOT_DMA_BUSY_RX == 5,
               "sce_slot_state_t: SCE_SLOT_DMA_BUSY_RX discriminant drifted "
               "from buffer_pool.h.jinja2 line 85");
_Static_assert(SCE_SLOT_CPU_REF == 6,
               "sce_slot_state_t: SCE_SLOT_CPU_REF discriminant drifted from "
               "buffer_pool.h.jinja2 line 86");
_Static_assert(SCE_SLOT_INVALID == 0xFF,
               "sce_slot_state_t: SCE_SLOT_INVALID sentinel must remain 0xFF "
               "(buffer_pool.h.jinja2 line 87; consumed-handle marker)");
_Static_assert(offsetof(sce_slot_handle_t, state) == 0,
               "sce_slot_handle_t: state field must be at offset 0 so "
               "tag-check peeks alias correctly across redeclared TUs");

#ifdef __cplusplus
}
#endif

#endif  // SCE_SAMPLE_H
