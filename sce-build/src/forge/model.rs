// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
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
    /// Sequential procedure with branching (Phase 2).
    Procedure,
    /// Range/plausibility/rate-of-change validation (Phase 2).
    Validator,
    /// Signal filtering: moving average, low-pass, debounce (Phase 3).
    Filter,
    /// 1D/2D table interpolation (Phase 3).
    Interpolation,
    /// Periodic/delayed task timing (Phase 3).
    Timer,
    /// Threshold monitoring with hysteresis (Phase 3).
    Observer,
}

impl ForgeKind {
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
            _ => None,
        }
    }

    /// Whether this kind can appear inline within a statechart `<data>` element.
    /// Only stateless kinds are inline-eligible.
    pub fn is_inline_eligible(&self) -> bool {
        matches!(
            self,
            Self::Transform | Self::Lookup | Self::Condition | Self::Codec
        )
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

/// Lookup: discrete value mapping. Generates switch/match + enum.
#[derive(Debug, Clone, Serialize)]
pub struct LookupModel {
    pub name: String,
    pub input: ForgeField,
    pub output: ForgeField,
    pub entries: Vec<LookupEntry>,
    /// Fallback value when no entry matches. If empty, uses first entry's value.
    pub default_value: String,
}

impl LookupModel {
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
        matches!(self.bit_size, BitSize::Tail | BitSize::LengthRef)
    }

    /// Fixed bit count, or None for variable-length.
    pub fn fixed_bits(&self) -> Option<u32> {
        match &self.bit_size {
            BitSize::Fixed { bits } => Some(*bits),
            _ => None,
        }
    }
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
}

impl ForgeDocument {
    pub fn name(&self) -> &str {
        match self {
            Self::Transform(m) => &m.name,
            Self::Lookup(m) => &m.name,
            Self::Condition(m) => &m.name,
            Self::Codec(m) => &m.name,
            Self::Validator(m) => &m.name,
        }
    }

    pub fn kind(&self) -> ForgeKind {
        match self {
            Self::Transform(_) => ForgeKind::Transform,
            Self::Lookup(_) => ForgeKind::Lookup,
            Self::Condition(_) => ForgeKind::Condition,
            Self::Codec(_) => ForgeKind::Codec,
            Self::Validator(_) => ForgeKind::Validator,
        }
    }
}
