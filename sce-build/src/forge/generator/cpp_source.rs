// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The C++ printer of a codec's encode plan (docs/adr/0081 §2, step 4).
//!
//! A C++ value presents its elements through accessor functions and is made
//! only by its constructor; the encode is a template over any source of the
//! codec, reading each element through a customization point (the member
//! function where the type has one, otherwise a function its namespace
//! declares) and checked by a C++17 trait (ADR 0059 §3). The printer reads the
//! model only through the plan, and lowers the plan's forms one landing at a
//! time (§6): [`unlowered`] names the first form a codec's plan uses that it
//! does not print yet, and such a codec keeps the template's encode whole.

use super::*;
use crate::forge::codec_roles::ElementId;
use crate::forge::encode_plan::{
    Absent, Bound, Clause, CodecPlan, Derive, EncodePlan, Guard, HandedFrom, Measured, Narrow,
    Number, NumberInto, Offset, Presence, Value, Write,
};

/// The first form of `plan` the C++ printer does not print yet, or `None`
/// where it prints the codec whole. `imports` are the codec's own, which say
/// whether a codec it holds is printed too.
pub(super) fn unlowered(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Option<String> {
    unlowered_forms(plan, imports).into_iter().next()
}

/// Every form of `plan` the C++ printer does not print yet, in the order
/// [`unlowered`] meets them; empty where it prints the codec whole. A codec
/// moves at the landing that lowers the last of them, so a landing is chosen
/// by the codecs whose every form it lowers.
pub(super) fn unlowered_forms(plan: &CodecPlan<'_>, imports: &[ImportContext]) -> Vec<String> {
    let plan = match plan {
        CodecPlan::Positional(plan) => plan,
        CodecPlan::Cbor(_) => return vec!["a CBOR codec".to_string()],
        CodecPlan::ContentLine(_) => return vec!["a content-line codec".to_string()],
    };
    let m = plan.model;
    let mut forms = Vec::new();
    if m.variant.is_some() {
        forms.push("a variant".to_string());
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(_) => {
                forms.push(format!("`{id}`, which is not an integer or a flag set"))
            }
            // A byte string is presented within its bound (docs/adr/0077 §3),
            // which a type holds only where the protocol states one.
            Value::Bytes(Bound { most: None, .. }) => {
                forms.push(format!("`{id}`, bytes of no stated bound"))
            }
            Value::Bytes(_) => {}
            // A text likewise, in UTF-8 bytes.
            Value::Text(Bound { most: None, .. }) => {
                forms.push(format!("`{id}`, a text of no stated bound"))
            }
            Value::Text(_) => {}
            // A body is encoded over a source of its codec, which a codec the
            // printer prints has; it is written whole, every input it takes
            // handed by what the parent binds to it.
            Value::Body { codec } => {
                if !held_printed(imports, codec) {
                    forms.push(format!(
                        "`{id}`, a body of a codec the printer does not print"
                    ));
                }
                if element.presence == Presence::Gated {
                    forms.push(format!("`{id}`, a body a gate decides"));
                }
                if plan
                    .handed
                    .get(*codec)
                    .is_some_and(|handed| handed.iter().any(|h| h.from.is_none()))
                {
                    forms.push(format!("`{id}`, a body handed an input nothing binds"));
                }
            }
            Value::List { .. } => forms.push(format!("`{id}`, a list")),
        }
        // A gated element is presented as an optional value; a number the
        // value states only while its content is absent is not yet.
        if element.presence == Presence::Stated {
            forms.push(format!(
                "`{id}`, a number the value states where its content is absent"
            ));
        }
    }
    if !plan.groups().is_empty() {
        forms.push("fields one gate decides together".to_string());
    }
    forms.extend(plan.derivations.iter().filter_map(derivation_unlowered));
    // Each field is an element the value presents, or a number the encode
    // derives whole (docs/adr/0081 §3).
    forms.extend(
        m.fields
            .iter()
            .filter(|f| {
                !plan.elements.iter().any(|e| e.field.id == f.id) && !derived_number(plan, &f.id)
            })
            .map(|field| format!("`{}`, which no element presents", field.id)),
    );
    // An input the caller hands the encode is a parameter of it; one the value
    // says (docs/adr/0065 §6 R4a) is not yet.
    if !plan.said.is_empty() {
        forms.push("a flag input the value says".to_string());
    }
    if !plan.gate_checks.is_empty() || !plan.input_checks.is_empty() || !plan.totality.is_total() {
        forms.push("an encode that can refuse".to_string());
    }
    forms.extend(
        m.fields
            .iter()
            .filter(|f| f.quantity.is_some())
            .map(|field| format!("`{}`, a physical quantity", field.id)),
    );
    let gated_element = |f: &CodecField| {
        plan.elements
            .iter()
            .any(|e| e.field.id == f.id && e.presence == Presence::Gated)
    };
    for write in &plan.program {
        match write {
            // A fixed layout has no field a gate decides; were one there, its
            // offset would not be fixed.
            Write::Packed(fields) => forms.extend(
                fields
                    .iter()
                    .filter(|f| gated_element(f))
                    .map(|f| format!("`{}`, a gated field of a fixed layout", f.id)),
            ),
            Write::Fixed(field) | Write::Tail { field, .. } if gated_element(field) => {
                forms.push(format!("`{}`, a gated field of a fixed layout", field.id))
            }
            Write::Fixed(_) | Write::Tail { .. } => {}
            Write::Streamed {
                field,
                guard,
                list: None,
            } => {
                // A streamed field is written whole, in bytes the sink writes.
                if matches!(field.bit_size, BitSize::Fixed { bits } if !matches!(bits, 8 | 16 | 32 | 64))
                {
                    forms.push(format!(
                        "`{}`, a streamed field of less than a byte",
                        field.id
                    ));
                }
                match guard {
                    Guard::Always => {}
                    // Written where the value holds it: an element it presents
                    // as an optional value.
                    Guard::Present if gated_element(field) => {}
                    Guard::Present => forms.push(format!(
                        "`{}`, a field written where a group holds it",
                        field.id
                    )),
                    // Written where its gate holds, which the printer tests on
                    // the carriers as the encode writes them and on the inputs
                    // it is handed.
                    Guard::Gate => {
                        let testable = field.present_if.as_ref().is_some_and(|gate| {
                            plan.clauses(gate)
                                .iter()
                                .all(|c| matches!(c, Clause::Flag { .. } | Clause::Input { .. }))
                        });
                        if !testable {
                            forms.push(format!("`{}`, gated on a chain's membership", field.id));
                        }
                    }
                }
            }
            Write::Streamed { field, .. } => forms.push(format!(
                "`{}`, a list or a chain walked as it is written",
                field.id
            )),
            // The variant is named once, by the model.
            Write::Variant if m.variant.is_some() => {}
            Write::Variant => forms.push("a variant".to_string()),
        }
    }
    // A field is written in its declared bits (a byte or less, or one of the
    // sink's whole widths), as the bytes it holds, as a VLE of a width the
    // sink writes, or as the body of the codec it embeds.
    forms.extend(
        m.fields
            .iter()
            .filter(|f| {
                !matches!(
                    f.bit_size,
                    BitSize::Fixed {
                        bits: 1..=8 | 16 | 32 | 64
                    } | BitSize::Tail
                        | BitSize::LengthRef
                        | BitSize::Embed
                        | BitSize::Vle {
                            width_bits: 16 | 32 | 64
                        }
                )
            })
            .map(|field| format!("`{}`, of no width the sink writes whole", field.id)),
    );
    forms
}

/// Whether the codec a value holds as `alias` is one the C++ printer prints,
/// whose namespace has the encode over any source of it (`Wire`).
fn held_printed(imports: &[ImportContext], alias: &str) -> bool {
    imports
        .iter()
        .any(|import| import.alias == alias && import.cpp_printed)
}

/// The C++ class of the codec a value holds as `alias`, and the namespace that
/// holds its `Wire` and its traits: a generated codec's class is named as its
/// namespace is (`::SCE::Generated::X::X`).
fn held_codec<'i>(imports: &'i [ImportContext], alias: &str) -> (&'i str, &'i str) {
    let import = imports
        .iter()
        .find(|import| import.alias == alias)
        .expect("`unlowered` refuses a body of a codec not imported");
    let class = import.member_type.as_str();
    let namespace = class
        .rsplit_once("::")
        .map(|(namespace, _)| namespace)
        .expect("a generated C++ codec's class is named in its namespace");
    (class, namespace)
}

/// What of the derivation `derived` the printer does not print yet, or `None`
/// for one it does:
///
/// - a flag set exactly when the one field it gates is present, or when the
///   fields one group holds are, which cannot disagree;
/// - how many bytes one byte string holds, as it is or plus or less a constant,
///   into a field it always fits (zero where a gate leaves the bytes out and the
///   field is gated beside them) or into a flag of a carrier (which keeps the
///   bits the value presents where the bytes are absent).
fn derivation_unlowered(derived: &crate::forge::encode_plan::Derived<'_>) -> Option<String> {
    let element = &derived.element;
    match (element, &derived.by) {
        (ElementId::Flag { .. }, Derive::Presence(presence)) => (presence.of.len() > 1
            && !presence.together)
            .then(|| format!("`{element}`, a flag of fields that may disagree on it")),
        (_, Derive::Number(number)) => number_unlowered(element, number),
        (ElementId::Field(id), _) => Some(format!("`{id}`, a tag or a flag the encode derives")),
        _ => Some(format!("`{element}`, a flag the encode derives")),
    }
}

/// What of the derived number `number` of `element` the printer does not
/// print yet, or `None` for one it does ([`derivation_unlowered`]).
fn number_unlowered(element: &ElementId, number: &Number<'_>) -> Option<String> {
    let [measure] = number.measures.as_slice() else {
        return Some(format!(
            "`{element}`, a number the encode derives from more than one measure"
        ));
    };
    match &measure.of {
        Measured::Bytes {
            offset: Offset::Less { held: false, .. },
            ..
        } => {
            return Some(format!(
                "`{element}`, a length less a constant that may not be held"
            ))
        }
        Measured::Bytes { .. } => {}
        // How many bytes a body encodes to, counted by its encode over a sink
        // that keeps none: a body whose encode can refuse passes the refusal
        // on, which the printer does not yet.
        Measured::Encoded { refuses: false, .. }
            if matches!(number.into, NumberInto::Field { .. }) => {}
        Measured::Encoded { refuses: false, .. } => {
            return Some(format!("`{element}`, an encoded length held in a flag"))
        }
        Measured::Encoded { refuses: true, .. } => {
            return Some(format!(
                "`{element}`, the encoded length of a body that can refuse"
            ))
        }
        _ => return Some(format!("`{element}`, a count")),
    }
    match (element, &number.into) {
        (
            ElementId::Field(_),
            NumberInto::Field {
                narrow: Narrow::Cast | Narrow::Widened,
                absent,
                ..
            },
        ) => match (measure.gated, absent) {
            (false, Absent::Never) | (true, Absent::Zero) => None,
            _ => Some(format!(
                "`{element}`, a number of content a gate decides that a group or the value \
                 states where it is absent"
            )),
        },
        (ElementId::Field(_), NumberInto::Field { .. }) => {
            Some(format!("`{element}`, a length that may not fit its field"))
        }
        (ElementId::Flag { .. }, NumberInto::Flag) => None,
        _ => Some(format!(
            "`{element}`, a number derived into what holds no number"
        )),
    }
}

/// Whether the plan derives the field `id` whole as a number, which the value
/// does not present: the encode computes it as the local `_d_<id>`, and the
/// decode reads it and drops it.
fn derived_number(plan: &EncodePlan<'_>, id: &str) -> bool {
    plan.derivations.iter().any(|derived| {
        matches!(&derived.element, ElementId::Field(element) if element == id)
            && matches!(derived.by, Derive::Number(_))
    })
}

/// The text the C++ printer gives the codec's header.
pub(super) struct CppSource {
    /// Before the value's class: the readers of each element, the trait a
    /// source satisfies, and the encode over any source (`Wire`).
    pub(super) before: String,
    /// Inside the class, where the public members were: the constructor and
    /// an accessor for each element.
    pub(super) members: String,
    /// At the end of the class: the elements it holds.
    pub(super) private: String,
    /// In the decode, once each field is read into a local of its name: the
    /// statements that make the value `_decoded` from them, refusing a byte
    /// string outside its bound as the cursor's `LengthOutOfRange`.
    pub(super) decoded: String,
}

/// The name the value's class holds the element `id` under.
pub(super) fn member(id: &str) -> String {
    format!("{id}_")
}

/// A bit-range of a carrier the encode derives, folded into the carrier's
/// value before the carrier is written.
struct Fold {
    /// The flag.
    flag: String,
    /// What the bits hold, as a `std::uint64_t`.
    bits: String,
    /// Where it is a number of bytes a gate may leave out, the test that they
    /// are there; the carrier keeps the bits the value presents where they are
    /// not.
    present: Option<String>,
}

/// One element a printed value presents.
struct Presented<'p> {
    field: &'p CodecField,
    /// The name of its accessor, of its reader in `source`, and of its
    /// constructor's parameter.
    name: String,
    /// What it is presented as where it is present.
    view: String,
    /// What the value holds it as where it is present.
    held: String,
    /// Whether it is a byte string or a text, held to its bound.
    bytes: bool,
    /// Whether it is a text, whose UTF-8 bytes are `char`s.
    text: bool,
    /// Whether a gate decides it, so that it is presented as an optional
    /// value (docs/adr/0055).
    optional: bool,
    /// Where it is the body of a codec it holds, that codec.
    body: Option<HeldCodec>,
}

/// A codec a printed value holds as a body (its class is the element's view):
/// the namespace whose `Wire` encodes it over any source of it and whose
/// `is_source` says what one is, and the alias the value imports it as.
struct HeldCodec {
    namespace: String,
    alias: String,
}

impl<'p> Presented<'p> {
    /// What it is presented as: its accessor's type, its constructor's
    /// parameter, and what a source's read of it converts to.
    fn presented(&self) -> String {
        if self.optional {
            format!("std::optional<{}>", self.view)
        } else {
            self.view.clone()
        }
    }

    /// What the value holds it as.
    fn holds(&self) -> String {
        if self.optional {
            format!("std::optional<{}>", self.held)
        } else {
            self.held.clone()
        }
    }

    /// Whether the encode reads it once, into `_e_<name>`, before it writes:
    /// a byte string is viewed within its bound, and an optional value is
    /// tested before it is written, so each is read once and kept.
    fn kept(&self) -> bool {
        self.bytes || self.optional
    }

    fn of(
        element: &crate::forge::encode_plan::Element<'p>,
        l: &LangCtx,
        imports: &[ImportContext],
    ) -> Self {
        let name = l.codec_field_id(&element.field.id);
        let optional = element.presence == Presence::Gated;
        match &element.value {
            Value::Body { codec } => {
                let (class, namespace) = held_codec(imports, codec);
                Presented {
                    field: element.field,
                    name,
                    view: class.to_string(),
                    held: class.to_string(),
                    bytes: false,
                    text: false,
                    optional,
                    body: Some(HeldCodec {
                        namespace: namespace.to_string(),
                        alias: (*codec).to_string(),
                    }),
                }
            }
            Value::Bytes(bound) | Value::Text(bound) => {
                let text = matches!(element.value, Value::Text(_));
                let most = bound
                    .most
                    .expect("`unlowered` refuses bytes or text of no stated bound");
                let bounds = if bound.least == 0 {
                    format!("{most}")
                } else {
                    format!("{most}, {}", bound.least)
                };
                let (view, list) = if text {
                    ("Text", "std::string")
                } else {
                    ("Bytes", "std::vector<std::uint8_t>")
                };
                Presented {
                    field: element.field,
                    name,
                    view: format!("::SCE::Forge::{view}<{bounds}>"),
                    held: format!("::SCE::Forge::Bounded<{list}, {bounds}>"),
                    bytes: true,
                    text,
                    optional,
                    body: None,
                }
            }
            _ => {
                let ty = l.type_name(&element.field.sce_type).to_string();
                Presented {
                    field: element.field,
                    name,
                    view: ty.clone(),
                    held: ty,
                    bytes: false,
                    text: false,
                    optional,
                    body: None,
                }
            }
        }
    }

    /// What a source's read of it must be: convertible to what it is
    /// presented as, or, for a body, a source of the codec it holds.
    fn source_requirement(&self) -> String {
        let read = format!("decltype(source::{}(std::declval<const S&>()))", self.name);
        match &self.body {
            Some(held) => format!("{}::is_source<std::decay_t<{read}>>", held.namespace),
            None => format!("std::is_convertible<{read}, {}>", self.presented()),
        }
    }

    /// Whether reading it from a source `S`, and taking it as what it is
    /// presented as, throws nothing; for a body, whether the read and every
    /// read of the body as a source of its codec throw nothing.
    fn nothrow_requirement(&self) -> String {
        let read = format!("source::{}(std::declval<const S&>())", self.name);
        match &self.body {
            Some(held) => format!(
                "noexcept({read}) && {}::is_nothrow_source<std::decay_t<decltype({read})>>::value",
                held.namespace
            ),
            None => format!(
                "noexcept(::SCE::Forge::Detail::converts_to<{}>({read}))",
                self.presented()
            ),
        }
    }
}

/// The C++ text of the codec `plan`, whose class is `struct_name`; the plan is
/// one [`unlowered`] answers `None` for.
pub(super) fn render(
    plan: &EncodePlan<'_>,
    l: &LangCtx,
    imports: &[ImportContext],
    struct_name: &str,
) -> CppSource {
    let elements: Vec<Presented<'_>> = plan
        .elements
        .iter()
        .map(|e| Presented::of(e, l, imports))
        .collect();
    let mut before = String::new();
    before.push_str(&format!(
        "/// The elements a value of `{struct_name}` presents, each read by name: the\n\
         /// member function where the type has one, otherwise a function the type's own\n\
         /// namespace declares (docs/adr/0081, step 4).\n\
         namespace source {{\n"
    ));
    for Presented { name, .. } in &elements {
        before.push_str(&format!(
            "namespace detail_{name} {{\n\
             void {name}() = delete;\n\
             template <class S>\n\
             constexpr auto read(const S& s, ::SCE::Forge::PreferMember) noexcept(noexcept(s.{name}())) -> decltype(s.{name}()) {{\n\
             \x20   return s.{name}();\n\
             }}\n\
             template <class S>\n\
             constexpr auto read(const S& s, ::SCE::Forge::PreferFree) noexcept(noexcept({name}(s))) -> decltype({name}(s)) {{\n\
             \x20   return {name}(s);\n\
             }}\n\
             struct Read {{\n\
             \x20   template <class S>\n\
             \x20   constexpr auto operator()(const S& s) const\n\
             \x20       noexcept(noexcept(read(s, ::SCE::Forge::PreferMember{{}})))\n\
             \x20       -> decltype(read(s, ::SCE::Forge::PreferMember{{}})) {{\n\
             \x20       return read(s, ::SCE::Forge::PreferMember{{}});\n\
             \x20   }}\n\
             }};\n\
             }}  // namespace detail_{name}\n\
             /// The `{name}` element of a source.\n\
             inline constexpr detail_{name}::Read {name}{{}};\n"
        ));
    }
    before.push_str("}  // namespace source\n\n");

    let names: Vec<&str> = elements.iter().map(|e| e.name.as_str()).collect();
    before.push_str(&format!(
        "/// Whether `S` presents every element of `{struct_name}`, each convertible to\n\
         /// its type (a C++17 trait, ADR 0059 §3).\n\
         template <class S, class = void>\n\
         struct is_source : std::false_type {{}};\n"
    ));
    if elements.is_empty() {
        before.push_str("template <class S>\nstruct is_source<S, void> : std::true_type {};\n\n");
    } else {
        let reads: Vec<String> = names
            .iter()
            .map(|n| format!("decltype(source::{n}(std::declval<const S&>()))"))
            .collect();
        let converts: Vec<String> = elements.iter().map(Presented::source_requirement).collect();
        before.push_str(&format!(
            "template <class S>\n\
             struct is_source<S, std::void_t<{}>>\n\
             \x20   : std::conjunction<{}> {{}};\n\n",
            reads.join(", "),
            converts.join(", ")
        ));
    }

    // The encode throws only what a source's reads throw, with the conversion
    // of each to the element's type: the sink's writes throw nothing, and the
    // generated value's accessors are `noexcept`, so its own `encode` is. A
    // type that is no source is not asked, so the encode's `static_assert` is
    // what names its missing elements.
    let nothrow = if elements.is_empty() {
        "true".to_string()
    } else {
        elements
            .iter()
            .map(Presented::nothrow_requirement)
            .collect::<Vec<_>>()
            .join(" && ")
    };
    before.push_str(&format!(
        "/// Whether reading every element of a source `S` throws nothing.\n\
         template <class S, class = void>\n\
         struct is_nothrow_source : std::false_type {{}};\n\
         template <class S>\n\
         struct is_nothrow_source<S, std::enable_if_t<is_source<S>::value>>\n\
         \x20   : std::bool_constant<{nothrow}> {{}};\n\n"
    ));
    // The flag inputs the caller hands the encode (docs/adr/0065 §6 R4a), one
    // byte each, named as the template's `encode` names them.
    let input_name = |name: &str| filters::to_snake_case(name.to_string());
    let input_params: String = plan
        .given
        .iter()
        .map(|input| format!(", std::uint8_t {}", input_name(&input.name)))
        .collect();
    before.push_str(&format!(
        "/// The encode of `{struct_name}` over any source of it: each element read\n\
         /// through `source`, written as the plan lays it out (docs/adr/0081). It\n\
         /// refuses nothing; what it returns is the sink's own failure, and it\n\
         /// throws only what a read of `src` throws.\n\
         struct Wire {{\n\
         \x20   template <class S>\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w{input_params}) noexcept(is_nothrow_source<S>::value) {{\n\
         \x20       static_assert(is_source<S>::value, \"S presents every element of {struct_name}: {}\");\n",
        if names.is_empty() { "none".to_string() } else { names.join(", ") }
    ));
    if elements.is_empty() {
        before.push_str("        (void)src;\n        (void)w;\n");
    }
    // An input is read where a gate or a held codec needs it, which some
    // layouts never do.
    for input in &plan.given {
        before.push_str(&format!("        (void){};\n", input_name(&input.name)));
    }
    // A byte string is read once and viewed within its bound, and an optional
    // value is read once and tested before it is written: what the read
    // returns lives to the end of the encode, so a view never outlives it. A
    // body is read once, as the source of its codec it is, and kept as read.
    for e in &elements {
        if e.body.is_some() {
            before.push_str(&format!(
                "        const auto& _r_{name} = source::{name}(src);\n",
                name = e.name
            ));
        } else if e.kept() {
            before.push_str(&format!(
                "        const auto& _r_{name} = source::{name}(src);\n\
                 \x20       const {presented} _e_{name} = _r_{name};\n",
                name = e.name,
                presented = e.presented()
            ));
        }
    }
    let element = |f: &CodecField| elements.iter().find(|e| e.field.id == f.id);
    let optional = |f: &CodecField| element(f).is_some_and(|e| e.optional);

    // What the encode derives (docs/adr/0081 §3): a number held whole in a
    // field, as the local `_d_<field>`, and the flags of a carrier, folded into
    // the carrier as `_c_<carrier>`, each bit-range replaced. A number is the
    // count of the bytes it measures as the wire states it, which the bytes'
    // bound keeps within what holds it; where a gate leaves the bytes out, a
    // field gated beside them holds zero and a flag keeps the bits the value
    // presents. A presence flag is set exactly when the field it gates is
    // there, or exactly when it is not for a field gated on the flag's clear
    // state; fields one group holds are there together, so the first says it.
    let mut folds: std::collections::BTreeMap<&str, Vec<Fold>> = std::collections::BTreeMap::new();
    let mut encoded: Vec<(&ElementId, &Number<'_>)> = Vec::new();
    for derived in &plan.derivations {
        match (&derived.element, &derived.by) {
            (ElementId::Flag { field, flag }, Derive::Presence(presence)) => {
                let (gated, when_clear) =
                    presence.of.first().expect("a presence flag gates a field");
                let name = l.codec_field_id(&gated.id);
                let test = if *when_clear {
                    format!("!_e_{name}.has_value()")
                } else {
                    format!("_e_{name}.has_value()")
                };
                folds.entry(field.as_str()).or_default().push(Fold {
                    flag: flag.clone(),
                    bits: format!("static_cast<std::uint64_t>({test})"),
                    present: None,
                });
            }
            (element_id, Derive::Number(number)) => {
                let [measure] = number.measures.as_slice() else {
                    unreachable!("`unlowered` refuses a number of more than one measure");
                };
                let (bytes, offset) = match &measure.of {
                    Measured::Bytes { field, offset } => (*field, offset),
                    // Counted below, once the carriers its body may be handed
                    // are as the encode writes them.
                    Measured::Encoded { .. } => {
                        encoded.push((element_id, number));
                        continue;
                    }
                    _ => unreachable!("`unlowered` refuses a count"),
                };
                let name = l.codec_field_id(&bytes.id);
                let arith = match offset {
                    Offset::None => String::new(),
                    Offset::More { by } => format!(" + {by}u"),
                    Offset::Less { by, .. } => format!(" - {by}u"),
                };
                let (size, present) = if measure.gated {
                    (
                        format!("_e_{name}->size(){arith}"),
                        Some(format!("_e_{name}.has_value()")),
                    )
                } else {
                    (format!("_e_{name}.size(){arith}"), None)
                };
                match (element_id, &number.into) {
                    (ElementId::Field(_), NumberInto::Field { field, .. }) => {
                        let ty = l.type_name(&field.sce_type);
                        let value = match &present {
                            None => format!("static_cast<{ty}>({size})"),
                            Some(there) => {
                                format!("{there} ? static_cast<{ty}>({size}) : {ty}{{0}}")
                            }
                        };
                        before.push_str(&format!(
                            "        const {ty} _d_{} = {value};\n",
                            l.codec_field_id(&field.id)
                        ));
                    }
                    (ElementId::Flag { field, flag }, NumberInto::Flag) => {
                        folds.entry(field.as_str()).or_default().push(Fold {
                            flag: flag.clone(),
                            bits: format!("static_cast<std::uint64_t>({size})"),
                            present,
                        });
                    }
                    _ => unreachable!("`unlowered` refuses a number into what holds none"),
                }
            }
            _ => unreachable!("`unlowered` refuses a derivation the printer does not lower"),
        }
    }
    for (carrier_id, flags) in &folds {
        let carrier = plan
            .model
            .fields
            .iter()
            .find(|f| f.id == *carrier_id)
            .expect("a derived flag is a flag of a carrier of the codec");
        let ty = l.type_name(&carrier.sce_type);
        let name = l.codec_field_id(&carrier.id);
        // The carrier as the source presents it, each derived bit-range
        // replaced in turn: one statement a bit-range, under the test that its
        // bytes are there where a gate may leave them out.
        let bits = format!("_f_{name}");
        before.push_str(&format!(
            "        std::uint64_t {bits} = static_cast<std::uint64_t>(source::{name}(src));\n"
        ));
        for fold in flags {
            let def = carrier
                .flags
                .iter()
                .find(|d| d.name == fold.flag)
                .expect("a derived flag is a flag of its carrier");
            let width_mask = if def.width >= 64 {
                u64::MAX
            } else {
                (1u64 << def.width) - 1
            };
            let mask = width_mask << def.bit;
            let placed = if def.bit == 0 {
                fold.bits.clone()
            } else {
                format!("({} << {})", fold.bits, def.bit)
            };
            let set = format!("{bits} = ({bits} & ~0x{mask:X}ull) | {placed};");
            before.push_str(&match &fold.present {
                None => format!("        {set}\n"),
                Some(there) => format!("        if ({there}) {{\n            {set}\n        }}\n"),
            });
        }
        before.push_str(&format!(
            "        const {ty} _c_{name} = static_cast<{ty}>({bits});\n"
        ));
    }

    // The value a field is written as: the number the encode derived, the
    // carrier with its derived flags, the present value of an optional element
    // (read only where it is tested present), or what the source presents.
    let read = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        if derived_number(plan, &f.id) {
            format!("_d_{name}")
        } else if folds.contains_key(f.id.as_str()) {
            format!("_c_{name}")
        } else if optional(f) {
            format!("(*_e_{name})")
        } else {
            format!("source::{name}(src)")
        }
    };
    // A text's UTF-8 bytes are `char`s, written as the bytes they are.
    let bytes_write = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        let at = if optional(f) { "->" } else { "." };
        let data = if element(f).is_some_and(|e| e.text) {
            format!("reinterpret_cast<const std::uint8_t*>(_e_{name}{at}data())")
        } else {
            format!("_e_{name}{at}data()")
        };
        format!(
            "        if (auto _e = w.write_bytes({data}, _e_{name}{at}size()); _e) return _e;\n"
        )
    };
    // A gate, tested on the carriers as the encode writes them and on the
    // inputs it is handed.
    let gate_test = |f: &CodecField| {
        let gate = f
            .present_if
            .as_ref()
            .expect("a field written where its gate holds is gated");
        let bit = |read: String, mask: u64, negate: bool| {
            format!(
                "((static_cast<std::uint64_t>({read}) & 0x{mask:X}ull) {} 0)",
                if negate { "==" } else { "!=" }
            )
        };
        plan.clauses(gate)
            .iter()
            .map(|clause| match clause {
                Clause::Flag {
                    carrier,
                    bit: at,
                    negate,
                } => bit(read(carrier), 1u64 << at, *negate),
                Clause::Input { name, negate } => bit(input_name(name), 1, *negate),
                Clause::Member { .. } => {
                    unreachable!("`unlowered` refuses a gate on a chain's membership")
                }
            })
            .collect::<Vec<_>>()
            .join(" || ")
    };
    // What a held codec is handed for each input it takes (docs/adr/0065 §6
    // R4a), in the order it declares them: bits of a carrier as the encode
    // writes it, or an input of this codec's own, passed on.
    let handed = |alias: &str| -> String {
        plan.handed
            .get(alias)
            .map(|inputs| {
                inputs
                    .iter()
                    .map(|handed| match &handed.from {
                        Some(HandedFrom::Carrier {
                            carrier,
                            bit,
                            width,
                        }) => {
                            let mask = if *width >= 64 {
                                u64::MAX
                            } else {
                                (1u64 << width) - 1
                            };
                            format!(
                                ", static_cast<std::uint8_t>((static_cast<std::uint64_t>({}) >> {bit}) & 0x{mask:X}ull)",
                                read(carrier)
                            )
                        }
                        Some(HandedFrom::Input { name }) => format!(", {}", input_name(name)),
                        None => unreachable!("`unlowered` refuses a body handed an input nothing binds"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    // How many bytes a body encodes to, counted by its encode over a sink that
    // keeps none; its encode refuses nothing, so the count is all it returns.
    for (element_id, number) in encoded {
        let [measure] = number.measures.as_slice() else {
            unreachable!("`unlowered` refuses a number of more than one measure");
        };
        let Measured::Encoded { body, .. } = &measure.of else {
            unreachable!("only an encoded length is counted here");
        };
        let held = element(body)
            .and_then(|e| e.body.as_ref())
            .expect("an encoded length measures a body the value presents");
        let name = l.codec_field_id(&body.id);
        before.push_str(&format!(
            "        ::SCE::Forge::CountingSink _k_{name};\n\
             \x20       (void){}::Wire::encode_source(_r_{name}, _k_{name}{});\n",
            held.namespace,
            handed(&held.alias)
        ));
        match (element_id, &number.into) {
            (ElementId::Field(_), NumberInto::Field { field, .. }) => {
                let ty = l.type_name(&field.sce_type);
                before.push_str(&format!(
                    "        const {ty} _d_{} = static_cast<{ty}>(_k_{name}.position());\n",
                    l.codec_field_id(&field.id)
                ));
            }
            _ => unreachable!("`unlowered` refuses an encoded length into what holds none"),
        }
    }
    // The statements of a guarded write, inside the test that guards it.
    let guarded = |test: String, body: String| {
        let mut out = format!("        if ({test}) {{\n");
        for line in body.lines() {
            out.push_str(&format!("    {line}\n"));
        }
        out.push_str("        }\n");
        out
    };
    for write in &plan.program {
        match write {
            Write::Packed(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        let bits = f.fixed_bits().unwrap_or(8);
                        let mask = (1u64 << bits) - 1;
                        let at = f.bit_offset.unwrap_or(0);
                        format!(
                            "((static_cast<std::uint32_t>({}) & 0x{mask:X}u) << {at})",
                            read(f)
                        )
                    })
                    .collect();
                before.push_str(&format!(
                    "        if (auto _e = w.write_u8(static_cast<std::uint8_t>({})); _e) return _e;\n",
                    parts.join(" | ")
                ));
            }
            Write::Fixed(f) => before.push_str(&fixed_write(plan, f, &read(f))),
            Write::Tail { field, pad } => {
                // A field the layout aligns follows zeros up to its offset.
                if *pad {
                    before.push_str(&format!(
                        "        while (w.position() < {}) {{\n\
                         \x20           if (auto _e = w.write_u8(0); _e) return _e;\n\
                         \x20       }}\n",
                        field.byte_offset
                    ));
                }
                before.push_str(&bytes_write(field));
            }
            Write::Streamed {
                field,
                guard,
                list: None,
            } => {
                let body = match &field.bit_size {
                    // A body, written by its codec's encode over the source the
                    // value read it as, with the inputs that codec is handed.
                    BitSize::Embed => {
                        let held = element(field)
                            .and_then(|e| e.body.as_ref())
                            .expect("an embedded field is a body the value presents");
                        format!(
                            "        if (auto _e = {}::Wire::encode_source(_r_{}, w{}); _e) return _e;\n",
                            held.namespace,
                            l.codec_field_id(&field.id),
                            handed(&held.alias)
                        )
                    }
                    BitSize::Fixed { .. } => fixed_write(plan, field, &read(field)),
                    BitSize::Tail | BitSize::LengthRef => bytes_write(field),
                    BitSize::Vle { width_bits } => format!(
                        "        if (auto _e = w.write_vle_u{width_bits}(static_cast<std::uint{width_bits}_t>({})); _e) return _e;\n",
                        read(field)
                    ),
                    _ => unreachable!("`unlowered` refuses a field of no width the sink writes"),
                };
                before.push_str(&match guard {
                    Guard::Always => body,
                    // Where the value holds it.
                    Guard::Present => guarded(format!("_e_{}", l.codec_field_id(&field.id)), body),
                    // Derived whole and gated: where its gate holds, as the
                    // carriers the encode writes say.
                    Guard::Gate => guarded(gate_test(field), body),
                });
            }
            Write::Streamed { .. } | Write::Variant => {
                unreachable!("`unlowered` refuses a plan with a write the printer does not lower")
            }
        }
    }
    before.push_str("        return std::nullopt;\n    }\n};\n\n");

    // A value that holds a byte string copies it in, which may throw and is no
    // constant expression in C++17; a value of integers alone is both.
    // A body is held by value and moved in; the codec it holds may hold bytes,
    // so a value holding a body is made as one holding bytes is.
    let holds_bytes = elements.iter().any(|e| e.bytes || e.body.is_some());
    let mut members = String::new();
    members.push_str(&format!("    {struct_name}() = default;\n"));
    if !elements.is_empty() {
        let params: Vec<String> = elements
            .iter()
            .map(|e| format!("{} {}", e.presented(), e.name))
            .collect();
        let inits: Vec<String> = elements
            .iter()
            .map(|e| {
                if e.body.is_some() {
                    format!("{}(std::move({}))", member(&e.name), e.name)
                } else {
                    format!("{}({})", member(&e.name), e.name)
                }
            })
            .collect();
        let (constexpr, noexcept) = if holds_bytes {
            ("", "")
        } else {
            ("constexpr ", " noexcept")
        };
        members.push_str(&format!(
            "    /// The value of each element it presents, in declaration order (ADR 0059 §5).\n\
             \x20   {constexpr}{struct_name}({}){noexcept}\n\
             \x20       : {} {{}}\n",
            params.join(", "),
            inits.join(", ")
        ));
    }
    for e in &elements {
        let name = &e.name;
        let held = member(name);
        // A body is presented by reference: the value holds it whole.
        if e.body.is_some() {
            members.push_str(&format!(
                "    /// The `{name}` element, the body it holds.\n\
                 \x20   [[nodiscard]] const {}& {name}() const noexcept {{ return {held}; }}\n",
                e.view
            ));
            continue;
        }
        let constexpr = if e.bytes { "" } else { "constexpr " };
        // An optional byte string is viewed where it is present.
        let body = if e.bytes && e.optional {
            format!(
                "if (!{held}) {{ return std::nullopt; }} return {}(*{held});",
                e.view
            )
        } else {
            format!("return {held};")
        };
        let what = if e.optional {
            format!("The `{name}` element, where a gate leaves it in.")
        } else {
            format!("The `{name}` element.")
        };
        members.push_str(&format!(
            "    /// {what}\n\
             \x20   [[nodiscard]] {constexpr}{} {name}() const noexcept {{ {body} }}\n",
            e.presented()
        ));
    }

    // A default-made value holds what its declared flag values give its
    // carrier (`<sce:flag value=..>`), which is what lets a variant's marked
    // default arm encode its own wire tag (RFC variant-default-uniformity);
    // a byte string holds the fewest bytes its bound admits, and every other
    // element holds its zero.
    let mut private = String::new();
    if !elements.is_empty() {
        private.push_str("\nprivate:\n");
        for e in &elements {
            let init = if !e.optional && e.field.flags.iter().any(|fl| fl.value.is_some()) {
                cpp_flag_default_init(e.field)
            } else {
                "{}".to_string()
            };
            private.push_str(&format!("    {} {}{init};\n", e.holds(), member(&e.name)));
        }
    }

    // The decode makes the value by its constructor from the locals it read:
    // a byte string through its bound's one door (where it is present, for one
    // a gate decides), and a derived number read and dropped, since the value
    // derives it. A carrier keeps the flags it was read with.
    let mut decoded = String::new();
    for e in elements.iter().filter(|e| e.bytes) {
        let (name, view) = (&e.name, &e.view);
        if e.optional {
            decoded.push_str(&format!(
                "        std::optional<{view}> _b_{name};\n\
                 \x20       if ({name}) {{\n\
                 \x20           _b_{name} = {view}::make(*{name});\n\
                 \x20           if (!_b_{name}) {{\n\
                 \x20               cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);\n\
                 \x20               return std::nullopt;\n\
                 \x20           }}\n\
                 \x20       }}\n"
            ));
        } else {
            decoded.push_str(&format!(
                "        const auto _b_{name} = {view}::make({name});\n\
                 \x20       if (!_b_{name}) {{\n\
                 \x20           cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);\n\
                 \x20           return std::nullopt;\n\
                 \x20       }}\n"
            ));
        }
    }
    for field in &plan.model.fields {
        if derived_number(plan, &field.id) {
            decoded.push_str(&format!("        (void){};\n", l.codec_field_id(&field.id)));
        }
    }
    decoded.push_str(&format!("        {struct_name} _decoded{{\n"));
    for e in &elements {
        let name = &e.name;
        let value = match (e.bytes, e.optional) {
            (true, false) => format!("*_b_{name}"),
            (true, true) => format!("_b_{name}"),
            // A body the decode read is moved into the value.
            (false, _) if e.body.is_some() => format!("std::move({name})"),
            (false, _) => name.clone(),
        };
        decoded.push_str(&format!("            /*{name}=*/{value},\n"));
    }
    decoded.push_str("        };\n");

    // Each text ends at its last line; the template places it on a line of
    // its own.
    let line = |s: String| s.trim_end_matches('\n').to_string();
    CppSource {
        before: line(before),
        members: line(members),
        private: line(private),
        decoded: line(decoded),
    }
}

/// The write of the fixed field `f`, whose value is `value`: a byte or less at
/// its bit offset, or one of the sink's whole widths in the field's byte order.
fn fixed_write(plan: &EncodePlan<'_>, f: &CodecField, value: &str) -> String {
    let bits = f.fixed_bits().unwrap_or(8);
    let endian = f.endian.unwrap_or(plan.model.default_endian);
    let call = match bits {
        b if b < 8 => {
            let at = f.bit_offset.unwrap_or(0);
            format!(
                "w.write_u8(static_cast<std::uint8_t>((static_cast<std::uint32_t>({value}) & 0x{:X}u) << {at}))",
                (1u64 << b) - 1
            )
        }
        8 => format!("w.write_u8(static_cast<std::uint8_t>({value}))"),
        b => {
            let side = if matches!(endian, Endian::Little) {
                "le"
            } else {
                "be"
            };
            format!("w.write_u{b}_{side}(static_cast<std::uint{b}_t>({value}))")
        }
    };
    format!("        if (auto _e = {call}; _e) return _e;\n")
}
