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

/// `invokes` grouped by the `type` they name, in name order — one generated
/// invoker interface and one registration per group.
fn by_type<'a, 'm>(
    invokes: &'a [TypedHostInvoke<'m>],
) -> BTreeMap<&'m str, Vec<&'a TypedHostInvoke<'m>>> {
    let mut groups: BTreeMap<&'m str, Vec<&'a TypedHostInvoke<'m>>> = BTreeMap::new();
    for invoke in invokes {
        groups.entry(invoke.invoke_type).or_default().push(invoke);
    }
    groups
}

/// An invoke's two records, each with the name suffix its generated type
/// takes and the sentence that type's documentation opens with.
fn records_of<'m>(
    invoke: &TypedHostInvoke<'m>,
) -> [(Option<&'m EventSchemaModel>, &'static str, &'static str); 2] {
    [
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
    ]
}

/// A request field's type as every runtime's descriptor names it: a scalar's
/// variant name, or a `bytes` field's capacity.
pub enum RequestFieldSpelling {
    Scalar(&'static str),
    Bytes(u32),
}

fn request_field_spelling(f: &ForgeField) -> RequestFieldSpelling {
    RequestFieldSpelling::Scalar(match &f.sce_type {
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
            return RequestFieldSpelling::Bytes(crate::forge::limits::resolve_bytes_max(f.max_size))
        }
        SceType::Enum(_) => {
            unreachable!("typed_invoke::validate refuses an enum-typed field in a host-run record")
        }
    })
}

/// What a start site holds each typed request field to: invoke id → field
/// name → the runtime's field-type descriptor, as `spell` writes it.
pub fn request_checks(
    model: &SCXMLModel,
    spell: impl Fn(RequestFieldSpelling) -> String,
) -> BTreeMap<String, BTreeMap<String, String>> {
    typed_host_invokes(model)
        .into_iter()
        .filter_map(|typed| {
            let request = typed.request?;
            let fields = request
                .fields
                .iter()
                .map(|f| (f.id.clone(), spell(request_field_spelling(f))))
                .collect();
            Some((typed.invoke_id.to_string(), fields))
        })
        .collect()
}

/// [`request_checks`] spelled as the Rust runtime's `RequestFieldType`
/// variant.
pub fn rust_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, |spelling| match spelling {
        RequestFieldSpelling::Scalar(variant) => variant.to_string(),
        RequestFieldSpelling::Bytes(cap) => format!("Bytes({cap})"),
    })
}

/// [`request_checks`] spelled as the Go runtime's `sce.RequestField*`.
pub fn go_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, |spelling| match spelling {
        RequestFieldSpelling::Scalar(variant) => format!("sce.RequestField{variant}"),
        RequestFieldSpelling::Bytes(cap) => format!("sce.RequestFieldBytes({cap})"),
    })
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
    for invoke in &typed {
        let pascal = filters::to_pascal_case(invoke.invoke_id.to_string());
        for (record, what, role) in records_of(invoke) {
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
    for (invoke_type, invokes) in &by_type(&typed) {
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

/// The Go host interface for `model`'s typed host-run invokes; empty when it
/// has none.
///
/// The shapes are the Rust ones in Go's idiom: record structs with exported
/// fields, an invoker interface per declared `type`, and — because `Engine`
/// is a foreign type — free functions for registration and completion.
pub fn render_go(model: &SCXMLModel, machine_name: &str) -> String {
    let typed = typed_host_invokes(model);
    if typed.is_empty() {
        return String::new();
    }
    let engine = format!("*sce.Engine[{machine_name}State, {machine_name}Event]");
    let exported = |id: &str| filters::to_pascal_case(id.to_string());
    let mut out = String::new();
    for invoke in &typed {
        let pascal = exported(invoke.invoke_id);
        for (record, what, role) in records_of(invoke) {
            let Some(schema) = record else { continue };
            let name = format!("{machine_name}{pascal}{what}");
            let fields: String = schema
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "\t{} {}\n",
                        exported(&f.id),
                        crate::forge::generator::go_type(&f.sce_type)
                    )
                })
                .collect();
            out.push_str(&format!(
                "// {name} is {role} `<invoke id=\"{id}\">`\n// with (SCE Accepted Subset \u{a7}2.12).\n\
type {name} struct {{\n{fields}}}\n\n",
                id = invoke.invoke_id,
            ));
            if what == "Result" {
                let items: String = schema
                    .fields
                    .iter()
                    .map(|f| {
                        let value = format!("r.{}", exported(&f.id));
                        let value = if matches!(f.sce_type, SceType::Bytes) {
                            format!("sce.BytesAsPayloadText({value})")
                        } else {
                            value
                        };
                        format!("\"{}\": {value}, ", f.id)
                    })
                    .collect();
                out.push_str(&format!(
                    "// Wire is the JSON `done.invoke.{id}` carries this record as.\n\
func (r {name}) Wire() string {{\n\treturn sce.PayloadJSON(map[string]any{{{items}}})\n}}\n\n",
                    id = invoke.invoke_id,
                    items = items.trim_end_matches(", "),
                ));
            }
        }
    }
    for (invoke_type, invokes) in &by_type(&typed) {
        let interface = format!(
            "{machine_name}{}Invoker",
            filters::to_pascal_case(invoke_type.to_string())
        );
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut methods = String::new();
        let mut start_cases = String::new();
        let mut cancel_cases = String::new();
        let mut completions = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let pascal = exported(id);
            let request_param = invoke
                .request
                .map(|_| format!("request {machine_name}{pascal}Request, "))
                .unwrap_or_default();
            let returns = if invoke.result.is_some() {
                format!("*{machine_name}{pascal}Result")
            } else {
                "*sce.HostInvokeResponse".to_string()
            };
            methods.push_str(&format!(
                "\t// Start{pascal} is \u{a7}scxml-6.4: begin `<invoke id=\"{id}\">`. token names\n\
\t// this start; a host that finishes later hands it back to\n\
\t// Complete{machine_name}{pascal} (or CompleteHostInvoke). Non-nil completes\n\
\t// the invocation now.\n\
\tStart{pascal}({request_param}token uint64) {returns}\n\
\t// Cancel{pascal} is \u{a7}scxml-6.4: `<invoke id=\"{id}\">`'s state exited while\n\
\t// the start token names was still running. Stop it.\n\
\tCancel{pascal}(token uint64)\n"
            ));
            let (read_request, typed_arg) = match invoke.request {
                Some(schema) => {
                    let fields: String = schema
                        .fields
                        .iter()
                        .map(|f| format!("{}: {}, ", exported(&f.id), go_request_reader(f)))
                        .collect();
                    (
                        format!(
                            "\t\t\t\ttyped := {machine_name}{pascal}Request{{{}}}\n",
                            fields.trim_end_matches(", ")
                        ),
                        "typed, ",
                    )
                }
                None => (String::new(), ""),
            };
            let start = if invoke.result.is_some() {
                format!(
                    "\t\t\t\tif result := invoker.Start{pascal}({typed_arg}start.Token); result != nil {{\n\
\t\t\t\t\tdata := result.Wire()\n\t\t\t\t\treturn &sce.HostInvokeResponse{{DoneData: &data}}\n\
\t\t\t\t}}\n\t\t\t\treturn nil\n"
                )
            } else {
                format!("\t\t\t\treturn invoker.Start{pascal}({typed_arg}start.Token)\n")
            };
            start_cases.push_str(&format!("\t\t\tcase \"{id}\":\n{read_request}{start}"));
            cancel_cases.push_str(&format!(
                "\t\t\tcase \"{id}\":\n\t\t\t\tinvoker.Cancel{pascal}(cancel.Token)\n\t\t\t\treturn nil\n"
            ));
            if invoke.result.is_some() {
                completions.push_str(&format!(
                    "// Complete{machine_name}{pascal} completes `<invoke id=\"{id}\">`'s start token\n\
// with its record — CompleteHostInvoke with the record's JSON, so a stale\n\
// or unknown token is refused the same way (false).\n\
func Complete{machine_name}{pascal}(e {engine}, token uint64, result {machine_name}{pascal}Result) bool {{\n\
\treturn e.CompleteHostInvoke(\"{invoke_type}\", \"{id}\", token, result.Wire())\n}}\n\n"
                ));
            }
        }
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as error.execution, not as running.
        let (fallback_param, fallback_doc, rest) = if fallback {
            (
                ", fallback sce.HostInvokeHandler",
                "\n// fallback serves the invokes of this type the document does not type.",
                "fallback(event)",
            )
        } else {
            // Every invoke of this type is typed, so no other id reaches this
            // handler.
            ("", "", "nil")
        };
        out.push_str(&format!(
            "// {interface} is the host side of this document's typed\n\
// `<invoke type=\"{invoke_type}\">`s (SCE Accepted Subset \u{a7}2.12). Register it with\n\
// Register{interface}.\n\
type {interface} interface {{\n{methods}}}\n\n\
// Register{interface} registers invoker as the handler for\n\
// `type=\"{invoke_type}\"`.{fallback_doc}\n\
func Register{interface}(e {engine}, invoker {interface}{fallback_param}) {{\n\
\te.RegisterInvoker(\"{invoke_type}\", func(event sce.HostInvokeEvent) *sce.HostInvokeResponse {{\n\
\t\tif start := event.Start; start != nil {{\n\t\t\tswitch start.InvokeID {{\n{start_cases}\t\t\t}}\n\t\t\treturn {rest}\n\t\t}}\n\
\t\tif cancel := event.Cancel; cancel != nil {{\n\t\t\tswitch cancel.InvokeID {{\n{cancel_cases}\t\t\t}}\n\t\t\treturn {rest}\n\t\t}}\n\
\t\treturn nil\n\t}})\n}}\n\n{completions}"
        ));
    }
    out
}

/// [`request_checks`] spelled as the Python runtime's `RequestFieldType`,
/// imported by the generated module as `_RequestFieldType`.
pub fn python_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, python_spelling)
}

/// The Python host interface for `model`'s typed host-run invokes; empty
/// when it has none.
///
/// The shapes are the Rust ones in Python's idiom: dataclass records, a
/// `Protocol` invoker per declared `type`, and module-level functions for
/// registration and completion. The names the generated module imports for
/// it (`dataclass`, `Protocol`, `_json`, `_request_field`,
/// `_RequestFieldType`) are gated in its header on this being non-empty.
pub fn render_python(model: &SCXMLModel) -> String {
    let typed = typed_host_invokes(model);
    if typed.is_empty() {
        return String::new();
    }
    let l = |ty: &SceType| -> &'static str {
        match ty {
            SceType::Float32 | SceType::Float64 => "float",
            SceType::Bool => "bool",
            SceType::String => "str",
            SceType::Bytes => "bytes",
            SceType::Enum(_) => unreachable!(
                "typed_invoke::validate refuses an enum-typed field in a host-run record"
            ),
            _ => "int",
        }
    };
    let mut out = String::new();
    for invoke in &typed {
        let pascal = filters::to_pascal_case(invoke.invoke_id.to_string());
        for (record, what, role) in records_of(invoke) {
            let Some(schema) = record else { continue };
            let fields: String = schema
                .fields
                .iter()
                .map(|f| format!("    {}: {}\n", f.id, l(&f.sce_type)))
                .collect();
            let wire = if what == "Result" {
                let items: Vec<String> = schema
                    .fields
                    .iter()
                    .map(|f| {
                        // JSON has no byte string, so a `bytes` field rides
                        // as its byte-exact Latin-1 text.
                        if matches!(f.sce_type, SceType::Bytes) {
                            format!("\"{0}\": self.{0}.decode(\"latin-1\")", f.id)
                        } else {
                            format!("\"{0}\": self.{0}", f.id)
                        }
                    })
                    .collect();
                format!(
                    "\n    def wire(self) -> str:\n        \
\"\"\"The JSON `done.invoke.{id}` carries this record as.\"\"\"\n        \
return _json.dumps({{{}}}, separators=(\",\", \":\"))\n",
                    items.join(", "),
                    id = invoke.invoke_id,
                )
            } else {
                String::new()
            };
            out.push_str(&format!(
                "@dataclass\nclass {pascal}{what}:\n    \
\"\"\"{role} `<invoke id=\"{id}\">` with (SCE Accepted\n    \
Subset \u{a7}2.12).\"\"\"\n\n{fields}{wire}\n\n",
                id = invoke.invoke_id,
            ));
        }
    }
    for (invoke_type, invokes) in &by_type(&typed) {
        let type_pascal = filters::to_pascal_case(invoke_type.to_string());
        let type_snake = filters::to_snake_case(invoke_type.to_string());
        let protocol = format!("{type_pascal}Invoker");
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut methods = String::new();
        let mut start_arms = String::new();
        let mut cancel_arms = String::new();
        let mut completions = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let pascal = filters::to_pascal_case(id.to_string());
            let snake = filters::to_snake_case(id.to_string());
            let request_param = invoke
                .request
                .map(|_| format!("request: {pascal}Request, "))
                .unwrap_or_default();
            let returns = if invoke.result.is_some() {
                format!("Optional[{pascal}Result]")
            } else {
                "Optional[_HostInvokeResponse]".to_string()
            };
            methods.push_str(&format!(
                "    def start_{snake}(self, {request_param}token: int) -> {returns}:\n        \
\"\"\"W3C SCXML 6.4: begin `<invoke id=\"{id}\">`. `token` names this\n        \
start; a host that finishes later hands it back to `complete_{snake}`\n        \
(or `complete_host_invoke`). A value completes the invocation now.\"\"\"\n        \
...\n\n    \
def cancel_{snake}(self, token: int) -> None:\n        \
\"\"\"W3C SCXML 6.4: `<invoke id=\"{id}\">`'s state exited while the start\n        \
`token` names was still running. Stop it.\"\"\"\n        \
...\n\n"
            ));
            let (read_request, typed_arg) = match invoke.request {
                Some(schema) => {
                    let fields: Vec<String> = schema
                        .fields
                        .iter()
                        .map(|f| {
                            format!(
                                "                    {0}=_request_field(start, \"{0}\", {1}),\n",
                                f.id,
                                python_spelling(request_field_spelling(f))
                            )
                        })
                        .collect();
                    (
                        format!(
                            "                typed = {pascal}Request(\n{}                )\n",
                            fields.concat()
                        ),
                        "typed, ",
                    )
                }
                None => (String::new(), ""),
            };
            let start = if invoke.result.is_some() {
                format!(
                    "                result = invoker.start_{snake}({typed_arg}start.token)\n                \
return None if result is None else _HostInvokeResponse(done_data=result.wire())\n"
                )
            } else {
                format!("                return invoker.start_{snake}({typed_arg}start.token)\n")
            };
            start_arms.push_str(&format!(
                "            if start.invoke_id == \"{id}\":\n{read_request}{start}"
            ));
            cancel_arms.push_str(&format!(
                "            if cancel.invoke_id == \"{id}\":\n                \
invoker.cancel_{snake}(cancel.token)\n                return None\n"
            ));
            if invoke.result.is_some() {
                completions.push_str(&format!(
                    "def complete_{snake}(engine, token: int, result: {pascal}Result) -> bool:\n    \
\"\"\"Complete `<invoke id=\"{id}\">`'s start `token` with its record —\n    \
`complete_host_invoke` with the record's JSON, so a stale or unknown token\n    \
is refused the same way (False).\"\"\"\n    \
return engine.complete_host_invoke(\"{invoke_type}\", \"{id}\", token, result.wire())\n\n\n"
                ));
            }
        }
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as error.execution, not as running.
        let (fallback_param, fallback_doc, rest) = if fallback {
            (
                ", fallback",
                "\n\n    `fallback` serves the invokes of this type the document does not type.",
                "fallback(event)",
            )
        } else {
            // Every invoke of this type is typed, so no other id reaches this
            // handler.
            ("", "", "None")
        };
        out.push_str(&format!(
            "class {protocol}(Protocol):\n    \
\"\"\"The host side of this document's typed `<invoke type=\"{invoke_type}\">`s\n    \
(SCE Accepted Subset \u{a7}2.12). Register it with\n    \
`register_{type_snake}_invoker`.\"\"\"\n\n{methods}\n\
def register_{type_snake}_invoker(engine, invoker: {protocol}{fallback_param}) -> None:\n    \
\"\"\"Register `invoker` as the handler for `type=\"{invoke_type}\"`.{fallback_doc}\"\"\"\n\n    \
def handler(event):\n        \
start = event.start\n        \
if start is not None:\n{start_arms}            \
return {rest}\n        \
cancel = event.cancel\n        \
if cancel is not None:\n{cancel_arms}            \
return {rest}\n        \
return None\n\n    \
engine.register_invoker(\"{invoke_type}\", handler)\n\n\n{completions}"
        ));
    }
    out
}

/// [`request_checks`] spelled as the Kotlin runtime's
/// `TypedRequest.FieldType`.
pub fn kotlin_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, |spelling| match spelling {
        RequestFieldSpelling::Scalar(variant) => {
            format!("TypedRequest.FieldType.{}", variant.to_ascii_uppercase())
        }
        RequestFieldSpelling::Bytes(cap) => format!("TypedRequest.FieldType.bytes({cap})"),
    })
}

/// The Kotlin host interface: what goes beside the machine class (`defs` —
/// the records and one invoker interface per declared `type`) and what goes
/// inside it (`members` — registration and typed completion, which reach the
/// engine's own `registerInvoker` / `completeHostInvoke`). Both empty when
/// the document has no typed host-run invoke.
pub struct KotlinHostInvokerInterface {
    pub defs: String,
    pub members: String,
}

/// The Kotlin host interface for `model`'s typed host-run invokes.
pub fn render_kotlin(model: &SCXMLModel, machine_name: &str) -> KotlinHostInvokerInterface {
    let mut defs = String::new();
    let mut members = String::new();
    let typed = typed_host_invokes(model);
    if typed.is_empty() {
        return KotlinHostInvokerInterface { defs, members };
    }
    for invoke in &typed {
        let pascal = filters::to_pascal_case(invoke.invoke_id.to_string());
        for (record, what, role) in records_of(invoke) {
            let Some(schema) = record else { continue };
            let params: Vec<String> = schema
                .fields
                .iter()
                .map(|f| {
                    format!(
                        "    val {}: {},\n",
                        f.id,
                        crate::forge::generator::kotlin_type(&f.sce_type)
                    )
                })
                .collect();
            let body = if what == "Result" {
                let items: Vec<String> = schema
                    .fields
                    .iter()
                    .map(|f| format!("\"{0}\" to {0}", f.id))
                    .collect();
                format!(
                    " {{\n    /** The JSON `done.invoke.{id}` carries this record as. */\n    \
fun wire(): String = EventPayload.encode(mapOf({}))\n}}",
                    items.join(", "),
                    id = invoke.invoke_id,
                )
            } else {
                String::new()
            };
            defs.push_str(&format!(
                "/** {role} `<invoke id=\"{id}\">` with (SCE Accepted Subset \u{a7}2.12). */\n\
data class {machine_name}{pascal}{what}(\n{}){body}\n\n",
                params.concat(),
                id = invoke.invoke_id,
            ));
        }
    }
    for (invoke_type, invokes) in &by_type(&typed) {
        let type_pascal = filters::to_pascal_case(invoke_type.to_string());
        let interface = format!("{machine_name}{type_pascal}Invoker");
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut methods = String::new();
        let mut start_arms = String::new();
        let mut cancel_arms = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let pascal = filters::to_pascal_case(id.to_string());
            let request_param = invoke
                .request
                .map(|_| format!("request: {machine_name}{pascal}Request, "))
                .unwrap_or_default();
            let returns = if invoke.result.is_some() {
                format!("{machine_name}{pascal}Result?")
            } else {
                "StateMachineEngine.HostInvokeResponse?".to_string()
            };
            methods.push_str(&format!(
                "    /**\n     * \u{a7}scxml-6.4: begin `<invoke id=\"{id}\">`. [token] names this start;\n     \
* a host that finishes later hands it back to `complete{pascal}` (or\n     \
* `completeHostInvoke`). Non-null completes the invocation now.\n     */\n    \
fun start{pascal}({request_param}token: Long): {returns}\n\n    \
/** \u{a7}scxml-6.4: `<invoke id=\"{id}\">`'s state exited while the start [token] names was still running. */\n    \
fun cancel{pascal}(token: Long)\n"
            ));
            let (read_request, typed_arg) = match invoke.request {
                Some(schema) => {
                    let fields: Vec<String> = schema
                        .fields
                        .iter()
                        .map(|f| {
                            format!(
                                "                            {0} = TypedRequest.{1}(start.params, start.invokeId, \"{0}\"),\n",
                                f.id,
                                kotlin_request_reader(&f.sce_type)
                            )
                        })
                        .collect();
                    (
                        format!(
                            "                        val typed = {machine_name}{pascal}Request(\n{}                        )\n",
                            fields.concat()
                        ),
                        "typed, ",
                    )
                }
                None => (String::new(), ""),
            };
            let answer = if invoke.result.is_some() {
                "?.let { HostInvokeResponse(doneData = it.wire()) }"
            } else {
                ""
            };
            let start_method = format!("start{pascal}");
            start_arms.push_str(&format!(
                "                    \"{id}\" -> {{\n{read_request}                        \
invoker.{start_method}({typed_arg}start.token){answer}\n                    }}\n"
            ));
            cancel_arms.push_str(&format!(
                "                    \"{id}\" -> {{\n                        invoker.cancel{pascal}(cancel.token)\n                        null\n                    }}\n"
            ));
            if invoke.result.is_some() {
                members.push_str(&format!(
                    "\n    /**\n     * Complete `<invoke id=\"{id}\">`'s start [token] with its record —\n     \
* `completeHostInvoke` with the record's JSON, so a stale or unknown token is\n     \
* refused the same way (`false`).\n     */\n    \
fun complete{pascal}(token: Long, result: {machine_name}{pascal}Result): Boolean =\n        \
completeHostInvoke(\"{invoke_type}\", \"{id}\", token, result.wire())\n"
                ));
            }
        }
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as error.execution, not as running.
        let (fallback_param, fallback_doc, rest) = if fallback {
            (
                ", fallback: (HostInvokeEvent) -> HostInvokeResponse?",
                "\n     * [fallback] serves the invokes of this type the document does not type.",
                "fallback(event)",
            )
        } else {
            // Every invoke of this type is typed, so no other id reaches this
            // handler.
            ("", "", "null")
        };
        defs.push_str(&format!(
            "/**\n * The host side of this document's typed `<invoke type=\"{invoke_type}\">`s\n \
* (SCE Accepted Subset \u{a7}2.12). Register it with `register{type_pascal}Invoker`.\n */\n\
interface {interface} {{\n{methods}}}\n\n"
        ));
        members.push_str(&format!(
            "\n    /**\n     * Register [invoker] as the handler for `type=\"{invoke_type}\"`.{fallback_doc}\n     */\n    \
fun register{type_pascal}Invoker(invoker: {interface}{fallback_param}) {{\n        \
registerInvoker(\"{invoke_type}\") {{ event ->\n            \
val start = event.start\n            \
val cancel = event.cancel\n            \
if (start != null) {{\n                \
when (start.invokeId) {{\n{start_arms}                    else -> {rest}\n                }}\n            \
}} else if (cancel != null) {{\n                \
when (cancel.invokeId) {{\n{cancel_arms}                    else -> {rest}\n                }}\n            \
}} else {{\n                null\n            }}\n        }}\n    }}\n"
        ));
    }
    KotlinHostInvokerInterface { defs, members }
}

/// [`request_checks`] spelled as the C++ runtime's `RequestFieldType`.
pub fn cpp_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, |spelling| match spelling {
        RequestFieldSpelling::Scalar(variant) => {
            format!("::SCE::RequestFieldType{{::SCE::RequestFieldKind::{variant}}}")
        }
        RequestFieldSpelling::Bytes(cap) => {
            format!("::SCE::RequestFieldType{{::SCE::RequestFieldKind::Bytes, {cap}}}")
        }
    })
}

/// The C++ host interface: what goes before the machine class (`defs` — the
/// records and one abstract invoker per declared `type`) and what goes inside
/// it (`members` — registration and typed completion, which reach the
/// engine's own `registerInvoker` / `completeHostInvoke`). Both empty when the
/// document has no typed host-run invoke.
pub struct CppHostInvokerInterface {
    pub defs: String,
    pub members: String,
}

/// The C++ host interface for `model`'s typed host-run invokes.
///
/// Header-only like the payload channel beside it: the records' JSON is
/// spelled by `EventPayloadLift.h`, and the adapter's reading by
/// `core/TypedHostInvokeRequest.h`.
pub fn render_cpp(model: &SCXMLModel) -> CppHostInvokerInterface {
    let mut defs = String::new();
    let mut members = String::new();
    let typed = typed_host_invokes(model);
    if typed.is_empty() {
        return CppHostInvokerInterface { defs, members };
    }
    let cpp_type = |f: &ForgeField| crate::forge::generator::cpp_type(&f.sce_type);
    for invoke in &typed {
        let pascal = filters::to_pascal_case(invoke.invoke_id.to_string());
        for (record, what, role) in records_of(invoke) {
            let Some(schema) = record else { continue };
            let fields: String = schema
                .fields
                .iter()
                .map(|f| format!("    {} {}{{}};\n", cpp_type(f), f.id))
                .collect();
            let wire = if what == "Result" {
                let items: Vec<String> = schema
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "::SCE::Common::EventPayloadFields::field(\"{0}\", {0})",
                            f.id
                        )
                    })
                    .collect();
                format!(
                    "\n    /// The JSON `done.invoke.{id}` carries this record as.\n    \
std::string wire() const {{\n        \
return ::SCE::Common::EventPayloadFields::wire({{{}}});\n    }}\n",
                    items.join(", "),
                    id = invoke.invoke_id,
                )
            } else {
                String::new()
            };
            defs.push_str(&format!(
                "/// {role} `<invoke id=\"{id}\">` with (SCE Accepted Subset \u{a7}2.12).\n\
struct {pascal}{what} {{\n{fields}{wire}\n    bool operator==(const {pascal}{what} &) const = default;\n}};\n\n",
                id = invoke.invoke_id,
            ));
        }
    }
    for (invoke_type, invokes) in &by_type(&typed) {
        let type_pascal = filters::to_pascal_case(invoke_type.to_string());
        let interface = format!("{type_pascal}Invoker");
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut methods = String::new();
        let mut start_arms = String::new();
        let mut cancel_arms = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let pascal = filters::to_pascal_case(id.to_string());
            let request_param = invoke
                .request
                .map(|_| format!("const {pascal}Request &request, "))
                .unwrap_or_default();
            let returns = if invoke.result.is_some() {
                format!("std::optional<{pascal}Result>")
            } else {
                "std::optional<::SCE::HostInvokeResponse>".to_string()
            };
            methods.push_str(&format!(
                "    /// \u{a7}scxml-6.4: begin `<invoke id=\"{id}\">`. `token` names this start; a\n    \
/// host that finishes later hands it back to `complete{pascal}` (or\n    \
/// `completeHostInvoke`). A value completes the invocation now.\n    \
virtual {returns} start{pascal}({request_param}uint64_t token) = 0;\n    \
/// \u{a7}scxml-6.4: `<invoke id=\"{id}\">`'s state exited while the start `token`\n    \
/// names was still running. Stop it.\n    \
virtual void cancel{pascal}(uint64_t token) = 0;\n"
            ));
            let (read_request, typed_arg) = match invoke.request {
                Some(schema) => {
                    let fields: String = schema
                        .fields
                        .iter()
                        .map(|f| {
                            format!(
                                "                    typed.{0} = ::SCE::requestField<{1}>(start, \"{0}\");\n",
                                f.id,
                                cpp_type(f)
                            )
                        })
                        .collect();
                    (
                        format!("                    {pascal}Request typed;\n{fields}"),
                        "typed, ",
                    )
                }
                None => (String::new(), ""),
            };
            let start = if invoke.result.is_some() {
                format!(
                    "                    const auto result = invoker->start{pascal}({typed_arg}start.token);\n                    \
if (!result) {{\n                        return std::nullopt;\n                    }}\n                    \
return ::SCE::HostInvokeResponse{{result->wire()}};\n"
                )
            } else {
                format!(
                    "                    return invoker->start{pascal}({typed_arg}start.token);\n"
                )
            };
            start_arms.push_str(&format!(
                "                if (start.invokeId == \"{id}\") {{\n{read_request}{start}                }}\n"
            ));
            cancel_arms.push_str(&format!(
                "                if (cancel.invokeId == \"{id}\") {{\n                    \
invoker->cancel{pascal}(cancel.token);\n                    return std::nullopt;\n                }}\n"
            ));
            if invoke.result.is_some() {
                members.push_str(&format!(
                    "    /// Complete `<invoke id=\"{id}\">`'s start `token` with its record —\n    \
/// `completeHostInvoke` with the record's JSON, so a stale or unknown token\n    \
/// is refused the same way (`false`).\n    \
bool complete{pascal}(uint64_t token, const {pascal}Result &result) {{\n        \
return this->completeHostInvoke(\"{invoke_type}\", \"{id}\", token, result.wire());\n    }}\n\n"
                ));
            }
        }
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as error.execution, not as running.
        let (fallback_param, fallback_doc, rest) = if fallback {
            (
                ", ::SCE::HostInvokeHandler fallback",
                "\n    /// `fallback` serves the invokes of this type the document does not type.",
                "fallback(event)",
            )
        } else {
            // Every invoke of this type is typed, so no other id reaches this
            // handler.
            ("", "", "std::nullopt")
        };
        let capture = if fallback {
            "invoker, fallback = std::move(fallback)"
        } else {
            "invoker"
        };
        defs.push_str(&format!(
            "/// The host side of this document's typed `<invoke type=\"{invoke_type}\">`s\n\
/// (SCE Accepted Subset \u{a7}2.12). Register it with `register{type_pascal}Invoker`.\n\
class {interface} {{\npublic:\n    virtual ~{interface}() = default;\n{methods}}};\n\n"
        ));
        members.push_str(&format!(
            "    /// Register `invoker` as the handler for `type=\"{invoke_type}\"`; the\n    \
/// engine shares its ownership.{fallback_doc}\n    \
void register{type_pascal}Invoker(std::shared_ptr<{interface}> invoker{fallback_param}) {{\n        \
this->registerInvoker(\"{invoke_type}\", [{capture}](const ::SCE::HostInvokeEvent &event)\n                                      \
-> std::optional<::SCE::HostInvokeResponse> {{\n            \
if (event.start) {{\n                const auto &start = *event.start;\n{start_arms}                return {rest};\n            }}\n            \
if (event.cancel) {{\n                const auto &cancel = *event.cancel;\n{cancel_arms}                return {rest};\n            }}\n            \
return std::nullopt;\n        }});\n    }}\n\n"
        ));
    }
    CppHostInvokerInterface { defs, members }
}

/// [`request_checks`] spelled as the C11 runtime's `sce_request_field_type_t`.
pub fn c11_request_checks(model: &SCXMLModel) -> BTreeMap<String, BTreeMap<String, String>> {
    request_checks(model, |spelling| match spelling {
        RequestFieldSpelling::Scalar(variant) => format!(
            "(sce_request_field_type_t){{SCE_REQUEST_FIELD_{}, 0u}}",
            variant.to_ascii_uppercase()
        ),
        RequestFieldSpelling::Bytes(cap) => {
            format!("(sce_request_field_type_t){{SCE_REQUEST_FIELD_BYTES, {cap}u}}")
        }
    })
}

/// The C11 host interface: the header's declarations (`decls` — the records,
/// one invoker table per declared `type`, registration and typed completion)
/// and the source's definitions (`defs` — the adapter, and the two functions).
/// Both empty when the document has no typed host-run invoke.
pub struct C11HostInvokerInterface {
    pub decls: String,
    pub defs: String,
}

/// The C11 host interface for `model`'s typed host-run invokes. `sm` is the
/// machine's symbol stem (`<prefix><name>`).
///
/// C has no closure, so the invoker is a table of function pointers with the
/// `user_data` handed back to each, as the runtime's own registry is, and the
/// adapter is registered with that table as its `user_data`. The table must
/// outlive the registry it is registered in.
pub fn render_c11(model: &SCXMLModel, sm: &str) -> C11HostInvokerInterface {
    let mut decls = String::new();
    let mut defs = String::new();
    let typed = typed_host_invokes(model);
    if typed.is_empty() {
        return C11HostInvokerInterface { decls, defs };
    }
    for invoke in &typed {
        let snake = filters::to_snake_case(invoke.invoke_id.to_string());
        for (record, what, role) in records_of(invoke) {
            let Some(schema) = record else { continue };
            let what_snake = what.to_ascii_lowercase();
            let name = format!("{sm}_{snake}_{what_snake}_t");
            let fields: String = schema
                .fields
                .iter()
                .map(crate::forge::generator::c11_record_field_decl)
                .collect();
            decls.push_str(&format!(
                "/* {role} `<invoke id=\"{id}\">` with (SCE Accepted Subset \u{a7}2.12).\n   \
A text field of a request is borrowed from it, valid for the start call. */\n\
typedef struct {sm}_{snake}_{what_snake}_s {{\n{fields}}} {name};\n\n",
                id = invoke.invoke_id,
            ));
            if what == "Result" {
                let crate::forge::generator::C11RecordWire {
                    locals,
                    format,
                    args,
                } = crate::forge::generator::c11_record_wire(&schema.fields, "result");
                defs.push_str(&format!(
                    "/* The JSON `done.invoke.{id}` carries `result` as, into `out` of `cap` bytes;\n   \
false when it does not fit. */\n\
static bool {sm}_{snake}_result_wire(const {name} *result, char *out, size_t cap) {{\n\
{locals}    const int _wire_len = snprintf(out, cap, \"{{{format}}}\"{args});\n    \
return _wire_len >= 0 && (size_t)_wire_len < cap;\n}}\n\n",
                    id = invoke.invoke_id,
                ));
            }
        }
    }
    for (invoke_type, invokes) in &by_type(&typed) {
        let type_snake = filters::to_snake_case(invoke_type.to_string());
        let table = format!("{sm}_{type_snake}_invoker_t");
        let fallback = has_untyped_invoke_of(model, invoke_type);
        let mut members = String::new();
        let mut start_arms = String::new();
        let mut cancel_arms = String::new();
        for invoke in invokes {
            let id = invoke.invoke_id;
            let snake = filters::to_snake_case(id.to_string());
            let request_param = invoke
                .request
                .map(|_| format!("const {sm}_{snake}_request_t *request, "))
                .unwrap_or_default();
            let (returns, answer_param, answer_doc) = if invoke.result.is_some() {
                (
                    "bool",
                    format!(", {sm}_{snake}_result_t *result"),
                    "Return true with `*result` filled to complete the invocation now.",
                )
            } else {
                (
                    "void",
                    ", sce_host_invoke_response_t *out".to_string(),
                    "Fill `*out` to complete the invocation now.",
                )
            };
            members.push_str(&format!(
                "    /* \u{a7}scxml-6.4: begin `<invoke id=\"{id}\">`. `token` names this start; a host\n       \
that finishes later hands it back to `{sm}_complete_{snake}`. {answer_doc} */\n    \
{returns} (*start_{snake})(void *user_data, {request_param}uint64_t token{answer_param});\n    \
/* \u{a7}scxml-6.4: `<invoke id=\"{id}\">`'s state exited while the start `token` names\n       \
was still running. Stop it. */\n    \
void (*cancel_{snake})(void *user_data, uint64_t token);\n"
            ));
            let read_request = match invoke.request {
                Some(schema) => {
                    let reads: Vec<String> = schema
                        .fields
                        .iter()
                        .map(|f| c11_request_read(&f.sce_type, &f.id))
                        .collect();
                    format!(
                        "            {sm}_{snake}_request_t request;\n            \
memset(&request, 0, sizeof(request));\n            \
/* The start site checked every field, so a reading that fails is a broken\n               \
promise between two halves of generated code; stop rather than hand the\n               \
host a record the document never sent. */\n            \
if (!({})) {{\n                abort();\n            }}\n",
                        reads.join(" &&\n                  ")
                    )
                }
                None => String::new(),
            };
            let request_arg = if invoke.request.is_some() {
                "&request, "
            } else {
                ""
            };
            let start = if invoke.result.is_some() {
                format!(
                    "            {sm}_{snake}_result_t result;\n            \
memset(&result, 0, sizeof(result));\n            \
if (invoker->start_{snake}(invoker->user_data, {request_arg}event->token, &result)) {{\n                \
out->has_done_data = {sm}_{snake}_result_wire(&result, out->done_data, sizeof(out->done_data));\n            \
}}\n"
                )
            } else {
                format!(
                    "            invoker->start_{snake}(invoker->user_data, {request_arg}event->token, out);\n"
                )
            };
            start_arms.push_str(&format!(
                "        if (strcmp(event->invoke_id, \"{id}\") == 0) {{\n{read_request}{start}            return;\n        }}\n"
            ));
            cancel_arms.push_str(&format!(
                "        if (strcmp(event->invoke_id, \"{id}\") == 0) {{\n            \
invoker->cancel_{snake}(invoker->user_data, event->token);\n            return;\n        }}\n"
            ));
            if invoke.result.is_some() {
                decls.push_str(&format!(
                    "/* Complete `<invoke id=\"{id}\">`'s start `token` with its record —\n   \
`_complete_host_invoke` with the record's JSON, so a stale or unknown token is\n   \
refused the same way (false), as is a record too long for SCE_MAX_DATA_LEN. */\n\
bool {sm}_complete_{snake}({sm}_t *sm, uint64_t token, const {sm}_{snake}_result_t *result);\n\n"
                ));
                defs.push_str(&format!(
                    "bool {sm}_complete_{snake}({sm}_t *sm, uint64_t token, const {sm}_{snake}_result_t *result) {{\n    \
char data[SCE_MAX_DATA_LEN];\n    \
if (!{sm}_{snake}_result_wire(result, data, sizeof(data))) {{\n        return false;\n    }}\n    \
return {sm}_complete_host_invoke(sm, \"{invoke_type}\", \"{id}\", token, data);\n}}\n\n"
                ));
            }
        }
        // An invoke of this type the document does not type still needs a
        // handler; without one the adapter could only drop it, and a start
        // nobody performed must read as error.execution, not as running.
        let (fallback_members, rest) = if fallback {
            (
                "    /* Serves the invokes of this type the document does not type. */\n    \
sce_host_invoke_handler_fn fallback;\n    void *fallback_user_data;\n",
                "    invoker->fallback(invoker->fallback_user_data, event, out);\n",
            )
        } else {
            // Every invoke of this type is typed, so no other id reaches this
            // handler.
            ("", "")
        };
        decls.push_str(&format!(
            "/* The host side of this document's typed `<invoke type=\"{invoke_type}\">`s (SCE\n   \
Accepted Subset \u{a7}2.12). Register it with `{sm}_register_{type_snake}_invoker`. */\n\
typedef struct {sm}_{type_snake}_invoker_s {{\n    \
/* Handed back unchanged to every function below. */\n    void *user_data;\n{members}{fallback_members}}} {table};\n\n\
/* Register `invoker` as the handler for `type=\"{invoke_type}\"` in `registry`. The\n   \
table is borrowed, and must outlive the registry. False when the registry\n   \
refuses it. */\n\
bool {sm}_register_{type_snake}_invoker(sce_host_invoker_registry_t *registry, const {table} *invoker);\n\n"
        ));
        defs.push_str(&format!(
            "static void {sm}_{type_snake}_invoker_adapter(void *user_data, const sce_host_invoke_event_t *event,\n        \
sce_host_invoke_response_t *out) {{\n    \
const {table} *invoker = (const {table} *)user_data;\n    \
if (event->phase == SCE_HOST_INVOKE_START) {{\n{start_arms}    }} else {{\n{cancel_arms}    }}\n{rest}}}\n\n\
bool {sm}_register_{type_snake}_invoker(sce_host_invoker_registry_t *registry, const {table} *invoker) {{\n    \
if (invoker == NULL) {{\n        return false;\n    }}\n    \
return sce_host_invoker_register(registry, \"{invoke_type}\", {sm}_{type_snake}_invoker_adapter, (void *)invoker);\n}}\n\n"
        ));
    }
    C11HostInvokerInterface { decls, defs }
}

/// The `sce_request_read_*` call reading field `id` of type `ty` into the
/// adapter's local `request`.
fn c11_request_read(ty: &SceType, id: &str) -> String {
    let suffix = match ty {
        SceType::Uint8 => "u8",
        SceType::Uint16 => "u16",
        SceType::Uint32 => "u32",
        SceType::Uint64 => "u64",
        SceType::Int8 => "i8",
        SceType::Int16 => "i16",
        SceType::Int32 => "i32",
        SceType::Int64 => "i64",
        SceType::Float32 => "f32",
        SceType::Float64 => "f64",
        SceType::Bool => "bool",
        SceType::String => "text",
        SceType::Bytes => {
            return format!(
                "sce_request_read_bytes(event, \"{id}\", request.{id}, sizeof(request.{id}), &request.{id}_len)"
            )
        }
        SceType::Enum(_) => {
            unreachable!("typed_invoke::validate refuses an enum-typed field in a host-run record")
        }
    };
    format!("sce_request_read_{suffix}(event, \"{id}\", &request.{id})")
}

/// The `TypedRequest` reader for a field of type `ty`.
fn kotlin_request_reader(ty: &SceType) -> &'static str {
    match ty {
        SceType::Uint8 => "uint8",
        SceType::Uint16 => "uint16",
        SceType::Uint32 => "uint32",
        SceType::Uint64 => "uint64",
        SceType::Int8 => "int8",
        SceType::Int16 => "int16",
        SceType::Int32 => "int32",
        SceType::Int64 => "int64",
        SceType::Float32 => "float32",
        SceType::Float64 => "float64",
        SceType::Bool => "boolean",
        SceType::String => "string",
        SceType::Bytes => "bytes",
        SceType::Enum(_) => {
            unreachable!("typed_invoke::validate refuses an enum-typed field in a host-run record")
        }
    }
}

fn python_spelling(spelling: RequestFieldSpelling) -> String {
    match spelling {
        RequestFieldSpelling::Scalar(variant) => {
            format!("_RequestFieldType(\"{}\")", variant.to_ascii_lowercase())
        }
        RequestFieldSpelling::Bytes(cap) => format!("_RequestFieldType(\"bytes\", {cap})"),
    }
}

/// The Go expression reading `f` out of the checked request `start`.
fn go_request_reader(f: &ForgeField) -> String {
    let id = &f.id;
    let ty = crate::forge::generator::go_type(&f.sce_type);
    match &f.sce_type {
        SceType::Uint8 | SceType::Uint16 | SceType::Uint32 | SceType::Uint64 => {
            format!("sce.RequestUnsigned[{ty}](*start, \"{id}\")")
        }
        SceType::Int8 | SceType::Int16 | SceType::Int32 | SceType::Int64 => {
            format!("sce.RequestSigned[{ty}](*start, \"{id}\")")
        }
        SceType::Float32 | SceType::Float64 => format!("sce.RequestFloat[{ty}](*start, \"{id}\")"),
        SceType::Bool => format!("sce.RequestBool(*start, \"{id}\")"),
        SceType::String => format!("sce.RequestString(*start, \"{id}\")"),
        SceType::Bytes => format!("sce.RequestBytes(*start, \"{id}\")"),
        SceType::Enum(_) => {
            unreachable!("typed_invoke::validate refuses an enum-typed field in a host-run record")
        }
    }
}
