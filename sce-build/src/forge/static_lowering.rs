// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The static data model, lowered for one backend (docs/SCE_ACCEPTED_SUBSET.md
// §2.15).
//
// [`crate::forge::static_datamodel`] judges a `sce-static` document; this
// rewrites the one a backend renders. Every expression goes through the forge
// expression lowerer against the same scope the judge used
// ([`crate::forge::type_ctx::StaticScope`]), and lands in a generate-time slot
// every backend's templates read before their own spellings: a transition's
// guard in `native_guard`, an `<if>`'s condition in `native_cond`, a whole
// statement in `native_code`. So a `sce-static` machine needs no template of
// its own beyond its variables' declarations — which this returns.
//
// ⚠ ONE walk for every backend. What differs between backends is a spelling —
// how a variable is named and reached, how a record field is replaced, how a
// list grows, how an execution error is raised — and each such question is a
// method of [`StaticTarget`]. What a document means, and what it is refused
// for, is decided once, here and in the judge, whatever the backend.

use std::collections::{BTreeSet, HashMap};

use crate::filters;
use crate::forge::error::GenerateError;
use crate::forge::expr::{
    transpile_into_owned, transpile_into_receiving, ExprTarget, Receiving, Refusal,
};
use crate::forge::model::{EnumModel, EventSchemaModel, FieldType, SceType, SchemaField};
use crate::forge::narrowing::{Chain, Facts, Writes};
use crate::forge::static_datamodel::{indexed_element, is_null_literal, proving};
use crate::forge::type_ctx::{
    CalleeKind, StateArg, StateCallRefusal, StateContext, StateRewrite, StaticCallee, StaticEnum,
    StaticScope, StaticSlot,
};
use crate::forge::types::{CallReading, InferredType};
use crate::generator::Language;
use crate::model::{Action, SCXMLModel};

/// One variable of a `sce-static` machine as the backend declares it: its
/// document id, the field name generated code spells, the backend type, and
/// the lowered initial value.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticField {
    pub id: String,
    pub name: String,
    pub ty: String,
    pub init: String,
    /// Declared `sce:direction="out"`: the host reads it, and the snapshot
    /// carries it. Every other variable is the machine's own, so that renaming
    /// one never changes what a host was written against.
    pub published: bool,
    /// The type a host reads the field through when it is not handed the
    /// value itself — a Rust list as a slice of its elements. `None` where
    /// the field is read as it is.
    pub view: Option<String>,
    /// The expression a host's reader answers, when the field is not read as it
    /// is — a string a target holds in a buffer, answered as the buffer's text.
    /// `None` where the reader answers the field.
    pub read: Option<String>,
    /// The bound of a list, a byte string or a string — elements, bytes or
    /// UTF-8 bytes: the machine never holds more, so neither may a restored
    /// value.
    pub bound: Option<u32>,
    /// How a saved state holds the value: `scalar`, `enum`, `list`, `record`,
    /// `record_list`, or `optional` for a value that may be absent, which a
    /// saved state holds as `null` or as its present value (docs/adr/0055).
    pub saved_kind: &'static str,
    /// What [`Self::saved_kind`] is of: a scalar's or a list element's
    /// `sce:type` spelling (`uint8`, `bool`, `bytes`, …), or a record's or an
    /// enum's backend type; for an optional value, what its present value is
    /// of, by the same rule. A backend without overloading on type reads each
    /// value with the function this names.
    pub saved_type: String,
    /// For an optional value, how a saved state holds its present value:
    /// `scalar`, or `enum` for a value of an enum the machine declares, which is
    /// written and read by its own type as a variable of the enum is
    /// ([`Self::saved_type`] is then that type). `None` for every other kind.
    pub saved_elem: Option<&'static str>,
    /// The name a host reads this field through when the field's own name is
    /// not it ([`StaticTarget::reader_name`]) — `None` for a name the backend
    /// cannot give a reader.
    pub reader: Option<String>,
    /// A record, or a list of records, whose schema has a byte-string field: what
    /// a host reads holds the machine's own bytes unless it is a copy, which a
    /// target whose byte buffer can be written into hands out
    /// (docs/adr/0005, decision 2).
    pub holds_bytes: bool,
    /// A parent's `<param>` or `namelist` may hand this variable a value
    /// (§scxml-6.4.1): a bool, a number or a text, and an optional value of one
    /// of them. A list, a record, an enum and bytes are not seeded, nor is an
    /// optional value of an enum, bytes or a record; the static judge refuses a
    /// value handed to one (the statechart judge's `seed_slot` is the one rule
    /// the judge and the lowering share). The child's `InvokeParams` carries one
    /// slot for each, and the slot of an optional variable is the optional type
    /// itself, so a seed that is absent is a value not handed.
    pub seeded: bool,
}

/// What lowering a `sce-static` machine produced beyond the rewritten model.
#[derive(Debug, Clone, Default)]
pub struct StaticLowering {
    /// The machine's fields, in declaration order.
    pub fields: Vec<StaticField>,
    /// Events whose typed payload a lowered expression reads — the payload
    /// channel must carry them (see
    /// [`crate::forge::generator::build_kotlin_event_payload`] and its Rust
    /// twin).
    pub payload_events: BTreeSet<String>,
    /// One type declaration per enum an `enum:<alias>` variable names and per
    /// event-schema a `record:<alias>` variable names, declared in the
    /// machine's own file the way its event payload types are.
    pub type_defs: Vec<String>,
    /// One adapter for each way a call of an imported algorithm that takes or
    /// returns a record or a list is made (docs/SCE_ACCEPTED_SUBSET.md §2.15), declared
    /// after the machine's payload types, which an adapter that takes an event's
    /// payload names.
    pub adapter_defs: Vec<String>,
    /// The annotations the machine's file begins with, before its package, for
    /// what it uses of the code it calls — Kotlin's opt-in to the unsigned arrays
    /// an algorithm's list is returned in.
    pub file_annotations: Vec<String>,
    /// The import line of each algorithm the document calls — the line a
    /// forge kind importing the same algorithm writes, so the machine reaches
    /// the function where the algorithm's own generation put it.
    pub imports: Vec<String>,
    /// The record types of [`Self::type_defs`], field by field — what a
    /// saved state writes a record value as.
    pub records: Vec<StaticRecord>,
    /// The enum types of [`Self::type_defs`], variant by variant — what a
    /// saved state writes an enum value as, for a backend that writes it
    /// outside the type ([`StaticEnumType`]).
    pub enums: Vec<StaticEnumType>,
    /// What a saved state of this machine is bound to ([`saved_shape`]), or
    /// `None` for a machine whose state a saved state cannot yet hold.
    pub saved_shape: Option<SavedShape>,
    /// Every expression the walk lowered, with the attribute the document
    /// wrote it in. A backend that renders the rewritten model reads the
    /// slots instead; a lowering that rewrites the document itself
    /// ([`crate::forge::static_js`]) reads these.
    pub sites: Vec<LoweredSite>,
    /// Every element the walk lowered to another — a `<sce:append>` to the
    /// `<assign>` that does it, a list or record `<data>` to the `<data>` that
    /// holds its initial value — for the same readers as [`Self::sites`].
    pub elements: Vec<LoweredElement>,
}

/// One element of the document, as the walk lowered it: which element, by an
/// attribute it carries, and the element that stands in its place.
///
/// Named by an attribute because that is what the model keeps of where an
/// element was written ([`crate::attribute_spelling`]); the element is the one
/// that owns it.
#[derive(Debug, Clone)]
pub struct LoweredElement {
    /// An attribute of the element, as the document wrote it. `None` for a
    /// model no document produced, which no edit may be placed against.
    pub anchor: Option<crate::attribute_spelling::AttributeSpelling>,
    /// The element that replaces it, in the target's own language.
    pub text: String,
}

/// One expression of the document, as the walk lowered it: the text the
/// author wrote, the attribute it sits in, and its lowering — the value alone,
/// before any statement, guard or failure handling is wrapped around it.
#[derive(Debug, Clone)]
pub struct LoweredSite {
    /// The expression as the model holds it, decoded.
    pub source: String,
    /// Where the document wrote it, for a model a document produced. `None`
    /// for one none did, and for an attribute whose written form does not
    /// decode to [`Self::source`] — a pass rewrote it after the parse — which
    /// no edit may be placed against.
    pub spelling: Option<crate::attribute_spelling::AttributeSpelling>,
    /// The lowering, in the target's own language.
    pub text: String,
}

impl LoweredSite {
    fn new(
        source: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        text: &str,
    ) -> Self {
        Self {
            source: source.to_string(),
            spelling: spelling.filter(|s| s.spells(source)).cloned(),
            text: text.to_string(),
        }
    }
}

/// A record type a `sce-static` machine declares, as a saved state writes it:
/// one member per schema field, keyed by the field's id as the schema writes
/// it, in the schema's order.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticRecord {
    /// The backend type ([`StaticTarget::record_type`]).
    pub ty: String,
    pub fields: Vec<StaticRecordField>,
}

/// What a saved state of a machine is bound to (SCE Accepted Subset §2.15,
/// "Saving and restoring"; docs/adr/0044): the document's shape, and the
/// revisions of each schema a record variable of it is of.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedShape {
    /// The shape a state names when every record schema it holds is in its
    /// first revision ([`saved_shape`]).
    pub first: String,
    /// The shape a state names when it names a later revision of any record
    /// schema: the digest of the first shape's text and one line more, so a
    /// reader that knows only the first shape refuses it.
    pub revised: String,
    /// [`Self::first`] as a format-2 saved state names it (docs/adr/0045,
    /// section 2): the same text with each enum-typed variable and record
    /// field spelled by its enum's document key, not by the alias it is
    /// imported under. Equal to [`Self::first`] for a document that holds no
    /// enum.
    pub format2_first: String,
    /// [`Self::revised`] as a format-2 saved state names it.
    pub format2_revised: String,
    /// Each record schema a variable is of, by the event it declares, ordered
    /// by it.
    pub records: Vec<SavedRecordLineage>,
    /// Each enum a value of the state is of — a variable's, or a record
    /// field's — by its key (`enum:` and its document's key), ordered by it.
    pub enums: Vec<SavedEnumLineage>,
}

impl SavedShape {
    /// What a format-2 saved state of the machine is bound to and names its
    /// enums by ([`SavedFormat2Keys`]).
    pub fn format2_keys(&self) -> SavedFormat2Keys {
        SavedFormat2Keys {
            first: self.format2_first.clone(),
            revised: self.format2_revised.clone(),
            enum_keys: self.enums.iter().map(|e| e.key.clone()).collect(),
        }
    }
}

/// The texts a format-2 saved state of a machine is bound to (docs/adr/0045,
/// section 2), as a backend whose templates do not read the whole
/// [`SavedShape`] declares them: the two format-2 shapes and the key of each
/// enum a value of the state is of. The generator computes them once, so every
/// backend's constant is the same text.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedFormat2Keys {
    /// [`SavedShape::format2_first`].
    pub first: String,
    /// [`SavedShape::format2_revised`].
    pub revised: String,
    /// [`SavedEnumLineage::key`] of each of [`SavedShape::enums`], in order.
    pub enum_keys: Vec<String>,
}

/// An enum a value of a machine's state is of, as a saved state names it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedEnumLineage {
    /// `enum:` and the key of the enum's document
    /// ([`crate::forge::schema_revision::document_key`]), which names it in
    /// `revisions`.
    pub key: String,
    /// The enum's `sce:revision` (1 when it declares none).
    pub revision: u32,
    /// The digest of what each revision from the second to [`Self::revision`]
    /// declares, in order ([`enum_revision_digest`]); empty for an enum of one
    /// revision.
    pub revisions: Vec<String>,
    /// Each variant a revision past the first added, in the enum's order.
    pub added: Vec<SavedAddedVariant>,
    /// Where a value of it is held, in declaration order.
    pub holders: Vec<SavedEnumHolder>,
}

/// A variant a revision past the first added to an enum.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedAddedVariant {
    /// The variant's declared name, which a saved state holds it by.
    pub name: String,
    /// The revision that added it.
    pub since: u32,
}

/// Where a value of a [`SavedEnumLineage`] is held.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedEnumHolder {
    /// The variable's document id.
    pub variable: String,
    /// The field of the record the variable holds, for a value held in a
    /// record; `None` for a variable of the enum.
    pub field: Option<String>,
    /// Whether the variable holds a list of such records rather than one.
    pub list: bool,
}

/// A record schema a machine's variables are of, as a saved state names it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedRecordLineage {
    /// The event the schema declares, which names it.
    pub event: String,
    /// The schema's `sce:revision` (1 when it declares none).
    pub revision: u32,
    /// The digest of what each revision from the second to [`Self::revision`]
    /// declares, in order ([`revision_digest`]); empty for a schema of one
    /// revision.
    pub revisions: Vec<String>,
    /// Each field a revision past the first added, in the schema's order.
    pub added: Vec<SavedAddedField>,
    /// Each field a revision widened from `T` to `optional<T>`
    /// (`sce:optional-since`), in the schema's order: a saved record written
    /// before the widening holds it as a value, so a `null` there is refused
    /// (docs/adr/0055, section 6).
    pub widened: Vec<SavedWidenedField>,
    /// The variables that hold a record of it, or a list of them, in
    /// declaration order.
    pub holders: Vec<SavedRecordHolder>,
}

/// A field a revision widened from `T` to `optional<T>`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedWidenedField {
    /// The field's id, the member a saved record holds it under.
    pub id: String,
    /// The revision that added the field (1 for a field of the first revision):
    /// a record saved before it holds no member for the field.
    pub since: u32,
    /// The revision that widened it (`sce:optional-since`): a record saved at
    /// an earlier revision, from `since` on, holds the field as a value of `T`.
    pub optional_since: u32,
}

/// A field a revision past the first added to a record schema.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedAddedField {
    /// The field's id.
    pub id: String,
    /// The revision that added it.
    pub since: u32,
    /// Its default as saved-state JSON: the JSON a delivery carries for it
    /// ([`crate::forge::schema_revision::default_wire`]), which is how a saved
    /// state writes a value of every type a field can have; `null` for an
    /// optional field that declares none.
    pub default: String,
}

/// A variable that holds a record of a [`SavedRecordLineage`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct SavedRecordHolder {
    /// The variable's document id.
    pub variable: String,
    /// Whether it holds a list of records rather than one.
    pub list: bool,
}

/// One field of a [`StaticRecord`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticRecordField {
    /// The schema's id — the key a saved state writes it under, the same on
    /// every backend.
    pub id: String,
    /// The backend identifier ([`StaticTarget::record_field`]).
    pub name: String,
    /// The field's `sce:type` spelling (see [`StaticField::saved_type`]); for an
    /// optional field, its present value's.
    pub saved_type: String,
    /// The most UTF-8 bytes a `string` field holds, or the most bytes a `bytes`
    /// field does, which a saved state being restored is held to as the machine is
    /// (the `sce:max-size` its schema declares); `None` for a field that is neither.
    pub bound: Option<u32>,
    /// Whether the field is optional (docs/adr/0055): a saved state holds it as
    /// `null` or as its present value, which is read as [`Self::saved_type`]
    /// and [`Self::bound`] say.
    pub optional: bool,
    /// The revision that added the field (`sce:since`), `None` for a field of the
    /// first revision. A saved record written at an earlier revision holds no
    /// member for it, and a backend that reads the record itself, rather than
    /// from a state grown by the runtime, takes [`Self::default`] for it.
    pub since: Option<u32>,
    /// The default of a field a later revision added, as saved-state JSON
    /// ([`SavedAddedField::default`]); `None` for a field of the first revision.
    pub default: Option<String>,
}

/// An enum type a `sce-static` machine declares, as a saved state writes it:
/// the variant's declared name, the same on every backend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticEnumType {
    /// The backend type ([`StaticTarget::enum_type`]).
    pub ty: String,
    /// The enum's alias, which a refusal of a saved value names.
    pub alias: String,
    pub variants: Vec<StaticEnumVariant>,
}

/// One variant of a [`StaticEnumType`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticEnumVariant {
    /// The name the enum document declares — the saved form.
    pub declared: String,
    /// The backend identifier ([`crate::forge::enum_naming::variant_ident`]).
    pub ident: String,
}

/// How a list of scalars is declared by a target that sizes it when the machine
/// is built ([`StaticTarget::bounded_list`]).
#[derive(Debug, Clone)]
pub struct BoundedList {
    /// The type a variable of the list is held in.
    pub ty: String,
    /// The declaration of [`Self::ty`], when the machine's own file is to make
    /// it. Made once however many variables name the type.
    pub def: Option<String>,
    /// The empty list, as an initial value of [`Self::ty`].
    pub empty: String,
}

/// How a string variable is held by a target whose own string type has no room
/// to hold one bounded by its capacity ([`StaticTarget::string_storage`]).
#[derive(Debug, Clone)]
pub struct StringStorage {
    /// The type a variable of the string is held in.
    pub ty: String,
    /// The declaration of [`Self::ty`], made once however many variables name
    /// the type.
    pub def: String,
    /// The initial value, built from the literal the variable starts at.
    pub init: String,
    /// The type a host reads the string through.
    pub view: String,
}

/// An imported callee as a target reaches it: the name a call is written with,
/// and the line that makes the name visible where the machine is.
#[derive(Debug, Clone)]
pub struct Callee {
    /// The callee, qualified as the generated unit that holds it requires.
    pub call: String,
    /// How the machine reaches the unit the callee is in ([`CalleeImport`]).
    pub import: CalleeImport,
}

/// What makes an imported callee's name visible where the machine is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalleeImport {
    /// The import line a forge kind importing the same document writes, so the
    /// machine reaches the function where the document's own generation put it.
    Line(String),
    /// No line: the callee is where the machine is already. A Swift unit is a
    /// file of the module the machine is a file of, and a module's files see one
    /// another's declarations; the Interpreter's ecmascript carries the function
    /// in the document.
    InScope,
    /// The target was built without what the line needs — a Go target without
    /// the module path its packages live under — and a call is all it can then
    /// spell: [`lower`] refuses a document that calls the callee rather than
    /// leave the name unimported.
    Unwritable,
}

/// [`Callee`] for a generated-code backend: the identity and symbol every
/// other forge kind derives for the same document, so a statechart and a
/// forge kind that imports it agree on where it lives. A function is called by
/// the name a forge kind importing the document calls it by
/// ([`crate::forge::generator::StatelessFunction::qualified_call`]); a validator
/// is built for the call and asked once
/// ([`crate::forge::generator::validator_check_call`]). Both are reached through
/// the import a stateless kind is: the module, never a type of it, so no name of
/// the document stands beside the machine's own.
///
/// `go_module_prefix` is what the Go import path is rooted at, and the one
/// thing of the identity that is not the document's own: the namespace a call
/// is qualified with is the package's name, which it is not. Any other backend
/// ignores it.
fn generated_callee(
    lang: Language,
    callee: &StaticCallee,
    go_module_prefix: Option<&str>,
) -> Callee {
    let options = crate::ForgeCompileOptions {
        go_module_prefix: go_module_prefix.map(str::to_owned),
        ..Default::default()
    };
    let call = |namespace: &str| match &callee.kind {
        CalleeKind::Function(function) => function.qualified_call(namespace, &lang),
        CalleeKind::Validator => {
            crate::forge::generator::validator_check_call(&callee.document_name, namespace, lang)
        }
    };
    // The identity of a Go document cannot be derived without the root its
    // path is written under, and the call does not need it: the package is
    // named for the document alone.
    if matches!(lang, Language::Go) && go_module_prefix.is_none() {
        let package = filters::to_snake_case(callee.document_name.clone());
        return Callee {
            call: call(&package),
            import: CalleeImport::Unwritable,
        };
    }
    let identity = crate::forge::generator::forge_import_identity(
        &callee.document_name,
        &lang,
        false,
        &options,
    );
    Callee {
        call: call(&identity.namespace),
        import: match lang {
            Language::Swift => CalleeImport::InScope,
            Language::Rust
            | Language::Cpp
            | Language::Kotlin
            | Language::Go
            | Language::Python
            | Language::C11 => CalleeImport::Line(identity.include_stmt),
        },
    }
}

/// The schema the record `alias` of `spec` is of, when every field of it is a number, a
/// truth value, a string the schema bounds or a value of an enum: the records a call
/// hands an algorithm and takes back are of those, whose copy is a field-by-field one on
/// every backend. One with a byte string field, or a string with no bound, is refused
/// where the algorithm is imported ([`crate::forge::static_imports`]), as is one whose
/// enum is not the document the statechart holds the field in.
fn plain_schema<'a>(spec: &AdapterSpec<'a>, alias: &str) -> Option<&'a EventSchemaModel> {
    // An optional field is held in the optional type of the machine's record and the
    // algorithm's, which an adapter copies where it is present
    // (docs/adr/0055), where the backend spells an algorithm's optional value of its
    // element ([`crate::forge::generator::algorithm_optional_spelled`]): a number or
    // a `bool` on every generated backend, and the rest as its own step 9 spells it.
    spec.schemas.get(alias).filter(|schema| {
        schema.fields.iter().all(|f| match &f.sce_type {
            FieldType::Scalar(ty) => record_field_copies(ty, f.max_size),
            // ... and a text that may be absent is bounded by the schema as a
            // text is, since the algorithm's record holds it in a buffer of that
            // bound.
            FieldType::Optional { elem } => {
                (!matches!(elem, SceType::String) || f.max_size.is_some())
                    && match spec.lang {
                        Some(lang) => crate::forge::generator::algorithm_optional_spelled(
                            lang,
                            &crate::forge::model::OptionalElemType::Scalar(elem.clone()),
                        ),
                        None => true,
                    }
            }
        })
    })
}

/// What a machine's declaration of its bounds says of itself
/// ([`StaticTarget::limits_def`]).
const LIMITS_DOC: &str =
    "The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050).";

/// What the constant of the bound `name` says of itself.
fn limit_doc(name: &str) -> String {
    format!("The bound `sce:limit=\"{name}\"` names, as this build resolved it.")
}

/// The names of the functions a value of the machine's own type `ty` for an enum
/// crosses through, into the enum document's type and back
/// ([`StaticTarget::enum_crossing_names`]): one spelling, named by the type, which
/// carries the machine's name, on every target that names them.
fn enum_crossing_function_names(ty: &str) -> (String, String) {
    (format!("{ty}ToDocument"), format!("{ty}FromDocument"))
}

/// Why a field of an event schema cannot be a property of the struct Swift holds a
/// record, or the payload of an event, in: a name Swift reserves, or a field that may
/// be absent over an element Swift does not spell (a byte string). `None` for a field
/// it writes. The one answer for a record a machine holds and for an event's payload,
/// which are the same struct of the same fields.
fn swift_field_unwritten(field: &crate::forge::model::SchemaField) -> Option<&'static str> {
    if matches!(
        &field.sce_type,
        FieldType::Optional { elem }
            if !crate::forge::generator::algorithm_optional_spelled(
                Language::Swift,
                &crate::forge::model::OptionalElemType::Scalar(elem.clone()),
            )
    ) {
        Some("an optional byte string")
    } else if crate::reader_names::is_reserved_word(Language::Swift, &field.id) {
        Some("a name Swift reserves")
    } else {
        None
    }
}

/// Whether `ty` is a number or a truth value (see [`plain_schema`]).
pub(crate) fn plain_scalar(ty: &SceType) -> bool {
    !matches!(ty, SceType::Enum(_) | SceType::String | SceType::Bytes)
}

/// Whether a field of a record handed to an algorithm, and back, is one an adapter
/// copies: a number or a truth value as it is, a string the schema bounds as the
/// text the algorithm's record holds within the same bound (SCE_FORGE.md §4.12), and
/// a value of an enum as the variant of the same declared name in the other side's
/// type ([`StaticTarget::enum_crossing`]).
pub(crate) fn record_field_copies(ty: &SceType, max_size: Option<u32>) -> bool {
    plain_scalar(ty)
        || (matches!(ty, SceType::String) && max_size.is_some())
        || matches!(ty, SceType::Enum(_))
}

/// One parameter of an adapter: a number the call hands the algorithm as it is, or the
/// record or list the machine holds, as the machine spells it.
pub enum AdapterParam<'a> {
    Scalar(&'a SceType),
    /// A value that may be absent, of a number or a `bool` (docs/adr/0055): held
    /// in the same optional type on both sides, so the adapter hands it on as it
    /// is ([`AdapterSpec::optional_type`]).
    Optional(&'a SceType),
    /// A state argument, held as `code` ([`crate::forge::type_ctx::STATE_CODES`]).
    State {
        slot: &'a StaticSlot,
        code: char,
    },
}

/// What a target needs to write the adapter of a call of an algorithm that takes or
/// returns a record or a list.
pub struct AdapterSpec<'a> {
    /// The name the adapter is declared and called by ([`StaticTarget::state_adapter_name`]).
    pub name: String,
    pub machine: &'a str,
    /// The backend the adapter is written for, which spells an algorithm's
    /// optional value of an element or does not
    /// ([`crate::forge::generator::algorithm_optional_spelled`]); `None` for a
    /// target that is no generated backend.
    pub lang: Option<Language>,
    /// The algorithm's function, as the target calls it ([`Callee::call`]).
    pub call: String,
    pub callee: &'a StaticCallee,
    pub params: Vec<AdapterParam<'a>>,
    /// Every schema the document imports, by the alias it imports it under.
    pub schemas: &'a std::collections::BTreeMap<String, EventSchemaModel>,
    /// The alias of the schema of the event each payload is held as: the payload
    /// struct of an argument held as `p` is that event's.
    pub payload_events: &'a std::collections::BTreeMap<String, String>,
    /// The largest bound any list of records of the schema `alias` has in the
    /// machine, for a target that copies such a list into a buffer of a bound.
    pub record_list_bounds: &'a std::collections::BTreeMap<String, u32>,
    /// Every enum the machine holds a value of, by the name it holds it by
    /// ([`SCXMLModel::held_enums`]): the variants a value of one crosses by
    /// ([`StaticTarget::enum_crossing`]).
    pub enums: &'a std::collections::BTreeMap<String, EnumModel>,
}

impl AdapterSpec<'_> {
    /// The type `target` holds a value that may be absent of the number or `bool`
    /// `ty` in, in the adapter of this call (docs/adr/0055): the one the machine's
    /// variable of it is held in ([`optional_present`], [`StaticTarget::optional_type`]),
    /// which the algorithm's slot is held in too, so it crosses as it is. `None`
    /// for a target with no spelling of it.
    fn optional_type(&self, target: &dyn StaticTarget, ty: &SceType) -> Option<String> {
        target.optional_type(&optional_present(target, self.machine, ty, None))
    }

    /// `value`, a value of the enum the document imports as `alias`, as the variant of
    /// the same declared name the other side holds ([`StaticTarget::enum_crossing`]);
    /// `None` where the target has no such spelling or the document no such enum.
    fn enum_crossing(
        &self,
        target: &dyn StaticTarget,
        alias: &str,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let model = self.enums.get(alias)?;
        let machine_type = target.enum_type(self.machine, alias)?;
        target.enum_crossing(&machine_type, model, value, crossing)
    }

    /// `value`, a field of a record of the schema `alias` read from the side
    /// `crossing` leaves, as the other side's record holds it: a value of an enum by
    /// [`Self::enum_crossing`], one that may be absent by
    /// [`Self::optional_enum_crossing`], any other field as it is (`None` for an
    /// enum field this target cannot spell). `text` spells the copy of a field that
    /// is no enum.
    fn record_field_crossing(
        &self,
        target: &dyn StaticTarget,
        field: &crate::forge::model::SchemaField,
        value: &str,
        crossing: Crossing,
        text: impl FnOnce(&str) -> String,
    ) -> Option<String> {
        match &field.sce_type {
            // The present value of an optional enum crosses by the same declared
            // name, mapped where it is present and absent where it is absent
            // (docs/adr/0055, step 9).
            FieldType::Optional {
                elem: ty @ SceType::Enum(_),
            } => self.optional_enum_crossing(target, ty, value, crossing),
            FieldType::Scalar(SceType::Enum(reference)) => {
                self.enum_crossing(target, &reference.alias, value, crossing)
            }
            FieldType::Scalar(_) | FieldType::Optional { .. } => Some(text(value)),
        }
    }

    /// `value`, an optional value of the enum `ty` names, as the other side's optional
    /// value of the variant of the same declared name where it is present
    /// ([`Self::enum_crossing`] applied through [`StaticTarget::optional_map`]), and
    /// absent where it is absent (docs/adr/0055). `None` for a `ty` that is no enum
    /// and for a target with no spelling of the mapping or of the optional type, which
    /// refuses the call by name.
    fn optional_enum_crossing(
        &self,
        target: &dyn StaticTarget,
        ty: &SceType,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let SceType::Enum(reference) = ty else {
            return None;
        };
        let model = self.enums.get(&reference.alias)?;
        let machine_type = target.enum_type(self.machine, &reference.alias)?;
        // The two types the enum is held in: the machine's own, and the one the enum
        // document's generation declares, which the algorithm holds. The map is bound
        // to the one it leaves, and a target that names both (Go's
        // `optional.Convert`) takes the one it arrives at.
        let algorithm_type = crate::forge::generator::enum_qualified_type(&model.name, self.lang?);
        let (from, to) = match crossing {
            Crossing::ToAlgorithm => (machine_type.clone(), algorithm_type),
            Crossing::FromAlgorithm => (algorithm_type, machine_type.clone()),
        };
        // The variant mapping is asked once for its spelling before it is bound, so a
        // target with no spelling of the crossing refuses the adapter rather than
        // write an empty one.
        target.enum_crossing(&machine_type, model, "v", crossing)?;
        target.optional_convert(value, &from, &to, &|present| {
            target
                .enum_crossing(&machine_type, model, present, crossing)
                .unwrap_or_default()
        })
    }
}

/// Which way a value of an enum crosses an adapter (docs/SCE_ACCEPTED_SUBSET.md
/// §2.15): from the machine's own type for the enum into the type the enum document's
/// generation declares, which the algorithm's records and parameters hold, or back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crossing {
    ToAlgorithm,
    FromAlgorithm,
}

/// The alias of each enum a call of `value` hands a value of over or takes one back
/// in: a slot that is a value of one, and an enum field of a record, or of the
/// records of a list, a slot holds — each a value an adapter maps by declared name.
fn crossing_enums(
    value: &crate::forge::type_ctx::StaticValueSignature,
    records: &std::collections::BTreeMap<String, EventSchemaModel>,
) -> BTreeSet<String> {
    let mut aliases = BTreeSet::new();
    for slot in value.params.iter().chain(value.ret.iter()) {
        if let Some(alias) = slot.enum_alias() {
            aliases.insert(alias.to_string());
        }
        if let StaticSlot::Record(record)
        | StaticSlot::RecordList(record)
        | StaticSlot::OptionalRecord(record) = slot
        {
            for field in records.get(record).iter().flat_map(|s| s.fields.iter()) {
                if let SceType::Enum(reference) = field.sce_type.present_type() {
                    aliases.insert(reference.alias.clone());
                }
            }
        }
    }
    aliases
}

/// Each declared variant of `model` as the two sides of a crossing spell it in `lang`,
/// `(from, to)`: the machine's own type `machine_type`, and the type the enum
/// document's generation declares. A value crosses as the variant of the same declared
/// name, which the statechart and the algorithm read from one document
/// ([`crate::forge::static_imports`]), so every variant has the one on the other side.
fn enum_variant_pairs(
    lang: Language,
    machine_type: &str,
    model: &EnumModel,
    crossing: Crossing,
) -> Vec<(String, String)> {
    let algorithm_type = crate::forge::generator::enum_qualified_type(&model.name, lang);
    model
        .variants
        .iter()
        .map(|v| {
            let machine =
                crate::forge::enum_naming::variant_ref(lang, machine_type, &model.name, &v.name);
            let algorithm =
                crate::forge::enum_naming::variant_ref(lang, &algorithm_type, &model.name, &v.name);
            match crossing {
                Crossing::ToAlgorithm => (machine, algorithm),
                Crossing::FromAlgorithm => (algorithm, machine),
            }
        })
        .collect()
}

/// How one backend spells what a `sce-static` document says. The walk
/// ([`lower`]) asks; the answer is the only thing that differs by backend.
pub trait StaticTarget {
    /// What this target is called where a refusal names it (`Kotlin`, `Rust`).
    ///
    /// Not a [`Language`]: a target need not be a generated-code backend. The
    /// Interpreter's ecmascript lowering is one, and `Language` means a backend
    /// every forge kind and every conformance run must cover.
    fn name(&self) -> &'static str;
    /// Whether this target leaves a `<sce:action>` as the document wrote it and
    /// lowers each `<sce:arg>` expression where it stands — the Interpreter's
    /// ecmascript, whose engine evaluates the arguments and hands them to the
    /// host. A generated backend renders the whole call into a host trait
    /// instead (`forge::native_action`), and is not asked.
    fn lowers_host_action_arguments(&self) -> bool {
        false
    }
    /// How a call of the imported `callee` is spelled, and the line that imports
    /// it — or `None` for a target that does not reach callees of its kind yet,
    /// which refuses a document that calls one rather than leaving the name
    /// undefined.
    fn callee(&self, callee: &StaticCallee) -> Option<Callee>;
    /// What the machine declares beside its calls of `callee` for [`Self::callee`]
    /// to name, for a target whose call of it is not the imported function itself:
    /// C's reading of a lookup whose miss is an error
    /// ([`crate::forge::generator::c11_lookup_found`]). `None` for every other.
    fn callee_def(&self, _callee: &StaticCallee) -> Option<String> {
        None
    }
    /// Whether this target lowers a call of an imported algorithm that takes or
    /// returns a record or a list (docs/SCE_ACCEPTED_SUBSET.md §2.15, "State an
    /// algorithm takes and gives back"). A target that does not refuses the
    /// document that makes one, by name, rather than leave the call undefined.
    fn lowers_state_calls(&self) -> bool {
        false
    }
    /// Whether this target lowers a variable that holds an optional value
    /// (docs/adr/0055). A target that does not refuses the document that
    /// declares one, by name, rather than generate a machine whose variable has
    /// no type that can hold absence. The target's expression emitter answers
    /// ([`ExprTarget::lowers_optional`]), so the walk and the emitter cannot
    /// disagree; each answers `true` in the step that lowers the type on it, and
    /// a target that does writes every hook below that a refusal names.
    fn lowers_optional(&self) -> bool {
        self.expr_target().lowers_optional()
    }
    /// The type an optional value is held in, given `present`, the type its
    /// present value is held in here ([`optional_present`]: the scalar's type,
    /// the machine's own type for an enum, the buffer a target holds a bounded
    /// text in). The backend's one spelling of the type
    /// ([`crate::forge::generator::optional_of`]) by default; `None` for a
    /// target that has none, which refuses the document by name.
    fn optional_type(&self, present: &str) -> Option<String> {
        self.expr_target()
            .language()
            .and_then(|lang| crate::forge::generator::optional_of(lang, present))
    }
    /// The absent value of the optional value whose present value is held in
    /// `present`: what `null` is where the whole value an `<assign>`, a `<data>`
    /// or an `<sce:set>` writes is the literal `null`, the place's type known.
    /// (A `null` inside an expression is the emitter's, which is handed the
    /// type its place expects.) `None` for a target with no spelling yet, which
    /// refuses the document by name.
    fn optional_none(&self, _present: &str) -> Option<String> {
        None
    }
    /// `value`, a present value held in `present`, lifted into the optional
    /// value of its element: what a value of `T` is where the whole value an
    /// `<assign>`, a `<data>` or an `<sce:set>` writes to a place of
    /// `optional<T>` is one of `T` (docs/adr/0055, section 2). `value` is
    /// already what the place's present value is held as (a text held to its
    /// bound, the buffer a target holds it in). (A value lifted inside an
    /// expression is the emitter's, [`crate::forge::types::InferredType::lifted_into`].)
    /// `None` for a target with no spelling yet, which refuses the document by
    /// name.
    fn optional_some(&self, _present: &str, _value: &str) -> Option<String> {
        None
    }
    /// The declaration the machine's own unit makes of the optional type over
    /// `present` (the type of the element `elem`), once however many places hold
    /// one, after the declaration of `present` itself: C11's one struct per
    /// element. `None` for a target whose optional type is the language's or its
    /// runtime's own, which is every other.
    fn optional_def(&self, _present: &str, _elem: &SceType) -> Option<String> {
        None
    }
    /// The line that brings the runtime's optional type into scope in a unit
    /// that holds an optional value, for a target whose import is not read off
    /// the generated text. `None` for a target that needs none.
    fn optional_import(&self) -> Option<String> {
        None
    }
    /// Whether the definition of an enum names the runtime's optional type, so that
    /// a unit that defines one imports it whether or not it holds an optional
    /// value: Go's `<Enum>FromUnderlying` answers `optional.Of[<Enum>]`
    /// (docs/adr/0055, section 5). `false` for a target whose enum names none.
    fn enum_def_names_optional(&self) -> bool {
        false
    }
    /// Whether this target lowers an optional record, `optional<record:X>`
    /// (docs/adr/0055, step 9): a variable that starts absent, a record taken
    /// whole into it — from a record by name, a payload, an element of a list or
    /// what an imported algorithm returns —, `r.value.f` read where a test proves
    /// `r` present, and a field of it written there. The target's expression
    /// emitter answers ([`ExprTarget::lowers_optional_record`]), so the walk and
    /// the emitter cannot disagree; a target that answers `false` refuses the
    /// document that holds one by name, as `generate/unsupported-feature` ("an
    /// optional record has no <language> lowering yet"). One that answers `true`
    /// spells the hooks below beside the ones every optional value is lowered by
    /// ([`Self::optional_type`], [`Self::optional_none`],
    /// [`Self::optional_some`]), all handed the record's own type
    /// ([`Self::record_type`]) as the type of the present value, and reads an
    /// element of a list of records whole ([`Self::record_at`]).
    fn lowers_optional_record(&self) -> bool {
        self.expr_target().lowers_optional_record()
    }
    /// Whether this target spells an optional value of `elem` in the slots of an
    /// imported algorithm and in a record it hands one (docs/adr/0055, steps 8
    /// and 9): the one answer the generated backends give by language
    /// ([`crate::forge::generator::algorithm_optional_spelled`]), and the
    /// Interpreter's lowering, whose values are the objects the algorithm holds,
    /// always.
    fn holds_algorithm_optional(&self, elem: &crate::forge::model::OptionalElemType) -> bool {
        match self.expr_target().language() {
            Some(lang) => crate::forge::generator::algorithm_optional_spelled(lang, elem),
            None => true,
        }
    }
    /// Whether this target spells a host method's parameter of `optional<elem>`
    /// (docs/adr/0055, step 10): the type the host interface declares for an
    /// argument of a `<sce:action>` that may be absent. The generated backends
    /// answer by language ([`crate::forge::generator::host_optional_argument_spelled`]);
    /// the Interpreter hands its host the value its engine computes, `null` when
    /// the argument is absent, so it spells every element.
    fn holds_host_optional(&self, elem: &SceType) -> bool {
        match self.expr_target().language() {
            Some(lang) => crate::forge::generator::host_optional_argument_spelled(lang, elem),
            None => true,
        }
    }
    /// The declaration the machine's own unit makes of the optional type over
    /// the record type `present` ([`Self::record_type`]), once however many
    /// variables hold one, after the declaration of the record itself: C11's one
    /// struct per record. `None` for a target whose optional type is the
    /// language's or its runtime's own, which is every other
    /// (docs/adr/0055, step 9).
    fn optional_record_def(&self, _present: &str) -> Option<String> {
        None
    }
    /// The statement that writes `value` into the field `field` of the present
    /// value of the optional record at `target` — the one place a record held in
    /// an optional is changed, where a test proves it present
    /// (docs/adr/0055, step 9): `target` is what [`Self::variable_ref`] gave, and
    /// `field` the identifier [`Self::record_field`] gave. `None` for a target
    /// with no spelling, which refuses the document by name.
    fn assign_present_field(&self, _target: &str, _field: &str, _value: &str) -> Option<String> {
        None
    }
    /// [`Self::assign_present_field`] for a string field of the present value,
    /// `value` already held to its bound ([`Self::assign_string_field`]).
    fn assign_present_string_field(
        &self,
        target: &str,
        field: &str,
        value: &str,
    ) -> Option<String> {
        self.assign_present_field(target, field, value)
    }
    /// [`Self::assign_present_field`] for a byte-string field of the present
    /// value, `value` already held to its bound ([`Self::assign_bytes_field`]).
    fn assign_present_bytes_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        self.assign_present_field(target, field, value)
    }
    /// `value`, an optional value whose present value is held in `present`,
    /// with `map` applied to its present value — `map(v)` spells it for a
    /// present value named `v`, a name the target binds — and absent where it
    /// is absent: how a present text is held to the bound of the place it is
    /// written to. `map` is the statement's own expression, so a failure it
    /// records is the statement's. `None` for a target with no spelling yet,
    /// which refuses the document by name.
    fn optional_map(
        &self,
        _value: &str,
        _present: &str,
        _map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        None
    }
    /// `value`, an optional value whose present value is of the type `from`, as the
    /// optional value of the type `to`, with `convert` applied to its present value
    /// — `convert(v)` spells it for a present value named `v`, a name the target
    /// binds — and absent where it is absent: how an optional value of an enum
    /// crosses an adapter, the machine and the algorithm holding the enum in two
    /// types (docs/adr/0055, step 9). A target whose optional type is generic over
    /// the type of its present value maps it with [`Self::optional_map`] and needs
    /// only the type it leaves; one whose conversion must name both types (Go's
    /// `optional.Convert[T, U]`) overrides this.
    fn optional_convert(
        &self,
        value: &str,
        from: &str,
        _to: &str,
        convert: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        self.optional_map(value, from, convert)
    }
    /// `value`, an optional value whose present value is held in `present`, as
    /// the typed value this backend's wire helpers take
    /// ([`Self::wire_value`]): JSON `null` where it is absent, and `wire(v)`
    /// where it is present — `wire(v)` spells the wire value of a present value
    /// named `v`, a name the target binds (docs/adr/0055, section 4). What a
    /// `<send>`'s and a `<donedata>`'s `<param>` carry. `None` for a target with
    /// no spelling yet, which refuses the `<param>` by name.
    fn wire_optional(
        &self,
        _value: &str,
        _present: &str,
        _wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        None
    }
    /// How a target whose payload is untyped data reads the optional field
    /// `field` of the payload `accessor` as the optional value it is: `null`
    /// absent, a value of the field's element present (held to `variants` for an
    /// enum), and a delivery that lacks the key refused, since a field of the
    /// first revision is carried by every delivery, or read as its default
    /// ([`crate::forge::schema_revision::default_wire`], `null` when it declares
    /// none) for a field a later revision added (docs/adr/0055, section 4).
    /// `None` for a target whose generated lift reads the payload into a struct
    /// of its own, whose optional field an expression reads as it is.
    fn payload_optional_read(
        &self,
        _accessor: &str,
        _field: &SchemaField,
        _variants: &[&str],
    ) -> Option<String> {
        None
    }
    /// How the state argument `arg` is handed to the adapter of its call, given
    /// `held`: how this target spells the variable, payload or loop item that
    /// holds it — the field a variable renames to, the payload accessor, the
    /// loop's item.
    fn state_argument(&self, _machine: &str, _arg: &StateArg, _held: &str) -> String {
        unreachable!("asked of a target that lowers_state_calls() refused")
    }
    /// What an adapter gave back as a list — `value`, its failure already passed on —
    /// as the list variable of `capacity` elements holds it, or the capacity failure
    /// recorded where every checked operation records one ([`Self::list_within`], for a
    /// list that is already the machine's own kind).
    fn state_list_within(&self, _adapter: &str, _value: &str, _capacity: u32) -> String {
        unreachable!("asked of a target that lowers_state_calls() refused")
    }
    /// Whether a call is made through an adapter this target declares
    /// ([`Self::state_adapter`]). A target whose records and lists are already the
    /// algorithm's (the Interpreter's ecmascript: plain objects and arrays) calls the
    /// algorithm itself and declares none.
    fn has_state_adapters(&self) -> bool {
        true
    }
    /// The name the call named `coded` ([`state_call_name`]) of `callee` is called
    /// by in `machine`: its adapter's, declared with it, or the algorithm's own where
    /// the target has no adapters. A target whose adapters share a scope with another
    /// machine's names them by `machine`: a parent and the child it invokes may call
    /// one algorithm the same way, each through an adapter in its own types.
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        coded.to_string()
    }
    /// The declaration of the adapter of one call: a function that takes the
    /// machine's own values — its variables, its event payload — and the call's
    /// scalars, converts what the algorithm's types differ in, calls it, and gives
    /// the machine's own value back. `None` for a target with no such lowering.
    fn state_adapter(&self, _spec: &AdapterSpec<'_>) -> Option<String> {
        None
    }
    /// The line that brings the generated unit of the event-schema document
    /// `schema_name` into scope, which an adapter names the algorithm's record type
    /// through; empty for a target that needs none.
    fn schema_import(&self, _schema_name: &str) -> Option<String> {
        None
    }
    /// `value`, a value of the enum `model` on the side `crossing` leaves, as the
    /// variant of the same declared name on the other: the machine holds it in its
    /// own type, `machine_type` ([`Self::enum_type`]), and the algorithm in the type
    /// the enum document's own generation declares. `None` for a target with no such
    /// spelling, which refuses the call by name.
    fn enum_crossing(
        &self,
        _machine_type: &str,
        _model: &EnumModel,
        _value: &str,
        _crossing: Crossing,
    ) -> Option<String> {
        None
    }
    /// The line that brings the generated unit of the enum document `enum_name` into
    /// scope, which an adapter names the algorithm's enum type through; `None` for a
    /// target that needs none.
    fn enum_import(&self, _enum_name: &str) -> Option<String> {
        None
    }
    /// The declaration of the bounds `machine` names (`sce:limit`), each a typed
    /// constant of the number the build resolved it to, named by the machine and the
    /// bound's name: what a host holds a value to before it hands it over
    /// (docs/adr/0050). `limits` is never empty. `None` for a target that declares
    /// none: the Interpreter's ecmascript, whose document is the host's to read.
    fn limits_def(&self, _machine: &str, _limits: &[(String, u32)]) -> Option<String> {
        None
    }
    /// The names of the two functions a value of the machine's own type for an enum,
    /// `ty`, crosses through into the type the enum document's generation declares
    /// and back, where a function of another kind the machine calls holds a value of
    /// the enum (docs/adr/0051). `None` for a target whose machine holds the
    /// document's own type, across which a value passes as it is.
    fn enum_crossing_names(&self, _ty: &str) -> Option<(String, String)> {
        None
    }
    /// The declaration of the two functions [`Self::enum_crossing_names`] names, each
    /// the crossing a call of an algorithm already makes ([`Self::enum_crossing`]) as a
    /// function of the machine's unit. `None` where the target names none.
    fn enum_crossing_def(&self, _ty: &str, _model: &EnumModel) -> Option<String> {
        None
    }
    /// Whether an adapter names the type the algorithm's unit gives a record it
    /// RETURNS, so that the unit of a schema only a return names is imported. Go
    /// reads such a record by its fields and builds the machine's own type, so it
    /// imports a package it would not use, which Go refuses; the others write the
    /// type or may import more than they name.
    fn adapter_names_returned_schema(&self) -> bool {
        true
    }
    /// The first construct of `model` this target has no lowering for yet,
    /// described for a refusal — or `None` when it lowers the whole document.
    /// Asked before the walk, so a target whose spellings for a construct are
    /// not written yet says so in one place instead of in each of them.
    fn unsupported(&self, _model: &SCXMLModel, _scope: &StaticScope) -> Option<String> {
        None
    }
    /// Whether a `<send>`'s `delayexpr` is lowered to the string it computes
    /// ([`Action::native_delay`]) that the backend reads as a CSS2 time when the
    /// send runs. A target that does not is refused where the `<send>` is
    /// walked, by name, rather than left to emit a send with no delay.
    fn lowers_delay_expr(&self) -> bool {
        false
    }
    /// Whether a `<send>`'s `eventexpr` is lowered to the string it computes
    /// ([`Action::native_event`]) that the backend resolves to the event it
    /// delivers when the send runs. A target that does not is refused where the
    /// `<send>` is walked, by name, rather than left to emit a send with no
    /// event.
    fn lowers_event_expr(&self) -> bool {
        false
    }
    /// What the `targetexpr` attribute of a `<send>` is rewritten to, for a
    /// target that runs the document's own attribute and so has no field of the
    /// machine to read the value from: `native_target`, the string the attribute
    /// computes, held to the `entries` the document declares — one that is none
    /// of them is the empty target, which the engine answers as it answers any
    /// address nobody is at. `None` for a target whose machine reads
    /// [`Action::native_target`] itself.
    fn target_expr_site(&self, _native_target: &str, _entries: &[String]) -> Option<String> {
        None
    }
    /// What the `typeexpr` attribute of a `<send>` is rewritten to, for a target
    /// that runs the document's own attribute (docs/adr/0005, decision 3):
    /// `native_type`, the string the attribute computes, held to the `entries` the
    /// document declares as `sce:types` — one that is none of them cannot be
    /// evaluated, which the engine answers as `error.execution` with nothing
    /// sent. `None` for a target that generates the choice: it is expanded where
    /// the `<send>` stands into one literal-type `<send>` for each entry
    /// ([`expand_computed_type`]), so each engine delivers by the processor its own
    /// arms already deliver a written `type` through.
    fn type_expr_site(&self, _native_type: &str, _entries: &[String]) -> Option<String> {
        None
    }
    /// Whether a `<cancel>`'s `sendidexpr` is lowered to the string it computes
    /// ([`Action::native_sendid`]) that the backend hands the scheduler when the
    /// cancel runs. A target that does not is refused where the `<cancel>` is
    /// walked, by name, rather than left to emit a cancel of nothing.
    fn lowers_cancel_expr(&self) -> bool {
        false
    }
    /// Whether the `<content expr>` of a `<send>` or of a `<final>`'s
    /// `<donedata>` that names one value, not a record, is lowered to the typed
    /// value the backend's wire helpers take ([`Action::native_content_value`],
    /// [`crate::model::DoneData::native_content_value`]) that the template
    /// carries as the event's data and, for a send, as text, as a host's
    /// `content`. A target that does not is refused where the element is walked,
    /// by name, rather than left to emit an event with no data.
    fn lowers_scalar_content(&self) -> bool {
        false
    }
    /// Whether a hybrid `<invoke>` is lowered: its `srcexpr` to the string it
    /// computes ([`crate::model::HybridInvokeInfo::native_src`]), the document
    /// stem of which the backend matches against the candidates it declares when
    /// the invocation starts, and the arguments to the values each candidate
    /// keeps ([`crate::model::InvokeCandidate::seeds`]). A target that does not
    /// is refused where the
    /// `<invoke>` is walked, by name, rather than left to start no child.
    fn lowers_hybrid_invoke(&self) -> bool {
        false
    }
    /// The id a start of a child session is started under,
    /// `<state>.<start>.<id>` (docs/adr/0045), as an owned string expression of
    /// this target read where the start writes it to the `<invoke>`'s
    /// `idlocation` ([`crate::model::InvokeSessionCommon::native_idlocation`]).
    /// `None` for a target that does not lower that write yet, which is refused
    /// where the `<invoke>` is walked, by name, rather than left to store
    /// nothing.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        None
    }
    /// Whether a Mesh request, `<invoke type="sce:mesh-rpc">`, is lowered by
    /// this target itself: its `<param>`s read from the machine's fields when the
    /// invocation starts and handed to the router the backend generates for the
    /// document's deployment (`docs/adr/0005`, decision 5). Only a backend with a
    /// router of its own does: every other one serves the request through its host
    /// ([`crate::host_processor_analyzer::lower_mesh`]), which has turned it into
    /// a host-served invoke before the walk, so a request that reaches a target
    /// that does not is one that has no route to be served by.
    fn lowers_mesh_invoke(&self) -> bool {
        false
    }
    /// What the `srcexpr` attribute of a hybrid `<invoke>` is rewritten to, for
    /// a target that runs the document's own attribute and so has no field of
    /// the machine to read the value from: `native_src`, the string the
    /// attribute computes, reduced to the candidate it names among `stems`.
    /// `None` for a target whose machine reads
    /// [`crate::model::HybridInvokeInfo::native_src`] itself.
    fn hybrid_src_site(&self, _native_src: &str, _stems: &[&str]) -> Option<String> {
        None
    }
    /// Whether the `srcexpr` and the `<content expr>` of an `<invoke>` a host
    /// runs are lowered to the string each computes
    /// ([`UnsupportedInvokeInfo::native_src`],
    /// [`UnsupportedInvokeInfo::native_content`]) that the backend hands the
    /// host as the request's `src` and `content` when the invocation starts. A
    /// target that does not is refused where the `<invoke>` is walked, by name,
    /// rather than left to start it with no `src` or `content`.
    fn lowers_host_src_expr(&self) -> bool {
        false
    }
    /// Whether a transition may read the payload of an event whose schema
    /// declares an enum field: the target holds the field in the machine's own
    /// type for the enum and reads it off the wire as the variant's declared
    /// name. A target that does not is refused where it would otherwise stop on
    /// the field ([`lower`]).
    fn payload_enum_fields(&self) -> bool {
        false
    }
    /// How a target whose payload is untyped data reads the enum field `field`
    /// of it — `accessor` is what [`Self::payload_accessor`] answers — as a
    /// call that refuses a value none of `variants` names. `None` for a target
    /// that holds the payload in a struct whose field has the enum's own type.
    fn payload_enum_read(
        &self,
        _accessor: &str,
        _field: &str,
        _variants: &[&str],
    ) -> Option<String> {
        None
    }
    /// How a target that holds a payload's byte-string field as a buffer and the
    /// length beside it reads the field `field` of the payload `accessor` as the
    /// one value an expression names — what a byte string held by the machine is
    /// read as. `None` for a target whose payload holds the byte string as a value.
    fn payload_bytes_read(&self, _accessor: &str, _field: &str) -> Option<String> {
        None
    }
    /// How a target whose payload is untyped data reads a field a later
    /// revision of its schema added: the field `field` of the payload
    /// `accessor`, held to its type (to `variants` for an enum field) as any
    /// field is, and read as `default` — the JSON its schema's default spells
    /// ([`crate::forge::schema_revision::default_wire`]) — where the delivery
    /// lacks it. `None` for a target whose generated lift filled the default
    /// into the payload it holds before any expression reads it.
    fn payload_revised_read(
        &self,
        _accessor: &str,
        _field: &SchemaField,
        _variants: &[&str],
        _default: &str,
    ) -> Option<String> {
        None
    }
    /// The expression lowerer's target.
    fn expr_target(&self) -> ExprTarget;
    /// The declared name of variable `id`'s field.
    fn field_name(&self, id: &str) -> String;
    /// The name a host reads variable `id` through, when that is not
    /// [`Self::field_name`]: the field's own name for a target whose member is
    /// the host's reader, and the author's spelling for one whose member
    /// carries a prefix. `None` for a name this target cannot spell a reader
    /// with.
    fn reader_name(&self, id: &str, _machine: &str) -> Option<String> {
        Some(self.field_name(id))
    }
    /// How a statement or a guard reaches the field named `name`.
    fn field_ref(&self, name: &str) -> String;
    /// How a variable's initial value reaches the field named `name` of a
    /// variable declared before it, while the machine is being built. The
    /// field's own name, for a target whose fields are in scope there; C
    /// builds the machine through a pointer to it.
    fn initial_field_ref(&self, name: &str) -> String {
        name.to_string()
    }
    /// How the machine's active-state test is called (`In(...)`).
    fn in_function(&self) -> &'static str;
    /// The type of a scalar variable.
    fn scalar_type(&self, ty: &SceType) -> String;
    /// The type a host reads a published scalar of `ty` through, when it is
    /// not the scalar's own type (see [`StaticField::view`]).
    fn scalar_view(&self, ty: &SceType) -> Option<String>;
    /// The type a record variable of `alias` is held in, declared in
    /// `machine`'s own file.
    fn record_type(&self, machine: &str, alias: &str) -> String;
    /// The declaration of [`Self::record_type`], one field per schema field in
    /// the schema's order.
    ///
    /// `enum_types` is the type each enum a field holds is declared as
    /// ([`Self::enum_type`]), by the name the field's enum is held by
    /// ([`SCXMLModel::held_enums`]).
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String;
    /// The identifier of schema field `id` on the record type.
    fn record_field(&self, id: &str) -> String;
    /// A record value built whole from `(field identifier, value)` pairs, one
    /// for every schema field, in the schema's order.
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String;
    /// The type a variable declared `enum:<alias>` is held in, declared in
    /// `machine`'s own file — or `None` for a target with no enum type of its
    /// own, which refuses the variable and every reference to a variant rather
    /// than leaving a name undefined.
    fn enum_type(&self, _machine: &str, _alias: &str) -> Option<String> {
        None
    }
    /// The declaration of [`Self::enum_type`]: the closed set of `model`'s
    /// variants, each spelled as [`Self::enum_variant`] does and carrying the
    /// name the enum document declares for it.
    fn enum_def(&self, _ty: &str, _alias: &str, _model: &EnumModel) -> String {
        String::new()
    }
    /// The functions `<alias>.toNumber(value)` and `<alias>.fromNumber(number)`
    /// are lowered to, as `(to_number, from_number)`, for the enum `model` the
    /// machine holds in its own type `ty` ([`Self::enum_type`]): each called with
    /// its one argument. `to_number` answers the number the value's variant
    /// declares, in the enum's carrier; `from_number` answers the variant a number
    /// declares, or that none does, in the shape [`crate::forge::expr`] hands the
    /// statement's failure channel — the same shape the enum document's own
    /// accessor answers in on this backend ([`crate::forge::generator::enum_number_symbols`]),
    /// so a statechart and an algorithm lower the two calls alike. What they call
    /// is declared by [`Self::enum_def`]. Required of every target, so one that
    /// declares an enum cannot leave the two calls undefined.
    fn enum_number_symbols(&self, ty: &str, model: &EnumModel) -> (String, String);
    /// The identifier a variant of the enum document `enum_name` has on `ty`,
    /// the [`Self::enum_type`] the machine holds it in — the declared name itself
    /// for a target whose data model holds no types, where the variant is that
    /// name.
    fn enum_variant(&self, _ty: &str, _enum_name: &str, variant: &str) -> String {
        variant.to_string()
    }
    /// The type of a list of `elem`.
    fn list_type(&self, elem: &SceType) -> String;
    /// The type a host reads a published list of `elem` through, when it is
    /// not the list's own type (see [`StaticField::view`]).
    fn list_view(&self, elem: &SceType) -> Option<String>;
    /// The type of a list of the record type `record` ([`Self::record_type`]).
    fn record_list_type(&self, record: &str) -> String;
    /// The type a host reads a published list of `record` through, when it is
    /// not the list's own type (see [`StaticField::view`]).
    fn record_list_view(&self, record: &str) -> Option<String>;
    /// The type a host reads a published record variable of the record type `ty`
    /// through, when it is not the record's own type (see [`StaticField::view`]):
    /// a record that owns text is lent, not moved out of the machine, as a string
    /// is. `None` for a record that is plain data and is read by value.
    fn record_view(&self, _ty: &str, _schema: &EventSchemaModel) -> Option<String> {
        None
    }
    /// A record `value` — a record variable or a loop's record item, named — as
    /// the copy a list or another variable holds: the name itself for a record
    /// that is plain data, and a clone of it for one that owns text.
    fn record_copy(&self, value: &str, _schema: &EventSchemaModel) -> String {
        value.to_string()
    }
    /// The statements `parts`, each lowered from an action that can fail, as the one
    /// statement of an action that runs them in order and stops at the first that
    /// fails. A statement that can fail is, for most targets, an expression that is
    /// `true` when it failed, so they are joined by `||`, which does not evaluate
    /// the rest once one has. A target whose such statement is a block that leaves
    /// by itself writes them one after another.
    fn in_sequence(&self, parts: &[String]) -> String {
        parts
            .iter()
            .map(|part| format!("({part})"))
            .collect::<Vec<_>>()
            .join(" || ")
    }
    /// The element of `list` at `index`, whole, as an expression that fails when the
    /// index is outside the list — for a target that holds a list as a value it can read
    /// an element of whole. `None` for a target whose record is a type no expression
    /// names: it copies the element into the record a field at a time.
    fn record_at(&self, _list: &str, _index: &str) -> Option<String> {
        None
    }
    /// The element of `list` at `index` as the copy a variable holds
    /// ([`Self::record_copy`] of [`Self::record_at`]), as an expression that fails
    /// when the index is outside the list: what an optional record takes into
    /// itself from a list's element (docs/adr/0055, step 9). A target whose copy of
    /// an element is not that composition — a Rust record that owns text is cloned
    /// out of the list by reference, and one that is plain data is read by value —
    /// spells its own. `None` for a target that reads no element whole.
    fn record_element(&self, list: &str, index: &str, schema: &EventSchemaModel) -> Option<String> {
        self.record_at(list, index)
            .map(|element| self.record_copy(&element, schema))
    }
    /// [`Self::receiving_write`] of a value that is a record of the type `held`,
    /// for a target that names the type of the local the value is computed into.
    /// The other targets' locals are typed by what they infer, as they were.
    fn receiving_write_of(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        _held: &str,
        failed: &str,
    ) -> String {
        self.receiving_write(write, value, InferredType::Unknown, failed)
    }
    /// How an expression reads the string field `field` of the record `record`
    /// (already spelled as the machine's field or the loop's item), for a target
    /// that holds such a string in a buffer of its own and reads its text through
    /// it. `None` for a target whose string field is the string itself.
    fn record_string_read(&self, _record: &str, _field: &str) -> Option<String> {
        None
    }
    /// The statement that writes `value` — already held to its bound — into the
    /// string field `field` of the record `target`.
    fn assign_string_field(&self, target: &str, field: &str, value: &str) -> String {
        self.assign_field(target, field, value)
    }
    /// A record's string field starting at the string literal `literal`, as the
    /// record is built whole: the literal itself, unless the target holds the
    /// field in a buffer of `bound` bytes.
    fn record_string_literal(&self, _bound: u32, literal: &str) -> String {
        literal.to_string()
    }
    /// A record's string field made of `text`, a string computed when the machine
    /// runs and already held to `bound` — a payload's field taken whole into a
    /// record: the text itself, unless the target holds the field in a buffer.
    fn record_string_runtime(&self, _bound: u32, text: &str) -> String {
        text.to_string()
    }
    /// A record's byte-string field starting at `value`, the bytes of a literal
    /// the judge held to `bound`, as the record is built whole: the value itself,
    /// unless the target holds the field in a buffer of `bound` bytes.
    fn record_bytes_literal(&self, _bound: u32, value: &str) -> String {
        value.to_string()
    }
    /// A record's byte-string field made of `bytes`, bytes computed when the
    /// machine runs and already held to `bound` — a payload's field taken whole
    /// into a record: the bytes themselves, unless the target holds the field in
    /// a buffer.
    fn record_bytes_runtime(&self, _bound: u32, bytes: &str) -> String {
        bytes.to_string()
    }
    /// The statement that writes `value` — already held to its bound — into the
    /// byte-string field `field` of the record `target`: the field's own
    /// assignment, unless the target holds it in a buffer it copies into.
    fn assign_bytes_field(&self, target: &str, field: &str, value: &str) -> String {
        self.assign_field(target, field, value)
    }
    /// An empty list.
    fn list_empty(&self) -> String;
    /// How a list of the scalar `elem`, bounded by `capacity`, is declared: its
    /// type, the declaration of that type when it is the target's own to make
    /// (made once however many variables name it), and its empty value.
    ///
    /// The default is [`Self::list_type`] and [`Self::list_empty`] with nothing
    /// declared, which is right for a list that grows to what it is given and
    /// holds its bound only where it is appended to. A target whose list is a
    /// buffer of a size fixed when the machine is built has to know the bound
    /// to name the type, and answers it here.
    fn bounded_list(&self, elem: &SceType, _capacity: Option<u32>) -> BoundedList {
        BoundedList {
            ty: self.list_type(elem),
            def: None,
            empty: self.list_empty(),
        }
    }
    /// [`Self::bounded_list`] for a list of the record type `record`
    /// ([`Self::record_type`]): the default is [`Self::record_list_type`] and
    /// [`Self::list_empty`] with nothing declared.
    fn bounded_record_list(&self, record: &str, _capacity: Option<u32>) -> BoundedList {
        BoundedList {
            ty: self.record_list_type(record),
            def: None,
            empty: self.list_empty(),
        }
    }
    /// [`Self::foreach_loop`] for a target that has to name the type of the item
    /// it binds and of the list it copies — C has no `auto` — handed both:
    /// `item_ty` is the element's type, a record's included, and `list_ty` the
    /// list's own ([`Self::bounded_list`], [`Self::bounded_record_list`]). The
    /// default is the untyped loop.
    fn foreach_loop_typed(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
        _item_ty: &str,
        _list_ty: &str,
    ) -> Option<(String, String)> {
        self.foreach_loop(list, item, index)
    }
    /// `value`, an owned string, as an expression that fails — as a checked
    /// operation does, so it is received as one is — when it is longer than
    /// `capacity` UTF-8 bytes, and is `value` itself otherwise: the bound a
    /// string variable keeps on every backend, so that a machine holds the same
    /// value wherever it runs.
    fn bounded_string(&self, value: &str, capacity: u32) -> String;
    /// [`Self::bounded_string`] for `value`, an owned byte string, held to
    /// `capacity` bytes: the bound a `bytes` variable keeps on every backend
    /// (docs/adr/0005, decision 2).
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String;
    /// How a string variable bounded by `capacity` UTF-8 bytes is held, starting
    /// at the literal `init` — for a target whose own string type holds no such
    /// bound, as a C buffer does not. `None` for a target whose strings own
    /// their storage, which is every other.
    fn string_storage(&self, _capacity: u32, _init: &str) -> Option<StringStorage> {
        None
    }
    /// [`Self::string_storage`] for a byte string bounded by `capacity` bytes,
    /// starting at `init`, the view of the literal it starts at: a buffer and the
    /// length it holds, which a host is lent as a view. `None` for a target whose
    /// own byte string owns its storage, which is every other.
    fn bytes_storage(&self, _capacity: u32, _init: &str) -> Option<StringStorage> {
        None
    }
    /// What an expression reads variable `var` as, given `field_ref`, the
    /// reference to the field that holds it. The reference itself for a value the
    /// field is; a string a target holds in a buffer is read through the buffer.
    fn variable_ref(&self, _var: &crate::model::Variable, field_ref: String) -> String {
        field_ref
    }
    /// `target = value` for a string variable, `target` being what
    /// [`Self::variable_ref`] gave. The assignment of any other value by default.
    fn assign_string(&self, target: &str, value: &str) -> String {
        self.assign(target, value)
    }
    /// `target = value` for a byte string, `value` being the bytes it was held to
    /// the bound as. The assignment of any other value by default.
    fn assign_bytes(&self, target: &str, value: &str) -> String {
        self.assign(target, value)
    }
    /// `target = value`.
    fn assign(&self, target: &str, value: &str) -> String;
    /// `target = <the list value holds>` for the list variable `target`, `value`
    /// being [`Self::list_within`] of what an imported algorithm returns. The
    /// assignment by default, which is right where that is the list variable's own
    /// kind of value.
    fn assign_list(&self, target: &str, value: &str) -> String {
        self.assign(target, value)
    }
    /// What an imported algorithm returned as a list — `value`, its failure
    /// already passed on — as the list variable of `capacity` elements holds it, or
    /// the capacity failure recorded where every checked operation records one. An
    /// algorithm's list may hold more than the bound it declared on a backend whose
    /// lists grow, and a machine holds the same list wherever it runs, so the list it
    /// takes is held to the variable's own bound. `symbol` is the callee as the
    /// target calls it, for a target whose helper is the algorithm's own.
    fn list_within(&self, value: &str, capacity: u32, symbol: &str, elem: &SceType) -> String;
    /// The annotation the machine's file begins with when it takes a list an
    /// algorithm returns: the opt-in to a type the call's result is held in, for a
    /// target whose language gates it. `None` for the others.
    fn file_annotation_for_list_call(&self) -> Option<String> {
        None
    }
    /// The type of the local a list `symbol` returns is held in while it is
    /// computed, for a target that names it — C has no `auto`. `None` for the
    /// other targets, whose locals are typed by what they infer.
    fn list_result_type(&self, _symbol: &str) -> Option<String> {
        None
    }
    /// The most bytes one list may take where it crosses a call of an algorithm, for a
    /// target that passes it by value on the stack. `None` for the others, whose lists
    /// live on the heap.
    fn list_stack_limit_bytes(&self) -> Option<u64> {
        None
    }
    /// The id the machine generates for a `<send idlocation>`, as an owned
    /// string expression evaluated where the send runs: `_auto_send_` and the
    /// number of ids this machine has generated, counted from one. The count is
    /// the machine's own, so two sends of a machine never hold one id, and a
    /// target that saves its state carries it through the save.
    fn fresh_send_id(&self) -> String;
    /// `target = <a fresh send id>` for the variable `target`, a string one when
    /// `is_string`, as [`Self::assign_string`] and [`Self::assign`] write it. The
    /// id is generated once: a target whose assignment reads its value twice
    /// (C's `memmove` and `strlen`) takes it into a local first.
    fn assign_fresh_send_id(&self, target: &str, is_string: bool) -> String {
        let id = self.fresh_send_id();
        if is_string {
            self.assign_string(target, &id)
        } else {
            self.assign(target, &id)
        }
    }
    /// Replace field `field` of the record at `target` with `value`.
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String;
    /// A target that rewrites the document itself rather than a model it
    /// renders: the `<assign>` that replaces field `field` of the record
    /// `record` with `value` as `(location, expr)` — the whole record, written
    /// again with that field changed. `None` for a target that spells it as a
    /// statement ([`Self::assign_field`]).
    fn field_assignment(
        &self,
        _record: &str,
        _field: &str,
        _value: &str,
    ) -> Option<(String, String)> {
        None
    }
    /// A target that rewrites the document itself: the `<data>` element that
    /// holds list or record variable `id` at its initial value `init`, which
    /// the document leaves to its own children and attributes. `None` for a
    /// target that declares the variable as a field.
    fn data_element(&self, _id: &str, _init: &str) -> Option<String> {
        None
    }
    /// Log `value`, of type `ty`, prefixed with `label` when there is one
    /// (§scxml-4.7). A target that formats by the value's type — C has no
    /// overloading — reads `ty`; the others show the value as the language does.
    fn log(&self, label: &str, value: &str, ty: InferredType) -> String;
    /// Append `value` to the list at `target` while it holds fewer than
    /// `capacity` elements, as an expression that is `true` when the append
    /// failed. It fails when the list is full — `overflow`, if any, runs, and
    /// nothing is appended — and, when `value_can_fail`, when computing
    /// `value` fails, where `failed` runs (both raise `error.execution`).
    /// Either way the statement has raised an error, and the block it stands
    /// in ends (§scxml-4.9), which is the dispatcher's to do.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String;
    /// Empty the list at `target`.
    fn clear(&self, target: &str) -> String;
    /// The loop a `<foreach>` over the list at `list` lowers to, as the head
    /// that opens it and the statements that open each iteration: `item`
    /// bound to each element, `index` — a `uint32`, when the document names one
    /// — to its position. The loop walks the list as it was when the loop
    /// began (§scxml-4.6: a shallow copy), so a body that appends to the list
    /// does not move it. `item` and `index` come spelled as the target spells a
    /// local (`ExprTarget::local_spelling`), the spelling the body's
    /// expressions read them by. `None` for a target that leaves a `<foreach>`
    /// as the document wrote it, as the Interpreter's ecmascript does.
    fn foreach_loop(
        &self,
        _list: &str,
        _item: &str,
        _index: Option<&str>,
    ) -> Option<(String, String)> {
        None
    }
    /// The head of a statement that runs a `<foreach>` body for the first
    /// element of the same copy [`Self::foreach_loop`] walks, binding the same
    /// loop variables, and runs nothing for an empty list: what a body that
    /// ends its block on every path is emitted under, since it never reaches
    /// a second element. Each iteration is opened by the prologue
    /// [`Self::foreach_loop`] gives. `None` for a target whose templates keep
    /// the loop for such a body.
    fn foreach_first(&self, _list: &str, _item: &str, _index: Option<&str>) -> Option<String> {
        None
    }
    /// Raise `error.execution` with `message` (§scxml-3.12.2).
    fn raise_execution_error(&self, machine: &str, message: &str) -> String;
    /// `statement`, whose expressions can fail (SCE_FORGE.md §3.4.1), as an
    /// expression that is `true` when it failed: it is run where its failure
    /// is received, a failure stops it before it writes anything, and
    /// `failed` runs instead (E12 D5). An error ends the block the element
    /// stands in (§scxml-4.9); which block that is, and what leaves it,
    /// the dispatcher knows from where it renders the action, so it acts on
    /// the `true`.
    ///
    /// A target with no expression that runs statements — C has no closure and
    /// no statement expression — spells the statement itself, and the block
    /// that ends the block in it, with the `return` its function ends on
    /// (§scxml-4.9: every block is a function there). Its dispatcher then has
    /// nothing to act on, and does not read [`crate::model::Action::native_fails`].
    fn receiving_statement(&self, statement: &str, failed: &str) -> String;
    /// `write(value)`, a statement that stores or shows `value`, as
    /// [`Self::receiving_statement`] receives it, for a `value` of type `ty`
    /// that can fail.
    ///
    /// The default is the statement with the value written in place, which is
    /// right where a failure leaves the statement before it writes — an early
    /// return, a throw. A target whose failed value is still a value (zero,
    /// with a flag raised) computes it first and writes only when it did not
    /// fail, so a statement that failed leaves what it was going to write
    /// as it was (§scxml-4.9). One that has to name the type of the value it
    /// holds in the meantime — C has no `auto` — reads `ty`.
    fn receiving_write(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        _ty: InferredType,
        failed: &str,
    ) -> String {
        self.receiving_statement(&write(value), failed)
    }
    /// `statement`, a call whose arguments can fail, run where its failure is
    /// received: a failure stops it before it happens, and `failed` runs
    /// instead. A statement, not an expression: the call sits inside the text
    /// a host action renders, which does not yet end its block on a failure.
    fn receiving_call(&self, statement: &str, failed: &str) -> String;
    /// [`Self::receiving_call`] for the host call `callee(args…)`, given as the
    /// finished `statement` and as the parts it was made of.
    ///
    /// The default is the statement, which is right where a failing argument
    /// leaves before the call is made. A target whose failed argument is still
    /// a value computes every argument first and calls only when none failed;
    /// one that has to name the type it holds each in meanwhile — C has no
    /// `auto` — reads `arg_types`, the declared type of each argument.
    fn receiving_host_call(
        &self,
        statement: &str,
        _callee: &str,
        _args: &[String],
        _arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        self.receiving_call(statement, failed)
    }
    /// A condition that can fail: its value, or `false` once `failed` has run
    /// and then `flag` (§scxml-5.9.1: a condition that cannot be evaluated is
    /// false, and `error.execution` says why). `flag` is
    /// [`Self::condition_failed_flag`] for the condition of an `<if>` or an
    /// `<elseif>`, and empty for a transition's guard, which stands in no
    /// block.
    ///
    /// A target with no expression that runs statements (see
    /// [`Self::receiving_statement`]) spells the head of the `if` instead: the
    /// statements that evaluate the condition, raising and setting `flag` when
    /// it fails, and then `if (<verdict>)`, which its dispatcher follows with
    /// the branch's block. The slot says which it holds
    /// ([`crate::model::Action::native_cond_fails`],
    /// [`crate::model::Transition::native_guard_fails`]).
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String;
    /// The statement that records, on the `<if>` numbered `if_ordinal`, that
    /// one of its conditions failed. The `<if>` runs its chain on — a
    /// condition that cannot be evaluated is false — and then ends its block
    /// (§scxml-4.9), as any element that raised does.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String;
    /// What `_event.data` is read through inside a guard or statement of an
    /// `event` carrying a typed payload.
    fn payload_accessor(&self, event: &str) -> String;
    /// `lowered`, a guard reading the payload, held to the delivery having
    /// carried one.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String;
    /// Whether [`Self::payload_guard`] is asked of the guard's value, before
    /// [`Self::receiving_condition`] receives its failure, rather than of the
    /// condition once received. The default — once received — is right where
    /// the received condition is an expression the check can wrap. A target
    /// whose failing condition is statements (C) takes it on the value.
    fn payload_guards_the_value(&self) -> bool {
        false
    }
    /// `value`, a lowered expression of type `ty` ([`InferredType::wire_param_slot`]
    /// admits the bool, string, narrow-integer and real types), as the typed
    /// value this backend's wire helpers take: the one a `<param>` crosses to a
    /// host as, text and JSON alike.
    fn wire_value(&self, ty: InferredType, value: &str) -> String;
    /// Whether this backend spells a value of `ty` for the wire
    /// ([`Self::wire_value`]). A type it does not is refused where the `<param>`
    /// is written, by name, rather than crossed as something no scenario holds it
    /// to. Every type [`InferredType::wire_param_slot`] admits, by default.
    fn wire_admits(&self, _ty: InferredType) -> bool {
        true
    }
    /// `value`, a lowered expression of a value of the enum imported as
    /// `alias`, as the string expression naming the variant it holds as its
    /// enum document declares it — what an enum value crosses a `<param>` as,
    /// and what a saved state holds. It is the string [`Self::wire_value`]
    /// takes for [`InferredType::Str`]. `None` for a backend with no such
    /// spelling, which refuses the `<param>` by name.
    fn enum_wire_name(&self, _alias: &str, _value: &str) -> Option<String> {
        None
    }
    /// `value`, a lowered expression of an owned byte string, as the typed wire
    /// value it crosses as: its byte-exact Latin-1 text, each byte the character
    /// of that code point (docs/adr/0005, decision 2) — the form
    /// [`Self::wire_value`] gives a string, in whichever type this backend carries
    /// one, so a text a request carries and the JSON value of the event's data are
    /// the same wherever the machine runs. `None` for a backend with no such
    /// spelling yet, which refuses the `<param>` by name.
    fn wire_bytes(&self, _value: &str) -> Option<String> {
        None
    }
}

/// The name a Kotlin `let` binds the present value of an optional value to
/// ([`StaticTarget::optional_map`], [`StaticTarget::wire_optional`]): a name no
/// document's variable or local is spelled as, since a variable is a property
/// named `v<PascalCase>` and a local keeps the author's spelling.
const KOTLIN_PRESENT: &str = "scePresent";

/// What the closure of a Swift `Optional.map` names the present value it is given: an
/// argument of its own, since a closure that has arguments cannot read the `$0` of the
/// one it is written in.
const SWIFT_PRESENT: &str = "scePresent";

/// Kotlin: a variable is a property of the machine class, a record an
/// immutable data class replaced field by field, a list an immutable `List`,
/// and an optional value (docs/adr/0055) a `T?`.
pub struct KotlinTarget;

impl KotlinTarget {
    /// The members a record with a byte-string field has beyond a data class's own,
    /// as the text between its `toSaved` and its companion — empty for one with none.
    ///
    /// A `ByteArray` is compared by identity, and a data class built over one
    /// inherits that, so two records of the same bytes would be two values: `equals`
    /// and `hashCode` read the bytes. And it can be written into, so `detached` is
    /// the record with a copy of each, which is what a host is handed: the machine
    /// never writes into an array, and a host that did would change the value
    /// behind the bound the machine keeps (docs/adr/0005, decision 2).
    fn record_bytes_members(&self, ty: &str, schema: &EventSchemaModel) -> String {
        if !holds_bytes(schema) {
            return String::new();
        }
        let is_bytes = |field: &crate::forge::model::SchemaField| {
            matches!(field.sce_type.present_type(), SceType::Bytes)
        };
        let equal: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let member = self.record_field(&field.id);
                if is_bytes(field) {
                    format!("{member}.contentEquals(other.{member})")
                } else {
                    format!("{member} == other.{member}")
                }
            })
            .collect();
        let hash = schema.fields.iter().fold("0".to_string(), |acc, field| {
            let member = self.record_field(&field.id);
            let each = if is_bytes(field) {
                format!("{member}.contentHashCode()")
            } else {
                format!("{member}.hashCode()")
            };
            format!("31 * ({acc}) + {each}")
        });
        let copies: Vec<String> = schema
            .fields
            .iter()
            .filter(|field| is_bytes(field))
            .map(|field| {
                let member = self.record_field(&field.id);
                // An optional byte string is copied where it is present
                // (docs/adr/0055); `contentEquals` and `contentHashCode` take an
                // absent one as they are.
                match field.sce_type {
                    FieldType::Optional { .. } => format!("{member} = {member}?.copyOf()"),
                    FieldType::Scalar(_) => format!("{member} = {member}.copyOf()"),
                }
            })
            .collect();
        format!(
            "\n\x20   override fun equals(other: Any?): Boolean =\n\
             \x20       other is {ty} && {equal}\n\n\
             \x20   override fun hashCode(): Int = {hash}\n\n\
             \x20   /** This value with a copy of each byte string, which a host may write into. */\n\
             \x20   fun detached(): {ty} = copy({copies})\n",
            equal = equal.join(" && "),
            copies = copies.join(", ")
        )
    }

    /// The arm of a failing expression: `after` run, when there is anything
    /// to run, and then `true` — the failure the expression answers.
    fn then_true(after: &str) -> String {
        if after.is_empty() {
            "true".to_string()
        } else {
            format!("{after}; true")
        }
    }
}

impl StaticTarget for KotlinTarget {
    // The start site holds the id it generates in the `String` local
    // `generatedInvokeId` (`deferStaticInvoke`), which it goes on to start the
    // child under, so the variable takes it as it stands.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("generatedInvokeId")
    }
    fn name(&self) -> &'static str {
        "Kotlin"
    }
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Kotlin, callee, None))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // `T?` (docs/adr/0055, section 5): the type is the default spelling
    // ([`crate::forge::generator::optional_of`]), absent is `null` and a present
    // value is the `T` itself.
    fn optional_none(&self, _present: &str) -> Option<String> {
        Some("null".to_string())
    }
    fn optional_some(&self, _present: &str, value: &str) -> Option<String> {
        Some(value.to_string())
    }
    // The present value is bound by a `let` the safe call runs only where the
    // value is present; what `map` throws is the statement's, received where
    // the statement stands.
    fn optional_map(
        &self,
        value: &str,
        _present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "({value})?.let {{ {KOTLIN_PRESENT} -> {} }}",
            map(KOTLIN_PRESENT)
        ))
    }
    // `null`, which `valueToJson` writes as JSON `null`, where the value is
    // absent.
    fn wire_optional(
        &self,
        value: &str,
        present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        self.optional_map(value, present, wire)
    }
    // An optional record is the record's own data class over `?`
    // (docs/adr/0055, step 9): `T?` over the type of `record_type`, which the
    // walk hands every hook above as the present type. A field of its present
    // value is written the way a record's field is, a data class replaced with
    // that field changed, the present value read by the one function a proven
    // read is spelled with, so the write names no `!!` a smart cast would turn
    // into a warning, and never names a value that is not there.
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(format!(
            "{target} = {}({target}).copy({field} = {value})",
            crate::forge::expr::KOTLIN_PROVEN
        ))
    }
    // The immutable `List` is read at an index by the runtime's checked read,
    // which throws `AlgorithmFailure` for an index outside the list, received
    // where the statement stands as any failure of one is.
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!(
            "com.sce.forge.runtime.SceChecked.at({list}, ({index}).toLong())"
        ))
    }
    fn state_argument(&self, _machine: &str, _arg: &StateArg, held: &str) -> String {
        held.to_string()
    }
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("call_{coded}")
    }
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("com.sce.forge.runtime.SceChecked.within({value}, {capacity})")
    }
    fn schema_import(&self, schema_name: &str) -> Option<String> {
        Some(
            crate::forge::generator::forge_import_identity(
                schema_name,
                &Language::Kotlin,
                false,
                &crate::ForgeCompileOptions::default(),
            )
            .include_stmt,
        )
    }
    // Both types are enum classes of the same declared variants, so a `when` over
    // every one of them is total.
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let arms: Vec<String> = enum_variant_pairs(Language::Kotlin, machine_type, model, crossing)
            .into_iter()
            .map(|(from, to)| format!("{from} -> {to}"))
            .collect();
        Some(format!("when ({value}) {{ {} }}", arms.join("; ")))
    }
    fn enum_import(&self, enum_name: &str) -> Option<String> {
        self.schema_import(enum_name)
    }
    // An object of the file, its constants unsigned as every bound is.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let constants: String = limits
            .iter()
            .map(|(name, value)| {
                format!(
                    "    /** {} */\n    const val {}: UInt = {value}u\n",
                    limit_doc(name),
                    name.to_uppercase()
                )
            })
            .collect();
        Some(format!(
            "/** {LIMITS_DOC} */\nobject {machine}Limits {{\n{constants}}}"
        ))
    }
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two functions of the file, each a `when` over every variant.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Kotlin);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "/** A value of [{ty}] as the variant of the same declared name of [{document}]. */\n\
             fun {to}(v: {ty}): {document} = {into}\n\n\
             /** A value of [{document}] as the variant of the same declared name of [{ty}]. */\n\
             fun {from}(v: {document}): {ty} = {back}"
        ))
    }
    // A function of the machine's file that takes the machine's values, builds the
    // algorithm's from them, calls it, and builds the machine's own from what it
    // returns, answering as the algorithm does: an `AlgorithmResult` where it may
    // fail, which the expression the machine lowers takes the value of.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{
            event_schema_field_ident, event_schema_payload_type, kotlin_array_type,
        };
        let value = spec.callee.value.as_ref()?;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        let algorithm_record = |alias: &str| {
            plain_schema(spec, alias)
                .map(|schema| event_schema_payload_type(&schema.name, Language::Kotlin))
        };
        // A record of `ty` built from `from`, a machine record, a payload or the
        // algorithm's record, field by field: an enum field crosses as the variant of
        // the same declared name the other side's type holds.
        let record_of =
            |alias: &str, ty: String, from: &str, crossing: Crossing| -> Option<String> {
                let schema = plain_schema(spec, alias)?;
                let fields = schema
                    .fields
                    .iter()
                    .map(|f| {
                        let field = event_schema_field_ident(&f.id, Language::Kotlin);
                        spec.record_field_crossing(
                            self,
                            f,
                            &format!("{from}.{field}"),
                            crossing,
                            str::to_string,
                        )
                        .map(|value| format!("{field} = {value}"))
                    })
                    .collect::<Option<Vec<String>>>()?;
                Some(format!("{ty}({})", fields.join(", ")))
            };
        // The algorithm's record built from `from`, a machine record or a payload.
        let to_algorithm = |alias: &str, from: &str| -> Option<String> {
            record_of(alias, algorithm_record(alias)?, from, Crossing::ToAlgorithm)
        };
        let from_algorithm = |alias: &str, from: &str| -> Option<String> {
            record_of(alias, machine_record(alias), from, Crossing::FromAlgorithm)
        };
        // The type the machine holds the record of `alias` in as the call's
        // argument is held as `code`: the payload class of the event the
        // transition is on for `p`, the machine's own record type for any other.
        let held_record = |alias: &str, code: char| -> Option<String> {
            if code == 'p' {
                let event = spec.payload_events.get(alias)?;
                Some(format!(
                    "{}{}Payload",
                    spec.machine,
                    filters::to_event_variant(event.to_string())
                ))
            } else {
                Some(machine_record(alias))
            }
        };
        let mut declarations = Vec::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "{name}: {}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) => {
                    if !plain_scalar(ty) {
                        return None;
                    }
                    declarations.push(format!("{name}: {}", self.scalar_type(ty)));
                    arguments.push(name);
                }
                // An optional value of an enum is the machine's `T?` over its own enum
                // class, and crosses where it is present by the declared name of its
                // variant (docs/adr/0055).
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    declarations.push(format!("{name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(spec.optional_enum_crossing(
                        self,
                        ty,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // `T?` on both sides (docs/adr/0055).
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("{name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    declarations.push(format!("{name}: {}", held_record(alias, *code)?));
                    arguments.push(to_algorithm(alias, &name)?);
                }
                // An optional record (docs/adr/0055, step 9) is held by the name of
                // the variable that holds it, which is an optional record variable,
                // a record variable (a present value, lifted) or the literal `null`
                // (`o`, `v` and `n`: one adapter serves the three, its parameter the
                // machine's `T?`, which a `T` and `null` stand in as they are), or
                // by the payload or the loop's item (`p`, `i`: a record, lifted).
                // The algorithm's record is built where the value is present.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => match *code {
                    'p' | 'i' => {
                        declarations.push(format!("{name}: {}", held_record(alias, *code)?));
                        arguments.push(to_algorithm(alias, &name)?);
                    }
                    _ => {
                        declarations.push(format!(
                            "{name}: {}",
                            self.optional_type(&machine_record(alias))?
                        ));
                        arguments.push(format!("{name}?.let {{ {} }}", to_algorithm(alias, "it")?));
                    }
                },
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => {
                    declarations.push(format!("{name}: {}", self.list_type(ty)));
                    arguments.push(format!("{name}.to{}()", kotlin_array_type(ty)));
                }
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    declarations.push(format!(
                        "{name}: {}",
                        self.record_list_type(&machine_record(alias))
                    ));
                    arguments.push(format!("{name}.map {{ {} }}", to_algorithm(alias, "it")?));
                }
                _ => return None,
            }
        }
        let converted = |from: &str| -> Option<(String, String)> {
            Some(match value.ret.as_ref()? {
                StaticSlot::Scalar(SceType::Enum(reference)) => (
                    self.enum_type(spec.machine, &reference.alias)?,
                    spec.enum_crossing(self, &reference.alias, from, Crossing::FromAlgorithm)?,
                ),
                StaticSlot::Scalar(ty) if plain_scalar(ty) => {
                    (self.scalar_type(ty), from.to_string())
                }
                StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => (
                    spec.optional_type(self, ty)?,
                    spec.optional_enum_crossing(self, ty, from, Crossing::FromAlgorithm)?,
                ),
                StaticSlot::Optional(ty) => (spec.optional_type(self, ty)?, from.to_string()),
                // The machine's record is built where the algorithm's answer is
                // present, and an absent one stays `null` (docs/adr/0055, step 9).
                StaticSlot::OptionalRecord(alias) => (
                    self.optional_type(&machine_record(alias))?,
                    format!("{from}?.let {{ {} }}", from_algorithm(alias, "it")?),
                ),
                StaticSlot::Record(alias) => (machine_record(alias), from_algorithm(alias, from)?),
                StaticSlot::ScalarList(ty) if plain_scalar(ty) => {
                    (self.list_type(ty), format!("{from}.toList()"))
                }
                StaticSlot::RecordList(alias) => (
                    self.record_list_type(&machine_record(alias)),
                    format!("{from}.map {{ {} }}", from_algorithm(alias, "it")?),
                ),
                _ => return None,
            })
        };
        let call = format!("{}({})", spec.call, arguments.join(", "));
        let (returned_type, _) = converted("r")?;
        Some(if spec.callee.may_fail {
            let (_, from_ok) = converted("r.value")?;
            format!(
                "private fun {}({}): com.sce.forge.runtime.AlgorithmResult<{returned_type}> =\n    when (val r = {call}) {{\n        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok({from_ok})\n        is com.sce.forge.runtime.AlgorithmResult.Failed -> r\n    }}",
                spec.name,
                declarations.join(", ")
            )
        } else {
            let (_, from_value) = converted("r")?;
            format!(
                "private fun {}({}): {returned_type} {{\n    val r = {call}\n    return {from_value}\n}}",
                spec.name,
                declarations.join(", ")
            )
        })
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Kotlin
    }
    // A prefix keeps a variable's field from meeting another member of the
    // class, which a bare name could: the generated machine extends the engine,
    // and a variable named `clock` under its own name hides the engine's.
    fn field_name(&self, id: &str) -> String {
        format!("v{}", filters::to_pascal_case(id.to_string()))
    }
    // The author's name in the case a Kotlin property is written in, and none
    // for a name the language reserves or the class already holds a property by.
    fn reader_name(&self, id: &str, _machine: &str) -> Option<String> {
        let spelled = filters::to_camel_case(id.to_string());
        (!crate::reader_names::is_reserved_word(Language::Kotlin, &spelled)
            && !crate::reader_names::kotlin_machine_holds_property(&spelled))
        .then_some(spelled)
    }
    fn field_ref(&self, name: &str) -> String {
        name.to_string()
    }
    fn in_function(&self) -> &'static str {
        "isStateActive"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::kotlin_type(ty).to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let params: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let present = match field.sce_type.present_type() {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::kotlin_type(other).to_string(),
                };
                // An optional field is the optional type over its present
                // value's (docs/adr/0055).
                let field_ty = match &field.sce_type {
                    FieldType::Optional { .. } => self
                        .optional_type(&present)
                        .expect("Kotlin spells an optional value"),
                    FieldType::Scalar(_) => present,
                };
                format!("val {}: {field_ty}", self.record_field(&field.id))
            })
            .collect();
        let bytes_members = self.record_bytes_members(ty, schema);
        // Its saved form (`com.sce.runtime.SavedValues`) lives on the type: an
        // object of the schema's fields keyed by their ids. Fixed member names
        // rather than per-type functions on the machine, whose name pattern
        // would reserve every variable name it could match
        // (`crate::reader_names`). An optional field is written as `null` or as
        // its present value, and read back absent from `null` and otherwise as
        // its present value is (docs/adr/0055, section 4).
        let writes: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let member = self.record_field(&field.id);
                // A present value: an enum is saved by its own type, as a
                // variable of it is.
                let write = |value: &str| match field.sce_type.present_type() {
                    SceType::Enum(_) => format!("{value}.toSaved()"),
                    _ => format!("SavedValues.of({value})"),
                };
                let saved = match &field.sce_type {
                    FieldType::Optional { .. } => {
                        format!("SavedValues.optional({member}) {{ {} }}", write("it"))
                    }
                    FieldType::Scalar(_) => write(&member),
                };
                format!("\"{}\" to {saved}", field.id)
            })
            .collect();
        let reads: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let name = self.record_field(&field.id);
                let id = &field.id;
                // A present value `saved`, named `what` in a refusal.
                let read =
                    |saved: &str, what: &str| match (field.sce_type.present_type(), field.max_size)
                    {
                        (SceType::Enum(reference), _) => format!(
                            "{}.fromSaved({saved}, {what})",
                            enum_types[&reference.alias]
                        ),
                        // A string is read back only if it fits the bound the
                        // schema declares, as a string variable's is.
                        (SceType::String, Some(bound)) => {
                            format!("SavedValues.string({saved}, {what}, {bound})")
                        }
                        // ... and so is a byte string, in bytes.
                        (SceType::Bytes, Some(bound)) => {
                            format!("SavedValues.bytes({saved}, {what}, {bound})")
                        }
                        (other, _) => format!("SavedValues.{}({saved}, {what})", other.as_attr()),
                    };
                let saved = format!("SavedValues.field(value, what, \"{id}\")");
                let what = format!("\"$what.{id}\"");
                match &field.sce_type {
                    FieldType::Optional { .. } => format!(
                        "{name} = SavedValues.optional({saved}, {what}) {{ e, w -> {} }}",
                        read("e", "w")
                    ),
                    FieldType::Scalar(_) => format!("{name} = {}", read(&saved, &what)),
                }
            })
            .collect();
        format!(
            "/** SCE Accepted Subset §2.15: a `record:{alias}` datamodel value. */\n\
             data class {ty}({params}) {{\n\
             \x20   /** This value as a saved state writes it. */\n\
             \x20   fun toSaved(): Any = linkedMapOf({writes})\n\
             {bytes_members}\n\
             \x20   companion object {{\n\
             \x20       /** The value a saved state holds, refused unless it is one. */\n\
             \x20       fun fromSaved(value: Any?, what: String): {ty} = {ty}({reads})\n\
             \x20   }}\n\
             }}",
            params = params.join(", "),
            writes = writes.join(", "),
            reads = reads.join(", "),
        )
    }
    fn record_field(&self, id: &str) -> String {
        crate::forge::generator::event_schema_field_ident(id, Language::Kotlin)
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!("{f} = {v}")).collect();
        format!("{ty}({})", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    fn enum_variant(&self, _ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::Kotlin, enum_name, variant)
    }
    // Its saved form lives on the type, as a record's does: the declared name
    // the enum document gives the variant, which is not the constant's own
    // spelling.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: Vec<String> = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "{}(\"{}\")",
                    self.enum_variant(ty, &model.name, &v.name),
                    filters::escape_kotlin(v.name.clone())
                )
            })
            .collect();
        // `toUnderlying` and `fromUnderlying` are the enum document's own
        // accessors, by the same names and shapes (`forge/kotlin/enum.kt.jinja2`):
        // what `<alias>.toNumber` and `<alias>.fromNumber` are lowered to
        // ([`StaticTarget::enum_number_symbols`]).
        let carrier = crate::forge::generator::kotlin_type(&model.underlying_type);
        let numbers: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "\x20       {} -> {}\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    crate::forge::generator::kotlin_literal(
                        &v.value.to_string(),
                        &model.underlying_type
                    )
                )
            })
            .collect();
        format!(
            "/** SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value. */\n\
             enum class {ty}(val declaredName: String) {{\n\
             \x20   {variants};\n\n\
             \x20   /** This value as a saved state writes it. */\n\
             \x20   fun toSaved(): Any = declaredName\n\n\
             \x20   /** The number the enum document declares for this value. */\n\
             \x20   fun toUnderlying(): {carrier} = when (this) {{\n\
             {numbers}\
             \x20   }}\n\n\
             \x20   companion object {{\n\
             \x20       /** The value a saved state holds, refused unless it is one. */\n\
             \x20       fun fromSaved(value: Any?, what: String): {ty} {{\n\
             \x20           val declared = SavedValues.string(value, what)\n\
             \x20           return entries.firstOrNull {{ it.declaredName == declared }}\n\
             \x20               ?: throw StateRefusal(\"'$what' ($declared) is not a variant of {alias}\")\n\
             \x20       }}\n\n\
             \x20       /** The variant that declares [raw], or null for a number no variant declares. */\n\
             \x20       fun fromUnderlying(raw: {carrier}): {ty}? = entries.firstOrNull {{ it.toUnderlying() == raw }}\n\
             \x20   }}\n\
             }}",
            variants = variants.join(",\n    ")
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (
            format!("({ty}::toUnderlying)"),
            format!("{ty}.fromUnderlying"),
        )
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("List<{}>", crate::forge::generator::kotlin_type(elem))
    }
    // An immutable `List` is handed out as it is.
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("List<{record}>")
    }
    fn record_list_view(&self, _record: &str) -> Option<String> {
        None
    }
    fn list_empty(&self) -> String {
        "emptyList()".to_string()
    }
    // Throws `AlgorithmFailure` past the bound, which is received where the
    // statement stands, as an overflow is.
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("com.sce.forge.runtime.SceChecked.bounded({value}, {capacity})")
    }
    // The overload that takes a `ByteArray` counts its size. A `ByteArray` the
    // machine never writes into, saved as its Latin-1 text; a record's field is
    // the same, in a data class that compares and hands out its bytes
    // ([`Self::record_bytes_members`]); a payload's is read where the machine's own
    // is, and held to the bound of the place it is written into.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        self.bounded_string(value, capacity)
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
    }
    // An algorithm's list is an array of its element; the machine's own is the
    // immutable `List` of it, which a failure past the bound is thrown from.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("com.sce.forge.runtime.SceChecked.within(({value}).toList(), {capacity})")
    }
    // The unsigned arrays an algorithm returns its list in are still
    // experimental in the standard library, and the machine holds one for the
    // length of the call.
    fn file_annotation_for_list_call(&self) -> Option<String> {
        Some("@file:OptIn(ExperimentalUnsignedTypes::class)".to_string())
    }
    // The machine counts the ids it generates, in the engine it extends.
    fn fresh_send_id(&self) -> String {
        "nextAutoSendId()".to_string()
    }
    // A record's field is a `val` of an immutable data class, so the
    // assignment builds the next value with that field replaced — the
    // lowering an algorithm's record local takes (E9).
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target} = {target}.copy({field} = {value})")
    }
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
        let label = if label.is_empty() {
            String::new()
        } else {
            format!("\"{}: \" + ", filters::escape_kotlin(label.to_string()))
        };
        format!("println({label}{value})")
    }
    // The list is immutable, so an append builds the next list — a snapshot
    // holding the old one keeps what it saw. The expression is `true` when it
    // failed: the list is full, or the value could not be computed.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        let body = format!(
            "if ({target}.size < {capacity}) {{ {target} = {target} + ({value}); false }} else {{ {} }}",
            Self::then_true(overflow)
        );
        if value_can_fail {
            format!(
                "try {{ {body} }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {} }}",
                Self::then_true(failed)
            )
        } else {
            body
        }
    }
    fn clear(&self, target: &str) -> String {
        format!("{target} = emptyList()")
    }
    // A `for` and not `forEach { }`, so a body that ends its block (an error)
    // leaves it with the `return` every other statement of the block uses. The
    // list is immutable and a body replaces the field rather than growing the
    // list, so the loop walks the list it began with. The position is a
    // `UInt`, as `len` is; the loop's own `Int` carries a name of the
    // generated code's.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        Some(match index {
            None => (format!("for ({item} in {list})"), String::new()),
            Some(index) => {
                let position = format!("sce_position_of_{index}");
                (
                    format!("for (({position}, {item}) in {list}.withIndex())"),
                    format!("@Suppress(\"UNUSED_VARIABLE\") val {index} = {position}.toUInt()"),
                )
            }
        })
    }
    fn raise_execution_error(&self, machine: &str, message: &str) -> String {
        format!(
            "raisePlatformError({machine}Event.Error.Execution, \"{}\")",
            filters::escape_kotlin(message.to_string())
        )
    }
    // The runtime's checked helpers throw `AlgorithmFailure` before the
    // statement writes anything; it is caught where the statement stands, as
    // an algorithm's boundary catches it. A `try` is an expression, so it
    // answers whether it failed, and stands outside any lambda the block
    // exit that follows it would otherwise have to leave.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "try {{ {statement}; false }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {} }}",
            Self::then_true(failed)
        )
    }
    fn receiving_call(&self, statement: &str, failed: &str) -> String {
        format!(
            "try {{ {statement} }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {failed} }}"
        )
    }
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        let after: Vec<&str> = [failed, flag]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect();
        let after = if after.is_empty() {
            String::new()
        } else {
            format!("{}; ", after.join("; "))
        };
        format!(
            "(try {{ {value} }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {after}false }})"
        )
    }
    // The flag is a local of the `<if>`, named by its ordinal: a nested `<if>`
    // cannot reuse its parent's, because Kotlin warns on a shadowed name.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String {
        format!("ifCondFailed{if_ordinal} = true")
    }
    // An enum field of the payload is held in the machine's own enum, lifted
    // from the variant's declared name and written back as it
    // (`build_kotlin_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`SendHelper.parseDelayMs`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes
    // (`resolveArrivingEvent`).
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes (`cancelSend`).
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template carries the value as the event's JSON and as the text a
    // host takes (`valueToJson`, `valueToWireString`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`hostInvokeSrc`).
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`DocumentStem.of`) and starts the candidate it names, handing it the
    // values it keeps (`seed_static_child`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    fn payload_accessor(&self, event: &str) -> String {
        format!("{}!!", kotlin_payload_field(event))
    }
    fn payload_guard(&self, _machine: &str, event: &str, lowered: &str) -> String {
        format!("{} != null && ({lowered})", kotlin_payload_field(event))
    }
    // The runtime's wire helpers read a `Boolean`, a `String`, or a number
    // through a `Double`, and Kotlin's unsigned types are not `Number`: so an
    // integer of at most 32 bits is widened to a `Long` (exact in a `Double`)
    // and a `Float` to a `Double`. A 64-bit integer is the text of its digits,
    // which `Long` and `ULong` both spell exactly.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool | InferredType::Str => value.to_string(),
            InferredType::Int { bits: 64, .. } => format!("({value}).toString()"),
            InferredType::Int { .. } => format!("({value}).toLong()"),
            InferredType::Float { bits: 32 } => format!("({value}).toDouble()"),
            _ => value.to_string(),
        }
    }
    // The enum class declares `declaredName` beside its entries.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("({value}).declaredName"))
    }
    // The text the runtime's payload encoder writes a byte array as, the string
    // every wire helper then reads.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(self.wire_value(
            InferredType::Str,
            &format!("com.sce.runtime.EventPayload.bytesAsText({value})"),
        ))
    }
}

/// Rust: a variable is a field of the machine's policy struct, a record a
/// plain `Copy` struct updated in place, a list a `Vec` bounded by its
/// declared capacity.
pub struct RustTarget;

impl RustTarget {
    /// `lines` as the text of one expression spread over several lines, each as
    /// it was given — indentation included, for the statements rustfmt leaves
    /// as they are (one carrying the long message of a raised error). A line
    /// with nothing on it is dropped: an error raised only when the document
    /// declares `error.execution` leaves one, and rustfmt refuses trailing
    /// whitespace it did not reflow.
    fn lines(lines: &[String]) -> String {
        lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The local a `<foreach>` binds an element's position to before the
    /// prologue reads it as the `uint32` index: `enumerate` counts in `usize`.
    fn loop_position(index: &str) -> String {
        format!("sce_position_of_{index}")
    }
}

impl StaticTarget for RustTarget {
    fn name(&self) -> &'static str {
        "Rust"
    }
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Rust, callee, None))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // `core`'s `Option<T>` (docs/adr/0055, section 5): the type is the default
    // spelling ([`crate::forge::generator::optional_of`]), and each value and
    // construct one of [`crate::forge::rust_optional`]'s, which the expression
    // emitter spells the same way.
    fn optional_none(&self, _present: &str) -> Option<String> {
        Some(crate::forge::rust_optional::ABSENT.to_string())
    }
    fn optional_some(&self, _present: &str, value: &str) -> Option<String> {
        Some(crate::forge::rust_optional::present(value))
    }
    // A `match`, so a failure `map` answers through `?` is the statement's.
    fn optional_map(
        &self,
        value: &str,
        _present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(crate::forge::rust_optional::mapped(value, map))
    }
    // The runtime's `ScriptValue::Null`, which its JSON writer spells `null`, where
    // the value is absent.
    fn wire_optional(
        &self,
        value: &str,
        _present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(crate::forge::rust_optional::wire(value, wire))
    }
    // Lent to the adapter, which converts what the algorithm's types differ in and
    // gives nothing back of it. The literal `null` handed to an optional record is
    // the absent value of the record's own type, lent as well (docs/adr/0055, step 9).
    fn state_argument(&self, machine: &str, arg: &StateArg, held: &str) -> String {
        match (&arg.slot, arg.code) {
            (StaticSlot::OptionalRecord(alias), 'n') => {
                format!("&None::<{}>", self.record_type(machine, alias))
            }
            _ => format!("&{held}"),
        }
    }
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("call_{coded}")
    }
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("sce_forge_runtime::algorithm::within(({value}).as_slice(), {capacity})?.to_vec()")
    }
    // The line a forge kind importing the schema writes, which is where the
    // algorithm's own generation put the record type the adapter names.
    fn schema_import(&self, schema_name: &str) -> Option<String> {
        let identity = crate::forge::generator::forge_import_identity(
            schema_name,
            &Language::Rust,
            false,
            &crate::ForgeCompileOptions::default(),
        );
        Some(identity.include_stmt)
    }
    // Both types are enums of the same declared variants, so a `match` over every
    // one of them is exhaustive, and a variant either side gains is a build error.
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let arms: Vec<String> = enum_variant_pairs(Language::Rust, machine_type, model, crossing)
            .into_iter()
            .map(|(from, to)| format!("{from} => {to}"))
            .collect();
        Some(format!("match {value} {{ {} }}", arms.join(", ")))
    }
    // The line that brings the enum document's module into scope, as for a schema.
    fn enum_import(&self, enum_name: &str) -> Option<String> {
        self.schema_import(enum_name)
    }
    // A unit struct of the module whose associated constants are the bounds, so a
    // host names one as `<Machine>Limits::<NAME>`. A machine whose host reads none is
    // not warned about them.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let constants: String = limits
            .iter()
            .map(|(name, value)| {
                format!(
                    "    /// {}\n    pub const {}: u32 = {value};\n",
                    limit_doc(name),
                    name.to_uppercase()
                )
            })
            .collect();
        Some(format!(
            "/// {LIMITS_DOC}\n#[allow(dead_code)]\npub struct {machine}Limits;\n\n#[allow(dead_code)]\nimpl {machine}Limits {{\n{constants}}}"
        ))
    }
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two functions of the module, each a `match` over every variant, named as the
    // adapters are.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Rust);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "/// A value of `{ty}` as the variant of the same declared name of `{document}`.\n\
             #[allow(non_snake_case, dead_code)]\n\
             fn {to}(v: {ty}) -> {document} {{\n    {into}\n}}\n\n\
             /// A value of `{document}` as the variant of the same declared name of `{ty}`.\n\
             #[allow(non_snake_case, dead_code)]\n\
             fn {from}(v: {document}) -> {ty} {{\n    {back}\n}}"
        ))
    }
    // A function of the machine's file that takes the values the machine holds —
    // a variable by reference, the event's payload by reference — and the call's
    // numbers, builds the algorithm's records from them field by field, calls it,
    // and builds the machine's own value from what it returns. The records and
    // lists are plain data and the conversion is a copy; nothing is shared with the
    // algorithm, so what it does cannot reach the machine's state except through
    // what it returns, and a variable is replaced only when the call succeeded.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{event_schema_field_ident, event_schema_payload_type};
        let value = spec.callee.value.as_ref()?;
        let plain = |alias: &str| plain_schema(spec, alias);
        let scalar_ok = plain_scalar;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        // The algorithm's record built from the machine's (or the payload's) value
        // `from`, field by field. A payload is the schema's own struct, so its fields
        // are named as the algorithm's are; a record the machine declares names them
        // as the machine does.
        let to_algorithm = |alias: &str, from: &str, payload: bool| -> Option<String> {
            let schema = plain(alias)?;
            let fields = schema
                .fields
                .iter()
                .map(|f| {
                    let field = event_schema_field_ident(&f.id, Language::Rust);
                    let held = if payload {
                        field.clone()
                    } else {
                        self.record_field(&f.id)
                    };
                    let read = format!("{from}.{held}");
                    spec.record_field_crossing(self, f, &read, Crossing::ToAlgorithm, |read| {
                        match (&f.sce_type, f.max_size) {
                            // The machine holds the text within the schema's bound,
                            // which is the bound the algorithm's record holds it in:
                            // the copy is the one a text already held to it takes.
                            (FieldType::Scalar(SceType::String), Some(bound)) => {
                                format!("::sce_portable_bytes::SceString::<{bound}>::held(&{read})")
                            }
                            // ... and a text that may be absent is copied where it is
                            // present, and stays absent (docs/adr/0055, step 9).
                            (
                                FieldType::Optional {
                                    elem: SceType::String,
                                },
                                Some(bound),
                            ) => format!(
                                "{read}.as_deref().map(|sce_present| \
                                 ::sce_portable_bytes::SceString::<{bound}>::held(sce_present))"
                            ),
                            _ => read.to_string(),
                        }
                    })
                    .map(|value| format!("{field}: {value}"))
                })
                .collect::<Option<Vec<String>>>()?;
            Some(format!(
                "{} {{ {} }}",
                event_schema_payload_type(&schema.name, Language::Rust),
                fields.join(", ")
            ))
        };
        // The machine's record built from what the algorithm returned, `from`.
        let from_algorithm = |alias: &str, from: &str| -> Option<String> {
            let schema = plain(alias)?;
            let fields = schema
                .fields
                .iter()
                .map(|f| {
                    let read =
                        format!("{from}.{}", event_schema_field_ident(&f.id, Language::Rust));
                    // The algorithm's text is held within the field's bound by
                    // construction, so the machine's `String` takes it whole.
                    spec.record_field_crossing(self, f, &read, Crossing::FromAlgorithm, |read| {
                        match &f.sce_type {
                            FieldType::Scalar(SceType::String) => {
                                format!("{read}.as_str().to_string()")
                            }
                            FieldType::Optional {
                                elem: SceType::String,
                            } => format!(
                                "{read}.as_ref().map(|sce_present| sce_present.as_str().to_string())"
                            ),
                            _ => read.to_string(),
                        }
                    })
                    .map(|value| format!("{}: {value}", self.record_field(&f.id)))
                })
                .collect::<Option<Vec<String>>>()?;
            Some(format!(
                "{} {{ {} }}",
                machine_record(alias),
                fields.join(", ")
            ))
        };
        let algorithm_record = |alias: &str| {
            plain(alias).map(|schema| event_schema_payload_type(&schema.name, Language::Rust))
        };
        // What a record that crosses a call is held as where the call is: the
        // machine's own record, or the payload struct of the event the transition
        // is on.
        let held_record = |alias: &str, code: char| -> Option<String> {
            if code == 'p' {
                let event = spec.payload_events.get(alias)?;
                Some(format!(
                    "{}{}Payload",
                    spec.machine,
                    filters::to_event_variant(event.to_string())
                ))
            } else {
                Some(machine_record(alias))
            }
        };
        let mut declarations = Vec::new();
        let mut prelude = Vec::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "{name}: {}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) => {
                    if !scalar_ok(ty) {
                        return None;
                    }
                    declarations.push(format!("{name}: {}", self.scalar_type(ty)));
                    arguments.push(name);
                }
                // `Option<T>` of a number or a `bool` is `Copy`, and the
                // algorithm's own type (docs/adr/0055). A text is handed as the
                // borrow the algorithm's parameter takes, which the expression of
                // the argument reads an optional text as already; a value of an
                // enum is the machine's `Option` over its own enum, and crosses where
                // it is present by the declared name of its variant; a byte string
                // is no value of an algorithm's slot, and is refused where the
                // import is read.
                AdapterParam::Optional(SceType::String) => {
                    declarations.push(format!("{name}: Option<&str>"));
                    arguments.push(name);
                }
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    declarations.push(format!("{name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(spec.optional_enum_crossing(
                        self,
                        ty,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Optional(SceType::Bytes) => return None,
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("{name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    declarations.push(format!("{name}: &{}", held_record(alias, *code)?));
                    arguments.push(to_algorithm(alias, &name, *code == 'p')?);
                }
                // An optional record the machine holds, or the literal `null`, is
                // lent as the optional it is and its record copied into the
                // algorithm's where it is present; any other record is lifted into
                // the present value of the algorithm's (docs/adr/0055, step 9).
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code: 'o' | 'n',
                } => {
                    declarations.push(format!(
                        "{name}: &{}",
                        self.optional_type(&machine_record(alias))?
                    ));
                    prelude.push(format!(
                        "    let {name}_algorithm: Option<{}> = {name}.as_ref().map(|r| {});\n",
                        algorithm_record(alias)?,
                        to_algorithm(alias, "r", false)?
                    ));
                    arguments.push(format!("{name}_algorithm"));
                }
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => {
                    declarations.push(format!("{name}: &{}", held_record(alias, *code)?));
                    arguments.push(format!(
                        "Some({})",
                        to_algorithm(alias, &name, *code == 'p')?
                    ));
                }
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } => {
                    if !scalar_ok(ty) {
                        return None;
                    }
                    declarations.push(format!("{name}: &[{}]", self.scalar_type(ty)));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    declarations.push(format!("{name}: &[{}]", machine_record(alias)));
                    prelude.push(format!(
                        "    let {name}_algorithm: Vec<{}> = {name}.iter().map(|r| {}).collect();\n",
                        algorithm_record(alias)?,
                        to_algorithm(alias, "r", false)?
                    ));
                    arguments.push(format!("&{name}_algorithm"));
                }
                // A number, a truth value and a value that may be absent are no
                // state a call lends: they are the scalar parameters above.
                AdapterParam::State {
                    slot: StaticSlot::Scalar(_) | StaticSlot::Optional(_),
                    ..
                } => return None,
            }
        }
        let (returned_type, converted) = match value.ret.as_ref()? {
            StaticSlot::Scalar(SceType::Enum(reference)) => (
                self.enum_type(spec.machine, &reference.alias)?,
                spec.enum_crossing(self, &reference.alias, "r", Crossing::FromAlgorithm)?,
            ),
            StaticSlot::Scalar(ty) if scalar_ok(ty) => (self.scalar_type(ty), "r".to_string()),
            StaticSlot::Scalar(_) => return None,
            // An optional value of an enum the algorithm answers is the machine's
            // `Option` over its own enum, mapped where it is present.
            StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => (
                spec.optional_type(self, ty)?,
                spec.optional_enum_crossing(self, ty, "r", Crossing::FromAlgorithm)?,
            ),
            StaticSlot::Optional(SceType::Bytes) => return None,
            StaticSlot::Optional(ty) => (spec.optional_type(self, ty)?, "r".to_string()),
            // The optional record the algorithm answers, its record copied into the
            // machine's where it is present (docs/adr/0055, step 9).
            StaticSlot::OptionalRecord(alias) => (
                self.optional_type(&machine_record(alias))?,
                format!(
                    "r.map(|sce_present| {})",
                    from_algorithm(alias, "sce_present")?
                ),
            ),
            StaticSlot::Record(alias) => (machine_record(alias), from_algorithm(alias, "r")?),
            StaticSlot::ScalarList(ty) if scalar_ok(ty) => (
                format!("Vec<{}>", self.scalar_type(ty)),
                "r.to_vec()".to_string(),
            ),
            StaticSlot::ScalarList(_) => return None,
            StaticSlot::RecordList(alias) => (
                format!("Vec<{}>", machine_record(alias)),
                format!(
                    "r.iter().map(|r| {}).collect::<Vec<_>>()",
                    from_algorithm(alias, "r")?
                ),
            ),
        };
        let call = format!("{}({})", spec.call, arguments.join(", "));
        let prelude: String = prelude.concat();
        Some(if spec.callee.may_fail {
            format!(
                "#[allow(non_snake_case)]\nfn {}({}) -> Result<{returned_type}, sce_forge_runtime::algorithm::AlgorithmError> {{\n{prelude}    let r = {call}?;\n    Ok({converted})\n}}",
                spec.name,
                declarations.join(", ")
            )
        } else {
            format!(
                "#[allow(non_snake_case)]\nfn {}({}) -> {returned_type} {{\n{prelude}    let r = {call};\n    {converted}\n}}",
                spec.name,
                declarations.join(", ")
            )
        })
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Rust
    }
    fn field_name(&self, id: &str) -> String {
        filters::to_snake_case(id.to_string())
    }
    fn field_ref(&self, name: &str) -> String {
        format!("self.{name}")
    }
    fn in_function(&self) -> &'static str {
        "self.is_state_active"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::rust_type(ty).to_string()
    }
    // An owned string or byte buffer is lent, not moved out of the machine.
    fn scalar_view(&self, ty: &SceType) -> Option<String> {
        match ty {
            SceType::String => Some("str".to_string()),
            SceType::Bytes => Some("[u8]".to_string()),
            _ => None,
        }
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    // Plain data by the record rule, so `Copy` — the derive set a plain
    // event-schema payload takes from the one policy that decides it — unless
    // a field is a string or a byte string, which owns its buffer and is `Clone`
    // only.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let derives = if owns_a_buffer(schema) {
            crate::rust_derive_policy::RustDeriveCategory::EventSchemaPayload
        } else {
            crate::rust_derive_policy::RustDeriveCategory::EventSchemaPlainPayload
        };
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                let present = match field.sce_type.present_type() {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::rust_type(other).to_string(),
                };
                // An optional field is held in the optional type over its present
                // value's (docs/adr/0055).
                let field_ty = match field.sce_type {
                    FieldType::Optional { .. } => self
                        .optional_type(&present)
                        .expect("Rust spells an optional value"),
                    FieldType::Scalar(_) => present,
                };
                format!("    pub {}: {field_ty},\n", self.record_field(&field.id))
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\n{}\n#[allow(non_snake_case)]\npub struct {ty} {{\n{fields}}}",
            derives.derives_attr()
        )
    }
    // A record that owns a buffer is `Clone` only, so a host is lent it and the
    // snapshot clones it, as it does a string.
    fn record_view(&self, ty: &str, schema: &EventSchemaModel) -> Option<String> {
        owns_a_buffer(schema).then(|| ty.to_string())
    }
    // ... and a copy of one is a clone.
    fn record_copy(&self, value: &str, schema: &EventSchemaModel) -> String {
        if owns_a_buffer(schema) {
            format!("{value}.clone()")
        } else {
            value.to_string()
        }
    }
    // An element of a list of records, taken whole (docs/adr/0055, step 9): a
    // record that owns text is read by reference and cloned out of the list, one
    // that is plain data is `Copy` and read by value, through the runtime's own
    // checked read of an index.
    fn record_element(&self, list: &str, index: &str, schema: &EventSchemaModel) -> Option<String> {
        Some(if owns_a_buffer(schema) {
            format!("sce_forge_runtime::algorithm::at_ref(&{list}, {index})?.clone()")
        } else {
            format!("sce_forge_runtime::algorithm::at(&{list}, {index})?")
        })
    }
    // A field of the present record of an optional one, written through the
    // runtime's `proven_mut` of a borrow of the optional: the record stays where it
    // is held, and the field is changed in place (docs/adr/0055, step 9).
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(format!(
            "{} = {value};",
            crate::forge::rust_optional::proven_field_place(target, field)
        ))
    }
    // The schema's id as written, as this file's payload structs spell their
    // fields — one file, one spelling of a schema field.
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!("{f}: {v}")).collect();
        format!("{ty} {{ {} }}", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    fn enum_variant(&self, _ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::Rust, enum_name, variant)
    }
    // The payload struct holds the field in the machine's own enum, lifted from
    // the variant's declared name and written back as it
    // (`forge::generator::build_rust_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`helpers::send::parse_delay_to_ms`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes
    // (`send_named_external`, `Self::resolve_event_by_name`).
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes
    // (`engine.cancel_event`).
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template carries the value as the event's JSON and as the text a
    // host takes (`script_value_to_json`, `script_value_to_wire_string`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`host_invoke_src`).
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`document_stem`) and starts the candidate it names, handing it the
    // values it keeps (`seed_static_child`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // The start site holds the id it generates in the `String` local
    // `generated_invoke_id` (`defer_scxml_invoke_fn`), which it goes on to
    // start the child under, so the variable takes a copy.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("generated_invoke_id.clone()")
    }
    // A closed set of unit variants, so `Copy` and `Eq` by the policy every
    // repr-tagged enum takes. Its saved form is written beside the machine's
    // own ([`StaticLowering::enums`]), where the saved-state runtime exists.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: String = model
            .variants
            .iter()
            .map(|v| format!("    {},\n", self.enum_variant(ty, &model.name, &v.name)))
            .collect();
        // `sce_name` answers the name the enum document declares for a value —
        // what a `<param>` carries it as. Not every machine reads it, so it is
        // allowed to go unused rather than warned about.
        let names: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "            Self::{} => \"{}\",\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    filters::escape_rust(v.name.clone())
                )
            })
            .collect();
        // `to_underlying` and `from_underlying` are the enum document's own
        // accessors, by the same names and shapes (`forge/rust/enum.rs.jinja2`):
        // what `<alias>.toNumber` and `<alias>.fromNumber` are lowered to
        // ([`StaticTarget::enum_number_symbols`]), read or not.
        let carrier = crate::forge::generator::rust_type(&model.underlying_type);
        let numbers: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "            Self::{} => {},\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    v.value
                )
            })
            .collect();
        let variants_of: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "            {} => Some(Self::{}),\n",
                    v.value,
                    self.enum_variant(ty, &model.name, &v.name)
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n{}\npub enum {ty} {{\n{variants}}}\n\n\
             impl {ty} {{\n    \
             /// The name the enum document declares for this value.\n    \
             #[allow(dead_code)]\n    \
             pub fn sce_name(self) -> &'static str {{\n        \
             match self {{\n{names}        }}\n    }}\n\n    \
             /// The number the enum document declares for this value.\n    \
             #[allow(dead_code)]\n    \
             pub fn to_underlying(self) -> {carrier} {{\n        \
             match self {{\n{numbers}        }}\n    }}\n\n    \
             /// The variant that declares `raw`, or `None` for a number no variant declares.\n    \
             #[allow(dead_code)]\n    \
             pub fn from_underlying(raw: {carrier}) -> Option<Self> {{\n        \
             match raw {{\n{variants_of}            _ => None,\n        }}\n    }}\n}}",
            crate::rust_derive_policy::RustDeriveCategory::ForgeEnum.derives_attr()
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (
            format!("{ty}::to_underlying"),
            format!("{ty}::from_underlying"),
        )
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("Vec<{}>", crate::forge::generator::rust_type(elem))
    }
    // A host reads the elements, never the machine's own `Vec`, so it cannot
    // grow the list past the bound the machine keeps.
    fn list_view(&self, elem: &SceType) -> Option<String> {
        Some(format!("[{}]", crate::forge::generator::rust_type(elem)))
    }
    // Plain data by the record rule, so the owned list of them is cloned
    // whole and lent as a slice, as a list of numbers is.
    fn record_list_type(&self, record: &str) -> String {
        format!("Vec<{record}>")
    }
    fn record_list_view(&self, record: &str) -> Option<String> {
        Some(format!("[{record}]"))
    }
    fn list_empty(&self) -> String {
        "Vec::new()".to_string()
    }
    // Answers through `?`, like every checked helper: the statement runs in a
    // closure that returns the failure before it writes.
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("sce_forge_runtime::algorithm::bounded({value}, {capacity})?")
    }
    // The one helper counts the bytes of whatever it is handed, a `String`'s
    // UTF-8 and a `Vec<u8>`'s alike. A `Vec<u8>` is bounded where it is written and
    // saved as its Latin-1 text; a record's field is the same, in a record that is
    // `Clone` and not `Copy` ([`owns_a_buffer`]); a payload's is the `&[u8]` the
    // payload channel borrows, copied into the `Vec<u8>` of the variable or the
    // record's field it is written to, under that place's bound.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        self.bounded_string(value, capacity)
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
    }
    // An algorithm's list is a fixed-capacity owned list; the machine's own is a
    // `Vec` of its elements, which a failure past the bound is returned from.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("sce_forge_runtime::algorithm::within(({value}).as_slice(), {capacity})?.to_vec()")
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value};")
    }
    // The count is the engine's the action is handed, and the id a `SceString`,
    // which the variable's `String` takes by `to_string` whatever the runtime's
    // string type is.
    fn fresh_send_id(&self) -> String {
        "engine.next_auto_send_id().to_string()".to_string()
    }
    // Debug formatting, as the script-engine arm of the same template logs a
    // value: a record has no `Display`, and one spelling serves every type. An
    // optional value logs its present value so, and `null` where it is absent
    // (docs/adr/0055), as every engine logs one.
    fn log(&self, label: &str, value: &str, ty: InferredType) -> String {
        let value = if ty.is_optional() {
            crate::forge::rust_optional::logged(value)
        } else {
            value.to_string()
        };
        if label.is_empty() {
            format!("::sce_rust_runtime::sce_log_info!(\"{{:?}}\", {value});")
        } else {
            format!(
                "::sce_rust_runtime::sce_log_info!(\"{{}}: {{:?}}\", \"{}\", {value});",
                filters::escape_rust(label.to_string())
            )
        }
    }
    // An expression that is `true` when the append failed: the list is full,
    // or the value could not be computed. The value is computed only where
    // there is room for it, and a checked helper answers through `?`, which
    // needs the closure a value that can fail runs in.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        if !value_can_fail {
            return Self::lines(&[
                format!("if {target}.len() < {capacity} {{"),
                format!("    {target}.push({value});"),
                "    false".to_string(),
                "} else {".to_string(),
                format!("    {overflow}"),
                "    true".to_string(),
                "}".to_string(),
            ]);
        }
        Self::lines(&[
            "match (|| -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {".to_string(),
            format!("    if {target}.len() < {capacity} {{"),
            format!("        {target}.push({value});"),
            "        Ok(false)".to_string(),
            "    } else {".to_string(),
            "        Ok(true)".to_string(),
            "    }".to_string(),
            "})() {".to_string(),
            "    Ok(false) => false,".to_string(),
            "    Ok(true) => {".to_string(),
            format!("        {overflow}"),
            "        true".to_string(),
            "    }".to_string(),
            "    Err(_) => {".to_string(),
            format!("        {failed}"),
            "        true".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ])
    }
    fn clear(&self, target: &str) -> String {
        format!("{target}.clear();")
    }
    // The loop owns a copy of the list: a body may append to the list it
    // walks, which a borrow of it would not allow. The elements are
    // fixed-width numbers and bools, so the copy is cheap and each element is
    // taken by value. A loop variable the body does not read is not a warning.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        Some(match index {
            None => (
                format!("for {item} in {list}.clone()"),
                format!("let _ = &{item};"),
            ),
            Some(index) => {
                let position = Self::loop_position(index);
                (
                    format!("for ({position}, {item}) in {list}.clone().into_iter().enumerate()"),
                    format!("let {index} = {position} as u32; let _ = (&{item}, &{index});"),
                )
            }
        })
    }
    // The first element of the copy the loop above walks, bound by the same
    // patterns, so the prologue it gives opens this one too: clippy reads a
    // loop whose body always leaves it as one that never loops.
    fn foreach_first(&self, list: &str, item: &str, index: Option<&str>) -> Option<String> {
        Some(match index {
            None => format!("if let Some({item}) = {list}.clone().into_iter().next()"),
            Some(index) => format!(
                "if let Some(({}, {item})) = {list}.clone().into_iter().enumerate().next()",
                Self::loop_position(index)
            ),
        })
    }
    fn raise_execution_error(&self, machine: &str, message: &str) -> String {
        format!(
            "engine.raise(sce_rust_runtime::EventWithMetadata::platform_error({machine}Event::{}, \"{}\"));",
            filters::to_event_variant("error.execution".to_string()),
            filters::escape_rust(message.to_string())
        )
    }
    // The checked helpers answer through `?`, which needs a `Result` to
    // return into: the statement runs in a closure that is one, so a failure
    // returns out of it before the statement writes anything.
    // A `match` is an expression, so it answers whether it failed, and the
    // block exit that follows it stands outside the closure it ran in.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        Self::lines(&[
            "match (|| -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {".to_string(),
            format!("    {statement}"),
            "    Ok(())".to_string(),
            "})() {".to_string(),
            "    Ok(()) => false,".to_string(),
            "    Err(_) => {".to_string(),
            format!("        {failed}"),
            "        true".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ])
    }
    fn receiving_call(&self, statement: &str, failed: &str) -> String {
        let run = format!(
            "(|| -> Result<(), sce_forge_runtime::algorithm::AlgorithmError> {{ {statement} Ok(()) }})()"
        );
        // With nothing to raise, the statement is skipped and that is all.
        if failed.is_empty() {
            format!("let _ = {run};")
        } else {
            format!("if {run}.is_err() {{ {failed} }}")
        }
    }
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        // A guard with nothing to raise and no flag to set (a transition's `cond`) is false when
        // it cannot be evaluated and that is all: `unwrap_or_default` says it, where a `match`
        // whose one failing arm is `false` is the shape `clippy::manual_unwrap_or_default` asks
        // to be written that way.
        if failed.is_empty() && flag.is_empty() {
            return format!(
                "(|| -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {{ Ok({value}) }})()\
                 .unwrap_or_default()"
            );
        }
        Self::lines(&[
            "match (|| -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {".to_string(),
            format!("    Ok({value})"),
            "})() {".to_string(),
            "    Ok(sce_value) => sce_value,".to_string(),
            "    Err(_) => {".to_string(),
            format!("        {failed}"),
            format!("        {flag}"),
            "        false".to_string(),
            "    }".to_string(),
            "}".to_string(),
        ])
    }
    // The flag is the `if_cond_failed` the `<if>`'s template declares; a
    // nested `<if>` declares its own in its own block, which shadows.
    fn condition_failed_flag(&self, _if_ordinal: u32) -> String {
        "if_cond_failed = true;".to_string()
    }
    fn payload_accessor(&self, _event: &str) -> String {
        "ev".to_string()
    }
    // The shape the typed-payload guard already takes on this backend
    // (`forge::generator::build_rust_event_payload`), binding the same `ev`.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "matches!(&self.pending_payload, {machine}Payload::{}(ev) if {lowered})",
            filters::to_event_variant(event.to_string())
        )
    }
    // The runtime's `ScriptValue` is what its wire helpers render both the
    // text and the JSON from, so a value built as one crosses exactly as a
    // script engine's would. A narrow integer widens to `i64` and a `float` to
    // `f64`, both exactly; a 64-bit integer is the text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!("::sce_rust_runtime::ScriptValue::Bool({value})"),
            InferredType::Str => format!("::sce_rust_runtime::ScriptValue::String({value})"),
            InferredType::Int { bits: 64, .. } => {
                format!("::sce_rust_runtime::ScriptValue::String(({value}).to_string())")
            }
            InferredType::Int { .. } => {
                format!("::sce_rust_runtime::ScriptValue::Int(i64::from({value}))")
            }
            InferredType::Float { bits: 32 } => {
                format!("::sce_rust_runtime::ScriptValue::Double(f64::from({value}))")
            }
            _ => format!("::sce_rust_runtime::ScriptValue::Double({value})"),
        }
    }
    // The enum's own `sce_name` ([`StaticTarget::enum_def`]), owned: the wire
    // value of a string holds a `String`.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("({value}).sce_name().to_string()"))
    }
    // The text the payload's own writer spells a byte slice as, which the wire
    // value of a string holds as the `String` it is.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(self.wire_value(
            InferredType::Str,
            &format!("::sce_rust_runtime::event_payload::bytes_as_payload_text(&{value})"),
        ))
    }
}

/// The scalar variables whose type `kind` holds, each with the `sce:capacity` its
/// declaration writes: what an `<assign>` to one is held to. A variable that
/// declares none was refused where it was read.
fn variable_bounds(
    variables: &[crate::model::Variable],
    kind: fn(&SceType) -> bool,
) -> impl Iterator<Item = (String, u32)> + '_ {
    variables
        .iter()
        .filter(move |v| {
            v.value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::scalar)
                .is_some_and(kind)
        })
        .filter_map(|v| Some((v.id.clone(), v.capacity?)))
}

/// The fields of the record variables whose type `kind` holds, each under the
/// place an `<assign>` names it (`last.label`) with the `sce:max-size` its schema
/// declares: what a write to one is held to. A field the schema leaves unbounded
/// has none, and was refused where the record was declared.
fn record_field_bounds<'a>(
    variables: &'a [crate::model::Variable],
    records: &'a std::collections::BTreeMap<String, EventSchemaModel>,
    kind: fn(&SceType) -> bool,
) -> impl Iterator<Item = (String, u32)> + 'a {
    variables.iter().flat_map(move |v| {
        // A field of an optional record's present value is written under
        // `<id>.value.<field>` (docs/adr/0055, step 9).
        let mut found = Vec::new();
        if let Some((alias, holder)) = crate::forge::type_ctx::record_holder(v) {
            if let Some(schema) = records.get(alias) {
                // An optional field is held to its bound as its present value is,
                // where it is present ([`OptionalPlace`]).
                for field in &schema.fields {
                    let (Some(scalar), Some(bound)) = (field.sce_type.scalar(), field.max_size)
                    else {
                        continue;
                    };
                    if kind(scalar) {
                        found.push((format!("{holder}.{}", field.id), bound));
                    }
                }
            }
        }
        found
    })
}

/// Every name a lowered `sce-static` expression spells differently on
/// `target`: each variable to the way the backend reaches its field, each
/// imported algorithm to the function its generation emits — the name a
/// forge kind calling the same algorithm uses. One list for every lowering, so
/// a guard, an assignment and a host action's argument cannot disagree about
/// a name.
fn names(scope: &StaticScope, target: &dyn StaticTarget) -> Vec<(String, String)> {
    scope
        .variables
        .iter()
        .map(|v| {
            (
                v.id.clone(),
                target.variable_ref(v, target.field_ref(&target.field_name(&v.id))),
            )
        })
        // A string field of a record variable, for a target that reads it through
        // the buffer that holds it: `last.label` is the buffer's text. The field of
        // the present value of an optional record variable is one too, read as
        // `pick.value.label` (docs/adr/0055, step 9): its owner is the record
        // variable and the member `value` that holds the record.
        .chain(scope.record_string_fields().filter_map(|(owner, field)| {
            let (var, member) = owner.split_once('.').unwrap_or((owner, ""));
            let mut record = target.field_ref(&target.field_name(var));
            if !member.is_empty() {
                record = format!("{record}.{member}");
            }
            target
                .record_string_read(&record, field)
                .map(|read| (format!("{owner}.{field}"), read))
        }))
        // A field of the element a list of records is indexed at: spelled as the
        // target spells a record's field.
        .chain(scope.indexed_record_fields().map(|(path, field)| {
            // A string field of a target that holds it in a buffer of its own is
            // read through that buffer, as a record variable's is.
            let spelled = if scope.indexed_field_is_string(path) {
                target
                    .record_string_read("", field)
                    .map(|read| read.trim_start_matches('.').to_string())
            } else {
                None
            };
            (
                path.to_string(),
                spelled.unwrap_or_else(|| target.record_field(field)),
            )
        }))
        // A callee the target cannot reach is left out: `lower` has already
        // refused a document that calls one.
        .chain(scope.callees.iter().filter_map(|c| {
            target
                .callee(c)
                .map(|callee| (c.alias.clone(), callee.call))
        }))
        .collect()
}

/// The refusal of `what` — a declaration that holds a value that may be absent
/// (docs/adr/0055) — by a backend, `lang`, that does not lower one yet: the one
/// wording every such refusal takes, whatever kind of document holds the value,
/// so an author reads by the same words which backend has its step to come.
pub(crate) fn optional_not_lowered(what: &str, lang: &str) -> GenerateError {
    GenerateError::unsupported(format!(
        "{what}: an optional value has no {lang} lowering yet"
    ))
}

/// The refusal of the first optional record of `scope` (docs/adr/0055, step 9)
/// on a target, `lang`, that does not lower one yet
/// ([`StaticTarget::lowers_optional_record`]): a variable of one, or a call of
/// an imported algorithm that takes or returns one. `None` for a document that
/// holds none. The rest of what a later step lowers is refused where it is
/// judged: a host action's optional argument, a typed request's optional field.
fn optional_record_not_lowered(scope: &StaticScope, lang: &str) -> Option<GenerateError> {
    use crate::forge::model::AlgorithmValueType;
    if let Some(variable) = scope.variables.iter().find(|v| {
        v.value_type
            .as_ref()
            .and_then(AlgorithmValueType::optional_elem)
            .is_some_and(|elem| elem.record_alias().is_some())
    }) {
        return Some(GenerateError::unsupported(format!(
            "<data id=\"{}\" sce:type=\"{}\">: an optional record has no {lang} lowering yet",
            variable.id,
            variable
                .value_type
                .as_ref()
                .map(AlgorithmValueType::as_attr)
                .unwrap_or_default()
        )));
    }
    let callee = scope.callees.iter().find(|c| {
        c.value.as_ref().is_some_and(|v| {
            v.params
                .iter()
                .chain(v.ret.iter())
                .any(|slot| matches!(slot, StaticSlot::OptionalRecord(_)))
        })
    })?;
    Some(GenerateError::unsupported(format!(
        "`{}(…)`: an imported algorithm that takes or returns an optional record has no {lang} \
         lowering yet",
        callee.alias
    )))
}

/// The refusal of the first call of an imported algorithm in `scope` whose slot
/// holds an optional value the target does not spell in an algorithm's slots yet
/// ([`StaticTarget::holds_algorithm_optional`], docs/adr/0055, steps 8 and 9),
/// worded as every refusal of an optional value is ([`optional_not_lowered`]).
/// `None` when the target spells every slot of every call.
fn algorithm_optional_not_lowered(
    scope: &StaticScope,
    target: &dyn StaticTarget,
) -> Option<GenerateError> {
    use crate::forge::model::OptionalElemType;
    scope.callees.iter().find_map(|callee| {
        let value = callee.value.as_ref()?;
        value
            .params
            .iter()
            .chain(value.ret.iter())
            .find_map(|slot| match slot {
                StaticSlot::Optional(elem)
                    if !target
                        .holds_algorithm_optional(&OptionalElemType::Scalar(elem.clone())) =>
                {
                    Some(optional_not_lowered(
                        &format!(
                            "`{}(…)`, a call of an imported algorithm whose slot is of type \
                             optional<{}>",
                            callee.alias,
                            elem.as_attr()
                        ),
                        target.name(),
                    ))
                }
                _ => None,
            })
    })
}

/// The type the present value of an optional value (docs/adr/0055) of the
/// element `elem` is held in by `target`, in `machine`: the machine's own type
/// for an enum ([`StaticTarget::enum_type`]), the buffer a target holds a text
/// of the bound `bound` in ([`StaticTarget::string_storage`],
/// [`StaticTarget::bytes_storage`]), and the scalar's own type otherwise
/// ([`StaticTarget::scalar_type`]). What every hook of an optional value is
/// handed as `present`, so each place spells the one type
/// ([`StaticTarget::optional_type`]) the same way.
pub fn optional_present(
    target: &dyn StaticTarget,
    machine: &str,
    elem: &SceType,
    bound: Option<u32>,
) -> String {
    let storage = match (elem, bound) {
        (SceType::String, Some(bound)) => target.string_storage(bound, "\"\""),
        (SceType::Bytes, Some(bound)) => target.bytes_storage(bound, "{ 0 }"),
        _ => None,
    };
    match (elem, storage) {
        (_, Some(storage)) => storage.ty,
        (SceType::Enum(reference), None) => target
            .enum_type(machine, &reference.alias)
            .unwrap_or_else(|| reference.alias.clone()),
        (_, None) => target.scalar_type(elem),
    }
}

/// An optional variable, or an optional field of a record variable, as an
/// `<assign>` to it is lowered (docs/adr/0055): what its present value is
/// ([`optional_present`]), and the bound a text it holds is held to.
#[derive(Debug, Clone)]
struct OptionalPlace {
    elem: SceType,
    present: String,
    /// The bound of a `string` or `bytes` element: the variable's
    /// `sce:capacity`, or the field's `sce:max-size`. `None` for any other.
    bound: Option<u32>,
}

/// A `sce-static` document's optional places ([`OptionalPlace`]), each under the
/// location an `<assign>` names it by: `due`, `item.due`.
type OptionalPlaces = std::collections::BTreeMap<String, OptionalPlace>;

/// The optional places of `variables` ([`OptionalPlaces`]): each optional
/// variable of a scalar element, and each optional field of a record variable.
fn optional_places(
    variables: &[crate::model::Variable],
    records: &std::collections::BTreeMap<String, EventSchemaModel>,
    target: &dyn StaticTarget,
    machine: &str,
) -> OptionalPlaces {
    let text_bound = |elem: &SceType, bound: Option<u32>| {
        matches!(elem, SceType::String | SceType::Bytes)
            .then_some(bound)
            .flatten()
    };
    let place = |elem: &SceType, bound: Option<u32>| OptionalPlace {
        elem: elem.clone(),
        present: optional_present(target, machine, elem, bound),
        bound: text_bound(elem, bound),
    };
    let mut places = OptionalPlaces::new();
    for var in variables {
        let Some(value_type) = var.value_type.as_ref() else {
            continue;
        };
        if let Some(elem) = value_type.optional_elem().and_then(|e| e.scalar()) {
            places.insert(var.id.clone(), place(elem, var.capacity));
        }
        // An optional field of a record variable is written as `<id>.<field>`, and
        // one of the present value of an optional record variable as
        // `<id>.value.<field>` (docs/adr/0055, step 9).
        let held = crate::forge::type_ctx::record_holder(var);
        let schema = held.as_ref().and_then(|(alias, _)| records.get(*alias));
        let holder = held.as_ref().map_or("", |(_, holder)| holder.as_str());
        for field in schema.iter().flat_map(|schema| schema.fields.iter()) {
            if let Some(elem) = field.sce_type.optional_elem() {
                places.insert(
                    format!("{holder}.{}", field.id),
                    place(elem, field.max_size),
                );
            }
        }
    }
    places
}

/// An optional record variable (docs/adr/0055, step 9), as an `<assign>` to it
/// is lowered: the alias of the schema it holds, the schema, and the type its
/// present value is held in on the target ([`StaticTarget::record_type`]) — what
/// every hook of an optional value is handed as `present`.
#[derive(Debug, Clone)]
struct OptionalRecordPlace {
    alias: String,
    schema: EventSchemaModel,
    present: String,
}

/// A `sce-static` document's optional record variables ([`OptionalRecordPlace`]),
/// each under its id.
type OptionalRecordPlaces = std::collections::BTreeMap<String, OptionalRecordPlace>;

/// The optional record variables of `variables` ([`OptionalRecordPlaces`]).
fn optional_record_places(
    variables: &[crate::model::Variable],
    records: &std::collections::BTreeMap<String, EventSchemaModel>,
    target: &dyn StaticTarget,
    machine: &str,
) -> OptionalRecordPlaces {
    variables
        .iter()
        .filter_map(|var| {
            let alias = var.value_type.as_ref()?.optional_elem()?.record_alias()?;
            let schema = records.get(alias)?.clone();
            Some((
                var.id.clone(),
                OptionalRecordPlace {
                    alias: alias.to_string(),
                    schema,
                    present: target.record_type(machine, alias),
                },
            ))
        })
        .collect()
}

/// The record variable and the field `location` names, when it is a field of a
/// record variable (`last.label`) or of the present value of an optional record
/// variable (`pick.value.count`, docs/adr/0055, step 9).
fn field_location<'l>(location: &'l str, rewrites: &Rewrites<'_>) -> Option<(&'l str, &'l str)> {
    if let Some(holder) = crate::forge::narrowing::present_record_of(location)
        .filter(|holder| rewrites.optional_records.contains_key(*holder))
    {
        let (_, field) = location.rsplit_once('.')?;
        return Some((holder, field));
    }
    location
        .split_once('.')
        .filter(|(var, _)| rewrites.records.contains_key(*var))
}

/// How the value a field is written holds: as it is, a text held to its bound,
/// or a byte string held to its bound ([`write_field`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldValue {
    Plain,
    Text,
    Bytes,
}

/// The statement that writes `value` into `field` (as [`StaticTarget::record_field`]
/// spells it) of the record variable, or of the present value of the optional
/// record variable, `var`, which the machine reaches as `name`: the target's own
/// spelling of each kind of field. `None` for an optional record on a target with
/// no spelling for one, which `lower` refuses by name before it is asked.
fn write_field(
    rewrites: &Rewrites<'_>,
    var: &str,
    name: &str,
    field: &str,
    value: &str,
    kind: FieldValue,
) -> Option<String> {
    let target = rewrites.target;
    if rewrites.optional_records.contains_key(var) {
        return match kind {
            FieldValue::Plain => target.assign_present_field(name, field, value),
            FieldValue::Text => target.assign_present_string_field(name, field, value),
            FieldValue::Bytes => target.assign_present_bytes_field(name, field, value),
        };
    }
    Some(match kind {
        FieldValue::Plain => target.assign_field(name, field, value),
        FieldValue::Text => target.assign_string_field(name, field, value),
        FieldValue::Bytes => target.assign_bytes_field(name, field, value),
    })
}

/// The absent value of the optional value whose present value is held in
/// `present`, on `target`, or the refusal of `what` by name for a target with no
/// spelling of it yet.
fn absent_value(
    target: &dyn StaticTarget,
    present: &str,
    what: &str,
) -> Result<String, GenerateError> {
    target
        .optional_none(present)
        .ok_or_else(|| optional_not_lowered(what, target.name()))
}

/// `value`, a present value held in `present`, lifted into the optional value
/// of its element on `target` ([`StaticTarget::optional_some`]), or the refusal
/// of `what` by name for a target with no spelling of it yet.
fn present_value(
    target: &dyn StaticTarget,
    present: &str,
    value: &str,
    what: &str,
) -> Result<String, GenerateError> {
    target
        .optional_some(present, value)
        .ok_or_else(|| optional_not_lowered(what, target.name()))
}

/// The whole value `written` gives the optional place `place` where the machine
/// is built — a variable's `<data expr>`, a record field's `<sce:set>`
/// (docs/adr/0055) — on `target`: the absent value for `null`, and otherwise the
/// value lowered by `initial` into the type its present value is and lifted.
/// `literal` makes the present value what the place holds it as (a target that
/// holds a text in a buffer is given the buffer its literal fills), which the
/// judge already held to the place's bound. An initial value is never one that
/// may be absent other than `null`: the judge admits a literal or a variant
/// there.
fn optional_initial(
    written: &str,
    place: &OptionalPlace,
    target: &dyn StaticTarget,
    what: &str,
    initial: &dyn Fn(&str, InferredType) -> Result<String, GenerateError>,
    literal: &dyn Fn(&str) -> String,
) -> Result<String, GenerateError> {
    if is_null_literal(written) {
        return absent_value(target, &place.present, what);
    }
    let value = initial(written, InferredType::from_sce_type(&place.elem))?;
    present_value(target, &place.present, &literal(&value), what)
}

/// Rewrite `model` — a clone the Kotlin backend renders — so every
/// expression of a `sce-static` document is native Kotlin.
pub fn lower_kotlin(
    model: &mut SCXMLModel,
    machine: &str,
) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &KotlinTarget)
}

/// Rewrite `model` — a clone the Rust backend renders — so every expression
/// of a `sce-static` document is native Rust.
pub fn lower_rust(model: &mut SCXMLModel, machine: &str) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &RustTarget)
}

/// Rewrite `model` — a clone one backend renders — so every expression of a
/// `sce-static` document is that backend's own code. A document under any
/// other data model is left as it is.
///
/// An enum a variable is declared as is declared in the machine's own unit, as
/// a record's type is, and a reference to one of its variants spells that
/// type's. A record whose schema has an enum field is refused: the schema is
/// the enum document's, which the machine's unit does not import.
pub fn lower(
    model: &mut SCXMLModel,
    machine: &str,
    target: &dyn StaticTarget,
) -> Result<StaticLowering, GenerateError> {
    let Some(scope) = StaticScope::of(model) else {
        return Ok(StaticLowering::default());
    };
    let lang = target.name();
    // An optional value (docs/adr/0055) is refused before anything reads the
    // variables' types, on every target that does not lower it yet: a variable
    // that holds one, and a field of a schema the machine reads a payload by or
    // holds a record of, which the payload lift and the record would hold.
    if !target.lowers_optional() {
        if let Some(variable) = scope.variables.iter().find(|v| {
            v.value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
                .is_some()
        }) {
            return Err(optional_not_lowered(
                &format!(
                    "<data id=\"{}\" sce:type=\"{}\">",
                    variable.id,
                    variable
                        .value_type
                        .as_ref()
                        .map(crate::forge::model::AlgorithmValueType::as_attr)
                        .unwrap_or_default()
                ),
                lang,
            ));
        }
        if let Some((schema, field)) = model
            .imported_event_schemas
            .values()
            .chain(model.imported_records.values())
            .find_map(|schema| schema.first_optional_field().map(|field| (schema, field)))
        {
            return Err(optional_not_lowered(
                &format!(
                    "the field `{}` of the event schema `{}`, sce:type=\"{}\"",
                    field.id,
                    schema.name,
                    field.sce_type.as_attr()
                ),
                lang,
            ));
        }
    }
    // What a target's own step of docs/adr/0055 has not written stays refused by
    // name, the targets that lower an optional value included: an optional
    // record, and an optional value of an element the target does not spell in an
    // algorithm's slots.
    if !target.lowers_optional_record() {
        if let Some(refusal) = optional_record_not_lowered(&scope, lang) {
            return Err(refusal);
        }
    }
    if let Some(refusal) = algorithm_optional_not_lowered(&scope, target) {
        return Err(refusal);
    }
    // A target that lowers an optional record spells the statement that writes a
    // field of its present value too: asked here, where one is declared, so that a
    // target that answered `true` and wrote no such statement is refused by name
    // rather than left to write nothing.
    let holds_optional_record = scope.variables.iter().any(|v| {
        v.value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
            .is_some_and(|elem| elem.record_alias().is_some())
    });
    if holds_optional_record
        && target.lowers_optional_record()
        && target.assign_present_field("", "", "").is_none()
    {
        return Err(optional_not_lowered(
            "a field written through an optional record",
            lang,
        ));
    }
    let variables = &scope.variables;
    let schemas = model.imported_event_schemas.clone();
    let records = model.imported_records.clone();
    // The enums the machine holds values of — the ones the document imports, and
    // the ones a field of its schemas holds that it does not (docs/adr/0054) —
    // each as the type the target declares it as. A target with none spells the
    // alias, which a reference to a variant then refuses by name
    // ([`crate::forge::expr`]). The document's text names only the enums it
    // imports, which the judge held it to before anything is lowered.
    let held_enums = model.held_enums();
    let enums = StaticEnum::from_imports(&held_enums, |alias, model| {
        let unlowered = crate::forge::type_ctx::StaticEnumSpelling::unlowered(alias);
        let Some(ty) = target.enum_type(machine, alias) else {
            return unlowered;
        };
        // The conversions between a value of the enum and the number its variant
        // declares, as the machine's own type for the enum answers them
        // ([`StaticTarget::enum_number_symbols`]).
        let (to_number, from_number) = target.enum_number_symbols(&ty, model);
        // The functions a value crosses into the enum document's type and back, where
        // a function of another kind the machine calls holds it (docs/adr/0051).
        let (to_document, from_document) = target.enum_crossing_names(&ty).unwrap_or_default();
        crate::forge::type_ctx::StaticEnumSpelling {
            qualified_type: ty,
            to_number,
            from_number,
            to_document,
            from_document,
        }
    });
    // A call the target cannot spell would be an undefined name where the
    // machine runs; it is refused here, where the document is read.
    for callee in &scope.callees {
        let kind = callee.kind.forge_kind();
        // A callee this backend writes in part and does not lower is refused for
        // the reason its own generation is.
        if let Some((_, why)) = callee
            .unlowered_by
            .iter()
            .find(|(lang, _)| target.expr_target().language() == Some(*lang))
        {
            return Err(GenerateError::unsupported(format!(
                "`{}(…)` calls an imported {kind} the {lang} backend does not generate: {why}",
                callee.alias
            )));
        }
        match target.callee(callee) {
            None => {
                return Err(GenerateError::unsupported(format!(
                    "`{}(…)`: a call of an imported {kind} has no {lang} lowering yet",
                    callee.alias
                )))
            }
            Some(Callee {
                import: CalleeImport::Unwritable,
                ..
            }) => {
                return Err(GenerateError::unsupported(format!(
                    "`{}(…)`: the import of the {kind} it calls cannot be written for \
                     {lang} without the module its packages live under",
                    callee.alias
                )))
            }
            Some(_) => {}
        }
    }
    if let Some(construct) = target.unsupported(model, &scope) {
        return Err(GenerateError::unsupported(format!(
            "{construct} has no {lang} lowering yet"
        )));
    }
    // A call that hands an algorithm a record or a list, or takes one back, needs
    // the target's adapter for it; a target without one refuses the document by
    // name instead of leaving the call undefined where the machine runs.
    if !target.lowers_state_calls() {
        if let Some(callee) = scope.callees.iter().find(|c| c.value.is_some()) {
            return Err(GenerateError::unsupported(format!(
                "`{}(…)`: an imported algorithm that takes or returns a record or a list has \
                 no {lang} lowering yet",
                callee.alias
            )));
        }
    }
    let names = names(&scope, target);
    // The runtime's optional type, for a target whose unit imports it by a line
    // of its own, in a machine that holds an optional value or hands one to an
    // algorithm and takes one back (an adapter's slot, a `null` argument).
    let calls_with_an_optional_slot = scope.callees.iter().any(|c| {
        c.value.as_ref().is_some_and(|v| {
            v.params.iter().chain(v.ret.iter()).any(|slot| {
                matches!(
                    slot,
                    StaticSlot::Optional(_) | StaticSlot::OptionalRecord(_)
                )
            })
        })
    });
    let optional_import = (holds_an_optional_value(model)
        || calls_with_an_optional_slot
        || starts_a_child_with_an_optional_value(model))
    .then(|| target.optional_import())
    .flatten();
    let imports: Vec<String> = scope
        .callees
        .iter()
        .filter_map(|c| match target.callee(c)?.import {
            CalleeImport::Line(line) => Some(line),
            CalleeImport::InScope | CalleeImport::Unwritable => None,
        })
        .chain(optional_import)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // The record variables, each with the schema its alias names — what a
    // field assignment is rewritten against.
    let record_vars: RecordVars = variables
        .iter()
        .filter_map(|v| {
            let alias = v.value_type.as_ref()?.record_alias()?;
            Some((
                v.id.clone(),
                (alias.to_string(), records.get(alias)?.clone()),
            ))
        })
        .collect();
    // The list variables, each with its element and bound — what an
    // `<sce:append>` and a `<foreach>` are rewritten against.
    let list_vars: ListVars = variables
        .iter()
        .filter_map(|v| {
            let elem = v.value_type.as_ref()?.list_elem()?.clone();
            Some((v.id.clone(), (elem, v.capacity?)))
        })
        .collect();
    // The string variables, each with its bound — what an `<assign>` to one is
    // held to — and the string fields of the record variables, each under the
    // place an `<assign>` names it (`last.label`) with the `sce:max-size` its
    // schema declares (docs/adr/0005, decision 1).
    let string_vars: StringVars = variable_bounds(variables, |ty| matches!(ty, SceType::String))
        .chain(record_field_bounds(variables, &records, |ty| {
            matches!(ty, SceType::String)
        }))
        .collect();
    // ... and the byte strings, in bytes, the same way (docs/adr/0005,
    // decision 2).
    let bytes_vars: BytesVars = variable_bounds(variables, |ty| matches!(ty, SceType::Bytes))
        .chain(record_field_bounds(variables, &records, |ty| {
            matches!(ty, SceType::Bytes)
        }))
        .collect();
    let rewrites = Rewrites {
        records: record_vars,
        schemas: records.clone(),
        enum_vars: crate::forge::static_datamodel::enum_variables(&scope, &records),
        payload_enum_paths: Default::default(),
        payload_fields: Default::default(),
        loop_records: Default::default(),
        lists: list_vars,
        strings: string_vars,
        bytes: bytes_vars,
        optionals: optional_places(variables, &records, target, machine),
        optional_records: optional_record_places(variables, &records, target, machine),
        machine,
        raises_error: model.events.contains("error.execution"),
        target,
        scope: &scope,
        payload_schema: Default::default(),
        state_calls: Default::default(),
        sites: Default::default(),
        elements: Default::default(),
        next_if_ordinal: std::cell::Cell::new(max_if_ordinal(model) + 1),
        host_optional: std::cell::Cell::new(false),
    };

    let refused = |what: &str, text: &str, refusal: Refusal| {
        GenerateError::unsupported(format!(
            "{what} `{text}` has no {lang} lowering: {}",
            refusal.error
        ))
    };

    // Every record variable's fields, in every scope below. An initial value
    // reads the variables declared before it by their field names, which is
    // how each backend holds them while the machine is being built.
    let no_payload = scope.paths(None);
    let mut fields = Vec::new();
    let mut declarations = TypeDeclarations::new(target, machine, &held_enums);
    // Taken before any expression is rewritten: the shape is the document's,
    // and the same for every backend.
    let saved_shape = saved_shape(model, &scope);
    {
        let ctx = scope.ctx(&no_payload, &enums);
        let init_names: Vec<(String, String)> = variables
            .iter()
            .map(|v| {
                (
                    v.id.clone(),
                    target.initial_field_ref(&target.field_name(&v.id)),
                )
            })
            .chain(names.iter().skip(variables.len()).cloned())
            .collect();
        let renames = renames(&init_names, None, target);
        for var in variables {
            let published = var.direction == Some(crate::forge::model::Direction::Out);
            let name = target.field_name(&var.id);
            // A record variable is built whole from its `<sce:set>`s, in
            // the schema's order — the rule the judge already held it to.
            if let Some(alias) = var.value_type.as_ref().and_then(|t| t.record_alias()) {
                let schema = records.get(alias).ok_or_else(|| {
                    GenerateError::unsupported(format!(
                        "<data id=\"{}\">: record:{alias} names no event-schema this build \
                         read",
                        var.id
                    ))
                })?;
                let ty = declarations.record(alias, schema)?;
                let mut values = Vec::with_capacity(schema.fields.len());
                for field in &schema.fields {
                    let init = var
                        .record_fields
                        .iter()
                        .find(|f| f.name == field.id)
                        .ok_or_else(|| {
                            GenerateError::unsupported(format!(
                                "<data id=\"{}\">: no <sce:set> gives `{}`",
                                var.id, field.id
                            ))
                        })?;
                    // A string field starts at its literal, which the judge held
                    // to the bound; a target that holds it in a buffer is given
                    // the buffer as that literal fills it.
                    let literal =
                        |value: &str| match (field.sce_type.present_type(), field.max_size) {
                            (SceType::String, Some(bound)) => {
                                target.record_string_literal(bound, value)
                            }
                            (SceType::Bytes, Some(bound)) => {
                                target.record_bytes_literal(bound, value)
                            }
                            _ => value.to_string(),
                        };
                    let initial = |written: &str, slot: InferredType| {
                        initial_value(written, target, &ctx, &renames, slot)
                            .map_err(|r| refused("the field value", written, r))
                    };
                    let place = rewrites.optionals.get(&format!("{}.{}", var.id, field.id));
                    let value = match place {
                        // An optional field starts absent, or at a present value
                        // lifted (docs/adr/0055).
                        Some(place) => optional_initial(
                            &init.expr,
                            place,
                            target,
                            &format!(
                                "<sce:set name=\"{}\"> of <data id=\"{}\">",
                                field.id, var.id
                            ),
                            &initial,
                            &literal,
                        )?,
                        None => literal(&initial(
                            &init.expr,
                            InferredType::of_field(&field.sce_type),
                        )?),
                    };
                    values.push((target.record_field(&field.id), value));
                }
                let init = target.record_value(&ty, &values);
                if let Some(element) = target.data_element(&var.id, &init) {
                    rewrites.note_element(var.value_type_spelling.as_ref(), &element);
                }
                let view = target.record_view(&ty, schema);
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    init,
                    saved_type: ty.clone(),
                    ty,
                    published,
                    view,
                    read: None,
                    bound: None,
                    saved_kind: "record",
                    saved_elem: None,
                    reader: None,
                    holds_bytes: holds_bytes(schema),
                    seeded: false,
                });
                continue;
            }
            // A list starts empty. Its elements are numbers and bools, or
            // records, which are declared as a record variable's are.
            if let Some(elem) = var.value_type.as_ref().and_then(|t| t.list_elem()) {
                use crate::forge::model::ListElemType;
                let (ty, view, saved_kind, saved_type, empty, holds) = match elem {
                    ListElemType::Scalar(elem) => {
                        let list = declarations.scalar_list(elem, var.capacity);
                        (
                            list.ty,
                            target.list_view(elem),
                            "list",
                            elem.as_attr(),
                            list.empty,
                            false,
                        )
                    }
                    ListElemType::Record { alias } => {
                        let schema = records.get(alias).ok_or_else(|| {
                            GenerateError::unsupported(format!(
                                "<data id=\"{}\">: list<record:{alias}> names no event-schema \
                                 this build read",
                                var.id
                            ))
                        })?;
                        let record_ty = declarations.record(alias, schema)?;
                        let list = declarations.record_list(&record_ty, var.capacity);
                        (
                            list.ty,
                            target.record_list_view(&record_ty),
                            "record_list",
                            record_ty,
                            list.empty,
                            holds_bytes(schema),
                        )
                    }
                };
                if let Some(element) = target.data_element(&var.id, &empty) {
                    rewrites.note_element(var.value_type_spelling.as_ref(), &element);
                }
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    ty,
                    init: empty,
                    published,
                    view,
                    read: None,
                    bound: var.capacity,
                    saved_kind,
                    saved_type,
                    saved_elem: None,
                    reader: None,
                    holds_bytes: holds,
                    seeded: false,
                });
                continue;
            }
            // An optional record is held in the target's one optional type over
            // the record's own type, declared as a record variable's is, and
            // starts absent: a record is put into it where the machine runs
            // (docs/adr/0055, step 9). A target that does not lower one yet was
            // refused above.
            if let Some(place) = rewrites.optional_records.get(&var.id) {
                let what = format!(
                    "<data id=\"{}\" sce:type=\"optional<record:{}>\">",
                    var.id, place.alias
                );
                let record_ty = declarations.record(&place.alias, &place.schema)?;
                declarations.optional_record_def(&record_ty);
                let ty = target
                    .optional_type(&record_ty)
                    .ok_or_else(|| optional_not_lowered(&what, lang))?;
                let init = absent_value(target, &record_ty, &what)?;
                rewrites.note(&var.expr, var.expr_spelling.as_ref(), &init);
                // A record a host is lent and not handed (one that owns text) is
                // lent in its optional too.
                let view = target
                    .record_view(&record_ty, &place.schema)
                    .map(|_| ty.clone());
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    ty,
                    init,
                    published,
                    view,
                    read: None,
                    bound: None,
                    // A saved state holds it as `null` or as the record it holds
                    // (docs/adr/0055, section 4): the kind names it apart from a
                    // record, which is never absent.
                    saved_kind: "optional",
                    saved_type: record_ty,
                    saved_elem: Some("record"),
                    reader: None,
                    holds_bytes: holds_bytes(&place.schema),
                    seeded: false,
                });
                continue;
            }
            // An optional value is held in the target's one optional type over
            // the type its present value is held in, which is declared as a
            // present value of it is, and starts absent or at a present value,
            // lifted (docs/adr/0055). An optional record was taken above, so
            // every place here is of a scalar.
            if let Some(place) = rewrites.optionals.get(&var.id) {
                let what = format!(
                    "<data id=\"{}\" sce:type=\"optional<{}>\">",
                    var.id,
                    place.elem.as_attr()
                );
                declarations.optional(place)?;
                let ty = target
                    .optional_type(&place.present)
                    .ok_or_else(|| optional_not_lowered(&what, lang))?;
                let initial = |written: &str, slot: InferredType| {
                    initial_value(written, target, &ctx, &renames, slot)
                        .map_err(|r| refused("the initial value", written, r))
                };
                // A text a target holds in a buffer starts at the buffer its
                // literal fills.
                let literal = |value: &str| {
                    place
                        .bound
                        .and_then(|bound| match place.elem {
                            SceType::String => target.string_storage(bound, value),
                            SceType::Bytes => target.bytes_storage(bound, value),
                            _ => None,
                        })
                        .map_or_else(|| value.to_string(), |storage| storage.init)
                };
                let init = optional_initial(&var.expr, place, target, &what, &initial, &literal)?;
                rewrites.note(&var.expr, var.expr_spelling.as_ref(), &init);
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    ty,
                    init,
                    published,
                    view: None,
                    read: None,
                    bound: place.bound,
                    // A saved state holds it as `null` or as its present value
                    // (docs/adr/0055, section 4); the kind names it apart from a
                    // scalar, which a child's `<param>` seeds and an optional
                    // value it does not.
                    saved_kind: "optional",
                    // A present value of an enum is saved by the machine's own
                    // type for it, as a variable of the enum is.
                    saved_type: match &place.elem {
                        SceType::Enum(_) => place.present.clone(),
                        other => other.as_attr(),
                    },
                    saved_elem: Some(match place.elem {
                        SceType::Enum(_) => "enum",
                        _ => "scalar",
                    }),
                    reader: None,
                    holds_bytes: false,
                    seeded: crate::forge::static_datamodel::seed_slot(var).is_some(),
                });
                continue;
            }
            let Some(ty) = var
                .value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::scalar)
            else {
                return Err(GenerateError::unsupported(format!(
                    "<data id=\"{}\"> has no scalar sce:type",
                    var.id
                )));
            };
            // An enum is held in a type of the machine's own, declared once
            // however many variables name it. Saved as the variant's declared
            // name, which the type reads back (see [`StaticEnumType`]).
            if let SceType::Enum(reference) = ty {
                let enum_ty = declarations.enum_type(&reference.alias).map_err(|_| {
                    GenerateError::unsupported(format!(
                        "<data id=\"{}\" sce:type=\"{}\">: an enum-typed variable has no {lang} \
                         lowering yet",
                        var.id,
                        ty.as_attr()
                    ))
                })?;
                let init = initial_value(&var.expr, target, &ctx, &renames, InferredType::Unknown)
                    .map_err(|r| refused("the initial value", &var.expr, r))?;
                rewrites.note(&var.expr, var.expr_spelling.as_ref(), &init);
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    init,
                    saved_type: enum_ty.clone(),
                    ty: enum_ty,
                    published,
                    view: None,
                    read: None,
                    bound: None,
                    saved_kind: "enum",
                    saved_elem: None,
                    reader: None,
                    holds_bytes: false,
                    seeded: false,
                });
                continue;
            }
            let slot = InferredType::from_sce_type(ty);
            let init = initial_value(&var.expr, target, &ctx, &renames, slot)
                .map_err(|r| refused("the initial value", &var.expr, r))?;
            rewrites.note(&var.expr, var.expr_spelling.as_ref(), &init);
            // A string a target holds in a buffer of its bound is declared once
            // however many variables share the bound, and read through the buffer.
            let storage = match (ty, var.capacity) {
                (SceType::String, Some(capacity)) => target.string_storage(capacity, &init),
                (SceType::Bytes, Some(capacity)) => target.bytes_storage(capacity, &init),
                _ => None,
            };
            // A byte string is read by a host as the view of its buffer and length,
            // which the machine builds from the two, and not as a text it holds.
            let read = storage
                .as_ref()
                .filter(|_| !matches!(ty, SceType::Bytes))
                .map(|_| target.variable_ref(var, target.field_ref(&name)));
            if let Some(storage) = &storage {
                declarations.string_storage(storage);
            }
            fields.push(StaticField {
                id: var.id.clone(),
                name,
                ty: storage
                    .as_ref()
                    .map_or_else(|| target.scalar_type(ty), |s| s.ty.clone()),
                init: storage.as_ref().map_or(init, |s| s.init.clone()),
                published,
                view: storage
                    .as_ref()
                    .map_or_else(|| target.scalar_view(ty), |s| Some(s.view.clone())),
                read,
                bound: matches!(ty, SceType::Bytes | SceType::String)
                    .then_some(var.capacity)
                    .flatten(),
                saved_kind: "scalar",
                saved_type: ty.as_attr(),
                saved_elem: None,
                reader: None,
                holds_bytes: false,
                seeded: crate::forge::static_datamodel::seed_slot(var).is_some(),
            });
        }
    }
    // The name a host reads each published field through, asked once the
    // fields are declared: a target whose member is not the reader says so.
    for field in &mut fields {
        field.reader = target.reader_name(&field.id, &model.name);
    }

    let mut payload_events = BTreeSet::new();
    for state in model.states.values_mut() {
        let plain_ctx = scope.ctx(&no_payload, &enums);
        let plain_renames = renames(&names, None, target);
        for block in state
            .on_entry_blocks
            .iter_mut()
            .chain(state.on_exit_blocks.iter_mut())
        {
            lower_actions(block, &plain_ctx, &plain_renames, &rewrites)?;
        }
        lower_actions(
            &mut state.initial_transition_actions,
            &plain_ctx,
            &plain_renames,
            &rewrites,
        )?;
        lower_actions(
            &mut state.initial_history_default_actions,
            &plain_ctx,
            &plain_renames,
            &rewrites,
        )?;
        // An invoke the host runs evaluates its request when it starts, at
        // the state's entry, where no event's payload is in scope. The names of
        // its `namelist` are params like any other: one list, folded in first.
        for invoke in &mut state.invokes {
            match invoke {
                crate::model::Invoke::Unsupported(info) => {
                    info.fold_namelist_into_params();
                    lower_host_request_strings(info, &plain_ctx, &plain_renames, &rewrites)?;
                    for param in &mut info.base.params {
                        lower_wire_param(param, &plain_ctx, &plain_renames, &rewrites)?;
                    }
                }
                crate::model::Invoke::Scxml(info) => {
                    lower_child_arguments(info, &plain_ctx, &plain_renames, &rewrites)?;
                    lower_invoke_idlocation(&mut info.common, &plain_renames, &rewrites)?;
                }
                crate::model::Invoke::Hybrid(info) => {
                    lower_hybrid_invoke(info, &plain_ctx, &plain_renames, &rewrites)?;
                    lower_invoke_idlocation(&mut info.common, &plain_renames, &rewrites)?;
                }
                crate::model::Invoke::MeshRpc(info) => {
                    lower_mesh_request(info, &plain_ctx, &plain_renames, &rewrites)?;
                }
            }
        }
        // What a `<final>` hands the event it raises is read from the
        // machine's fields when the state is entered (§scxml-5.5).
        if let Some(done) = &mut state.donedata {
            // A record named by `<content expr>` is the pairs of its fields, as
            // a `<send>`'s is; they have no attribute of their own to be
            // rewritten at, for the reason a `<send>`'s have none.
            let record = match &done.content {
                crate::model::DoneDataContent::Expression(expr) => Some(expr.trim().to_string()),
                _ => None,
            };
            //
            // An expression that names no record is the one value the event
            // carries: it is lowered below as the typed value its own type is
            // (`DoneData::native_content_value`), as a `<send>`'s is.
            let noted = rewrites.sites.borrow().len();
            let mut content_value = false;
            if let Some(record) = &record {
                match rewrites.content_fields(record) {
                    Some(fields) => done.fold_content_record_into_params(record, &fields),
                    None if target.lowers_scalar_content() => content_value = true,
                    None => {
                        return Err(GenerateError::unsupported(format!(
                            "a <donedata> whose <content expr=\"{record}\"> names a value has no \
                             {lang} lowering yet"
                        )))
                    }
                }
            }
            lower_done_params(&mut done.params, &plain_ctx, &plain_renames, &rewrites)?;
            if record.is_some() && !content_value {
                rewrites.sites.borrow_mut().truncate(noted);
            }
            // The value is read once, as a param's is, when the state is
            // entered, and the expression is cleared once it is lowered so no
            // template evaluates it a second time: the machine has no engine to
            // do it with.
            if let (true, Some(record)) = (content_value, &record) {
                let view = crate::forge::static_datamodel::WireParam::of_content(
                    record,
                    done.content_spelling.as_ref(),
                );
                let lowered = lower_wire_value(&view, &plain_ctx, &plain_renames, &rewrites)?;
                if let Some((value, fails)) = lowered {
                    done.native_content_value = value;
                    done.native_content_value_fails = fails;
                }
                done.content = crate::model::DoneDataContent::None;
            }
            // Inline `<content>` is the event's data as written, finished here
            // ([`crate::filters::static_content_wire`]) as a `<send>`'s is: the
            // machine has no engine to evaluate the text with (§scxml-5.5).
            if let crate::model::DoneDataContent::InlineText(text) = &done.content {
                done.native_content = crate::filters::static_content_wire(text);
                // Where the text is written, for a backend that runs the
                // document's own `<content>`: the same finished text, which an
                // engine then reads as the string it spells and not as the value
                // it could be read as.
                rewrites.note(
                    text,
                    done.content_text_spelling.as_ref(),
                    &done.native_content,
                );
            }
        }
        for transition in &mut state.transitions {
            let schema = schemas.get(&transition.event);
            rewrites.open_payload(schema);
            let paths = scope.paths(schema);
            let ctx = scope.ctx(&paths, &enums);
            let accessor = target.payload_accessor(&transition.event);
            let payload_reads = payload_enum_reads(target, schema, &accessor, &held_enums);
            let mut renames = renames(&names, schema.map(|_| accessor.as_str()), target);
            renames.extend(
                payload_reads
                    .iter()
                    .map(|(path, read)| (path.as_str(), read.as_str())),
            );
            // A pure `In()` predicate is lowered like any other condition, so
            // a guard and an `<if>` of one document spell it alike; the guard
            // macros read `native_guard` before their own `In()` arm.
            if !transition.cond.trim().is_empty()
                && !transition.is_cpp_condition
                && !transition.is_kt_condition
            {
                let cond = rewrites
                    .receiving(&transition.cond, &ctx, &renames, InferredType::Bool)
                    .map_err(|r| refused("the condition", &transition.cond, r))?;
                rewrites.note(
                    &transition.cond,
                    transition.cond_spelling.as_ref(),
                    &cond.text,
                );
                // §scxml-5.9.1: a condition that fails is false, and
                // `error.execution` says why (E12 D5).
                // A guard stands in no block, so there is nothing to end.
                transition.native_guard_fails = cond.can_fail;
                // A condition that reads the payload holds only while the
                // dequeued event carried one — the guard every typed
                // payload read on the backend takes. A target whose failing
                // condition is statements has no expression to put that check
                // around, so it takes it on the value, before the failure is
                // received: the operations of a delivery that did not carry a
                // payload are not run, and so fail nothing.
                let reads_payload = schema.is_some()
                    && crate::forge::expr::references_event_data_lexically(&transition.cond);
                let value = if reads_payload && target.payload_guards_the_value() {
                    target.payload_guard(machine, &transition.event, &cond.text)
                } else {
                    cond.text
                };
                let lowered = if cond.can_fail {
                    target.receiving_condition(
                        &value,
                        &execution_failure(
                            &rewrites,
                            &format!("<transition cond='{}'>", transition.cond),
                        ),
                        "",
                    )
                } else {
                    value
                };
                // It lands in the one slot every backend's guard macro reads
                // for a guard lowered at generate time; `cond_kt` stays the
                // author's `kt:` text.
                transition.native_guard = if reads_payload {
                    payload_events.insert(transition.event.clone());
                    if target.payload_guards_the_value() {
                        lowered
                    } else {
                        target.payload_guard(machine, &transition.event, &lowered)
                    }
                } else {
                    lowered
                };
                transition.cond_constant = None;
            }
            // Content that reads the payload cannot run for a delivery that
            // did not carry one; the template opens it with the check that
            // says so ([`crate::model::Transition::content_reads_payload`]).
            //
            // A test of a field of the payload in the `cond` proves it in the
            // content, as the judge read it (docs/adr/0055, section 3).
            let guarded = if transition.is_cpp_condition || transition.is_kt_condition {
                Facts::none()
            } else {
                crate::forge::narrowing::proves_written(&transition.cond, true).of_payload()
            };
            if lower_actions(
                &mut transition.actions,
                &ctx.proving(guarded),
                &renames,
                &rewrites,
            )? && schema.is_some()
            {
                payload_events.insert(transition.event.clone());
                transition.content_reads_payload = true;
            }
        }
        rewrites.open_payload(None);
    }
    for script in &mut model.global_scripts {
        let ctx = scope.ctx(&no_payload, &enums);
        lower_action(script, &ctx, &renames(&names, None, target), &rewrites)?;
    }
    // A payload that is read rides the typed channel, whose struct holds every
    // field its schema declares. An enum field is held there in the machine's
    // own type for the enum document the schema imports, under the name the
    // document holds that enum by, whether or not it imports it
    // (docs/adr/0054), as a record's field is; a target that has not written
    // that spelling yet is refused here, by name, because a generator that
    // reached the field would stop on it. Under any other data model the
    // guard keeps the script engine instead, and this one has none.
    // [`crate::forge::event_schema_check::schema_is_native_payload_eligible`] is
    // the one rule for what the channel holds without it.
    for event in &payload_events {
        let Some(schema) = schemas.get(event) else {
            continue;
        };
        if crate::forge::event_schema_check::schema_is_native_payload_eligible(schema) {
            continue;
        }
        let enum_fields = || {
            schema
                .fields
                .iter()
                .filter_map(|f| match f.sce_type.present_type() {
                    SceType::Enum(reference) => Some((f, reference.alias.as_str())),
                    _ => None,
                })
        };
        if !target.payload_enum_fields() {
            let carried = enum_fields().next().map_or_else(
                || "a field it cannot carry".to_string(),
                |(f, _)| format!("`{}` of type {}", f.id, f.sce_type.as_attr()),
            );
            return Err(GenerateError::unsupported(format!(
                "a transition that reads the payload of `{event}`, an event whose payload \
                 carries {carried} has no {lang} lowering yet"
            )));
        }
        for (field, alias) in enum_fields() {
            declarations.enum_type(alias).map_err(|enum_alias| {
                GenerateError::unsupported(format!(
                    "a transition that reads the payload of `{event}`, whose field `{}` holds \
                     the enum `{enum_alias}`: {lang} has no enum type for it, or the enum \
                     document the schema imports could not be read",
                    field.id
                ))
            })?;
        }
        // An optional field is held in the optional type over its present
        // value's, which a target that declares one declares here once
        // (docs/adr/0055).
        for field in &schema.fields {
            if let Some(elem) = field.sce_type.optional_elem() {
                let present = optional_present(target, machine, elem, field.max_size);
                declarations.optional_def(&present, elem);
            }
        }
    }
    // A target that passes an algorithm's list by value on the stack is refused a list of
    // numbers a stack cannot hold as well (the lists of records are judged below, with the
    // adapters they are copied by).
    if target.list_stack_limit_bytes().is_some() {
        for callee in &scope.callees {
            if let Some(returned) = &callee.list_return {
                if let Some(refusal) = crate::forge::generator::c11_list_too_large(
                    &format!("the list `{}(…)` returns", callee.alias),
                    crate::forge::generator::c11_scalar_bytes(&returned.elem, None),
                    returned.max_size,
                ) {
                    return Err(refusal);
                }
            }
        }
    }
    // A machine that takes a list an algorithm returns holds, in the code that
    // calls it, the type the algorithm returns it in, which a target may have to
    // be told it may use.
    let file_annotations = if scope.callees.iter().any(|c| {
        c.list_return.is_some()
            || c.value.as_ref().is_some_and(|v| {
                v.params
                    .iter()
                    .chain(v.ret.iter())
                    .any(|slot| matches!(slot, StaticSlot::ScalarList(_)))
            })
    }) {
        target.file_annotation_for_list_call().into_iter().collect()
    } else {
        Vec::new()
    };
    // One adapter for each way a call of an algorithm that takes or returns a record
    // or a list was made (docs/SCE_ACCEPTED_SUBSET.md §2.15), and the unit of each
    // schema it names a record of.
    // The bounds the machine names, each a constant of the number the build resolved
    // it to (docs/adr/0050). A name two elements resolve to two numbers has no one
    // value, and is declared by none: a read of it is refused where it is written.
    let resolved_limits: Vec<(String, u32)> = scope
        .limits()
        .iter()
        .filter_map(|(name, value)| Some((name.clone(), (*value).ok()?)))
        .collect();
    if !resolved_limits.is_empty() {
        if let Some(def) = target.limits_def(machine, &resolved_limits) {
            declarations.type_defs.push(def);
        }
    }
    let type_defs = declarations.type_defs;
    let mut adapter_defs = Vec::new();
    let mut imports: BTreeSet<String> = imports.into_iter().collect();
    // A host operation that takes an argument that may be absent names the runtime's
    // optional type in the host interface, though the machine holds no variable of
    // it (`flag ? 3 : null`): the unit imports it as it does for a variable.
    if rewrites.host_optional.get() {
        imports.extend(target.optional_import());
    }
    // An enum's definition names the optional type where the target answers a number
    // it declares as one, whether or not the machine holds an optional value.
    if target.enum_def_names_optional() && !declarations.enums.is_empty() {
        imports.extend(target.optional_import());
    }
    let payload_events_by_alias: std::collections::BTreeMap<String, String> = records
        .iter()
        .map(|(alias, schema)| (alias.clone(), schema.event_name.clone()))
        .collect();
    let mut record_list_bounds: std::collections::BTreeMap<String, u32> = Default::default();
    for (elem, capacity) in rewrites.lists.values() {
        if let crate::forge::model::ListElemType::Record { alias } = elem {
            let bound = record_list_bounds.entry(alias.clone()).or_default();
            *bound = (*bound).max(*capacity);
        }
    }
    let adapted: Vec<String> = if target.has_state_adapters() {
        rewrites.state_calls.borrow().iter().cloned().collect()
    } else {
        Vec::new()
    };
    for coded in &adapted {
        let call = scope
            .state_call(coded)
            .expect("a rewrite names a call the scope registered");
        let callee = scope
            .value_callee(&call.alias)
            .expect("a state call is of a callee with a value signature");
        let value = callee
            .value
            .as_ref()
            .expect("value_callee answers a callee with a value signature");
        let mut codes = call.codes.iter();
        let params = value
            .params
            .iter()
            .map(|slot| match slot {
                StaticSlot::Scalar(ty) => AdapterParam::Scalar(ty),
                StaticSlot::Optional(ty) => AdapterParam::Optional(ty),
                state => AdapterParam::State {
                    slot: state,
                    code: *codes.next().expect("one code per state slot"),
                },
            })
            .collect();
        let spec = AdapterSpec {
            name: target.state_adapter_name(machine, callee, coded),
            machine,
            lang: target.expr_target().language(),
            call: target
                .callee(callee)
                .expect("checked when the callees were read")
                .call,
            callee,
            params,
            schemas: &records,
            payload_events: &payload_events_by_alias,
            record_list_bounds: &record_list_bounds,
            enums: &held_enums,
        };
        // A target that passes an algorithm's list by value on the stack is refused a list
        // a stack cannot hold, here where the bound is built in and not at the call.
        if target.list_stack_limit_bytes().is_some() {
            let record_bytes = |alias: &str| -> u64 {
                records
                    .get(alias)
                    .map(|schema| {
                        schema
                            .fields
                            .iter()
                            .map(|f| {
                                crate::forge::generator::c11_field_bytes(&f.sce_type, f.max_size)
                            })
                            .sum()
                    })
                    .unwrap_or(0)
            };
            for slot in &value.params {
                if let StaticSlot::RecordList(alias) = slot {
                    let bound = record_list_bounds.get(alias).copied().unwrap_or(1);
                    if let Some(refusal) = crate::forge::generator::c11_list_too_large(
                        &format!(
                            "`{}(…)` is handed a list of the records `{alias}`, which the machine \
                             copies for it: the list",
                            callee.alias
                        ),
                        record_bytes(alias),
                        bound,
                    ) {
                        return Err(refusal);
                    }
                }
            }
            if let (Some(slot), Some(max)) = (value.ret.as_ref(), value.max_size) {
                let element_bytes = match slot {
                    StaticSlot::ScalarList(ty) => {
                        Some(crate::forge::generator::c11_scalar_bytes(ty, None))
                    }
                    StaticSlot::RecordList(alias) => Some(record_bytes(alias)),
                    _ => None,
                };
                if let Some(refusal) = element_bytes.and_then(|bytes| {
                    crate::forge::generator::c11_list_too_large(
                        &format!("the list `{}(…)` returns", callee.alias),
                        bytes,
                        max,
                    )
                }) {
                    return Err(refusal);
                }
            }
        }
        adapter_defs.push(target.state_adapter(&spec).ok_or_else(|| {
            GenerateError::unsupported(format!(
                "`{}(…)`: an imported algorithm that takes or returns a record or a list has \
                 no {lang} lowering yet",
                callee.alias
            ))
        })?);
        let returned = value
            .ret
            .iter()
            .filter(|_| target.adapter_names_returned_schema());
        for slot in value.params.iter().chain(returned) {
            if let StaticSlot::Record(alias)
            | StaticSlot::RecordList(alias)
            | StaticSlot::OptionalRecord(alias) = slot
            {
                if let Some(line) = records
                    .get(alias)
                    .and_then(|schema| target.schema_import(&schema.name))
                {
                    imports.insert(line);
                }
            }
        }
        // The unit of each enum a value crosses the adapter in — a slot of its own
        // or a field of a record the call hands over or takes back — which the
        // adapter names the algorithm's variants through.
        for alias in crossing_enums(value, &records) {
            if let Some(line) = held_enums
                .get(&alias)
                .and_then(|model| target.enum_import(&model.name))
            {
                imports.insert(line);
            }
        }
    }
    // What a target declares beside its calls of a callee it does not call as the
    // imported function itself: C's reading of a lookup whose miss is an error.
    adapter_defs.extend(scope.callees.iter().filter_map(|c| target.callee_def(c)));
    // The functions a value of an enum crosses through where a function of another
    // kind the machine calls holds it (docs/adr/0051), each enum's once, and the unit
    // of the enum document they name.
    let crossed: BTreeSet<&str> = scope
        .callees
        .iter()
        .filter(|c| !c.kind.is_algorithm())
        .flat_map(|c| c.params.iter().chain(c.ret.iter()))
        .filter_map(|ty| match ty {
            SceType::Enum(reference) => Some(reference.alias.as_str()),
            _ => None,
        })
        .collect();
    for alias in crossed {
        let (Some(model), Some(ty)) = (held_enums.get(alias), target.enum_type(machine, alias))
        else {
            continue;
        };
        if let Some(def) = target.enum_crossing_def(&ty, model) {
            adapter_defs.push(def);
            if let Some(line) = target.enum_import(&model.name) {
                imports.insert(line);
            }
        }
    }
    let imports: Vec<String> = imports.into_iter().collect();
    Ok(StaticLowering {
        fields,
        payload_events,
        type_defs,
        adapter_defs,
        file_annotations,
        imports,
        records: declarations.records,
        enums: declarations.enums,
        saved_shape,
        sites: rewrites.sites.into_inner(),
        elements: rewrites.elements.into_inner(),
    })
}

/// The enum and record types a `sce-static` machine declares in its own file,
/// each the first time any variable names it, with what a saved state writes of
/// each. One owner for both, because a record's field can hold an enum: the
/// record's declaration asks for the enum's.
struct TypeDeclarations<'t> {
    target: &'t dyn StaticTarget,
    machine: &'t str,
    /// Every enum the machine holds a value of, by the name it holds it by
    /// ([`SCXMLModel::held_enums`]).
    held_enums: &'t std::collections::BTreeMap<String, EnumModel>,
    declared: BTreeSet<String>,
    type_defs: Vec<String>,
    records: Vec<StaticRecord>,
    enums: Vec<StaticEnumType>,
}

impl<'t> TypeDeclarations<'t> {
    fn new(
        target: &'t dyn StaticTarget,
        machine: &'t str,
        held_enums: &'t std::collections::BTreeMap<String, EnumModel>,
    ) -> Self {
        Self {
            target,
            machine,
            held_enums,
            declared: BTreeSet::new(),
            type_defs: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
        }
    }

    /// The type a value of the enum the machine holds as `alias` is held in,
    /// declared the first time it is asked for. `Err` carries what the target
    /// cannot do: it has no enum type, or the machine holds no such enum.
    fn enum_type(&mut self, alias: &str) -> Result<String, String> {
        let (Some(ty), Some(model)) = (
            self.target.enum_type(self.machine, alias),
            self.held_enums.get(alias),
        ) else {
            return Err(alias.to_string());
        };
        if self.declared.insert(ty.clone()) {
            self.type_defs.push(self.target.enum_def(&ty, alias, model));
            self.enums.push(StaticEnumType {
                ty: ty.clone(),
                alias: alias.to_string(),
                variants: model
                    .variants
                    .iter()
                    .map(|v| StaticEnumVariant {
                        declared: v.name.clone(),
                        ident: self.target.enum_variant(&ty, &model.name, &v.name),
                    })
                    .collect(),
            });
        }
        Ok(ty)
    }

    /// The type a `record:<alias>` value is held in — of a record variable or
    /// of a list's elements.
    ///
    /// A field of an enum is held in the machine's own type for the enum
    /// document the schema imports, named as the machine holds that enum
    /// (docs/adr/0054), and is refused naming the field when the target has no
    /// type for it.
    fn record(&mut self, alias: &str, schema: &EventSchemaModel) -> Result<String, GenerateError> {
        let mut enum_types = std::collections::BTreeMap::new();
        for field in &schema.fields {
            if let SceType::Enum(reference) = field.sce_type.present_type() {
                let ty = self.enum_type(&reference.alias).map_err(|enum_alias| {
                    GenerateError::unsupported(format!(
                        "record:{alias} has the enum-typed field `{}`, whose enum `{enum_alias}` \
                         {} has no enum type for, or whose document the schema imports could \
                         not be read",
                        field.id,
                        self.target.name()
                    ))
                })?;
                enum_types.insert(reference.alias.clone(), ty);
            }
        }
        let ty = self.target.record_type(self.machine, alias);
        if self.declared.insert(ty.clone()) {
            // A string or byte-string field a target holds in a buffer is of the type
            // that buffer is declared as, which comes before the record that holds it.
            for field in &schema.fields {
                let storage = match (field.sce_type.present_type(), field.max_size) {
                    (SceType::String, Some(bound)) => self.target.string_storage(bound, "\"\""),
                    (SceType::Bytes, Some(bound)) => self.target.bytes_storage(bound, "{ 0 }"),
                    _ => None,
                };
                if let Some(storage) = storage {
                    self.string_storage(&storage);
                }
                // ... and so is an optional field's type, over its present value's
                // (docs/adr/0055).
                if let Some(elem) = field.sce_type.optional_elem() {
                    let present = optional_present(self.target, self.machine, elem, field.max_size);
                    self.optional_def(&present, elem);
                }
            }
            self.type_defs
                .push(self.target.record_def(&ty, alias, schema, &enum_types));
            self.records.push(StaticRecord {
                ty: ty.clone(),
                fields: schema
                    .fields
                    .iter()
                    .map(|f| StaticRecordField {
                        id: f.id.clone(),
                        name: self.target.record_field(&f.id),
                        // A saved value is named by the type its present value
                        // is (docs/adr/0055, its shape type).
                        saved_type: match f.sce_type.present_type() {
                            SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                            other => other.as_attr(),
                        },
                        bound: matches!(
                            f.sce_type.present_type(),
                            SceType::String | SceType::Bytes
                        )
                        .then_some(f.max_size)
                        .flatten(),
                        optional: f.sce_type.optional_elem().is_some(),
                        since: f.since,
                        default: f
                            .since
                            .and_then(|_| crate::forge::schema_revision::default_wire(f)),
                    })
                    .collect(),
            });
        }
        Ok(ty)
    }

    /// The types an optional variable of `place` names (docs/adr/0055), each
    /// declared the first time any variable names it: its present value's — the
    /// machine's own type for an enum, the buffer a target holds a bounded text
    /// in — and then the optional type over it, for a target that declares one
    /// ([`StaticTarget::optional_def`]).
    fn optional(&mut self, place: &OptionalPlace) -> Result<(), GenerateError> {
        let storage = match (&place.elem, place.bound) {
            (SceType::Enum(reference), _) => {
                self.enum_type(&reference.alias).map_err(|alias| {
                    GenerateError::unsupported(format!(
                        "optional<enum:{alias}>: {} has no enum type for it, or the enum \
                         document could not be read",
                        self.target.name()
                    ))
                })?;
                None
            }
            (SceType::String, Some(bound)) => self.target.string_storage(bound, "\"\""),
            (SceType::Bytes, Some(bound)) => self.target.bytes_storage(bound, "{ 0 }"),
            _ => None,
        };
        if let Some(storage) = storage {
            self.string_storage(&storage);
        }
        self.optional_def(&place.present, &place.elem);
        Ok(())
    }

    /// The declaration of the optional type over the record type `present`, for a
    /// target that declares one ([`StaticTarget::optional_record_def`]), made once
    /// however many variables hold one (docs/adr/0055, step 9).
    fn optional_record_def(&mut self, present: &str) {
        if let Some(def) = self.target.optional_record_def(present) {
            if self.declared.insert(format!("optional<{present}>")) {
                self.type_defs.push(def);
            }
        }
    }

    /// The declaration of the optional type over `present`, of the element
    /// `elem`, for a target that declares one ([`StaticTarget::optional_def`]),
    /// made once however many places hold one.
    fn optional_def(&mut self, present: &str, elem: &SceType) {
        if let Some(def) = self.target.optional_def(present, elem) {
            if self.declared.insert(format!("optional<{present}>")) {
                self.type_defs.push(def);
            }
        }
    }

    /// The type a string variable is held in ([`StaticTarget::string_storage`]),
    /// declared the first time any variable names it.
    fn string_storage(&mut self, storage: &StringStorage) {
        if self.declared.insert(storage.ty.clone()) {
            self.type_defs.push(storage.def.clone());
        }
    }

    /// How a list of the scalar `elem`, bounded by `capacity`, is held
    /// ([`StaticTarget::bounded_list`]), its type declared the first time any
    /// variable names it.
    fn scalar_list(&mut self, elem: &SceType, capacity: Option<u32>) -> BoundedList {
        let list = self.target.bounded_list(elem, capacity);
        if let Some(def) = &list.def {
            if self.declared.insert(list.ty.clone()) {
                self.type_defs.push(def.clone());
            }
        }
        list
    }

    /// [`Self::scalar_list`] for a list of the record type `record`.
    fn record_list(&mut self, record: &str, capacity: Option<u32>) -> BoundedList {
        let list = self.target.bounded_record_list(record, capacity);
        if let Some(def) = &list.def {
            if self.declared.insert(list.ty.clone()) {
                self.type_defs.push(def.clone());
            }
        }
        list
    }
}

/// The shape a saved state of this machine is bound to (SCE Accepted Subset
/// §2.15, "Saving and restoring"): a SHA-256 over every state with its kind
/// and parent, in document order, and every variable with its type and bound,
/// a record's fields and an enum's variants included — what a saved state
/// names, and nothing else.
///
/// Not the document's source hash: that one changes with a comment, and a
/// saved state is data a user keeps across an app update. A guard or an
/// action rewritten leaves a saved state restorable; a state or variable
/// renamed, re-typed, re-parented or re-bounded refuses it.
///
/// A `<history>` is named by the saved state, so each is part of the shape —
/// its id, whether it is deep, and the state it is declared in — and a document
/// without one hashes exactly what it did before histories were saved.
///
/// A record's fields are those of its schema's FIRST revision (docs/adr/0044): a
/// field a later revision added is named by the saved state's `revisions`
/// instead, through the digest of each revision ([`revision_digest`]), so a
/// state saved at an earlier revision of a schema still has the shape, and a
/// restore reads each field it lacks as its default. A schema of one revision
/// declares every field in its first, so a document whose schemas never grew
/// hashes exactly what it did before schemas had revisions. The shape a state
/// that names a revision past the first writes is [`SavedShape::revised`].
///
/// An `<invoke type="scxml">` is named by the saved state too — which child a
/// restore starts again — so each is part of the shape with its id and the
/// state that holds it, a hybrid one under a line of its own, and so is each
/// `<invoke>` a declared host invoker serves, with the type as well. A document
/// without any hashes exactly what it did before invocations were saved.
///
/// `None` for a machine whose state a saved state cannot hold whole
/// ([`a_tree_can_hold`]): one that waits on a session outside its tree — an
/// `<invoke>` a restore cannot start again, or a delayed `<send>` to a session
/// the tree does not hold — or one that can start a child whose own state a
/// saved state cannot hold. Such a machine is generated without the save API
/// rather than with one that would silently drop part of its state. A delayed
/// `<send>` to this session, to a host-served processor, to the parent or to a
/// child of this session is not such a send: each is a session or a processor
/// the tree holds (docs/adr/0045).
fn saved_shape(model: &SCXMLModel, scope: &StaticScope) -> Option<SavedShape> {
    use std::collections::BTreeMap;

    if !a_tree_can_hold(model) {
        return None;
    }
    let mut text = ShapeTexts::new("sce-saved-state-shape 1");
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    for state in states {
        let kind = if state.is_parallel {
            "parallel"
        } else if state.is_final {
            "final"
        } else if state.children.is_empty() {
            "atomic"
        } else {
            "compound"
        };
        text.line(&format!(
            "state {} {kind} {}",
            state.id,
            state.parent.as_deref().unwrap_or("-")
        ));
    }
    // By id: the map is ordered, so the same document hashes the same.
    for (id, history) in &model.history_states {
        let kind = if history.history_type == "deep" {
            "deep"
        } else {
            "shallow"
        };
        text.line(&format!("history {id} {kind} {}", history.parent));
    }
    // By the state that holds each, in document order, so the same document
    // hashes the same.
    let mut holders: Vec<_> = model
        .states
        .values()
        .filter(|s| !s.invokes.is_empty())
        .collect();
    holders.sort_by_key(|s| s.document_order);
    for state in holders {
        for invoke in &state.invokes {
            match invoke {
                crate::model::Invoke::Scxml(_) => {
                    text.line(&format!("invoke {} {}", invoke.base().invoke_id, state.id));
                }
                // Named apart from a child session of the same id: the two are
                // started differently (a hybrid one by the stem its `srcexpr`
                // computes), so a saved state of one is not the other's.
                crate::model::Invoke::Hybrid(_) => {
                    text.line(&format!(
                        "hybridinvoke {} {}",
                        invoke.base().invoke_id,
                        state.id
                    ));
                }
                // A host-run one is named with the type the host serves, which
                // is part of what a restore starts it with: the same id under
                // another type is another invocation. One no host serves is
                // refused when it starts, so a saved state names nothing of it.
                crate::model::Invoke::Unsupported(info) if info.host_served => {
                    text.line(&format!(
                        "hostinvoke {} {} {}",
                        info.invoke_type, info.base.invoke_id, state.id
                    ));
                }
                _ => {}
            }
        }
    }
    // Each record schema a variable is of, by the event it declares: ordered, so
    // the same document names them in the same order on every backend.
    let mut records: BTreeMap<String, SavedRecordLineage> = BTreeMap::new();
    // Where each value of an enum is held — a variable, or a field of a record a
    // variable holds — with the document the enum is: the statechart's own
    // import for a variable, the schema's for a record's field.
    let mut enum_holders: Vec<(
        &crate::forge::schema_revision::EnumDocument,
        SavedEnumHolder,
    )> = Vec::new();
    for var in &scope.variables {
        // A variable is named by its shape type, the type its present value is
        // (docs/adr/0055, section 6): an optional value by the type inside it,
        // so a variable widened from `T` to `optional<T>` keeps the shape, and
        // whether a saved `null` may stand in it is the restoring build's to
        // judge, value by value.
        let shaped = var.value_type.as_ref().map(shape_type);
        let ty = shaped.as_ref().map_or_else(
            String::new,
            crate::forge::model::AlgorithmValueType::as_attr,
        );
        let bound = var
            .capacity
            .map_or_else(|| "-".to_string(), |c| c.to_string());
        let scalar = shaped.as_ref().and_then(|t| t.scalar());
        let by_document = match scalar {
            Some(SceType::Enum(reference)) => {
                enum_type_by_document(model.enum_documents.by_alias.get(&reference.alias), &ty)
            }
            _ => ty.clone(),
        };
        text.spelled(
            &format!("variable {} {ty} {bound}", var.id),
            &format!("variable {} {by_document} {bound}", var.id),
        );
        // A record's fields, whether it is the variable or the element of the
        // list the variable is: what a saved state names of it.
        let own_record = shaped.as_ref().and_then(|t| t.record_alias());
        let listed_record = shaped
            .as_ref()
            .and_then(|t| t.list_elem().and_then(|e| e.record_alias()));
        let record_alias = own_record.or(listed_record);
        if let Some(schema) = record_alias.and_then(|alias| model.imported_records.get(alias)) {
            records
                .entry(schema.event_name.clone())
                .or_insert_with(|| record_lineage(model, schema))
                .holders
                .push(SavedRecordHolder {
                    variable: var.id.clone(),
                    list: own_record.is_none(),
                });
            // Every enum field holds a value of its enum, whichever revision of
            // the schema added the field.
            let field_documents = model.enum_documents.by_field.get(&schema.event_name);
            for field in &schema.fields {
                let document = field_documents.and_then(|documents| documents.get(&field.id));
                if let Some(document) = document {
                    enum_holders.push((
                        document,
                        SavedEnumHolder {
                            variable: var.id.clone(),
                            field: Some(field.id.clone()),
                            list: own_record.is_none(),
                        },
                    ));
                }
            }
            // The first revision's fields: a later one's are the saved state's
            // `revisions` to name.
            for field in schema.fields.iter().filter(|f| f.since.is_none()) {
                // A field is named by its shape type, the type its present value
                // is (docs/adr/0055).
                let by_alias = field.sce_type.present_type().as_attr();
                let by_document = match field.sce_type.present_type() {
                    SceType::Enum(_) => enum_type_by_document(
                        field_documents.and_then(|documents| documents.get(&field.id)),
                        &by_alias,
                    ),
                    _ => by_alias.clone(),
                };
                text.spelled(
                    &format!("field {}.{} {by_alias}", var.id, field.id),
                    &format!("field {}.{} {by_document}", var.id, field.id),
                );
                if let SceType::Enum(reference) = field.sce_type.present_type() {
                    text.variants(model, &reference.alias, &format!("{}.{}", var.id, field.id));
                }
            }
        }
        if let Some(SceType::Enum(reference)) = scalar {
            text.variants(model, &reference.alias, &var.id);
            if let Some(document) = model.enum_documents.by_alias.get(&reference.alias) {
                enum_holders.push((
                    document,
                    SavedEnumHolder {
                        variable: var.id.clone(),
                        field: None,
                        list: false,
                    },
                ));
            }
        }
    }
    // Each enum a value of the state is of, by the document it is: ordered by
    // its key, so the same document names them in the same order on every
    // backend, and one document read under two aliases is one enum.
    let mut enums: BTreeMap<String, SavedEnumLineage> = BTreeMap::new();
    for (document, holder) in enum_holders {
        enums
            .entry(enum_key(&document.key))
            .or_insert_with(|| enum_lineage(document))
            .holders
            .push(holder);
    }
    let [first, format2_first] = text.digests();
    // One line more, the same for every document: what makes a reader that
    // knows only the first shape refuse a state that names revisions, instead of
    // reading it without the fields it does not know.
    text.line("revisions");
    let [revised, format2_revised] = text.digests();
    Some(SavedShape {
        first,
        revised,
        format2_first,
        format2_revised,
        records: records.into_values().collect(),
        enums: enums.into_values().collect(),
    })
}

/// The shape type of `value_type` (docs/adr/0055, section 6): the type a
/// present value of it is. An optional value's is the type inside it, and every
/// other type's is the type itself, so a value widened from `T` to
/// `optional<T>` is named in a shape as it was before.
fn shape_type(
    value_type: &crate::forge::model::AlgorithmValueType,
) -> crate::forge::model::AlgorithmValueType {
    use crate::forge::model::{AlgorithmValueType, OptionalElemType};
    match value_type {
        AlgorithmValueType::Optional {
            elem: OptionalElemType::Scalar(present),
        } => AlgorithmValueType::Scalar(present.clone()),
        AlgorithmValueType::Optional {
            elem: OptionalElemType::Record { alias },
        } => AlgorithmValueType::Record {
            alias: alias.clone(),
        },
        other => other.clone(),
    }
}

/// The text of a document's shape, as each saved-state format names it
/// (docs/adr/0045, section 2): one text per format, line for line the same
/// except where an enum-typed variable or record field is named, which format 1
/// spells `enum:<alias>` (the import's alias, as it always has, so a state
/// already saved restores) and format 2 by the key of the enum's document
/// ([`enum_type_by_document`]), the key `revisions` names it by.
struct ShapeTexts {
    /// The format-1 text.
    format1: String,
    /// The format-2 text.
    format2: String,
}

impl ShapeTexts {
    /// Both texts, each beginning with the line `header`.
    fn new(header: &str) -> Self {
        let mut texts = Self {
            format1: String::new(),
            format2: String::new(),
        };
        texts.line(header);
        texts
    }

    /// `line` in both texts: a line that names no enum.
    fn line(&mut self, line: &str) {
        self.spelled(line, line);
    }

    /// `format1` in the format-1 text and `format2` in the format-2 one: the
    /// same line, with an enum named the way each format names it.
    fn spelled(&mut self, format1: &str, format2: &str) {
        for (text, line) in [(&mut self.format1, format1), (&mut self.format2, format2)] {
            text.push_str(line);
            text.push('\n');
        }
    }

    /// The variants of the enum imported as `alias`, under `holder`, in both
    /// texts ([`write_variants`]): a variant's name is the same in either.
    fn variants(&mut self, model: &SCXMLModel, alias: &str, holder: &str) {
        write_variants(&mut self.format1, model, alias, holder);
        write_variants(&mut self.format2, model, alias, holder);
    }

    /// The SHA-256 of each text as it stands, as lowercase hex: format 1's,
    /// then format 2's.
    fn digests(&self) -> [String; 2] {
        use sha2::{Digest, Sha256};
        [&self.format1, &self.format2].map(|text| format!("{:x}", Sha256::digest(text.as_bytes())))
    }
}

/// An enum-typed variable's or record field's type as the format-2 shape spells
/// it: `enum:` and the key of the enum's `document` ([`enum_key`]), the same
/// text a saved state's `revisions` names the enum by, so renaming the alias it
/// is imported under leaves it as it was, and moving the enum's file moves it.
///
/// `by_alias` (`enum:<alias>`) where the build read no document for the import,
/// as for a model no file was parsed from: the alias is then the only name the
/// enum has, and a key made up in its place is one no other build could
/// compute.
fn enum_type_by_document(
    document: Option<&crate::forge::schema_revision::EnumDocument>,
    by_alias: &str,
) -> String {
    document.map_or_else(|| by_alias.to_string(), |d| enum_key(&d.key))
}

/// What a saved state names of `schema`, a record schema a variable of `model`
/// is of: its revision, the digest of each revision past the first, and the
/// fields those revisions added with their defaults. Its holders are the
/// caller's to add.
fn record_lineage(model: &SCXMLModel, schema: &EventSchemaModel) -> SavedRecordLineage {
    let revision = schema.revision();
    SavedRecordLineage {
        event: schema.event_name.clone(),
        revision,
        revisions: (2..=revision)
            .map(|at| revision_digest(model, schema, at))
            .collect(),
        added: schema
            .fields
            .iter()
            .filter_map(|field| {
                Some(SavedAddedField {
                    id: field.id.clone(),
                    since: field.since?,
                    // The parser refuses a field with a `since` and no default
                    // that is a value of it, but for an optional one, whose
                    // default is then `null` (docs/adr/0055, section 4), so
                    // this is never `None` for one.
                    default: crate::forge::schema_revision::default_wire(field)?,
                })
            })
            .collect(),
        widened: schema
            .fields
            .iter()
            .filter_map(|field| {
                Some(SavedWidenedField {
                    id: field.id.clone(),
                    since: crate::forge::schema_revision::added_at(field),
                    optional_since: field.optional_since?,
                })
            })
            .collect(),
        holders: Vec::new(),
    }
}

/// The SHA-256, as lowercase hex, of what revision `at` of `schema` declares, as
/// a saved state names it (docs/adr/0044): each field the revision has (its
/// `sce:since` at most `at`, or none), in the schema's order, with its type and
/// the variant names of its enum. Not a field's default or `since`: a saved
/// state holds a value for each field and names neither, so two builds that
/// agree on what a revision holds agree on its digest.
fn revision_digest(model: &SCXMLModel, schema: &EventSchemaModel, at: u32) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut text = String::from("sce-saved-record 1\n");
    let _ = writeln!(text, "event {}", schema.event_name);
    for field in schema
        .fields
        .iter()
        .filter(|f| crate::forge::schema_revision::added_at(f) <= at)
    {
        let _ = writeln!(
            text,
            "field {} {}",
            field.id,
            field.sce_type.present_type().as_attr()
        );
        if let SceType::Enum(reference) = field.sce_type.present_type() {
            write_variants(&mut text, model, &reference.alias, &field.id);
        }
    }
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The key a saved state's `revisions` names an enum by: `enum:` and the
/// document's key ([`crate::forge::schema_revision::document_key`]) — the
/// document it is, as every reader of an enum identifies it, not an alias one
/// document imports it under, so an alias renamed leaves it as it was.
fn enum_key(document_key: &str) -> String {
    format!("enum:{document_key}")
}

/// What a saved state names of the enum `document`: its revision, the digest of
/// each revision past the first, and the variants those revisions added. Its
/// holders are the caller's to add.
fn enum_lineage(document: &crate::forge::schema_revision::EnumDocument) -> SavedEnumLineage {
    let enum_model = &document.model;
    let revision = enum_model.revision();
    SavedEnumLineage {
        key: enum_key(&document.key),
        revision,
        revisions: (2..=revision)
            .map(|at| enum_revision_digest(&document.key, enum_model, at))
            .collect(),
        added: enum_model
            .variants
            .iter()
            .filter_map(|variant| {
                Some(SavedAddedVariant {
                    name: variant.name.clone(),
                    since: variant.since?,
                })
            })
            .collect(),
        holders: Vec::new(),
    }
}

/// The SHA-256, as lowercase hex, of what revision `at` of the enum whose
/// document is `document_key` declares, as a saved state names it
/// (docs/adr/0044): the name of each variant the revision has (its `sce:since`
/// at most `at`, or none), sorted. Not a variant's value: a saved state holds an
/// enum by its declared name.
fn enum_revision_digest(document_key: &str, enum_model: &EnumModel, at: u32) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut text = String::from("sce-saved-enum 1\n");
    let _ = writeln!(text, "enum {document_key}");
    let mut names: Vec<&str> = enum_model
        .variants
        .iter()
        .filter(|v| v.added_at() <= at)
        .map(|v| v.name.as_str())
        .collect();
    names.sort_unstable();
    for name in names {
        let _ = writeln!(text, "variant {name}");
    }
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// The variants of the enum the machine holds as `alias` ([`SCXMLModel::held_enum`]),
/// one line each under `holder` — a variable, or a record's field. An enum's
/// variants are its type: one
/// renamed or removed leaves a saved value no variant of it, so the shape
/// refuses before a value is read. By name and sorted, so reordering them
/// leaves a saved state restorable — it holds the declared name, not a
/// position.
///
/// The variants of the enum's FIRST revision only (docs/adr/0044): a variant a
/// later revision added is named by the saved state's `revisions`, through the
/// digest of each revision of the enum ([`enum_revision_digest`]), so a state
/// saved at an earlier revision of the enum still has the shape. An enum of
/// one revision declares every variant in its first.
fn write_variants(text: &mut String, model: &SCXMLModel, alias: &str, holder: &str) {
    use std::fmt::Write as _;
    if let Some(enum_model) = model.held_enum(alias) {
        let mut names: Vec<&str> = enum_model
            .variants
            .iter()
            .filter(|v| v.since.is_none())
            .map(|v| v.name.as_str())
            .collect();
        names.sort_unstable();
        for name in names {
            let _ = writeln!(text, "variant {holder}.{name}");
        }
    }
}

/// Whether a saved state can hold the whole state of a machine of `model`, and
/// of every session it can start (docs/adr/0045, "Which documents save"): the
/// document is `datamodel="sce-static"`, it waits on no session outside its
/// tree ([`invokes_what_a_restore_cannot_start`],
/// [`delays_a_send_to_another_session`]), and every child it can start through an
/// `<invoke type="scxml">` — each candidate of a hybrid one — is a document of
/// which the same holds.
///
/// A child's answer is read from what the parse recorded on its `<invoke>`
/// ([`crate::model::InvokeSessionCommon::child_saves`]), which this function
/// computed when the child's own document was parsed, so a grandchild is
/// judged by the same rule without being read twice. A child the build could
/// not read answers no. A hybrid `<invoke>` with no candidate the build knows
/// starts a document nobody judged, and answers no as well.
pub(crate) fn a_tree_can_hold(model: &SCXMLModel) -> bool {
    model.datamodel == crate::model::Datamodel::SceStatic
        && !invokes_what_a_restore_cannot_start(model)
        && !delays_a_send_to_another_session(model)
        && model
            .states
            .values()
            .flat_map(|s| &s.invokes)
            .all(|invoke| match invoke {
                crate::model::Invoke::Scxml(info) => info.common.child_saves,
                crate::model::Invoke::Hybrid(info) => {
                    !info.candidates.is_empty() && info.candidates.iter().all(|c| c.child_saves)
                }
                crate::model::Invoke::Unsupported(_) | crate::model::Invoke::MeshRpc(_) => true,
            })
}

/// Whether the document holds an optional value (docs/adr/0055): a variable of
/// one, or a field of a schema it holds a record of or reads a payload by — the
/// machine whose unit imports the target's optional type, on a target that
/// imports it by a line of its own.
fn holds_an_optional_value(model: &SCXMLModel) -> bool {
    let optional_variable = model
        .variables
        .iter()
        .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
        .any(|v| {
            v.value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
                .is_some()
        });
    optional_variable
        || model
            .imported_event_schemas
            .values()
            .chain(model.imported_records.values())
            .any(|schema| schema.first_optional_field().is_some())
}

/// Whether an `<invoke type="scxml">` of the document starts a child, or a
/// candidate of a hybrid one, that declares an optional variable: what the
/// `<param>` handing it a value writes is the target's optional type
/// (docs/adr/0055, step 10), so the machine whose unit imports the optional
/// type by a line of its own needs the line.
fn starts_a_child_with_an_optional_value(model: &SCXMLModel) -> bool {
    let optional = |variables: &Option<Vec<crate::model::Variable>>| {
        variables.iter().flatten().any(|v| {
            v.value_type
                .as_ref()
                .and_then(crate::forge::model::AlgorithmValueType::optional_elem)
                .is_some()
        })
    };
    model
        .states
        .values()
        .flat_map(|state| state.invokes.iter())
        .any(|invoke| match invoke {
            crate::model::Invoke::Scxml(info) => optional(&info.common.child_static_variables),
            crate::model::Invoke::Hybrid(info) => info
                .candidates
                .iter()
                .any(|candidate| optional(&candidate.child_static_variables)),
            crate::model::Invoke::MeshRpc(_) | crate::model::Invoke::Unsupported(_) => false,
        })
}

/// Whether the document makes a delayed `<send>` that, when it comes due, is
/// delivered to a session outside its tree (§scxml-6.2.4): its target is a `#_`
/// location other than this session's own queue (`#_internal`), its parent
/// (`#_parent`) and a child of this session, named by the literal id of one of
/// its `<invoke type="scxml">`s (`#_<id>`).
///
/// A saved state holds a waiting send as an event for this session's own
/// queues, a request a host-served processor performs, or an event for the
/// parent or a child (docs/adr/0045): those sessions are in the saved tree. A
/// send waiting on any other session — one named by its session id, a child of
/// some other session — waits on a session the tree does not hold.
///
/// A `delayexpr` counts as a delay: the time it computes is known only when the
/// send runs and may be one that waits, so a machine that has one cannot be
/// told apart from a machine that delays, and the saved state would drop the
/// send it holds.
///
/// A `targetexpr` is a value this document computes, and which sessions it could
/// name is what it declares as `sce:targets` (docs/adr/0005, decision 3): a
/// delayed send whose declared routes include one outside the tree is a send
/// waiting on another session, as a written target would be.
/// `a_delayed_sends_target_is_a_literal_under_sce_static` holds the written
/// form, and `a_delayed_send_to_a_computed_target_waits_on_another_session` the
/// declared one.
fn delays_a_send_to_another_session(model: &SCXMLModel) -> bool {
    let children: Vec<String> = model
        .states
        .values()
        .flat_map(|s| &s.invokes)
        .filter_map(|invoke| match invoke {
            crate::model::Invoke::Scxml(info) if !info.common.base.id_generated => {
                Some(format!("#_{}", info.common.base.invoke_id))
            }
            crate::model::Invoke::Hybrid(info) if !info.common.base.id_generated => {
                Some(format!("#_{}", info.common.base.invoke_id))
            }
            _ => None,
        })
        .collect();
    let outside = |route: &str| {
        route.starts_with("#_")
            && route != "#_internal"
            && route != "#_parent"
            && !children.iter().any(|child| child == route)
    };
    model.sends().into_iter().any(|(_, send)| {
        (!send.delay.is_empty() || !send.delayexpr.is_empty())
            && (outside(&send.target) || send.targets.iter().any(|t| outside(t)))
    })
}

/// Whether the document holds an `<invoke>` a restore cannot start again: a
/// mesh call or a peer on another device.
///
/// A saved state names an invocation and a restore starts it again
/// (§scxml-6.4). Three kinds reduce to that:
///
/// - a child session (`type="scxml"`) starts from the beginning, and what it
///   was started with is its arguments, read again from the restored fields as
///   its start reads them — it has no `<finalize>`, which `static_datamodel`
///   refuses;
/// - a hybrid invoke (`srcexpr` among declared `sce:candidates`) starts the
///   same way, and the `srcexpr` is one more of what the start reads: the
///   restored fields name the candidate, as they would had the state been
///   entered, so a candidate chosen before the save is not what a restore
///   starts unless the fields still choose it;
/// - a host-run invocation (`<invoke type>` a declared invoker serves) starts
///   again from the request it was started with, which the saved state holds,
///   with the deadline it had left;
/// - a mesh-rpc call is saved the same way, and is the one that is NOT started
///   again: it reaches this point already lowered to a host-served invoke of the
///   type SCE reserves for it (`lower_mesh_invokes`), and its one request in
///   flight may already have reached its peer, which would act on it twice if it
///   were sent again. A restore starts nothing and tells the document the call
///   was interrupted (`error.invoke.<id>`, data `"interrupted"`), and a document
///   that means to ask again sends a new request.
///
/// One no host serves is refused when it starts (`error.execution`), so nothing
/// is running and a saved state has nothing to name.
///
/// The rest do not reduce: a mesh peer is a session this machine does not own.
/// A saved state that left one out would restore a machine waiting on something
/// nobody is doing.
fn invokes_what_a_restore_cannot_start(model: &SCXMLModel) -> bool {
    model
        .states
        .values()
        .flat_map(|s| &s.invokes)
        .any(|invoke| match invoke {
            crate::model::Invoke::Scxml(info) => info.remote_mesh_target.is_some(),
            // Lowered to a host-served invoke before this point; one that was not has no
            // host invoke a saved state could hold.
            crate::model::Invoke::MeshRpc(_) => true,
            crate::model::Invoke::Unsupported(_) | crate::model::Invoke::Hybrid(_) => false,
        })
}

/// A variable's initial value. It is computed while the machine is being
/// built, before there is a running session to raise `error.execution` in,
/// so one that could fail (E12 D5) has nowhere to put the failure and is
/// refused where it is written instead.
fn initial_value(
    expr: &str,
    target: &dyn StaticTarget,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    slot: InferredType,
) -> Result<String, Refusal> {
    // A variable owns its value, so a `string` is made owned on the way in.
    let value = transpile_into_owned(expr, target.expr_target(), ctx, renames, slot)?;
    if value.can_fail {
        return Err(crate::forge::error::ExprError::UnsupportedConstruct {
            construct: "an initial value that can overflow or fail (a machine that is not yet \
                        running has no error.execution to raise; give a value that fits)"
                .to_string(),
            observed: Some(expr.trim().to_string()),
        }
        .at(None));
    }
    Ok(value.text)
}

/// A `sce-static` document's record variables, each with the alias it is
/// declared under and the schema that alias names.
type RecordVars = std::collections::BTreeMap<String, (String, EventSchemaModel)>;

/// A `sce-static` document's list variables, each with its element type and
/// its declared capacity.
type ListVars = std::collections::BTreeMap<String, (crate::forge::model::ListElemType, u32)>;

/// A `sce-static` document's string variables, each with its declared capacity
/// in UTF-8 bytes.
type StringVars = std::collections::BTreeMap<String, u32>;

/// A `sce-static` document's byte-string variables, each with its declared
/// capacity in bytes.
type BytesVars = std::collections::BTreeMap<String, u32>;

/// What rewriting an action needs beyond its expressions: the record and
/// list variables a write to one is rewritten against, the machine name the
/// generated event type is spelled from, whether the document declares
/// `error.execution` — without it there is no variant to raise, and nothing
/// could match one — and the backend spelling it all.
struct Rewrites<'m> {
    records: RecordVars,
    /// Every schema the document imports, by alias — what the fields of a
    /// record a loop walks are read from.
    schemas: std::collections::BTreeMap<String, EventSchemaModel>,
    /// The enum each variable declared `enum:<alias>` holds, and each enum field
    /// of a record variable by its `<id>.<field>` path — what tells an enum
    /// value from a number where a `<param>` is spelled for the wire.
    enum_vars: std::collections::BTreeMap<String, String>,
    /// The enum each enum field of the payload of the transition the walk is in
    /// holds, by `_event.data.<field>` path — [`Self::open_payload`]. Written
    /// through a shared reference for the reason [`Self::sites`] is.
    payload_enum_paths: std::cell::RefCell<Vec<(String, String)>>,
    /// The fields of the payload of the transition the walk is in, which
    /// `_event.data` taken whole is a record of — [`Self::open_payload`].
    payload_fields: std::cell::RefCell<Option<Vec<String>>>,
    /// The record items of the `<foreach>`es the walk is inside, each with the
    /// alias of the schema it holds.
    loop_records: std::cell::RefCell<Vec<(String, String)>>,
    lists: ListVars,
    strings: StringVars,
    /// The byte-string variables, each held to its bound as a string is.
    bytes: BytesVars,
    /// The optional variables and the optional fields of the record variables
    /// (docs/adr/0055): what an `<assign>` to one writes the absent value of, a
    /// present value lifted into, and a text held to the bound of.
    optionals: OptionalPlaces,
    /// The optional record variables (docs/adr/0055, step 9): what an `<assign>`
    /// to one takes whole, and a field written through one is told by.
    optional_records: OptionalRecordPlaces,
    machine: &'m str,
    raises_error: bool,
    target: &'m dyn StaticTarget,
    /// The document's scope, which rewrites a call that hands an algorithm a record
    /// or a list ([`StaticScope::rewrite_state_calls`]).
    scope: &'m StaticScope,
    /// The schema of the payload of the transition the walk is in
    /// ([`Self::open_payload`]), which `_event.data` is a record of.
    payload_schema: std::cell::RefCell<Option<EventSchemaModel>>,
    /// The names ([`crate::forge::type_ctx::state_call_name`]) of the calls of an
    /// algorithm with state in a slot the walk has lowered: the adapters to declare.
    state_calls: std::cell::RefCell<BTreeSet<String>>,
    /// What the walk has lowered so far ([`StaticLowering::sites`]). Written
    /// through a shared reference because the walk reads `Rewrites` from
    /// closures.
    sites: std::cell::RefCell<Vec<LoweredSite>>,
    /// The elements the walk has lowered to another
    /// ([`StaticLowering::elements`]).
    elements: std::cell::RefCell<Vec<LoweredElement>>,
    /// The number the next `<if>` the walk writes takes
    /// ([`Action::if_ordinal`]): one past the largest the document holds, so a
    /// choice the walk expands ([`expand_computed_type`]) never shares the flag a
    /// condition that fails is recorded on with one the author wrote.
    next_if_ordinal: std::cell::Cell<u32>,
    /// Whether a host operation of the document takes an argument that may be
    /// absent (docs/adr/0055, step 10), which the host interface the machine's
    /// unit declares names the optional type of, so the unit imports the runtime's
    /// where the target's import is not read off the generated text. Set by the
    /// walk where it meets one, read once it is done.
    host_optional: std::cell::Cell<bool>,
}

impl Rewrites<'_> {
    /// The ordinal for an `<if>` the walk writes, and the one after it.
    fn take_if_ordinal(&self) -> u32 {
        let ordinal = self.next_if_ordinal.get();
        self.next_if_ordinal.set(ordinal + 1);
        ordinal
    }

    /// Opens the walk of a transition whose event's payload `schema` is
    /// declared (`None` for a walk that has none in scope): the enum fields it
    /// holds are values of the enums the schema names, and the payload taken
    /// whole is a record of its fields.
    fn open_payload(&self, schema: Option<&EventSchemaModel>) {
        *self.payload_enum_paths.borrow_mut() = schema
            .map(|schema| {
                crate::forge::static_datamodel::enum_fields(
                    crate::forge::event_schema_check::EVENT_DATA_PATH,
                    schema,
                )
            })
            .unwrap_or_default();
        *self.payload_fields.borrow_mut() =
            schema.map(|schema| schema.fields.iter().map(|f| f.id.clone()).collect());
        *self.payload_schema.borrow_mut() = schema.cloned();
    }

    /// The adapter ([`StaticTarget::state_adapter_name`]) of the call `expr` is, taken
    /// whole, when it is one of an algorithm that takes or returns a record or a
    /// list; and what it gives back.
    fn whole_state_call(&self, expr: &str) -> Option<(String, &StaticSlot)> {
        let callee = self.scope.whole_value_call(expr)?;
        let ret = callee.value.as_ref()?.ret.as_ref()?;
        let rewrite = self.rewrite_state_calls(expr).ok()?;
        let coded = rewrite.calls.first()?;
        Some((
            self.target.state_adapter_name(self.machine, callee, coded),
            ret,
        ))
    }

    /// `text` as the lowering types it: each call of an algorithm that takes a
    /// record or a list rewritten to the call it is lowered as
    /// ([`Self::with_state_calls`]), which the scope's context types by the
    /// algorithm's own signature ([`StaticScope::ctx`]). The callee's alias names
    /// no function there, so typed as written such a call is of no type and a
    /// value it answers that may be absent (docs/adr/0055) is taken for a present
    /// one. `text` itself where it makes no such call, or where the rewrite refuses
    /// it, which the lowering then reports.
    fn as_lowered(&self, text: &str) -> String {
        match self.rewrite_state_calls(text) {
            Ok(rewrite) if !rewrite.calls.is_empty() => rewrite.text,
            _ => text.to_string(),
        }
    }

    /// The calls of `text` that hand an algorithm a record or a list, found in the
    /// scope of the transition the walk is in ([`StaticScope::rewrite_state_calls`]).
    fn rewrite_state_calls(&self, text: &str) -> Result<StateRewrite, StateCallRefusal> {
        let loops = self.loop_records.borrow().clone();
        let payload = self.payload_schema.borrow().clone();
        let payload_alias = payload.as_ref().and_then(|schema| {
            self.schemas
                .iter()
                .find(|(_, held)| held.name == schema.name)
                .map(|(alias, _)| alias.as_str())
        });
        let payload_holds_text = payload.as_ref().is_some_and(|schema| {
            schema
                .fields
                .iter()
                .any(|f| matches!(f.sce_type.present_type(), SceType::String))
        });
        self.scope.rewrite_state_calls(
            text,
            &StateContext {
                loop_records: &loops,
                payload_alias,
                payload_holds_text,
            },
        )
    }

    /// `text` lowered by `lower`, with each call of an algorithm that takes a record
    /// or a list rewritten first: the argument in a state slot becomes a placeholder
    /// that renames to the value the target hands the call's adapter, and the callee
    /// becomes the adapter. An expression with no such call is lowered as it stands.
    fn with_state_calls<T>(
        &self,
        text: &str,
        renames: &HashMap<&str, &str>,
        lower: impl FnOnce(&str, &HashMap<&str, &str>) -> Result<T, Refusal>,
    ) -> Result<T, Refusal> {
        let rewrite = self.rewrite_state_calls(text).map_err(|refusal| {
            crate::forge::error::ExprError::UnsupportedConstruct {
                construct: format!("a call of `{}`: {}", refusal.callee, refusal.reason),
                observed: Some(refusal.callee),
            }
            .at(None)
        })?;
        if rewrite.calls.is_empty() {
            return lower(text, renames);
        }
        let held: Vec<String> = rewrite
            .args
            .iter()
            .map(|arg| {
                let key = if arg.code == 'p' {
                    crate::forge::event_schema_check::EVENT_DATA_PATH
                } else {
                    arg.written.as_str()
                };
                let held = match renames.get(key) {
                    Some(held) => (*held).to_string(),
                    // A loop's record item is no field of the machine: it is
                    // the local the loop binds, under the spelling every local
                    // of the target takes ([`ExprTarget::local_spelling`]).
                    None if arg.code == 'i' => self.target.expr_target().local_spelling(key),
                    None => key.to_string(),
                };
                self.target.state_argument(self.machine, arg, &held)
            })
            .collect();
        let adapters: Vec<String> = rewrite
            .calls
            .iter()
            .map(|coded| {
                let callee = self
                    .scope
                    .state_call(coded)
                    .and_then(|call| self.scope.value_callee(&call.alias))
                    .expect("a rewrite names a call of a callee with a value signature");
                self.target.state_adapter_name(self.machine, callee, coded)
            })
            .collect();
        let mut local = renames.clone();
        for (arg, text) in rewrite.args.iter().zip(&held) {
            local.insert(arg.placeholder.as_str(), text.as_str());
        }
        for (coded, adapter) in rewrite.calls.iter().zip(&adapters) {
            local.insert(coded.as_str(), adapter.as_str());
        }
        self.state_calls
            .borrow_mut()
            .extend(rewrite.calls.iter().cloned());
        lower(&rewrite.text, &local)
    }

    /// [`transpile_into_owned`] of `text`, with its state calls rewritten
    /// ([`Self::with_state_calls`]).
    fn owned(
        &self,
        text: &str,
        ctx: &crate::forge::types::TypeCtx<'_>,
        renames: &HashMap<&str, &str>,
        slot: InferredType,
    ) -> Result<Receiving, Refusal> {
        let target = self.target.expr_target();
        self.with_state_calls(text, renames, |text, renames| {
            transpile_into_owned(text, target, ctx, renames, slot)
        })
    }

    /// [`transpile_into_receiving`] of `text`, with its state calls rewritten
    /// ([`Self::with_state_calls`]).
    fn receiving(
        &self,
        text: &str,
        ctx: &crate::forge::types::TypeCtx<'_>,
        renames: &HashMap<&str, &str>,
        slot: InferredType,
    ) -> Result<Receiving, Refusal> {
        let target = self.target.expr_target();
        self.with_state_calls(text, renames, |text, renames| {
            transpile_into_receiving(text, target, ctx, renames, slot)
        })
    }

    /// The ids of the fields of the record a `<send>`'s `<content expr>` names:
    /// a record variable, a loop's record item, or the payload of the
    /// transition's event. The judge accepted only these
    /// ([`crate::forge::static_datamodel`]), so `None` is a lowering this walk
    /// lacks and not a mistake in the document.
    fn content_fields(&self, written: &str) -> Option<Vec<String>> {
        let ids = |schema: &EventSchemaModel| schema.fields.iter().map(|f| f.id.clone()).collect();
        if let Some((_, schema)) = self.records.get(written) {
            return Some(ids(schema));
        }
        if let Some((_, alias)) = self
            .loop_records
            .borrow()
            .iter()
            .find(|(item, _)| item == written)
        {
            return self.schemas.get(alias).map(ids);
        }
        if written == crate::forge::event_schema_check::EVENT_DATA_PATH {
            return self.payload_fields.borrow().clone();
        }
        None
    }

    /// The alias of the enum the variable, record field or payload field at
    /// `path` holds — what tells an enum value from a number where a `<param>`
    /// is spelled for the wire. The judge's answer
    /// ([`crate::forge::static_datamodel`]) for the same question.
    fn enum_of_wire_value(&self, path: &str) -> Option<String> {
        self.enum_vars.get(path).cloned().or_else(|| {
            self.payload_enum_paths
                .borrow()
                .iter()
                .find(|(held, _)| held == path)
                .map(|(_, alias)| alias.clone())
        })
    }

    /// Note that `source`, written in the attribute `spelling`, lowered to
    /// `text`.
    fn note(
        &self,
        source: &str,
        spelling: Option<&crate::attribute_spelling::AttributeSpelling>,
        text: &str,
    ) {
        self.sites
            .borrow_mut()
            .push(LoweredSite::new(source, spelling, text));
    }

    /// Note that the element carrying the attribute `anchor` lowered to the
    /// element `text`.
    fn note_element(
        &self,
        anchor: Option<&crate::attribute_spelling::AttributeSpelling>,
        text: &str,
    ) {
        self.elements.borrow_mut().push(LoweredElement {
            anchor: anchor.cloned(),
            text: text.to_string(),
        });
    }
}

/// C++: a variable is a member of the machine's policy struct, set by the
/// machine's own statements, and a value that can fail is computed into a local
/// first, because a failed checked operation is a value (zero, with a flag
/// raised) rather than a jump.
///
/// Lowers scalar variables, an optional value of a scalar (docs/adr/0055) in
/// `std::optional<T>`, guards, `<assign>`, `<if>`, `<log>`, `<raise>`
/// and `In()`; every construct past those is refused by name
/// ([`StaticTarget::unsupported`]) until its spelling is written, rather than
/// left as an undefined name in generated code.
pub struct CppTarget;

/// The name a C++ lambda binds the present value of an optional value to
/// ([`StaticTarget::optional_map`], [`StaticTarget::wire_optional`]): a name no
/// document's variable or local is spelled as, since a variable is a member of
/// the policy spelled `v_<id>` and a local keeps the author's spelling, which
/// takes no `sce` prefix.
const CPP_PRESENT: &str = "scePresent";

impl CppTarget {
    /// The first action of `actions`, or of a block nested in one, that this
    /// target has no lowering for yet.
    fn unlowered_action(actions: &[Action]) -> Option<String> {
        for action in actions {
            match action.action_type.as_str() {
                // A host call's arguments are typed against the machine's own
                // variables; one that reads an event payload is refused with
                // the payload itself.
                "assign" | "log" | "if" | "raise" | "cancel" | "native_action" => {}
                // A list is filled, emptied and walked by native statements.
                "sce_append" | "sce_clear" | "foreach" => {}
                // A param is the typed value of the data model crossed to the
                // event's JSON and, as the text a form carries
                // (`ScriptResultUtils::valueText`), to the request of a BasicHTTP
                // send, which the engine hands to its transport
                // (docs/adr/0005, decision 4). A literal `<content>` is the text it
                // spells.
                "send" => {}
                other => return Some(format!("<{other}>")),
            }
            for block in action.nested_blocks() {
                if let Some(found) = Self::unlowered_action(block.actions) {
                    return Some(found);
                }
            }
        }
        None
    }
}

/// `attr`, a `sce:type` spelling (`uint8`, `float64`, `bytes`), as the suffix of the
/// runtime function that writes or reads a value of it into a saved state
/// (`savedWriteUint8`, `savedReadFloat64`: `sce/include/saved/SavedValues.h`).
fn cpp_saved_name(attr: &str) -> String {
    let mut chars = attr.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

impl StaticTarget for CppTarget {
    // The start site holds the id it generates in the `std::string` local
    // `generatedInvokeId` (`defer_scxml_invoke`), which it goes on to start the
    // child under, so the variable takes it as it stands.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("generatedInvokeId")
    }
    fn name(&self) -> &'static str {
        "C++"
    }
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Cpp, callee, None))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // `std::optional<T>` (docs/adr/0055, section 5): the type is the default
    // spelling ([`crate::forge::generator::optional_of`]), and each whole value
    // names it, so a value computed into a local is of the place's own type.
    fn optional_none(&self, present: &str) -> Option<String> {
        Some(format!("{}()", self.optional_type(present)?))
    }
    fn optional_some(&self, present: &str, value: &str) -> Option<String> {
        Some(format!("{}({value})", self.optional_type(present)?))
    }
    // The present value is bound by a lambda the runtime calls only where the
    // value is present (`SCE::Forge::mapPresent`); it captures the statement's
    // `sce_failure_`, so a failure it records is the statement's.
    fn optional_map(
        &self,
        value: &str,
        present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "SCE::Forge::mapPresent({value}, [&](const {present} &{CPP_PRESENT}) {{ return {}; }})",
            map(CPP_PRESENT)
        ))
    }
    // `ScriptNull`, which the engine's wire helpers write as JSON `null`, where
    // the value is absent; the value is read once.
    fn wire_optional(
        &self,
        value: &str,
        _present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "[&]() -> ScriptValue {{ const auto &sceOptional = {value}; \
             if (!sceOptional.has_value()) {{ return ScriptValue(ScriptNull{{}}); }} \
             const auto &{CPP_PRESENT} = *sceOptional; return {}; }}()",
            wire(CPP_PRESENT)
        ))
    }
    // The literal `null` handed to an optional record is the absent value the slot's
    // type has (docs/adr/0055, step 9), which the adapter's `std::optional` takes
    // as it takes the variable that holds one.
    fn state_argument(&self, _machine: &str, arg: &StateArg, held: &str) -> String {
        if arg.code == 'n' {
            "std::nullopt".to_string()
        } else {
            held.to_string()
        }
    }
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("call_{coded}")
    }
    // The element is read by the checked read an algorithm's index is, which records
    // an index outside the list as the failure every statement of the machine ends
    // its block at, and answers the element by value.
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!(
            "SCE::Forge::Checked::at(sce_failure_, {list}, {index})"
        ))
    }
    // The present value is read through `std::optional`'s own `*`, as an expression
    // reads it where a test proves it present.
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(format!("(*{target}).{field} = {value};"))
    }
    // The machine's list is the `std::vector` the adapter answers, counted by its
    // `size()` as a byte string is.
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("SCE::Forge::Checked::bounded(sce_failure_, {value}, {capacity}u)")
    }
    fn schema_import(&self, schema_name: &str) -> Option<String> {
        Some(
            crate::forge::generator::forge_import_identity(
                schema_name,
                &Language::Cpp,
                false,
                &crate::ForgeCompileOptions::default(),
            )
            .include_stmt,
        )
    }
    // A conditional per declared variant, the last taken when every one before it is
    // not: an expression, which an aggregate initializer and a return both take, and
    // one with no path that ends without a value, which a `switch` over a scoped
    // enumeration has where its type could hold a value no variant names.
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let mut pairs = enum_variant_pairs(Language::Cpp, machine_type, model, crossing);
        let (_, last) = pairs.pop()?;
        let chosen: String = pairs
            .into_iter()
            .map(|(from, to)| format!("{value} == {from} ? {to} : "))
            .collect();
        Some(format!("({chosen}{last})"))
    }
    fn enum_import(&self, enum_name: &str) -> Option<String> {
        self.schema_import(enum_name)
    }
    // A struct of the machine's namespace whose static members are the bounds, so a
    // host names one as `<Machine>Limits::<NAME>`.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let constants: String = limits
            .iter()
            .map(|(name, value)| {
                format!(
                    "    /// {}\n    static constexpr std::uint32_t {} = {value};\n",
                    limit_doc(name),
                    name.to_uppercase()
                )
            })
            .collect();
        Some(format!(
            "/// {LIMITS_DOC}\nstruct {machine}Limits {{\n{constants}}};"
        ))
    }
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two inline functions of the machine's namespace, each the conditional over
    // every variant a call of an algorithm crosses by.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Cpp);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "/// A value of `{ty}` as the variant of the same declared name of `{document}`.\n\
             inline {document} {to}({ty} v) {{\n    return {into};\n}}\n\n\
             /// A value of `{document}` as the variant of the same declared name of `{ty}`.\n\
             inline {ty} {from}({document} v) {{\n    return {back};\n}}"
        ))
    }
    // A function of the machine's file that builds the algorithm's records from the
    // machine's, calls it, and builds the machine's own from what it returns,
    // answering as the algorithm does: an `AlgorithmResult` where it may fail, which
    // the expression the machine lowers takes the value of. What the algorithm is
    // handed is a copy, and what it returns is copied, so nothing it does reaches the
    // machine's state.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{
            cpp_positional_init, event_schema_field_ident, event_schema_payload_type,
        };
        let value = spec.callee.value.as_ref()?;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        let algorithm_record = |alias: &str| {
            plain_schema(spec, alias)
                .map(|schema| event_schema_payload_type(&schema.name, Language::Cpp))
        };
        // A record of `ty` made field by field from `from`, an enum field as the
        // variant of the same declared name the other side's type holds. Both
        // sides declare one member per schema field in the schema's order (the
        // payload struct and [`Self::record_def`]), so the record is made by
        // position in that order (docs/adr/0059, section 5).
        let record_of =
            |alias: &str, ty: String, from: &str, crossing: Crossing| -> Option<String> {
                let schema = plain_schema(spec, alias)?;
                let fields = schema
                    .fields
                    .iter()
                    .map(|f| {
                        let field = event_schema_field_ident(&f.id, Language::Cpp);
                        spec.record_field_crossing(
                            self,
                            f,
                            &format!("{from}.{field}"),
                            crossing,
                            str::to_string,
                        )
                        .map(|value| (field, value))
                    })
                    .collect::<Option<Vec<(String, String)>>>()?;
                Some(format!("{ty}{{{}}}", cpp_positional_init(&fields)))
            };
        let mut declarations = Vec::new();
        let mut prelude = String::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "{} {name}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) => {
                    if !plain_scalar(ty) {
                        return None;
                    }
                    declarations.push(format!("{} {name}", self.scalar_type(ty)));
                    arguments.push(name);
                }
                // A value of an enum that may be absent is the machine's
                // `std::optional` over its own enum, and crosses where it is present
                // by the declared name of its variant (docs/adr/0055).
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    declarations.push(format!("{} {name}", spec.optional_type(self, ty)?));
                    arguments.push(spec.optional_enum_crossing(
                        self,
                        ty,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // `std::optional<T>` on both sides, by value (docs/adr/0055).
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("{} {name}", spec.optional_type(self, ty)?));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    let held = if *code == 'p' {
                        let event = spec.payload_events.get(alias)?;
                        format!(
                            "{}{}Payload",
                            spec.machine,
                            filters::to_event_variant(event.to_string())
                        )
                    } else {
                        machine_record(alias)
                    };
                    declarations.push(format!("const {held} &{name}"));
                    arguments.push(record_of(
                        alias,
                        algorithm_record(alias)?,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // An optional record (docs/adr/0055, step 9): the algorithm takes a
                // `std::optional` of its own record type. An optional record variable
                // and `null` are held as the machine's own `std::optional`, and the
                // present record is mapped where there is one; a record variable, a
                // payload and a loop's item are records, lifted into the present
                // value.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => {
                    let algorithm = algorithm_record(alias)?;
                    let machine = machine_record(alias);
                    let present = |value: &str| {
                        record_of(alias, algorithm.clone(), value, Crossing::ToAlgorithm)
                    };
                    if matches!(*code, 'o' | 'n') {
                        // Asked once before it is bound, so a record this target
                        // cannot cross refuses the adapter.
                        present(CPP_PRESENT)?;
                        declarations
                            .push(format!("const {} &{name}", self.optional_type(&machine)?));
                        arguments.push(self.optional_map(&name, &machine, &|held| {
                            present(held).unwrap_or_default()
                        })?);
                    } else {
                        let held = if *code == 'p' {
                            let event = spec.payload_events.get(alias)?;
                            format!(
                                "{}{}Payload",
                                spec.machine,
                                filters::to_event_variant(event.to_string())
                            )
                        } else {
                            machine
                        };
                        declarations.push(format!("const {held} &{name}"));
                        arguments.push(self.optional_some(&algorithm, &present(&name)?)?);
                    }
                }
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => {
                    declarations.push(format!("const {} &{name}", self.list_type(ty)));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    declarations.push(format!(
                        "const {} &{name}",
                        self.record_list_type(&machine_record(alias))
                    ));
                    let algorithm = algorithm_record(alias)?;
                    prelude.push_str(&format!(
                        "    std::vector<{algorithm}> a{index};\n    a{index}.reserve({name}.size());\n    for (const auto &e : {name}) {{\n        a{index}.push_back({});\n    }}\n",
                        record_of(alias, algorithm.clone(), "e", Crossing::ToAlgorithm)?
                    ));
                    arguments.push(format!("a{index}"));
                }
                _ => return None,
            }
        }
        let returned_type = match value.ret.as_ref()? {
            StaticSlot::Scalar(SceType::Enum(reference)) => {
                self.enum_type(spec.machine, &reference.alias)?
            }
            StaticSlot::Scalar(ty) if plain_scalar(ty) => self.scalar_type(ty),
            StaticSlot::Optional(ty) => spec.optional_type(self, ty)?,
            StaticSlot::OptionalRecord(alias) => self.optional_type(&machine_record(alias))?,
            StaticSlot::Record(alias) => machine_record(alias),
            StaticSlot::ScalarList(ty) if plain_scalar(ty) => self.list_type(ty),
            StaticSlot::RecordList(alias) => self.record_list_type(&machine_record(alias)),
            _ => return None,
        };
        // The machine's own value from the algorithm's, `from`.
        let converted = |from: &str| -> Option<String> {
            Some(match value.ret.as_ref()? {
                StaticSlot::Scalar(SceType::Enum(reference)) => {
                    spec.enum_crossing(self, &reference.alias, from, Crossing::FromAlgorithm)?
                }
                // The machine's `std::optional` over its own enum, mapped where the
                // algorithm's answer is present (docs/adr/0055).
                StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    spec.optional_enum_crossing(self, ty, from, Crossing::FromAlgorithm)?
                }
                StaticSlot::Scalar(_) | StaticSlot::ScalarList(_) | StaticSlot::Optional(_) => {
                    from.to_string()
                }
                // The machine's own record, made of the algorithm's where there is
                // one, and absent where it is absent (docs/adr/0055, step 9).
                StaticSlot::OptionalRecord(alias) => {
                    let algorithm = algorithm_record(alias)?;
                    let machine = machine_record(alias);
                    let present = |value: &str| {
                        record_of(alias, machine.clone(), value, Crossing::FromAlgorithm)
                    };
                    present(CPP_PRESENT)?;
                    self.optional_map(from, &algorithm, &|held| present(held).unwrap_or_default())?
                }
                StaticSlot::Record(alias) => {
                    record_of(alias, machine_record(alias), from, Crossing::FromAlgorithm)?
                }
                StaticSlot::RecordList(alias) => {
                    let list = self.record_list_type(&machine_record(alias));
                    let element =
                        record_of(alias, machine_record(alias), "e", Crossing::FromAlgorithm)?;
                    format!(
                        "[&] {{ {list} out; out.reserve({from}.size()); for (const auto &e : {from}) {{ out.push_back({element}); }} return out; }}()"
                    )
                }
            })
        };
        let call = format!("{}({})", spec.call, arguments.join(", "));
        Some(if spec.callee.may_fail {
            let result = format!("SCE::Forge::AlgorithmResult<{returned_type}>");
            format!(
                "inline {result} {}({}) {{\n{prelude}    const auto r = {call};\n    if (!r.ok()) {{\n        return {result}::failure(r.error());\n    }}\n    return {result}::success({});\n}}",
                spec.name,
                declarations.join(", "),
                converted("r.value()")?
            )
        } else {
            format!(
                "inline {returned_type} {}({}) {{\n{prelude}    const auto r = {call};\n    return {};\n}}",
                spec.name,
                declarations.join(", "),
                converted("r")?
            )
        })
    }
    fn unsupported(&self, model: &SCXMLModel, _scope: &StaticScope) -> Option<String> {
        for state in model.states.values() {
            let blocks = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for block in blocks {
                if let Some(found) = Self::unlowered_action(block) {
                    return Some(found);
                }
            }
            if let Some(other) = unlowered_invoke(
                &state.invokes,
                true,
                self.lowers_hybrid_invoke(),
                self.lowers_mesh_invoke(),
            ) {
                return Some(other);
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Cpp
    }
    // A prefix keeps a variable's member from meeting another member of the
    // policy, which a bare name could (`state`, `engine`).
    fn field_name(&self, id: &str) -> String {
        format!("v_{}", filters::to_snake_case(id.to_string()))
    }
    // C++ keeps the author's spelling for a reader, as every typed reader of
    // the language does (`crate::reader_names`), and gives none to a name the
    // language reserves or the policy or the machine already has a member by.
    fn reader_name(&self, id: &str, machine: &str) -> Option<String> {
        let spelled = id.replace(['.', '-'], "_");
        (!crate::reader_names::reader_is_taken(Language::Cpp, &spelled, machine)).then_some(spelled)
    }
    fn field_ref(&self, name: &str) -> String {
        name.to_string()
    }
    fn in_function(&self) -> &'static str {
        "isStateActive"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::cpp_type(ty).to_string()
    }
    // A string or a byte string is lent to the host, not copied out of the
    // machine, and as a constant reference no host can write into it.
    fn scalar_view(&self, ty: &SceType) -> Option<String> {
        match ty {
            SceType::String => Some("const std::string&".to_string()),
            SceType::Bytes => Some("const std::vector<uint8_t>&".to_string()),
            _ => None,
        }
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    // Plain data by the record rule: a struct of the schema's fields in the
    // schema's order, which a value is built from by position in that order
    // (docs/adr/0059, section 5).
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                let present = match field.sce_type.present_type() {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::cpp_type(other).to_string(),
                };
                // An optional field is held in the optional type over its
                // present value's (docs/adr/0055).
                let field_ty = match field.sce_type {
                    FieldType::Optional { .. } => self
                        .optional_type(&present)
                        .expect("C++ spells an optional value"),
                    FieldType::Scalar(_) => present,
                };
                format!("    {field_ty} {};\n", self.record_field(&field.id))
            })
            .collect();
        // Its saved form (`SCE::Saved`) is two functions beside the type, so the record
        // stays the plain data a value is built from by position: an
        // object of the schema's fields keyed by their ids, written by `{ty}ToSaved` and
        // read by `{ty}FromSaved`, the names the enum's own saved form has
        // ([`Self::enum_def`]). An optional field is written as null or as its present
        // value, and read back absent from null and otherwise as its present value is
        // (docs/adr/0055, section 4).
        let writes: String = schema
            .fields
            .iter()
            .map(|field| {
                let member = format!("value.{}", self.record_field(&field.id));
                let writer = match field.sce_type.present_type() {
                    SceType::Enum(reference) => format!("{}ToSaved", enum_types[&reference.alias]),
                    other => format!(
                        "::SCE::Saved::savedWrite{}",
                        cpp_saved_name(&other.as_attr())
                    ),
                };
                let saved = match field.sce_type {
                    FieldType::Optional { .. } => {
                        format!("::SCE::Saved::savedWriteOptional({member}, {writer})")
                    }
                    FieldType::Scalar(_) => format!("{writer}({member})"),
                };
                format!(
                    "    members.set(\"{}\", {saved});\n",
                    filters::escape_cpp(field.id.clone())
                )
            })
            .collect();
        let reads: String = schema
            .fields
            .iter()
            .map(|field| {
                let id = filters::escape_cpp(field.id.clone());
                let reader = match (field.sce_type.present_type(), field.max_size) {
                    (SceType::Enum(reference), _) => {
                        format!("{}FromSaved", enum_types[&reference.alias])
                    }
                    // A string is read back only if it fits the bound the schema
                    // declares, as a string variable's is, and so is a byte string, in
                    // bytes.
                    (SceType::String, Some(bound)) => {
                        format!("::SCE::Saved::savedBoundedString({bound}u)")
                    }
                    (SceType::Bytes, Some(bound)) => {
                        format!("::SCE::Saved::savedBoundedBytes({bound}u)")
                    }
                    (SceType::Bytes, None) => {
                        format!("::SCE::Saved::savedBoundedBytes({}u)", i32::MAX)
                    }
                    (other, _) => {
                        format!(
                            "::SCE::Saved::savedRead{}",
                            cpp_saved_name(&other.as_attr())
                        )
                    }
                };
                let what = format!("what + \".{id}\"");
                let held = format!("::SCE::Saved::savedField(value, what, \"{id}\")");
                let read = match &field.sce_type {
                    FieldType::Optional { .. } => {
                        format!("::SCE::Saved::savedReadOptional({held}, {what}, {reader})")
                    }
                    FieldType::Scalar(_) => format!("({reader})({held}, {what})"),
                };
                // In the order `fields` declares the struct, one member each.
                format!(
                    "        {},\n",
                    crate::forge::generator::cpp_positional_member(
                        &self.record_field(&field.id),
                        &read
                    )
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\nstruct {ty} {{\n{fields}}};\n\n\
             /// `value` as a saved state writes it.\n\
             inline ::SCE::Saved::JsonValue {ty}ToSaved(const {ty} &value) {{\n    \
             ::SCE::Saved::JsonObject members;\n{writes}    \
             return ::SCE::Saved::JsonValue(std::move(members));\n}}\n\n\
             /// The value a saved state holds, refused unless it is one.\n\
             inline {ty} {ty}FromSaved(const ::SCE::Saved::JsonValue &value, const std::string &what) {{\n    \
             return {ty}{{\n{reads}    }};\n}}"
        )
    }
    // The schema's id spelled as this file's payload structs spell their
    // fields: one file, one spelling of a schema field.
    fn record_field(&self, id: &str) -> String {
        crate::forge::generator::event_schema_field_ident(id, Language::Cpp)
    }
    // Every caller hands every schema field, in the schema's order, which is the
    // order [`Self::record_def`] declares the struct: so by position.
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        format!(
            "{ty}{{{}}}",
            crate::forge::generator::cpp_positional_init(fields)
        )
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    fn enum_variant(&self, _ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::Cpp, enum_name, variant)
    }
    // A scoped enumeration over the enum document's own carrier, each variant
    // holding the value the document declares for it. `sceLogName` answers the
    // name the document gives a value, which is what a `<log>` shows and what a
    // host compares to a scenario: a scoped enum has no `fmt` spelling.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    {} = {},\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    v.value
                )
            })
            .collect();
        let names: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    case {ty}::{}: return \"{}\";\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    filters::escape_cpp(v.name.clone())
                )
            })
            .collect();
        // The enum document's own accessors (`forge/cpp/enum.h.jinja2`), named for
        // the machine's type, since a machine's enums share its namespace and a
        // function cannot be overloaded on what it returns: what `<alias>.toNumber`
        // and `<alias>.fromNumber` are lowered to ([`StaticTarget::enum_number_symbols`]).
        let carrier = crate::forge::generator::cpp_type(&model.underlying_type);
        let variants_of: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    case {}:\n        return {ty}::{};\n",
                    v.value,
                    self.enum_variant(ty, &model.name, &v.name)
                )
            })
            .collect();
        // Its saved form is the declared name the enum document gives the variant,
        // written by `{ty}ToSaved` and read by `{ty}FromSaved`, beside the type as a
        // record's own are ([`Self::record_def`]).
        let saved_variants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "        {{\"{}\", {ty}::{}}},\n",
                    filters::escape_cpp(v.name.clone()),
                    self.enum_variant(ty, &model.name, &v.name)
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n\
             enum class {ty} : {carrier} {{\n{variants}}};\n\n\
             /// The name the enum document declares for `value`.\n\
             inline const char *sceLogName({ty} value) {{\n    switch (value) {{\n{names}    }}\n    return \"\";\n}}\n\n\
             /// The number the enum document declares for `value`.\n\
             inline {carrier} {ty}ToUnderlying({ty} value) {{\n    return static_cast<{carrier}>(value);\n}}\n\n\
             /// The variant that declares `raw`, or `std::nullopt` for a number no variant declares.\n\
             inline std::optional<{ty}> {ty}FromUnderlying({carrier} raw) {{\n    switch (raw) {{\n{variants_of}    default:\n        return std::nullopt;\n    }}\n}}\n\n\
             /// `value` as a saved state writes it.\n\
             inline ::SCE::Saved::JsonValue {ty}ToSaved({ty} value) {{\n    return ::SCE::Saved::JsonValue(std::string(sceLogName(value)));\n}}\n\n\
             /// The value a saved state holds, refused unless it is a variant.\n\
             inline {ty} {ty}FromSaved(const ::SCE::Saved::JsonValue &value, const std::string &what) {{\n    \
             static const std::map<std::string, {ty}> variants = {{\n{saved_variants}    }};\n    \
             return ::SCE::Saved::savedReadVariant(value, what, \"{alias}\", variants);\n}}"
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (format!("{ty}ToUnderlying"), format!("{ty}FromUnderlying"))
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("std::vector<{}>", crate::forge::generator::cpp_type(elem))
    }
    // The machine's own vector is lent, so a host cannot grow it past the
    // bound the machine keeps.
    fn list_view(&self, elem: &SceType) -> Option<String> {
        Some(format!(
            "const std::vector<{}>&",
            crate::forge::generator::cpp_type(elem)
        ))
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("std::vector<{record}>")
    }
    fn record_list_view(&self, record: &str) -> Option<String> {
        Some(format!("const std::vector<{record}>&"))
    }
    fn list_empty(&self) -> String {
        "{}".to_string()
    }
    // Records the failure and answers an empty string, as a checked operation
    // answers zero. A literal is a `const char *`, which holds no size, so the
    // value is made the owned `std::string` a variable holds before it is
    // counted.
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("SCE::Forge::Checked::bounded(sce_failure_, std::string({value}), {capacity}u)")
    }
    // A byte string is already the `std::vector<uint8_t>` a variable holds (a
    // literal is the one the emitter writes), whose `size()` counts bytes. A record's
    // `std::vector<uint8_t>` field is the same, in a struct that is handed to a host
    // by value and held in a list the host is lent as a constant reference; a
    // payload's is read where the machine's own is, and held to the bound of the
    // variable or the record's field it is written to. The inject seam takes the
    // vector by value and the payload holds it by value, so what the machine keeps
    // is never the host's own.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        format!("SCE::Forge::Checked::bounded(sce_failure_, {value}, {capacity}u)")
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
    }
    // An algorithm's list is the `std::vector` the machine's own is, counted by
    // its `size()` as a byte string is.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("SCE::Forge::Checked::bounded(sce_failure_, {value}, {capacity}u)")
    }
    // The count is the engine's the action is handed.
    fn fresh_send_id(&self) -> String {
        "engine.nextAutoSendId()".to_string()
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value};")
    }
    // Through `sceLogName`, which an enum declares beside its type and every
    // other value passes through unchanged.
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
        if label.is_empty() {
            format!("SCE_LOG_INFO(\"{{}}\", sceLogName({value}));")
        } else {
            format!(
                "SCE_LOG_INFO(\"{{}}: {{}}\", \"{}\", sceLogName({value}));",
                filters::escape_cpp(label.to_string())
            )
        }
    }
    // An expression that is `true` when the append failed: the list is full, or
    // the value could not be computed. The room is checked first, so a full
    // list computes nothing, and the value is computed into a local before it
    // is pushed, so a failed one leaves the list as it was.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        _value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        format!(
            "([&]() -> bool {{ [[maybe_unused]] SCE::Forge::AlgorithmFailure sce_failure_; \
             if ({target}.size() >= {capacity}) {{ {overflow} return true; }} \
             auto sce_value = {value}; \
             if (sce_failure_.failed()) {{ {failed} return true; }} \
             {target}.push_back(sce_value); return false; }})()"
        )
    }
    fn clear(&self, target: &str) -> String {
        format!("{target}.clear();")
    }
    // The loop walks the list as it was when it began (§scxml-4.6: a shallow
    // copy), so a body that appends to the list does not move it. The copy is
    // made by `sceCopy` / `sceIndexed`, declared beside the machine's types, and
    // lives as long as the loop. The position is a `uint32`, as `len` is.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        Some(match index {
            None => (
                format!("for ([[maybe_unused]] const auto &{item} : sceCopy({list}))"),
                String::new(),
            ),
            Some(index) => (
                format!(
                    "for ([[maybe_unused]] const auto &[{index}, {item}] : sceIndexed({list}))"
                ),
                String::new(),
            ),
        })
    }
    fn raise_execution_error(&self, _machine: &str, message: &str) -> String {
        format!(
            "engine.raise(typename Engine::EventWithMetadata(Event::Error_execution, \"{}\"));",
            filters::escape_cpp(message.to_string())
        )
    }
    // The runtime's checked helpers record a failure in `sce_failure_` and
    // answer a zero, so a statement that wrote its value would write that zero.
    // The expression is a lambda called where it stands, answering whether it
    // failed, so the dispatcher can end its block as it does for any error.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "([&]() -> bool {{ SCE::Forge::AlgorithmFailure sce_failure_; {statement} \
             if (sce_failure_.failed()) {{ {failed} return true; }} return false; }})()"
        )
    }
    fn receiving_write(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        _ty: InferredType,
        failed: &str,
    ) -> String {
        format!(
            "([&]() -> bool {{ SCE::Forge::AlgorithmFailure sce_failure_; auto sce_value = {value}; \
             if (sce_failure_.failed()) {{ {failed} return true; }} {} return false; }})()",
            write("sce_value")
        )
    }
    // Only a host call's arguments reach here, and its one spelling is
    // [`Self::receiving_host_call`].
    fn receiving_call(&self, _statement: &str, _failed: &str) -> String {
        unreachable!("a C++ host call is received through receiving_host_call")
    }
    // A failed checked operation is a value, so the call could not be stopped
    // after its arguments were evaluated: each is computed into a local first,
    // and the host is called only when none of them failed. A block of its own,
    // so two calls in one scope do not declare the same locals.
    fn receiving_host_call(
        &self,
        _statement: &str,
        callee: &str,
        args: &[String],
        _arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        let locals: String = args
            .iter()
            .enumerate()
            .map(|(i, arg)| format!("auto sce_arg{i} = {arg}; "))
            .collect();
        let names: Vec<String> = (0..args.len()).map(|i| format!("sce_arg{i}")).collect();
        format!(
            "{{ SCE::Forge::AlgorithmFailure sce_failure_; {locals}\
             if (sce_failure_.failed()) {{ {failed} }} else {{ {callee}({}); }} }}",
            names.join(", ")
        )
    }
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        format!(
            "([&]() -> bool {{ SCE::Forge::AlgorithmFailure sce_failure_; bool sce_value = {value}; \
             if (sce_failure_.failed()) {{ {failed} {flag} return false; }} return sce_value; }})()"
        )
    }
    // The flag is a local of the `<if>`, named by its ordinal, which the
    // condition's lambda sets by reference.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String {
        format!("ifCondFailed{if_ordinal} = true;")
    }
    // An enum field of the payload is held in the machine's own enum, lifted
    // from the variant's declared name and written back as `sceLogName` answers
    // it (`build_cpp_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`SendSchedulingHelper::parseDelayString`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes
    // (`engine.resolveEventByName`).
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes
    // (`engine.cancelEvent`).
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template carries the value as the event's JSON and as the text a
    // host takes (`EventDataHelper::scriptValueToJsonString`,
    // `ScriptResultUtils::resultToString`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`hostInvoke.src`).
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`SCE::documentStem`) and starts the candidate it names, handing it the
    // values it keeps (`seed_static_child`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // A Mesh request goes out by the router the build generates for the
    // deployment (`performMeshInvoke`); its params are written from the
    // machine's fields as a host-run invoke's are.
    fn lowers_mesh_invoke(&self) -> bool {
        true
    }
    // The member the payload channel fills when the engine dequeues an event
    // of this name (`build_cpp_event_payload`), read by the typed guards and
    // by a `<sce:action>`'s arguments alike.
    fn payload_accessor(&self, event: &str) -> String {
        format!(
            "pending{}Payload_",
            filters::to_event_variant(event.to_string())
        )
    }
    // The tag says which event's payload the channel holds now, so a guard
    // that reads this event's fields holds only for a delivery that carried
    // them.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "pendingPayloadTag_ == {machine}PayloadTag::{} && ({lowered})",
            filters::to_event_variant(event.to_string())
        )
    }
    // The runtime's `ScriptValue`, which `DoneDataHelper::collectParams` writes
    // as the pair's JSON value, built as the alternative it names: a narrow
    // integer widens to `int64_t` and a real to `double`, both exactly; a 64-bit
    // integer is the text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!("ScriptValue(std::in_place_type<bool>, {value})"),
            InferredType::Str => {
                format!("ScriptValue(std::in_place_type<std::string>, {value})")
            }
            InferredType::Int { bits: 64, .. } => {
                format!("ScriptValue(std::in_place_type<std::string>, std::to_string({value}))")
            }
            InferredType::Int { .. } => {
                format!("ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>({value}))")
            }
            _ => format!("ScriptValue(std::in_place_type<double>, static_cast<double>({value}))"),
        }
    }
    // `sceLogName`, which an enum declares beside its type, answers a
    // `const char *`; the string value holds a `std::string`.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("std::string(sceLogName({value}))"))
    }
    // The text `SCE::Latin1Bytes` spells a byte vector as, which the string value
    // holds as the `std::string` it is.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(self.wire_value(
            InferredType::Str,
            &format!("SCE::Latin1Bytes::textOf({value})"),
        ))
    }
}

/// The first name of `namelist` that a `<param>` of `params` or an earlier name of
/// the namelist already is.
///
/// A `<param>` name that repeats collects its values, in document order, into one
/// array of the event's data on every engine (ARCHITECTURE.md, "JSON Object Key
/// Order"; W3C SCXML test178), which a C writer writes by itself as it is given the
/// pairs. A `namelist` name is a pair like a `<param>`, but where it stands
/// among the `<param>`s that share its name is not a document order the engines
/// were held to, so a target that writes only the `<param>`s' repeats refuses it by
/// name rather than choose one.
fn namelist_name_taken<'a>(
    params: impl Iterator<Item = &'a str>,
    namelist: &'a str,
) -> Option<&'a str> {
    let mut seen: std::collections::BTreeSet<&str> = params.collect();
    namelist.split_whitespace().find(|name| !seen.insert(*name))
}

/// The first of `invokes` a target that lowers a `<invoke type="scxml">` has no
/// lowering for yet, described for a refusal. A scxml child is started by the
/// machine's own invoke code and handed its values by the build; a mesh one is
/// not lowered yet, a hybrid one only by a target that `lowers_hybrid` — one whose
/// invoke code reads the stem of the string its `srcexpr` computes — and one the
/// host runs only by a target that `lowers_host_run` — one whose invoke code reads
/// a request's `<param>`s from the machine's own fields, and a Mesh request only by
/// a target that `lowers_mesh` — one whose backend generates a router of its own.
fn unlowered_invoke(
    invokes: &[crate::model::Invoke],
    lowers_host_run: bool,
    lowers_hybrid: bool,
    lowers_mesh: bool,
) -> Option<String> {
    invokes
        .iter()
        .find(|i| match i {
            crate::model::Invoke::Scxml(_) => false,
            crate::model::Invoke::Hybrid(_) => !lowers_hybrid,
            crate::model::Invoke::MeshRpc(_) => !lowers_mesh,
            crate::model::Invoke::Unsupported(info) => !(lowers_host_run && info.host_served),
        })
        .map(|other| {
            match other {
                crate::model::Invoke::Hybrid(_) => "a hybrid <invoke>",
                crate::model::Invoke::MeshRpc(_) => "a mesh <invoke>",
                _ => "a host-run <invoke>",
            }
            .to_string()
        })
}

/// Whether a record of `schema` has a byte-string field ([`StaticField::holds_bytes`]).
fn holds_bytes(schema: &EventSchemaModel) -> bool {
    schema
        .fields
        .iter()
        .any(|f| matches!(f.sce_type.present_type(), SceType::Bytes))
}

/// Whether a record of `schema` owns a buffer — has a `string` or a `bytes` field —
/// which a target that holds plain data by value must hold otherwise.
fn owns_a_buffer(schema: &EventSchemaModel) -> bool {
    schema
        .fields
        .iter()
        .any(|f| matches!(f.sce_type.present_type(), SceType::String | SceType::Bytes))
}

/// Rewrite `model` — a clone the C++ backend renders — so every expression of
/// a `sce-static` document is native C++.
pub fn lower_cpp(model: &mut SCXMLModel, machine: &str) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &CppTarget)
}

/// The name a Go function literal binds the present value of an optional value
/// to ([`StaticTarget::optional_map`], [`StaticTarget::wire_optional`]): a name no
/// document's variable or local is spelled as, since a local keeps the author's
/// spelling and a variable is a field of the policy.
const GO_PRESENT: &str = "scePresent";

/// Go: a variable is a field of the machine's policy struct, set by the
/// machine's own statements. A failed checked operation is a value (zero, with
/// a flag raised in `sceFailure`) rather than a jump, as in C++, so a value that
/// can fail is computed into a local first and written only when it did not.
///
/// Lowers scalar, enum, list and record variables, an optional value of a scalar
/// (docs/adr/0055), guards, `<assign>`, `<if>`, `<foreach>`, `<log>`, `<raise>`,
/// `In()`, a typed payload, a call of an imported algorithm and a `<sce:action>`
/// whose arguments are typed expressions of the machine's variables; every
/// construct past those is refused by name ([`StaticTarget::unsupported`]) until
/// its spelling is written, rather than left as an undefined name in generated
/// code.
pub struct GoTarget<'a> {
    /// The Go module path the generated packages live under, which an import
    /// line of an algorithm's package is written from. `None` for a target
    /// that only spells calls (`target_for`), and for a document that calls no
    /// algorithm.
    import_root: Option<&'a str>,
}

impl GoTarget<'_> {
    /// `parts` as the statements of one block, the empty ones left out — what
    /// runs on a failure may be nothing, when the document declares no
    /// `error.execution` to raise.
    fn then(parts: &[&str]) -> String {
        parts
            .iter()
            .map(|part| part.trim().trim_end_matches(';').trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The first action of `actions`, or of a block nested in one, that this
    /// target has no lowering for yet.
    fn unlowered_action(actions: &[Action]) -> Option<String> {
        for action in actions {
            match action.action_type.as_str() {
                "assign" | "log" | "if" | "raise" | "cancel" | "native_action" => {}
                // A param is the typed value of the data model crossed to the
                // event's JSON, and a literal `<content>` is the text it spells.
                "send" => {}
                // A list is filled, emptied and walked by native statements.
                "sce_append" | "sce_clear" | "foreach" => {}
                other => return Some(format!("<{other}>")),
            }
            for block in action.nested_blocks() {
                if let Some(found) = Self::unlowered_action(block.actions) {
                    return Some(found);
                }
            }
        }
        None
    }
}

impl StaticTarget for GoTarget<'_> {
    // The start site holds the id it generates in the `string` local
    // `generatedInvokeID` (`ExecuteEntryActions`), which it goes on to start the
    // child under, so the variable takes it as it stands.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("generatedInvokeID")
    }
    fn name(&self) -> &'static str {
        "Go"
    }
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Go, callee, self.import_root))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // `optional.Of[T]` of the Go forge runtime (docs/adr/0055, section 5): the
    // type is the default spelling ([`crate::forge::generator::optional_of`]),
    // each value one of the package's constructors, which name `T`.
    fn optional_none(&self, present: &str) -> Option<String> {
        Some(crate::forge::generator::go_optional_none(present))
    }
    fn optional_some(&self, present: &str, value: &str) -> Option<String> {
        Some(format!("optional.Some[{present}]({value})"))
    }
    // The present value is bound by a function literal the package calls only
    // where the value is present; what it records in the statement's
    // `sceFailure` is the statement's.
    fn optional_map(
        &self,
        value: &str,
        present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "optional.Map({value}, func({GO_PRESENT} {present}) {present} {{ return {} }})",
            map(GO_PRESENT)
        ))
    }
    // The package's `Convert`, over a function literal that names both types.
    fn optional_convert(
        &self,
        value: &str,
        from: &str,
        to: &str,
        convert: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "optional.Convert({value}, func({GO_PRESENT} {from}) {to} {{ return {} }})",
            convert(GO_PRESENT)
        ))
    }
    // `nil`, which `ScriptValueToJSON` and `PayloadJSON` write as `null`, where
    // the value is absent.
    fn wire_optional(
        &self,
        value: &str,
        present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "optional.Wire({value}, func({GO_PRESENT} {present}) any {{ return {} }})",
            wire(GO_PRESENT)
        ))
    }
    // Kept only where the machine's text names the package
    // ([`crate::std_imports::go_imports_named_in`]).
    fn optional_import(&self) -> Option<String> {
        Some(format!(
            "\t\"{}\"",
            crate::std_imports::GO_FORGE_OPTIONAL_PATH
        ))
    }
    // An optional record is `optional.Of[R]` over the record's own struct
    // (docs/adr/0055, step 9), built by the same two constructors as any other;
    // the present record is a value `Proven` answers, so a field of it is written
    // into a copy of the record and the copy is put back as the present value.
    // `target` is read before the field is, and the value beside it is evaluated
    // inside the block, so a value that reads the record reads what it held.
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(format!(
            "{{ sceRecord := {target}.Proven(); sceRecord.{field} = {value}; \
             {target} = optional.Some(sceRecord) }}"
        ))
    }
    // The element is read through the runtime's `At`, which records an
    // out-of-range failure in the statement's `sceFailure` and answers the zero
    // value, as every read of an element does; a list of records is a slice of the
    // machine's own struct, so the element is the record whole.
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!(
            "scealgorithm.At(&sceFailure, {list}[:], int64({index}))"
        ))
    }
    // `null` handed to an optional record slot is the absent value of the machine's
    // own record type; any other argument is the value the walk holds.
    fn state_argument(&self, machine: &str, arg: &StateArg, held: &str) -> String {
        match (&arg.slot, arg.code) {
            (StaticSlot::OptionalRecord(alias), 'n') => {
                format!("optional.None[{}]()", self.record_type(machine, alias))
            }
            _ => held.to_string(),
        }
    }
    // The prefix tells the expression layer the adapter answers an `Outcome`. The
    // machine follows it because a package holds a parent and every child it
    // invokes, and two of them may call one algorithm the same way, each through an
    // adapter in its own record types.
    fn state_adapter_name(&self, machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!(
            "{}{machine}_{coded}",
            crate::forge::expr::STATE_ADAPTER_PREFIX
        )
    }
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("scealgorithm.Within(&sceFailure, {value}, {capacity})")
    }
    fn schema_import(&self, schema_name: &str) -> Option<String> {
        let options = crate::ForgeCompileOptions {
            go_module_prefix: self.import_root.map(str::to_owned),
            ..Default::default()
        };
        Some(
            crate::forge::generator::forge_import_identity(
                schema_name,
                &Language::Go,
                false,
                &options,
            )
            .include_stmt,
        )
    }
    // Go has no conditional expression, so the variants are a map literal read at the
    // value: an expression, which a composite literal's field and an argument take,
    // and one that holds a key for every value either type has a name for.
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let algorithm_type =
            crate::forge::generator::enum_qualified_type(&model.name, Language::Go);
        let (from_type, to_type) = match crossing {
            Crossing::ToAlgorithm => (machine_type.to_string(), algorithm_type),
            Crossing::FromAlgorithm => (algorithm_type, machine_type.to_string()),
        };
        let entries: Vec<String> = enum_variant_pairs(Language::Go, machine_type, model, crossing)
            .into_iter()
            .map(|(from, to)| format!("{from}: {to}"))
            .collect();
        Some(format!(
            "map[{from_type}]{to_type}{{{}}}[{value}]",
            entries.join(", ")
        ))
    }
    // The enum document's package, imported as a schema's is.
    fn enum_import(&self, enum_name: &str) -> Option<String> {
        self.schema_import(enum_name)
    }
    // Typed constants of the package, named by the machine as everything a package
    // that may hold several machines declares is.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let constants: Vec<String> = limits
            .iter()
            .map(|(name, value)| {
                let constant = format!(
                    "{machine}Limit{}",
                    filters::to_pascal_case(name.to_string())
                );
                format!(
                    "// {constant} is the bound `sce:limit=\"{name}\"` names, as this build \
                     resolved it.\nconst {constant} uint32 = {value}"
                )
            })
            .collect();
        Some(format!("// {LIMITS_DOC}\n\n{}", constants.join("\n\n")))
    }
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two functions of the package, each the map over every variant a call of an
    // algorithm crosses by.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Go);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "// {to} is a value of {ty} as the variant of the same declared name of {document}.\n\
             func {to}(v {ty}) {document} {{\n\treturn {into}\n}}\n\n\
             // {from} is a value of {document} as the variant of the same declared name of {ty}.\n\
             func {from}(v {document}) {ty} {{\n\treturn {back}\n}}"
        ))
    }
    // The adapter reads what the algorithm returns by its fields (`e.Row`) and writes
    // the machine's own type, so the package of a record only a return names would be
    // an import nothing uses.
    fn adapter_names_returned_schema(&self) -> bool {
        false
    }
    // A function of the machine's file that builds the algorithm's records from the
    // machine's, calls it, and builds the machine's own record or list from what it
    // gave back, answering an `Outcome` of the machine's type where the algorithm may
    // fail. A record or a list it is handed is never one the machine keeps, and the
    // one it returns is copied, so what it does to them cannot reach the machine's
    // state.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{event_schema_field_ident, event_schema_payload_type};
        let value = spec.callee.value.as_ref()?;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        let algorithm_record = |alias: &str| {
            plain_schema(spec, alias)
                .map(|schema| event_schema_payload_type(&schema.name, Language::Go))
        };
        let to_algorithm = |alias: &str, from: &str| -> Option<String> {
            let schema = plain_schema(spec, alias)?;
            let fields = schema
                .fields
                .iter()
                .map(|f| {
                    spec.record_field_crossing(
                        self,
                        f,
                        &format!("{from}.{}", self.record_field(&f.id)),
                        Crossing::ToAlgorithm,
                        str::to_string,
                    )
                    .map(|value| {
                        format!("{}: {value}", event_schema_field_ident(&f.id, Language::Go))
                    })
                })
                .collect::<Option<Vec<String>>>()?;
            Some(format!(
                "{}{{{}}}",
                algorithm_record(alias)?,
                fields.join(", ")
            ))
        };
        let mut declarations = Vec::new();
        let mut prelude = String::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "{name} {}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) => {
                    if !plain_scalar(ty) {
                        return None;
                    }
                    declarations.push(format!("{name} {}", self.scalar_type(ty)));
                    arguments.push(name);
                }
                // A value of an enum that may be absent is the machine's
                // `optional.Of` over its own enum, and crosses where it is present
                // by the declared name of its variant (docs/adr/0055).
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    declarations.push(format!("{name} {}", spec.optional_type(self, ty)?));
                    arguments.push(spec.optional_enum_crossing(
                        self,
                        ty,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // `optional.Of[T]` on both sides, a value (docs/adr/0055).
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("{name} {}", spec.optional_type(self, ty)?));
                    arguments.push(name);
                }
                // An optional record the machine holds as one (a variable of the
                // type, or the literal `null`, which the call hands as the absent
                // value of it) is the algorithm's own optional record where it is
                // present: its fields are copied as a record's are, and no type of
                // the algorithm's is named but the one the argument is declared as
                // (docs/adr/0055, step 9).
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code: 'o' | 'n',
                } => {
                    declarations.push(format!(
                        "{name} {}",
                        self.optional_type(&machine_record(alias))?
                    ));
                    prelude.push_str(&format!(
                        "\tvar a{index} {}\n\tif v, ok := {name}.Get(); ok {{\n\t\ta{index} = optional.Some({})\n\t}}\n",
                        self.optional_type(&algorithm_record(alias)?)?,
                        to_algorithm(alias, "v")?
                    ));
                    arguments.push(format!("a{index}"));
                }
                // A record the optional slot takes as a present value, held as a
                // record slot's argument is.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => {
                    let held = if *code == 'p' {
                        let event = spec.payload_events.get(alias)?;
                        format!(
                            "{}{}Payload",
                            spec.machine,
                            filters::to_event_variant(event.to_string())
                        )
                    } else {
                        machine_record(alias)
                    };
                    declarations.push(format!("{name} {held}"));
                    arguments.push(format!("optional.Some({})", to_algorithm(alias, &name)?));
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    let held = if *code == 'p' {
                        let event = spec.payload_events.get(alias)?;
                        format!(
                            "{}{}Payload",
                            spec.machine,
                            filters::to_event_variant(event.to_string())
                        )
                    } else {
                        machine_record(alias)
                    };
                    declarations.push(format!("{name} {held}"));
                    arguments.push(to_algorithm(alias, &name)?);
                }
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => {
                    declarations.push(format!("{name} {}", self.list_type(ty)));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    declarations.push(format!(
                        "{name} {}",
                        self.record_list_type(&machine_record(alias))
                    ));
                    prelude.push_str(&format!(
                        "\ta{index} := make([]{}, len({name}))\n\tfor i, e := range {name} {{\n\t\ta{index}[i] = {}\n\t}}\n",
                        algorithm_record(alias)?,
                        to_algorithm(alias, "e")?
                    ));
                    arguments.push(format!("a{index}"));
                }
                _ => return None,
            }
        }
        // The machine's own value from what the algorithm returned, `r`.
        let from_algorithm = |alias: &str, from: &str| -> Option<String> {
            let schema = plain_schema(spec, alias)?;
            let fields = schema
                .fields
                .iter()
                .map(|f| {
                    spec.record_field_crossing(
                        self,
                        f,
                        &format!("{from}.{}", event_schema_field_ident(&f.id, Language::Go)),
                        Crossing::FromAlgorithm,
                        str::to_string,
                    )
                    .map(|value| format!("{}: {value}", self.record_field(&f.id)))
                })
                .collect::<Option<Vec<String>>>()?;
            Some(format!(
                "{}{{{}}}",
                machine_record(alias),
                fields.join(", ")
            ))
        };
        let (returned_type, converted) = match value.ret.as_ref()? {
            // The value the machine holds is a statement's, as a record's is.
            StaticSlot::Scalar(SceType::Enum(reference)) => (
                self.enum_type(spec.machine, &reference.alias)?,
                format!(
                    "\tout := {}\n",
                    spec.enum_crossing(self, &reference.alias, "r", Crossing::FromAlgorithm)?
                ),
            ),
            StaticSlot::Scalar(ty) if plain_scalar(ty) => (self.scalar_type(ty), String::new()),
            // The machine's `optional.Of` over its own enum, converted where the
            // algorithm's answer is present (docs/adr/0055).
            StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => (
                spec.optional_type(self, ty)?,
                format!(
                    "\tout := {}\n",
                    spec.optional_enum_crossing(self, ty, "r", Crossing::FromAlgorithm)?
                ),
            ),
            StaticSlot::Optional(ty) => (spec.optional_type(self, ty)?, String::new()),
            // The algorithm's optional record is read for its present record
            // without naming its type, so the schema's package is no import the
            // adapter needs for a return alone ([`Self::adapter_names_returned_schema`]).
            StaticSlot::OptionalRecord(alias) => {
                let held = self.optional_type(&machine_record(alias))?;
                (
                    held.clone(),
                    format!(
                        "\tvar out {held}\n\tif v, ok := r.Get(); ok {{\n\t\tout = optional.Some({})\n\t}}\n",
                        from_algorithm(alias, "v")?
                    ),
                )
            }
            StaticSlot::Record(alias) => (
                machine_record(alias),
                format!("\tout := {}\n", from_algorithm(alias, "r")?),
            ),
            StaticSlot::ScalarList(ty) if plain_scalar(ty) => (
                self.list_type(ty),
                "\tout := append(r[:0:0], r...)\n".to_string(),
            ),
            StaticSlot::RecordList(alias) => {
                let list = self.record_list_type(&machine_record(alias));
                (
                    list.clone(),
                    format!(
                        "\tout := make({list}, len(r))\n\tfor i, e := range r {{\n\t\tout[i] = {}\n\t}}\n",
                        from_algorithm(alias, "e")?
                    ),
                )
            }
            _ => return None,
        };
        let call = format!("{}({})", spec.call, arguments.join(", "));
        let scalar = converted.is_empty();
        Some(match (spec.callee.may_fail, scalar) {
            (true, true) => format!(
                "func {}({}) scealgorithm.Outcome[{returned_type}] {{\n{prelude}\tr, err := {call}\n\treturn scealgorithm.Outcome[{returned_type}]{{Value: r, Err: err}}\n}}",
                spec.name,
                declarations.join(", ")
            ),
            (true, false) => format!(
                "func {}({}) scealgorithm.Outcome[{returned_type}] {{\n{prelude}\tr, err := {call}\n\tif err != nil {{\n\t\treturn scealgorithm.Outcome[{returned_type}]{{Err: err}}\n\t}}\n{converted}\treturn scealgorithm.Outcome[{returned_type}]{{Value: out}}\n}}",
                spec.name,
                declarations.join(", ")
            ),
            (false, true) => format!(
                "func {}({}) {returned_type} {{\n{prelude}\treturn {call}\n}}",
                spec.name,
                declarations.join(", ")
            ),
            (false, false) => format!(
                "func {}({}) {returned_type} {{\n{prelude}\tr := {call}\n{converted}\treturn out\n}}",
                spec.name,
                declarations.join(", ")
            ),
        })
    }
    fn unsupported(&self, model: &SCXMLModel, _scope: &StaticScope) -> Option<String> {
        // A record's field is named as the author wrote it, so one Go reserves
        // cannot be a field.
        if let Some((alias, field)) = model.imported_records.iter().find_map(|(alias, schema)| {
            schema
                .fields
                .iter()
                .find(|f| crate::reader_names::is_reserved_word(Language::Go, &f.id))
                .map(|f| (alias, f))
        }) {
            return Some(format!(
                "record:{alias} with the field `{}`, a name Go reserves",
                field.id
            ));
        }
        // The reader a field gets is its id in Pascal case, and the method a
        // record writes its saved form with is `ToSaved`: one field cannot have
        // both.
        if let Some((alias, field)) = model.imported_records.iter().find_map(|(alias, schema)| {
            schema
                .fields
                .iter()
                .find(|f| filters::to_pascal_case(f.id.clone()) == "ToSaved")
                .map(|f| (alias, f))
        }) {
            return Some(format!(
                "record:{alias} with the field `{}`, whose reader is the name Go writes the saved form of a record with",
                field.id
            ));
        }
        for state in model.states.values() {
            let blocks = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for block in blocks {
                if let Some(found) = Self::unlowered_action(block) {
                    return Some(found);
                }
            }
            if let Some(other) = unlowered_invoke(
                &state.invokes,
                true,
                self.lowers_hybrid_invoke(),
                self.lowers_mesh_invoke(),
            ) {
                return Some(other);
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Go
    }
    // A prefix keeps a variable's field from meeting another member of the
    // policy, which a bare name could (`state`, `engine`).
    fn field_name(&self, id: &str) -> String {
        format!("v{}", filters::to_pascal_case(id.to_string()))
    }
    // The author's name with its first letter raised, as an exported method
    // must be, and none for a name the language or the policy already holds.
    fn reader_name(&self, id: &str, machine: &str) -> Option<String> {
        let spelled = filters::to_pascal_case(id.to_string());
        (!crate::reader_names::reader_is_taken(Language::Go, &spelled, machine)).then_some(spelled)
    }
    fn field_ref(&self, name: &str) -> String {
        format!("p.{name}")
    }
    fn in_function(&self) -> &'static str {
        "p.IsStateActive"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::go_type(ty).to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    // A struct of the schema's fields in the schema's order, spelled as the
    // typed payload's are, because an expression reads a field by the name the
    // author wrote. The fields are unexported, so each has an exported reader
    // of its own — a host reads a record through those.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let go_present =
            |field: &crate::forge::model::SchemaField| match field.sce_type.present_type() {
                SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                other => crate::forge::generator::go_type(other).to_string(),
            };
        // An optional field is held in the optional type over its present
        // value's (docs/adr/0055).
        let go_field_type = |field: &crate::forge::model::SchemaField| {
            let present = go_present(field);
            match field.sce_type {
                FieldType::Optional { .. } => self
                    .optional_type(&present)
                    .expect("Go spells an optional value"),
                FieldType::Scalar(_) => present,
            }
        };
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                format!(
                    "\t{} {}\n",
                    self.record_field(&field.id),
                    go_field_type(field)
                )
            })
            .collect();
        let readers: String = schema
            .fields
            .iter()
            .map(|field| {
                // A byte string is answered as a copy, so a host cannot write
                // through the slice into what the machine holds
                // (docs/adr/0005, decision 2).
                let answer = match &field.sce_type {
                    FieldType::Scalar(SceType::Bytes) => {
                        format!("append([]byte{{}}, r.{}...)", self.record_field(&field.id))
                    }
                    // ... and so is the present value of an optional one.
                    FieldType::Optional {
                        elem: SceType::Bytes,
                    } => self
                        .optional_map(
                            &format!("r.{}", self.record_field(&field.id)),
                            "[]byte",
                            &|v| format!("append([]byte{{}}, {v}...)"),
                        )
                        .expect("Go maps an optional value"),
                    _ => format!("r.{}", self.record_field(&field.id)),
                };
                format!(
                    "\n// {reader} reports the `{id}` field.\nfunc (r {ty}) {reader}() {} {{\n\treturn {answer}\n}}\n",
                    go_field_type(field),
                    reader = filters::to_pascal_case(field.id.clone()),
                    id = field.id,
                )
            })
            .collect();
        // Its saved form (`sce.SavedState`) lives on the type: an object of the
        // schema's fields keyed by their ids, written by `ToSaved` and read by
        // `{ty}FromSaved`, the names the enum's own saved form has
        // ([`Self::enum_def`]). An optional field is written as null or as its
        // present value, and read back absent from null and otherwise as its
        // present value is (docs/adr/0055, section 4).
        let saved_name =
            |sce_type: &SceType| filters::to_pascal_case(sce_type.as_attr().to_string());
        let writes: String = schema
            .fields
            .iter()
            .map(|field| {
                let writer = match field.sce_type.present_type() {
                    SceType::Enum(reference) => format!(
                        "func(v {}) any {{ return v.ToSaved() }}",
                        enum_types[&reference.alias]
                    ),
                    other => format!("sce.SavedWrite{}", saved_name(other)),
                };
                let member = self.record_field(&field.id);
                let saved = match &field.sce_type {
                    FieldType::Optional { .. } => format!("optional.Wire(r.{member}, {writer})"),
                    FieldType::Scalar(_) => format!("({writer})(r.{member})"),
                };
                format!(
                    "\tsaved.Set(\"{}\", {saved})\n",
                    filters::escape_go(field.id.clone())
                )
            })
            .collect();
        let reads: String = schema
            .fields
            .iter()
            .map(|field| {
                let id = filters::escape_go(field.id.clone());
                let reader = match (field.sce_type.present_type(), field.max_size) {
                    (SceType::Enum(reference), _) => {
                        format!("{}FromSaved", enum_types[&reference.alias])
                    }
                    // A string is read back only if it fits the bound the schema
                    // declares, as a string variable's is, and so is a byte
                    // string, in bytes.
                    (SceType::String, Some(bound)) => format!("sce.SavedBoundedString({bound})"),
                    (SceType::Bytes, Some(bound)) => format!("sce.SavedBoundedBytes({bound})"),
                    (SceType::Bytes, None) => format!("sce.SavedBoundedBytes({})", i32::MAX),
                    (other, _) => format!("sce.SavedRead{}", saved_name(other)),
                };
                let what = format!("what + \".{id}\"");
                let read = match &field.sce_type {
                    FieldType::Optional { .. } => {
                        let present = go_present(field);
                        format!(
                            "sce.SavedReadOptional(saved, {what}, {reader}, optional.None[{present}], optional.Some[{present}])"
                        )
                    }
                    FieldType::Scalar(_) => format!("({reader})(saved, {what})"),
                };
                format!(
                    "\t{{\n\t\tsaved, err := sce.SavedField(value, what, \"{id}\")\n\t\tif err != nil {{\n\t\t\treturn record, err\n\t\t}}\n\t\tif record.{member}, err = {read}; err != nil {{\n\t\t\treturn record, err\n\t\t}}\n\t}}\n",
                    member = self.record_field(&field.id),
                )
            })
            .collect();
        format!(
            "// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\n\
             type {ty} struct {{\n{fields}}}\n{readers}\n\
             // ToSaved is this value as a saved state writes it.\n\
             func (r {ty}) ToSaved() any {{\n\tsaved := sce.NewJSONObject()\n{writes}\treturn saved\n}}\n\n\
             // {ty}FromSaved is the value a saved state holds, refused unless it is one.\n\
             func {ty}FromSaved(value any, what string) ({ty}, error) {{\n\tvar record {ty}\n{reads}\treturn record, nil\n}}"
        )
    }
    // The author's spelling, which is what an expression's member access writes
    // (`shown.year` is `p.vShown.year`); `unsupported` has refused a name Go
    // reserves.
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!("{f}: {v}")).collect();
        format!("{ty}{{{}}}", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    // A constant is a package-level identifier in Go, so it carries the name of
    // the machine's own type `ty`: a package holds a parent and every child it
    // invokes, and two of them may hold one enum, each in a type of its own. A
    // reference spells it the same way, through the same type
    // ([`crate::forge::enum_naming::variant_ref`]).
    fn enum_variant(&self, ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ref(Language::Go, ty, enum_name, variant)
    }
    // A named integer over the enum document's own carrier, each variant
    // holding the value the document declares for it. `String` answers the name
    // the document gives a value — what a `<log>` shows and what a host
    // compares to a scenario.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let constants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "\t{} {ty} = {}\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    v.value
                )
            })
            .collect();
        let names: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "\tcase {}:\n\t\treturn \"{}\"\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    filters::escape_go(v.name.clone())
                )
            })
            .collect();
        // The enum document's own accessors, by the same names and shapes
        // (`forge/go/enum.go.jinja2`): what `<alias>.toNumber` and
        // `<alias>.fromNumber` are lowered to ([`StaticTarget::enum_number_symbols`]).
        let carrier = crate::forge::generator::go_type(&model.underlying_type);
        let declared: Vec<String> = model.variants.iter().map(|v| v.value.to_string()).collect();
        // Its saved form lives on the type, as a record's does: the declared name
        // the enum document gives the variant.
        let saved_variants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "\tcase \"{}\":\n\t\treturn {}, nil\n",
                    filters::escape_go(v.name.clone()),
                    self.enum_variant(ty, &model.name, &v.name)
                )
            })
            .collect();
        format!(
            "// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n\
             type {ty} {carrier}\n\n\
             const (\n{constants})\n\n\
             // String is the name the enum document declares for the value.\n\
             func (v {ty}) String() string {{\n\tswitch v {{\n{names}\t}}\n\treturn \"\"\n}}\n\n\
             // ToUnderlying is the number the enum document declares for the value.\n\
             func (v {ty}) ToUnderlying() {carrier} {{\n\treturn {carrier}(v)\n}}\n\n\
             // {ty}FromUnderlying returns the variant that declares raw, and the absent\n\
             // value for a number no variant declares.\n\
             func {ty}FromUnderlying(raw {carrier}) optional.Of[{ty}] {{\n\tswitch raw {{\n\tcase {}:\n\t\treturn optional.Some({ty}(raw))\n\t}}\n\treturn optional.None[{ty}]()\n}}\n\n\
             // ToSaved is this value as a saved state writes it.\n\
             func (v {ty}) ToSaved() any {{\n\treturn v.String()\n}}\n\n\
             // {ty}FromSaved is the value a saved state holds, refused unless it is a variant.\n\
             func {ty}FromSaved(value any, what string) ({ty}, error) {{\n\tdeclared, err := sce.SavedReadString(value, what)\n\tif err != nil {{\n\t\treturn 0, err\n\t}}\n\tswitch declared {{\n{saved_variants}\t}}\n\treturn 0, sce.Refuse(fmt.Sprintf(\"'%s' (%s) is not a variant of {alias}\", what, declared))\n}}",
            declared.join(", ")
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (format!("{ty}.ToUnderlying"), format!("{ty}FromUnderlying"))
    }
    // `<Enum>FromUnderlying` answers an `optional.Of`.
    fn enum_def_names_optional(&self) -> bool {
        true
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("[]{}", crate::forge::generator::go_type(elem))
    }
    // The reader answers a copy of the slice (the template's own), so a host
    // cannot write through it into what the machine holds.
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("[]{record}")
    }
    fn record_list_view(&self, _record: &str) -> Option<String> {
        None
    }
    // A list starts empty, and an empty slice is `nil`.
    fn list_empty(&self) -> String {
        "nil".to_string()
    }
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("scealgorithm.Bounded(&sceFailure, {value}, {capacity})")
    }
    // The slice a machine holds is its own copy, so a host that kept the one it
    // raised an event with changes nothing of what the machine read. A `[]byte` the
    // machine never writes into, handed to a host as a copy; a record's field is the
    // same — the machine replaces the slice and never writes into it, so a copy of
    // the record shares it safely, and the reader the record gives the field answers
    // a copy; a payload's is read where the machine's own is, and held to the bound
    // of the variable or the record's field it is written to.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        format!("scealgorithm.BoundedBytes(&sceFailure, {value}, {capacity})")
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
    }
    // An algorithm's list is the slice the machine's own is.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("scealgorithm.Within(&sceFailure, {value}, {capacity})")
    }
    // The count is the engine's the action is handed.
    fn fresh_send_id(&self) -> String {
        "engine.NextAutoSendID()".to_string()
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value}")
    }
    // The label is an argument of `Printf`, never part of its format, as the
    // script-engine arm's is.
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
        format!(
            "fmt.Printf(\"%s%v\\n\", \"{}\", {value})",
            filters::escape_go(label.to_string())
        )
    }
    // An expression that is `true` when the append failed: the list is full, or
    // the value could not be computed. The room is checked first, so a full
    // list computes nothing, and the value is computed into a local before it
    // is appended, so a failed one leaves the list as it was.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        _value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        format!(
            "func() bool {{ var sceFailure scealgorithm.Failure; \
             if len({target}) >= {capacity} {{ {} }}; \
             sceValue := scealgorithm.ElementOf({target}, {value}); \
             if sceFailure.Failed() {{ {} }}; \
             {target} = append({target}, sceValue); return false }}()",
            Self::then(&[overflow, "return true"]),
            Self::then(&[failed, "return true"]),
        )
    }
    fn clear(&self, target: &str) -> String {
        format!("{target} = {target}[:0]")
    }
    // The loop walks the list as it was when it began (§scxml-4.6: a shallow
    // copy), so a body that appends to or empties the list does not move it —
    // `append(list[:0:0], list...)` is the copy, whatever the element type. The
    // position is a `uint32`, as `len` is, and a name the body never reads is
    // still used.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        let copy = format!("append({list}[:0:0], {list}...)");
        Some(match index {
            None => (
                format!("for _, {item} := range {copy}"),
                format!("_ = {item}"),
            ),
            Some(index) => (
                format!("for sceIndex, {item} := range {copy}"),
                format!("{index} := uint32(sceIndex); _ = {index}; _ = {item}"),
            ),
        })
    }
    fn raise_execution_error(&self, machine: &str, message: &str) -> String {
        format!(
            "engine.Raise(sce.NewPlatformError({machine}Event{}, \"{}\"))",
            filters::to_event_variant("error.execution".to_string()),
            filters::escape_go(message.to_string())
        )
    }
    // The runtime's checked helpers record a failure in `sceFailure` and answer
    // a zero, so a statement that wrote its value would write that zero. The
    // expression is a function literal called where it stands, answering whether
    // it failed, so the dispatcher can end its block as it does for any error.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "func() bool {{ var sceFailure scealgorithm.Failure; {statement}; \
             if sceFailure.Failed() {{ {} }}; return false }}()",
            Self::then(&[failed, "return true"])
        )
    }
    fn receiving_write(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        _ty: InferredType,
        failed: &str,
    ) -> String {
        format!(
            "func() bool {{ var sceFailure scealgorithm.Failure; sceValue := {value}; \
             if sceFailure.Failed() {{ {} }}; {}; return false }}()",
            Self::then(&[failed, "return true"]),
            write("sceValue")
        )
    }
    // Only a host call's arguments reach here, and its one spelling is
    // [`Self::receiving_host_call`].
    fn receiving_call(&self, _statement: &str, _failed: &str) -> String {
        unreachable!("a Go host call is received through receiving_host_call")
    }
    // A failed checked operation is a value, so the call could not be stopped
    // after its arguments were evaluated: each is computed into a local first,
    // and the host is called only when none of them failed. A block of its own,
    // so two calls in one scope do not declare the same locals.
    fn receiving_host_call(
        &self,
        _statement: &str,
        callee: &str,
        args: &[String],
        _arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        let locals: String = args
            .iter()
            .enumerate()
            .map(|(i, arg)| format!("sceArg{i} := {arg}; "))
            .collect();
        let names: Vec<String> = (0..args.len()).map(|i| format!("sceArg{i}")).collect();
        format!(
            "{{ var sceFailure scealgorithm.Failure; {locals}\
             if sceFailure.Failed() {{ {} }} else {{ {callee}({}) }} }}",
            Self::then(&[failed]),
            names.join(", ")
        )
    }
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        format!(
            "func() bool {{ var sceFailure scealgorithm.Failure; sceValue := {value}; \
             if sceFailure.Failed() {{ {} }}; return sceValue }}()",
            Self::then(&[failed, flag, "return false"])
        )
    }
    // The flag is a local of the `<if>`, named by its ordinal, which the
    // condition's function literal sets by reference.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String {
        format!("ifCondFailed{if_ordinal} = true")
    }
    // An enum field of the payload is held in the machine's own enum, lifted
    // from the variant's declared name and written back as the enum's `String`
    // answers it (`build_go_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`sce.ParseDelayToMs`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes
    // (`engine.SendNamedExternal`).
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes
    // (`engine.CancelEvent`).
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template carries the value as the event's JSON and as the text a
    // host takes (`sce.ScriptValueToJSON`, `sce.ToWireString`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`hostInvokeSrc`).
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`sce.DocumentStem`) and starts the candidate it names, handing it the
    // values it keeps (`seed_static_child`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // The field the payload channel fills when the engine dequeues an event of
    // this name (`build_go_event_payload`), read by the typed guards and by a
    // `<sce:action>`'s arguments alike.
    fn payload_accessor(&self, event: &str) -> String {
        format!(
            "p.pending{}Payload",
            filters::to_event_variant(event.to_string())
        )
    }
    // The tag says which event's payload the channel holds now, so a guard
    // that reads this event's fields holds only for a delivery that carried
    // them.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "p.pendingPayloadTag == {machine}PayloadTag{} && ({lowered})",
            filters::to_event_variant(event.to_string())
        )
    }
    // What the runtime's `ScriptValueToJSON` writes as the pair's value: a bool
    // and a string as they are, a narrow integer widened to `int64` and a real
    // to `float64`, both exactly; a 64-bit integer is the text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool | InferredType::Str => value.to_string(),
            InferredType::Int {
                signed: true,
                bits: 64,
            } => format!("sce.Int64PayloadText(int64({value}))"),
            InferredType::Int { bits: 64, .. } => {
                format!("sce.Uint64PayloadText(uint64({value}))")
            }
            InferredType::Int { .. } => format!("int64({value})"),
            _ => format!("float64({value})"),
        }
    }
    // The enum's `String` answers the name the enum document declares.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("({value}).String()"))
    }
    // The text the runtime's own writer spells a byte slice as, a Go `string`.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(self.wire_value(
            InferredType::Str,
            &format!("sce.BytesAsPayloadText({value})"),
        ))
    }
}

/// Rewrite `model` — a clone the Go backend renders — so every expression of a
/// `sce-static` document is native Go.
///
/// `go_module_prefix` is the module path the generated packages live under
/// (`--go-module-prefix`). A machine that calls an imported algorithm, or a
/// document of another kind it calls (docs/adr/0047), imports its package by
/// that path, and a Go import path has no valid bare form, so a
/// document that calls one without a usable prefix is refused as any forge kind
/// importing another is.
pub fn lower_go(
    model: &mut SCXMLModel,
    machine: &str,
    go_module_prefix: Option<&str>,
) -> Result<StaticLowering, GenerateError> {
    let calls_an_import = StaticScope::of(model).is_some_and(|scope| !scope.callees.is_empty());
    let import_root = if calls_an_import {
        Some(crate::forge::generator::go_import_root(go_module_prefix)?)
    } else {
        None
    };
    lower(model, machine, &GoTarget { import_root })
}

/// Python: a variable is an attribute of the machine's policy, set by the
/// machine's own statements. Python's failure channel is an exception — a
/// checked operation raises `AlgorithmFailure` in place of a value — so a
/// statement that can fail is wrapped where it stands: the exception stops it
/// before it writes anything, `error.execution` is raised in its place, and
/// `_ActionAbort` ends the block (§scxml-4.9), the sentinel every action block
/// of the generated module already catches. A statement of such a machine is
/// therefore a block of lines, not an expression: the templates indent each.
///
/// Lowers scalar, enum, list and record variables, an optional value of a scalar
/// or of a record held as `Optional[T]` (docs/adr/0055), guards, `<assign>`, `<if>`,
/// `<foreach>`, `<log>`, `<raise>`, `In()`, a typed payload and a call of an
/// imported algorithm — a module beside the machine's own, whose failing call
/// raises the same `AlgorithmFailure` a checked operation does; every construct
/// past those is refused by name ([`StaticTarget::unsupported`]) until its
/// spelling is written, rather than left as an undefined name in generated
/// code.
pub struct PythonTarget;

impl PythonTarget {
    /// `statement`, wrapped where a failing checked operation of it is
    /// received: the lines of a block that leaves its action block on a failure.
    fn guarded(statement: &str, failed: &str) -> String {
        let mut lines = vec![
            "try:".to_string(),
            format!("    {statement}"),
            "except sce_algorithm.AlgorithmFailure:".to_string(),
        ];
        if !failed.is_empty() {
            lines.push(format!("    {failed}"));
        }
        lines.push("    raise _ActionAbort".to_string());
        lines.join("\n")
    }

    /// The first action of `actions`, or of a block nested in one, that this
    /// target has no lowering for yet.
    fn unlowered_action(actions: &[Action]) -> Option<String> {
        for action in actions {
            match action.action_type.as_str() {
                "assign" | "log" | "if" | "raise" | "cancel" => {}
                // A list is filled, emptied and walked by native statements.
                "sce_append" | "sce_clear" | "foreach" => {}
                // A param is the typed value of the data model crossed to the
                // event's data, and a literal `<content>` is the text it spells.
                "send" => {}
                // A host action whose arguments are typed expressions of the
                // machine's variables, called on the machine's `Protocol`.
                "native_action" => {}
                other => return Some(format!("<{other}>")),
            }
            for block in action.nested_blocks() {
                if let Some(found) = Self::unlowered_action(block.actions) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// `value`, an optional value, as `absent` where it is `None` and as
    /// `present` — which reads the present value as [`PYTHON_PRESENT`] — where it
    /// is not: a `lambda` called at once binds the value, so it is computed once.
    /// Presence is `is None`, never a truth test: a present `0`, `''` or `False`
    /// is a value (docs/adr/0055).
    fn present_or(value: &str, absent: &str, present: &str) -> String {
        format!(
            "(lambda {PYTHON_PRESENT}: {absent} if {PYTHON_PRESENT} is None else {present})({value})"
        )
    }
}

/// The name a Python `lambda` binds the present value of an optional value to
/// ([`StaticTarget::optional_map`], [`StaticTarget::wire_optional`], the payload's
/// inject seam): under the generator's prefix, which no author's name is spelled
/// as (`crate::forge::generator::python_local_spelling`), and a variable is an
/// attribute of the policy.
pub(crate) const PYTHON_PRESENT: &str = "sce_present";

impl StaticTarget for PythonTarget {
    // The start site holds the id it generates in the `str` local
    // `_runtime_invoke_id` (`defer_invokes_on_entry`), which it goes on to start
    // the child under, so the variable takes it as it stands.
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("_runtime_invoke_id")
    }
    fn name(&self) -> &'static str {
        "Python"
    }
    // A statement that fails raises, which leaves the block: the lines follow one another.
    fn in_sequence(&self, parts: &[String]) -> String {
        parts.join("\n")
    }
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Python, callee, None))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // `Optional[T]` (docs/adr/0055, section 5) is the default spelling of the type
    // ([`crate::forge::generator::optional_of`]), which the module's `typing`
    // import already names: an absent value is `None`, and a present one is the
    // value itself.
    fn optional_none(&self, _present: &str) -> Option<String> {
        Some("None".to_string())
    }
    fn optional_some(&self, _present: &str, value: &str) -> Option<String> {
        Some(value.to_string())
    }
    // A field of the present value of an optional record (docs/adr/0055, step 9):
    // the record the variable holds is a frozen dataclass, so the field is written
    // by replacing it with one whose field is changed, as a record variable's field
    // is ([`Self::assign_field`]). The statement stands where a test proved the
    // record present, so what it replaces is the record and never `None`. A text
    // or a byte string is handed in held to the bound its field declares, which
    // is how the walk holds every write ([`write_field`]), so the string and bytes
    // forms are this one.
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(self.assign_field(target, field, value))
    }
    // The element of a list of records at an index, whole: the runtime's checked
    // read, which raises `AlgorithmFailure` for an index outside the list as every
    // checked operation raises one, and the statement catches it where it stands.
    // The records a list holds are frozen, so the element is the value and a copy
    // of it would be the element again ([`StaticTarget::record_copy`]).
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!("sce_algorithm.at({list}, {index})"))
    }
    // A present text held to its bound, which raises `AlgorithmFailure` in the
    // statement that writes it; an absent one stays `None`.
    fn optional_map(
        &self,
        value: &str,
        _present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(Self::present_or(value, "None", &map(PYTHON_PRESENT)))
    }
    // JSON `null` (`ScriptValue.null()`, which `to_json_literal` writes as
    // `null`) where the value is absent, and the present value's own wire value
    // where it is present (docs/adr/0055, section 4).
    fn wire_optional(
        &self,
        value: &str,
        _present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(Self::present_or(
            value,
            "_ScriptValue.null()",
            &wire(PYTHON_PRESENT),
        ))
    }
    // The literal `null` an optional record's slot takes holds no variable: it is
    // the absent value (docs/adr/0055, step 9).
    fn state_argument(&self, _machine: &str, arg: &StateArg, held: &str) -> String {
        if arg.code == 'n' {
            "None".to_string()
        } else {
            held.to_string()
        }
    }
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("_call_{coded}")
    }
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("sce_algorithm.bounded(list({value}), {capacity})")
    }
    fn schema_import(&self, schema_name: &str) -> Option<String> {
        Some(
            crate::forge::generator::forge_import_identity(
                schema_name,
                &Language::Python,
                false,
                &crate::ForgeCompileOptions::default(),
            )
            .include_stmt,
        )
    }
    // A dict of the variants read at the value: both types are `IntEnum`s, and the
    // member of the same declared name is the one the value is mapped to.
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let entries: Vec<String> =
            enum_variant_pairs(Language::Python, machine_type, model, crossing)
                .into_iter()
                .map(|(from, to)| format!("{from}: {to}"))
                .collect();
        Some(format!("{{{}}}[{value}]", entries.join(", ")))
    }
    fn enum_import(&self, enum_name: &str) -> Option<String> {
        self.schema_import(enum_name)
    }
    // A class of the module whose attributes are the bounds, so a host names one as
    // `<Machine>Limits.<NAME>`.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let constants: String = limits
            .iter()
            .map(|(name, value)| {
                format!(
                    "    # {}\n    {}: int = {value}\n",
                    limit_doc(name),
                    name.to_uppercase()
                )
            })
            .collect();
        Some(format!(
            "class {machine}Limits:\n    \"\"\"{LIMITS_DOC}\"\"\"\n\n{constants}"
        ))
    }
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two functions of the module, each the dict over every variant a call of an
    // algorithm crosses by.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Python);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "def {to}(v):\n    \"\"\"A value of {ty} as the variant of the same declared name of \
             {document}.\"\"\"\n    return {into}\n\n\n\
             def {from}(v):\n    \"\"\"A value of {document} as the variant of the same declared \
             name of {ty}.\"\"\"\n    return {back}"
        ))
    }
    // A function of the machine's file that builds the algorithm's records from the
    // machine's, calls it, and builds the machine's from what it returns. The
    // algorithm raises where it fails, as the machine's own checked operations do,
    // and a record or a list it is handed is never one the machine keeps: each is
    // copied on the way in and on the way out, so what it does to its arguments
    // cannot reach the machine's state.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{event_schema_field_ident, event_schema_payload_type};
        let value = spec.callee.value.as_ref()?;
        // The keyword arguments of a record built from `from`, field by field, an
        // enum field as the variant of the same declared name the other side holds.
        let fields_of = |alias: &str, from: &str, crossing: Crossing| -> Option<String> {
            let schema = plain_schema(spec, alias)?;
            let fields = schema
                .fields
                .iter()
                .map(|f| {
                    let field = event_schema_field_ident(&f.id, Language::Python);
                    let member = format!("{from}.{field}");
                    let crossed = match &f.sce_type {
                        // An optional value of an enum crosses by the declared name
                        // of its variant where it is present, and is absent on the
                        // other side where it is absent here (docs/adr/0055,
                        // section 2): the variant is mapped, `None` is not.
                        FieldType::Optional {
                            elem: SceType::Enum(reference),
                        } => spec
                            .enum_crossing(self, &reference.alias, PYTHON_PRESENT, crossing)
                            .map(|present| Self::present_or(&member, "None", &present)),
                        _ => spec.record_field_crossing(self, f, &member, crossing, str::to_string),
                    };
                    crossed.map(|value| format!("{field}={value}"))
                })
                .collect::<Option<Vec<String>>>()?;
            Some(fields.join(", "))
        };
        let to_algorithm = |alias: &str, from: &str| -> Option<String> {
            let schema = plain_schema(spec, alias)?;
            Some(format!(
                "{}({})",
                event_schema_payload_type(&schema.name, Language::Python),
                fields_of(alias, from, Crossing::ToAlgorithm)?
            ))
        };
        let from_algorithm = |alias: &str, from: &str| -> Option<String> {
            Some(format!(
                "{}({})",
                self.record_type(spec.machine, alias),
                fields_of(alias, from, Crossing::FromAlgorithm)?
            ))
        };
        let mut declarations = Vec::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => arguments.push(
                    spec.enum_crossing(self, &reference.alias, &name, Crossing::ToAlgorithm)?,
                ),
                AdapterParam::Scalar(ty) => {
                    if !plain_scalar(ty) {
                        return None;
                    }
                    arguments.push(name.clone());
                }
                // A value of an enum that may be absent is `None` or the machine's
                // enum, and crosses where it is present by the declared name of its
                // variant (docs/adr/0055).
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => arguments
                    .push(spec.optional_enum_crossing(self, ty, &name, Crossing::ToAlgorithm)?),
                // `None` or the value itself on both sides (docs/adr/0055).
                AdapterParam::Optional(_) => arguments.push(name.clone()),
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    ..
                } => arguments.push(to_algorithm(alias, &name)?),
                // An optional record is `None`, or a record built as a record
                // argument is (docs/adr/0055, step 9): the argument is what the
                // call held, the machine's optional variable, a record variable, a
                // payload, a loop's item or the literal `null`, and a record is
                // never `None`, so one test says which it is whichever it was.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    ..
                } => arguments.push(format!(
                    "(None if {name} is None else {})",
                    to_algorithm(alias, &name)?
                )),
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => arguments.push(format!("list({name})")),
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => arguments.push(format!("[{} for e in {name}]", to_algorithm(alias, "e")?)),
                _ => return None,
            }
            declarations.push(name);
        }
        let call = format!("{}({})", spec.call, arguments.join(", "));
        let converted = match value.ret.as_ref()? {
            StaticSlot::Scalar(SceType::Enum(reference)) => {
                spec.enum_crossing(self, &reference.alias, "r", Crossing::FromAlgorithm)?
            }
            StaticSlot::Scalar(ty) if plain_scalar(ty) => "r".to_string(),
            // `None`, or the machine's enum where the algorithm's answer is present.
            StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                spec.optional_enum_crossing(self, ty, "r", Crossing::FromAlgorithm)?
            }
            StaticSlot::Optional(_) => "r".to_string(),
            StaticSlot::Record(alias) => from_algorithm(alias, "r")?,
            // `None`, or the machine's own record built from what the algorithm
            // gave back (docs/adr/0055, step 9).
            StaticSlot::OptionalRecord(alias) => {
                format!("(None if r is None else {})", from_algorithm(alias, "r")?)
            }
            StaticSlot::ScalarList(ty) if plain_scalar(ty) => "list(r)".to_string(),
            StaticSlot::RecordList(alias) => {
                format!("[{} for e in r]", from_algorithm(alias, "e")?)
            }
            _ => return None,
        };
        Some(format!(
            "def {}({}):\n    r = {call}\n    return {converted}",
            spec.name,
            declarations.join(", ")
        ))
    }
    fn unsupported(&self, model: &SCXMLModel, _scope: &StaticScope) -> Option<String> {
        // A record's field is named as the author wrote it, so one Python
        // reserves cannot be a field.
        if let Some((alias, field)) = model.imported_records.iter().find_map(|(alias, schema)| {
            schema
                .fields
                .iter()
                .find(|f| crate::reader_names::is_reserved_word(Language::Python, &f.id))
                .map(|f| (alias, f))
        }) {
            return Some(format!(
                "record:{alias} with the field `{}`, a name Python reserves",
                field.id
            ));
        }
        // The methods a record writes and reads its saved form with are named on the
        // type, beside its fields: one field cannot have either name.
        if let Some((alias, field)) = model.imported_records.iter().find_map(|(alias, schema)| {
            schema
                .fields
                .iter()
                .find(|f| f.id == "to_saved" || f.id == "from_saved")
                .map(|f| (alias, f))
        }) {
            return Some(format!(
                "record:{alias} with the field `{}`, a name Python writes the saved form of a record with",
                field.id
            ));
        }
        for state in model.states.values() {
            let blocks = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for block in blocks {
                if let Some(found) = Self::unlowered_action(block) {
                    return Some(found);
                }
            }
            if let Some(other) = unlowered_invoke(
                &state.invokes,
                true,
                self.lowers_hybrid_invoke(),
                self.lowers_mesh_invoke(),
            ) {
                return Some(other);
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Python
    }
    // A prefix keeps a variable's attribute from meeting another member of the
    // policy, which a bare name could (`state`, `engine`).
    fn field_name(&self, id: &str) -> String {
        format!("v_{}", filters::to_snake_case(id.to_string()))
    }
    // The author's spelling, as every typed reader of the language has it
    // (`crate::reader_names`), and none for a name the language reserves.
    fn reader_name(&self, id: &str, machine: &str) -> Option<String> {
        let spelled = id.replace(['.', '-'], "_");
        (!crate::reader_names::reader_is_taken(Language::Python, &spelled, machine))
            .then_some(spelled)
    }
    fn field_ref(&self, name: &str) -> String {
        format!("self.{name}")
    }
    fn in_function(&self) -> &'static str {
        "self._sce_in"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::python_type(ty).to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    // A frozen dataclass of the schema's fields in the schema's order, spelled
    // as the typed payload's are, because an expression reads a field by the
    // name the author wrote. Frozen, so a copy a host was handed cannot be
    // written through into what the machine holds: a field changes by replacing
    // the record ([`Self::assign_field`]).
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                let present = match field.sce_type.present_type() {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::python_type(other).to_string(),
                };
                // An optional field is held in the optional type over its
                // present value's (docs/adr/0055).
                let field_ty = match field.sce_type {
                    FieldType::Optional { .. } => self
                        .optional_type(&present)
                        .expect("Python spells an optional value over its present one"),
                    FieldType::Scalar(_) => present,
                };
                format!("    {}: {field_ty}\n", self.record_field(&field.id))
            })
            .collect();
        // Its saved form (`sce_runtime.SavedState`) lives on the type: an object of
        // the schema's fields keyed by their ids, written by `to_saved` and read by
        // `from_saved`, the names the enum's own saved form has
        // ([`Self::enum_def`]). An optional field is written as null or as its
        // present value, and read back absent from null and otherwise as its
        // present value is (docs/adr/0055, section 4).
        let writes: String = schema
            .fields
            .iter()
            .map(|field| {
                let member = self.record_field(&field.id);
                let writer = match field.sce_type.present_type() {
                    SceType::Enum(_) => "lambda v: v.to_saved()".to_string(),
                    other => format!("sce_saved.saved_write_{}", other.as_attr()),
                };
                let saved = match (&field.sce_type, field.sce_type.present_type()) {
                    (FieldType::Optional { .. }, _) => {
                        format!("sce_saved.saved_write_optional(self.{member}, {writer})")
                    }
                    (FieldType::Scalar(_), SceType::Enum(_)) => format!("self.{member}.to_saved()"),
                    (FieldType::Scalar(_), other) => {
                        format!("sce_saved.saved_write_{}(self.{member})", other.as_attr())
                    }
                };
                format!(
                    "            {}: {saved},\n",
                    filters::py_string_literal(field.id.clone())
                )
            })
            .collect();
        let reads: String = schema
            .fields
            .iter()
            .map(|field| {
                let id = filters::py_string_literal(field.id.clone());
                let reader = match (field.sce_type.present_type(), field.max_size) {
                    (SceType::Enum(reference), _) => {
                        format!("{}.from_saved", enum_types[&reference.alias])
                    }
                    // A string is read back only if it fits the bound the schema
                    // declares, as a string variable's is, and so is a byte string,
                    // in bytes.
                    (SceType::String, Some(bound)) => {
                        format!("sce_saved.saved_bounded_string({bound})")
                    }
                    (SceType::Bytes, Some(bound)) => {
                        format!("sce_saved.saved_bounded_bytes({bound})")
                    }
                    (SceType::Bytes, None) => {
                        format!("sce_saved.saved_bounded_bytes({})", i32::MAX)
                    }
                    (other, _) => format!("sce_saved.saved_read_{}", other.as_attr()),
                };
                let what = format!(
                    "what + {}",
                    filters::py_string_literal(format!(".{}", field.id))
                );
                let held = format!("sce_saved.saved_field(value, what, {id})");
                let read = match &field.sce_type {
                    FieldType::Optional { .. } => {
                        format!("sce_saved.saved_read_optional({held}, {what}, {reader})")
                    }
                    FieldType::Scalar(_) => format!("({reader})({held}, {what})"),
                };
                format!("            {}={read},\n", self.record_field(&field.id))
            })
            .collect();
        format!(
            "@sce_dataclasses.dataclass(frozen=True)\nclass {ty}:\n    \
             \"\"\"SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\"\"\"\n\n{fields}\n    \
             def to_saved(self):\n        \
             \"\"\"This value as a saved state writes it.\"\"\"\n        \
             return {{\n{writes}        }}\n\n    \
             @classmethod\n    \
             def from_saved(cls, value, what):\n        \
             \"\"\"The value a saved state holds, refused unless it is one.\"\"\"\n        \
             return cls(\n{reads}        )\n"
        )
    }
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!("{f}={v}")).collect();
        format!("{ty}({})", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    fn enum_variant(&self, _ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::Python, enum_name, variant)
    }
    // An `IntEnum` over the values the enum document declares, each member
    // holding the value the document gives it. `sce_name` answers the name the
    // document declares, which is what a `<log>` shows and what a host compares
    // to a scenario: an `IntEnum` member's own text is its number.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    {} = {}\n",
                    self.enum_variant(ty, &model.name, &v.name),
                    v.value
                )
            })
            .collect();
        let names: Vec<String> = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "{}: {}",
                    v.value,
                    filters::py_string_literal(v.name.clone())
                )
            })
            .collect();
        // `to_underlying` and `from_underlying` are the enum document's own
        // accessors, by the same names and shapes (`forge/python/enum.py.jinja2`):
        // what `<alias>.toNumber` and `<alias>.fromNumber` are lowered to
        // ([`StaticTarget::enum_number_symbols`]).
        // Its saved form lives on the type, as a record's does: the declared name the
        // enum document gives the variant.
        let saved_variants: Vec<String> = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "{}: cls.{}",
                    filters::py_string_literal(v.name.clone()),
                    self.enum_variant(ty, &model.name, &v.name)
                )
            })
            .collect();
        format!(
            "class {ty}(IntEnum):\n    \
             \"\"\"SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\"\"\"\n\n\
             {variants}\n    @property\n    def sce_name(self) -> str:\n        \
             \"\"\"The name the enum document declares for this value.\"\"\"\n        \
             return {{{}}}[int(self)]\n\n    \
             def to_underlying(self) -> int:\n        \
             \"\"\"The number the enum document declares for this value.\"\"\"\n        \
             return int(self)\n\n    \
             @classmethod\n    \
             def from_underlying(cls, raw: int) -> Optional[\"{ty}\"]:\n        \
             \"\"\"The variant that declares ``raw``, or None for a number no variant declares.\"\"\"\n        \
             try:\n            \
             return cls(raw)\n        \
             except ValueError:\n            \
             return None\n\n    \
             def to_saved(self):\n        \
             \"\"\"This value as a saved state writes it.\"\"\"\n        \
             return self.sce_name\n\n    \
             @classmethod\n    \
             def from_saved(cls, value, what):\n        \
             \"\"\"The value a saved state holds, refused unless it is a variant.\"\"\"\n        \
             return sce_saved.saved_read_variant(value, what, {}, {{{}}})\n",
            names.join(", "),
            filters::py_string_literal(alias.to_string()),
            saved_variants.join(", ")
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (
            format!("{ty}.to_underlying"),
            format!("{ty}.from_underlying"),
        )
    }
    fn list_type(&self, elem: &SceType) -> String {
        format!("List[{}]", crate::forge::generator::python_type(elem))
    }
    // The machine's own list is never lent: a reader answers a copy.
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("List[{record}]")
    }
    fn record_list_view(&self, _record: &str) -> Option<String> {
        None
    }
    fn list_empty(&self) -> String {
        "[]".to_string()
    }
    // Raises `AlgorithmFailure` past the bound, which the statement catches
    // where it stands.
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("sce_algorithm.bounded({value}, {capacity})")
    }
    // The one helper counts a `str` in UTF-8 bytes and a `bytes` as it is. A Python
    // `bytes` cannot be written into, so a host is handed the value itself and the
    // bound the machine keeps cannot be changed from outside; a record's field is the
    // same, in a frozen dataclass that compares by the bytes it holds and is
    // replaced, not written into, to change a field; a payload's is read where the
    // machine's own is, and held to the bound of the variable or the record's field
    // it is written to, the host's value being the machine's to keep.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        self.bounded_string(value, capacity)
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
    }
    // The machine's list is its own: a copy of the one the algorithm returned,
    // which a failure past the bound is raised from.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("sce_algorithm.bounded(list({value}), {capacity})")
    }
    // The count is the engine's the action is handed, which spells the id
    // `_auto_send_` and the number, as every backend does.
    fn fresh_send_id(&self) -> String {
        "engine._next_auto_sendid()".to_string()
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target} = sce_dataclasses.replace({target}, {field}=({value}))")
    }
    // Through `_sce_log_name`, which an enum member answers with the name its
    // document declares and every other value passes through unchanged, to the
    // policy's `log_hook`, which a host overrides to redirect `<log>` output.
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
        format!(
            "self.log_hook({}, _sce_log_name({value}))",
            filters::py_string_literal(label.to_string())
        )
    }
    // The lines of a block that leaves its action block when the append failed:
    // the list is full — `overflow` runs, and nothing is appended — or the value
    // could not be computed, where `failed` runs. The room is checked first, so
    // a full list computes nothing, and the value is computed before it is
    // appended, so a failed one leaves the list as it was.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        let mut lines = vec![format!("if len({target}) >= {capacity}:")];
        if !overflow.is_empty() {
            lines.push(format!("    {overflow}"));
        }
        lines.push("    raise _ActionAbort".to_string());
        if value_can_fail {
            lines.push(Self::guarded(&format!("sce_value = {value}"), failed));
            lines.push(format!("{target}.append(sce_value)"));
        } else {
            lines.push(format!("{target}.append({value})"));
        }
        lines.join("\n")
    }
    fn clear(&self, target: &str) -> String {
        format!("{target}.clear()")
    }
    // The loop walks the list as it was when it began (§scxml-4.6: a shallow
    // copy), so a body that appends to the list does not move it. The head is
    // the whole line, colon included, because Python's block opens with it.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        Some(match index {
            None => (format!("for {item} in list({list}):"), String::new()),
            Some(index) => (
                format!("for {index}, {item} in enumerate(list({list})):"),
                String::new(),
            ),
        })
    }
    // A call expression, so it is a statement where a block raises it and the
    // body of a `lambda` where a condition does.
    fn raise_execution_error(&self, _machine: &str, message: &str) -> String {
        format!(
            "self._raise_error_execution(engine, {})",
            filters::py_string_literal(message.to_string())
        )
    }
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        Self::guarded(statement, failed)
    }
    fn receiving_call(&self, statement: &str, failed: &str) -> String {
        Self::guarded(statement, failed)
    }
    // A host call whose argument failed is not made — the exception leaves the
    // statement before the call — and `error.execution` is raised in its place.
    // The block does not end, as on Kotlin, Rust, Go and C++: a host action's
    // failed argument costs the call and not the statements after it.
    fn receiving_host_call(
        &self,
        statement: &str,
        _callee: &str,
        _args: &[String],
        _arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        let on_failure = if failed.is_empty() { "pass" } else { failed };
        format!("try:\n    {statement}\nexcept sce_algorithm.AlgorithmFailure:\n    {on_failure}")
    }
    // The condition is computed where a helper can catch the exception: a
    // `lambda` is the only expression Python has that defers one. The helper
    // runs `failed` and records the failure in `flag` (the list an `<if>`
    // declares, or `None` for a guard, which stands in no block).
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        let on_failure = if failed.is_empty() {
            "None".to_string()
        } else {
            format!("lambda: {failed}")
        };
        let flag = if flag.is_empty() { "None" } else { flag };
        format!("self._sce_condition(lambda: ({value}), {on_failure}, {flag})")
    }
    // The list `emit_if` declares for the `<if>` numbered `if_ordinal`.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String {
        format!("_if_cond_failed_{if_ordinal}")
    }
    // An enum field of the payload is held in the machine's own enum, lifted
    // from the variant's declared name and written back as its `sce_name`
    // (`build_python_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`parse_delay_ms`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes
    // (`resolve_event_by_name`).
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes
    // (`engine.cancel_send`).
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template carries the value as the event's JSON and as the text a
    // host takes (`to_json_literal`, `to_wire_string`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`_host_src`).
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`_document_stem`) and starts the candidate it names, handing it the
    // values it keeps (`accept_params`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // The attribute the payload channel fills when the engine dequeues an
    // event of this name (`build_python_event_payload`).
    fn payload_accessor(&self, event: &str) -> String {
        format!(
            "self._pending_{}_payload",
            filters::to_snake_case(event.to_string())
        )
    }
    // `None` between events and for an event that carried no typed payload, so
    // a guard that reads this event's fields holds only for a delivery that
    // carried them.
    fn payload_guard(&self, _machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "{} is not None and ({lowered})",
            self.payload_accessor(event)
        )
    }
    // What the runtime's `ScriptValue.to_json_literal` writes as the pair's
    // value: a bool, an integer and a string as Python holds them, and a real as
    // a float, so that one an integer-valued computation produced is still read
    // as a real. A 64-bit integer is the text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Int { bits: 64, .. } => format!("_ScriptValue.of(str({value}))"),
            InferredType::Bool | InferredType::Str | InferredType::Int { .. } => {
                format!("_ScriptValue.of({value})")
            }
            _ => format!("_ScriptValue.of(float({value}))"),
        }
    }
    // The enum's `sce_name` answers the name the enum document declares.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("({value}).sce_name"))
    }
    // A Python `bytes` read as the Latin-1 text the other engines spell.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(self.wire_value(InferredType::Str, &format!("({value}).decode(\"latin-1\")")))
    }
}

/// Rewrite `model` — a clone the Python backend renders — so every expression
/// of a `sce-static` document is native Python.
pub fn lower_python(
    model: &mut SCXMLModel,
    machine: &str,
) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &PythonTarget)
}

/// The C type of a buffer that holds a `string` of at most `capacity` UTF-8 bytes:
/// named by the bound so that two variables, or a variable and a record's field,
/// of one bound share it ([`CTarget::string_storage`]).
pub(crate) fn c_string_storage_type(capacity: u32) -> String {
    format!("sce_static_string_{capacity}_t")
}

/// The function that fills the buffer of [`c_string_storage_type`] from a text
/// that fits it.
pub(crate) fn c_string_storage_of(capacity: u32) -> String {
    format!("sce_static_string_{capacity}_of")
}

/// The declaration of [`c_string_storage_type`] and of its fill function,
/// under a guard named by the bound. Every header that holds a text of that
/// bound writes this one text: a `sce-static` machine's record or variable,
/// and an event-schema struct an algorithm takes as a record (SCE_FORGE.md
/// §4.12), so a program that includes both declares the type once.
///
/// `_of` is the buffer holding a text already held to the bound — a payload's
/// field taken whole into a record, a copy from a field of the same or a
/// smaller bound — written without a library call, since the header includes
/// none beyond `<stddef.h>`.
pub(crate) fn c_string_storage_def(capacity: u32) -> String {
    let ty = c_string_storage_type(capacity);
    let guard = ty.to_uppercase().trim_end_matches("_T").to_string();
    format!(
        "#ifndef {guard}\n#define {guard}\n\
         /* SCE Accepted Subset §2.15: a `string` of at most {capacity} UTF-8 bytes. */\n\
         typedef struct {{\n    char data[{capacity} + 1];\n}} {ty};\n\n\
         static inline {ty} {of}(const char *text) {{\n    \
         {ty} value = {{ {{ 0 }} }};\n    \
         for (size_t i = 0; i < {capacity}u && text[i] != '\\0'; ++i) {{\n        \
         value.data[i] = text[i];\n    }}\n    return value;\n}}\n#endif",
        of = c_string_storage_of(capacity)
    )
}

/// The C statement that writes `value`, a place holding a `ty`, to the saved
/// state `w` writes (`sce/saved_values.h`): a number and a `bool` by the function
/// named for the type, a text and a byte string from the buffer that holds them,
/// and an enum by the function its type declares beside it
/// ([`CTarget::enum_def`]).
fn c_saved_write(ty: &SceType, enum_ty: Option<&str>, value: &str) -> String {
    match ty {
        SceType::Enum(_) => format!(
            "{}_saved_write(w, {value});",
            enum_ty.expect("an enum value is written by the type of its enum")
        ),
        SceType::String => format!("sce_saved_write_string(w, ({value}).data);"),
        SceType::Bytes => format!("sce_saved_write_bytes(w, ({value}).data, ({value}).len);"),
        other => format!("sce_saved_write_{}(w, {value});", other.as_attr()),
    }
}

/// The C condition that reads the saved value `member`, which `what` names in a
/// refusal, into `dest`, a place holding a `ty` of at most `bound` bytes when it
/// is a text or a byte string. True when the value was one, with the refusal in
/// `err` otherwise.
fn c_saved_read(
    ty: &SceType,
    bound: Option<u32>,
    enum_ty: Option<&str>,
    member: &str,
    what: &str,
    dest: &str,
) -> String {
    match (ty, bound) {
        (SceType::Enum(_), _) => format!(
            "{}_saved_read({member}, {what}, &{dest}, err)",
            enum_ty.expect("an enum value is read by the type of its enum")
        ),
        (SceType::String, Some(bound)) => {
            format!("sce_saved_read_string_bounded({member}, {what}, ({dest}).data, {bound}u, err)")
        }
        (SceType::Bytes, Some(bound)) => format!(
            "sce_saved_read_bytes({member}, {what}, ({dest}).data, {bound}u, &({dest}).len, err)"
        ),
        (other, _) => format!(
            "sce_saved_read_{}({member}, {what}, &{dest}, err)",
            other.as_attr()
        ),
    }
}

/// The statements that write each field of `schema` into a saved record, and the
/// ones that read them back (`value` being the record read and `at` the revision
/// it was saved at): an object keyed by the fields' ids, an optional field
/// written as `null` or as its present value, and a field a later revision added
/// taken at its default from a record saved before it (docs/adr/0044).
fn c_record_saved_members(
    schema: &EventSchemaModel,
    enum_types: &std::collections::BTreeMap<String, String>,
    target: &CTarget,
) -> (String, String) {
    let mut writes = String::new();
    let mut reads = String::new();
    for field in &schema.fields {
        let name = target.record_field(&field.id);
        let id = filters::escape_c(field.id.clone());
        let ty = field.sce_type.present_type();
        let enum_ty = match ty {
            SceType::Enum(reference) => Some(enum_types[&reference.alias].as_str()),
            _ => None,
        };
        let bound = matches!(ty, SceType::String | SceType::Bytes)
            .then_some(field.max_size)
            .flatten();
        let optional = field.sce_type.optional_elem().is_some();
        let held = format!("value->{name}");
        let write = if optional {
            format!(
                "if ({held}.present) {{\n        {}\n    }} else {{\n        sce_jw_null(w);\n    }}",
                c_saved_write(ty, enum_ty, &format!("{held}.value"))
            )
        } else {
            c_saved_write(ty, enum_ty, &held)
        };
        writes.push_str(&format!("    sce_jw_key(w, \"{id}\");\n    {write}\n"));

        let dest = format!("value.{name}");
        let take = match field.since {
            None => format!(
                "    if (!sce_saved_field(v, what, \"{id}\", &member, err)) {{\n        return false;\n    }}\n"
            ),
            Some(since) => {
                let default = crate::forge::schema_revision::default_wire(field)
                    .map(filters::escape_c)
                    .unwrap_or_default();
                let event = filters::escape_c(schema.event_name.clone());
                format!(
                    "    if (at >= {since}u) {{\n        \
                     if (!sce_saved_field(v, what, \"{id}\", &member, err)) {{\n            return false;\n        }}\n    }} else {{\n        \
                     if (sce_jv_get(v, \"{id}\", &member)) {{\n            \
                     return sce_saved_refuse(err, \"'%s' holds '{id}', which revision %u of '{event}' does not declare: revision {since} added it\", what, at);\n        }}\n        \
                     if (!sce_json_parse(\"{default}\", strlen(\"{default}\"), &member, err)) {{\n            return false;\n        }}\n    }}\n"
                )
            }
        };
        let what = "member_what";
        let read = if optional {
            // A record saved before the revision that widened the field holds
            // it as a value, so a `null` there is no absence (docs/adr/0055,
            // section 6). A member the record lacks, taken at its default, was
            // not saved at all and is not judged.
            let held_as_value = match field.optional_since {
                Some(widened) => {
                    let declared = field
                        .since
                        .map_or_else(String::new, |since| format!("at >= {since}u && "));
                    let event = filters::escape_c(schema.event_name.clone());
                    format!(
                        "    if ({declared}at < {widened}u && sce_jv_is_null(member)) {{\n        \
                         return sce_saved_refuse(err, \"'%s' is null, which revision %u of '{event}' holds as a value: revision {widened} widened it\", {what}, at);\n    }}\n"
                    )
                }
                None => String::new(),
            };
            format!(
                "{held_as_value}    if (sce_jv_is_null(member)) {{\n        memset(&{dest}, 0, sizeof({dest}));\n    }} else {{\n        \
                 {dest}.present = true;\n        if (!{}) {{\n            return false;\n        }}\n    }}\n",
                c_saved_read(ty, bound, enum_ty, "member", what, &format!("{dest}.value"))
            )
        } else {
            format!(
                "    if (!{}) {{\n        return false;\n    }}\n",
                c_saved_read(ty, bound, enum_ty, "member", what, &dest)
            )
        };
        reads.push_str(&format!(
            "{take}    sce_saved_path_field(member_what, sizeof(member_what), what, \"{id}\");\n{read}"
        ));
    }
    (writes, reads)
}

/// The type a byte string of at most `capacity` bytes is held in: a buffer and
/// the length it holds, named by the bound so that two variables, or a variable
/// and a record's field, of one bound share it ([`CTarget::bytes_storage`]).
pub(crate) fn c_bytes_storage_type(capacity: u32) -> String {
    format!("sce_static_bytes_{capacity}_t")
}

/// The function that fills the buffer of [`c_bytes_storage_type`] from the view of
/// bytes already held to the bound.
fn c_bytes_storage_of(capacity: u32) -> String {
    format!("sce_static_bytes_{capacity}_of")
}

/// C11: a variable is a member of the machine's policy struct (`sm->policy`),
/// set by the machine's own statements. A failed checked operation is a value
/// (zero, with a flag raised in `sce_failure_`) rather than a jump, as in C++, so
/// a value that can fail is computed into a local first and written only when it
/// did not.
///
/// C has no closure and no statement expression, so what the other backends
/// spell as an expression that runs a statement is spelled here as the
/// statement itself, ending its block with the `return` every block's function
/// ends on (§scxml-4.9), and a condition that can fail as the head of its `if`
/// ([`StaticTarget::receiving_condition`]).
///
/// Lowers integer and bool variables, guards, `<assign>`, `<if>`, `<log>`,
/// `<raise>` and `In()`; every construct past those is refused by name
/// ([`StaticTarget::unsupported`]) until its spelling is written, rather than
/// left as an undefined name in generated code.
pub struct CTarget {
    /// The machine's name without the suite prefix, which the payload channel's
    /// tag constants are spelled from: the channel's own types do not carry the
    /// prefix, the machine's symbols do.
    stem: String,
    /// How many conditions that can fail the walk has lowered. Each leaves its
    /// verdict in a local of its own, named by this, so that no two share a
    /// scope: an `<if>` in a branch of another would otherwise shadow it.
    conditions: std::sync::atomic::AtomicU32,
    /// How many `<foreach>` loops the walk has lowered. Each copies its list into
    /// a local of its own and counts with another, named by this, so that a loop
    /// in the body of another does not meet its names.
    loops: std::sync::atomic::AtomicU32,
    /// The name each held enum's document declares, by the name the machine
    /// holds it by ([`SCXMLModel::held_enums`]). The type and its constants are
    /// spelled from the document's name, not the alias, so that two machines
    /// holding one enum — whatever they call it — share one declaration in a
    /// program that includes both.
    enums: std::collections::BTreeMap<String, String>,
}

/// What lowers the arguments of a host action ([`target_for`]): the one thing a
/// caller outside the walk asks of C, and it asks no condition, no payload
/// guard and no enum, so there is nothing for it to count or to name a machine
/// for.
static C_ARGUMENTS: CTarget = CTarget::arguments();

impl CTarget {
    fn new(stem: &str, held_enums: &std::collections::BTreeMap<String, EnumModel>) -> Self {
        Self {
            stem: stem.to_string(),
            conditions: std::sync::atomic::AtomicU32::new(0),
            loops: std::sync::atomic::AtomicU32::new(0),
            enums: held_enums
                .iter()
                .map(|(alias, model)| (alias.clone(), model.name.clone()))
                .collect(),
        }
    }

    const fn arguments() -> Self {
        Self {
            stem: String::new(),
            conditions: std::sync::atomic::AtomicU32::new(0),
            loops: std::sync::atomic::AtomicU32::new(0),
            enums: std::collections::BTreeMap::new(),
        }
    }

    /// The type of the optional value of a record of the schema `alias` that the
    /// machine `machine` holds ([`Self::record_type`]).
    fn optional_record_type(&self, machine: &str, alias: &str) -> Option<String> {
        c11_optional_type(&self.record_type(machine, alias))
    }

    /// The name the document of the enum imported as `alias` declares — the
    /// alias itself for one the walk has no document of.
    fn enum_document_name(&self, alias: &str) -> String {
        self.enums
            .get(alias)
            .cloned()
            .unwrap_or_else(|| alias.to_string())
    }

    /// The first action of `actions`, or of a block nested in one, that this
    /// target has no lowering for yet.
    fn unlowered_action(actions: &[Action]) -> Option<String> {
        for action in actions {
            match action.action_type.as_str() {
                // A host action's arguments are typed expressions of the
                // machine's variables, lowered where the call is rendered
                // ([`crate::forge::native_action`]).
                "assign" | "log" | "if" | "raise" | "native_action" | "sce_append"
                | "sce_clear" | "foreach" | "cancel" => {}
                // The pairs of a `<send>`'s `<param>`s are written as the JSON
                // object of the event's data from the machine's own fields, as a
                // `<donedata>`'s are, and for a processor the host serves also
                // as the text of the request's `params`, from the same value.
                // Its literal `<content>` is the text it spells, finished at
                // build time ([`crate::filters::static_content_wire`]) and
                // copied into the event's data; an evaluated one names a record,
                // which crosses as the pairs of its fields, as a `namelist`'s
                // names do. A processor that no one is declared to serve is
                // refused by name.
                "send" => {
                    // A computed type is delivered by the processor each type it
                    // declares names, so each is held to what a written one is.
                    for send_type in effective_send_types(action) {
                        let scxml_processor = send_type.is_empty()
                            || send_type == "scxml"
                            || send_type.ends_with("#SCXMLEventProcessor");
                        // BasicHTTP is the one processor the machine performs
                        // itself (§scxml-C-2): its request is built by the arm a
                        // host-served send is, and made by the runtime's HTTP client
                        // (docs/adr/0005, decision 4).
                        let basic_http = send_type.ends_with("#BasicHTTPEventProcessor");
                        if !scxml_processor && !basic_http && !action.send_type_host_served {
                            return Some(format!("a <send> of type `{send_type}`"));
                        }
                    }
                    if let Some(name) = namelist_name_taken(
                        action.params.iter().map(|p| p.name.as_str()),
                        &action.namelist,
                    ) {
                        return Some(format!(
                            "a <send> whose namelist names `{name}`, which a <param> or the namelist already does"
                        ));
                    }
                }
                other => return Some(format!("<{other}>")),
            }
            for block in action.nested_blocks() {
                if let Some(found) = Self::unlowered_action(block.actions) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// The first of `invokes` that C11 has no lowering for yet. A child session
    /// of `type="scxml"` is started by the machine's own invoke code, which
    /// hands a static child its parent's values between the two steps its
    /// start takes (§scxml-6.4.1). An `<invoke>` the host serves is lowered: its
    /// `<param>`s are written into the request from the machine's own fields
    /// (`sce/forge/wire.h`), as a final's are.
    fn unlowered_invoke(invokes: &[crate::model::Invoke], lowers_hybrid: bool) -> Option<String> {
        if let Some(other) = unlowered_invoke(invokes, true, lowers_hybrid, false) {
            return Some(other);
        }
        invokes.iter().find_map(|invoke| match invoke {
            // A `namelist` name is a pair of the request like a `<param>`.
            crate::model::Invoke::Unsupported(info) if info.host_served => {
                namelist_name_taken(info.base.params.iter().map(|p| p.name.as_str()), &info.namelist)
                    .map(|name| {
                        format!(
                            "an <invoke id=\"{}\"> whose namelist names `{name}`, which a <param> or the namelist already does",
                            info.base.invoke_id
                        )
                    })
            }
            _ => None,
        })
    }

    /// The C type a value of `ty` is held in while it is computed. An enum's
    /// value has no inferred type of its own, and is held in an `int` — which
    /// the enumerated type takes back by assignment.
    fn value_type(ty: InferredType) -> String {
        match ty {
            InferredType::Unknown => "int".to_string(),
            // A byte string is computed as the view it is written from.
            InferredType::Bytes => "sce_forge_bytes_view_t".to_string(),
            _ => match ty.to_sce_type() {
                Some(held) => crate::forge::generator::c_type(&held).to_string(),
                None => unreachable!("C11 lowers only values that have a declared type: {ty:?}"),
            },
        }
    }
}

impl StaticTarget for CTarget {
    fn name(&self) -> &'static str {
        "C11"
    }
    // The start is deferred to the macrostep's end by the entry of the state, which
    // formats the id it generates, `<state>.<start>.<invoke>`, into the char buffer
    // `_runtime_id_buf` and writes it to the `idlocation` there, before the invocation is
    // queued and so before a sibling read in the same macrostep (§scxml-6.4.1, test225).
    fn generated_invoke_id(&self) -> Option<&'static str> {
        Some("_runtime_id_buf")
    }
    // An algorithm's list is passed and returned by value, a few copies of it at a call.
    fn list_stack_limit_bytes(&self) -> Option<u64> {
        Some(crate::forge::generator::C11_LIST_STACK_LIMIT_BYTES)
    }
    // A statement that fails is a block of its own that returns from the function: the
    // blocks follow one another.
    fn in_sequence(&self, parts: &[String]) -> String {
        parts.join("\n")
    }
    // An algorithm's C artifact is a header of `static inline` functions, which
    // the machine includes by the line a forge kind importing it writes; a call
    // that can fail is received through the `<name>_take` the header declares
    // (the expression lowerer spells it). A lookup whose miss is an error answers
    // through a pointer, which no expression holds: it is called through the
    // reading the machine declares for it ([`Self::callee_def`]).
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        let mut reached = generated_callee(Language::C11, callee, None);
        if callee.reading == CallReading::Found {
            reached.call = crate::forge::generator::c11_lookup_found_name(&reached.call);
        }
        Some(reached)
    }
    fn callee_def(&self, callee: &StaticCallee) -> Option<String> {
        if callee.reading != CallReading::Found {
            return None;
        }
        let function = generated_callee(Language::C11, callee, None).call;
        // A value of an enum is of the type its document declares, which the lookup
        // and the machine both hold it in.
        let c_slot = |ty: &SceType| match ty {
            SceType::Enum(reference) => crate::forge::generator::enum_qualified_type(
                &self.enum_document_name(&reference.alias),
                Language::C11,
            ),
            other => crate::forge::generator::c_type(other).to_string(),
        };
        Some(crate::forge::generator::c11_lookup_found(
            &function,
            &c_slot(callee.params.first()?),
            &c_slot(callee.ret.as_ref()?),
        ))
    }
    fn lowers_state_calls(&self) -> bool {
        true
    }
    // The elements the algorithm's unit spells in a slot
    // ([`crate::forge::generator::algorithm_optional_spelled`]): a text is held
    // only as the field of a record, over the buffer of its `sce:max-size`, and
    // the slot of an algorithm declares no bound to hold one in.
    fn holds_algorithm_optional(&self, elem: &crate::forge::model::OptionalElemType) -> bool {
        !matches!(
            elem,
            crate::forge::model::OptionalElemType::Scalar(SceType::String)
        ) && crate::forge::generator::algorithm_optional_spelled(Language::C11, elem)
    }
    // A list is handed as the borrowed view of its elements, `{data, len}`, as the
    // algorithm's own list parameter is; a record or a payload as the struct it is.
    //
    // An optional record is handed as the optional value of the machine's record
    // type where the machine holds one (docs/adr/0055, step 9): `null` (code `n`) as
    // the absent value and an optional record variable (`o`) as it is. A record
    // variable, a loop's record item and the payload are handed as the struct each
    // is, which the adapter lifts as it does for a record slot.
    fn state_argument(&self, machine: &str, arg: &StateArg, held: &str) -> String {
        match &arg.slot {
            StaticSlot::ScalarList(ty) => format!(
                "({}){{ .data = {held}.data, .len = {held}.len }}",
                self.list_view(ty).unwrap_or_default()
            ),
            StaticSlot::RecordList(alias) => format!(
                "({}){{ .data = {held}.data, .len = {held}.len }}",
                self.record_list_view(&self.record_type(machine, alias))
                    .unwrap_or_default()
            ),
            StaticSlot::OptionalRecord(alias) => {
                let elem = c11_optional_elem(&self.record_type(machine, alias))
                    .expect("a record type is the element of an optional record");
                match arg.code {
                    'n' => format!("sce_optional_{elem}_none()"),
                    _ => held.to_string(),
                }
            }
            _ => held.to_string(),
        }
    }
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("call_{coded}")
    }
    // The list an adapter gave back is its result struct, read from `items` and `len`,
    // as an algorithm's is; the adapter declares the `_within` that holds it to the
    // variable's bound.
    fn state_list_within(&self, adapter: &str, value: &str, capacity: u32) -> String {
        format!("{adapter}_within(&sce_failure_, {value}, {capacity}u)")
    }
    // A function of the machine's header that builds the algorithm's records from the
    // machine's, calls it, and builds the machine's own from what it returns,
    // answering as the algorithm does: a result struct and the `<name>_take` that
    // receives it where the algorithm may fail, which is what the expression the
    // machine lowers calls. What the algorithm is handed is a copy, and what it
    // returns is copied, so nothing it does reaches the machine's state.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{event_schema_field_ident, event_schema_payload_type};
        let value = spec.callee.value.as_ref()?;
        let name = &spec.name;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        let algorithm_record = |alias: &str| {
            plain_schema(spec, alias)
                .map(|schema| event_schema_payload_type(&schema.name, Language::C11))
        };
        // A record of the type `ty` made field by field from `from`, an enum field as
        // the variant of the same declared name the other side holds.
        let record_of =
            |alias: &str, ty: String, from: &str, crossing: Crossing| -> Option<String> {
                let schema = plain_schema(spec, alias)?;
                let fields = schema
                    .fields
                    .iter()
                    .map(|f| {
                        let field = event_schema_field_ident(&f.id, Language::C11);
                        spec.record_field_crossing(
                            self,
                            f,
                            &format!("{from}.{field}"),
                            crossing,
                            str::to_string,
                        )
                        .map(|value| format!(".{field} = {value}"))
                    })
                    .collect::<Option<Vec<String>>>()?;
                Some(format!("({ty}){{ {} }}", fields.join(", ")))
            };
        let mut declarations = Vec::new();
        let mut prelude = String::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let p = format!("p{index}");
            match param {
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "{} {p}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &p,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) => {
                    if !plain_scalar(ty) {
                        return None;
                    }
                    declarations.push(format!("{} {p}", self.scalar_type(ty)));
                    arguments.push(p);
                }
                // `sce_optional_<elem>_t` of the runtime on both sides, a value
                // (docs/adr/0055).
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("{} {p}", spec.optional_type(self, ty)?));
                    arguments.push(p);
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    let held = if *code == 'p' {
                        let event = spec.payload_events.get(alias)?;
                        format!(
                            "{}_{}_payload_t",
                            spec.machine,
                            filters::to_c11_lower_ident(std::borrow::Cow::Borrowed(event.as_str()))
                        )
                    } else {
                        machine_record(alias)
                    };
                    declarations.push(format!("{held} {p}"));
                    arguments.push(record_of(
                        alias,
                        algorithm_record(alias)?,
                        &p,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // An optional record the algorithm takes is the optional value of its
                // own record, built from what the call hands over (docs/adr/0055, step
                // 9): the optional value of the machine's record, for `null` and for an
                // optional record variable, whose record is copied field by field when
                // it is present; and a record the machine holds, a record variable, a
                // loop's item or the payload, copied the same way and lifted.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => {
                    let algorithm = algorithm_record(alias)?;
                    let elem = c11_optional_elem(&algorithm)?;
                    if matches!(*code, 'n' | 'o') {
                        declarations.push(format!(
                            "{} {p}",
                            self.optional_record_type(spec.machine, alias)?
                        ));
                        let record = record_of(
                            alias,
                            algorithm,
                            &format!("{p}.value"),
                            Crossing::ToAlgorithm,
                        )?;
                        arguments.push(format!(
                            "({p}.present ? sce_optional_{elem}_some({record}) : \
                             sce_optional_{elem}_none())"
                        ));
                    } else {
                        let held = if *code == 'p' {
                            let event = spec.payload_events.get(alias)?;
                            format!(
                                "{}_{}_payload_t",
                                spec.machine,
                                filters::to_c11_lower_ident(std::borrow::Cow::Borrowed(
                                    event.as_str()
                                ))
                            )
                        } else {
                            machine_record(alias)
                        };
                        declarations.push(format!("{held} {p}"));
                        let record = record_of(alias, algorithm, &p, Crossing::ToAlgorithm)?;
                        arguments.push(format!("sce_optional_{elem}_some({record})"));
                    }
                }
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => {
                    declarations.push(format!("{} {p}", self.list_view(ty)?));
                    arguments.push(p);
                }
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    let algorithm = algorithm_record(alias)?;
                    // The list is copied into a buffer of the largest bound any list of
                    // these records has in the machine, which the list handed in fits.
                    let bound = spec
                        .record_list_bounds
                        .get(alias)
                        .copied()
                        .unwrap_or(1)
                        .max(1);
                    declarations.push(format!(
                        "{} {p}",
                        self.record_list_view(&machine_record(alias))?
                    ));
                    prelude.push_str(&format!(
                        "    {algorithm} a{index}[{bound}u];\n    for (size_t i = 0; i < {p}.len && i < {bound}u; ++i) {{\n        a{index}[i] = {};\n    }}\n",
                        record_of(
                            alias,
                            algorithm.clone(),
                            &format!("{p}.data[i]"),
                            Crossing::ToAlgorithm
                        )?
                    ));
                    arguments.push(format!(
                        "({}_view_t){{ .data = a{index}, .len = {p}.len }}",
                        algorithm.trim_end_matches("_t")
                    ));
                }
                _ => return None,
            }
        }
        let call = format!("{}({})", spec.call, arguments.join(", "));
        let result_type = format!("{name}_result_t");
        // The machine's own element or value, and the bound the algorithm's list has.
        enum Shape {
            Value(String),
            List(String, u32),
        }
        let shape = match value.ret.as_ref()? {
            StaticSlot::Scalar(SceType::Enum(reference)) => {
                Shape::Value(self.enum_type(spec.machine, &reference.alias)?)
            }
            StaticSlot::Scalar(ty) if plain_scalar(ty) => Shape::Value(self.scalar_type(ty)),
            StaticSlot::Optional(ty) => Shape::Value(spec.optional_type(self, ty)?),
            StaticSlot::OptionalRecord(alias) => {
                Shape::Value(self.optional_record_type(spec.machine, alias)?)
            }
            StaticSlot::Record(alias) => Shape::Value(machine_record(alias)),
            StaticSlot::ScalarList(ty) if plain_scalar(ty) => {
                Shape::List(self.scalar_type(ty), value.max_size?)
            }
            StaticSlot::RecordList(alias) => Shape::List(machine_record(alias), value.max_size?),
            _ => return None,
        };
        // From the algorithm's result `r`: the machine's value, or the statements
        // that fill `out` with the machine's list.
        let converted = |from: &str| -> Option<String> {
            Some(match value.ret.as_ref()? {
                StaticSlot::Scalar(SceType::Enum(reference)) => {
                    spec.enum_crossing(self, &reference.alias, from, Crossing::FromAlgorithm)?
                }
                StaticSlot::Scalar(_) | StaticSlot::Optional(_) => from.to_string(),
                // The optional value of the algorithm's record is the optional value
                // of the machine's, its record copied field by field where it is
                // present (docs/adr/0055, step 9).
                StaticSlot::OptionalRecord(alias) => {
                    let elem = c11_optional_elem(&machine_record(alias))?;
                    let record = record_of(
                        alias,
                        machine_record(alias),
                        &format!("{from}.value"),
                        Crossing::FromAlgorithm,
                    )?;
                    format!(
                        "({from}.present ? sce_optional_{elem}_some({record}) : \
                         sce_optional_{elem}_none())"
                    )
                }
                StaticSlot::Record(alias) => {
                    record_of(alias, machine_record(alias), from, Crossing::FromAlgorithm)?
                }
                _ => String::new(),
            })
        };
        let list_fill = match value.ret.as_ref()? {
            StaticSlot::ScalarList(_) => "        out.items[i] = r.items[i];\n".to_string(),
            StaticSlot::RecordList(alias) => format!(
                "        out.items[i] = {};\n",
                record_of(
                    alias,
                    machine_record(alias),
                    "r.items[i]",
                    Crossing::FromAlgorithm
                )?
            ),
            _ => String::new(),
        };
        let scalar_or_record_value = |from: &str| converted(from);
        // The algorithm's own result struct.
        let call_result = format!("{}_result_t", spec.call);
        Some(match (spec.callee.may_fail, &shape) {
            (true, Shape::Value(ty)) => format!(
                "typedef struct {{\n    {ty} value;\n    bool ok;\n    sce_forge_algorithm_error_t why;\n}} {result_type};\n\n\
                 static inline {ty} {name}_take(sce_forge_algorithm_failure_t *failure, {result_type} result) {{\n    if (!result.ok) {{\n        sce_forge_algorithm_fail(failure, result.why);\n        return ({ty}){{0}};\n    }}\n    return result.value;\n}}\n\n\
                 static inline {result_type} {name}({}) {{\n{prelude}    {call_result} r = {call};\n    if (!r.ok) {{\n        return ({result_type}){{ .ok = false, .why = r.why }};\n    }}\n    return ({result_type}){{ .value = {}, .ok = true }};\n}}",
                declarations.join(", "),
                scalar_or_record_value("r.value")?,
            ),
            (true, Shape::List(ty, max)) => format!(
                "typedef struct {{\n    {ty} items[{max}u];\n    size_t len;\n    bool ok;\n    sce_forge_algorithm_error_t why;\n}} {result_type};\n\n\
                 static inline {result_type} {name}_take(sce_forge_algorithm_failure_t *failure, {result_type} result) {{\n    if (!result.ok) {{\n        sce_forge_algorithm_fail(failure, result.why);\n        return ({result_type}){{0}};\n    }}\n    return result;\n}}\n\n\
                 static inline {result_type} {name}_within(sce_forge_algorithm_failure_t *failure, {result_type} result, uint32_t capacity) {{\n    if (result.len > capacity) {{\n        sce_forge_algorithm_fail(failure, SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED);\n        return ({result_type}){{0}};\n    }}\n    return result;\n}}\n\n\
                 static inline {result_type} {name}({}) {{\n{prelude}    {call_result} r = {call};\n    if (!r.ok) {{\n        return ({result_type}){{ .ok = false, .why = r.why }};\n    }}\n    {result_type} out = {{ .ok = true }};\n    for (size_t i = 0; i < r.len && i < {max}u; ++i) {{\n{list_fill}    }}\n    out.len = r.len;\n    return out;\n}}",
                declarations.join(", "),
            ),
            (false, Shape::Value(ty)) => {
                // What the algorithm returns, in the algorithm's own type.
                let returned = match value.ret.as_ref()? {
                    StaticSlot::Record(alias) => algorithm_record(alias)?,
                    StaticSlot::OptionalRecord(alias) => {
                        c11_optional_type(&algorithm_record(alias)?)?
                    }
                    _ => ty.clone(),
                };
                format!(
                    "static inline {ty} {name}({}) {{\n{prelude}    {returned} r = {call};\n    return {};\n}}",
                    declarations.join(", "),
                    scalar_or_record_value("r")?
                )
            }
            (false, Shape::List(..)) => return None,
        })
    }
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        // The integers, the bool and an enum: a `bool`, a number and an
        // enumerated type are the values the checked helpers and the policy
        // hold without a length. A list of integers or bools too: it is a
        // buffer of its bound, which every list declares ([`Self::bounded_list`]).
        // A string is a buffer of the bound its variable declares, which the
        // document requires of every one ([`Self::string_storage`]). A byte string
        // is the same, a buffer and its length of the bound
        // ([`Self::bytes_storage`]). A 64-bit
        // real is a `double`, written to the wire as ECMAScript spells it, alone,
        // in a list and as a record's field. A 32-bit one is a `float`, alone, in
        // a list and as a record's field, written to the wire as the `double` it
        // widens to. A record is a struct the machine's own header declares, of
        // fields held as those are.
        let held_scalar = |ty: &SceType| {
            matches!(
                ty,
                SceType::Bool
                    | SceType::Uint8
                    | SceType::Uint16
                    | SceType::Uint32
                    | SceType::Uint64
                    | SceType::Int8
                    | SceType::Int16
                    | SceType::Int32
                    | SceType::Int64
            )
        };
        if let Some(var) = scope.variables.iter().find(|v| {
            let Some(value_type) = v.value_type.as_ref() else {
                return true;
            };
            // A record, alone or as a list's element, is judged below by its
            // schema.
            if value_type.record_alias().is_some() {
                return false;
            }
            // An optional record is the struct `sce/forge/optional.h` declares over
            // the machine's record type, whose schema is judged below
            // (docs/adr/0055, step 9).
            if value_type
                .optional_elem()
                .is_some_and(|elem| elem.record_alias().is_some())
            {
                return false;
            }
            // An optional value of what a variable holds is held as the struct
            // `sce/forge/optional.h` declares over it (docs/adr/0055).
            if let Some(elem) = value_type.optional_elem().and_then(|e| e.scalar()) {
                return !(held_scalar(elem)
                    || matches!(elem, SceType::Enum(_) | SceType::Float32 | SceType::Float64)
                    || (matches!(elem, SceType::String | SceType::Bytes) && v.capacity.is_some()));
            }
            if let Some(elem) = value_type.list_elem() {
                return !(v.capacity.is_some()
                    && match elem {
                        crate::forge::model::ListElemType::Scalar(ty) => {
                            held_scalar(ty) || matches!(ty, SceType::Float32 | SceType::Float64)
                        }
                        crate::forge::model::ListElemType::Record { .. } => true,
                    });
            }
            !matches!(
                value_type.scalar(),
                Some(ty) if held_scalar(ty)
                    || matches!(ty, SceType::Enum(_) | SceType::Float32 | SceType::Float64)
                    || (matches!(ty, SceType::String | SceType::Bytes) && v.capacity.is_some())
            )
        }) {
            let ty = match &var.value_type {
                Some(t) if t.optional_elem().is_some() => t.as_attr(),
                Some(t) => t.scalar().map_or_else(
                    || {
                        t.record_alias()
                            .map_or("list".to_string(), |alias| format!("record:{alias}"))
                    },
                    SceType::as_attr,
                ),
                None => "no type".to_string(),
            };
            return Some(format!("<data id=\"{}\" sce:type=\"{ty}\">", var.id));
        }
        // A record is a struct of the fields its schema declares: the numbers
        // and bools a list holds, reals of either width, a string or a byte string
        // of the bound the schema declares (a buffer, as a variable's is), and an enum of
        // the document the schema imports (docs/adr/0054). A field is named as
        // the author wrote it; a
        // name C reserves is refused where the schema is read, for every
        // backend at once.
        for var in &scope.variables {
            let Some(alias) = var.value_type.as_ref().and_then(|t| {
                t.record_alias()
                    .or_else(|| t.list_elem().and_then(|e| e.record_alias()))
                    .or_else(|| t.optional_elem().and_then(|e| e.record_alias()))
            }) else {
                continue;
            };
            let Some(schema) = model.imported_records.get(alias) else {
                continue;
            };
            if let Some(field) = schema.fields.iter().find(|f| {
                // `lower` refused an optional field before asking (docs/adr/0055).
                let ty = f.sce_type.present_type();
                !held_scalar(ty)
                    && !matches!(ty, SceType::Enum(_) | SceType::Float32 | SceType::Float64)
                    // A string or a byte string with no bound was refused where the
                    // record was declared, so the one that reaches here has its buffer.
                    && !(matches!(ty, SceType::String | SceType::Bytes) && f.max_size.is_some())
            }) {
                return Some(format!(
                    "record:{alias} with the field `{}` of type {}",
                    field.id,
                    field.sce_type.as_attr()
                ));
            }
        }
        // An optional text or byte string of a schema the machine holds a record
        // of or reads a payload by is held in the buffer of its `sce:max-size`
        // (docs/adr/0055, section 1: a text keeps the bound its type declares);
        // one that declares none has no buffer to be held in.
        if let Some((schema, field)) = model
            .imported_records
            .values()
            .chain(model.imported_event_schemas.values())
            .find_map(|schema| {
                schema
                    .fields
                    .iter()
                    .find(|f| {
                        f.max_size.is_none()
                            && matches!(
                                f.sce_type.optional_elem(),
                                Some(SceType::String | SceType::Bytes)
                            )
                    })
                    .map(|field| (schema, field))
            })
        {
            return Some(format!(
                "the field `{}` of the event schema `{}`, sce:type=\"{}\" with no sce:max-size",
                field.id,
                schema.name,
                field.sce_type.as_attr()
            ));
        }
        if !model.global_scripts.is_empty() {
            return Some("a <script>".to_string());
        }
        for state in model.states.values() {
            let blocks = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for block in blocks {
                if let Some(found) = Self::unlowered_action(block) {
                    return Some(found);
                }
            }
            // A typed payload is read through the channel the machine's own
            // file declares for it, field by field. A number and a bool are the
            // fields it holds as values, and a string is a borrowed pointer into
            // a buffer the machine owns, which an assignment then holds to its
            // own variable's bound. An enum is the machine's own type, lifted
            // from the variant's declared name. A byte field is a buffer with a
            // length, held to the bound of the place it is written to.
            if let Some(other) = Self::unlowered_invoke(&state.invokes, self.lowers_hybrid_invoke())
            {
                return Some(other);
            }
            // The pairs of a `<donedata>` are written as the JSON object of the
            // done event's data from the machine's own fields, and its inline
            // `<content>` is the text it spells, finished at build time
            // ([`crate::filters::static_content_wire`]) and copied into that
            // data. An evaluated `<content expr>` names a record, which the
            // model let through as the pairs of its fields. A name that repeats
            // is one array, as in a `<send>`'s.
            if state.donedata.as_ref().is_some_and(|done| {
                !matches!(
                    done.content,
                    crate::model::DoneDataContent::None
                        | crate::model::DoneDataContent::InlineText(_)
                        | crate::model::DoneDataContent::Expression(_)
                )
            }) {
                return Some("a <donedata> with a <content> that is not inline text".to_string());
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::C
    }
    // A prefix keeps a variable's member from meeting another member of the
    // policy, which a bare name could (`last_transition_source_state`).
    fn field_name(&self, id: &str) -> String {
        format!("v_{}", filters::to_snake_case(id.to_string()))
    }
    // A free function of the machine, as every typed reader of the backend is;
    // the `get_` keeps it from meeting `_init`, `_step` or `_raise`, which a
    // bare name could.
    fn reader_name(&self, id: &str, _machine: &str) -> Option<String> {
        Some(format!("get_{}", id.replace(['.', '-'], "_")))
    }
    fn field_ref(&self, name: &str) -> String {
        format!("sm->policy.{name}")
    }
    // The machine is built through its pointer, so the variables declared
    // before an initial value are reached as every statement reaches them.
    fn initial_field_ref(&self, name: &str) -> String {
        self.field_ref(name)
    }
    // A name the lowered text still carries as a call, which the template's
    // `to_in_predicate_c11` turns into the enumerator test the machine already
    // answers a pure `In()` with: C has no string-keyed test of a state.
    fn in_function(&self) -> &'static str {
        "In"
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::c_type(ty).to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    // The type is the document's, shared by every machine that imports the
    // enum: one declaration, guarded, in each machine's header ([`Self::enum_def`]).
    fn enum_type(&self, _machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{}_enum_t",
            filters::to_snake_case(self.enum_document_name(alias))
        ))
    }
    // C has no namespace, so the constant carries the document's name — the one
    // spelling the enum kind's own C artifact declares, and a reference to a
    // variant ([`crate::forge::enum_naming::variant_ref`]) contains.
    fn enum_variant(&self, _ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::C11, enum_name, variant)
    }
    // The type the enum document's own header declares, written as the same text
    // under the guard both headers share ([`crate::forge::generator::c11_enum_declaration`]):
    // a program that includes this machine and an algorithm that holds a value of
    // the enum declares the constants once, and the value crosses between them as
    // the value it is. The machine's name for the type is an alias of it, under a
    // guard of the document's name, so that a program including two machines which
    // import the same enum declares it once. `<prefix>_declared_name` answers the
    // name the document declares for a value — what a saved or observed value is —
    // and NULL for one no variant names.
    //
    // `<prefix>_number` answers the number the document declares for a value — the
    // value itself, since the type's constants carry the declared numbers — and the
    // `<prefix>_from_number` family the variant a number declares, or that none
    // does: what `<alias>.toNumber` and `<alias>.fromNumber` are lowered to
    // ([`StaticTarget::enum_number_symbols`]). The family is the one text an
    // algorithm that may fail declares beside the same enum, under the same guard
    // ([`crate::forge::generator::c11_enum_from_number`]).
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let prefix = crate::forge::enum_naming::c11_function_prefix(&model.name);
        let guard = format!("SCE_STATIC_ENUM_{}", prefix.to_uppercase());
        let carrier = crate::forge::generator::c_type(&model.underlying_type);
        let variants: Vec<(String, i128)> = model
            .variants
            .iter()
            .map(|v| (v.name.clone(), v.value))
            .collect();
        // Its saved form is the name the enum document declares for the variant, the
        // same on every backend: written by `{ty}_saved_write` and read by
        // `{ty}_saved_read`, beside the type under its guard, so a program that
        // includes two machines of one enum declares them once. The reader names the
        // enum by the document, which both machines import it from, and not by the
        // alias one of them happens to.
        let saved_names: String = model
            .variants
            .iter()
            .map(|v| format!("        \"{}\",\n", filters::escape_c(v.name.clone())))
            .collect();
        let saved_values: String = model
            .variants
            .iter()
            .map(|v| format!("        {},\n", self.enum_variant(ty, &model.name, &v.name)))
            .collect();
        format!(
            "{}\n\n#ifndef {guard}\n#define {guard}\n\
             /* SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value. */\n\
             typedef {} {ty};\n\n\
             /* The number the enum document declares for `value`. */\n\
             static inline {carrier} {prefix}_number({ty} value) {{\n    return ({carrier})value;\n}}\n\n\
             /* `value` as a saved state writes it: the name the enum document declares for it. */\n\
             static inline void {ty}_saved_write(sce_jw_t *w, {ty} value) {{\n    \
             const char *declared = {prefix}_declared_name(value);\n    \
             sce_jw_string(w, declared != NULL ? declared : \"\");\n}}\n\n\
             /* The value a saved state holds, refused unless it is a variant. `out` may be NULL, \
             which judges the value and keeps nothing. */\n\
             static inline bool {ty}_saved_read(sce_jv_t v, const char *what, {ty} *out, sce_saved_error_t *err) {{\n    \
             static const char *const names[] = {{\n{saved_names}    }};\n    \
             static const {ty} values[] = {{\n{saved_values}    }};\n    \
             size_t index = 0u;\n    \
             if (!sce_saved_read_variant(v, what, \"{document}\", names, sizeof(names) / sizeof(names[0]), &index, err)) {{\n        \
             return false;\n    }}\n    \
             if (out != NULL) {{\n        *out = values[index];\n    }}\n    return true;\n}}\n#endif\n\n{}",
            crate::forge::generator::c11_enum_declaration(model),
            crate::forge::generator::enum_qualified_type(&model.name, Language::C11),
            crate::forge::generator::c11_enum_from_number(
                &model.name,
                &model.underlying_type,
                &variants
            ),
            document = filters::escape_c(model.name.clone())
        )
    }
    fn enum_number_symbols(&self, _ty: &str, model: &EnumModel) -> (String, String) {
        let prefix = crate::forge::enum_naming::c11_function_prefix(&model.name);
        (format!("{prefix}_number"), format!("{prefix}_from_number"))
    }
    // The machine holds the value in the type the algorithm holds it in
    // ([`Self::enum_def`]), so it crosses as it is.
    fn enum_crossing(
        &self,
        _machine_type: &str,
        _model: &EnumModel,
        value: &str,
        _crossing: Crossing,
    ) -> Option<String> {
        Some(value.to_string())
    }
    // C has no namespace: each bound is a macro named by the machine, typed by
    // `UINT32_C`, which a host can size an array with.
    fn limits_def(&self, machine: &str, limits: &[(String, u32)]) -> Option<String> {
        let prefix = machine.to_uppercase();
        let constants: Vec<String> = limits
            .iter()
            .map(|(name, value)| {
                format!(
                    "/* {} */\n#define {prefix}_LIMIT_{} UINT32_C({value})",
                    limit_doc(name),
                    name.to_uppercase()
                )
            })
            .collect();
        Some(format!("/* {LIMITS_DOC} */\n{}", constants.join("\n")))
    }
    // The machine's own, named by its symbol (prefix included): a record is the
    // machine's, declared in its header like the payload types, so two machines
    // of one program that import one schema declare two types.
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}_record_{}_t",
            filters::to_snake_case(alias.to_string())
        )
    }
    // Plain data by the record rule: a struct of the schema's fields in the
    // schema's order, copied by assignment, and the borrowed view a host reads
    // a published list of them through. An enum field is held in the machine's
    // own type for the enum, which is declared before the record.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                let present_ty = match (field.sce_type.present_type(), field.max_size) {
                    (SceType::Enum(reference), _) => enum_types[&reference.alias].clone(),
                    // A string is the buffer of the bound its schema declares, and a
                    // byte string that buffer with the length it holds.
                    (SceType::String, Some(bound)) => c_string_storage_type(bound),
                    (SceType::Bytes, Some(bound)) => c_bytes_storage_type(bound),
                    (other, _) => crate::forge::generator::c_type(other).to_string(),
                };
                // An optional field is the struct over its present value's type
                // (docs/adr/0055), declared before the record
                // ([`StaticTarget::optional_def`]); `unsupported` refused one with
                // no type to be held over.
                let field_ty = match field.sce_type.optional_elem() {
                    Some(_) => c11_optional_type(&present_ty).unwrap_or(present_ty),
                    None => present_ty,
                };
                format!("    {field_ty} {};\n", self.record_field(&field.id))
            })
            .collect();
        let view = format!("{}_view_t", ty.trim_end_matches("_t"));
        let (writes, reads) = c_record_saved_members(schema, enum_types, self);
        format!(
            "/* SCE Accepted Subset §2.15: a `record:{alias}` datamodel value. */\n\
             typedef struct {{\n{fields}}} {ty};\n\n\
             /* A borrowed view of a list of them: what a host reads, and cannot grow past\n   \
             the bound the machine keeps. */\n\
             typedef struct {{\n    const {ty} *data;\n    size_t len;\n}} {view};\n\n\
             /* `value` as a saved state writes it: an object of the schema's fields keyed by\n   \
             their ids. */\n\
             static inline void {ty}_saved_write(sce_jw_t *w, const {ty} *value) {{\n    \
             sce_jw_begin_object(w);\n{writes}    sce_jw_end_object(w);\n}}\n\n\
             /* The value a saved state holds, refused unless it is one. `at` is the revision of\n   \
             the schema it was saved at: a field a later revision added is taken at its default\n   \
             (docs/adr/0044). `out` may be NULL, which judges the value and keeps nothing. */\n\
             static inline bool {ty}_saved_read(sce_jv_t v, const char *what, unsigned at, {ty} *out,\n    \
             sce_saved_error_t *err) {{\n    \
             {ty} value;\n    char member_what[SCE_SAVED_AT_LEN];\n    sce_jv_t member;\n    \
             (void)at;\n    (void)member_what;\n    (void)member;\n    \
             memset(&value, 0, sizeof(value));\n    \
             if (sce_jv_kind(v) != SCE_JSON_OBJECT) {{\n        \
             return sce_saved_refuse(err, \"'%s' is not an object\", what);\n    }}\n{reads}    \
             if (out != NULL) {{\n        *out = value;\n    }}\n    return true;\n}}"
        )
    }
    // The author's spelling, which is what an expression's member access writes
    // (`shown.year` is `sm->policy.v_shown.year`); a schema that names a field
    // for a word C reserves is refused where it is read.
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let inits: Vec<String> = fields.iter().map(|(f, v)| format!(".{f} = {v}")).collect();
        format!("({ty}){{ {} }}", inits.join(", "))
    }
    // A list's type carries its bound, so it is named by [`Self::bounded_list`].
    fn list_type(&self, _elem: &SceType) -> String {
        unreachable!("a C11 list is declared through `bounded_list`")
    }
    // The library's borrowed view over the elements — `{data, len}` — which a host
    // reads and cannot grow past the bound the machine keeps. It is named by the
    // element's C type, as an algorithm's list parameter names it, and not by the
    // document's type name: the two agree for an integer and differ for a real
    // (`float64` is a `double`).
    fn list_view(&self, elem: &SceType) -> Option<String> {
        Some(format!(
            "sce_forge_{}_view_t",
            crate::forge::generator::c_type(elem).trim_end_matches("_t")
        ))
    }
    fn record_list_type(&self, _record: &str) -> String {
        unreachable!("a C11 list is declared through `bounded_record_list`")
    }
    fn record_list_view(&self, record: &str) -> Option<String> {
        Some(format!("{}_view_t", record.trim_end_matches("_t")))
    }
    fn list_empty(&self) -> String {
        unreachable!("a C11 list is declared through `bounded_list` or `bounded_record_list`")
    }
    // The buffer of a list of numbers, of a record's own type: named by the
    // record and the bound, and declared once beside the record, which is the
    // machine's alone and so needs no guard.
    fn bounded_record_list(&self, record: &str, capacity: Option<u32>) -> BoundedList {
        let capacity = capacity.expect("a C11 list has the bound `unsupported` asked of it");
        let ty = format!("{}_list_{capacity}_t", record.trim_end_matches("_t"));
        let def = format!(
            "/* SCE Accepted Subset §2.15: a list of at most {capacity} of `{record}`. */\n\
             typedef struct {{\n    size_t len;\n    {record} data[{capacity}];\n}} {ty};"
        );
        BoundedList {
            empty: format!("({ty}){{ 0 }}"),
            ty,
            def: Some(def),
        }
    }
    // A buffer of its bound with the count of what it holds, named by the element
    // and the bound so that two variables of one shape share a type, and declared
    // under a guard so that two machines in one program do. The fields are the
    // library's views' (`.data`, `.len`), so `len(x)` and `x[i]` lower over a list
    // as they do over an algorithm's parameter.
    fn bounded_list(&self, elem: &SceType, capacity: Option<u32>) -> BoundedList {
        let capacity = capacity.expect("a C11 list has the bound `unsupported` asked of it");
        let ty = format!("sce_static_list_{}_{capacity}_t", elem.as_attr());
        let guard = ty.to_uppercase().trim_end_matches("_T").to_string();
        let def = format!(
            "#ifndef {guard}\n#define {guard}\n\
             /* SCE Accepted Subset §2.15: a `list<{elem}>` of at most {capacity}. */\n\
             typedef struct {{\n    size_t len;\n    {} data[{capacity}];\n}} {ty};\n#endif",
            crate::forge::generator::c_type(elem),
            elem = elem.as_attr()
        );
        BoundedList {
            empty: format!("({ty}){{ 0 }}"),
            ty,
            def: Some(def),
        }
    }
    // A copy of the list the loop began with is walked, by a count of its own
    // (§scxml-4.6). The outer `for` is how a statement declares the copy in a
    // head that is one statement: it runs its body once.
    fn foreach_loop_typed(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
        item_ty: &str,
        list_ty: &str,
    ) -> Option<(String, String)> {
        let n = self
            .loops
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let head = format!(
            "for ({list_ty} sce_list_{n}_ = {list}, *sce_once_{n}_ = &sce_list_{n}_; \
             sce_once_{n}_ != NULL; sce_once_{n}_ = NULL) \
             for (size_t sce_at_{n}_ = 0; sce_at_{n}_ < sce_list_{n}_.len; ++sce_at_{n}_)"
        );
        let mut prologue =
            format!("{item_ty} {item} = sce_list_{n}_.data[sce_at_{n}_]; (void){item};");
        if let Some(index) = index {
            prologue.push_str(&format!(
                " uint32_t {index} = (uint32_t)sce_at_{n}_; (void){index};"
            ));
        }
        Some((head, prologue))
    }
    // A C string is a pointer to bytes, so the bound is `strlen`'s: the runtime's
    // helper answers the value, or "" with the failure recorded, which the
    // statement around it receives as it does a checked operation.
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("sce_forge_bounded_string(&sce_failure_, {value}, {capacity}u)")
    }
    // A byte string is a view, so the bound is its length: the runtime's helper
    // answers the view, or an empty one with the failure recorded.
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        format!("sce_forge_bounded_bytes(&sce_failure_, {value}, {capacity}u)")
    }
    // A buffer of the bound and the length it holds, named by the bound so that
    // two variables of one bound share a type, and declared under a guard so that
    // two machines in one program do. It has the shape of a list's view
    // (`{data, len}`), so an expression reads its length and an element as it
    // does a list's, and a host is lent the view of it
    // (`sce_forge_bytes_view_t`) built by the reader. `_of` fills it from the view
    // of a value already held to the bound, which a literal's is.
    fn bytes_storage(&self, capacity: u32, init: &str) -> Option<StringStorage> {
        let ty = c_bytes_storage_type(capacity);
        let guard = ty.to_uppercase().trim_end_matches("_T").to_string();
        let of = c_bytes_storage_of(capacity);
        let def = format!(
            "#ifndef {guard}\n#define {guard}\n\
             /* SCE Accepted Subset §2.15: a `bytes` of at most {capacity} bytes. */\n\
             typedef struct {{\n    uint8_t data[{capacity}];\n    size_t len;\n}} {ty};\n\n\
             static inline {ty} {of}(sce_forge_bytes_view_t bytes) {{\n    \
             {ty} value = {{ {{ 0 }}, 0 }};\n    \
             for (size_t i = 0; i < bytes.len && i < {capacity}u; ++i) {{\n        \
             value.data[i] = bytes.data[i];\n    }}\n    \
             value.len = bytes.len < {capacity}u ? bytes.len : {capacity}u;\n    return value;\n}}\n#endif"
        );
        Some(StringStorage {
            init: format!("{of}({init})"),
            ty,
            def,
            view: "sce_forge_bytes_view_t".to_string(),
        })
    }
    // The bytes and their length move together into the buffer, which the value
    // may overlap (`frame = frame`), and the bound was judged before this runs.
    fn assign_bytes(&self, target: &str, value: &str) -> String {
        format!("memmove(({target}).data, ({value}).data, ({value}).len); ({target}).len = ({value}).len;")
    }
    // A record's byte-string field is the buffer of its bound and its length, which
    // an expression reads as a variable's is, and which is written as one is.
    fn record_bytes_literal(&self, bound: u32, value: &str) -> String {
        format!("{}({value})", c_bytes_storage_of(bound))
    }
    fn assign_bytes_field(&self, target: &str, field: &str, value: &str) -> String {
        self.assign_bytes(&format!("{target}.{field}"), value)
    }
    // The buffer a bytes field of a record is made of, filled from the view of the
    // bytes a payload carried, which were held to the bound.
    fn record_bytes_runtime(&self, bound: u32, bytes: &str) -> String {
        format!("{}({bytes})", c_bytes_storage_of(bound))
    }
    // A payload holds a byte string as an array of the schema's bound and the
    // length beside it (`build_c11_event_payload`); an expression reads the two as
    // the view a byte string held by the machine is read as.
    fn payload_bytes_read(&self, accessor: &str, field: &str) -> Option<String> {
        Some(format!(
            "(sce_forge_bytes_view_t){{ {accessor}.{field}, {accessor}.{field}_len }}"
        ))
    }
    // A buffer of the bound and its terminator, named by the bound so that two
    // variables of one bound share a type, and declared under a guard so that
    // two machines in one program do. The text is the buffer's `data`, which is
    // what an expression reads and a reader answers ([`Self::variable_ref`]).
    fn string_storage(&self, capacity: u32, init: &str) -> Option<StringStorage> {
        let ty = c_string_storage_type(capacity);
        let def = c_string_storage_def(capacity);
        Some(StringStorage {
            init: format!("({ty}){{ {init} }}"),
            ty,
            def,
            view: "const char *".to_string(),
        })
    }
    fn variable_ref(&self, var: &crate::model::Variable, field_ref: String) -> String {
        let held = var
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::scalar);
        if matches!(held, Some(SceType::String)) {
            format!("{field_ref}.data")
        } else {
            field_ref
        }
    }
    // The text and its terminator move together into the buffer, which the value
    // may overlap (`title = title`), and the bound was judged before this runs.
    fn assign_string(&self, target: &str, value: &str) -> String {
        format!("memmove({target}, {value}, strlen({value}) + 1u);")
    }
    // A record's string field is the buffer of its bound, which an expression
    // reads as the buffer's text, as it reads a string variable's.
    fn record_string_read(&self, record: &str, field: &str) -> Option<String> {
        Some(format!("{record}.{field}.data"))
    }
    fn assign_string_field(&self, target: &str, field: &str, value: &str) -> String {
        self.assign_string(&format!("{target}.{field}.data"), value)
    }
    // The buffer a literal fills, as a variable's does ([`Self::string_storage`]).
    fn record_string_literal(&self, bound: u32, literal: &str) -> String {
        format!("({}){{ {literal} }}", c_string_storage_type(bound))
    }
    // The text is the bound's own helper's answer, which fits, so the buffer is
    // filled from it.
    fn record_string_runtime(&self, bound: u32, text: &str) -> String {
        format!("{}({text})", c_string_storage_of(bound))
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
    }
    // The list an algorithm returned is its result struct, read from `items` and
    // `len`; the machine's own list is the library's `{len, data[bound]}`, which the
    // algorithm's own `_within` has held to the variable's bound.
    fn list_within(&self, value: &str, capacity: u32, symbol: &str, _elem: &SceType) -> String {
        format!("{symbol}_within(&sce_failure_, {value}, {capacity}u)")
    }
    fn assign_list(&self, target: &str, value: &str) -> String {
        format!(
            "memcpy({target}.data, {value}.items, {value}.len * sizeof({target}.data[0])); \
             {target}.len = {value}.len;"
        )
    }
    fn list_result_type(&self, symbol: &str) -> Option<String> {
        Some(format!("{symbol}_result_t"))
    }
    // The count and the buffer the id is formatted into are fields of the
    // machine (`SCXMLModel::needs_auto_send_id`), and the runtime's helper
    // answers the buffer.
    fn fresh_send_id(&self) -> String {
        "sce_next_auto_send_id(&sm->auto_send_seq, sm->auto_send_id)".to_string()
    }
    // Generated once, into a local: the copy into the variable's buffer reads
    // its value twice (`memmove` and `strlen`), and a count taken twice is two
    // ids.
    fn assign_fresh_send_id(&self, target: &str, is_string: bool) -> String {
        let local = "sce_fresh_id_";
        let store = if is_string {
            self.assign_string(target, local)
        } else {
            self.assign(target, local)
        };
        format!(
            "{{ const char *{local} = {}; {store} }}",
            self.fresh_send_id()
        )
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value};")
    }
    // To stderr, as the machine's other `<log>` is. The value is widened to the
    // type its conversion names, because C has no `fmt` that picks one: a
    // signed integer to `long long`, an unsigned one to `unsigned long long`, a
    // bool to its word. The label is an argument, not part of the format, so a
    // `%` in it is text.
    fn log(&self, label: &str, value: &str, ty: InferredType) -> String {
        // An optional value is logged as its present value, and as `null` where
        // it is absent, as on every engine (docs/adr/0055, section 2).
        if let InferredType::Optional(elem) = ty {
            let held = format!("({value})");
            let present = self.log(
                label,
                &crate::forge::expr::c_present_read(&held, ty),
                elem.present(),
            );
            let absent = if label.is_empty() {
                "(void)fprintf(stderr, \"null\\n\");".to_string()
            } else {
                format!(
                    "(void)fprintf(stderr, \"%s: null\\n\", \"{}\");",
                    filters::escape_c(label.to_string())
                )
            };
            return format!("if ({held}.present) {{ {present} }} else {{ {absent} }}");
        }
        let (conversion, argument) = match ty {
            InferredType::Bool => ("%s", format!("({value}) ? \"true\" : \"false\"")),
            InferredType::Str => ("%s", value.to_string()),
            InferredType::Int { signed: false, .. } => {
                ("%llu", format!("(unsigned long long)({value})"))
            }
            InferredType::Float { .. } | InferredType::UntypedFloat => {
                ("%g", format!("(double)({value})"))
            }
            // A signed integer, and an integer no context typed.
            _ => ("%lld", format!("(long long)({value})")),
        };
        if label.is_empty() {
            format!("(void)fprintf(stderr, \"{conversion}\\n\", {argument});")
        } else {
            format!(
                "(void)fprintf(stderr, \"%s: {conversion}\\n\", \"{}\", {argument});",
                filters::escape_c(label.to_string())
            )
        }
    }
    // The value is computed into the slot the append would fill, which nothing
    // reads until the count moves past it, so a value that failed leaves the list
    // as it was; the bound is judged first, so a full list computes nothing.
    // Either failure raises and ends the block the element stands in (§scxml-4.9)
    // with the `return` every block's function ends on, so the statement spells
    // its own end and the dispatcher has nothing to act on.
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        let write = format!("{target}.data[{target}.len] = {value};");
        let full = format!("if ({target}.len >= {capacity}u) {{ {overflow} return; }}");
        if value_can_fail {
            format!(
                "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {full} {write} \
                 if (sce_failure_.failed) {{ {failed} return; }} {target}.len++; }}"
            )
        } else {
            format!("{{ {full} {write} {target}.len++; }}")
        }
    }
    fn clear(&self, target: &str) -> String {
        format!("{target}.len = 0;")
    }
    // `machine` is the symbol every name of the machine starts with, prefix
    // included, which the generator hands the walk for it.
    fn raise_execution_error(&self, machine: &str, message: &str) -> String {
        format!(
            "{machine}_raise_platform_error(sm, {}_EVENT_ERROR_EXECUTION, \"{}\");",
            machine.to_uppercase(),
            filters::escape_c(message.to_string())
        )
    }
    // The runtime's checked helpers record a failure in `sce_failure_` and
    // answer a zero, so a statement that wrote its value would write that zero.
    // The statement runs in a block of its own, and a failure ends the block
    // the element stands in by returning from its function.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {statement} \
             if (sce_failure_.failed) {{ {failed} return; }} }}"
        )
    }
    fn receiving_write(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        ty: InferredType,
        failed: &str,
    ) -> String {
        // An optional value's type carries a bound or a document its inferred type
        // does not (docs/adr/0055), so its local is declared of the value's own
        // type, as a failed index read of an aggregate names its type.
        let local = if ty.is_optional() {
            format!("__typeof__({value})")
        } else {
            Self::value_type(ty)
        };
        format!(
            "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {local} sce_value = {value}; \
             if (sce_failure_.failed) {{ {failed} return; }} {} }}",
            write("sce_value")
        )
    }
    // A record made whole is computed into a local of its own type, which a C
    // local must name.
    fn receiving_write_of(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        held: &str,
        failed: &str,
    ) -> String {
        format!(
            "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {held} sce_value = {value}; \
             if (sce_failure_.failed) {{ {failed} return; }} {} }}",
            write("sce_value")
        )
    }
    // Only a host call's arguments reach here, and its one spelling is
    // [`Self::receiving_host_call`].
    fn receiving_call(&self, _statement: &str, _failed: &str) -> String {
        unreachable!("a C host call is received through receiving_host_call")
    }
    // A failed checked operation is a value, so the call could not be stopped
    // after its arguments were evaluated: each is computed into a local of its
    // declared type first, and the host is called only when none of them
    // failed. A block of its own, so two calls in one scope do not declare the
    // same locals. The block does not end: a host action's failed argument
    // costs the call and not the statements after it, as on every backend.
    // `callee` is the vtable member, which is handed the host's own state first.
    fn receiving_host_call(
        &self,
        _statement: &str,
        callee: &str,
        args: &[String],
        arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        // An argument that may be absent is held in the struct `sce/forge/optional.h`
        // declares for its element (docs/adr/0055, step 10), the type the host's
        // member takes it in.
        let locals: String = args
            .iter()
            .zip(arg_types)
            .enumerate()
            .map(|(i, (arg, ty))| {
                let held = match ty {
                    FieldType::Scalar(scalar) => {
                        crate::forge::generator::c_type(scalar).to_string()
                    }
                    FieldType::Optional { .. } => {
                        crate::forge::generator::host_param_type(Language::C11, ty)
                    }
                };
                format!("{held} sce_arg{i} = {arg}; ")
            })
            .collect();
        let names: String = (0..args.len()).map(|i| format!(", sce_arg{i}")).collect();
        format!(
            "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {locals}\
             if (sce_failure_.failed) {{ {failed} }} else {{ {callee}(sm->actions.user_data{names}); }} }}"
        )
    }
    // The head of the `if`: the verdict is `false` when the condition failed,
    // and its local is the walk's own, never declared twice in one scope.
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        let n = self
            .conditions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        format!(
            "bool sce_cond_{n}_ = false; \
             {{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; bool sce_value = {value}; \
             if (sce_failure_.failed) {{ {failed} {flag} }} else {{ sce_cond_{n}_ = sce_value; }} }} \
             if (sce_cond_{n}_)"
        )
    }
    // The local the `<if>` declares for it.
    fn condition_failed_flag(&self, _if_ordinal: u32) -> String {
        "_if_cond_failed = true;".to_string()
    }
    // An enum field of the payload is held in the machine's own enum, declared
    // in its header before the channel, lifted from the variant's declared name
    // and written back as it (`build_c11_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The send template reads the string it computes as a CSS2 time
    // (`sce_parse_delay_ms`). A string C has no storage to build — a
    // concatenation — is refused where the expression is lowered.
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    // The send template names the event it delivers by the string it computes,
    // held by a variable or written out; one that joins text is refused for the
    // reason a delay is.
    fn lowers_event_expr(&self) -> bool {
        true
    }
    // The cancel template hands the scheduler the id it computes
    // (`scheduled_cancel`), held by a variable or written out; one that joins
    // text is refused for the reason a delay is.
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // The send template writes the value as the event's JSON and as the text a
    // host takes (`sce_forge_wire_json`, `sce_forge_wire_text`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The host invoke hands the host the `src` it computes (`_host_inv.src`),
    // held by a variable or written out; one that joins text is refused for the
    // reason a delay is.
    fn lowers_host_src_expr(&self) -> bool {
        true
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`sce_document_stem`) and starts the candidate it names in two steps,
    // handing it the values it keeps between them (`seed_static_child`). A
    // string joined from text is refused where it is lowered, as a delay is.
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    // The member of the channel's union the event's payload is lifted into
    // (`build_c11_event_payload`), whose fields carry the schema's own ids.
    fn payload_accessor(&self, event: &str) -> String {
        format!(
            "sm->pending_payload.as.{}",
            filters::to_c11_lower_ident(std::borrow::Cow::Borrowed(event))
        )
    }
    // The tag says which event's payload the channel holds now, so a guard that
    // reads this event's fields holds only for a delivery that carried them
    // (`_PAYLOAD_NONE` for one that did not, or whose payload could not be
    // read). The channel's constants are spelled from the machine's name
    // without the suite prefix.
    fn payload_guard(&self, _machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "sm->pending_payload.tag == {}_PAYLOAD_{} && ({lowered})",
            self.stem.to_uppercase(),
            filters::to_c11_upper_ident(std::borrow::Cow::Borrowed(event))
        )
    }
    // A failing condition is statements here, so the check wraps the value.
    fn payload_guards_the_value(&self) -> bool {
        true
    }
    // The runtime's typed wire value (`sce/forge/wire.h`), which a pair's JSON
    // is written from: a number crosses as the widest of its signedness, which
    // is exact; a 64-bit real is written as ECMAScript spells it
    // (ARCHITECTURE.md, "JSON Number Text"). A 32-bit real is not spelled
    // ([`Self::wire_admits`]).
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!("sce_forge_wire_bool({value})"),
            InferredType::Str => format!("sce_forge_wire_string({value})"),
            // A 64-bit integer crosses as the text of its digits: the writer prints
            // it quoted from the value it holds.
            InferredType::Int {
                signed: true,
                bits: 64,
            } => format!("sce_forge_wire_wide_int((int64_t)({value}))"),
            InferredType::Int { bits: 64, .. } => {
                format!("sce_forge_wire_wide_uint((uint64_t)({value}))")
            }
            InferredType::Int { signed: true, .. } | InferredType::UntypedInt => {
                format!("sce_forge_wire_int((int64_t)({value}))")
            }
            InferredType::Int { signed: false, .. } => {
                format!("sce_forge_wire_uint((uint64_t)({value}))")
            }
            // A 32-bit real crosses as the 64-bit real it widens to, which is
            // exact: the one spelling every engine can make of it
            // (docs/SCE_ACCEPTED_SUBSET.md §2.15, "A 32-bit real").
            InferredType::Float { .. } => format!("sce_forge_wire_real((double)({value}))"),
            other => unreachable!("a C11 wire value of {other:?} is refused by `wire_admits`"),
        }
    }
    // A real of either width: the 32-bit one is written as the double it widens
    // to, so the contract that pins the 64-bit spelling pins it.
    fn wire_admits(&self, ty: InferredType) -> bool {
        matches!(
            ty,
            InferredType::Bool
                | InferredType::Str
                | InferredType::Int { .. }
                | InferredType::Float { .. }
        )
    }
    // The enum's own function answers the name its document declares
    // ([`StaticTarget::enum_def`]): a `const char *` of static storage, which
    // the wire string borrows.
    fn enum_wire_name(&self, alias: &str, value: &str) -> Option<String> {
        let prefix =
            crate::forge::enum_naming::c11_function_prefix(&self.enum_document_name(alias));
        Some(format!("{prefix}_declared_name({value})"))
    }
    // A byte string is the view of its buffer and length, which the runtime's wire
    // value holds with the length: a string is NUL-terminated and a byte string may
    // hold a 0x00 (`sce/forge/wire.h`).
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(format!("sce_forge_wire_bytes({value})"))
    }
    // The one struct `sce/forge/optional.h` declares for each element
    // (docs/adr/0055, section 5), named by the type the present value is held in.
    fn optional_type(&self, present: &str) -> Option<String> {
        c11_optional_type(present)
    }
    fn optional_none(&self, present: &str) -> Option<String> {
        Some(format!(
            "sce_optional_{}_none()",
            c11_optional_elem(present)?
        ))
    }
    fn optional_some(&self, present: &str, value: &str) -> Option<String> {
        Some(format!(
            "sce_optional_{}_some({value})",
            c11_optional_elem(present)?
        ))
    }
    // A number's and a `bool`'s are the runtime's own. A text's and a byte
    // string's are declared beside the buffer of the bound, which is declared
    // first under its own guard however many places hold one, and an enum's under
    // the guard of its document, after the enum: two machines of one program
    // that hold the same one declare it once.
    fn optional_def(&self, present: &str, elem: &SceType) -> Option<String> {
        let name = c11_optional_elem(present)?;
        let bound = |prefix: &str| -> Option<u32> {
            present
                .strip_prefix(prefix)?
                .strip_suffix("_t")?
                .parse()
                .ok()
        };
        let (storage, declaration) = match elem {
            SceType::String => {
                let bound = bound("sce_static_string_")?;
                (
                    c_string_storage_def(bound),
                    format!("SCE_OPTIONAL_DECLARE_STRING({bound})"),
                )
            }
            SceType::Bytes => {
                let bound = bound("sce_static_bytes_")?;
                (
                    self.bytes_storage(bound, "{ 0 }")?.def,
                    format!("SCE_OPTIONAL_DECLARE_BYTES({bound})"),
                )
            }
            SceType::Enum(_) => return Some(c11_optional_enum_declaration(&name, present)),
            _ => return None,
        };
        Some(c11_optional_buffer_declaration(
            &name,
            present,
            &storage,
            &declaration,
        ))
    }
    fn optional_import(&self) -> Option<String> {
        Some(C11_OPTIONAL_INCLUDE.to_string())
    }
    // A record's own: the machine declares the type of the record in its header, so
    // the optional value of it is declared beside it, once however many variables
    // hold one (docs/adr/0055, step 9).
    fn optional_record_def(&self, present: &str) -> Option<String> {
        Some(c11_optional_record_declaration(
            &c11_optional_elem(present)?,
            present,
        ))
    }
    // The record is the struct `value` holds, so a field is written through it, as a
    // record variable's is: the flag is not touched, and the build proved it set.
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(self.assign_field(&format!("{target}.value"), field, value))
    }
    fn assign_present_string_field(
        &self,
        target: &str,
        field: &str,
        value: &str,
    ) -> Option<String> {
        Some(self.assign_string_field(&format!("{target}.value"), field, value))
    }
    fn assign_present_bytes_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(self.assign_bytes_field(&format!("{target}.value"), field, value))
    }
    // The element of a list of records, whole, as a checked index read: the
    // failure is recorded and the read gives a zero record when the index is
    // outside the list, which the statement around it never uses
    // ([`StaticTarget::receiving_write_of`]). The index is read for its range and
    // for the element, which an expression writes nothing between.
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!(
            "(sce_forge_checked_index_from_i(&sce_failure_, (int64_t)({index}), ({list}).len) \
             ? ({list}).data[{index}] : (__typeof__(({list}).data[0])){{0}})"
        ))
    }
    // C has no closure: the present value is mapped under a test of the flag,
    // which reads the value twice, as `??` does (`sce/forge/optional.h`).
    fn optional_map(
        &self,
        value: &str,
        present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        let name = c11_optional_elem(present)?;
        let held = format!("({value})");
        let read = crate::forge::expr::c_present_read(&held, c11_present_kind(present));
        Some(format!(
            "({held}.present ? sce_optional_{name}_some({}) : sce_optional_{name}_none())",
            map(&read)
        ))
    }
    // `null` where the value is absent, and the wire value of its present value
    // where it is present: the writer writes the key either way.
    fn wire_optional(
        &self,
        value: &str,
        present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        let held = format!("({value})");
        let read = crate::forge::expr::c_present_read(&held, c11_present_kind(present));
        Some(format!(
            "({held}.present ? {} : sce_forge_wire_null())",
            wire(&read)
        ))
    }
}

/// The line a C11 unit that holds an optional value (docs/adr/0055) includes the
/// runtime's optional type by: the machine's header, whose policy and records hold
/// one, and its source.
pub(crate) const C11_OPTIONAL_INCLUDE: &str = "#include <sce/forge/optional.h>";

/// The name `sce/forge/optional.h` declares the optional value whose present
/// value is held in the C type `present` under (`sce_optional_<name>_t`,
/// docs/adr/0055): the type the document writes for a number or a `bool`
/// (`int64`, `float64`, `bool`), `string_<N>` and `bytes_<N>` over the buffer of
/// a bound, `enum_<document>` over the machine's type for an enum
/// ([`StaticTarget::enum_type`]), and `record_<type>` over the struct type of a
/// record, the machine's own ([`StaticTarget::record_type`]) or the algorithm's
/// ([`crate::forge::generator::event_schema_payload_type`]), in the one name its
/// type gives it (step 9). `None` for a type no optional value is held over: a
/// text or a byte string with no bound, whose present value has no buffer.
pub(crate) fn c11_optional_elem(present: &str) -> Option<String> {
    if let Some(bound) = present
        .strip_prefix("sce_static_string_")
        .and_then(|rest| rest.strip_suffix("_t"))
    {
        return Some(format!("string_{bound}"));
    }
    if let Some(bound) = present
        .strip_prefix("sce_static_bytes_")
        .and_then(|rest| rest.strip_suffix("_t"))
    {
        return Some(format!("bytes_{bound}"));
    }
    if let Some(document) = present.strip_suffix("_enum_t") {
        return Some(format!("enum_{document}"));
    }
    let scalar = match present {
        "bool" => "bool",
        "int8_t" => "int8",
        "int16_t" => "int16",
        "int32_t" => "int32",
        "int64_t" => "int64",
        "uint8_t" => "uint8",
        "uint16_t" => "uint16",
        "uint32_t" => "uint32",
        "uint64_t" => "uint64",
        "float" => "float32",
        "double" => "float64",
        // A struct type of a record: the name its own type gives it, whose `_t` is
        // the one the optional value's ends in.
        other => {
            return other
                .strip_suffix("_t")
                .filter(|record| {
                    !record.is_empty()
                        && record
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                })
                .map(|record| format!("record_{record}"));
        }
    };
    Some(scalar.to_string())
}

/// The declaration of the optional value of a text or a byte string held in the
/// buffer type `present`, named `name` ([`c11_optional_elem`]: `string_<N>`,
/// `bytes_<N>`), under a guard of its own: `declaration` is the runtime's macro
/// for it (`SCE_OPTIONAL_DECLARE_STRING(N)`), and `storage` the declaration of the
/// buffer, written first inside the guard by the unit that declares it nowhere
/// else — empty where the unit declared the buffer before, as a schema does.
pub(crate) fn c11_optional_buffer_declaration(
    name: &str,
    present: &str,
    storage: &str,
    declaration: &str,
) -> String {
    let guard = format!("SCE_OPTIONAL_{}", name.to_uppercase());
    let storage = if storage.is_empty() {
        String::new()
    } else {
        format!("{storage}\n")
    };
    format!(
        "#ifndef {guard}\n#define {guard}\n{storage}\
         /* docs/adr/0055: a value of `{present}` that may be absent. */\n\
         {declaration}\n#endif"
    )
}

/// The declaration of the optional value of the enum whose type is `present`, named
/// `name` ([`c11_optional_elem`]: `enum_<document>`), under the guard of the enum
/// document, after the enum itself: the one text a machine that holds a value of
/// the enum and an algorithm or a schema that holds one both write, so that a
/// program that includes them declares it once.
pub(crate) fn c11_optional_enum_declaration(name: &str, present: &str) -> String {
    let guard = format!("SCE_OPTIONAL_{}", name.to_uppercase());
    format!(
        "#ifndef {guard}\n#define {guard}\n\
         /* docs/adr/0055: a value of `{present}` that may be absent. */\n\
         SCE_OPTIONAL_DECLARE({name}, {present}, SCE_OPTIONAL_SAME_VALUE)\n#endif"
    )
}

/// The declaration of the optional value of the record whose struct type is
/// `present`, named `name` ([`c11_optional_elem`]), under the guard that makes a
/// program that includes two units holding it declare it once (docs/adr/0055,
/// step 9): the machine writes it beside its own record type, and an algorithm
/// that holds an optional record beside the schema's type it includes. A record
/// has no `_eq`: two of them are no operand of `===`.
pub(crate) fn c11_optional_record_declaration(name: &str, present: &str) -> String {
    let guard = format!("SCE_OPTIONAL_{}", name.to_uppercase());
    format!(
        "#ifndef {guard}\n#define {guard}\n\
         /* docs/adr/0055: a record of `{present}` that may be absent. */\n\
         SCE_OPTIONAL_DECLARE_RECORD({name}, {present})\n#endif"
    )
}

/// The C11 type of an optional value whose present value is held in `present`
/// ([`c11_optional_elem`]): the one spelling the policy, a record, a payload and
/// a host's reader name it by.
pub(crate) fn c11_optional_type(present: &str) -> Option<String> {
    c11_optional_elem(present).map(|name| format!("sce_optional_{name}_t"))
}

/// The declaration a C11 unit that holds the optional text or byte string `elem`,
/// held to `bound` bytes, makes of it (docs/adr/0055): the buffer of the bound
/// and the optional value over it, each under the guard every unit that holds one
/// shares, so a program that includes two declares them once. A host-run
/// record's result is such a unit, as a `sce-static` machine is. `None` for an
/// element the runtime's own header declares an optional value of, or that has
/// no buffer.
pub(crate) fn c11_optional_buffer_def(elem: &SceType, bound: u32) -> Option<String> {
    let target = CTarget::arguments();
    let present = optional_present(&target, "", elem, Some(bound));
    target.optional_def(&present, elem)
}

/// The type of an optional value whose present value is held in the C type
/// `present`, as far as how it is read tells them apart
/// ([`crate::forge::expr::c_present_read`]): an optional text — over a buffer, or
/// over the pointer a wire value takes — read as its text, an optional byte
/// string read as its view, and `Unknown` for any other, read as itself.
fn c11_present_kind(present: &str) -> InferredType {
    use crate::forge::types::OptionalElem;
    if present.starts_with("sce_static_string_") || present == "const char *" {
        InferredType::Optional(OptionalElem::Str)
    } else if present.starts_with("sce_static_bytes_") || present == "const uint8_t *" {
        InferredType::Optional(OptionalElem::Bytes)
    } else {
        InferredType::Unknown
    }
}

/// Rewrite `model` — a clone the C11 backend renders — so every expression of
/// a `sce-static` document is native C. `csym_prefix` is the suite symbol
/// prefix (`"<prefix>_"`, empty when unset) every symbol of the machine starts
/// with.
pub fn lower_c11(
    model: &mut SCXMLModel,
    csym_prefix: &str,
) -> Result<StaticLowering, GenerateError> {
    let stem = model.name.clone();
    let target = CTarget::new(&stem, &model.held_enums());
    lower(model, &format!("{csym_prefix}{stem}"), &target)
}

/// Swift: a variable is a stored property of the machine's policy class, set by
/// the machine's own statements.
///
/// The failure of a checked operation is a value, as in Go and C++: a helper of
/// `Checked` records it in the statement's `sceFailure` and yields zero, so a value
/// that can fail is computed into a local first and written only when it did not.
/// The statement is a closure called where it stands, answering whether it failed,
/// which the dispatcher acts on by leaving the labelled block it stands in
/// (§scxml-4.9).
///
/// Lowers scalar variables, guards, `<assign>`, `<if>`, `<log>`, `<raise>`, `In()`
/// and a call of an imported algorithm of scalars; every construct past those is
/// refused by name
/// ([`StaticTarget::unsupported`]) until its spelling is written, rather than left
/// as an undefined name in generated code.
pub struct SwiftTarget;

impl SwiftTarget {
    /// `parts` as the statements of one block, the empty ones left out — what runs
    /// on a failure may be nothing, when the document declares no `error.execution`
    /// to raise.
    fn then(parts: &[&str]) -> String {
        parts
            .iter()
            .map(|part| part.trim().trim_end_matches(';').trim())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// The first action of `actions`, or of a block nested in one, that this
    /// target has no lowering for yet.
    fn unlowered_action(actions: &[Action]) -> Option<String> {
        for action in actions {
            match action.action_type.as_str() {
                "assign" | "log" | "if" | "raise" | "cancel" => {}
                // A list is filled, emptied and walked by native statements.
                "sce_append" | "sce_clear" | "foreach" => {}
                // A call of the machine's host, lowered ahead of this walk by
                // `native_action::render`; an argument that reads the event's data is
                // refused before it (`first_swift_unlowered_host_argument`).
                "native_action" => {}
                "send" => {
                    if let Some(what) = Self::unlowered_send(action) {
                        return Some(what);
                    }
                }
                other => return Some(format!("<{}>", other.replace('_', ":"))),
            }
            for block in action.nested_blocks() {
                if let Some(found) = Self::unlowered_action(block.actions) {
                    return Some(found);
                }
            }
        }
        None
    }
}

impl SwiftTarget {
    /// What of `send` this target has no lowering for yet, described for a refusal:
    /// a processor but the SCXML one and BasicHTTP, and a target but the machine's own
    /// queues and its parent.
    fn unlowered_send(send: &Action) -> Option<String> {
        let scxml_processor = |name: &str| {
            name == crate::host_processor_analyzer::SCXML_EVENT_PROCESSOR_TYPE || name == "scxml"
        };
        if !send.send_type.is_empty() && !scxml_processor(&send.send_type) {
            if send.send_type.contains("BasicHTTPEventProcessor") && send.types.is_empty() {
                return Self::unlowered_http_send(send);
            }
            return Some(format!("a <send type=\"{}\">", send.send_type));
        }
        if let Some(other) = send.types.iter().find(|t| !scxml_processor(t)) {
            return Some(format!(
                "a <send> that may take the processor type \"{other}\""
            ));
        }
        // The machine's own external queue, its internal queue, itself by its session
        // id, and its parent: `#_<invokeid>`, a URL and a Mesh peer are other
        // transports.
        let own = |target: &str| {
            target.is_empty()
                || target == "#_internal"
                || target == "#_parent"
                || target.starts_with("#_scxml_")
        };
        // A `targetexpr` is held to the routes the document declares (`sce:targets`).
        std::iter::once(send.target.as_str())
            .chain(send.targets.iter().map(String::as_str))
            .find(|target| !own(target))
            .map(|target| format!("a <send target=\"{target}\">"))
    }
}

impl SwiftTarget {
    /// What of a BasicHTTP `<send>` (§scxml-C-2) this target has no lowering for yet.
    ///
    /// The send that is lowered posts at once to a URL the document writes, with the
    /// `<param>`s it names as the form. A delay needs a scheduler of HTTP requests the
    /// Swift engine has not got, a `<content>` is a body of another kind, and a target,
    /// event or type an expression computes is a request whose address is known only
    /// when it runs.
    fn unlowered_http_send(send: &Action) -> Option<String> {
        if send.target.is_empty() || !send.targetexpr.is_empty() {
            return Some("a BasicHTTP <send> whose target is not a URL the document writes".into());
        }
        if !send.delay.is_empty() || !send.delayexpr.is_empty() {
            return Some("a delayed BasicHTTP <send>".into());
        }
        if !send.content.is_empty() || !send.contentexpr.is_empty() {
            return Some("a BasicHTTP <send> with <content>".into());
        }
        if !send.eventexpr.is_empty() || !send.typeexpr.is_empty() {
            return Some("a BasicHTTP <send> whose event or type an expression computes".into());
        }
        if !send.idlocation.is_empty() {
            return Some("a BasicHTTP <send idlocation>".into());
        }
        None
    }
}

impl StaticTarget for SwiftTarget {
    fn name(&self) -> &'static str {
        "Swift"
    }
    // The send template reads the string a `delayexpr`, an `eventexpr` and a
    // `sendidexpr` compute as a CSS2 time, an event name and a send id
    // (`parseDelayToMs`, `Engine.sendNamedExternal`, `Engine.cancelEvent`).
    fn lowers_delay_expr(&self) -> bool {
        true
    }
    fn lowers_event_expr(&self) -> bool {
        true
    }
    fn lowers_cancel_expr(&self) -> bool {
        true
    }
    // Every kind that is a function of its inputs (an algorithm, a condition, a lookup,
    // a transform, an interpolation, a validator) is a static member of the caseless
    // enum its unit declares, in the machine's own module. A document the Swift unit
    // does not lower (in part, [`crate::forge::codegen_matrix::ships_in_part`]) is
    // refused where it is imported, `StaticCallee::unlowered_by`.
    fn callee(&self, callee: &StaticCallee) -> Option<Callee> {
        Some(generated_callee(Language::Swift, callee, None))
    }
    // A call that hands an algorithm a record, or takes one back, goes through an
    // adapter of the machine's file: the machine holds a record in a struct of its own
    // (`record_type`) and the algorithm in the one its schema's unit declares, and the
    // adapter copies one into the other by field.
    fn lowers_state_calls(&self) -> bool {
        true
    }
    fn state_argument(&self, _machine: &str, _arg: &StateArg, held: &str) -> String {
        held.to_string()
    }
    // A function of the file, `private` to it: two machines of one module that call
    // one algorithm the same way each declare theirs, in their own types.
    fn state_adapter_name(&self, _machine: &str, _callee: &StaticCallee, coded: &str) -> String {
        format!("{}{coded}", crate::forge::expr::SWIFT_STATE_ADAPTER_PREFIX)
    }
    // `T?` (docs/adr/0055, section 5): the type is the default spelling
    // ([`crate::forge::generator::optional_of`]), absent is `nil` and a present value
    // is the `T` itself, which Swift lifts into a `T?` wherever one is wanted.
    fn optional_none(&self, _present: &str) -> Option<String> {
        Some("nil".to_string())
    }
    fn optional_some(&self, _present: &str, value: &str) -> Option<String> {
        Some(value.to_string())
    }
    // The element is read by the runtime's checked read of a list of values with a zero,
    // which records an index outside the list as the failure the statement ends at and
    // answers the record at its zero: the machine's record is `SceDefault`, and a list of
    // records is an `Array` of the machine's own struct, so the element is the record
    // whole, and a copy of it as a struct is ([`Self::record_copy`]).
    fn record_at(&self, list: &str, index: &str) -> Option<String> {
        Some(format!("Checked.atDefault(sceFailure, {list}, {index})"))
    }
    // A record is a struct, a value held inside the optional, so a field of the present
    // one is written by taking the record out by the one function a proven read is spelled
    // with, changing the copy and putting it back, in a block of its own so two such writes
    // of one body do not declare the copy twice. A write that has proven the value present
    // never names a value that is not there ([`SceOptional.proven`]).
    fn assign_present_field(&self, target: &str, field: &str, value: &str) -> Option<String> {
        Some(format!(
            "do {{ var sceRecord = SceOptional.proven({target}); sceRecord.{field} = {value}; \
             {target} = sceRecord }}"
        ))
    }
    // `Optional.map`, over a closure with an argument of its own: the present value is
    // bound where the value is present, and a statement that fails inside it records the
    // failure in the one `sceFailure` of the block, as every expression of it does.
    fn optional_map(
        &self,
        value: &str,
        _present: &str,
        map: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        Some(format!(
            "({value}).map {{ ({SWIFT_PRESENT}) in {} }}",
            map(SWIFT_PRESENT)
        ))
    }
    // What an adapter gave back as a list: held to the bound of the variable that
    // takes it, which a list the algorithm built may exceed (an `Array` grows).
    fn state_list_within(&self, _adapter: &str, value: &str, capacity: u32) -> String {
        format!("Checked.within(sceFailure, {value}, capacity: {capacity})")
    }
    // A function of the machine's file that takes the machine's values, builds the
    // algorithm's from them, calls it, and builds the machine's own from what it
    // returns. It spells what an algorithm over numbers, truth values, values of an
    // enum (which cross as the variant of the same declared name), records of those and
    // lists of them, handed in or taken back, is called with. An algorithm that may
    // fail is called with `try` and the adapter `throws` what it throws, which the call
    // of the adapter records (`Checked.takeDefault`). A value that may be absent
    // returns `None`, which refuses the call by name until it is spelled.
    fn state_adapter(&self, spec: &AdapterSpec<'_>) -> Option<String> {
        use crate::forge::generator::{event_schema_field_ident, event_schema_payload_type};
        let value = spec.callee.value.as_ref()?;
        let machine_record = |alias: &str| self.record_type(spec.machine, alias);
        // A record of `ty` built from `from`, a machine record, a payload or the
        // algorithm's record, field by field, by the label each initializer names it
        // under.
        let record_of =
            |alias: &str, ty: String, from: &str, crossing: Crossing| -> Option<String> {
                let schema = plain_schema(spec, alias)?;
                let fields = schema
                    .fields
                    .iter()
                    .map(|f| {
                        let field = event_schema_field_ident(&f.id, Language::Swift);
                        spec.record_field_crossing(
                            self,
                            f,
                            &format!("{from}.{field}"),
                            crossing,
                            str::to_string,
                        )
                        .map(|value| format!("{field}: {value}"))
                    })
                    .collect::<Option<Vec<String>>>()?;
                Some(format!("{ty}({})", fields.join(", ")))
            };
        let to_algorithm = |alias: &str, from: &str| -> Option<String> {
            let ty = plain_schema(spec, alias)
                .map(|schema| event_schema_payload_type(&schema.name, Language::Swift))?;
            record_of(alias, ty, from, Crossing::ToAlgorithm)
        };
        let from_algorithm = |alias: &str, from: &str| -> Option<String> {
            record_of(alias, machine_record(alias), from, Crossing::FromAlgorithm)
        };
        // The type the machine holds the record of `alias` in as the argument is held
        // as `code`: the payload struct of the event the transition is on for `p`
        // (`build_swift_event_payload`), the machine's own record type for any other.
        let held_record = |alias: &str, code: char| -> Option<String> {
            if code == 'p' {
                let event = spec.payload_events.get(alias)?;
                Some(format!(
                    "{}{}Payload",
                    spec.machine,
                    filters::to_event_variant(event.to_string())
                ))
            } else {
                Some(machine_record(alias))
            }
        };
        let mut declarations = Vec::new();
        let mut arguments = Vec::new();
        for (index, param) in spec.params.iter().enumerate() {
            let name = format!("p{index}");
            match param {
                // The machine holds a value of an enum in its own enum, and the
                // algorithm in the unit of the enum document: it crosses as the variant
                // of the same declared name.
                AdapterParam::Scalar(SceType::Enum(reference)) => {
                    declarations.push(format!(
                        "_ {name}: {}",
                        self.enum_type(spec.machine, &reference.alias)?
                    ));
                    arguments.push(spec.enum_crossing(
                        self,
                        &reference.alias,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                AdapterParam::Scalar(ty) if plain_scalar(ty) => {
                    declarations.push(format!("_ {name}: {}", self.scalar_type(ty)));
                    arguments.push(name);
                }
                // A value that may be absent, of a value of an enum: the machine's own
                // `T?`, crossing where it is present by the declared name of its variant
                // (docs/adr/0055).
                AdapterParam::Optional(ty) if matches!(ty, SceType::Enum(_)) => {
                    declarations.push(format!("_ {name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(spec.optional_enum_crossing(
                        self,
                        ty,
                        &name,
                        Crossing::ToAlgorithm,
                    )?);
                }
                // ... of a number or a `bool`: `T?` on both sides, handed on as it is.
                AdapterParam::Optional(ty) => {
                    declarations.push(format!("_ {name}: {}", spec.optional_type(self, ty)?));
                    arguments.push(name);
                }
                AdapterParam::State {
                    slot: StaticSlot::Record(alias),
                    code,
                } => {
                    declarations.push(format!("_ {name}: {}", held_record(alias, *code)?));
                    arguments.push(to_algorithm(alias, &name)?);
                }
                // ... of a record: the algorithm's struct is built where the machine's is
                // present, and an absent one stays `nil`.
                AdapterParam::State {
                    slot: StaticSlot::OptionalRecord(alias),
                    code,
                } => {
                    to_algorithm(alias, SWIFT_PRESENT)?;
                    let held = held_record(alias, *code)?;
                    declarations.push(format!("_ {name}: {}", self.optional_type(&held)?));
                    arguments.push(self.optional_map(&name, &held, &|present| {
                        to_algorithm(alias, present).unwrap_or_default()
                    })?);
                }
                // A list of numbers or truth values is an `Array` of the same element
                // on both sides, so it crosses as it is.
                AdapterParam::State {
                    slot: StaticSlot::ScalarList(ty),
                    ..
                } if plain_scalar(ty) => {
                    declarations.push(format!("_ {name}: {}", self.list_type(ty)));
                    arguments.push(name);
                }
                // A list of records is mapped element by element into the schema's
                // struct, the machine's own staying as it was.
                AdapterParam::State {
                    slot: StaticSlot::RecordList(alias),
                    ..
                } => {
                    declarations.push(format!(
                        "_ {name}: {}",
                        self.record_list_type(&machine_record(alias))
                    ));
                    arguments.push(format!("{name}.map {{ {} }}", to_algorithm(alias, "$0")?));
                }
                _ => return None,
            }
        }
        let (returned_type, returned) = match value.ret.as_ref()? {
            // The variant of the same declared name in the machine's own enum.
            StaticSlot::Scalar(SceType::Enum(reference)) => (
                self.enum_type(spec.machine, &reference.alias)?,
                spec.enum_crossing(self, &reference.alias, "r", Crossing::FromAlgorithm)?,
            ),
            StaticSlot::Scalar(ty) if plain_scalar(ty) => (self.scalar_type(ty), "r".to_string()),
            // A value that may be absent, of a value of an enum: the variant of the same
            // declared name where it is present, absent where it is absent
            // (docs/adr/0055).
            StaticSlot::Optional(ty) if matches!(ty, SceType::Enum(_)) => (
                spec.optional_type(self, ty)?,
                spec.optional_enum_crossing(self, ty, "r", Crossing::FromAlgorithm)?,
            ),
            // ... of a number or a `bool`: `T?` on both sides, handed on as it is.
            StaticSlot::Optional(ty) => (spec.optional_type(self, ty)?, "r".to_string()),
            // ... of a record: the machine's record is built where the algorithm's answer
            // is present, and an absent one stays `nil`.
            StaticSlot::OptionalRecord(alias) => {
                // Asked once for its spelling before it is bound, so a record Swift
                // cannot cross refuses the adapter by name and never writes an empty one.
                from_algorithm(alias, SWIFT_PRESENT)?;
                (
                    self.optional_type(&machine_record(alias))?,
                    self.optional_map("r", &machine_record(alias), &|present| {
                        from_algorithm(alias, present).unwrap_or_default()
                    })?,
                )
            }
            StaticSlot::Record(alias) => (machine_record(alias), from_algorithm(alias, "r")?),
            // An `Array` of numbers or truth values is the same on both sides.
            StaticSlot::ScalarList(ty) if plain_scalar(ty) => (self.list_type(ty), "r".to_string()),
            // A list of records is mapped element by element into the machine's own
            // struct, the algorithm's staying as it was.
            StaticSlot::RecordList(alias) => (
                self.record_list_type(&machine_record(alias)),
                format!("r.map {{ {} }}", from_algorithm(alias, "$0")?),
            ),
            _ => return None,
        };
        let (throws, attempt) = if spec.callee.may_fail {
            (" throws", "try ")
        } else {
            ("", "")
        };
        Some(format!(
            "private func {}({}){throws} -> {returned_type} {{\n    let r = {attempt}{}({})\n    return {returned}\n}}",
            spec.name,
            declarations.join(", "),
            spec.call,
            arguments.join(", ")
        ))
    }
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        // A scalar of a number, a bool, a string or an enum, a record, and a value that
        // may be absent over one of those; a list of byte strings is not spelled yet.
        if let Some(var) = scope.variables.iter().find(|v| {
            !v.value_type.as_ref().is_some_and(|t| {
                t.scalar().is_some()
                    // A record, and a list of numbers, truth values or records, bound by
                    // `sce:capacity`; a list of enums is not spelled yet.
                    || t.record_alias().is_some()
                    || t.list_elem().is_some_and(|elem| {
                        elem.scalar()
                            .is_none_or(|ty| !matches!(ty, SceType::Enum(_) | SceType::Bytes))
                    })
                    // `T?` over the element Swift spells ([`algorithm_optional_spelled`]).
                    || t.optional_elem().is_some_and(|elem| {
                        crate::forge::generator::algorithm_optional_spelled(Language::Swift, elem)
                    })
            })
        }) {
            return Some(format!(
                "<data id=\"{}\"> of a list of enums, of byte strings or an optional byte string",
                var.id
            ));
        }
        // A record's field is named as the author wrote it, so one Swift reserves cannot
        // be a field, and a field that may be absent is `T?` over an element Swift spells
        // ([`swift_field_unwritten`]).
        if let Some((alias, field, why)) = model
            .imported_records
            .iter()
            .flat_map(|(alias, schema)| schema.fields.iter().map(move |field| (alias, field)))
            .find_map(|(alias, field)| swift_field_unwritten(field).map(|why| (alias, field, why)))
        {
            return Some(format!(
                "the field `{}` of the record `{alias}`, {why}",
                field.id
            ));
        }
        // A variant is a member of the enum's type, named lowerCamelCase, and one Swift
        // reserves is not a member a `switch` or a leading dot can name.
        if let Some((enum_name, variant)) = model.held_enums().values().find_map(|e| {
            e.variants
                .iter()
                .find(|v| {
                    let ident =
                        crate::forge::enum_naming::variant_ident(Language::Swift, &e.name, &v.name);
                    crate::reader_names::is_reserved_word(Language::Swift, &ident)
                })
                .map(|v| (e.name.clone(), v.name.clone()))
        }) {
            return Some(format!(
                "the variant `{variant}` of the enum `{enum_name}`, a name Swift reserves"
            ));
        }
        // An event's typed payload is read through a struct of its schema's fields, each
        // named as the author wrote it: one Swift reserves cannot be a property, and a
        // field that may be absent is `T?` over an element Swift spells, the same
        // struct a record is ([`swift_field_unwritten`]).
        if let Some((schema, field, why)) = model
            .imported_event_schemas
            .values()
            .flat_map(|schema| schema.fields.iter().map(move |field| (schema, field)))
            .find_map(|(schema, field)| {
                swift_field_unwritten(field).map(|why| (schema, field, why))
            })
        {
            return Some(format!(
                "the field `{}` of the event schema `{}`, {why}",
                field.id, schema.name
            ));
        }
        for state in model.states.values() {
            let blocks = state
                .on_entry_blocks
                .iter()
                .chain(&state.on_exit_blocks)
                .map(Vec::as_slice)
                .chain([
                    state.initial_transition_actions.as_slice(),
                    state.initial_history_default_actions.as_slice(),
                ])
                .chain(state.transitions.iter().map(|t| t.actions.as_slice()));
            for block in blocks {
                if let Some(found) = Self::unlowered_action(block) {
                    return Some(found);
                }
            }
            // The same judgment the statechart generator makes of a document of any data
            // model: a static child session is lowered, and each other kind of
            // `<invoke>` is named.
            for invoke in &state.invokes {
                if let Some(found) = crate::generator::swift_invoke_unlowered(invoke, true) {
                    return Some(found);
                }
            }
        }
        None
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Swift
    }
    // A prefix keeps a variable's property from meeting another member of the
    // policy, which a bare name could (`engine`, `sessionID`).
    fn field_name(&self, id: &str) -> String {
        format!("v{}", filters::to_pascal_case(id.to_string()))
    }
    // The author's name in the case a Swift property is written in, and none for a
    // name the language reserves or the class already holds a member by.
    fn reader_name(&self, id: &str, machine: &str) -> Option<String> {
        let spelled = filters::to_camel_case(id.to_string());
        (!crate::reader_names::reader_is_taken(Language::Swift, &spelled, machine))
            .then_some(spelled)
    }
    fn field_ref(&self, name: &str) -> String {
        format!("self.{name}")
    }
    fn in_function(&self) -> &'static str {
        "self.isStateActive"
    }
    // A hybrid invoke reads the stem of the string its `srcexpr` computes
    // (`documentStem`) and starts the candidate it names, handing it the values it
    // keeps (`seed_child` of `invoke_methods.swift.jinja2`).
    fn lowers_hybrid_invoke(&self) -> bool {
        true
    }
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::swift_type(ty).to_string()
    }
    fn scalar_view(&self, _ty: &SceType) -> Option<String> {
        None
    }
    fn record_type(&self, machine: &str, alias: &str) -> String {
        format!(
            "{machine}{}Record",
            filters::to_pascal_case(alias.to_string())
        )
    }
    // A struct of the schema's fields in the schema's order, named as the author
    // wrote them because an expression reads a field by that name
    // (`shown.dayOfMonth` is `self.vShown.dayOfMonth`). It is a value, so a host that
    // reads one holds a copy, and it is `Equatable` and `Sendable` as its fields are.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String {
        // A field of the present type, spelled as the machine holds it.
        let present_of =
            |field: &crate::forge::model::SchemaField| match field.sce_type.present_type() {
                SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                other => crate::forge::generator::swift_type(other).to_string(),
            };
        // A field that may be absent is the optional type over its present value's
        // (docs/adr/0055), the type an expression reads it as.
        let held_of = |field: &crate::forge::model::SchemaField| {
            let present = present_of(field);
            match &field.sce_type {
                FieldType::Optional { .. } => self
                    .optional_type(&present)
                    .expect("Swift spells an optional value"),
                FieldType::Scalar(_) => present,
            }
        };
        // Every field has a value at its zero, so the record is `SceDefault`: the value
        // that stands where a checked read of a list element that is not there has none
        // to answer, and the one `init()` a record is built whole from. The zero of a
        // field that may be absent is its absence.
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                let held = held_of(field);
                let zero = match (&field.sce_type, field.sce_type.present_type()) {
                    (FieldType::Optional { .. }, _) => "nil".to_string(),
                    (_, SceType::Enum(_)) => format!("{held}.allCases[0]"),
                    (_, other) => crate::forge::generator::swift_zero_value(other).to_string(),
                };
                format!(
                    "    public var {}: {held} = {zero}\n",
                    self.record_field(&field.id)
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\n\
             public struct {ty}: Equatable, Sendable, SceDefault {{\n{fields}\n\
             \x20   public init() {{}}\n\n\
             \x20   public init({params}) {{\n{assigns}\x20   }}\n}}",
            params = schema
                .fields
                .iter()
                .map(|f| format!("{}: {}", self.record_field(&f.id), held_of(f)))
                .collect::<Vec<_>>()
                .join(", "),
            assigns = schema
                .fields
                .iter()
                .map(|f| format!("        self.{0} = {0}\n", self.record_field(&f.id)))
                .collect::<String>(),
        )
    }
    // The author's spelling, which is what an expression's member access writes;
    // [`Self::unsupported`] has refused a name Swift reserves.
    fn record_field(&self, id: &str) -> String {
        id.to_string()
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!("{f}: {v}")).collect();
        format!("{ty}({})", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    // A member is scoped under the type, named lowerCamelCase
    // ([`crate::forge::enum_naming::variant_ident`]); a word Swift reserves is refused
    // where the enum is held ([`Self::unsupported`]).
    fn enum_variant(&self, ty: &str, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ref(Language::Swift, ty, enum_name, variant)
    }
    // The enum the enum document is written as, under the machine's own name for it
    // (`crate::forge::generator::swift_enum_declaration`, which the unit of the
    // document is declared by too).
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        crate::forge::generator::swift_enum_declaration(
            ty,
            &format!("/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value."),
            model,
        )
    }
    fn enum_number_symbols(&self, ty: &str, _model: &EnumModel) -> (String, String) {
        (format!("{ty}.toUnderlying"), format!("{ty}.fromUnderlying"))
    }
    // Both types are enums of the same declared variants, so a `switch` over every one
    // of them is total. It is written as a closure called where it stands, since Swift
    // has no `switch` that is an expression in every place a value is wanted (an
    // argument, the body of a `map`).
    fn enum_crossing(
        &self,
        machine_type: &str,
        model: &EnumModel,
        value: &str,
        crossing: Crossing,
    ) -> Option<String> {
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Swift);
        let (from_type, into) = match crossing {
            Crossing::ToAlgorithm => (machine_type, document.as_str()),
            Crossing::FromAlgorithm => (document.as_str(), machine_type),
        };
        let arms: Vec<String> = enum_variant_pairs(Language::Swift, machine_type, model, crossing)
            .into_iter()
            .map(|(from, to)| format!("case {from}: return {to}"))
            .collect();
        // The value is an argument of the closure, not read inside it: it may be a
        // member of the `$0` of a `map` this crossing is written in, which a closure
        // with arguments of its own cannot name.
        Some(format!(
            "{{ (sceValue: {from_type}) -> {into} in switch sceValue {{ {} }} }}({value})",
            arms.join("; ")
        ))
    }
    // A function of another kind the machine calls (a lookup, a transform) takes and
    // answers the enum document's own type, so a value of the machine's crosses through
    // these two, the crossing a call of an algorithm writes inline.
    fn enum_crossing_names(&self, ty: &str) -> Option<(String, String)> {
        Some(enum_crossing_function_names(ty))
    }
    // Two functions of the file, each a `switch` over every variant.
    fn enum_crossing_def(&self, ty: &str, model: &EnumModel) -> Option<String> {
        let (to, from) = enum_crossing_function_names(ty);
        let document = crate::forge::generator::enum_qualified_type(&model.name, Language::Swift);
        let into = self.enum_crossing(ty, model, "v", Crossing::ToAlgorithm)?;
        let back = self.enum_crossing(ty, model, "v", Crossing::FromAlgorithm)?;
        Some(format!(
            "/// A value of `{ty}` as the variant of the same declared name of `{document}`.\n\
             private func {to}(_ v: {ty}) -> {document} {{\n    return {into}\n}}\n\n\
             /// A value of `{document}` as the variant of the same declared name of `{ty}`.\n\
             private func {from}(_ v: {document}) -> {ty} {{\n    return {back}\n}}"
        ))
    }
    // An `Array` of the element: a value, so a host that reads one holds a copy and
    // cannot write through it into what the machine keeps.
    fn list_type(&self, elem: &SceType) -> String {
        format!("[{}]", crate::forge::generator::swift_type(elem))
    }
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn record_list_type(&self, record: &str) -> String {
        format!("[{record}]")
    }
    fn record_list_view(&self, _record: &str) -> Option<String> {
        None
    }
    fn list_empty(&self) -> String {
        "[]".to_string()
    }
    // The bound a string variable keeps, counted in UTF-8 bytes
    // (`Checked.bounded`, which records the failure and answers the empty text).
    fn bounded_string(&self, value: &str, capacity: u32) -> String {
        format!("Checked.bounded(sceFailure, {value}, capacity: {capacity})")
    }
    fn bounded_bytes(&self, value: &str, capacity: u32) -> String {
        format!("Checked.bounded(sceFailure, {value}, capacity: {capacity})")
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
    }
    // An algorithm's list is an `Array` of its element, which is the machine's own, held to the
    // variable's bound by the one function the adapters hold a list to it with.
    fn list_within(&self, value: &str, capacity: u32, _symbol: &str, _elem: &SceType) -> String {
        format!("Checked.within(sceFailure, {value}, capacity: {capacity})")
    }
    // The count is the engine's the action is handed.
    fn fresh_send_id(&self) -> String {
        "engine.nextAutoSendID()".to_string()
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value}")
    }
    // The label is text and the value is shown as Swift shows it, never a format.
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
        format!(
            "print(\"{}\" + String(describing: {value}))",
            filters::escape_swift(label.to_string())
        )
    }
    // An expression that is `true` when the append failed: the list is full, or the
    // value could not be computed. The room is checked first, so a full list computes
    // nothing, and the value is computed into a local before it is appended, so a
    // failed one leaves the list as it was. The local takes the element's type from the
    // list, which a literal alone could not give it (`Checked.element`).
    fn append(
        &self,
        target: &str,
        capacity: u32,
        value: &str,
        _value_can_fail: bool,
        overflow: &str,
        failed: &str,
    ) -> String {
        format!(
            "{{ () -> Bool in let sceFailure = Failure(); \
             if {target}.count >= {capacity} {{ {} }}; \
             let sceValue = Checked.element(of: {target}, {value}); \
             if sceFailure.failed {{ {} }}; \
             {target}.append(sceValue); return false }}()",
            Self::then(&[overflow, "return true"]),
            Self::then(&[failed, "return true"]),
        )
    }
    fn clear(&self, target: &str) -> String {
        format!("{target}.removeAll()")
    }
    // The loop walks the list as it was when it began (§scxml-4.6: a shallow copy): an
    // `Array` is a value, so a body that appends to or empties the list does not move
    // it. The position is a `uint32`, as `len` is, and a name the body never reads is
    // still used.
    fn foreach_loop(
        &self,
        list: &str,
        item: &str,
        index: Option<&str>,
    ) -> Option<(String, String)> {
        Some(match index {
            None => (format!("for {item} in {list}"), format!("_ = {item}")),
            Some(index) => (
                format!("for (sceIndex, {item}) in {list}.enumerated()"),
                format!(
                    "let {index} = UInt32(truncatingIfNeeded: sceIndex); _ = {index}; _ = {item}"
                ),
            ),
        })
    }
    fn raise_execution_error(&self, machine: &str, message: &str) -> String {
        format!(
            "engine.raise(EventWithMetadata.platformError({machine}Event.{}, message: \"{}\"))",
            filters::to_swift_variant("error.execution".to_string()),
            filters::escape_swift(message.to_string())
        )
    }
    // The runtime's checked helpers record a failure in `sceFailure` and answer a
    // zero, so a statement that wrote its value would write that zero. The
    // expression is a closure called where it stands, answering whether it failed,
    // so the dispatcher can end its block as it does for any error.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "{{ () -> Bool in let sceFailure = Failure(); {statement}; \
             if sceFailure.failed {{ {} }}; return false }}()",
            Self::then(&[failed, "return true"])
        )
    }
    fn receiving_write(
        &self,
        write: &dyn Fn(&str) -> String,
        value: &str,
        ty: InferredType,
        failed: &str,
    ) -> String {
        // The local takes the type the value is of where that can be named, so a
        // value no operand of which is typed is not an `Int`.
        let annotation = crate::forge::expr::swift::nameable_type(ty)
            .map(|name| format!(": {name}"))
            .unwrap_or_default();
        format!(
            "{{ () -> Bool in let sceFailure = Failure(); let sceValue{annotation} = {value}; \
             if sceFailure.failed {{ {} }}; {}; return false }}()",
            Self::then(&[failed, "return true"]),
            write("sceValue")
        )
    }
    // Only a host call's arguments reach here, and its one spelling is
    // [`Self::receiving_host_call`].
    fn receiving_call(&self, _statement: &str, _failed: &str) -> String {
        unreachable!("a Swift host call is received through receiving_host_call")
    }
    // A failed checked operation is a value, so the call could not be stopped after
    // its arguments were evaluated: each is computed into a local first, and the
    // host is called only when none of them failed. A block of its own, so two
    // calls in one scope do not declare the same locals.
    fn receiving_host_call(
        &self,
        _statement: &str,
        callee: &str,
        args: &[String],
        arg_types: &[FieldType],
        failed: &str,
    ) -> String {
        // Each local takes the type the host method declares for it, so a literal is
        // the integer the parameter is and not an `Int`.
        let locals: String = args
            .iter()
            .zip(arg_types)
            .enumerate()
            .map(|(i, (arg, ty))| {
                format!(
                    "let sceArg{i}: {} = {arg}; ",
                    crate::forge::generator::host_param_type(Language::Swift, ty)
                )
            })
            .collect();
        let names: Vec<String> = (0..args.len()).map(|i| format!("sceArg{i}")).collect();
        format!(
            "do {{ let sceFailure = Failure(); {locals}\
             if sceFailure.failed {{ {} }} else {{ {callee}({}) }} }}",
            Self::then(&[failed]),
            names.join(", ")
        )
    }
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String {
        format!(
            "{{ () -> Bool in let sceFailure = Failure(); let sceValue: Bool = {value}; \
             if sceFailure.failed {{ {} }}; return sceValue }}()",
            Self::then(&[failed, flag, "return false"])
        )
    }
    // The flag is a local of the `<if>`, named by its ordinal, which the condition's
    // closure sets by reference.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String {
        format!("ifCondFailed{if_ordinal} = true")
    }
    // Asked for every event of the document when the paths an expression reads are
    // built, though only an event that carries a typed payload has one to read, and
    // [`Self::unsupported`] refuses a document that declares one: the name is the
    // one the payload channel will give the field.
    fn payload_accessor(&self, event: &str) -> String {
        format!(
            "self.pending{}Payload",
            filters::to_event_variant(event.to_string())
        )
    }
    // An enum field of the payload is held in the machine's own enum, lifted from the
    // variant's declared name (`build_swift_event_payload`).
    fn payload_enum_fields(&self) -> bool {
        true
    }
    // The tag says which event's payload the channel holds now, so a guard that reads
    // this event's fields holds only for a delivery that carried them.
    fn payload_guard(&self, _machine: &str, event: &str, lowered: &str) -> String {
        format!(
            "self.pendingPayloadTag == .{} && ({lowered})",
            filters::to_swift_variant(event.to_string())
        )
    }
    // The send template reads the string a `delayexpr`/`<content expr>` computes as a
    // value the runtime writes as JSON (`WireValue`).
    fn lowers_scalar_content(&self) -> bool {
        true
    }
    // The `WireValue` the pair's JSON value is written from, as the runtime's own
    // `PayloadPair` writes the same value: a bool and a string as they are, a narrow
    // integer as a number and a real as a double, both exactly; a 64-bit integer is the
    // text of its digits.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!(".truth({value})"),
            InferredType::Str => format!(".text({value})"),
            InferredType::Int {
                signed: true,
                bits: 64,
            } => format!(".signed64({value})"),
            InferredType::Int { bits: 64, .. } => format!(".unsigned64({value})"),
            InferredType::Int { .. } => format!(".integer(Int64({value}))"),
            _ => format!(".real(Double({value}))"),
        }
    }
    // The enum's `declaredName` answers the name the enum document declares.
    fn enum_wire_name(&self, _alias: &str, value: &str) -> Option<String> {
        Some(format!("({value}).declaredName"))
    }
    // The byte string the runtime writes as its Latin-1 text.
    fn wire_bytes(&self, value: &str) -> Option<String> {
        Some(format!(".bytes({value})"))
    }
    // `WireValue.null`, which the runtime writes as JSON `null`, where the value is
    // absent.
    fn wire_optional(
        &self,
        value: &str,
        present: &str,
        wire: &dyn Fn(&str) -> String,
    ) -> Option<String> {
        let wired = self.optional_map(value, present, wire)?;
        Some(format!("{wired} ?? WireValue.null"))
    }
}

/// Rewrite `model` — a clone the Swift backend renders — so every expression of a
/// `sce-static` document is native Swift.
pub fn lower_swift(model: &mut SCXMLModel, machine: &str) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &SwiftTarget)
}

/// The target that spells `lang`, when it lowers `sce-static` at all.
pub(crate) fn target_for(lang: Language) -> Option<&'static dyn StaticTarget> {
    match lang {
        Language::Kotlin => Some(&KotlinTarget),
        Language::Rust => Some(&RustTarget),
        Language::Cpp => Some(&CppTarget),
        // Only the call is spelled here, which the package's name alone fixes.
        Language::Go => Some(&GoTarget { import_root: None }),
        Language::Python => Some(&PythonTarget),
        // A C machine's lowering counts the conditions it lowers ([`CTarget`]),
        // so it is a value of its own per document ([`lower_c11`]). What is
        // asked of it from outside the walk is a host action's arguments, which
        // count and name nothing.
        Language::C11 => Some(&C_ARGUMENTS),
        Language::Swift => Some(&SwiftTarget),
    }
}

/// A `<sce:action>` argument of a `sce-static` document, lowered for one
/// backend.
#[derive(Debug, Clone)]
pub(crate) struct StaticArgument {
    /// The argument as the backend's code, reading the machine's fields and,
    /// when [`Self::reads_payload`], the bound payload.
    pub text: String,
    /// The type the host method declares for it
    /// ([`crate::forge::native_action::static_argument_type`]): a scalar, or an
    /// optional scalar for an argument that may be absent (docs/adr/0055, step 10).
    pub ty: FieldType,
    /// Whether it reads the triggering event's typed payload, so the call
    /// must sit under the payload channel's guard.
    pub reads_payload: bool,
    /// Whether computing it can fail (SCE_FORGE.md §3.4.1), so the call must
    /// sit where the failure is received (E12 D5).
    pub can_fail: bool,
}

/// Lower one `<sce:action>` argument of a `sce-static` document for `lang`:
/// judged against the scope validation judged it against — the document's
/// `scope`, and the event's payload when the action sits on a transition
/// whose event carries one — and spelled with the renames every other
/// lowered expression takes. `proven` are the subjects of the payload the
/// transition's `cond` proves present where the action stands (docs/adr/0055,
/// section 3), so a `.value` validation admitted is lowered and not refused. `None`
/// for an argument validation refused, which never reaches here, and for a backend
/// that does not lower the model.
pub(crate) fn lower_static_argument(
    scope: &StaticScope,
    event: Option<(&str, &EventSchemaModel)>,
    arg: &crate::model::Param,
    lang: Language,
    proven: &Facts,
) -> Option<StaticArgument> {
    let target = target_for(lang)?;
    let paths = scope.paths(event.map(|(_, schema)| schema));
    let ctx = scope.ctx(&paths, &[]).proving(proven.clone());
    let names = names(scope, target);
    let accessor = event.map(|(event, _)| target.payload_accessor(event));
    let renames = renames(&names, accessor.as_deref(), target);
    let ty = crate::forge::native_action::static_argument_type(&ctx, arg).ok()?;
    // The argument is lowered into a place of its own type: an optional value is
    // read as it is, and is lifted into nothing.
    let lowered = transpile_into_receiving(
        &arg.expr,
        target.expr_target(),
        &ctx,
        &renames,
        InferredType::of_field(&ty),
    )
    .ok()?;
    Some(StaticArgument {
        text: lowered.text,
        ty,
        reads_payload: event.is_some()
            && crate::forge::expr::references_event_data_lexically(&arg.expr),
        can_fail: lowered.can_fail,
    })
}

/// `statement` — a host call whose arguments can fail — run where the failure
/// is received, with `error.execution` naming `construct` in its place when
/// the document declares that event (E12 D5). `args` is each argument's lowered
/// text with the type it is held in. `None` for a backend that does not lower
/// the model.
pub(crate) fn receive_static_statement(
    lang: Language,
    statement: &str,
    callee: &str,
    args: &[(String, FieldType)],
    machine: &str,
    raises_error: bool,
    construct: &str,
) -> Option<String> {
    let target = target_for(lang)?;
    let failed = if raises_error {
        target.raise_execution_error(
            machine,
            &format!("{construct}: an integer operation overflowed or failed"),
        )
    } else {
        String::new()
    };
    let (texts, types): (Vec<String>, Vec<FieldType>) = args.iter().cloned().unzip();
    Some(target.receiving_host_call(statement, callee, &texts, &types, &failed))
}

/// The nullable field the Kotlin payload channel binds `event`'s typed
/// payload to — the spelling [`crate::forge::generator::build_kotlin_event_payload`]
/// declares.
fn kotlin_payload_field(event: &str) -> String {
    format!(
        "pending{}Payload",
        filters::to_event_variant(event.to_string())
    )
}

/// The renames a lowered expression takes: each variable to its field, `In`
/// to the machine's active-state test (the function a pure `In()` guard
/// lowers to), and `_event.data` to the payload accessor when there is one.
fn renames<'a>(
    names: &'a [(String, String)],
    payload: Option<&'a str>,
    target: &dyn StaticTarget,
) -> HashMap<&'a str, &'a str> {
    let mut map: HashMap<&str, &str> = names
        .iter()
        .map(|(id, name)| (id.as_str(), name.as_str()))
        .collect();
    map.insert("In", target.in_function());
    if let Some(accessor) = payload {
        map.insert("_event.data", accessor);
    }
    map
}

/// A whole record of the schema `alias` made from the payload of the event the
/// walk is in (validation judged that payload to be of that schema): each field
/// is read as `_event.data.<field>`, lowered like any expression into the type
/// the record holds it in, and the record is made in one value — so what cannot
/// be read leaves the variable as it was, and never half of a payload is written.
///
/// A string field is held to the `sce:max-size` its schema declares, as it is when
/// it is assigned alone: a payload whose text is past it does not fit the record,
/// so the whole assignment fails and the record is left as it was.
fn payload_record_value(
    machine: &str,
    alias: &str,
    schema: &EventSchemaModel,
    target: &dyn StaticTarget,
    lower: &dyn Fn(&str, InferredType) -> Result<Receiving, GenerateError>,
) -> Result<Receiving, GenerateError> {
    let ty = target.record_type(machine, alias);
    let mut values = Vec::with_capacity(schema.fields.len());
    let mut bounded = false;
    for field in &schema.fields {
        let read = lower(
            &format!(
                "{}.{}",
                crate::forge::event_schema_check::EVENT_DATA_PATH,
                field.id
            ),
            InferredType::of_field(&field.sce_type),
        )?;
        if read.can_fail {
            return Err(GenerateError::unsupported(format!(
                "`_event.data.{}` can fail, which a record made whole from a payload does \
                 not receive",
                field.id
            )));
        }
        let text = match (&field.sce_type, field.max_size) {
            (FieldType::Scalar(SceType::String), Some(bound)) => {
                bounded = true;
                let held = target.bounded_string(&read.text, bound);
                target.record_string_runtime(bound, &held)
            }
            (FieldType::Scalar(SceType::Bytes), Some(bound)) => {
                bounded = true;
                let held = target.bounded_bytes(&read.text, bound);
                target.record_bytes_runtime(bound, &held)
            }
            // An optional text is held to the bound where it is present, and
            // absent where it is absent (docs/adr/0055).
            (
                FieldType::Optional {
                    elem: elem @ (SceType::String | SceType::Bytes),
                },
                Some(bound),
            ) => {
                bounded = true;
                let held = |v: &str| match elem {
                    SceType::Bytes => {
                        target.record_bytes_runtime(bound, &target.bounded_bytes(v, bound))
                    }
                    _ => target.record_string_runtime(bound, &target.bounded_string(v, bound)),
                };
                let present = optional_present(target, machine, elem, Some(bound));
                target
                    .optional_map(&read.text, &present, &held)
                    .ok_or_else(|| {
                        optional_not_lowered(
                            &format!("the field `{}` of `_event.data` taken whole", field.id),
                            target.name(),
                        )
                    })?
            }
            _ => read.text,
        };
        values.push((target.record_field(&field.id), text));
    }
    Ok(Receiving {
        text: target.record_value(&ty, &values),
        can_fail: bounded,
    })
}

/// How `target` reads each enum field of the payload `schema` declares, keyed by
/// the `_event.data.<field>` path an expression names it by — for a target
/// whose payload is untyped data, which it reads a field of through a call
/// that holds the value to the variants the enum declares. A target that holds
/// the payload in a struct of its own reads the field off it and asks for none.
/// A byte-string field is read here too, by a target that holds it as a buffer and
/// its length ([`StaticTarget::payload_bytes_read`]), and so is a field of any type
/// a later revision of the schema added, by a target whose untyped payload has to
/// be told the default a delivery that lacks the field takes
/// ([`StaticTarget::payload_revised_read`]).
fn payload_enum_reads(
    target: &dyn StaticTarget,
    schema: Option<&EventSchemaModel>,
    accessor: &str,
    enums: &std::collections::BTreeMap<String, EnumModel>,
) -> Vec<(String, String)> {
    let Some(schema) = schema else {
        return Vec::new();
    };
    schema
        .fields
        .iter()
        .filter_map(|field| {
            let variants: Vec<&str> = match field.sce_type.present_type() {
                SceType::Enum(reference) => enums
                    .get(&reference.alias)?
                    .variants
                    .iter()
                    .map(|v| v.name.as_str())
                    .collect(),
                _ => Vec::new(),
            };
            let path = format!(
                "{}.{}",
                crate::forge::event_schema_check::EVENT_DATA_PATH,
                field.id
            );
            // An optional field is read as the optional value it is
            // (docs/adr/0055), by its own read, which reads the default of one a
            // later revision added where the delivery lacks it.
            if field.sce_type.optional_elem().is_some() {
                return target
                    .payload_optional_read(accessor, field, &variants)
                    .map(|read| (path, read));
            }
            let revised = if field.since.is_some() {
                crate::forge::schema_revision::default_wire(field).and_then(|default| {
                    target.payload_revised_read(accessor, field, &variants, &default)
                })
            } else {
                None
            };
            let read = match (revised, field.sce_type.present_type()) {
                (Some(read), _) => read,
                (None, SceType::Enum(_)) => {
                    target.payload_enum_read(accessor, &field.id, &variants)?
                }
                (None, SceType::Bytes) => target.payload_bytes_read(accessor, &field.id)?,
                (None, _) => return None,
            };
            Some((path, read))
        })
        .collect()
}

/// What runs in place of a statement or condition whose expression failed
/// (E12 D5): `error.execution` naming `construct` — when the document
/// declares that event. Without it there is nothing to raise and nothing
/// could match one, so the statement is skipped silently, as an append past
/// a list's bound is.
fn execution_failure(rewrites: &Rewrites<'_>, construct: &str) -> String {
    if rewrites.raises_error {
        rewrites.target.raise_execution_error(
            rewrites.machine,
            &format!("{construct}: an integer operation overflowed or failed"),
        )
    } else {
        String::new()
    }
}

/// The processor types a `<send>` is delivered through: the ones its `typeexpr`
/// may compute to, as the document declares them (`sce:types`), or the one it
/// writes — empty when it writes none, which is the SCXML Event I/O Processor.
/// What a target refuses of a written `type` it refuses of each of these.
fn effective_send_types(action: &Action) -> impl Iterator<Item = &str> {
    let declared = !action.typeexpr.trim().is_empty() && !action.types.is_empty();
    let written = (!declared).then_some(action.send_type.as_str());
    action
        .types
        .iter()
        .map(String::as_str)
        .filter(move |_| declared)
        .chain(written)
}

/// The largest [`Action::if_ordinal`] `model` holds, over every block of
/// executable content and the default transition of every history.
fn max_if_ordinal(model: &SCXMLModel) -> u32 {
    let mut largest = 0;
    let mut note = |action: &Action| {
        action.walk(&mut |a| {
            if a.action_type == "if" {
                largest = largest.max(a.if_ordinal);
            }
        });
    };
    for state in model.states.values() {
        for block in state.executable_blocks() {
            block.iter().for_each(&mut note);
        }
    }
    for history in model.history_states.values() {
        history.default_actions.iter().for_each(&mut note);
    }
    largest
}

/// The type a `<send>` is given where the value its `typeexpr` computes is none
/// the document declares: no processor this build delivers through, so the send
/// raises `error.execution` as it does for a written `type` that names one
/// (§scxml-6.2.5).
const UNDECLARED_SEND_TYPE: &str = "sce:undeclared-type";

/// Expand a `<send typeexpr>` of a `sce-static` document into the choice it makes
/// (docs/adr/0005, decision 3): an `<if>` whose branches are one `<send>` for each
/// processor the document declares as `sce:types`, each with the `type` written
/// out, taken when the value the expression computes is that processor's, and
/// whose `<else>` is the `<send>` of a type nothing delivers through — so a value
/// the document did not declare is `error.execution` with nothing sent, by the
/// arm every engine already has for it.
///
/// The `<send>` is copied whole: its event, target, delay, `<param>`s and
/// `<content>` are the same for every branch, and the branch taken is the one that
/// runs. What the type decides — whether the target is one the SCXML Event I/O
/// Processor can address, and whether the type is one this build delivers through
/// — is decided again for the copy, as the parser decides it for a written `type`.
fn expand_computed_type(send: &Action, if_ordinal: u32) -> Action {
    use crate::host_processor_analyzer::{
        is_supported_send_type, is_unsupported_scxml_target, SCXML_EVENT_PROCESSOR_TYPE,
    };
    let delivered_by = |send_type: &str| -> Action {
        let mut copy = send.clone();
        copy.typeexpr.clear();
        copy.types.clear();
        copy.send_type = send_type.to_string();
        copy.send_type_unsupported = !is_supported_send_type(send_type);
        copy.target_unsupported = copy.targetexpr.is_empty()
            && send_type == SCXML_EVENT_PROCESSOR_TYPE
            && is_unsupported_scxml_target(&copy.target);
        copy
    };
    let names = |entry: &str| format!("({}) === '{}'", send.typeexpr.trim(), entry);
    let mut entries = send.types.iter();
    let first = entries
        .next()
        .expect("the judge held a computed type to a declared set");
    let resolved = crate::parser::resolve_cond(&names(first), &Default::default());
    let mut chain = Action::if_then(names(first), vec![delivered_by(first)]);
    chain.cond_constant = resolved.cond_constant;
    chain.if_ordinal = if_ordinal;
    chain.source_location = send.source_location.clone();
    for entry in entries {
        chain.push_elseif(names(entry), vec![delivered_by(entry)]);
    }
    chain.set_else(vec![delivered_by(UNDECLARED_SEND_TYPE)]);
    chain
}

/// Lower every action of `actions` in place. `true` when any expression
/// lowered reads the triggering event's payload.
///
/// Each action is lowered with the subjects proven present where it stands
/// (docs/adr/0055, section 3): those `ctx` holds where the block is entered,
/// less each one an action before it may write ([`Facts::after`]) — the walk
/// the judge took ([`crate::forge::static_datamodel`]), by the one analysis
/// ([`crate::forge::narrowing`]), so a `.value` the judge admitted is lowered
/// and not refused.
fn lower_actions(
    actions: &mut [Action],
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let mut reads_payload = false;
    let mut proven = ctx.proven.clone();
    for action in actions {
        // What the action writes, read before the walk rewrites it.
        let writes = Writes::of(&*action);
        reads_payload |= lower_action(action, &proving(ctx, &proven), renames, rewrites)?;
        proven = proven.after(&writes);
    }
    Ok(reads_payload)
}

/// Lower one action and what it nests, in place. `true` when any expression
/// lowered reads the triggering event's payload.
fn lower_action(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    // What a statement writes lands in a place that owns it (a variable, a
    // record's field, a list's element), so a `string` is made owned on the way
    // in. A condition's slot is `bool` and a logged value's is untyped, and
    // neither is a string slot, so they lower exactly as they did.
    let lower = |text: &str, slot: InferredType| {
        rewrites.owned(text, ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no {lang} lowering: {}", r.error))
        })
    };
    let failed = |construct: String| execution_failure(rewrites, &construct);
    // `write(value)`, received where it stands when `value` can fail — and
    // then an expression that says whether it did, which the dispatcher acts
    // on by ending the block (§scxml-4.9) — with whether it can.
    let statement = |value: &Receiving,
                     ty: InferredType,
                     write: &dyn Fn(&str) -> String,
                     construct: String|
     -> (String, bool) {
        if value.can_fail {
            (
                target.receiving_write(write, &value.text, ty, &failed(construct)),
                true,
            )
        } else {
            (write(&value.text), false)
        }
    };
    let reads = crate::forge::expr::references_event_data_lexically;
    let mut reads_payload = false;
    // A processor named by an expression (docs/adr/0005, decision 3): a target
    // that runs the document's own `<send>` holds the value to the types the
    // document declares where the attribute stands; every other generates the
    // choice, a literal-type `<send>` for each type the document declares under
    // an `<if>` that compares the value with it, so each engine delivers by the
    // processor its own arms deliver a written `type` through. What the walk goes
    // on to lower is then that `<if>` and the sends it holds.
    if action.action_type == "send" && !action.typeexpr.trim().is_empty() {
        let value = lower(&action.typeexpr, InferredType::Str)?;
        match target.type_expr_site(&value.text, &action.types) {
            Some(site) => {
                reads_payload |= reads(&action.typeexpr);
                rewrites.note(&action.typeexpr, action.spellings.get("typeexpr"), &site);
            }
            None => *action = expand_computed_type(action, rewrites.take_if_ordinal()),
        }
    }
    // Each statement lands whole in `native_code`, and each condition in
    // `native_cond` — the slots every backend's action dispatcher reads before
    // its own spellings, so the IR the author wrote is left as it was.
    match action.action_type.as_str() {
        // A whole record is taken from a record of its schema by name — a
        // record variable or a loop's record item, which validation judged
        // before this walk — and is a plain value, so the variable holds a copy
        // of it as it stands now. The name lowers as any other: renamed to the
        // field that holds it, which is all the assignment needs. The payload of
        // the event the transition is on is a record of its schema too, taken
        // whole as `_event.data`: it is made field by field into one value
        // ([`payload_record_value`]).
        "assign" => {
            reads_payload = reads(&action.expr);
            // A place of an optional value takes its whole value by the rule of
            // one (docs/adr/0055).
            if let Some(place) = rewrites.optionals.get(action.location.trim()) {
                lower_optional_assign(action, place, ctx, renames, rewrites)?;
                return Ok(reads_payload);
            }
            // ... and a place of an optional record takes a whole record, or
            // `null` (docs/adr/0055, step 9).
            if let Some(place) = rewrites.optional_records.get(action.location.trim()) {
                lower_optional_record_assign(action, place, ctx, renames, rewrites)?;
                return Ok(reads_payload);
            }
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            let location = action.location.trim();
            // The type of the record a payload, or the element of a list of records
            // a target reads whole ([`StaticTarget::record_at`]), is taken whole
            // into, when it is: the value is held while it is computed in a value of
            // its own type, as a read that can fail is.
            let whole_record = match rewrites.records.get(location) {
                Some((alias, _))
                    if action.expr.trim() == "_event.data"
                        || indexed_element(&action.expr).is_some() =>
                {
                    Some(target.record_type(rewrites.machine, alias))
                }
                _ => None,
            };
            let mut value = match rewrites.records.get(location) {
                Some((alias, schema)) if action.expr.trim() == "_event.data" => {
                    payload_record_value(rewrites.machine, alias, schema, target, &lower)?
                }
                // The element of a list of records at an index, whole: a target
                // that reads one whole copies it as it copies a named record; any
                // other copies the element a field at a time.
                Some((_, schema)) if indexed_element(&action.expr).is_some() => {
                    let (list, index) =
                        indexed_element(&action.expr).expect("guarded by the match above");
                    let lowered = lower(index, InferredType::Unknown)?;
                    let named = renames.get(list).copied().unwrap_or(list);
                    match target.record_at(named, &lowered.text) {
                        Some(element) => Receiving {
                            text: target.record_copy(&element, schema),
                            can_fail: true,
                        },
                        None => {
                            let (list, index) = (list.to_string(), index.to_string());
                            return lower_element_fields(
                                action, &list, &index, schema, ctx, renames, rewrites,
                            )
                            .map(|reads| reads || reads_payload);
                        }
                    }
                }
                // A record taken from another by name is a copy of it.
                Some((_, schema)) => {
                    let value = lower(&action.expr, slot)?;
                    Receiving {
                        text: target.record_copy(&value.text, schema),
                        can_fail: value.can_fail,
                    }
                }
                _ => lower(&action.expr, slot)?,
            };
            // A string variable holds at most what it declared, whatever the
            // value came from: past it the assignment fails as any other does,
            // writing nothing and ending its block (§scxml-4.9).
            if let Some(capacity) = rewrites.strings.get(location) {
                value = Receiving {
                    text: target.bounded_string(&value.text, *capacity),
                    can_fail: true,
                };
            } else if let Some(capacity) = rewrites.bytes.get(location) {
                // ... and so does a byte string, in bytes.
                value = Receiving {
                    text: target.bounded_bytes(&value.text, *capacity),
                    can_fail: true,
                };
            }
            let construct = format!("<assign location='{location}'>");
            let record_field = field_location(location, rewrites);
            // A target that rewrites the document itself writes a record's
            // field as an `<assign>` of the whole record, changed in that
            // field: its location and its value are both the walk's to note.
            let rewritten = record_field.and_then(|(var, field)| {
                let name = renames.get(var).copied().unwrap_or(var);
                target.field_assignment(name, field.trim(), &value.text)
            });
            match &rewritten {
                Some((record, expr)) => {
                    rewrites.note(&action.location, action.spellings.get("location"), record);
                    rewrites.note(&action.expr, action.spellings.get("expr"), expr);
                }
                None => rewrites.note(&action.expr, action.spellings.get("expr"), &value.text),
            }
            (action.native_code, action.native_fails) = match record_field {
                Some((var, field)) => {
                    let name = renames.get(var).copied().unwrap_or(var);
                    let field = target.record_field(field.trim());
                    // A string field is written as the bound it was held to
                    // says, which a target that holds it in a buffer copies
                    // into it.
                    let kind = if rewrites.strings.contains_key(location) {
                        FieldValue::Text
                    } else if rewrites.bytes.contains_key(location) {
                        FieldValue::Bytes
                    } else {
                        FieldValue::Plain
                    };
                    statement(
                        &value,
                        slot,
                        &|v| write_field(rewrites, var, name, &field, v, kind).unwrap_or_default(),
                        construct,
                    )
                }
                None => {
                    let name = renames.get(location).copied().unwrap_or(location);
                    let is_string = rewrites.strings.contains_key(location);
                    let is_bytes = rewrites.bytes.contains_key(location);
                    // A list variable takes what an imported algorithm returns
                    // as a list, whole — the judge held the right side to that
                    // call and the variable to its bound.
                    //
                    // A call that takes or returns state is the adapter's, not the
                    // algorithm's own: what it gives back is the adapter's result,
                    // which the algorithm's `_within` does not take.
                    let state_call = rewrites.whole_state_call(&action.expr);
                    let returned_list = rewrites
                        .lists
                        .get(location)
                        .filter(|_| state_call.is_none())
                        .and_then(|(elem, capacity)| match elem {
                            crate::forge::model::ListElemType::Scalar(elem) => {
                                Some((elem.clone(), *capacity))
                            }
                            crate::forge::model::ListElemType::Record { .. } => None,
                        })
                        .and_then(|(elem, capacity)| {
                            let (whole, _) = crate::forge::expr::called_names(&action.expr).ok()?;
                            Some((elem, capacity, whole?))
                        });
                    // The list is taken as the variable holds it: held to the
                    // variable's own bound, whatever the algorithm did past its own.
                    if let Some((elem, capacity, alias)) = &returned_list {
                        let symbol = renames.get(alias.as_str()).copied().unwrap_or(alias);
                        value = Receiving {
                            text: target.list_within(&value.text, *capacity, symbol, elem),
                            can_fail: true,
                        };
                    }
                    // A call of an algorithm that takes or returns state, taken whole: what
                    // an adapter gives back is the machine's own kind of value already. A
                    // list is held to the bound of the variable it is written to all the
                    // same: the algorithm's bound is advisory where lists grow, and the
                    // machine holds the same list wherever it runs.
                    let state_list = state_call.as_ref().and_then(|(adapter, ret)| match ret {
                        StaticSlot::ScalarList(_) | StaticSlot::RecordList(_) => rewrites
                            .lists
                            .get(location)
                            .map(|(_, capacity)| (adapter.as_str(), *capacity)),
                        _ => None,
                    });
                    if let Some((adapter, capacity)) = state_list {
                        value = Receiving {
                            text: target.state_list_within(adapter, &value.text, capacity),
                            can_fail: true,
                        };
                    }
                    let write = |v: &str| {
                        if returned_list.is_some() || state_list.is_some() {
                            target.assign_list(name, v)
                        } else if is_string {
                            target.assign_string(name, v)
                        } else if is_bytes {
                            target.assign_bytes(name, v)
                        } else {
                            target.assign(name, v)
                        }
                    };
                    // A target that names the type of the local the returned list
                    // is held in while it is computed.
                    let held_list = returned_list
                        .as_ref()
                        .and_then(|(_, _, alias)| {
                            target.list_result_type(renames.get(alias.as_str()).copied()?)
                        })
                        .or_else(|| {
                            state_list.and_then(|(adapter, _)| target.list_result_type(adapter))
                        });
                    // ... and the record an adapter gave back.
                    let held_record = state_call.as_ref().and_then(|(_, ret)| match ret {
                        StaticSlot::Record(alias) => {
                            Some(target.record_type(rewrites.machine, alias))
                        }
                        _ => None,
                    });
                    match &whole_record {
                        _ if held_record.is_some() && value.can_fail => (
                            target.receiving_write_of(
                                &write,
                                &value.text,
                                held_record.as_deref().unwrap_or_default(),
                                &failed(construct),
                            ),
                            true,
                        ),
                        _ if held_list.is_some() && value.can_fail => (
                            target.receiving_write_of(
                                &write,
                                &value.text,
                                held_list.as_deref().unwrap_or_default(),
                                &failed(construct),
                            ),
                            true,
                        ),
                        // A record made whole from a payload that can fail is
                        // held while it is computed in a value of its own type.
                        Some(held) if value.can_fail => (
                            target.receiving_write_of(
                                &write,
                                &value.text,
                                held,
                                &failed(construct),
                            ),
                            true,
                        ),
                        _ => statement(&value, slot, &write, construct),
                    }
                }
            };
        }
        "if" if !action.is_cpp_condition && !action.is_kt_condition => {
            reads_payload = reads(&action.cond);
            let cond = lower(&action.cond, InferredType::Bool)?;
            rewrites.note(&action.cond, action.spellings.get("cond"), &cond.text);
            action.native_cond = if cond.can_fail {
                action.native_cond_fails = true;
                target.receiving_condition(
                    &cond.text,
                    &failed(format!("<if cond='{}'>", action.cond)),
                    &target.condition_failed_flag(action.if_ordinal),
                )
            } else {
                cond.text
            };
            action.cond_constant = None;
        }
        "log" if !action.expr.trim().is_empty() => {
            reads_payload = reads(&action.expr);
            // Nothing is declared where a logged value lands, so any value
            // stands there.
            let value = lower(&action.expr, InferredType::Unknown)?;
            rewrites.note(&action.expr, action.spellings.get("expr"), &value.text);
            let label = action.label.clone();
            // The value's own type, which a target that formats by it reads, as
            // the value is lowered.
            let ty = crate::forge::expr::judge_into(
                &rewrites.as_lowered(&action.expr),
                ctx,
                crate::forge::expr::Expected::Hint(InferredType::Unknown),
            )
            .map_err(|r| {
                GenerateError::unsupported(format!(
                    "`{}` has no {lang} lowering: {}",
                    action.expr, r.error
                ))
            })?;
            (action.native_code, action.native_fails) = statement(
                &value,
                ty,
                &|v| target.log(&label, v, ty),
                "<log>".to_string(),
            );
        }
        // An append happens only while the list is under its bound — on
        // every backend, so a machine holds the same list wherever it runs.
        // Past the bound nothing is appended and `error.execution` says so —
        // the processor's own signal for an error in executing the document,
        // raised the way every other execution error of the backend is — and
        // the block ends (§scxml-4.9), as it does for a value that could
        // not be computed. So an append can always fail.
        "sce_append" => {
            reads_payload = reads(&action.expr);
            let list = action.location.trim();
            let (elem, capacity) = rewrites.lists.get(list).ok_or_else(|| {
                GenerateError::unsupported(format!(
                    "<sce:append target=\"{list}\"> names no list variable"
                ))
            })?;
            // A record is appended whole, as the record the expression names —
            // a record variable or a loop's record item — stands now: a plain
            // value the list then holds a copy of.
            let value = match elem {
                crate::forge::model::ListElemType::Scalar(elem) => {
                    lower(&action.expr, InferredType::from_sce_type(elem))?
                }
                // The payload of the event the transition is on is a record of
                // its schema too, taken whole as `_event.data`.
                crate::forge::model::ListElemType::Record { alias }
                    if action.expr.trim() == "_event.data" =>
                {
                    let schema = rewrites.schemas.get(alias).ok_or_else(|| {
                        GenerateError::unsupported(format!(
                            "<sce:append target=\"{list}\">: record:{alias} names no \
                             event-schema this build read"
                        ))
                    })?;
                    payload_record_value(rewrites.machine, alias, schema, target, &lower)?
                }
                crate::forge::model::ListElemType::Record { alias } => {
                    reads_payload = false;
                    let written = action.expr.trim();
                    let named = renames.get(written).copied().unwrap_or(written);
                    Receiving {
                        text: match rewrites.schemas.get(alias) {
                            Some(schema) => target.record_copy(named, schema),
                            None => named.to_string(),
                        },
                        can_fail: false,
                    }
                }
            };
            let name = renames.get(list).copied().unwrap_or(list);
            let overflow = if rewrites.raises_error {
                target.raise_execution_error(
                    rewrites.machine,
                    &format!(
                        "<sce:append target='{list}'>: the list already holds its capacity of \
                         {capacity}"
                    ),
                )
            } else {
                String::new()
            };
            action.native_code = target.append(
                name,
                *capacity,
                &value.text,
                value.can_fail,
                &overflow,
                &failed(format!("<sce:append target='{list}'>")),
            );
            action.native_fails = true;
            // The whole element, for a target that rewrites the document
            // itself: its statement is the element that does the append, and
            // holds the value, so the value is not noted beside it.
            rewrites.note_element(action.spellings.get("target"), &action.native_code);
        }
        "sce_clear" => {
            let list = action.location.trim();
            let name = renames.get(list).copied().unwrap_or(list);
            action.native_code = target.clear(name);
            rewrites.note_element(action.spellings.get("target"), &action.native_code);
        }
        // A `<foreach>` walks a list variable; its loop variables are the
        // body's own, so the body is lowered in a scope that has them and
        // nothing else is nested below it (§scxml-4.6).
        "foreach" => {
            let list = action.array.trim();
            let (elem, capacity) = rewrites.lists.get(list).ok_or_else(|| {
                GenerateError::unsupported(format!(
                    "<foreach array=\"{list}\"> names no list variable"
                ))
            })?;
            let item = action.item.trim();
            let index = Some(action.index.trim()).filter(|name| !name.is_empty());
            // The loop binds its variables under the spelling every expression
            // of the body reads them by, the target's for any local: the type
            // scope and the renames stay keyed by the names as written.
            let expr_target = target.expr_target();
            let item_local = expr_target.local_spelling(item);
            let index_local = index.map(|index| expr_target.local_spelling(index));
            // A loop variable the generated machine already has a field of
            // that name would hide it in a backend that reaches fields by
            // their bare names.
            if let Some((clash, _)) = [Some((item, &item_local)), index.zip(index_local.as_ref())]
                .into_iter()
                .flatten()
                .find(|(_, local)| renames.values().any(|field| field == local))
            {
                return Err(GenerateError::unsupported(format!(
                    "<foreach> names a loop variable `{clash}` that is the name {lang} gives a \
                     field of the machine"
                )));
            }
            let loop_variables =
                crate::forge::type_ctx::LoopVariables::new(item, index, elem, &rewrites.schemas);
            let mut inner = loop_variables.bind(ctx);
            // A fact proven before the loop holds in its body only if nothing in
            // the body ends it, as the judge read it (docs/adr/0055, section 3).
            inner.proven = crate::forge::narrowing::loop_entry(&ctx.proven, &action.actions);
            let name = renames.get(list).copied().unwrap_or(list);
            // What a target that has to name the item's type and the list it
            // copies is told: the element's type and the list's own.
            let (item_ty, list_ty) = match elem {
                crate::forge::model::ListElemType::Scalar(scalar) => (
                    target.scalar_type(scalar),
                    target.bounded_list(scalar, Some(*capacity)).ty,
                ),
                crate::forge::model::ListElemType::Record { alias } => {
                    let record_ty = target.record_type(rewrites.machine, alias);
                    let list = target.bounded_record_list(&record_ty, Some(*capacity));
                    (record_ty, list.ty)
                }
            };
            if let Some((head, prologue)) = target.foreach_loop_typed(
                name,
                &item_local,
                index_local.as_deref(),
                &item_ty,
                &list_ty,
            ) {
                action.native_loop = head;
                action.native_loop_prologue = prologue;
                action.native_loop_first = target
                    .foreach_first(name, &item_local, index_local.as_deref())
                    .unwrap_or_default();
            }
            // A record item is a record a `<send>` of the body may take whole.
            let holds_a_record = match elem {
                crate::forge::model::ListElemType::Record { alias } => Some(alias.clone()),
                crate::forge::model::ListElemType::Scalar(_) => None,
            };
            if let Some(alias) = &holds_a_record {
                rewrites
                    .loop_records
                    .borrow_mut()
                    .push((item.to_string(), alias.clone()));
            }
            // A string field of a record item, for a target that reads it through
            // the buffer that holds it, is read through it in the body.
            let item_reads: Vec<(String, String)> = loop_variables
                .string_fields()
                .filter_map(|field| {
                    target
                        .record_string_read(&item_local, field)
                        .map(|read| (format!("{item}.{field}"), read))
                })
                .collect();
            let inner_renames: HashMap<&str, &str> = renames
                .iter()
                .map(|(from, to)| (*from, *to))
                .chain(
                    item_reads
                        .iter()
                        .map(|(from, to)| (from.as_str(), to.as_str())),
                )
                .collect();
            let lowered = lower_actions(&mut action.actions, &inner, &inner_renames, rewrites);
            if holds_a_record.is_some() {
                rewrites.loop_records.borrow_mut().pop();
            }
            return lowered;
        }
        // A host operation whose arguments the Interpreter's engine computes
        // when the action runs: each is lowered as any expression is, to the
        // value the host takes (a wide integer passes the library's `out`).
        "native_action" if target.lowers_host_action_arguments() => {
            for param in &mut action.params {
                reads_payload |= reads(&param.expr);
                let value = lower(&param.expr, InferredType::Unknown)?;
                rewrites.note(&param.expr, param.expr_spelling.as_ref(), &value.text);
            }
        }
        // A host operation a generated backend declares as a method of its host
        // interface (`forge::native_action` renders the call): an argument that may
        // be absent is the parameter of the backend's optional type over its element
        // (docs/adr/0055, step 10), which a target with no spelling of one for that
        // element refuses by name. Validation judged the arguments; one it refused
        // is not asked about again.
        "native_action" => {
            for param in &action.params {
                if let Ok(FieldType::Optional { elem }) =
                    crate::forge::native_action::static_argument_type(ctx, param)
                {
                    if !target.holds_host_optional(&elem) {
                        return Err(optional_not_lowered(
                            &format!(
                                "`<sce:arg expr=\"{}\">` of the host operation `{}`, of type \
                                 optional<{}>",
                                param.expr,
                                action.native_action_name,
                                crate::forge::native_action::canonical_type(&elem)
                            ),
                            lang,
                        ));
                    }
                    rewrites.host_optional.set(true);
                }
            }
        }
        // What a `<send>` carries is read from the machine's fields now, when
        // it runs (§scxml-6.2.3 evaluates its arguments once, at the send).
        "send" => {
            // A record named by `<content expr>` is the pairs of its fields,
            // and the names of a `namelist` are params like any other: one list
            // of pairs is read from here on, each value as the machine holds it
            // when the send runs.
            //
            // An expression that names no record is the one value the event
            // carries: it is lowered below, once the pairs are, as the typed
            // value its own type is (`Action::native_content_value`).
            let content = action.contentexpr.trim().to_string();
            let mut content_value = false;
            if !content.is_empty() {
                match rewrites.content_fields(&content) {
                    Some(fields) => action.fold_content_record_into_params(&content, &fields),
                    None if target.lowers_scalar_content() => content_value = true,
                    None => {
                        return Err(GenerateError::unsupported(format!(
                            "a <send> whose <content expr=\"{content}\"> names a value has no \
                             {lang} lowering yet"
                        )))
                    }
                }
            }
            action.fold_namelist_into_params();
            let noted = rewrites.sites.borrow().len();
            for param in &mut action.params {
                lower_wire_param(param, ctx, renames, rewrites)?;
                // A value read from the payload runs only for a delivery that
                // carried one, as the rest of the transition's content does.
                reads_payload |=
                    reads(crate::forge::static_datamodel::WireParam::of_param(param).written);
            }
            // The pairs of a record are the fields of the one value `<content
            // expr>` names, which a backend that runs the document's own
            // element reads as the object it is: they have no attribute of
            // their own to be rewritten at.
            if !content.is_empty() && !content_value {
                rewrites.sites.borrow_mut().truncate(noted);
            }
            // The value is read once, as a param's is, and the attribute is
            // cleared once it is lowered so no template evaluates it a second
            // time: the machine has no engine to do it with.
            if content_value {
                reads_payload |= reads(&content);
                let view = crate::forge::static_datamodel::WireParam::of_content(
                    &content,
                    action.contentexpr_spelling.as_ref(),
                );
                let lowered = lower_wire_value(&view, ctx, renames, rewrites)?;
                if let Some((value, fails)) = lowered {
                    action.native_content_value = value;
                    action.native_content_value_fails = fails;
                }
                action.contentexpr.clear();
            }
            // A delay computed from the machine's fields is the string it
            // spells, read as a CSS2 time when the send runs; a value that is
            // none, or an operation that fails, is an argument that cannot be
            // evaluated, which keeps the message from being sent.
            if !action.delayexpr.trim().is_empty() {
                if !target.lowers_delay_expr() {
                    return Err(GenerateError::unsupported(format!(
                        "a <send> with a delayexpr has no {lang} lowering yet"
                    )));
                }
                reads_payload |= reads(&action.delayexpr);
                let value = lower(&action.delayexpr, InferredType::Str)?;
                rewrites.note(
                    &action.delayexpr,
                    action.spellings.get("delayexpr"),
                    &value.text,
                );
                action.native_delay = value.text;
                action.native_delay_fails = value.can_fail;
            }
            // An event named by an expression is the string it computes, which
            // the backend resolves to the event it delivers; a name that is empty,
            // or an operation that fails, is an argument that cannot be
            // evaluated, which keeps the message from being sent.
            if !action.eventexpr.trim().is_empty() {
                if !target.lowers_event_expr() {
                    return Err(GenerateError::unsupported(format!(
                        "a <send> with an eventexpr has no {lang} lowering yet"
                    )));
                }
                reads_payload |= reads(&action.eventexpr);
                let value = lower(&action.eventexpr, InferredType::Str)?;
                rewrites.note(
                    &action.eventexpr,
                    action.spellings.get("eventexpr"),
                    &value.text,
                );
                action.native_event = value.text;
                action.native_event_fails = value.can_fail;
            }
            // A target named by an expression is the string it computes, which
            // the machine holds to the routes the document declares before it
            // sends by the one that matches; a value none of them names is an
            // address nobody is at. The attribute stays, as an `eventexpr` does:
            // it is what says the target is computed, and the declaration is what
            // makes the computation finite.
            if !action.targetexpr.trim().is_empty() {
                reads_payload |= reads(&action.targetexpr);
                let value = lower(&action.targetexpr, InferredType::Str)?;
                match target.target_expr_site(&value.text, &action.targets) {
                    Some(site) => rewrites.note(
                        &action.targetexpr,
                        action.spellings.get("targetexpr"),
                        &site,
                    ),
                    None => {
                        action.native_target = value.text;
                        action.native_target_fails = value.can_fail;
                    }
                }
            }
            // The id the machine generates for the send is written to the
            // variable `idlocation` names before any other argument is read, and
            // is then the id the send is known by, read back from that variable
            // as the id a `<cancel>` computes is. The judge held the variable's
            // bound to one the id fits, so the write has nothing to fail at.
            if !action.idlocation.trim().is_empty() {
                let location = action.idlocation.trim().to_string();
                let name = renames
                    .get(location.as_str())
                    .copied()
                    .unwrap_or(location.as_str());
                action.native_idlocation = target
                    .assign_fresh_send_id(name, rewrites.strings.contains_key(location.as_str()));
                let held = lower(&location, InferredType::Str)?;
                action.native_sendid = held.text;
                action.native_sendid_fails = held.can_fail;
            }
            // A literal `<content>` is the event's data as written, finished
            // here ([`crate::filters::static_content_wire`]): the machine has
            // no engine to evaluate the text with.
            if !action.content.trim().is_empty() {
                action.native_content = crate::filters::static_content_wire(&action.content);
                // Where the text is written, for a backend that runs the
                // document's own `<content>`: the same finished text, which an
                // engine reads as the string it spells and not as the value it
                // could be read as.
                rewrites.note(
                    &action.content,
                    action.content_text_spelling.as_ref(),
                    &action.native_content,
                );
            }
        }
        // The id of the send a `<cancel>` removes is the string its expression
        // computes, read from the machine's fields when the cancel runs. An id
        // no send holds cancels nothing; an operation that fails is an argument
        // that cannot be evaluated, which raises `error.execution` and ends the
        // block, as every error in executable content does.
        "cancel" if !action.sendidexpr.trim().is_empty() => {
            if !target.lowers_cancel_expr() {
                return Err(GenerateError::unsupported(format!(
                    "a <cancel> with a sendidexpr has no {lang} lowering yet"
                )));
            }
            reads_payload |= reads(&action.sendidexpr);
            let value = lower(&action.sendidexpr, InferredType::Str)?;
            rewrites.note(
                &action.sendidexpr,
                action.spellings.get("sendidexpr"),
                &value.text,
            );
            action.native_sendid = value.text;
            action.native_sendid_fails = value.can_fail;
        }
        _ => {}
    }
    Ok(lower_nested(action, ctx, renames, rewrites)? || reads_payload)
}

/// The whole value `expr` gives `place`, an optional variable or an optional
/// field of a record variable, or a child's optional variable a `<param>` hands
/// a value (docs/adr/0055): the one rule every target writes the whole value of
/// an optional place by, `construct` naming where it is written in a refusal.
/// The literal `null` is the absent value ([`StaticTarget::optional_none`]); an
/// optional value is written as it is, its present text held to the place's
/// bound ([`StaticTarget::optional_map`]); a value of `T` is lowered into the
/// place's element — so the narrowing of an integer and the bound of a text are
/// the element's, as a write of a place of `T` is held to them — and lifted
/// ([`StaticTarget::optional_some`]). What inside an expression is absent,
/// lifted, tested, read or compared is the emitter's
/// ([`crate::forge::expr::OptionalForm`]).
fn optional_whole_value(
    expr: &str,
    place: &OptionalPlace,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
    construct: &str,
) -> Result<Receiving, GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    let lower = |text: &str, slot: InferredType| {
        rewrites.owned(text, ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no {lang} lowering: {}", r.error))
        })
    };
    let optional = InferredType::Optional(crate::forge::types::OptionalElem::of(&place.elem));
    // A present text, held to the bound of the place it is written to, as the
    // place's present value is held: the buffer of the bound, for a target that
    // holds a text in one, as a payload's field taken whole into a record is.
    let held = |value: &str| match (&place.elem, place.bound) {
        (SceType::Bytes, Some(bound)) => {
            target.record_bytes_runtime(bound, &target.bounded_bytes(value, bound))
        }
        (_, Some(bound)) => {
            target.record_string_runtime(bound, &target.bounded_string(value, bound))
        }
        (_, None) => value.to_string(),
    };
    if is_null_literal(expr) {
        return Ok(Receiving {
            text: absent_value(target, &place.present, construct)?,
            can_fail: false,
        });
    }
    // Typed as it is lowered: a call of an algorithm that takes a record or
    // a list is its adapter's, which answers the optional value the algorithm
    // returns.
    let written = crate::forge::expr::infer_expr_type(&rewrites.as_lowered(expr), ctx)
        .unwrap_or(InferredType::Unknown);
    if written.is_optional() {
        // The judge admits an optional value of another width; no target
        // spells the narrowing of a present value inside one yet.
        if written != optional {
            return Err(GenerateError::unsupported(format!(
                "{construct}: `{}` is {}, written to {}: an optional value of another \
                 element has no {lang} lowering yet (docs/adr/0055)",
                expr.trim(),
                written.describe(),
                optional.describe()
            )));
        }
        let value = lower(expr, optional)?;
        return Ok(if place.bound.is_some() {
            Receiving {
                text: target
                    .optional_map(&value.text, &place.present, &held)
                    .ok_or_else(|| optional_not_lowered(construct, lang))?,
                can_fail: true,
            }
        } else {
            value
        });
    }
    let value = lower(expr, InferredType::from_sce_type(&place.elem))?;
    let (text, can_fail) = if place.bound.is_some() {
        (held(&value.text), true)
    } else {
        (value.text, value.can_fail)
    };
    Ok(Receiving {
        text: present_value(target, &place.present, &text, construct)?,
        can_fail,
    })
}

/// An `<assign>` to `place`, an optional variable or an optional field of a
/// record variable (docs/adr/0055), in place: the whole value it gives the
/// place is [`optional_whole_value`]'s, written by the target's own statement,
/// and a value that can fail is received where its failure is.
fn lower_optional_assign(
    action: &mut Action,
    place: &OptionalPlace,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    let location = action.location.trim().to_string();
    let construct = format!("<assign location='{location}'>");
    let optional = InferredType::Optional(crate::forge::types::OptionalElem::of(&place.elem));
    let value = optional_whole_value(&action.expr, place, ctx, renames, rewrites, &construct)?;
    let record_field = field_location(&location, rewrites);
    // A target that rewrites the document itself writes a record's field as an
    // `<assign>` of the whole record, changed in that field.
    let rewritten = record_field.and_then(|(var, field)| {
        let name = renames.get(var).copied().unwrap_or(var);
        target.field_assignment(name, field.trim(), &value.text)
    });
    match &rewritten {
        Some((record, expr)) => {
            rewrites.note(&action.location, action.spellings.get("location"), record);
            rewrites.note(&action.expr, action.spellings.get("expr"), expr);
        }
        None => rewrites.note(&action.expr, action.spellings.get("expr"), &value.text),
    }
    let write = |v: &str| match record_field {
        Some((var, field)) => write_field(
            rewrites,
            var,
            renames.get(var).copied().unwrap_or(var),
            &target.record_field(field.trim()),
            v,
            FieldValue::Plain,
        )
        .unwrap_or_default(),
        None => target.assign(
            renames
                .get(location.as_str())
                .copied()
                .unwrap_or(location.as_str()),
            v,
        ),
    };
    (action.native_code, action.native_fails) = if value.can_fail {
        (
            target.receiving_write(
                &write,
                &value.text,
                optional,
                &execution_failure(rewrites, &construct),
            ),
            true,
        )
    } else {
        (write(&value.text), false)
    };
    Ok(())
}

/// An `<assign>` to `place`, an optional record variable (docs/adr/0055, step 9),
/// in place: the whole value it takes, by the one rule every target writes it by.
/// The literal `null` is the absent value ([`StaticTarget::optional_none`]); a
/// record is lifted into the present value ([`StaticTarget::optional_some`]) —
/// one by the name of a record variable or a loop's record item, the payload of
/// the event the transition is on made whole ([`payload_record_value`]), an element
/// of a list of its schema read whole at an index ([`StaticTarget::record_at`]),
/// or what an imported algorithm returns as a record — and an optional record
/// is written as it is: the name of another optional record variable, or what an
/// imported algorithm returns as one. Which of these the judge admitted
/// ([`crate::forge::static_datamodel`]) is not asked again.
fn lower_optional_record_assign(
    action: &mut Action,
    place: &OptionalRecordPlace,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    let location = action.location.trim().to_string();
    let construct = format!("<assign location='{location}'>");
    let lower = |text: &str, slot: InferredType| {
        rewrites.owned(text, ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no {lang} lowering: {}", r.error))
        })
    };
    // A record lifted into the present value of the optional record.
    let lifted = |record: Receiving| -> Result<Receiving, GenerateError> {
        Ok(Receiving {
            text: present_value(target, &place.present, &record.text, &construct)?,
            can_fail: record.can_fail,
        })
    };
    let written = action.expr.trim().to_string();
    let value = if is_null_literal(&written) {
        Receiving {
            text: absent_value(target, &place.present, &construct)?,
            can_fail: false,
        }
    } else if written == crate::forge::event_schema_check::EVENT_DATA_PATH {
        lifted(payload_record_value(
            rewrites.machine,
            &place.alias,
            &place.schema,
            target,
            &lower,
        )?)?
    } else if let Some((list, index)) = indexed_element(&written) {
        let lowered = lower(index, InferredType::Unknown)?;
        let named = renames.get(list).copied().unwrap_or(list);
        let element = target
            .record_element(named, &lowered.text, &place.schema)
            .ok_or_else(|| {
                optional_not_lowered(
                    &format!("{construct}: an element of a list of records taken whole"),
                    lang,
                )
            })?;
        lifted(Receiving {
            text: element,
            can_fail: true,
        })?
    } else if rewrites.optional_records.contains_key(written.as_str()) {
        // Another optional record, absent or present, is a copy of it, as a
        // record is: what holds it keeps its own.
        let value = lower(&written, InferredType::Unknown)?;
        Receiving {
            text: target.record_copy(&value.text, &place.schema),
            can_fail: value.can_fail,
        }
    } else {
        let value = lower(&written, InferredType::Unknown)?;
        // What an imported algorithm answers as an optional record is optional
        // already; any other record is a present value of it.
        let optional_already = matches!(
            rewrites.whole_state_call(&written),
            Some((_, StaticSlot::OptionalRecord(_)))
        );
        if optional_already {
            value
        } else {
            lifted(Receiving {
                text: target.record_copy(&value.text, &place.schema),
                can_fail: value.can_fail,
            })?
        }
    };
    rewrites.note(&action.expr, action.spellings.get("expr"), &value.text);
    let name = renames
        .get(location.as_str())
        .copied()
        .unwrap_or(location.as_str());
    let write = |v: &str| target.assign(name, v);
    let optional_ty = target.optional_type(&place.present).unwrap_or_default();
    (action.native_code, action.native_fails) = if value.can_fail {
        (
            target.receiving_write_of(
                &write,
                &value.text,
                &optional_ty,
                &execution_failure(rewrites, &construct),
            ),
            true,
        )
    } else {
        (write(&value.text), false)
    };
    Ok(())
}

/// `<assign location="last" expr="days[i]">` for a target that cannot read an element
/// whole: one `<assign location="last.f" expr="days[i].f">` for each field `f` of the
/// schema, lowered as any such assignment is and written one after another in the
/// statement the action lands in. A read that fails — an index outside the list fails
/// the first, and the same index fails every one — ends the block there (§scxml-4.9),
/// before anything is written.
///
/// `list` and `index` are as the document spells them; each synthetic assignment
/// renames them as the document's own would be.
fn lower_element_fields(
    action: &mut Action,
    list: &str,
    index: &str,
    schema: &EventSchemaModel,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let record = action.location.trim().to_string();
    let mut statements = Vec::new();
    let mut fails = Vec::new();
    let mut reads_payload = false;
    for field in &schema.fields {
        let mut copy = action.clone();
        copy.location = format!("{record}.{}", field.id);
        copy.expr = format!("{list}[{index}].{}", field.id);
        // Nothing of the copy is written in the document: no site to note.
        copy.spellings = Default::default();
        reads_payload |= lower_action(&mut copy, ctx, renames, rewrites)?;
        fails.push(copy.native_fails);
        statements.push(copy.native_code);
    }
    // Every read of an element can fail, so every statement does. A schema whose copy
    // were a mix of the two would have no one way to join them.
    if fails.iter().any(|f| *f) != fails.iter().all(|f| *f) {
        return Err(GenerateError::unsupported(format!(
            "`{record} = {list}[{index}]`: some fields of the record are copied by a statement \
             that can fail and some by one that cannot, which {} has no way to run in order",
            rewrites.target.name()
        )));
    }
    let can_fail = fails.first().copied().unwrap_or(false);
    action.native_code = if can_fail {
        rewrites.target.in_sequence(&statements)
    } else {
        statements.join("\n")
    };
    action.native_fails = can_fail;
    Ok(reads_payload)
}

/// Lower the value of a `<param>` that crosses as data (SCE Accepted Subset
/// §2.15): the expression, read from the machine's fields, as the typed value
/// the backend's wire helpers take ([`StaticTarget::wire_value`]), and whether
/// it can fail. `None` for one with nothing to lower: a static literal, which
/// is folded at build time and left as it is, and a param with no expression.
///
/// Validation already judged the expression against the same scope and held
/// its type to [`InferredType::wire_param_slot`], or took it for an enum value
/// held by a variable, which crosses as a string ([`StaticTarget::enum_wire_name`]),
/// so a refusal here is a lowering this backend lacks, not a mistake in the
/// document.
fn lower_wire_value(
    param: &crate::forge::static_datamodel::WireParam<'_>,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<Option<(String, bool)>, GenerateError> {
    let written = param.written;
    if param.is_static_literal || written.trim().is_empty() {
        return Ok(None);
    }
    let target = rewrites.target;
    let lang = target.name();
    let construct = if param.name.is_empty() {
        "<content expr>".to_string()
    } else {
        format!("<param name=\"{}\">", param.name)
    };
    let refused = |why: String| {
        GenerateError::unsupported(format!(
            "{construct} `{written}` has no {lang} lowering: {why}"
        ))
    };
    // Typed as it is lowered ([`Rewrites::as_lowered`]).
    let ty = crate::forge::expr::judge_into(
        &rewrites.as_lowered(written),
        ctx,
        crate::forge::expr::Expected::Hint(InferredType::Unknown),
    )
    .map_err(|r| refused(r.error.to_string()))?;
    // An enum value crosses as the name its enum declares, a string: the value
    // is lowered as the enum value it is, and the backend names the variant it
    // holds. The judge accepted only a variable or a field of a record variable
    // as one, which is what `enum_vars` holds.
    let held_by_a_variable = |name: &str| rewrites.enum_of_wire_value(name);
    let reading = crate::forge::static_enum::EnumReading {
        var_alias: &held_by_a_variable,
        calls: &|name| rewrites.scope.call_enums(name),
    };
    // An optional value crosses as JSON: `null` where it is absent, and the
    // wire value of its present value where it is present (docs/adr/0055,
    // section 4). The judge refused it on a carrier that is text alone. Which
    // enum an optional enum value is of is what the judge read of it
    // ([`crate::forge::static_enum::Held`]).
    if let InferredType::Optional(elem) = ty {
        let held = crate::forge::static_enum::value_held(written, ctx, &reading)
            .map_err(|r| refused(r.error.to_string()))?;
        // The present value's type, and the enum it is of, if it is one.
        let (present, enum_alias) = match (&held, elem.scalar()) {
            (crate::forge::static_enum::Held::Optional(alias), _) => {
                let present = target.enum_type(rewrites.machine, alias).ok_or_else(|| {
                    refused("an optional value of an enum has no type here".to_string())
                })?;
                (present, Some(alias.clone()))
            }
            (_, Some(scalar)) => (target.scalar_type(&scalar), None),
            (_, None) => {
                return Err(refused(format!("{} has no wire spelling", ty.describe())));
            }
        };
        // The wire value of a present value named `v`, as a present value of
        // `T` crosses (the rest of this function).
        let slot = elem.present();
        let bytes = slot == InferredType::Bytes;
        let wire = |v: &str| match &enum_alias {
            Some(alias) => target.wire_value(
                InferredType::Str,
                &target.enum_wire_name(alias, v).unwrap_or_default(),
            ),
            None if bytes => target.wire_bytes(v).unwrap_or_default(),
            None => target.wire_value(slot, v),
        };
        let spelled = match &enum_alias {
            Some(alias) => target.enum_wire_name(alias, "v").is_some(),
            None if bytes => target.wire_bytes("v").is_some(),
            None => target.wire_admits(slot),
        };
        if !spelled {
            return Err(refused(format!(
                "a present value of {} has no wire spelling here yet",
                ty.describe()
            )));
        }
        let value = rewrites
            .owned(written, ctx, renames, ty)
            .map_err(|r| refused(r.error.to_string()))?;
        let wired = target
            .wire_optional(&value.text, &present, &wire)
            .ok_or_else(|| optional_not_lowered(&construct, lang))?;
        if target.lowers_host_action_arguments() {
            rewrites.note(written, param.spelling, &wired);
        } else {
            rewrites.note(written, param.spelling, &value.text);
        }
        return Ok(Some((wired, value.can_fail)));
    }
    if let Some(alias) = crate::forge::static_enum::value_enum(written, ctx, &reading)
        .map_err(|r| refused(r.error.to_string()))?
    {
        let value = rewrites
            .owned(written, ctx, renames, InferredType::Unknown)
            .map_err(|r| refused(r.error.to_string()))?;
        let name = target.enum_wire_name(&alias, &value.text).ok_or_else(|| {
            refused("a value of an enum has no wire spelling here yet".to_string())
        })?;
        rewrites.note(written, param.spelling, &value.text);
        return Ok(Some((
            target.wire_value(InferredType::Str, &name),
            value.can_fail,
        )));
    }
    let slot = ty
        .wire_param_slot()
        .ok_or_else(|| refused("its type has no wire spelling".to_string()))?;
    // A byte string crosses as its Latin-1 text, which each backend spells in the
    // type it carries a string in; one that does not yet is refused by name.
    let bytes = slot == InferredType::Bytes;
    if !bytes && !target.wire_admits(slot) {
        return Err(refused(format!(
            "a value of {slot:?} has no wire spelling here yet"
        )));
    }
    // The value is read once and is its own: an owned string, for the typed
    // value that carries it.
    let value = rewrites
        .owned(written, ctx, renames, slot)
        .map_err(|r| refused(r.error.to_string()))?;
    let wired = if bytes {
        target
            .wire_bytes(&value.text)
            .ok_or_else(|| refused("a byte string has no wire spelling here yet".to_string()))?
    } else {
        target.wire_value(slot, &value.text)
    };
    // A target that leaves the expression in the document (the Interpreter) writes
    // the value as it crosses: a 64-bit integer as the text of its digits.
    if target.lowers_host_action_arguments() && !bytes {
        rewrites.note(written, param.spelling, &wired);
    } else {
        rewrites.note(written, param.spelling, &value.text);
    }
    Ok(Some((wired, value.can_fail)))
}

/// Lower the value of a `<param>` of a `<send>` or of a host-run `<invoke>`, in
/// place: `Param::native_value`, and whether it can fail.
fn lower_wire_param(
    param: &mut crate::model::Param,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let view = crate::forge::static_datamodel::WireParam::of_param(param);
    if let Some((value, fails)) = lower_wire_value(&view, ctx, renames, rewrites)? {
        param.native_value = value;
        param.native_fails = fails;
    }
    Ok(())
}

/// Lower the `srcexpr` and the `<content expr>` of an `<invoke>` a host runs, in
/// place: each is the string it computes, which the host is handed as the
/// request's `src` ([`crate::model::UnsupportedInvokeInfo::native_src`]) and as
/// its `content`, the body the service runs
/// ([`crate::model::UnsupportedInvokeInfo::native_content`]), and whether it can
/// fail. The attribute is cleared once it is lowered, so no template evaluates
/// it a second time. Read when the invocation starts, where no event's payload
/// is in scope, so the lowering is given none.
fn lower_host_request_strings(
    info: &mut crate::model::UnsupportedInvokeInfo,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    let lower = |written: &str, attribute: &str| -> Result<(String, bool), GenerateError> {
        if !target.lowers_host_src_expr() {
            return Err(GenerateError::unsupported(format!(
                "an <invoke> a host runs with a {attribute} has no {lang} lowering yet"
            )));
        }
        let value = rewrites
            .owned(written, ctx, renames, InferredType::Str)
            .map_err(|r| {
                GenerateError::unsupported(format!(
                    "`{written}` has no {lang} lowering: {}",
                    r.error
                ))
            })?;
        Ok((value.text, value.can_fail))
    };
    if !info.srcexpr.trim().is_empty() {
        (info.native_src, info.native_src_fails) = lower(&info.srcexpr, "srcexpr")?;
        info.srcexpr.clear();
    }
    if !info.contentexpr.trim().is_empty() {
        (info.native_content, info.native_content_fails) =
            lower(&info.contentexpr, "<content expr>")?;
        info.contentexpr.clear();
    }
    Ok(())
}

/// Lower a child session's `<invoke idlocation>`, in place: the statement that
/// writes the id each start generates to the variable the attribute names
/// ([`crate::model::InvokeSessionCommon::native_idlocation`]; docs/adr/0045).
/// Validation already held the variable to a string the id fits
/// ([`crate::forge::static_datamodel`]), so the write has nothing to fail at; a
/// target that does not lower it is refused by name.
fn lower_invoke_idlocation(
    common: &mut crate::model::InvokeSessionCommon,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let location = common.base.idlocation.trim().to_string();
    if location.is_empty() {
        return Ok(());
    }
    let target = rewrites.target;
    let Some(id) = target.generated_invoke_id() else {
        return Err(GenerateError::unsupported(format!(
            "an <invoke idlocation> under datamodel=\"sce-static\" has no {} lowering yet",
            target.name()
        )));
    };
    let name = renames
        .get(location.as_str())
        .copied()
        .unwrap_or(location.as_str());
    common.native_idlocation = target.assign_string(name, id);
    Ok(())
}

/// Lower a Mesh request, in place: its `srcexpr` to the string it computes, which
/// the machine reads when the invocation starts to name the peer it asks
/// ([`crate::model::MeshRpcInvokeInfo::native_src`]), and each `<param>` to the
/// value the request carries. The attribute is left standing, since the templates
/// read it as "this request resolves its peer when it starts" and only the site that
/// evaluates it prefers the lowered string. Read when the invocation starts, where
/// no event's payload is in scope, so the lowering is given none; the event name and
/// the deadline are constants of the build and never reach here.
///
/// Validation already held the expression to a string
/// ([`crate::forge::static_datamodel`]), so a refusal here is a lowering this
/// backend lacks, not a mistake in the document.
fn lower_mesh_request(
    info: &mut crate::model::MeshRpcInvokeInfo,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    if let crate::model::MeshRpcTarget::SrcExpr { srcexpr } = &info.target {
        let lang = target.name();
        let value = rewrites
            .owned(srcexpr, ctx, renames, InferredType::Str)
            .map_err(|r| {
                GenerateError::unsupported(format!(
                    "`{srcexpr}` has no {lang} lowering: {}",
                    r.error
                ))
            })?;
        info.native_src = value.text;
        info.native_src_fails = value.can_fail;
    }
    for param in &mut info.base.params {
        lower_wire_param(param, ctx, renames, rewrites)?;
    }
    Ok(())
}

/// Lower the value of each `<param>` of a `<final>`'s `<donedata>`, in place:
/// `DoneDataParam::native_value`, and whether it can fail. The value is read
/// when the `<final>` is entered, where no event's payload is in scope.
fn lower_done_params(
    params: &mut [crate::model::DoneDataParam],
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    for param in params {
        let view = crate::forge::static_datamodel::WireParam::of_done_param(param);
        if let Some((value, fails)) = lower_wire_value(&view, ctx, renames, rewrites)? {
            param.native_value = value;
            param.native_fails = fails;
        }
    }
    Ok(())
}

/// Lower what an `<invoke type="scxml">` hands its child (§scxml-6.4.1): each
/// `<param>` and each `namelist` name, as the value of the child's variable of
/// the same name, in the backend's own language — `Param::native_seed`, and
/// whether it can fail. The `namelist` is folded into the params
/// ([`crate::model::ScxmlInvokeInfo::arguments`]) and cleared, so the
/// templates read one list.
///
/// Validation already held each value to the type of the variable it lands in
/// ([`crate::forge::static_datamodel`]), so a refusal here is a lowering this
/// backend lacks, not a mistake in the document.
fn lower_child_arguments(
    info: &mut crate::model::ScxmlInvokeInfo,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let mut arguments = info.arguments();
    if arguments.is_empty() {
        return Ok(());
    }
    let declared = info.common.child_static_variables.as_deref().unwrap_or(&[]);
    for param in &mut arguments {
        let variable = declared.iter().find(|v| v.id == param.name);
        lower_child_argument(param, variable, ctx, renames, rewrites)?;
    }
    info.common.base.params = arguments;
    info.namelist.clear();
    Ok(())
}

/// Lower one argument an `<invoke>` hands a child to the value of the child's
/// `variable` of the same name (§scxml-6.4.1): `Param::native_seed`, the type of
/// the variable it lands in, and whether it can fail. A variable of `optional<T>`
/// takes the whole optional value, flagged `Param::native_seed_optional`.
fn lower_child_argument(
    param: &mut crate::model::Param,
    variable: Option<&crate::model::Variable>,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    let (written, spelling) = if param.expr.trim().is_empty() {
        (param.location.clone(), param.location_spelling.clone())
    } else {
        (param.expr.clone(), param.expr_spelling.clone())
    };
    let refused = |why: String| {
        GenerateError::unsupported(format!(
            "<param name=\"{}\"> `{written}` has no {lang} lowering: {why}",
            param.name
        ))
    };
    // A child's variable of `optional<T>` is handed the whole optional value
    // (docs/adr/0055, step 10), by the rule an `<assign>` to it writes one by:
    // the literal `null` is absent, a value of `T` is lifted into its element,
    // and a text is held to the bound the child declared. Absent is a value not
    // handed, which is the child's own start: the judge held a variable that
    // starts at a value to a value that is never absent.
    let optional = variable
        .filter(|v| crate::forge::static_datamodel::seed_slot(v).is_some())
        .and_then(|v| {
            let elem = v.value_type.as_ref()?.optional_elem()?.scalar()?;
            Some((elem, v.capacity))
        });
    if let Some((elem, capacity)) = optional {
        let bound = matches!(elem, SceType::String)
            .then_some(capacity)
            .flatten();
        let place = OptionalPlace {
            elem: elem.clone(),
            present: optional_present(target, rewrites.machine, elem, bound),
            bound,
        };
        let construct = format!("<param name=\"{}\"> `{written}`", param.name);
        let value = optional_whole_value(&written, &place, ctx, renames, rewrites, &construct)?;
        rewrites.note(&written, spelling.as_ref(), &value.text);
        param.native_seed_type = target
            .optional_type(&place.present)
            .ok_or_else(|| optional_not_lowered(&construct, lang))?;
        param.native_seed = value.text;
        param.native_seed_optional = true;
        param.native_fails = value.can_fail;
        return Ok(());
    }
    let (Some(slot), Some(held)) = (
        variable.and_then(crate::forge::static_datamodel::seed_slot),
        variable
            .and_then(|v| v.value_type.as_ref())
            .and_then(|t| t.scalar()),
    ) else {
        return Err(refused(
            "the child declares no variable it can be handed to".into(),
        ));
    };
    let value = rewrites
        .owned(&written, ctx, renames, slot)
        .map_err(|r| refused(r.error.to_string()))?;
    // A string handed to the child's variable is held to the bound the child
    // declared for it, whatever the value came from, as an `<assign>` to it
    // would be (§scxml-4.9): past it the value fails as any other does, is
    // left out, and the child starts with the one its `<data>` gave it.
    let (seed, fails) = match (held, variable.and_then(|v| v.capacity)) {
        (SceType::String, Some(capacity)) => (target.bounded_string(&value.text, capacity), true),
        _ => (value.text, value.can_fail),
    };
    // Where the value is written, for a backend that runs the document's own
    // expression: the seed, which carries the bound a string is held to.
    rewrites.note(&written, spelling.as_ref(), &seed);
    param.native_seed = seed;
    param.native_seed_type = target.scalar_type(held);
    param.native_fails = fails;
    Ok(())
}

/// Lower a hybrid `<invoke>` (§scxml-6.4): its `srcexpr` to the string it
/// computes, which the machine reads the document stem of when the invocation
/// starts to name the candidate to start
/// ([`crate::model::HybridInvokeInfo::native_src`]), and what the invoke hands
/// each candidate — the arguments the candidate declares a variable for as the
/// values of those variables ([`crate::model::InvokeCandidate::seeds`]), and the
/// rest as values evaluated and left out
/// ([`crate::model::InvokeCandidate::unkept`], §scxml-6.4.3). The attribute and
/// the `namelist` are cleared once they are lowered, so no template evaluates
/// either a second time.
///
/// Validation already held each value to the type of the variable it lands in
/// ([`crate::forge::static_datamodel`]), so a refusal here is a lowering this
/// backend lacks, not a mistake in the document.
fn lower_hybrid_invoke(
    info: &mut crate::model::HybridInvokeInfo,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    let target = rewrites.target;
    let lang = target.name();
    if !target.lowers_hybrid_invoke() {
        return Err(GenerateError::unsupported(format!(
            "a hybrid <invoke> has no {lang} lowering yet"
        )));
    }
    let value = rewrites
        .owned(&info.srcexpr, ctx, renames, InferredType::Str)
        .map_err(|r| {
            GenerateError::unsupported(format!(
                "`{}` has no {lang} lowering: {}",
                info.srcexpr, r.error
            ))
        })?;
    let stems: Vec<&str> = info.candidates.iter().map(|c| c.stem.as_str()).collect();
    if let Some(site) = target.hybrid_src_site(&value.text, &stems) {
        rewrites.note(&info.srcexpr, info.srcexpr_spelling.as_ref(), &site);
    }
    info.native_src = value.text;
    info.native_src_fails = value.can_fail;
    info.srcexpr.clear();
    let arguments = info.arguments();
    for candidate in &mut info.candidates {
        let declared = candidate.child_static_variables.as_deref().unwrap_or(&[]);
        for argument in &arguments {
            let mut param = argument.clone();
            match declared.iter().find(|v| v.id == param.name) {
                Some(variable) => {
                    lower_child_argument(&mut param, Some(variable), ctx, renames, rewrites)?;
                    candidate.seeds.push(param);
                }
                None => {
                    lower_wire_param(&mut param, ctx, renames, rewrites)?;
                    candidate.unkept.push(param);
                }
            }
        }
    }
    info.common.base.params.clear();
    info.namelist.clear();
    Ok(())
}

/// Every `<elseif>` condition and every block nested inside `action`,
/// through the model's own accessors for them. `true` when any expression
/// lowered reads the triggering event's payload.
fn lower_nested(
    action: &mut Action,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let target = rewrites.target;
    let mut reads_payload = false;
    // Each condition of an `<if>` chain is read where every earlier one is
    // false, each block where its own is true besides, and the `<else>` where
    // all are false (docs/adr/0055, section 3): the chain the judge read, taken
    // before any of its conditions is lowered.
    let chain = Chain::of(
        &ctx.proven,
        &crate::forge::static_datamodel::chain_conds(action),
    );
    // Every condition of the chain sets the flag of the `<if>` it belongs to.
    let flag = target.condition_failed_flag(action.if_ordinal);
    for (k, branch) in action.branch_conditions_mut().enumerate() {
        if branch.is_cpp_condition || branch.is_kt_condition || branch.cond.trim().is_empty() {
            continue;
        }
        let ctx = proving(ctx, &chain.cond(k + 1));
        reads_payload |= crate::forge::expr::references_event_data_lexically(&branch.cond);
        let cond = rewrites
            .receiving(&branch.cond, &ctx, renames, InferredType::Bool)
            .map_err(|r| {
                GenerateError::unsupported(format!(
                    "`{}` has no {} lowering: {}",
                    branch.cond,
                    target.name(),
                    r.error
                ))
            })?;
        rewrites.note(&branch.cond, branch.cond_spelling.as_ref(), &cond.text);
        branch.native_cond = if cond.can_fail {
            branch.native_cond_fails = true;
            target.receiving_condition(
                &cond.text,
                &execution_failure(rewrites, &format!("<elseif cond='{}'>", branch.cond)),
                &flag,
            )
        } else {
            cond.text
        };
        branch.cond_constant = None;
    }
    // The blocks in the model's one order ([`Action::nested_blocks_mut`]): the
    // body of an action that is no `<if>`, the `then` block, each `<elseif>`'s,
    // and the `<else>`.
    let blocks = action.nested_blocks_mut();
    let last = blocks.len().saturating_sub(1);
    for (k, block) in blocks.into_iter().enumerate() {
        let facts = match k {
            0 => ctx.proven.clone(),
            k if k == last => chain.otherwise(),
            k => chain.block(k - 1),
        };
        reads_payload |= lower_actions(block, &proving(ctx, &facts), renames, rewrites)?;
    }
    Ok(reads_payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    /// C11 holds an optional value in the struct `sce/forge/optional.h` declares
    /// over the type its present value is held in (docs/adr/0055, section 5): one
    /// name per element, the runtime's own for a number or a `bool`, declared
    /// beside the buffer for a text and under the enum for an enum; a whole value
    /// mapped or written to the wire under a test of its flag, `null` where it is
    /// absent, and logged as `null` where it is absent.
    #[test]
    fn c11_spells_an_optional_value_over_its_present_type() {
        let c = CTarget::arguments();
        let spelled = |present: &str| c.optional_type(present);
        assert_eq!(spelled("int64_t").as_deref(), Some("sce_optional_int64_t"));
        assert_eq!(spelled("double").as_deref(), Some("sce_optional_float64_t"));
        assert_eq!(spelled("bool").as_deref(), Some("sce_optional_bool_t"));
        assert_eq!(
            spelled("sce_static_string_16_t").as_deref(),
            Some("sce_optional_string_16_t")
        );
        assert_eq!(
            spelled("sce_static_bytes_8_t").as_deref(),
            Some("sce_optional_bytes_8_t")
        );
        assert_eq!(
            spelled("enum_item_status_enum_t").as_deref(),
            Some("sce_optional_enum_enum_item_status_t")
        );
        // A text with no bound has no buffer to be held over.
        assert_eq!(spelled("const char *"), None);
        assert_eq!(
            c.optional_none("int64_t").as_deref(),
            Some("sce_optional_int64_none()")
        );
        assert_eq!(
            c.optional_some("double", "x").as_deref(),
            Some("sce_optional_float64_some(x)")
        );
        assert_eq!(
            c.optional_import().as_deref(),
            Some("#include <sce/forge/optional.h>")
        );
        // A number's type is the runtime's; a text's is declared with its buffer,
        // and an enum's after the enum, each under a guard of its own.
        assert_eq!(c.optional_def("int64_t", &SceType::Int64), None);
        let text = c
            .optional_def("sce_static_string_16_t", &SceType::String)
            .expect("a bounded text declares its optional type");
        assert!(
            text.starts_with("#ifndef SCE_OPTIONAL_STRING_16\n")
                && text.contains("sce_static_string_16_t")
                && text.contains("SCE_OPTIONAL_DECLARE_STRING(16)"),
            "{text}"
        );
        let status = c
            .optional_def(
                "enum_item_status_enum_t",
                &SceType::Enum(crate::forge::model::EnumRef {
                    alias: "Status".to_string(),
                }),
            )
            .expect("an enum declares its optional type");
        assert!(
            status.contains("#ifndef SCE_OPTIONAL_ENUM_ENUM_ITEM_STATUS")
                && status.contains(
                    "SCE_OPTIONAL_DECLARE(enum_enum_item_status, enum_item_status_enum_t, \
                     SCE_OPTIONAL_SAME_VALUE)"
                ),
            "{status}"
        );
        assert_eq!(
            c.optional_map("title", "sce_static_string_16_t", &|v: &str| format!(
                "hold({v})"
            ))
            .as_deref(),
            Some(
                "((title).present ? sce_optional_string_16_some(hold((title).value.data)) \
                 : sce_optional_string_16_none())"
            )
        );
        assert_eq!(
            c.wire_optional("due", "int64_t", &|v: &str| format!("wire({v})"))
                .as_deref(),
            Some("((due).present ? wire((due).value) : sce_forge_wire_null())")
        );
        assert_eq!(
            c.wire_optional("title", "const char *", &|v: &str| format!("wire({v})"))
                .as_deref(),
            Some("((title).present ? wire((title).value.data) : sce_forge_wire_null())")
        );
        let int64 = crate::forge::types::OptionalElem::Number(
            crate::forge::quantity::NumericBaseType::Int {
                signed: true,
                bits: 64,
            },
        );
        let logged = c.log("due", "due", InferredType::Optional(int64));
        assert!(
            logged.starts_with("if ((due).present) {")
                && logged.contains("(long long)((due).value)")
                && logged.contains("\"%s: null\\n\", \"due\""),
            "{logged}"
        );
    }

    /// An optional record is the struct `sce/forge/optional.h` declares over the
    /// record's own struct type, named by that type, whichever side holds the type —
    /// the machine's own or the algorithm's — and a field of its present value is
    /// written through `.value` (docs/adr/0055, step 9).
    #[test]
    fn c11_spells_an_optional_record_over_its_struct_type() {
        let c = CTarget::arguments();
        let machine = "static_optional_record_record_item_t";
        let algorithm = "ItemPickPayload_t";
        assert_eq!(
            c.optional_type(machine).as_deref(),
            Some("sce_optional_record_static_optional_record_record_item_t")
        );
        assert_eq!(
            c.optional_type(algorithm).as_deref(),
            Some("sce_optional_record_ItemPickPayload_t")
        );
        assert_eq!(
            c.optional_none(algorithm).as_deref(),
            Some("sce_optional_record_ItemPickPayload_none()")
        );
        assert_eq!(
            c.optional_some(algorithm, "it").as_deref(),
            Some("sce_optional_record_ItemPickPayload_some(it)")
        );
        // A type with no struct name is no record; the families of an element
        // keep the names they had.
        assert_eq!(c11_optional_elem("const char *"), None);
        assert_eq!(c11_optional_elem("int64_t").as_deref(), Some("int64"));
        assert_eq!(
            c11_optional_elem("enum_item_status_enum_t").as_deref(),
            Some("enum_enum_item_status")
        );

        let def = c
            .optional_record_def(machine)
            .expect("a record declares its optional type");
        assert!(
            def.starts_with("#ifndef SCE_OPTIONAL_RECORD_STATIC_OPTIONAL_RECORD_RECORD_ITEM\n")
                && def.contains(
                    "SCE_OPTIONAL_DECLARE_RECORD(record_static_optional_record_record_item, \
                     static_optional_record_record_item_t)"
                )
                && def.ends_with("\n#endif"),
            "{def}"
        );

        // A field of the present value is written through `.value`, the flag untouched.
        assert_eq!(
            c.assign_present_field("sm->policy.v_pick", "count", "3")
                .as_deref(),
            Some("sm->policy.v_pick.value.count = 3;")
        );
        assert_eq!(
            c.assign_present_string_field("sm->policy.v_pick", "title", "text")
                .as_deref(),
            Some("memmove(sm->policy.v_pick.value.title.data, text, strlen(text) + 1u);")
        );
        assert_eq!(
            c.assign_present_bytes_field("sm->policy.v_pick", "frame", "v")
                .as_deref(),
            Some(
                "memmove((sm->policy.v_pick.value.frame).data, (v).data, (v).len); \
                 (sm->policy.v_pick.value.frame).len = (v).len;"
            )
        );
        // An element of a list, whole, is a checked index read of a zero record.
        assert_eq!(
            c.record_at("sm->policy.v_items", "1").as_deref(),
            Some(
                "(sce_forge_checked_index_from_i(&sce_failure_, (int64_t)(1), \
                 (sm->policy.v_items).len) ? (sm->policy.v_items).data[1] : \
                 (__typeof__((sm->policy.v_items).data[0])){0})"
            )
        );
    }

    const COUNTER: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="counting" datamodel="sce-static" name="m">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="counting">
    <transition event="tick" cond="count &lt; 10" type="internal">
      <assign location="count" expr="count + 1"/>
    </transition>
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>"#;

    /// A machine that holds an optional value is one a saved state holds
    /// (docs/adr/0055, step 6): it writes the value as `null` or as its present
    /// value, so holding one takes nothing away from what a saved state holds,
    /// and the machine declares a saved shape as the same machine without it
    /// does.
    #[test]
    fn a_machine_holding_an_optional_value_is_one_a_saved_state_holds() {
        let read = |document: &str| {
            let mut model = SCXMLParser::new()
                .parse_string(document, "m")
                .expect("parses");
            crate::analyzer::analyze(&mut model, "m.scxml");
            model
        };
        let optional = COUNTER.replace(
            r#"<data id="count" sce:type="uint32" expr="0"/>"#,
            r#"<data id="count" sce:type="uint32" expr="0"/>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>"#,
        );
        assert_ne!(optional, COUNTER, "the variable is added");
        assert!(a_tree_can_hold(&read(COUNTER)));
        assert!(!holds_an_optional_value(&read(COUNTER)));
        assert!(holds_an_optional_value(&read(&optional)));
        assert!(a_tree_can_hold(&read(&optional)));
        assert!(shape(&optional).is_some(), "the machine declares a shape");
    }

    /// What a saved state is bound to for the machine of [`marked`].
    fn marked_shape(variables: &str, fields: &str) -> SavedShape {
        let (_dir, mut model) = marked(variables, fields);
        lower_rust(&mut model, "M")
            .expect("lowers")
            .saved_shape
            .expect("a machine of values has a shape")
    }

    /// A machine whose `<datamodel>` rows are `variables`, beside
    /// `schema_mark.scxml`, whose rows are `fields`, and `enum_level.scxml`; the
    /// machine imports the two as `Mark` and `Level`, and the schema the enum as
    /// `Level`. Read and analysed, with the directory that holds the three.
    fn marked(variables: &str, fields: &str) -> (tempfile::TempDir, SCXMLModel) {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            dir.path().join("enum_level.scxml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="0"/>
      <sce:variant name="high" value="1"/>
    </data>
  </datamodel>
</scxml>"#,
        )
        .expect("the enum");
        std::fs::write(
            dir.path().join("schema_mark.scxml"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_mark" sce:event-name="mark.set">
  <sce:import src="enum_level.scxml" kind="enum" as="Level"/>
  <datamodel>
    {fields}
  </datamodel>
</scxml>"#
            ),
        )
        .expect("the schema");
        let statechart = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="m">
  <sce:import kind="event-schema" src="schema_mark.scxml" as="Mark"/>
  <sce:import kind="enum" src="enum_level.scxml" as="Level"/>
  <datamodel>
    {variables}
  </datamodel>
  <state id="idle"/>
</scxml>"#
        );
        let path = dir.path().join("m.scxml");
        std::fs::write(&path, statechart).expect("the statechart");
        let mut model = SCXMLParser::new()
            .parse_file(path.to_str().expect("a UTF-8 path"))
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        (dir, model)
    }

    /// The rows of [`marked_shape`]'s machine with each scalar variable of
    /// `number`, `string` and `enum:Level` written by `wrap` (the type itself,
    /// or the optional type over it), and its record and list of records.
    fn marked_variables(wrap: impl Fn(&str) -> String) -> String {
        format!(
            r#"<data id="due" sce:type="{}" expr="0"/>
    <data id="title" sce:type="{}" sce:capacity="8" expr="'first'"/>
    <data id="level" sce:type="{}" expr="Level.low"/>
    <data id="last" sce:type="record:Mark"><sce:set name="due" expr="0"/><sce:set name="level" expr="Level.high"/></data>
    <data id="seen" sce:type="list&lt;record:Mark&gt;" sce:capacity="2"/>"#,
            wrap("int64"),
            wrap("string"),
            wrap("enum:Level")
        )
    }

    /// The rows of [`marked_shape`]'s schema, each field's type written by
    /// `wrap`.
    fn marked_fields(wrap: impl Fn(&str) -> String) -> String {
        format!(
            r#"<data id="due" sce:type="{}" sce:direction="in"/>
    <data id="level" sce:type="{}" sce:direction="in"/>"#,
            wrap("int64"),
            wrap("enum:Level")
        )
    }

    /// docs/adr/0055, section 6 (the owner's answer 6): a saved shape names an
    /// optional value by the type its present value is, so a variable, a
    /// record's field and a field of a list's record widened from `T` to
    /// `optional<T>` keep the shape in both formats, the enums the state holds
    /// and where, and the records; any other change of type still moves it.
    #[test]
    fn a_widening_to_an_optional_value_keeps_the_saved_shape() {
        let plain = |t: &str| t.to_string();
        let optional = |t: &str| format!("optional&lt;{t}&gt;");
        let before = marked_shape(&marked_variables(plain), &marked_fields(plain));
        let widened = marked_shape(&marked_variables(optional), &marked_fields(optional));
        assert_eq!(widened.first, before.first);
        assert_eq!(widened.revised, before.revised);
        assert_eq!(widened.format2_first, before.format2_first);
        assert_eq!(widened.format2_revised, before.format2_revised);
        // What `revisions` and the restore read of each enum and record: the
        // same holders, so a value is judged where it was before.
        let json = |shape: &SavedShape| {
            (
                serde_json::to_value(&shape.enums).expect("the enums serialise"),
                serde_json::to_value(&shape.records).expect("the records serialise"),
            )
        };
        assert_eq!(json(&widened), json(&before));
        // The variables alone, and the fields alone, each keep it too.
        assert_eq!(
            marked_shape(&marked_variables(optional), &marked_fields(plain)).first,
            before.first
        );
        assert_eq!(
            marked_shape(&marked_variables(plain), &marked_fields(optional)).first,
            before.first
        );
        // Another type is another shape, widened or not: a variable or a
        // field of `int64` taken to `int32`.
        let narrower = |t: &str| t.replace("int64", "int32");
        let renumbered = marked_shape(&marked_variables(narrower), &marked_fields(plain));
        assert_ne!(renumbered.first, before.first);
        assert_ne!(renumbered.format2_first, before.format2_first);
        let renumbered_optional = |t: &str| optional(&narrower(t));
        assert_ne!(
            marked_shape(
                &marked_variables(renumbered_optional),
                &marked_fields(plain)
            )
            .first,
            before.first
        );
        assert_ne!(
            marked_shape(&marked_variables(plain), &marked_fields(narrower)).first,
            before.first
        );
    }

    /// What a backend's template saves an optional variable with
    /// (docs/adr/0055, step 6): a present value of an enum by the machine's own
    /// type for it, as a variable of the enum is, and every other one as the
    /// scalar its `sce:type` names, a text with its bound; and a record's
    /// optional field is marked so.
    #[test]
    fn an_optional_variable_says_how_its_present_value_is_saved() {
        let optional = |t: &str| format!("optional&lt;{t}&gt;");
        let (_dir, mut model) = marked(&marked_variables(optional), &marked_fields(optional));
        let lowered = lower_kotlin(&mut model, "M").expect("Kotlin lowers optional values");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        let saved = |id: &str| {
            let f = field(id);
            (f.saved_kind, f.saved_type.as_str(), f.saved_elem, f.bound)
        };
        assert_eq!(saved("due"), ("optional", "int64", Some("scalar"), None));
        assert_eq!(
            saved("title"),
            ("optional", "string", Some("scalar"), Some(8))
        );
        assert_eq!(
            saved("level"),
            ("optional", "MLevelEnum", Some("enum"), None)
        );
        assert_eq!(saved("last"), ("record", "MMarkRecord", None, None));
        let record = lowered
            .records
            .iter()
            .find(|r| r.ty == "MMarkRecord")
            .expect("the record type");
        let optional_fields: Vec<(&str, bool)> = record
            .fields
            .iter()
            .map(|f| (f.id.as_str(), f.optional))
            .collect();
        assert_eq!(optional_fields, [("due", true), ("level", true)]);
    }

    /// Go holds an optional value in `optional.Of[T]` and writes the whole value
    /// of an optional place by the walk's one rule (docs/adr/0055): `null` its
    /// absent value, a value of `T` lowered into `T` (a text held to its bound)
    /// and lifted, an optional value as it is; a read under an `<if>` whose
    /// condition proves the value present is a proven read, and the runtime's
    /// package is imported.
    #[test]
    fn go_writes_an_optional_place_whole_by_the_walks_one_rule() {
        const OPTIONAL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
    </transition>
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
    </transition>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(OPTIONAL, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_go(&mut model, "M", None).expect("Go lowers an optional value");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        assert_eq!(field("due").ty, "optional.Of[int64]");
        assert_eq!(field("due").init, "optional.None[int64]()");
        assert_eq!(field("other").init, "optional.Some[int64](7)");
        assert_eq!(field("title").ty, "optional.Of[string]");
        assert_eq!(field("title").saved_kind, "optional");
        assert!(
            lowered.saved_shape.is_some(),
            "a saved state holds an optional value (docs/adr/0055, step 6)"
        );
        assert!(
            lowered.imports.contains(&format!(
                "\t\"{}\"",
                crate::std_imports::GO_FORGE_OPTIONAL_PATH
            )),
            "{:?}",
            lowered.imports
        );
        let state = model.states.get("s").expect("the state");
        let read = &state.transitions[0].actions[0];
        assert_eq!(read.native_cond, "p.vDue.IsPresent()");
        // Read through the model's one definition of what lies inside an action.
        let blocks = read.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Then)
            .expect("the <if> has a then block");
        assert_eq!(then.actions[0].native_code, "p.vDays = p.vDue.Proven()");
        let write = &state.transitions[1].actions;
        assert_eq!(write[0].native_code, "p.vDue = optional.None[int64]()");
        assert!(
            write[1].native_code.contains(
                "sceValue := optional.Some[string](scealgorithm.Bounded(&sceFailure, \"abc\", 4))"
            ),
            "{}",
            write[1].native_code
        );
        assert_eq!(write[2].native_code, "p.vOther = p.vDue");
    }

    /// Python holds an optional value in `Optional[T]`, `None` absent and the
    /// value itself present, and writes the whole value of an optional place by
    /// the walk's one rule (docs/adr/0055): `null` is `None`, a value of `T` is
    /// lowered into `T` (a text held to its bound) and is the value, an optional
    /// value is written as it is. A read under an `<if>` whose condition proves
    /// the value present is the value, and a `<param>` of an optional value
    /// crosses as JSON `null` where it is absent.
    #[test]
    fn python_writes_an_optional_place_whole_by_the_walks_one_rule() {
        const OPTIONAL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
    </transition>
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
    </transition>
    <transition event="relay" type="internal">
      <send event="sent">
        <param name="due" expr="due"/>
      </send>
    </transition>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(OPTIONAL, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_python(&mut model, "M").expect("Python lowers an optional value");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        assert_eq!(field("due").ty, "Optional[int]");
        assert_eq!(field("due").init, "None");
        assert_eq!(field("other").init, "7");
        assert_eq!(field("title").ty, "Optional[str]");
        assert_eq!(field("title").saved_kind, "optional");
        assert!(
            lowered.saved_shape.is_some(),
            "a saved state holds an optional value (docs/adr/0055, step 6)"
        );
        let state = model.states.get("s").expect("the state");
        let read = &state.transitions[0].actions[0];
        assert_eq!(read.native_cond, "self.v_due is not None");
        // Read through the model's one definition of what lies inside an action.
        let blocks = read.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Then)
            .expect("the <if> has a then block");
        assert_eq!(then.actions[0].native_code, "self.v_days = self.v_due");
        let write = &state.transitions[1].actions;
        assert_eq!(write[0].native_code, "self.v_due = None");
        assert!(
            write[1]
                .native_code
                .contains("self.v_title = sce_algorithm.bounded('abc', 4)"),
            "{}",
            write[1].native_code
        );
        assert_eq!(write[2].native_code, "self.v_other = self.v_due");
        let relay = &state.transitions[2].actions[0];
        assert_eq!(
            relay.params[0].native_value,
            "(lambda sce_present: _ScriptValue.null() if sce_present is None else \
             _ScriptValue.of(str(sce_present)))(self.v_due)"
        );
    }

    /// Kotlin holds an optional value in `T?` and writes the whole value of an
    /// optional place by the walk's one rule (docs/adr/0055): `null` its absent
    /// value, a value of `T` lowered into `T` (a text held to its bound) and
    /// lifted as it is, an optional value as it is; a read under an `<if>` whose
    /// condition proves the value present is the runtime's proven read.
    #[test]
    fn kotlin_writes_an_optional_place_whole_by_the_walks_one_rule() {
        const OPTIONAL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
    </transition>
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
    </transition>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(OPTIONAL, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_kotlin(&mut model, "M").expect("Kotlin lowers an optional value");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        assert_eq!(field("due").ty, "Long?");
        assert_eq!(field("due").init, "null");
        assert_eq!(field("other").init, "7");
        assert_eq!(field("title").ty, "String?");
        assert_eq!(field("title").saved_kind, "optional");
        // What the template reads the present value with.
        assert_eq!(field("title").saved_type, "string");
        assert_eq!(field("title").saved_elem, Some("scalar"));
        assert_eq!(field("title").bound, Some(4));
        assert_eq!(field("days").saved_elem, None);
        assert!(
            lowered.saved_shape.is_some(),
            "a saved state holds an optional value (docs/adr/0055, step 6)"
        );
        let state = model.states.get("s").expect("the state");
        let read = &state.transitions[0].actions[0];
        assert_eq!(read.native_cond, "vDue != null");
        // Read through the model's one definition of what lies inside an action.
        let blocks = read.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Then)
            .expect("the <if> has a then block");
        assert_eq!(
            then.actions[0].native_code,
            "vDays = com.sce.forge.runtime.SceOptional.proven(vDue)"
        );
        let write = &state.transitions[1].actions;
        assert_eq!(write[0].native_code, "vDue = null");
        assert!(
            write[1]
                .native_code
                .contains("vTitle = com.sce.forge.runtime.SceChecked.bounded(\"abc\", 4)"),
            "{}",
            write[1].native_code
        );
        assert_eq!(write[2].native_code, "vOther = vDue");
    }

    /// A record whose schema has an optional field holds it as `T?`, and saves
    /// it as `null` or as its present value, read back absent from `null` and
    /// otherwise as a present value of the field is, a text held to its bound
    /// (docs/adr/0055, step 6).
    #[test]
    fn a_kotlin_record_with_an_optional_field_holds_it_nullable_and_saves_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            dir.path().join("schema_due.scxml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_due" sce:event-name="due.set">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:max-size="8" sce:direction="in"/>
  </datamodel>
</scxml>"#,
        )
        .expect("the schema");
        let path = dir.path().join("m.scxml");
        std::fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <sce:import kind="event-schema" src="schema_due.scxml" as="Due"/>
  <datamodel>
    <data id="item" sce:type="record:Due">
      <sce:set name="id" expr="0"/>
      <sce:set name="due" expr="null"/>
      <sce:set name="title" expr="null"/>
    </data>
  </datamodel>
  <state id="s">
    <transition event="clear" type="internal">
      <assign location="item.due" expr="null"/>
    </transition>
  </state>
</scxml>"#,
        )
        .expect("the statechart");
        let mut model = SCXMLParser::new()
            .parse_file(path.to_str().expect("a UTF-8 path"))
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_kotlin(&mut model, "M").expect("Kotlin lowers an optional field");
        let def = lowered
            .type_defs
            .iter()
            .find(|def| def.contains("data class MDueRecord("))
            .unwrap_or_else(|| panic!("the record's class in {:?}", lowered.type_defs));
        assert!(
            def.contains("data class MDueRecord(val id: UInt, val due: Long?, val title: String?)"),
            "{def}"
        );
        // Written: a field of `T` as it always was, an optional one as `null` or
        // its present value.
        assert!(
            def.contains(
                "fun toSaved(): Any = linkedMapOf(\"id\" to SavedValues.of(id), \
                 \"due\" to SavedValues.optional(due) { SavedValues.of(it) }, \
                 \"title\" to SavedValues.optional(title) { SavedValues.of(it) })"
            ),
            "{def}"
        );
        // Read: the present value by the reader of its type, the text held to
        // the schema's bound.
        assert!(
            def.contains(
                "due = SavedValues.optional(SavedValues.field(value, what, \"due\"), \"$what.due\") \
                 { e, w -> SavedValues.int64(e, w) }"
            ),
            "{def}"
        );
        assert!(
            def.contains(
                "title = SavedValues.optional(SavedValues.field(value, what, \"title\"), \
                 \"$what.title\") { e, w -> SavedValues.string(e, w, 8) }"
            ),
            "{def}"
        );
        assert!(
            def.contains(
                "id = SavedValues.uint32(SavedValues.field(value, what, \"id\"), \"$what.id\")"
            ),
            "{def}"
        );
        let item = lowered
            .fields
            .iter()
            .find(|f| f.id == "item")
            .expect("the record variable");
        assert_eq!(
            item.init,
            "MDueRecord(id = 0.toUInt(), due = null, title = null)"
        );
        let clear = &model.states.get("s").expect("the state").transitions[0].actions[0];
        assert_eq!(clear.native_code, "vItem = vItem.copy(due = null)");
    }

    /// Rust holds an optional value in `Option<T>` and writes the whole value of
    /// an optional place by the walk's one rule (docs/adr/0055): `null` its
    /// absent value, a value of `T` lowered into `T` (a text held to its bound)
    /// and lifted, an optional value as it is; a read under an `<if>` whose
    /// condition proves the value present is a proven read, and a `<log>` prints
    /// an absent value as `null`.
    #[test]
    fn rust_writes_an_optional_place_whole_by_the_walks_one_rule() {
        const OPTIONAL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
    </transition>
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
      <log expr="title"/>
    </transition>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(OPTIONAL, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_rust(&mut model, "M").expect("Rust lowers an optional value");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        assert_eq!(field("due").ty, "Option<i64>");
        assert_eq!(field("due").init, "None");
        assert_eq!(field("other").init, "Some(7)");
        assert_eq!(field("title").ty, "Option<String>");
        assert_eq!(field("title").saved_kind, "optional");
        assert!(
            lowered.saved_shape.is_some(),
            "a saved state holds an optional value (docs/adr/0055, step 6)"
        );
        let state = model.states.get("s").expect("the state");
        let read = &state.transitions[0].actions[0];
        assert_eq!(read.native_cond, "self.due.is_some()");
        // Read through the model's one definition of what lies inside an action.
        let blocks = read.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Then)
            .expect("the <if> has a then block");
        assert_eq!(
            then.actions[0].native_code,
            "self.days = sce_forge_runtime::optional::proven(self.due);"
        );
        let write = &state.transitions[1].actions;
        assert_eq!(write[0].native_code, "self.due = None;");
        assert!(
            write[1].native_code.contains(
                "self.title = Some(sce_forge_runtime::algorithm::bounded(\"abc\".to_string(), 4)?);"
            ),
            "{}",
            write[1].native_code
        );
        assert_eq!(write[2].native_code, "self.other = self.due;");
        assert_eq!(
            write[3].native_code,
            "::sce_rust_runtime::sce_log_info!(\"{:?}\", \
             sce_forge_runtime::optional::logged(&self.title.as_deref()));"
        );
    }

    fn shape(document: &str) -> Option<String> {
        let mut model = SCXMLParser::new()
            .parse_string(document, "m")
            .expect("parses");
        // The generator lowers a model the analyzer has read, and a delayed
        // `<send>` is known to need the scheduler only after that pass: asked
        // of a model nothing analysed, a pending timer is invisible
        // (`a_machine_waiting_on_another_session_has_no_save_api` runs the
        // generator itself).
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_rust(&mut model, "M")
            .expect("lowers")
            .saved_shape
            .map(|shape| shape.first)
    }

    /// What a saved state is bound to for a machine holding a `record:Reading`
    /// (`last`) and a list of them (`seen`), with `schema` beside it as the
    /// schema `Reading` imports. `sets` builds `last` whole.
    fn reading_shape(schema: &str, sets: &str) -> SavedShape {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(dir.path().join("schema_reading.scxml"), schema).expect("the schema");
        let statechart = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="reading" datamodel="sce-static" name="m">
  <sce:import kind="event-schema" src="schema_reading.scxml" as="Reading"/>
  <datamodel>
    <data id="last" sce:type="record:Reading">{sets}</data>
    <data id="seen" sce:type="list&lt;record:Reading&gt;" sce:capacity="4"/>
  </datamodel>
  <state id="reading"/>
</scxml>"#
        );
        let path = dir.path().join("m.scxml");
        std::fs::write(&path, statechart).expect("the statechart");
        let mut model = SCXMLParser::new()
            .parse_file(path.to_str().expect("a UTF-8 path"))
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_rust(&mut model, "M")
            .expect("lowers")
            .saved_shape
            .expect("a machine of records has a shape")
    }

    /// The schema `Reading` imports, at `revision` (written past the first),
    /// declaring the `<data>` rows `fields`.
    fn reading_schema(revision: u32, fields: &[&str]) -> String {
        let revision = if revision > 1 {
            format!(r#" sce:revision="{revision}""#)
        } else {
            String::new()
        };
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_reading"
       sce:event-name="reading.taken"{revision}>
  <datamodel>
    {}
  </datamodel>
</scxml>"#,
            fields.join("\n    ")
        )
    }

    /// A Rust `<foreach>` is lowered to its loop and, beside it, to the head
    /// that runs its body for the first element only: the same copy of the
    /// list, bound by the same patterns, so the one prologue opens both.
    #[test]
    fn a_rust_foreach_is_given_the_head_of_its_first_element() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <onentry>
      <foreach array="picked" item="v">
        <assign location="total" expr="total + v"/>
      </foreach>
      <foreach array="picked" item="v" index="i">
        <assign location="total" expr="total + i"/>
      </foreach>
    </onentry>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(document, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_rust(&mut model, "M").expect("lowers");
        let heads: Vec<(String, String)> = model.states["s"].on_entry_blocks[0]
            .iter()
            .map(|action| (action.native_loop.clone(), action.native_loop_first.clone()))
            .collect();
        assert_eq!(heads.len(), 2, "{heads:?}");

        let (walk, first) = &heads[0];
        let (pattern, copy) = walk
            .strip_prefix("for ")
            .and_then(|rest| rest.split_once(" in "))
            .unwrap_or_else(|| panic!("a loop head: {walk}"));
        assert!(copy.ends_with(".clone()"), "the loop walks a copy: {walk}");
        assert_eq!(
            first,
            &format!("if let Some({pattern}) = {copy}.into_iter().next()"),
            "the first element of the same copy, by the same pattern"
        );

        let (walk, first) = &heads[1];
        let (pattern, elements) = walk
            .strip_prefix("for ")
            .and_then(|rest| rest.split_once(" in "))
            .unwrap_or_else(|| panic!("a loop head: {walk}"));
        assert!(
            pattern.starts_with("(sce_position_of_") && elements.ends_with(".enumerate()"),
            "the indexed loop binds the position: {walk}"
        );
        assert_eq!(
            first,
            &format!("if let Some({pattern}) = {elements}.next()"),
            "the first position and element, by the same pattern"
        );
    }

    const LEVEL: &str = r#"<data id="level" sce:type="uint8" sce:direction="in"/>"#;
    const URGENT: &str = r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2" sce:default="true"/>"#;
    // A list holds records of fixed-width fields only, so the added fields are.
    const NOTE: &str =
        r#"<data id="note" sce:type="uint16" sce:direction="in" sce:since="3" sce:default="7"/>"#;
    const SET_LEVEL: &str = r#"<sce:set name="level" expr="0"/>"#;
    const SET_URGENT: &str = r#"<sce:set name="urgent" expr="false"/>"#;
    const SET_NOTE: &str = r#"<sce:set name="note" expr="0"/>"#;

    #[test]
    fn a_schema_that_grew_leaves_the_shape_its_first_revision_had() {
        let first = reading_shape(&reading_schema(1, &[LEVEL]), SET_LEVEL);
        let second = reading_shape(
            &reading_schema(2, &[LEVEL, URGENT]),
            &format!("{SET_LEVEL}{SET_URGENT}"),
        );
        let third = reading_shape(
            &reading_schema(3, &[LEVEL, URGENT, NOTE]),
            &format!("{SET_LEVEL}{SET_URGENT}{SET_NOTE}"),
        );
        // The first shape names the first revision's fields alone, so a state
        // saved at any revision has it.
        assert_eq!(first.first, second.first);
        assert_eq!(first.first, third.first);
        // A state that names revisions has another, which a reader that knows
        // only the first refuses.
        assert_ne!(third.first, third.revised);
        assert_eq!(first.revised, third.revised);

        // What each revision declares has one digest in every build that has it.
        assert_eq!(first.records.len(), 1);
        assert_eq!(first.records[0].event, "reading.taken");
        assert_eq!(first.records[0].revision, 1);
        assert!(first.records[0].revisions.is_empty());
        assert_eq!(second.records[0].revisions.len(), 1);
        assert_eq!(third.records[0].revision, 3);
        assert_eq!(third.records[0].revisions.len(), 2);
        assert_eq!(
            third.records[0].revisions[0],
            second.records[0].revisions[0]
        );
        assert_ne!(third.records[0].revisions[0], third.records[0].revisions[1]);

        // The fields a later revision added, with their defaults as a saved state
        // writes them, and the variables that hold the record.
        let added: Vec<(&str, u32, &str)> = third.records[0]
            .added
            .iter()
            .map(|f| (f.id.as_str(), f.since, f.default.as_str()))
            .collect();
        assert_eq!(added, [("urgent", 2, "true"), ("note", 3, "7")]);
        let holders: Vec<(&str, bool)> = third.records[0]
            .holders
            .iter()
            .map(|h| (h.variable.as_str(), h.list))
            .collect();
        assert_eq!(holders, [("last", false), ("seen", true)]);
    }

    #[test]
    fn a_revisions_digest_follows_the_types_it_declares_and_not_its_defaults() {
        let sets = format!("{SET_LEVEL}{SET_URGENT}");
        let second = reading_shape(&reading_schema(2, &[LEVEL, URGENT]), &sets);
        // Another default: a saved state holds a value for the field, never the
        // default, so the digest is the same.
        let other_default = reading_shape(
            &reading_schema(
                2,
                &[
                    LEVEL,
                    r#"<data id="urgent" sce:type="bool" sce:direction="in" sce:since="2" sce:default="false"/>"#,
                ],
            ),
            &sets,
        );
        assert_eq!(
            second.records[0].revisions,
            other_default.records[0].revisions
        );
        // Another type: a saved value of the one is not a value of the other.
        let retyped = reading_shape(
            &reading_schema(
                2,
                &[
                    LEVEL,
                    r#"<data id="urgent" sce:type="uint8" sce:direction="in" sce:since="2" sce:default="1"/>"#,
                ],
            ),
            &format!("{SET_LEVEL}<sce:set name=\"urgent\" expr=\"0\"/>"),
        );
        assert_ne!(second.records[0].revisions, retyped.records[0].revisions);
        // Neither moves the first shape, which names the first revision only.
        assert_eq!(second.first, retyped.first);
    }

    /// A revision that only widens a field from `T` to `optional<T>`
    /// (docs/adr/0055, section 6) declares, as a saved state names it, what the
    /// revision before it declared: a digest names each field by the type its
    /// present value is, so the widened field's earlier revisions keep theirs and
    /// a state saved at either revision restores into a build of the other.
    #[test]
    fn a_revision_that_widens_a_field_writes_the_digest_of_the_revision_before() {
        const LEVEL_WIDENED_AT_3: &str = r#"<data id="level" sce:type="optional&lt;uint8&gt;" sce:direction="in" sce:optional-since="3"/>"#;
        let sets = format!("{SET_LEVEL}{SET_URGENT}");
        let second = reading_shape(&reading_schema(2, &[LEVEL, URGENT]), &sets);
        let unwidened = reading_shape(&reading_schema(3, &[LEVEL, URGENT]), &sets);
        let widened = reading_shape(&reading_schema(3, &[LEVEL_WIDENED_AT_3, URGENT]), &sets);
        assert_eq!(widened.records[0].revision, 3);
        assert_eq!(widened.records[0].revisions, unwidened.records[0].revisions);
        // Revision 3 widened only, so its digest is revision 2's, which a build
        // of revision 2 computes alike.
        assert_eq!(
            widened.records[0].revisions,
            [
                second.records[0].revisions[0].clone(),
                second.records[0].revisions[0].clone()
            ]
        );
        assert_eq!(widened.first, second.first);
        assert_eq!(widened.revised, second.revised);
        // The fields a later revision added are what they were: a widening adds
        // none.
        let added: Vec<(&str, u32)> = widened.records[0]
            .added
            .iter()
            .map(|f| (f.id.as_str(), f.since))
            .collect();
        assert_eq!(added, [("urgent", 2)]);
        // What a restore refuses a `null` of: the widened field with the
        // revision that added it and the one that widened it, and none for a
        // schema that widened nothing.
        let widened_fields: Vec<(&str, u32, u32)> = widened.records[0]
            .widened
            .iter()
            .map(|f| (f.id.as_str(), f.since, f.optional_since))
            .collect();
        assert_eq!(widened_fields, [("level", 1, 3)]);
        assert!(unwidened.records[0].widened.is_empty());
        assert!(second.records[0].widened.is_empty());
    }

    /// The record reader a C machine writes refuses a `null` in a field saved
    /// before the revision that widened it (docs/adr/0055, section 6): the check
    /// names the field's path and both revisions, is made only for the field
    /// that was widened, and only for a record that held the field as a value
    /// (saved at or after the revision that added it, before the one that
    /// widened it).
    #[test]
    fn a_c_record_reader_refuses_a_null_saved_before_the_field_was_widened() {
        const LEVEL_WIDENED_AT_3: &str = r#"<data id="level" sce:type="optional&lt;uint8&gt;" sce:direction="in" sce:optional-since="3"/>"#;
        const NOTE_ADDED_AT_2_WIDENED_AT_3: &str = r#"<data id="note" sce:type="optional&lt;uint8&gt;" sce:direction="in" sce:since="2" sce:default="7" sce:optional-since="3"/>"#;
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(
            dir.path().join("schema_reading.scxml"),
            reading_schema(
                3,
                &[LEVEL_WIDENED_AT_3, URGENT, NOTE_ADDED_AT_2_WIDENED_AT_3],
            ),
        )
        .expect("the schema");
        let path = dir.path().join("m.scxml");
        std::fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="reading" datamodel="sce-static" name="m">
  <sce:import kind="event-schema" src="schema_reading.scxml" as="Reading"/>
  <datamodel>
    <data id="last" sce:type="record:Reading">
      <sce:set name="level" expr="0"/>
      <sce:set name="urgent" expr="false"/>
      <sce:set name="note" expr="0"/>
    </data>
  </datamodel>
  <state id="reading"/>
</scxml>"#,
        )
        .expect("the statechart");
        let mut model = SCXMLParser::new()
            .parse_file(path.to_str().expect("a UTF-8 path"))
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let schema = model
            .imported_records
            .get("Reading")
            .expect("the schema Reading imports");
        let (_, reads) = c_record_saved_members(
            schema,
            &std::collections::BTreeMap::new(),
            &CTarget::arguments(),
        );
        // `level` is of the first revision: judged for a record saved before 3.
        assert!(
            reads.contains("if (at < 3u && sce_jv_is_null(member))"),
            "{reads}"
        );
        // `note` was added at 2: a record saved before it holds no member to judge.
        assert!(
            reads.contains("if (at >= 2u && at < 3u && sce_jv_is_null(member))"),
            "{reads}"
        );
        assert!(reads.contains("is null, which revision %u of 'reading.taken'"));
        assert!(reads.contains("revision 3 widened it"));
        // `urgent` is a value of a bool in every revision: nothing is judged of it.
        assert_eq!(reads.matches("widened it").count(), 2, "{reads}");
    }

    /// An optional field a later revision added with no default (docs/adr/0055,
    /// section 4) is grown into a record saved before it as `null`: a sender
    /// that predates the field said nothing about it, which is what absent means.
    #[test]
    fn an_optional_field_added_with_no_default_grows_a_saved_record_absent() {
        const REMIND: &str = r#"<data id="remind" sce:type="optional&lt;uint16&gt;" sce:direction="in" sce:since="2"/>"#;
        let shape = reading_shape(
            &reading_schema(2, &[LEVEL, REMIND]),
            &format!("{SET_LEVEL}<sce:set name=\"remind\" expr=\"null\"/>"),
        );
        let added: Vec<(&str, u32, &str)> = shape.records[0]
            .added
            .iter()
            .map(|f| (f.id.as_str(), f.since, f.default.as_str()))
            .collect();
        assert_eq!(added, [("remind", 2, "null")]);
    }

    /// What a saved state is bound to for a machine holding `level`, a variable
    /// of the enum at `enum_path` (relative to the machine's own directory), and
    /// `last`, a record whose field `level` is of it. The machine and the schema
    /// both import the enum under `alias`, as a record's enum field requires.
    fn enum_held_shape(enum_path: &str, alias: &str) -> SavedShape {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let enum_file = dir.path().join(enum_path);
        std::fs::create_dir_all(enum_file.parent().expect("the enum's directory"))
            .expect("the enum's directory");
        std::fs::write(
            &enum_file,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="0"/>
      <sce:variant name="high" value="1"/>
    </data>
  </datamodel>
</scxml>"#,
        )
        .expect("the enum");
        std::fs::write(
            dir.path().join("schema_mark.scxml"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_mark" sce:event-name="mark.set">
  <sce:import src="{enum_path}" kind="enum" as="{alias}"/>
  <datamodel>
    <data id="level" sce:type="enum:{alias}" sce:direction="in"/>
  </datamodel>
</scxml>"#
            ),
        )
        .expect("the schema");
        let statechart = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="sce-static" name="m">
  <sce:import kind="event-schema" src="schema_mark.scxml" as="Mark"/>
  <sce:import kind="enum" src="{enum_path}" as="{alias}"/>
  <datamodel>
    <data id="level" sce:type="enum:{alias}" expr="{alias}.low"/>
    <data id="last" sce:type="record:Mark"><sce:set name="level" expr="{alias}.high"/></data>
  </datamodel>
  <state id="idle"/>
</scxml>"#
        );
        let path = dir.path().join("m.scxml");
        std::fs::write(&path, statechart).expect("the statechart");
        let mut model = SCXMLParser::new()
            .parse_file(path.to_str().expect("a UTF-8 path"))
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_rust(&mut model, "M")
            .expect("lowers")
            .saved_shape
            .expect("a machine of an enum and a record has a shape")
    }

    #[test]
    fn the_format_2_shape_names_an_enum_by_its_document_and_format_1_by_its_alias() {
        let level = enum_held_shape("enum_level.scxml", "Level");
        // docs/adr/0045, section 2: the format-2 shape spells the enum by the
        // key `revisions` names it by, so the two formats' shapes differ for a
        // machine that holds an enum.
        assert_ne!(level.first, level.format2_first);
        assert_ne!(level.revised, level.format2_revised);
        let keys: Vec<&str> = level.enums.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["enum:enum_level.scxml"]);

        // The alias renamed, in the machine and in the schema: format 2 names the
        // same document, so a state saved under the old alias still restores;
        // format 1 spells the alias, as it always has.
        let renamed = enum_held_shape("enum_level.scxml", "Grade");
        assert_eq!(renamed.format2_first, level.format2_first);
        assert_eq!(renamed.format2_revised, level.format2_revised);
        assert_ne!(renamed.first, level.first);
        assert_ne!(renamed.revised, level.revised);

        // The enum's file moved: another document to every reader, so its key
        // and the format-2 shape move with it. Format 1 names no document, and
        // stays as it was.
        let moved = enum_held_shape("kinds/enum_level.scxml", "Level");
        let keys: Vec<&str> = moved.enums.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["enum:kinds/enum_level.scxml"]);
        assert_ne!(moved.format2_first, level.format2_first);
        assert_ne!(moved.format2_revised, level.format2_revised);
        assert_eq!(moved.first, level.first);
    }

    #[test]
    fn a_machine_that_holds_no_enum_has_one_shape_in_both_formats() {
        let counter = {
            let mut model = SCXMLParser::new()
                .parse_string(COUNTER, "m")
                .expect("parses");
            crate::analyzer::analyze(&mut model, "m.scxml");
            lower_rust(&mut model, "M")
                .expect("lowers")
                .saved_shape
                .expect("a machine of fields alone has a shape")
        };
        assert_eq!(counter.format2_first, counter.first);
        assert_eq!(counter.format2_revised, counter.revised);
    }

    #[test]
    fn the_saved_shape_follows_what_a_saved_state_names_and_nothing_else() {
        let base = shape(COUNTER).expect("a machine of fields alone has a shape");
        assert_eq!(
            shape(&COUNTER.replace("count &lt; 10", "count &lt; 20")),
            Some(base.clone()),
            "a guard rewritten: a saved state still restores"
        );
        assert_eq!(
            shape(&COUNTER.replace(r#"expr="0""#, r#"expr="3""#)),
            Some(base.clone()),
            "an initial value changed: a restore never evaluates it"
        );
        assert_ne!(
            shape(&COUNTER.replace(r#"sce:type="uint32""#, r#"sce:type="uint16""#)),
            Some(base.clone()),
            "a variable re-typed"
        );
        assert_ne!(
            shape(
                &COUNTER
                    .replace(r#"final id="done""#, r#"final id="finished""#)
                    .replace(r#"target="done""#, r#"target="finished""#)
            ),
            Some(base),
            "a state renamed"
        );
    }

    /// `COUNTER` with `counting` inside a compound `outer`, which carries
    /// `history` (an element, or nothing).
    fn counter_in_outer(history: &str) -> String {
        COUNTER
            .replace(
                r#"<state id="counting">"#,
                &format!(r#"<state id="outer" initial="counting">{history}<state id="counting">"#),
            )
            .replace(r#"<final id="done"/>"#, r#"</state><final id="done"/>"#)
    }

    #[test]
    fn a_history_is_part_of_the_shape_a_saved_state_is_bound_to() {
        let none = shape(&counter_in_outer("")).expect("a machine of fields alone has a shape");
        let shallow = shape(&counter_in_outer(
            r#"<history id="h"><transition target="counting"/></history>"#,
        ))
        .expect("a machine with a history has one too");
        assert_ne!(shallow, none, "the history is declared");
        assert_ne!(
            shape(&counter_in_outer(
                r#"<history id="h" type="deep"><transition target="counting"/></history>"#
            ))
            .expect("shape"),
            shallow,
            "shallow or deep is what the recorded value means"
        );
        assert_ne!(
            shape(&counter_in_outer(
                r#"<history id="g"><transition target="counting"/></history>"#
            ))
            .expect("shape"),
            shallow,
            "the id is what a saved state keys the value by"
        );
    }

    /// `COUNTER` whose `go` transition makes `send`.
    fn counter_sending(send: &str) -> String {
        COUNTER.replace(
            r#"<transition event="go" target="done"/>"#,
            &format!(r#"<transition event="go" target="done">{send}</transition>"#),
        )
    }

    #[test]
    fn a_delayed_send_to_this_session_changes_nothing_a_saved_state_names() {
        // A waiting send to this session's own queues is held by the saved
        // state as the moment it comes due, so a machine that makes one has a
        // shape — and the same one: it adds no state or variable.
        let base = shape(COUNTER).expect("shape");
        for send in [
            r#"<send event="later" delay="5s"/>"#,
            r#"<send id="timer" event="later" delay="5s"/>"#,
            r##"<send event="later" target="#_internal" delay="5s"/>"##,
        ] {
            assert_eq!(shape(&counter_sending(send)), Some(base.clone()), "{send}");
        }
        assert_eq!(
            shape(&counter_sending(r#"<cancel sendid="timer"/>"#)),
            Some(base),
            "a <cancel> names a send of this session and is state of no other"
        );
    }

    #[test]
    fn a_delayed_send_to_a_session_outside_the_tree_has_no_saved_shape() {
        // It is delivered through a session the saved tree does not carry: a
        // session named by its id, or a child this document does not invoke.
        for target in ["#_child", "#_scxml_session"] {
            let send = format!(r#"<send event="later" target="{target}" delay="5s"/>"#);
            assert_eq!(shape(&counter_sending(&send)), None, "{send}");
        }
    }

    #[test]
    fn a_delayed_send_to_the_parent_or_a_child_keeps_the_saved_shape() {
        // docs/adr/0045: the parent and a child of this session are sessions of
        // the saved tree, so a send waiting on either is held as the moment it
        // comes due, and adds nothing the shape names.
        let base = shape(COUNTER).expect("shape");
        let to_parent = r##"<send event="later" target="#_parent" delay="5s"/>"##;
        assert_eq!(
            shape(&counter_sending(to_parent)),
            Some(base),
            "{to_parent}"
        );
        let invoking = shape(&counter_invoking(&child_session("worker"))).expect("shape");
        let to_child = counter_invoking(&child_session("worker")).replace(
            r#"<transition event="go" target="done"/>"#,
            r##"<transition event="go" target="done"><send event="later" target="#_worker" delay="5s"/></transition>"##,
        );
        assert_ne!(to_child, counter_invoking(&child_session("worker")));
        assert_eq!(
            shape(&to_child),
            Some(invoking),
            "a send to the child it invokes"
        );
    }

    /// `COUNTER` whose `counting` state holds `invoke`.
    fn counter_invoking(invoke: &str) -> String {
        COUNTER.replace(
            r#"<state id="counting">"#,
            &format!(r#"<state id="counting">{invoke}"#),
        )
    }

    /// A static child session of `id`, which ends as it starts: a
    /// `datamodel="sce-static"` document, whose state a saved tree holds.
    fn child_session(id: &str) -> String {
        child_session_of(id, r#"datamodel="sce-static""#)
    }

    /// A child session of `id` whose root carries `attributes`.
    fn child_session_of(id: &str, attributes: &str) -> String {
        format!(
            r#"<invoke type="scxml" id="{id}"><content>
          <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f" {attributes}><final id="f"/></scxml>
        </content></invoke>"#
        )
    }

    #[test]
    fn a_child_whose_state_a_saved_tree_cannot_hold_takes_the_saved_shape_away() {
        // docs/adr/0045, "Which documents save": a restore resumes every child
        // it was running from the child's own saved state, so a parent saves
        // only when each child it can start does. A child under another data
        // model has no saved state at all.
        assert!(shape(&counter_invoking(&child_session("worker"))).is_some());
        assert_eq!(
            shape(&counter_invoking(&child_session_of("worker", ""))),
            None,
            "a child under the null data model"
        );
        // A child that itself waits on a session outside its tree.
        let waiting = r##"<invoke type="scxml" id="worker"><content>
          <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="sce-static">
            <state id="s"><onentry><send event="later" target="#_scxml_other" delay="5s"/></onentry></state>
          </scxml>
        </content></invoke>"##;
        assert_eq!(shape(&counter_invoking(waiting)), None, "{waiting}");
        // A grandchild that cannot be held takes it away from the root too.
        let grandparent = format!(
            r#"<invoke type="scxml" id="worker"><content>
          <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="sce-static">
            <state id="s">{}</state>
          </scxml>
        </content></invoke>"#,
            child_session_of("inner", "")
        );
        assert_eq!(shape(&counter_invoking(&grandparent)), None, "a grandchild");
        let held = format!(
            r#"<invoke type="scxml" id="worker"><content>
          <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s" datamodel="sce-static">
            <state id="s">{}</state>
          </scxml>
        </content></invoke>"#,
            child_session("inner")
        );
        assert!(
            shape(&counter_invoking(&held)).is_some(),
            "a grandchild held too"
        );
    }

    #[test]
    fn a_static_child_session_is_part_of_the_shape_a_saved_state_is_bound_to() {
        // A saved state names the child it starts again, by id, and the state
        // that holds it: a document that renamed the invocation or moved it to
        // another state is one a saved state must refuse.
        let none = shape(COUNTER).expect("shape");
        let worker = shape(&counter_invoking(&child_session("worker")))
            .expect("a machine whose only invoke is a child session has a shape");
        assert_ne!(worker, none, "the invocation is declared");
        assert_ne!(
            shape(&counter_invoking(&child_session("helper"))).expect("shape"),
            worker,
            "the id is what a saved state names it by"
        );
        let moved = COUNTER
            .replace(
                r#"<final id="done"/>"#,
                &format!(
                    r#"<state id="after">{}</state><final id="done"/>"#,
                    child_session("worker")
                ),
            )
            .replace(
                r#"<transition event="go" target="done"/>"#,
                r#"<transition event="go" target="after"/>"#,
            );
        assert_ne!(
            shape(&moved).expect("shape"),
            worker,
            "the state that holds it decides whether a restore may start it"
        );
    }

    /// [`shape`] of a document whose host declared an invoker for each of
    /// `invoke_types`, as the generator reads one.
    fn shape_declaring(document: &str, invoke_types: &[&str]) -> Option<String> {
        let mut model = SCXMLParser::new()
            .parse_string(document, "m")
            .expect("parses");
        let types: Vec<String> = invoke_types.iter().map(|t| t.to_string()).collect();
        crate::host_processor_analyzer::declare_host_surfaces(&mut model, &[], &types)
            .expect("declares");
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_rust(&mut model, "M")
            .expect("lowers")
            .saved_shape
            .map(|shape| shape.first)
    }

    #[test]
    fn a_host_run_invocation_is_part_of_the_shape_a_saved_state_is_bound_to() {
        // A host-run invocation was started with a request the saved state
        // holds, and starts again from it: a document that hands the host a job
        // has a shape that names the type, the id and the state.
        let host = r#"<invoke type="x-sce-host" id="h"/>"#;
        let base = shape(COUNTER).expect("shape");
        let declared = shape_declaring(&counter_invoking(host), &["x-sce-host"])
            .expect("an invocation a declared invoker serves has a shape");
        assert_ne!(declared, base, "the invocation is named");
        assert_ne!(
            shape_declaring(
                &counter_invoking(&host.replace("x-sce-host", "x-other")),
                &["x-other"]
            )
            .expect("shape"),
            declared,
            "the type is what the host was handed it under"
        );
        assert_ne!(
            shape_declaring(
                &counter_invoking(&host.replace(r#"id="h""#, r#"id="g""#)),
                &["x-sce-host"]
            )
            .expect("shape"),
            declared,
            "the id is what `done.invoke.<id>` names"
        );
        // Beside a child session, both are named.
        assert_ne!(
            shape_declaring(
                &counter_invoking(&format!("{}{host}", child_session("worker"))),
                &["x-sce-host"]
            )
            .expect("shape"),
            declared
        );
    }

    #[test]
    fn an_invoke_no_host_serves_changes_nothing_a_saved_state_names() {
        // Its start is refused with `error.execution`, so nothing is running and
        // a saved state has nothing to say of it: the machine saves, with the
        // shape it would have without the element.
        let base = shape(COUNTER).expect("shape");
        assert_eq!(
            shape(&counter_invoking(r#"<invoke type="x-sce-host" id="h"/>"#)),
            Some(base)
        );
    }

    #[test]
    fn an_invoke_a_restore_cannot_start_has_no_saved_shape() {
        // A Mesh request, before it is lowered to a host-served invoke: its
        // request may already have reached the peer, and sending it again would
        // have the peer act on it twice.
        let mesh = r##"<invoke type="sce:mesh-rpc" id="ask" src="#peer">
          <param name="_mesh_event" expr="'service.request'"/>
        </invoke>"##;
        assert_eq!(shape(&counter_invoking(mesh)), None);
        // And beside a child session, which on its own would save.
        assert_eq!(
            shape(&counter_invoking(&format!(
                "{}{mesh}",
                child_session("worker")
            ))),
            None
        );
    }

    #[test]
    fn a_delayed_sends_target_is_a_literal_under_sce_static() {
        // `delays_a_send_to_another_session` reads the target as written, or the
        // routes the document declares for a computed one. A target that is only
        // known at run time and declares nothing may be any session, so it is
        // refused where it is written, naming the attribute that would say which.
        let send = r#"<send event="later" targetexpr="where" delay="5s"/>"#;
        let refusal = SCXMLParser::new()
            .parse_string(&counter_sending(send), "m")
            .expect_err(send);
        assert!(
            format!("{refusal:?}").contains("add `sce:targets`"),
            "{send}: {refusal:?}"
        );
    }

    /// `COUNTER` with a string variable `route` for a `targetexpr` to read,
    /// whose `go` transition makes `send`.
    fn counter_routing(send: &str) -> String {
        counter_sending(send).replace(
            r#"<data id="count" sce:type="uint32" expr="0"/>"#,
            r#"<data id="count" sce:type="uint32" expr="0"/>
    <data id="route" sce:type="string" sce:capacity="16" expr="'#_internal'"/>"#,
        )
    }

    #[test]
    fn a_delayed_send_to_a_computed_target_waits_on_another_session() {
        // The declared routes are the sessions the expression could name: one
        // outside the saved tree makes a delayed send a send waiting on another
        // session, as a written target of it would.
        for routes in [
            "#_scxml_peer",
            "#_internal #_scxml_peer",
            "#_scxml_peer #_parent",
        ] {
            let send = format!(
                r#"<send event="later" targetexpr="route" sce:targets="{routes}" delay="5s"/>"#
            );
            let model = SCXMLParser::new()
                .parse_string(&counter_routing(&send), "m")
                .expect("parses");
            assert!(
                super::delays_a_send_to_another_session(&model),
                "{routes}: a delayed send that may go to another session"
            );
        }
    }

    #[test]
    fn a_delayed_send_to_computed_targets_of_the_tree_waits_on_no_other() {
        // This session's own queue and its parent are both in the saved tree.
        for routes in ["#_internal", "#_internal #_parent"] {
            let send = format!(
                r#"<send event="later" targetexpr="route" sce:targets="{routes}" delay="5s"/>"#
            );
            let model = SCXMLParser::new()
                .parse_string(&counter_routing(&send), "m")
                .expect("parses");
            assert!(!super::delays_a_send_to_another_session(&model), "{routes}");
        }
    }

    #[test]
    fn a_computed_delay_to_another_session_has_no_saved_shape() {
        // The time is known only when the send runs, so it may be one that
        // waits: the machine is told apart from one that delays by nothing.
        for target in ["#_child", "#_scxml_session"] {
            let send = format!(r#"<send event="later" target="{target}" delayexpr="'5s'"/>"#);
            assert_eq!(shape(&counter_sending(&send)), None, "{send}");
        }
    }

    #[test]
    fn a_computed_delay_to_this_session_keeps_its_saved_shape() {
        // A saved state holds a send to this session as the moment it comes due.
        for target in ["", r##" target="#_internal""##] {
            let send = format!(r##"<send event="later"{target} delayexpr="'5s'"/>"##);
            assert!(shape(&counter_sending(&send)).is_some(), "{send}");
        }
    }

    #[test]
    fn a_send_to_another_session_that_is_not_delayed_waits_for_nothing() {
        // Delivered at once, it leaves nothing in the machine to save.
        assert!(shape(&counter_sending(
            r##"<send event="now" target="#_parent"/>"##
        ))
        .is_some());
    }

    #[test]
    fn a_delayed_send_in_a_history_default_is_seen() {
        // A `<history>`'s default transition belongs to its parent state, and
        // is not among the blocks a state's own transitions carry.
        let history = r##"<history id="h"><transition target="counting"><send event="later" target="#_scxml_session" delay="5s"/></transition></history>"##;
        assert_eq!(shape(&counter_in_outer(history)), None);
    }

    // ── a string a variable holds ───────────────────────────────────────────
    //
    // A string inside a Rust expression is borrowed (`&str`) and a variable
    // holds an owned `String`, so a value written into one has to be made
    // owned on the way in — as a value read out of one into a host call's
    // `&str` parameter must NOT be. Kotlin has one `String` for both.

    const WITH_STRING: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="label" sce:type="string" sce:capacity="16" expr="'idle'"/>
    <data id="other" sce:type="string" sce:capacity="16" expr="'x'"/>
  </datamodel>
  <state id="s">
    <transition event="go" target="done">
      <assign location="label" expr="'busy'"/>
      <assign location="other" expr="label"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"#;

    /// The initial values of `WITH_STRING`'s fields and the two statements its
    /// `go` transition lowers to, for `lang`.
    fn lowered_strings(lang: Language) -> (Vec<String>, Vec<String>) {
        let mut model = SCXMLParser::new()
            .parse_string(WITH_STRING, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowering = match lang {
            Language::Rust => lower_rust(&mut model, "M"),
            Language::Kotlin => lower_kotlin(&mut model, "M"),
            other @ (Language::Cpp
            | Language::Go
            | Language::Python
            | Language::C11
            | Language::Swift) => {
                panic!("{other:?} does not lower sce-static")
            }
        }
        .expect("lowers");
        let statements = model.states["s"].transitions[0]
            .actions
            .iter()
            .map(|a| a.native_code.clone())
            .collect();
        (
            lowering.fields.iter().map(|f| f.init.clone()).collect(),
            statements,
        )
    }

    #[test]
    fn a_string_written_into_a_rust_variable_is_made_owned() {
        let (inits, statements) = lowered_strings(Language::Rust);
        // A literal is borrowed, and so is a read of another variable.
        assert!(inits[0].ends_with(".to_string()"), "{inits:?}");
        assert!(inits[1].ends_with(".to_string()"), "{inits:?}");
        assert_eq!(statements.len(), 2, "{statements:?}");
        for statement in &statements {
            assert!(statement.contains(".to_string()"), "{statements:?}");
        }
    }

    // ── a `<param>` read from the machine's fields ──────────────────────────
    //
    // What a `<send>` carries is lowered to the typed value the backend's wire
    // helpers take, so it crosses exactly as a script engine's would: Rust
    // builds a `ScriptValue`, Kotlin widens to what `valueToWireString` reads.

    const WITH_SEND_PARAMS: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="flag" sce:type="bool" expr="true"/>
    <data id="label" sce:type="string" sce:capacity="16" expr="'x'"/>
    <data id="small" sce:type="uint8" expr="1"/>
    <data id="signed" sce:type="int16" expr="-2"/>
    <data id="real" sce:type="float64" expr="0.5"/>
    <data id="single" sce:type="float32" expr="0.5"/>
  </datamodel>
  <state id="s">
    <transition event="go" target="done">
      <send event="out">
        <param name="flag" expr="flag"/>
        <param name="label" expr="label"/>
        <param name="small" expr="small"/>
        <param name="signed" expr="signed"/>
        <param name="real" expr="real"/>
        <param name="single" expr="single"/>
        <param name="sum" expr="small + 1"/>
        <param name="lit" expr="'lit'"/>
        <param name="loc" location="small"/>
      </send>
    </transition>
  </state>
  <final id="done"/>
</scxml>"#;

    /// `WITH_SEND_PARAMS`'s params after lowering for `lang`, by name: the
    /// typed value and whether it can fail.
    fn lowered_params(lang: Language) -> std::collections::BTreeMap<String, (String, bool)> {
        let mut model = SCXMLParser::new()
            .parse_string(WITH_SEND_PARAMS, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        match lang {
            Language::Rust => lower_rust(&mut model, "M"),
            Language::Kotlin => lower_kotlin(&mut model, "M"),
            Language::C11 => lower_c11(&mut model, ""),
            other @ (Language::Cpp | Language::Go | Language::Python | Language::Swift) => {
                panic!("{other:?} does not lower sce-static")
            }
        }
        .expect("lowers");
        model.states["s"].transitions[0].actions[0]
            .params
            .iter()
            .map(|p| (p.name.clone(), (p.native_value.clone(), p.native_fails)))
            .collect()
    }

    #[test]
    fn a_rust_param_is_a_script_value_of_the_type_the_expression_has() {
        let params = lowered_params(Language::Rust);
        let value = |name: &str| params[name].0.as_str();
        let rt = "::sce_rust_runtime::ScriptValue::";
        assert!(
            value("flag").starts_with(&format!("{rt}Bool(")),
            "{params:?}"
        );
        assert!(
            value("label").starts_with(&format!("{rt}String("))
                && value("label").contains(".to_string()"),
            "an owned string: {params:?}"
        );
        for narrow in ["small", "signed", "sum", "loc"] {
            assert!(
                value(narrow).starts_with(&format!("{rt}Int(i64::from(")),
                "{narrow}: an integer of at most 32 bits widens to i64: {params:?}"
            );
        }
        assert!(
            value("real").starts_with(&format!("{rt}Double(")) && !value("real").contains("from"),
            "{params:?}"
        );
        assert!(
            value("single").starts_with(&format!("{rt}Double(f64::from(")),
            "{params:?}"
        );
    }

    #[test]
    fn a_kotlin_param_is_widened_to_what_the_wire_helpers_read() {
        let params = lowered_params(Language::Kotlin);
        let value = |name: &str| params[name].0.as_str();
        // A `Boolean`, a `String` and a `Double` are read as they are.
        for plain in ["flag", "label", "real"] {
            assert!(
                !value(plain).is_empty()
                    && !value(plain).contains("toLong")
                    && !value(plain).contains("toDouble"),
                "{plain}: {params:?}"
            );
        }
        // Kotlin's unsigned types are not `Number`, and a `Long` is exact in a
        // `Double`: so an integer of at most 32 bits is widened to one.
        for narrow in ["small", "signed", "sum", "loc"] {
            assert!(value(narrow).ends_with(".toLong()"), "{narrow}: {params:?}");
        }
        assert!(value("single").ends_with(".toDouble()"), "{params:?}");
    }

    #[test]
    fn a_c_param_is_the_typed_wire_value_and_a_single_crosses_as_the_double_it_widens_to() {
        let params = lowered_params(Language::C11);
        let value = |name: &str| params[name].0.as_str();
        assert!(
            value("real").starts_with("sce_forge_wire_real((double)("),
            "{params:?}"
        );
        // A 32-bit real is written as the 64-bit real it is exactly, so the one
        // spelling the contract pins is the one it takes.
        assert!(
            value("single").starts_with("sce_forge_wire_real((double)("),
            "{params:?}"
        );
        for narrow in ["small", "sum", "loc"] {
            assert!(
                value(narrow).starts_with("sce_forge_wire_uint((uint64_t)("),
                "{narrow}: {params:?}"
            );
        }
        assert!(
            value("signed").starts_with("sce_forge_wire_int((int64_t)("),
            "{params:?}"
        );
    }

    // ── a 32-bit real a variable holds ──────────────────────────────────────
    //
    // A single is rounded to binary32 where an operation on it is made. The
    // backends that have one compute it as one and need the literal written as
    // one; Python holds every real in a double, so the lowering writes the
    // rounding itself.

    const WITH_SINGLE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="tenth" sce:type="float32" expr="0.1"/>
    <data id="drift" sce:type="float32" expr="0"/>
    <data id="wide" sce:type="float64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" target="done">
      <assign location="drift" expr="tenth + 0.2"/>
      <assign location="wide" expr="tenth + 0.2"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"#;

    /// The initial values of `WITH_SINGLE`'s fields and the two statements its
    /// `go` transition lowers to, for `lang`.
    fn lowered_single(lang: Language) -> (Vec<String>, Vec<String>) {
        let mut model = SCXMLParser::new()
            .parse_string(WITH_SINGLE, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowering = match lang {
            Language::Kotlin => lower_kotlin(&mut model, "M"),
            Language::Python => lower_python(&mut model, "M"),
            Language::C11 => lower_c11(&mut model, ""),
            other @ (Language::Rust | Language::Cpp | Language::Go | Language::Swift) => {
                panic!("{other:?} is not lowered by this test")
            }
        }
        .expect("lowers");
        let statements = model.states["s"].transitions[0]
            .actions
            .iter()
            .map(|a| a.native_code.clone())
            .collect();
        (
            lowering.fields.iter().map(|f| f.init.clone()).collect(),
            statements,
        )
    }

    #[test]
    fn a_c_machine_holds_a_single_and_writes_its_literals_as_singles() {
        let (inits, statements) = lowered_single(Language::C11);
        assert_eq!(inits, ["0.1f", "0.0f", "0.0"], "{inits:?}");
        // In a `float` slot the sum is a `float`'s, in a `double` slot a
        // `double`'s: the literal is the partner's width.
        assert!(statements[0].contains("v_tenth + 0.2f"), "{statements:?}");
        assert!(
            statements[1].contains("v_tenth + 0.2;") && !statements[1].contains("0.2f"),
            "{statements:?}"
        );
    }

    #[test]
    fn a_kotlin_machine_writes_its_literals_beside_a_single_as_singles() {
        let (inits, statements) = lowered_single(Language::Kotlin);
        assert_eq!(inits, ["0.1f", "0.0f", "0.0"], "{inits:?}");
        assert!(statements[0].contains("vTenth + 0.2f"), "{statements:?}");
        assert!(
            statements[1].contains("vTenth.toDouble() + 0.2") && !statements[1].contains("0.2f"),
            "{statements:?}"
        );
    }

    #[test]
    fn a_python_machine_rounds_what_it_computes_as_a_single_to_one() {
        let (inits, statements) = lowered_single(Language::Python);
        assert_eq!(
            inits,
            ["sce_algorithm.to_f32(0.1)", "sce_algorithm.to_f32(0)", "0"],
            "{inits:?}"
        );
        assert!(
            statements[0]
                .contains("sce_algorithm.to_f32(self.v_tenth + sce_algorithm.to_f32(0.2))"),
            "{statements:?}"
        );
        // A double's sum is not rounded: nothing in it is made as a single.
        assert!(
            statements[1].contains("self.v_tenth + 0.2") && !statements[1].contains("to_f32"),
            "{statements:?}"
        );
    }

    #[test]
    fn a_c_machine_holds_a_list_of_singles_and_appends_a_single_to_it() {
        let document = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="tenth" sce:type="float32" expr="0.1"/>
    <data id="samples" sce:type="list&lt;float32&gt;" sce:capacity="3"/>
  </datamodel>
  <state id="s">
    <transition event="go" target="done">
      <sce:append target="samples" expr="tenth + 0.2"/>
    </transition>
  </state>
  <final id="done"/>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(document, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        lower_c11(&mut model, "").expect("a list of `float` is held by C11");
        let statement = &model.states["s"].transitions[0].actions[0].native_code;
        // The element is the sum a single makes: the literal is the partner's width.
        assert!(statement.contains("v_tenth + 0.2f"), "{statement}");
    }

    #[test]
    fn only_a_checked_operation_can_fail_and_a_literal_is_left_alone() {
        for lang in [Language::Rust, Language::Kotlin] {
            let params = lowered_params(lang);
            for (name, (value, fails)) in &params {
                match name.as_str() {
                    // `small + 1` on a `uint8` is checked.
                    "sum" => assert!(*fails, "{lang:?} {name}: {params:?}"),
                    // A string literal is folded at build time and crosses as
                    // written, so it has no native value at all.
                    "lit" => assert!(value.is_empty() && !*fails, "{lang:?} {name}: {params:?}"),
                    _ => assert!(!*fails && !value.is_empty(), "{lang:?} {name}: {params:?}"),
                }
            }
        }
    }

    #[test]
    fn a_param_naming_a_location_is_read_as_the_variable_it_names() {
        for lang in [Language::Rust, Language::Kotlin] {
            let params = lowered_params(lang);
            assert_eq!(
                params["loc"].0, params["small"].0,
                "{lang:?}: `location=\"small\"` and `expr=\"small\"` read the same field"
            );
        }
    }

    #[test]
    fn a_string_written_into_a_kotlin_variable_needs_no_conversion() {
        let (inits, statements) = lowered_strings(Language::Kotlin);
        for text in inits.iter().chain(&statements) {
            assert!(!text.contains("to_string"), "{inits:?} {statements:?}");
            assert!(!text.contains("toString"), "{inits:?} {statements:?}");
        }
    }

    /// A `<send event="e" typeexpr="kind">` declaring `types`.
    fn send_computing_its_type(types: &[&str]) -> Action {
        Action {
            action_type: "send".to_string(),
            event: "e".to_string(),
            typeexpr: " kind ".to_string(),
            types: types.iter().map(|t| t.to_string()).collect(),
            ..Action::default()
        }
    }

    #[test]
    fn a_computed_type_expands_to_one_send_for_each_processor_it_declares() {
        use crate::host_processor_analyzer::{
            BASIC_HTTP_EVENT_PROCESSOR_TYPE as HTTP, SCXML_EVENT_PROCESSOR_TYPE as SCXML,
        };
        let expanded = expand_computed_type(&send_computing_its_type(&[SCXML, HTTP]), 7);
        assert_eq!(expanded.action_type, "if");
        assert_eq!(
            expanded.if_ordinal, 7,
            "the choice takes the ordinal it was given"
        );
        assert_eq!(expanded.cond, format!("(kind) === '{SCXML}'"));

        // Read through the model's one definition of what lies inside an action.
        use crate::model::BlockRole;
        let blocks = expanded.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == BlockRole::Then)
            .expect("the first entry is the then");
        let elseifs: Vec<_> = blocks
            .iter()
            .filter(|block| block.role == BlockRole::ElseIf)
            .collect();
        assert_eq!(elseifs.len(), 1);
        let http_cond = format!("(kind) === '{HTTP}'");
        assert_eq!(elseifs[0].cond, Some(http_cond.as_str()));

        let branches = [(then.actions, SCXML), (elseifs[0].actions, HTTP)];
        for (actions, send_type) in branches {
            let [send] = actions else {
                panic!("a branch holds the one send: {actions:?}")
            };
            assert_eq!(send.send_type, send_type);
            assert_eq!(send.event, "e", "the send is copied whole");
            assert!(
                send.typeexpr.is_empty() && send.types.is_empty(),
                "a branch writes its type out: {send:?}"
            );
            assert!(
                !send.send_type_unsupported,
                "{send_type} is delivered through"
            );
        }
    }

    #[test]
    fn a_computed_type_none_of_the_declared_ones_matches_is_sent_by_a_type_nothing_serves() {
        use crate::host_processor_analyzer::SCXML_EVENT_PROCESSOR_TYPE as SCXML;
        let expanded = expand_computed_type(&send_computing_its_type(&[SCXML]), 1);
        let blocks = expanded.nested_blocks();
        let otherwise = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Else)
            .expect("the choice has an else");
        let [otherwise] = otherwise.actions else {
            panic!("the else holds the one send: {:?}", otherwise.actions)
        };
        assert_eq!(otherwise.send_type, UNDECLARED_SEND_TYPE);
        assert!(
            otherwise.send_type_unsupported,
            "the undeclared type raises error.execution, as a written one nothing serves does"
        );
    }

    /// C++ holds an optional value in `std::optional<T>` and writes the whole
    /// value of an optional place by the walk's one rule (docs/adr/0055): `null`
    /// its absent value, a value of `T` lowered into `T` (a text held to its
    /// bound) and lifted, an optional value as it is; a read under an `<if>`
    /// whose condition proves the value present is `*x`; a record's optional
    /// field is declared over its present value's type.
    #[test]
    fn cpp_writes_an_optional_place_whole_by_the_walks_one_rule() {
        const OPTIONAL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="m">
  <datamodel>
    <data id="due" sce:type="optional&lt;int64&gt;" expr="null"/>
    <data id="other" sce:type="optional&lt;int64&gt;" expr="7"/>
    <data id="title" sce:type="optional&lt;string&gt;" sce:capacity="4" expr="null"/>
    <data id="days" sce:type="int64" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="read" type="internal">
      <if cond="due !== null">
        <assign location="days" expr="due.value"/>
      </if>
    </transition>
    <transition event="write" type="internal">
      <assign location="due" expr="null"/>
      <assign location="title" expr="'abc'"/>
      <assign location="other" expr="due"/>
    </transition>
  </state>
</scxml>"#;
        let mut model = SCXMLParser::new()
            .parse_string(OPTIONAL, "m")
            .expect("parses");
        crate::analyzer::analyze(&mut model, "m.scxml");
        let lowered = lower_cpp(&mut model, "M").expect("C++ lowers an optional value");
        let field = |id: &str| {
            lowered
                .fields
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("a field `{id}`"))
        };
        assert_eq!(field("due").ty, "std::optional<int64_t>");
        assert_eq!(field("due").init, "std::optional<int64_t>()");
        assert_eq!(field("other").init, "std::optional<int64_t>(7)");
        assert_eq!(field("title").ty, "std::optional<std::string>");
        assert!(
            lowered.saved_shape.is_some(),
            "a saved state holds an optional value (docs/adr/0055, step 6)"
        );
        let state = model.states.get("s").expect("the state");
        let read = &state.transitions[0].actions[0];
        assert_eq!(read.native_cond, "v_due.has_value()");
        // Read through the model's one definition of what lies inside an action.
        let blocks = read.nested_blocks();
        let then = blocks
            .iter()
            .find(|block| block.role == crate::model::BlockRole::Then)
            .expect("the <if> has a then block");
        assert_eq!(then.actions[0].native_code, "v_days = (*v_due);");
        let write = &state.transitions[1].actions;
        assert_eq!(write[0].native_code, "v_due = std::optional<int64_t>();");
        assert!(
            write[1].native_code.contains(
                "auto sce_value = std::optional<std::string>(SCE::Forge::Checked::bounded(sce_failure_, "
            ),
            "{}",
            write[1].native_code
        );
        assert_eq!(write[2].native_code, "v_other = v_due;");

        // A record's optional field is held in the optional type over its
        // present value's.
        const SCHEMA: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="schema_due" sce:event-name="due.set">
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
  </datamodel>
</scxml>"#;
        let Some(crate::forge::model::ForgeDocument::EventSchema(schema)) =
            crate::forge::parser::parse_forge(
                SCHEMA,
                crate::DocumentLabel::symmetric("schema_due.scxml"),
            )
            .expect("the schema parses")
        else {
            panic!("an event schema")
        };
        let def = CppTarget.record_def("MDueRecord", "Due", &schema, &Default::default());
        assert!(def.contains("    uint32_t id;\n"), "{def}");
        assert!(def.contains("    std::optional<int64_t> due;\n"), "{def}");
        // The saved form is read back by position, in the order the struct
        // declares its fields, each value beside its field's name: C++17 has
        // no designated initializer (docs/adr/0059, section 5).
        let id = def
            .find("        /*id=*/")
            .expect("the id is read by position");
        let due = def
            .find("        /*due=*/::SCE::Saved::savedReadOptional(")
            .expect("the due is read by position");
        assert!(id < due, "the fields are read in declaration order:\n{def}");
        assert!(!def.contains(".id = "), "no designated initializer:\n{def}");
        assert_eq!(
            CppTarget.record_value(
                "MDueRecord",
                &[
                    ("id".to_string(), "7u".to_string()),
                    ("due".to_string(), "std::nullopt".to_string()),
                ],
            ),
            "MDueRecord{/*id=*/7u, /*due=*/std::nullopt}"
        );
    }
}
