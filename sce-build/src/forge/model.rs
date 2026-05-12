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
/// W3C SCXML Section 3.1 allows foreign namespace attributes on any element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    /// Signal filtering: moving average, low-pass, debounce (Phase 3).
    Filter,
    /// 1D/2D table interpolation (Phase 3).
    Interpolation,
    /// Periodic/delayed task timing (Phase 3).
    Timer,
    /// Threshold monitoring with hysteresis (Phase 3).
    Observer,
    /// Pure synchronous function with bounded loops and mutable locals
    /// — watching-zenoh RFC §5.A (Phase A3). Free function emit on
    /// every backend (`#![no_std]`-clean on Rust when no bytes param).
    Algorithm,
    /// Byte-stream link endpoint — watching-zenoh RFC §5.C. MCU-class
    /// kind (RFC §5.J.4): emits only on `(rust, *)` and
    /// `(c11, bare_metal)`. SCE owns the `Link` trait surface in
    /// `sce-link-runtime`; per-OS impls (`sce_link_runtime_lwip`,
    /// `_tokio`, `_qnx`) live downstream in watching-zenoh.
    Link,
    /// SRAM-placed, DMA-aligned slot table — watching-zenoh RFC §5.E.
    /// Second MCU-class kind (RFC §5.J.4): emits only on `(rust, *)`
    /// and `(c11, bare_metal)`. B7-α ships the minimum slot table on
    /// `(rust, std)` with `<sce:slot-count>` / `<sce:slot-size>` /
    /// `<sce:section>` / `<sce:alignment>` / `<sce:dma-channel>` /
    /// `<sce:cache-policy>` schema. The 7-state lifecycle FSM
    /// (`free` / `cpu-mut` / `dma-armed-{tx,rx}` / `dma-busy-{tx,rx}` /
    /// `cpu-ref`) defers to B7-γ; cache maintenance pinning defers to
    /// B7-δ (gated on §5.I `<sce:call>` intrinsic registry).
    BufferPool,
    /// Concurrent execution context driven by a `<sce:link-rx>` source —
    /// watching-zenoh RFC §5.D lines 858-913. Third MCU-class kind (RFC
    /// §5.J.4): emits only on `(rust, *)` (tokio::spawn on AP, cooperative-
    /// scheduler slot on MCU) and `(c11, bare_metal)` (cooperative-
    /// scheduler slot, fixed ring-buffer inbox). C2-α ships the schema +
    /// parse-time author guard for `worker/shared-mutable-state`; codegen
    /// (Rust + C11 dual-emit using heapless::spsc / opaque sce_inbox_*
    /// types) lands in C2-β alongside cross-resolution + ordering codes;
    /// deploy-aware `MachineSchedulerConfig` + worker-count validation
    /// lands in C2-γ.
    Worker,
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
            _ => None,
        }
    }

    /// Whether this kind can appear inline within a statechart `<data>` element.
    /// Only stateless kinds are inline-eligible.
    pub fn is_inline_eligible(&self) -> bool {
        // RFC §5.A: Algorithm emits a free function (stateless) but is
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
            // RFC §5.A: Algorithm is a free function — no instance state.
            Self::Algorithm => false,
            // RFC §5.C: Link emits a struct that owns an `impl Link`
            // driver and routes RX/TX through the framer codec. The
            // generated module exposes a constructor that the consumer
            // wires to a downstream `sce_link_runtime_<os>` impl.
            Self::Link => true,
            // RFC §5.E: BufferPool emits a struct owning a fixed-size
            // slot table (B7-α `(rust, std)` initial shape; B7-γ adds
            // the 7-state lifecycle FSM with phantom-typed `Slot<state>`
            // API + IR-level borrow check). Acquire/return surface lives
            // on the struct, not as free functions.
            Self::BufferPool => true,
            // RFC §5.D: Worker emits a struct owning an SPSC inbox
            // (heapless::spsc on Rust; opaque sce_inbox_{producer,
            // consumer}_t handle pair on C11). The inbox storage + head/
            // tail indices are instance state of the generated struct.
            Self::Worker => true,
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
            Self::Transform | Self::Condition | Self::Codec
            | Self::Validator => RuntimeDep::None,
            // Lookup: string output = None (enum dispatch), numeric = ForgeRuntime.
            // Procedure: L1 = None, L2 = ForgeRuntime.
            // Upper bound for both is ForgeRuntime.
            Self::Lookup | Self::Procedure => RuntimeDep::ForgeRuntime,
            Self::Filter | Self::Interpolation | Self::Observer => RuntimeDep::ForgeRuntime,
            Self::Timer => RuntimeDep::ForgeRuntimeHal,
            Self::Statechart => RuntimeDep::SceRuntime,
            // RFC §5.A: Algorithm bottom-outs to language-native loops
            // and locals, no helper crate. `#![no_std]`-clean on Rust
            // when no `bytes` parameter.
            Self::Algorithm => RuntimeDep::None,
            // RFC §5.C: Link's generated code depends on the `Link`
            // trait surface owned by SCE's `sce-link-runtime` crate.
            // No SCE-side runtime dependency tier captures "downstream
            // crate" — the trait surface is contract, the impl lives
            // downstream. Tier `None` is honest at the SCE level.
            Self::Link => RuntimeDep::None,
            // RFC §5.E: BufferPool ships a self-contained slot table
            // on `(rust, std)` in B7-α — no runtime helper crate
            // dependency. B7-γ's IR-level borrow check is codegen-time;
            // B7-δ's cache maintenance pinning routes through §5.I
            // intrinsics, themselves contracts not runtime helpers.
            // SCE-side tier `None` matches Link's stance.
            Self::BufferPool => RuntimeDep::None,
            // RFC §5.D: Worker uses `heapless::spsc` on Rust (third-party
            // crate, not SCE-side helper) and bare ring-buffer + atomics
            // intrinsics on C11. The atomic ordering intrinsics come
            // through §5.I `<sce:extern>` whitelist (`sce_intrinsics_runtime`
            // baseline); the SCE-side helper-crate tier is `None`.
            Self::Worker => RuntimeDep::None,
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
        }
    }
}

// ── Cross-language type system ─────────────────────────────────

/// Canonical SCE type — used in `sce:type` attributes.
/// Language-specific mappings are in the generator module (SRP).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

impl SceType {
    pub fn from_attr(s: &str) -> Option<Self> {
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
        matches!(self, Self::Uint8 | Self::Uint16 | Self::Uint32 | Self::Uint64)
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

    /// Bit width for fixed-width integer types — used by RFC §5.B B1-γ
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
}

// ── Field direction ────────────────────────────────────────────

/// Data flow direction for kind fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
pub struct ForgeField {
    pub id: String,
    pub sce_type: SceType,
    pub direction: Direction,
    /// ECMAScript expression (for computed/output fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    /// Documentation-only unit (no codegen effect).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Per-slot capacity for `bytes`-typed fields, declared via
    /// `sce:max-size="N"`. `None` ⇒ fall back to
    /// [`crate::forge::limits::BYTES_DEFAULT_MAX`]. Ignored for non-bytes
    /// types. See `claudedocs/rfc-forge-bytes-bounded.md` §3 B1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u32>,
}

// ── Transform kind ─────────────────────────────────────────────

/// Transform: pure mathematical formula. Input -> computation -> output.
/// No state, no side effects. Generates an inline function.
#[derive(Debug, Clone, Serialize)]
pub struct TransformModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub outputs: Vec<ForgeField>,
}

// ── Lookup kind ────────────────────────────────────────────────

/// A single key-value entry in a lookup table.
#[derive(Debug, Clone, Serialize)]
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
pub struct LookupModel {
    pub name: String,
    pub input: ForgeField,
    pub output: ForgeField,
    pub entries: Vec<LookupEntry>,
    pub miss_policy: MissPolicy,
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

// ── Condition kind ─────────────────────────────────────────────

/// Condition: named boolean guard expression. Generates an inline bool function.
#[derive(Debug, Clone, Serialize)]
pub struct ConditionModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    /// ECMAScript expression that evaluates to boolean.
    pub expr: String,
}

// ── Validator kind ─────────────────────────────────────────

/// Validator: range/rate-of-change/plausibility checks.
/// Has internal state (previous values for rate-of-change detection).
#[derive(Debug, Clone, Serialize)]
pub struct ValidatorModel {
    pub name: String,
    pub inputs: Vec<ForgeField>,
    pub rules: ValidatorRules,
}

/// Validation rules container.
#[derive(Debug, Clone, Serialize)]
pub struct ValidatorRules {
    pub ranges: Vec<RangeRule>,
    pub rate_of_changes: Vec<RateOfChangeRule>,
    pub plausibility: Option<String>,
}

/// Range check: field value must be within [min, max].
#[derive(Debug, Clone, Serialize)]
pub struct RangeRule {
    pub id: String,
    pub min: Option<String>,
    pub max: Option<String>,
}

/// Rate-of-change check: delta between successive calls must not exceed max_delta.
/// sample_interval_ms is informational (documents expected call frequency).
#[derive(Debug, Clone, Serialize)]
pub struct RateOfChangeRule {
    pub id: String,
    pub max_delta: String,
    pub sample_interval_ms: u32,
}

// ── Procedure kind ────────────────────────────────────────────

// ── Level 2 types (event-driven procedure) ───────────────────

/// A `<send>` action within `<onentry>` of a procedure state.
/// Dispatches a service request through the procedure's service handler.
/// W3C SCXML 6.2 + SCE extensions (sce:service, sce:subfunc, sce:addr, sce:payload).
#[derive(Debug, Clone, Serialize)]
pub struct ProcedureSendAction {
    /// Service name (sce:service attribute). Required.
    pub service: String,
    /// Sub-function code (sce:subfunc attribute). Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subfunc: Option<String>,
    /// Address expression — typically a variable name (sce:addr attribute). Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    /// Payload expression (sce:payload attribute). Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// Cap on the bytes the service handler may return as `_event.data`,
    /// declared via `sce:response-max-size="N"`. `None` ⇒ fall back to
    /// [`crate::forge::limits::BYTES_DEFAULT_MAX`]. See
    /// `claudedocs/rfc-forge-bytes-bounded.md` §3 B1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_max_size: Option<u32>,
}

/// An `<assign>` action within a `<transition>` body.
/// Mutates internal state during transition execution.
#[derive(Debug, Clone, Serialize)]
pub struct ProcedureAssign {
    /// Target variable name (location attribute).
    pub location: String,
    /// Value expression (expr attribute).
    pub expr: String,
}

/// A `<param>` within `<donedata>` on a `<final>` state.
/// Provides result data when the procedure completes.
#[derive(Debug, Clone, Serialize)]
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
pub struct ProcedureTransition {
    /// Target state id.
    pub target: String,
    /// Optional ECMAScript guard expression. `None` = unconditional (else branch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cond: Option<String>,
    /// Event trigger (Level 2). `None` = eventless transition (guard-only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Assign actions executed during transition (Level 2). Empty for Level 1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
pub struct ProcedureState {
    pub id: String,
    /// Whether this is a `<final>` state (terminal — no outgoing transitions).
    pub is_final: bool,
    /// Ordered transitions (evaluated top-to-bottom). Empty for final states.
    pub transitions: Vec<ProcedureTransition>,
    /// Send actions in `<onentry>` (Level 2). Empty for Level 1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub on_entry_sends: Vec<ProcedureSendAction>,
    /// Done data parameters (Level 2, final states only). Empty for Level 1.
    #[serde(skip_serializing_if = "Vec::is_empty")]
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
    /// when `returns` is non-bytes. See
    /// `claudedocs/rfc-forge-bytes-bounded.md` §3 B1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub returns_max_size: Option<u32>,
}

/// Procedure: sequential branching logic with states and guarded transitions.
/// Level 1 (guard-only): CRTP base + switch/case, stateless `execute()`.
/// Level 2 (event-driven): StaticExecutionEngine<Policy> + `runToCompletion()`.
#[derive(Debug, Clone, Serialize)]
pub struct ProcedureModel {
    pub name: String,
    /// Input parameters (sce:direction="in").
    pub inputs: Vec<ForgeField>,
    /// Internal state variables (sce:direction="internal"). Level 2 only.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub internals: Vec<ForgeField>,
    /// User-declared helper function DI points (see [`ProcedureHelper`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub helpers: Vec<ProcedureHelper>,
    /// Id of the initial state (from `initial` attribute on `<scxml>`).
    pub initial: String,
    /// All states in document order (regular + final).
    pub states: Vec<ProcedureState>,
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
            || self.states.iter().any(|s| {
                !s.on_entry_sends.is_empty() || !s.done_params.is_empty()
            })
    }
}

// ── Codec kind ─────────────────────────────────────────────────

/// Endianness for byte-level operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Endian {
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

impl Default for Endian {
    fn default() -> Self {
        Self::Big
    }
}

/// RFC §5.B B2 repeat primitive — element count source for a
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

/// RFC §5.B B3 TLV chain on-overflow policy. Names what the decoder does
/// when the cursor still has bytes after `max_depth` entries have been
/// consumed (i.e. the wire carries more entries than the codec author
/// declared).
///
/// v1 ships `Reject` + `Truncate`. `DiagnosticEvent` (RFC line 488) is
/// deferred until §5.A diagnostic-event runtime infrastructure surfaces a
/// reachable consumer — adding it now would be built-but-unconsumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

/// RFC §5.B Y3 — TLV chain termination strategy. Names how the chain
/// decoder decides where the chain ends; pairs with the existing
/// `max_depth` safety bound + `on_overflow` policy. v1 has two shapes:
///
///   - [`TlvTerminateStrategy::ExhaustOrDepth`] (B3 trunk default): the
///     chain consumes entries until the cursor exhausts or `max_depth`
///     is reached. Used by `codec_tlv_chain_basic` + (pre-Y3) the
///     `codec_zenoh_ext_envelope` demo. Acceptable when nothing
///     follows the chain on the wire — the trailing cursor IS the
///     chain's tail.
///
///   - [`TlvTerminateStrategy::EntryFlag`] (Y3): each entry's outer
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
/// about chain length.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(tag = "type")]
pub enum TlvTerminateStrategy {
    /// Trunk default: chain ends on cursor exhaustion or `max_depth`.
    #[default]
    ExhaustOrDepth,
    /// Y3: chain ends when the last-decoded entry's named flag bit is
    /// clear. Forward-compat with multi-ext zenoh wire format.
    EntryFlag { flag_name: String },
}

fn is_exhaust_or_depth(s: &TlvTerminateStrategy) -> bool {
    matches!(s, TlvTerminateStrategy::ExhaustOrDepth)
}

/// Bit size specification for codec fields.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum BitSize {
    /// Fixed bit count: 8, 16, 24, 32, 64.
    Fixed { bits: u32 },
    /// Remaining bytes from byte offset to end of frame.
    Tail,
    /// Size determined by another field's value (see CodecField::length_field).
    LengthRef,
    /// Variable-length encoded integer (RFC §5.B Appendix B). The
    /// `width_bits` cap (16/32/64) names the value-type max width;
    /// wire bytes consumed is `1..=ceil(width_bits / 7)`. Canonical
    /// Zenoh ZInt is `Vle { width_bits: 64 }`.
    Vle { width_bits: u32 },
    /// RFC §5.B B2 repeat primitive — list of imported-codec elements.
    /// The decoded host type is `Vec<T>` / `std::vector<T>` / ... where
    /// T is resolved via the field's [`CodecField::repeat_body_alias`].
    /// `count_ref` selects the loop termination strategy
    /// (length-field vs until-eof).
    Repeat { count_ref: CountRef },
    /// RFC §5.B B3 TLV chain primitive — bounded extension list
    /// (Type-Length-Value). Iteratively decodes entries up to
    /// `max_depth` (RFC: "iterative, never recursive"); residual bytes
    /// after the cap are handled per [`TlvOverflowPolicy`]. Entry body
    /// codec is resolved via [`CodecField::tlv_chain_body_alias`]
    /// (mirrors `repeat_body_alias`). MCU-class — emits only on Rust +
    /// C11; cpp/kotlin/go/python codecs containing this field type are
    /// rejected at codegen via `codegen/mcu-class-kind-on-non-mcu-language`
    /// (the existing kind-class diagnostic is repurposed at the
    /// codec-content granularity, see RFC §5.B "MCU-only codec
    /// sub-features").
    TlvChain {
        max_depth: u32,
        on_overflow: TlvOverflowPolicy,
        /// RFC §5.B Y3 — termination strategy. Defaults to
        /// `ExhaustOrDepth` for backward-compat with B3 trunk fixtures
        /// (cursor-exhaust + max_depth). `EntryFlag(flag_name)` reads
        /// the entry's named flag after each decode and terminates
        /// the loop when the flag is clear — required when something
        /// follows the chain on the wire (zenoh-pico ext chains
        /// followed by message body variants).
        #[serde(skip_serializing_if = "is_exhaust_or_depth", default)]
        terminate_on: TlvTerminateStrategy,
    },
    /// RFC §5.B Y0c — single imported-codec field embedded inline.
    /// The host language emits a nested struct of the imported codec's
    /// type; the streaming codec calls the imported codec's
    /// decode/encode for this position. No wire-level boundary bytes
    /// (no length prefix, no tag) — the embedded codec's own field
    /// layout consumes/produces bytes directly. Imported codec is
    /// resolved via [`CodecField::embed_body_alias`] (mirrors
    /// `repeat_body_alias`). RFC §5.B B5-γ parent-flag threading
    /// applies when the embedded codec declares
    /// `<sce:requires-parent-flags>`.
    ///
    /// First reachable consumer: zenoh-pico declare/undeclare bodies
    /// (decl_keyexpr, decl_subscriber, decl_queryable, decl_token)
    /// embed `codec_zenoh_wireexpr` after a VLE id field.
    Embed,
}

/// A single named bit-range on a `<sce:flags>` carrier field — RFC §5.B
/// B1-γ + B5-α. `bit` is the LSB position within the carrier's natural
/// integer width (0 = LSB), validated at parse time against the
/// carrier's [`SceType::int_bit_width`]. `width` is the contiguous
/// bit-range size (defaults to 1 for B1-γ single-bit shape; >1 for
/// B5-α multi-bit accessors like Zenoh's `_z_n_qos_t._val.priority:3`).
/// `bit + width <= carrier_int_bit_width` is parser-enforced; per-flag
/// bit-ranges within the same carrier may not overlap.
#[derive(Debug, Clone, Serialize)]
pub struct FlagDef {
    pub name: String,
    pub bit: u32,
    pub width: u32,
}

/// RFC §5.B B5-γ present-if predicate scope — distinguishes the
/// B1-δ local form (carrier in same codec) from the B5-γ parent
/// form (carrier in declared `<sce:requires-parent-flags>` block,
/// passed by value into the codec's decode/encode signature as
/// `parent_flags`).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PresentIfScope {
    /// `<field_id>.<flag_name>` — B1-δ form. Carrier is a
    /// flags-bearing sibling field declared earlier in the same
    /// codec; predicate reads `(self.<carrier> & mask) != 0`.
    Local,
    /// `parent.<flag_name>` — B5-γ form. Carrier is the codec's
    /// declared `requires_parent_flags`; predicate reads
    /// `(parent_flags & mask) != 0` against the value threaded in
    /// by the variant arm dispatcher.
    Parent,
}

/// RFC §5.B B1-δ + B5-γ + B5-λ present-if predicate — a single
/// bit-test on either a flags-bearing sibling field declared earlier
/// in the same codec (B1-δ Local scope) or a flag declared in the
/// codec's `<sce:requires-parent-flags>` block (B5-γ Parent scope).
///
/// v1 grammar covers six forms:
///   - `<field_id>.<flag_name>` (Local positive) — predicate is true
///     iff the named flag bit on the local carrier is set.
///   - `!<field_id>.<flag_name>` (Local negative, B5-λ) — predicate
///     is true iff the bit is clear. Required for Zenoh OpenSyn body
///     where cookie is present iff parent.A is NOT set.
///   - `parent.<flag_name>` (Parent positive) — true iff the named
///     flag bit is set on the value passed in by the parent variant
///     dispatcher.
///   - `!parent.<flag_name>` (Parent negative, B5-λ) — true iff the
///     bit is clear. Same Zenoh OpenSyn rationale at parent scope.
///   - `<a> || <b> [|| <c> ...]` (Disjunction, Y3 atomic 2b-ii) —
///     predicate is true iff ANY listed clause is true. Each clause
///     is itself one of the four forms above (each can independently
///     carry a leading `!`). Required for Zenoh interest where
///     `not is_final` is `header.CURRENT || header.FUTURE` —
///     `_Z_INTEREST_NOT_FINAL_MASK = (CURRENT | FUTURE)` per
///     `interest.h:35`. Outer negation `!(a || b)` defers to a
///     future RFC stage.
///
/// `field_id` is empty when scope = Parent (carrier is implicit —
/// the codec's declared `requires_parent_flags.carrier`).
///
/// Conjunction (`flag1 && flag2`) and equality (`field == value`)
/// remain deferred to later B-stages when a reachable consumer
/// surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PresentIfPredicate {
    /// Predicate scope. Defaults to `Local` for back-compat with
    /// pre-B5-γ goldens; serialized only when `Parent` to keep
    /// existing local-scope JSON shape byte-stable.
    #[serde(skip_serializing_if = "is_local_scope")]
    pub scope: PresentIfScope,
    /// Carrier field id when `scope = Local`. Empty when `scope =
    /// Parent` (the carrier is implicit — the codec's declared
    /// `requires_parent_flags.carrier`).
    pub field_id: String,
    pub flag_name: String,
    /// B5-λ negation: when true, predicate fires when the named flag
    /// bit is *clear* (not set). Defaults to false to preserve
    /// pre-B5-λ JSON shape byte-stable across all existing codec
    /// goldens (skip_serializing_if elides the field).
    #[serde(skip_serializing_if = "is_false", default)]
    pub negate: bool,
    /// RFC §5.B Y3 atomic 2b-ii disjunction tail — when `Some`, the
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

/// RFC §5.B B5-γ — the codec's declared dependency on a parent
/// codec's flags carrier. Authored as `<sce:requires-parent-flags
/// carrier="X"><sce:flag name="N" bit="B"/></sce:requires-parent-flags>`
/// under `<sce:codec>`.
///
/// Validator (cross-codec, at variant arm wire-up) confirms the
/// parent codec has `<sce:flags id="<carrier>">` of `uint8` with
/// each declared flag name + bit position matching exactly.
/// Mismatch surfaces as `codec/parent-flag-mismatch`.
///
/// Codegen extends the codec's decode/encode signature with a
/// `parent_flags: u8` parameter (per-language idiom) when this
/// field is `Some`; variant arm dispatcher threads the parent's
/// flag-carrier value into the arm decoder call.
///
/// v1 fixes the parent flag carrier type at `uint8` (Zenoh
/// transport pattern). Future widening to uint16+ defers to a
/// reachable consumer.
#[derive(Debug, Clone, Serialize)]
pub struct RequiresParentFlags {
    /// Parent codec's flags-carrier field id (e.g. `"header"`).
    pub carrier: String,
    /// Per-flag declarations mirroring the parent's `<sce:flags>`
    /// child layout. Validator confirms exact name + bit match
    /// against the parent codec.
    pub flags: Vec<FlagDef>,
}

/// A single field in a codec's byte layout.
#[derive(Debug, Clone, Serialize)]
pub struct CodecField {
    pub id: String,
    pub sce_type: SceType,
    /// Byte offset within the frame.
    pub byte_offset: u32,
    /// Bit offset within the byte (for sub-byte fields).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_offset: Option<u32>,
    /// Bit size of this field.
    pub bit_size: BitSize,
    /// Per-field endianness override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endian: Option<Endian>,
    /// Maximum size for variable-length fields (tail, length-ref).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u32>,
    /// Referenced field ID for LengthRef bit size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_field: Option<String>,
    /// Named bits — RFC §5.B B1-γ flags primitive. Empty for plain
    /// fields; populated when the field was authored as a `<sce:flags>`
    /// container with `<sce:flag name=... bit=N/>` children. Codegen
    /// emits per-flag get/set accessors after the encode/decode methods
    /// without changing the wire layout (the field still occupies the
    /// same bytes as a regular unsigned-int).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub flags: Vec<FlagDef>,
    /// RFC §5.B B1-δ present-if primitive. `Some(predicate)` when the
    /// field carries a `sce:present-if="<carrier>.<flag>"` attribute;
    /// the field's host-language type is wrapped as a per-language
    /// optional (`Option<T>` / `std::optional<T>` / `T?` / `*T` /
    /// `Optional[T]` / `bool has_<id>; T <id>` paired in C11) and the
    /// streaming decode/encode skips the field's bytes when the
    /// predicate is false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub present_if: Option<PresentIfPredicate>,
    /// RFC §5.B B2 repeat primitive — imported codec alias whose
    /// decode/encode handles each element. `Some(alias)` only when
    /// `bit_size = BitSize::Repeat`. Resolved against `<sce:import>`
    /// aliases at codegen time (mirrors variant arm body_alias
    /// resolution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_body_alias: Option<String>,
    /// RFC §5.B B2 maximum element count for `BitSize::Repeat` fields
    /// — used by encode-buffer sizing to bound `min_frame + count *
    /// element_max`. Defaults to [`crate::forge::limits::REPEAT_DEFAULT_MAX_COUNT`]
    /// when absent (mirrors `max_size`'s fallback for Tail/LengthRef
    /// bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u32>,
    /// RFC §5.B B3 TLV chain primitive — imported codec alias whose
    /// decode/encode handles each entry. `Some(alias)` only when
    /// `bit_size = BitSize::TlvChain`. Resolved against `<sce:import>`
    /// aliases at codegen time (mirrors `repeat_body_alias`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tlv_chain_body_alias: Option<String>,
    /// RFC §5.B B3 DMA alignment primitive — `sce:dma-burst-align="N"`
    /// declares this field's start offset within the encoded buffer is
    /// constrained to an N-byte boundary (typical N: 16, 32, 64). The
    /// constraint is build-time: codegen verifies `byte_offset % N == 0`
    /// (rejects with `codec/dma-alignment-unsatisfiable` otherwise) AND
    /// validates that every preceding field is Fixed bit-size (so the
    /// post-padding layout is statically computable). Codegen emits
    /// language-level alignment assertions (`_Static_assert` / `const _:
    /// () = assert!`) on the literal offset for drift protection.
    /// MCU-class — codecs containing this attribute emit only on Rust +
    /// C11. After B5-ε closures TLV chain is no longer MCU-only (Zenoh
    /// extension envelopes ship on server-class peers too); DMA align
    /// is the only remaining codec sub-feature gated to MCU backends.
    /// `None` when no alignment constraint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dma_burst_align: Option<u32>,
    /// RFC §5.B Y0c — embedded imported-codec alias for
    /// `BitSize::Embed` fields. `Some(alias)` only when bit_size is
    /// Embed; mirrors `repeat_body_alias` / `tlv_chain_body_alias`
    /// for the single-codec inline case. Resolved against
    /// `<sce:import>` aliases at codegen time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_body_alias: Option<String>,
    /// RFC §5.B Y0b — `sce:length-from="<id>"` on a `BitSize::Embed`
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
    /// `None` for the Y0c always-present always-cursor-direct shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed_length_from: Option<String>,
    /// RFC §5.B B5-δ Surface F — arithmetic offset on the
    /// `length-field` source value. Authored as `sce:length-arith="+1"`
    /// or `sce:length-arith="-1"` paired with `sce:bit-size="length-ref"`
    /// + `sce:length-field="..."`. Effective payload length is
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
    /// trust contract from B1-β).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_arith: Option<i32>,
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

    /// Whether this field is a repeat-of-imported-codec list (RFC §5.B
    /// B2). The host language emits `Vec<T>` / `std::vector<T>` / etc.
    /// and the streaming codec iterates element decode/encode according
    /// to [`CountRef`].
    pub fn is_repeat(&self) -> bool {
        matches!(self.bit_size, BitSize::Repeat { .. })
    }

    /// Whether this field is a TLV chain (RFC §5.B B3). The host
    /// language emits `Vec<T>` (Rust) / fixed-array + len pair (C11)
    /// and the streaming codec iterates element decode/encode up to
    /// `max_depth` then applies the [`TlvOverflowPolicy`]. MCU-class
    /// — codecs containing this field type emit only on Rust + C11.
    pub fn is_tlv_chain(&self) -> bool {
        matches!(self.bit_size, BitSize::TlvChain { .. })
    }

    /// Whether this field is a Y0c single-codec embed (RFC §5.B Y0c).
    /// The host language emits a nested struct of the imported codec's
    /// type; the streaming codec calls the imported codec's
    /// decode/encode for this position with no wire-level boundary.
    pub fn is_embed(&self) -> bool {
        matches!(self.bit_size, BitSize::Embed)
    }

    /// Whether this field carries a `<sce:flag>` set (RFC §5.B B1-γ).
    /// Used by the present-if validator (B1-δ) to verify a predicate's
    /// LHS resolves to a flags-bearing carrier.
    pub fn is_flags_carrier(&self) -> bool {
        !self.flags.is_empty()
    }

    /// Whether this field is a UTF-8 string (RFC §5.B B5-ζ Surface H).
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
/// type provides the arm's body. RFC §5.B "Discriminated union":
/// `<sce:arm value="0x01" type="SessionOpen"/>` where `SessionOpen` is
/// an imported codec alias (B1-β v1 limits arm bodies to imported codec
/// kinds; primitive arm bodies and `<sce:default>` body inheritance
/// arrive when their first reachable consumer ships).
#[derive(Debug, Clone, Serialize)]
pub struct VariantArm {
    /// Discriminator value (matches the tag field's read value).
    /// Held as `u64` to fit any unsigned tag width up to uint64.
    pub value: u64,
    /// Import alias naming the body codec for this arm.
    pub body_alias: String,
}

/// RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte dispatch shape on a
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

/// Discriminated-union suffix on a codec — RFC §5.B Codec DSL.
///
/// Decode reads the named tag field (or named flag bit-range within it,
/// when `tag_flag` is set), then dispatches into the matching arm's
/// body codec. Encode writes the tag bytes followed by the active
/// arm's body bytes. The optional `<sce:default>` arm catches any tag
/// value not enumerated; absent default + non-exhaustive arm coverage
/// fires `codec/variant-arm-unreachable` at build time (see RFC §5.B).
#[derive(Debug, Clone, Serialize)]
pub struct CodecVariant {
    /// `id` of the field (within this codec's `fields`) whose decoded
    /// value (or named bit-range, see `tag_flag`) selects an arm. Must
    /// reference an unsigned-int field (uint8/uint16/uint32/uint64) —
    /// enforced at parse time.
    ///
    /// RFC §5.B Y3 atomic 2b-ii peek-byte peek-byte mode: when
    /// `peek_byte` is `Some`, `tag_field` equals `peek_byte.id` (the
    /// peek slot's name, NOT a real codec field). The carrier value
    /// is the cursor's next byte (read via `peek_slice(1)`, no
    /// advance); arm body decoder reads the same byte as its own
    /// header.
    pub tag_field: String,
    /// RFC §5.B B5-β multi-bit-flag dispatch: when `Some(name)`, the
    /// `tag_field` MUST be a `<sce:flags>`-bearing carrier and `name`
    /// names one of its `<sce:flag>` bit-ranges. The dispatch value is
    /// `(carrier >> bit) & ((1 << width) - 1)` — the bit-range's
    /// shifted-and-masked unsigned scalar. When `None`, the dispatch
    /// reads the field's whole value (B1-β whole-field form).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_flag: Option<String>,
    /// Enumerated arms in document order.
    pub arms: Vec<VariantArm>,
    /// Catch-all arm for tag values outside the enumerated set.
    /// `None` ⇒ build-time `codec/variant-arm-unreachable` when the
    /// tag domain isn't fully covered by `arms`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_arm: Option<VariantArm>,
    /// RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte mode dispatch
    /// (Zenoh response/request body MID). `Some` ⇒ variant tag reads
    /// `peek_byte`'s next-byte view (carrier is the cursor's next
    /// byte without advancing); arm body codec reads same byte as
    /// own header. `None` ⇒ B1-β own-field mode (back-compat:
    /// existing variant goldens omit this field via
    /// skip_serializing_if).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peek_byte: Option<PeekByteSpec>,
}

/// Codec: byte-level encode/decode. Generates struct with decode/encode methods.
#[derive(Debug, Clone, Serialize)]
pub struct CodecModel {
    pub name: String,
    /// Document-level default endianness.
    pub default_endian: Endian,
    /// Expected input frame length (from `sce:length` on input data).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_length: Option<u32>,
    /// Ordered list of fields in the codec.
    pub fields: Vec<CodecField>,
    /// Optional discriminated-union suffix — RFC §5.B variant primitive
    /// (B1-β). When present the codec emits a sum type per language
    /// (Rust enum, Kotlin sealed class, C11 tagged union, etc.) rather
    /// than a flat struct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<CodecVariant>,
    /// RFC §5.B B5-γ parent-flags dependency — the codec's body
    /// fields read flags from a parent codec's flags carrier
    /// (Zenoh upstream pattern: `_z_init_decode(.., uint8_t header)`
    /// gates `sn_res + req_id_res + batch_size` on parent's S-flag
    /// bit 6). When `Some`, codegen extends the decode/encode
    /// signature with a `parent_flags: u8` parameter and the variant
    /// arm dispatcher threads the parent's flag-carrier value
    /// through. Cross-codec validator confirms the parent codec's
    /// `<sce:flags id="<carrier>">` matches the body's declared
    /// flag layout (name + bit).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_parent_flags: Option<RequiresParentFlags>,
    /// RFC §5.B B5-θ inline test vectors. Each row carries one wire
    /// `hex` byte sequence + the expected decoded field-value tree.
    /// Generates a per-backend round-trip sidecar (`<fixture>_test.{rs,h}`)
    /// next to the codec header — symmetric with the algorithm
    /// `<sce:test-vector>` machinery (RFC §5.B B2). Trunk lands on
    /// Rust + C11 with plain (non-variant, non-TLV-chain, non-parent-
    /// flags) codecs only; variant + recursive-variant + TLV-chain +
    /// parent-flags codecs reject through the per-language gate in
    /// `render_codec_test_vector_sidecar` until B5-θ closures land.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub test_vectors: Vec<CodecTestVector>,
}

impl CodecModel {
    /// Minimum required frame bytes for decode (from fixed fields).
    pub fn min_frame_bytes(&self) -> u32 {
        let mut max_end = 0u32;
        for field in &self.fields {
            if let Some(bits) = field.fixed_bits() {
                let end = field.byte_offset + (bits + 7) / 8;
                max_end = max_end.max(end);
            }
        }
        max_end
    }

    /// Maximum frame bytes for encode buffer sizing: fixed bytes plus the
    /// resolved per-field cap of every variable-length field. Caps:
    ///   - `tail` / `length-ref`: `sce:max-size`, fallback to
    ///     [`crate::forge::limits::BYTES_DEFAULT_MAX`].
    ///   - `vle { width_bits }`: `ceil(width_bits / 7)` (3 / 5 / 10 for
    ///     u16 / u32 / u64) — base-128 worst case from RFC §5.B App. B.
    ///   - `repeat { count_ref }` (RFC §5.B B2): contributes 0 here —
    ///     the per-element body size lives on the imported codec and
    ///     is only available after import enrichment, so the generator
    ///     adds `max_count * imported_codec.max_frame_bytes()` itself
    ///     (mirrors the variant arm body sizing at codegen time).
    ///   - `tlv-chain { max_depth, .. }` (RFC §5.B B3): contributes 0
    ///     here for the same reason — generator adds `max_depth *
    ///     imported_codec.max_frame_bytes()` post-enrichment.
    pub fn max_frame_bytes(&self) -> u32 {
        let var_max: u32 = self
            .fields
            .iter()
            .filter(|f| f.is_variable_length())
            .map(|f| match &f.bit_size {
                BitSize::Vle { width_bits } => width_bits.div_ceil(7),
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

    /// Whether the codec has any repeat field (RFC §5.B B2). Forces
    /// the streaming decode/encode path because a repeat field's wire
    /// length is runtime-determined (count_ref or until-eof) and the
    /// per-element body invokes the imported codec's encode/decode.
    pub fn has_repeat_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_repeat())
    }

    /// Whether the codec has any present-if-gated field (RFC §5.B
    /// B1-δ). Forces the streaming decode/encode path because a
    /// gated field's start offset depends on the runtime predicate
    /// value, and per-language type wraps the field as an optional.
    pub fn has_present_if_fields(&self) -> bool {
        self.fields.iter().any(|f| f.present_if.is_some())
    }

    /// Whether the codec has any TLV chain field (RFC §5.B B3). Forces
    /// the streaming decode/encode path (same machinery as repeat) and
    /// classifies the codec as MCU-class.
    pub fn has_tlv_chain_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_tlv_chain())
    }

    /// Whether the codec has any single-codec embed field (RFC §5.B Y0c).
    /// Forces the streaming decode/encode path (same machinery as
    /// repeat / TLV chain). The streaming codec calls the embedded
    /// codec's decode/encode with optional parent-flag threading.
    pub fn has_embed_fields(&self) -> bool {
        self.fields.iter().any(|f| f.is_embed())
    }

    /// Whether the codec has any UTF-8 string field (RFC §5.B B5-ζ
    /// Surface H). Forces the streaming decode/encode path so the
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
    /// §5.B B3). Forces the encode buffer to be zero-initialised so
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
    /// bytes they actually decoded — used by RFC §5.B B3 to make
    /// length-ref entry codecs decode-iterable inside a TLV chain
    /// (the B1-prep "consume entire cursor" path was the deferred
    /// "first multi-frame consumer" the comment references; B3 is
    /// that consumer).
    pub fn has_tail_fields(&self) -> bool {
        self.fields
            .iter()
            .any(|f| matches!(f.bit_size, BitSize::Tail))
    }

    /// RFC §5.B "MCU-only codec sub-features" — whether the codec
    /// contains any feature that emits only on Rust + C11. Drives the
    /// codec-content classification used by `render_codec` to typed-
    /// reject cpp/kotlin/go/python via the existing kind-class
    /// diagnostic (`codegen/mcu-class-kind-on-non-mcu-language`,
    /// repurposed at codec-content granularity).
    ///
    /// B3-β: DMA alignment — the only sub-feature that genuinely needs
    /// MCU-class hardware (DMA controllers, fixed-offset wire layout
    /// invariants tied to memory-mapped peripherals). TLV chain (B3-α)
    /// was originally bundled here as a conservative scope choice; it
    /// is in fact server-class-relevant too (Zenoh extension envelopes
    /// land on zenoh-rs / zenoh-cpp / zenoh-kotlin server peers, not
    /// just zenoh-pico MCU). B5-ε closures (cpp/kotlin/go/python TLV
    /// chain emit) lifted that gating; only DMA align stays MCU-only.
    pub fn has_mcu_only_features(&self) -> bool {
        self.has_dma_aligned_fields()
    }

    /// Whether the codec declares a `<sce:requires-parent-flags>`
    /// block (RFC §5.B B5-γ). Drives the per-backend signature
    /// extension that adds a `parent_flags: u8` parameter to
    /// decode/encode and the variant arm dispatcher's threading
    /// of the parent's flag-carrier value through the call.
    pub fn has_parent_flags(&self) -> bool {
        self.requires_parent_flags.is_some()
    }
}

// ── Codec test vectors (RFC §5.B B5-θ) ────────────────────────

/// One `<sce:test-vector hex="...">` row on a codec. Captures the
/// wire-byte form (`hex`) and the expected decoded field-value tree
/// (`decoded`) so the per-backend sidecar can render a single
/// round-trip oracle (encode → bytes parity, decode → field parity).
///
/// Trunk shape (`Plain` only) covers codecs whose decoded form is a
/// flat list of named fields. Variant / recursive-variant / TLV-chain
/// expansions reuse the same `CodecTestVector` envelope and only add
/// new arms to `DecodedValue` — keeps the parser + codegen surface
/// uniform across B5-θ closures.
#[derive(Debug, Clone, Serialize)]
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
/// emits `Plain` (flat field list); variant / TLV-chain closures
/// add `Variant` and `Chain` arms following the B1-β / B3-α
/// trunk-then-closures cadence.
#[derive(Debug, Clone, Serialize)]
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
pub struct DecodedField {
    pub name: String,
    pub value: DecodedFieldValue,
}

/// Typed value literal for one `<sce:decoded>` row. Variant chosen
/// at parse time from the matching codec field's `SceType`:
/// integer types → `Uint` / `Int`, `Bool` → `Bool`, `Bytes` → `Bytes`,
/// `String` → `String`. Float fields are not yet supported (no codec
/// field uses them; closures land alongside the first consumer).
#[derive(Debug, Clone, Serialize)]
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
    /// validates UTF-8 invariant per B5-ζ codec contract; the
    /// test-vector value is stored as canonical Rust `String`.
    String(String),
}

// ── Filter kind ───────────────────────────────────────────────

/// Filter type: algorithm used for signal filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
pub struct FilterModel {
    pub name: String,
    pub input: ForgeField,
    pub output: ForgeField,
    pub filter_type: FilterType,
    /// Window size for moving-average and debounce.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<u32>,
    /// Smoothing factor (0..1) for low-pass filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f64>,
}

// ── Interpolation kind ────────────────────────────────────────

/// Interpolation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
#[serde(rename_all = "snake_case")]
pub enum OutOfBounds {
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

impl Default for OutOfBounds {
    fn default() -> Self {
        Self::Clamp
    }
}

/// Axis definition for interpolation (breakpoints for one dimension).
#[derive(Debug, Clone, Serialize)]
pub struct InterpolationAxis {
    /// Input field id this axis corresponds to.
    pub input_id: String,
    /// Breakpoint values (monotonically increasing).
    pub breakpoints: Vec<f64>,
}

/// Interpolation: 1D/2D table lookup with linear/bilinear interpolation.
/// Generates a struct with static tables and a `lookup()` method.
#[derive(Debug, Clone, Serialize)]
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
}

// ── Timer kind ────────────────────────────────────────────────

/// Timer kind shape per watching-zenoh RFC §5.D line 880-886.
///
/// One timer per forge doc — keepalive, retry, watchdog, etc. The
/// timer self-manages its lifecycle through `reset_on` (event-driven
/// restart) and `cancel_on_state_exit` (state-exit-driven cancel),
/// and emits `fire_event` when the period elapses.
///
/// **Pre-C1 legacy shape removed.** Before 2026-05-12, this kind
/// declared multiple `TimerEntry` records under `<datamodel>` with
/// `sce:timer="periodic|timeout|delayed"` + integer-millisecond
/// attributes. The legacy shape did not cover the §5.D semantics
/// (event-driven reset, state-exit cancel, single named timer per
/// doc); C1 migrates SCE to the spec-mandated shape per
/// `feedback_spec_mirror_parity.md` and
/// `feedback_pre_release_no_compat.md` (pre-1.0 rename permitted).
///
/// Codegen contract (RFC §5.D lines 902-906):
/// - AP: deadline tracker on top of generic event queue
///   (e.g. `tokio::time::interval` for Rust, `ScheduledExecutorService`
///   for Kotlin, `time.AfterFunc` for Go, `asyncio.sleep` for Python)
/// - MCU: compile-time slot in a static timer wheel
///
/// Diagnostics (RFC §5.D lines 909-910):
/// - `timer/period-below-tick-rate` — `period_us` declared shorter
///   than `scheduler.tick_period_us`; cooperative scheduler cannot
///   dispatch faster than its tick rate.
/// - `timer/slot-overflow` — total timer doc count for a machine
///   exceeds `scheduler.timer_wheel_depth`; the static wheel cannot
///   hold any more.
#[derive(Debug, Clone, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_on_event: Option<String>,
    /// State id whose exit cancels the timer.
    /// `<sce:cancel-on state-exit="..."/>` (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_on_state_exit: Option<String>,
    /// Event name raised when the timer fires.
    /// `<sce:fire-event>...</sce:fire-event>` (required).
    pub fire_event: String,
}

// ── Observer kind ─────────────────────────────────────────────

/// A single threshold monitor within an observer kind.
#[derive(Debug, Clone, Serialize)]
pub struct ThresholdMonitor {
    pub id: String,
    /// ECMAScript expression for entering the active state.
    pub enter_expr: String,
    /// ECMAScript expression for leaving the active state. Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_expr: Option<String>,
    /// Event name emitted on entering active state.
    pub on_enter: String,
    /// Event name emitted on leaving active state. Optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_leave: Option<String>,
}

/// Observer: threshold monitoring with hysteresis.
/// Generates a struct with per-monitor boolean state and an `update()` method
/// returning a collection of triggered events.
#[derive(Debug, Clone, Serialize)]
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
}

// ── Build manifest ────────────────────────────────────────────

/// A single entry in a forge build manifest.
#[derive(Debug, Clone, Serialize)]
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
pub struct ForgeManifest {
    pub entries: Vec<ManifestEntry>,
    pub build_order: Vec<String>,
}

// ── Inline kind (within statechart) ────────────────────────────

/// Inline kind data — embedded within a statechart `<data>` element.
/// Derived values (unique_values, entries_by_value) are computed by the
/// generator at render time, not stored in the model (state normalization).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum InlineKindData {
    #[serde(rename = "lookup")]
    Lookup {
        input_id: String,
        entries: Vec<LookupEntry>,
        default_value: String,
    },
    #[serde(rename = "condition")]
    Condition {
        expr: String,
    },
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
pub struct InlineKind {
    pub id: String,
    pub data: InlineKindData,
}

// ── Algorithm kind (RFC §5.A) ──────────────────────────────────

/// One parameter of an algorithm signature. Parameters are by-value
/// scalars or by-reference slices for `bytes`. Read-only in v1
/// (assigning to a parameter raises `algorithm/lvalue-unsupported`).
#[derive(Debug, Clone, Serialize)]
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
pub struct AlgorithmSignature {
    pub params: Vec<AlgorithmParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<SceType>,
}

/// Type carried by an `<sce:const>` declaration. Scalar form is the
/// only shape produced by hand-authored algorithm bodies (RFC §5.A);
/// the array form is reserved for `sce:compute-at="build"` consts
/// whose body is an `<sce:fold>` block (RFC §5.F build-time const-fold).
/// Keeping array out of [`SceType`] keeps the parameter / var / field
/// surfaces scalar-only, which is the v1 contract everywhere else.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum AlgorithmConstType {
    /// Scalar form (any `SceType`). Emitted as a language-native const.
    Scalar(SceType),
    /// Array form `array<T, N>` — emitted as a language-native static
    /// array literal once §5.F const-fold lands. Length is the fixed
    /// element count; element type is one of the scalar `SceType`s.
    Array {
        elem: SceType,
        len: u32,
    },
}

impl AlgorithmConstType {
    /// Element type of an array shape, or the scalar's own type. Used
    /// by the host interpreter (Phase A4-β) to coerce yielded values
    /// before serializing the array literal, and by the §5.J.5 emitters
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
    /// spellings; the alias map mirrors RFC §5.F's example which uses
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
/// scalar type spellings. RFC §5.F's worked example uses the short
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

/// Body of an `<sce:fold>` element — RFC §5.F build-time evaluation.
///
/// Iterates `iter_var` over `[range_start, range_end)`, executing
/// `body` against a fresh local scope per iteration and emitting
/// `yield_expr` (typed at `elem_type`) as one element of the produced
/// array.
#[derive(Debug, Clone, Serialize)]
pub struct FoldBody {
    /// Inclusive lower bound of the integer range driving the fold.
    pub range_start: u32,
    /// Exclusive upper bound of the integer range driving the fold.
    pub range_end: u32,
    /// Loop-variable name visible inside the fold body.
    pub iter_var: String,
    /// Element type — must match the array shape's `elem` and the
    /// static type of `yield_expr` (validated by Phase A4-γ
    /// diagnostic `algorithm/const-yield-type-mismatch`).
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

/// Build-time const inside an algorithm body. RFC §5.A v1 admits the
/// scalar literal form (`<sce:const name=... type=... init="..."/>`).
/// The `<sce:const sce:compute-at="build">` form with an `<sce:fold>`
/// body resolves through RFC §5.F build-time const-fold; the IR shape
/// lands in Phase A4-α (parser + model), the host interpreter and
/// per-language emit land in A4-β.
#[derive(Debug, Clone, Serialize)]
pub struct AlgorithmConst {
    pub name: String,
    #[serde(rename = "type")]
    pub sce_type: AlgorithmConstType,
    /// Literal init expression for scalar consts. Lowered at codegen
    /// time via the typed expression pipeline (`forge::expr`). `None`
    /// when the const carries a `<sce:fold>` body instead — the
    /// parser enforces "exactly one of `init` / `fold`".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<String>,
    /// `<sce:fold>` body for `sce:compute-at="build"` consts. `None`
    /// for scalar consts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fold: Option<FoldBody>,
    /// `sce:compute-at="build"` flag (§5.F hook). Required to be
    /// `true` when [`AlgorithmConst::fold`] is `Some`, and required
    /// to be `false` otherwise. The parser enforces this invariant.
    #[serde(skip_serializing_if = "is_false")]
    pub compute_at_build: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// One statement in an algorithm body. Lowered to language-idiomatic
/// constructs by each backend (RFC §5.J.5 emitter table).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "stmt", rename_all = "snake_case")]
pub enum AlgorithmStmt {
    /// `<sce:var name=... type=... init=.../>` — local mutable binding.
    Var {
        name: String,
        #[serde(rename = "type")]
        sce_type: SceType,
        init: String,
    },
    /// `<sce:assign target="lvalue" expr=".../>"` — mutates an existing
    /// l-value. v1 l-values are identifier, member access, or index
    /// (RFC §5.A "LValue scope v1"). Stored as raw string and validated
    /// by the parser.
    Assign { target: String, expr: String },
    /// `<sce:if cond="..."> ... <sce:else>...</sce:else></sce:if>`.
    If {
        cond: String,
        then_body: Vec<AlgorithmStmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
        else_body: Option<Vec<AlgorithmStmt>>,
    },
    /// `<sce:while cond="..." max-iter="N">...</sce:while>` — counted
    /// loop. `max_iter = None` is allowed only when the `cond`
    /// expression bounds itself; on MCU targets unbounded loops fire
    /// `algorithm/while-unbounded` (Phase B once deploy.yaml MCU
    /// detection lands).
    While {
        cond: String,
        body: Vec<AlgorithmStmt>,
        #[serde(skip_serializing_if = "Option::is_none")]
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
        #[serde(skip_serializing_if = "Option::is_none")]
        expr: Option<String>,
    },
    /// `<sce:call target="other_algo" args="a, b"/>` — invoke another
    /// algorithm kind imported via `<sce:import>`. v1 forbids
    /// recursion (`algorithm/call-cycle`).
    Call { target: String, args: Vec<String> },
}

/// Algorithm document — pure synchronous function (RFC §5.A).
#[derive(Debug, Clone, Serialize)]
pub struct AlgorithmModel {
    pub name: String,
    pub signature: AlgorithmSignature,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub consts: Vec<AlgorithmConst>,
    pub body: Vec<AlgorithmStmt>,
    /// RFC §5.B "Test vector": inline `<sce:test-vector hex value/>`
    /// reference oracles. Each entry generates a per-backend round-trip
    /// test that runs the algorithm on `hex` and asserts the return
    /// value equals `value`. v1 supports algorithm kind only with
    /// scalar return — multi-field codec test-vector defers to B5.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub test_vectors: Vec<TestVector>,
}

/// RFC §5.B test-vector value literal. v1 covers the scalar types an
/// algorithm signature can return; child-element form for multi-field
/// codec results defers to B5 alongside the Zenoh msg-set authoring
/// where the consumer signal first lands.
#[derive(Debug, Clone, Serialize, PartialEq)]
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

/// Single `<sce:test-vector hex value/>` row (RFC §5.B). Captures the
/// declared input bytes and expected output literal; the emitter pairs
/// these with the algorithm's signature to render an idiomatic
/// per-backend test function.
#[derive(Debug, Clone, Serialize)]
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

// ── Link kind (RFC §5.C) ───────────────────────────────────────

/// `<sce:link-class>` enumeration — RFC §5.C "Link-class enumeration"
/// table. Five strings shipped today; OS-specific classes (e.g.
/// `unix_socket`, `qnx_msg`) land additively in later phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkClass {
    /// Datagram, byte-stream framer (RFC §5.C row 1; A on MCU lwIP).
    Udp,
    /// Stream, byte-stream framer (RFC §5.C row 2; B on MCU).
    Tcp,
    /// UART (RFC §5.C row 3; C on MCU).
    Serial,
    /// TCP + WebSocket framing (RFC §5.C row 4; C on MCU).
    Websocket,
    /// L2 frames, target-plugin only (RFC §5.C row 5; C MCU plugin).
    RawEth,
}

impl LinkClass {
    /// Every legal `<sce:link-class>` value, in declaration order.
    pub const ALL_NAMES: &'static [&'static str] = &[
        "udp",
        "tcp",
        "serial",
        "websocket",
        "raw_eth",
    ];

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
    /// per RFC §5.C "Link-class enumeration" table (lines 765-771).
    /// Strict-literal reading per RFC §5.C lines 776-782 "additive"
    /// policy: classes are added when wired, not pre-reserved as
    /// namespace placeholder. Anything off the table fires
    /// `link/class-unsupported-on-target` at validate-time.
    ///
    /// Single source of truth for [`forge::validate`]'s η check.
    /// Future OS-specific classes (e.g. `unix_socket`, `qnx_msg`) land
    /// additively as new enum rows alongside their phase opening.
    pub fn admits_os(self, os: crate::mesh::deploy::OsKind) -> bool {
        use crate::mesh::deploy::OsKind;
        match self {
            // RFC §5.C row 1: A (MCU lwIP) | D.1 (AP linux) | D.2 (AP qnx).
            Self::Udp | Self::Tcp => {
                matches!(os, OsKind::BareMetal | OsKind::Linux | OsKind::Qnx)
            }
            // RFC §5.C rows 3-5: C (MCU) only.
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

/// `<sce:backpressure>` policy — RFC §5.C body. Three policies; B6-α
/// surfaces all three at parse time but the rust template lowers them
/// uniformly (the policy threading into the runtime crate is an
/// implementation concern of `sce-link-runtime`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
pub struct LinkInboundEvent {
    /// SCXML event name to inject when the framer decode + `when`
    /// predicate succeed.
    pub event: String,
    /// Optional decode-side predicate (`when="decoded.msg_id == 0x02"`).
    /// `None` means inject every successful decode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
}

/// `<sce:outbound>` — SCXML event → framer encode + driver send contract.
#[derive(Debug, Clone, Serialize)]
pub struct LinkOutboundEvent {
    /// SCXML event name that triggers TX.
    pub event: String,
    /// Encoder codec reference — same `<sce:framer ref>` namespace.
    pub encode: String,
}

/// Link document — RFC §5.C byte-stream link endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct LinkModel {
    pub name: String,
    /// `<sce:link-class>` body text, parsed into the closed enum.
    pub class: LinkClass,
    /// `<sce:framer ref="...">` — the §5.B codec that decodes/encodes
    /// wire bytes. B6-α makes this required; absence raises
    /// `link/framer-missing` (parser).
    pub framer: String,
    /// `<sce:backpressure>` policy. Required in B6-α; absence is the
    /// `link/backpressure-undeclared` diagnostic that B6-γ formalizes.
    /// For B6-α we accept the missing-element case as default `drop`
    /// (parser-side fallback) — the dedicated diagnostic ships with
    /// B6-γ's reject fixture.
    pub backpressure: BackpressurePolicy,
    /// `<sce:events><sce:inbound .../></sce:events>` rows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub inbound: Vec<LinkInboundEvent>,
    /// `<sce:events><sce:outbound .../></sce:events>` rows.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outbound: Vec<LinkOutboundEvent>,
    /// `<sce:rx-pool ref="...">` — RX buffer-pool name (RFC §5.C body
    /// + §5.E B7-α schema-only). Authors who want zero-copy RX path
    /// declare a `<scxml sce:kind="buffer-pool" name="...">` document
    /// and bind it here. B7-α schema-only: parser accepts the element,
    /// emits the ref as a `pub const` on the generated wrapper. Cross-
    /// resolution validator (`link/pool-slot-smaller-than-framer-max`)
    /// defers to a later atomic that wires pool ↔ framer at
    /// `compile_forge_with_imports`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rx_pool: Option<String>,
    /// `<sce:tx-pool ref="...">` — TX buffer-pool counterpart. Same
    /// shape as `rx_pool`. Symmetric in B7-α; size validation defers
    /// alongside the rx-pool case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_pool: Option<String>,
    /// `<sce:stage-pool ref="...">` — stage-copy destination pool.
    /// Single source of truth for `Sample::take()`'s copy target,
    /// per watching-zenoh RFC §5.E (B7-η' Atomic A1: schema locality
    /// belongs on the link kind, not on the deploy.yaml binding —
    /// rx_pool/tx_pool precedent). When a SCXML state declares
    /// `<sce:on-sample link="X">`, the η' validator looks up link X
    /// in the [`super::cross_doc_registry::SceCrossDocRegistry`]; the link's
    /// `stage_pool` field decides whether `take()` is wired (resolves
    /// to a buffer-pool kind whose slots back the owned-copy
    /// destination) or whether `take()` will panic at runtime
    /// (`PanicOnTakeHook` default — Q-η'-5). Absence is legal for
    /// borrow-only callbacks that never call `take()`; presence + a
    /// missing on-sample subscriber is fine too (the field is link-
    /// declared, not on-sample-coupled). The SCXML-side enforcement
    /// raises `pool/sample-take-without-stage-pool` only when an
    /// on-sample subscriber exists for a link without `stage_pool`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_pool: Option<String>,
}

/// `<sce:cache-policy>` enum — RFC §5.E lines 948-957. Three policies
/// determine how codegen positions cache-maintenance ops on FSM edges:
/// `maintain` emits clean/invalidate around DMA boundaries (B7-δ);
/// `non-cacheable` declares MPU non-cacheable region (deploy.yaml
/// `attr: [non_cacheable]`); `none` targets without D-cache.
///
/// B7-α parses the enum + carries it on `BufferPoolModel`. The cache-
/// maintenance pinning that uses this field defers to B7-δ (gated on
/// §5.I `<sce:call>` intrinsic registry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CachePolicy {
    /// Codegen inserts `cache_clean` before DMA TX, `cache_invalidate`
    /// after DMA RX (B7-δ). Pool MUST live in a `cacheable` region.
    Maintain,
    /// Pool MUST live in a `non_cacheable` MPU region; no maintenance
    /// ops emitted. CPU access pays the uncached-load penalty.
    NonCacheable,
    /// Target has no D-cache (e.g. Cortex-M0/M3/M4). No maintenance
    /// ops, no MPU setup. `cache-policy: maintain` on a `has_dcache:
    /// false` target raises `mem/cache-policy-unsupported-on-no-dcache-core`
    /// (B7-δ family).
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

/// Buffer-pool document — RFC §5.E SRAM-placed, DMA-aligned slot table.
///
/// B7-α schema (per `<scxml sce:kind="buffer-pool">` body):
/// - `<sce:slot-count>` (u32, > 0) — number of slots in the pool
/// - `<sce:slot-size>` (u32, > 0) — bytes per slot
/// - `<sce:section>` (string) — SRAM region name (matches deploy.yaml
///   `machines.<m>.memory.sram_regions.<name>`); η-second-consumer
///   validator `mem/pool-section-conflict` fires when the section is
///   declared but absent from the resolved machine's memory map
/// - `<sce:alignment>` (u32, power of 2, > 0) — DMA alignment requirement
/// - `<sce:dma-channel>` (string, optional) — DMA channel binding
///   (matches deploy.yaml `machines.<m>.memory.dma_channels` entry);
///   `mem/dma-channel-collision` deferred to B7-γ
/// - `<sce:cache-policy>` ([`CachePolicy`]) — `maintain` /
///   `non-cacheable` / `none`; cache-maintenance pinning defers to B7-δ
#[derive(Debug, Clone, Serialize)]
pub struct BufferPoolModel {
    pub name: String,
    /// `<sce:slot-count>` — slot count in the pool. Parser rejects 0.
    pub slot_count: u32,
    /// `<sce:slot-size>` — bytes per slot. Parser rejects 0.
    pub slot_size: u32,
    /// `<sce:section>` — SRAM region name. Validated against deploy.yaml
    /// `memory.sram_regions` via [`compile_forge_with_deploy`] (B7-α).
    pub section: String,
    /// `<sce:alignment>` — DMA alignment. Parser rejects 0; power-of-2
    /// check defers to B7-β linker fragment emission.
    pub alignment: u32,
    /// `<sce:dma-channel>` — DMA channel name (optional). Empty when
    /// the pool is purely CPU-managed. Cross-resolution defers to B7-γ.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dma_channel: Option<String>,
    /// `<sce:cache-policy>` — `maintain` / `non-cacheable` / `none`.
    pub cache_policy: CachePolicy,
}

// ── Worker kind ────────────────────────────────────────────────

/// SPSC inbox ordering choice (RFC §5.I lines 1752-1758, C2-β). Drives
/// the atomic operations emitted on head/tail indices in both Rust and
/// C11 codegen. `AcqRel` is the safe default (every push/pop pairs
/// acquire+release on the index); `Relaxed` is the single-core
/// fast-path (no inter-thread synchronization). Cross-core placement +
/// `Relaxed` fires `worker/inbox-ordering-relaxed-across-cores` at
/// codegen time per spec line 1755-1756.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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

/// `<sce:inbox>` configuration — RFC §5.D line 894 + §5.I lines
/// 1752-1758. SPSC ring-buffer inbox shape; per Q-C2-8 lock, the
/// producer/consumer split is the type-level FSM
/// (`heapless::spsc::{Producer,Consumer}` on Rust; opaque
/// `sce_inbox_{producer,consumer}_t` family on C11). No separate FSM
/// IR module — slot lifecycle is 2-state (free/in-use), degenerate
/// compared to BufferPool's 7-state DMA lifecycle.
///
/// C2-α schema: only `depth` attribute (spec verbatim). C2-β adds the
/// `ordering` attribute as required (RFC §5.I lines 1757-1758 spec-
/// verbatim per `feedback_spec_mirror_parity.md`; SCE's error-only
/// wire realizes the spec "warning, codegen defaults to acq/rel" as
/// a required-when-worker-exists error so the author makes an
/// explicit choice before codegen emits ambiguous atomic ops).
/// MPSC variant deferred until consumer signal (RFC §6 tracked).
#[derive(Debug, Clone, Serialize)]
pub struct InboxConfig {
    /// `<sce:inbox depth="N"/>` attribute body — fixed ring-buffer
    /// depth. Parser rejects 0. Spec line 894 verbatim attribute form.
    pub depth: u32,
    /// `<sce:inbox ordering="acq_rel|relaxed"/>` (RFC §5.I lines
    /// 1752-1758). Required at parse time; absence fires
    /// `worker/inbox-ordering-unspecified`. Codegen wires the chosen
    /// memory ordering into the head/tail atomic operations on both
    /// Rust + C11 backends.
    pub ordering: InboxOrdering,
}

/// Worker document — RFC §5.D concurrent execution context driven
/// by a `<sce:link-rx>` source.
///
/// C2-α schema (per `<scxml sce:kind="worker">` body):
/// - `<sce:link-rx ref="...">` (required) — `<scxml sce:kind="link">`
///   document that drives this worker. Cross-resolution validator
///   `worker/link-rx-ref-unknown` defers to C2-β (consumer-co-landed
///   with codegen needing the resolved Link).
/// - `<sce:inbox depth="N"/>` (required) — SPSC ring-buffer inbox.
///   Producer/consumer pair drawn from `heapless::spsc::split()` on
///   Rust; opaque `sce_inbox_{producer,consumer}_t` on C11.
/// - `<sce:outbox ref="...">` (optional) — recipient worker/state-
///   machine inbox for emitted events. Three cross-resolution
///   validators (`worker/outbox-ref-unknown` +
///   `worker/outbox-target-wrong-kind` +
///   `worker/outbox-target-suffix-invalid`) live in
///   [`crate::validate_worker_outbox_references`] (C2 follow-up
///   Atomic B). Parser keeps `outbox` as an opaque
///   `Option<String>`; semantic resolution against statechart +
///   worker recipients happens against the
///   [`crate::forge::cross_doc_registry::SceCrossDocRegistry`] in
///   the orchestrator pass.
/// - `<sce:body>` (optional, usually empty per spec line 897) — SCXML
///   actions. C2-α scans for `worker/shared-mutable-state` violations
///   (any `<sce:import kind="worker">` in the document, plus body
///   SCXML data-refs to foreign namespaces).
///
/// MCU-class kind (RFC §5.J.4): Rust + C11 emitters only. cpp/kotlin/
/// go/python rejection via existing `codegen/mcu-class-kind-on-non-
/// mcu-language` family (lands in C2-β alongside codegen).
#[derive(Debug, Clone, Serialize)]
pub struct WorkerModel {
    pub name: String,
    /// `<sce:link-rx ref="...">` — required driver source. Spec line
    /// 893 phrasing "drives this worker" → required. Empty-ref parse
    /// rejects; cross-ref resolution against `SceCrossDocRegistry`
    /// defers to C2-β.
    pub link_rx: String,
    /// `<sce:inbox depth="N"/>` — required typed event queue.
    pub inbox: InboxConfig,
    /// `<sce:outbox ref="...">` — optional recipient inbox. When
    /// absent, the worker only injects events into the parent state
    /// machine via `<sce:link-rx>`-driven event mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbox: Option<String>,
}

// ── Forge document ─────────────────────────────────────────────

/// Top-level forge document — dispatched by `sce:kind` on `<scxml>` root.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum ForgeDocument {
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
}

impl ForgeDocument {
    pub fn name(&self) -> &str {
        match self {
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
        }
    }

    pub fn kind(&self) -> ForgeKind {
        match self {
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
            Self::Transform(_) | Self::Condition(_) | Self::Codec(_)
            | Self::Validator(_) => RuntimeDep::None,
            Self::Lookup(m) => {
                if m.output_is_string() { RuntimeDep::None } else { RuntimeDep::ForgeRuntime }
            }
            Self::Filter(_) | Self::Interpolation(_)
            | Self::Observer(_) => RuntimeDep::ForgeRuntime,
            Self::Timer(_) => RuntimeDep::ForgeRuntimeHal,
            Self::Procedure(m) => {
                if m.is_l2() { RuntimeDep::ForgeRuntime } else { RuntimeDep::None }
            }
            // RFC §5.A: free function over language-native loops/locals.
            Self::Algorithm(_) => RuntimeDep::None,
            // RFC §5.C: trait surface owned by SCE's `sce-link-runtime`;
            // per-OS impls live downstream. SCE-side tier `None`.
            Self::Link(_) => RuntimeDep::None,
            // RFC §5.E: B7-α self-contained slot table on `(rust, std)`
            // — no SCE-side runtime helper crate. Cache maintenance ops
            // (B7-δ) route through §5.I intrinsics; B7-α tier `None`.
            Self::BufferPool(_) => RuntimeDep::None,
            // RFC §5.D: heapless::spsc on Rust + bare ring-buffer on C11
            // with §5.I atomic intrinsics. No SCE-side runtime helper.
            Self::Worker(_) => RuntimeDep::None,
        }
    }
}

/// Parsed forge result — combines the document model with cross-file imports.
/// The `imports` field is empty for standalone documents (no `<sce:import>` elements).
#[derive(Debug, Clone, Serialize)]
pub struct ParsedForge {
    pub document: ForgeDocument,
    pub imports: Vec<ForgeImport>,
    /// `<sce:extern>` declarations parsed from the document root
    /// (watching-zenoh RFC §5.I, Atomic A). Empty for documents that
    /// do not declare any externs. Each entry has already been
    /// validated against the §5.I baseline registry — wire-format
    /// rejection (4-code family `extern/symbol-not-in-whitelist` /
    /// `extern/abi-mismatch` / `extern/signature-mismatch` /
    /// `extern/ordering-unspecified`) raises during parsing, so a
    /// `ParsedForge` carrying any externs is guaranteed registry-clean.
    /// Atomic A produces this list; downstream codegen consumption
    /// (per-language `extern "..." {}` emission) lands with a future
    /// atomic gated on a published `sce_intrinsics_runtime` crate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extern_declarations: Vec<ExternDeclaration>,
}

/// One `<sce:extern>` declaration after parse-time validation has
/// matched it against the §5.I baseline registry. Carried through
/// `ParsedForge` so future codegen atomics can emit per-language
/// `extern "C" { fn ... }` blocks without re-walking the XML.
#[derive(Debug, Clone, Serialize)]
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
