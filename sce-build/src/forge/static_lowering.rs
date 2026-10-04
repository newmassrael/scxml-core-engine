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
    /// `target = value`.
    fn assign(&self, target: &str, value: &str) -> String;
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
}

/// Kotlin: a variable is a property of the machine class, a record an
/// immutable data class replaced field by field, a list an immutable `List`.
pub struct KotlinTarget;

impl KotlinTarget {
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
                match &field.sce_type {
                    SceType::Enum(reference) => format!(
                        "{name} = {}.fromSaved({saved}, \"$what.{id}\")",
                        enum_types[&reference.alias]
                    ),
                    other => format!(
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
             \x20   fun toSaved(): Any = linkedMapOf({writes})\n\n\
             \x20   companion object {{\n\
             \x20       /** The value a saved state holds, refused unless it is one. */\n\
             \x20       fun fromSaved(value: Any?, what: String): {ty} = {ty}({reads})\n\
             \x20   }}\n\
             }}",
            params = params.join(", "),
            writes = writes.join(", "),
            reads = reads.join(", ")
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
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
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
    // event-schema payload takes from the one policy that decides it.
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
                    other => crate::forge::generator::rust_type(other).to_string(),
                };
                format!("    pub {}: {field_ty},\n", self.record_field(&field.id))
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` datamodel value.\n{}\n#[allow(non_snake_case)]\npub struct {ty} {{\n{fields}}}",
            crate::rust_derive_policy::RustDeriveCategory::EventSchemaPlainPayload.derives_attr()
        )
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
    // A closed set of unit variants, so `Copy` and `Eq` by the policy every
    // repr-tagged enum takes. Its saved form is written beside the machine's
    // own ([`StaticLowering::enums`]), where the saved-state runtime exists.
    fn enum_def(&self, ty: &str, alias: &str, model: &EnumModel) -> String {
        let variants: String = model
            .variants
            .iter()
            .map(|v| format!("    {},\n", self.enum_variant(&model.name, &v.name)))
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: an `enum:{alias}` datamodel value.\n{}\npub enum {ty} {{\n{variants}}}",
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
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value};")
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
        .map(|v| (v.id.clone(), target.field_ref(&target.field_name(&v.id))))
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
            Some((v.id.clone(), records.get(alias)?.clone()))
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
    // held to.
    let string_vars: StringVars = variables
        .iter()
        .filter(|v| {
            matches!(
                v.value_type
                    .as_ref()
                    .and_then(crate::forge::model::AlgorithmValueType::scalar),
                Some(SceType::String)
            )
        })
        .filter_map(|v| Some((v.id.clone(), v.capacity?)))
        .collect();
    let rewrites = Rewrites {
        records: record_vars,
        schemas: records.clone(),
        lists: list_vars,
        strings: string_vars,
        machine,
        raises_error: model.events.contains("error.execution"),
        target,
        sites: Default::default(),
        elements: Default::default(),
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
                    values.push((target.record_field(&field.id), value));
                }
                let init = target.record_value(&ty, &values);
                if let Some(element) = target.data_element(&var.id, &init) {
                    rewrites.note_element(var.value_type_spelling.as_ref(), &element);
                }
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    init,
                    saved_type: ty.clone(),
                    ty,
                    published,
                    view: None,
                    bound: None,
                    saved_kind: "record",
                    reader: None,
                });
                continue;
            }
            // A list starts empty. Its elements are numbers and bools, or
            // records, which are declared as a record variable's are.
            if let Some(elem) = var.value_type.as_ref().and_then(|t| t.list_elem()) {
                use crate::forge::model::ListElemType;
                let (ty, view, saved_kind, saved_type, empty) = match elem {
                    ListElemType::Scalar(elem) => {
                        let list = declarations.scalar_list(elem, var.capacity);
                        (
                            list.ty,
                            target.list_view(elem),
                            "list",
                            elem.as_attr(),
                            list.empty,
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
                    bound: var.capacity,
                    saved_kind,
                    saved_type,
                    reader: None,
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
                    bound: None,
                    saved_kind: "enum",
                    reader: None,
                });
                continue;
            }
            let slot = InferredType::from_sce_type(ty);
            let init = initial_value(&var.expr, target, &ctx, &renames, slot)
                .map_err(|r| refused("the initial value", &var.expr, r))?;
            rewrites.note(&var.expr, var.expr_spelling.as_ref(), &init);
            fields.push(StaticField {
                id: var.id.clone(),
                name,
                ty: target.scalar_type(ty),
                init,
                published,
                view: target.scalar_view(ty),
                bound: matches!(ty, SceType::Bytes | SceType::String)
                    .then_some(var.capacity)
                    .flatten(),
                saved_kind: "scalar",
                saved_type: ty.as_attr(),
                reader: None,
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
        // the state's entry, where no event's payload is in scope.
        for invoke in &mut state.invokes {
            match invoke {
                crate::model::Invoke::Unsupported(info) => {
                    for param in &mut info.base.params {
                        lower_wire_param(param, &plain_ctx, &plain_renames, &rewrites)?;
                    }
                }
                crate::model::Invoke::Scxml(info) => {
                    lower_child_arguments(info, &plain_ctx, &plain_renames, &rewrites)?;
                }
                _ => {}
            }
        }
        // What a `<final>` hands the event it raises is read from the
        // machine's fields when the state is entered (§scxml-5.5).
        if let Some(done) = &mut state.donedata {
            lower_done_params(&mut done.params, &plain_ctx, &plain_renames, &rewrites)?;
        }
        for transition in &mut state.transitions {
            let schema = schemas.get(&transition.event);
            let paths = scope.paths(schema);
            let ctx = scope.ctx(&paths, &enums);
            let accessor = target.payload_accessor(&transition.event);
            let renames = renames(&names, schema.map(|_| accessor.as_str()), target);
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
    }
    for script in &mut model.global_scripts {
        let ctx = scope.ctx(&no_payload, &enums);
        lower_action(script, &ctx, &renames(&names, None, target), &rewrites)?;
    }
    Ok(StaticLowering {
        fields,
        payload_events,
        type_defs: declarations.type_defs,
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
                    })
                    .collect(),
            });
        }
        Ok(ty)
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
/// state that holds it, and so is each `<invoke>` a declared host invoker
/// serves, with the type as well. A document without either hashes exactly what
/// it did before invocations were saved.
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
/// The delay and the target are read as written because this data model
/// refuses the attributes that would leave either to run time (`delayexpr`,
/// `targetexpr`: `UNTYPED_ACTION_ATTRIBUTES` in `static_datamodel`), so a
/// send's target is never a value this document computes.
/// `a_delayed_sends_target_and_delay_are_literals_under_sce_static` holds that:
/// a data model that typed `targetexpr` would have to decide here which
/// sessions it could name.
fn delays_a_send_to_another_session(model: &SCXMLModel) -> bool {
    model.sends().into_iter().any(|(_, send)| {
        !send.delay.is_empty() && send.target.starts_with("#_") && send.target != "#_internal"
    })
}

/// Whether the document holds an `<invoke>` a restore cannot start again: a
/// mesh call, a peer on another device, or one whose child is chosen by an
/// expression at run time.
///
/// A saved state names an invocation and a restore starts it again
/// (§scxml-6.4). Two kinds reduce to that:
///
/// - a child session (`type="scxml"`) starts from the beginning, and what it
///   was started with is the document — it has no parameters and no
///   `<finalize>`, both refused in `static_datamodel`;
/// - a host-run invocation (`<invoke type>` a declared invoker serves) starts
///   again from the request it was started with, which the saved state holds,
///   with the deadline it had left.
///
/// One no host serves is refused when it starts (`error.execution`), so nothing
/// is running and a saved state has nothing to name.
///
/// The rest do not reduce: a mesh-rpc call has one request in flight that
/// cannot be sent twice, a mesh peer is a session this machine does not own,
/// and a hybrid invoke chooses its child by an expression this data model
/// refuses. A saved state that left one out would restore a machine waiting on
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
            crate::model::Invoke::Hybrid(_) | crate::model::Invoke::MeshRpc(_) => true,
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

/// A `sce-static` document's record variables, each with the schema its
/// alias names.
type RecordVars = std::collections::BTreeMap<String, EventSchemaModel>;

/// A `sce-static` document's list variables, each with its element type and
/// its declared capacity.
type ListVars = std::collections::BTreeMap<String, (crate::forge::model::ListElemType, u32)>;

/// A `sce-static` document's string variables, each with its declared capacity
/// in UTF-8 bytes.
type StringVars = std::collections::BTreeMap<String, u32>;

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
    lists: ListVars,
    strings: StringVars,
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
}

impl Rewrites<'_> {
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
                // event's JSON, and a literal `<content>` is the text it spells.
                // A BasicHTTP send also needs each value as the text a form
                // carries, which the machine does not spell yet.
                "send"
                    if !action.params.is_empty()
                        && action.send_type
                            == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor" =>
                {
                    return Some("a BasicHTTP <send> carrying a <param>".to_string())
                }
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
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        // Every type a datamodel holds is spelled but bytes: a list admits only
        // numbers, bools and records (`AlgorithmValueType::list_elem_admitted`).
        if let Some(var) = scope.variables.iter().find(|v| {
            !v.value_type
                .as_ref()
                .is_some_and(|t| !matches!(t.scalar(), Some(SceType::Bytes)))
        }) {
            return Some(format!("<data id=\"{}\"> of a bytes type", var.id));
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
            if let Some(other) = unlowered_invoke(&state.invokes) {
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
    // A string is lent to the host, not copied out of the machine.
    fn scalar_view(&self, ty: &SceType) -> Option<String> {
        match ty {
            SceType::String => Some("const std::string&".to_string()),
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
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
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
}

/// The first of `invokes` a target that lowers a `<invoke type="scxml">` has no
/// lowering for yet, described for a refusal. A scxml child is started by the
/// machine's own invoke code and handed its values by the build; one the host
/// runs, a hybrid one and a mesh one are not lowered yet.
fn unlowered_invoke(invokes: &[crate::model::Invoke]) -> Option<String> {
    invokes
        .iter()
        .find(|i| !matches!(i, crate::model::Invoke::Scxml(_)))
        .map(|other| {
            match other {
                crate::model::Invoke::Hybrid(_) => "a hybrid <invoke>",
                crate::model::Invoke::MeshRpc(_) => "a mesh <invoke>",
                _ => "a host-run <invoke>",
            }
            .to_string()
        })
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
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        // Every type a datamodel holds is spelled but bytes: a list admits only
        // numbers, bools and records (`AlgorithmValueType::list_elem_admitted`).
        if let Some(var) = scope.variables.iter().find(|v| {
            !v.value_type
                .as_ref()
                .is_some_and(|t| !matches!(t.scalar(), Some(SceType::Bytes)))
        }) {
            return Some(format!("<data id=\"{}\"> of a bytes type", var.id));
        }
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
            if let Some(other) = unlowered_invoke(&state.invokes) {
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
                format!(
                    "\n// {reader} reports the `{id}` field.\nfunc (r {ty}) {reader}() {} {{\n\treturn r.{}\n}}\n",
                    go_field_type(field),
                    self.record_field(&field.id),
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
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
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
    fn callee(&self, document_name: &str) -> Option<Callee> {
        Some(generated_callee(Language::Python, document_name, None))
    }
    fn unsupported(&self, model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
        // Every type a datamodel holds is spelled but bytes: a list admits only
        // numbers, bools and records (`AlgorithmValueType::list_elem_admitted`).
        if let Some(var) = scope.variables.iter().find(|v| {
            !v.value_type
                .as_ref()
                .is_some_and(|t| !matches!(t.scalar(), Some(SceType::Bytes)))
        }) {
            return Some(format!("<data id=\"{}\"> of a bytes type", var.id));
        }
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
            if let Some(other) = unlowered_invoke(&state.invokes) {
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
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value}")
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
}

/// Rewrite `model` — a clone the Python backend renders — so every expression
/// of a `sce-static` document is native Python.
pub fn lower_python(
    model: &mut SCXMLModel,
    machine: &str,
) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, &PythonTarget)
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
                | "sce_clear" | "foreach" => {}
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

    /// The C type a value of `ty` is held in while it is computed. An enum's
    /// value has no inferred type of its own, and is held in an `int` — which
    /// the enumerated type takes back by assignment.
    fn value_type(ty: InferredType) -> String {
        match ty {
            InferredType::Unknown => "int".to_string(),
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
        // A string or a bytes value needs a capacity the C11 contract does not
        // carry yet, and a real is not yet held to a scenario. A record is a
        // struct the machine's own header declares, of fields held as those are.
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
                        crate::forge::model::ListElemType::Scalar(ty) => held_scalar(ty),
                        crate::forge::model::ListElemType::Record { .. } => true,
                    });
            }
            !matches!(
                value_type.scalar(),
                Some(ty) if held_scalar(ty) || matches!(ty, SceType::Enum(_))
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
        // and bools a list holds, and an enum the machine imports under the
        // alias the schema writes. A field is named as the author wrote it; a
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
            if let Some(field) = schema
                .fields
                .iter()
                .find(|f| !held_scalar(&f.sce_type) && !matches!(f.sce_type, SceType::Enum(_)))
            {
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
            // fields it holds as values; a string is a borrowed pointer into a
            // buffer the machine owns, bytes a buffer with a length, and an
            // enum has no field type there — none of them is held to a
            // scenario.
            for t in &state.transitions {
                let Some(schema) = model.imported_event_schemas.get(&t.event) else {
                    continue;
                };
                if let Some(field) = schema.fields.iter().find(|f| {
                    matches!(
                        f.sce_type,
                        SceType::String | SceType::Bytes | SceType::Enum(_)
                    )
                }) {
                    return Some(format!(
                        "a transition on `{}`, an event whose payload carries `{}` of type {}",
                        t.event,
                        field.id,
                        field.sce_type.as_attr()
                    ));
                }
            }
            if !state.invokes.is_empty() {
                return Some("an <invoke>".to_string());
            }
            if state.donedata.as_ref().is_some_and(|done| {
                !done.params.is_empty()
                    || !matches!(done.content, crate::model::DoneDataContent::None)
            }) {
                return Some("a <donedata>".to_string());
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
                let field_ty = match &field.sce_type {
                    SceType::Enum(reference) => enum_types[&reference.alias].clone(),
                    other => crate::forge::generator::c_type(other).to_string(),
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
    // reads and cannot grow past the bound the machine keeps.
    fn list_view(&self, elem: &SceType) -> Option<String> {
        Some(format!("sce_forge_{}_view_t", elem.as_attr()))
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
    fn bounded_string(&self, _value: &str, _capacity: u32) -> String {
        unreachable!("a C11 document with a string variable is refused by `unsupported`")
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
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
    fn wire_value(&self, _ty: InferredType, _value: &str) -> String {
        unreachable!("a C11 document with a <send> or an <invoke> is refused by `unsupported`")
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
    // Each statement lands whole in `native_code`, and each condition in
    // `native_cond` — the slots every backend's action dispatcher reads before
    // its own spellings, so the IR the author wrote is left as it was.
    match action.action_type.as_str() {
        // A whole record is taken from a record of its schema by name — a
        // record variable or a loop's record item, which validation judged
        // before this walk — and is a plain value, so the variable holds a copy
        // of it as it stands now. The name lowers as any other: renamed to the
        // field that holds it, which is all the assignment needs.
        "assign" => {
            reads_payload = reads(&action.expr);
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            let mut value = lower(&action.expr, slot)?;
            let location = action.location.trim();
            // A string variable holds at most what it declared, whatever the
            // value came from: past it the assignment fails as any other does,
            // writing nothing and ending its block (§scxml-4.9).
            if let Some(capacity) = rewrites.strings.get(location) {
                value = Receiving {
                    text: target.bounded_string(&value.text, *capacity),
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
                    statement(
                        &value,
                        slot,
                        &|v| target.assign_field(name, &field, v),
                        construct,
                    )
                }
                None => {
                    let name = renames.get(location).copied().unwrap_or(location);
                    statement(&value, slot, &|v| target.assign(name, v), construct)
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
                crate::forge::model::ListElemType::Record { .. } => {
                    reads_payload = false;
                    let written = action.expr.trim();
                    Receiving {
                        text: renames.get(written).copied().unwrap_or(written).to_string(),
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
            return lower_actions(&mut action.actions, &inner, renames, rewrites);
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
            for param in &mut action.params {
                lower_wire_param(param, ctx, renames, rewrites)?;
            }
            // A literal `<content>` is the event's data as written, finished
            // here ([`crate::filters::static_content_wire`]): the machine has
            // no engine to evaluate the text with.
            if !action.content.trim().is_empty() {
                action.native_content = crate::filters::static_content_wire(&action.content);
            }
        }
        _ => {}
    }
    Ok(lower_nested(action, ctx, renames, rewrites)? || reads_payload)
}

/// Lower the value of a `<param>` that crosses as data (SCE Accepted Subset
/// §2.15): the expression, read from the machine's fields, as the typed value
/// the backend's wire helpers take ([`StaticTarget::wire_value`]), and whether
/// it can fail. `None` for one with nothing to lower: a static literal, which
/// is folded at build time and left as it is, and a param with no expression.
///
/// Validation already judged the expression against the same scope and held
/// its type to [`InferredType::wire_param_slot`], so a refusal here is a
/// lowering this backend lacks, not a mistake in the document.
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
    let refused = |why: String| {
        GenerateError::unsupported(format!(
            "<param name=\"{}\"> `{written}` has no {lang} lowering: {why}",
            param.name
        ))
    };
    let ty = crate::forge::expr::judge_into(
        written,
        ctx,
        crate::forge::expr::Expected::Hint(InferredType::Unknown),
    )
    .map_err(|r| refused(r.error.to_string()))?;
    let slot = ty
        .wire_param_slot()
        .ok_or_else(|| refused("its type has no wire spelling".to_string()))?;
    // The value is read once and is its own: an owned string, for the typed
    // value that carries it.
    let value = transpile_into_owned(written, target.expr_target(), ctx, renames, slot)
        .map_err(|r| refused(r.error.to_string()))?;
    rewrites.note(written, param.spelling, &value.text);
    Ok(Some((target.wire_value(slot, &value.text), value.can_fail)))
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
    let target = rewrites.target;
    let lang = target.name();
    let declared = info.common.child_static_variables.as_deref().unwrap_or(&[]);
    for param in &mut arguments {
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
        let slot = declared
            .iter()
            .find(|v| v.id == param.name)
            .and_then(crate::forge::static_datamodel::seed_slot)
            .ok_or_else(|| refused("the child declares no variable it can be handed to".into()))?;
        let value = transpile_into_owned(&written, target.expr_target(), ctx, renames, slot)
            .map_err(|r| refused(r.error.to_string()))?;
        rewrites.note(&written, spelling.as_ref(), &value.text);
        param.native_seed = value.text;
        param.native_fails = value.can_fail;
    }
    info.common.base.params = arguments;
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
    fn a_delayed_sends_target_and_delay_are_literals_under_sce_static() {
        // `delays_a_send_to_another_session` reads both as written. A target
        // that is only known at run time may be any session, so the day this
        // data model types `targetexpr` (or a `delayexpr` that makes a send
        // delayed) this fails, and the predicate has to say what it does.
        for send in [
            r#"<send event="later" targetexpr="where" delay="5s"/>"#,
            r#"<send event="later" delayexpr="how_long"/>"#,
        ] {
            let refusal = SCXMLParser::new()
                .parse_string(&counter_sending(send), "m")
                .expect_err(send);
            assert!(
                format!("{refusal:?}").contains("this attribute has no typed form"),
                "{send}: {refusal:?}"
            );
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
}
