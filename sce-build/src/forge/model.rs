// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge data model — kind-specific structures for Extended SCXML.
//
// Each `sce:kind` has a dedicated model struct that captures its semantics.
// These models are separate from SCXMLModel (statechart) because they
// represent fundamentally different patterns (formulas, mappings, codecs).
//
// Language-specific type mappings live in the generator, not here.
// The model is language-agnostic.

use crate::forge::error::SourceLocation;
use serde::Serialize;

/// SCE Extension namespace URI (SCE_FORGE.md Section 3.5).
/// Shared by both SCE Forge (kind system) and SCE Mesh (distributed runtime).
pub const SCE_NAMESPACE: &str = "http://sce.dev/ext";

/// Runtime dependency tier — codifies SCE_FORGE.md §8 Kind Summary.
///
/// C1 (static linking only) and C2 (no stateful global services) are the two
/// non-negotiable embedded deployment constraints defined in §2.1. Every tier
/// satisfies both by construction: `None` has zero deps, `ForgeRuntime` is
/// header-only templates, `ForgeRuntimeHal` uses DI-injected interfaces, and
/// `SceRuntime` is the existing W3C engine (outside forge scope).
///
/// The predicates `satisfies_c1()`/`satisfies_c2()` exist as gates: adding a
/// new kind that violates either constraint forces an explicit policy decision
/// rather than a silent pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDep {
    /// No runtime dependency. Pure inline code.
    None,
    /// Depends on `sce_forge_runtime` (header-only templates, static linking).
    ForgeRuntime,
    /// Depends on `sce_forge_runtime` HAL interface (user-injected DI).
    ForgeRuntimeHal,
    /// Depends on `sce_runtime` (W3C SCXML engine). Outside forge codegen scope.
    SceRuntime,
}

// C1/C2 compliance rationale per tier (SCE_FORGE.md §2.1):
//
//   None           — no dependency at all; trivially C1+C2.
//   ForgeRuntime   — header-only templates, static linking (C1),
//                    pure functions + class templates, no global state (C2).
//   ForgeRuntimeHal — abstract interface, user-injected DI (C1+C2).
//   SceRuntime     — W3C engine, static linking, own lifecycle (C1+C2).
//
// All current tiers satisfy both constraints by construction. If a future
// tier violates either, the Rust exhaustive match in max_runtime_dep() and
// runtime_dep() will force a conscious decision at the addition site.

impl std::fmt::Display for RuntimeDep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::ForgeRuntime => write!(f, "sce_forge_runtime"),
            Self::ForgeRuntimeHal => write!(f, "sce_forge_runtime::hal"),
            Self::SceRuntime => write!(f, "sce_runtime"),
        }
    }
}

/// SCE Forge kind — declares what pattern an Extended SCXML document represents.
/// §scxml-3.1 allows foreign namespace attributes on any element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ForgeKind {
    /// Standard W3C SCXML state machine (default, existing pipeline).
    Statechart,
    /// Pure mathematical formula: input -> computation -> output.
    Transform,
    /// Discrete value mapping: enumerated input -> enumerated output.
    Lookup,
    /// Named boolean guard expression.
    Condition,
    /// Byte-level encode/decode with bit-field layout.
    Codec,
    /// Sequential procedure with branching: states + guarded transitions + run-to-completion.
    Procedure,
    /// Range/plausibility/rate-of-change validation.
    Validator,
    /// Signal filtering: moving average, low-pass, debounce.
    Filter,
    /// 1D/2D table interpolation.
    Interpolation,
    /// Periodic/delayed task timing.
    Timer,
    /// Threshold monitoring with hysteresis.
    Observer,
    /// Pure synchronous function with bounded loops and mutable locals
    /// — SCE Protocol-Synthesis RFC §synth-5-A (item A3). Free function emit on
    /// every backend (`#![no_std]`-clean on Rust when no bytes param).
    Algorithm,
    /// Byte-stream link endpoint — SCE Protocol-Synthesis RFC §synth-5-C. MCU-class
    /// kind (RFC §synth-5-J-4): emits only on `(rust, *)` and
    /// `(c11, bare_metal)`. SCE owns the `Link` trait surface in
    /// `sce-link-runtime`; per-OS impls (`sce_link_runtime_lwip`,
    /// `_tokio`, `_qnx`) live downstream.
    Link,
    /// SRAM-placed, DMA-aligned slot table — SCE Protocol-Synthesis RFC §synth-5-E.
    /// Second MCU-class kind (RFC §synth-5-J-4): emits only on `(rust, *)`
    /// and `(c11, bare_metal)`. The slot table carries the
    /// `<sce:slot-count>` / `<sce:slot-size>` / `<sce:section>` /
    /// `<sce:alignment>` / `<sce:dma-channel>` / `<sce:cache-policy>`
    /// schema, the 7-state lifecycle FSM (`free` / `cpu-mut` /
    /// `dma-armed-{tx,rx}` / `dma-busy-{tx,rx}` / `cpu-ref`), and
    /// cache-maintenance pinning routed through the §synth-5-I `<sce:call>`
    /// intrinsic registry.
    BufferPool,
    /// Concurrent execution context driven by a `<sce:link-rx>` source —
    /// SCE Protocol-Synthesis RFC §synth-5-D lines 858-913. Third MCU-class kind (RFC
    /// §synth-5-J-4): emits only on `(rust, *)` (tokio::spawn on AP, cooperative-
    /// scheduler slot on MCU) and `(c11, bare_metal)` (cooperative-
    /// scheduler slot, fixed ring-buffer inbox). The schema carries a
    /// parse-time author guard for `worker/shared-mutable-state`; codegen
    /// is Rust + C11 dual-emit (heapless::spsc / opaque sce_inbox_*
    /// types) with cross-resolution + ordering codes; deploy-aware
    /// `MachineSchedulerConfig` + worker-count validation runs against
    /// deploy.yaml.
    Worker,
    /// Typed container with build-time-declared capacity but runtime-varying
    /// occupancy — SCE Protocol-Synthesis RFC §synth-5-L lines 2540-2655. zenoh-pico parity
    /// (runtime subscription declare/undeclare + queryable + reassembly
    /// tables) requires this on MCU where heap is forbidden. Six-language
    /// emitter table per §synth-5-J-5 (Rust `heapless::Vec<T, N>` / C11 slot+bitmap
    /// struct / Cpp `std::array<T, N>` + `std::bitset<N>` / Kotlin
    /// `Array<T?>` + `BooleanArray` / Go fixed-array + mask / Python list +
    /// bytearray mask). Parse-time validators cover the two
    /// structure-only codes (`collection/ordering-sorted-requires-
    /// index-by` + `collection/overflow-policy-oldest-wins-requires-ordering-
    /// insertion`); cross-doc element-type resolution + index-by-field
    /// verification + the atomics-import check run as cross-doc
    /// validators; capacity resolves against deploy.yaml and all six
    /// backends emit.
    BoundedCollection,
    /// Closed set of named variants with 1:1 wire-key values.
    /// Distinct from [`Self::Lookup`]
    /// (many-to-one dispatch) by the bijectivity invariant: each
    /// variant's `value` is unique within the document, and the
    /// generated typed enum carries that wire byte verbatim. Surface
    /// form is a top-level `<scxml sce:kind="enum" sce:underlying-type="<int>">`
    /// document with one `<sce:variant name="…" value="…"/>` child per
    /// variant. Per-backend codegen lowers to `enum class : uint8_t`
    /// / `#[repr(u8)] enum` / `IntEnum` / etc. The inline form
    /// (`<sce:enum>` as child of `<scxml>`) is not implemented until a
    /// consumer needs it.
    Enum,
    /// Typed contract for `_event.data` of a named SCXML event (the
    /// 18th forge kind). Maps an event name
    /// (`job.completed`, `task.failed`, …) to a list of typed `<data>`
    /// declarations so the receive-side typecheck can validate
    /// `_event.data.<field>` expressions in transition `cond` attributes
    /// at parse time. Send-side payload validation
    /// (`<send>/<param name="F">`) and mesh cross-machine validation
    /// ride the same shape. Schemaless fallback: events
    /// without an imported EventSchema retain dynamic `_event.data`
    /// behavior. W3C built-in events (`error.*`, `done.invoke.*`,
    /// `done.state.*`) are explicitly excluded — declaring a schema for
    /// these raises `validation/event-schema-on-builtin-event`.
    /// Per-backend payload struct codegen emits a typed
    /// record per backend that references imported Enum kinds by
    /// qualified name via `LangCtx::resolved_type`; the Enum kind owns
    /// its emitted type, so EventSchema never re-emits enum variants
    /// (the Enum document is the single source of truth).
    EventSchema,
}

impl ForgeKind {
    /// Every legal `sce:kind` attribute value, in declaration order.
    /// Single source of truth for `from_attr()` and any diagnostic
    /// that needs to surface the closed enumeration (e.g. the
    /// `validation/unsupported-kind` candidate list).
    pub const ALL_ATTR_NAMES: &'static [&'static str] = &[
        "statechart",
        "transform",
        "lookup",
        "condition",
        "codec",
        "procedure",
        "validator",
        "filter",
        "interpolation",
        "timer",
        "observer",
        "algorithm",
        "link",
        "buffer-pool",
        "worker",
        "bounded-collection",
        "enum",
        "event-schema",
    ];

    /// Parse from `sce:kind` attribute value. Returns `None` for unknown kinds.
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "statechart" => Some(Self::Statechart),
            "transform" => Some(Self::Transform),
            "lookup" => Some(Self::Lookup),
            "condition" => Some(Self::Condition),
            "codec" => Some(Self::Codec),
            "procedure" => Some(Self::Procedure),
            "validator" => Some(Self::Validator),
            "filter" => Some(Self::Filter),
            "interpolation" => Some(Self::Interpolation),
            "timer" => Some(Self::Timer),
            "observer" => Some(Self::Observer),
            "algorithm" => Some(Self::Algorithm),
            "link" => Some(Self::Link),
            "buffer-pool" => Some(Self::BufferPool),
            "worker" => Some(Self::Worker),
            "bounded-collection" => Some(Self::BoundedCollection),
            "enum" => Some(Self::Enum),
            "event-schema" => Some(Self::EventSchema),
            _ => None,
        }
    }

    /// Whether this kind can appear inline within a statechart `<data>` element.
    /// Only stateless kinds are inline-eligible.
    pub fn is_inline_eligible(&self) -> bool {
        // RFC §synth-5-A: Algorithm emits a free function (stateless) but is
        // a top-level kind imported via `<sce:import>` rather than
        // inlined into a statechart `<data>` element. Future RFC
        // revisions may flip this when an inline-Algorithm consumer
        // appears; today every fixture imports.
        matches!(
            self,
            Self::Transform | Self::Lookup | Self::Condition | Self::Codec
        )
    }

    /// Whether this kind generates a struct/class with instance methods.
    /// Struct-based kinds need member variables when imported cross-file.
    /// Pure-function kinds (transform, lookup, condition) generate free functions
    /// and only need include/import statements, not member declarations.
    pub fn needs_instance(&self) -> bool {
        match self {
            Self::Codec | Self::Validator | Self::Procedure => true,
            Self::Filter | Self::Observer | Self::Timer => true,
            Self::Transform | Self::Lookup | Self::Condition => false,
            Self::Interpolation => false,
            Self::Statechart => false,
            // RFC §synth-5-A: Algorithm is a free function — no instance state.
            Self::Algorithm => false,
            // RFC §synth-5-C: Link emits a struct that owns an `impl Link`
            // driver and routes RX/TX through the framer codec. The
            // generated module exposes a constructor that the consumer
            // wires to a downstream `sce_link_runtime_<os>` impl.
            Self::Link => true,
            // RFC §synth-5-E: BufferPool emits a struct owning a fixed-size
            // slot table with the 7-state lifecycle FSM, phantom-typed
            // `Slot<state>` API + IR-level borrow check.
            // Acquire/return surface lives
            // on the struct, not as free functions.
            Self::BufferPool => true,
            // RFC §synth-5-D: Worker emits a struct owning an SPSC inbox
            // (heapless::spsc on Rust; opaque sce_inbox_{producer,
            // consumer}_t handle pair on C11). The inbox storage + head/
            // tail indices are instance state of the generated struct.
            Self::Worker => true,
            // RFC §synth-5-L lines 2566-2581: BoundedCollection emits a typed
            // fixed-capacity container — Rust `heapless::Vec<T, N>` /
            // Cpp `std::array<T, N>` + `std::bitset<N>` / Kotlin
            // `Array<T?>` + `BooleanArray` / Go fixed array + mask /
            // Python list + bytearray mask / C11 slot+bitmap struct.
            // The slot table, occupancy mask, and generation counters
            // are instance state of the generated struct.
            Self::BoundedCollection => true,
            // Enum is a typed vocabulary
            // declaration — codegen emits a type definition
            // (`enum class` / `#[repr] enum` / `IntEnum` / etc.) not
            // a struct instance. No runtime state.
            Self::Enum => false,
            // EventSchema is parse-time metadata
            // — a typed contract for `_event.data` of a named event.
            // Codegen emits a per-backend payload struct
            // (`struct <Schema>Payload` etc.) that downstream consumers
            // construct or destructure at the `<send>/<param>` site;
            // the schema document itself carries no runtime instance.
            Self::EventSchema => false,
        }
    }

    /// Conservative (worst-case) runtime dependency for this kind.
    ///
    /// Returns the maximum `RuntimeDep` tier the kind can require. For kinds
    /// whose dependency varies by document content (e.g. Procedure L1 vs L2),
    /// this returns the upper bound. Use `ForgeDocument::runtime_dep()` for
    /// a precise answer after parsing.
    pub fn max_runtime_dep(&self) -> RuntimeDep {
        match self {
            Self::Transform | Self::Condition | Self::Codec | Self::Validator => RuntimeDep::None,
            // Lookup: string output = None (enum dispatch), numeric = ForgeRuntime.
            // Procedure: L1 = None, L2 = ForgeRuntime.
            // Upper bound for both is ForgeRuntime.
            Self::Lookup | Self::Procedure => RuntimeDep::ForgeRuntime,
            Self::Filter | Self::Interpolation | Self::Observer => RuntimeDep::ForgeRuntime,
            Self::Timer => RuntimeDep::ForgeRuntimeHal,
            Self::Statechart => RuntimeDep::SceRuntime,
            // RFC §synth-5-A: Algorithm bottom-outs to language-native loops
            // and locals, no helper crate. `#![no_std]`-clean on Rust
            // when no `bytes` parameter.
            Self::Algorithm => RuntimeDep::None,
            // RFC §synth-5-C: Link's generated code depends on the `Link`
            // trait surface owned by SCE's `sce-link-runtime` crate.
            // No SCE-side runtime dependency tier captures "downstream
            // crate" — the trait surface is contract, the impl lives
            // downstream. Tier `None` is honest at the SCE level.
            Self::Link => RuntimeDep::None,
            // RFC §synth-5-E: BufferPool ships a self-contained slot table
            // — no runtime helper crate dependency. The IR-level
            // borrow check is codegen-time; cache-maintenance pinning
            // routes through §synth-5-I
            // intrinsics, themselves contracts not runtime helpers.
            // SCE-side tier `None` matches Link's stance.
            Self::BufferPool => RuntimeDep::None,
            // RFC §synth-5-D: Worker uses `heapless::spsc` on Rust (third-party
            // crate, not SCE-side helper) and bare ring-buffer + atomics
            // intrinsics on C11. The atomic ordering intrinsics come
            // through §synth-5-I `<sce:extern>` whitelist (`sce_intrinsics_runtime`
            // baseline); the SCE-side helper-crate tier is `None`.
            Self::Worker => RuntimeDep::None,
            // RFC §synth-5-L lines 2571-2581: BoundedCollection lowers to a
            // self-contained slot table per backend (no SCE-side helper
            // crate) — `heapless` is a third-party Rust crate, the
            // C11/Cpp/Kotlin/Go/Python forms use language-builtin
            // primitives only. The cross-backend parity contract
            // (§synth-6.2.6) is codegen-time, not a runtime helper. Tier
            // `None` matches BufferPool's stance.
            Self::BoundedCollection => RuntimeDep::None,
            // Enum lowers to a pure type
            // declaration per backend — no helper crate dependency.
            Self::Enum => RuntimeDep::None,
            // EventSchema's codegen
            // emits a per-backend payload struct with primitive /
            // bool / string / bytes / typed-Enum fields. No SCE-side
            // runtime helper crate (the typed-Enum fields reference
            // the Enum kind's emitted type by qualified name; no
            // shared runtime). Tier `None` matches Enum's stance.
            Self::EventSchema => RuntimeDep::None,
        }
    }

    /// Whether this kind is currently implemented and supported.
    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Self::Statechart
                | Self::Transform
                | Self::Lookup
                | Self::Condition
                | Self::Codec
                | Self::Validator
                | Self::Procedure
                | Self::Filter
                | Self::Interpolation
                | Self::Timer
                | Self::Observer
                | Self::Algorithm
                | Self::Link
                | Self::BufferPool
                | Self::Worker
                | Self::BoundedCollection
                | Self::Enum
                | Self::EventSchema
        )
    }
}

impl std::fmt::Display for ForgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Statechart => write!(f, "statechart"),
            Self::Transform => write!(f, "transform"),
            Self::Lookup => write!(f, "lookup"),
            Self::Condition => write!(f, "condition"),
            Self::Codec => write!(f, "codec"),
            Self::Procedure => write!(f, "procedure"),
            Self::Validator => write!(f, "validator"),
            Self::Filter => write!(f, "filter"),
            Self::Interpolation => write!(f, "interpolation"),
            Self::Timer => write!(f, "timer"),
            Self::Observer => write!(f, "observer"),
            Self::Algorithm => write!(f, "algorithm"),
            Self::Link => write!(f, "link"),
            Self::BufferPool => write!(f, "buffer-pool"),
            Self::Worker => write!(f, "worker"),
            Self::BoundedCollection => write!(f, "bounded-collection"),
            Self::Enum => write!(f, "enum"),
            Self::EventSchema => write!(f, "event-schema"),
        }
    }
}

// ── Cross-language type system ─────────────────────────────────

/// Reference to an imported `sce:kind="enum"` document, carried as the
/// payload of [`SceType::Enum`].
/// Fields may declare `sce:type="enum:<alias>"`, which parses into
/// `SceType::Enum(EnumRef { alias: "<alias>".into() })`. The alias is
/// resolved against the surrounding document's `<sce:import>` table by
/// the cross-kind binding pass so the underlying integer type can be
/// looked up at validation / codegen time.
///
/// `EnumRef` is a named struct rather than a bare `String` so future
/// extensions (cached resolved underlying type, source-line tracking
/// for the `enum:` token, etc.) can be added without churning every
/// callsite that pattern-matches `SceType::Enum`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumRef {
    /// Import alias declared on the surrounding document's
    /// `<sce:import alias="…">` element. Cross-kind binding pass
    /// resolves this against the imports table to find the underlying
    /// [`EnumModel`] and consult its `underlying_type`.
    pub alias: String,
}

/// Canonical SCE type — used in `sce:type` attributes.
/// Language-specific mappings are in the generator module (SRP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum SceType {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    String,
    Bytes,
    /// Enum-typed value drawn from an imported `sce:kind="enum"`
    /// document. Surface form
    /// `sce:type="enum:<alias>"`. The lattice rule is:
    /// `Enum(E) ⊑ E.underlying_type` (e.g. `Enum(E) ⊑ Uint8` when E's
    /// `sce:underlying-type="uint8"`), and cross-type comparison
    /// `Enum(E) == 0x11` lowers to the underlying integer comparison
    /// after cross-kind resolution. Per-backend codegen
    /// emits language-native typed enums (`enum class : uint8_t`,
    /// `#[repr(u8)] enum`, `IntEnum`, …). At the IR level
    /// `is_unsigned()` / `int_bit_width()` / etc. return
    /// `false` / `None` and callers that need the underlying type
    /// must resolve via the cross-kind binding pass first.
    #[serde(rename = "enum")]
    Enum(EnumRef),
}

impl SceType {
    pub fn from_attr(s: &str) -> Option<Self> {
        // `sce:type="enum:<alias>"` parses to
        // `SceType::Enum(EnumRef { alias })`. The alias must be
        // non-empty; whitespace-only or empty payloads return None
        // here so the diagnostic surfaces as "unknown sce:type" rather
        // than as a downstream import-resolution failure with no
        // alias to dereference.
        if let Some(alias) = s.strip_prefix("enum:") {
            let alias = alias.trim();
            if alias.is_empty() {
                return None;
            }
            return Some(Self::Enum(EnumRef {
                alias: alias.to_string(),
            }));
        }
        match s {
            "uint8" => Some(Self::Uint8),
            "uint16" => Some(Self::Uint16),
            "uint32" => Some(Self::Uint32),
            "uint64" => Some(Self::Uint64),
            "int8" => Some(Self::Int8),
            "int16" => Some(Self::Int16),
            "int32" => Some(Self::Int32),
            "int64" => Some(Self::Int64),
            "float32" => Some(Self::Float32),
            "float64" => Some(Self::Float64),
            "bool" => Some(Self::Bool),
            "string" => Some(Self::String),
            "bytes" => Some(Self::Bytes),
            _ => None,
        }
    }

    /// Unsigned integer types (uint8..uint64).
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64
        )
    }

    /// Stringified maximum value for unsigned integer types — used by the
    /// C11 validator template to detect when a `range-max` annotation
    /// equals the type's natural ceiling so the generated `> max`
    /// comparison can be elided. gcc's `-Wtype-limits` (promoted to
    /// `-Werror` in the C11 conformance build) rejects tautological
    /// comparisons like `uint8_t > 255`, so the generator must surface
    /// the type's max as a string for jinja-side comparison against the
    /// rule's `max` text. Returns `None` for non-unsigned types — the
    /// equivalent signed-max elision would require also tracking the
    /// signed-min boundary, and no current fixture pins type-extremal
    /// signed bounds.
    pub fn unsigned_max_str(&self) -> Option<&'static str> {
        match self {
            Self::Uint8 => Some("255"),
            Self::Uint16 => Some("65535"),
            Self::Uint32 => Some("4294967295"),
            Self::Uint64 => Some("18446744073709551615"),
            _ => None,
        }
    }

    /// Signed integer types (int8..int64).
    pub fn is_signed(&self) -> bool {
        matches!(self, Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64)
    }

    /// Floating-point types (float32, float64).
    pub fn is_float(&self) -> bool {
        matches!(self, Self::Float32 | Self::Float64)
    }

    /// Bit width for fixed-width integer types — used by the RFC §synth-5-B
    /// flags primitive to bound `<sce:flag bit="N"/>` against the carrier
    /// type's domain. Returns `None` for non-integer types (float / bool /
    /// string / bytes), where bit-positioned flags have no meaning.
    pub fn int_bit_width(&self) -> Option<u32> {
        match self {
            Self::Uint8 | Self::Int8 => Some(8),
            Self::Uint16 | Self::Int16 => Some(16),
            Self::Uint32 | Self::Int32 => Some(32),
            Self::Uint64 | Self::Int64 => Some(64),
            _ => None,
        }
    }

    /// Inclusive `(min, max)` value range of a fixed-width integer type,
    /// as mathematical integers. `None` for non-integer types.
    ///
    /// The single place the Enum kind's admissible variant values are
    /// defined. `parser::parse_enum` range-checks each `<sce:variant>`
    /// against it at parse time and
    /// `event_schema_check::value_fits_underlying` narrows comparison
    /// literals against the same answer — two checks that must agree by
    /// construction, since a value the parser accepted and the narrowing
    /// layer then rejected would make a well-formed document
    /// un-comparable against its own variants.
    ///
    /// `i128` is the narrowest carrier holding both `u64::MAX` and
    /// `i64::MIN`, so no legal range is truncated here.
    pub fn int_value_range(&self) -> Option<(i128, i128)> {
        let bits = self.int_bit_width()?;
        Some(if self.is_signed() {
            let half = 1i128 << (bits - 1);
            (-half, half - 1)
        } else {
            (0, (1i128 << bits) - 1)
        })
    }
}

// ── Field direction ────────────────────────────────────────────

/// Data flow direction for kind fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    In,
    Out,
    Internal,
}

impl Direction {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "in" => Some(Self::In),
            "out" => Some(Self::Out),
            "internal" => Some(Self::Internal),
            _ => None,
        }
    }
}

// ── Typed field ────────────────────────────────────────────────

/// A typed data field common to all kinds.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ForgeField {
    pub id: String,
    pub sce_type: SceType,
    pub direction: Direction,
    /// ECMAScript expression (for computed/output fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    /// Physical-quantity annotation (`sce:quantity`).
    /// `Some({ scale, offset, unit })` when the field carries a
    /// `sce:quantity="…"` attribute. Drives type checking (units that
    /// disagree across an arithmetic operator are rejected) and codegen
    /// (raw + physical accessor pair emitted on numeric fields).
    /// Absent quantity declarations preserve the dimensionless baseline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<crate::forge::quantity::Quantity>,
    /// Per-slot capacity for `bytes`-typed fields, declared via
    /// `sce:max-size="N"`. `None` ⇒ fall back to
    /// [`crate::forge::limits::BYTES_DEFAULT_MAX`]. Ignored for non-bytes
    /// types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u32>,
}

// ── Transform kind ─────────────────────────────────────────────

/// Transform: pure mathematical formula. Input -> computation -> output.
/// No state, no side effects. Generates an inline function.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TransformModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub outputs: Vec<ForgeField>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="transform">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Lookup kind ────────────────────────────────────────────────

/// A single key-value entry in a lookup table.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct LookupEntry {
    pub key: String,
    pub value: String,
}

/// Miss-handling policy for `sce:kind="lookup"`. Captured from the
/// `sce:on-miss` attribute on the mapping `<data>` element, with `sce:default`
/// implied as `Default`. These are two orthogonal codegen strategies, not two
/// different kinds — see SCE_FORGE.md Section 4.9.
///
/// `Default(fallback)`: total function — every input produces an output, and
/// the fallback string is the value returned when no entry key matches. Used
/// by symbolic dispatch (gear positions, engine status, etc.).
///
/// `Error`: partial function — the generated `lookup(x)` returns
/// `Option<V>` / `std::optional<V>` / `(V, bool)` / `V?` depending on the
/// target language. Callers must handle the miss case explicitly. Used by
/// numeric data tables where an unknown key is a bug, not a fallback case.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", content = "value")]
pub enum MissPolicy {
    #[serde(rename = "default")]
    Default(String),
    #[serde(rename = "error")]
    Error,
}

impl MissPolicy {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }
}

/// Lookup: finite key→value mapping with an explicit miss policy.
///
/// Two orthogonal codegen strategies are chosen by the generator based on
/// `output.sce_type`:
///   * `string` output → enum + switch/case (symbolic dispatch). Every
///     unique value becomes an enum variant; input matches select the
///     variant. The `Default` policy embeds a `default:` branch; the
///     `Error` policy wraps the return in the language's optional type.
///   * non-string output → parallel `KEYS[]` / `VALUES[]` constants plus a
///     call into `sce_forge_runtime::lookup::lookup()`. The `Default` policy
///     unwraps with `or(default)`; the `Error` policy forwards the optional.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct LookupModel {
    pub name: String,
    pub input: ForgeField,
    pub output: ForgeField,
    pub entries: Vec<LookupEntry>,
    pub miss_policy: MissPolicy,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="lookup">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

impl LookupModel {
    /// The codegen strategy contract: string output → enum dispatch + switch/case;
    /// any non-string output → parallel const arrays + runtime helper. The two
    /// branches are *exclusive*, never combined. See `MissPolicy` for the
    /// orthogonal miss-handling axis. SCE_FORGE.md §4.9 documents the rationale.
    pub fn output_is_string(&self) -> bool {
        matches!(self.output.sce_type, SceType::String)
    }

    /// Collect unique output values (for enum generation), preserving insertion order.
    pub fn unique_values(&self) -> Vec<String> {
        let mut seen = std::collections::BTreeSet::new();
        let mut values = Vec::new();
        for entry in &self.entries {
            if seen.insert(entry.value.clone()) {
                values.push(entry.value.clone());
            }
        }
        values
    }

    /// Group entries by value (for combined switch cases).
    pub fn entries_by_value(&self) -> Vec<(String, Vec<String>)> {
        let mut map: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for entry in &self.entries {
            map.entry(entry.value.clone())
                .or_default()
                .push(entry.key.clone());
        }
        map.into_iter().collect()
    }
}

// ── Enum kind ──────────────────────────────────────────────────

/// A single named variant in an `sce:kind="enum"` document. Each variant
/// pairs a `name` (the source-level identifier authors reference as
/// `<EnumName>.<variant>`) with a `value` (the wire byte authors author
/// as the `value` attribute, parsed as an unsigned integer per the
/// declaring document's `sce:underlying-type`). Validation invariants
/// land at parse time:
///   * variant `name` must be unique within the document
///   * variant `value` must be unique within the document (bijectivity)
///   * variant `value` must fit within the declared `underlying_type`
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumVariant {
    pub name: String,
    /// Parsed numeric value of the variant, as a mathematical integer.
    ///
    /// `i128` is the narrowest carrier that holds every legal value of
    /// every legal `underlying_type` without per-variant tagging: the
    /// unsigned carriers reach `u64::MAX` and the signed ones reach
    /// `i64::MIN`, and no smaller primitive spans both. The
    /// document-level `underlying_type` remains the size-of-record —
    /// this field never carries a value the parser did not first prove
    /// in range for it.
    pub value: i128,
    /// Source line of the `<sce:variant>` element, captured for
    /// duplicate-value / duplicate-name / overflow diagnostics that
    /// anchor on the specific variant. `#[serde(skip)]` keeps the
    /// AST-export wire format stable.
    #[serde(skip, default)]
    pub source_line: Option<u32>,
}

/// Enum: a closed set of named variants with 1:1 wire-key mapping.
/// Distinct from
/// [`LookupModel`] (many-to-one dispatch table) by the bijectivity
/// invariant on `variants[i].value`. Surface form:
///
/// ```xml
/// <scxml sce:kind="enum" name="result" sce:underlying-type="uint8">
///   <datamodel>
///     <data id="variants">
///       <sce:variant name="ok"      value="0"/>
///       <sce:variant name="error"   value="1"/>
///       <sce:variant name="timeout" value="2"/>
///     </data>
///   </datamodel>
/// </scxml>
/// ```
///
/// Per-backend codegen lowers to `enum class :
/// <int>`, `#[repr(<int>)] enum`, `IntEnum`, etc. — single source of
/// truth on the Enum document; importers reference the emitted type
/// by qualified name rather than re-emitting variants.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumModel {
    pub name: String,
    /// Underlying integer carrier — one of `uint8`/`uint16`/`uint32`/
    /// `uint64`. Unsigned only — signed underlying types are not
    /// implemented until a consumer needs negative wire values. The
    /// parser rejects non-unsigned-integer `sce:underlying-type`
    /// values with `validation/enum-unsupported-underlying-type`.
    pub underlying_type: SceType,
    /// All variants in declaration order. Empty `variants` is
    /// rejected at parse time with `validation/enum-no-variants`.
    pub variants: Vec<EnumVariant>,
    /// Strict variant membership
    /// switch. When `true` (the default), integer literals compared
    /// against (receive-side) or assigned to (send-side) an
    /// `enum:<alias>`-typed field must be one of the values declared
    /// on this enum's `variants`. Set to `false` via
    /// `sce:strict-variants="false"` on the enum's `<scxml>` root for
    /// open-set status vocabularies (UDS NRC, OEM-extensible response
    /// codes) where wire-side values outside the declared set are
    /// legal. Width narrowing remains active regardless of this opt-
    /// out — only the membership check silent-skips when `false`.
    pub strict_variants: bool,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="enum">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── EventSchema kind ────────────────────────────────────────────

/// Typed contract for the `_event.data` payload of a named SCXML event.
/// Each schema names exactly one event (`job.completed`, `task.failed`,
/// …) and declares the typed fields authors may read via
/// `_event.data.<field>` in transition `cond` attributes and write via
/// `<send>/<param>` payloads.
///
/// Two surface forms produce identical models (inline lowering):
///   * Top-level form (primary):
///     `<scxml sce:kind="event-schema" sce:event-name="job.completed">`
///     parsed via the regular `sce:kind` dispatch.
///   * Inline form (sugar; not implemented until a consumer needs
///     it): `<sce:event-schema event-name="job.completed">` as a
///     child of `<scxml>` — parsed and lowered to an anonymous
///     top-level `EventSchemaModel` so downstream IR consumers see a
///     single shape.
///
/// Schemaless fallback: events without an imported EventSchema
/// retain the dynamic `_event.data` behavior (no diagnostic). W3C
/// built-in events (`error.*`, `done.invoke.*`, `done.state.*`) are
/// explicitly excluded — declaring a schema for these raises
/// `validation/event-schema-on-builtin-event`.
///
/// Each field uses [`ForgeField`] with `Direction::In` (payload is the
/// receiver's read-only view). `sce_type` may be a primitive
/// (Uint8/Int32/Float64/Bool/String/Bytes) or [`SceType::Enum`]
/// referring to an imported `sce:kind="enum"` document.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EventSchemaModel {
    /// Symbol name for the schema document — drives symbol mangling
    /// and import-table lookup. For the top-level surface form this is
    /// the document's basename (e.g. `job_completed_schema`); for the
    /// inline form, the parser synthesizes a deterministic name from
    /// the enclosing statechart name + event name so anonymous lowered
    /// schemas remain referenceable in diagnostics.
    pub name: String,
    /// SCXML event name this schema constrains (e.g. `"job.completed"`,
    /// `"task.failed"`). Treated case-sensitively per W3C SCXML 1.0
    /// §5.10 event-matching rules. Built-in event names (`error.*`,
    /// `done.invoke.*`, `done.state.*`) are rejected at parse time by
    /// the schema-on-builtin guard.
    pub event_name: String,
    /// Typed fields exposed via `_event.data.<field>`. Each field's
    /// `direction` is `Direction::In` (payload is the receiver's
    /// read-only view); the parser enforces this so authors cannot
    /// declare an `out`-direction field that would have no meaning on
    /// the receive side. `sce_type` may be a primitive
    /// (Uint8/Int32/Float64/Bool/String/Bytes) or
    /// [`SceType::Enum`] referring to an imported `sce:kind="enum"`
    /// document.
    pub fields: Vec<ForgeField>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="event-schema">` root element
    /// (or the synthesized location of the lowered inline form).
    /// Drives the per-kind body function's SCE-MAP marker. Same
    /// convention as `EnumModel.source_location`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

impl EventSchemaModel {
    /// W3C SCXML built-in event-name prefixes that an EventSchema may
    /// not declare against. Centralised here so the parse-time guard
    /// and any cross-kind binding pass agree on the closed set without
    /// drift. Order is documentation only — callers test membership.
    pub const BUILTIN_EVENT_PREFIXES: &'static [&'static str] = &[
        // W3C SCXML 1.0 §5.10: error.* is the error event namespace.
        // The `error.execution`, `error.communication`, `error.platform`
        // sub-events are the most-named members but the prefix matches
        // any author-extended `error.*` event the platform may raise.
        "error.",
        // W3C SCXML 1.0 §6.4: completion of an `<invoke>` raises
        // `done.invoke.<invokeid>`.
        "done.invoke.",
        // W3C SCXML 1.0 §3.7: a `<final>` state in a compound state
        // raises `done.state.<id>`; in the document root, it raises
        // `done.state.<id>` on the root.
        "done.state.",
    ];

    /// Returns true when `event_name` is reserved for the W3C SCXML
    /// platform (matches one of [`Self::BUILTIN_EVENT_PREFIXES`]).
    /// Used by the parse-time guard that emits
    /// `validation/event-schema-on-builtin-event`. Exact match
    /// against a prefix-stripped tail is unnecessary — any event whose
    /// name *begins* with a built-in prefix lives in that namespace,
    /// even if the platform has not yet raised the specific
    /// `error.<author-defined>` variant.
    pub fn is_builtin_event_name(name: &str) -> bool {
        Self::BUILTIN_EVENT_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
    }
}

// ── Condition kind ─────────────────────────────────────────────

/// Condition: named boolean guard expression. Generates an inline bool function.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ConditionModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    /// ECMAScript expression that evaluates to boolean.
    pub expr: String,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="condition">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Validator kind ─────────────────────────────────────────

/// Validator: range/rate-of-change/plausibility checks.
/// Has internal state (previous values for rate-of-change detection).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ValidatorModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub rules: ValidatorRules,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="validator">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// Validation rules container.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ValidatorRules {
    pub ranges: Vec<RangeRule>,
    pub rate_of_changes: Vec<RateOfChangeRule>,
    pub plausibility: Option<String>,
}

/// Range check: field value must be within [min, max].
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct RangeRule {
    pub id: String,
    pub min: Option<String>,
    pub max: Option<String>,
}

/// Rate-of-change check: delta between successive calls must not exceed max_delta.
/// sample_interval_ms is informational (documents expected call frequency).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct RateOfChangeRule {
    pub id: String,
    pub max_delta: String,
    pub sample_interval_ms: u32,
}

// ── Procedure kind ────────────────────────────────────────────

// ── Level 2 types (event-driven procedure) ───────────────────

/// A `<send>` action within `<onentry>` of a procedure state.
/// Dispatches a service request through the procedure's service handler.
/// §scxml-6.2 + SCE extensions (sce:service, sce:subfunc, sce:addr, sce:payload).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureSendAction {
    /// Service name (sce:service attribute). Required.
    pub service: String,
    /// Sub-function code (sce:subfunc attribute). Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subfunc: Option<String>,
    /// Address expression — typically a variable name (sce:addr attribute). Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    /// Payload expression (sce:payload attribute). Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// Cap on the bytes the service handler may return as `_event.data`,
    /// declared via `sce:response-max-size="N"`. `None` ⇒ fall back to
    /// [`crate::forge::limits::BYTES_DEFAULT_MAX`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_max_size: Option<u32>,
}

/// An `<assign>` action within a `<transition>` body.
/// Mutates internal state during transition execution.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureAssign {
    /// Target variable name (location attribute).
    pub location: String,
    /// Value expression (expr attribute).
    pub expr: String,
}

/// A `<param>` within `<donedata>` on a `<final>` state.
/// Provides result data when the procedure completes.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureDoneParam {
    /// Parameter name.
    pub name: String,
    /// Value expression.
    pub expr: String,
}

// ── Shared types (Level 1 + Level 2) ─────────────────────────

/// A single transition within a procedure state.
/// Level 1: guard-only (cond + target).
/// Level 2: event-driven (event + cond + target + assigns).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureTransition {
    /// Target state id.
    pub target: String,
    /// Optional ECMAScript guard expression. `None` = unconditional (else branch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cond: Option<String>,
    /// Event trigger (Level 2). `None` = eventless transition (guard-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Assign actions executed during transition (Level 2). Empty for Level 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assigns: Vec<ProcedureAssign>,
    /// 1-based source line of the `<transition>` element. Populated by
    /// `parse_procedure_transitions` so post-loop validators (e.g. the
    /// transition target reference check) can anchor diagnostics at the
    /// offending element rather than its parent `<state>`. Skipped from
    /// serialization to preserve manifest wire-format byte-stability.
    #[serde(skip)]
    pub line: Option<u32>,
}

/// A state within a procedure (either regular or final).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureState {
    pub id: String,
    /// Whether this is a `<final>` state (terminal — no outgoing transitions).
    pub is_final: bool,
    /// Ordered transitions (evaluated top-to-bottom). Empty for final states.
    pub transitions: Vec<ProcedureTransition>,
    /// Send actions in `<onentry>` (Level 2). Empty for Level 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub on_entry_sends: Vec<ProcedureSendAction>,
    /// Done data parameters (Level 2, final states only). Empty for Level 1.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done_params: Vec<ProcedureDoneParam>,
    /// 1-based source line of the `<state>`/`<final>` element. Populated by
    /// `parse_procedure` so the post-loop "non-final state with no
    /// transitions" check can anchor at the offending element without
    /// requiring a twin `state_nodes` vector. Skipped from serialization
    /// to preserve manifest wire-format byte-stability.
    #[serde(skip)]
    pub line: Option<u32>,
}

/// A user-declared helper function referenced by procedure expressions
/// (typically from `sce:payload` / `sce:addr` attributes). Declared as
/// `<sce:helper name="..." args="..." returns="..."/>` inside the procedure's
/// `<datamodel>`. The generator emits a typed closure member + setter per
/// helper, mirroring the existing `serviceHandler` dependency-injection
/// pattern — rather than emitting the user's identifier verbatim and relying
/// on it being in scope at compile time. The helper's signature seeds the
/// expression-pipeline type context so inference can propagate return types
/// through enclosing expressions.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureHelper {
    /// User-visible identifier as it appears in expressions
    /// (e.g. `computeKey` in `sce:payload="computeKey(seed)"`).
    pub name: String,
    /// Parameter types in positional order.
    pub args: Vec<SceType>,
    /// Return type.
    pub returns: SceType,
    /// Cap on the bytes the helper closure may return when `returns =
    /// bytes`, declared via `sce:returns-max-size="N"`. `None` ⇒ fall
    /// back to [`crate::forge::limits::BYTES_DEFAULT_MAX`]. Ignored
    /// when `returns` is non-bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns_max_size: Option<u32>,
}

/// Procedure: sequential branching logic with states and guarded transitions.
/// Level 1 (guard-only): CRTP base + switch/case, stateless `execute()`.
/// Level 2 (event-driven): StaticExecutionEngine<Policy> + `runToCompletion()`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ProcedureModel {
    pub name: String,
    /// Input parameters (sce:direction="in").
    pub inputs: Vec<ForgeField>,
    /// Internal state variables (sce:direction="internal"). Level 2 only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub internals: Vec<ForgeField>,
    /// User-declared helper function DI points (see [`ProcedureHelper`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub helpers: Vec<ProcedureHelper>,
    /// Id of the initial state (from `initial` attribute on `<scxml>`).
    pub initial: String,
    /// All states in document order (regular + final).
    pub states: Vec<ProcedureState>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="procedure">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

impl ProcedureModel {
    /// Whether this procedure is Level 2 (event-driven).
    ///
    /// A procedure is L2 if it uses any feature that requires the
    /// `sce_forge_runtime::procedure` execution engine: internal state,
    /// helper DI points, `<send>` entry actions, or `<donedata>` on final
    /// states. L1 procedures are pure guard-only diamond flows with zero
    /// runtime dependency.
    pub fn is_l2(&self) -> bool {
        !self.internals.is_empty()
            || !self.helpers.is_empty()
            || self
                .states
                .iter()
                .any(|s| !s.on_entry_sends.is_empty() || !s.done_params.is_empty())
    }
}

// ── Codec kind ─────────────────────────────────────────────────

/// Endianness for byte-level operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Endian {
    #[default]
    Big,
    Little,
    Native,
}

impl Endian {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "big" => Some(Self::Big),
            "little" => Some(Self::Little),
            "native" => Some(Self::Native),
            _ => None,
        }
    }
}

/// RFC §synth-5-B B2 repeat primitive — element count source for a
/// `BitSize::Repeat` field.
///
/// Two shapes:
///   - [`CountRef::LengthField`] — count comes from a sibling integer
///     field declared earlier in the same codec (the count value is
///     the decoded integer). Mirrors the `<sce:repeat sce:count="<id>"/>`
///     authoring form.
///   - [`CountRef::UntilEof`] — count is implicit; the decoder consumes
///     elements until the cursor's remaining bytes are exhausted. The
///     last element MUST decode cleanly off a frame boundary; a partial
///     final element returns `NeedMoreBytes`. Mirrors `<sce:repeat
///     sce:until-eof="true"/>`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "type")]
pub enum CountRef {
    /// Sibling integer field id whose decoded value names the count.
    /// Validated at parse time against forward references and against
    /// non-integer-typed targets.
    LengthField(String),
    /// Greedy consume-remaining; element count is whatever fits
    /// before cursor exhaustion.
    UntilEof,
}

/// RFC §synth-5-B B3 TLV chain on-overflow policy. Names what the decoder does
/// when the cursor still has bytes after `max_depth` entries have been
/// consumed (i.e. the wire carries more entries than the codec author
/// declared).
///
/// v1 ships `Reject` + `Truncate`. `DiagnosticEvent` (RFC line 488) is
/// deferred until §synth-5-A diagnostic-event runtime infrastructure surfaces a
/// reachable consumer — adding it now would be built-but-unconsumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TlvOverflowPolicy {
    /// Return [`crate::forge::limits`]-sized `TlvChainOverflow` typed
    /// error. Caller treats the frame as corrupt.
    Reject,
    /// Silently drop the post-cap bytes from the decoded list. Cursor
    /// is left advanced past whatever was consumed; the caller may
    /// inspect `cursor.remaining()` for the residual.
    Truncate,
}

/// RFC §synth-5-B — TLV chain termination strategy. Names how the chain
/// decoder decides where the chain ends; pairs with the existing
/// `max_depth` safety bound + `on_overflow` policy. v1 has two shapes:
///
///   - [`TlvTerminateStrategy::ExhaustOrDepth`] (trunk default): the
///     chain consumes entries until the cursor exhausts or `max_depth`
///     is reached. Used by `codec_tlv_chain_basic` + the
///     `codec_zenoh_ext_envelope` demo. Acceptable when nothing
///     follows the chain on the wire — the trailing cursor IS the
///     chain's tail.
///
///   - [`TlvTerminateStrategy::EntryFlag`]: each entry's outer
///     header carries a "next-entry follows" boolean flag (zenoh-pico
///     `_Z_FLAG_Z_Z = 0x80` at bit 7 in every ext_msg header). The
///     chain decoder reads the flag *after* each entry and stops the
///     loop when it's clear, leaving the cursor at the start of
///     whatever follows the chain on the wire (e.g. zenoh-pico
///     request body's query/put/del variant after the ext chain).
///     Mirrors `_z_msg_ext_decode_iter` (ext.c:226-238) verbatim.
///     `flag_name` references a flag declared on the entry codec's
///     flags carrier (typically the entry's outer header byte). The
///     codec author guarantees the entry codec exposes a flags-bearing
///     carrier with that flag — codegen reads `entry.<flag_name>()`
///     after each decode.
///
/// `max_depth` and `on_overflow` apply uniformly across both
/// strategies — they bound the worst case where the wire stream lies
/// about chain length. What differs is the evidence each strategy has
/// that the wire lied:
///
///   - under `ExhaustOrDepth` the chain owns the codec's remaining wire,
///     so bytes left after the loop are themselves the overflow;
///
///   - under `EntryFlag` other fields follow the chain, so leftover bytes
///     prove nothing and the evidence is the flag: the last entry
///     decoded declared a successor the loop did not take. That happens
///     for two disjoint reasons and they are different failures — with
///     bytes still on the cursor the depth cap refused an entry the peer
///     sent, which is the overflow `on_overflow` governs; with the
///     cursor empty the peer's frame simply ended mid-chain, which is a
///     short frame and is reported as such whatever `on_overflow` says,
///     because no policy asks a decoder to invent the entry the wire
///     promised.
///
/// Neither is inferable from a downstream field's failure: whether the
/// next field's read happens to fail on a refused entry's bytes is a
/// property of the peer's wire, not of the codec.
#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "type")]
pub enum TlvTerminateStrategy {
    /// Trunk default: chain ends on cursor exhaustion or `max_depth`.
    #[default]
    ExhaustOrDepth,
    /// Chain ends when the last-decoded entry's named flag bit is
    /// clear. Forward-compat with multi-ext zenoh wire format.
    EntryFlag { flag_name: String },
}

fn is_exhaust_or_depth(s: &TlvTerminateStrategy) -> bool {
    matches!(s, TlvTerminateStrategy::ExhaustOrDepth)
}

/// Bit size specification for codec fields.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "type")]
pub enum BitSize {
    /// Fixed bit count: 8, 16, 24, 32, 64.
    Fixed { bits: u32 },
    /// Remaining bytes from byte offset to end of frame.
    Tail,
    /// Size determined by another field's value (see CodecField::length_field).
    LengthRef,
    /// Variable-length encoded integer (RFC §synth-5-B Appendix B). The
    /// `width_bits` cap (16/32/64) names the value-type max width; the
    /// wire occupies at most `ceil((width_bits - 1) / 7)` bytes — the
    /// canonical Zenoh ZInt cap, in which the final byte carries a full
    /// 8 data bits so a u64 tops out at 9 bytes (not 10). Canonical
    /// Zenoh ZInt is `Vle { width_bits: 64 }`.
    Vle { width_bits: u32 },
    /// RFC §synth-5-B B2 repeat primitive — list of imported-codec elements.
    /// The decoded host type is `Vec<T>` / `std::vector<T>` / ... where
    /// T is resolved via the field's [`CodecField::repeat_body_alias`].
    /// `count_ref` selects the loop termination strategy
    /// (length-field vs until-eof).
    Repeat { count_ref: CountRef },
    /// RFC §synth-5-B B3 TLV chain primitive — bounded extension list
    /// (Type-Length-Value). Iteratively decodes entries up to
    /// `max_depth` (RFC: "iterative, never recursive"); residual bytes
    /// after the cap are handled per [`TlvOverflowPolicy`]. Entry body
    /// codec is resolved via [`CodecField::tlv_chain_body_alias`]
    /// (mirrors `repeat_body_alias`). MCU-class — emits only on Rust +
    /// C11; cpp/kotlin/go/python codecs containing this field type are
    /// rejected at codegen via `codegen/mcu-class-kind-on-non-mcu-language`
    /// (the existing kind-class diagnostic is repurposed at the
    /// codec-content granularity, see RFC §synth-5-B "MCU-only codec
    /// sub-features").
    TlvChain {
        max_depth: u32,
        on_overflow: TlvOverflowPolicy,
        /// RFC §synth-5-B — termination strategy. Defaults to
        /// `ExhaustOrDepth` for backward-compat with trunk fixtures
        /// (cursor-exhaust + max_depth). `EntryFlag(flag_name)` reads
        /// the entry's named flag after each decode and terminates
        /// the loop when the flag is clear — required when something
        /// follows the chain on the wire (zenoh-pico ext chains
        /// followed by message body variants).
        #[serde(skip_serializing_if = "is_exhaust_or_depth", default)]
        terminate_on: TlvTerminateStrategy,
    },
    /// RFC §synth-5-B embed primitive — single imported-codec field embedded inline.
    /// The host language emits a nested struct of the imported codec's
    /// type; the streaming codec calls the imported codec's
    /// decode/encode for this position. No wire-level boundary bytes
    /// (no length prefix, no tag) — the embedded codec's own field
    /// layout consumes/produces bytes directly. Imported codec is
    /// resolved via [`CodecField::embed_body_alias`] (mirrors
    /// `repeat_body_alias`). RFC §synth-5-B parent-flag threading
    /// applies when the embedded codec declares
    /// `<sce:requires-parent-flags>`.
    ///
    /// First reachable consumer: zenoh-pico declare/undeclare bodies
    /// (decl_keyexpr, decl_subscriber, decl_queryable, decl_token)
    /// embed `codec_zenoh_wireexpr` after a VLE id field.
    Embed,
}

/// Canonical Zenoh ZInt byte cap for a `width_bits`-wide VLE field: the
/// final byte carries a full 8 data bits (the continuation flag is
/// reused as the high data bit), so a W-bit value occupies at most
/// `ceil((W - 1) / 7)` bytes — one fewer than naive base-128 when `W`
/// is a multiple of 8. u64 -> 9 (not 10), u32 -> 5, u16 -> 3 (RFC
/// §synth-5-B App. B). The single byte-count SSOT shared by
/// [`CodecModel::max_frame_bytes`] (worst-case encode-buffer sizing)
/// and the generator's VLE encode/`vle_max_bytes` emission, so worst-
/// case sizing and the emitted continuation-loop bound never diverge.
pub(crate) fn vle_cap_bytes(width_bits: u32) -> u32 {
    (width_bits - 1).div_ceil(7)
}

/// A single named bit-range on a `<sce:flags>` carrier field — RFC §synth-5-B
/// flags primitive. `bit` is the LSB position within the carrier's natural
/// integer width (0 = LSB), validated at parse time against the
/// carrier's [`SceType::int_bit_width`]. `width` is the contiguous
/// bit-range size (defaults to 1 for the single-bit shape; >1 for
/// multi-bit accessors like Zenoh's `_z_n_qos_t._val.priority:3`).
/// `bit + width <= carrier_int_bit_width` is parser-enforced; per-flag
/// bit-ranges within the same carrier may not overlap.
///
/// `value` is the optional wire-constant baked into the carrier's
/// `Default::default()` (variant default-arm uniformity).
/// Authors declare this on an inner codec's MID flag so the codec's
/// default instance is wire-valid for its own dispatch tag — required
/// for round-trip uniformity when the codec is referenced as the
/// declared default arm of an outer `<sce:variant>`. The masked
/// constant `(value & ((1 << width) - 1)) << bit` ORs into the
/// carrier's Default. `None` ⇒ no constant, Default zero-fills the
/// bit-range (back-compat with goldens that predate the attribute). Stored as
/// `u64` to fit any carrier width up to uint64.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct FlagDef {
    pub name: String,
    pub bit: u32,
    pub width: u32,
    /// Optional wire-constant — see field doc. Skipped from
    /// serialization when `None` so goldens that predate this
    /// attribute keep their on-disk shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
}

/// RFC §synth-5-B present-if predicate scope —
/// distinguishes the local form (carrier in same codec) from the
/// flag-input form (bare name resolves to a declared
/// `<sce:flag-input>`, passed as a positional typed parameter).
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum PresentIfScope {
    /// `<field_id>.<flag_name>` — local form. Carrier is a
    /// flags-bearing sibling field declared earlier in the same
    /// codec; predicate reads `(self.<carrier> & mask) != 0`.
    ///
    /// `#[default]` (Local) matches the existing `is_local_scope`
    /// skip predicate so the schema-vs-wire alignment holds: when
    /// scope is Local, emit skips; when emit skips, deserialise
    /// rehydrates to Local.
    #[default]
    Local,
    /// `<name>` (bare, no dot) — flag-input form. Resolves to a
    /// declared `<sce:flag-input name="X">` on the codec itself; the
    /// input arrives as a typed positional parameter (`X: u8` for v1
    /// width=1). Predicate reads `(X & ((1<<width)-1)) != 0` against
    /// the parameter directly — no carrier-byte threading, no struct
    /// prefix.
    Input,
}

/// RFC §synth-5-B present-if predicate — a single
/// bit-test on either a flags-bearing sibling field declared earlier
/// in the same codec (Local scope) or a declared
/// `<sce:flag-input>` on the codec itself (Input scope).
///
/// v1 grammar covers four forms:
///   - `<field_id>.<flag_name>` (Local positive) — predicate is true
///     iff the named flag bit on the local carrier is set.
///   - `!<field_id>.<flag_name>` (Local negative) — predicate
///     is true iff the bit is clear. Required for Zenoh OpenSyn body
///     where cookie is present iff the `A` input is NOT set.
///   - `<name>` (Input positive) — true iff the named `<sce:flag-input>`
///     value is non-zero. Single-bit input (v1 width=1).
///   - `!<name>` (Input negative) — true iff the input is zero.
///   - `<a> || <b> [|| <c> ...]` (Disjunction) —
///     predicate is true iff ANY listed clause is true. Each clause
///     is itself one of the four forms above (each can independently
///     carry a leading `!`). Required for Zenoh interest where
///     `not is_final` is `header.CURRENT || header.FUTURE` —
///     `_Z_INTEREST_NOT_FINAL_MASK = (CURRENT | FUTURE)` per
///     `interest.h:35`. Outer negation `!(a || b)` is not
///     implemented until a consumer needs it.
///
/// `field_id` is empty when `scope = Input` (carrier is implicit —
/// the codec's own `<sce:flag-input>` parameter).
///
/// Conjunction (`flag1 && flag2`) and equality (`field == value`)
/// are not implemented until a reachable consumer
/// surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct PresentIfPredicate {
    /// Predicate scope. Defaults to `Local`; serialized only for
    /// non-Local scopes (`Input`) to keep the legacy local-scope
    /// JSON shape byte-stable.
    #[serde(default, skip_serializing_if = "is_local_scope")]
    pub scope: PresentIfScope,
    /// Carrier field id when `scope = Local`. Empty when `scope =
    /// Input` (the carrier is the codec's own `<sce:flag-input>`
    /// parameter named by `flag_name`).
    pub field_id: String,
    pub flag_name: String,
    /// Negation: when true, predicate fires when the named flag
    /// bit is *clear* (not set). Defaults to false to keep the
    /// JSON shape of all existing codec
    /// goldens byte-stable (skip_serializing_if elides the field).
    #[serde(skip_serializing_if = "is_false", default)]
    pub negate: bool,
    /// RFC §synth-5-B disjunction tail — when `Some`, the
    /// composite predicate is `<self> || <or_with>` (i.e. the named
    /// flag tests on this struct OR'd with the recursive tail).
    /// `or_with` is itself a `PresentIfPredicate`, so chains of
    /// length ≥ 3 (`a || b || c || ...`) compose by recursion. None
    /// for the v1 single-clause grammar (back-compat: existing
    /// goldens omit this field via skip_serializing_if).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub or_with: Option<Box<PresentIfPredicate>>,
}

fn is_local_scope(scope: &PresentIfScope) -> bool {
    matches!(scope, PresentIfScope::Local)
}

/// Caller-supplied flag input — a named flag-shaped input the codec receives
/// from its caller. Replaces the legacy `<sce:requires-parent-flags>`
/// shape: leaf declares logical input names + bit widths only,
/// with no claim about the parent's carrier name or bit position.
///
/// Authored as `<sce:flag-inputs><sce:flag-input name="X" width="N"/>
/// </sce:flag-inputs>` under `<scxml sce:kind="codec">`. The leaf's
/// `decode`/`encode` signature gains one typed parameter per declared
/// input (positional order = declaration order). Predicates inside the
/// leaf body reference the input by bare name (e.g.
/// `sce:present-if="X"`), evaluating `X != 0` on the local typed
/// parameter — no carrier-byte threading, no shift+mask.
///
/// The parent codec, at its `<sce:import>` site, supplies values via
/// `<sce:flag-bind input="X" source="Y.Z"/>` directives — see
/// [`FlagBind`].
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct FlagInput {
    /// Logical input name. Author-visible in predicates
    /// (`present-if="<name>"`) and as a positional parameter in the
    /// generated `decode`/`encode` signatures.
    pub name: String,
    /// Bit-width of the input value. v1 lock-in: width = 1 (single-bit
    /// flag carrying 0/1). Future widening to width > 1 (multi-bit
    /// dispatch inputs) defers to a reachable consumer.
    pub width: u32,
}

/// Parent-side flag binding — a parent codec's binding of one of its own
/// flags to a leaf's declared flag-input. Lives on `<sce:import>` as a
/// child element: `<sce:import src="..." as="..."><sce:flag-bind
/// input="X" source="Y.Z"/></sce:import>`.
///
/// `source` resolves via the standard dotted-form rule:
///   - `<carrier>.<flag>` ⇒ parent has a local `<sce:flags id="<carrier>">`
///     declaring `<sce:flag name="<flag>" bit=... width=.../>`. Codegen
///     extracts `(carrier_value >> bit) & ((1 << width) - 1)` at the
///     embed call-site and threads it as the leaf's named parameter.
///   - `<name>` (no dot) ⇒ parent has a local `<sce:flag-input name=
///     "<name>" width=.../>` of its own (chain-forwarder pattern). The
///     parent's own input is threaded into the leaf's input by name.
///
/// Cross-doc validator (`validate_cross_codec_flag_bind`, parent-local)
/// confirms each `<sce:flag-bind>` resolves to a parent-side flag-input
/// or carrier+flag; every leaf-declared input is bound exactly once;
/// widths agree.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct FlagBind {
    /// Leaf-side input name being supplied.
    pub input: String,
    /// Parent-side source kind, resolved at parse time.
    pub source: FlagBindSource,
}

/// Resolved flag-bind source. Two variants per the
/// dotted-form rule documented on [`FlagBind`].
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FlagBindSource {
    /// `source="<carrier>.<flag>"` — parent's local flags-carrier.
    Carrier { carrier: String, flag: String },
    /// `source="<input>"` — parent's own flag-input (chain forwarder).
    Input { name: String },
}

/// A single field in a codec's byte layout.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CodecField {
    pub id: String,
    pub sce_type: SceType,
    /// Byte offset within the frame.
    pub byte_offset: u32,
    /// Bit offset within the byte (for sub-byte fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_offset: Option<u32>,
    /// Bit size of this field.
    pub bit_size: BitSize,
    /// Per-field endianness override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endian: Option<Endian>,
    /// Maximum size for variable-length fields (tail, length-ref).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u32>,
    /// Referenced field ID for LengthRef bit size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_field: Option<String>,
    /// Named bits — RFC §synth-5-B flags primitive. Empty for plain
    /// fields; populated when the field was authored as a `<sce:flags>`
    /// container with `<sce:flag name=... bit=N/>` children. Codegen
    /// emits per-flag get/set accessors after the encode/decode methods
    /// without changing the wire layout (the field still occupies the
    /// same bytes as a regular unsigned-int).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub flags: Vec<FlagDef>,
    /// RFC §synth-5-B present-if primitive. `Some(predicate)` when the
    /// field carries a `sce:present-if="<carrier>.<flag>"` attribute;
    /// the field's host-language type is wrapped as a per-language
    /// optional (`Option<T>` / `std::optional<T>` / `T?` / `*T` /
    /// `Optional[T]` / `bool has_<id>; T <id>` paired in C11) and the
    /// streaming decode/encode skips the field's bytes when the
    /// predicate is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub present_if: Option<PresentIfPredicate>,
    /// RFC §synth-5-B B2 repeat primitive — imported codec alias whose
    /// decode/encode handles each element. `Some(alias)` only when
    /// `bit_size = BitSize::Repeat`. Resolved against `<sce:import>`
    /// aliases at codegen time (mirrors variant arm body_alias
    /// resolution).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repeat_body_alias: Option<String>,
    /// RFC §synth-5-B B2 maximum element count for `BitSize::Repeat` fields
    /// — used by encode-buffer sizing to bound `min_frame + count *
    /// element_max`. Defaults to [`crate::forge::limits::REPEAT_DEFAULT_MAX_COUNT`]
    /// when absent (mirrors `max_size`'s fallback for Tail/LengthRef
    /// bytes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u32>,
    /// RFC §synth-5-B B3 TLV chain primitive — imported codec alias whose
    /// decode/encode handles each entry. `Some(alias)` only when
    /// `bit_size = BitSize::TlvChain`. Resolved against `<sce:import>`
    /// aliases at codegen time (mirrors `repeat_body_alias`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlv_chain_body_alias: Option<String>,
    /// RFC §synth-5-B B3 DMA alignment primitive — `sce:dma-burst-align="N"`
    /// declares this field's start offset within the encoded buffer is
    /// constrained to an N-byte boundary (typical N: 16, 32, 64). The
    /// constraint is build-time: codegen verifies `byte_offset % N == 0`
    /// (rejects with `codec/dma-alignment-unsatisfiable` otherwise) AND
    /// validates that every preceding field is Fixed bit-size (so the
    /// post-padding layout is statically computable). Codegen emits
    /// language-level alignment assertions (`_Static_assert` / `const _:
    /// () = assert!`) on the literal offset for drift protection.
    /// MCU-class — codecs containing this attribute emit only on Rust +
    /// C11. TLV chain is no longer MCU-only (Zenoh
    /// extension envelopes ship on server-class peers too); DMA align
    /// is the only remaining codec sub-feature gated to MCU backends.
    /// `None` when no alignment constraint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dma_burst_align: Option<u32>,
    /// RFC §synth-5-B — embedded imported-codec alias for
    /// `BitSize::Embed` fields. `Some(alias)` only when bit_size is
    /// Embed; mirrors `repeat_body_alias` / `tlv_chain_body_alias`
    /// for the single-codec inline case. Resolved against
    /// `<sce:import>` aliases at codegen time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_body_alias: Option<String>,
    /// RFC §synth-5-B — `sce:length-from="<id>"` on a `BitSize::Embed`
    /// field bounds the embedded codec's decode-time cursor scope to
    /// the named sibling field's decoded value (a prior-position
    /// integer field — typically a VLE total-length prefix). The
    /// embedded codec consumes exactly that many bytes via an
    /// inner-cursor sub-window; encode side trusts the author to set
    /// the length sibling consistently with the embedded codec's
    /// emitted byte count (mirrors the `LengthRef` author-trust
    /// contract — round-trip correctness is the author's
    /// responsibility, not codec-derived).
    ///
    /// First reachable consumer: zenoh-pico
    /// `_z_decl_ext_keyexpr_encode` (declarations.c:38-50) where the
    /// outer envelope's VLE total-length prefix bounds the inner
    /// `wireexpr`-shaped body (inner_header + VLE id + suffix-Tail).
    /// `None` for the always-present always-cursor-direct embed shape.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_length_from: Option<String>,
    /// RFC §synth-5-B — arithmetic offset on the
    /// `length-field` source value. Authored as `sce:length-arith="+1"`
    /// or `sce:length-arith="-1"` paired with `sce:bit-size="length-ref"` +
    /// `sce:length-field="..."`. Effective payload length is
    /// `sibling_value + length_arith` bytes.
    ///
    /// First reachable consumer: zenoh-pico Scout/Hello/Init `zid`,
    /// where the wire stores `zid_len_m1 = actual_len - 1` to pack the
    /// length into 4 bits, and decode reconstructs `actual_len =
    /// zid_len_m1 + 1`.
    ///
    /// v1 grammar restricts the offset to `±1` (parser rejects 0 and
    /// `|x| > 1`). Widening defers to a reachable consumer.
    /// `length-arith` requires `length-field`; standalone `length-arith`
    /// without a sibling reference is rejected at parse time. Author
    /// trust contract: payload length stays `len_sibling + arith` bytes
    /// across encode/decode round-trips (mirrors the variant tag/body
    /// trust contract of the variant primitive).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_arith: Option<i32>,
    /// Physical-quantity annotation on
    /// codec fields. ARXML COMPU-METHOD blocks map onto this layer:
    /// the raw wire-level integer carries a linear `physical = raw *
    /// scale + offset` conversion in a named unit. Codegen emits a
    /// physical accessor (`<field>_phys()` / `set_<field>_phys()`)
    /// alongside the existing raw field. Absent annotation preserves
    /// the legacy byte-identical codec emission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<crate::forge::quantity::Quantity>,
}

impl CodecField {
    /// Effective endianness (field override or document default).
    pub fn effective_endian(&self, default: Endian) -> Endian {
        self.endian.unwrap_or(default)
    }

    /// Whether this is a variable-length field.
    pub fn is_variable_length(&self) -> bool {
        matches!(
            self.bit_size,
            BitSize::Tail
                | BitSize::LengthRef
                | BitSize::Vle { .. }
                | BitSize::Repeat { .. }
                | BitSize::TlvChain { .. }
                | BitSize::Embed
        )
    }

    /// Whether this field uses VLE encoding (consumes a streaming
    /// 1..=ceil(N/7) bytes from the cursor instead of a fixed window).
    pub fn is_vle(&self) -> bool {
        matches!(self.bit_size, BitSize::Vle { .. })
    }

    /// Whether this field is a repeat-of-imported-codec list (RFC §synth-5-B
    /// B2). The host language emits `Vec<T>` / `std::vector<T>` / etc.
    /// and the streaming codec iterates element decode/encode according
    /// to [`CountRef`].
    pub fn is_repeat(&self) -> bool {
        matches!(self.bit_size, BitSize::Repeat { .. })
    }

    /// Whether this field is a TLV chain (RFC §synth-5-B B3). The host
    /// language emits `Vec<T>` (Rust) / fixed-array + len pair (C11)
    /// and the streaming codec iterates element decode/encode up to
    /// `max_depth` then applies the [`TlvOverflowPolicy`]. MCU-class
    /// — codecs containing this field type emit only on Rust + C11.
    pub fn is_tlv_chain(&self) -> bool {
        matches!(self.bit_size, BitSize::TlvChain { .. })
    }

    /// Whether this field is a single-codec embed (RFC §synth-5-B).
    /// The host language emits a nested struct of the imported codec's
    /// type; the streaming codec calls the imported codec's
    /// decode/encode for this position with no wire-level boundary.
    pub fn is_embed(&self) -> bool {
        matches!(self.bit_size, BitSize::Embed)
    }

    /// Whether this field carries a `<sce:flag>` set (RFC §synth-5-B).
    /// Used by the present-if validator to verify a predicate's
    /// LHS resolves to a flags-bearing carrier.
    pub fn is_flags_carrier(&self) -> bool {
        !self.flags.is_empty()
    }

    /// Whether this field is a UTF-8 string (RFC §synth-5-B).
    /// Wire shape mirrors a length-prefixed bytes field; the host-
    /// language type is `String` / `std::string` / `kotlin.String` /
    /// `string` / `str`. Decode validates UTF-8 and emits typed
    /// `CodecError::InvalidUtf8` (Rust / Go / Python) or returns the
    /// truncation sentinel (Cpp / Kotlin) on malformed input. Parser
    /// constrains String fields to `BitSize::LengthRef` only.
    pub fn is_string(&self) -> bool {
        matches!(self.sce_type, SceType::String)
    }

    /// Fixed bit count, or None for variable-length.
    pub fn fixed_bits(&self) -> Option<u32> {
        match &self.bit_size {
            BitSize::Fixed { bits } => Some(*bits),
            _ => None,
        }
    }
}

/// One arm of a discriminated-union variant suffix on a codec.
/// `body_alias` references an `<sce:import>` alias whose imported codec
/// type provides the arm's body. RFC §synth-5-B "Discriminated union":
/// `<sce:arm value="0x01" type="SessionOpen"/>` where `SessionOpen` is
/// an imported codec alias (v1 limits arm bodies to imported codec
/// kinds; primitive arm bodies and `<sce:default>` body inheritance
/// arrive when their first reachable consumer ships).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct VariantArm {
    /// Discriminator value (matches the tag field's read value).
    /// Held as `u64` to fit any unsigned tag width up to uint64.
    pub value: u64,
    /// Import alias naming the body codec for this arm.
    pub body_alias: String,
    /// `default="true"` marker — declares this arm as the default
    /// chosen by the outer codec's `Default::default()` (variant
    /// default-arm uniformity). Distinct from the
    /// catch-all `<sce:default>` arm (`CodecVariant::default_arm`) —
    /// that one fires on decode for unknown tag values; this flag
    /// only picks which `<sce:arm>`'s body type becomes the
    /// Default-trait starting value. At most one arm per variant
    /// may set this (parser-enforced via
    /// `codec/variant-duplicate-default-arm`). `false` ⇒ not the
    /// default arm; serialized only when `true` to keep
    /// goldens that predate the attribute byte-identical.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_default: bool,
}

/// RFC §synth-5-B peek-byte — peek-byte dispatch shape on a
/// `<sce:variant>`. When `Some` on `CodecVariant.peek_byte`, the tag
/// reads the cursor's NEXT byte without advancing; the arm body codec
/// then reads that same byte as its own header. Models Zenoh
/// response/request body dispatch where the next-byte MID identifies
/// the inner body codec (per `network.c:347-364` + `220-235` —
/// `_z_uint8_decode(&inner_header, ...)` consumed by outer decoder
/// then passed by VALUE to arm body decoder).
///
/// `id` names the peek slot — referenced by the variant's
/// `tag="<id>.<flag>"` dotted form (parser-enforced — peek mode tag
/// MUST be dotted, MUST equal `peek_byte.id` for the carrier half).
///
/// `flags` mirrors `<sce:flags>`-style children: `<sce:flag name="X"
/// bit="N" width="W"/>`. v1 fixes the peeked width at uint8 (Zenoh
/// single-byte network dispatch); future widening to peek-multi-byte
/// is a separate primitive (peek-bytes) when a reachable consumer
/// surfaces, NOT a `sce:type` widening on this element.
///
/// Cross-codec validator confirms the dispatch flag (the one named
/// in the variant's tag) matches every arm body codec's own first
/// flag-bearing field's `<sce:flag>` declaration of the same
/// (name, bit, width) — the peeked byte == arm body's own header
/// byte, so the flag layout must agree on the dispatch bit-range.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct PeekByteSpec {
    /// Logical name for the peek slot. Used as the carrier half of
    /// the variant's `tag="<id>.<flag>"` dotted form. Not a wire
    /// field — the byte stays on the cursor for the arm body to read.
    pub id: String,
    /// Per-flag bit-range layout on the peeked byte. Mirrors
    /// `<sce:flags>` semantics — unique flag names + non-overlapping
    /// bit-ranges within `[0, 8)` (peek width is uint8).
    pub flags: Vec<FlagDef>,
}

/// RFC §synth-5-B — variant tag scope. Determines whether the tag
/// field reference is resolved within the codec itself (`Local` —
/// own-field / multi-bit-flag / peek-byte) or against the codec's
/// `<sce:requires-parent-flags>` carrier (`Parent`).
///
/// `Local` is the back-compat default and skips serialization so
/// Discriminated-union suffix on a codec — RFC §synth-5-B Codec DSL.
///
/// Decode reads the named tag field (or named flag bit-range within it,
/// when `tag_flag` is set), then dispatches into the matching arm's
/// body codec. Encode writes the tag bytes followed by the active
/// arm's body bytes. The optional `<sce:default>` arm catches any tag
/// value not enumerated; absent default + non-exhaustive arm coverage
/// fires `codec/variant-arm-unreachable` at build time (see RFC §synth-5-B).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CodecVariant {
    /// `id` of the field whose decoded value (or named bit-range,
    /// see `tag_flag`) selects an arm. Must reference an unsigned-int
    /// field — enforced at parse time. `tag_field` resolves to a
    /// field in the codec's own `fields` (whole-field
    /// or multi-bit-flag dispatch), or equals `peek_byte.id` when
    /// peek-byte mode is active.
    ///
    /// `None` ⇒ caller-tag shape: `<sce:variant>`
    /// element with no `tag=` attribute. The leaf has no own carrier
    /// field; the dispatch value is supplied by the caller as the
    /// `tag: u8` decode parameter. Encode signature unchanged
    /// (active arm comes from the language-level enum discriminant).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_field: Option<String>,
    /// RFC §synth-5-B multi-bit-flag dispatch: when `Some(name)`, the
    /// `tag_field` MUST be a `<sce:flags>`-bearing carrier and `name`
    /// names one of its `<sce:flag>` bit-ranges. The dispatch value is
    /// `(carrier >> bit) & ((1 << width) - 1)` — the bit-range's
    /// shifted-and-masked unsigned scalar. When `None`, the dispatch
    /// reads the field's whole value (whole-field form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_flag: Option<String>,
    /// Enumerated arms in document order.
    pub arms: Vec<VariantArm>,
    /// Catch-all arm for tag values outside the enumerated set.
    /// `None` ⇒ build-time `codec/variant-arm-unreachable` when the
    /// tag domain isn't fully covered by `arms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_arm: Option<VariantArm>,
    /// RFC §synth-5-B peek-byte — peek-byte mode dispatch
    /// (Zenoh response/request body MID). `Some` ⇒ variant tag reads
    /// `peek_byte`'s next-byte view (carrier is the cursor's next
    /// byte without advancing); arm body codec reads same byte as
    /// own header. `None` ⇒ own-field mode (back-compat:
    /// existing variant goldens omit this field via
    /// skip_serializing_if).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peek_byte: Option<PeekByteSpec>,
}

/// Codec: byte-level encode/decode. Generates struct with decode/encode methods.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CodecModel {
    pub name: String,
    /// Document-level default endianness.
    pub default_endian: Endian,
    /// Expected input frame length (from `sce:length` on input data).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_length: Option<u32>,
    /// Ordered list of fields in the codec.
    pub fields: Vec<CodecField>,
    /// Optional discriminated-union suffix — RFC §synth-5-B variant primitive.
    /// When present the codec emits a sum type per language
    /// (Rust enum, Kotlin sealed class, C11 tagged union, etc.) rather
    /// than a flat struct.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<CodecVariant>,
    /// Declared flag-shaped inputs the codec
    /// receives from its caller. Empty when the codec needs no caller-
    /// supplied flags. Each [`FlagInput`] becomes a positional typed
    /// parameter on the generated `decode`/`encode` signatures (in
    /// declaration order), addressable by bare name in
    /// `sce:present-if="<name>"` predicates. See [`FlagInput`] for the
    /// per-input shape and [`FlagBind`] for the parent-side binding
    /// directive at import-site.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub flag_inputs: Vec<FlagInput>,
    /// RFC §synth-5-B inline test vectors. Each row carries one wire
    /// `hex` byte sequence + the expected decoded field-value tree.
    /// Generates a per-backend round-trip sidecar (`<fixture>_test.{rs,h}`)
    /// next to the codec header — symmetric with the algorithm
    /// `<sce:test-vector>` machinery (RFC §synth-5-B item B2). The sidecar
    /// ships on Rust + C11 with plain (non-variant, non-TLV-chain,
    /// non-parent-flags) codecs only; variant + recursive-variant +
    /// TLV-chain + parent-flags codecs reject through the per-language
    /// gate in `render_codec_test_vector_sidecar` — wider coverage is
    /// not implemented until a consumer needs it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_vectors: Vec<CodecTestVector>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="codec">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

impl CodecModel {
    /// Minimum required frame bytes for decode (from fixed fields).
    pub fn min_frame_bytes(&self) -> u32 {
        let mut max_end = 0u32;
        for field in &self.fields {
            if let Some(bits) = field.fixed_bits() {
                let end = field.byte_offset + bits.div_ceil(8);
                max_end = max_end.max(end);
            }
        }
        max_end
    }

    /// Maximum frame bytes for encode buffer sizing: fixed bytes plus the
    /// resolved per-field cap of every variable-length field. Caps:
    ///   - `tail` / `length-ref`: `sce:max-size`, fallback to
    ///     [`crate::forge::limits::BYTES_DEFAULT_MAX`].
    ///   - `vle { width_bits }`: [`vle_cap_bytes`] (3 / 5 / 9 for
    ///     u16 / u32 / u64) — canonical Zenoh ZInt cap from RFC
    ///     §synth-5-B App. B (the u64 final byte carries 8 data bits).
    ///   - `repeat { count_ref }` (RFC §synth-5-B B2): contributes 0 here —
    ///     the per-element body size lives on the imported codec and
    ///     is only available after import enrichment, so the generator
    ///     adds `max_count * imported_codec.max_frame_bytes()` itself
    ///     (mirrors the variant arm body sizing at codegen time).
    ///   - `tlv-chain { max_depth, .. }` (RFC §synth-5-B B3): contributes 0
    ///     here for the same reason — generator adds `max_depth *
    ///     imported_codec.max_frame_bytes()` post-enrichment.
    pub fn max_frame_bytes(&self) -> u32 {
        let var_max: u32 = self
            .fields
            .iter()
            .filter(|f| f.is_variable_length())
            .map(|f| match &f.bit_size {
                BitSize::Vle { width_bits } => vle_cap_bytes(*width_bits),
                BitSize::Repeat { .. } | BitSize::TlvChain { .. } => 0,
                _ => crate::forge::limits::resolve_bytes_max(f.max_size),
            })
            .sum();
        self.min_frame_bytes() + var_max
    }

    /// Whether the codec has any variable-length field. Drives the
    /// per-backend encode-template branch (initializer-list literal for
    /// fixed-only fixtures vs builder-pattern for variable-length).
    pub fn has_variable_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_variable_length())
    }

    /// SSOT for codec path selection: whether the positional codec path
    /// can correctly express this codec's wire layout. Its negation is
    /// "this codec must use the streaming cursor path".
    ///
    /// The positional path (`generate_decode_expr` / `generate_encode_exprs`)
    /// reads every field at a compile-time-fixed `raw[byte_off]` and appends
    /// the single variable-length field last. That is correct only for the
    /// exact shape below; everything else streams.
    ///
    /// Stated as an *allowlist of what positional supports* rather than a
    /// denylist of streaming-forcing features. This is deliberate and
    /// fail-safe: a `BitSize` added to the enum later defaults to the
    /// streaming path (the general, always-correct one) unless it is
    /// explicitly proven positional-expressible here — so a new variant
    /// cannot silently fall through to a positional emit that mis-places
    /// it. (The two historical mis-routings this guards against —
    /// `sce:type="string"` length-ref payloads and a fixed field after a
    /// length-ref payload — were both denylist omissions.)
    pub fn positional_layout_is_valid(&self) -> bool {
        // The positional path has no conditional read (present-if) and no
        // UTF-8 decode (string); both require the streaming form.
        if self.has_present_if_fields() || self.has_string_fields() {
            return false;
        }
        let last = self.fields.len().saturating_sub(1);
        self.fields
            .iter()
            .enumerate()
            .all(|(i, f)| match f.bit_size {
                // Fixed fields have a statically-known offset and width.
                BitSize::Fixed { .. } => true,
                // The positional path implements only `tail` / `length-ref`
                // among the variable bit-sizes, and only as the final field
                // (a field after a variable payload has a runtime offset).
                // `tail` not-last is rejected upstream
                // (`validate_codec_tail_is_last`); the bound here keeps the
                // predicate self-contained.
                BitSize::Tail | BitSize::LengthRef => i == last,
                // VLE / repeat / TLV-chain / embed have no positional form at
                // all — they always stream.
                BitSize::Vle { .. }
                | BitSize::Repeat { .. }
                | BitSize::TlvChain { .. }
                | BitSize::Embed => false,
            })
    }

    /// Borrowed zero-copy round — SSOT for "does this codec's generated
    /// Rust struct need a `<'a>` lifetime parameter?". True iff it holds
    /// a scalar `Bytes` / `String` field (decoded as a zero-copy
    /// `&'a [u8]` / `&'a str` view per the `SceCursor::peek_slice ->
    /// &'a [u8]` borrow contract) OR it embeds / repeats / tlv-chains /
    /// variant-dispatches a body codec that is itself borrowed.
    ///
    /// The body-codec query is the caller's responsibility via
    /// `body_borrowed(alias)` because the two call sites resolve it from
    /// different sources: the enrichment-time recursion
    /// (`lib.rs::codec_is_borrowed_recursive`) re-parses the import file
    /// graph, while the codegen-time check
    /// (`generator.rs::codec_self_borrowed`) reads the already-enriched
    /// `ImportContext::codec_is_borrowed` flags. Both share THIS
    /// structural predicate — the scalar-field rule and the
    /// embed/repeat/tlv/variant traversal live in exactly one place.
    ///
    /// Repeat / tlv-chain / embed fields carry a `SceType::Bytes`
    /// *sentinel* (their real type is the body codec), so the scalar
    /// check excludes them; their borrowed-ness flows through
    /// `body_borrowed` instead. Including them would over-approximate and
    /// emit an unused `<'a>` (E0392) — inference must stay exact.
    pub(crate) fn is_borrowed_with(&self, mut body_borrowed: impl FnMut(&str) -> bool) -> bool {
        // Direct scalar Bytes / String view.
        if self.fields.iter().any(|f| {
            !f.is_repeat()
                && !f.is_tlv_chain()
                && !f.is_embed()
                && matches!(f.sce_type, SceType::Bytes | SceType::String)
        }) {
            return true;
        }
        // Variant arm bodies (incl. the default arm).
        if let Some(v) = &self.variant {
            if v.arms
                .iter()
                .chain(v.default_arm.iter())
                .any(|arm| body_borrowed(&arm.body_alias))
            {
                return true;
            }
        }
        // Repeat / tlv-chain / embed body codecs.
        self.fields.iter().any(|f| {
            let alias = if f.is_repeat() {
                f.repeat_body_alias.as_deref()
            } else if f.is_tlv_chain() {
                f.tlv_chain_body_alias.as_deref()
            } else if f.is_embed() {
                f.embed_body_alias.as_deref()
            } else {
                None
            };
            alias.is_some_and(&mut body_borrowed)
        })
    }

    /// Owned→borrowed projection round — SSOT for "must this codec's
    /// `{Codec}Owned::as_borrowed` projection be fallible
    /// (`try_as_borrowed -> Result`) rather than infallible (`as_borrowed
    /// -> {Codec}<'_>`)?". True iff it holds a bounded list field
    /// (`<sce:repeat>` / `<sce:tlv-chain>`): the borrowed view spells the
    /// list `heapless::Vec<_, N>` while the owned mirror is an unbounded
    /// `Vec`, so projecting owned back to borrowed can exceed `N` and
    /// raises `CodecError::TooManyElements` (same bound, same error decode
    /// enforces — the system invariant is `<= N` end-to-end). It is also
    /// fallible if it embeds / variant-dispatches a body codec whose own
    /// projection is fallible.
    ///
    /// Mirrors [`is_borrowed_with`]: the body-codec query is the caller's
    /// responsibility via `body_fallible(alias)` because the enrichment-
    /// time recursion (`lib.rs::codec_as_borrowed_fallible_recursive`) and
    /// the codegen-time check
    /// (`generator.rs::codec_self_as_borrowed_fallible`) resolve it from
    /// different sources but share THIS structural predicate so they can
    /// never diverge. `body_fallible(alias)` is the body's *contribution*
    /// — true only when the body is itself borrowed (else it is cloned
    /// wholesale by the projection, which never fails) AND its own
    /// projection is fallible.
    ///
    /// A scalar-only / string / option / embed-of-infallible-body codec
    /// stays infallible — a uniform-`Result` API would graft a spurious
    /// `Result` onto a projection that can never fail (the `TryFrom` /
    /// `From` distinction; §4 policy).
    pub(crate) fn is_as_borrowed_fallible_with(
        &self,
        mut body_fallible: impl FnMut(&str) -> bool,
    ) -> bool {
        // Direct bounded list field: owned `Vec` -> borrowed
        // `heapless::Vec<_, N>` can overflow `N` at projection time.
        if self.has_repeat_fields() || self.has_tlv_chain_fields() {
            return true;
        }
        // Variant arm bodies (incl. the default arm).
        if let Some(v) = &self.variant {
            if v.arms
                .iter()
                .chain(v.default_arm.iter())
                .any(|arm| body_fallible(&arm.body_alias))
            {
                return true;
            }
        }
        // Embed body codecs (repeat / tlv-chain already short-circuited
        // above; their fallibility is the bounded-list rule, not the body).
        self.fields.iter().any(|f| {
            f.is_embed()
                && f.embed_body_alias
                    .as_deref()
                    .is_some_and(&mut body_fallible)
        })
    }

    /// Whether the codec has any repeat field (RFC §synth-5-B B2). Forces
    /// the streaming decode/encode path because a repeat field's wire
    /// length is runtime-determined (count_ref or until-eof) and the
    /// per-element body invokes the imported codec's encode/decode.
    pub fn has_repeat_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_repeat())
    }

    /// Whether the codec has any present-if-gated field (RFC §synth-5-B
    /// present-if primitive). Forces the streaming decode/encode path because a
    /// gated field's start offset depends on the runtime predicate
    /// value, and per-language type wraps the field as an optional.
    pub fn has_present_if_fields(&self) -> bool {
        self.fields.iter().any(|f| f.present_if.is_some())
    }

    /// Whether the codec has any TLV chain field (RFC §synth-5-B B3). Forces
    /// the streaming decode/encode path (same machinery as repeat) and
    /// classifies the codec as MCU-class.
    pub fn has_tlv_chain_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_tlv_chain())
    }

    /// Whether the codec has any single-codec embed field (RFC §synth-5-B).
    /// Forces the streaming decode/encode path (same machinery as
    /// repeat / TLV chain). The streaming codec calls the embedded
    /// codec's decode/encode with optional parent-flag threading.
    pub fn has_embed_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_embed())
    }

    /// Whether the codec has any UTF-8 string field (RFC §synth-5-B).
    /// Forces the streaming decode/encode path so the
    /// String dispatch in `present_if_decode_length_ref` /
    /// `present_if_encode_length_ref` always runs (any String field is
    /// guaranteed `BitSize::LengthRef` by parser validation; a String-
    /// bearing codec without VLE / present-if / repeat siblings would
    /// otherwise route through `generate_decode_expr` and silently
    /// emit bytes-shape into a String field). Bytes-only codecs stay
    /// byte-stable — the rollup only flips when at least one field
    /// declares `sce:type="string"`.
    pub fn has_string_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_string())
    }

    /// Whether the codec has any field with `sce:dma-burst-align` (RFC
    /// §synth-5-B B3). Forces the encode buffer to be zero-initialised so
    /// padding bytes between fields land as deterministic zeros on the
    /// wire (peer interop), and triggers per-field language-level
    /// alignment assertions in the generated code. MCU-class.
    pub fn has_dma_aligned_fields(&self) -> bool {
        self.fields.iter().any(|f| f.dma_burst_align.is_some())
    }

    /// Whether the codec has any tail-bytes field (`<sce:field
    /// sce:bit-size="tail">`). A tail field by definition consumes
    /// to the end of the frame, so the codec's decode cannot be
    /// stream-correct (it must consume the entire cursor remaining).
    /// Codecs WITHOUT tail can stream-correctly advance only the
    /// bytes they actually decoded — used by RFC §synth-5-B B3 to make
    /// length-ref entry codecs decode-iterable inside a TLV chain
    /// (length-ref entry codecs used to consume the entire remaining
    /// cursor; the TLV chain is the first multi-frame consumer that
    /// required stream-correct advancement).
    pub fn has_tail_fields(&self) -> bool {
        self.fields
            .iter()
            .any(|f| matches!(f.bit_size, BitSize::Tail))
    }

    /// RFC §synth-5-B "MCU-only codec sub-features" — whether the codec
    /// contains any feature that emits only on Rust + C11. Drives the
    /// codec-content classification used by `render_codec` to typed-
    /// reject cpp/kotlin/go/python via the existing kind-class
    /// diagnostic (`codegen/mcu-class-kind-on-non-mcu-language`,
    /// repurposed at codec-content granularity).
    ///
    /// DMA alignment is the only sub-feature that genuinely needs
    /// MCU-class hardware (DMA controllers, fixed-offset wire layout
    /// invariants tied to memory-mapped peripherals). TLV chain
    /// was originally bundled here as a conservative scope choice; it
    /// is in fact server-class-relevant too (Zenoh extension envelopes
    /// land on zenoh-rs / zenoh-cpp / zenoh-kotlin server peers, not
    /// just zenoh-pico MCU). The cpp/kotlin/go/python TLV-chain
    /// emitters lifted that gating; only DMA align stays MCU-only.
    pub fn has_mcu_only_features(&self) -> bool {
        self.has_dma_aligned_fields()
    }
}

// ── Codec test vectors (RFC §synth-5-B) ──────────────────────────────

/// One `<sce:test-vector hex="...">` row on a codec. Captures the
/// wire-byte form (`hex`) and the expected decoded field-value tree
/// (`decoded`) so the per-backend sidecar can render a single
/// round-trip oracle (encode → bytes parity, decode → field parity).
///
/// Trunk shape (`Plain` only) covers codecs whose decoded form is a
/// flat list of named fields. Variant / recursive-variant / TLV-chain
/// expansions reuse the same `CodecTestVector` envelope and only add
/// new arms to `DecodedValue` — keeps the parser + codegen surface
/// uniform as coverage widens.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CodecTestVector {
    /// Decoded wire bytes (parsed from the `hex=` attribute).
    pub hex: Vec<u8>,
    /// Expected decoded value tree.
    pub decoded: DecodedValue,
    /// 1-based source line of the `<sce:test-vector>` element.
    /// Round-tripped to per-backend test-function naming so a failing
    /// row is locatable in the SCXML source from the test output.
    pub source_line: usize,
}

/// Decoded value tree for a `<sce:test-vector>` row. Trunk only
/// emits `Plain` (flat field list); variant / TLV-chain coverage
/// adds `Variant` and `Chain` arms additively.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "shape", rename_all = "kebab-case")]
pub enum DecodedValue {
    /// Plain codec: ordered list of named field assignments. Field
    /// names + value types validated against the codec's `fields`
    /// list at parse time.
    Plain { fields: Vec<DecodedField> },
}

/// One `<sce:decoded field="..." value|hex|string="..."/>` row inside
/// a `<sce:test-vector>`. Field name is matched against
/// `CodecModel.fields` at parse time; value carries the typed scalar
/// / bytes / string literal compatible with that field's `sce_type`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct DecodedField {
    pub name: String,
    pub value: DecodedFieldValue,
}

/// Typed value literal for one `<sce:decoded>` row. Variant chosen
/// at parse time from the matching codec field's `SceType`:
/// integer types → `Uint` / `Int`, `Bool` → `Bool`, `Bytes` → `Bytes`,
/// `String` → `String`. Float fields are not yet supported (no codec
/// field uses them; support lands alongside the first consumer).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum DecodedFieldValue {
    Bool(bool),
    /// Unsigned integer literal — narrowed at codegen time against
    /// the field's declared SceType (uint8/uint16/uint32/uint64).
    Uint(u64),
    /// Signed integer literal — narrowed at codegen time against
    /// the field's declared SceType (int8/int16/int32/int64).
    Int(i64),
    /// Byte sequence (parsed from the `hex=` attribute).
    Bytes(Vec<u8>),
    /// UTF-8 string (parsed from the `string=` attribute). Decode
    /// validates UTF-8 invariant per the codec string contract; the
    /// test-vector value is stored as canonical Rust `String`.
    String(String),
}

// ── Filter kind ───────────────────────────────────────────────

/// Filter type: algorithm used for signal filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FilterType {
    MovingAverage,
    LowPass,
    Debounce,
}

impl FilterType {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "moving-average" => Some(Self::MovingAverage),
            "low-pass" => Some(Self::LowPass),
            "debounce" => Some(Self::Debounce),
            _ => None,
        }
    }

    /// Template-safe identifier (snake_case, no hyphens).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MovingAverage => "moving_average",
            Self::LowPass => "low_pass",
            Self::Debounce => "debounce",
        }
    }
}

/// Filter: signal filtering with internal state (moving average, low-pass, debounce).
/// Generates a struct with `update()` and `reset()` methods.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct FilterModel {
    pub name: String,
    pub input: ForgeField,
    pub output: ForgeField,
    pub filter_type: FilterType,
    /// Window size for moving-average and debounce.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<u32>,
    /// Smoothing factor (0..1) for low-pass filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f64>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="filter">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Interpolation kind ────────────────────────────────────────

/// Interpolation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum InterpolationMethod {
    Linear,
    Bilinear,
}

impl InterpolationMethod {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "linear" => Some(Self::Linear),
            "bilinear" => Some(Self::Bilinear),
            _ => None,
        }
    }
}

/// Out-of-bounds handling for interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OutOfBounds {
    #[default]
    Clamp,
    Extrapolate,
    Error,
}

impl OutOfBounds {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "clamp" => Some(Self::Clamp),
            "extrapolate" => Some(Self::Extrapolate),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Clamp => "clamp",
            Self::Extrapolate => "extrapolate",
            Self::Error => "error",
        }
    }
}

/// Axis definition for interpolation (breakpoints for one dimension).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct InterpolationAxis {
    /// Input field id this axis corresponds to.
    pub input_id: String,
    /// Breakpoint values (monotonically increasing).
    pub breakpoints: Vec<f64>,
}

/// Interpolation: 1D/2D table lookup with linear/bilinear interpolation.
/// Generates a struct with static tables and a `lookup()` method.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct InterpolationModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub output: ForgeField,
    pub method: InterpolationMethod,
    pub out_of_bounds: OutOfBounds,
    /// Axes in order (1 for linear, 2 for bilinear). First = rows, second = columns.
    pub axes: Vec<InterpolationAxis>,
    /// Table values (flat, row-major for 2D).
    pub values: Vec<f64>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="interpolation">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Timer kind ────────────────────────────────────────────────

/// Timer kind shape per SCE Protocol-Synthesis RFC §synth-5-D line 880-886.
///
/// One timer per forge doc — keepalive, retry, watchdog, etc. The
/// timer self-manages its lifecycle through `reset_on` (event-driven
/// restart) and `cancel_on_state_exit` (state-exit-driven cancel),
/// and emits `fire_event` when the period elapses.
///
/// **Pre-C1 legacy shape removed.** Before 2026-05-12, this kind
/// declared multiple `TimerEntry` records under `<datamodel>` with
/// `sce:timer="periodic|timeout|delayed"` + integer-millisecond
/// attributes. The legacy shape did not cover the §synth-5-D semantics
/// (event-driven reset, state-exit cancel, single named timer per
/// doc); C1 migrates SCE to the spec-mandated shape per
/// `feedback_spec_mirror_parity.md` and
/// `feedback_pre_release_no_compat.md` (pre-1.0 rename permitted).
///
/// Codegen contract (RFC §synth-5-D lines 902-906):
/// - AP: deadline tracker on top of generic event queue
///   (e.g. `tokio::time::interval` for Rust, `ScheduledExecutorService`
///   for Kotlin, `time.AfterFunc` for Go, `asyncio.sleep` for Python)
/// - MCU: compile-time slot in a static timer wheel
///
/// Diagnostics (RFC §synth-5-D lines 909-910):
/// - `timer/period-below-tick-rate` — `period_us` declared shorter
///   than `scheduler.tick_period_us`; cooperative scheduler cannot
///   dispatch faster than its tick rate.
/// - `timer/slot-overflow` — total timer doc count for a machine
///   exceeds `scheduler.timer_wheel_depth`; the static wheel cannot
///   hold any more.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TimerModel {
    /// Timer name from `<scxml sce:kind="timer" name="...">`. Used
    /// as the codegen struct prefix (`<Name>Timer` / `<name>_timer`).
    pub name: String,
    /// Timer period in microseconds (parsed from `<sce:period>` body
    /// text with unit suffix — `5s` / `100ms` / `30us` / `2m`). u64
    /// chosen because periods up to ~5800 years fit; u32 microseconds
    /// would cap at ~71 minutes.
    pub period_us: u64,
    /// Event name that resets the timer's deadline when raised.
    /// `<sce:reset-on event="..."/>` (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_on_event: Option<String>,
    /// State id whose exit cancels the timer.
    /// `<sce:cancel-on state-exit="..."/>` (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel_on_state_exit: Option<String>,
    /// Event name raised when the timer fires.
    /// `<sce:fire-event>...</sce:fire-event>` (required).
    pub fire_event: String,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="timer">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Observer kind ─────────────────────────────────────────────

/// A single threshold monitor within an observer kind.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ThresholdMonitor {
    pub id: String,
    /// ECMAScript expression for entering the active state.
    pub enter_expr: String,
    /// ECMAScript expression for leaving the active state. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave_expr: Option<String>,
    /// Event name emitted on entering active state.
    pub on_enter: String,
    /// Event name emitted on leaving active state. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_leave: Option<String>,
}

/// Observer: threshold monitoring with hysteresis.
/// Generates a struct with per-monitor boolean state and an `update()` method
/// returning a collection of triggered events.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ObserverModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub monitors: Vec<ThresholdMonitor>,
    /// Optional event domain tag declared via `sce:event-domain` on the
    /// `<scxml>` root. When present, generated code emits events of type
    /// `SCE::Forge::Event<Domain>`, enabling cross-file observer composition.
    /// When absent, the observer falls back to a file-local enum and cannot
    /// be composed with other observers (see SCE_FORGE.md §4.11).
    pub event_domain: Option<String>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="observer">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Cross-file import ─────────────────────────────────────────

/// A cross-file kind import declared via `<sce:import>` in an Extended SCXML document.
/// Enables kind composition: e.g., a procedure referencing a codec's encode/decode.
///
/// ```xml
/// <sce:import src="can_frame.scxml" kind="codec" as="frame"/>
/// ```
///
/// The `alias` is used in expressions to access the imported kind's API:
/// - Codec: `frame.encode(...)`, `frame.decode(...)`
/// - Transform: `keygen.compute(...)`
/// - Condition: `check.evaluate(...)`
/// - Lookup: `table.lookup(...)`
/// - Validator: `val.validate(...)`
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ForgeImport {
    /// Relative path to the imported SCXML file.
    pub src: String,
    /// Kind of the imported document (for validation and API generation).
    pub kind: ForgeKind,
    /// Alias used in expressions to reference the imported kind.
    pub alias: String,
    /// 1-based source line of the `<sce:import>` element in the
    /// importing document. Populated by `parse_imports` so later
    /// passes (e.g. `validate_and_enrich_imports`) can anchor their
    /// diagnostics at the exact element rather than at the document
    /// root. Skipped from serialization so existing manifest JSON
    /// output stays byte-stable.
    #[serde(skip)]
    pub line: Option<u32>,
    /// RFC §synth-5-B variant-dispatch — when the importing
    /// parent codec declares `<sce:variant-dispatch flag="X.Y"/>` as a
    /// child of this `<sce:import>`, the imported variant codec's arm
    /// choice drives a bit value into the parent's named flag carrier
    /// at encode time. `None` for imports with no dispatch declaration
    /// (either the imported codec has no variant, or the parent author
    /// selects the arm explicitly without wire-level
    /// dispatch). Cross-doc validator confirms the
    /// named carrier + flag exist in the parent's own model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embed_dispatch: Option<EmbedDispatch>,
    /// Parent-side bindings supplying values for
    /// the imported leaf codec's declared `<sce:flag-inputs>`. Each
    /// [`FlagBind`] names one leaf-side input and a source on the
    /// parent (either a local flags-carrier flag in dotted form, or
    /// one of the parent's own [`FlagInput`]s by bare name for the
    /// chain-forwarder pattern). Empty when the import declares no
    /// `<sce:flag-bind>` children — valid only when the imported leaf
    /// declares no `<sce:flag-inputs>` (cross-doc validator enforces
    /// the binding-completeness invariant).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub flag_binds: Vec<FlagBind>,
}

/// RFC §synth-5-B variant-dispatch — parent-side declaration
/// of which of its own flags drives an imported variant codec's arm
/// dispatch. Attached to a `ForgeImport` via the `<sce:variant-dispatch>`
/// child of `<sce:import>`.
///
/// The dotted `flag_source` value names the parent's own carrier and
/// flag (e.g. `"header.M"` means "the M flag bit of the `header` field
/// in this codec"). Cross-doc validator confirms the named carrier
/// field exists, the named flag exists on it, the flag has no
/// conflicting `value=` constant, the flag's width fits the imported
/// codec's arm count, and the carrier appears before the embed field
/// in declaration order.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EmbedDispatch {
    /// Dotted `<carrier>.<flag>` reference into the parent codec's
    /// own flag space. Parser splits on the dot; cross-doc validator
    /// resolves both halves against the parent's `fields` list.
    pub flag_source: String,
    /// 1-based source line of the `<sce:variant-dispatch>` element
    /// for diagnostic anchoring.
    #[serde(skip)]
    pub line: Option<u32>,
}

// ── Build manifest ────────────────────────────────────────────

/// A single entry in a forge build manifest.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ManifestEntry {
    pub src: String,
    pub name: String,
    pub kind: ForgeKind,
    /// Conservative (upper-bound) runtime dependency derived from `kind`.
    /// Precise value requires parsing the document (see `ForgeDocument::runtime_dep()`).
    pub runtime_dep: RuntimeDep,
    pub imports: Vec<ForgeImport>,
}

/// Forge build manifest — dependency graph for a set of forge SCXML files.
/// Includes topological build order (leaves first).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ForgeManifest {
    pub entries: Vec<ManifestEntry>,
    pub build_order: Vec<String>,
}

// ── Inline kind (within statechart) ────────────────────────────

/// Inline kind data — embedded within a statechart `<data>` element.
/// Derived values (unique_values, entries_by_value) are computed by the
/// generator at render time, not stored in the model (state normalization).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind")]
pub enum InlineKindData {
    #[serde(rename = "lookup")]
    Lookup {
        input_id: String,
        entries: Vec<LookupEntry>,
        default_value: String,
    },
    #[serde(rename = "condition")]
    Condition { expr: String },
    #[serde(rename = "codec")]
    Codec {
        fields: Vec<CodecField>,
        default_endian: Endian,
    },
    #[serde(rename = "transform")]
    Transform {
        inputs: Vec<ForgeField>,
        expr: String,
        output_type: SceType,
    },
}

/// An inline kind declaration within a statechart.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct InlineKind {
    pub id: String,
    pub data: InlineKindData,
}

// ── Algorithm kind (RFC §synth-5-A) ──────────────────────────────────

/// One parameter of an algorithm signature. Parameters are by-value
/// scalars or by-reference slices for `bytes`. Read-only in v1
/// (assigning to a parameter raises `algorithm/lvalue-unsupported`).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct AlgorithmParam {
    pub name: String,
    #[serde(rename = "type")]
    pub sce_type: SceType,
}

/// Algorithm signature — parameters and return type. `return_type =
/// None` denotes void (no `<sce:return type=...>`); a body without a
/// terminal `<sce:return>` raises `algorithm/return-missing` only when
/// `return_type` is Some.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct AlgorithmSignature {
    pub params: Vec<AlgorithmParam>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_type: Option<SceType>,
    /// Cap on the bytes a `return_type = bytes` algorithm may produce,
    /// declared via `sce:returns-max-size="N"` on the `<sce:signature>`.
    /// Required when `return_type` is `bytes` (the output buffer's fixed
    /// capacity on the no-alloc profile); ignored otherwise. Mirrors
    /// [`ProcedureHelper::returns_max_size`]. `None` ⇒ non-bytes return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub returns_max_size: Option<u32>,
}

/// Type carried by an `<sce:const>` declaration. Scalar form is the
/// only shape produced by hand-authored algorithm bodies (RFC §synth-5-A);
/// the array form is reserved for `sce:compute-at="build"` consts
/// whose body is an `<sce:fold>` block (RFC §synth-5-F build-time const-fold).
/// Keeping array out of [`SceType`] keeps the parameter / var / field
/// surfaces scalar-only, which is the v1 contract everywhere else.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum AlgorithmConstType {
    /// Scalar form (any `SceType`). Emitted as a language-native const.
    Scalar(SceType),
    /// Array form `array<T, N>` — emitted as a language-native static
    /// array literal via §synth-5-F const-fold. Length is the fixed
    /// element count; element type is one of the scalar `SceType`s.
    Array { elem: SceType, len: u32 },
}

impl AlgorithmConstType {
    /// Element type of an array shape, or the scalar's own type. Used
    /// by the host interpreter to coerce yielded values
    /// before serializing the array literal, and by the §synth-5-J-5 emitters
    /// to derive the per-language type name.
    pub fn elem_or_scalar(&self) -> &SceType {
        match self {
            Self::Scalar(t) => t,
            Self::Array { elem, .. } => elem,
        }
    }

    /// Parse the textual `type=` attribute of `<sce:const>`. Accepts
    /// any scalar [`SceType`] keyword as well as the
    /// `array<<elem>, <len>>` form (e.g. `array<u16, 256>`,
    /// `array<uint16, 256>`). `<elem>` accepts both Rust-style
    /// (`u8`..`u64`, `i8`..`i64`, `f32`/`f64`) and SCXML-style
    /// (`uint8`..`uint64`, `int8`..`int64`, `float32`/`float64`)
    /// spellings; the alias map mirrors RFC §synth-5-F's example which uses
    /// `array<u16, 256>` while the rest of the schema uses the
    /// long-form spelling. Returns `None` on any other input — caller
    /// raises a `ValidationError::InvalidAttribute` that names both
    /// forms in the `expected:` field.
    pub fn from_attr(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(rest) = s.strip_prefix("array<").and_then(|t| t.strip_suffix('>')) {
            let (elem_str, len_str) = rest.split_once(',')?;
            let elem = parse_scetype_with_aliases(elem_str.trim())?;
            let len: u32 = len_str.trim().parse().ok()?;
            return Some(Self::Array { elem, len });
        }
        SceType::from_attr(s).map(Self::Scalar)
    }
}

/// Recognises both SCXML-style (`uint16`) and Rust-style (`u16`)
/// scalar type spellings. RFC §synth-5-F's worked example uses the short
/// form (`array<u16, 256>`) while the rest of the IR speaks the long
/// form — accepting both keeps authoring fluent without forking the
/// canonical `SceType` enum.
fn parse_scetype_with_aliases(s: &str) -> Option<SceType> {
    if let Some(t) = SceType::from_attr(s) {
        return Some(t);
    }
    Some(match s {
        "u8" => SceType::Uint8,
        "u16" => SceType::Uint16,
        "u32" => SceType::Uint32,
        "u64" => SceType::Uint64,
        "i8" => SceType::Int8,
        "i16" => SceType::Int16,
        "i32" => SceType::Int32,
        "i64" => SceType::Int64,
        "f32" => SceType::Float32,
        "f64" => SceType::Float64,
        _ => return None,
    })
}

/// Body of an `<sce:fold>` element — RFC §synth-5-F build-time evaluation.
///
/// Iterates `iter_var` over `[range_start, range_end)`, executing
/// `body` against a fresh local scope per iteration and emitting
/// `yield_expr` (typed at `elem_type`) as one element of the produced
/// array.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct FoldBody {
    /// Inclusive lower bound of the integer range driving the fold.
    pub range_start: u32,
    /// Exclusive upper bound of the integer range driving the fold.
    pub range_end: u32,
    /// Loop-variable name visible inside the fold body.
    pub iter_var: String,
    /// Element type — must match the array shape's `elem` and the
    /// static type of `yield_expr` (validated by the
    /// `algorithm/const-yield-type-mismatch` diagnostic).
    pub elem_type: SceType,
    /// Statements executed per iteration before `yield_expr`. Re-uses
    /// the algorithm-body statement vocabulary (Var / Assign / If /
    /// While / Foreach). `Return` and `Call` are forbidden inside a
    /// fold body (the host interpreter rejects them with
    /// `algorithm/const-not-foldable`).
    pub body: Vec<AlgorithmStmt>,
    /// Per-iteration yielded expression — its evaluated value at
    /// `elem_type` becomes the next array element.
    pub yield_expr: String,
}

/// Build-time const inside an algorithm body. RFC §synth-5-A v1 admits the
/// scalar literal form (`<sce:const name=... type=... init="..."/>`).
/// The `<sce:const sce:compute-at="build">` form with an `<sce:fold>`
/// body resolves through RFC §synth-5-F build-time const-fold; the parser +
/// model own the IR shape, and the host interpreter plus per-language
/// emit lower it.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct AlgorithmConst {
    pub name: String,
    #[serde(rename = "type")]
    pub sce_type: AlgorithmConstType,
    /// Literal init expression for scalar consts. Lowered at codegen
    /// time via the typed expression pipeline (`forge::expr`). `None`
    /// when the const carries a `<sce:fold>` body instead — the
    /// parser enforces "exactly one of `init` / `fold`".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init: Option<String>,
    /// `<sce:fold>` body for `sce:compute-at="build"` consts. `None`
    /// for scalar consts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fold: Option<FoldBody>,
    /// `sce:compute-at="build"` flag (§synth-5-F hook). Required to be
    /// `true` when [`AlgorithmConst::fold`] is `Some`, and required
    /// to be `false` otherwise. The parser enforces this invariant.
    #[serde(default, skip_serializing_if = "is_false")]
    pub compute_at_build: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One statement in an algorithm body. Lowered to language-idiomatic
/// constructs by each backend (RFC §synth-5-J-5 emitter table).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "stmt", rename_all = "snake_case")]
pub enum AlgorithmStmt {
    /// `<sce:var name=... type=... init=... capacity=.../>` — local mutable
    /// binding. `capacity` is the fixed buffer bound, in bytes, and is
    /// required for — and only meaningful on — `type="bytes"` locals: the
    /// growable byte buffer that `<sce:append>` ([`AlgorithmStmt::Append`])
    /// fills (SCE byte-buffer-build, SCE_FORGE.md §4.12). It is `None` for
    /// scalar locals.
    Var {
        name: String,
        #[serde(rename = "type")]
        sce_type: SceType,
        init: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        capacity: Option<u32>,
    },
    /// `<sce:assign target="lvalue" expr="..."/>` — mutates an existing
    /// l-value. SCE's `validate_lvalue_shape` accepts an identifier or a
    /// one-level member access only; a computed index (`buf[i]`) is rejected
    /// because it is not a statically-known storage location. Byte buffers are
    /// built forward-only via `<sce:append>` ([`AlgorithmStmt::Append`]), never
    /// index writes. Stored as a raw string and validated by the parser.
    Assign { target: String, expr: String },
    /// `<sce:append target="buf" expr="..."/>` — append to a `bytes` local
    /// buffer (SCE byte-buffer-build, SCE_FORGE.md §4.12). `target` is the
    /// identifier of a `type="bytes"` local; `expr` lowers by its static type:
    /// a `uint8` appends one byte, a `bytes` value extends the buffer. The
    /// append is bounds-checked against the buffer's declared `capacity`;
    /// overflow surfaces fallibly (the codec's `CapacityExceeded` model),
    /// never silent truncation. This is the only buffer-mutation statement —
    /// index writes are deliberately excluded (see `Assign`).
    Append { target: String, expr: String },
    /// `<sce:if cond="..."> ... <sce:else>...</sce:else></sce:if>`.
    If {
        cond: String,
        then_body: Vec<AlgorithmStmt>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        else_body: Option<Vec<AlgorithmStmt>>,
    },
    /// `<sce:while cond="..." max-iter="N">...</sce:while>` — counted
    /// loop. `max_iter = None` is allowed only when the `cond`
    /// expression bounds itself; the MCU-side `algorithm/while-unbounded`
    /// rejection is not implemented until a consumer needs it.
    While {
        cond: String,
        body: Vec<AlgorithmStmt>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_iter: Option<u32>,
    },
    /// `<sce:foreach item="b" in="data">...</sce:foreach>` — iterate
    /// over a collection. Lowers to a counted byte-loop on `bytes`-typed
    /// sources.
    Foreach {
        item: String,
        source: String,
        body: Vec<AlgorithmStmt>,
    },
    /// `<sce:return expr=".../>"` — terminate body with an optional
    /// expression. Required at the body terminus when the signature
    /// declares a non-void `return_type`.
    Return {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expr: Option<String>,
    },
    /// `<sce:call target="other_algo" args="a, b"/>` — invoke another
    /// algorithm kind imported via `<sce:import>`. v1 forbids
    /// recursion; the dedicated `algorithm/call-cycle` diagnostic is
    /// not implemented until a consumer needs it.
    Call { target: String, args: Vec<String> },
}

/// Algorithm document — pure synchronous function (RFC §synth-5-A).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct AlgorithmModel {
    pub name: String,
    pub signature: AlgorithmSignature,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consts: Vec<AlgorithmConst>,
    pub body: Vec<AlgorithmStmt>,
    /// RFC §synth-5-B "Test vector": inline `<sce:test-vector hex value/>`
    /// reference oracles. Each entry generates a per-backend round-trip
    /// test that runs the algorithm on `hex` and asserts the return
    /// value equals `value`. This shape covers the algorithm kind with
    /// scalar return; codec documents carry their own multi-field
    /// `<sce:test-vector>` form (see [`CodecTestVector`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_vectors: Vec<TestVector>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="algorithm">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// RFC §synth-5-B test-vector value literal. v1 covers the scalar types an
/// algorithm signature can return; the child-element form for
/// multi-field codec results lives on the codec kind (see
/// [`DecodedValue`]).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "type", content = "value")]
pub enum TestVectorValue {
    /// Boolean literal (`true` / `false`).
    #[serde(rename = "bool")]
    Bool(bool),
    /// Unsigned integer literal (`0x29B1`, `42`). Source preserved as
    /// `u64` and narrowed at codegen time against the algorithm's
    /// declared return type.
    #[serde(rename = "uint")]
    Uint(u64),
    /// Signed integer literal (`-1`, `-127`). Distinct from `Uint` so
    /// the per-backend emitter can pick the correct signed integer
    /// type without inference.
    #[serde(rename = "int")]
    Int(i64),
}

/// Single `<sce:test-vector hex value/>` row (RFC §synth-5-B). Captures the
/// declared input bytes and expected output literal; the emitter pairs
/// these with the algorithm's signature to render an idiomatic
/// per-backend test function.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct TestVector {
    /// Decoded input bytes (parsed from the `hex=` attribute).
    pub hex: Vec<u8>,
    /// Expected return value.
    pub value: TestVectorValue,
    /// 1-based source line of the `<sce:test-vector>` element.
    /// Round-tripped to per-backend test-function naming so authors
    /// reading a failing test can find the SCXML row that produced it.
    pub source_line: usize,
}

// ── Link kind (RFC §synth-5-C) ───────────────────────────────────────

/// `<sce:link-class>` enumeration — RFC §synth-5-C "Link-class enumeration"
/// table. Five strings shipped today; OS-specific classes (e.g.
/// `unix_socket`, `qnx_msg`) land additively when wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum LinkClass {
    /// Datagram, byte-stream framer (RFC §synth-5-C row 1; A on MCU lwIP).
    Udp,
    /// Stream, byte-stream framer (RFC §synth-5-C row 2; B on MCU).
    Tcp,
    /// UART (RFC §synth-5-C row 3; C on MCU).
    Serial,
    /// TCP + WebSocket framing (RFC §synth-5-C row 4; C on MCU).
    Websocket,
    /// L2 frames, target-plugin only (RFC §synth-5-C row 5; C MCU plugin).
    RawEth,
}

impl LinkClass {
    /// Every legal `<sce:link-class>` value, in declaration order.
    pub const ALL_NAMES: &'static [&'static str] =
        &["udp", "tcp", "serial", "websocket", "raw_eth"];

    /// Parse from `<sce:link-class>` body text. Returns `None` for
    /// unknown classes — the parser raises `link/link-class-unknown`.
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "udp" => Some(Self::Udp),
            "tcp" => Some(Self::Tcp),
            "serial" => Some(Self::Serial),
            "websocket" => Some(Self::Websocket),
            "raw_eth" => Some(Self::RawEth),
            _ => None,
        }
    }

    /// Returns `true` iff this link class admits the given target OS
    /// per RFC §synth-5-C "Link-class enumeration" table (lines 765-771).
    /// Strict-literal reading per RFC §synth-5-C lines 776-782 "additive"
    /// policy: classes are added when wired, not pre-reserved as
    /// namespace placeholder. Anything off the table fires
    /// `link/class-unsupported-on-target` at validate-time.
    ///
    /// Single source of truth for [`forge::validate`]'s link-class × OS
    /// admissibility check.
    /// Future OS-specific classes (e.g. `unix_socket`, `qnx_msg`) land
    /// additively as new enum rows when the corresponding OS support
    /// is wired.
    pub fn admits_os(self, os: crate::mesh::deploy::OsKind) -> bool {
        use crate::mesh::deploy::OsKind;
        match self {
            // RFC §synth-5-C row 1: A (MCU lwIP) | D.1 (AP linux) | D.2 (AP qnx).
            Self::Udp | Self::Tcp => {
                matches!(os, OsKind::BareMetal | OsKind::Linux | OsKind::Qnx)
            }
            // RFC §synth-5-C rows 3-5: C (MCU) only.
            Self::Serial | Self::Websocket | Self::RawEth => {
                matches!(os, OsKind::BareMetal)
            }
        }
    }

    /// Lists every `OsKind` variant this class admits — used by the
    /// `link/class-unsupported-on-target` diagnostic to populate the
    /// `Fix::ReplaceOneOf` candidate axis (author can change either
    /// the class or the deployment target).
    pub fn admitted_os_names(self) -> Vec<&'static str> {
        use crate::mesh::deploy::OsKind;
        const ALL: &[OsKind] = &[
            OsKind::BareMetal,
            OsKind::Rtos,
            OsKind::Linux,
            OsKind::Qnx,
            OsKind::Macos,
            OsKind::Freebsd,
            OsKind::Windows,
        ];
        ALL.iter()
            .copied()
            .filter(|&os| self.admits_os(os))
            .map(|os| os.as_str())
            .collect()
    }
}

impl std::fmt::Display for LinkClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Udp => write!(f, "udp"),
            Self::Tcp => write!(f, "tcp"),
            Self::Serial => write!(f, "serial"),
            Self::Websocket => write!(f, "websocket"),
            Self::RawEth => write!(f, "raw_eth"),
        }
    }
}

/// `<sce:backpressure>` policy — RFC §synth-5-C body. All three policies
/// parse, and the rust template lowers them
/// uniformly (the policy threading into the runtime crate is an
/// implementation concern of `sce-link-runtime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum BackpressurePolicy {
    /// Drop incoming bytes when the consumer cannot keep up.
    Drop,
    /// Block the driver until the consumer drains.
    Block,
    /// Inject an `sce.link.<name>.overflow` event on the SCXML side.
    SignalEvent,
}

impl BackpressurePolicy {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "drop" => Some(Self::Drop),
            "block" => Some(Self::Block),
            "signal-event" => Some(Self::SignalEvent),
            _ => None,
        }
    }
}

impl std::fmt::Display for BackpressurePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drop => write!(f, "drop"),
            Self::Block => write!(f, "block"),
            Self::SignalEvent => write!(f, "signal-event"),
        }
    }
}

/// `<sce:inbound>` — RX byte-stream → SCXML event injection contract.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct LinkInboundEvent {
    /// SCXML event name to inject when the framer decode + `when`
    /// predicate succeed.
    pub event: String,
    /// Optional decode-side predicate (`when="decoded.msg_id == 0x02"`).
    /// `None` means inject every successful decode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// `<sce:outbound>` — SCXML event → framer encode + driver send contract.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct LinkOutboundEvent {
    /// SCXML event name that triggers TX.
    pub event: String,
    /// Encoder codec reference — same `<sce:framer ref>` namespace.
    pub encode: String,
}

/// Link document — RFC §synth-5-C byte-stream link endpoint.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct LinkModel {
    pub name: String,
    /// `<sce:link-class>` body text, parsed into the closed enum.
    pub class: LinkClass,
    /// `<sce:framer ref="...">` — the §synth-5-B codec that decodes/encodes
    /// wire bytes. Required; absence raises
    /// `link/framer-missing` (parser).
    pub framer: String,
    /// `<sce:backpressure>` policy. Required; absence raises the
    /// `link/backpressure-undeclared` diagnostic at parse time so
    /// authors declare `drop` / `block` / `signal-event` explicitly.
    pub backpressure: BackpressurePolicy,
    /// `<sce:events><sce:inbound .../></sce:events>` rows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inbound: Vec<LinkInboundEvent>,
    /// `<sce:events><sce:outbound .../></sce:events>` rows.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outbound: Vec<LinkOutboundEvent>,
    /// `<sce:rx-pool ref="...">` — RX buffer-pool name (RFC §synth-5-C body +
    /// §synth-5-E). Authors who want zero-copy RX path
    /// declare a `<scxml sce:kind="buffer-pool" name="...">` document
    /// and bind it here. The parser accepts the element and
    /// emits the ref as a `pub const` on the generated wrapper; the
    /// cross-resolution validator
    /// (`link/pool-slot-smaller-than-framer-max`) wires pool ↔ framer
    /// at `compile_forge_with_imports`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rx_pool: Option<String>,
    /// `<sce:tx-pool ref="...">` — TX buffer-pool counterpart. Same
    /// shape as `rx_pool`, including the slot-size cross-resolution
    /// validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tx_pool: Option<String>,
    /// `<sce:stage-pool ref="...">` — stage-copy destination pool.
    /// Single source of truth for `Sample::take()`'s copy target,
    /// per SCE Protocol-Synthesis RFC §synth-5-E (schema locality
    /// belongs on the link kind, not on the deploy.yaml binding —
    /// rx_pool/tx_pool precedent). When a SCXML state declares
    /// `<sce:on-sample link="X">`, the on-sample validator looks up link X
    /// in the [`super::cross_doc_registry::SceCrossDocRegistry`]; the link's
    /// `stage_pool` field decides whether `take()` is wired (resolves
    /// to a buffer-pool kind whose slots back the owned-copy
    /// destination) or whether `take()` will panic at runtime
    /// (`PanicOnTakeHook` default). Absence is legal for
    /// borrow-only callbacks that never call `take()`; presence + a
    /// missing on-sample subscriber is fine too (the field is link-
    /// declared, not on-sample-coupled). The SCXML-side enforcement
    /// raises `pool/sample-take-without-stage-pool` only when an
    /// on-sample subscriber exists for a link without `stage_pool`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_pool: Option<String>,
    /// `<sce:accept-stage-copy-rate/>` — SCE Protocol-Synthesis RFC §synth-5-K
    /// lines 2356-2361 + 2509-2511 per-link opt-out for the §synth-5-M /
    /// ARCHITECTURE §9.3 stage-copy-rate gate. Presence of the
    /// element (presence-only: no body / no attribute
    /// parsing) suppresses
    /// `reassembly/expected-fragmentation-rate-high` under
    /// `pool_defaults.stage_copy_policy: warn` and
    /// `pool/stage-copy-policy-error` under `error`. Under `forbid`
    /// (RFC line 2512-2516) the opt-out itself is rejected via
    /// `pool/stage-copy-accept-rejected-under-forbid`.
    ///
    /// Default `false` so existing fixtures without the opt-out
    /// preserve their prior behavior verbatim.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub accept_stage_copy_rate: bool,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="link">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// SCE Protocol-Synthesis RFC §synth-5-C lines 814-820 — listener-link sibling-pair
/// role discriminator. A `<sce:link>` whose deploy-resolved
/// `domain_attrs.trust_class` is `session_arming` and whose machine
/// source SCXML carries any `Accepting.*` substate (RFC §synth-5-C line 806 +
/// `docs/session-fsm.md` §2.6) is modeled by codegen as TWO logical
/// link-instances sharing one physical socket: the `Listener` half
/// receives bytes pre-handshake (`session_arming` trust), the
/// `Sibling` half receives them post-handshake (`established_session`
/// trust). RFC §synth-5-C lines 802-803 + §synth-5-M lines 2782-2783 mandate the
/// codegen-time split so `reassembly/untrusted-link-binding` retains
/// its static semantics (the runtime per-peer `Established` flag is
/// out of reach at validate-time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum LinkInstanceRole {
    /// Pre-handshake half. Receives bytes while the per-peer session
    /// FSM is in `Accepting.*`. Inherits the §synth-5-K accept-side hardening
    /// fields (`session_arming_quota`, `accept_rate_*`,
    /// `accepting_inactivity_timeout_ms`, `stateless_accept`). For a
    /// non-listener `<sce:link>` (no synthesized sibling), this is the
    /// only role emitted and the role is implicit.
    Listener,
    /// Post-handshake half. Synthesized — NOT author-declared. Receives
    /// bytes when the per-peer session FSM has transitioned out of
    /// `Accepting.*`. Inherits `bind`, `driver`, `mtu_bytes`,
    /// `expected_p99_bytes`, `burst_pps`, `rx_dispatch` from the
    /// Listener entry (RFC §synth-5-C lines 814-816); deliberately does NOT
    /// inherit the §synth-5-K accept-side hardening fields per RFC §synth-5-C
    /// lines 816-820 (those are meaningful only on the `session_arming`
    /// half).
    Sibling,
}

impl LinkInstanceRole {
    /// Canonical wire form used by diagnostic `key_fragments` and the
    /// downstream type-name suffix on the Sibling half.
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkInstanceRole::Listener => "listener",
            LinkInstanceRole::Sibling => "established-session",
        }
    }
}

/// SCE Protocol-Synthesis RFC §synth-5-C lines 802-833 + §synth-5-M lines 2782-2783
/// — resolved link-instance produced by the orchestrator's
/// listener-pair walker. For a non-listener `<sce:link>` the
/// orchestrator emits a single `Listener`-role instance; for a
/// listener (trust_class `session_arming` + machine SCXML carries
/// `Accepting.*`), it emits BOTH a `Listener` (pre-handshake) and a
/// `Sibling` (post-handshake) instance keyed by the same source link
/// name.
///
/// **Consumers** (IR-level synthesis, codegen-time split):
/// - [`crate::mesh::deploy::validate_reassembly_cross_doc`] reads the
///   per-link Sibling presence flag to gate
///   `reassembly/binding-on-unpaired-listener`.
/// - The per-language `render_link_*` templates emit the Sibling half
///   under a distinct type-name suffix when the link is a listener;
///   the post-render substring self-check
///   (`link/listener-link-not-paired-with-established-sibling`)
///   fires when the suffix is missing.
///
/// The inherited 6 fields are carried by VALUE rather than holding a
/// reference to the source `LinkConfig` so the orchestrator's
/// downstream consumers can iterate without re-joining against the
/// deploy schema. The Listener role echoes the source link's fields
/// verbatim; the Sibling role echoes the same six values (per RFC
/// §synth-5-C lines 814-816 the sibling is field-by-field identical on
/// these axes), and the validator + template never reach for the
/// hardening fields on a Sibling instance.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ResolvedLinkInstance {
    /// Deploy `machines.<name>` that owns the source link declaration.
    /// The orchestrator preserves the original machine name so the
    /// diagnostic `key_fragments` for
    /// `reassembly/binding-on-unpaired-listener` quote the same
    /// `(machine, link_name)` shape the existing reassembly codes use.
    pub machine: String,
    /// `<sce:link name="X">` body name. The Listener + Sibling pair
    /// share this name; the role discriminator separates them.
    pub link_name: String,
    /// Listener vs Sibling. See [`LinkInstanceRole`].
    pub role: LinkInstanceRole,
    /// RFC §synth-5-C line 814 — `bind`.
    pub bind: String,
    /// RFC §synth-5-C line 814 — `driver`.
    pub driver: String,
    /// RFC §synth-5-C line 814-816 — `mtu_bytes`.
    pub mtu_bytes: Option<u32>,
    /// RFC §synth-5-C line 814-816 — `expected_p99_bytes`.
    pub expected_p99_bytes: Option<u32>,
    /// RFC §synth-5-C line 814-816 — `burst_pps`.
    pub burst_pps: Option<u32>,
    /// RFC §synth-5-C line 814-816 — `rx_dispatch` (resolved per
    /// [`crate::mesh::deploy::LinkConfig::resolved_rx_dispatch`]).
    /// Carried by value so downstream consumers don't re-resolve the
    /// default. Wire form is the kebab-case `as_str` of `RxDispatch`.
    pub rx_dispatch: String,
}

/// `<sce:cache-policy>` enum — RFC §synth-5-E lines 948-957. Three policies
/// determine how codegen positions cache-maintenance ops on FSM edges:
/// `maintain` emits clean/invalidate around DMA boundaries;
/// `non-cacheable` declares MPU non-cacheable region (deploy.yaml
/// `attr: [non_cacheable]`); `none` targets without D-cache.
///
/// The parser carries the enum on `BufferPoolModel`; the cache-
/// maintenance pinning that uses this field routes through the
/// §synth-5-I `<sce:call>` intrinsic registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CachePolicy {
    /// Codegen inserts `cache_clean` before DMA TX, `cache_invalidate`
    /// after DMA RX. Pool MUST live in a `cacheable` region.
    Maintain,
    /// Pool MUST live in a `non_cacheable` MPU region; no maintenance
    /// ops emitted. CPU access pays the uncached-load penalty.
    NonCacheable,
    /// Target has no D-cache (e.g. Cortex-M0/M3/M4). No maintenance
    /// ops, no MPU setup. `cache-policy: maintain` on a `has_dcache:
    /// false` target raises `mem/cache-policy-unsupported-on-no-dcache-core`.
    None,
}

impl CachePolicy {
    pub const ALL_NAMES: &'static [&'static str] = &["maintain", "non-cacheable", "none"];

    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "maintain" => Some(Self::Maintain),
            "non-cacheable" => Some(Self::NonCacheable),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

impl std::fmt::Display for CachePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Maintain => write!(f, "maintain"),
            Self::NonCacheable => write!(f, "non-cacheable"),
            Self::None => write!(f, "none"),
        }
    }
}

/// `<sce:variant>` on `<scxml sce:kind="buffer-pool">` — RFC §synth-5-M lines
/// 2676-2698. The reassembly variant of §synth-5-E carries three additional
/// fields (`max-fragments-per-message`, `reassembly-timeout-ms`,
/// `per-peer-quota`) bundled on the `Reassembly` arm via the
/// [`ReassemblyConfig`] payload. The `Default` arm covers the absence
/// of `<sce:variant>` (regular RX/TX/stage pool) — the plain
/// buffer-pool behavior of `<scxml sce:kind="buffer-pool">`.
///
/// **Why a sum type**: the three reassembly-specific
/// fields are cross-field-coupled — all three required when
/// `variant=Reassembly`, all three forbidden when `variant=Default`.
/// Lifting the discriminator + payload into a single enum makes that
/// invariant type-system-enforced at every consumer; flat
/// `Option<u32>` fields would spread the coupling across the
/// `BufferPoolModel` struct.
///
/// Future variants per spec §synth-5-E lines 1166-1180 "FSM extension
/// policy" (TX-only, crypto-DMA, etc.) add new arms additively — the
/// existing `Default` / `Reassembly` arms are stable.
///
/// **Backend coverage (spec line 2659-2664)**: reassembly variant
/// inherits §synth-5-E backend coverage — emits only on `(rust, *)` and
/// `(c11, bare_metal)`. The existing
/// `codegen/mcu-class-kind-on-non-mcu-language` diagnostic on the
/// `ForgeKind::BufferPool` axis gates non-MCU targets;
/// no reassembly-specific MCU-class diagnostic exists.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "variant", rename_all = "kebab-case")]
pub enum BufferPoolVariant {
    /// No `<sce:variant>` element (or the explicit form
    /// `<sce:variant>default</sce:variant>`). Regular RX / TX / stage
    /// pool semantics; carries no extra fields.
    Default,
    /// `<sce:variant>reassembly</sce:variant>` (RFC §synth-5-M lines 2682,
    /// 2688-2690). Per-slot fragment-index bitmap + deadline + peer-id
    /// emission (codegen contract per RFC §synth-5-M). Parse-time enforces
    /// the three sibling elements `<sce:max-fragments-per-message>`,
    /// `<sce:reassembly-timeout-ms>`, `<sce:per-peer-quota>` are all
    /// present (single-arm payload).
    Reassembly(ReassemblyConfig),
}

/// Reassembly-variant configuration — RFC §synth-5-M lines 2682, 2688-2690.
///
/// Bundles the three required fields a reassembly-variant buffer pool
/// declares beyond the base §synth-5-E shape. The shape is validated at
/// parse time (all three required, all `> 0`); cross-doc invariants
/// reference `links.<name>.{mtu_bytes, expected_p99_bytes,
/// domain_attrs.trust_class}` and enforce the peer-table-capacity
/// build-time invariant against the §synth-5-K `links:` block.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ReassemblyConfig {
    /// `<sce:max-fragments-per-message>` (RFC §synth-5-M line 2688) — fragment-
    /// index bitmap width per slot; bounds the worst-case fragment count
    /// per reassembled message. Parse rejection: missing element
    /// (`mem/reassembly-pool-variant-missing-max-fragments`) or
    /// value == 0 (`<sce:max-fragments-per-message> body text: 0`
    /// generic `InvalidAttribute`).
    pub max_fragments_per_message: u32,
    /// `<sce:reassembly-timeout-ms>` (RFC §synth-5-M line 2689) — per-slot
    /// deadline emitted at codegen time. Parse rejection: missing
    /// element (`mem/reassembly-pool-variant-missing-timeout`) or
    /// value == 0 (generic `InvalidAttribute`).
    pub reassembly_timeout_ms: u32,
    /// `<sce:per-peer-quota>` (RFC §synth-5-M lines 2690, 2841-2861) — caps
    /// the in-flight reassembly slots a single peer may hold. The
    /// `peer_table.capacity × per-peer-quota ≥ slot_count` invariant
    /// is enforced build-time against the §synth-5-K `peer_table.capacity`
    /// source. Parse rejection: missing element (we reuse the
    /// generic `MissingElement` rather than minting a third
    /// reassembly-specific code — the
    /// two spec-named codes are reserved for the cases the spec
    /// explicitly names at line 2944-2945) or value == 0 (generic).
    pub per_peer_quota: u32,
}

/// Buffer-pool document — RFC §synth-5-E SRAM-placed, DMA-aligned slot table.
///
/// Schema (per `<scxml sce:kind="buffer-pool">` body):
/// - `<sce:slot-count>` (u32, > 0) — number of slots in the pool
/// - `<sce:slot-size>` (u32, > 0) — bytes per slot
/// - `<sce:section>` (string) — SRAM region name (matches deploy.yaml
///   `machines.<m>.memory.sram_regions.<name>`); the deploy-aware
///   second-consumer validator `mem/pool-section-conflict` fires when the section is
///   declared but absent from the resolved machine's memory map
/// - `<sce:alignment>` (u32, power of 2, > 0) — DMA alignment requirement
/// - `<sce:dma-channel>` (string, optional) — DMA channel binding
///   (matches deploy.yaml `machines.<m>.memory.dma_channels` entry);
///   the `mem/dma-channel-collision` cross-check is not implemented
///   until a consumer needs it
/// - `<sce:cache-policy>` ([`CachePolicy`]) — `maintain` /
///   `non-cacheable` / `none`; cache-maintenance pinning rides the
///   lifecycle-FSM edges
/// - `<sce:variant>` (optional, RFC §synth-5-M line 2682) — `default` (no
///   `<sce:variant>` element; plain buffer-pool behavior) OR
///   `reassembly` ([`ReassemblyConfig`] payload).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct BufferPoolModel {
    pub name: String,
    /// `<sce:slot-count>` — slot count in the pool. Parser rejects 0.
    pub slot_count: u32,
    /// `<sce:slot-size>` — bytes per slot. Parser rejects 0.
    pub slot_size: u32,
    /// `<sce:section>` — SRAM region name. Validated against deploy.yaml
    /// `memory.sram_regions` via [`compile_forge_with_deploy`].
    pub section: String,
    /// `<sce:alignment>` — DMA alignment. Parser rejects 0; the
    /// power-of-2 check happens at linker-fragment emission.
    pub alignment: u32,
    /// `<sce:dma-channel>` — DMA channel name (optional). Empty when
    /// the pool is purely CPU-managed. Cross-resolution against
    /// deploy.yaml `dma_channels` is not implemented until a consumer
    /// needs it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dma_channel: Option<String>,
    /// `<sce:cache-policy>` — `maintain` / `non-cacheable` / `none`.
    pub cache_policy: CachePolicy,
    /// `<sce:variant>` discriminator (RFC §synth-5-M line 2682). The
    /// `Default` arm covers the absence of `<sce:variant>` —
    /// regular RX/TX/stage pool behavior. `Reassembly` carries the
    /// three reassembly-specific fields (max-fragments-per-message,
    /// reassembly-timeout-ms, per-peer-quota).
    pub variant: BufferPoolVariant,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="buffer-pool">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Worker kind ────────────────────────────────────────────────

/// SPSC inbox ordering choice (RFC §synth-5-I lines 1752-1758). Drives
/// the atomic operations emitted on head/tail indices in both Rust and
/// C11 codegen. `AcqRel` is the safe default (every push/pop pairs
/// acquire+release on the index); `Relaxed` is the single-core
/// fast-path (no inter-thread synchronization). Cross-core placement +
/// `Relaxed` fires `worker/inbox-ordering-relaxed-across-cores` at
/// codegen time per spec line 1755-1756.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum InboxOrdering {
    /// `ordering="acq_rel"` — atomic load_acquire on head/tail reads,
    /// atomic store_release on writes. Required for any inbox whose
    /// producer + consumer halves are pinned to different cores.
    AcqRel,
    /// `ordering="relaxed"` — atomic load_relaxed / store_relaxed on
    /// head/tail. Single-core fast-path; cross-core placement with
    /// this choice raises `worker/inbox-ordering-relaxed-across-cores`.
    Relaxed,
}

impl std::fmt::Display for InboxOrdering {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AcqRel => write!(f, "acq_rel"),
            Self::Relaxed => write!(f, "relaxed"),
        }
    }
}

/// `<sce:inbox>` configuration — RFC §synth-5-D line 894 + §synth-5-I lines
/// 1752-1758. SPSC ring-buffer inbox shape; the
/// producer/consumer split is the type-level FSM
/// (`heapless::spsc::{Producer,Consumer}` on Rust; opaque
/// `sce_inbox_{producer,consumer}_t` family on C11). No separate FSM
/// IR module — slot lifecycle is 2-state (free/in-use), degenerate
/// compared to BufferPool's 7-state DMA lifecycle.
///
/// The `depth` attribute is spec-verbatim; the
/// `ordering` attribute is required (RFC §synth-5-I lines 1757-1758 —
/// SCE's error-only
/// wire realizes the spec "warning, codegen defaults to acq/rel" as
/// a required-when-worker-exists error so the author makes an
/// explicit choice before codegen emits ambiguous atomic ops).
/// The MPSC variant is not modelled: `ordering` is a per-worker
/// property here, and MPSC would make it a property of the queue
/// shared by N producers — a different type, not a value of this one
/// (RFC §6 tracked).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct InboxConfig {
    /// `<sce:inbox depth="N"/>` attribute body — fixed ring-buffer
    /// depth. Parser rejects 0. Spec line 894 verbatim attribute form.
    pub depth: u32,
    /// `<sce:inbox ordering="acq_rel|relaxed"/>` (RFC §synth-5-I lines
    /// 1752-1758). Required at parse time; absence fires
    /// `worker/inbox-ordering-unspecified`. Codegen wires the chosen
    /// memory ordering into the head/tail atomic operations on both
    /// Rust + C11 backends.
    pub ordering: InboxOrdering,
}

// ── Bounded-collection kind (RFC §synth-5-L) ─────────────────────────

/// `<sce:capacity>` source — RFC §synth-5-L lines 2553-2554 + 2600-2603.
///
/// Either resolved from `deploy.yaml` at codegen time
/// (`<sce:capacity source="deploy" key="machines.X.limits.Y"/>`) or a
/// build-time constant (`<sce:capacity const="N"/>`). Spec line 2583-2585
/// fixes the lowering: both shapes resolve to a per-language compile-time
/// constant in the emitted code. The cross-backend parity contract
/// (§synth-6.2.6) verifies that the same insertion sequence produces the same
/// iterator order on every backend.
///
/// The parser validates the XML attribute structure (exactly one of
/// `source="deploy" key=...` or `const="..."`); deploy-key
/// resolution against `MachineDeployConfig.limits` raises
/// `collection/capacity-unresolved` when the key does not resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CapacitySource {
    /// `<sce:capacity source="deploy" key="machines.X.limits.Y"/>` — key
    /// path against deploy.yaml resolved at codegen time. Empty / missing
    /// `key` attribute parse-rejects.
    DeployKey { key: String },
    /// `<sce:capacity const="N"/>` — build-time constant. Positive `u32`
    /// only; `0` or non-numeric parse-rejects.
    CompileConst { value: u32 },
}

/// `<sce:on-overflow>` policy — RFC §synth-5-L lines 2556-2557 + 2604.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum OverflowPolicy {
    /// `<sce:on-overflow>diagnostic-event</sce:on-overflow>` — emit a
    /// runtime diagnostic event when the table is full (default per spec
    /// line 2556).
    DiagnosticEvent,
    /// `<sce:on-overflow>reject</sce:on-overflow>` — return
    /// `Err(OverflowError)` from `insert`; caller decides.
    Reject,
    /// `<sce:on-overflow>oldest-wins</sce:on-overflow>` — drop the
    /// oldest entry to make room. Requires `<sce:ordering>insertion`
    /// (validated by [`DiagnosticCode::CollectionOverflowPolicyOldestWinsRequiresOrderingInsertion`]).
    OldestWins,
}

/// `<sce:ordering>` mode — RFC §synth-5-L lines 2558-2559 + 2605.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CollectionOrdering {
    /// `<sce:ordering>insertion</sce:ordering>` — iterator yields entries
    /// in insertion order (default per spec line 2558).
    Insertion,
    /// `<sce:ordering>sorted-by(index-by)</sce:ordering>` — iterator
    /// yields entries sorted by the `<sce:index-by field="...">` field.
    /// Requires `<sce:index-by>` to be present (validated by
    /// [`DiagnosticCode::CollectionOrderingSortedRequiresIndexBy`]).
    SortedByIndex,
}

/// `<sce:concurrency>` mode — RFC §synth-5-L lines 2560-2562 + 2606.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ConcurrencyMode {
    /// `<sce:concurrency>single-writer</sce:concurrency>` — exactly one
    /// task may call mutating ops. Default per spec line 2560.
    SingleWriter,
    /// `<sce:concurrency>multi-writer</sce:concurrency>` — multiple
    /// tasks may call mutating ops. Requires §synth-5-I `<sce:call>` atomic
    /// intrinsics imported (validated cross-doc by
    /// `collection/multi-writer-without-atomics`).
    MultiWriter,
}

/// Bounded-collection document — RFC §synth-5-L lines 2540-2655.
///
/// Typed container with build-time-declared capacity but runtime-varying
/// occupancy. The MCU use case (zenoh-pico parity) requires runtime
/// subscription declare/undeclare + queryable + in-flight reassembly
/// tables without a heap allocator.
///
/// Schema (per `<scxml sce:kind="bounded-collection">` body):
/// - `<sce:element-type>NAME</sce:element-type>` (required, body text)
///   — references another forge kind by name (codec-kind struct or
///   procedure-kind state record per spec line 2566-2567). Stored
///   as opaque `String` at parse time; cross-doc resolution against
///   `SceCrossDocRegistry` (verifying the name resolves AND is a
///   codec/procedure kind) raises
///   `collection/element-type-not-a-kind`.
/// - `<sce:capacity .../>` (required, exactly one attribute form) — see
///   [`CapacitySource`] doc.
/// - `<sce:index-by field="FIELD"/>` (optional) — enables `find_by_index`
///   API per spec line 2615. Stores the field name as
///   `Option<String>`; cross-doc verification that the field exists in
///   the element-type struct raises
///   `collection/index-by-field-missing`.
/// - `<sce:on-overflow>POLICY</sce:on-overflow>` (optional, default
///   `diagnostic-event`) — see [`OverflowPolicy`].
/// - `<sce:ordering>MODE</sce:ordering>` (optional, default `insertion`)
///   — see [`CollectionOrdering`].
/// - `<sce:concurrency>MODE</sce:concurrency>` (optional, default
///   `single-writer`) — see [`ConcurrencyMode`].
///
/// Parse-time validators:
/// - `collection/ordering-sorted-requires-index-by` — if `ordering` is
///   `SortedByIndex`, `<sce:index-by>` must be present.
/// - `collection/overflow-policy-oldest-wins-requires-ordering-insertion`
///   — if `on_overflow` is `OldestWins`, `ordering` must be `Insertion`
///   (spec line 2655).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct BoundedCollectionModel {
    pub name: String,
    /// `<sce:element-type>` body text — references another forge kind by
    /// name. Opaque `String` at parse time; resolved cross-doc.
    pub element_type: String,
    /// `<sce:capacity .../>` — deploy-key or compile-time constant.
    pub capacity: CapacitySource,
    /// `<sce:index-by field="...">` — optional index field for
    /// `find_by_index`. Opaque `Option<String>` at parse time; cross-doc
    /// element-struct field verification raises
    /// `collection/index-by-field-missing`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_by: Option<String>,
    /// `<sce:on-overflow>` — default `DiagnosticEvent` per spec line 2556.
    pub on_overflow: OverflowPolicy,
    /// `<sce:ordering>` — default `Insertion` per spec line 2558.
    pub ordering: CollectionOrdering,
    /// `<sce:concurrency>` — default `SingleWriter` per spec line 2560.
    pub concurrency: ConcurrencyMode,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="bounded-collection">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// Worker document — RFC §synth-5-D concurrent execution context driven
/// by a `<sce:link-rx>` source.
///
/// Schema (per `<scxml sce:kind="worker">` body):
/// - `<sce:link-rx ref="...">` (required) — `<scxml sce:kind="link">`
///   document that drives this worker. Cross-resolution validator:
///   `worker/link-rx-ref-unknown`.
/// - `<sce:inbox depth="N"/>` (required) — SPSC ring-buffer inbox.
///   Producer/consumer pair drawn from `heapless::spsc::split()` on
///   Rust; opaque `sce_inbox_{producer,consumer}_t` on C11.
/// - `<sce:outbox ref="...">` (optional) — recipient worker/state-
///   machine inbox for emitted events. Three cross-resolution
///   validators (`worker/outbox-ref-unknown` +
///   `worker/outbox-target-wrong-kind` +
///   `worker/outbox-target-suffix-invalid`) live in
///   [`crate::validate_worker_outbox_references`].
///   Parser keeps `outbox` as an opaque
///   `Option<String>`; semantic resolution against statechart +
///   worker recipients happens against the
///   [`crate::forge::cross_doc_registry::SceCrossDocRegistry`] in
///   the orchestrator pass.
/// - `<sce:body>` (optional, usually empty per spec line 897) — SCXML
///   actions. The parser scans for `worker/shared-mutable-state` violations
///   (any `<sce:import kind="worker">` in the document, plus body
///   SCXML data-refs to foreign namespaces).
///
/// MCU-class kind (RFC §synth-5-J-4): Rust + C11 emitters only. cpp/kotlin/
/// go/python rejection via the existing `codegen/mcu-class-kind-on-non-
/// mcu-language` family.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct WorkerModel {
    pub name: String,
    /// `<sce:link-rx ref="...">` — required driver source. Spec line
    /// 893 phrasing "drives this worker" → required. Empty-ref parse
    /// rejects; cross-ref resolution against `SceCrossDocRegistry`
    /// raises `worker/link-rx-ref-unknown`.
    pub link_rx: String,
    /// `<sce:inbox depth="N"/>` — required typed event queue.
    pub inbox: InboxConfig,
    /// `<sce:outbox ref="...">` — optional recipient inbox. When
    /// absent, the worker only injects events into the parent state
    /// machine via `<sce:link-rx>`-driven event mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outbox: Option<String>,
    /// SCE Protocol-Synthesis RFC §synth-5-O: post-preprocessor source
    /// position of the `<scxml sce:kind="worker">` root element.
    /// Drives the per-kind body function's SCE-MAP marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

// ── Forge document ─────────────────────────────────────────────

/// Top-level forge document — dispatched by `sce:kind` on `<scxml>` root.
///
/// `Statechart(Box<SCXMLModel>)` is the W3C SCXML arm — added with
/// AST-export v1 to close the internal symmetry left
/// open by the Forge-only first cut. The `Box` keeps the enum size
/// bounded by the largest forge variant: `size_of::<SCXMLModel>() ==
/// 816` bytes (measured at landing), while the largest forge model
/// (CodecModel) is 312 bytes. Inlining SCXMLModel would inflate
/// every `ForgeDocument` allocation by ~500 bytes regardless of
/// which variant carries data — boxing pays one indirection on the
/// statechart arm only.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind")]
pub enum ForgeDocument {
    #[serde(rename = "statechart")]
    Statechart(Box<crate::model::SCXMLModel>),
    #[serde(rename = "transform")]
    Transform(TransformModel),
    #[serde(rename = "lookup")]
    Lookup(LookupModel),
    #[serde(rename = "condition")]
    Condition(ConditionModel),
    #[serde(rename = "codec")]
    Codec(CodecModel),
    #[serde(rename = "validator")]
    Validator(ValidatorModel),
    #[serde(rename = "procedure")]
    Procedure(ProcedureModel),
    #[serde(rename = "filter")]
    Filter(FilterModel),
    #[serde(rename = "interpolation")]
    Interpolation(InterpolationModel),
    #[serde(rename = "timer")]
    Timer(TimerModel),
    #[serde(rename = "observer")]
    Observer(ObserverModel),
    #[serde(rename = "algorithm")]
    Algorithm(AlgorithmModel),
    #[serde(rename = "link")]
    Link(LinkModel),
    #[serde(rename = "buffer-pool")]
    BufferPool(BufferPoolModel),
    #[serde(rename = "worker")]
    Worker(WorkerModel),
    #[serde(rename = "bounded-collection")]
    BoundedCollection(BoundedCollectionModel),
    #[serde(rename = "enum")]
    Enum(EnumModel),
    /// Typed `_event.data` payload contract.
    /// Parse-time metadata plus per-backend payload struct
    /// codegen. See [`EventSchemaModel`].
    #[serde(rename = "event-schema")]
    EventSchema(EventSchemaModel),
}

impl ForgeDocument {
    pub fn name(&self) -> &str {
        match self {
            Self::Statechart(m) => &m.name,
            Self::Transform(m) => &m.name,
            Self::Lookup(m) => &m.name,
            Self::Condition(m) => &m.name,
            Self::Codec(m) => &m.name,
            Self::Validator(m) => &m.name,
            Self::Procedure(m) => &m.name,
            Self::Filter(m) => &m.name,
            Self::Interpolation(m) => &m.name,
            Self::Timer(m) => &m.name,
            Self::Observer(m) => &m.name,
            Self::Algorithm(m) => &m.name,
            Self::Link(m) => &m.name,
            Self::BufferPool(m) => &m.name,
            Self::Worker(m) => &m.name,
            Self::BoundedCollection(m) => &m.name,
            Self::Enum(m) => &m.name,
            Self::EventSchema(m) => &m.name,
        }
    }

    pub fn kind(&self) -> ForgeKind {
        match self {
            Self::Statechart(_) => ForgeKind::Statechart,
            Self::Transform(_) => ForgeKind::Transform,
            Self::Lookup(_) => ForgeKind::Lookup,
            Self::Condition(_) => ForgeKind::Condition,
            Self::Codec(_) => ForgeKind::Codec,
            Self::Validator(_) => ForgeKind::Validator,
            Self::Procedure(_) => ForgeKind::Procedure,
            Self::Filter(_) => ForgeKind::Filter,
            Self::Interpolation(_) => ForgeKind::Interpolation,
            Self::Timer(_) => ForgeKind::Timer,
            Self::Observer(_) => ForgeKind::Observer,
            Self::Algorithm(_) => ForgeKind::Algorithm,
            Self::Link(_) => ForgeKind::Link,
            Self::BufferPool(_) => ForgeKind::BufferPool,
            Self::Worker(_) => ForgeKind::Worker,
            Self::BoundedCollection(_) => ForgeKind::BoundedCollection,
            Self::Enum(_) => ForgeKind::Enum,
            Self::EventSchema(_) => ForgeKind::EventSchema,
        }
    }

    /// Precise runtime dependency for this parsed document.
    ///
    /// Unlike `ForgeKind::max_runtime_dep()` which returns the conservative
    /// upper bound, this inspects the actual parsed model to determine the
    /// exact tier. Content-dependent kinds:
    ///   - Procedure: L1 (guard-only) = None, L2 (event-driven) = ForgeRuntime.
    ///   - Lookup: string output (enum dispatch) = None, numeric = ForgeRuntime.
    pub fn runtime_dep(&self) -> RuntimeDep {
        match self {
            // W3C SCXML statechart — built on the SCE runtime
            // (sce_runtime). Matches `ForgeKind::Statechart`'s
            // conservative upper bound; AST export does not change
            // the runtime tier semantics.
            Self::Statechart(_) => RuntimeDep::SceRuntime,
            Self::Transform(_) | Self::Condition(_) | Self::Codec(_) | Self::Validator(_) => {
                RuntimeDep::None
            }
            Self::Lookup(m) => {
                if m.output_is_string() {
                    RuntimeDep::None
                } else {
                    RuntimeDep::ForgeRuntime
                }
            }
            Self::Filter(_) | Self::Interpolation(_) | Self::Observer(_) => {
                RuntimeDep::ForgeRuntime
            }
            Self::Timer(_) => RuntimeDep::ForgeRuntimeHal,
            Self::Procedure(m) => {
                if m.is_l2() {
                    RuntimeDep::ForgeRuntime
                } else {
                    RuntimeDep::None
                }
            }
            // RFC §synth-5-A: free function over language-native loops/locals.
            Self::Algorithm(_) => RuntimeDep::None,
            // RFC §synth-5-C: trait surface owned by SCE's `sce-link-runtime`;
            // per-OS impls live downstream. SCE-side tier `None`.
            Self::Link(_) => RuntimeDep::None,
            // RFC §synth-5-E: self-contained slot table
            // — no SCE-side runtime helper crate. Cache maintenance ops
            // route through §synth-5-I intrinsics; tier `None`.
            Self::BufferPool(_) => RuntimeDep::None,
            // RFC §synth-5-D: heapless::spsc on Rust + bare ring-buffer on C11
            // with §synth-5-I atomic intrinsics. No SCE-side runtime helper.
            Self::Worker(_) => RuntimeDep::None,
            // RFC §synth-5-L lines 2571-2581: self-contained slot table per
            // backend (heapless::Vec / std::array / Array+BooleanArray /
            // fixed array + mask / list + bytearray / C array + bitmap).
            // No SCE-side runtime helper crate. Cross-backend parity is
            // codegen-time (§synth-6.2.6).
            Self::BoundedCollection(_) => RuntimeDep::None,
            // Enum is a pure type declaration —
            // codegen lowers to a backend-native typed enum without
            // any SCE-side runtime helper.
            Self::Enum(_) => RuntimeDep::None,
            // EventSchema's codegen
            // emits a per-backend payload struct (typed fields only,
            // typed-Enum fields reference the Enum kind's emitted
            // type by qualified name). No SCE-side runtime helper —
            // matches Enum's stance.
            Self::EventSchema(_) => RuntimeDep::None,
        }
    }
}

/// Parsed forge result — combines the document model with cross-file imports.
/// The `imports` field is empty for standalone documents (no `<sce:import>` elements).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ParsedForge {
    pub document: ForgeDocument,
    pub imports: Vec<ForgeImport>,
    /// `<sce:extern>` declarations parsed from the document root
    /// (SCE Protocol-Synthesis RFC §synth-5-I). Empty for documents that
    /// do not declare any externs. Each entry has already been
    /// validated against the §synth-5-I baseline registry — wire-format
    /// rejection (4-code family `extern/symbol-not-in-whitelist` /
    /// `extern/abi-mismatch` / `extern/signature-mismatch` /
    /// `extern/ordering-unspecified`) raises during parsing, so a
    /// `ParsedForge` carrying any externs is guaranteed registry-clean.
    /// The parser produces this list; downstream codegen consumes it
    /// for per-language `extern "..." {}` emission (see
    /// [`crate::forge::extern_emit`]).
    ///
    /// Wire shape: always emitted (even as `[]`) so consumers see a
    /// uniform schema. `imports` follows the same rule. Earlier
    /// `skip_serializing_if = "Vec::is_empty"` forced consumers to
    /// branch on presence-vs-empty, a paper cut the AST export's
    /// long-term-correctness review surfaced.
    #[serde(default)]
    pub externs: Vec<ExternDeclaration>,
}

/// One `<sce:extern>` declaration after parse-time validation has
/// matched it against the §synth-5-I baseline registry. Carried through
/// `ParsedForge` so codegen can emit per-language
/// `extern "C" { fn ... }` blocks without re-walking the XML.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ExternDeclaration {
    /// Symbol name as authored — guaranteed to exactly match a
    /// registry entry's `name` (closed-set lookup at parse time).
    pub name: String,
    /// Signature as authored — guaranteed to byte-match the registry
    /// entry's `sig`.
    pub sig: String,
    /// ABI as authored, normalized to lowercase. One of `c` / `rust`.
    pub abi: String,
    /// `crate=` attribute value. Defaults to the registry entry's
    /// `crate_name` (today, `sce_intrinsics_runtime`) when the
    /// author omits the attribute. Stored rather than always-defaulted
    /// so a future plugin extension that overrides `crate` per-symbol
    /// surfaces here directly.
    pub crate_name: String,
    /// Source line of the `<sce:extern>` element, captured for
    /// downstream diagnostics that anchor on the declaration site.
    /// `#[serde(skip)]` for byte-stable wire format (mirrors
    /// `ForgeImport.line`).
    #[serde(skip)]
    pub line: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn forge_import_serialization_omits_line_field() {
        let imp = ForgeImport {
            src: "a.scxml".into(),
            kind: ForgeKind::Codec,
            alias: "a".into(),
            line: Some(42),
            embed_dispatch: None,
            flag_binds: Vec::new(),
        };
        let json = serde_json::to_value(&imp).expect("serialize ForgeImport");
        assert!(
            json.get("line").is_none(),
            "ForgeImport.line must be #[serde(skip)] for manifest byte-stability; got {json}"
        );
        let actual: BTreeSet<&str> = json
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        let expected: BTreeSet<&str> = ["src", "kind", "alias"].into_iter().collect();
        assert_eq!(actual, expected, "wire-format keys must remain stable");
    }

    #[test]
    fn procedure_transition_serialization_omits_line_field() {
        let tr = ProcedureTransition {
            target: "B".into(),
            cond: None,
            event: None,
            assigns: Vec::new(),
            line: Some(7),
        };
        let json = serde_json::to_value(&tr).expect("serialize ProcedureTransition");
        assert!(
            json.get("line").is_none(),
            "ProcedureTransition.line must be #[serde(skip)] for byte-stability; got {json}"
        );
    }

    #[test]
    fn procedure_state_serialization_omits_line_field() {
        let st = ProcedureState {
            id: "A".into(),
            is_final: false,
            transitions: Vec::new(),
            on_entry_sends: Vec::new(),
            done_params: Vec::new(),
            line: Some(11),
        };
        let json = serde_json::to_value(&st).expect("serialize ProcedureState");
        assert!(
            json.get("line").is_none(),
            "ProcedureState.line must be #[serde(skip)] for byte-stability; got {json}"
        );
    }
}
