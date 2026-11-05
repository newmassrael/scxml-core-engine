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
    Absent, Bound, CodecPlan, Derive, EncodePlan, Guard, Measured, Narrow, NumberInto, Offset,
    Presence, Value, Write,
};

/// The first form of `plan` the C++ printer does not print yet, or `None`
/// where it prints the codec whole.
pub(super) fn unlowered(plan: &CodecPlan<'_>) -> Option<String> {
    let plan = match plan {
        CodecPlan::Positional(plan) => plan,
        CodecPlan::Cbor(_) => return Some("a CBOR codec".to_string()),
        CodecPlan::ContentLine(_) => return Some("a content-line codec".to_string()),
    };
    let m = plan.model;
    if m.variant.is_some() {
        return Some("a variant".to_string());
    }
    for element in &plan.elements {
        let id = &element.field.id;
        match &element.value {
            Value::Scalar(t) if t.int_bit_width().is_some() => {}
            Value::Scalar(_) => {
                return Some(format!("`{id}`, which is not an integer or a flag set"))
            }
            // A byte string is presented within its bound (docs/adr/0077 §3),
            // which a type holds only where the protocol states one.
            Value::Bytes(Bound { most: None, .. }) => {
                return Some(format!("`{id}`, bytes of no stated bound"))
            }
            Value::Bytes(_) => {}
            Value::Text(_) => return Some(format!("`{id}`, a text")),
            Value::Body { .. } => return Some(format!("`{id}`, a body")),
            Value::List { .. } => return Some(format!("`{id}`, a list")),
        }
        if element.presence != Presence::Always {
            return Some(format!("`{id}`, which a gate decides"));
        }
    }
    if !plan.groups().is_empty() {
        return Some("fields one gate decides together".to_string());
    }
    if let Some(why) = plan.derivations.iter().find_map(derivation_unlowered) {
        return Some(why);
    }
    // Each field is an element the value presents, or a length the encode
    // derives (docs/adr/0081 §3).
    if let Some(field) = m.fields.iter().find(|f| {
        !plan.elements.iter().any(|e| e.field.id == f.id) && derived_length(plan, &f.id).is_none()
    }) {
        return Some(format!("`{}`, which no element presents", field.id));
    }
    if !m.flag_inputs.is_empty() {
        return Some("a flag input".to_string());
    }
    if !plan.gate_checks.is_empty() || !plan.input_checks.is_empty() || !plan.totality.is_total() {
        return Some("an encode that can refuse".to_string());
    }
    if let Some(field) = m.fields.iter().find(|f| f.quantity.is_some()) {
        return Some(format!("`{}`, a physical quantity", field.id));
    }
    for write in &plan.program {
        match write {
            Write::Packed(_) | Write::Fixed(_) | Write::Tail { .. } => {}
            Write::Streamed {
                field,
                guard: Guard::Always,
                list: None,
            } => {
                // A streamed field is written whole, in bytes the sink writes.
                if matches!(field.bit_size, BitSize::Fixed { bits } if !matches!(bits, 8 | 16 | 32 | 64))
                {
                    return Some(format!(
                        "`{}`, a streamed field of less than a byte",
                        field.id
                    ));
                }
            }
            Write::Streamed { field, .. } => {
                return Some(format!(
                    "`{}`, a streamed field written where it is present",
                    field.id
                ))
            }
            Write::Variant => return Some("a variant".to_string()),
        }
    }
    // A field is written in its declared bits (a byte or less, or one of the
    // sink's whole widths), as the bytes it holds, or as a VLE of a width the
    // sink writes.
    if let Some(field) = m.fields.iter().find(|f| {
        !matches!(
            f.bit_size,
            BitSize::Fixed {
                bits: 1..=8 | 16 | 32 | 64
            } | BitSize::Tail
                | BitSize::LengthRef
                | BitSize::Vle {
                    width_bits: 16 | 32 | 64
                }
        )
    }) {
        return Some(format!("`{}`, of no width the sink writes whole", field.id));
    }
    None
}

/// What of the derivation `derived` the printer does not print yet, or `None`
/// for one it does: a field that holds how many bytes one byte string always
/// there holds, as it is or plus or less a constant, which always fits the
/// field.
fn derivation_unlowered(derived: &crate::forge::encode_plan::Derived<'_>) -> Option<String> {
    let ElementId::Field(id) = &derived.element else {
        return Some(format!("`{}`, a flag the encode derives", derived.element));
    };
    let Derive::Number(number) = &derived.by else {
        return Some(format!("`{id}`, a tag or a flag the encode derives"));
    };
    let [measure] = number.measures.as_slice() else {
        return Some(format!(
            "`{id}`, a number the encode derives from more than one measure"
        ));
    };
    if measure.gated {
        return Some(format!("`{id}`, a number of content a gate decides"));
    }
    match &measure.of {
        Measured::Bytes {
            offset: Offset::Less { held: false, .. },
            ..
        } => {
            return Some(format!(
                "`{id}`, a length less a constant that may not be held"
            ))
        }
        Measured::Bytes { .. } => {}
        _ => return Some(format!("`{id}`, a count or an encoded length")),
    }
    match &number.into {
        NumberInto::Field {
            narrow: Narrow::Cast | Narrow::Widened,
            absent: Absent::Never,
            ..
        } => None,
        NumberInto::Field { .. } => Some(format!("`{id}`, a length that may not fit its field")),
        NumberInto::Flag => Some(format!("`{id}`, a length held in a flag")),
    }
}

/// The byte string whose length the field `id` holds, and how the wire's
/// number differs from it, where the plan derives the field so
/// ([`derivation_unlowered`] answers `None` for it).
fn derived_length<'p>(plan: &EncodePlan<'p>, id: &str) -> Option<(&'p CodecField, Offset)> {
    plan.derivations.iter().find_map(|derived| {
        let ElementId::Field(element) = &derived.element else {
            return None;
        };
        let Derive::Number(number) = &derived.by else {
            return None;
        };
        if element != id {
            return None;
        }
        match number.measures.as_slice() {
            [measure] => match &measure.of {
                Measured::Bytes { field, offset } => Some((*field, *offset)),
                _ => None,
            },
            _ => None,
        }
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

/// One element a printed value presents.
struct Presented<'p> {
    field: &'p CodecField,
    /// The name of its accessor, of its reader in `source`, and of its
    /// constructor's parameter.
    name: String,
    /// What it is presented as: its accessor's type, its constructor's
    /// parameter, and what a source's read of it converts to.
    view: String,
    /// What the value holds it as.
    held: String,
    /// Whether it is a byte string, held to its bound.
    bytes: bool,
}

impl<'p> Presented<'p> {
    fn of(element: &crate::forge::encode_plan::Element<'p>, l: &LangCtx) -> Self {
        let name = l.codec_field_id(&element.field.id);
        match &element.value {
            Value::Bytes(bound) => {
                let most = bound
                    .most
                    .expect("`unlowered` refuses bytes of no stated bound");
                let bounds = if bound.least == 0 {
                    format!("{most}")
                } else {
                    format!("{most}, {}", bound.least)
                };
                Presented {
                    field: element.field,
                    name,
                    view: format!("::SCE::Forge::Bytes<{bounds}>"),
                    held: format!("::SCE::Forge::Bounded<std::vector<std::uint8_t>, {bounds}>"),
                    bytes: true,
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
                }
            }
        }
    }
}

/// The C++ text of the codec `plan`, whose class is `struct_name`; the plan is
/// one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> CppSource {
    let elements: Vec<Presented<'_>> = plan.elements.iter().map(|e| Presented::of(e, l)).collect();
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
        let converts: Vec<String> = elements
            .iter()
            .map(|Presented { name, view, .. }| {
                format!(
                    "std::is_convertible<decltype(source::{name}(std::declval<const S&>())), {view}>"
                )
            })
            .collect();
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
            .map(|Presented { name, view, .. }| {
                format!(
                    "noexcept(::SCE::Forge::Detail::converts_to<{view}>(source::{name}(std::declval<const S&>())))"
                )
            })
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
    before.push_str(&format!(
        "/// The encode of `{struct_name}` over any source of it: each element read\n\
         /// through `source`, written as the plan lays it out (docs/adr/0081). It\n\
         /// refuses nothing; what it returns is the sink's own failure, and it\n\
         /// throws only what a read of `src` throws.\n\
         struct Wire {{\n\
         \x20   template <class S>\n\
         \x20   [[nodiscard]] static std::optional<::SCE::Forge::CodecError> encode_source(const S& src, ::SCE::Forge::SceSink& w) noexcept(is_nothrow_source<S>::value) {{\n\
         \x20       static_assert(is_source<S>::value, \"S presents every element of {struct_name}: {}\");\n",
        if names.is_empty() { "none".to_string() } else { names.join(", ") }
    ));
    if elements.is_empty() {
        before.push_str("        (void)src;\n        (void)w;\n");
    }
    // A byte string is read once and viewed within its bound: what the read
    // returns lives to the end of the encode, so the view never outlives it.
    for Presented { name, view, .. } in elements.iter().filter(|e| e.bytes) {
        before.push_str(&format!(
            "        const auto& _r_{name} = source::{name}(src);\n\
             \x20       const {view} _e_{name} = _r_{name};\n"
        ));
    }
    // Each length the encode derives is the count of the bytes it measures,
    // as the wire states it; the bound of the bytes keeps it within its field.
    for field in &plan.model.fields {
        let Some((measured, offset)) = derived_length(plan, &field.id) else {
            continue;
        };
        let ty = l.type_name(&field.sce_type);
        let arith = match offset {
            Offset::None => String::new(),
            Offset::More { by } => format!(" + {by}u"),
            Offset::Less { by, .. } => format!(" - {by}u"),
        };
        before.push_str(&format!(
            "        const {ty} _d_{} = static_cast<{ty}>(_e_{}.size(){arith});\n",
            l.codec_field_id(&field.id),
            l.codec_field_id(&measured.id)
        ));
    }
    let read = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        if derived_length(plan, &f.id).is_some() {
            format!("_d_{name}")
        } else {
            format!("source::{name}(src)")
        }
    };
    let bytes_write = |f: &CodecField| {
        let name = l.codec_field_id(&f.id);
        format!(
            "        if (auto _e = w.write_bytes(_e_{name}.data(), _e_{name}.size()); _e) return _e;\n"
        )
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
                guard: Guard::Always,
                list: None,
            } => match &field.bit_size {
                BitSize::Fixed { .. } => before.push_str(&fixed_write(plan, field, &read(field))),
                BitSize::Tail | BitSize::LengthRef => before.push_str(&bytes_write(field)),
                BitSize::Vle { width_bits } => before.push_str(&format!(
                    "        if (auto _e = w.write_vle_u{width_bits}(static_cast<std::uint{width_bits}_t>({})); _e) return _e;\n",
                    read(field)
                )),
                _ => unreachable!("`unlowered` refuses a field of no width the sink writes"),
            },
            Write::Streamed { .. } | Write::Variant => {
                unreachable!("`unlowered` refuses a plan with a write the printer does not lower")
            }
        }
    }
    before.push_str("        return std::nullopt;\n    }\n};\n\n");

    // A value that holds a byte string copies it in, which may throw and is no
    // constant expression in C++17; a value of integers alone is both.
    let holds_bytes = elements.iter().any(|e| e.bytes);
    let mut members = String::new();
    members.push_str(&format!("    {struct_name}() = default;\n"));
    if !elements.is_empty() {
        let params: Vec<String> = elements
            .iter()
            .map(|Presented { name, view, .. }| format!("{view} {name}"))
            .collect();
        let inits: Vec<String> = elements
            .iter()
            .map(|Presented { name, .. }| format!("{}({name})", member(name)))
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
    for Presented {
        name, view, bytes, ..
    } in &elements
    {
        let constexpr = if *bytes { "" } else { "constexpr " };
        members.push_str(&format!(
            "    /// The `{name}` element.\n\
             \x20   [[nodiscard]] {constexpr}{view} {name}() const noexcept {{ return {}; }}\n",
            member(name)
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
        for Presented {
            field, name, held, ..
        } in &elements
        {
            let init = if field.flags.iter().any(|fl| fl.value.is_some()) {
                cpp_flag_default_init(field)
            } else {
                "{}".to_string()
            };
            private.push_str(&format!("    {held} {}{init};\n", member(name)));
        }
    }

    // The decode makes the value by its constructor from the locals it read:
    // a byte string through its bound's one door, and a derived length read
    // and dropped, since the value derives it.
    let mut decoded = String::new();
    for Presented { name, view, .. } in elements.iter().filter(|e| e.bytes) {
        decoded.push_str(&format!(
            "        const auto _b_{name} = {view}::make({name});\n\
             \x20       if (!_b_{name}) {{\n\
             \x20           cursor.refuse(::SCE::Forge::CodecError::LengthOutOfRange);\n\
             \x20           return std::nullopt;\n\
             \x20       }}\n"
        ));
    }
    for field in &plan.model.fields {
        if derived_length(plan, &field.id).is_some() {
            decoded.push_str(&format!("        (void){};\n", l.codec_field_id(&field.id)));
        }
    }
    decoded.push_str(&format!("        {struct_name} _decoded{{\n"));
    for Presented { name, bytes, .. } in &elements {
        let value = if *bytes {
            format!("*_b_{name}")
        } else {
            name.clone()
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
