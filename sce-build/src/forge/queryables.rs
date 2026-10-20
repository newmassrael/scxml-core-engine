// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Rendering `machines.<m>.queryables` as Rust (SCE Protocol-Synthesis RFC
//! §synth-5-R, Queryables a machine serves; ADR 0041).
//!
//! The deploy block is read once, by [`crate::mesh::queryable`]; this module
//! turns the resolved legs into the context `queryables.rs.jinja2` renders:
//! the `Leg` enum and the matcher over declared keys, one document type per
//! object a reply declares, and for every reply the function that writes it
//! into a buffer the caller owns. All the Rust that is written out is in the
//! template. What is built here is names and types: the flat list of types and
//! emission functions the template walks, since a template that recursed over
//! a document tree would carry the same logic in a second language.

use std::path::Path;

use serde::Serialize;

use crate::filters;
use crate::forge::error::{ForgeError, GenerateError};
use crate::generator::{self, Language};
use crate::mesh::queryable::{pascal_case, DocKind, DocNode, DocObject, LegReply, QueryableLeg};
use crate::rust_derive_policy::RustDeriveCategory;

/// A value of a document, as the template needs to write it.
#[derive(Debug, Clone, Serialize)]
struct NodeCtx {
    /// `bool`, `uint`, `int`, `string`, `object` or `array`.
    kind: &'static str,
    nullable: bool,
    /// The Rust type of a struct field holding the value.
    field_type: String,
    /// The same type as a function parameter, with lifetimes elided.
    param_type: String,
    max_len: u32,
    max_items: u32,
    /// Where the value is in the document, for the error that names it.
    path: String,
    /// The function that writes an object or an array.
    fn_name: String,
    /// The expression that is the value, when it is not nullable.
    expr: String,
    /// The expression that is the value once a nullable one is known to be
    /// present, bound to `x`.
    some_expr: String,
}

#[derive(Debug, Clone, Serialize)]
struct FieldCtx {
    name: String,
    node: NodeCtx,
}

/// One object of a document: its type and the function that writes it.
#[derive(Debug, Clone, Serialize)]
struct ObjectCtx {
    struct_name: String,
    fn_name: String,
    /// `<'a>`, or nothing when no field of the object borrows.
    lifetime: &'static str,
    /// `<'_>`, or nothing.
    param_lifetime: &'static str,
    fields: Vec<FieldCtx>,
}

/// One array of a document: the function that writes it.
#[derive(Debug, Clone, Serialize)]
struct ArrayCtx {
    fn_name: String,
    elem_param_type: String,
    max_items: u32,
    path: String,
    item: NodeCtx,
}

#[derive(Debug, Clone, Serialize)]
struct LegCtx {
    name: String,
    pascal: String,
    snake: String,
    upper: String,
    key_lit: String,
    encoding_lit: String,
    key_is_literal: bool,
    max_body_bytes: u32,
    /// `static` or `document`.
    kind: &'static str,
    body_lit: String,
    /// The root document type and whether it borrows.
    root_struct: String,
    root_lifetime: &'static str,
    root_param_lifetime: &'static str,
    root_fn: String,
    objects: Vec<ObjectCtx>,
    arrays: Vec<ArrayCtx>,
}

/// A Rust string literal for `text`: every character that a literal cannot
/// carry as it is, written as an escape the compiler reads back as the same
/// text.
fn rust_str_lit(text: &str) -> String {
    format!("\"{}\"", text.escape_default())
}

/// Whether a value, written as a field, borrows from the caller.
fn borrows(node: &DocNode) -> bool {
    match &node.kind {
        DocKind::String { .. } | DocKind::Array { .. } => true,
        DocKind::Object(object) => object.fields.iter().any(|f| borrows(&f.node)),
        _ => false,
    }
}

struct Builder {
    objects: Vec<ObjectCtx>,
    arrays: Vec<ArrayCtx>,
}

impl Builder {
    /// The Rust type a node is held as, as a struct field and as a parameter.
    fn types(&mut self, node: &DocNode, type_name: &str) -> (String, String) {
        let (field, param) = match &node.kind {
            DocKind::Bool => ("bool".to_string(), "bool".to_string()),
            DocKind::Unsigned(bits) => (format!("u{bits}"), format!("u{bits}")),
            DocKind::Signed(bits) => (format!("i{bits}"), format!("i{bits}")),
            DocKind::String { .. } => ("&'a str".to_string(), "&str".to_string()),
            DocKind::Array { item, .. } => {
                let (item_field, item_param) = self.types(item, &format!("{type_name}Item"));
                (format!("&'a [{item_field}]"), format!("&[{item_param}]"))
            }
            DocKind::Object(object) => {
                let lifetime = object.fields.iter().any(|f| borrows(&f.node));
                (
                    format!("{type_name}{}", if lifetime { "<'a>" } else { "" }),
                    format!("{type_name}{}", if lifetime { "<'_>" } else { "" }),
                )
            }
        };
        if node.nullable {
            (format!("Option<{field}>"), format!("Option<{param}>"))
        } else {
            (field, param)
        }
    }

    /// The context of one value. `access` is the expression that is the value
    /// when it is not nullable, and `some_access` the one that is when a
    /// nullable value has been matched as `Some(x)`.
    fn node(&mut self, node: &DocNode, type_name: &str, path: &str, access: Access) -> NodeCtx {
        let (field_type, param_type) = self.types(node, type_name);
        let (kind, max_len, max_items, fn_name) = match &node.kind {
            DocKind::Bool => ("bool", 0, 0, String::new()),
            DocKind::Unsigned(_) => ("uint", 0, 0, String::new()),
            DocKind::Signed(_) => ("int", 0, 0, String::new()),
            DocKind::String { max_len } => ("string", *max_len, 0, String::new()),
            DocKind::Array { max_items, item } => {
                let fn_name = format!("emit_{}", filters::to_snake_case(type_name.to_string()));
                let item_path = format!("{path}[]");
                let item_ctx =
                    self.node(item, &format!("{type_name}Item"), &item_path, Access::Item);
                let elem_param_type = item_ctx.param_type.clone();
                self.arrays.push(ArrayCtx {
                    fn_name: fn_name.clone(),
                    elem_param_type,
                    max_items: *max_items,
                    path: path.to_string(),
                    item: item_ctx,
                });
                ("array", 0, *max_items, fn_name)
            }
            DocKind::Object(object) => {
                let fn_name = format!("emit_{}", filters::to_snake_case(type_name.to_string()));
                self.object(object, type_name, path, &fn_name);
                ("object", 0, 0, fn_name)
            }
        };
        let deref = !matches!(kind, "object");
        let (expr, some_expr) = match (access, deref) {
            (Access::Field(name), true) => (format!("v.{name}"), "*x".to_string()),
            (Access::Field(name), false) => (format!("&v.{name}"), "x".to_string()),
            (Access::Item, true) => ("*item".to_string(), "*x".to_string()),
            (Access::Item, false) => ("item".to_string(), "x".to_string()),
        };
        NodeCtx {
            kind,
            nullable: node.nullable,
            field_type,
            param_type,
            max_len,
            max_items,
            path: path.to_string(),
            fn_name,
            expr,
            some_expr,
        }
    }

    fn object(&mut self, object: &DocObject, type_name: &str, path: &str, fn_name: &str) {
        let mut fields = Vec::with_capacity(object.fields.len());
        for field in &object.fields {
            let field_type_name = format!("{type_name}{}", pascal_case(&field.name));
            let field_path = if path.is_empty() {
                field.name.clone()
            } else {
                format!("{path}.{}", field.name)
            };
            let node = self.node(
                &field.node,
                &field_type_name,
                &field_path,
                Access::Field(field.name.as_str()),
            );
            fields.push(FieldCtx {
                name: field.name.clone(),
                node,
            });
        }
        let lifetime = object.fields.iter().any(|f| borrows(&f.node));
        self.objects.push(ObjectCtx {
            struct_name: type_name.to_string(),
            fn_name: fn_name.to_string(),
            lifetime: if lifetime { "<'a>" } else { "" },
            param_lifetime: if lifetime { "<'_>" } else { "" },
            fields,
        });
    }
}

#[derive(Clone, Copy)]
enum Access<'n> {
    /// A field of the object being written, `v.<name>`.
    Field(&'n str),
    /// The element of the array being written, `item`.
    Item,
}

fn leg_ctx(leg: &QueryableLeg) -> LegCtx {
    let snake = filters::to_snake_case(leg.name.clone());
    let upper = filters::to_upper_snake_case(leg.name.clone());
    let mut ctx = LegCtx {
        name: leg.name.clone(),
        pascal: leg.pascal.clone(),
        snake: snake.clone(),
        upper,
        key_lit: rust_str_lit(&leg.key),
        encoding_lit: rust_str_lit(&leg.encoding),
        key_is_literal: leg.key_is_literal,
        max_body_bytes: leg.max_body_bytes,
        kind: "static",
        body_lit: String::new(),
        root_struct: String::new(),
        root_lifetime: "",
        root_param_lifetime: "",
        root_fn: String::new(),
        objects: Vec::new(),
        arrays: Vec::new(),
    };
    match &leg.reply {
        LegReply::Static { body } => {
            ctx.body_lit = rust_str_lit(body);
        }
        LegReply::Document { root } => {
            let root_struct = format!("{}Doc", leg.pascal);
            let root_fn = format!("emit_{}", filters::to_snake_case(root_struct.clone()));
            let mut builder = Builder {
                objects: Vec::new(),
                arrays: Vec::new(),
            };
            builder.object(root, &root_struct, "", &root_fn);
            let lifetime = root.fields.iter().any(|f| borrows(&f.node));
            ctx.kind = "document";
            ctx.root_struct = root_struct;
            ctx.root_lifetime = if lifetime { "<'a>" } else { "" };
            ctx.root_param_lifetime = if lifetime { "<'_>" } else { "" };
            ctx.root_fn = root_fn;
            ctx.objects = builder.objects;
            ctx.arrays = builder.arrays;
        }
    }
    ctx
}

/// Render the per-machine queryables module.
pub fn render_machine_queryables_rust(
    env: &minijinja::Environment<'_>,
    machine_name: &str,
    legs: &[QueryableLeg],
) -> Result<String, ForgeError> {
    let tmpl = env.get_template("queryables.rs.jinja2").map_err(|e| {
        ForgeError::from(GenerateError::TemplateLoad(format!(
            "queryables.rs.jinja2 (rust): {e}"
        )))
    })?;
    let leg_ctxs: Vec<LegCtx> = legs.iter().map(leg_ctx).collect();
    let max_body_bytes = legs.iter().map(|l| l.max_body_bytes).max().unwrap_or(0);
    let ctx = minijinja::context! {
        machine_name => machine_name,
        legs => minijinja::Value::from_serialize(&leg_ctxs),
        leg_count => legs.len(),
        leg_set_words => legs.len().div_ceil(32).max(1),
        max_body_bytes => max_body_bytes,
        queryable_derives_attr => RustDeriveCategory::QueryableData.derives_attr(),
    };
    tmpl.render(ctx).map_err(|e| {
        ForgeError::from(GenerateError::TemplateRender(format!(
            "queryables.rs.jinja2 (rust): {e}"
        )))
    })
}

/// Per-machine queryables emission entry point used by the orchestrator. A
/// machine that declares none emits nothing, so every existing output stays
/// byte-identical.
///
/// Rust is the only backend that emits: the writers and the matcher are Rust
/// the host links. Other backends silent-skip, as the session runtime does.
pub fn render_machine_queryables_artifacts(
    template_dir: &Path,
    language: Language,
    machine_name: &str,
    legs: &[QueryableLeg],
) -> Result<Vec<(String, String)>, ForgeError> {
    if legs.is_empty() {
        return Ok(Vec::new());
    }
    match language {
        Language::Rust => {
            let forge_dir = template_dir.join("forge/rust");
            let mut env = generator::new_env();
            generator::load_templates(&mut env, &forge_dir, Language::Rust)?;
            let module = render_machine_queryables_rust(&env, machine_name, legs)?;
            let snake = filters::to_snake_case(machine_name.to_string());
            Ok(vec![(format!("{snake}_queryables.rs"), module)])
        }
        Language::Cpp | Language::Kotlin | Language::Go | Language::Python | Language::C11 => {
            Ok(Vec::new())
        }
    }
}
