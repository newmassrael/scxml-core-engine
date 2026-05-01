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
}

impl CodecField {
    /// Effective endianness (field override or document default).
    pub fn effective_endian(&self, default: Endian) -> Endian {
        self.endian.unwrap_or(default)
    }

    /// Whether this is a variable-length field.
    pub fn is_variable_length(&self) -> bool {
        matches!(self.bit_size, BitSize::Tail | BitSize::LengthRef | BitSize::Vle { .. })
    }

    /// Whether this field uses VLE encoding (consumes a streaming
    /// 1..=ceil(N/7) bytes from the cursor instead of a fixed window).
    pub fn is_vle(&self) -> bool {
        matches!(self.bit_size, BitSize::Vle { .. })
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

/// Discriminated-union suffix on a codec — RFC §5.B Codec DSL.
///
/// Decode reads the named tag field, then dispatches into the matching
/// arm's body codec. Encode writes the tag bytes followed by the active
/// arm's body bytes. The optional `<sce:default>` arm catches any tag
/// value not enumerated; absent default + non-exhaustive arm coverage
/// fires `codec/variant-arm-unreachable` at build time (see RFC §5.B).
#[derive(Debug, Clone, Serialize)]
pub struct CodecVariant {
    /// `id` of the field (within this codec's `fields`) whose decoded
    /// value selects an arm. Must reference an unsigned-int field
    /// (uint8/uint16/uint32/uint64) — enforced at parse time.
    pub tag_field: String,
    /// Enumerated arms in document order.
    pub arms: Vec<VariantArm>,
    /// Catch-all arm for tag values outside the enumerated set.
    /// `None` ⇒ build-time `codec/variant-arm-unreachable` when the
    /// tag domain isn't fully covered by `arms`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_arm: Option<VariantArm>,
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
    pub fn max_frame_bytes(&self) -> u32 {
        let var_max: u32 = self
            .fields
            .iter()
            .filter(|f| f.is_variable_length())
            .map(|f| match &f.bit_size {
                BitSize::Vle { width_bits } => width_bits.div_ceil(7),
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

/// Timer scheduling type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerType {
    Periodic,
    Timeout,
    Delayed,
}

impl TimerType {
    pub fn from_attr(s: &str) -> Option<Self> {
        match s {
            "periodic" => Some(Self::Periodic),
            "timeout" => Some(Self::Timeout),
            "delayed" => Some(Self::Delayed),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::Timeout => "timeout",
            Self::Delayed => "delayed",
        }
    }
}

/// A single timer definition within a timer kind.
#[derive(Debug, Clone, Serialize)]
pub struct TimerEntry {
    pub id: String,
    pub timer_type: TimerType,
    /// Time in milliseconds (interval for periodic, duration for timeout, delay for delayed).
    pub time_ms: u32,
    /// Event name to emit when timer fires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Callback name for timeout expiry (alternative to event).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_timeout: Option<String>,
}

/// Timer: periodic/delayed/timeout task scheduling.
/// Generates a struct with timer members and start/cancel methods.
#[derive(Debug, Clone, Serialize)]
pub struct TimerModel {
    pub name: String,
    pub timers: Vec<TimerEntry>,
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
        }
    }
}

/// Parsed forge result — combines the document model with cross-file imports.
/// The `imports` field is empty for standalone documents (no `<sce:import>` elements).
#[derive(Debug, Clone, Serialize)]
pub struct ParsedForge {
    pub document: ForgeDocument,
    pub imports: Vec<ForgeImport>,
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
