// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a content-line codec's plan (docs/adr/0081 step 4,
//! SCE_FORGE.md §4.6.4, docs/adr/0014).
//!
//! A component of content lines has no length or count that is not the shape of
//! the lines themselves, so every entry is presented, as the Rust printer
//! presents it (`rust_source::content_line`): an accessor for a property's value
//! and one for each parameter declared for it. A property that may be absent is
//! a `std::optional`; a repeated one, and the values of one line, a range (a
//! `begin` and an `end`); a repeated property that declares a parameter, or
//! whose line holds a list of values, a range of line records, each a source of
//! its own. A text is a `std::string_view`. A line refuses a value by its content
//! as well as by its size, so no bound refines a text or a list here: the encode
//! holds each where the plan's checks say.
//!
//! The encoding rule is the runtime's `ContentLine::Writer`: folding at 75
//! octets, TEXT escapes and quoting are its. Where a line can refuse a value, the
//! encode writes into a sink that only counts first (ADR 0058 §3), so a refusal
//! leaves the caller's sink untouched.

use super::super::*;
use super::source_readers;
use crate::forge::codec_totality::RefusalKind;
use crate::forge::content_line_codec::bound_literal;
use crate::forge::encode_plan::{
    ContentLinePlan, LineShape, LineValue, Param, ParamValue, Property,
};
use crate::forge::model::ContentLineEntry;
use crate::generator::Language;

/// What one element of a value, or of a line record, is.
enum Shape {
    /// One value: `view` where a source presents it, `held` where a value
    /// holds it; an optional of each where `required` is false.
    Scalar {
        view: String,
        held: String,
        required: bool,
    },
    /// A range of values, each presented as `view` and held as `held`.
    Items { view: String, held: String },
    /// A range of line records of the record class `record`, each a source of
    /// one, judged by the traits `is_{lines}` and `is_nothrow_{lines}`.
    Records { record: String, lines: String },
}

/// One element: its accessor's name, what its accessor's documentation says of
/// it, its shape, and the initializer of its member where a value made by
/// default does not hold its type's zero (an enum starts at its first declared
/// variant).
struct Element {
    name: String,
    doc: String,
    shape: Shape,
    init: String,
}

impl Element {
    /// What a source presents it as, which its accessor returns.
    fn presented(&self) -> String {
        match &self.shape {
            Shape::Scalar {
                view,
                required: true,
                ..
            } => view.clone(),
            Shape::Scalar { view, .. } => format!("std::optional<{view}>"),
            Shape::Items { held, .. } => format!("::SCE::Forge::Span<const {held}>"),
            Shape::Records { record, .. } => format!("::SCE::Forge::Span<const {record}>"),
        }
    }

    /// What a value holds it as.
    fn held(&self) -> String {
        match &self.shape {
            Shape::Scalar {
                held,
                required: true,
                ..
            } => held.clone(),
            Shape::Scalar { held, .. } => format!("std::optional<{held}>"),
            Shape::Items { held, .. } => format!("std::vector<{held}>"),
            Shape::Records { record, .. } => format!("std::vector<{record}>"),
        }
    }

    /// What one value of it is held as, where it is one value.
    fn held_value(&self) -> &str {
        match &self.shape {
            Shape::Scalar { held, .. } => held,
            Shape::Items { .. } | Shape::Records { .. } => {
                unreachable!("a parameter is one value")
            }
        }
    }

    /// The body of its accessor: a text is viewed where it is held, and a
    /// range is the span of the list that holds it.
    fn accessor_body(&self) -> String {
        match &self.shape {
            Shape::Scalar {
                view,
                required: false,
                ..
            } if view == "std::string_view" => format!(
                "if (!{name}_) {{ return std::nullopt; }} return std::string_view(*{name}_);",
                name = self.name
            ),
            Shape::Scalar { .. } => format!("return {}_;", self.name),
            Shape::Items { .. } | Shape::Records { .. } => {
                format!("return {}({}_);", self.presented(), self.name)
            }
        }
    }

    /// The member initializer its constructor's parameter of the same name
    /// gives: a copy of what it views.
    fn constructor_init(&self) -> String {
        match &self.shape {
            Shape::Items { .. } | Shape::Records { .. } => {
                format!("{name}_({name}.begin(), {name}.end())", name = self.name)
            }
            Shape::Scalar { .. } => format!("{name}_({name})", name = self.name),
        }
    }

    /// The condition a source of it meets, and the condition that reading it
    /// throws nothing; `read` reads it from `std::declval<const S&>()`.
    fn requirement(&self, read: &str) -> (String, String) {
        let r = format!("std::decay_t<decltype({read})>");
        match &self.shape {
            Shape::Scalar { .. } => (
                format!(
                    "std::is_convertible<decltype({read}), {}>",
                    self.presented()
                ),
                format!(
                    "noexcept(::SCE::Forge::Detail::converts_to<{}>({read}))",
                    self.presented()
                ),
            ),
            Shape::Items { view, .. } => (
                format!("::SCE::Forge::Detail::IsRangeOf<{r}, {view}>"),
                format!(
                    "noexcept({read}) && ::SCE::Forge::Detail::IsNothrowRangeOf<{r}, {view}>::value"
                ),
            ),
            Shape::Records { lines, .. } => (
                format!("is_{lines}<{r}>"),
                format!("noexcept({read}) && is_nothrow_{lines}<{r}>::value"),
            ),
        }
    }
}

/// A bound as the runtime's reader and writer take it.
fn bound(n: Option<u32>) -> String {
    bound_literal(Language::Cpp, n)
}

/// The C++ printer of a content-line codec's plan.
struct Printer<'a> {
    l: &'a LangCtx,
    /// `to_pascal_case(m.name)`.
    name: &'a str,
    /// The plan, whose totality every refusal written here must be in.
    plan: ContentLinePlan<'a>,
}

impl Printer<'_> {
    fn member(&self, id: &str) -> String {
        self.l.codec_field_id(id)
    }

    /// The carrier type of the enum the codec imports as `alias`, which a read
    /// text's number is cast to before the enum's conversion takes it.
    fn carrier_type(&self, alias: &str) -> String {
        self.l
            .enum_carrier_type(alias)
            .expect("the import of an enum a content line reads resolved its carrier")
    }

    /// The class of one line of the records of `e`.
    fn record_type(&self, e: &ContentLineEntry) -> String {
        format!("{}{}Line", self.name, filters::to_pascal_case(e.id.clone()))
    }

    /// The name the readers and traits of one line of the records of `e` take.
    fn record_name(&self, e: &ContentLineEntry) -> String {
        format!("{}_line", self.member(&e.id))
    }

    /// What one value of `e` is presented as, and what it is held as.
    fn value_types(&self, e: &ContentLineEntry) -> (String, String) {
        match e.present_type() {
            SceType::String => ("std::string_view".to_string(), "std::string".to_string()),
            other => {
                let ty = self.l.type_name(other).into_owned();
                (ty.clone(), ty)
            }
        }
    }

    /// The initializer of a required value of `e` a value made by default
    /// holds: an enum's first declared variant, else its type's zero.
    fn default_of(&self, e: &ContentLineEntry) -> String {
        match e.present_type() {
            SceType::Enum(r) => self.l.enum_default_expr(&r.alias),
            _ => String::new(),
        }
    }

    /// The element of the parameter `p` of the property `owner`.
    fn param_element(&self, p: &Param<'_>, owner: &ContentLineEntry) -> Element {
        let (view, held) = self.value_types(p.entry);
        let param = p.entry.param.as_deref().unwrap_or_default();
        Element {
            name: self.member(&p.entry.id),
            doc: if p.required {
                format!("Parameter `{param}` of `{}`.", owner.property)
            } else {
                format!(
                    "Parameter `{param}` of `{}`, where the line holds it.",
                    owner.property
                )
            },
            shape: Shape::Scalar {
                view,
                held,
                required: p.required,
            },
            init: if p.required {
                self.default_of(p.entry)
            } else {
                String::new()
            },
        }
    }

    /// Each element of the value, in the order its constructor takes them: a
    /// property's value, then each parameter of a property whose line is no
    /// record.
    fn elements(&self) -> Vec<Element> {
        let mut out = Vec::new();
        for p in &self.plan.properties {
            let e = p.entry;
            let property = &e.property;
            let (view, held) = self.value_types(e);
            let (shape, doc, init) = match p.shape {
                LineShape::Records { .. } => (
                    Shape::Records {
                        record: self.record_type(e),
                        lines: format!("{}s", self.record_name(e)),
                    },
                    format!("Property `{property}`: its lines, each a record."),
                    String::new(),
                ),
                LineShape::Repeated { .. } => (
                    Shape::Items { view, held },
                    format!("Property `{property}`: the value of each of its lines."),
                    String::new(),
                ),
                LineShape::One { values: true, .. } => (
                    Shape::Items {
                        view: "std::string_view".to_string(),
                        held: "std::string".to_string(),
                    },
                    format!("Property `{property}`: the values of its line."),
                    String::new(),
                ),
                LineShape::One { required, .. } => (
                    Shape::Scalar {
                        view,
                        held,
                        required,
                    },
                    if required {
                        format!("Property `{property}`.")
                    } else {
                        format!("Property `{property}`, where the component holds it.")
                    },
                    if required {
                        self.default_of(e)
                    } else {
                        String::new()
                    },
                ),
            };
            out.push(Element {
                name: self.member(&e.id),
                doc,
                shape,
                init,
            });
            if !matches!(p.shape, LineShape::Records { .. }) {
                for param in &p.params {
                    out.push(self.param_element(param, e));
                }
            }
        }
        out
    }

    /// Each element of one line of the records of `p`: its parameters, then
    /// its value or values.
    fn record_elements(&self, p: &Property<'_>) -> Vec<Element> {
        let e = p.entry;
        let mut out: Vec<Element> = p
            .params
            .iter()
            .map(|param| self.param_element(param, e))
            .collect();
        out.push(if matches!(p.value, LineValue::Values { .. }) {
            Element {
                name: self.member("values"),
                doc: "The values of the line.".to_string(),
                shape: Shape::Items {
                    view: "std::string_view".to_string(),
                    held: "std::string".to_string(),
                },
                init: String::new(),
            }
        } else {
            let (view, held) = self.value_types(e);
            Element {
                name: self.member("value"),
                doc: "The value of the line.".to_string(),
                shape: Shape::Scalar {
                    view,
                    held,
                    required: true,
                },
                init: self.default_of(e),
            }
        });
        out
    }

    /// The readers of `elements` in the namespace `ns`, and the traits
    /// `is_{source}` and `is_nothrow_{source}` that judge a source of them;
    /// `what` names what presents them.
    fn source_interface(&self, what: &str, ns: &str, source: &str, elements: &[Element]) -> String {
        let names: Vec<&str> = elements.iter().map(|e| e.name.as_str()).collect();
        let read = |e: &Element| format!("{ns}::{}(std::declval<const S&>())", e.name);
        let reads: Vec<String> = elements
            .iter()
            .map(|e| format!("decltype({})", read(e)))
            .collect();
        let (meets, nothrow): (Vec<String>, Vec<String>) =
            elements.iter().map(|e| e.requirement(&read(e))).unzip();
        format!(
            "/// The elements {what} presents, each read by name: the member function\n\
             /// where the type has one, otherwise a function the type's own namespace\n\
             /// declares (docs/adr/0081, step 4).\n\
             namespace {ns} {{\n{readers}}}  // namespace {ns}\n\n\
             /// Whether `S` presents every element {what} presents, each as its type\n\
             /// (a C++17 trait, ADR 0059 §3).\n\
             template <class S, class = void>\n\
             struct is_{source} : std::false_type {{}};\n\
             template <class S>\n\
             struct is_{source}<S, std::void_t<{reads}>>\n\
             \x20   : std::conjunction<{meets}> {{}};\n\n\
             /// Whether reading every element of a source `S` throws nothing.\n\
             template <class S, class = void>\n\
             struct is_nothrow_{source} : std::false_type {{}};\n\
             template <class S>\n\
             struct is_nothrow_{source}<S, std::enable_if_t<is_{source}<S>::value>>\n\
             \x20   : std::bool_constant<{nothrow}> {{}};\n\n",
            readers = source_readers(&names),
            reads = reads.join(", "),
            meets = meets.join(", "),
            nothrow = nothrow.join(" && ")
        )
    }

    /// The statement that refuses at the entry `e` with `kind`.
    fn refuse(&self, kind: RefusalKind, e: &ContentLineEntry) -> String {
        format!(
            "return ::SCE::Forge::{};",
            self.plan.totality.require(kind, &e.id)
        )
    }

    /// The statement that writes `v`, the value of the property `p`, on its
    /// line.
    fn write_value(&self, p: &Property<'_>, v: &str) -> String {
        let e = p.entry;
        // A value the line writer can refuse: recorded where it is written.
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
            LineValue::Enum { alias } => format!(
                "out.enum_value(kTexts_{alias}, static_cast<std::uint64_t>({}))",
                self.l.codec_carrier_expr(e.present_type(), v)
            ),
            LineValue::Bool => format!("out.boolean({v})"),
            LineValue::Uint { .. } => format!("out.uint(static_cast<std::uint64_t>({v}))"),
            LineValue::Int { .. } => format!("out.integer(static_cast<std::int64_t>({v}))"),
        };
        format!("if (auto _e = {call}; _e) return _e;")
    }

    /// The statement that writes the parameter `p`, its value `value`.
    fn write_param(&self, p: &Param<'_>, value: &str) -> String {
        // Every parameter's value is one the line writer can refuse.
        self.plan
            .totality
            .require(RefusalKind::LineBadValue, &p.entry.id);
        let param = p.entry.param.as_deref().unwrap_or_default();
        let call = match &p.value {
            ParamValue::Enum { alias } => format!(
                "out.enum_param(\"{param}\", kTexts_{alias}, static_cast<std::uint64_t>({}))",
                self.l.codec_carrier_expr(p.entry.present_type(), value)
            ),
            ParamValue::Text { max_size } => {
                format!("out.param(\"{param}\", {value}, {})", bound(*max_size))
            }
        };
        format!("if (auto _e = {call}; _e) return _e;")
    }

    /// The statements that read each parameter of `p` from `holder` through
    /// the readers of `ns`, once each, into `{prefix}{name}` as what it is
    /// presented as. What a reader returns is held by reference first, so a
    /// view of a value it returns lasts as long as the reference does.
    fn read_params(&self, p: &Property<'_>, ns: &str, holder: &str, prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        for param in &p.params {
            let element = self.param_element(param, p.entry);
            out.push(format!(
                "const auto& _r{prefix}{name} = {ns}::{name}({holder});",
                name = element.name
            ));
            out.push(format!(
                "const {ty} {prefix}{name} = _r{prefix}{name};",
                ty = element.presented(),
                name = element.name
            ));
        }
        out
    }

    /// The statements that write each parameter of `p`, read into
    /// `{prefix}{name}`: a parameter that may be absent where it is there.
    fn write_params(&self, p: &Property<'_>, prefix: &str) -> Vec<String> {
        let mut out = Vec::new();
        for param in &p.params {
            let local = format!("{prefix}{}", self.member(&param.entry.id));
            if param.required {
                out.push(self.write_param(param, &local));
            } else {
                out.push(format!("if ({local}) {{"));
                out.push(format!(
                    "    {}",
                    self.write_param(param, &format!("*{local}"))
                ));
                out.push("}".to_string());
            }
        }
        out
    }

    /// The judgement of a repeated property's lines, the range `lines`, before
    /// any is written: none where one is required, and no more than its
    /// `sce:max-count`, where the protocol states one. A source's range is not
    /// held to the bound, so the encode holds it there, as every backend does.
    fn judge_lines(
        &self,
        e: &ContentLineEntry,
        required: bool,
        at_most: Option<u32>,
        lines: &str,
    ) -> Vec<String> {
        let mut out = Vec::new();
        if required {
            out.push(format!(
                "if (!({lines}.begin() != {lines}.end())) {}",
                self.refuse(RefusalKind::LineRequiredMissing, e)
            ));
        }
        if let Some(max_count) = at_most {
            out.push("{".to_string());
            out.push("    std::size_t _n = 0;".to_string());
            out.push(format!(
                "    for (auto _it = {lines}.begin(); _it != {lines}.end(); ++_it) ++_n;"
            ));
            out.push(format!(
                "    if (_n > {max_count}) {}",
                self.refuse(RefusalKind::LineTooMany, e)
            ));
            out.push("}".to_string());
        }
        out
    }

    /// The statements that write the lines of the property `p`, read from
    /// `src`.
    fn write_entry(&self, p: &Property<'_>) -> Vec<String> {
        let e = p.entry;
        let name = self.member(&e.id);
        let line = format!(
            "if (auto _e = out.property(\"{}\"); _e) return _e;",
            e.property
        );
        let indented = |lines: Vec<String>, by: &str| -> Vec<String> {
            lines.into_iter().map(|l| format!("{by}{l}")).collect()
        };
        let mut body = vec![
            "{".to_string(),
            format!("    const auto& _r_{name} = source::{name}(src);"),
        ];
        match p.shape {
            LineShape::Records { required, at_most } => {
                body.extend(indented(
                    self.judge_lines(e, required, at_most, &format!("_r_{name}")),
                    "    ",
                ));
                let ns = format!("{}_source", self.record_name(e));
                let mut each = self.read_params(p, &ns, &format!("_l_{name}"), "_lp_");
                if matches!(p.value, LineValue::Values { .. }) {
                    each.push(format!(
                        "const auto& _v_{name} = {ns}::{}(_l_{name});",
                        self.member("values")
                    ));
                } else {
                    let (view, _) = self.value_types(e);
                    each.push(format!(
                        "const auto& _rv_{name} = {ns}::{}(_l_{name});",
                        self.member("value")
                    ));
                    each.push(format!("const {view} _v_{name} = _rv_{name};"));
                }
                each.push(line);
                each.extend(self.write_params(p, "_lp_"));
                each.push(self.write_value(p, &format!("_v_{name}")));
                body.push(format!("    for (const auto& _l_{name} : _r_{name}) {{"));
                body.extend(indented(each, "        "));
                body.push("    }".to_string());
            }
            LineShape::Repeated { required, at_most } => {
                body.extend(indented(
                    self.judge_lines(e, required, at_most, &format!("_r_{name}")),
                    "    ",
                ));
                let (view, _) = self.value_types(e);
                body.push(format!("    for (const auto& _i_{name} : _r_{name}) {{"));
                body.push(format!("        const {view} _v_{name} = _i_{name};"));
                body.push(format!("        {line}"));
                body.push(format!(
                    "        {}",
                    self.write_value(p, &format!("_v_{name}"))
                ));
                body.push("    }".to_string());
            }
            LineShape::One {
                required: true,
                values,
            } => {
                let value = if values {
                    format!("_r_{name}")
                } else {
                    let (view, _) = self.value_types(e);
                    body.push(format!("    const {view} _v_{name} = _r_{name};"));
                    format!("_v_{name}")
                };
                let mut each = self.read_params(p, "source", "src", "_p_");
                each.push(line);
                each.extend(self.write_params(p, "_p_"));
                each.push(self.write_value(p, &value));
                body.extend(indented(each, "    "));
            }
            // A property that may be absent: its line, or nothing; a parameter
            // with no value of its property has no line to stand on.
            LineShape::One {
                required: false,
                values,
            } => {
                let (present, value) = if values {
                    (
                        format!("_r_{name}.begin() != _r_{name}.end()"),
                        format!("_r_{name}"),
                    )
                } else {
                    let (view, _) = self.value_types(e);
                    body.push(format!(
                        "    const std::optional<{view}> _o_{name} = _r_{name};"
                    ));
                    (format!("_o_{name}"), format!("*_o_{name}"))
                };
                body.extend(indented(
                    self.read_params(p, "source", "src", "_p_"),
                    "    ",
                ));
                body.push(format!("    if ({present}) {{"));
                body.push(format!("        {line}"));
                body.extend(indented(self.write_params(p, "_p_"), "        "));
                body.push(format!("        {}", self.write_value(p, &value)));
                if p.params.is_empty() {
                    body.push("    }".to_string());
                } else {
                    let any = p
                        .params
                        .iter()
                        .map(|param| format!("_p_{}", self.member(&param.entry.id)))
                        .collect::<Vec<_>>()
                        .join(" || ");
                    body.push(format!("    }} else if ({any}) {{"));
                    body.push(format!(
                        "        {}",
                        self.refuse(RefusalKind::LineRequiredMissing, e)
                    ));
                    body.push("    }".to_string());
                }
            }
        }
        body.push("}".to_string());
        body
    }

    /// The constructors, accessors and members of the class `class` of
    /// `elements`, with `public` after its accessors and `private` before its
    /// members.
    fn class_members(
        &self,
        class: &str,
        elements: &[Element],
        public: &str,
        private: &str,
    ) -> String {
        let params: Vec<String> = elements
            .iter()
            .map(|e| format!("{} {}", e.presented(), e.name))
            .collect();
        let inits: Vec<String> = elements.iter().map(Element::constructor_init).collect();
        let mut out = format!(
            "    {class}() = default;\n\
             \x20   /// The value of each element it presents, in declaration order (ADR 0059 §5).\n\
             \x20   {class}({})\n\
             \x20       : {} {{}}\n",
            params.join(", "),
            inits.join(", ")
        );
        for e in elements {
            out.push_str(&format!(
                "    /// {}\n\
                 \x20   [[nodiscard]] {} {}() const noexcept {{ {} }}\n",
                e.doc,
                e.presented(),
                e.name,
                e.accessor_body()
            ));
        }
        out.push_str(public);
        out.push_str("\nprivate:\n");
        out.push_str(private);
        for e in elements {
            out.push_str(&format!("    {} {}_{{{}}};\n", e.held(), e.name, e.init));
        }
        out
    }

    /// The source interface of one line of the records of `p`, and its class.
    fn record_class(&self, p: &Property<'_>) -> String {
        let e = p.entry;
        let record = self.record_type(e);
        let name = self.record_name(e);
        let elements = self.record_elements(p);
        let mut out = self.source_interface(
            &format!("one line of `{}`", e.property),
            &format!("{name}_source"),
            &format!("{name}_source"),
            &elements,
        );
        out.push_str(&format!(
            "/// Whether `R` is a range of lines of `{property}` a source may present:\n\
             /// any range, each line a source of one.\n\
             template <class R, class = void>\n\
             struct is_{name}s : std::false_type {{}};\n\
             template <class R>\n\
             struct is_{name}s<R, std::enable_if_t<::SCE::Forge::Detail::IsRange<R>::value>>\n\
             \x20   : is_{name}_source<::SCE::Forge::Detail::ElementOf<R>> {{}};\n\n\
             /// Whether walking a range `R` of lines, and reading each, throws nothing.\n\
             template <class R, class = void>\n\
             struct is_nothrow_{name}s : std::false_type {{}};\n\
             template <class R>\n\
             struct is_nothrow_{name}s<R, std::enable_if_t<is_{name}s<R>::value>>\n\
             \x20   : std::bool_constant<::SCE::Forge::Detail::IsNothrowRange<R>::value &&\n\
             \x20                        is_nothrow_{name}_source<::SCE::Forge::Detail::ElementOf<R>>::value> {{}};\n\n\
             /// One line of `{property}` (docs/adr/0014): {what}.\n\
             struct {record} {{\n\
             {members}\
             }};\n\n",
            property = e.property,
            what = if matches!(p.value, LineValue::Values { .. }) {
                "the parameters it carries and its list of values"
            } else {
                "the parameters it carries and its value"
            },
            members = self.class_members(&record, &elements, "", "")
        ));
        out
    }

    /// The statements that read the value of one line of `p`, and the
    /// expression of what was read; a refusal returns `std::nullopt` from the
    /// decode, this backend's decode convention. The line grammar (unfolding,
    /// TEXT escapes, quoting) is the runtime's (`SCE::Forge::ContentLine`).
    fn read_value(&self, p: &Property<'_>) -> (Vec<String>, String) {
        let e = p.entry;
        match &p.value {
            LineValue::Values {
                separator,
                text,
                max_size,
                max_values,
            } => (
                vec![
                    format!(
                        "auto v = line->read_strings('{separator}', {}, {}, {text});",
                        bound(*max_values),
                        bound(*max_size)
                    ),
                    "if (!v) return std::nullopt;".to_string(),
                ],
                "std::move(*v)".to_string(),
            ),
            LineValue::Text { text, max_size } => (
                vec![
                    format!("auto v = line->read_string({}, {text});", bound(*max_size)),
                    "if (!v) return std::nullopt;".to_string(),
                ],
                "std::move(*v)".to_string(),
            ),
            LineValue::Bool => (
                vec![
                    "const auto v = line->read_bool();".to_string(),
                    "if (!v) return std::nullopt;".to_string(),
                ],
                "*v".to_string(),
            ),
            // The text names a carrier and the enum's own conversion takes it to
            // a variant, so a closed and an open enum are read by one rule
            // (docs/adr/0015): a closed enum answers `std::nullopt` for a
            // carrier it does not declare.
            LineValue::Enum { alias } => {
                let open = self.l.enum_is_open(alias);
                let mut read = vec![
                    format!("const auto raw = line->read_enum(kTexts_{alias});"),
                    "if (!raw) return std::nullopt;".to_string(),
                    format!(
                        "const auto v = {}(static_cast<{}>(*raw));",
                        self.l.enum_from_underlying(alias),
                        self.carrier_type(alias)
                    ),
                ];
                if !open {
                    read.push("if (!v) return std::nullopt;".to_string());
                }
                (read, if open { "v" } else { "*v" }.to_string())
            }
            LineValue::Uint { .. } => {
                let ty = self.l.type_name(e.present_type());
                (
                    vec![
                        format!(
                            "const auto raw = line->read_uint(static_cast<std::uint64_t>(std::numeric_limits<{ty}>::max()));"
                        ),
                        "if (!raw) return std::nullopt;".to_string(),
                    ],
                    format!("static_cast<{ty}>(*raw)"),
                )
            }
            LineValue::Int { .. } => {
                let ty = self.l.type_name(e.present_type());
                (
                    vec![
                        format!(
                            "const auto raw = line->read_int(static_cast<std::int64_t>(std::numeric_limits<{ty}>::min()), static_cast<std::int64_t>(std::numeric_limits<{ty}>::max()));"
                        ),
                        "if (!raw) return std::nullopt;".to_string(),
                    ],
                    format!("static_cast<{ty}>(*raw)"),
                )
            }
        }
    }

    /// The statements that read the parameters of one line of `p`, each into
    /// `{prefix}{name}` (a `std::optional` of what it is held as): a parameter
    /// given twice is refused, one this codec does not declare skipped.
    fn read_line_params(&self, p: &Property<'_>, prefix: &str) -> Vec<String> {
        if p.params.is_empty() {
            return Vec::new();
        }
        let mut out = vec![
            "for (;;) {".to_string(),
            "    const auto more = line->next_param();".to_string(),
            "    if (!more) return std::nullopt;".to_string(),
            "    if (!*more) break;".to_string(),
        ];
        for (i, param) in p.params.iter().enumerate() {
            let local = format!("{prefix}{}", self.member(&param.entry.id));
            let name = param.entry.param.as_deref().unwrap_or_default();
            let otherwise = if i == 0 { "" } else { "else " };
            out.push(format!("    {otherwise}if (line->param_is(\"{name}\")) {{"));
            out.push(format!("        if ({local}) return std::nullopt;"));
            match &param.value {
                ParamValue::Enum { alias } => {
                    let open = self.l.enum_is_open(alias);
                    out.push(format!(
                        "        const auto praw = line->read_param_enum(kTexts_{alias});"
                    ));
                    out.push("        if (!praw) return std::nullopt;".to_string());
                    out.push(format!(
                        "        const auto pe = {}(static_cast<{}>(*praw));",
                        self.l.enum_from_underlying(alias),
                        self.carrier_type(alias)
                    ));
                    if !open {
                        out.push("        if (!pe) return std::nullopt;".to_string());
                    }
                    out.push(format!(
                        "        {local} = {};",
                        if open { "pe" } else { "*pe" }
                    ));
                }
                ParamValue::Text { max_size } => {
                    out.push(format!(
                        "        auto pv = line->read_param_string({});",
                        bound(*max_size)
                    ));
                    out.push("        if (!pv) return std::nullopt;".to_string());
                    out.push(format!("        {local} = std::move(*pv);"));
                }
            }
            out.push("    }".to_string());
        }
        out.push("}".to_string());
        out
    }

    /// The decode: each line of a property this codec declares read in
    /// whatever order the component holds it; a second line of a property of
    /// one line, a line past its `sce:max-count` and a required property or
    /// parameter missing refused; a property this codec does not declare
    /// skipped.
    fn decode_at(&self) -> String {
        let name = self.name;
        let mut body: Vec<String> = vec![
            "namespace cl = ::SCE::Forge::ContentLine;".to_string(),
            format!(
                "auto reader = cl::Reader::begin(c.peek_slice(c.remaining()), c.remaining(), \"{}\");",
                self.plan.lines.component
            ),
            "if (!reader) return std::nullopt;".to_string(),
            format!("{name} out;"),
        ];
        // Whether a property of one line has been read, and each parameter of
        // a property whose line is no record, as it was read.
        for p in &self.plan.properties {
            if matches!(p.shape, LineShape::One { .. }) {
                body.push(format!("bool _seen_{} = false;", self.member(&p.entry.id)));
            }
            if !matches!(p.shape, LineShape::Records { .. }) {
                for param in &p.params {
                    let element = self.param_element(param, p.entry);
                    body.push(format!(
                        "std::optional<{}> _p_{};",
                        element.held_value(),
                        element.name
                    ));
                }
            }
        }
        body.extend([
            "for (;;) {".to_string(),
            "    auto line = reader->next();".to_string(),
            "    if (!line) {".to_string(),
            "        if (reader->failed()) return std::nullopt;".to_string(),
            "        break;".to_string(),
            "    }".to_string(),
        ]);
        for (i, p) in self.plan.properties.iter().enumerate() {
            let e = p.entry;
            let member = self.member(&e.id);
            let otherwise = if i == 0 { "" } else { "else " };
            body.push(format!(
                "    {otherwise}if (line->is(\"{}\")) {{",
                e.property
            ));
            let mut each: Vec<String> = Vec::new();
            let (read, value) = self.read_value(p);
            match p.shape {
                // The parameters of this line are held in the record of the
                // line, and the line is judged on them before its value is read.
                LineShape::Records { at_most, .. } => {
                    if let Some(max_count) = at_most {
                        each.push(format!(
                            "if (out.{member}_.size() >= {max_count}) return std::nullopt;"
                        ));
                    }
                    for param in &p.params {
                        let element = self.param_element(param, e);
                        each.push(format!(
                            "std::optional<{}> _lp_{};",
                            element.held_value(),
                            element.name
                        ));
                    }
                    each.extend(self.read_line_params(p, "_lp_"));
                    let mut args: Vec<String> = Vec::new();
                    for param in &p.params {
                        let local = format!("_lp_{}", self.member(&param.entry.id));
                        if param.required {
                            each.push(format!("if (!{local}) return std::nullopt;"));
                            args.push(format!("*{local}"));
                        } else {
                            args.push(local);
                        }
                    }
                    each.extend(read);
                    args.push(value);
                    each.push(format!(
                        "out.{member}_.push_back({}({}));",
                        self.record_type(e),
                        args.join(", ")
                    ));
                }
                LineShape::Repeated { at_most, .. } => {
                    if let Some(max_count) = at_most {
                        each.push(format!(
                            "if (out.{member}_.size() >= {max_count}) return std::nullopt;"
                        ));
                    }
                    each.extend(read);
                    each.push(format!("out.{member}_.push_back({value});"));
                }
                LineShape::One { .. } => {
                    each.push(format!("if (_seen_{member}) return std::nullopt;"));
                    each.extend(self.read_line_params(p, "_p_"));
                    each.extend(read);
                    each.push(format!("_seen_{member} = true;"));
                    each.push(format!("out.{member}_ = {value};"));
                }
            }
            body.extend(each.into_iter().map(|l| format!("        {l}")));
            body.push("    }".to_string());
        }
        body.push("}".to_string());
        for p in &self.plan.properties {
            let member = self.member(&p.entry.id);
            match p.shape {
                LineShape::Records { required: true, .. }
                | LineShape::Repeated { required: true, .. } => {
                    body.push(format!("if (out.{member}_.empty()) return std::nullopt;"));
                }
                LineShape::One { required: true, .. } => {
                    body.push(format!("if (!_seen_{member}) return std::nullopt;"));
                }
                LineShape::Records { .. } | LineShape::Repeated { .. } | LineShape::One { .. } => {}
            }
            if !matches!(p.shape, LineShape::Records { .. }) {
                for param in &p.params {
                    let pname = self.member(&param.entry.id);
                    if param.required {
                        body.push(format!("if (!_p_{pname}) return std::nullopt;"));
                        body.push(format!("out.{pname}_ = std::move(*_p_{pname});"));
                    } else {
                        body.push(format!("out.{pname}_ = std::move(_p_{pname});"));
                    }
                }
            }
        }
        body.push("// Only a component that decoded whole moves the cursor.".to_string());
        body.push("if (!c.advance(reader->consumed())) return std::nullopt;".to_string());
        body.push("return out;".to_string());
        format!(
            "    static std::optional<{name}> decode_at(::SCE::Forge::SceCursor& c) {{\n{}    }}\n\n",
            indent(&body, 8)
        )
    }

    /// The encode over any source: the lines of each property in declaration
    /// order, written into a sink that only counts first where a line can
    /// refuse a value.
    fn wire(&self, elements: &[Element]) -> String {
        let name = self.name;
        let mut body: Vec<String> = vec![
            "namespace cl = ::SCE::Forge::ContentLine;".to_string(),
            format!("cl::Writer out(w, \"{}\");", self.plan.lines.component),
            "if (auto _e = out.begin(); _e) return _e;".to_string(),
        ];
        for p in &self.plan.properties {
            body.extend(self.write_entry(p));
        }
        body.push("return out.finish();".to_string());
        let names: Vec<&str> = elements.iter().map(|e| e.name.as_str()).collect();
        // A component whose values a line can refuse is written into a sink
        // that only counts first, so a refusal leaves the caller's sink
        // untouched; one whose values no line refuses has nothing to find.
        let (doc, guard) = if self.plan.totality.is_total() {
            (
                "\x20   /// It refuses nothing; what it returns is the sink's own failure.\n",
                "",
            )
        } else {
            (
                "\x20   /// A value the lines cannot carry is refused before the first byte is\n\
                 \x20   /// written: the lines are written into a sink that only counts first.\n",
                "\x20       {\n\
                 \x20           ::SCE::Forge::CountingSink _count;\n\
                 \x20           if (auto _e = write_source(src, _count); _e) return _e;\n\
                 \x20       }\n",
            )
        };
        format!(
            "/// The encode of `{name}` over any source of it: the properties present in\n\
             /// declaration order, folded at 75 octets (SCE_FORGE.md §4.6.4). It throws\n\
             /// only what a read of `src` throws.\n\
             struct Wire {{\n\
             \x20   /// Encode what `src` says as one component into `w`.\n\
             {doc}\
             \x20   template <class S>\n\
             \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {{\n\
             \x20       static_assert(is_source<S>::value, \"S presents every element of {name}: {names}\");\n\
             {guard}\
             \x20       return write_source(src, w);\n\
             \x20   }}\n\n\
             private:\n\
             \x20   template <class S>\n\
             \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> write_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {{\n\
             {body}\
             \x20   }}\n\
             }};\n\n",
            names = names.join(", "),
            body = indent(&body, 8)
        )
    }

    fn text(&self) -> String {
        let name = self.name;
        let elements = self.elements();
        let mut out = String::new();
        for p in self
            .plan
            .properties
            .iter()
            .filter(|p| matches!(p.shape, LineShape::Records { .. }))
        {
            out.push_str(&self.record_class(p));
        }
        out.push_str(&self.source_interface(
            &format!("a value of `{name}`"),
            "source",
            "source",
            &elements,
        ));
        out.push_str(&self.wire(&elements));

        let total = self.plan.totality.is_total();
        let (encoded, refused, encode_to_vec) = if total {
            (
                "std::vector<std::uint8_t>",
                "",
                "        std::vector<std::uint8_t> out;\n\
                 \x20       ::SCE::Forge::VectorSink sink(out);\n\
                 \x20       (void)encode(sink);\n\
                 \x20       return out;\n",
            )
        } else {
            (
                "std::optional<std::vector<std::uint8_t>>",
                "; `std::nullopt` when encode refuses the value",
                "        std::vector<std::uint8_t> out;\n\
                 \x20       ::SCE::Forge::VectorSink sink(out);\n\
                 \x20       if (encode(sink)) {\n\
                 \x20           return std::nullopt;\n\
                 \x20       }\n\
                 \x20       return out;\n",
            )
        };
        let public = format!(
            "\n\
             \x20   /// Decode one component from `cursor`. On success the cursor advances past\n\
             \x20   /// its `END:` line; `std::nullopt` on any refusal, with the cursor left\n\
             \x20   /// where it was.\n\
             \x20   static std::optional<{name}> decode(::SCE::Forge::SceCursor& cursor) {{\n\
             \x20       ::SCE::Forge::SceCursor c = cursor;\n\
             \x20       auto value = decode_at(c);\n\
             \x20       if (value) {{\n\
             \x20           cursor = c;\n\
             \x20       }}\n\
             \x20       return value;\n\
             \x20   }}\n\n\
             \x20   /// Encode this component into `w`: the encode over any source, this value\n\
             \x20   /// one of them.\n\
             \x20   [[nodiscard]] std::optional<::SCE::Forge::CodecError> encode(::SCE::Forge::SceSink& w) const noexcept {{\n\
             \x20       return Wire::encode_source(*this, w);\n\
             \x20   }}\n\n\
             \x20   /// `encode` into a new vector{refused}.\n\
             \x20   [[nodiscard]] {encoded} encode_to_vec() const {{\n\
             {encode_to_vec}\
             \x20   }}\n"
        );
        out.push_str(&format!(
            "/// One `{component}` component of content lines (SCE_FORGE.md §4.6.4).\n\
             /// Decode reads the properties this codec declares in any order and skips the\n\
             /// rest; encode writes the properties present in declaration order, folded at\n\
             /// 75 octets. A value made by default holds each required entry at its type's\n\
             /// default and each optional one absent.\n\
             struct {name} {{\n\
             {members}\
             }};\n",
            component = self.plan.lines.component,
            members = self.class_members(name, &elements, &public, &self.decode_at())
        ));
        out
    }
}

/// The text of the printed content-line codec `m` inside its namespace: the
/// source interface and class of each line record, the source interface of the
/// value, its encode over any source, and the value, made by its constructor
/// and read by its accessors.
pub(in super::super) fn render(m: &CodecModel, l: &LangCtx, struct_name: &str) -> String {
    let printer = Printer {
        l,
        name: struct_name,
        plan: ContentLinePlan::of(m),
    };
    let text = printer.text();
    printer.plan.totality.assert_all_written(&m.name);
    text
}
