// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The generated host interface of a typed host-run `<invoke>` (SCE Accepted
//! Subset §2.12).
//!
//! `sce:request` and `sce:result` make an invocation's request and its
//! completion records. The engine's contract stays the generic one — a
//! registered handler receives `HostInvokeEvent`s whose params are text,
//! and a completion is `complete_host_invoke` with the record's JSON — so
//! what this generates sits ABOVE it:
//!
//! - one record struct per typed request and result;
//! - per declared `type` the document types an invoke of, an invoker
//!   interface with `start_<id>` / `cancel_<id>` taking and returning those
//!   records, never text;
//! - an adapter that registers that interface as the type's generic handler,
//!   dispatching by invoke id, and hands an invoke of the same type that is
//!   NOT typed to a fallback handler the host supplies with it;
//! - a typed `complete_<id>` for a typed result.
//!
//! The adapter's reading of a request cannot fail: every value was held to
//! its field where the invocation started (the generated start site calls
//! the runtime's `request_field_wire`), and what the adapter parses is the
//! text that check produced.

use std::collections::BTreeMap;

use crate::filters;
use crate::forge::model::{EventSchemaModel, ForgeField, SceType};
use crate::model::{Invoke, SCXMLModel};

/// One host-run `<invoke>` that names a request or a result record.
pub struct TypedHostInvoke<'m> {
    /// The `type` it names, one the host declared.
    pub invoke_type: &'m str,
    /// Its `id` — required on a typed invoke, so it is the document's own.
    pub invoke_id: &'m str,
    pub request: Option<&'m EventSchemaModel>,
    pub result: Option<&'m EventSchemaModel>,
}

/// The typed host-run invokes of `model`, in document order.
pub fn typed_host_invokes(model: &SCXMLModel) -> Vec<TypedHostInvoke<'_>> {
    host_invokes(model)
        .filter_map(|info| {
            let request = model.imported_records.get(&info.request_schema);
            let result = model.imported_records.get(&info.result_schema);
            (request.is_some() || result.is_some()).then_some(TypedHostInvoke {
                invoke_type: &info.invoke_type,
                invoke_id: &info.base.invoke_id,
                request,
                result,
            })
        })
        .collect()
}

/// Whether `model` runs an invoke of `invoke_type` that names no record — the
/// ones the adapter hands to the host's fallback handler.
fn has_untyped_invoke_of(model: &SCXMLModel, invoke_type: &str) -> bool {
    host_invokes(model).any(|info| {
        info.invoke_type == invoke_type
            && !model.imported_records.contains_key(&info.request_schema)
            && !model.imported_records.contains_key(&info.result_schema)
    })
}

fn host_invokes(model: &SCXMLModel) -> impl Iterator<Item = &crate::model::UnsupportedInvokeInfo> {
    let mut states: Vec<_> = model.states.values().collect();
    states.sort_by_key(|s| s.document_order);
    states
        .into_iter()
        .flat_map(|s| s.invokes.iter())
        .filter_map(|invoke| match invoke {
            Invoke::Unsupported(info) if info.host_served => Some(info),
            _ => None,
        })
}

/// What the Rust start site holds each typed request field to: invoke id →
/// field name → the `RequestFieldType` it names.
pub fn rust_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    typed_host_invokes(model)
        .into_iter()
        .filter_map(|typed| {
            let request = typed.request?;
            let fields = request
                .fields
                .iter()
                .map(|f| (f.id.clone(), rust_request_field_type(f)))
                .collect();
            Some((typed.invoke_id.to_string(), fields))
        })
        .collect()
}

fn rust_request_field_type(f: &ForgeField) -> String {
    let variant = match &f.sce_type {
        SceType::Uint8 => "Uint8",
        SceType::Uint16 => "Uint16",
        SceType::Uint32 => "Uint32",
        SceType::Uint64 => "Uint64",
        SceType::Int8 => "Int8",
        SceType::Int16 => "Int16",
        SceType::Int32 => "Int32",
        SceType::Int64 => "Int64",
        SceType::Float32 => "Float32",
        SceType::Float64 => "Float64",
        SceType::Bool => "Bool",
        SceType::String => "String",
        SceType::Bytes => {
            let cap = crate::forge::limits::resolve_bytes_max(f.max_size);
            return format!("Bytes({cap})");
        }
        SceType::Enum(_) => {
            unreachable!("typed_invoke::validate refuses an enum-typed field in a host-run record")
        }
    };
    variant.to_string()
}

/// The Rust host interface for `model`'s typed host-run invokes; empty when
/// it has none, or under `no_std`, which has no host invokers.
pub fn render_rust(
    model: &SCXMLModel,
    machine_name: &str,
    policy_generics_decl: &str,
    policy_generics_use: &str,
    no_std: bool,
) -> String {
    let typed = typed_host_invokes(model);
    if typed.is_empty() || no_std {
        return String::new();
    }
    let mut records = String::new();
    let mut by_type: BTreeMap<&str, Vec<&TypedHostInvoke>> = BTreeMap::new();
    for invoke in &typed {
        by_type.entry(invoke.invoke_type).or_default().push(invoke);
        let pascal = filters::to_pascal_case(invoke.invoke_id.to_string());
        for (record, what, role) in [
            (
                invoke.request,
                "Request",
                "`sce:request`: the record the host is asked to start",
            ),
            (
                invoke.result,
                "Result",
                "`sce:result`: the record the host completes",
            ),
        ] {
            if let Some(schema) = record {
                records.push_str(&format!(
                    "/// {role} `<invoke id=\"{id}\">`\n/// with (SCE Accepted Subset \u{a7}2.12).\n\
#[derive(Clone, Debug, Default, PartialEq)]\npub struct {machine_name}{pascal}{what} {{\n{fields}}}\n\n",
                    id = invoke.invoke_id,
                    fields = schema
                        .fields
                        .iter()
                        .map(|f| format!(
                            "    pub {}: {},\n",
                            f.id,
                            crate::forge::generator::rust_record_type(f)
                        ))
                        .collect::<String>(),
                ));
            }
        }
    }

    let mut interfaces = String::new();
    let mut ext_methods = String::new();
    let mut ext_impls = String::new();
    for (invoke_type, invokes) in &by_type {
        let type_pascal = filters::to_pascal_case(invoke_type.to_string());
        let type_snake = filters::to_snake_case(invoke_type.to_string());
        let trait_name = format!("{machine_name}{type_pascal}Invoker");
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut methods = String::new();
        let mut start_arms = String::new();
        let mut cancel_arms = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let pascal = filters::to_pascal_case(id.to_string());
            let snake = filters::to_snake_case(id.to_string());
            let request_param = invoke
                .request
                .map(|_| format!("request: {machine_name}{pascal}Request, "))
                .unwrap_or_default();
            let returns = if invoke.result.is_some() {
                format!("{machine_name}{pascal}Result")
            } else {
                "::sce_rust_runtime::HostInvokeResponse".to_string()
            };
            methods.push_str(&format!(
                "    /// \u{a7}scxml-6.4: begin `<invoke id=\"{id}\">`. `token` names this start;\n    \
/// a host that finishes later hands it back to `complete_{snake}` (or\n    \
/// `complete_host_invoke`). `Some` completes the invocation now.\n    \
fn start_{snake}(&mut self, {request_param}token: u64) -> Option<{returns}>;\n    \
/// \u{a7}scxml-6.4: `<invoke id=\"{id}\">`'s state exited while the start\n    \
/// `token` names was still running. Stop it.\n    \
fn cancel_{snake}(&mut self, token: u64);\n"
            ));
            let read_request = match invoke.request {
                Some(schema) => {
                    let fields: String = schema
                        .fields
                        .iter()
                        .map(|f| {
                            let reader = if matches!(f.sce_type, SceType::Bytes) {
                                "request_bytes_field"
                            } else {
                                "request_field"
                            };
                            format!(
                                "                        {0}: ::sce_rust_runtime::{reader}(&request, \"{0}\"),\n",
                                f.id
                            )
                        })
                        .collect();
                    format!(
                        "                    let typed = {machine_name}{pascal}Request {{\n{fields}                    }};\n"
                    )
                }
                None => String::new(),
            };
            let typed_arg = if invoke.request.is_some() {
                "typed, "
            } else {
                ""
            };
            let answer = if invoke.result.is_some() {
                format!(
                    ".map(|result| ::sce_rust_runtime::HostInvokeResponse {{\n                            \
done_data: Some({machine_name}{pascal}Result::wire(&result)),\n                        }})"
                )
            } else {
                String::new()
            };
            start_arms.push_str(&format!(
                "                \"{id}\" => {{\n{read_request}                    \
invoker\n                        .start_{snake}({typed_arg}request.token){answer}\n                }}\n"
            ));
            cancel_arms.push_str(&format!(
                "                \"{id}\" => {{\n                    invoker.cancel_{snake}(cancel.token);\n                    None\n                }}\n"
            ));
            if let Some(schema) = invoke.result {
                records.push_str(&format!(
                    "impl {machine_name}{pascal}Result {{\n    \
/// The JSON `done.invoke.{id}` carries this record as.\n    \
pub fn wire(&self) -> ::sce_rust_runtime::SceString {{\n        {}\n    }}\n}}\n\n",
                    crate::forge::generator::rust_record_wire(&schema.fields, "self", "        ")
                ));
                ext_methods.push_str(&format!(
                    "    /// Complete `<invoke id=\"{id}\">`'s start `token` with its record —\n    \
/// `complete_host_invoke` with the record's JSON, so a stale or unknown\n    \
/// token is refused the same way (`false`).\n    \
fn complete_{snake}(&mut self, token: u64, result: {machine_name}{pascal}Result) -> bool;\n"
                ));
                ext_impls.push_str(&format!(
                    "    fn complete_{snake}(&mut self, token: u64, result: {machine_name}{pascal}Result) -> bool {{\n        \
self.complete_host_invoke(\"{invoke_type}\", \"{id}\", token, &result.wire())\n    }}\n"
                ));
            }
        }
        interfaces.push_str(&format!(
            "/// The host side of this document's typed `<invoke type=\"{invoke_type}\">`s\n\
/// (SCE Accepted Subset \u{a7}2.12). Register it with\n\
/// `register_{type_snake}_invoker`.\n\
pub trait {trait_name}: Send + 'static {{\n{methods}}}\n\n"
        ));
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as `error.execution`, not as running.
        let (fallback_generic, fallback_param, fallback_where, start_rest, cancel_rest) =
            if fallback {
                (
                    ", F",
                    ", fallback: F",
                    "\n        F: FnMut(::sce_rust_runtime::HostInvokeEvent) -> Option<::sce_rust_runtime::HostInvokeResponse>\n            + Send\n            + 'static,",
                    "fallback(::sce_rust_runtime::HostInvokeEvent::Start(request))",
                    "fallback(::sce_rust_runtime::HostInvokeEvent::Cancel(cancel))",
                )
            } else {
                // Every invoke of this type is typed, so no other id reaches
                // this handler.
                ("", "", "", "None", "None")
            };
        let fallback_doc = if fallback {
            "\n    /// `fallback` serves the invokes of this type the document does not type."
        } else {
            ""
        };
        let signature = format!(
            "fn register_{type_snake}_invoker<I{fallback_generic}>(&mut self, invoker: I{fallback_param})\n    \
where\n        I: {trait_name},{fallback_where}"
        );
        ext_methods.push_str(&format!(
            "    /// Register `invoker` as the handler for `type=\"{invoke_type}\"`.{fallback_doc}\n    \
{signature};\n"
        ));
        // A binding pattern is refused in a declaration without a body, so
        // only the implementation spells the parameters `mut`.
        let signature_body = signature
            .replace("invoker: I", "mut invoker: I")
            .replace("fallback: F)", "mut fallback: F)");
        ext_impls.push_str(&format!(
            "    {signature_body}\n    {{\n        \
self.register_invoker(\"{invoke_type}\", move |event| match event {{\n            \
::sce_rust_runtime::HostInvokeEvent::Start(request) => match request.invoke_id.as_str() {{\n\
{start_arms}                _ => {start_rest},\n            }},\n            \
::sce_rust_runtime::HostInvokeEvent::Cancel(cancel) => match cancel.invoke_id.as_str() {{\n\
{cancel_arms}                _ => {cancel_rest},\n            }},\n        }});\n    }}\n"
        ));
    }

    format!(
        "{records}{interfaces}/// Registration and typed completion for this document's typed host-run\n\
/// invokes. Bring into scope with `use …::{machine_name}HostInvokers;`.\n\
pub trait {machine_name}HostInvokers {{\n{ext_methods}}}\n\n\
impl{policy_generics_decl} {machine_name}HostInvokers for ::sce_rust_runtime::Engine<{machine_name}Policy{policy_generics_use}> {{\n{ext_impls}}}\n"
    )
}
