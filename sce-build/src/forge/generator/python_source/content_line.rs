// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python printer of a content-line codec's plan (docs/adr/0081 step 4,
//! SCE_FORGE.md §4.6.4, docs/adr/0014).
//!
//! A component of content lines has no length or count that is not the shape
//! of its lines, so every entry is presented, as the C++ printer presents it
//! (`cpp_source::content_line`): a member of the `{Codec}Source` `Protocol` for
//! a property's value and one for each parameter declared for it, `Optional`
//! for one that may be absent, a `Sequence` for a repeated property and for the
//! values of one line, and a `Sequence` of line records for a repeated property
//! that declares a parameter or whose line holds a list of values, each record
//! a source of its own `Protocol`. A line refuses a text by its content as well
//! as by its size, so no bound refines a text or a list: the encode holds each
//! where the plan's checks say, as the C++ printer does. An integer the value
//! holds alone is refined by the class of its width (`BoundedInt`), as a
//! positional or a CBOR integer is; one in a list, which a value appends to,
//! is held to its width as the line is written, the runtime's `LineBadValue`,
//! as the template's encode held it.
//!
//! The encoding rule is the runtime's `ContentLineWriter`: folding at 75
//! octets, TEXT escapes and quoting are its. Where a line can refuse a value,
//! the encode writes the component whole into a buffer first and hands it to
//! the caller's sink once it is written (docs/adr/0081 §1), so a refusal
//! leaves the caller's sink untouched.

use super::super::*;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::content_line_codec::bound_literal;
use crate::forge::encode_plan::{
    ContentLinePlan, LineShape, LineValue, Param, ParamValue, Property,
};
use crate::forge::model::ContentLineEntry;
use crate::generator::Language;
use std::collections::{BTreeMap, BTreeSet};

/// The Python text the printer gives a content-line codec's module.
pub(in super::super) struct ContentLineSource {
    /// Before the line records and the value's class: the class of each
    /// integer's width, the `Protocol` of each line record and of the value,
    /// and the encode over any source, `encode_source`.
    pub(in super::super) before: String,
    /// The value's `__setattr__`, which holds each integer to its class
    /// wherever it is set; empty where it holds none alone.
    pub(in super::super) hold: String,
    /// For each property and parameter, by its id, the type the field that
    /// holds it is declared as and the default it takes.
    pub(in super::super) declared: PythonDeclared,
    /// For each property whose lines are records, by its id, how the record
    /// holds its value: the type of the field `value` or `values`, its
    /// default, and the record's `__setattr__`, empty where it has none.
    pub(in super::super) records: BTreeMap<String, (String, String, String)>,
    /// The import of the runtime's classes the module names; empty where it
    /// names none.
    pub(in super::super) runtime: String,
}

impl ContentLineSource {
    /// What the codec's template reads of the printed text, as its
    /// `python_source`.
    pub(in super::super) fn context(&self) -> serde_json::Value {
        serde_json::json!({
            "before": self.before,
            "hold": self.hold,
            "runtime": self.runtime,
        })
    }
}

/// A bound as the runtime's reader and writer take it.
fn bound(n: Option<u32>) -> String {
    bound_literal(Language::Python, n)
}

/// A Python boolean literal.
fn py_bool(b: bool) -> &'static str {
    if b {
        "True"
    } else {
        "False"
    }
}

/// The least and the most number the integer type `ty` holds, `None` for
/// another type.
fn int_span(ty: &SceType) -> Option<(i128, i128)> {
    let bits = ty.int_bit_width()?;
    Some(if ty.is_signed() {
        (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1)
    } else {
        (0, (1i128 << bits) - 1)
    })
}

/// The Python printer of a content-line codec's plan.
struct Printer<'a> {
    l: &'a LangCtx,
    /// The value's class, `to_pascal_case(m.name)`.
    name: &'a str,
    /// The plan, whose totality every refusal raised here must be in.
    plan: ContentLinePlan<'a>,
}

impl Printer<'_> {
    fn member(&self, id: &str) -> String {
        self.l.codec_field_id(id)
    }

    /// The class of one line of the records of `e`, which the template
    /// defines.
    fn record_type(&self, e: &ContentLineEntry) -> String {
        format!("{}{}Line", self.name, filters::to_pascal_case(e.id.clone()))
    }

    /// The `Protocol` one line of the records of `e` satisfies.
    fn record_source(&self, e: &ContentLineEntry) -> String {
        format!("{}Source", self.record_type(e))
    }

    /// The class the integer `e` holds alone is held in, a subclass of the
    /// runtime's `BoundedInt` of its type's width; `None` where `e` is no
    /// integer.
    fn int_class(&self, e: &ContentLineEntry) -> Option<String> {
        e.present_type()
            .int_bit_width()
            .map(|_| format!("{}{}Int", self.name, filters::to_pascal_case(e.id.clone())))
    }

    /// The type one value of `e` is held as where it is one value: the class
    /// of its width for an integer, else the type of the value.
    fn one(&self, e: &ContentLineEntry) -> String {
        match e.present_type() {
            SceType::String => "str".to_string(),
            other => self
                .int_class(e)
                .unwrap_or_else(|| self.l.type_name(other).into_owned()),
        }
    }

    /// The type one value of `e` is held as in a list.
    fn item(&self, e: &ContentLineEntry) -> String {
        match e.present_type() {
            SceType::String => "str".to_string(),
            ty if ty.int_bit_width().is_some() => "int".to_string(),
            other => self.l.type_name(other).into_owned(),
        }
    }

    /// The default a required value of `e` held alone takes: an integer's
    /// zero, which its class is made of, an enum's first declared variant,
    /// else its type's zero.
    fn default_of(&self, e: &ContentLineEntry) -> String {
        if self.int_class(e).is_some() {
            "0".to_string()
        } else {
            self.l.default_expr(e.present_type())
        }
    }

    /// The type and the default of the field that holds the parameter `p`.
    fn param_declared(&self, p: &Param<'_>) -> (String, String) {
        if p.required {
            (self.one(p.entry), self.default_of(p.entry))
        } else {
            (
                format!("Optional[{}]", self.one(p.entry)),
                "None".to_string(),
            )
        }
    }

    /// The type a source presents the property `p` as, and the type and the
    /// default of the field that holds it.
    fn property_declared(&self, p: &Property<'_>) -> (String, String, String) {
        let e = p.entry;
        let list = "field(default_factory=list)".to_string();
        match p.shape {
            LineShape::Records { .. } => (
                format!("Sequence[{}]", self.record_source(e)),
                format!("List[{}]", self.record_type(e)),
                list,
            ),
            LineShape::Repeated { .. } => (
                format!("Sequence[{}]", self.item(e)),
                format!("List[{}]", self.item(e)),
                list,
            ),
            LineShape::One { values: true, .. } => {
                ("Sequence[str]".to_string(), "List[str]".to_string(), list)
            }
            LineShape::One { required: true, .. } => (self.one(e), self.one(e), self.default_of(e)),
            LineShape::One { .. } => {
                let ty = format!("Optional[{}]", self.one(e));
                (ty.clone(), ty, "None".to_string())
            }
        }
    }

    /// The statement that raises the refusal of `kind` at the entry `e`, which
    /// the totality analysis names (`Totality::require`, docs/adr/0065 §2).
    fn raise(&self, kind: RefusalKind, e: &ContentLineEntry) -> String {
        let error = self.plan.totality.require(kind, &e.id);
        let class = error.trim_start_matches("CodecError::");
        format!("raise content_line.{class}(\"{}\")", e.id)
    }

    /// The carrier of the enum value `value` of `e`.
    fn carrier(&self, e: &ContentLineEntry, value: &str) -> String {
        self.l.codec_carrier_expr(e.present_type(), value)
    }

    /// The statements that write `v`, the value of one line of `p`, each at
    /// `indent`, ending the line; `held` where `v` is held in the class of its
    /// width.
    fn write_value(&self, p: &Property<'_>, v: &str, held: bool, indent: &str) -> String {
        let e = p.entry;
        // A value the line writer can refuse: raised where it is written.
        if matches!(
            p.value,
            LineValue::Values { .. } | LineValue::Text { .. } | LineValue::Enum { .. }
        ) {
            self.plan.totality.require(RefusalKind::LineBadValue, &e.id);
        }
        let mut out = String::new();
        // An integer of a list, which no class holds, is held to its type's
        // width as it is written, as the line writer refuses an integer out
        // of its type.
        if let (false, Some((least, most))) = (held, int_span(e.present_type())) {
            out.push_str(&format!(
                "{indent}if not {least} <= {v} <= {most}:\n\
                 {indent}    raise content_line.LineBadValue(\"{}\")\n",
                e.id
            ));
        }
        let call = match &p.value {
            LineValue::Values {
                separator,
                text,
                max_size,
                max_values,
            } => format!(
                "out.strings({v}, \"{separator}\", {}, {}, {})",
                py_bool(*text),
                bound(*max_size),
                bound(*max_values)
            ),
            LineValue::Text { text, max_size } => {
                format!("out.string({v}, {}, {})", py_bool(*text), bound(*max_size))
            }
            LineValue::Enum { alias } => {
                format!("out.enum_value(_TEXTS_{alias}, {})", self.carrier(e, v))
            }
            LineValue::Bool => format!("out.boolean({v})"),
            LineValue::Uint { .. } => format!("out.uint({v})"),
            LineValue::Int { .. } => format!("out.integer({v})"),
        };
        out.push_str(&format!("{indent}{call}\n"));
        out
    }

    /// The statements that write each parameter of `p`, read off `holder`,
    /// each at `indent`: one that may be absent where it is there.
    fn write_params(&self, p: &Property<'_>, holder: &str, indent: &str) -> String {
        let mut out = String::new();
        for param in &p.params {
            // Every parameter's value is one the line writer can refuse.
            self.plan
                .totality
                .require(RefusalKind::LineBadValue, &param.entry.id);
            let name = param.entry.param.as_deref().unwrap_or_default();
            let read = format!("{holder}.{}", self.member(&param.entry.id));
            let call = |value: &str| match &param.value {
                ParamValue::Enum { alias } => format!(
                    "out.enum_param(\"{name}\", _TEXTS_{alias}, {})",
                    self.carrier(param.entry, value)
                ),
                ParamValue::Text { max_size } => {
                    format!("out.param(\"{name}\", {value}, {})", bound(*max_size))
                }
            };
            if param.required {
                out.push_str(&format!("{indent}{}\n", call(&read)));
            } else {
                out.push_str(&format!(
                    "{indent}_pv = {read}\n\
                     {indent}if _pv is not None:\n\
                     {indent}    {}\n",
                    call("_pv")
                ));
            }
        }
        out
    }

    /// The judgement of a repeated property's lines, the sequence `lines`,
    /// before any is written: none where one is required, and no more than
    /// its `sce:max-count`, where the protocol states one. A source's sequence
    /// is not held to the bound, so the encode holds it there, as every
    /// backend does.
    fn judge_lines(
        &self,
        e: &ContentLineEntry,
        required: bool,
        at_most: Option<u32>,
        lines: &str,
    ) -> String {
        let mut out = String::new();
        if required {
            out.push_str(&format!(
                "    if len({lines}) == 0:\n        {}\n",
                self.raise(RefusalKind::LineRequiredMissing, e)
            ));
        }
        if let Some(max_count) = at_most {
            out.push_str(&format!(
                "    if len({lines}) > {max_count}:\n        {}\n",
                self.raise(RefusalKind::LineTooMany, e)
            ));
        }
        out
    }

    /// The statements that write the lines of the property `p`, read off
    /// `src`.
    fn write_entry(&self, p: &Property<'_>) -> String {
        let e = p.entry;
        let name = self.member(&e.id);
        let line = format!("out.property(\"{}\")", e.property);
        let mut out = format!("    # Property `{}`.\n    v = src.{name}\n", e.property);
        match p.shape {
            LineShape::Records { required, at_most } => {
                out.push_str(&self.judge_lines(e, required, at_most, "v"));
                let value = if matches!(p.value, LineValue::Values { .. }) {
                    "values"
                } else {
                    "value"
                };
                out.push_str(&format!("    for _line in v:\n        {line}\n"));
                out.push_str(&self.write_params(p, "_line", "        "));
                out.push_str(&self.write_value(
                    p,
                    &format!("_line.{}", self.member(value)),
                    true,
                    "        ",
                ));
            }
            LineShape::Repeated { required, at_most } => {
                out.push_str(&self.judge_lines(e, required, at_most, "v"));
                out.push_str(&format!("    for _v in v:\n        {line}\n"));
                out.push_str(&self.write_value(p, "_v", false, "        "));
            }
            LineShape::One { required: true, .. } => {
                out.push_str(&format!("    {line}\n"));
                out.push_str(&self.write_params(p, "src", "    "));
                out.push_str(&self.write_value(p, "v", true, "    "));
            }
            // A property that may be absent: its line, or nothing; a parameter
            // with no value of its property has no line to stand on.
            LineShape::One {
                required: false,
                values,
            } => {
                let present = if values {
                    "len(v) != 0"
                } else {
                    "v is not None"
                };
                out.push_str(&format!("    if {present}:\n        {line}\n"));
                out.push_str(&self.write_params(p, "src", "        "));
                out.push_str(&self.write_value(p, "v", true, "        "));
                if !p.params.is_empty() {
                    let any = p
                        .params
                        .iter()
                        .map(|param| format!("src.{} is not None", self.member(&param.entry.id)))
                        .collect::<Vec<_>>()
                        .join(" or ");
                    out.push_str(&format!(
                        "    elif {any}:\n        {}\n",
                        self.raise(RefusalKind::LineRequiredMissing, e)
                    ));
                }
            }
        }
        out
    }

    /// The `Protocol` named `class` of `members`, each a name and the type a
    /// source presents it as; `what` names what presents them.
    fn protocol(class: &str, what: &str, members: &[(String, String)]) -> String {
        let mut out = format!(
            "class {class}(Protocol):\n\
             \x20   \"\"\"The elements {what} presents, which the encode reads\n\
             \x20   (docs/adr/0081 §2): any object that presents them is a source of it.\"\"\"\n"
        );
        for (name, ty) in members {
            out.push_str(&format!(
                "\n    @property\n    def {name}(self) -> {ty}: ...\n"
            ));
        }
        out.push_str("\n\n");
        out
    }

    fn render(&self) -> ContentLineSource {
        let name = self.name;
        let mut classes = String::new();
        let mut defined = BTreeSet::new();
        let mut int_class = |class: &str, e: &ContentLineEntry, classes: &mut String| {
            if !defined.insert(class.to_string()) {
                return;
            }
            let ty = e.present_type();
            let bits = ty.int_bit_width().expect("an integer has a width");
            let signed = ty.is_signed();
            classes.push_str(&format!(
                "class {class}(BoundedInt):\n\
                 \x20   \"\"\"The ``{id}`` entry of :class:`{name}`, {kind} integer of {bits}\n\
                 \x20   bits, held to them where a value of it is made (docs/adr/0081 §2).\"\"\"\n\
                 \n\
                 \x20   BITS = {bits}\n\
                 \x20   SIGNED = {}\n\n\n",
                py_bool(signed),
                id = e.id,
                kind = if signed { "a signed" } else { "an unsigned" },
            ));
        };
        let mut protocols = String::new();
        let mut members: Vec<(String, String)> = Vec::new();
        let mut hold = String::new();
        let mut declared = PythonDeclared::new();
        let mut records = BTreeMap::new();
        for p in &self.plan.properties {
            let e = p.entry;
            let (presented, ty, default) = self.property_declared(p);
            members.push((self.member(&e.id), presented));
            declared.insert(e.id.clone(), (ty, default));
            let alone = matches!(p.shape, LineShape::One { values: false, .. });
            if let (true, Some(class)) = (alone, self.int_class(e)) {
                int_class(&class, e, &mut classes);
                let made = if matches!(p.shape, LineShape::One { required: true, .. }) {
                    format!("{class}(value)")
                } else {
                    format!("None if value is None else {class}(value)")
                };
                hold.push_str(&super::hold_line(&hold, &self.member(&e.id), &made));
            }
            let params: Vec<(String, String)> = p
                .params
                .iter()
                .map(|param| {
                    let declared_param = self.param_declared(param);
                    declared.insert(param.entry.id.clone(), declared_param.clone());
                    (self.member(&param.entry.id), declared_param.0)
                })
                .collect();
            if let LineShape::Records { .. } = p.shape {
                // A record holds its parameters, then its value or values.
                let (value, value_ty, value_default) =
                    if matches!(p.value, LineValue::Values { .. }) {
                        (
                            self.member("values"),
                            "List[str]".to_string(),
                            "field(default_factory=list)".to_string(),
                        )
                    } else {
                        (self.member("value"), self.one(e), self.default_of(e))
                    };
                let presented = if matches!(p.value, LineValue::Values { .. }) {
                    "Sequence[str]".to_string()
                } else {
                    value_ty.clone()
                };
                let mut record_members = params;
                record_members.push((value.clone(), presented));
                protocols.push_str(&Self::protocol(
                    &self.record_source(e),
                    &format!("one line of ``{}``", e.property),
                    &record_members,
                ));
                let record_hold = match self.int_class(e) {
                    Some(class) if !matches!(p.value, LineValue::Values { .. }) => {
                        int_class(&class, e, &mut classes);
                        super::setattr(&super::hold_line("", &value, &format!("{class}(value)")))
                            .trim_end_matches('\n')
                            .to_string()
                    }
                    _ => String::new(),
                };
                records.insert(e.id.clone(), (value_ty, value_default, record_hold));
            } else {
                members.extend(params);
            }
        }
        protocols.push_str(&Self::protocol(
            &format!("{name}Source"),
            &format!("a value of :class:`{name}`"),
            &members,
        ));

        // The encode: each property's lines in declaration order, into a
        // buffer first where a line can refuse the value.
        let staged = !self.plan.totality.is_total();
        let mut encode = format!(
            "def encode_source(src: {name}Source, w: SceSink) -> None:\n\
             \x20   \"\"\"Write what ``src`` says as one ``{component}`` component into ``w``:\n\
             \x20   the properties present in declaration order, folded at 75 octets\n\
             \x20   (SCE_FORGE.md §4.6.4).",
            component = self.plan.lines.component
        );
        if staged {
            encode.push_str(
                " A value the lines cannot carry raises the runtime's\n\
                 \x20   typed error before a byte reaches ``w``: the component is written\n\
                 \x20   whole into a buffer first.\"\"\"\n\
                 \x20   _w = w\n\
                 \x20   _frame = bytearray()\n\
                 \x20   w = BytearraySink(_frame)\n",
            );
        } else {
            encode.push_str(" It refuses nothing.\"\"\"\n");
        }
        encode.push_str(&format!(
            "    out = content_line.ContentLineWriter(w, \"{}\")\n    out.begin()\n",
            self.plan.lines.component
        ));
        for p in &self.plan.properties {
            encode.push_str(&self.write_entry(p));
        }
        encode.push_str("    out.finish()\n");
        if staged {
            encode.push_str("    _w.write_bytes(_frame)\n");
        }

        let runtime = if defined.is_empty() {
            String::new()
        } else {
            "from sce_forge_runtime.bounded import BoundedInt".to_string()
        };
        ContentLineSource {
            before: format!("{classes}{protocols}{encode}"),
            hold: super::setattr(&hold).trim_end_matches('\n').to_string(),
            declared,
            records,
            runtime,
        }
    }
}

/// The byte case of `source`, a value of the content-line codec `plan` at
/// `path` (docs/adr/0081 §5): its dataclass made by its constructor, the values
/// of a repeated property a `list`, and each entry compared as the value holds
/// it, a list by its values in order. A component takes no flag input and no
/// caller's tag.
pub(crate) fn source_case(
    plan: &ContentLinePlan<'_>,
    path: &super::source_case::CodecPath,
    source: &serde_json::Value,
) -> Result<super::source_case::SourceCase, String> {
    let elements = super::super::source_notation::content_line_elements(plan, source)?;
    super::source_case::map_or_line_case(path, elements)
}

/// The Python text of the content-line codec `m`, whose value's class is
/// `struct_name`: the class of each integer's width, the `Protocol` of each
/// line record and of the value, the encode over any source, and how the value
/// and each line record hold their elements.
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
