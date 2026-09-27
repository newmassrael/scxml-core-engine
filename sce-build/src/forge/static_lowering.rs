// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
use crate::forge::expr::{transpile_into_receiving, ExprTarget, Receiving, Refusal};
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

/// How one backend spells what a `sce-static` document says. The walk
/// ([`lower`]) asks; the answer is the only thing that differs by backend.
pub trait StaticTarget {
    /// The backend, for the spellings shared with other kinds (a callee's
    /// import, a record field's identifier).
    fn lang(&self) -> Language;
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
    /// Log `value`, prefixed with `label` when there is one (§scxml-4.7).
    fn log(&self, label: &str, value: &str) -> String;
    /// Append `value` to the list at `target` while it holds fewer than
    /// `capacity` elements; otherwise run `overflow`, if any.
    fn append(&self, target: &str, capacity: u32, value: &str, overflow: Option<&str>) -> String;
    /// Empty the list at `target`.
    fn clear(&self, target: &str) -> String;
    /// Raise `error.execution` with `message` (§scxml-3.12.2).
    fn raise_execution_error(&self, machine: &str, message: &str) -> String;
    /// `statement`, whose expressions can fail (SCE_FORGE.md §3.4.1), run
    /// where its failure is received: a failure stops it before it writes
    /// anything, and `failed` runs instead (E12 D5).
    fn receiving_statement(&self, statement: &str, failed: &str) -> String;
    /// A condition that can fail: its value, or `false` once `failed` has run
    /// (§scxml-5.9.1: a condition that cannot be evaluated is false, and
    /// `error.execution` says why).
    fn receiving_condition(&self, value: &str, failed: &str) -> String;
    /// What `_event.data` is read through inside a guard or statement of an
    /// `event` carrying a typed payload.
    fn payload_accessor(&self, event: &str) -> String;
    /// `lowered`, a guard reading the payload, held to the delivery having
    /// carried one.
    fn payload_guard(&self, machine: &str, event: &str, lowered: &str) -> String;
}

/// Kotlin: a variable is a property of the machine class, a record an
/// immutable data class replaced field by field, a list an immutable `List`.
pub struct KotlinTarget;

impl StaticTarget for KotlinTarget {
    fn lang(&self) -> Language {
        Language::Kotlin
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
    // holding the old one keeps what it saw.
    fn append(&self, target: &str, capacity: u32, value: &str, overflow: Option<&str>) -> String {
        let otherwise = overflow.map_or(String::new(), |o| format!(" else {{ {o} }}"));
        format!("if ({target}.size < {capacity}) {{ {target} = {target} + ({value}) }}{otherwise}")
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
    // an algorithm's boundary catches it.
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
        format!(
            "try {{ {statement} }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {failed} }}"
        )
    }
    fn receiving_condition(&self, value: &str, failed: &str) -> String {
        let failed = if failed.is_empty() {
            String::new()
        } else {
            format!("{failed}; ")
        };
        format!(
            "(try {{ {value} }} catch (_: com.sce.forge.runtime.AlgorithmFailure) {{ {failed}false }})"
        )
    }
    fn payload_accessor(&self, event: &str) -> String {
        format!("{}!!", kotlin_payload_field(event))
    }
    fn payload_guard(&self, _machine: &str, event: &str, lowered: &str) -> String {
        format!("{} != null && ({lowered})", kotlin_payload_field(event))
    }
}

/// Rust: a variable is a field of the machine's policy struct, a record a
/// plain `Copy` struct updated in place, a list a `Vec` bounded by its
/// declared capacity.
pub struct RustTarget;

impl StaticTarget for RustTarget {
    fn lang(&self) -> Language {
        Language::Rust
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
    fn append(&self, target: &str, capacity: u32, value: &str, overflow: Option<&str>) -> String {
        let otherwise = overflow.map_or(String::new(), |o| format!(" else {{ {o} }}"));
        format!("if {target}.len() < {capacity} {{ {target}.push({value}); }}{otherwise}")
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
    fn receiving_statement(&self, statement: &str, failed: &str) -> String {
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
    fn receiving_condition(&self, value: &str, failed: &str) -> String {
        format!(
            "match (|| -> Result<bool, sce_forge_runtime::algorithm::AlgorithmError> {{ Ok({value}) }})() {{ Ok(sce_value) => sce_value, Err(_) => {{ {failed} false }} }}"
        )
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
}

/// Every name a lowered `sce-static` expression spells differently on
/// `target`: each variable to the way the backend reaches its field, each
/// imported algorithm to the function its generation emits — the name a
/// forge kind calling the same algorithm uses. One list for every lowering, so
/// a guard, an assignment and a host action's argument cannot disagree about
/// a name.
fn names(scope: &StaticScope, target: &dyn StaticTarget) -> Vec<(String, String)> {
    let lang = target.lang();
    scope
        .variables
        .iter()
        .map(|v| (v.id.clone(), target.field_ref(&target.field_name(&v.id))))
        .chain(scope.callees.iter().map(|c| {
            let identity = crate::forge::generator::forge_import_identity(
                &c.document_name,
                &lang,
                false,
                &crate::ForgeCompileOptions::default(),
            );
            let symbol = crate::forge::generator::forge_algorithm_symbol(&c.document_name, lang);
            (
                c.alias.clone(),
                crate::build_qualified_call(&symbol, &identity.namespace, &lang),
            )
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
    let lang = target.lang();
    let variables = &scope.variables;
    let schemas = model.imported_event_schemas.clone();
    let records = model.imported_records.clone();
    let names = names(&scope, target);
    let imports: Vec<String> = scope
        .callees
        .iter()
        .map(|c| {
            crate::forge::generator::forge_import_identity(
                &c.document_name,
                &lang,
                false,
                &crate::ForgeCompileOptions::default(),
            )
            .include_stmt
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
    };

    let refused = |what: &str, text: &str, refusal: Refusal| {
        GenerateError::unsupported(format!(
            "{what} `{text}` has no {lang:?} lowering: {}",
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
                        "record:{alias} has the enum-typed field `{}`, which has no {lang:?} \
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
                fields.push(StaticField {
                    id: var.id.clone(),
                    name,
                    init: target.record_value(&ty, &values),
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
                    "<data id=\"{}\" sce:type=\"{}\">: an enum-typed variable has no {lang:?} \
                     lowering in a statechart yet",
                    var.id,
                    ty.as_attr()
                )));
            }
            let slot = InferredType::from_sce_type(ty);
            let init = initial_value(&var.expr, target, &ctx, &renames, slot)
                .map_err(|r| refused("the initial value", &var.expr, r))?;
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
                // §scxml-5.9.1: a condition that fails is false, and
                // `error.execution` says why (E12 D5).
                let lowered = if cond.can_fail {
                    target.receiving_condition(
                        &cond.text,
                        &execution_failure(
                            &rewrites,
                            &format!("<transition cond='{}'>", transition.cond),
                        ),
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
/// `None` for a machine whose state lives partly in the runtime rather than
/// in its fields — what a `<history>` recorded, a delayed `<send>` still
/// pending, an invoked session — which this version of the saved state cannot
/// hold. Such a machine is generated without the save API rather than with
/// one that would silently drop part of its state.
fn saved_shape(model: &SCXMLModel, scope: &StaticScope) -> Option<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    if model.has_history_states || model.needs_event_scheduler_driving() || model.has_invoke() {
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
    let value = transpile_into_receiving(expr, target.expr_target(), ctx, renames, slot)?;
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
    Some(target.receiving_statement(statement, &failed))
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
    let lang = target.lang();
    let lower = |text: &str, slot: InferredType| {
        transpile_into_receiving(text, target.expr_target(), ctx, renames, slot).map_err(|r| {
            GenerateError::unsupported(format!("`{text}` has no {lang:?} lowering: {}", r.error))
        })
    };
    let failed = |construct: String| execution_failure(rewrites, &construct);
    // `write(value)`, received where it stands when `value` can fail.
    let statement = |value: &Receiving, write: &dyn Fn(&str) -> String, construct: String| {
        let written = write(&value.text);
        if value.can_fail {
            target.receiving_statement(&written, &failed(construct))
        } else {
            written
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
            action.native_code = match location
                .split_once('.')
                .filter(|(var, _)| rewrites.records.contains_key(*var))
            {
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
            action.native_cond = if cond.can_fail {
                target.receiving_condition(
                    &cond.text,
                    &failed(format!("<if cond='{}'>", action.cond)),
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
            let label = action.label.clone();
            action.native_code = statement(&value, &|v| target.log(&label, v), "<log>".to_string());
        }
        // An append happens only while the list is under its bound — on
        // every backend, so a machine holds the same list wherever it runs.
        // Past the bound nothing is appended and `error.execution` says so —
        // the processor's own signal for an error in executing the document,
        // raised the way every other execution error of the backend is.
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
            let overflow = rewrites.raises_error.then(|| {
                target.raise_execution_error(
                    rewrites.machine,
                    &format!(
                        "<sce:append target='{list}'>: the list already holds its capacity of \
                         {capacity}"
                    ),
                )
            });
            action.native_code = statement(
                &value,
                &|v| target.append(name, *capacity, v, overflow.as_deref()),
                format!("<sce:append target='{list}'>"),
            );
        }
        "sce_clear" => {
            let list = action.location.trim();
            let name = renames.get(list).copied().unwrap_or(list);
            action.native_code = target.clear(name);
        }
        _ => {}
    }
    Ok(lower_nested(action, ctx, renames, rewrites)? || reads_payload)
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
                "`{}` has no {:?} lowering: {}",
                branch.cond,
                target.lang(),
                r.error
            ))
        })?;
        branch.native_cond = if cond.can_fail {
            target.receiving_condition(
                &cond.text,
                &execution_failure(rewrites, &format!("<elseif cond='{}'>", branch.cond)),
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

    #[test]
    fn a_machine_with_a_history_has_no_saved_shape() {
        // What a <history> recorded lives in the runtime, which this version
        // of the saved state does not hold.
        let with_history = COUNTER
            .replace(
                r#"<state id="counting">"#,
                r#"<state id="outer" initial="counting"><history id="h"><transition target="counting"/></history><state id="counting">"#,
            )
            .replace(r#"<final id="done"/>"#, r#"</state><final id="done"/>"#);
        assert_eq!(shape(&with_history), None);
    }
}
