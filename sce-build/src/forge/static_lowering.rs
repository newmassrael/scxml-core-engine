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
use crate::forge::model::{EnumModel, EventSchemaModel, SceType};
use crate::forge::static_datamodel::indexed_element;
use crate::forge::type_ctx::{StaticEnum, StaticScope};
use crate::forge::types::InferredType;
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
    /// How a saved state holds the value: `scalar`, `list` or `record`.
    pub saved_kind: &'static str,
    /// What [`Self::saved_kind`] is of: a scalar's or a list element's
    /// `sce:type` spelling (`uint8`, `bool`, `bytes`, …), or a record's
    /// backend type. A backend without overloading on type reads each value
    /// with the function this names.
    pub saved_type: String,
    /// The name a host reads this field through when the field's own name is
    /// not it ([`StaticTarget::reader_name`]) — `None` for a name the backend
    /// cannot give a reader.
    pub reader: Option<String>,
    /// A record, or a list of records, whose schema has a byte-string field: what
    /// a host reads holds the machine's own bytes unless it is a copy, which a
    /// target whose byte buffer can be written into hands out
    /// (docs/adr/0005, decision 2).
    pub holds_bytes: bool,
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
    /// The shape a saved state of this machine is bound to ([`saved_shape`]),
    /// or `None` for a machine whose state a saved state cannot yet hold.
    pub saved_shape: Option<String>,
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

/// One field of a [`StaticRecord`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaticRecordField {
    /// The schema's id — the key a saved state writes it under, the same on
    /// every backend.
    pub id: String,
    /// The backend identifier ([`StaticTarget::record_field`]).
    pub name: String,
    /// The field's `sce:type` spelling (see [`StaticField::saved_type`]).
    pub saved_type: String,
    /// The most UTF-8 bytes a `string` field holds, or the most bytes a `bytes`
    /// field does, which a saved state being restored is held to as the machine is
    /// (the `sce:max-size` its schema declares); `None` for a field that is neither.
    pub bound: Option<u32>,
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

/// An imported algorithm as a target reaches it: the name a call is written
/// with, and the line that makes the name visible where the machine is.
#[derive(Debug, Clone)]
pub struct Callee {
    /// The callee, qualified as the generated unit that holds it requires.
    pub call: String,
    /// The import line a forge kind importing the same algorithm writes, so
    /// the machine reaches the function where the algorithm's own generation
    /// put it. `None` when the target was built without what the line needs —
    /// a Go target without the module path its packages live under — and a
    /// call is all it can then spell: [`lower`] refuses a document that calls
    /// the algorithm rather than leave the name unimported.
    pub import: Option<String>,
}

/// [`Callee`] for a generated-code backend: the identity and symbol every
/// other forge kind derives for the same document, so a statechart and an
/// algorithm that imports it agree on where it lives.
///
/// `go_module_prefix` is what the Go import path is rooted at, and the one
/// thing of the identity that is not the document's own: the namespace a call
/// is qualified with is the package's name, which it is not. Any other backend
/// ignores it.
fn generated_callee(lang: Language, document_name: &str, go_module_prefix: Option<&str>) -> Callee {
    let options = crate::ForgeCompileOptions {
        go_module_prefix: go_module_prefix.map(str::to_owned),
        ..Default::default()
    };
    let symbol = crate::forge::generator::forge_algorithm_symbol(document_name, lang);
    // The identity of a Go document cannot be derived without the root its
    // path is written under, and the call does not need it: the package is
    // named for the document alone.
    if matches!(lang, Language::Go) && go_module_prefix.is_none() {
        let package = filters::to_snake_case(document_name.to_string());
        return Callee {
            call: crate::algorithm_qualified_call(&symbol, &package, &lang),
            import: None,
        };
    }
    let identity =
        crate::forge::generator::forge_import_identity(document_name, &lang, false, &options);
    Callee {
        call: crate::algorithm_qualified_call(&symbol, &identity.namespace, &lang),
        import: Some(identity.include_stmt),
    }
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
    /// How a call of the imported algorithm `document_name` is spelled, and
    /// the line that imports it — or `None` for a target that does not reach
    /// algorithms yet, which refuses a document that calls one rather than
    /// leaving the name undefined.
    fn callee(&self, document_name: &str) -> Option<Callee>;
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
    /// The expression lowerer's target.
    fn expr_target(&self) -> ExprTarget;
    /// The declared name of variable `id`'s field.
    fn field_name(&self, id: &str) -> String;
    /// The name a host reads variable `id` through, when that is not
    /// [`Self::field_name`]: the field's own name for a target whose member is
    /// the host's reader, and the author's spelling for one whose member
    /// carries a prefix. `None` for a name this target cannot spell a reader
    /// with.
    fn reader_name(&self, id: &str) -> Option<String> {
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
    /// ([`Self::enum_type`]), by the enum alias the schema writes.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        schema: &EventSchemaModel,
        enum_types: &std::collections::BTreeMap<String, String>,
    ) -> String;
    /// The identifier of schema field `id` on the record type.
    fn record_field(&self, id: &str) -> String;
    /// A record value built whole from `(field identifier, value)` pairs.
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
    /// The identifier a variant of the enum document `enum_name` has on
    /// [`Self::enum_type`] — the declared name itself for a target whose data
    /// model holds no types, where the variant is that name.
    fn enum_variant(&self, _enum_name: &str, variant: &str) -> String {
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
    fn assign_list(&self, target: &str, value: &str, _elem: &SceType) -> String {
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
    /// does not move it. `None` for a target that leaves a `<foreach>` as the
    /// document wrote it, as the Interpreter's ecmascript does.
    fn foreach_loop(
        &self,
        _list: &str,
        _item: &str,
        _index: Option<&str>,
    ) -> Option<(String, String)> {
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
        _arg_types: &[SceType],
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

/// Kotlin: a variable is a property of the machine class, a record an
/// immutable data class replaced field by field, a list an immutable `List`.
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
        let is_bytes =
            |field: &crate::forge::model::ForgeField| matches!(field.sce_type, SceType::Bytes);
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
                format!("{member} = {member}.copyOf()")
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
    fn name(&self) -> &'static str {
        "Kotlin"
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::Kotlin, document_name, None))
    }
    fn expr_target(&self) -> ExprTarget {
        ExprTarget::Kotlin
    }
    fn field_name(&self, id: &str) -> String {
        filters::to_camel_case(id.to_string())
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
                let field_ty = match &field.sce_type {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::kotlin_type(other).to_string(),
                };
                format!("val {}: {field_ty}", self.record_field(&field.id))
            })
            .collect();
        // Its saved form (`com.sce.runtime.SavedValues`) lives on the type: an
        // object of the schema's fields keyed by their ids. Fixed member names
        // rather than per-type functions on the machine, whose name pattern
        // would reserve every variable name it could match
        // (`crate::reader_names`).
        let writes: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let member = self.record_field(&field.id);
                match &field.sce_type {
                    // An enum is saved by its own type, as a variable of it is.
                    SceType::Enum(_) => format!("\"{}\" to {member}.toSaved()", field.id),
                    _ => format!("\"{}\" to SavedValues.of({member})", field.id),
                }
            })
            .collect();
        let reads: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                let name = self.record_field(&field.id);
                let id = &field.id;
                let saved = format!("SavedValues.field(value, what, \"{id}\")");
                match (&field.sce_type, field.max_size) {
                    (SceType::Enum(reference), _) => format!(
                        "{name} = {}.fromSaved({saved}, \"$what.{id}\")",
                        enum_types[&reference.alias]
                    ),
                    // A string is read back only if it fits the bound the
                    // schema declares, as a string variable's is.
                    (SceType::String, Some(bound)) => {
                        format!("{name} = SavedValues.string({saved}, \"$what.{id}\", {bound})")
                    }
                    // ... and so is a byte string, in bytes.
                    (SceType::Bytes, Some(bound)) => {
                        format!("{name} = SavedValues.bytes({saved}, \"$what.{id}\", {bound})")
                    }
                    (other, _) => format!(
                        "{name} = SavedValues.{}({saved}, \"$what.{id}\")",
                        other.as_attr()
                    ),
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
            bytes_members = self.record_bytes_members(ty, schema)
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
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
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
                    self.enum_variant(&model.name, &v.name),
                    filters::escape_kotlin(v.name.clone())
                )
            })
            .collect();
        format!(
            "/** SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value. */\n\
             enum class {ty}(val declaredName: String) {{\n\
             \x20   {variants};\n\n\
             \x20   /** This value as a saved state writes it. */\n\
             \x20   fun toSaved(): Any = declaredName\n\n\
             \x20   companion object {{\n\
             \x20       /** The value a saved state holds, refused unless it is one. */\n\
             \x20       fun fromSaved(value: Any?, what: String): {ty} {{\n\
             \x20           val declared = SavedValues.string(value, what)\n\
             \x20           return entries.firstOrNull {{ it.declaredName == declared }}\n\
             \x20               ?: throw StateRefusal(\"'$what' ($declared) is not a variant of {alias}\")\n\
             \x20       }}\n\
             \x20   }}\n\
             }}",
            variants = variants.join(",\n    ")
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
    // and a `Float` to a `Double`.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool | InferredType::Str => value.to_string(),
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
}

impl StaticTarget for RustTarget {
    fn name(&self) -> &'static str {
        "Rust"
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::Rust, document_name, None))
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
                let field_ty = match &field.sce_type {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::rust_type(other).to_string(),
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
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
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
    // A closed set of unit variants, so `Copy` and `Eq` by the policy every
    // repr-tagged enum takes. Its saved form is written beside the machine's
    // own ([`StaticLowering::enums`]), where the saved-state runtime exists.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: String = model
            .variants
            .iter()
            .map(|v| format!("    {},\n", self.enum_variant(&model.name, &v.name)))
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
                    self.enum_variant(&model.name, &v.name),
                    filters::escape_rust(v.name.clone())
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n{}\npub enum {ty} {{\n{variants}}}\n\n\
             impl {ty} {{\n    \
             /// The name the enum document declares for this value.\n    \
             #[allow(dead_code)]\n    \
             pub fn sce_name(self) -> &'static str {{\n        \
             match self {{\n{names}        }}\n    }}\n}}",
            crate::rust_derive_policy::RustDeriveCategory::ForgeEnum.derives_attr()
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
    // value: a record has no `Display`, and one spelling serves every type.
    fn log(&self, label: &str, value: &str, _ty: InferredType) -> String {
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
                let position = format!("sce_position_of_{index}");
                (
                    format!("for ({position}, {item}) in {list}.clone().into_iter().enumerate()"),
                    format!("let {index} = {position} as u32; let _ = (&{item}, &{index});"),
                )
            }
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
    // `f64`, both exactly.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!("::sce_rust_runtime::ScriptValue::Bool({value})"),
            InferredType::Str => format!("::sce_rust_runtime::ScriptValue::String({value})"),
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
        let schema = v
            .value_type
            .as_ref()
            .and_then(crate::forge::model::AlgorithmValueType::record_alias)
            .and_then(|alias| records.get(alias));
        schema.into_iter().flat_map(move |schema| {
            schema.fields.iter().filter_map(move |field| {
                kind(&field.sce_type).then_some((format!("{}.{}", v.id, field.id), field.max_size?))
            })
        })
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
        // the buffer that holds it: `last.label` is the buffer's text.
        .chain(scope.record_string_fields().filter_map(|(var, field)| {
            let record = target.field_ref(&target.field_name(var));
            target
                .record_string_read(&record, field)
                .map(|read| (format!("{var}.{field}"), read))
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
                .callee(&c.document_name)
                .map(|callee| (c.alias.clone(), callee.call))
        }))
        .collect()
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
    let variables = &scope.variables;
    let schemas = model.imported_event_schemas.clone();
    let records = model.imported_records.clone();
    // The enums the document imports, each as the type the target declares it
    // as. A target with none spells the alias, which a reference to a variant
    // then refuses by name ([`crate::forge::expr`]).
    let imported_enums = model.imported_enums.clone();
    let enums = StaticEnum::from_imports(&imported_enums, |alias, _| {
        target
            .enum_type(machine, alias)
            .unwrap_or_else(|| alias.to_string())
    });
    // A call the target cannot spell would be an undefined name where the
    // machine runs; it is refused here, where the document is read.
    for callee in &scope.callees {
        match target.callee(&callee.document_name) {
            None => {
                return Err(GenerateError::unsupported(format!(
                    "`{}(…)`: an imported algorithm has no {lang} lowering yet",
                    callee.alias
                )))
            }
            Some(Callee { import: None, .. }) => {
                return Err(GenerateError::unsupported(format!(
                    "`{}(…)`: the import of an algorithm cannot be written for {lang} \
                     without the module its packages live under",
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
    let names = names(&scope, target);
    let imports: Vec<String> = scope
        .callees
        .iter()
        .filter_map(|c| {
            target
                .callee(&c.document_name)
                .and_then(|callee| callee.import)
        })
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
        machine,
        raises_error: model.events.contains("error.execution"),
        target,
        sites: Default::default(),
        elements: Default::default(),
        next_if_ordinal: std::cell::Cell::new(max_if_ordinal(model) + 1),
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
    let mut declarations = TypeDeclarations::new(target, machine, &imported_enums);
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
                    let value = initial_value(
                        &init.expr,
                        target,
                        &ctx,
                        &renames,
                        InferredType::from_sce_type(&field.sce_type),
                    )
                    .map_err(|r| refused("the field value", &init.expr, r))?;
                    // A string field starts at its literal, which the judge held
                    // to the bound; a target that holds it in a buffer is given
                    // the buffer as that literal fills it.
                    let value = match (&field.sce_type, field.max_size) {
                        (SceType::String, Some(bound)) => {
                            target.record_string_literal(bound, &value)
                        }
                        (SceType::Bytes, Some(bound)) => target.record_bytes_literal(bound, &value),
                        _ => value,
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
                    reader: None,
                    holds_bytes: holds_bytes(schema),
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
                    reader: None,
                    holds_bytes: holds,
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
                    reader: None,
                    holds_bytes: false,
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
                reader: None,
                holds_bytes: false,
            });
        }
    }
    // The name a host reads each published field through, asked once the
    // fields are declared: a target whose member is not the reader says so.
    for field in &mut fields {
        field.reader = target.reader_name(&field.id);
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
                }
                crate::model::Invoke::Hybrid(info) => {
                    lower_hybrid_invoke(info, &plain_ctx, &plain_renames, &rewrites)?;
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
            let payload_reads = payload_enum_reads(target, schema, &accessor, &imported_enums);
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
                let cond = transpile_into_receiving(
                    &transition.cond,
                    target.expr_target(),
                    &ctx,
                    &renames,
                    InferredType::Bool,
                )
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
            if lower_actions(&mut transition.actions, &ctx, &renames, &rewrites)?
                && schema.is_some()
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
    // own type for the enum, so the document imports the enum under the alias
    // the schema writes, as it does for a record's field; a target that has not
    // written that spelling yet is refused here, by name, because a generator
    // that reached the field would stop on it. Under any other data model the
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
            schema.fields.iter().filter_map(|f| match &f.sce_type {
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
                     the enum `{enum_alias}`: this document does not import it under that alias \
                     (<sce:import kind=\"enum\" as=\"{enum_alias}\">) or {lang} has no enum type \
                     for it",
                    field.id
                ))
            })?;
        }
    }
    // A machine that takes a list an algorithm returns holds, in the code that
    // calls it, the type the algorithm returns it in, which a target may have to
    // be told it may use.
    let file_annotations = if scope.callees.iter().any(|c| c.list_return.is_some()) {
        target.file_annotation_for_list_call().into_iter().collect()
    } else {
        Vec::new()
    };
    Ok(StaticLowering {
        fields,
        payload_events,
        type_defs: declarations.type_defs,
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
    imported_enums: &'t std::collections::BTreeMap<String, EnumModel>,
    declared: BTreeSet<String>,
    type_defs: Vec<String>,
    records: Vec<StaticRecord>,
    enums: Vec<StaticEnumType>,
}

impl<'t> TypeDeclarations<'t> {
    fn new(
        target: &'t dyn StaticTarget,
        machine: &'t str,
        imported_enums: &'t std::collections::BTreeMap<String, EnumModel>,
    ) -> Self {
        Self {
            target,
            machine,
            imported_enums,
            declared: BTreeSet::new(),
            type_defs: Vec::new(),
            records: Vec::new(),
            enums: Vec::new(),
        }
    }

    /// The type a value of the enum imported as `alias` is held in, declared
    /// the first time it is asked for. `Err` carries what the target cannot do:
    /// it has no enum type, or the document imports no such enum.
    fn enum_type(&mut self, alias: &str) -> Result<String, String> {
        let (Some(ty), Some(model)) = (
            self.target.enum_type(self.machine, alias),
            self.imported_enums.get(alias),
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
                        ident: self.target.enum_variant(&model.name, &v.name),
                    })
                    .collect(),
            });
        }
        Ok(ty)
    }

    /// The type a `record:<alias>` value is held in — of a record variable or
    /// of a list's elements.
    ///
    /// A field of an enum is held in the machine's own type for that enum, so
    /// the statechart imports it under the alias the schema writes — the
    /// convention the typed payload's width check already keeps — and is
    /// refused naming the alias when it does not.
    fn record(&mut self, alias: &str, schema: &EventSchemaModel) -> Result<String, GenerateError> {
        let mut enum_types = std::collections::BTreeMap::new();
        for field in &schema.fields {
            if let SceType::Enum(reference) = &field.sce_type {
                let ty = self.enum_type(&reference.alias).map_err(|enum_alias| {
                    GenerateError::unsupported(format!(
                        "record:{alias} has the enum-typed field `{}`, whose enum `{enum_alias}` \
                         this document does not import under that alias (<sce:import \
                         kind=\"enum\" as=\"{enum_alias}\">) or {} has no enum type for",
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
                let storage = match (&field.sce_type, field.max_size) {
                    (SceType::String, Some(bound)) => self.target.string_storage(bound, "\"\""),
                    (SceType::Bytes, Some(bound)) => self.target.bytes_storage(bound, "{ 0 }"),
                    _ => None,
                };
                if let Some(storage) = storage {
                    self.string_storage(&storage);
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
                        saved_type: match &f.sce_type {
                            SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                            other => other.as_attr(),
                        },
                        bound: matches!(f.sce_type, SceType::String | SceType::Bytes)
                            .then_some(f.max_size)
                            .flatten(),
                    })
                    .collect(),
            });
        }
        Ok(ty)
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
/// An `<invoke type="scxml">` is named by the saved state too — which child a
/// restore starts again — so each is part of the shape with its id and the
/// state that holds it, a hybrid one under a line of its own, and so is each
/// `<invoke>` a declared host invoker serves, with the type as well. A document
/// without any hashes exactly what it did before invocations were saved.
///
/// `None` for a machine whose state lives partly in a session other than this
/// one that a restore cannot start again — an `<invoke>` of any other kind
/// ([`invokes_what_a_restore_cannot_start`]), or a delayed `<send>` waiting to
/// be delivered to another session ([`delays_a_send_to_another_session`]) —
/// which this version of the saved state cannot hold. Such a machine is
/// generated without the save API rather than with one that would silently drop
/// part of its state. A delayed `<send>` to this session or to a host-served
/// processor is not such a send: a saved state holds it as the moment it comes
/// due.
fn saved_shape(model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    if invokes_what_a_restore_cannot_start(model) || delays_a_send_to_another_session(model) {
        return None;
    }
    let mut text = String::from("sce-saved-state-shape 1\n");
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
        let _ = writeln!(
            text,
            "state {} {kind} {}",
            state.id,
            state.parent.as_deref().unwrap_or("-")
        );
    }
    // By id: the map is ordered, so the same document hashes the same.
    for (id, history) in &model.history_states {
        let kind = if history.history_type == "deep" {
            "deep"
        } else {
            "shallow"
        };
        let _ = writeln!(text, "history {id} {kind} {}", history.parent);
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
                    let _ = writeln!(text, "invoke {} {}", invoke.base().invoke_id, state.id);
                }
                // Named apart from a child session of the same id: the two are
                // started differently (a hybrid one by the stem its `srcexpr`
                // computes), so a saved state of one is not the other's.
                crate::model::Invoke::Hybrid(_) => {
                    let _ = writeln!(
                        text,
                        "hybridinvoke {} {}",
                        invoke.base().invoke_id,
                        state.id
                    );
                }
                // A host-run one is named with the type the host serves, which
                // is part of what a restore starts it with: the same id under
                // another type is another invocation. One no host serves is
                // refused when it starts, so a saved state names nothing of it.
                crate::model::Invoke::Unsupported(info) if info.host_served => {
                    let _ = writeln!(
                        text,
                        "hostinvoke {} {} {}",
                        info.invoke_type, info.base.invoke_id, state.id
                    );
                }
                _ => {}
            }
        }
    }
    for var in &scope.variables {
        let ty = var
            .value_type
            .as_ref()
            .map_or_else(String::new, |t| t.as_attr());
        let bound = var
            .capacity
            .map_or_else(|| "-".to_string(), |c| c.to_string());
        let _ = writeln!(text, "variable {} {ty} {bound}", var.id);
        // A record's fields, whether it is the variable or the element of the
        // list the variable is: what a saved state names of it.
        let record_alias = var.value_type.as_ref().and_then(|t| {
            t.record_alias()
                .or_else(|| t.list_elem().and_then(|e| e.record_alias()))
        });
        if let Some(schema) = record_alias.and_then(|alias| model.imported_records.get(alias)) {
            for field in &schema.fields {
                let _ = writeln!(
                    text,
                    "field {}.{} {}",
                    var.id,
                    field.id,
                    field.sce_type.as_attr()
                );
                if let SceType::Enum(reference) = &field.sce_type {
                    write_variants(
                        &mut text,
                        model,
                        &reference.alias,
                        &format!("{}.{}", var.id, field.id),
                    );
                }
            }
        }
        if let Some(SceType::Enum(reference)) = var.value_type.as_ref().and_then(|t| t.scalar()) {
            write_variants(&mut text, model, &reference.alias, &var.id);
        }
    }
    Some(format!("{:x}", Sha256::digest(text.as_bytes())))
}

/// The variants of the enum imported as `alias`, one line each under `holder` —
/// a variable, or a record's field. An enum's variants are its type: one
/// renamed or removed leaves a saved value no variant of it, so the shape
/// refuses before a value is read. By name and sorted, so reordering them
/// leaves a saved state restorable — it holds the declared name, not a
/// position.
fn write_variants(text: &mut String, model: &SCXMLModel, alias: &str, holder: &str) {
    use std::fmt::Write as _;
    if let Some(enum_model) = model.imported_enums.get(alias) {
        let mut names: Vec<&str> = enum_model
            .variants
            .iter()
            .map(|v| v.name.as_str())
            .collect();
        names.sort_unstable();
        for name in names {
            let _ = writeln!(text, "variant {holder}.{name}");
        }
    }
}

/// Whether the document makes a delayed `<send>` that, when it comes due, is
/// delivered through a session other than this one (§scxml-6.2.4): its target
/// is a `#_` location other than `#_internal` — the parent, an invocation, a
/// child session.
///
/// A saved state holds a waiting send as an event for this session's own
/// queues or a request a host-served processor performs; a send waiting on
/// another session is neither, and the session it waits for is not part of the
/// state.
///
/// A `delayexpr` counts as a delay: the time it computes is known only when the
/// send runs and may be one that waits, so a machine that has one cannot be
/// told apart from a machine that delays, and the saved state would drop the
/// send it holds.
///
/// A `targetexpr` is a value this document computes, and which sessions it could
/// name is what it declares as `sce:targets` (docs/adr/0005, decision 3): a
/// delayed send whose declared routes include one that is not this session's own
/// queue is a send waiting on another session, as a written target would be.
/// `a_delayed_sends_target_is_a_literal_under_sce_static` holds the written
/// form, and `a_delayed_send_to_a_computed_target_waits_on_another_session` the
/// declared one.
fn delays_a_send_to_another_session(model: &SCXMLModel) -> bool {
    model.sends().into_iter().any(|(_, send)| {
        let another_session = |route: &str| route.starts_with("#_") && route != "#_internal";
        (!send.delay.is_empty() || !send.delayexpr.is_empty())
            && (another_session(&send.target) || send.targets.iter().any(|t| another_session(t)))
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
///   with the deadline it had left.
///
/// One no host serves is refused when it starts (`error.execution`), so nothing
/// is running and a saved state has nothing to name.
///
/// The rest do not reduce: a mesh-rpc call has one request in flight that
/// cannot be sent twice, and a mesh peer is a session this machine does not
/// own. A saved state that left one out would restore a machine waiting on
/// something nobody is doing.
///
/// A mesh-rpc call reaches this point already lowered to a host-served invoke
/// of the type SCE reserves for it (`lower_mesh_invokes`), so it is told from
/// a host's own by that type: a host may not declare one under the reserved
/// prefix, so a host-served invoke that has one is SCE's own. The request such
/// a call was started with may already have reached its peer, and starting it
/// again would have the peer act on it twice.
fn invokes_what_a_restore_cannot_start(model: &SCXMLModel) -> bool {
    model
        .states
        .values()
        .flat_map(|s| &s.invokes)
        .any(|invoke| match invoke {
            crate::model::Invoke::Scxml(info) => info.remote_mesh_target.is_some(),
            crate::model::Invoke::Unsupported(info) => {
                info.host_served
                    && crate::host_processor_analyzer::is_reserved_type(&info.invoke_type)
            }
            crate::model::Invoke::Hybrid(_) => false,
            crate::model::Invoke::MeshRpc(_) => true,
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
    machine: &'m str,
    raises_error: bool,
    target: &'m dyn StaticTarget,
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
/// Lowers scalar variables, guards, `<assign>`, `<if>`, `<log>`, `<raise>`
/// and `In()`; every construct past those is refused by name
/// ([`StaticTarget::unsupported`]) until its spelling is written, rather than
/// left as an undefined name in generated code.
pub struct CppTarget;

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

impl StaticTarget for CppTarget {
    fn name(&self) -> &'static str {
        "C++"
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::Cpp, document_name, None))
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
    // language reserves.
    fn reader_name(&self, id: &str) -> Option<String> {
        let spelled = id.replace(['.', '-'], "_");
        (!crate::reader_names::is_reserved_word(Language::Cpp, &spelled)).then_some(spelled)
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
    // schema's order, which a value is built from by designated initializers.
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
                let field_ty = match &field.sce_type {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::cpp_type(other).to_string(),
                };
                format!("    {field_ty} {};\n", self.record_field(&field.id))
            })
            .collect();
        format!("/// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\nstruct {ty} {{\n{fields}}};")
    }
    // The schema's id spelled as this file's payload structs spell their
    // fields: one file, one spelling of a schema field.
    fn record_field(&self, id: &str) -> String {
        crate::forge::generator::event_schema_field_ident(id, Language::Cpp)
    }
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String {
        let args: Vec<String> = fields.iter().map(|(f, v)| format!(".{f} = {v}")).collect();
        format!("{ty}{{{}}}", args.join(", "))
    }
    fn enum_type(&self, machine: &str, alias: &str) -> Option<String> {
        Some(format!(
            "{machine}{}Enum",
            filters::to_pascal_case(alias.to_string())
        ))
    }
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
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
                    self.enum_variant(&model.name, &v.name),
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
                    self.enum_variant(&model.name, &v.name),
                    filters::escape_cpp(v.name.clone())
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n\
             enum class {ty} : {} {{\n{variants}}};\n\n\
             /// The name the enum document declares for `value`.\n\
             inline const char *sceLogName({ty} value) {{\n    switch (value) {{\n{names}    }}\n    return \"\";\n}}",
            crate::forge::generator::cpp_type(&model.underlying_type)
        )
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
        _arg_types: &[SceType],
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
    // integer widens to `int64_t` and a real to `double`, both exactly.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool => format!("ScriptValue(std::in_place_type<bool>, {value})"),
            InferredType::Str => {
                format!("ScriptValue(std::in_place_type<std::string>, {value})")
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
        .any(|f| matches!(f.sce_type, SceType::Bytes))
}

/// Whether a record of `schema` owns a buffer — has a `string` or a `bytes` field —
/// which a target that holds plain data by value must hold otherwise.
fn owns_a_buffer(schema: &EventSchemaModel) -> bool {
    schema
        .fields
        .iter()
        .any(|f| matches!(f.sce_type, SceType::String | SceType::Bytes))
}

/// Rewrite `model` — a clone the C++ backend renders — so every expression of
/// a `sce-static` document is native C++.
pub fn lower_cpp(model: &mut SCXMLModel, machine: &str) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &CppTarget)
}

/// Go: a variable is a field of the machine's policy struct, set by the
/// machine's own statements. A failed checked operation is a value (zero, with
/// a flag raised in `sceFailure`) rather than a jump, as in C++, so a value that
/// can fail is computed into a local first and written only when it did not.
///
/// Lowers scalar, enum, list and record variables, guards, `<assign>`, `<if>`,
/// `<foreach>`, `<log>`, `<raise>`, `In()`, a typed payload, a call of an
/// imported algorithm and a `<sce:action>` whose arguments are typed
/// expressions of the machine's variables; every construct past those is
/// refused by name ([`StaticTarget::unsupported`]) until its spelling is
/// written, rather than left as an undefined name in generated code.
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
    fn name(&self) -> &'static str {
        "Go"
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(
            Language::Go,
            document_name,
            self.import_root,
        ))
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
    fn reader_name(&self, id: &str) -> Option<String> {
        let spelled = filters::to_pascal_case(id.to_string());
        (!crate::reader_names::is_reserved_word(Language::Go, &spelled)).then_some(spelled)
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
        let go_field_type = |field: &crate::forge::model::ForgeField| match &field.sce_type {
            SceType::Enum(reference) => enum_types[&reference.alias].clone(),
            other => crate::forge::generator::go_type(other).to_string(),
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
                let answer = if matches!(field.sce_type, SceType::Bytes) {
                    format!("append([]byte{{}}, r.{}...)", self.record_field(&field.id))
                } else {
                    format!("r.{}", self.record_field(&field.id))
                };
                format!(
                    "\n// {reader} reports the `{id}` field.\nfunc (r {ty}) {reader}() {} {{\n\treturn {answer}\n}}\n",
                    go_field_type(field),
                    reader = filters::to_pascal_case(field.id.clone()),
                    id = field.id,
                )
            })
            .collect();
        format!(
            "// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\n\
             type {ty} struct {{\n{fields}}}\n{readers}"
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
    // A constant is a package-level identifier in Go, prefixed by the enum's
    // own name, and the machine is a package of its own.
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::Go, enum_name, variant)
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
                    self.enum_variant(&model.name, &v.name),
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
                    self.enum_variant(&model.name, &v.name),
                    filters::escape_go(v.name.clone())
                )
            })
            .collect();
        format!(
            "// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n\
             type {ty} {}\n\n\
             const (\n{constants})\n\n\
             // String is the name the enum document declares for the value.\n\
             func (v {ty}) String() string {{\n\tswitch v {{\n{names}\t}}\n\treturn \"\"\n}}",
            crate::forge::generator::go_type(&model.underlying_type)
        )
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
        _arg_types: &[SceType],
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
    // to `float64`, both exactly.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
            InferredType::Bool | InferredType::Str => value.to_string(),
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
/// (`--go-module-prefix`). A machine that calls an imported algorithm imports
/// its package by that path, and a Go import path has no valid bare form, so a
/// document that calls one without a usable prefix is refused as any forge kind
/// importing another is.
pub fn lower_go(
    model: &mut SCXMLModel,
    machine: &str,
    go_module_prefix: Option<&str>,
) -> Result<StaticLowering, GenerateError> {
    let calls_an_algorithm = StaticScope::of(model).is_some_and(|scope| !scope.callees.is_empty());
    let import_root = if calls_an_algorithm {
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
/// Lowers scalar, enum, list and record variables, guards, `<assign>`, `<if>`,
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
}

impl StaticTarget for PythonTarget {
    fn name(&self) -> &'static str {
        "Python"
    }
    // A statement that fails raises, which leaves the block: the lines follow one another.
    fn in_sequence(&self, parts: &[String]) -> String {
        parts.join("\n")
    }
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::Python, document_name, None))
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
    fn reader_name(&self, id: &str) -> Option<String> {
        let spelled = id.replace(['.', '-'], "_");
        (!crate::reader_names::is_reserved_word(Language::Python, &spelled)).then_some(spelled)
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
                let field_ty = match &field.sce_type {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::python_type(other).to_string(),
                };
                format!("    {}: {field_ty}\n", self.record_field(&field.id))
            })
            .collect();
        format!(
            "@sce_dataclasses.dataclass(frozen=True)\nclass {ty}:\n    \
             \"\"\"SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\"\"\"\n\n{fields}"
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
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
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
                    self.enum_variant(&model.name, &v.name),
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
        format!(
            "class {ty}(IntEnum):\n    \
             \"\"\"SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\"\"\"\n\n\
             {variants}\n    @property\n    def sce_name(self) -> str:\n        \
             \"\"\"The name the enum document declares for this value.\"\"\"\n        \
             return {{{}}}[int(self)]\n",
            names.join(", ")
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
        _arg_types: &[SceType],
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
    // as a real.
    fn wire_value(&self, ty: InferredType, value: &str) -> String {
        match ty {
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
fn c_string_storage_type(capacity: u32) -> String {
    format!("sce_static_string_{capacity}_t")
}

/// The function that fills the buffer of [`c_string_storage_type`] from a text
/// that fits it.
fn c_string_storage_of(capacity: u32) -> String {
    format!("sce_static_string_{capacity}_of")
}

/// The type a byte string of at most `capacity` bytes is held in: a buffer and
/// the length it holds, named by the bound so that two variables, or a variable
/// and a record's field, of one bound share it ([`CTarget::bytes_storage`]).
fn c_bytes_storage_type(capacity: u32) -> String {
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
    /// The name each imported enum's document declares, by the alias the
    /// machine imports it as. The type and its constants are spelled from the
    /// document's name, not the alias, so that two machines importing one enum —
    /// whatever they call it — share one declaration in a program that includes
    /// both.
    enums: std::collections::BTreeMap<String, String>,
}

/// What lowers the arguments of a host action ([`target_for`]): the one thing a
/// caller outside the walk asks of C, and it asks no condition, no payload
/// guard and no enum, so there is nothing for it to count or to name a machine
/// for.
static C_ARGUMENTS: CTarget = CTarget::arguments();

impl CTarget {
    fn new(stem: &str, imported_enums: &std::collections::BTreeMap<String, EnumModel>) -> Self {
        Self {
            stem: stem.to_string(),
            conditions: std::sync::atomic::AtomicU32::new(0),
            loops: std::sync::atomic::AtomicU32::new(0),
            enums: imported_enums
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
    // A statement that fails is a block of its own that returns from the function: the
    // blocks follow one another.
    fn in_sequence(&self, parts: &[String]) -> String {
        parts.join("\n")
    }
    // An algorithm's C artifact is a header of `static inline` functions, which
    // the machine includes by the line a forge kind importing it writes; a call
    // that can fail is received through the `<name>_take` the header declares
    // (the expression lowerer spells it).
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::C11, document_name, None))
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
        // of the bound the schema declares (a buffer, as a variable's is), and an enum the
        // machine imports under the alias the schema writes. A field is named as
        // the author wrote it; a
        // name C reserves is refused where the schema is read, for every
        // backend at once.
        for var in &scope.variables {
            let Some(alias) = var.value_type.as_ref().and_then(|t| {
                t.record_alias()
                    .or_else(|| t.list_elem().and_then(|e| e.record_alias()))
            }) else {
                continue;
            };
            let Some(schema) = model.imported_records.get(alias) else {
                continue;
            };
            if let Some(field) = schema.fields.iter().find(|f| {
                !held_scalar(&f.sce_type)
                    && !matches!(
                        f.sce_type,
                        SceType::Enum(_) | SceType::Float32 | SceType::Float64
                    )
                    // A string or a byte string with no bound was refused where the
                    // record was declared, so the one that reaches here has its buffer.
                    && !(matches!(f.sce_type, SceType::String | SceType::Bytes)
                        && f.max_size.is_some())
            }) {
                return Some(format!(
                    "record:{alias} with the field `{}` of type {}",
                    field.id,
                    field.sce_type.as_attr()
                ));
            }
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
    fn reader_name(&self, id: &str) -> Option<String> {
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
    fn enum_variant(&self, enum_name: &str, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(Language::C11, enum_name, variant)
    }
    // The declaration is guarded by a macro of the document's name, so that a
    // program including two machines which import the same enum declares it
    // once. The function answers the name the document declares for a value —
    // what a saved or observed value is — and NULL for one no variant names.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let prefix = crate::forge::enum_naming::c11_function_prefix(&model.name);
        let guard = format!("SCE_STATIC_ENUM_{}", prefix.to_uppercase());
        let variants: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    {} = {},\n",
                    self.enum_variant(&model.name, &v.name),
                    v.value
                )
            })
            .collect();
        let names: String = model
            .variants
            .iter()
            .map(|v| {
                format!(
                    "    case {}:\n        return \"{}\";\n",
                    self.enum_variant(&model.name, &v.name),
                    filters::escape_c(v.name.clone())
                )
            })
            .collect();
        format!(
            "#ifndef {guard}\n#define {guard}\n\
             /* SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value. */\n\
             typedef enum {{\n{variants}}} {ty};\n\n\
             /* The name the enum document declares for `value`, or NULL for a value no\n   \
             variant names. */\n\
             static inline const char *{prefix}_declared_name({ty} value) {{\n    \
             switch (value) {{\n{names}    }}\n    return NULL;\n}}\n#endif"
        )
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
                let field_ty = match (&field.sce_type, field.max_size) {
                    (SceType::Enum(reference), _) => enum_types[&reference.alias].clone(),
                    // A string is the buffer of the bound its schema declares, and a
                    // byte string that buffer with the length it holds.
                    (SceType::String, Some(bound)) => c_string_storage_type(bound),
                    (SceType::Bytes, Some(bound)) => c_bytes_storage_type(bound),
                    (other, _) => crate::forge::generator::c_type(other).to_string(),
                };
                format!("    {field_ty} {};\n", self.record_field(&field.id))
            })
            .collect();
        let view = format!("{}_view_t", ty.trim_end_matches("_t"));
        format!(
            "/* SCE Accepted Subset §2.15: a `record:{alias}` datamodel value. */\n\
             typedef struct {{\n{fields}}} {ty};\n\n\
             /* A borrowed view of a list of them: what a host reads, and cannot grow past\n   \
             the bound the machine keeps. */\n\
             typedef struct {{\n    const {ty} *data;\n    size_t len;\n}} {view};"
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
        let guard = ty.to_uppercase().trim_end_matches("_T").to_string();
        // `_of` is the buffer holding a text already held to the bound — a
        // payload's field taken whole into a record — written without a library
        // call, since the header includes none.
        let def = format!(
            "#ifndef {guard}\n#define {guard}\n\
             /* SCE Accepted Subset §2.15: a `string` of at most {capacity} UTF-8 bytes. */\n\
             typedef struct {{\n    char data[{capacity} + 1];\n}} {ty};\n\n\
             static inline {ty} {of}(const char *text) {{\n    \
             {ty} value = {{ {{ 0 }} }};\n    \
             for (size_t i = 0; i < {capacity}u && text[i] != '\\0'; ++i) {{\n        \
             value.data[i] = text[i];\n    }}\n    return value;\n}}\n#endif",
            of = c_string_storage_of(capacity)
        );
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
    fn assign_list(&self, target: &str, value: &str, _elem: &SceType) -> String {
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
        format!(
            "{{ sce_forge_algorithm_failure_t sce_failure_ = {{0}}; {} sce_value = {value}; \
             if (sce_failure_.failed) {{ {failed} return; }} {} }}",
            Self::value_type(ty),
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
        arg_types: &[SceType],
        failed: &str,
    ) -> String {
        let locals: String = args
            .iter()
            .zip(arg_types)
            .enumerate()
            .map(|(i, (arg, ty))| {
                format!(
                    "{} sce_arg{i} = {arg}; ",
                    crate::forge::generator::c_type(ty)
                )
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
    let target = CTarget::new(&stem, &model.imported_enums);
    lower(model, &format!("{csym_prefix}{stem}"), &target)
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
    /// ([`crate::forge::native_action::static_argument_type`]).
    pub ty: SceType,
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
/// lowered expression takes. `None` for an argument validation refused,
/// which never reaches here, and for a backend that does not lower the model.
pub(crate) fn lower_static_argument(
    scope: &StaticScope,
    event: Option<(&str, &EventSchemaModel)>,
    arg: &crate::model::Param,
    lang: Language,
) -> Option<StaticArgument> {
    let target = target_for(lang)?;
    let paths = scope.paths(event.map(|(_, schema)| schema));
    let ctx = scope.ctx(&paths, &[]);
    let names = names(scope, target);
    let accessor = event.map(|(event, _)| target.payload_accessor(event));
    let renames = renames(&names, accessor.as_deref(), target);
    let ty = crate::forge::native_action::static_argument_type(&ctx, arg).ok()?;
    let lowered = transpile_into_receiving(
        &arg.expr,
        target.expr_target(),
        &ctx,
        &renames,
        InferredType::from_sce_type(&ty),
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
    args: &[(String, SceType)],
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
    let (texts, types): (Vec<String>, Vec<SceType>) = args.iter().cloned().unzip();
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
            InferredType::from_sce_type(&field.sce_type),
        )?;
        if read.can_fail {
            return Err(GenerateError::unsupported(format!(
                "`_event.data.{}` can fail, which a record made whole from a payload does \
                 not receive",
                field.id
            )));
        }
        let text = match (&field.sce_type, field.max_size) {
            (SceType::String, Some(bound)) => {
                bounded = true;
                let held = target.bounded_string(&read.text, bound);
                target.record_string_runtime(bound, &held)
            }
            (SceType::Bytes, Some(bound)) => {
                bounded = true;
                let held = target.bounded_bytes(&read.text, bound);
                target.record_bytes_runtime(bound, &held)
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
/// its length ([`StaticTarget::payload_bytes_read`]).
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
            let read = match &field.sce_type {
                SceType::Enum(reference) => {
                    let variants: Vec<&str> = enums
                        .get(&reference.alias)?
                        .variants
                        .iter()
                        .map(|v| v.name.as_str())
                        .collect();
                    target.payload_enum_read(accessor, &field.id, &variants)?
                }
                SceType::Bytes => target.payload_bytes_read(accessor, &field.id)?,
                _ => return None,
            };
            Some((
                format!(
                    "{}.{}",
                    crate::forge::event_schema_check::EVENT_DATA_PATH,
                    field.id
                ),
                read,
            ))
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
fn lower_actions(
    actions: &mut [Action],
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<bool, GenerateError> {
    let mut reads_payload = false;
    for action in actions {
        reads_payload |= lower_action(action, ctx, renames, rewrites)?;
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
        transpile_into_owned(text, target.expr_target(), ctx, renames, slot).map_err(|r| {
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
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            let location = action.location.trim();
            // The type of the record a payload is taken whole into, when it is.
            let whole_record = match rewrites.records.get(location) {
                Some((alias, _)) if action.expr.trim() == "_event.data" => {
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
            let record_field = location
                .split_once('.')
                .filter(|(var, _)| rewrites.records.contains_key(*var));
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
                    let is_string = rewrites.strings.contains_key(location);
                    let is_bytes = rewrites.bytes.contains_key(location);
                    statement(
                        &value,
                        slot,
                        &|v| {
                            if is_string {
                                target.assign_string_field(name, &field, v)
                            } else if is_bytes {
                                target.assign_bytes_field(name, &field, v)
                            } else {
                                target.assign_field(name, &field, v)
                            }
                        },
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
                    let returned_list = rewrites
                        .lists
                        .get(location)
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
                    let write = |v: &str| {
                        if let Some((elem, _, _)) = &returned_list {
                            target.assign_list(name, v, elem)
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
                    let held_list = returned_list.as_ref().and_then(|(_, _, alias)| {
                        target.list_result_type(renames.get(alias.as_str()).copied()?)
                    });
                    match &whole_record {
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
            // The value's own type, which a target that formats by it reads.
            let ty = crate::forge::expr::judge_into(
                &action.expr,
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
            // A loop variable the generated machine already has a field of
            // that name would hide it in a backend that reaches fields by
            // their bare names.
            if let Some(clash) = [Some(item), index]
                .into_iter()
                .flatten()
                .find(|name| renames.values().any(|field| field == name))
            {
                return Err(GenerateError::unsupported(format!(
                    "<foreach> names a loop variable `{clash}` that is the name {lang} gives a \
                     field of the machine"
                )));
            }
            let loop_variables =
                crate::forge::type_ctx::LoopVariables::new(item, index, elem, &rewrites.schemas);
            let inner = loop_variables.bind(ctx);
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
            if let Some((head, prologue)) =
                target.foreach_loop_typed(name, item, index, &item_ty, &list_ty)
            {
                action.native_loop = head;
                action.native_loop_prologue = prologue;
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
                        .record_string_read(item, field)
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
    let ty = crate::forge::expr::judge_into(
        written,
        ctx,
        crate::forge::expr::Expected::Hint(InferredType::Unknown),
    )
    .map_err(|r| refused(r.error.to_string()))?;
    // An enum value crosses as the name its enum declares, a string: the value
    // is lowered as the enum value it is, and the backend names the variant it
    // holds. The judge accepted only a variable or a field of a record variable
    // as one, which is what `enum_vars` holds.
    let held_by_a_variable = |name: &str| rewrites.enum_of_wire_value(name);
    if let Some(alias) = crate::forge::static_enum::value_enum(written, ctx, &held_by_a_variable)
        .map_err(|r| refused(r.error.to_string()))?
    {
        let value = transpile_into_owned(
            written,
            target.expr_target(),
            ctx,
            renames,
            InferredType::Unknown,
        )
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
    let value = transpile_into_owned(written, target.expr_target(), ctx, renames, slot)
        .map_err(|r| refused(r.error.to_string()))?;
    rewrites.note(written, param.spelling, &value.text);
    let wired = if bytes {
        target
            .wire_bytes(&value.text)
            .ok_or_else(|| refused("a byte string has no wire spelling here yet".to_string()))?
    } else {
        target.wire_value(slot, &value.text)
    };
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
        let value = transpile_into_owned(
            written,
            target.expr_target(),
            ctx,
            renames,
            InferredType::Str,
        )
        .map_err(|r| {
            GenerateError::unsupported(format!("`{written}` has no {lang} lowering: {}", r.error))
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
        let value = transpile_into_owned(
            srcexpr,
            target.expr_target(),
            ctx,
            renames,
            InferredType::Str,
        )
        .map_err(|r| {
            GenerateError::unsupported(format!("`{srcexpr}` has no {lang} lowering: {}", r.error))
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
/// the variable it lands in, and whether it can fail.
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
    let value = transpile_into_owned(&written, target.expr_target(), ctx, renames, slot)
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
    let value = transpile_into_owned(
        &info.srcexpr,
        target.expr_target(),
        ctx,
        renames,
        InferredType::Str,
    )
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
    // Every condition of the chain sets the flag of the `<if>` it belongs to.
    let flag = target.condition_failed_flag(action.if_ordinal);
    for branch in action.branch_conditions_mut() {
        if branch.is_cpp_condition || branch.is_kt_condition || branch.cond.trim().is_empty() {
            continue;
        }
        reads_payload |= crate::forge::expr::references_event_data_lexically(&branch.cond);
        let cond = transpile_into_receiving(
            &branch.cond,
            target.expr_target(),
            ctx,
            renames,
            InferredType::Bool,
        )
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
    for block in action.nested_blocks_mut() {
        reads_payload |= lower_actions(block, ctx, renames, rewrites)?;
    }
    Ok(reads_payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

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
        lower_rust(&mut model, "M").expect("lowers").saved_shape
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
    fn a_delayed_send_to_another_session_has_no_saved_shape() {
        // It is delivered through a session the saved state does not carry:
        // the parent, an invocation, a child by its session id.
        for target in ["#_parent", "#_child", "#_scxml_session"] {
            let send = format!(r#"<send event="later" target="{target}" delay="5s"/>"#);
            assert_eq!(shape(&counter_sending(&send)), None, "{send}");
        }
    }

    /// `COUNTER` whose `counting` state holds `invoke`.
    fn counter_invoking(invoke: &str) -> String {
        COUNTER.replace(
            r#"<state id="counting">"#,
            &format!(r#"<state id="counting">{invoke}"#),
        )
    }

    /// A static child session of `id`, which ends as it starts.
    fn child_session(id: &str) -> String {
        format!(
            r#"<invoke type="scxml" id="{id}"><content>
          <scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="f"><final id="f"/></scxml>
        </content></invoke>"#
        )
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
        lower_rust(&mut model, "M").expect("lowers").saved_shape
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
        // that is not this session's own queue makes a delayed send a send
        // waiting on another session, as a written target of it would.
        for routes in ["#_parent", "#_internal #_parent", "#_scxml_peer #_internal"] {
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
    fn a_delayed_send_to_computed_targets_of_this_session_waits_on_no_other() {
        let send =
            r##"<send event="later" targetexpr="route" sce:targets="#_internal" delay="5s"/>"##;
        let model = SCXMLParser::new()
            .parse_string(&counter_routing(send), "m")
            .expect("parses");
        assert!(!super::delays_a_send_to_another_session(&model));
    }

    #[test]
    fn a_computed_delay_to_another_session_has_no_saved_shape() {
        // The time is known only when the send runs, so it may be one that
        // waits: the machine is told apart from one that delays by nothing.
        for target in ["#_parent", "#_child", "#_scxml_session"] {
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
        let history = r##"<history id="h"><transition target="counting"><send event="later" target="#_parent" delay="5s"/></transition></history>"##;
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
            other => panic!("{other:?} does not lower sce-static"),
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
            other => panic!("{other:?} does not lower sce-static"),
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
            other => panic!("{other:?} is not lowered by this test"),
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
        assert!(statements[0].contains("tenth + 0.2f"), "{statements:?}");
        assert!(
            statements[1].contains("tenth.toDouble() + 0.2") && !statements[1].contains("0.2f"),
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
}
