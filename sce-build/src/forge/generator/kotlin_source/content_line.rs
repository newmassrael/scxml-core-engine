// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin printer of a content-line codec's plan (docs/adr/0081 step 4,
//! SCE_FORGE.md §4.6.4, docs/adr/0014).
//!
//! A component of content lines has no length or count that is not the shape
//! of its lines, so every entry is presented, as the C++ and Python printers
//! present it: a property of the `{Codec}Source` interface for a property's
//! value and one for each parameter declared for it, nullable for one that may
//! be absent, a `List` for a repeated property and for the values of one line,
//! and a `List` of line records for a repeated property that declares a
//! parameter or whose line holds a list of values, each record a source of its
//! own `{Codec}{Entry}LineSource`. A line refuses a text by its content as well
//! as by its size, so no bound refines a text or a list: the encode holds each
//! where the plan's checks say. An integer is the Kotlin type of its width,
//! which holds no number past it.
//!
//! The encoding rule is the runtime's `ContentLineWriter`: folding at 75
//! octets, TEXT escapes and quoting are its. Where a line can refuse a value,
//! the encode writes the component whole into a staged sink first and hands it
//! to the caller's sink once it is written (docs/adr/0081 §1), as a positional
//! encode stages a frame, so a refusal leaves the caller's sink untouched.

use super::super::*;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::content_line_codec::bound_literal;
use crate::forge::encode_plan::{
    ContentLinePlan, LineShape, LineValue, Param, ParamValue, Property,
};
use crate::forge::model::ContentLineEntry;
use crate::generator::Language;

/// The Kotlin text the printer gives a content-line codec's file.
pub(in super::super) struct ContentLineSource {
    /// Before the line records and the value's class: the interface of each
    /// line record and of the value.
    pub(in super::super) before: String,
    /// The encode over any source, `encodeSource`, a function of the value's
    /// companion object, at its indentation there.
    pub(in super::super) encode: String,
}

/// A bound as the runtime's reader and writer take it.
fn bound(n: Option<u32>) -> String {
    bound_literal(Language::Kotlin, n)
}

/// The Kotlin printer of a content-line codec's plan.
struct Printer<'a> {
    l: &'a LangCtx,
    /// The value's data class, `to_pascal_case(m.name)`.
    name: &'a str,
    /// The plan, whose totality every refusal answered here must be in.
    plan: ContentLinePlan<'a>,
}

impl Printer<'_> {
    fn member(&self, id: &str) -> String {
        self.l.codec_field_id(id)
    }

    /// The data class of one line of the records of `e`, which the template
    /// declares (`content_line_codec::entries_context`).
    fn record_type(&self, e: &ContentLineEntry) -> String {
        format!("{}{}Line", self.name, filters::to_pascal_case(e.id.clone()))
    }

    /// The interface one line of the records of `e` implements.
    fn record_source(&self, e: &ContentLineEntry) -> String {
        format!("{}Source", self.record_type(e))
    }

    /// The type one value of `e` is.
    fn one(&self, e: &ContentLineEntry) -> String {
        self.l.type_name(e.present_type()).into_owned()
    }

    /// The type a source presents the parameter `p` as.
    fn param_presented(&self, p: &Param<'_>) -> String {
        if p.required {
            self.one(p.entry)
        } else {
            format!("{}?", self.one(p.entry))
        }
    }

    /// The type a source presents the property `p` as.
    fn property_presented(&self, p: &Property<'_>) -> String {
        let e = p.entry;
        match p.shape {
            LineShape::Records { .. } => format!("List<{}>", self.record_source(e)),
            LineShape::Repeated { .. } => format!("List<{}>", self.one(e)),
            LineShape::One { values: true, .. } => "List<String>".to_string(),
            LineShape::One { required: true, .. } => self.one(e),
            LineShape::One { .. } => format!("{}?", self.one(e)),
        }
    }

    /// The statement that answers the refusal of `kind` at the entry `e`,
    /// which the totality analysis names (`Totality::require`, docs/adr/0065
    /// §2): the encode's answer is the error itself, as every Kotlin encode
    /// answers a `CodecError` or `null` (docs/adr/0099, amendment of
    /// 2026-11-16).
    fn fail(&self, kind: RefusalKind, e: &ContentLineEntry) -> String {
        let error = self.plan.totality.require(kind, &e.id).replace("::", ".");
        format!("return {error}")
    }

    /// The carrier of the enum value `value` of `e`, as the `ULong` the line
    /// writer takes.
    fn carrier(&self, e: &ContentLineEntry, value: &str) -> String {
        let carrier = self.l.codec_carrier_expr(e.present_type(), value);
        let alias = match e.present_type() {
            SceType::Enum(r) => r.alias.as_str(),
            _ => unreachable!("only an enum entry is written by its carrier"),
        };
        if self.l.enum_carrier_type(alias).as_deref() == Some("ULong") {
            carrier
        } else {
            format!("{carrier}.toULong()")
        }
    }

    /// The statement that writes `v`, the value of one line of `p`, at
    /// `indent`, ending the line.
    fn write_value(&self, p: &Property<'_>, v: &str, indent: &str) -> String {
        let e = p.entry;
        // A value the line writer can refuse, answered where it is written.
        if matches!(
            p.value,
            LineValue::Values { .. } | LineValue::Text { .. } | LineValue::Enum { .. }
        ) {
            self.plan.totality.require(RefusalKind::LineBadValue, &e.id);
        }
        let call = match &p.value {
            LineValue::Values {
                separator,
                text,
                max_size,
                max_values,
            } => format!(
                "out.strings({v}, '{separator}', {text}, {}, {})",
                bound(*max_size),
                bound(*max_values)
            ),
            LineValue::Text { text, max_size } => {
                format!("out.string({v}, {text}, {})", bound(*max_size))
            }
            LineValue::Enum { alias } => {
                format!("out.enumValue(TEXTS_{alias}, {})", self.carrier(e, v))
            }
            LineValue::Bool => format!("out.boolean({v})"),
            LineValue::Uint { wide: true } => format!("out.uint({v})"),
            LineValue::Uint { wide: false } => format!("out.uint({v}.toULong())"),
            LineValue::Int { wide: true } => format!("out.int({v})"),
            LineValue::Int { wide: false } => format!("out.int({v}.toLong())"),
        };
        format!("{indent}{call}?.let {{ return it }}\n")
    }

    /// The local a parameter that may be absent is read into, once, which
    /// Kotlin narrows where the encode tests it.
    fn param_local(&self, param: &Param<'_>) -> String {
        format!("_p_{}", self.member(&param.entry.id))
    }

    /// The statements that write each parameter of `p`, held by `holder` (the
    /// source, or a line record), each at `indent`: one that may be absent is
    /// written out of its local where it is there, the local read here unless
    /// `read_before` says the encode read it already.
    fn write_params(
        &self,
        p: &Property<'_>,
        holder: &str,
        read_before: bool,
        indent: &str,
    ) -> String {
        let mut out = String::new();
        for param in &p.params {
            // Every parameter's value is one the line writer can refuse.
            self.plan
                .totality
                .require(RefusalKind::LineBadValue, &param.entry.id);
            let name = param.entry.param.as_deref().unwrap_or_default();
            let call = |value: &str| match &param.value {
                ParamValue::Enum { alias } => format!(
                    "out.enumParam(\"{name}\", TEXTS_{alias}, {})",
                    self.carrier(param.entry, value)
                ),
                ParamValue::Text { max_size } => {
                    format!("out.param(\"{name}\", {value}, {})", bound(*max_size))
                }
            };
            let read = format!("{holder}.{}", self.member(&param.entry.id));
            if param.required {
                out.push_str(&format!("{indent}{}?.let {{ return it }}\n", call(&read)));
            } else {
                let local = self.param_local(param);
                if !read_before {
                    out.push_str(&format!("{indent}val {local} = {read}\n"));
                }
                out.push_str(&format!(
                    "{indent}if ({local} != null) {}?.let {{ return it }}\n",
                    call(&local)
                ));
            }
        }
        out
    }

    /// The statements that read each parameter of `p` that may be absent off
    /// the source into its local, before the property's line, so a test of
    /// the property's absence can read them too.
    fn read_params(&self, p: &Property<'_>) -> String {
        p.params
            .iter()
            .filter(|param| !param.required)
            .map(|param| {
                format!(
                    "            val {} = src.{}\n",
                    self.param_local(param),
                    self.member(&param.entry.id)
                )
            })
            .collect()
    }

    /// The judgement of a repeated property's lines, the list `lines`, before
    /// any is written: none where one is required, and no more than its
    /// `sce:max-count`, where the protocol states one. A source's list is not
    /// held to the bound, so the encode holds it there, as every backend does.
    fn judge_lines(
        &self,
        e: &ContentLineEntry,
        required: bool,
        at_most: Option<u32>,
        lines: &str,
    ) -> String {
        let mut out = String::new();
        if let Some(max_count) = at_most {
            out.push_str(&format!(
                "            if ({lines}.size > {max_count}) {}\n",
                self.fail(RefusalKind::LineTooMany, e)
            ));
        }
        if required {
            out.push_str(&format!(
                "            if ({lines}.isEmpty()) {}\n",
                self.fail(RefusalKind::LineRequiredMissing, e)
            ));
        }
        out
    }

    /// The statements that write the lines of the property `p`, read off
    /// `src`.
    fn write_entry(&self, p: &Property<'_>) -> String {
        let e = p.entry;
        let name = self.member(&e.id);
        let v = format!("_e_{name}");
        let line = format!("out.property(\"{}\")?.let {{ return it }}", e.property);
        let mut out = format!(
            "            // Property `{}`.\n            val {v} = src.{name}\n",
            e.property
        );
        match p.shape {
            LineShape::Records { required, at_most } => {
                out.push_str(&self.judge_lines(e, required, at_most, &v));
                let value = self.member(if matches!(p.value, LineValue::Values { .. }) {
                    "values"
                } else {
                    "value"
                });
                out.push_str(&format!(
                    "            for (_line in {v}) {{\n                {line}\n"
                ));
                out.push_str(&self.write_params(p, "_line", false, "                "));
                out.push_str(&self.write_value(p, &format!("_line.{value}"), "                "));
                out.push_str("            }\n");
            }
            LineShape::Repeated { required, at_most } => {
                out.push_str(&self.judge_lines(e, required, at_most, &v));
                out.push_str(&format!(
                    "            for (_v in {v}) {{\n                {line}\n"
                ));
                out.push_str(&self.write_value(p, "_v", "                "));
                out.push_str("            }\n");
            }
            LineShape::One { required: true, .. } => {
                out.push_str(&format!("            {line}\n"));
                out.push_str(&self.write_params(p, "src", false, "            "));
                out.push_str(&self.write_value(p, &v, "            "));
            }
            // A property that may be absent: its line, or nothing; a parameter
            // with no value of its property has no line to stand on.
            LineShape::One {
                required: false,
                values,
            } => {
                out.push_str(&self.read_params(p));
                let present = if values {
                    format!("{v}.isNotEmpty()")
                } else {
                    format!("{v} != null")
                };
                out.push_str(&format!(
                    "            if ({present}) {{\n                {line}\n"
                ));
                out.push_str(&self.write_params(p, "src", true, "                "));
                out.push_str(&self.write_value(p, &v, "                "));
                if p.params.is_empty() {
                    out.push_str("            }\n");
                } else {
                    // A parameter every value holds is always given, so its
                    // property's absence is always refused; one that may be
                    // absent is refused where it is there.
                    let refusal = self.fail(RefusalKind::LineRequiredMissing, e);
                    if p.params.iter().any(|param| param.required) {
                        out.push_str(&format!(
                            "            }} else {{\n                {refusal}\n            }}\n"
                        ));
                    } else {
                        let any = p
                            .params
                            .iter()
                            .map(|param| format!("{} != null", self.param_local(param)))
                            .collect::<Vec<_>>()
                            .join(" || ");
                        out.push_str(&format!(
                            "            }} else if ({any}) {{\n                {refusal}\n            }}\n"
                        ));
                    }
                }
            }
        }
        out
    }

    /// The interface named `class` of `members`, each a name and the type a
    /// source presents it as; `what` names what presents them.
    fn interface(class: &str, what: &str, members: &[(String, String)]) -> String {
        let mut out = format!(
            "/// The elements {what} presents, which the encode reads (docs/adr/0081 §2):\n\
             /// any object that presents them is a source of it.\n\
             interface {class} {{\n"
        );
        for (name, ty) in members {
            out.push_str(&format!("    val {name}: {ty}\n"));
        }
        out.push_str("}\n\n");
        out
    }

    fn render(&self) -> ContentLineSource {
        let name = self.name;
        let mut interfaces = String::new();
        let mut members: Vec<(String, String)> = Vec::new();
        for p in &self.plan.properties {
            let e = p.entry;
            members.push((self.member(&e.id), self.property_presented(p)));
            let params: Vec<(String, String)> = p
                .params
                .iter()
                .map(|param| (self.member(&param.entry.id), self.param_presented(param)))
                .collect();
            if let LineShape::Records { .. } = p.shape {
                // A record holds its parameters, then its value or values.
                let value = if matches!(p.value, LineValue::Values { .. }) {
                    (self.member("values"), "List<String>".to_string())
                } else {
                    (self.member("value"), self.one(e))
                };
                let mut record_members = params;
                record_members.push(value);
                interfaces.push_str(&Self::interface(
                    &self.record_source(e),
                    &format!("one line of `{}`", e.property),
                    &record_members,
                ));
            } else {
                members.extend(params);
            }
        }
        interfaces.push_str(&Self::interface(
            &format!("{name}Source"),
            &format!("a value of [{name}]"),
            &members,
        ));

        // The encode: each property's lines in declaration order, into a
        // staged sink first where a line can refuse the value.
        let staged = !self.plan.totality.is_total();
        let component = &self.plan.lines.component;
        let mut encode = format!(
            "        /// Write what [src] says as one `{component}` component into [w]: the\n\
             \x20       /// properties present in declaration order, folded at 75 octets\n\
             \x20       /// (SCE_FORGE.md §4.6.4). `null` on success.\n"
        );
        if staged {
            encode.push_str(&format!(
                "        /// A value the lines cannot carry is refused, before a byte reaches\n\
                 \x20       /// [w], with the [CodecError] of the rule that refused it: the\n\
                 \x20       /// component is written whole into a staged sink first.\n\
                 \x20       fun encodeSource(src: {name}Source, w: SceSink): CodecError? {{\n\
                 \x20           val _f = mutableListOf<Byte>()\n\
                 \x20           val out = ContentLineWriter(MutableListSink(_f), \"{component}\")\n"
            ));
        } else {
            encode.push_str(&format!(
                "        /// It refuses nothing a sink does not.\n\
                 \x20       fun encodeSource(src: {name}Source, w: SceSink): CodecError? {{\n\
                 \x20           val out = ContentLineWriter(w, \"{component}\")\n"
            ));
        }
        encode.push_str("            out.begin()?.let { return it }\n");
        for p in &self.plan.properties {
            encode.push_str(&self.write_entry(p));
        }
        if staged {
            encode.push_str(
                "            out.finish()?.let { return it }\n\
                 \x20           return w.writeBytes(_f.toByteArray())\n",
            );
        } else {
            encode.push_str("            return out.finish()\n");
        }
        encode.push_str("        }");

        ContentLineSource {
            before: interfaces.trim_end_matches('\n').to_string(),
            encode,
        }
    }
}

/// The byte case of `source`, a value of the content-line codec `plan` at
/// `path` (docs/adr/0081 §5): its data class made by its constructor, which
/// holds a text as a bare `String`, an integer as the Kotlin type of its width
/// and the values of a repeated property as a `MutableList`, each element
/// compared as the value holds it. A component takes no flag input and no
/// caller's tag.
pub(crate) fn source_case(
    plan: &ContentLinePlan<'_>,
    path: &super::source_case::CodecPath,
    source: &serde_json::Value,
) -> Result<super::source_case::SourceCase, String> {
    use super::source_case::Holds;
    let elements = super::super::source_notation::content_line_elements(plan, source)?;
    // In the order the notation reads them: each property, then the
    // parameters declared for it, every one a text.
    let mut holds = Vec::new();
    for p in &plan.properties {
        holds.push(match &p.value {
            LineValue::Text { .. } => Holds::Text,
            LineValue::Bool => Holds::Bool,
            LineValue::Uint { .. } | LineValue::Int { .. } => Holds::Int(p.entry.present_type()),
            LineValue::Values { .. } | LineValue::Enum { .. } => unreachable!(
                "the notation refuses a line of a list of values and a value of an enum \
                 (`source_notation::content_line_elements`)"
            ),
        });
        holds.extend(p.params.iter().map(|_| Holds::Text));
    }
    super::source_case::map_or_line_case(path, elements.into_iter().zip(holds).collect())
}

/// The Kotlin text of the content-line codec `m`, whose value's data class is
/// `struct_name`: the interface of each line record and of the value, and the
/// encode over any source.
pub(in super::super) fn render(
    m: &CodecModel,
    l: &LangCtx,
    struct_name: &str,
) -> ContentLineSource {
    let printer = Printer {
        l,
        name: struct_name,
        plan: ContentLinePlan::of(m),
    };
    let printed = printer.render();
    printer.plan.totality.assert_all_written(&m.name);
    printed
}
