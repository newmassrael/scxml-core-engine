// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language conformance fixture catalog.
//
// Deserializes tests/forge/conformance/fixtures.json into a typed model the
// `sce-codegen generate-conformance` subcommand renders into per-language
// test harnesses via Jinja2 templates. The manifest is the single source of
// truth for fixture structural metadata (kind, arg types, output shape);
// expected oracle values live separately in numerical_reference.json.
//
// Adding a new fixture kind requires extending `FixtureSpec` here *and* the
// corresponding `{{ kind }}` branch in every per-language harness template.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::forge::codec_failure::CodecFailure;
use crate::forge::encode_failure::EncodeFailure;
use crate::forge::model::RuntimeDep;
use crate::generator::Language;

/// Repository-relative location of the numerical fixture catalog.
///
/// The sibling registry (`w3c_registry::W3C_REGISTRY_RELATIVE_PATH`)
/// states its own path the same way, and for the same reason: the CLI
/// help that tells a caller where a catalog lives is formatted from the
/// constant, so the documented path and the read path cannot become two
/// different answers.
pub const FORGE_CATALOG_RELATIVE_PATH: &str = "tests/forge/conformance/fixtures.json";

/// Canonical primitive types used across the manifest. Each per-language
/// template maps these to its native type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CanonicalType {
    Bool,
    I32,
    /// Signed 64-bit — day counts, epoch milliseconds and any other quantity
    /// past the 32-bit range. Without it the harness could not observe a
    /// lowering that narrows a 64-bit operand to 32 bits
    /// (`transform_mixed_width_add`, `transform_mixed_width_or`).
    I64,
    U8,
    U16,
    U32,
    /// Used by codec fields declared `sce:type="uint64" sce:bit-size="vle"`
    /// (RFC §synth-5-B App. B canonical Zenoh ZInt). Round-trip oracle vectors
    /// store the decoded value as a JSON integer literal up to u64::MAX.
    U64,
    F32,
    F64,
    String,
    /// Variable-length byte sequence. Codec fields with
    /// `sce:bit-size="tail"` or `"length-ref"` surface as
    /// `bytes` in the manifest. Round-trip oracle cases carry
    /// the decoded payload as a JSON array of integers.
    Bytes,
}

/// Floating-point comparison vs exact equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum CompareMode {
    Tolerance,
    Equality,
}

/// `kind = "lookup"` miss-handling policy. Mirrors `forge::model::MissPolicy`
/// but stays in the conformance manifest namespace so the harness manifest
/// parser does not have to depend on the SCE Forge model types.
///
/// `Error`: the generated function returns the language's optional type;
/// fixtures must hit a key on every input (happy-path only) and the harness
/// fragment unwraps with `.expect()` / `.value()` / `?: error()`.
///
/// `Default`: the generated function returns the raw value with a fallback;
/// fixtures may include miss inputs that test the fallback path. The
/// reference oracle simply lists the expected fallback value for those cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum LookupMissPolicy {
    Error,
    Default,
}

/// Scalar output descriptor for fixtures whose `expected` JSON value is a
/// single primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct ScalarOutput {
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
    /// Only used by `kind = "transform"` with a single output function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
}

/// List output descriptor for an algorithm that returns `list<T>`
/// (SCE_FORGE.md §4.12): `expected` is a JSON array, compared element by
/// element and by length. `list_of` names the element's canonical type;
/// `compare` applies to each element.
///
/// ⚠ Cases stay within the declared capacity. Past it the backends differ
/// by contract (Rust/C11 fail, the heap backends grow), so an overflowing
/// case would measure that difference rather than the algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ListOutput {
    pub list_of: ListElemArg,
    pub compare: CompareMode,
}

/// The element of a list argument or output: a canonical scalar (`"i64"`)
/// or a record typed by an event-schema fixture (`{"record": "<fixture>"}`),
/// whose elements the oracle writes as JSON objects keyed by the schema's
/// field ids. Untagged, as [`AlgorithmArg`] is, so a scalar list keeps its
/// bare-string spelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum ListElemArg {
    Scalar(CanonicalType),
    Record(RecordRef),
}

impl ListElemArg {
    /// The record the element is, or `None` for a scalar element.
    pub fn record(&self) -> Option<&RecordRef> {
        match self {
            Self::Record(r) => Some(r),
            Self::Scalar(_) => None,
        }
    }

    fn record_mut(&mut self) -> Option<&mut RecordRef> {
        match self {
            Self::Record(r) => Some(r),
            Self::Scalar(_) => None,
        }
    }
}

/// What an algorithm fixture's single call returns: one scalar, a
/// `list<T>`, a record, or a scalar that may be absent. Untagged so a scalar
/// output keeps its `{type, compare}` spelling; the list form is told apart by
/// `list_of`, the record form by `record`, the optional form by `optional`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum AlgorithmOutput {
    List(ListOutput),
    Record(RecordRef),
    Optional(OptionalOutput),
    Scalar(ScalarOutput),
}

/// An `optional<T>` return (docs/adr/0055): `expected` is a value of `optional`
/// or `null` where the algorithm answers none, and each fragment compares the
/// two in its language's optional type, so an absent answer equals only an
/// absent one.
///
/// ⚠ The element is a fixed-width number or a `bool`, compared for equality:
/// what every backend holds an optional value of in an algorithm's slot, and
/// what [`Manifest::validate`] admits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OptionalOutput {
    pub optional: CanonicalType,
    pub compare: CompareMode,
}

/// One argument of an algorithm fixture: a canonical scalar (`"i64"`), a
/// record typed by an event-schema fixture (`{"record": "<fixture>"}`), a
/// read-only `list<T>` (`{"list_of": "i64"}`) — SCE_FORGE.md §4.12 — a value
/// of an enum (`{"enum": "<enum fixture>"}`), a scalar that may be absent
/// (`{"optional": "i32"}`, docs/adr/0055), or a value of a union
/// (`{"union": "<union fixture>"}`, docs/adr/0092). Untagged so a scalar
/// argument keeps its bare-string spelling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum AlgorithmArg {
    Scalar(CanonicalType),
    Record(RecordRef),
    List(ListArg),
    Enum(EnumArg),
    Optional(OptionalArg),
    Union(UnionArg),
}

/// A value of an enum as an argument: the oracle writes it as the name its
/// variant declares, and each fragment builds the enum fixture's own type from
/// that name, as it builds a record's enum field.
///
/// ⚠ `variants` is DERIVED, never written: it is read at harness-render time
/// from the enum fixture's document, with each variant's member as this
/// language's declaration emits it, so the harness cannot name a member the
/// generator did not write. A manifest that carries it is refused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EnumArg {
    /// The enum fixture of this manifest that generates the type.
    #[serde(rename = "enum")]
    pub enum_document: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<RecordEnumVariant>,
}

/// An `optional<T>` argument (docs/adr/0055): the oracle writes a value of
/// `optional`, or `null` for an absent one, and each fragment hands the
/// algorithm its language's optional type over that value. The element is a
/// fixed-width number or a `bool` ([`Manifest::validate`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OptionalArg {
    pub optional: CanonicalType,
}

/// A value of a union as an argument (docs/adr/0092): the oracle writes it as
/// its wire object, the tag beside the fields of its case (section 2), and each
/// fragment builds the union fixture's own type from it, case by case.
///
/// ⚠ `shape` is DERIVED, never written: it is read at harness-render time from
/// the union fixture's document, so the harness cannot name a case the unit
/// does not declare. A manifest that carries it is refused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct UnionArg {
    /// The union fixture of this manifest that generates the type.
    #[serde(rename = "union")]
    pub union_document: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<UnionShape>,
}

/// A union as the rendering language spells it (docs/adr/0092, sections 1.1
/// and 7), read from its document with the parse the generator reads: what a
/// fragment builds a value of the union from, and reads its tag by.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct UnionShape {
    /// The union fixture, whose module, package or header holds the type.
    pub document: String,
    /// The type the union's unit declares: its document's name in Pascal case,
    /// qualified by the fragment as any type of the fixture's unit is, and on
    /// C11 the typedef the unit's header declares.
    pub type_name: String,
    /// The key the tag is written under on the wire.
    pub tag: String,
    /// The member every value of the unit answers its tag by.
    pub tag_member: String,
    /// The enum fixture of this manifest the tag is a value of.
    pub tag_enum: String,
    /// The member a C11 value holds the record of its case in; no other
    /// language names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cases_member: Option<String>,
    /// One case per variant of the tag's enum, in the enum's declaration order.
    pub cases: Vec<UnionCaseSpec>,
}

/// One case of a union, as the rendering language spells it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct UnionCaseSpec {
    /// The name the tag's enum document declares for the case's variant, which
    /// the oracle writes the tag as.
    pub variant: String,
    /// The case's identifier as the union's unit declares it.
    pub ident: String,
    /// The member of the tag's enum the case is for, as this language's
    /// declaration emits it (`forge::enum_naming::variant_ident`).
    pub kind: String,
    /// The record the case holds, its fields derived as a record argument's
    /// are; `None` for an empty case, the tag alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<RecordRef>,
}

/// A `list<T>` argument: the oracle writes it as a JSON array of `list_of`
/// values, and each fragment builds the backend's list from it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ListArg {
    pub list_of: ListElemArg,
}

/// A record argument or output: the event-schema fixture whose payload
/// struct types it. The oracle writes the value as a JSON object keyed by
/// the schema's field ids, and each fragment builds or reads the struct
/// field by field.
///
/// ⚠ `fields` is DERIVED, never written: it is read at harness-render time
/// from the event-schema document itself (`<name>.scxml`), the one place
/// the fields are declared, so the harness cannot disagree with the struct
/// the generator emits. A manifest that carries it is refused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordRef {
    pub record: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<RecordFieldSpec>,
}

/// One field of a record, as the rendering language spells it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct RecordFieldSpec {
    /// The schema's field id — the oracle's JSON key.
    pub name: String,
    /// The field's identifier on the payload struct in this language
    /// (`forge::generator::event_schema_field_ident`).
    pub ident: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    /// The `sce:max-size` the schema declares on a `string` field — the number
    /// of UTF-8 bytes its text is held within (SCE_FORGE.md §4.12). `None` for
    /// every other field. Derived with `fields`, never written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u32>,
    /// The enum document a field declared `enum:<alias>` holds a value of: the
    /// name of the enum fixture of this manifest that generates its type. `None`
    /// for every other field. Derived with `fields`, never written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_document: Option<String>,
    /// The variants of that enum, each by the name its document declares, which
    /// the oracle writes the value as, and the member this language's declaration
    /// emits for it. Empty for every other field. Derived with `fields`, never
    /// written.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<RecordEnumVariant>,
}

/// One variant of the enum a record's field or an argument holds, as a fragment
/// builds the value from the name the oracle writes and compares it with one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct RecordEnumVariant {
    /// The name the enum document declares for the variant.
    pub name: String,
    /// The member this language's declaration emits for it
    /// (`forge::enum_naming::variant_ident`).
    pub ident: String,
}

/// Compound output descriptor for fixtures whose `expected` JSON value is an
/// object with multiple named fields, each computed by a distinct free function.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct CompoundOutput {
    pub key: String,
    pub function: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
    /// Symbol-name SSOT: the field this output has in a transform holder's
    /// outputs record, derived at harness-rendering time from
    /// `forge_transform_output_field` — and only for a transform that has a
    /// holder. Never present in fixtures.json.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// Canonical deterministic implementations that the conformance harness can
/// bake into a per-fixture helper stub closure at codegen time. Each variant
/// declares just enough data for each per-language template to render the
/// same byte-level transform — every fixture that declares a `<sce:helper>`
/// DI point pairs it with one of these, so the oracle
/// `dispatched_payload_bytes` can be pre-computed from the stub arithmetic
/// without the harness needing to talk to the generated state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StubKind {
    /// `|bytes| -> bytes` that XORs every byte with `mask`. Models a
    /// simplified OEM-specific UDS Security Access key derivation.
    XorBytes { mask: u8 },
}

/// Derived helper stub descriptor used inside the clone-and-mutate harness-
/// render path. Carries the helper name (sourced from the SCXML) and its
/// stub implementation (sourced from the fixtures.json dict); templates
/// iterate this vector in SCXML declaration order so positional `execute()`
/// args line up with the generated wrapper's parameter list.
///
/// Derives `Deserialize` so the containing `FixtureSpec::Procedure` struct
/// parses cleanly, but [`Manifest::validate`] rejects any user-supplied
/// `helpers_ordered` value — the SCXML is the single source of truth for
/// helper names and order; fixtures.json only supplies the stub dict keyed
/// by name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct HelperStub {
    pub name: String,
    pub stub: StubKind,
}

/// Struct-return output descriptor for fixtures whose `expected` JSON value is
/// an object matching the layout of a generated return struct — a single call
/// returns multiple fields (e.g. `ValidationResult { valid, reason }`). This
/// differs from [`CompoundOutput`] in that every field comes from the *same*
/// call site rather than a distinct free function, so the fragment emits one
/// `validate(...)` / `decode(...)` and then iterates the fields to generate
/// per-field assertions. The descriptor is intentionally kept minimal — name,
/// type, compare mode — because the forge product code generator is the
/// authority on the struct layout; this manifest entry exists only so the
/// conformance fragments don't have to hard-code field names in five places.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct StructField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: CanonicalType,
    pub compare: CompareMode,
    /// The field may be absent — an entry of an `sce:encoding="cbor"` codec
    /// typed `optional<T>` (SCE_FORGE.md §4.6.1), which each language carries
    /// as its optional type. A round-trip case writes an absent value as JSON
    /// `null`. Derived from the document at render time, like
    /// `has_test_vectors`; fixtures.json never states it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// The field is a `bytes` or `string` whose type holds its bound — a
    /// `length-ref` or `tail` field of a binary codec, or a CBOR entry with a
    /// `sce:max-size` or `sce:length` — which the Rust codec holds as a `Bytes`
    /// or `Text` (docs/adr/0077 §3), so a fragment builds it through the type's
    /// door. Derived from the document, like `optional`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bounded: bool,
    /// The field is an element the encode of a positional codec derives whole
    /// — a length of the bytes it measures — which a value printed from its
    /// plan does not present (docs/adr/0081 §3), so a fragment neither makes
    /// the value with it nor reads it back. Derived from the document, like
    /// `optional`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub derived: bool,
    /// The field is a list — a property of an `sce:encoding="content-line"`
    /// codec that declares `sce:max-count` (a list of lines) or
    /// `sce:separator` (a list of the values of one line), which each language
    /// carries as its bounded list (SCE_FORGE.md §4.6.4). A round-trip case
    /// writes it as a JSON array, and an empty one is an absent property.
    /// Derived from the document, like `optional`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub list: bool,
    /// The name of the enum document this field reads by the text of its
    /// variants — an entry or a parameter of an `sce:encoding="content-line"`
    /// codec typed `enum:<alias>` (docs/adr/0015). A round-trip case writes the
    /// value as the carrier number the enum document gives the variant, and a
    /// language's fragment takes it to and from its own enum type through the
    /// enum's conversions; this names the type that conversion belongs to.
    /// Derived from the document, like `optional`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_document: Option<String>,
    /// The enum this field reads by text is open (`sce:strict-variants="false"`),
    /// so its conversion from a carrier is total; a closed enum's answers whether
    /// the carrier is a variant, which a language that spells that as an optional
    /// value must be told apart from. Derived from the document, with
    /// [`Self::enum_document`].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub enum_open: bool,
    /// The members of one record of a list of line records — a repeated
    /// property that declares a parameter or a separator (docs/adr/0014),
    /// which each language carries as a record type. The parameters come in
    /// declaration order, optional when typed `optional<T>`, and then the line's
    /// value as `value`, or as the list `values` when the entry has a
    /// separator. A round-trip case writes each record as a JSON object keyed
    /// by these names. Empty for every field that is not a list of records.
    /// Derived from the document, like `optional`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<StructField>,
}

/// One `<sce:variant name="..." value="..."/>` of an `sce:kind="enum"`.
///
/// `name` is the SOURCE spelling — the document's — and not any language's
/// rendering of it. That is what makes one manifest serve six arms whose
/// identifier conventions disagree; each arm's identifier is added at render
/// time from the generator's own spelling rule, never typed here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EnumVariantFixture {
    pub name: String,
    /// The declared number. `i64` rather than the underlying carrier type
    /// because the manifest describes vocabularies over several widths and
    /// the fragments compare against the emitted constant, which the
    /// language has already typed.
    pub value: i64,
}

/// One payload field of an `sce:kind="event-schema"`.
///
/// ⚠ WHAT THIS FIXTURE CAN AND CANNOT ASSERT. An event schema emits a payload
/// STRUCT and nothing else — no computation, and in C++/Rust no event-name
/// constant either. So the cross-language claim is about SHAPE: the six arms
/// agree on the field names, their order, and that a value written to a field
/// reads back unchanged.
///
/// ⚠⚠ IT IS DELIBERATELY NOT A WIDTH CLAIM. The same `uint8` field lands as
/// `uint8_t` in C++, `u8` in Rust and a plain `int` in Python's dataclass, so
/// a boundary value would wrap in three arms and not in the others — the
/// fixture would fail for a difference the kind does not promise to remove.
/// `value` is therefore chosen in range for EVERY arm, which keeps the
/// round-trip a statement about the schema rather than about integer models.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct EventSchemaFieldFixture {
    /// The SOURCE spelling, as with enum variants and codec fields: one
    /// manifest serves six arms whose identifier conventions disagree, and
    /// each fragment applies its own case filter.
    pub name: String,
    /// A value in range for every arm's rendering of the declared type — a
    /// number, or a `bool` for a field declared `bool` (SCE_FORGE.md §4.12
    /// admits `bool` fields; the kind of value must match `sce_type`, which
    /// the manifest check holds it to).
    pub value: EventSchemaFieldValue,
    /// The `sce:type` the document declares, verbatim (`uint16`, `int32`, …).
    ///
    /// ⚠ Carried for CONSTRUCTION, not as a width claim. Kotlin needs
    /// `(200).toUByte()` where Python needs `200`, so a typed arm cannot
    /// build the payload without knowing what the field was declared as. The
    /// assertion still compares values, not widths — see the type's header.
    pub sce_type: String,
}

/// The value an event-schema field fixture writes and reads back. Untagged,
/// so the manifest keeps writing a bare JSON number, `true`/`false` or text,
/// and a template renders it as the literal it is — each arm branches on
/// `sce_type == "bool"` and `sce_type == "string"` where its language spells
/// one differently. A text is the value of a `string` field the schema bounds
/// with `sce:max-size` (SCE_FORGE.md §4.12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum EventSchemaFieldValue {
    Int(i64),
    Bool(bool),
    Text(String),
}

/// One fixture entry. `name` and `ref_section` are common to every kind;
/// kind-specific data lives in the `spec` tagged enum below. The
/// `#[serde(flatten)]` on `spec` means the on-disk JSON stays a single flat
/// object (`{name, ref_section, kind, args, output, ...}`) so per-language
/// Jinja2 templates can continue to read `f.args` / `f.output` / etc. without
/// having to walk a nested `f.spec.*` path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Fixture {
    pub name: String,
    pub ref_section: String,
    /// When false, the fixture is included in the per-language harness for
    /// **compile-verification** but no oracle round-trip assertions are
    /// emitted. The fixture's generated golden is still pulled into the
    /// harness translation unit (so the codegen pipeline catches a
    /// silently-broken `kotlinc` / `go build` / `cargo build` regression of
    /// the kind that motivated commit `91ae355a` — Kotlin embed default
    /// overwriting `<EmbedType>()` with `byteArrayOf()`, Go pointer/value
    /// mismatch in struct literals), and a minimal type-resolution probe
    /// (`Default::default()` instantiation in each language's idiom) is
    /// emitted so the `pub` API of the generated module reaches the type
    /// checker. This closes the systemic compile-check gap behind the
    /// historical "byte-golden-only convention" without requiring the
    /// `StructField` / `CanonicalType` schema to grow nested-codec or
    /// nullable variants — both extensions are deferred to a separate RFC
    /// per-fixture upgrade path.
    ///
    /// Default true so existing manifest entries keep the historical
    /// oracle-driven behaviour without churn.
    #[serde(default = "default_oracle_eligible")]
    pub oracle_eligible: bool,
    /// The document this fixture exercises, when it is not
    /// `<name>.scxml` in the resources directory — `sce:std/...` names a
    /// document of SCE's standard library, which a consumer imports by that
    /// name and the harness generates by it too, so the library is held to
    /// the same oracle as every fixture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    #[serde(flatten)]
    pub spec: FixtureSpec,
}

fn default_oracle_eligible() -> bool {
    true
}

impl Fixture {
    /// Where this fixture's document is: its `document`, resolved as an
    /// import is, or `<name>.scxml` in `resource_dir`.
    ///
    /// Every reader of a fixture's document asks here — the harness render,
    /// the per-language filter, and `sce-codegen list-fixtures`, which hands
    /// the answer to each language's build — so no build names the path
    /// itself.
    pub fn document_path(&self, resource_dir: &Path) -> PathBuf {
        match &self.document {
            Some(document) => crate::forge::stdlib::resolve(resource_dir, document),
            None => resource_dir.join(format!("{}.scxml", self.name)),
        }
    }
}

/// Whether the document at `path` — a file, or an `sce:std/...` name — is
/// there to read.
pub fn document_exists(path: &Path) -> bool {
    if crate::forge::stdlib::names_standard(path) {
        crate::forge::stdlib::lookup(path).is_some()
    } else {
        path.exists()
    }
}

/// The text of the document at `path`: a file, or a standard document read
/// from the library the generator embeds.
pub fn read_document(path: &Path) -> std::io::Result<String> {
    if crate::forge::stdlib::names_standard(path) {
        crate::forge::stdlib::lookup(path)
            .map(str::to_string)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no such document in the SCE standard library built into this sce-codegen",
                )
            })
    } else {
        std::fs::read_to_string(path)
    }
}

/// Kind-specific fixture data. The `#[serde(tag = "kind")]` attribute makes
/// each variant discriminate on a top-level `"kind"` string, which matches
/// what fixtures.json already stores and what the templates already read.
///
/// The Rust type system now enforces "this kind requires these fields", so
/// `Manifest::validate` no longer has to hand-check field presence for every
/// kind — it only checks cross-field semantic invariants that cannot be
/// expressed in the types (e.g. Transform needs at least one of the two
/// output shapes; Lookup's `function` is derived at runtime so users must
/// not supply it in the manifest).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum FixtureSpec {
    Interpolation {
        args: Vec<CanonicalType>,
        output: ScalarOutput,
        /// Symbol-name SSOT: the per-language accessor symbol
        /// render_interpolation defines, derived at harness-rendering time
        /// from `forge_interpolation_symbol` (the conformance call site reads
        /// it via `f.symbol`). Like Lookup's `function`, it is never present
        /// in fixtures.json — the manifest carries only structural data.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        symbol: Option<String>,
    },
    Transform {
        args: Vec<CanonicalType>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<ScalarOutput>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        compound_outputs: Vec<CompoundOutput>,
        /// Derived at harness-rendering time from the SCXML: present iff
        /// the document reads a field through `previous()`. It is asked of
        /// `forge::generator::transform_holder` — the call the generator
        /// itself makes to decide whether to emit a holder — so the harness
        /// cannot answer it differently from the product. Never present in
        /// fixtures.json.
        ///
        /// A fixture with a holder is driven through it: `args` are the
        /// inputs `update` takes, and every output is compared from the
        /// record one activation returns, by its `compound_outputs` key.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        holder: Option<crate::forge::generator::TransformHolderSymbols>,
    },
    Condition {
        function: String,
        args: Vec<CanonicalType>,
        output: ScalarOutput,
    },
    Filter {
        input: CanonicalType,
        output: ScalarOutput,
    },
    Observer {
        input: CanonicalType,
        event_tags: Vec<String>,
    },
    Procedure {
        args: Vec<CanonicalType>,
        /// Raw helper stub declarations from fixtures.json — a name-keyed
        /// dict (`{"computeKey": {"kind": "xor_bytes", "mask": 165}}`) whose
        /// keys must match `<sce:helper name="...">` declarations in the
        /// fixture's SCXML. Keying the dict by name rather than storing an
        /// ordered list makes drift between fixtures.json and SCXML
        /// structurally impossible: a mismatch fails loudly at
        /// [`render_harness`] time rather than drifting silently into
        /// positional argument swaps.
        #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
        helpers: std::collections::BTreeMap<String, StubKind>,
        /// Derived at harness-rendering time: the SCXML's helper declarations
        /// in source order, each paired with its stub implementation looked
        /// up from `helpers` by name. Templates iterate this ordered vector
        /// so positional `execute()` arguments match the generator's emitted
        /// wrapper parameter list. Must be absent in the manifest JSON —
        /// `Manifest::validate` rejects any user-supplied value.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        helpers_ordered: Vec<HelperStub>,
        /// Derived at harness-rendering time from the SCXML — true iff the
        /// procedure exercises any of the four L2 markers: `<sce:helper>`,
        /// `<data sce:direction="internal">`, `<onentry><send>`, or
        /// `<donedata>` on a `<final>` state. Mirrors `ProcedureModel::is_l2()`
        /// (which operates on the parsed model) but uses a lightweight
        /// roxmltree scan so the predicate is cheap to consult at fixture-
        /// filtering time. Must be absent in the manifest JSON — the user
        /// cannot override what the SCXML structurally declares.
        ///
        /// Consumed at harness-render time: the per-language procedure
        /// fragments branch on it where the L1 emit shape diverges from
        /// L2 (e.g. `kinds/procedure.c.jinja2`'s `{% if f.is_l2 %}`).
        /// Every procedure shape (L1/L2, with or without imports) is
        /// admitted by every backend — the field selects emit shape, it
        /// does not gate admission.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        is_l2: bool,
    },
    Lookup {
        args: Vec<CanonicalType>,
        output: ScalarOutput,
        on_miss: LookupMissPolicy,
        /// Derived at harness-rendering time from the SCXML
        /// `<data sce:direction="out">` id. Must be absent in the manifest
        /// JSON — `Manifest::validate` rejects any user-supplied value so
        /// the SCXML stays the single source of truth.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        function: Option<String>,
        /// Derived at harness-rendering time: the raw SCXML output field id
        /// (e.g. `"status"`). Used by harness scaffolds to reference the
        /// generated enum type (e.g. `Status` in PascalCase). Must be absent
        /// in the manifest JSON.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_id: Option<String>,
        /// Derived at harness-rendering time for string-output lookups:
        /// the unique enum variant names in declaration order. The harness
        /// scaffold generates an enum-to-string conversion helper from
        /// this list. Empty for numeric-output lookups.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        enum_values: Vec<String>,
    },
    /// Stateful data validator. Generated struct exposes
    /// `validate(args...) -> <ReturnStruct>` where `<ReturnStruct>` has the
    /// fields listed in `output` (canonical order). Internal sample memory
    /// for rate-of-change checks is implicit in the struct — not modelled in
    /// the manifest. The oracle lives in the `validators` reference section
    /// and uses stateful sequences that exercise range / ROC / plausibility
    /// branches and the documented rule that state only advances on a
    /// successful validation.
    ///
    /// Every existing forge validator returns `ValidationResult { valid, reason }`,
    /// but modelling the return shape as `Vec<StructField>` keeps the oracle
    /// format, the conformance fragments, and future multi-field codec-style
    /// kinds all sharing one descriptor instead of every fragment carrying a
    /// duplicated hard-coded list of field names.
    Validator {
        args: Vec<CanonicalType>,
        output: Vec<StructField>,
        /// Derived at harness-rendering time from SCXML — true iff at least
        /// one `<data sce:max-delta=...>` is present (rate-of-change rule
        /// is the only state-bearing validator construct). Empty in
        /// fixtures.json (manifest-side override is rejected); the value is
        /// computed by `read_validator_has_state` at render time so the
        /// SCXML remains the single source of truth.
        ///
        /// All backends except C11 generate uniform `validator.validate(...)`
        /// regardless of state, so the flag is unused for them. C11 needs
        /// it because its C11 emitter uses a free function for stateless
        /// validators (no instance to call a method on) and a state struct +
        /// pointer pass for stateful ones; the harness fragment branches
        /// on this flag to emit the matching call shape.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        has_state: bool,
    },
    /// Byte-layout codec. Generated struct exposes `encode()` returning the
    /// raw byte vector and a static `decode(raw)` returning the typed struct
    /// (wrapped in the language's optional type). The fixture oracle holds
    /// per-case `{decoded, encoded}` pairs so every test exercises both
    /// directions and catches drift in either path.
    ///
    /// Field layout (names, canonical types, byte/bit offsets) lives in the
    /// SCXML source — the forge product code generator is the authority on
    /// it. The manifest entry mirrors only the field *names and types* so
    /// the conformance fragments can iterate them to build the decoded
    /// literal and per-field equality assertions without hard-coding names
    /// across five templates. Field names are declared in SCXML source case
    /// (camelCase from `<data id="...">`) and each per-language fragment
    /// applies its own case filter to match the generated struct's native
    /// naming convention.
    Codec {
        fields: Vec<StructField>,
        /// RFC §synth-5-B test-vector flag: derived at harness-rendering time
        /// from SCXML — true iff at least one `<sce:test-vector>` element
        /// appears under the codec root. The per-backend codec
        /// sidecar (`<fixture>_test.{rs,h}`) is emitted on Rust + C11 only;
        /// Cpp/Kotlin/Go/Python have no codec sidecar template, so the
        /// flag is forced false for them. ⚠ This gate is the codec's
        /// own and no longer mirrors the algorithm's: the algorithm
        /// sidecar's language gate was DELETED when its closure
        /// rotation completed, so `FixtureSpec::Algorithm`'s field of
        /// the same name is now true on all 6 backends. Do not read
        /// one as evidence for the other. Empty in fixtures.json
        /// (manifest-side override is rejected); computed by
        /// `read_codec_has_test_vectors` so the SCXML stays the
        /// single source of truth.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        has_test_vectors: bool,
        /// The document is `sce:encoding="cbor"` (SCE_FORGE.md §4.6.1): its
        /// encode can refuse a value, so a fragment unwraps the encode's
        /// result, and its `optional<T>` entries are `optional` fields.
        /// Derived from the same parse the generator reads, like
        /// `has_test_vectors`; fixtures.json never states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        cbor: bool,
        /// The document is `sce:encoding="content-line"` (SCE_FORGE.md §4.6.4):
        /// its encode and decode can refuse a value, a case may be `decode_only`
        /// (an input the codec reads and does not write), and its `optional<T>`
        /// entries are `optional` and its `sce:max-count` ones `list` fields.
        /// Derived from the same parse the generator reads; fixtures.json never
        /// states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        content_line: bool,
        /// The codec has no form at the default profile a fragment can name
        /// (docs/adr/0077 §7): `Inline` holds no container of some entry of a
        /// content-line codec, or no list of a positional one or of a codec it
        /// holds, whose protocol bounds it by nothing. A fragment names the
        /// growing profile instead. Derived from the same parse the generator
        /// reads; fixtures.json never states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        grows: bool,
        /// The document declares `sce:encode-may-fail` (docs/adr/0065): its
        /// `encode_to_vec` returns a `Result` the fragment unwraps, where a
        /// codec whose encode is total returns its bytes. Derived from the
        /// same parse the generator reads; fixtures.json never states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        encode_may_fail: bool,
        /// The language's encode refuses a value outside a field's bound before
        /// its first byte (docs/adr/0077 §3), so its C++, Go or Kotlin
        /// convenience facade returns an optional or an error the fragment
        /// unwraps. Derived per language from the document; fixtures.json never
        /// states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        encode_refuses_bound: bool,
        /// The C++ backend prints the codec from its encode plan
        /// (docs/adr/0081 step 4): the fragment makes a value by its
        /// constructor and reads each element through its accessor. Derived
        /// from the document; fixtures.json never states it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        cpp_printed: bool,
    },
    /// Periodic timer (`SCE_FORGE.md` §4.10). ONE per document — the
    /// generated class wraps a single `ITimer&` and exposes `start()`,
    /// `cancel()`, and a hook per declared lifecycle element. The
    /// conformance fragment injects a recording mock, calls `start()`,
    /// and verifies the registered period; callback dispatch is verified
    /// by triggering the mock's stored callback and checking that the
    /// recording handler received `fire<FireEvent>()`.
    ///
    /// ⚠ The fields mirror `forge::model::TimerModel`, which is the
    /// document shape and the only authority on it. They were previously
    /// a `Vec<TimerFixtureEntry>` of `{timer_type, interval_ms}` records,
    /// describing the multi-timer `<data sce:timer="...">` shape removed on
    /// 2026-05-12 — a fixture written against that type would have been
    /// asked for two fields no document can carry and none of the four it
    /// must.
    ///
    /// ⚠⚠ No fixture selects this variant and no fragment template reads
    /// it: measured 2026-09-17, `timer` is the one kind with zero entries
    /// in `fixtures.json`, so no language arm verifies timer codegen.
    /// Retyping it does not close that gap, it only stops the type from
    /// misdirecting whoever closes it.
    Timer {
        /// `<sce:period>` in microseconds, as `TimerModel::period_us`.
        period_us: u64,
        /// `<sce:fire-event>` — the event fired on every expiry.
        fire_event: String,
        /// `<sce:reset-on event="..."/>`, when the document declares one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reset_on_event: Option<String>,
        /// `<sce:cancel-on state-exit="..."/>`, when the document declares one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cancel_on_state_exit: Option<String>,
    },
    /// A named value vocabulary (`sce:kind="enum"`).
    ///
    /// The one thing six languages must agree on is the NUMBER each variant
    /// carries, because that number is on a wire. Everything else about the
    /// emitted type differs by language and legitimately so: C++ and Rust
    /// spell a variant `GeneralReject`, Python and Kotlin `GENERAL_REJECT`,
    /// and Go prefixes the type name onto each constant.
    ///
    /// So `variants` holds the SOURCE spelling the document writes — a
    /// canonical key that is not any one language's. The identifier each arm
    /// asserts on is not re-derived by the fragment: the render puts the
    /// declaration's own spelling beside each variant (see
    /// `fold_enum_identifiers`).
    Enum {
        /// Declared variants in document order, as `{name, value}`.
        variants: Vec<EnumVariantFixture>,
    },
    /// An `sce:kind="event-schema"` payload. The kind emits a struct and no
    /// behaviour, so this fixture states the SHAPE the six arms must agree on
    /// — see [`EventSchemaFieldFixture`] for what that does and does not
    /// include, and why a width claim is not part of it.
    ///
    /// ⚠ `event_name` is carried even though C++ and Rust emit no constant
    /// for it: it is what the document declares the struct to be the payload
    /// OF, and a manifest that dropped it would describe a nameless record.
    /// The fragments that can reach it assert it; the others assert the
    /// fields, which is the part every arm emits.
    // ⚠ RENAMED, because `rename_all = "snake_case"` turns `EventSchema` into
    // `eventschema` and the DOCUMENT's attribute is `event-schema`. Leaving
    // the derived tag would make the manifest spell the kind one way and
    // every `.scxml`, `ForgeKind::from_attr` and `kind_str()` spell it
    // another — the exact hyphen this survey has already been wrong about
    // once, in the other direction.
    #[serde(rename = "event-schema")]
    EventSchema {
        /// The document's `sce:event-name`.
        event_name: String,
        /// Payload fields in document order.
        fields: Vec<EventSchemaFieldFixture>,
    },
    /// RFC §synth-5-A pure free function with bounded loops. Mirrors `Transform`'s
    /// `(args -> scalar output)` shape but admits `bytes` parameters because
    /// the algorithm body's `<sce:foreach>` over bytes is the canonical
    /// surface for byte-stream computations (CRC, parity, hash). The
    /// `function` field carries the algorithm's exported symbol name so the
    /// fragment can call it directly; in C11 the harness prepends the
    /// fixture name (mirrors `Transform`'s flat-scope prefix).
    Algorithm {
        args: Vec<AlgorithmArg>,
        output: AlgorithmOutput,
        /// Exported function symbol — equals the algorithm `<sce:name>` in
        /// snake-case for Rust/C11. Carried in the manifest because RFC §synth-5-A
        /// allows the algorithm name to differ from the fixture file name
        /// (the algorithm fixture's SCXML root carries `sce:kind="algorithm"`
        /// and the function's identity comes from the model, not the
        /// filename). C11 harness rendering rewrites this to the prefixed
        /// form `<fixture>_<function>` to match the flat-scope identifier.
        function: String,
        /// Derived at harness-rendering time from SCXML — true iff at
        /// least one `<sce:test-vector>` element appears under the
        /// algorithm root. RFC §synth-5-B test-vector support (item B2)
        /// emits a per-backend sidecar on ALL 6 backends —
        /// `<snake>_test.rs` (Rust), `<snake>_test.h` (C11 + Cpp),
        /// `<snake>_test.go` (Go), `<snake>_test.py` (Python),
        /// `<Pascal>TestVectors.kt` (Kotlin) — so this flag is set
        /// unconditionally, with no language gate in the filter below.
        ///
        /// ⚠ This doc used to say "Rust + C11 only; the filter drops the
        /// fixture from Cpp/Kotlin/Go/Python". That was true before the
        /// closure rotation and is FALSE now: the defensive language gate
        /// in `render_algorithm_test_vector_sidecar` was deleted in the
        /// final (Python) closure, and its match arms are exhaustive over
        /// `Language`. Ask the emitter, not this comment.
        ///
        /// ⚠⚠ How the sidecar is RUN differs per backend, which is why a
        /// per-report test count is not comparable across languages:
        /// Rust / C11 / Cpp fold it into the numerical-conformance harness,
        /// Python imports its class into the harness module, while Go
        /// (`*_test.go` picked up by `go test`) and Kotlin (a standalone
        /// JUnit class with its own `TEST-*.xml`) run it OUTSIDE that
        /// harness. Measured 2026-09-18: Kotlin's
        /// `TEST-NumericalConformanceTest.xml` reports 129 where Python's
        /// module collects 130 — the missing one is `AlgorithmCrc16
        /// TestVectors`, running green in its own report file. Compare
        /// whole-suite totals (jvmTest: 4 suites, 143 tests), never one
        /// report against another language's module.
        ///
        /// Empty in fixtures.json (manifest-side override is rejected);
        /// computed by `has_test_vectors_in_file` so the SCXML
        /// stays the single source of truth.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        has_test_vectors: bool,
        /// Derived at harness-rendering time: the algorithm declares
        /// `<sce:return may-fail="true">` (SCE_FORGE.md §3.4.1), read with
        /// the parse the generator reads. Its call returns through the
        /// backend's failure channel, and a case may expect a failure
        /// (`"fails": "<contract name>"`) in place of a value. Only a backend
        /// that lowers `may-fail` schedules the fixture
        /// ([`lang_supports_fixture`]). Never present in fixtures.json.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        may_fail: bool,
    },
    /// A union document (`sce:kind="union"`, docs/adr/0092). The kind emits a
    /// sum type and no behaviour, so the oracle states one value of each case
    /// as its wire object, and each fragment builds it and reads its tag back.
    /// A union an algorithm takes is generated as a fixture of its own, as the
    /// records of its cases are, since its unit names theirs as siblings.
    Union {
        /// Derived at harness-rendering time from the union's document (see
        /// [`UnionArg`]). Never present in fixtures.json.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shape: Option<UnionShape>,
    },
}

impl FixtureSpec {
    /// Lowercase discriminator string, used by this module's validate/render
    /// logic when it needs a stable identifier without pattern-matching the
    /// full variant. Matches the `#[serde(tag)]` value so it also matches
    /// what `{{ f.kind }}` renders inside the Jinja2 templates.
    pub fn kind_str(&self) -> &'static str {
        match self {
            FixtureSpec::Interpolation { .. } => "interpolation",
            FixtureSpec::Transform { .. } => "transform",
            FixtureSpec::Condition { .. } => "condition",
            FixtureSpec::Filter { .. } => "filter",
            FixtureSpec::Observer { .. } => "observer",
            FixtureSpec::Procedure { .. } => "procedure",
            FixtureSpec::Lookup { .. } => "lookup",
            FixtureSpec::Validator { .. } => "validator",
            FixtureSpec::Codec { .. } => "codec",
            FixtureSpec::Timer { .. } => "timer",
            FixtureSpec::Enum { .. } => "enum",
            // ⚠ HYPHENATED. `ForgeKind::from_attr` is keyed by the attribute
            // the document writes, and this kind's is `event-schema`. This
            // survey has already been wrong about that spelling once, in the
            // other direction — reporting the kind as having no templates
            // after searching for `eventschema`.
            FixtureSpec::EventSchema { .. } => "event-schema",
            FixtureSpec::Algorithm { .. } => "algorithm",
            FixtureSpec::Union { .. } => "union",
        }
    }

    /// Conservative (upper-bound) runtime dependency for this fixture kind.
    ///
    /// Delegates to `ForgeKind::max_runtime_dep()` via `kind_str()` so the
    /// classification table exists in exactly one place. For the precise
    /// L1-vs-L2 distinction on actual parsed documents, see
    /// `ForgeDocument::runtime_dep()`.
    pub fn runtime_dep(&self) -> RuntimeDep {
        crate::forge::model::ForgeKind::from_attr(self.kind_str())
            .expect("all fixture kind_str values are valid ForgeKind names")
            .max_runtime_dep()
    }
}

// ── Per-language canonical type mapping ────────────────────────
//
// Single source of truth: per-language harness templates used to define
// their own inline macros mapping "u16"/"f64"/... to their native type
// system. These helpers centralise that table so every template dispatches
// through one function per (language, canonical_type) pair. Registered as
// minijinja filters in `render_harness` below.
//
// The canonical_type argument is the lowercase string that
// `CanonicalType` serializes to; it is accepted as `&str` here so the
// filters can forward template values without allocating.

/// Map a canonical type to its Rust native type name.
pub fn rust_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "i32" => "i32",
        "i64" => "i64",
        "u8" => "u8",
        "u16" => "u16",
        "u32" => "u32",
        "u64" => "u64",
        "f32" => "f32",
        "f64" => "f64",
        "string" => "&str",
        "bytes" => "Vec<u8>",
        _ => "/* unknown canonical type */ ()",
    }
}

/// Build a Rust expression that unmarshals `raw` (a `serde_json::Value`
/// accessor expression) into a value of canonical type `ty`.
pub fn rust_unmarshal_expr(raw: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("{raw}.as_bool().expect(\"bool\")"),
        "f64" => format!("{raw}.as_f64().expect(\"f64\")"),
        "f32" => format!("{raw}.as_f64().expect(\"f64\") as f32"),
        "i32" => format!("{raw}.as_i64().expect(\"i64\") as i32"),
        "i64" => format!("{raw}.as_i64().expect(\"i64\")"),
        "u8" => format!("{raw}.as_u64().expect(\"u64\") as u8"),
        "u16" => format!("{raw}.as_u64().expect(\"u64\") as u16"),
        "u32" => format!("{raw}.as_u64().expect(\"u64\") as u32"),
        "u64" => format!("{raw}.as_u64().expect(\"u64\")"),
        "string" => format!("{raw}.as_str().expect(\"string\")"),
        "bytes" => format!(
            "{raw}.as_array().expect(\"array\").iter()\
             .map(|x| x.as_u64().expect(\"u64\") as u8)\
             .collect::<Vec<u8>>()"
        ),
        _ => format!("/* unknown canonical type {ty} */"),
    }
}

/// Map a canonical type to its C++ native type name.
pub fn cpp_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "f64" => "double",
        "f32" => "float",
        "i32" => "std::int32_t",
        "i64" => "std::int64_t",
        "u8" => "std::uint8_t",
        "u16" => "std::uint16_t",
        "u32" => "std::uint32_t",
        "u64" => "std::uint64_t",
        "string" => "std::string",
        "bytes" => "std::vector<std::uint8_t>",
        _ => "/* unknown canonical type */ void",
    }
}

/// Map a canonical type to its Go native type name (distinct from the
/// SCXML-variable `to_go_type` filter in `filters.rs`, which targets a
/// different type domain).
pub fn go_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "f64" => "float64",
        "f32" => "float32",
        "i32" => "int32",
        "i64" => "int64",
        "u8" => "uint8",
        "u16" => "uint16",
        "u32" => "uint32",
        "u64" => "uint64",
        "string" => "string",
        "bytes" => "[]byte",
        _ => "/* unknown canonical type */ interface{}",
    }
}

/// Map a canonical type to its C11 native type name. Codec `bytes`
/// fields land as a fixed-buffer + len pair in the C11 emission,
/// so this helper reports just the buffer
/// element type — the harness fragment generates the matching `[N]`
/// declaration alongside an explicit `_len` companion.
pub fn c_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "f64" => "double",
        "f32" => "float",
        "i32" => "int32_t",
        "i64" => "int64_t",
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        "string" => "const char *",
        "bytes" => "uint8_t",
        _ => "/* unknown canonical type */ void",
    }
}

/// The C11 struct a value of canonical type `ty` that may be absent is held in
/// (`sce_optional_int32_t`; docs/adr/0055), from the rule the generator names an
/// algorithm's optional slot by (`static_lowering::c11_optional_type`), so the
/// harness hands over the very type the parameter declares.
pub fn c_optional_type_for(ty: &str) -> String {
    let present = c_type_for(ty);
    crate::forge::static_lowering::c11_optional_type(present)
        .unwrap_or_else(|| format!("/* no optional of canonical type {ty} */ void"))
}

/// Format a JSON literal value (from `numerical_reference.json`) as a
/// C11 source-level literal for the requested canonical type. Used by
/// the C11 conformance harness to pre-bake oracle arrays at codegen
/// time (C has no JSON parser compatible with the
/// zero-runtime-dependency rule, so the oracle is materialised into
/// the harness source itself).
pub fn c_literal_for(value: &serde_json::Value, ty: &str) -> String {
    match (value, ty) {
        (serde_json::Value::Bool(b), _) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        (serde_json::Value::Number(n), "f32") => {
            let f = n.as_f64().unwrap_or(0.0);
            // Match Rust's "decimal int → .0_f32" promotion pattern but
            // emit C `f` suffix.
            if f.fract() == 0.0 && f.is_finite() {
                format!("{f:.1}f")
            } else {
                format!("{f}f")
            }
        }
        (serde_json::Value::Number(n), "f64") => {
            let f = n.as_f64().unwrap_or(0.0);
            if f.fract() == 0.0 && f.is_finite() {
                format!("{f:.1}")
            } else {
                format!("{f}")
            }
        }
        (serde_json::Value::Number(n), "u64") => {
            // u64 literals require an explicit ULL suffix in C11 — bare
            // text exceeding INT_MAX trips -Werror's
            // "integer constant is so large that it is unsigned"
            // (the C11 default-typing ladder picks signed long, which
            // overflows for ZInt u64 oracle vectors near u64::MAX).
            if let Some(u) = n.as_u64() {
                format!("{u}uLL")
            } else {
                format!("{n}uLL")
            }
        }
        (serde_json::Value::Number(n), "i64") => {
            // An `LL` suffix keeps a value past INT_MAX from being typed as
            // the default ladder picks it. `INT64_MIN` has no literal form,
            // so it is the one expression the forge emitters spell it as.
            match n.as_i64() {
                Some(i64::MIN) => crate::forge::expr::C_INT64_MIN.into(),
                Some(i) => format!("{i}LL"),
                None => format!("/* i64 oracle value out of range: {n} */"),
            }
        }
        (serde_json::Value::Number(n), _) => {
            // Integer canonical types — print the integer text.
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(u) = n.as_u64() {
                u.to_string()
            } else {
                n.to_string()
            }
        }
        (serde_json::Value::String(s), _) => format!("\"{s}\""),
        // Codec `bytes` field (sce:bit-size="tail" / "length-ref"):
        // oracle JSON shape is `[10, 20, 30]`; emit a C array initializer
        // so the harness fragment can drop it directly into a
        // `uint8_t input_<f>[<MAX>]` declaration. Empty payloads
        // collapse to `{ 0 }` because ISO C forbids `{ }` until C23.
        (serde_json::Value::Array(elems), "bytes") => {
            if elems.is_empty() {
                "{ 0 }".into()
            } else {
                let inner: Vec<String> = elems
                    .iter()
                    .map(|e| match e {
                        serde_json::Value::Number(n) => {
                            n.as_u64().map_or_else(|| n.to_string(), |u| u.to_string())
                        }
                        _ => "/* non-int bytes element */".into(),
                    })
                    .collect();
                format!("{{ {} }}", inner.join(", "))
            }
        }
        // Null / array / object are not valid scalar oracle values for
        // transform fixtures; emit a syntactic marker so the C
        // compiler rejects with a clear message if reached.
        _ => format!("/* unsupported oracle value: {value} */"),
    }
}

/// The bytes an oracle value stands for: a string as its UTF-8, an array of
/// integers as the bytes they name. `None` for anything else.
fn oracle_bytes(value: &serde_json::Value) -> Option<Vec<u8>> {
    match value {
        serde_json::Value::String(s) => Some(s.as_bytes().to_vec()),
        serde_json::Value::Array(items) => items
            .iter()
            .map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok()))
            .collect(),
        _ => None,
    }
}

/// A C string literal for `bytes`, every byte exactly as it is: a printable
/// ASCII character stands for itself, except `"`, `\` and `?` (a trigraph
/// opener), and every other byte is a three-digit octal escape, which — unlike
/// a hex escape — cannot swallow the character after it.
///
/// A content-line oracle value is arbitrary text: it may hold a newline, a
/// quote, a backslash or UTF-8, none of which `c_literal`'s bare `"{s}"`
/// carries.
pub fn c_bytes_literal_for(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() + 2);
    out.push('"');
    for &b in bytes {
        if (0x20..0x7F).contains(&b) && !matches!(b, b'"' | b'\\' | b'?') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("\\{b:03o}"));
        }
    }
    out.push('"');
    out
}

/// Map a canonical type to its Kotlin native type name.
pub fn kt_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "Boolean",
        "f64" => "Double",
        "f32" => "Float",
        "i32" => "Int",
        "i64" => "Long",
        "u8" => "UByte",
        "u16" => "UShort",
        "u32" => "UInt",
        "u64" => "ULong",
        "string" => "String",
        "bytes" => "ByteArray",
        _ => "/* unknown canonical type */ Any",
    }
}

/// Build a Kotlin expression that unmarshals `raw` (a `JsonElement`
/// accessor expression) into a value of canonical type `ty`.
pub fn kt_unmarshal_expr(raw: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("{raw}.jsonPrimitive.boolean"),
        "f64" => format!("{raw}.jsonPrimitive.double"),
        "f32" => format!("{raw}.jsonPrimitive.double.toFloat()"),
        "i32" => format!("{raw}.jsonPrimitive.int"),
        "i64" => format!("{raw}.jsonPrimitive.long"),
        "u8" => format!("{raw}.jsonPrimitive.int.toUByte()"),
        "u16" => format!("{raw}.jsonPrimitive.int.toUShort()"),
        // `jsonPrimitive.int` rejects values > Int.MAX (~2.1B) so it
        // can't carry the full unsigned 32-bit range; widen through Long.
        "u32" => format!("{raw}.jsonPrimitive.long.toUInt()"),
        // u64::MAX (18446744073709551615) overflows signed Long. Parse
        // the JSON number's textual content directly so the full
        // unsigned 64-bit range round-trips.
        "u64" => format!("{raw}.jsonPrimitive.content.toULong()"),
        "string" => format!("{raw}.jsonPrimitive.content"),
        "bytes" => format!("{raw}.jsonArray.map {{ it.jsonPrimitive.int.toByte() }}.toByteArray()"),
        _ => format!("/* unknown canonical type {ty} */"),
    }
}

/// The Swift type a canonical conformance type is held as: the width the algorithm's
/// parameter declares, so a value read from the oracle crosses the call as it is.
pub fn swift_type_for(ty: &str) -> &'static str {
    match ty {
        "bool" => "Bool",
        "f64" => "Double",
        "f32" => "Float",
        "i8" => "Int8",
        "i16" => "Int16",
        "i32" => "Int32",
        "i64" => "Int64",
        "u8" => "UInt8",
        "u16" => "UInt16",
        "u32" => "UInt32",
        "u64" => "UInt64",
        "string" => "String",
        "bytes" => "[UInt8]",
        _ => "/* unknown canonical type */ Any",
    }
}

/// Build a Swift expression that reads `raw` (an expression of the harness's `JSONValue`)
/// as a value of canonical type `ty`, through the typed readers the harness scaffold
/// defines (`oracleInteger`, `oracleReal`, ...). Each throws for a value of another shape,
/// which is a defect of the oracle or the manifest and fails the test that read it.
///
/// An integer is read from the text the number was written in, so the whole range of a
/// 64-bit type round-trips (the same reason the Kotlin reader goes through the content
/// for `u64`).
pub fn swift_unmarshal_expr(raw: &str, ty: &str) -> String {
    match ty {
        "bool" => format!("try oracleTruth({raw})"),
        "f64" => format!("try oracleReal({raw})"),
        "f32" => format!("Float(try oracleReal({raw}))"),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => {
            format!("try oracleInteger({raw}, as: {}.self)", swift_type_for(ty))
        }
        "string" => format!("try oracleText({raw})"),
        "bytes" => format!("try oracleBytes({raw})"),
        _ => format!("/* unknown canonical type {ty} */"),
    }
}

/// The case of a Swift observer's `Tag` enumeration that the oracle's tag `tag` names.
///
/// The oracle writes an observer's events as the upper-snake names the other backends
/// give their enumeration constants (`EMIT_WARNING`), and a Swift case is lowerCamel
/// (`emitWarning`), spelled from the document's event name as an enum's case is
/// ([`crate::forge::enum_naming::variant_ident`]). Lowering the tag first makes the
/// camel-casing of its words the inverse of the upper-snake spelling.
pub fn swift_event_case_of_tag(tag: &str) -> String {
    crate::forge::enum_naming::variant_ident(
        crate::generator::Language::Swift,
        "",
        &tag.to_lowercase(),
    )
}

/// Register the conformance-specific type-mapping filters on a minijinja
/// environment. Called from `render_harness` for every language so the
/// same filter names are available to every template — but note that not
/// every filter is meaningful for every language:
///
/// * `rust_type`, `cpp_type`, `go_type`, `kt_type` — pure type-name mapping,
///   defined for all four typed targets. Python is dynamically typed and
///   never references these.
/// * `rust_unmarshal`, `kt_unmarshal` — expression builders that wrap a JSON
///   accessor. Only Rust and Kotlin need them because their JSON libraries
///   require typed extraction expressions (`as_u64() as u16`, `.jsonPrimitive
///   .int.toUShort()`). C++ uses `nlohmann::json::get<T>()` directly, Go
///   uses `json.Unmarshal` with a typed `var`, and Python's stdlib produces
///   the right Python types on `json.load`. Adding `cpp_unmarshal` /
///   `go_unmarshal` would just duplicate what `cpp_type` / `go_type` already
///   feed into native accessors at the call site.
pub fn register_conformance_filters(env: &mut minijinja::Environment) {
    env.add_filter("rust_type", |ty: String| rust_type_for(&ty).to_string());
    env.add_filter("rust_unmarshal", |raw: String, ty: String| {
        rust_unmarshal_expr(&raw, &ty)
    });
    env.add_filter("cpp_type", |ty: String| cpp_type_for(&ty).to_string());
    env.add_filter("go_type", |ty: String| go_type_for(&ty).to_string());
    env.add_filter("kt_type", |ty: String| kt_type_for(&ty).to_string());
    env.add_filter("kt_unmarshal", |raw: String, ty: String| {
        kt_unmarshal_expr(&raw, &ty)
    });
    env.add_filter("swift_unmarshal", |raw: String, ty: String| {
        swift_unmarshal_expr(&raw, &ty)
    });
    env.add_filter("swift_event_case", |tag: String| {
        swift_event_case_of_tag(&tag)
    });
    // C11: `c_type` is the type-name mapping the harness
    // template uses for pre-baked oracle struct fields. `c_literal` is
    // a binary filter that takes a JSON value and a canonical type and
    // produces the exact C11 source literal — replacing the runtime
    // JSON parsing every other backend does at test execution time.
    env.add_filter("c_type", |ty: String| c_type_for(&ty).to_string());
    // The struct C11 holds a value of `ty` that may be absent in
    // (`sce_optional_int32_t`, `sce/forge/optional.h`), named by the one rule
    // the generated algorithm's slot is named by.
    env.add_filter("c_optional_type", |ty: String| c_optional_type_for(&ty));
    // A string or a byte array as a C string literal, and its length in bytes
    // (a string's is its UTF-8 length, which the template's own `length` —
    // a count of characters — is not).
    env.add_filter("c_bytes_literal", |value: minijinja::Value| -> String {
        let json: serde_json::Value =
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        match oracle_bytes(&json) {
            Some(bytes) => c_bytes_literal_for(&bytes),
            None => format!("/* not bytes: {json} */"),
        }
    });
    env.add_filter("byte_len", |value: minijinja::Value| -> usize {
        let json: serde_json::Value =
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        oracle_bytes(&json).map_or(0, |bytes| bytes.len())
    });
    env.add_filter(
        "c_literal",
        |value: minijinja::Value, ty: String| -> String {
            // minijinja Value → serde_json::Value via serialization,
            // since `c_literal_for` operates on serde_json::Value (which
            // is what the upstream `cases` enrichment hands us).
            let json: serde_json::Value =
                serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
            c_literal_for(&json, &ty)
        },
    );
}

/// Top-level manifest document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
pub struct Manifest {
    pub version: u32,
    #[serde(default)]
    pub description: String,
    pub fixtures: Vec<Fixture>,
}

impl Manifest {
    /// Load and parse a manifest file from disk.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read manifest at {}: {e}", path.display()))?;
        let manifest: Manifest = serde_json::from_str(&text)
            .map_err(|e| format!("invalid manifest JSON at {}: {e}", path.display()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Semantic cross-field checks the tagged-enum type system cannot express.
    ///
    /// Field-presence invariants ("filter requires `input`", "lookup requires
    /// `on_miss`", "observer requires `event_tags`", etc.) are now enforced
    /// by `FixtureSpec`'s per-variant struct fields — deserialization fails
    /// with a clear serde error before this function runs. What remains here
    /// is anything that the type system alone cannot check:
    ///
    /// * fixture names must be unique across the catalog,
    /// * `transform` fixtures must carry at least one of the two output
    ///   shapes (`output` scalar OR non-empty `compound_outputs`),
    /// * `validator` fixtures must declare at least one output field — an
    ///   empty `output` would render a test body with no assertions and
    ///   silently pass regardless of the generated struct's behaviour,
    /// * `lookup` fixtures must not preset `function` — it is derived from
    ///   the SCXML output id at harness-rendering time, and letting users
    ///   supply it in the manifest would invite the SCXML/manifest pair to
    ///   drift out of sync.
    ///
    /// This is still the **only** runtime validator for the manifest. The
    /// `tests/forge/conformance/fixtures.schema.json` document is purely for
    /// editor / IDE / CI-linter integration via the `$schema` reference in
    /// `fixtures.json`; it is never consulted at codegen time.
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        for f in &self.fixtures {
            if !seen.insert(f.name.clone()) {
                return Err(format!("duplicate fixture name: {}", f.name));
            }
            match &f.spec {
                FixtureSpec::Transform {
                    output,
                    compound_outputs,
                    holder,
                    ..
                } => {
                    if output.is_none() && compound_outputs.is_empty() {
                        return Err(format!(
                            "fixture {}: transform needs `output` or `compound_outputs`",
                            f.name
                        ));
                    }
                    if holder.is_some() {
                        return Err(format!(
                            "fixture {}: transform `holder` is derived from the \
                             SCXML (whether it reads a field through previous()); \
                             remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if let Some(co) = compound_outputs.iter().find(|co| co.field.is_some()) {
                        return Err(format!(
                            "fixture {}: compound output `{}` names its `field`, which \
                             is derived from the output id; remove it from fixtures.json",
                            f.name, co.key
                        ));
                    }
                }
                FixtureSpec::Lookup {
                    function,
                    output_id,
                    enum_values,
                    ..
                } => {
                    if function.is_some() {
                        return Err(format!(
                            "fixture {}: lookup `function` is derived from \
                             the SCXML output id; remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if output_id.is_some() {
                        return Err(format!(
                            "fixture {}: lookup `output_id` is derived from \
                             the SCXML; remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if !enum_values.is_empty() {
                        return Err(format!(
                            "fixture {}: lookup `enum_values` is derived from \
                             the SCXML entries; remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Validator {
                    output, has_state, ..
                } => {
                    if output.is_empty() {
                        return Err(format!(
                            "fixture {}: validator requires at least one \
                             `output` field — an empty output would render \
                             an assertion-free test body",
                            f.name
                        ));
                    }
                    if *has_state {
                        return Err(format!(
                            "fixture {}: validator `has_state` is derived from \
                             the SCXML (presence of <data sce:max-delta=...>); \
                             remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Codec {
                    fields,
                    has_test_vectors,
                    cbor,
                    content_line,
                    grows,
                    encode_may_fail,
                    encode_refuses_bound,
                    cpp_printed,
                } => {
                    if *cpp_printed {
                        return Err(format!(
                            "fixture {}: codec `cpp_printed` is derived from the SCXML \
                             (its encode plan); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *grows {
                        return Err(format!(
                            "fixture {}: codec `grows` is derived from the SCXML (the \
                             bounds its entries state); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *encode_may_fail {
                        return Err(format!(
                            "fixture {}: codec `encode_may_fail` is derived from the \
                             SCXML (sce:encode-may-fail); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *encode_refuses_bound {
                        return Err(format!(
                            "fixture {}: codec `encode_refuses_bound` is derived from the \
                             SCXML and the language; remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *cbor {
                        return Err(format!(
                            "fixture {}: codec `cbor` is derived from the \
                             SCXML (sce:encoding); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *content_line {
                        return Err(format!(
                            "fixture {}: codec `content_line` is derived from the \
                             SCXML (sce:encoding); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if let Some(field) = fields.iter().find(|field| field.optional) {
                        return Err(format!(
                            "fixture {}: field `{}` states `optional`, which is \
                             derived from the SCXML (an optional<T> CBOR or \
                             content-line entry); remove it from fixtures.json",
                            f.name, field.name
                        ));
                    }
                    if let Some(field) = fields.iter().find(|field| field.list) {
                        return Err(format!(
                            "fixture {}: field `{}` states `list`, which is \
                             derived from the SCXML (sce:max-count or \
                             sce:separator on a content-line entry); remove it \
                             from fixtures.json",
                            f.name, field.name
                        ));
                    }
                    if let Some(field) = fields
                        .iter()
                        .find(|field| field.enum_document.is_some() || field.enum_open)
                    {
                        return Err(format!(
                            "fixture {}: field `{}` states `enum_document` or `enum_open`, which are derived \
                             from the SCXML (the enum an `enum:<alias>` content-line entry \
                             imports); remove it from fixtures.json",
                            f.name, field.name
                        ));
                    }
                    if let Some(field) = fields.iter().find(|field| !field.members.is_empty()) {
                        return Err(format!(
                            "fixture {}: field `{}` states `members`, which are \
                             derived from the SCXML (the sce:param entries of a \
                             repeated content-line property); remove it from \
                             fixtures.json",
                            f.name, field.name
                        ));
                    }
                    // RFC §synth-5-B empty-codec rule: zero-field codecs
                    // (Zenoh KeepAlive et al.) are permitted. The
                    // round-trip test body still asserts encode →
                    // decode → encoded byte parity (an empty body
                    // round-trips an empty byte sequence), so the
                    // harness is not assertion-free.
                    if *has_test_vectors {
                        return Err(format!(
                            "fixture {}: codec `has_test_vectors` is \
                             derived from the SCXML (presence of \
                             <sce:test-vector>); remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Procedure {
                    helpers_ordered,
                    is_l2,
                    ..
                } => {
                    if !helpers_ordered.is_empty() {
                        return Err(format!(
                            "fixture {}: procedure `helpers_ordered` is \
                             derived from the SCXML at harness-rendering \
                             time; remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if *is_l2 {
                        return Err(format!(
                            "fixture {}: procedure `is_l2` is derived from \
                             the SCXML (presence of <sce:helper> / internal \
                             <data> / <send> in onentry / <donedata>); \
                             remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Timer {
                    period_us,
                    fire_event,
                    ..
                } => {
                    // Both checks exist for one reason: the rendered body
                    // asserts on the registered period and on the handler
                    // call named by `fire_event`. Either one absent leaves
                    // a test that starts a timer and claims nothing.
                    if *period_us == 0 {
                        return Err(format!(
                            "fixture {}: timer `period_us` must not be zero — \
                             the fragment asserts on the period the mock \
                             records, and zero is not a period a document \
                             can declare",
                            f.name
                        ));
                    }
                    if fire_event.is_empty() {
                        return Err(format!(
                            "fixture {}: timer `fire_event` must not be empty \
                             — it names the `fire<Event>()` call the recording \
                             handler is checked for, so an empty one would \
                             render an assertion-free test body",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Enum { variants } => {
                    // The rendered body is one assertion per variant, so an
                    // empty list renders a test that compiles, runs, and
                    // claims nothing — the vacuous shape this validator
                    // exists to refuse.
                    if variants.is_empty() {
                        return Err(format!(
                            "fixture {}: enum requires at least one `variants` \
                             entry — the fragment asserts one value per \
                             variant, and an empty vocabulary asserts nothing",
                            f.name
                        ));
                    }
                    // Two variants sharing a name cannot both be checked:
                    // the second assertion would name the first's constant.
                    let mut seen = std::collections::BTreeSet::new();
                    for v in variants {
                        if !seen.insert(v.name.as_str()) {
                            return Err(format!(
                                "fixture {}: enum variant `{}` is declared \
                                 twice — the fragment emits one assertion per \
                                 name, so a duplicate silently drops a check",
                                f.name, v.name
                            ));
                        }
                    }
                }
                FixtureSpec::EventSchema { event_name, fields } => {
                    // Same vacuity shape as `enum`: the body is one assertion
                    // per field, so an empty payload renders a test that
                    // constructs a struct and claims nothing about it.
                    if fields.is_empty() {
                        return Err(format!(
                            "fixture {}: event-schema requires at least one \
                             `fields` entry — the fragment asserts one \
                             round-trip per field, and an empty payload \
                             asserts nothing",
                            f.name
                        ));
                    }
                    if event_name.is_empty() {
                        return Err(format!(
                            "fixture {}: event-schema `event_name` must not be \
                             empty — it is what the payload is the payload OF, \
                             and a fixture without it describes a nameless \
                             record",
                            f.name
                        ));
                    }
                    // A duplicate field name would have the second assertion
                    // name the first field, dropping a check in silence.
                    let mut seen = std::collections::BTreeSet::new();
                    for field in fields {
                        if !seen.insert(field.name.as_str()) {
                            return Err(format!(
                                "fixture {}: event-schema field `{}` is \
                                 declared twice — the fragment emits one \
                                 assertion per name, so a duplicate silently \
                                 drops a check",
                                f.name, field.name
                            ));
                        }
                        // A bool written to a numeric field (or a number to a
                        // bool) would be rendered as a literal of the wrong
                        // kind — a compile error in the typed arms, a silent
                        // coercion in Python.
                        // A text is written between double quotes in every arm,
                        // so one with a quote, a backslash or a character an
                        // arm spells differently has no single literal.
                        if let EventSchemaFieldValue::Text(text) = &field.value {
                            if let Some(bad) = text
                                .chars()
                                .find(|c| !(c.is_ascii_alphanumeric() || " _.-".contains(*c)))
                            {
                                return Err(format!(
                                    "fixture {}: event-schema field `{}` holds text with `{}`, \
                                     which not every arm spells alike — a text is letters, \
                                     digits, spaces and `_.-`",
                                    f.name, field.name, bad
                                ));
                            }
                        }
                        let declared_bool = field.sce_type == "bool";
                        // An enum field's value is the name a variant declares.
                        let declared_text =
                            field.sce_type == "string" || field.sce_type.starts_with("enum:");
                        let value_bool = matches!(field.value, EventSchemaFieldValue::Bool(_));
                        let value_text = matches!(field.value, EventSchemaFieldValue::Text(_));
                        if declared_bool != value_bool || declared_text != value_text {
                            return Err(format!(
                                "fixture {}: event-schema field `{}` is declared \
                                 `{}` but its value is {} — a `bool` field takes \
                                 true/false, a `string` field text, an `enum:` field \
                                 the name a variant declares and every other field \
                                 a number",
                                f.name,
                                field.name,
                                field.sce_type,
                                match (value_bool, value_text) {
                                    (true, _) => "a bool",
                                    (_, true) => "text",
                                    _ => "a number",
                                }
                            ));
                        }
                    }
                }
                FixtureSpec::Algorithm {
                    args,
                    output,
                    function,
                    has_test_vectors,
                    may_fail,
                } => {
                    if *may_fail {
                        return Err(format!(
                            "fixture {}: algorithm `may_fail` is derived from the \
                             SCXML (<sce:return may-fail>); remove it from fixtures.json",
                            f.name
                        ));
                    }
                    if let AlgorithmOutput::List(list) = output {
                        if matches!(
                            list.list_of,
                            ListElemArg::Scalar(CanonicalType::String | CanonicalType::Bytes)
                        ) {
                            return Err(format!(
                                "fixture {}: algorithm `list_of` must be a \
                                 scalar element type — list<T> admits no \
                                 string or bytes element (SCE_FORGE.md §4.12)",
                                f.name
                            ));
                        }
                    }
                    // A record names an event-schema fixture of this manifest,
                    // and its fields are that document's, derived when the
                    // harness renders — never written here.
                    // A list's record element is held to the same rule.
                    let records = args
                        .iter()
                        .filter_map(|a| match a {
                            AlgorithmArg::Record(r) => Some(r),
                            AlgorithmArg::List(list) => list.list_of.record(),
                            AlgorithmArg::Scalar(_)
                            | AlgorithmArg::Enum(_)
                            | AlgorithmArg::Optional(_)
                            | AlgorithmArg::Union(_) => None,
                        })
                        .chain(match output {
                            AlgorithmOutput::Record(r) => Some(r),
                            AlgorithmOutput::List(list) => list.list_of.record(),
                            AlgorithmOutput::Optional(_) | AlgorithmOutput::Scalar(_) => None,
                        });
                    // A value of an enum names an enum fixture of this manifest,
                    // whose variants are derived when the harness renders.
                    for e in args.iter().filter_map(|a| match a {
                        AlgorithmArg::Enum(e) => Some(e),
                        _ => None,
                    }) {
                        let names_an_enum = self.fixtures.iter().any(|g| {
                            g.name == e.enum_document && matches!(g.spec, FixtureSpec::Enum { .. })
                        });
                        if !names_an_enum {
                            return Err(format!(
                                "fixture {}: enum `{}` names no enum fixture in this manifest",
                                f.name, e.enum_document
                            ));
                        }
                        if !e.variants.is_empty() {
                            return Err(format!(
                                "fixture {}: enum `{}` carries `variants`, which are \
                                 derived from the enum document; remove them",
                                f.name, e.enum_document
                            ));
                        }
                    }
                    // A value of a union names a union fixture of this manifest,
                    // whose cases are derived when the harness renders.
                    for u in args.iter().filter_map(|a| match a {
                        AlgorithmArg::Union(u) => Some(u),
                        _ => None,
                    }) {
                        let names_a_union = self.fixtures.iter().any(|g| {
                            g.name == u.union_document
                                && matches!(g.spec, FixtureSpec::Union { .. })
                        });
                        if !names_a_union {
                            return Err(format!(
                                "fixture {}: union `{}` names no union fixture in this manifest",
                                f.name, u.union_document
                            ));
                        }
                        if u.shape.is_some() {
                            return Err(format!(
                                "fixture {}: union `{}` carries `shape`, which is derived \
                                 from the union document; remove it",
                                f.name, u.union_document
                            ));
                        }
                    }
                    // A value that may be absent is held by every backend's
                    // algorithm over a fixed-width number or a `bool`
                    // (docs/adr/0055), and an optional answer is compared for
                    // equality: an absent value is near no number.
                    let optional_elements = args
                        .iter()
                        .filter_map(|a| match a {
                            AlgorithmArg::Optional(o) => Some(o.optional),
                            _ => None,
                        })
                        .chain(match output {
                            AlgorithmOutput::Optional(o) => Some(o.optional),
                            _ => None,
                        });
                    for element in optional_elements {
                        if matches!(element, CanonicalType::String | CanonicalType::Bytes) {
                            return Err(format!(
                                "fixture {}: algorithm `optional` must be a number or a \
                                 bool — the harness holds no optional text or bytes",
                                f.name
                            ));
                        }
                    }
                    if let AlgorithmOutput::Optional(o) = output {
                        if o.compare != CompareMode::Equality {
                            return Err(format!(
                                "fixture {}: an `optional` output is compared for \
                                 equality, so its `compare` is \"equality\"",
                                f.name
                            ));
                        }
                    }
                    for r in records {
                        let names_a_schema = self.fixtures.iter().any(|g| {
                            g.name == r.record && matches!(g.spec, FixtureSpec::EventSchema { .. })
                        });
                        if !names_a_schema {
                            return Err(format!(
                                "fixture {}: record `{}` names no event-schema \
                                 fixture in this manifest",
                                f.name, r.record
                            ));
                        }
                        if !r.fields.is_empty() {
                            return Err(format!(
                                "fixture {}: record `{}` carries `fields`, which are \
                                 derived from the event-schema document; remove them",
                                f.name, r.record
                            ));
                        }
                    }
                    if function.is_empty() {
                        return Err(format!(
                            "fixture {}: algorithm `function` must not be \
                             empty — it names the exported symbol the \
                             harness calls",
                            f.name
                        ));
                    }
                    if args.is_empty() {
                        return Err(format!(
                            "fixture {}: algorithm requires at least one \
                             `args` entry — a zero-arg algorithm has no \
                             oracle-driven cases to compare",
                            f.name
                        ));
                    }
                    if *has_test_vectors {
                        return Err(format!(
                            "fixture {}: algorithm `has_test_vectors` is \
                             derived from the SCXML (presence of \
                             <sce:test-vector>); remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Union { shape } => {
                    // The cases are the document's, read when the harness
                    // renders; a manifest that wrote them would be a second
                    // statement of the union that could disagree with it.
                    if shape.is_some() {
                        return Err(format!(
                            "fixture {}: union `shape` is derived from the union \
                             document; remove it from fixtures.json",
                            f.name
                        ));
                    }
                }
                FixtureSpec::Interpolation { .. }
                | FixtureSpec::Condition { .. }
                | FixtureSpec::Filter { .. }
                | FixtureSpec::Observer { .. } => {}
            }
        }
        Ok(())
    }
}

/// Per-language harness filesystem layout. Returned as a single bundle by
/// [`harness_layout`] so adding a new language is one match arm in one
/// function instead of three siblings that can drift out of sync.
pub struct HarnessLayout {
    /// Output filename written under the build's chosen output directory.
    pub output_filename: &'static str,
    /// Subdirectory of `tools/codegen/templates/` containing this language's
    /// conformance scaffold and `kinds/` fragments.
    pub template_subdir: &'static str,
    /// Top-level scaffold filename inside `template_subdir`.
    pub template_filename: &'static str,
}

/// Canonical layout (output name, template directory, scaffold name) for
/// the given language. Single source of truth: every consumer that used to
/// call the legacy `harness_filename` / `template_subdir` / `template_filename`
/// trio now goes through this one function.
pub fn harness_layout(language: Language) -> HarnessLayout {
    match language {
        Language::Rust => HarnessLayout {
            output_filename: "numerical_conformance.rs",
            template_subdir: "forge/rust/conformance",
            template_filename: "harness.rs.jinja2",
        },
        // Not test_-prefixed so the committed shim at
        // backends/python/forge-runtime/tests/test_numerical_conformance.py can
        // import from this module without name collision during unittest
        // discovery.
        Language::Python => HarnessLayout {
            output_filename: "conformance_generated.py",
            template_subdir: "forge/python/conformance",
            template_filename: "harness.py.jinja2",
        },
        Language::Go => HarnessLayout {
            output_filename: "numerical_conformance_test.go",
            template_subdir: "forge/go/conformance",
            template_filename: "harness.go.jinja2",
        },
        Language::Kotlin => HarnessLayout {
            output_filename: "NumericalConformanceTest.kt",
            template_subdir: "forge/kotlin/conformance",
            template_filename: "harness.kt.jinja2",
        },
        Language::Cpp => HarnessLayout {
            output_filename: "numerical_conformance_test.cpp",
            template_subdir: "forge/cpp/conformance",
            template_filename: "harness.cpp.jinja2",
        },
        // RFC §synth-5-J-1: C11 backend foundation lands in M1; the
        // conformance harness template (`forge/c/conformance/harness.c.jinja2`)
        // is M2+ work. The layout is wired here so a CLI invocation with
        // `--language c11` fails at template-load with a precise file path
        // (clear "missing template" error) instead of panicking on an
        // exhaustiveness gap.
        Language::C11 => HarnessLayout {
            output_filename: "numerical_conformance_test.c",
            template_subdir: "forge/c/conformance",
            template_filename: "harness.c.jinja2",
        },
        // The harness template is written with the Swift forge emitters. A run
        // with `--language swift` is refused before this layout is read
        // (`codegen_matrix::template_ships`), and one that reached it would
        // fail at template-load naming the file, as C11's did.
        Language::Swift => HarnessLayout {
            output_filename: "NumericalConformanceTests.swift",
            template_subdir: "forge/swift/conformance",
            template_filename: "harness.swift.jinja2",
        },
    }
}

/// Backwards-compatible wrapper. Kept so existing call sites in
/// `sce_codegen.rs` and `backends/rust/forge-conformance/build.rs` continue to
/// compile; new code should call [`harness_layout`] directly.
pub fn harness_filename(language: Language) -> &'static str {
    harness_layout(language).output_filename
}

/// Gate which fixture kinds the C11 conformance harness includes. The
/// filter exists because `harness.c.jinja2` does dynamic
/// `{% include "kinds/" ~ f.kind ~ ".c.jinja2" %}` and would fail at
/// render time on any kind whose fragment is not present. The
/// predicate is currently total — every shipped `FixtureSpec` variant
/// lowers through the C11 backend — but it stays an exhaustive match
/// (see the note on the final arm) so a future kind addition forces an
/// explicit C11 admission decision alongside its
/// `kinds/<kind>.c.jinja2` fragment.
///
/// Exposed as `pub` so the `sce-codegen list-fixtures --language c11`
/// CLI can apply the same filter that `generate-conformance` uses,
/// keeping the c11 cmake harness's fixture set auto-derived from the
/// single manifest source of truth instead of hand-maintained.
/// RFC §synth-5-J-4 per-fixture matrix gate: combines the kind-level
/// `template_ships` matrix with two per-fixture predicates that the
/// matrix cannot express:
///
///   1. **C11 kind filter** (`c11_supported_kind`) — the explicit
///      per-kind C11 admission predicate, documented separately.
///   2. **MCU-only fixture filter** (`codec_has_mcu_only_features`) —
///      `<sce:dma-aligned>` codecs lower to Rust + C11 only; cpp /
///      kotlin / go / python codegen rejects them with the typed
///      `MCU-class kind` error. The kind-matrix can't express this
///      because `kind=codec` ships everywhere — the per-fixture SCXML
///      decides.
///
/// Used by both `render_harness` (the harness fragment renderer
/// retains the matching subset) and `list-fixtures --language <X>`
/// (cmake reads the same subset to schedule per-fixture
/// add_custom_command). Single source of truth so `cmake --build` and
/// `cargo test --test numerical_conformance` cannot drift on which
/// fixtures the per-language harness compiles.
///
/// `resource_dir` is `tests/forge/resources` (the SCXML location);
/// MCU detection requires reading the SCXML, which is why this needs
/// the path. Passes `Ok(true)` through when the SCXML is missing —
/// non-codec fixtures or test-only entries don't have a paired SCXML
/// and aren't MCU candidates anyway.
pub fn lang_supports_fixture(
    fixture: &Fixture,
    language: crate::generator::Language,
    resource_dir: &Path,
) -> Result<bool, String> {
    let Some(kind) = crate::forge::model::ForgeKind::from_attr(fixture.spec.kind_str()) else {
        return Ok(false);
    };
    if !crate::forge::codegen_matrix::template_ships(kind, language) {
        // A backend that writes the kind in part is scheduled for the fixtures its
        // generator writes, which it alone can say: it refuses the rest by name.
        if !crate::forge::codegen_matrix::ships_in_part(kind, language)
            || !has_conformance_fragment(language, fixture.spec.kind_str())
            || !generator_writes_fixture(fixture, language, resource_dir)?
        {
            return Ok(false);
        }
    }
    if matches!(language, crate::generator::Language::C11) && !c11_supported_kind(&fixture.spec) {
        return Ok(false);
    }
    // Per-fixture MCU-only gate: only the four non-MCU backends need
    // the SCXML scan. Rust + C11 are the MCU substrate — every
    // codec's product code lowers there; no need to filter.
    let is_non_mcu = matches!(
        language,
        crate::generator::Language::Cpp
            | crate::generator::Language::Kotlin
            | crate::generator::Language::Go
            | crate::generator::Language::Python
            | crate::generator::Language::Swift
    );
    if is_non_mcu && matches!(fixture.spec, FixtureSpec::Codec { .. }) {
        let scxml_path = fixture.document_path(resource_dir);
        if document_exists(&scxml_path) && codec_has_mcu_only_features(&scxml_path)? {
            return Ok(false);
        }
    }
    // Per-fixture CBOR gate (SCE_FORGE.md §4.6.1): a backend generates a
    // `sce:encoding="cbor"` codec where `cbor_codec::refusal` has no reason
    // not to — the language lowers it and, on C11, every string entry is
    // bounded — which is the same answer the generator reads.
    if matches!(fixture.spec, FixtureSpec::Codec { .. }) {
        let scxml_path = fixture.document_path(resource_dir);
        if document_exists(&scxml_path) {
            let model = read_codec_model(&scxml_path, &fixture.name)?;
            if model.encoding == crate::forge::model::CodecEncoding::Cbor
                && crate::forge::cbor_codec::refusal(language, &model).is_some()
            {
                return Ok(false);
            }
            // The same for a `sce:encoding="content-line"` codec (SCE_FORGE.md
            // §4.6.4): scheduled exactly where `content_line_codec::refusal`
            // admits it.
            if model.encoding == crate::forge::model::CodecEncoding::ContentLine
                && crate::forge::content_line_codec::refusal(language, &model).is_some()
            {
                return Ok(false);
            }
        }
    }
    // Per-fixture `may-fail` gate (SCE_FORGE.md §3.4.1): a backend lowers a
    // `may-fail` algorithm in the commit that teaches it the checked
    // lowering, and refuses it until then — so the harness schedules the
    // fixture exactly where the generator's own list admits it.
    if matches!(fixture.spec, FixtureSpec::Algorithm { .. })
        && !crate::forge::generator::lowers_may_fail(language)
    {
        let scxml_path = fixture.document_path(resource_dir);
        if document_exists(&scxml_path) && read_algorithm_may_fail(&scxml_path, &fixture.name)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether the template tree holds the conformance fragment of `kind` for `language`
/// (`<template_subdir>/kinds/<kind>.*`). A harness cannot render a fixture of a kind it has
/// no fragment for, so a backend that writes the kind in part is scheduled for it only once
/// the fragment is written: the harness grows one kind at a time, and a fixture is never
/// skipped for want of a fragment that exists.
///
/// Asked of the template tree on disk, the one `render_harness` loads the fragment from, and
/// not of the registry embedded in the binary: a binary is not rebuilt for a template edit
/// (the generator witness leaves the templates out because they are read at run time), so
/// an embedded copy would say a fragment is missing that the renderer is about to read.
fn has_conformance_fragment(language: crate::generator::Language, kind: &str) -> bool {
    let layout = harness_layout(language);
    let kinds = crate::find_template_base()
        .join(layout.template_subdir)
        .join("kinds");
    let stem = format!("{kind}.");
    std::fs::read_dir(kinds).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with(&stem))
    })
}

/// Whether the generator of `language`, which writes the fixture's kind in part, writes
/// the fixture's document: the answer `sce-codegen check` gives. A refusal by name
/// (`generate/unsupported-feature`, exit status 7) is a document the backend does not
/// lower yet and the harness leaves out; any other failure is a defect and is returned,
/// since a fixture skipped for it would be a conformance claim nobody made.
fn generator_writes_fixture(
    fixture: &Fixture,
    language: crate::generator::Language,
    resource_dir: &Path,
) -> Result<bool, String> {
    let path = fixture.document_path(resource_dir);
    if !document_exists(&path) {
        return Ok(false);
    }
    match crate::compile_forge_file(&path, language, &[], &crate::ForgeCompileOptions::default()) {
        Ok(_) => Ok(true),
        Err(refused) if refused.error.exit_code() == 7 => Ok(false),
        Err(failed) => Err(format!(
            "fixture {}: the {} generator failed on {} with something other than a refusal by \
             name: {}",
            fixture.name,
            language.canonical_name(),
            path.display(),
            failed.error
        )),
    }
}

pub fn c11_supported_kind(spec: &FixtureSpec) -> bool {
    match spec {
        FixtureSpec::Transform { .. }
        | FixtureSpec::Condition { .. }
        | FixtureSpec::Lookup { .. }
        | FixtureSpec::Validator { .. } => true,
        // String-bearing codecs included: fields declared
        // `sce:type="string"` lower to `char[N] + size_t len` storage
        // with `sce_forge_is_valid_utf8` decode-time validation and the
        // SCE_FORGE_CODEC_INVALID_UTF8 typed return; see `render_codec`
        // and `c/codec.h.jinja2`'s `field.is_string` branch.
        FixtureSpec::Codec { .. } => true,
        // Stateful imports (`<sce:import kind="codec"/>` etc.) lower
        // into the procedure state struct via the C11 codegen import
        // path + AST pre-pass (expr::lower_stateful_import_calls), so
        // cross-file procedure fixtures flow through the same harness
        // as in-file ones.
        FixtureSpec::Procedure { .. } => true,
        // Signal filter kinds (low_pass / moving_average / debounce):
        // C11 bakes the algorithm at codegen time per fixture (no
        // runtime header surface) — matches codec's bake-at-codegen
        // pattern. The unified `render_filter` adds `<snake>` +
        // `<input_id_snake>` to its context so the C11 template emits
        // free functions instead of templated classes.
        FixtureSpec::Filter { .. } => true,
        // Hysteresis observer kind: C11 bakes the ThresholdState bool
        // flag + fixed-cap EventQueue + tag enum inline per fixture.
        // `render_observer` follows Rust's `<id>_active` naming
        // convention and inserts `<snake>`/`<upper>` so global C
        // identifiers (enum tag constants, capacity macro) are
        // observer-prefixed.
        FixtureSpec::Observer { .. } => true,
        // 1D linear / 2D bilinear interpolation: C11 inlines the
        // algorithm per fixture with the array sizes substituted in
        // (cpp/Rust use templates over array sizes; bake-at-codegen
        // sidesteps the lack of generics). Static const breakpoint +
        // value tables are emitted at codegen time.
        FixtureSpec::Interpolation { .. } => true,
        // Timer scheduler: C11 emits a vtable-based ITimer interface +
        // function-pointer handler struct + per-timer start/cancel
        // pairs. cpp uses `class ITimer { virtual ... };` + a
        // `Handler&` template parameter (duck typing); the C11 mirror
        // is an explicit method-pointer struct + a NULL-checked handler
        // dispatch in the trampoline.
        //
        // Every `FixtureSpec` variant is admitted — all 10 forge kinds
        // emit through the same C11 codegen path. No catch-all arm, so
        // a future kind addition forces an explicit decision here.
        FixtureSpec::Timer { .. } => true,
        // Enum vocabulary: `c/enum.h.jinja2` emits a plain C enum with the
        // declared values, which is the whole surface the conformance
        // fragments compare.
        //
        // ⚠ This arm used to say it was "admitted for completeness, NOT
        // because it is exercised — forge-conformance.yml has no C11 job and
        // scripts/gates/ has no forge-c.sh, so no C11 fixture of any kind is
        // run today", and called that absence a lane to build. The lane was
        // built on 2026-09-17: `scripts/gates/forge-c11.sh` (named `forge-c`
        // until 2026-10-20) runs the suite and `conformance-c` runs it in CI.
        // The note is kept because a reader who remembers the old one needs
        // to see that it was retired rather than quietly edited away.
        FixtureSpec::Enum { .. } => true,
        // Event-schema payload: `c/event_schema.h.jinja2` emits a plain C
        // struct with the declared fields, which is the whole surface the
        // fragments compare. Nothing to bake — the kind carries no behaviour.
        FixtureSpec::EventSchema { .. } => true,
        // RFC §synth-7 A6: §synth-5-A algorithm kind. The C11 algorithm template
        // (`tools/codegen/templates/forge/c/algorithm.h.jinja2`) shipped
        // at A5; A6 promotes the byte-equivalence preview to a permanent
        // ctest gate by routing the two CRC16 fixtures through the
        // conformance harness. `bytes` arg lowers to `sce_forge_bytes_t`
        // value-type per the A5 `algorithm_param_type` arm.
        FixtureSpec::Algorithm { .. } => true,
        // Union: `c/union.h.jinja2` emits the struct of the tag and a C
        // `union` of the cases' records (docs/adr/0092, section 7), which the
        // fragment bakes each value of and reads the tag of.
        FixtureSpec::Union { .. } => true,
    }
}

/// Parse a `<scxml sce:kind="procedure">` source file and return the
/// `<sce:helper name="...">` declarations in source order. The conformance
/// harness uses this to drive positional `execute()` argument ordering from
/// the SCXML (single source of truth) while fixtures.json only supplies the
/// stub implementation dict, making name-drift structurally impossible.
pub(crate) fn read_procedure_helper_names(scxml_path: &Path) -> Result<Vec<String>, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    let Some(datamodel) = doc.root_element().children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "datamodel"
            && n.tag_name().namespace() == Some(scxml_ns)
    }) else {
        // No datamodel → no helpers. Procedures without datamodel are valid.
        return Ok(Vec::new());
    };
    let mut names = Vec::new();
    for child in datamodel.children().filter(|n| {
        n.is_element()
            && n.tag_name().name() == "helper"
            && n.tag_name().namespace() == Some(sce_ns)
    }) {
        let name = child.attribute("name").ok_or_else(|| {
            format!(
                "{}: <sce:helper> missing 'name' attribute",
                scxml_path.display()
            )
        })?;
        names.push(name.to_string());
    }
    Ok(names)
}

/// Parse a `<scxml sce:kind="procedure">` source file and detect whether
/// the procedure exercises any L2 marker. Mirrors `ProcedureModel::is_l2()`
/// (which operates on the fully parsed model) but uses a lightweight
/// roxmltree scan so the predicate is cheap to consult at fixture-
/// filtering time without invoking the full forge-model pipeline.
///
/// The four L2 markers, any of which flips the result to `true`:
///   1. `<sce:helper>` declaration in `<datamodel>`
///   2. `<data sce:direction="internal">` in `<datamodel>`
///   3. `<onentry><send>` (any send action — service/event/etc.)
///   4. `<donedata>` on a `<final>` state
pub(crate) fn read_procedure_is_l2(scxml_path: &Path) -> Result<bool, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;

    let root = doc.root_element();

    // Marker 1+2: scan <datamodel> for <sce:helper> or <data direction="internal">.
    if let Some(datamodel) = root.children().find(|n| {
        n.is_element()
            && n.tag_name().name() == "datamodel"
            && n.tag_name().namespace() == Some(scxml_ns)
    }) {
        for child in datamodel.children().filter(|n| n.is_element()) {
            let name = child.tag_name().name();
            let ns = child.tag_name().namespace();
            if name == "helper" && ns == Some(sce_ns) {
                return Ok(true);
            }
            if name == "data"
                && ns == Some(scxml_ns)
                && child.attribute((sce_ns, "direction")) == Some("internal")
            {
                return Ok(true);
            }
        }
    }

    // Marker 3+4: walk <state>/<final> children for <onentry><send> and
    // <donedata>. A procedure root only nests states one level deep
    // (no <parallel>, no compound states), so a flat scan suffices.
    for state in root.children().filter(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(scxml_ns)
            && (n.tag_name().name() == "state" || n.tag_name().name() == "final")
    }) {
        for inner in state.children().filter(|n| n.is_element()) {
            let inner_name = inner.tag_name().name();
            let inner_ns = inner.tag_name().namespace();
            if inner_ns != Some(scxml_ns) {
                continue;
            }
            if inner_name == "donedata" {
                return Ok(true);
            }
            if inner_name == "onentry" {
                let has_send = inner.children().any(|n| {
                    n.is_element()
                        && n.tag_name().name() == "send"
                        && n.tag_name().namespace() == Some(scxml_ns)
                });
                if has_send {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}

/// Parse a `<scxml sce:kind="lookup">` source file and return the
/// `<data sce:direction="out">` field's id (in source case). The conformance
/// harness uses this to derive the generated lookup function name without
/// duplicating it in fixtures.json — the SCXML file is the single source of
/// truth.
fn read_lookup_output_id(scxml_path: &Path) -> Result<String, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    let datamodel = doc
        .root_element()
        .children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "datamodel"
                && n.tag_name().namespace() == Some(scxml_ns)
        })
        .ok_or_else(|| format!("{}: missing <datamodel>", scxml_path.display()))?;
    for data in datamodel
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "data")
    {
        let dir = data.attribute((sce_ns, "direction")).unwrap_or("");
        if dir == "out" {
            return data.attribute("id").map(|s| s.to_string()).ok_or_else(|| {
                format!(
                    "{}: <data sce:direction=\"out\"> missing id",
                    scxml_path.display()
                )
            });
        }
    }
    Err(format!(
        "{}: no <data sce:direction=\"out\"> found",
        scxml_path.display()
    ))
}

/// Parse a `<scxml sce:kind="lookup">` source file with string output and
/// return the unique entry values in declaration order. Used by the harness
/// to generate an enum-to-string conversion helper for string-output lookups
/// without duplicating the values in fixtures.json.
fn read_lookup_enum_values(scxml_path: &Path) -> Result<Vec<String>, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    let datamodel = doc
        .root_element()
        .children()
        .find(|n| {
            n.is_element()
                && n.tag_name().name() == "datamodel"
                && n.tag_name().namespace() == Some(scxml_ns)
        })
        .ok_or_else(|| format!("{}: missing <datamodel>", scxml_path.display()))?;
    let mut values = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in datamodel.descendants().filter(|n| {
        n.is_element() && n.tag_name().name() == "entry" && n.tag_name().namespace() == Some(sce_ns)
    }) {
        if let Some(val) = entry.attribute("value") {
            if seen.insert(val.to_string()) {
                values.push(val.to_string());
            }
        }
    }
    Ok(values)
}

/// Parse a `<scxml sce:kind="validator">` source file and return whether
/// any `<data>` declares a `sce:max-delta` attribute (the only state-bearing
/// validator construct — rate-of-change rules retain a previous-value field
/// across calls). Used by the C11 harness fragment to choose between the
/// stateless free-function call shape and the stateful struct + pointer
/// pass shape. Other backends emit uniform method calls
/// regardless of state and ignore the flag.
/// RFC §synth-5-B test-vector: scan an SCXML root for any
/// `<sce:test-vector>` child element. The forge parser already
/// validates the attributes and stores the parsed vectors on
/// `AlgorithmModel.test_vectors` or `CodecModel.test_vectors`;
/// this scan only needs to surface whether any are present
/// so the harness filter can pick the right `(kind, language)`
/// matrix arm. Returns false when the file contains zero
/// `<sce:test-vector>` children, matching the no-sidecar default.
/// The element is kind-agnostic — both algorithm and codec roots
/// declare it, and the per-language sidecar emitter dispatches on
/// the model kind.
///
/// Exposed publicly under the `has_test_vectors` re-export so the
/// `sce-codegen list-fixtures --has-test-vectors` CLI can drive the
/// cmake harness's sidecar OUTPUT enumeration from the same SCXML
/// scan as the harness renderer.
pub fn has_test_vectors(scxml_path: &Path) -> Result<bool, String> {
    has_test_vectors_in_file(scxml_path)
}

/// Internal helper used by `render_harness` enrichment so the
/// destructured-field binding `has_test_vectors` (a `&mut bool`)
/// does not shadow the kind-agnostic public scan function.
fn has_test_vectors_in_file(scxml_path: &Path) -> Result<bool, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    Ok(doc.root_element().children().any(|n| {
        n.is_element()
            && n.tag_name().namespace() == Some(sce_ns)
            && n.tag_name().name() == "test-vector"
    }))
}

/// RFC §synth-5-J-4 per-fixture matrix gate: scan a codec SCXML for MCU-only
/// features. Mirrors `CodecModel::has_mcu_only_features` (currently
/// `has_dma_aligned_fields` — any `<sce:field>` carrying the
/// `sce:dma-burst-align` attribute). Non-MCU backends (cpp / kotlin /
/// go / python) reject these at codegen time with a hard
/// `MCU-class kind` error, so the conformance harness must prefilter
/// them before invoking sce-codegen — otherwise CMake's per-fixture
/// `add_custom_command` schedules a generation that will fail.
///
/// The kind-level matrix (`template_ships`) doesn't help here because
/// `kind=codec` ships on every backend; the MCU restriction is
/// per-fixture (the SCXML's feature set decides). Mirrors
/// `has_test_vectors_in_file`'s SCXML-scan approach so the conformance
/// pipeline stays free of extra manifest fields the author would have
/// to remember to flip.
///
/// Returns `Ok(false)` for non-codec SCXMLs and SCXMLs that exist but
/// don't carry any MCU-only marker; returns the IO/parse error
/// otherwise.
pub fn codec_has_mcu_only_features(scxml_path: &Path) -> Result<bool, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    // Walk every descendant. MCU-only markers today are attribute-only
    // (`sce:dma-burst-align` on `<sce:field>`); the element-form check
    // is preserved as a forward-compat hook for future MCU-only
    // markers that surface as standalone elements.
    fn walk(node: roxmltree::Node, sce_ns: &str) -> bool {
        for child in node.children() {
            if child.is_element() {
                // Attribute marker: `sce:dma-burst-align` on any
                // `<sce:field>` (mirrors `has_dma_aligned_fields`).
                if child
                    .attributes()
                    .any(|a| a.namespace() == Some(sce_ns) && a.name() == "dma-burst-align")
                {
                    return true;
                }
                if walk(child, sce_ns) {
                    return true;
                }
            }
        }
        false
    }
    Ok(walk(doc.root_element(), sce_ns))
}

/// Whether an algorithm fixture declares `may-fail` (SCE_FORGE.md §3.4.1) —
/// asked of the parse the generator reads, as [`read_transform_holder`]
/// asks its question, so the harness and the product cannot disagree.
fn read_algorithm_may_fail(scxml_path: &Path, fixture_name: &str) -> Result<bool, String> {
    let text = crate::load_forge_source(scxml_path, &[])
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?
        .positions
        .expanded;
    match crate::forge::parser::parse_forge(&text, crate::DocumentLabel::symmetric(fixture_name))
        .map_err(|e| format!("{}: {e}", scxml_path.display()))?
    {
        Some(crate::forge::model::ForgeDocument::Algorithm(m)) => Ok(m.signature.may_fail),
        _ => Err(format!(
            "fixture {fixture_name}: the manifest says `algorithm`, and {} is not an \
             algorithm document",
            scxml_path.display()
        )),
    }
}

/// The codec fixture's document, as the generator parses it.
fn read_codec_model(
    scxml_path: &Path,
    fixture_name: &str,
) -> Result<crate::forge::model::CodecModel, String> {
    let text = crate::load_forge_source(scxml_path, &[])
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?
        .positions
        .expanded;
    match crate::forge::parser::parse_forge(&text, crate::DocumentLabel::symmetric(fixture_name))
        .map_err(|e| format!("{}: {e}", scxml_path.display()))?
    {
        Some(crate::forge::model::ForgeDocument::Codec(m)) => Ok(m),
        _ => Err(format!(
            "fixture {fixture_name}: the manifest says `codec`, and {} is not a codec document",
            scxml_path.display()
        )),
    }
}

/// An enum a content-line codec reads an entry by the text of (docs/adr/0015),
/// as a vector states it.
struct EnumCarrier {
    /// The manifest type of the number a vector writes for a variant: the
    /// carrier the enum document declares.
    ty: CanonicalType,
    /// The name of the enum document, which names the type a language's fragment
    /// converts the number to and from.
    document: String,
    /// Whether the enum admits a carrier no variant declares.
    open: bool,
}

/// The carrier, as a manifest type, of each enum a codec document imports —
/// what a content-line entry typed `enum:<alias>` is in `decoded` (docs/adr/0015):
/// the number the enum document gives the variant. Read with the parse the
/// generator reads, so the harness and the product cannot name another width.
fn read_enum_carriers(
    scxml_path: &Path,
    fixture_name: &str,
) -> Result<std::collections::BTreeMap<String, EnumCarrier>, String> {
    use crate::forge::model::SceType;
    let text = crate::load_forge_source(scxml_path, &[])
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?
        .positions
        .expanded;
    let parsed = crate::forge::parser::parse_forge_with_imports(
        &text,
        crate::DocumentLabel::symmetric(fixture_name),
    )
    .map_err(|e| format!("{}: {e}", scxml_path.display()))?
    .ok_or_else(|| format!("{} is not a forge document", scxml_path.display()))?;
    let base_dir = scxml_path.parent().unwrap_or_else(|| Path::new("."));
    let mut carriers = std::collections::BTreeMap::new();
    for import in &parsed.imports {
        let Some(vocabulary) =
            crate::forge::import_source::enum_model(&parsed, base_dir, &import.alias)
        else {
            continue;
        };
        let ty = match vocabulary.underlying_type {
            SceType::Uint8 => CanonicalType::U8,
            SceType::Uint16 => CanonicalType::U16,
            SceType::Uint32 => CanonicalType::U32,
            SceType::Uint64 => CanonicalType::U64,
            other => {
                return Err(format!(
                    "fixture {fixture_name}: enum `{}` has the carrier {other:?}, which a \
                     content-line vector cannot write as a number",
                    import.alias
                ))
            }
        };
        carriers.insert(
            import.alias.clone(),
            EnumCarrier {
                ty,
                document: vocabulary.name.clone(),
                open: !vocabulary.strict_variants,
            },
        );
    }
    Ok(carriers)
}

/// Fill a codec fixture's derived shape from its document: whether it is CBOR
/// or content-line, and which listed fields are optional or lists. Every listed
/// field of such a codec must name one of its entries — a manifest field the
/// document does not declare would be asserted against a struct member that does
/// not exist, which the fragment would report as a compile error rather than as
/// the manifest's.
fn derive_codec_shape(
    scxml_path: &Path,
    fixture_name: &str,
    fields: &mut [StructField],
    cbor: &mut bool,
    content_line: &mut bool,
    encode_may_fail: &mut bool,
) -> Result<(), String> {
    let model = read_codec_model(scxml_path, fixture_name)?;
    *cbor = model.encoding == crate::forge::model::CodecEncoding::Cbor;
    *content_line = model.encoding == crate::forge::model::CodecEncoding::ContentLine;
    *encode_may_fail = model.encode_may_fail;
    let missing = |field: &StructField, what: &str| {
        format!(
            "fixture {fixture_name}: field `{}` is not an entry of the {what} codec {}",
            field.name,
            scxml_path.display()
        )
    };
    if *cbor {
        for field in fields.iter_mut() {
            let entry = model
                .cbor_entries
                .iter()
                .find(|e| e.id == field.name)
                .ok_or_else(|| missing(field, "CBOR"))?;
            field.optional = !entry.is_required();
            field.bounded = crate::forge::cbor_codec::rust_bound_args(entry).is_some();
        }
    }
    if !*cbor && !*content_line {
        for field in fields.iter_mut() {
            field.bounded = model
                .fields
                .iter()
                .find(|f| f.id == field.name)
                .is_some_and(|f| {
                    crate::forge::codec_totality::byte_bound(f, &model.fields).is_some()
                });
        }
        // The fields the encode derives whole (docs/adr/0081 §3), as its plan
        // says.
        let derived: Vec<String> = crate::encode_plan_file(
            scxml_path,
            &[],
            &crate::ForgeCompileOptions::default(),
            |plan| match plan {
                crate::forge::encode_plan::CodecPlan::Positional(plan) => plan
                    .derivations
                    .iter()
                    .filter_map(|d| match &d.element {
                        crate::forge::codec_roles::ElementId::Field(id) => Some(id.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
        )
        .map_err(|e| format!("{}: {e:?}", scxml_path.display()))?
        .unwrap_or_default();
        for field in fields.iter_mut() {
            field.derived = derived.contains(&field.name);
        }
    }
    if let Some(lines) = model.content_line.as_ref().filter(|_| *content_line) {
        let entries = &lines.entries;
        // A parameter of a repeated property that reads its lines as records is a
        // member of each record, not a member of the codec's struct, so it is never
        // a listed field (docs/adr/0014).
        let record_member_of = |entry: &crate::forge::model::ContentLineEntry| {
            entry.param.as_ref()?;
            entries.iter().find(|o| {
                o.param.is_none() && o.property.eq_ignore_ascii_case(&entry.property) && o.repeated
            })
        };
        // What a member of a record is in `decoded`: a string, or the carrier of
        // the enum it reads by text (docs/adr/0015).
        let carriers = read_enum_carriers(scxml_path, fixture_name)?;
        let enum_of = |entry: &crate::forge::model::ContentLineEntry| match entry.present_type() {
            crate::forge::model::SceType::Enum(eref) => {
                carriers.get(&eref.alias).map(Some).ok_or_else(|| {
                    format!(
                        "fixture {fixture_name}: entry `{}` imports no readable enum as `{}`",
                        entry.id, eref.alias
                    )
                })
            }
            _ => Ok(None),
        };
        let entry_type = |entry: &crate::forge::model::ContentLineEntry| {
            Ok::<_, String>(enum_of(entry)?.map_or(CanonicalType::String, |e| e.ty))
        };
        let entry_document = |entry: &crate::forge::model::ContentLineEntry| {
            Ok::<_, String>(enum_of(entry)?.map(|e| e.document.clone()))
        };
        let entry_open = |entry: &crate::forge::model::ContentLineEntry| {
            Ok::<_, String>(enum_of(entry)?.is_some_and(|e| e.open))
        };
        for field in fields.iter_mut() {
            let entry = entries
                .iter()
                .find(|e| e.id == field.name)
                .ok_or_else(|| missing(field, "content-line"))?;
            if let Some(owner) = record_member_of(entry) {
                return Err(format!(
                    "fixture {fixture_name}: field `{}` is a parameter of the lines of \
                     `{}`, a member of each of its records and not a field of the codec \
                     (docs/adr/0014); remove it from fixtures.json",
                    field.name, owner.id
                ));
            }
            // An enum entry is its carrier in `decoded`, so the manifest states the
            // carrier the enum document declares and no other width.
            if matches!(entry.present_type(), crate::forge::model::SceType::Enum(_))
                && field.ty != entry_type(entry)?
            {
                return Err(format!(
                    "fixture {fixture_name}: field `{}` reads an enum, whose carrier in \
                     `decoded` is {:?}, and the manifest says {:?}",
                    field.name,
                    entry_type(entry)?,
                    field.ty
                ));
            }
            field.enum_document = entry_document(entry)?;
            field.enum_open = entry_open(entry)?;
            field.list = entry.repeated || entry.separator.is_some();
            field.optional = !entry.is_required() && !field.list;
            field.members = if entry.repeated
                && (entry.separator.is_some()
                    || entries
                        .iter()
                        .any(|p| record_member_of(p).is_some_and(|o| o.id == entry.id)))
            {
                let mut members: Vec<StructField> = entries
                    .iter()
                    .filter(|p| record_member_of(p).is_some_and(|o| o.id == entry.id))
                    .map(|p| {
                        Ok(StructField {
                            name: p.id.clone(),
                            ty: entry_type(p)?,
                            compare: CompareMode::Equality,
                            optional: !p.is_required(),
                            bounded: false,
                            derived: false,
                            list: false,
                            enum_document: entry_document(p)?,
                            enum_open: entry_open(p)?,
                            members: Vec::new(),
                        })
                    })
                    .collect::<Result<_, String>>()?;
                let has_list = entry.separator.is_some();
                members.push(StructField {
                    name: if has_list { "values" } else { "value" }.to_owned(),
                    ty: entry_type(entry)?,
                    compare: CompareMode::Equality,
                    optional: false,
                    bounded: false,
                    derived: false,
                    list: has_list,
                    enum_document: entry_document(entry)?,
                    enum_open: entry_open(entry)?,
                    members: Vec::new(),
                });
                members
            } else {
                Vec::new()
            };
        }
        // The fields in the order the codec presents them: each property, then
        // each parameter of a property whose lines are no records. A backend that
        // makes a value by its constructor (C++, docs/adr/0081 step 4) takes
        // them in this order, so the manifest lists every one of them in it.
        let presented: Vec<&str> = entries
            .iter()
            .filter(|e| e.param.is_none())
            .flat_map(|property| {
                std::iter::once(property.id.as_str()).chain(
                    entries
                        .iter()
                        .filter(|p| {
                            p.param.is_some()
                                && p.property.eq_ignore_ascii_case(&property.property)
                                && record_member_of(p).is_none()
                        })
                        .map(|p| p.id.as_str()),
                )
            })
            .collect();
        let listed: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        if listed != presented {
            return Err(format!(
                "fixture {fixture_name}: the manifest lists the fields {listed:?}; a content-line \
                 codec presents {presented:?}, each property followed by its parameters, and the \
                 manifest lists every one of them in that order"
            ));
        }
    }
    Ok(())
}

/// The oracle section a transform with a holder is judged under.
const STATEFUL_TRANSFORMS: &str = "stateful_transforms";

/// The holder a transform fixture generates on `language`, if it has one —
/// asked of `forge::generator::transform_holder` with the same parse the
/// generator reads, not a scan of the XML, so the harness and the product
/// cannot answer it differently.
fn read_transform_holder(
    scxml_path: &Path,
    fixture_name: &str,
    language: Language,
) -> Result<Option<crate::forge::generator::TransformHolderSymbols>, String> {
    let text = crate::load_forge_source(scxml_path, &[])
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?
        .positions
        .expanded;
    match crate::forge::parser::parse_forge(&text, crate::DocumentLabel::symmetric(fixture_name))
        .map_err(|e| format!("{}: {e}", scxml_path.display()))?
    {
        Some(crate::forge::model::ForgeDocument::Transform(m)) => {
            Ok(crate::forge::generator::transform_holder(&m, language))
        }
        _ => Err(format!(
            "fixture {fixture_name}: the manifest says `transform`, and {} is not a \
             transform document",
            scxml_path.display()
        )),
    }
}

/// The fields of the record an event-schema fixture declares, as `language`
/// spells them — read from the document with the parse the generator reads,
/// so a record argument or output is built against the struct that is
/// actually emitted (SCE_FORGE.md §4.12).
///
/// The document is the event-schema fixture's own — `<record>.scxml`, or the
/// standard document its `document` names — so a record type the standard
/// library declares is read where every other reader of it reads it.
fn read_record_fields(
    manifest: &Manifest,
    resource_dir: &Path,
    record: &str,
    language: Language,
) -> Result<Vec<RecordFieldSpec>, String> {
    use crate::forge::model::SceType;
    let path = manifest
        .fixtures
        .iter()
        .find(|g| g.name == record && matches!(g.spec, FixtureSpec::EventSchema { .. }))
        .ok_or_else(|| format!("record `{record}` names no event-schema fixture in this manifest"))?
        .document_path(resource_dir);
    let (m, enums) = read_event_schema(&path, record)?;
    m.fields
        .iter()
        .map(|f| {
            // A field that may be absent (docs/adr/0055) has no canonical type a
            // harness builds yet.
            let Some(declared) = f.sce_type.scalar() else {
                return Err(format!(
                    "record `{record}`: field `{}` is {}, which the harness has no canonical \
                     type for",
                    f.id,
                    f.sce_type.as_attr()
                ));
            };
            // An enum field is the enum document's own type, which the harness
            // reaches through the enum fixture of this manifest that generates
            // it, and which the oracle writes by the name each variant declares.
            if let SceType::Enum(reference) = declared {
                let model = enums.get(&reference.alias).ok_or_else(|| {
                    format!(
                        "record `{record}`: field `{}` holds the enum `{}`, which the schema \
                         imports from no readable enum document",
                        f.id, reference.alias
                    )
                })?;
                let generated = manifest
                    .fixtures
                    .iter()
                    .any(|g| g.name == model.name && matches!(g.spec, FixtureSpec::Enum { .. }));
                if !generated {
                    return Err(format!(
                        "record `{record}`: field `{}` holds the enum `{}`, which names no enum \
                         fixture in this manifest",
                        f.id, model.name
                    ));
                }
                return Ok(RecordFieldSpec {
                    name: f.id.clone(),
                    ident: crate::forge::generator::event_schema_field_ident(&f.id, language),
                    ty: enum_carrier(record, &f.id, &model.underlying_type)?,
                    max_size: None,
                    enum_document: Some(model.name.clone()),
                    variants: enum_variants_of(model, language),
                });
            }
            let ty = match declared {
                SceType::Bool => CanonicalType::Bool,
                SceType::Uint8 => CanonicalType::U8,
                SceType::Uint16 => CanonicalType::U16,
                SceType::Uint32 => CanonicalType::U32,
                SceType::Uint64 => CanonicalType::U64,
                SceType::Int32 => CanonicalType::I32,
                SceType::Int64 => CanonicalType::I64,
                SceType::Float32 => CanonicalType::F32,
                SceType::Float64 => CanonicalType::F64,
                // A record's text is held within the bound its schema declares
                // (SCE_FORGE.md §4.12), which a harness builds and reads it
                // through, so a `string` field with none has nothing to be
                // built at.
                SceType::String if f.max_size.is_some() => CanonicalType::String,
                other => {
                    return Err(format!(
                        "record `{record}`: field `{}` is {}, which the harness has no \
                         canonical type for",
                        f.id,
                        other.as_attr()
                    ))
                }
            };
            Ok(RecordFieldSpec {
                name: f.id.clone(),
                ident: crate::forge::generator::event_schema_field_ident(&f.id, language),
                ty,
                max_size: match ty {
                    CanonicalType::String => f.max_size,
                    _ => None,
                },
                enum_document: None,
                variants: Vec::new(),
            })
        })
        .collect()
}

/// The canonical type of the carrier an enum's values are held in, which a record
/// field of the enum declares as its `type`: the harness builds and compares the
/// field by its variants, and the carrier is what the enum document says it is.
fn enum_carrier(
    record: &str,
    field: &str,
    underlying: &crate::forge::model::SceType,
) -> Result<CanonicalType, String> {
    use crate::forge::model::SceType;
    Ok(match underlying {
        SceType::Uint8 => CanonicalType::U8,
        SceType::Uint16 => CanonicalType::U16,
        SceType::Uint32 => CanonicalType::U32,
        SceType::Uint64 => CanonicalType::U64,
        other => {
            return Err(format!(
                "record `{record}`: field `{field}` holds an enum carried as {}, which the \
                 harness has no canonical type for",
                other.as_attr()
            ))
        }
    })
}

/// Each variant of the enum `model`, by the name its document declares and the
/// member this language's declaration emits for it
/// ([`crate::forge::enum_naming::variant_ident`]): what a fragment builds a value
/// of the enum from, a record's field or an argument alike.
fn enum_variants_of(
    model: &crate::forge::model::EnumModel,
    language: Language,
) -> Vec<RecordEnumVariant> {
    model
        .variants
        .iter()
        .map(|v| RecordEnumVariant {
            name: v.name.clone(),
            ident: crate::forge::enum_naming::variant_ident(language, &model.name, &v.name),
        })
        .collect()
}

/// The variants of the enum an algorithm's argument is a value of, read from the
/// document of the enum fixture `enum_fixture` names with the parse the generator
/// reads, so the members a fragment names are the ones the declaration emits.
fn read_enum_arg_variants(
    manifest: &Manifest,
    resource_dir: &Path,
    enum_fixture: &str,
    language: Language,
) -> Result<Vec<RecordEnumVariant>, String> {
    let path = manifest
        .fixtures
        .iter()
        .find(|g| g.name == enum_fixture && matches!(g.spec, FixtureSpec::Enum { .. }))
        .ok_or_else(|| format!("enum `{enum_fixture}` names no enum fixture in this manifest"))?
        .document_path(resource_dir);
    let text = crate::load_forge_source(&path, &[])
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?
        .positions
        .expanded;
    match crate::forge::parser::parse_forge(&text, crate::DocumentLabel::symmetric(enum_fixture))
        .map_err(|e| format!("{}: {e}", path.display()))?
    {
        Some(crate::forge::model::ForgeDocument::Enum(model)) => {
            Ok(enum_variants_of(&model, language))
        }
        _ => Err(format!(
            "enum `{enum_fixture}`: {} is not an enum document",
            path.display()
        )),
    }
}

/// The union the union fixture `union_fixture` of `manifest` generates, as
/// `language` spells it (docs/adr/0092, sections 1.1 and 7): the key its tag is
/// written under and the member a value answers it by, and one case per variant
/// of the tag's enum, in the enum's declaration order, each by the identifier
/// its unit declares, the member of the tag's enum it is for and the record it
/// holds, whose fields are derived as a record argument's are. Read from the
/// union's document with the parse the generator reads, so a fragment names no
/// case, enum or record the units do not declare; the tag's enum and every
/// case's record are fixtures of the manifest, generated beside the union's unit,
/// which names them as siblings.
fn read_union_shape(
    manifest: &Manifest,
    resource_dir: &Path,
    union_fixture: &str,
    language: Language,
) -> Result<UnionShape, String> {
    use crate::forge::model::{ForgeDocument, ForgeKind, ParsedForge};
    let path = manifest
        .fixtures
        .iter()
        .find(|g| g.name == union_fixture && matches!(g.spec, FixtureSpec::Union { .. }))
        .ok_or_else(|| format!("union `{union_fixture}` names no union fixture in this manifest"))?
        .document_path(resource_dir);
    let text = crate::load_forge_source(&path, &[])
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?
        .positions
        .expanded;
    let parsed = crate::forge::parser::parse_forge_with_imports(
        &text,
        crate::DocumentLabel::symmetric(union_fixture),
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    let Some(ParsedForge {
        document: ForgeDocument::Union(model),
        imports,
        ..
    }) = parsed
    else {
        return Err(format!(
            "union `{union_fixture}`: {} is not a union document",
            path.display()
        ));
    };
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tag_enum = crate::forge::import_source::enum_model_in(&imports, base_dir, &model.tag_enum)
        .ok_or_else(|| {
            format!(
                "union `{union_fixture}`: the tag's enum `{}` is no readable enum the document \
                 imports",
                model.tag_enum
            )
        })?;
    let generated = manifest
        .fixtures
        .iter()
        .any(|g| g.name == tag_enum.name && matches!(g.spec, FixtureSpec::Enum { .. }));
    if !generated {
        return Err(format!(
            "union `{union_fixture}`: the tag's enum `{}` names no enum fixture in this manifest",
            tag_enum.name
        ));
    }
    let mut cases = Vec::with_capacity(tag_enum.variants.len());
    for variant in &tag_enum.variants {
        let case = model.case(&variant.name).ok_or_else(|| {
            format!(
                "union `{union_fixture}`: the variant `{}` of its tag has no case",
                variant.name
            )
        })?;
        let record = match &case.record {
            None => None,
            Some(alias) => {
                let import = imports
                    .iter()
                    .find(|i| i.kind == ForgeKind::EventSchema && i.alias == *alias)
                    .ok_or_else(|| {
                        format!(
                            "union `{union_fixture}`: the case `{}` holds `{alias}`, which names \
                             no event schema the document imports",
                            variant.name
                        )
                    })?;
                let Some(ForgeDocument::EventSchema(schema)) =
                    crate::forge::import_source::parse_quietly(base_dir, import)
                        .map(|p| p.document)
                else {
                    return Err(format!(
                        "union `{union_fixture}`: the case `{}` holds `{alias}`, whose document \
                         is no readable event schema",
                        variant.name
                    ));
                };
                Some(RecordRef {
                    fields: read_record_fields(manifest, resource_dir, &schema.name, language)?,
                    record: schema.name,
                })
            }
        };
        cases.push(UnionCaseSpec {
            ident: union_case_ident(language, &tag_enum.name, &variant.name),
            kind: crate::forge::enum_naming::variant_ident(language, &tag_enum.name, &variant.name),
            variant: variant.name.clone(),
            record,
        });
    }
    Ok(UnionShape {
        document: union_fixture.to_string(),
        // C11 names a union's unit by the typedef its header declares; every
        // other language by the type its module, package or header declares,
        // which the fragment qualifies as it qualifies any type of a fixture.
        type_name: match language {
            Language::C11 => crate::forge::generator::union_unit_type(&model.name, language),
            _ => crate::filters::to_pascal_case(model.name.clone()),
        },
        tag_member: union_tag_member(language, &model.tag)
            .map_err(|why| format!("union `{union_fixture}`: {why}"))?,
        tag: model.tag,
        tag_enum: tag_enum.name,
        cases_member: matches!(language, Language::C11)
            .then(|| crate::forge::composed_lowering::C11_UNION_CASES.to_string()),
        cases,
    })
}

/// The identifier of the case `variant` of a union whose tag is a value of the
/// enum `tag_document`, as `language`'s unit of the union declares the case
/// (docs/adr/0092, section 7 item 1): the one helper each union renderer spells
/// it by. Go and Python name a case's constructor and class after the union's
/// type, which the fragment writes before it, as their units do.
fn union_case_ident(language: Language, tag_document: &str, variant: &str) -> String {
    use crate::forge::composed_lowering as spelling;
    match language {
        Language::Rust => spelling::rust_case_ident(variant),
        Language::Kotlin => crate::forge::generator::kotlin_case_ident(variant),
        Language::Swift => spelling::swift_case_ident(tag_document, variant),
        Language::Go => spelling::go_case_ident(variant),
        Language::Cpp => spelling::cpp_case_ident(variant),
        Language::Python => spelling::python_case_ident(variant),
        Language::C11 => spelling::c11_case_ident(variant),
    }
}

/// The member every value of `language`'s unit of a union answers its tag by,
/// whatever key `tag` the tag is written under (docs/adr/0092, section 7): a
/// method on Rust, Go and C++, a property on Kotlin and Swift, a class
/// attribute on Python and the struct's member on C11.
fn union_tag_member(language: Language, tag: &str) -> Result<String, String> {
    use crate::forge::composed_lowering as spelling;
    Ok(match language {
        Language::Rust => spelling::rust_union_tag_method(tag).ok_or_else(|| {
            format!("the tag's key `{tag}` names no Rust method its value can be read by")
        })?,
        Language::Kotlin => spelling::KOTLIN_UNION_KIND.to_string(),
        Language::Swift => spelling::SWIFT_UNION_KIND.to_string(),
        Language::Go => spelling::GO_UNION_KIND.to_string(),
        Language::Cpp => spelling::CPP_UNION_KIND.to_string(),
        Language::Python => spelling::PYTHON_UNION_KIND.to_string(),
        Language::C11 => spelling::C11_UNION_TAG.to_string(),
    })
}

/// An event-schema document, and each enum it imports by the alias it imports it
/// under ([`read_event_schema`]).
type EventSchemaAndEnums = (
    crate::forge::model::EntrySchema,
    std::collections::BTreeMap<String, crate::forge::model::EnumModel>,
);

/// The event-schema document at `path`, and each enum it imports by the alias it
/// imports it under, read from the schema's own directory as every reader of the
/// schema reads it. `label` names the document in a refusal.
fn read_event_schema(path: &Path, label: &str) -> Result<EventSchemaAndEnums, String> {
    use crate::forge::model::{ForgeDocument, ForgeKind};
    let text = crate::load_forge_source(path, &[])
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?
        .positions
        .expanded;
    let parsed = crate::forge::parser::parse_forge_with_imports(
        &text,
        crate::DocumentLabel::symmetric(label),
    )
    .map_err(|e| format!("{}: {e}", path.display()))?;
    let Some(crate::forge::model::ParsedForge {
        document: ForgeDocument::EventSchema(model),
        imports,
        ..
    }) = parsed
    else {
        return Err(format!(
            "record `{label}`: {} is not an event-schema document",
            path.display()
        ));
    };
    // The harness builds and compares a record one field at a time, each one
    // value: a field that is a list, a record or an optional over either is
    // refused by name.
    let model = model
        .entry_schema()
        .map_err(|composed| composed.sentence(label, "the conformance harness"))?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut enums = std::collections::BTreeMap::new();
    for import in imports.iter().filter(|i| i.kind == ForgeKind::Enum) {
        if let Some(ForgeDocument::Enum(enum_model)) =
            crate::forge::import_source::parse_quietly(base_dir, import).map(|p| p.document)
        {
            enums.insert(import.alias.clone(), enum_model);
        }
    }
    Ok((model, enums))
}

fn read_validator_has_state(scxml_path: &Path) -> Result<bool, String> {
    let text = read_document(scxml_path)
        .map_err(|e| format!("cannot read {}: {e}", scxml_path.display()))?;
    let doc = roxmltree::Document::parse(&text)
        .map_err(|e| format!("invalid SCXML at {}: {e}", scxml_path.display()))?;
    let scxml_ns = "http://www.w3.org/2005/07/scxml";
    let sce_ns = crate::forge::model::SCE_NAMESPACE;
    // Trigger 1: any ROC field — `sce:max-delta` requires prev-value
    // tracking which lives in the validator state struct. Trigger 2: any
    // `<sce:import>` whose kind is stateful (codec / filter / observer /
    // validator / procedure / timer) — the imported instance becomes a
    // member of the validator state struct so the C11 emitter must use
    // the stateful `<state_t> *_st`-passing signature instead of the
    // stateless free-function form. Both triggers must mirror the
    // `c_has_state` decision in `render_validator` (generator.rs); when
    // the two diverge the harness emits a call signature that doesn't
    // match the generated function — typically "too few arguments to
    // crossfile_validator_codec_validate".
    let Some(scxml_root) = doc
        .root()
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "scxml")
    else {
        return Ok(false);
    };
    for child in scxml_root.children() {
        if !child.is_element() {
            continue;
        }
        let tag = child.tag_name().name();
        let ns = child.tag_name().namespace();
        if tag == "import" && ns == Some(sce_ns) {
            if let Some(kind) = child.attribute("kind") {
                let stateful = matches!(
                    kind,
                    "codec" | "filter" | "observer" | "validator" | "procedure" | "timer"
                );
                if stateful {
                    return Ok(true);
                }
            }
        }
        if tag == "datamodel" && ns == Some(scxml_ns) {
            for data in child
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() == "data")
            {
                if data.attribute((sce_ns, "max-delta")).is_some() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// A rendered conformance harness and the inputs its render consumed.
///
/// `extra_inputs` holds the files read beyond the manifest, the fixture
/// documents under `resource_dir` and the template tree — the three a
/// caller can enumerate for itself. Today that is
/// `conformance/numerical_reference.json`, read only on the C11 route,
/// which bakes the oracle into the harness source because the backend's
/// zero-dependency rule leaves no runtime JSON parser. It sits outside
/// `resource_dir`, so the synth-6.2.6 source set does not cover it, and
/// editing it changes the emitted harness.
///
/// Carried on the result rather than re-derived by the caller so the
/// build's prerequisite list and the render agree by construction. The
/// C11 CMake step used to name the file in `DEPENDS` from its own
/// knowledge of the C11 route — a second statement of the same fact,
/// free to survive the route changing under it.
pub struct RenderedHarness {
    /// The generated harness source.
    pub source: String,
    /// Files this render read that the caller could not have named.
    pub extra_inputs: Vec<PathBuf>,
}

/// RFC §synth-5-B — fold a codec fixture's decode-rejection vectors out of
/// the oracle and into the value the per-language fragment renders from.
///
/// A reject vector is wire the codec MUST refuse, paired with the failure it
/// must refuse with:
///
/// ```json
/// "rejects": [
///   {
///     "why": "the chain saturates max-depth with the entry flag still set",
///     "encoded": [0, 129, 130, 3],
///     "error": "too-many-elements"
///   }
/// ]
/// ```
///
/// These carry no field values, so — unlike the round-trip `cases` — they do
/// not depend on the fixture's field shape being expressible in the
/// `StructField` / `CanonicalType` schema. That matters: every tlv-chain
/// fixture in the catalog is `oracle_eligible: false`, so before reject
/// vectors the harness could only prove those codecs *compile*.
///
/// Each vector gains a `symbol` key naming the backend's spelling of the
/// failure, absent on backends where a refused decode is observable but its
/// reason is not (see [`CodecFailure::observable_symbol`]). The fragment
/// asserts against the symbol when it is there and against refusal when it
/// is not.
///
/// Answers whether anything was folded, so the caller can declare the
/// oracle as a prerequisite exactly when this render baked something out
/// of it — see the `baked_from_reference` comment at the call site.
fn fold_reject_vectors(
    value: &mut serde_json::Value,
    reference: &serde_json::Value,
    fixture: &Fixture,
    language: Language,
    reference_path: &Path,
) -> Result<bool, String> {
    let Some(rejects) = reference
        .get(&fixture.ref_section)
        .and_then(|s| s.get(&fixture.name))
        .and_then(|e| e.get("rejects"))
    else {
        return Ok(false);
    };
    let where_ = format!("fixture {} in {}", fixture.name, reference_path.display());
    if !matches!(fixture.spec, FixtureSpec::Codec { .. }) {
        return Err(format!(
            "{where_}: 'rejects' is a codec-kind contract (wire the decoder must refuse); \
             this fixture is not a codec"
        ));
    }
    let rows = rejects
        .as_array()
        .ok_or_else(|| format!("{where_}: 'rejects' must be an array"))?;
    if rows.is_empty() {
        return Err(format!(
            "{where_}: 'rejects' is present but empty — drop the key rather than \
             declaring a contract with no cases in it"
        ));
    }

    let mut folded = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let at = format!("{where_}: rejects[{i}]");
        let obj = row
            .as_object()
            .ok_or_else(|| format!("{at} must be an object"))?;
        let why = obj
            .get("why")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                format!("{at} must carry a non-empty 'why' — the assertion message a failing run prints")
            })?;
        // `why` is inlined into a source-level string literal in six
        // languages, one of which (C) passes it to `fprintf` as the format
        // string. Keep it to characters that mean themselves everywhere.
        if let Some(bad) = why.chars().find(|c| "\"\\%".contains(*c) || c.is_control()) {
            return Err(format!(
                "{at}: 'why' contains {bad:?}, which does not survive being inlined as a \
                 string literal in every backend (C passes it to fprintf as the format string)"
            ));
        }
        let encoded = obj
            .get("encoded")
            .and_then(|v| v.as_array())
            .filter(|a| !a.is_empty())
            .ok_or_else(|| format!("{at} must carry a non-empty 'encoded' byte array"))?;
        for (b, byte) in encoded.iter().enumerate() {
            match byte.as_u64() {
                Some(v) if v <= 0xFF => {}
                _ => {
                    return Err(format!("{at}: encoded[{b}] is not a byte in 0..=255"));
                }
            }
        }
        let error_name = obj
            .get("error")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{at} must name the failure it expects in 'error'"))?;
        let failure = CodecFailure::parse(error_name).map_err(|e| format!("{at}: {e}"))?;
        // Only a CBOR map reader can refuse with a CBOR failure; naming one
        // on a positional codec is a vector no decode could ever satisfy.
        let codec_is_cbor = matches!(fixture.spec, FixtureSpec::Codec { cbor: true, .. });
        if failure.is_cbor() && !codec_is_cbor {
            return Err(format!(
                "{at}: '{error_name}' is a CBOR map failure, and {} is not an \
                 sce:encoding=\"cbor\" codec",
                fixture.name
            ));
        }
        // The same for a content-line reader's failures, and the other way
        // round: a positional or CBOR failure on a content-line codec is a
        // vector its decode could never satisfy.
        let codec_is_content_line = matches!(
            fixture.spec,
            FixtureSpec::Codec {
                content_line: true,
                ..
            }
        );
        if failure.is_content_line() && !codec_is_content_line {
            return Err(format!(
                "{at}: '{error_name}' is a content-line failure, and {} is not an \
                 sce:encoding=\"content-line\" codec",
                fixture.name
            ));
        }
        if codec_is_content_line
            && !failure.is_content_line()
            && failure != CodecFailure::NeedMoreBytes
        {
            return Err(format!(
                "{at}: '{error_name}' is not a failure a content-line codec raises"
            ));
        }

        let mut out = serde_json::Map::new();
        out.insert("why".into(), serde_json::Value::from(why));
        out.insert("encoded".into(), serde_json::Value::Array(encoded.clone()));
        out.insert("error".into(), serde_json::Value::from(failure.wire_name()));
        if let Some(symbol) = failure.observable_symbol(language) {
            out.insert("symbol".into(), serde_json::Value::from(symbol));
        }
        // Where the decode answers one sentinel and keeps the reason on the
        // cursor (C++, Kotlin), the vector also asserts the record: the
        // reason for a bound's refusal (docs/adr/0077 §4), and none for a
        // frame short of bytes, so the two are never mistaken for each other.
        if let Some(record) = failure.cursor_record(language) {
            out.insert("cursor_record".into(), serde_json::Value::from(record));
        }
        if failure.records_nothing(language) {
            out.insert(
                "cursor_records_nothing".into(),
                serde_json::Value::Bool(true),
            );
        }
        folded.push(serde_json::Value::Object(out));
    }

    value
        .as_object_mut()
        .ok_or_else(|| format!("{where_} did not serialize to a JSON object"))?
        .insert("rejects".into(), serde_json::Value::Array(folded));
    Ok(true)
}

/// The module of the Rust harness that holds every fixture's module, as
/// `forge/rust/conformance/harness.rs.jinja2` declares it: a byte case names
/// the codecs it holds from there.
const HARNESS_FIXTURES: &str = "fixtures";

/// The codecs a codec imports, read as the codec was, for a byte case of a
/// codec that holds others (docs/adr/0081 §5); and the enums it imports, for a
/// case that states a value of one.
struct HeldFrom<'a> {
    imported: &'a crate::Imported,
    include: &'a [PathBuf],
}

impl crate::forge::generator::RustHeldCodecs for HeldFrom<'_> {
    fn read(
        &mut self,
        alias: &str,
        read: &mut crate::forge::generator::RustReadHeld<'_>,
    ) -> Result<String, String> {
        let from = self
            .imported
            .codecs
            .iter()
            .find(|i| i.alias == alias)
            .ok_or_else(|| format!("no codec is imported as `{alias}`"))?;
        let include = self.include;
        crate::encode_plan_file_with_imports(
            &from.path,
            include,
            &crate::ForgeCompileOptions::default(),
            |plan, imported| {
                let crate::forge::encode_plan::CodecPlan::Positional(plan) = plan else {
                    return Err(format!("`{alias}` is not a positional codec"));
                };
                let path = crate::forge::generator::RustCodecPath {
                    module: format!("{HARNESS_FIXTURES}::{}", from.module),
                    type_name: from.type_name.clone(),
                };
                read(plan, &path, &mut HeldFrom { imported, include })
            },
        )
        .map_err(|e| format!("{}: {e:?}", from.path.display()))?
        .ok_or_else(|| format!("{} is not a codec", from.path.display()))?
    }

    fn held_enum(&self, alias: &str) -> Result<crate::forge::generator::RustHeldEnum, String> {
        let from = self
            .imported
            .enums
            .iter()
            .find(|i| i.alias == alias)
            .ok_or_else(|| format!("no enum is imported as `{alias}`"))?;
        // The enum is a fixture of its own, a sibling of its importer under
        // the harness's module of fixtures.
        Ok(crate::forge::generator::RustHeldEnum {
            name: from.name.clone(),
            rust_type: format!("{HARNESS_FIXTURES}::{}", from.rust_type),
            variants: from.variants.clone(),
        })
    }
}

/// Fold a codec fixture's byte cases stated as its interface presents the value
/// (docs/adr/0081 §5) into the value the fragment renders from, for a backend
/// whose encode prints the codec's plan: each `source` becomes a literal of the
/// backend's ready-made source, after the bindings of the bodies and entries it
/// holds, with an expression for each element that holds when a decode of the
/// bytes presents what the literal does. Rust prints the plan as `{Codec}Parts`
/// (docs/adr/0058 §4), and C++ as the printed value its constructor makes
/// (docs/adr/0081 step 4); the others are handed nothing, and run their `cases`
/// as before.
///
/// Answers whether anything was folded, as [`fold_reject_vectors`] does.
fn fold_source_cases(
    value: &mut serde_json::Value,
    reference: &serde_json::Value,
    fixture: &Fixture,
    language: Language,
    resource_dir: &Path,
    reference_path: &Path,
) -> Result<bool, String> {
    let Some(sources) = reference
        .get(&fixture.ref_section)
        .and_then(|s| s.get(&fixture.name))
        .and_then(|e| e.get("sources"))
    else {
        return Ok(false);
    };
    let where_ = format!("fixture {} in {}", fixture.name, reference_path.display());
    let (rows, cpp_printed) = codec_case_rows(fixture, sources, "sources", &where_)?;
    let Some(printer) = CasePrinter::of(cpp_printed, language, "sources", &where_)? else {
        return Ok(false);
    };
    let folded = fold_case_rows(
        fixture,
        printer,
        rows,
        "sources",
        &where_,
        resource_dir,
        |_, at, row, case| {
            let encoded = row
                .get("encoded")
                .and_then(|v| v.as_array())
                .ok_or_else(|| format!("{at} must carry an 'encoded' byte array"))?;
            if encoded.iter().any(|b| b.as_u64().is_none_or(|b| b > 0xFF)) {
                return Err(format!("{at}: 'encoded' holds a value that is not a byte"));
            }
            let checks: Vec<serde_json::Value> = case
                .checks
                .into_iter()
                .map(|(element, holds)| serde_json::json!({ "element": element, "holds": holds }))
                .collect();
            Ok(serde_json::json!({
                "bindings": case.bindings,
                "literal": case.literal,
                "checks": checks,
                "inputs": case.inputs,
                "encode_inputs": case.encode_inputs,
                "encoded": encoded,
                "note": case.note,
            }))
        },
    )?;
    value
        .as_object_mut()
        .ok_or_else(|| format!("{where_} did not serialize to a JSON object"))?
        .insert("sources".into(), serde_json::Value::Array(folded));
    Ok(true)
}

/// Fold a codec fixture's refused cases (docs/adr/0081 §5) into the value the
/// fragment renders from, for a backend whose encode prints the codec's plan:
/// each `source` is a value of the codec's interface its encode refuses, made
/// as [`fold_source_cases`] makes a source, and its `error` the refusal the
/// encode answers it with (docs/adr/0065 §2), spelled as the backend's runtime
/// names it. The harness encodes the value and holds the encode to answering
/// exactly that refusal and to leaving the sink empty. The stated refusal must
/// be one the codec's encode can answer, and one the backend's runtime names:
/// a case a backend cannot state is refused by name, never left out.
///
/// Answers whether anything was folded, as [`fold_reject_vectors`] does.
fn fold_refused_cases(
    value: &mut serde_json::Value,
    reference: &serde_json::Value,
    fixture: &Fixture,
    language: Language,
    resource_dir: &Path,
    reference_path: &Path,
) -> Result<bool, String> {
    let Some(refused) = reference
        .get(&fixture.ref_section)
        .and_then(|s| s.get(&fixture.name))
        .and_then(|e| e.get("refused"))
    else {
        return Ok(false);
    };
    let where_ = format!("fixture {} in {}", fixture.name, reference_path.display());
    let (rows, cpp_printed) = codec_case_rows(fixture, refused, "refused", &where_)?;
    let Some(printer) = CasePrinter::of(cpp_printed, language, "refused", &where_)? else {
        return Ok(false);
    };
    let folded = fold_case_rows(
        fixture,
        printer,
        rows,
        "refused",
        &where_,
        resource_dir,
        |plan, at, row, case| {
            let error = row
                .get("error")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("{at} must name the refusal it expects in 'error'"))?;
            let failure = EncodeFailure::parse(error).map_err(|e| format!("{at}: {e}"))?;
            // The encode answers its own refusals, and a held codec's, which
            // may be of any kind.
            let answers = plan.totality.refusals().iter().any(|r| {
                r.kind == failure.refusal_kind()
                    || r.kind == crate::forge::codec_totality::RefusalKind::Child
            });
            if !answers {
                return Err(format!(
                    "{at}: the encode of {} never answers '{error}' (docs/adr/0065 §2): no \
                     value of it is refused with that",
                    fixture.name
                ));
            }
            let symbol = failure.symbol(printer.language()).ok_or_else(|| {
                format!(
                    "{at}: the {:?} runtime names no '{error}', so its harness cannot state \
                     this case",
                    printer.language()
                )
            })?;
            Ok(serde_json::json!({
                "bindings": case.bindings,
                "literal": case.literal,
                "encode_inputs": case.encode_inputs,
                "error": failure.wire_name(),
                "symbol": symbol,
                "note": case.note,
            }))
        },
    )?;
    value
        .as_object_mut()
        .ok_or_else(|| format!("{where_} did not serialize to a JSON object"))?
        .insert("refused".into(), serde_json::Value::Array(folded));
    Ok(true)
}

/// The rows of a codec fixture's byte cases stated under `key` (`sources`,
/// `refused`), and whether the C++ backend prints the codec from its plan.
/// Cases of a fixture that is not a codec, and a key with no case, are
/// authoring mistakes.
fn codec_case_rows<'r>(
    fixture: &Fixture,
    cases: &'r serde_json::Value,
    key: &str,
    where_: &str,
) -> Result<(&'r [serde_json::Value], bool), String> {
    let FixtureSpec::Codec { cpp_printed, .. } = &fixture.spec else {
        return Err(format!(
            "{where_}: '{key}' are a codec's byte cases; this fixture is not a codec"
        ));
    };
    let rows = cases
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or_else(|| format!("{where_}: '{key}' must be a non-empty array"))?;
    Ok((rows.as_slice(), *cpp_printed))
}

/// The printer that writes a codec's byte cases into a harness: the Rust one,
/// whose value is `{Codec}Parts` (docs/adr/0058 §4), or the C++ one, whose
/// value its constructor makes (docs/adr/0081 step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CasePrinter {
    Rust,
    Cpp,
}

impl CasePrinter {
    /// The printer of `language`'s harness, or `None` for a backend whose
    /// encode does not print the plan, which runs its `cases` as before. A
    /// codec the C++ backend does not print (`cpp_printed`) is refused by
    /// name: a printer runs every case of every shape it prints
    /// (docs/adr/0081 §5), and a codec the C++ backend refuses has no value a
    /// constructor makes from the case.
    fn of(
        cpp_printed: bool,
        language: Language,
        key: &str,
        where_: &str,
    ) -> Result<Option<Self>, String> {
        Ok(match language {
            Language::Rust => Some(CasePrinter::Rust),
            Language::Cpp if cpp_printed => Some(CasePrinter::Cpp),
            Language::Cpp => {
                return Err(format!(
                    "{where_}: the C++ backend refuses this codec (docs/adr/0081 step 4), \
                     so no value of it is made from a case; a codec with '{key}' is one it \
                     prints from its plan"
                ))
            }
            // Their encodes do not print the plan: they run the `cases`.
            Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11
            | Language::Swift => None,
        })
    }

    /// The backend whose harness this printer writes.
    fn language(self) -> Language {
        match self {
            CasePrinter::Rust => Language::Rust,
            CasePrinter::Cpp => Language::Cpp,
        }
    }
}

/// One byte case of a codec as a printer spells it for its harness.
struct PrintedCase {
    /// The declarations the literal reads, in order.
    bindings: Vec<String>,
    /// The value the case states.
    literal: String,
    /// Each element, group or input by its label, with an expression that holds
    /// when a decode of the bytes presents what the value does.
    checks: Vec<(String, String)>,
    /// The flag inputs a decode is handed, as its arguments.
    inputs: Vec<String>,
    /// The flag inputs the encode is handed, those the value does not say, as
    /// its arguments after the sink.
    encode_inputs: Vec<String>,
    /// The case's note, as the harness inlines it into a string literal.
    note: String,
}

/// Each row of a codec fixture's byte cases under `key`, its `source` made into
/// the value `printer` spells (after the bindings it reads, with its checks and
/// inputs), read from the codec's plan, and handed with its row and its place
/// (`at`) to `fold_row`, which answers what the fragment renders from. The one
/// reading of a case's value, note and inputs that every kind of byte case
/// shares.
fn fold_case_rows(
    fixture: &Fixture,
    printer: CasePrinter,
    rows: &[serde_json::Value],
    key: &str,
    where_: &str,
    resource_dir: &Path,
    mut fold_row: impl FnMut(
        &crate::forge::encode_plan::EncodePlan<'_>,
        &str,
        &serde_json::Value,
        PrintedCase,
    ) -> Result<serde_json::Value, String>,
) -> Result<Vec<serde_json::Value>, String> {
    let document = fixture.document_path(resource_dir);
    let include = [resource_dir.to_path_buf()];
    crate::encode_plan_file_with_imports(
        &document,
        &include,
        &crate::ForgeCompileOptions::default(),
        |plan, imported| -> Result<Vec<serde_json::Value>, String> {
            let crate::forge::encode_plan::CodecPlan::Positional(plan) = plan else {
                return Err(format!(
                    "{where_}: '{key}' are written for a positional codec"
                ));
            };
            let type_name = crate::filters::to_pascal_case(fixture.name.clone());
            let rust_path = crate::forge::generator::RustCodecPath {
                module: format!("{HARNESS_FIXTURES}::{}", fixture.name),
                type_name: type_name.clone(),
            };
            let cpp_path = crate::forge::generator::CppCodecPath::new(type_name);
            rows.iter()
                .enumerate()
                .map(|(i, row)| {
                    let at = format!("{where_}: {key}[{i}]");
                    let source = row
                        .get("source")
                        .ok_or_else(|| format!("{at} must carry 'source'"))?;
                    let note = row.get("note").and_then(|v| v.as_str()).unwrap_or_default();
                    // The note is inlined into a string literal of the harness.
                    if note.chars().any(|c| "\"\\{}".contains(c) || c.is_control()) {
                        return Err(format!(
                            "{at}: 'note' holds a character a literal does not keep"
                        ));
                    }
                    let mut held = HeldFrom {
                        imported,
                        include: &include,
                    };
                    let inputs = row.get("inputs").unwrap_or(&serde_json::Value::Null);
                    let case = match printer {
                        CasePrinter::Cpp => {
                            let case = crate::forge::generator::cpp_source_case(
                                plan, &cpp_path, source, inputs, &mut held,
                            )
                            .map_err(|e| format!("{at}: {e}"))?;
                            // The C++ fragment hands the note and each label to
                            // a `printf` as a string literal's text.
                            PrintedCase {
                                bindings: case.bindings,
                                literal: case.literal,
                                checks: case
                                    .checks
                                    .into_iter()
                                    .map(|(element, holds)| {
                                        (crate::filters::escape_cpp(element), holds)
                                    })
                                    .collect(),
                                inputs: case.inputs,
                                encode_inputs: case.encode_inputs,
                                note: crate::filters::escape_cpp(note.to_string()),
                            }
                        }
                        CasePrinter::Rust => {
                            let case = crate::forge::generator::rust_source_case(
                                plan, &rust_path, source, inputs, &mut held,
                            )
                            .map_err(|e| format!("{at}: {e}"))?;
                            PrintedCase {
                                bindings: case.bindings,
                                literal: case.literal,
                                checks: case.checks,
                                inputs: case.inputs,
                                encode_inputs: case.encode_inputs,
                                note: note.to_string(),
                            }
                        }
                    };
                    fold_row(plan, &at, row, case)
                })
                .collect()
        },
    )
    .map_err(|e| format!("{where_}: {}: {e:?}", document.display()))?
    .ok_or_else(|| format!("{where_}: {} is not a codec", document.display()))?
}

/// Put on each variant of an enum fixture the identifier this language's
/// declaration emits for it, as `ident`, taken from
/// [`crate::forge::enum_naming::variant_ident`].
///
/// The manifest keeps the SOURCE spelling, and that stays right: one
/// manifest serves six arms whose conventions disagree. What a fragment must
/// not do is turn that spelling into its language's identifier with its own
/// case filters, because the declaration already has one owner for that and
/// a fragment that re-derives it is a second copy that learns nothing when
/// the owner changes. The C arm's copy had already drifted — it upper-cased
/// the type prefix where the owner snake-cases it — and agreed only because
/// every fixture name so far is snake case.
fn fold_enum_identifiers(
    value: &mut serde_json::Value,
    fixture: &Fixture,
    language: Language,
) -> Result<(), String> {
    let FixtureSpec::Enum { variants } = &fixture.spec else {
        return Ok(());
    };
    // The prefix of C11's free functions for this enum, from the same owner
    // the declaration reads it from.
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "c11_prefix".into(),
            crate::forge::enum_naming::c11_function_prefix(&fixture.name).into(),
        );
    }
    let rows = value
        .get_mut("variants")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            format!(
                "fixture {}: an enum fixture did not serialize its 'variants' as an array",
                fixture.name
            )
        })?;
    for (row, variant) in rows.iter_mut().zip(variants) {
        let ident =
            crate::forge::enum_naming::variant_ident(language, &fixture.name, &variant.name);
        row.as_object_mut()
            .ok_or_else(|| {
                format!(
                    "fixture {}: variant '{}' did not serialize to a JSON object",
                    fixture.name, variant.name
                )
            })?
            .insert("ident".into(), serde_json::Value::from(ident));
    }
    Ok(())
}

/// Put on each enum field of an event-schema fixture the enum document it holds a
/// value of, `enum_document`, and the member this language's declaration of that
/// enum emits for the variant the field's value names, `ident` — what a fragment
/// builds the payload's field from and compares it with. The manifest writes such
/// a field as `sce_type: "enum:<alias>"` and its value as the variant's declared
/// name; the alias is read from the schema document's own imports.
fn fold_event_schema_enum_fields(
    value: &mut serde_json::Value,
    fixture: &Fixture,
    language: Language,
    resource_dir: &Path,
) -> Result<(), String> {
    let FixtureSpec::EventSchema { fields, .. } = &fixture.spec else {
        return Ok(());
    };
    if !fields.iter().any(|f| f.sce_type.starts_with("enum:")) {
        return Ok(());
    }
    let (_, enums) = read_event_schema(&fixture.document_path(resource_dir), &fixture.name)?;
    let rows = value
        .get_mut("fields")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| {
            format!(
                "fixture {}: an event-schema fixture did not serialize its 'fields' as an array",
                fixture.name
            )
        })?;
    for (row, field) in rows.iter_mut().zip(fields) {
        let Some(alias) = field.sce_type.strip_prefix("enum:") else {
            continue;
        };
        let model = enums.get(alias).ok_or_else(|| {
            format!(
                "fixture {}: field `{}` holds the enum `{alias}`, which the schema imports \
                 from no readable enum document",
                fixture.name, field.name
            )
        })?;
        let EventSchemaFieldValue::Text(declared) = &field.value else {
            return Err(format!(
                "fixture {}: field `{}` holds an enum, whose value is the name a variant \
                 declares",
                fixture.name, field.name
            ));
        };
        if !model.variants.iter().any(|v| &v.name == declared) {
            return Err(format!(
                "fixture {}: field `{}` holds `{declared}`, which no variant of `{}` declares",
                fixture.name, field.name, model.name
            ));
        }
        let row = row.as_object_mut().ok_or_else(|| {
            format!(
                "fixture {}: field `{}` did not serialize to a JSON object",
                fixture.name, field.name
            )
        })?;
        row.insert(
            "enum_document".into(),
            serde_json::Value::from(model.name.clone()),
        );
        row.insert(
            "ident".into(),
            serde_json::Value::from(crate::forge::enum_naming::variant_ident(
                language,
                &model.name,
                declared,
            )),
        );
    }
    Ok(())
}

/// Render the per-language conformance harness from a manifest, returning
/// the rendered source code.
///
/// `resource_dir` is the directory containing the per-fixture `<name>.scxml`
/// files. The harness generator parses each lookup fixture's SCXML to derive
/// its generated function name (avoiding the SCXML/manifest duplication
/// footgun where the user has to keep both in sync).
///
/// This is the shared entry point used by both the `sce-codegen
/// generate-conformance` subcommand and any language's build system calling
/// sce-build in-process (e.g. the Rust `build.rs`).
///
/// Rendering the same manifest per language from one source of fixtures is
/// what realises RFC §synth-6.2.1 cross-backend parity: every backend runs
/// the same test vectors, so a divergence is a test failure rather than an
/// unnoticed difference between two hand-written suites.
///
/// Returns the source together with [`RenderedHarness::extra_inputs`] —
/// the files this render read that no caller could name from its own
/// arguments. Reporting them is the render's job precisely because it
/// alone knows: which files those are depends on the language and on the
/// manifest's contents, and the one caller that tried to keep the list
/// by hand (the C11 CMake step's `DEPENDS`) is the shape this whole
/// depfile axis keeps finding wrong.
pub fn render_harness(
    manifest: &Manifest,
    language: Language,
    template_base: &Path,
    resource_dir: &Path,
) -> Result<RenderedHarness, String> {
    let layout = harness_layout(language);
    let template_dir: PathBuf = template_base.join(layout.template_subdir);
    if !template_dir.exists() {
        return Err(format!(
            "conformance template directory not found: {}",
            template_dir.display()
        ));
    }

    // Match the product-template environment settings exactly
    // (`trim_blocks` + `lstrip_blocks` + Python-Jinja2 method compat) so
    // per-kind `{% include %}` fragments lay out predictably without every
    // line needing a manual `{%- -%}` dash.
    let mut env = crate::generator::new_env();
    // Case-conversion filters are shared with the forge product templates so
    // the naming conventions stay aligned between product and test code.
    // The harness templates render test *code*, not an authored
    // datamodel: nothing here pipes an SCXML expression through
    // `to_lua_expr`, so the scope has no document to carry and the
    // installed vocabulary is the whole of it.
    let scope = std::sync::Arc::new(crate::ecmascript::DocumentScope::installed());
    crate::filters::register_filters(&mut env, &scope);
    crate::filters::register_go_filters(&mut env, &scope);
    // Conformance type-mapping filters: every per-language template uses the
    // same filter names (`rust_type`, `cpp_type`, `go_type`, `kt_type`, plus
    // the `*_unmarshal` variants) so the canonical-type table lives in one
    // place (see `register_conformance_filters`) instead of being duplicated
    // as inline macros inside each harness template.
    register_conformance_filters(&mut env);
    crate::generator::load_templates(&mut env, &template_dir, language)
        .map_err(|e| e.to_string())?;

    let tmpl_name = layout.template_filename;
    let tmpl = env
        .get_template(tmpl_name)
        .map_err(|e| format!("template {tmpl_name} not found: {e}"))?;

    // Clone fixtures so we can fill derived fields from each SCXML source
    // (lookup's `function` id + procedure's `helpers_ordered` list) without
    // mutating the loaded manifest.
    let mut fixtures = manifest.fixtures.clone();

    // Matrix-aware fixture filter (RFC §synth-5-J-4 single source of truth).
    // The forge codegen matrix at `forge::codegen_matrix::template_ships`
    // declares which `(kind, language)` pairs have a shipped product
    // template. The conformance harness piggy-backs on the same gate
    // because the conformance fragment is co-shipped with the product
    // template — there is no fixture whose product emits but whose
    // conformance fragment is missing, so a single matrix flip closes
    // both sides at once. C11 layers an extra C11-specific predicate
    // on top via `c11_supported_kind` (kept as a separate function so
    // a C11-internal subset stays expressible without polluting
    // the language-agnostic matrix). A backend that writes a kind in part
    // is scheduled for the fixtures its generator writes
    // (`generator_writes_fixture`).
    // Single per-fixture gate (kind-matrix + C11 kind subset +
    // MCU-only-on-non-MCU exclusion). Mirrors the same predicate
    // applied by `sce-codegen list-fixtures --language <X>` so
    // `cargo test` and `cmake --build` see identical fixture sets.
    //
    // Applied before the derived fields are filled, which read each fixture's
    // document by the rules of its kind: a fixture this backend does not write
    // has no symbol here to derive, and a derivation that has none for it is no
    // defect of the document.
    let mut filter_err: Option<String> = None;
    fixtures.retain(|f| {
        if filter_err.is_some() {
            return false;
        }
        match lang_supports_fixture(f, language, resource_dir) {
            Ok(b) => b,
            Err(e) => {
                filter_err = Some(e);
                false
            }
        }
    });
    if let Some(e) = filter_err {
        return Err(e);
    }

    for f in fixtures.iter_mut() {
        let fixture_name = f.name.clone();
        let scxml_path = f.document_path(resource_dir);
        match &mut f.spec {
            FixtureSpec::Lookup {
                function,
                output_id,
                output,
                enum_values,
                ..
            } => {
                let raw_output_id = read_lookup_output_id(&scxml_path)?;
                // Symbol-name SSOT: lower to the exact symbol render_lookup
                // defines — the bare call-base plus the C11 flat prefix, via
                // the same two helpers the codegen uses (flat-scope naming rule).
                let base = crate::forge::generator::forge_lookup_symbol(&raw_output_id, language);
                *function = Some(crate::forge::generator::forge_stateless_def_symbol(
                    &fixture_name,
                    &base,
                    language,
                ));
                *output_id = Some(raw_output_id);
                // String-output lookups: derive enum variant names from SCXML
                // entries so the harness can generate enum-to-string helpers.
                if output.ty == CanonicalType::String {
                    *enum_values = read_lookup_enum_values(&scxml_path)?;
                }
            }
            FixtureSpec::Validator { has_state, .. } => {
                *has_state = read_validator_has_state(&scxml_path)?;
            }
            FixtureSpec::Condition { function, .. } => {
                // Symbol-name SSOT: lower the manifest `function` to the
                // exact symbol render_condition defines — the bare call-base
                // plus the C11 flat prefix, via the same two helpers the
                // codegen uses (flat-scope naming rule).
                let base = crate::forge::generator::forge_condition_symbol(function, language);
                *function = crate::forge::generator::forge_stateless_def_symbol(
                    &fixture_name,
                    &base,
                    language,
                );
            }
            FixtureSpec::Transform {
                output,
                compound_outputs,
                holder,
                ..
            } => {
                if let Some(symbols) = read_transform_holder(&scxml_path, &fixture_name, language)?
                {
                    // A holder is driven by a sequence and compared from the
                    // record its update returns — so the oracle is the
                    // stateful section, and every output has a key there.
                    if f.ref_section != STATEFUL_TRANSFORMS {
                        return Err(format!(
                            "fixture {fixture_name}: the document reads a field through \
                             previous(), so it is driven through its holder and its \
                             oracle belongs under ref_section \"{STATEFUL_TRANSFORMS}\", \
                             not \"{}\"",
                            f.ref_section
                        ));
                    }
                    if output.is_some() || compound_outputs.is_empty() {
                        return Err(format!(
                            "fixture {fixture_name}: a holder returns every output in \
                             one record, so declare each as a `compound_outputs` entry \
                             keyed by its id, and no scalar `output`"
                        ));
                    }
                    *holder = Some(symbols);
                    for co in compound_outputs.iter_mut() {
                        co.field = Some(crate::forge::generator::forge_transform_output_field(
                            &co.key, language,
                        ));
                    }
                } else if f.ref_section == STATEFUL_TRANSFORMS {
                    return Err(format!(
                        "fixture {fixture_name}: ref_section \"{STATEFUL_TRANSFORMS}\" is \
                         for a transform that reads a field through previous(), and \
                         this document reads none"
                    ));
                }
                // Symbol-name SSOT: lower each output accessor to the exact
                // symbol render_transform defines — the bare call-base plus the
                // C11 flat prefix, via the same two helpers the codegen uses
                // (flat-scope naming rule). The output id is the compound `key`
                // directly; for the scalar output it is the `compute_`-stripped
                // function (the inverse of render_transform's `compute_<out.id>`
                // composition).
                if let Some(out) = output.as_mut() {
                    if let Some(orig) = out.function.as_ref() {
                        let output_id = orig.strip_prefix("compute_").unwrap_or(orig);
                        let base =
                            crate::forge::generator::forge_transform_symbol(output_id, language);
                        out.function = Some(crate::forge::generator::forge_stateless_def_symbol(
                            &fixture_name,
                            &base,
                            language,
                        ));
                    }
                }
                for co in compound_outputs.iter_mut() {
                    let base = crate::forge::generator::forge_transform_symbol(&co.key, language);
                    co.function = crate::forge::generator::forge_stateless_def_symbol(
                        &fixture_name,
                        &base,
                        language,
                    );
                }
            }
            FixtureSpec::Codec {
                fields,
                has_test_vectors,
                cbor,
                content_line,
                grows,
                encode_may_fail,
                encode_refuses_bound,
                cpp_printed,
            } => {
                if document_exists(&scxml_path) && matches!(language, Language::Cpp) {
                    *cpp_printed = crate::cpp_codec_printed_file(
                        &scxml_path,
                        &[],
                        &crate::ForgeCompileOptions::default(),
                    )
                    .map_err(|e| format!("{}: {e:?}", scxml_path.display()))?
                    .unwrap_or(false);
                }
                if document_exists(&scxml_path) {
                    derive_codec_shape(
                        &scxml_path,
                        &fixture_name,
                        fields,
                        cbor,
                        content_line,
                        encode_may_fail,
                    )?;
                    if *content_line {
                        // A content-line codec imports enums alone, so whether
                        // `Inline` holds it is its own entries' answer.
                        let model = read_codec_model(&scxml_path, &fixture_name)?;
                        *grows = !model.inline_holds_with(|_| true);
                    }
                    if !*cbor && !*content_line {
                        let model = read_codec_model(&scxml_path, &fixture_name)?;
                        // A C++ codec printed from its plan holds each bound in
                        // a type, and its encode refuses nothing of it
                        // (docs/adr/0081 step 4).
                        *encode_refuses_bound = !*cpp_printed
                            && crate::forge::generator::encode_refuses_a_bound(
                                &model.fields,
                                language,
                            );
                        // A positional codec has an eager form at the default
                        // profile only where `Inline` holds every list of it
                        // and of what it holds; otherwise a fragment names the
                        // eager form at the growing profile.
                        *grows = !crate::codec_lent_holds_file(
                            &scxml_path,
                            &[],
                            &crate::ForgeCompileOptions::default(),
                        )
                        .map_err(|e| format!("{}: {e:?}", scxml_path.display()))?
                        .unwrap_or(true);
                    }
                }
                // RFC §synth-5-B test-vector: enrich the manifest with the
                // SCXML-derived `<sce:test-vector>` flag so the per-language
                // harness fragment can fold the per-fixture sidecar's failure
                // count into the global accumulator without re-scanning
                // the SCXML at every render site. The codec sidecar ships
                // on Rust + C11 only — for Cpp / Kotlin / Go / Python the
                // `render_codec_test_vector_sidecar` gate returns
                // `Ok(None)` so no sidecar file exists; the harness
                // must therefore skip the `#include`. Force the flag
                // false on those 4 backends (mirrors the rotating
                // gate-rejection tests in `forge_conformance.rs`).
                *has_test_vectors = if matches!(language, Language::Rust | Language::C11) {
                    has_test_vectors_in_file(&scxml_path)?
                } else {
                    false
                };
            }
            FixtureSpec::Algorithm {
                args,
                output,
                function,
                has_test_vectors,
                may_fail,
            } => {
                *has_test_vectors = has_test_vectors_in_file(&scxml_path)?;
                *may_fail = read_algorithm_may_fail(&scxml_path, &fixture_name)?;
                // A record's fields come from its event-schema document, as
                // this language spells them.
                // A list's record element too.
                let arg_records = args.iter_mut().filter_map(|arg| match arg {
                    AlgorithmArg::Record(r) => Some(r),
                    AlgorithmArg::List(list) => list.list_of.record_mut(),
                    AlgorithmArg::Scalar(_)
                    | AlgorithmArg::Enum(_)
                    | AlgorithmArg::Optional(_)
                    | AlgorithmArg::Union(_) => None,
                });
                let output_record = match output {
                    AlgorithmOutput::Record(r) => Some(r),
                    AlgorithmOutput::List(list) => list.list_of.record_mut(),
                    AlgorithmOutput::Optional(_) | AlgorithmOutput::Scalar(_) => None,
                };
                for r in arg_records.chain(output_record) {
                    r.fields = read_record_fields(manifest, resource_dir, &r.record, language)?;
                }
                // A value of an enum is built from the name its variant
                // declares, into the member this language emits for it.
                // A value of a union is built from its wire object, case by
                // case, into the type the union's unit declares.
                for arg in args.iter_mut() {
                    match arg {
                        AlgorithmArg::Enum(e) => {
                            e.variants = read_enum_arg_variants(
                                manifest,
                                resource_dir,
                                &e.enum_document,
                                language,
                            )?;
                        }
                        AlgorithmArg::Union(u) => {
                            u.shape = Some(read_union_shape(
                                manifest,
                                resource_dir,
                                &u.union_document,
                                language,
                            )?);
                        }
                        AlgorithmArg::Scalar(_)
                        | AlgorithmArg::Record(_)
                        | AlgorithmArg::List(_)
                        | AlgorithmArg::Optional(_) => {}
                    }
                }
                // Symbol-name SSOT: lower the manifest's `function` to the
                // exact per-language symbol the algorithm template defines,
                // via the one helper the product codegen and the cross-doc
                // resolver also read. This replaces the per-language casing
                // filter (`| to_pascal_case` / `| to_camel_case` /
                // `| to_snake_case`) the harness fragments used to apply, so
                // the casing rule lives in one place. RFC §synth-7 items A5/A6: the
                // algorithm name *is* the fixture stem in every shipped SCXML,
                // so — unlike lookup/condition/transform on C11 — no fixture
                // prefix is composed; `forge_algorithm_symbol` returns the
                // bare lowered symbol on every backend.
                *function = crate::forge::generator::forge_algorithm_symbol(function, language);
            }
            FixtureSpec::Procedure {
                helpers,
                helpers_ordered,
                is_l2,
                ..
            } => {
                // Parse the SCXML for <sce:helper> declarations and cross-check
                // against the fixtures.json helpers dict. Missing in either
                // direction fails loud here rather than drifting silently into
                // positional-argument swaps at test compile time.
                let scxml_helper_names = read_procedure_helper_names(&scxml_path)?;
                for scxml_name in &scxml_helper_names {
                    if !helpers.contains_key(scxml_name) {
                        return Err(format!(
                            "fixture {}: SCXML declares <sce:helper name=\"{}\"> \
                             but fixtures.json has no matching stub entry under \
                             procedure.helpers",
                            fixture_name, scxml_name
                        ));
                    }
                }
                for dict_name in helpers.keys() {
                    if !scxml_helper_names.contains(dict_name) {
                        return Err(format!(
                            "fixture {}: fixtures.json has helper stub \"{}\" \
                             but the SCXML does not declare <sce:helper name=\"{}\">",
                            fixture_name, dict_name, dict_name
                        ));
                    }
                }
                // Build the ordered list in SCXML source order. The stub dict
                // is consumed by name during this fold so any leftover entries
                // (already checked above) would have failed earlier.
                *helpers_ordered = scxml_helper_names
                    .iter()
                    .map(|name| HelperStub {
                        name: name.clone(),
                        stub: helpers.get(name).cloned().expect("cross-checked above"),
                    })
                    .collect();
                // Populate the derived L2 flag from the SCXML so the
                // C11 backend's kind-subset fixture filter
                // (`c11_supported_kind`) can gate by procedure level
                // without re-parsing each candidate fixture.
                *is_l2 = read_procedure_is_l2(&scxml_path)?;
            }
            FixtureSpec::Interpolation { symbol, .. } => {
                // Symbol-name SSOT: derive the exact accessor symbol
                // render_interpolation defines — the bare call-base plus the
                // C11 flat prefix, via the same two helpers the codegen uses —
                // so the harness call site reads the bare `f.symbol` value.
                let base = crate::forge::generator::forge_interpolation_symbol(language);
                *symbol = Some(crate::forge::generator::forge_stateless_def_symbol(
                    &fixture_name,
                    &base,
                    language,
                ));
            }
            FixtureSpec::Union { shape } => {
                *shape = Some(read_union_shape(
                    manifest,
                    resource_dir,
                    &fixture_name,
                    language,
                )?);
            }
            _ => {}
        }
    }

    // Per-kind presence flags let per-language harness scaffolds pull in
    // kind-specific runtime imports (e.g. Go `forge` package, Kotlin
    // `sce-kotlin-runtime` types) only when the fixture set actually uses
    // that kind, keeping unused-import warnings and transitive deps scoped.
    let has_procedure = fixtures
        .iter()
        .any(|f| matches!(f.spec, FixtureSpec::Procedure { .. }));
    let has_timer = fixtures
        .iter()
        .any(|f| matches!(f.spec, FixtureSpec::Timer { .. }));
    let has_codec = fixtures
        .iter()
        .any(|f| matches!(f.spec, FixtureSpec::Codec { .. }));
    // A codec with an entry that may be absent: the Go harness holds its decoded
    // cases in the runtime's optional type, the one its generated codec holds the
    // entry in (docs/adr/0055, section 5), and imports it only then.
    let has_optional_codec_field = fixtures.iter().any(|f| {
        matches!(&f.spec, FixtureSpec::Codec { fields, .. } if fields.iter().any(|field| field.optional))
    });
    // An algorithm with an argument or an answer that may be absent: the Go
    // harness holds it in the same optional type, the one the generated
    // algorithm's slot is (docs/adr/0055, section 5).
    let has_optional_algorithm_slot = fixtures.iter().any(|f| {
        matches!(
            &f.spec,
            FixtureSpec::Algorithm { args, output, .. }
                if matches!(output, AlgorithmOutput::Optional(_))
                    || args.iter().any(|a| matches!(a, AlgorithmArg::Optional(_)))
        )
    });

    // The oracle is an input to every backend's render.
    //
    // C11 pre-bakes the whole entry because it has no runtime JSON parser
    // compatible with the backend's zero-dependency rule — the other five
    // parse `numerical_reference.json` at test-runtime via their language's
    // JSON library, so for them only the `cases` fold is C11-specific.
    //
    // Decode-rejection vectors are pre-baked for all six: they are short
    // byte literals plus one symbol, and inlining them keeps the six codec
    // fragments to one shape instead of six JSON-navigation dialects.
    //
    // `float_tolerance` is exposed to C11 so the harness can `#define` the
    // tolerance once instead of duplicating the literal across every
    // per-kind fragment; the other backends read it from the JSON directly.
    let mut extra_inputs: Vec<PathBuf> = Vec::new();
    let reference_path = resource_dir
        .parent()
        .map(|p| p.join("conformance/numerical_reference.json"))
        .ok_or_else(|| {
            "cannot derive numerical_reference.json path from resource_dir".to_string()
        })?;
    let reference_text = std::fs::read_to_string(&reference_path)
        .map_err(|e| format!("read {}: {e}", reference_path.display()))?;
    let reference: serde_json::Value = serde_json::from_str(&reference_text)
        .map_err(|e| format!("parse {}: {e}", reference_path.display()))?;
    // Declared below rather than here, once the fold has run. Opening a
    // file is not reading it: on the five non-C11 routes the oracle
    // contributes to the harness only through the reject vectors it
    // carries, so a catalog with none leaves those renders byte-identical
    // no matter how the file is edited. Declaring it anyway would name a
    // prerequisite that only forces rebuilds, which the depfile-content
    // gate reports in exactly those words.
    let mut baked_from_reference = false;

    let (fixtures_value, float_tolerance): (minijinja::Value, Option<f64>) = {
        let bake_cases = matches!(language, Language::C11);
        let tol = if bake_cases {
            // C11 bakes the tolerance into a `#define`, so the value itself
            // reaches the harness source and the file is read on this route
            // whatever the fixtures carry.
            baked_from_reference = true;
            reference.get("float_tolerance").and_then(|v| v.as_f64())
        } else {
            None
        };

        let mut enriched = Vec::with_capacity(fixtures.len());
        for f in &fixtures {
            let mut v = serde_json::to_value(f)
                .map_err(|e| format!("serialize fixture {}: {e}", f.name))?;
            baked_from_reference |=
                fold_reject_vectors(&mut v, &reference, f, language, &reference_path)?;
            baked_from_reference |= fold_source_cases(
                &mut v,
                &reference,
                f,
                language,
                resource_dir,
                &reference_path,
            )?;
            baked_from_reference |= fold_refused_cases(
                &mut v,
                &reference,
                f,
                language,
                resource_dir,
                &reference_path,
            )?;
            fold_enum_identifiers(&mut v, f, language)?;
            fold_event_schema_enum_fields(&mut v, f, language, resource_dir)?;
            // `oracle_eligible: false` fixtures don't have a field oracle —
            // they're compile-only. Skip the lookup + merge so the harness
            // fragment renders the minimal probe block without any `cases`
            // array. Their reject vectors, folded above, do not depend on
            // the field shape being expressible.
            if !bake_cases || !f.oracle_eligible {
                enriched.push(v);
                continue;
            }
            {
                // Fold every key of the oracle entry (e.g. `cases`, `sequence`,
                // `tags`) into the fixture object so per-kind fragments can
                // reference them as `f.<key>` regardless of which shape the
                // oracle uses for that kind. transform/condition/lookup/codec
                // expose `cases`; validator/observer expose `sequence`. New
                // kinds can introduce their own shape without changing this
                // enrichment site.
                let oracle_entry = reference
                    .get(&f.ref_section)
                    .and_then(|s| s.get(&f.name))
                    .ok_or_else(|| {
                        format!(
                            "fixture {} not found in {} under section {}",
                            f.name,
                            reference_path.display(),
                            f.ref_section
                        )
                    })?
                    .as_object()
                    .ok_or_else(|| {
                        format!(
                            "fixture {} oracle entry is not a JSON object in {}",
                            f.name,
                            reference_path.display()
                        )
                    })?;
                let fixture_obj = v.as_object_mut().ok_or_else(|| {
                    format!("fixture {} did not serialize to a JSON object", f.name)
                })?;
                for (k, val) in oracle_entry {
                    // Manifest fields take precedence — the oracle is for
                    // case data, not for redefining the fixture's identity
                    // or shape. This guards against accidental overrides
                    // (e.g. timer's oracle `timers` block colliding with
                    // its manifest `timers` list when a future phase adds
                    // timer to c11_supported_kind).
                    fixture_obj.entry(k.clone()).or_insert_with(|| val.clone());
                }
                enriched.push(v);
            }
        }
        (
            minijinja::Value::from_serialize(serde_json::Value::Array(enriched)),
            tol,
        )
    };

    if baked_from_reference {
        extra_inputs.push(reference_path.clone());
    }

    let ctx = minijinja::context! {
        fixtures => fixtures_value,
        has_procedure => has_procedure,
        has_timer => has_timer,
        has_codec => has_codec,
        has_optional_codec_field => has_optional_codec_field,
        has_optional_algorithm_slot => has_optional_algorithm_slot,
        float_tolerance => float_tolerance,
    };

    let source = tmpl
        .render(ctx)
        .map_err(|e| format!("render template {tmpl_name}: {e}"))?;
    Ok(RenderedHarness {
        source,
        extra_inputs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn derived_fields(fixture: &str) -> Vec<StructField> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let manifest = Manifest::load(&root.join("tests/forge/conformance/fixtures.json"))
            .expect("manifest must load and validate");
        let found = manifest
            .fixtures
            .iter()
            .find(|f| f.name == fixture)
            .expect("the fixture is registered");
        let FixtureSpec::Codec { fields, .. } = &found.spec else {
            panic!("{fixture} is a codec");
        };
        let mut fields = fields.clone();
        let (mut cbor, mut content_line, mut encode_may_fail) = (false, false, false);
        derive_codec_shape(
            &found.document_path(&root.join("tests/forge/resources")),
            fixture,
            &mut fields,
            &mut cbor,
            &mut content_line,
            &mut encode_may_fail,
        )
        .expect("the shape derives");
        assert!(content_line && !cbor);
        fields
    }

    /// A repeated property that declares a parameter or a separator is a list of
    /// records whose members the document names, and one that does not is the list
    /// of values it always was (docs/adr/0014).
    #[test]
    fn a_line_record_list_derives_its_members_from_the_document() {
        let records = derived_fields("codec_content_line_records");
        let field = |name: &str| records.iter().find(|f| f.name == name).expect("a field");
        let names = |f: &StructField| -> Vec<(String, bool, bool)> {
            f.members
                .iter()
                .map(|m| (m.name.clone(), m.optional, m.list))
                .collect()
        };
        // Parameters in declaration order, optional unless required, then the value.
        assert_eq!(
            names(field("attendee")),
            [
                ("attendeeCn".to_owned(), true, false),
                ("attendeeRole".to_owned(), false, false),
                ("value".to_owned(), false, false),
            ]
        );
        assert_eq!(
            names(field("exdate")),
            [
                ("exdateTzid".to_owned(), true, false),
                ("values".to_owned(), false, true),
            ]
        );
        // A single-line list has values and no records; a plain property has neither.
        for (name, list) in [("categories", true), ("geo", true), ("uid", false)] {
            assert_eq!(field(name).list, list, "{name}");
            assert!(field(name).members.is_empty(), "{name}");
        }
        assert!(field("attendee").list && field("exdate").list);

        // The event fixture's repeated property declares neither, so it is unchanged.
        let event = derived_fields("codec_content_line_event");
        let exdate = event.iter().find(|f| f.name == "exdate").expect("a field");
        assert!(exdate.list && exdate.members.is_empty());
    }

    /// A parameter of a record's lines is a member of each record; listing it as a
    /// field of the codec would assert a struct member that does not exist.
    #[test]
    fn a_record_parameter_is_refused_as_a_field_of_the_codec() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut fields = vec![StructField {
            name: "attendeeCn".to_owned(),
            ty: CanonicalType::String,
            compare: CompareMode::Equality,
            optional: false,
            bounded: false,
            derived: false,
            list: false,
            enum_document: None,
            enum_open: false,
            members: Vec::new(),
        }];
        let (mut cbor, mut content_line, mut encode_may_fail) = (false, false, false);
        let refusal = derive_codec_shape(
            &root.join("tests/forge/resources/codec_content_line_records.scxml"),
            "codec_content_line_records",
            &mut fields,
            &mut cbor,
            &mut content_line,
            &mut encode_may_fail,
        )
        .expect_err("a record parameter is not a field");
        assert!(
            refusal.contains("member of each of its records"),
            "{refusal}"
        );
    }

    #[test]
    fn loads_real_manifest() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let m = Manifest::load(&path).expect("manifest must load and validate");
        assert_eq!(m.version, 1);
        assert!(
            !m.fixtures.is_empty(),
            "manifest should contain at least one fixture"
        );
    }

    /// Drift guard between the Rust `Manifest`/`FixtureSpec` type hierarchy
    /// and the checked-in `tests/forge/conformance/fixtures.schema.json`.
    ///
    /// Prior to this test the schema was hand-maintained and could silently
    /// lag behind type changes — the validator session added a field and the
    /// schema update was forgotten for a full session. Now every type in the
    /// manifest hierarchy derives `JsonSchema` (gated on `cfg(test)` so the
    /// dependency stays in dev-deps), and this test re-derives the schema on
    /// every run and fails with a diff if the checked-in file drifts.
    ///
    /// Bootstrap / refresh: run `UPDATE_EXPECT=1 cargo test -p sce-build
    /// schema_drift_guard` to rewrite the file from the current types. The
    /// reviewer inspects the diff and commits alongside whatever type change
    /// triggered the update — the invariant is *not* that the schema is
    /// frozen, it is that the schema matches the Rust types at every commit.
    #[test]
    fn schema_drift_guard() {
        let schema = schemars::schema_for!(Manifest);
        let generated =
            serde_json::to_string_pretty(&schema).expect("schema must serialize to JSON");
        // Trailing newline so the file is POSIX-friendly and `git diff`
        // doesn't flag a "no newline at end of file" on every write.
        let generated = format!("{generated}\n");

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/conformance/fixtures.schema.json");

        if std::env::var_os("UPDATE_EXPECT").is_some() {
            std::fs::write(&path, &generated)
                .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
            return;
        }

        let checked_in = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(
            checked_in, generated,
            "\nfixtures.schema.json is out of sync with the Rust types in \
             sce-build/src/conformance.rs. Run:\n    \
             UPDATE_EXPECT=1 cargo test -p sce-build schema_drift_guard\n\
             and commit the regenerated file.\n"
        );
    }

    /// Render the conformance harness for every target language and verify
    /// that template rendering succeeds.
    ///
    /// This catches:
    /// - Missing `kinds/<kind>.*.jinja2` fragment for any fixture kind
    /// - Undefined template variable references (e.g. `{{ f.output_id }}`)
    /// - SCXML parse failures during derive-at-render (lookup function,
    ///   procedure helpers, enum values)
    /// - Fixture/template schema mismatches
    ///
    /// It does **not** compile the generated source — that requires each
    /// language's toolchain. But it guarantees the template layer is
    /// internally consistent across all 5 languages for every fixture in
    /// the manifest.
    #[test]
    fn render_harness_all_languages() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");

        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");

        // Every backend that emits anything has a harness; one that emits
        // nothing yet has no template directory to render.
        for &lang in Language::ALL
            .iter()
            .filter(|lang| crate::forge::codegen_matrix::emits_anything(**lang))
        {
            let result = render_harness(&manifest, lang, &template_base, &resource_dir);
            match result {
                Ok(rendered) => {
                    assert!(
                        !rendered.source.is_empty(),
                        "{lang:?}: render_harness returned empty output"
                    );
                }
                Err(e) => {
                    panic!(
                        "{lang:?}: render_harness failed: {e}\n\
                         This usually means a conformance template references \
                         an undefined variable or a kinds/ fragment is missing \
                         for a fixture kind in fixtures.json."
                    );
                }
            }
        }
    }

    /// A byte case stated as the codec's interface presents the value
    /// (docs/adr/0081 §5) reaches the Rust harness as a literal of the codec's
    /// ready-made source, encoded and compared with its bytes.
    #[test]
    fn a_source_case_reaches_the_rust_harness_as_a_parts_literal() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let rust = render_harness(&manifest, Language::Rust, &template_base, &resource_dir)
            .expect("the Rust harness renders")
            .source;
        for literal in [
            "CodecPresentIfBasicParts { flags: 1, seq: Some(4660) }",
            "CodecPresentIfBasicParts { flags: 254, seq: None }",
            "CodecFlagsBasicParts { header: 165 }",
            // Text held to its bound; a length gated with it is no member.
            "CodecPresentIfStringParts { carrier: 1, text: Some(::sce_forge_runtime::codec::Bounded::new(\"abc\")",
            // A VLE at its largest, and a codec of no element.
            "CodecVleZintU64Parts { value: 18446744073709551615 }",
            "CodecZenohDelParts {  }",
            // The entries of a list, bound first, as the parts of the codec
            // they are, with the path the harness reaches it by; and a body.
            "let held_0: [fixtures::codec_repeat_elem::CodecRepeatElemParts; 2] = \
             [fixtures::codec_repeat_elem::CodecRepeatElemParts { seq: 1 }, \
             fixtures::codec_repeat_elem::CodecRepeatElemParts { seq: 43981 }];",
            "fixtures::codec_repeat_basic::CodecRepeatBasicParts { frags: \
             ::sce_forge_runtime::codec::Bounded::new(&held_0[..])",
            "fixtures::codec_embed_basic::CodecEmbedBasicParts { tag: 2, locator: &held_0 }",
            // A gate group: the arm the value holds, and an `Option` of it for
            // a group the value may hold no arm of.
            "flags_has_payload: fixtures::codec_present_if_length_ref::\
             CodecPresentIfLengthRefFlagsHasPayload::NotHasPayload { payload_size: 9 }",
            "flags_wants_a_or_wants_b: Some(fixtures::codec_present_if_disjunction::\
             CodecPresentIfDisjunctionFlagsWantsAOrWantsB::WantsB { seq: 5 })",
            // A flag input the value says: the decode is handed it, and the
            // value is held to saying the one the case hands, as the bool
            // itself (a comparison with `true` is clippy's `bool_comparison`).
            "fixtures::codec_zenoh_wireexpr::CodecZenohWireexprWire::input_n(&parts),",
            // A variant's default arm, holding the tag no arm names.
            "body: fixtures::codec_variant_dispatch::CodecVariantDispatchPartsBody::Default { \
             tag: fixtures::codec_variant_dispatch::CodecVariantDispatchUnknownTag::new(127)",
        ] {
            assert!(rust.contains(literal), "the Rust harness lacks `{literal}`");
        }
        assert!(rust.contains("CodecPresentIfBasicWire::encode_source("));
    }

    /// The same byte cases reach the C++ harness as the printed value its
    /// constructor makes (docs/adr/0081 §5): each element that holds a bound
    /// or a law is made by `make` of the type the class's own accessor presents
    /// it as, a byte string and a list over a vector bound first, and a body,
    /// a group's arm and a variant's arm by value. The value is encoded over a
    /// sink and compared with the bytes, and a decode of the bytes compared
    /// with it element by element.
    #[test]
    fn a_source_case_reaches_the_cpp_harness_as_a_constructed_value() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let cpp = render_harness(&manifest, Language::Cpp, &template_base, &resource_dir)
            .expect("the C++ harness renders")
            .source;
        for literal in [
            // A number, and a number a gate decides, present and absent.
            "::SCE::Generated::CodecPresentIfBasic::CodecPresentIfBasic{1u, 4660u}",
            "::SCE::Generated::CodecPresentIfBasic::CodecPresentIfBasic{254u, std::nullopt}",
            // A VLE at its largest as one unsigned literal, and a codec of no
            // element.
            "::SCE::Generated::CodecVleZintU64::CodecVleZintU64{18446744073709551615u}",
            "::SCE::Generated::CodecZenohDel::CodecZenohDel{}",
            // Text made within the bound its accessor presents it in, where a
            // gate leaves it in; its length stated, a byte past ASCII escaped.
            "{1u, decltype(std::declval<const \
             ::SCE::Generated::CodecPresentIfString::CodecPresentIfString&>().text())::\
             value_type::make(std::string_view(\"abc\", 3)).value()}",
            "make(std::string_view(\"h\\303\\251llo\", 6)).value()",
            // Bytes, bound first as the vector their view views.
            "const std::vector<std::uint8_t> held_0{170};",
            "::SCE::Generated::CodecLengthRef::CodecLengthRef{2u, decltype(std::declval<const \
             ::SCE::Generated::CodecLengthRef::CodecLengthRef&>().payload())::make(held_0)\
             .value()}",
            // A body, made by value by its own class.
            "::SCE::Generated::CodecEmbedBasic::CodecEmbedBasic{2u, \
             ::SCE::Generated::CodecZenohLocator::CodecZenohLocator{decltype(std::declval<const \
             ::SCE::Generated::CodecZenohLocator::CodecZenohLocator&>().locator())::make(\
             std::string_view(\"abc\", 3)).value()}}",
            // The entries of a list, bound first as values of the codec they
            // are, and the list made over them.
            "const std::vector<::SCE::Generated::CodecRepeatElem::CodecRepeatElem> held_0{\
             ::SCE::Generated::CodecRepeatElem::CodecRepeatElem{1u}, \
             ::SCE::Generated::CodecRepeatElem::CodecRepeatElem{43981u}};",
            "::SCE::Generated::CodecRepeatBasic::CodecRepeatBasic{decltype(std::declval<const \
             ::SCE::Generated::CodecRepeatBasic::CodecRepeatBasic&>().frags())::make(held_0)\
             .value()}",
            // A gate group: the arm the value holds at its place among the
            // arms, and none of a group the value may hold no arm of.
            "::SCE::Generated::CodecPresentIfLengthRef::CodecPresentIfLengthRef{0u, \
             ::SCE::Generated::CodecPresentIfLengthRef::CodecPresentIfLengthRefFlagsHasPayload{\
             std::in_place_index<1>, ::SCE::Generated::CodecPresentIfLengthRef::\
             CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload{9u}}}",
            "::SCE::Generated::CodecPresentIfDisjunction::CodecPresentIfDisjunction{2u, \
             ::SCE::Generated::CodecPresentIfDisjunction::\
             CodecPresentIfDisjunctionFlagsWantsAOrWantsB{std::in_place_index<1>, \
             ::SCE::Generated::CodecPresentIfDisjunction::\
             CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB{5u}}}",
            "::SCE::Generated::CodecPresentIfDisjunction::CodecPresentIfDisjunction{0u, \
             std::nullopt}",
            // A variant's known arm, and its default arm holding the tag no arm
            // names beside its body.
            "::SCE::Generated::CodecVariantDispatch::CodecVariantDispatch{\
             ::SCE::Generated::CodecVariantDispatch::CodecVariantDispatchVariant{\
             std::in_place_index<0>, \
             ::SCE::Generated::CodecVariantSessionOpen::CodecVariantSessionOpen{258u}}}",
            "::SCE::Generated::CodecVariantDispatch::CodecVariantDispatchVariant{\
             std::in_place_index<2>, ::SCE::Generated::CodecVariantDispatch::\
             CodecVariantDispatchDefault{127u, \
             ::SCE::Generated::CodecVariantSessionClose::CodecVariantSessionClose{9u}}}",
            // A list held to a law: its start, and its entries made into the
            // list the law runs over.
            "decltype(std::declval<const \
             ::SCE::Generated::CodecChainFlagCarrier::CodecChainFlagCarrier&>().entries())::\
             make(true, decltype(std::declval<const \
             ::SCE::Generated::CodecChainFlagCarrier::CodecChainFlagCarrier&>().entries())::\
             list_type::make(held_0).value()).value()",
            // A bare entry's place among the others, here none.
            "const std::vector<::SCE::Generated::CodecZenohExtEntry::CodecZenohExtEntry> \
             held_0{};",
            "decltype(std::declval<const ::SCE::Generated::CodecChainHasMarker::\
             CodecChainHasMarkerExtensionsHas0x12Has0x12&>().extensions())::make(held_0, 0u)\
             .value()",
            // Two lists of one count, each made as the pair holds it.
            "decltype(std::declval<const ::SCE::Generated::CodecRepeatUnconditionalCount::\
             CodecRepeatUnconditionalCountOptionsHH&>().links_weights())::make(std::decay_t<\
             decltype(std::declval<const decltype(std::declval<const \
             ::SCE::Generated::CodecRepeatUnconditionalCount::\
             CodecRepeatUnconditionalCountOptionsHH&>().links_weights())&>().first())>::make(\
             held_0).value(), std::decay_t<decltype(std::declval<const decltype(std::declval<\
             const ::SCE::Generated::CodecRepeatUnconditionalCount::\
             CodecRepeatUnconditionalCountOptionsHH&>().links_weights())&>().second())>::make(\
             held_1).value()).value()",
            // A flag input the value says: the decode is handed it, and the
            // value is held to saying the one the case hands.
            "SCE::Generated::CodecZenohWireexpr::CodecZenohWireexpr::decode(cursor, 1);",
            "::SCE::Generated::CodecZenohWireexpr::input_n(value) == 1",
            // A decode compared element by element: a number whole, a list
            // entry by entry into the elements of each.
            "CHECK(((*decoded).seq() == value.seq()),",
            "[&](const auto& xs1, const auto& ys1) { auto i1 = xs1.begin(); auto j1 = \
             ys1.begin(); for (; i1 != xs1.end() && j1 != ys1.end(); ++i1, ++j1) { if \
             (!((*i1).seq() == (*j1).seq())) { return false; } } return i1 == xs1.end() && j1 \
             == ys1.end(); }((*decoded).frags(), value.frags())",
        ] {
            assert!(cpp.contains(literal), "the C++ harness lacks `{literal}`");
        }
        assert!(
            cpp.contains("SCE::Generated::CodecPresentIfBasic::Wire::encode_source(value, sink);")
        );
    }

    /// A byte case reaches only a harness whose printer prints the codec from
    /// its plan: Kotlin's is handed none, and the C++ one refuses by name a
    /// codec its backend refuses, whose value no case can be made into
    /// (docs/adr/0081 §5, step 4).
    #[test]
    fn a_source_case_reaches_no_harness_whose_printer_does_not_print_the_codec() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let reference_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/conformance/numerical_reference.json");
        let reference: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&reference_path).expect("the reference reads"),
        )
        .expect("the reference parses");
        // As the manifest states it: whether the C++ backend prints it is
        // derived by the render, which this does not run, so here it does not.
        let fixture = manifest
            .fixtures
            .iter()
            .find(|f| f.name == "codec_present_if_basic")
            .expect("the catalog holds codec_present_if_basic");
        let mut value = serde_json::to_value(fixture).expect("a fixture serializes");
        let folded = fold_source_cases(
            &mut value,
            &reference,
            fixture,
            Language::Kotlin,
            &resource_dir,
            &reference_path,
        )
        .expect("Kotlin is handed nothing");
        assert!(
            !folded && value.get("sources").is_none(),
            "Kotlin does not print the plan"
        );
        let refused = fold_source_cases(
            &mut value,
            &reference,
            fixture,
            Language::Cpp,
            &resource_dir,
            &reference_path,
        )
        .expect_err("a codec the C++ backend does not print is refused");
        assert!(
            refused.contains("the C++ backend refuses this codec"),
            "{refused}"
        );
    }

    /// A refused case (docs/adr/0081 §5) reaches the Rust and the C++ harness
    /// as the value its printer spells for a source, encoded and held to the
    /// refusal its encode answers, by the symbol each runtime names it with.
    #[test]
    fn a_refused_case_reaches_the_rust_and_cpp_harnesses_as_the_refusal_its_encode_answers() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let rust = render_harness(&manifest, Language::Rust, &template_base, &resource_dir)
            .expect("the Rust harness renders")
            .source;
        for line in [
            // A trailer given where neither flag is set, and one left out where
            // seq sets has_seq.
            "CodecPresentIfCheckedParts { flags: 0, seq: None, trailer: Some(5) }",
            "CodecPresentIfCheckedParts { flags: 0, seq: Some(258), trailer: None }",
            "let refused = fixtures::codec_present_if_checked::CodecPresentIfCheckedWire::\
             encode_source(",
            "Err(::sce_forge_runtime::codec::CodecError::PresentIfMismatch),",
            "codec_present_if_checked refused 3: the encode wrote bytes before it refused",
        ] {
            assert!(rust.contains(line), "the Rust harness lacks `{line}`");
        }
        let cpp = render_harness(&manifest, Language::Cpp, &template_base, &resource_dir)
            .expect("the C++ harness renders")
            .source;
        for line in [
            "::SCE::Generated::CodecPresentIfChecked::CodecPresentIfChecked{0u, std::nullopt, 5u}",
            "::SCE::Generated::CodecPresentIfChecked::CodecPresentIfChecked{0u, 258u, std::nullopt}",
            "refused.has_value() && *refused == ::SCE::Forge::CodecError::PresentIfMismatch",
            "codec_present_if_checked refused 3: the encode wrote %zu bytes before it refused",
        ] {
            assert!(cpp.contains(line), "the C++ harness lacks `{line}`");
        }
    }

    /// A value of an imported enum is stated by the variant it holds and
    /// spelled as that variant of the type each harness names the enum by; a
    /// flag input the value does not say is handed to the encode as it is to
    /// the decode (docs/adr/0065 §6 R4a); a number the value states while its
    /// content is absent is an `Option` in Rust and a `std::optional` in C++;
    /// and the refusal of an inconsistent source reaches both harnesses by the
    /// name each runtime gives it (docs/adr/0081 §5).
    #[test]
    fn an_enum_a_given_input_and_a_stated_number_reach_both_harnesses() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let rust = render_harness(&manifest, Language::Rust, &template_base, &resource_dir)
            .expect("the Rust harness renders")
            .source;
        for line in [
            "state: fixtures::enum_session_state::EnumSessionState::Programming, attempts: 5",
            "CodecLengthStatedParts { flags: 0, span_len: Some(5), alpha: None, beta: None }",
            "CodecChainHandedEntryWire::encode_source(\n            &parts,\n            \
             &mut ::sce_forge_runtime::codec::VecSink::new(&mut encoded), 1,\n",
            "CodecChainHandedEntry::decode(&mut cursor, 1)",
            "Err(::sce_forge_runtime::codec::CodecError::InconsistentSource),",
        ] {
            assert!(rust.contains(line), "the Rust harness lacks `{line}`");
        }
        let cpp = render_harness(&manifest, Language::Cpp, &template_base, &resource_dir)
            .expect("the C++ harness renders")
            .source;
        for line in [
            "{::SCE::Generated::EnumSessionState::EnumSessionState::Programming, 5u}",
            "::SCE::Generated::CodecLengthStated::CodecLengthStated{0u, 5u, std::nullopt, \
             std::nullopt}",
            "SCE::Generated::CodecChainHandedEntry::Wire::encode_source(value, sink, 1);",
            "SCE::Generated::CodecChainHandedEntry::CodecChainHandedEntry::decode(cursor, 1);",
            "refused.has_value() && *refused == ::SCE::Forge::CodecError::InconsistentSource",
        ] {
            assert!(cpp.contains(line), "the C++ harness lacks `{line}`");
        }
    }

    /// A refused case names a refusal the codec's encode can answer, and one
    /// the backend's runtime names; a backend whose encode does not print the
    /// plan is handed none. Each is read from a reference written here, so the
    /// rule is held apart from the catalog's own cases.
    #[test]
    fn a_refused_case_names_a_refusal_its_encode_answers() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let reference_path = Path::new("numerical_reference.json");
        let fixture_of = |name: &str| {
            manifest
                .fixtures
                .iter()
                .find(|f| f.name == name)
                .unwrap_or_else(|| panic!("the catalog holds {name}"))
                .clone()
        };
        let reference_of = |name: &str, error: &str| {
            serde_json::json!({ "codecs": { name: { "refused": [{
                "source": { "flags": 0, "seq": null, "trailer": 5 },
                "error": error,
                "note": "the trailer given where neither flag is set",
            }] } } })
        };
        let fold = |fixture: &Fixture, reference: &serde_json::Value, language: Language| {
            let mut value = serde_json::to_value(fixture).expect("a fixture serializes");
            fold_refused_cases(
                &mut value,
                reference,
                fixture,
                language,
                &resource_dir,
                reference_path,
            )
            .map(|folded| (folded, value))
        };
        let checked = fixture_of("codec_present_if_checked");
        let mismatch = reference_of("codec_present_if_checked", "present-if-mismatch");

        let (folded, value) = fold(&checked, &mismatch, Language::Rust).expect("Rust states it");
        assert!(folded);
        assert_eq!(
            value["refused"][0]["symbol"],
            "::sce_forge_runtime::codec::CodecError::PresentIfMismatch"
        );
        assert_eq!(value["refused"][0]["error"], "present-if-mismatch");

        // As the render derives it: the C++ backend prints this codec.
        let mut printed = checked.clone();
        if let FixtureSpec::Codec { cpp_printed, .. } = &mut printed.spec {
            *cpp_printed = true;
        }
        let (folded, value) = fold(&printed, &mismatch, Language::Cpp).expect("C++ states it");
        assert!(folded);
        assert_eq!(
            value["refused"][0]["symbol"],
            "::SCE::Forge::CodecError::PresentIfMismatch"
        );

        let (folded, value) = fold(&checked, &mismatch, Language::Kotlin).expect("Kotlin: none");
        assert!(!folded && value.get("refused").is_none());

        // A refusal this codec's encode never answers, a name no refusal has,
        // and a codec whose encode refuses nothing.
        let never = fold(
            &checked,
            &reference_of("codec_present_if_checked", "inconsistent-source"),
            Language::Rust,
        )
        .expect_err("the encode answers no inconsistent source");
        assert!(
            never.contains("never answers 'inconsistent-source'"),
            "{never}"
        );
        let unknown = fold(
            &checked,
            &reference_of("codec_present_if_checked", "need-more-bytes"),
            Language::Rust,
        )
        .expect_err("a decode failure is no refusal");
        assert!(unknown.contains("unknown encode refusal"), "{unknown}");
        let total = fold(
            &fixture_of("codec_present_if_disjunction"),
            &serde_json::json!({ "codecs": { "codec_present_if_disjunction": { "refused": [{
                "source": { "flags": 0, "flags.wants_a || flags.wants_b": null },
                "error": "present-if-mismatch",
                "note": "a total encode refuses nothing",
            }] } } }),
            Language::Rust,
        )
        .expect_err("a total encode refuses nothing");
        assert!(
            total.contains("never answers 'present-if-mismatch'"),
            "{total}"
        );
    }

    /// An enum fragment must name the identifier the declaration emits,
    /// which only `enum_naming::variant_ident` knows. Every catalog fixture
    /// is snake case, a name every case filter maps the same way — so the
    /// catalog cannot tell a fragment that asks the owner from one that
    /// re-derives the spelling. A camelCase name can: C's owner snake-cases
    /// the type prefix (`CACHE_MODE_…`), and the fragment's own `upper`
    /// filter did not (`CACHEMODE_…`).
    #[test]
    fn an_enum_fragment_names_the_identifier_the_declaration_emits() {
        let root = tempfile::tempdir().expect("tempdir");
        let resource_dir = root.path().join("resources");
        std::fs::create_dir_all(&resource_dir).expect("resources dir");
        std::fs::create_dir_all(root.path().join("conformance")).expect("conformance dir");
        std::fs::write(
            root.path().join("conformance/numerical_reference.json"),
            r#"{"version": 1, "float_tolerance": 1e-9,
                "enums": {"cacheMode": {"variants": [
                    {"name": "readOnly", "value": 3},
                    {"name": "readWrite", "value": 7}]}}}"#,
        )
        .expect("write oracle");
        let manifest: Manifest = serde_json::from_str(
            r#"{"version": 1, "fixtures": [{
                "name": "cacheMode", "kind": "enum", "ref_section": "enums",
                "variants": [{"name": "readOnly", "value": 3},
                             {"name": "readWrite", "value": 7}]}]}"#,
        )
        .expect("manifest");
        manifest.validate().expect("manifest validates");
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");

        for lang in crate::forge::codegen_matrix::emitting(crate::forge::model::ForgeKind::Enum) {
            let rendered = render_harness(&manifest, lang, &template_base, &resource_dir)
                .unwrap_or_else(|e| panic!("{lang:?}: render failed: {e}"));
            for variant in ["readOnly", "readWrite"] {
                let ident = crate::forge::enum_naming::variant_ident(lang, "cacheMode", variant);
                assert!(
                    rendered.source.contains(&ident),
                    "{lang:?}: the harness does not name `{ident}`, the identifier the \
                     declaration emits for cacheMode.{variant}"
                );
            }
        }
    }

    /// The untagged forms of an algorithm's slots read each spelling as what it
    /// is: a value of an enum by `enum`, a value that may be absent by
    /// `optional`, a value of a union by `union`, and an answer that may be
    /// absent by `optional` beside its `compare`, which a scalar output, whose
    /// `type` is required, does not take.
    #[test]
    fn an_enum_and_an_optional_slot_are_read_as_themselves() {
        let args: Vec<AlgorithmArg> = serde_json::from_str(
            r#"["i64", {"enum": "date_kind"}, {"optional": "u8"}, {"union": "date_expression"}]"#,
        )
        .expect("the arguments parse");
        assert!(
            matches!(&args[3], AlgorithmArg::Union(u) if u.union_document == "date_expression" && u.shape.is_none())
        );
        assert!(matches!(args[0], AlgorithmArg::Scalar(CanonicalType::I64)));
        assert!(
            matches!(&args[1], AlgorithmArg::Enum(e) if e.enum_document == "date_kind" && e.variants.is_empty())
        );
        assert!(matches!(
            args[2],
            AlgorithmArg::Optional(OptionalArg {
                optional: CanonicalType::U8
            })
        ));
        let output: AlgorithmOutput =
            serde_json::from_str(r#"{"optional": "i64", "compare": "equality"}"#)
                .expect("the output parses");
        assert!(matches!(
            output,
            AlgorithmOutput::Optional(OptionalOutput {
                optional: CanonicalType::I64,
                compare: CompareMode::Equality
            })
        ));
    }

    /// One algorithm over `args` answering `output`, beside the enum fixture
    /// `date_kind`, as [`Manifest::validate`] judges it.
    fn validated_algorithm(args: &str, output: &str) -> Result<(), String> {
        let manifest: Manifest = serde_json::from_str(&format!(
            r#"{{"version": 1, "fixtures": [
                {{"name": "date_kind", "kind": "enum", "ref_section": "enums",
                  "variants": [{{"name": "none", "value": 0}}]}},
                {{"name": "resolve", "kind": "algorithm", "ref_section": "pure_functions",
                  "function": "resolve", "args": {args}, "output": {output}}}]}}"#
        ))
        .expect("the manifest parses");
        manifest.validate()
    }

    const OPTIONAL_DAY: &str = r#"{"optional": "i64", "compare": "equality"}"#;

    /// A value of an enum names an enum fixture of the manifest, the one its
    /// variants are derived from.
    #[test]
    fn an_enum_argument_names_an_enum_fixture_of_the_manifest() {
        validated_algorithm(r#"["i64", {"enum": "date_kind"}]"#, OPTIONAL_DAY)
            .expect("an enum fixture of the manifest is admitted");
        let refused = validated_algorithm(r#"["i64", {"enum": "no_such_enum"}]"#, OPTIONAL_DAY)
            .expect_err("an enum the manifest does not hold is refused");
        assert!(refused.contains("names no enum fixture"), "{refused}");
    }

    /// The variants of an enum argument are the document's, so a manifest that
    /// writes them is refused.
    #[test]
    fn an_enum_argument_that_writes_its_variants_is_refused() {
        let refused = validated_algorithm(
            r#"[{"enum": "date_kind", "variants": [{"name": "none", "ident": "None"}]}]"#,
            OPTIONAL_DAY,
        )
        .expect_err("written variants are refused");
        assert!(refused.contains("carries `variants`"), "{refused}");
    }

    /// An optional slot holds a number or a `bool`: a text or a byte string that
    /// may be absent is refused as an argument and as an answer.
    #[test]
    fn an_optional_text_or_byte_string_is_refused() {
        validated_algorithm(
            r#"[{"optional": "bool"}, {"optional": "u64"}]"#,
            OPTIONAL_DAY,
        )
        .expect("an optional number or bool is admitted");
        let argument = validated_algorithm(r#"[{"optional": "string"}]"#, OPTIONAL_DAY)
            .expect_err("an optional text argument is refused");
        assert!(
            argument.contains("must be a number or a bool"),
            "{argument}"
        );
        let answer = validated_algorithm(
            r#"["i64"]"#,
            r#"{"optional": "bytes", "compare": "equality"}"#,
        )
        .expect_err("an optional byte-string answer is refused");
        assert!(answer.contains("must be a number or a bool"), "{answer}");
    }

    /// An answer that may be absent is compared for equality; a tolerance has
    /// nothing to measure between an absent answer and a number.
    #[test]
    fn an_optional_answer_compared_within_a_tolerance_is_refused() {
        let refused = validated_algorithm(
            r#"["i64"]"#,
            r#"{"optional": "f64", "compare": "tolerance"}"#,
        )
        .expect_err("a tolerance on an optional answer is refused");
        assert!(refused.contains("compared for equality"), "{refused}");
    }

    /// The documents of the standard date expression the harness generates: the
    /// resolver, the union it takes, the tag's enum and the record of each case.
    const DATE_EXPRESSION_FIXTURES: [&str; 9] = [
        "resolve_date",
        "date_expression",
        "date_kind",
        "civil_date",
        "month_day",
        "date_shift",
        "date_weekday",
        "week_weekday",
        "month_offset_day",
    ];

    /// The catalog's fixtures named in `names`, alone.
    fn catalog_of(names: &[&str]) -> Manifest {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let catalog = Manifest::load(&manifest_path).expect("manifest must load and validate");
        let manifest = Manifest {
            version: catalog.version,
            description: String::new(),
            fixtures: catalog
                .fixtures
                .into_iter()
                .filter(|f| names.contains(&f.name.as_str()))
                .collect(),
        };
        assert_eq!(
            manifest.fixtures.len(),
            names.len(),
            "the catalog holds each of {names:?}"
        );
        manifest
    }

    /// `sce:std/time/resolve_date`, which takes a value of a union and answers
    /// one that may be absent, is scheduled and rendered on every backend's
    /// harness: the library promises it to every generator, and a backend that
    /// refused it would drop it from its harness without a word. The union is
    /// built case by case, each case by the identifier its unit declares, and
    /// the C11 slots are the struct the generator names (docs/adr/0092, Landing
    /// order step 6).
    #[test]
    fn an_algorithm_of_a_union_slot_reaches_every_harness() {
        let manifest = catalog_of(&DATE_EXPRESSION_FIXTURES);
        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        for &lang in Language::ALL
            .iter()
            .filter(|lang| crate::forge::codegen_matrix::emits_anything(**lang))
        {
            let rendered = render_harness(&manifest, lang, &template_base, &resource_dir)
                .unwrap_or_else(|e| panic!("{lang:?}: render failed: {e}"));
            let symbol = crate::forge::generator::forge_algorithm_symbol("resolve_date", lang);
            assert!(
                rendered.source.contains(&format!("{symbol}(")),
                "{lang:?}: the harness does not call `{symbol}`, so resolve_date was not \
                 scheduled for it"
            );
            let shape = read_union_shape(&manifest, &resource_dir, "date_expression", lang)
                .unwrap_or_else(|e| panic!("{lang:?}: the union's shape is not read: {e}"));
            for case in &shape.cases {
                assert!(
                    rendered.source.contains(&case.ident),
                    "{lang:?}: the harness does not name `{}`, the case {} of the union's unit",
                    case.ident,
                    case.variant
                );
            }
            if lang == Language::C11 {
                assert!(
                    rendered.source.contains("date_expression_t input_arg1;")
                        && rendered.source.contains("sce_optional_int64_t expected;"),
                    "C11: the case table does not hold the slots in the types the \
                     algorithm declares"
                );
            }
        }
    }

    /// A union's shape is its document's: one case per variant of the tag's
    /// enum, in the enum's order, each holding the record its document names
    /// with that record's fields, an empty case holding none, and the tag read
    /// by the member each unit declares (docs/adr/0092, sections 1.1, 7 and
    /// 10.1).
    #[test]
    fn a_union_shape_is_read_from_its_document_in_the_order_of_its_tag() {
        let manifest = catalog_of(&DATE_EXPRESSION_FIXTURES);
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");
        let shape = read_union_shape(&manifest, &resource_dir, "date_expression", Language::Rust)
            .expect("the standard date expression is a union");
        assert_eq!(shape.tag, "kind");
        assert_eq!(shape.tag_member, "kind");
        assert_eq!(shape.tag_enum, "date_kind");
        assert_eq!(shape.type_name, "DateExpression");
        assert_eq!(shape.cases_member, None);
        let cases: Vec<(&str, Option<&str>, Vec<&str>)> = shape
            .cases
            .iter()
            .map(|c| {
                (
                    c.variant.as_str(),
                    c.record.as_ref().map(|r| r.record.as_str()),
                    c.record
                        .iter()
                        .flat_map(|r| r.fields.iter().map(|f| f.name.as_str()))
                        .collect(),
                )
            })
            .collect();
        assert_eq!(
            cases,
            vec![
                ("none", None, vec![]),
                ("absolute", Some("civil_date"), vec!["year", "month", "day"]),
                ("monthDay", Some("month_day"), vec!["month", "day"]),
                ("relativeDays", Some("date_shift"), vec!["count"]),
                ("relativeWeeks", Some("date_shift"), vec!["count"]),
                ("relativeMonths", Some("date_shift"), vec!["count"]),
                ("relativeYears", Some("date_shift"), vec!["count"]),
                ("comingWeekday", Some("date_weekday"), vec!["weekday"]),
                ("previousWeekday", Some("date_weekday"), vec!["weekday"]),
                (
                    "weekdayOfWeek",
                    Some("week_weekday"),
                    vec!["weekday", "count"]
                ),
                (
                    "dayOfMonthOffset",
                    Some("month_offset_day"),
                    vec!["day", "count"]
                ),
            ]
        );
        assert_eq!(shape.cases[1].ident, "Absolute");
        assert_eq!(shape.cases[1].kind, "Absolute");
        let c11 = read_union_shape(&manifest, &resource_dir, "date_expression", Language::C11)
            .expect("the standard date expression is a union on C11");
        assert_eq!(c11.type_name, "date_expression_t");
        assert_eq!(c11.tag_member, "tag");
        assert_eq!(c11.cases_member.as_deref(), Some("as"));
        assert_eq!(c11.cases[2].ident, "month_day");
    }

    /// One algorithm over `args`, beside the enum fixture `date_kind` and the
    /// union fixture `date_expression`, as [`Manifest::validate`] judges it.
    fn validated_union_algorithm(args: &str, union_fixture: &str) -> Result<(), String> {
        let manifest: Manifest = serde_json::from_str(&format!(
            r#"{{"version": 1, "fixtures": [
                {{"name": "date_kind", "kind": "enum", "ref_section": "enums",
                  "variants": [{{"name": "none", "value": 0}}]}},
                {union_fixture},
                {{"name": "resolve", "kind": "algorithm", "ref_section": "pure_functions",
                  "function": "resolve", "args": {args}, "output": {OPTIONAL_DAY}}}]}}"#
        ))
        .expect("the manifest parses");
        manifest.validate()
    }

    const DATE_EXPRESSION_UNION: &str =
        r#"{"name": "date_expression", "kind": "union", "ref_section": "unions"}"#;

    /// A value of a union names a union fixture of the manifest, the one its
    /// cases are derived from; an enum fixture of that name is not one.
    #[test]
    fn a_union_argument_names_a_union_fixture_of_the_manifest() {
        validated_union_algorithm(
            r#"["i64", {"union": "date_expression"}]"#,
            DATE_EXPRESSION_UNION,
        )
        .expect("a union fixture of the manifest is admitted");
        let refused =
            validated_union_algorithm(r#"["i64", {"union": "date_kind"}]"#, DATE_EXPRESSION_UNION)
                .expect_err("an enum fixture is no union");
        assert!(refused.contains("names no union fixture"), "{refused}");
    }

    /// The cases of a union are its document's, so a manifest that writes them,
    /// on the union fixture or on an argument, is refused.
    #[test]
    fn a_union_whose_shape_is_written_is_refused() {
        let written = r#"{"document": "date_expression", "type_name": "DateExpression",
            "tag": "kind", "tag_member": "kind", "tag_enum": "date_kind", "cases": []}"#;
        let argument = validated_union_algorithm(
            &format!(r#"["i64", {{"union": "date_expression", "shape": {written}}}]"#),
            DATE_EXPRESSION_UNION,
        )
        .expect_err("a written shape on an argument is refused");
        assert!(argument.contains("carries `shape`"), "{argument}");
        let fixture = validated_union_algorithm(
            r#"["i64", {"union": "date_expression"}]"#,
            &format!(
                r#"{{"name": "date_expression", "kind": "union", "ref_section": "unions",
                    "shape": {written}}}"#
            ),
        )
        .expect_err("a written shape on the union fixture is refused");
        assert!(fixture.contains("union `shape` is derived"), "{fixture}");
    }

    /// Oracle cross-check: verify structural integrity between fixtures.json
    /// and numerical_reference.json.
    ///
    /// This catches:
    /// - Fixtures declared in the catalog but missing from the oracle data
    /// - Oracle entries with zero test cases (dead oracle data)
    /// - ref_section mismatch (fixture says "pure_functions" but oracle has
    ///   no such section or no matching entry)
    /// - Missing version/tolerance fields in oracle
    ///
    /// It does NOT verify the oracle values themselves — that requires
    /// executing generated code. But it ensures the two JSON manifests stay
    /// in sync structurally.
    #[test]
    fn oracle_cross_check() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");

        let oracle_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/forge/conformance/numerical_reference.json");
        let oracle_text =
            std::fs::read_to_string(&oracle_path).unwrap_or_else(|e| panic!("read oracle: {e}"));
        let oracle: serde_json::Value =
            serde_json::from_str(&oracle_text).unwrap_or_else(|e| panic!("parse oracle JSON: {e}"));

        // Structural checks on oracle root
        assert_eq!(
            oracle.get("version").and_then(|v| v.as_u64()),
            Some(1),
            "oracle must have version: 1"
        );
        assert!(
            oracle
                .get("float_tolerance")
                .and_then(|v| v.as_f64())
                .is_some(),
            "oracle must have float_tolerance field"
        );

        let mut failures: Vec<String> = Vec::new();

        for fixture in &manifest.fixtures {
            // `oracle_eligible: false` fixtures live in the harness purely
            // for compile-verification (golden include + Default::default()
            // probe) — they have no oracle entry by design. Skip them here
            // so a missing oracle row is not flagged as drift.
            if !fixture.oracle_eligible {
                continue;
            }
            let section_name = &fixture.ref_section;
            let fixture_name = &fixture.name;

            // Check that the oracle has the referenced section
            let section = match oracle.get(section_name) {
                Some(s) if s.is_object() => s,
                Some(_) => {
                    failures.push(format!(
                        "{fixture_name}: ref_section '{section_name}' exists in oracle but is not an object"
                    ));
                    continue;
                }
                None => {
                    failures.push(format!(
                        "{fixture_name}: ref_section '{section_name}' not found in oracle"
                    ));
                    continue;
                }
            };

            // Check that the fixture has an entry in its section
            let entry = match section.get(fixture_name) {
                Some(e) if e.is_object() => e,
                Some(_) => {
                    failures.push(format!(
                        "{fixture_name}: entry in oracle section '{section_name}' is not an object"
                    ));
                    continue;
                }
                None => {
                    failures.push(format!(
                        "{fixture_name}: not found in oracle section '{section_name}'"
                    ));
                    continue;
                }
            };

            // Check that the entry has at least one test case.
            //
            // The key that carries it depends on what the kind IS. A kind
            // with behaviour states inputs and expected outputs (`cases`), a
            // stateful one an ordered `sequence`, the timer kind its
            // `timers`. A DECLARATIVE kind — `enum`, `event-schema` — has no
            // inputs at all: what it declares IS the test data, so its list
            // is `variants` or `fields`.
            //
            // ⚠ THIS LIST WAS SHORT BY TWO AND THE CONSEQUENCE WAS A RED
            // NOBODY READ. The `enum` fixture landed on 2026-09-17 and left
            // this check failing — `enum_uds_nrc: oracle entry has no
            // 'cases', 'sequence', or 'timers' test data` — because the round
            // that added it ran the five conformance ARMS, saw them green,
            // and never ran the manifest validator that owns this assertion.
            // Measuring one axis and reporting the other is the failure, not
            // the missing key.
            const CASE_KEYS: [&str; 5] = ["cases", "sequence", "timers", "variants", "fields"];
            let has_test_data = CASE_KEYS.iter().any(|k| {
                entry
                    .get(*k)
                    .and_then(|v| v.as_array())
                    .is_some_and(|a| !a.is_empty())
            });
            if !has_test_data {
                failures.push(format!(
                    "{fixture_name}: oracle entry has none of {} — every fixture must \
                     carry its own test data, and which key holds it depends on whether \
                     the kind has behaviour (cases/sequence/timers) or only declares \
                     something (variants/fields)",
                    CASE_KEYS.join(", ")
                ));
            }
        }

        if !failures.is_empty() {
            panic!(
                "Oracle cross-check failed ({} error(s)):\n  {}",
                failures.len(),
                failures.join("\n  ")
            );
        }
    }

    /// An enum's variants are stated three times and must be the same three:
    /// by the document that declares them, by the manifest (`fixtures.json`,
    /// which the harnesses name each variant's identifier from), and by the
    /// oracle (`numerical_reference.json`, which the C11 harness bakes in and
    /// the others read at run time).
    ///
    /// The manifest and the oracle are written apart from the document on
    /// purpose: a number that is on a wire must not be read back from the
    /// declaration it checks, or an edit moves both and nothing fails. The cost
    /// of that is three places to rename one word, and the harness for one
    /// language was the first to notice (measured 2026-10-21: renaming a
    /// status variant failed `forge-go` at link time, twice, after the other
    /// two statements were fixed one at a time). This is the test that notices
    /// first, and says which statement differs and where.
    #[test]
    fn an_enum_is_stated_the_same_by_its_document_the_manifest_and_the_oracle() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let manifest = Manifest::load(&root.join("tests/forge/conformance/fixtures.json"))
            .expect("manifest must load and validate");
        let oracle: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("tests/forge/conformance/numerical_reference.json"))
                .expect("read oracle"),
        )
        .expect("parse oracle JSON");
        let resource_dir = root.join("tests/forge/resources");

        let mut checked = 0;
        let mut failures: Vec<String> = Vec::new();
        for fixture in &manifest.fixtures {
            let FixtureSpec::Enum { variants } = &fixture.spec else {
                continue;
            };
            checked += 1;
            let stated: Vec<(String, i128)> = variants
                .iter()
                .map(|v| (v.name.clone(), i128::from(v.value)))
                .collect();

            let in_oracle: Option<Vec<(String, i128)>> = oracle
                .get(&fixture.ref_section)
                .and_then(|section| section.get(&fixture.name))
                .and_then(|entry| entry.get("variants"))
                .and_then(|rows| rows.as_array())
                .map(|rows| {
                    rows.iter()
                        .map(|row| {
                            (
                                row["name"].as_str().unwrap_or("<no name>").to_string(),
                                row["value"].as_i64().map_or(i128::MIN, i128::from),
                            )
                        })
                        .collect()
                });

            let path = fixture.document_path(&resource_dir);
            let declared: Result<Vec<(String, i128)>, String> =
                crate::load_forge_source(&path, &[])
                    .map_err(|e| format!("cannot read {}: {e}", path.display()))
                    .and_then(|loaded| {
                        crate::forge::parser::parse_forge_with_imports(
                            &loaded.positions.expanded,
                            crate::DocumentLabel::symmetric(&fixture.name),
                        )
                        .map_err(|e| format!("{}: {e}", path.display()))
                    })
                    .and_then(|parsed| match parsed.map(|p| p.document) {
                        Some(crate::forge::model::ForgeDocument::Enum(model)) => Ok(model
                            .variants
                            .iter()
                            .map(|v| (v.name.clone(), v.value))
                            .collect()),
                        _ => Err(format!("{} is not an enum document", path.display())),
                    });

            match declared {
                Err(why) => failures.push(format!("{}: {why}", fixture.name)),
                Ok(declared) => {
                    if declared != stated {
                        failures.push(format!(
                            "{}: the document declares {declared:?} but the manifest \
                             (tests/forge/conformance/fixtures.json) states {stated:?}",
                            fixture.name
                        ));
                    }
                    match &in_oracle {
                        None => failures.push(format!(
                            "{}: the oracle has no `variants` under {}.{} \
                             (tests/forge/conformance/numerical_reference.json)",
                            fixture.name, fixture.ref_section, fixture.name
                        )),
                        Some(oracle_rows) if *oracle_rows != declared => failures.push(format!(
                            "{}: the document declares {declared:?} but the oracle \
                             (tests/forge/conformance/numerical_reference.json) states \
                             {oracle_rows:?}",
                            fixture.name
                        )),
                        Some(_) => {}
                    }
                }
            }
        }

        // A floor, not an equality: adding an enum must not have to touch this
        // number, but a manifest that lost its enums must not pass on none.
        assert!(
            checked >= 8,
            "only {checked} enum fixture(s) were compared; the manifest lists more"
        );
        assert!(
            failures.is_empty(),
            "{} disagreement(s) between the document, the manifest and the oracle on what an \
             enum declares:\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }

    /// Compile gate: verify that every generated Rust conformance fixture
    /// and the rendered harness are syntactically valid Rust.
    ///
    /// Uses `syn::parse_file` to parse the generated Rust source. This
    /// catches:
    /// - Invalid syntax from template rendering (wrong braces, missing
    ///   semicolons, unclosed blocks)
    /// - Type reference typos (`Strig` instead of `String`)
    /// - Malformed `use` / `mod` statements
    /// - Broken macro invocations
    ///
    /// It does NOT verify semantic correctness (type checking, borrow
    /// checking) — that requires `rustc`. But syntax validation catches
    /// the most common template-level regressions without requiring a
    /// full toolchain or external build system.
    #[test]
    fn rust_conformance_compile_gate() {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/conformance/fixtures.json");
        let manifest = Manifest::load(&manifest_path).expect("manifest must load and validate");

        let template_base =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/codegen/templates");
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/forge/resources");

        // The options the Rust harness generates its fixtures with
        // (`backends/rust/forge-conformance/build.rs`): with views, which a codec
        // whose list states no bound requires (docs/adr/0077 §7).
        let options = crate::ForgeCompileOptions {
            views: true,
            ..crate::ForgeCompileOptions::default()
        };
        let mut failures: Vec<String> = Vec::new();

        // Gate 1: per-fixture generated Rust code
        for fixture in &manifest.fixtures {
            // A fixture the language refuses by name is not scheduled for it, so the
            // harness never asks for its code: the same predicate the harness reads
            // (a codec whose lines are records waits on each backend, docs/adr/0014).
            match lang_supports_fixture(fixture, Language::Rust, &resource_dir) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(e) => {
                    failures.push(format!("{}: schedule: {e}", fixture.name));
                    continue;
                }
            }
            let scxml_path = fixture.document_path(&resource_dir);
            let base_dir = scxml_path
                .parent()
                .map_or_else(|| resource_dir.clone(), Path::to_path_buf);
            // Read + expand: a fixture using `<sce:use>` must be checked
            // as the author wrote it, not as the parser would see it with
            // the expansion pass skipped.
            let content = match crate::load_forge_source(&scxml_path, &[]) {
                Ok(loaded) => loaded.positions.expanded,
                Err(e) => {
                    failures.push(format!("{}: read SCXML: {e}", fixture.name));
                    continue;
                }
            };

            let output = match crate::compile_forge_with_imports(
                &content,
                crate::DocumentLabel::symmetric(&fixture.name),
                Language::Rust,
                &base_dir,
                &options,
            ) {
                Ok(o) => o,
                Err(e) => {
                    failures.push(format!("{}: codegen: {e}", fixture.name));
                    continue;
                }
            };

            for (filename, code) in &output.files {
                if let Err(e) = syn::parse_file(code) {
                    failures.push(format!("{}/{filename}: syn parse error: {e}", fixture.name));
                }
            }
        }

        // Gate 2: rendered harness template
        match render_harness(&manifest, Language::Rust, &template_base, &resource_dir) {
            Ok(harness) => {
                if let Err(e) = syn::parse_file(&harness.source) {
                    failures.push(format!(
                        "harness ({}): syn parse error: {e}",
                        harness_filename(Language::Rust)
                    ));
                }
            }
            Err(e) => {
                failures.push(format!("harness render: {e}"));
            }
        }

        if !failures.is_empty() {
            panic!(
                "Rust conformance compile gate failed ({} error(s)):\n  {}",
                failures.len(),
                failures.join("\n  ")
            );
        }
    }
}
