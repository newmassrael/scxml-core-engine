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
//! What the encode writes, and which refusals precede a line, is the codec's
//! encode plan ([`ContentLinePlan`], docs/adr/0081); this module spells it in
//! Rust. The encoding rule is the runtime's `ContentLineWriter`: folding at 75
//! octets, TEXT escapes and quoting are its. A refusal found by writing into a
//! sink that only counts leaves the caller's sink untouched.

use super::super::*;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::content_line_codec::bound_literal;
use crate::forge::encode_plan::{
    ContentLinePlan, LineShape, LineValue, Param, ParamValue, Property,
};
use crate::forge::model::ContentLineEntry;
use crate::generator::Language;

/// The runtime's codec module, as the generated text spells it.
const RT: &str = "::sce_forge_runtime::codec";

/// What the interface of one codec is made from.
pub(in super::super) struct Input<'a> {
    pub(in super::super) m: &'a CodecModel,
    pub(in super::super) l: &'a LangCtx,
    /// `to_pascal_case(m.name)`.
    pub(in super::super) struct_name: &'a str,
    /// Whether the function that writes is always inlined (`--inline-encode`).
    pub(in super::super) inline_encode: bool,
}

/// The Rust printer of a content-line codec's encode plan.
struct Printer<'a> {
    input: &'a Input<'a>,
    /// The plan, whose totality every refusal written here must be in.
    plan: ContentLinePlan<'a>,
}

impl Printer<'_> {
    /// `return Err(CodecError::LineRequiredMissing);` for the entry `e`.
    fn required_missing(&self, e: &ContentLineEntry) -> String {
        format!(
            "return Err({});",
            self.plan
                .totality
                .require(RefusalKind::LineRequiredMissing, &e.id)
        )
    }

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
        match e.present_type() {
            SceType::String => "&str".to_string(),
            other => self.l().type_name(other).into_owned(),
        }
    }

    /// Where a value of `e` is held in the value, as the reading of it: a text
    /// through what its profile holds it in (docs/adr/0077 §7).
    fn read_scalar(&self, e: &ContentLineEntry, field: &str, optional: bool) -> String {
        match (e.present_type(), optional) {
            (SceType::String, false) => format!("{RT}::HeldStr::held_str(&self.{field})"),
            (SceType::String, true) => {
                format!("self.{field}.as_ref().map({RT}::HeldStr::held_str)")
            }
            _ => format!("self.{field}"),
        }
    }

    /// The items of a list of values of `e`, read from the list its profile
    /// holds them in.
    fn read_items(&self, e: &ContentLineEntry, field: &str) -> String {
        match e.present_type() {
            SceType::String => format!(
                "{RT}::SceList::as_slice(&self.{field}).iter().map({RT}::HeldStr::held_str)"
            ),
            _ => format!("{RT}::SceList::as_slice(&self.{field}).iter().copied()"),
        }
    }

    /// The statement that writes the value `v` of the property `p` on the line.
    fn write_value(&self, p: &Property<'_>) -> String {
        let e = p.entry;
        // A value the line writer can refuse: recorded where it is written.
        if matches!(
            p.value,
            LineValue::Values { .. } | LineValue::Text { .. } | LineValue::Enum { .. }
        ) {
            self.plan.totality.require(RefusalKind::LineBadValue, &e.id);
        }
        match &p.value {
            LineValue::Values {
                separator,
                text,
                max_size,
                max_values,
            } => format!(
                "out.strings_of(v, b'{separator}', {text}, {}, {})?;",
                bound_literal(Language::Rust, *max_size),
                bound_literal(Language::Rust, *max_values)
            ),
            LineValue::Text { text, max_size } => format!(
                "out.string(v, {text}, {})?;",
                bound_literal(Language::Rust, *max_size)
            ),
            LineValue::Enum { alias } => format!(
                "out.enum_value(TEXTS_{}, u64::from({}))?;",
                alias.to_uppercase(),
                self.l().codec_carrier_expr(e.present_type(), "v")
            ),
            LineValue::Bool => "out.boolean(v)?;".to_string(),
            LineValue::Uint { wide: true } => "out.uint(v)?;".to_string(),
            LineValue::Int { wide: true } => "out.int(v)?;".to_string(),
            LineValue::Int { wide: false } => "out.int(i64::from(v))?;".to_string(),
            LineValue::Uint { wide: false } => "out.uint(u64::from(v))?;".to_string(),
        }
    }

    /// The statements that write the parameter `p`, read from `holder`.
    fn write_param(&self, p: &Param<'_>, holder: &str) -> Vec<String> {
        let accessor = self.member(&p.entry.id);
        let param = p.entry.param.as_deref().unwrap_or_default();
        // Every parameter's value is one the line writer can refuse.
        self.plan
            .totality
            .require(RefusalKind::LineBadValue, &p.entry.id);
        let call = |value: &str| match &p.value {
            ParamValue::Enum { alias } => format!(
                "out.enum_param(\"{param}\", TEXTS_{}, u64::from({}))?;",
                alias.to_uppercase(),
                self.l().codec_carrier_expr(p.entry.present_type(), value)
            ),
            ParamValue::Text { max_size } => format!(
                "out.param(\"{param}\", {value}, {})?;",
                bound_literal(Language::Rust, *max_size)
            ),
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
    fn param_type(&self, p: &Param<'_>) -> String {
        let value = self.value_type(p.entry);
        if p.required {
            value
        } else {
            self.l().codec_optional_of(&value)
        }
    }

    fn items_type(&self, item: &str) -> String {
        format!("impl Iterator<Item = {item}> + Clone")
    }

    /// The type an accessor of the property's value returns.
    fn entry_type(&self, p: &Property<'_>) -> String {
        match p.shape {
            LineShape::Records { .. } => format!(
                "impl Iterator<Item = impl {}Source> + Clone",
                self.record_type(p.entry)
            ),
            LineShape::One { values: true, .. } => self.items_type("&str"),
            LineShape::Repeated { .. } => self.items_type(&self.value_type(p.entry)),
            LineShape::One { required: true, .. } => self.value_type(p.entry),
            LineShape::One {
                required: false, ..
            } => self.l().codec_optional_of(&self.value_type(p.entry)),
        }
    }

    /// The type an accessor of the value of one line record returns.
    fn record_value_type(&self, p: &Property<'_>) -> String {
        if matches!(p.value, LineValue::Values { .. }) {
            self.items_type("&str")
        } else {
            self.value_type(p.entry)
        }
    }

    /// The member a line record's value or values are read by.
    fn record_value(&self, p: &Property<'_>) -> String {
        self.member(if matches!(p.value, LineValue::Values { .. }) {
            "values"
        } else {
            "value"
        })
    }

    fn record_traits(&self) -> String {
        let mut out = String::new();
        for p in self
            .plan
            .properties
            .iter()
            .filter(|p| matches!(p.shape, LineShape::Records { .. }))
        {
            let e = p.entry;
            let record = self.record_type(e);
            let value = self.record_value(p);
            let value_ty = self.record_value_type(p);
            let mut items = String::new();
            let mut ref_items = String::new();
            let mut builder_items = String::new();
            for param in &p.params {
                let (acc, ty) = (self.member(&param.entry.id), self.param_type(param));
                items.push_str(&format!(
                    "    /// Parameter `{}`.\n    fn {acc}(&self) -> {ty};\n",
                    param.entry.param.as_deref().unwrap_or_default()
                ));
                ref_items.push_str(&format!(
                    "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
                ));
                builder_items.push_str(&format!(
                    "    fn {acc}(&self) -> {ty} {{\n        {}\n    }}\n",
                    self.read_scalar(param.entry, &acc, !param.required)
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
                if matches!(p.value, LineValue::Values { .. }) {
                    self.read_items(e, &value)
                } else {
                    self.read_scalar(e, &value, false)
                }
            ));
            out.push_str(&format!(
                "/// What one line of `{property}` says: its parameters and its value.\n\
                 pub trait {record}Source {{\n{items}}}\n\n\
                 impl<T: {record}Source + ?Sized> {record}Source for &T {{\n{ref_items}}}\n\n\
                 impl<S: {name}Storage> {record}Source for {record}In<S> {{\n{builder_items}}}\n\n",
                property = e.property,
                name = self.name(),
            ));
        }
        out
    }

    /// The judgement of a repeated property's lines before any is written: none
    /// where one is required, and no more than its `sce:max-count`, where the
    /// protocol states one. A source's list is not held to the bound, so the
    /// encode holds it there, as every backend does.
    fn judge_lines(
        &self,
        e: &ContentLineEntry,
        required: bool,
        at_most: Option<u32>,
        accessor: &str,
        body: &mut Vec<String>,
    ) {
        if required {
            body.push(format!(
                "if src.{accessor}().next().is_none() {{\n    {}\n}}",
                self.required_missing(e)
            ));
        }
        if let Some(max_count) = at_most {
            body.push(format!(
                "if src.{accessor}().count() > {max_count} {{\n    return Err({});\n}}",
                self.plan.totality.require(RefusalKind::LineTooMany, &e.id)
            ));
        }
    }

    fn write_entry(&self, p: &Property<'_>, body: &mut Vec<String>) {
        let e = p.entry;
        let accessor = self.member(&e.id);
        let property = &e.property;
        match p.shape {
            LineShape::Records { required, at_most } => {
                self.judge_lines(e, required, at_most, &accessor, body);
                body.push(format!("for record in src.{accessor}() {{"));
                body.push(format!("    let v = record.{}();", self.record_value(p)));
                body.push(format!("    out.property(\"{property}\")?;"));
                for param in &p.params {
                    body.extend(
                        self.write_param(param, "record")
                            .into_iter()
                            .map(|l| format!("    {l}")),
                    );
                }
                body.push(format!("    {}", self.write_value(p)));
                body.push("}".to_string());
            }
            LineShape::Repeated { required, at_most } => {
                self.judge_lines(e, required, at_most, &accessor, body);
                body.push(format!("for v in src.{accessor}() {{"));
                body.push(format!("    out.property(\"{property}\")?;"));
                body.push(format!("    {}", self.write_value(p)));
                body.push("}".to_string());
            }
            LineShape::One { required: true, .. } => {
                body.push("{".to_string());
                body.push(format!("    let v = src.{accessor}();"));
                body.push(format!("    out.property(\"{property}\")?;"));
                for param in &p.params {
                    body.extend(
                        self.write_param(param, "src")
                            .into_iter()
                            .map(|l| format!("    {l}")),
                    );
                }
                body.push(format!("    {}", self.write_value(p)));
                body.push("}".to_string());
            }
            // A property that may be absent: its line, or nothing; a parameter
            // with no value of its property has no line to stand on.
            LineShape::One {
                required: false,
                values,
            } => {
                let any_param = p
                    .params
                    .iter()
                    .map(|param| format!("src.{}().is_some()", self.member(&param.entry.id)))
                    .collect::<Vec<_>>()
                    .join(" || ");
                if values {
                    body.push("{".to_string());
                    body.push(format!("    let v = src.{accessor}();"));
                    body.push("    if v.clone().next().is_some() {".to_string());
                    body.push(format!("        out.property(\"{property}\")?;"));
                    for param in &p.params {
                        body.extend(
                            self.write_param(param, "src")
                                .into_iter()
                                .map(|l| format!("        {l}")),
                        );
                    }
                    body.push(format!("        {}", self.write_value(p)));
                    if p.params.is_empty() {
                        body.push("    }".to_string());
                    } else {
                        body.push(format!("    }} else if {any_param} {{"));
                        body.push(format!("        {}", self.required_missing(e)));
                        body.push("    }".to_string());
                    }
                    body.push("}".to_string());
                } else {
                    body.push(format!("if let Some(v) = src.{accessor}() {{"));
                    body.push(format!("    out.property(\"{property}\")?;"));
                    for param in &p.params {
                        body.extend(
                            self.write_param(param, "src")
                                .into_iter()
                                .map(|l| format!("    {l}")),
                        );
                    }
                    body.push(format!("    {}", self.write_value(p)));
                    if p.params.is_empty() {
                        body.push("}".to_string());
                    } else {
                        body.push(format!("}} else if {any_param} {{"));
                        body.push(format!("    {}", self.required_missing(e)));
                        body.push("}".to_string());
                    }
                }
            }
        }
    }

    fn text(&self) -> String {
        let name = self.name();
        let trait_name = format!("{name}Source");
        let component = &self.plan.lines.component;
        let mut items = String::new();
        let mut ref_items = String::new();
        let mut builder_items = String::new();
        for p in &self.plan.properties {
            let e = p.entry;
            let acc = self.member(&e.id);
            let ty = self.entry_type(p);
            items.push_str(&format!(
                "    /// Property `{}`.\n    fn {acc}(&self) -> {ty};\n",
                e.property
            ));
            ref_items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        (**self).{acc}()\n    }}\n"
            ));
            let read = match p.shape {
                LineShape::Records { .. } => {
                    format!("{RT}::SceList::as_slice(&self.{acc}).iter()")
                }
                LineShape::Repeated { .. } | LineShape::One { values: true, .. } => {
                    self.read_items(e, &acc)
                }
                LineShape::One { required, .. } => self.read_scalar(e, &acc, !required),
            };
            builder_items.push_str(&format!(
                "    fn {acc}(&self) -> {ty} {{\n        {read}\n    }}\n"
            ));
            if !matches!(p.shape, LineShape::Records { .. }) {
                for param in &p.params {
                    let (pacc, pty) = (self.member(&param.entry.id), self.param_type(param));
                    items.push_str(&format!(
                        "    /// Parameter `{}` of `{}`.\n    fn {pacc}(&self) -> {pty};\n",
                        param.entry.param.as_deref().unwrap_or_default(),
                        e.property
                    ));
                    ref_items.push_str(&format!(
                        "    fn {pacc}(&self) -> {pty} {{\n        (**self).{pacc}()\n    }}\n"
                    ));
                    builder_items.push_str(&format!(
                        "    fn {pacc}(&self) -> {pty} {{\n        {}\n    }}\n",
                        self.read_scalar(param.entry, &pacc, !param.required)
                    ));
                }
            }
        }
        let mut body: Vec<String> = Vec::new();
        body.push(format!(
            "let mut out = ContentLineWriter::begin(w, \"{component}\")?;"
        ));
        for p in &self.plan.properties {
            self.write_entry(p, &mut body);
        }
        body.push("out.finish()?;".to_string());
        body.push("Ok(())".to_string());
        let body = super::indent(&body, 8);
        let may_fail = !self.plan.totality.is_total();
        // A component whose values a line can refuse is written into a sink
        // that only counts first, so a refusal leaves the caller's sink
        // untouched; one whose values no line refuses has nothing to find.
        let (doc, guard) = if may_fail {
            (
                "\x20   /// cannot carry is refused before the first byte is written.\n",
                "\x20       Self::count_source(src)?;\n",
            )
        } else {
            (
                "\x20   /// cannot carry is not one a value of it can say: the only error is the\n\
                 \x20   /// sink's (docs/adr/0065).\n",
                "",
            )
        };
        let result = super::write_result(may_fail, "K");
        format!(
            "{records}\
             // ── Source (ADR 0058) ────────────────────────────────────────────────\n\
             /// What a value of `{name}` says: its properties and their parameters.\n\
             pub trait {trait_name} {{\n{items}}}\n\n\
             impl<T: {trait_name} + ?Sized> {trait_name} for &T {{\n{ref_items}}}\n\n\
             impl<S: {name}Storage> {trait_name} for {name}In<S> {{\n{builder_items}}}\n\n\
             #[allow(dead_code)]\n\
             impl {name}Wire {{\n\
             \x20   /// Encode what `src` says as one component into `w`: the properties\n\
             \x20   /// present in declaration order, folded at 75 octets. A value the lines\n\
             {doc}\
             \x20   pub fn encode_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
             {guard}\
             \x20       Self::write_source(src, w)\n\
             \x20   }}\n\n\
             \x20   /// [`Self::encode_source`] without the pre-pass a parent has already made.\n\
             \x20   #[doc(hidden)]\n\
             {inline}\
             \x20   pub fn write_source<S: {trait_name} + ?Sized, K: SceSink>(src: &S, w: &mut K) -> {result} {{\n\
             {body}\
             \x20   }}\n\n\
             {count}\
             }}\n\n\
             {measure}",
            count = super::count_function(&trait_name, "", "", may_fail),
            records = self.record_traits(),
            inline = super::write_attribute(self.input.inline_encode),
            measure = super::measure_impl(
                &format!("{name}Wire"),
                "",
                &trait_name,
                self.input.inline_encode,
                may_fail
            ),
        )
    }
}

/// The source interface of a content-line codec, as the text its file holds.
pub(in super::super) fn render(input: &Input<'_>) -> String {
    let printer = Printer {
        input,
        plan: ContentLinePlan::of(input.m),
    };
    let text = printer.text();
    printer.plan.totality.assert_all_written(&input.m.name);
    text
}
