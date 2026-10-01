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
use crate::forge::model::{EventSchemaModel, SceType};
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
    /// The bound of a list or a byte string: the machine never holds more,
    /// so neither may a restored value.
    pub bound: Option<u32>,
    /// How a saved state holds the value: `scalar`, `list` or `record`.
    pub saved_kind: &'static str,
    /// What [`Self::saved_kind`] is of: a scalar's or a list element's
    /// `sce:type` spelling (`uint8`, `bool`, `bytes`, …), or a record's
    /// backend type. A backend without overloading on type reads each value
    /// with the function this names.
    pub saved_type: String,
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
    /// One type declaration per event-schema a `record:<alias>` variable
    /// names, declared in the machine's own file the way its event payload
    /// types are.
    pub record_defs: Vec<String>,
    /// The import line of each algorithm the document calls — the line a
    /// forge kind importing the same algorithm writes, so the machine reaches
    /// the function where the algorithm's own generation put it.
    pub imports: Vec<String>,
    /// The record types of [`Self::record_defs`], field by field — what a
    /// saved state writes a record value as.
    pub records: Vec<StaticRecord>,
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

/// An imported algorithm as a target reaches it: the name a call is written
/// with, and the line that makes the name visible where the machine is.
#[derive(Debug, Clone)]
pub struct Callee {
    /// The callee, qualified as the generated unit that holds it requires.
    pub call: String,
    /// The import line a forge kind importing the same algorithm writes, so
    /// the machine reaches the function where the algorithm's own generation
    /// put it.
    pub import: String,
}

/// [`Callee`] for a generated-code backend: the identity and symbol every
/// other forge kind derives for the same document, so a statechart and an
/// algorithm that imports it agree on where it lives.
fn generated_callee(lang: Language, document_name: &str) -> Callee {
    let identity = crate::forge::generator::forge_import_identity(
        document_name,
        &lang,
        false,
        &crate::ForgeCompileOptions::default(),
    );
    let symbol = crate::forge::generator::forge_algorithm_symbol(document_name, lang);
    Callee {
        call: crate::build_qualified_call(&symbol, &identity.namespace, &lang),
        import: identity.include_stmt,
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
    /// How a statement or a guard reaches the field named `name`.
    fn field_ref(&self, name: &str) -> String;
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
    fn record_def(&self, ty: &str, alias: &str, schema: &EventSchemaModel) -> String;
    /// The identifier of schema field `id` on the record type.
    fn record_field(&self, id: &str) -> String;
    /// A record value built whole from `(field identifier, value)` pairs.
    fn record_value(&self, ty: &str, fields: &[(String, String)]) -> String;
    /// The type of a list of `elem`.
    fn list_type(&self, elem: &SceType) -> String;
    /// The type a host reads a published list of `elem` through, when it is
    /// not the list's own type (see [`StaticField::view`]).
    fn list_view(&self, elem: &SceType) -> Option<String>;
    /// An empty list.
    fn list_empty(&self) -> String;
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
    /// Log `value`, prefixed with `label` when there is one (§scxml-4.7).
    fn log(&self, label: &str, value: &str) -> String;
    /// Append `value` to the list at `target` while it holds fewer than
    /// `capacity` elements, as an expression that is `true` when the append
    /// failed. It fails when the list is full — `overflow`, if any, runs, and
    /// nothing is appended — and, when `value_can_fail`, when computing
    /// `value` fails, where `failed` runs (both raise `error.execution`).
    /// Either way the statement has raised an error, and the block it stands
    /// in ends (W3C SCXML 4.9), which is the dispatcher's to do.
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
    /// Raise `error.execution` with `message` (§scxml-3.12.2).
    fn raise_execution_error(&self, machine: &str, message: &str) -> String;
    /// `statement`, whose expressions can fail (SCE_FORGE.md §3.4.1), as an
    /// expression that is `true` when it failed: it is run where its failure
    /// is received, a failure stops it before it writes anything, and
    /// `failed` runs instead (E12 D5). An error ends the block the element
    /// stands in (W3C SCXML 4.9); which block that is, and what leaves it,
    /// the dispatcher knows from where it renders the action, so it acts on
    /// the `true`.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String;
    /// `statement`, a call whose arguments can fail, run where its failure is
    /// received: a failure stops it before it happens, and `failed` runs
    /// instead. A statement, not an expression: the call sits inside the text
    /// a host action renders, which does not yet end its block on a failure.
    fn receiving_call(&self, statement: &str, failed: &str) -> String;
    /// A condition that can fail: its value, or `false` once `failed` has run
    /// and then `flag` (§scxml-5.9.1: a condition that cannot be evaluated is
    /// false, and `error.execution` says why). `flag` is
    /// [`Self::condition_failed_flag`] for the condition of an `<if>` or an
    /// `<elseif>`, and empty for a transition's guard, which stands in no
    /// block.
    fn receiving_condition(&self, value: &str, failed: &str, flag: &str) -> String;
    /// The statement that records, on the `<if>` numbered `if_ordinal`, that
    /// one of its conditions failed. The `<if>` runs its chain on — a
    /// condition that cannot be evaluated is false — and then ends its block
    /// (W3C SCXML 4.9), as any element that raised does.
    fn condition_failed_flag(&self, if_ordinal: u32) -> String;
    /// What `_event.data` is read through inside a guard or statement of an
    /// `event` carrying a typed payload.
    fn payload_accessor(&self, event: &str) -> String;
    /// `lowered`, a guard reading the payload, held to the delivery having
    /// carried one.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String;
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
        Some(generated_callee(Language::Kotlin, document_name))
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
    fn record_def(&self, ty: &str, alias: &str, schema: &EventSchemaModel) -> String {
        let params: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                format!(
                    "val {}: {}",
                    self.record_field(&field.id),
                    crate::forge::generator::kotlin_type(&field.sce_type)
                )
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
                format!(
                    "\"{}\" to SavedValues.of({})",
                    field.id,
                    self.record_field(&field.id)
                )
            })
            .collect();
        let reads: Vec<String> = schema
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{name} = SavedValues.{ty}(SavedValues.field(value, what, \"{id}\"), \"$what.{id}\")",
                    name = self.record_field(&field.id),
                    ty = field.sce_type.as_attr(),
                    id = field.id
                )
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
    fn list_type(&self, elem: &SceType) -> String {
        format!("List<{}>", crate::forge::generator::kotlin_type(elem))
    }
    // An immutable `List` is handed out as it is.
    fn list_view(&self, _elem: &SceType) -> Option<String> {
        None
    }
    fn list_empty(&self) -> String {
        "emptyList()".to_string()
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
    fn log(&self, label: &str, value: &str) -> String {
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
        Some(generated_callee(Language::Rust, document_name))
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
    fn record_def(&self, ty: &str, alias: &str, schema: &EventSchemaModel) -> String {
        let fields: String = schema
            .fields
            .iter()
            .map(|field| {
                format!(
                    "    pub {}: {},\n",
                    self.record_field(&field.id),
                    crate::forge::generator::rust_type(&field.sce_type)
                )
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
    fn list_type(&self, elem: &SceType) -> String {
        format!("Vec<{}>", crate::forge::generator::rust_type(elem))
    }
    // A host reads the elements, never the machine's own `Vec`, so it cannot
    // grow the list past the bound the machine keeps.
    fn list_view(&self, elem: &SceType) -> Option<String> {
        Some(format!("[{}]", crate::forge::generator::rust_type(elem)))
    }
    fn list_empty(&self) -> String {
        "Vec::new()".to_string()
    }
    fn assign(&self, target: &str, value: &str) -> String {
        format!("{target} = {value};")
    }
    fn assign_field(&self, target: &str, field: &str, value: &str) -> String {
        format!("{target}.{field} = {value};")
    }
    // Debug formatting, as the script-engine arm of the same template logs a
    // value: a record has no `Display`, and one spelling serves every type.
    fn log(&self, label: &str, value: &str) -> String {
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
    enums: &[StaticEnum],
) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, enums, &KotlinTarget)
}

/// Rewrite `model` — a clone the Rust backend renders — so every expression
/// of a `sce-static` document is native Rust.
pub fn lower_rust(
    model: &mut SCXMLModel,
    machine: &str,
    enums: &[StaticEnum],
) -> Result<StaticLowering, GenerateError> {
    lower(model, machine, enums, &RustTarget)
}

/// Rewrite `model` — a clone one backend renders — so every expression of a
/// `sce-static` document is that backend's own code. A document under any
/// other data model is left as it is.
///
/// An enum-typed variable, or a record whose schema has an enum field, is
/// refused: its type is the enum document's, which a statechart does not yet
/// import into its generated unit.
pub fn lower(
    model: &mut SCXMLModel,
    machine: &str,
    enums: &[StaticEnum],
    target: &dyn StaticTarget,
) -> Result<StaticLowering, GenerateError> {
    let Some(scope) = StaticScope::of(model) else {
        return Ok(StaticLowering::default());
    };
    let lang = target.name();
    let variables = &scope.variables;
    let schemas = model.imported_event_schemas.clone();
    let records = model.imported_records.clone();
    // A call the target cannot spell would be an undefined name where the
    // machine runs; it is refused here, where the document is read.
    if let Some(unreached) = scope
        .callees
        .iter()
        .find(|c| target.callee(&c.document_name).is_none())
    {
        return Err(GenerateError::unsupported(format!(
            "`{}(…)`: an imported algorithm has no {lang} lowering yet",
            unreached.alias
        )));
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
        .filter_map(|c| target.callee(&c.document_name).map(|callee| callee.import))
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
    // `<sce:append>` is rewritten against. A machine list is of scalars: the
    // parser refuses a list of records on a variable.
    let list_vars: ListVars = variables
        .iter()
        .filter_map(|v| {
            let elem = v.value_type.as_ref()?.list_elem()?.scalar()?.clone();
            Some((v.id.clone(), (elem, v.capacity?)))
        })
        .collect();
    let rewrites = Rewrites {
        records: record_vars,
        lists: list_vars,
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
    let mut record_defs = Vec::new();
    let mut saved_records = Vec::new();
    let mut declared_types = BTreeSet::new();
    // Taken before any expression is rewritten: the shape is the document's,
    // and the same for every backend.
    let saved_shape = saved_shape(model, &scope);
    {
        let ctx = scope.ctx(&no_payload, enums);
        let init_names: Vec<(String, String)> = variables
            .iter()
            .map(|v| (v.id.clone(), target.field_name(&v.id)))
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
                if let Some(field) = schema
                    .fields
                    .iter()
                    .find(|f| matches!(f.sce_type, SceType::Enum(_)))
                {
                    return Err(GenerateError::unsupported(format!(
                        "record:{alias} has the enum-typed field `{}`, which has no {lang} \
                         lowering in a statechart yet",
                        field.id
                    )));
                }
                let ty = target.record_type(machine, alias);
                if declared_types.insert(ty.clone()) {
                    record_defs.push(target.record_def(&ty, alias, schema));
                    saved_records.push(StaticRecord {
                        ty: ty.clone(),
                        fields: schema
                            .fields
                            .iter()
                            .map(|f| StaticRecordField {
                                id: f.id.clone(),
                                name: target.record_field(&f.id),
                                saved_type: f.sce_type.as_attr(),
                            })
                            .collect(),
                    });
                }
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
                });
                continue;
            }
            // A list starts empty.
            if let Some(elem) = var
                .value_type
                .as_ref()
                .and_then(|t| t.list_elem())
                .and_then(crate::forge::model::ListElemType::scalar)
            {
                if let Some(element) = target.data_element(&var.id, &target.list_empty()) {
                    rewrites.note_element(var.value_type_spelling.as_ref(), &element);
                }
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    ty: target.list_type(elem),
                    init: target.list_empty(),
                    published,
                    view: target.list_view(elem),
                    bound: var.capacity,
                    saved_kind: "list",
                    saved_type: elem.as_attr(),
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
            if matches!(ty, SceType::Enum(_)) {
                return Err(GenerateError::unsupported(format!(
                    "<data id=\"{}\" sce:type=\"{}\">: an enum-typed variable has no {lang} \
                     lowering in a statechart yet",
                    var.id,
                    ty.as_attr()
                )));
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
                bound: matches!(ty, SceType::Bytes)
                    .then_some(var.capacity)
                    .flatten(),
                saved_kind: "scalar",
                saved_type: ty.as_attr(),
            });
        }
    }

    let mut payload_events = BTreeSet::new();
    for state in model.states.values_mut() {
        let plain_ctx = scope.ctx(&no_payload, enums);
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
            if let crate::model::Invoke::Unsupported(info) = invoke {
                for param in &mut info.base.params {
                    lower_wire_param(param, &plain_ctx, &plain_renames, &rewrites)?;
                }
            }
        }
        for transition in &mut state.transitions {
            let schema = schemas.get(&transition.event);
            let paths = scope.paths(schema);
            let ctx = scope.ctx(&paths, enums);
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
                let lowered = if cond.can_fail {
                    target.receiving_condition(
                        &cond.text,
                        &execution_failure(
                            &rewrites,
                            &format!("<transition cond='{}'>", transition.cond),
                        ),
                        "",
                    )
                } else {
                    cond.text
                };
                // A condition that reads the payload holds only while the
                // dequeued event carried one — the guard every typed
                // payload read on the backend takes. It lands in the one
                // slot every backend's guard macro reads for a guard lowered
                // at generate time; `cond_kt` stays the author's `kt:` text.
                transition.native_guard = if schema.is_some()
                    && crate::forge::expr::references_event_data_lexically(&transition.cond)
                {
                    payload_events.insert(transition.event.clone());
                    target.payload_guard(machine, &transition.event, &lowered)
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
        let ctx = scope.ctx(&no_payload, enums);
        lower_action(script, &ctx, &renames(&names, None, target), &rewrites)?;
    }
    Ok(StaticLowering {
        fields,
        payload_events,
        record_defs,
        imports,
        records: saved_records,
        saved_shape,
        sites: rewrites.sites.into_inner(),
        elements: rewrites.elements.into_inner(),
    })
}

/// The shape a saved state of this machine is bound to (SCE Accepted Subset
/// §2.15, "Saving and restoring"): a SHA-256 over every state with its kind
/// and parent, in document order, and every variable with its type and bound,
/// a record's fields included — what a saved state names, and nothing else.
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
        if let Some(schema) = var
            .value_type
            .as_ref()
            .and_then(|t| t.record_alias())
            .and_then(|alias| model.imported_records.get(alias))
        {
            for field in &schema.fields {
                let _ = writeln!(
                    text,
                    "field {}.{} {}",
                    var.id,
                    field.id,
                    field.sce_type.as_attr()
                );
            }
        }
    }
    Some(format!("{:x}", Sha256::digest(text.as_bytes())))
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
type ListVars = std::collections::BTreeMap<String, (SceType, u32)>;

/// What rewriting an action needs beyond its expressions: the record and
/// list variables a write to one is rewritten against, the machine name the
/// generated event type is spelled from, whether the document declares
/// `error.execution` — without it there is no variant to raise, and nothing
/// could match one — and the backend spelling it all.
struct Rewrites<'m> {
    records: RecordVars,
    lists: ListVars,
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

/// The target that spells `lang`, when it lowers `sce-static` at all.
pub(crate) fn target_for(lang: Language) -> Option<&'static dyn StaticTarget> {
    match lang {
        Language::Kotlin => Some(&KotlinTarget),
        Language::Rust => Some(&RustTarget),
        Language::Cpp | Language::C11 | Language::Go | Language::Python => None,
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
/// the document declares that event (E12 D5). `None` for a backend that does
/// not lower the model.
pub(crate) fn receive_static_statement(
    lang: Language,
    statement: &str,
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
    Some(target.receiving_call(statement, &failed))
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
    // on by ending the block (W3C SCXML 4.9) — with whether it can.
    let statement =
        |value: &Receiving, write: &dyn Fn(&str) -> String, construct: String| -> (String, bool) {
            let written = write(&value.text);
            if value.can_fail {
                (
                    target.receiving_statement(&written, &failed(construct)),
                    true,
                )
            } else {
                (written, false)
            }
        };
    let reads = crate::forge::expr::references_event_data_lexically;
    let mut reads_payload = false;
    // Each statement lands whole in `native_code`, and each condition in
    // `native_cond` — the slots every backend's action dispatcher reads before
    // its own spellings, so the IR the author wrote is left as it was.
    match action.action_type.as_str() {
        "assign" => {
            reads_payload = reads(&action.expr);
            let slot = crate::forge::expr::infer_expr_type(&action.location, ctx)
                .unwrap_or(InferredType::Unknown);
            let value = lower(&action.expr, slot)?;
            let location = action.location.trim();
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
                    statement(&value, &|v| target.assign_field(name, &field, v), construct)
                }
                None => {
                    let name = renames.get(location).copied().unwrap_or(location);
                    statement(&value, &|v| target.assign(name, v), construct)
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
            (action.native_code, action.native_fails) =
                statement(&value, &|v| target.log(&label, v), "<log>".to_string());
        }
        // An append happens only while the list is under its bound — on
        // every backend, so a machine holds the same list wherever it runs.
        // Past the bound nothing is appended and `error.execution` says so —
        // the processor's own signal for an error in executing the document,
        // raised the way every other execution error of the backend is — and
        // the block ends (W3C SCXML 4.9), as it does for a value that could
        // not be computed. So an append can always fail.
        "sce_append" => {
            reads_payload = reads(&action.expr);
            let list = action.location.trim();
            let (elem, capacity) = rewrites.lists.get(list).ok_or_else(|| {
                GenerateError::unsupported(format!(
                    "<sce:append target=\"{list}\"> names no list variable"
                ))
            })?;
            let value = lower(&action.expr, InferredType::from_sce_type(elem))?;
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
        // What a `<send>` carries is read from the machine's fields now, when
        // it runs (W3C SCXML 6.2.3 evaluates its arguments once, at the send).
        "send" => {
            for param in &mut action.params {
                lower_wire_param(param, ctx, renames, rewrites)?;
            }
        }
        _ => {}
    }
    Ok(lower_nested(action, ctx, renames, rewrites)? || reads_payload)
}

/// Lower the value of a `<param>` of a `<send>` or of a host-run `<invoke>`
/// (SCE Accepted Subset §2.15), in place: the expression, read from the
/// machine's fields, as the typed value the backend's wire helpers take
/// ([`StaticTarget::wire_value`]) — `Param::native_value`, and whether it can
/// fail.
///
/// A static literal is folded at build time and left as it is. Validation
/// already judged the expression against the same scope and held its type to
/// [`InferredType::wire_param_slot`], so a refusal here is a lowering this
/// backend lacks, not a mistake in the document.
fn lower_wire_param(
    param: &mut crate::model::Param,
    ctx: &crate::forge::types::TypeCtx<'_>,
    renames: &HashMap<&str, &str>,
    rewrites: &Rewrites<'_>,
) -> Result<(), GenerateError> {
    if param.is_static_literal {
        return Ok(());
    }
    // A `location` names a variable, and reading one is reading it as an
    // expression.
    let (written, spelling) = if param.expr.trim().is_empty() {
        (param.location.clone(), param.location_spelling.clone())
    } else {
        (param.expr.clone(), param.expr_spelling.clone())
    };
    if written.trim().is_empty() {
        return Ok(());
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
        &written,
        ctx,
        crate::forge::expr::Expected::Hint(InferredType::Unknown),
    )
    .map_err(|r| refused(r.error.to_string()))?;
    let slot = ty
        .wire_param_slot()
        .ok_or_else(|| refused("its type has no wire spelling".to_string()))?;
    // The value is read once and is its own: an owned string, for the typed
    // value that carries it.
    let value = transpile_into_owned(&written, target.expr_target(), ctx, renames, slot)
        .map_err(|r| refused(r.error.to_string()))?;
    rewrites.note(&written, spelling.as_ref(), &value.text);
    param.native_value = target.wire_value(slot, &value.text);
    param.native_fails = value.can_fail;
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
        lower_rust(&mut model, "M", &[])
            .expect("lowers")
            .saved_shape
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
        lower_rust(&mut model, "M", &[])
            .expect("lowers")
            .saved_shape
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
    <data id="label" sce:type="string" expr="'idle'"/>
    <data id="other" sce:type="string" expr="'x'"/>
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
            Language::Rust => lower_rust(&mut model, "M", &[]),
            Language::Kotlin => lower_kotlin(&mut model, "M", &[]),
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
    <data id="label" sce:type="string" expr="'x'"/>
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
            Language::Rust => lower_rust(&mut model, "M", &[]),
            Language::Kotlin => lower_kotlin(&mut model, "M", &[]),
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
