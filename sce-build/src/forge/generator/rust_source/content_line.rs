// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The source interface of a content-line codec (docs/adr/0058, SCE_FORGE.md
//! §4.6.4, docs/adr/0014).
//!
//! A component of content lines has no length or count that is not the shape of
//! the lines themselves, so every entry is presented: a method each for a
//! property's value and for the parameters declared for it. A property that may
//! be absent is an `Option`; a repeated one, and the values of one line, are an
//! iterator; a repeated property that declares a parameter, or whose line holds
//! a list of values, is an iterator of line records, and a record is a source
//! of its own (the parameters and the value or values of one line).
//!
//! The encoding rule is the runtime's `ContentLineWriter`: folding at 75
//! octets, TEXT escapes and quoting are its. What is written here is which
//! property follows which, in declaration order, and which refusals precede a
//! line: a required property absent, a parameter with no value of its property
//! to stand on. A refusal found by writing into a sink that only counts leaves
//! the caller's sink untouched.

use super::super::*;
use crate::forge::model::{ContentLineEntry, ContentLineModel};

/// What the interface of one codec is made from.
pub(in super::super) struct Input<'a> {
    pub(in super::super) m: &'a CodecModel,
    pub(in super::super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`.
    pub(in super::super) struct_name: &'a str,
    /// Whether the function that writes is always inlined (`--inline-encode`).
    pub(in super::super) inline_encode: bool,
}

struct Plan<'a> {
    input: &'a Input<'a>,
    model: &'a ContentLineModel,
}

impl Plan<'_> {
    fn l(&self) -> &LangCtx {
        self.input.l
    }

    fn name(&self) -> &str {
        self.input.struct_name
    }

    fn member(&self, id: &str) -> String {
        self.l().codec_field_id(id)
    }

    fn record_type(&self, e: &ContentLineEntry) -> String {
        format!(
            "{}{}Line",
            self.name(),
            filters::to_pascal_case(e.id.clone())
        )
    }

    /// The type one value of `e` has when it is read through the interface.
    fn value_type(&self, e: &ContentLineEntry) -> String {
        match &e.sce_type {
            SceType::String => "&str".to_string(),
            other => self.l().type_name(other).into_owned(),
        }
    }

    /// Where a value of `e` is held in the builder, as the reading of it.
    fn read_scalar(&self, e: &ContentLineEntry, field: &str, optional: bool) -> String {
        match (&e.sce_type, optional) {
            (SceType::String, false) => format!("self.{field}.as_str()"),
            (SceType::String, true) => format!("self.{field}.as_deref()"),
            _ => format!("self.{field}"),
        }
    }

    /// The items of a list of values of `e`, read from the builder.
    fn read_items(&self, e: &ContentLineEntry, field: &str) -> String {
        match &e.sce_type {
            SceType::String => format!("self.{field}.iter().map(|s| s.as_str())"),
            _ => format!("self.{field}.iter().copied()"),
        }
    }

    /// The statements that write the value `v` of `e` on the line.
    fn write_value(&self, e: &ContentLineEntry) -> String {
        let text = e.text;
        let max_size = e.max_size.unwrap_or(0);
        if let Some(separator) = &e.separator {
            let max_values = e.max_values.unwrap_or(0);
            return format!(
                "out.strings_of(v, b'{separator}', {text}, {max_size}, {max_values})?;"
            );
        }
        match &e.sce_type {
            SceType::String => format!("out.string(v, {text}, {max_size})?;"),
            SceType::Enum(r) => format!(
                "out.enum_value(TEXTS_{}, u64::from({}))?;",
                r.alias.to_uppercase(),
                self.l().codec_carrier_expr(&e.sce_type, "v")
            ),
            SceType::Bool => "out.boolean(v)?;".to_string(),
            SceType::Uint64 => "out.uint(v)?;".to_string(),
            SceType::Int64 => "out.int(v)?;".to_string(),
            ty if ty.is_signed() => "out.int(i64::from(v))?;".to_string(),
            _ => "out.uint(u64::from(v))?;".to_string(),
        }
    }

    /// The statements that write the parameter `p`, read from `holder`.
    fn write_param(&self, p: &ContentLineEntry, holder: &str) -> Vec<String> {
        let accessor = self.member(&p.id);
        let param = p.param.as_deref().unwrap_or_default();
        let max_size = p.max_size.unwrap_or(0);
        let call = |value: &str| match &p.sce_type {
            SceType::Enum(r) => format!(
                "out.enum_param(\"{param}\", TEXTS_{}, u64::from({}))?;",
                r.alias.to_uppercase(),
                self.l().codec_carrier_expr(&p.sce_type, value)
            ),
            _ => format!("out.param(\"{param}\", {value}, {max_size})?;"),
        };
        if p.required {
            vec![call(&format!("{holder}.{accessor}()"))]
        } else {
            vec![
                format!("if let Some(p) = {holder}.{accessor}() {{"),
                format!("    {}", call("p")),
                "}".to_string(),
            ]
        }
    }

    /// The accessor type of a parameter.
    fn param_type(&self, p: &ContentLineEntry) -> String {
        let value = self.value_type(p);
        if p.required {
            value
        } else {
            format!("Option<{value}>")
        }
    }

    fn items_type(&self, item: &str) -> String {
        format!("impl Iterator<Item = {item}> + Clone")
    }

    /// The type an accessor of the entry's value returns.
    fn entry_type(&self, e: &ContentLineEntry) -> String {
        if self.model.reads_records(e) {
            format!(
                "impl Iterator<Item = impl {}Source> + Clone",
                self.record_type(e)
            )
        } else if self.model.reads_values(e) {
            self.items_type("&str")
        } else if e.max_count.is_some() {
            self.items_type(&self.value_type(e))
        } else if e.required {
            self.value_type(e)
        } else {
            format!("Option<{}>", self.value_type(e))
        }
    }

    /// The type an accessor of the value of one line record returns.
    fn record_value_type(&self, e: &ContentLineEntry) -> String {
        if e.separator.is_some() {
            self.items_type("&str")
        } else {
            self.value_type(e)
        }
    }

    fn record_traits(&self) -> String {
        let mut out = String::new();
        for e in self
            .model
            .entries
            .iter()
            .filter(|e| e.param.is_none() && self.model.reads_records(e))
        {
            let record = self.record_type(e);
            let value = self.member(if e.separator.is_some() {
                "values"
            } else {
                "value"
            });
            let value_ty = self.record_value_type(e);
            let mut items = String::new();
            let mut ref_items = String::new();
            let mut builder_items = String::new();
            for p in self.model.parameters_of(e) {
                let (acc, ty) = (self.member(&p.id), self.param_type(p));
                items.push_str(&format!(
                    "    /// Parameter `{}`.\n    fn {acc}(&self) -> {ty};\n",
                    p.param.as_deref().unwrap_or_default()
                ));
                ref_items.push_str(&format!(
                    "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
                ));
                builder_items.push_str(&format!(
                    "    fn {acc}(&self) -> {ty} {{\n        {}\n    }}\n",
                    self.read_scalar(p, &acc, !p.required)
                ));
            }
            items.push_str(&format!(
                "    /// The value of the line.\n    fn {value}(&self) -> {value_ty};\n"
            ));
            ref_items.push_str(&format!(
                "    fn {value}(&self) -> {value_ty} {{\n        (**self).{value}()\n    }}\n"
            ));
            builder_items.push_str(&format!(
                "    fn {value}(&self) -> {value_ty} {{\n        {}\n    }}\n",
                if e.separator.is_some() {
                    self.read_items(e, &value)
                } else {
                    self.read_scalar(e, &value, false)
                }
            ));
            out.push_str(&format!(
                "/// What one line of `{property}` says: its parameters and its value.\n\
                 pub trait {record}Source {{\n{items}}}\n\n\
                 impl<T: {record}Source + ?Sized> {record}Source for &T {{\n{ref_items}}}\n\n\
                 impl {record}Source for {record} {{\n{builder_items}}}\n\n",
                property = e.property,
            ));
        }
        out
    }

    fn write_entry(&self, e: &ContentLineEntry, body: &mut Vec<String>) {
        let accessor = self.member(&e.id);
        let property = &e.property;
        let params: Vec<&ContentLineEntry> = self.model.parameters_of(e).collect();
        if self.model.reads_records(e) {
            if e.required {
                body.push(format!(
                    "if src.{accessor}().next().is_none() {{\n    return Err(CodecError::LineRequiredMissing);\n}}"
                ));
            }
            body.push(format!("for record in src.{accessor}() {{"));
            let value = self.member(if e.separator.is_some() {
                "values"
            } else {
                "value"
            });
            body.push(format!("    let v = record.{value}();"));
            body.push(format!("    out.property(\"{property}\")?;"));
            for p in &params {
                body.extend(
                    self.write_param(p, "record")
                        .into_iter()
                        .map(|l| format!("    {l}")),
                );
            }
            body.push(format!("    {}", self.write_value(e)));
            body.push("}".to_string());
        } else if e.max_count.is_some() {
            if e.required {
                body.push(format!(
                    "if src.{accessor}().next().is_none() {{\n    return Err(CodecError::LineRequiredMissing);\n}}"
                ));
            }
            body.push(format!("for v in src.{accessor}() {{"));
            body.push(format!("    out.property(\"{property}\")?;"));
            body.push(format!("    {}", self.write_value(e)));
            body.push("}".to_string());
        } else if e.required {
            body.push("{".to_string());
            body.push(format!("    let v = src.{accessor}();"));
            body.push(format!("    out.property(\"{property}\")?;"));
            for p in &params {
                body.extend(
                    self.write_param(p, "src")
                        .into_iter()
                        .map(|l| format!("    {l}")),
                );
            }
            body.push(format!("    {}", self.write_value(e)));
            body.push("}".to_string());
        } else {
            // A property that may be absent: its line, or nothing; a parameter
            // with no value of its property has no line to stand on.
            let values = self.model.reads_values(e);
            let any_param = params
                .iter()
                .map(|p| format!("src.{}().is_some()", self.member(&p.id)))
                .collect::<Vec<_>>()
                .join(" || ");
            if values {
                body.push("{".to_string());
                body.push(format!("    let v = src.{accessor}();"));
                body.push("    if v.clone().next().is_some() {".to_string());
                body.push(format!("        out.property(\"{property}\")?;"));
                for p in &params {
                    body.extend(
                        self.write_param(p, "src")
                            .into_iter()
                            .map(|l| format!("        {l}")),
                    );
                }
                body.push(format!("        {}", self.write_value(e)));
                if params.is_empty() {
                    body.push("    }".to_string());
                } else {
                    body.push(format!("    }} else if {any_param} {{"));
                    body.push("        return Err(CodecError::LineRequiredMissing);".to_string());
                    body.push("    }".to_string());
                }
                body.push("}".to_string());
            } else {
                body.push(format!("if let Some(v) = src.{accessor}() {{"));
                body.push(format!("    out.property(\"{property}\")?;"));
                for p in &params {
                    body.extend(
                        self.write_param(p, "src")
                            .into_iter()
                            .map(|l| format!("    {l}")),
                    );
                }
                body.push(format!("    {}", self.write_value(e)));
                if params.is_empty() {
                    body.push("}".to_string());
                } else {
                    body.push(format!("}} else if {any_param} {{"));
                    body.push("    return Err(CodecError::LineRequiredMissing);".to_string());
                    body.push("}".to_string());
                }
            }
        }
    }

    fn text(&self) -> String {
        let name = self.name();
        let trait_name = format!("{name}Source");
        let component = &self.model.component;
        let mut items = String::new();
        let mut ref_items = String::new();
        let mut builder_items = String::new();
        for e in self.model.entries.iter().filter(|e| e.param.is_none()) {
            let acc = self.member(&e.id);
            let ty = self.entry_type(e);
            items.push_str(&format!(
                "    /// Property `{}`.\n    fn {acc}(&self) -> {ty};\n",
                e.property
            ));
            ref_items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
            ));
            let read = if self.model.reads_records(e) {
                format!("self.{acc}.iter()")
            } else if self.model.reads_values(e) || e.max_count.is_some() {
                self.read_items(e, &acc)
            } else {
                self.read_scalar(e, &acc, !e.required)
            };
            builder_items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        {read}\n    }}\n"
            ));
            if !self.model.reads_records(e) {
                for p in self.model.parameters_of(e) {
                    let (pacc, pty) = (self.member(&p.id), self.param_type(p));
                    items.push_str(&format!(
                        "    /// Parameter `{}` of `{}`.\n    fn {pacc}(&self) -> {pty};\n",
                        p.param.as_deref().unwrap_or_default(),
                        e.property
                    ));
                    ref_items.push_str(&format!(
                        "    fn {pacc}(&self) -> {pty} {{\n        (**self).{pacc}()\n    }}\n"
                    ));
                    builder_items.push_str(&format!(
                        "    fn {pacc}(&self) -> {pty} {{\n        {}\n    }}\n",
                        self.read_scalar(p, &pacc, !p.required)
                    ));
                }
            }
        }
        let mut body: Vec<String> = Vec::new();
        body.push(format!(
            "let mut out = ContentLineWriter::begin(w, \"{component}\")?;"
        ));
        for e in self.model.entries.iter().filter(|e| e.param.is_none()) {
            self.write_entry(e, &mut body);
        }
        body.push("out.finish()".to_string());
        let body = super::indent(&body, 8);
        format!(
            "{records}\
             // ── Source (ADR 0058) ────────────────────────────────────────────────\n\
             /// What a value of `{name}` says: its properties and their parameters.\n\
             pub trait {trait_name} {{\n{items}}}\n\n\
             impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{ref_items}}}\n\n\
             impl {trait_name} for {name} {{\n{builder_items}}}\n\n\
             #[allow(dead_code)]\n\
             impl {name} {{\n\
             \x20   /// Encode what `src` says as one component into `w`: the properties\n\
             \x20   /// present in declaration order, folded at 75 octets. A value the lines\n\
             \x20   /// cannot carry is refused before the first byte is written.\n\
             \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {{\n\
             \x20       Self::write_source(src, &mut ::sce_forge_runtime::codec::CountSink::new())?;\n\
             \x20       Self::write_source(src, w)\n\
             \x20   }}\n\n\
             \x20   /// [`Self::encode_source`] without the pre-pass a parent has already made.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> Result<(), CodecError> {{\n\
             {body}\
             \x20   }}\n\
             }}\n\n\
             {measure}",
            records = self.record_traits(),
            inline = super::write_attribute(self.input.inline_encode),
            measure = super::measure_impl(name, "", &trait_name, self.input.inline_encode),
        )
    }
}

/// The source interface of a content-line codec, as the text its file holds.
pub(in super::super) fn render(input: &Input<'_>) -> String {
    let model = input
        .m
        .content_line
        .as_ref()
        .expect("a content-line codec carries its entries");
    Plan { input, model }.text()
}
