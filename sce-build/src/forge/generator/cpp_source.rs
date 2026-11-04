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
use crate::forge::encode_plan::{CodecPlan, EncodePlan, Presence, Value, Write};

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
    if let Some(element) = plan
        .elements
        .iter()
        .find(|e| !matches!(e.value, Value::Scalar(t) if t.int_bit_width().is_some()))
    {
        return Some(format!(
            "`{}`, which is not an integer or a flag set",
            element.field.id
        ));
    }
    if let Some(element) = plan
        .elements
        .iter()
        .find(|e| e.presence != Presence::Always)
    {
        return Some(format!("`{}`, which a gate decides", element.field.id));
    }
    if plan.elements.len() != m.fields.len() {
        return Some("an element the encode derives".to_string());
    }
    if !plan.derivations.is_empty() || !plan.groups().is_empty() {
        return Some("an element the encode derives".to_string());
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
    if plan
        .program
        .iter()
        .any(|w| !matches!(w, Write::Packed(_) | Write::Fixed(_)))
    {
        return Some("a field of a variable length".to_string());
    }
    // A field is written in its declared bits: a byte or less, or one of the
    // sink's whole widths. A VLE has no declared width.
    if let Some(field) = m
        .fields
        .iter()
        .find(|f| !matches!(f.fixed_bits(), Some(1..=8 | 16 | 32 | 64)))
    {
        return Some(format!("`{}`, of no width the sink writes whole", field.id));
    }
    None
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
}

/// The name the value's class holds the element `id` under.
pub(super) fn member(id: &str) -> String {
    format!("{id}_")
}

/// The C++ text of the codec `plan`, whose class is `struct_name`; the plan is
/// one [`unlowered`] answers `None` for.
pub(super) fn render(plan: &EncodePlan<'_>, l: &LangCtx, struct_name: &str) -> CppSource {
    let elements: Vec<(String, String)> = plan
        .elements
        .iter()
        .map(|e| {
            (
                l.codec_field_id(&e.field.id),
                l.type_name(&e.field.sce_type).to_string(),
            )
        })
        .collect();
    let mut before = String::new();
    before.push_str(&format!(
        "/// The elements a value of `{struct_name}` presents, each read by name: the\n\
         /// member function where the type has one, otherwise a function the type's own\n\
         /// namespace declares (docs/adr/0081, step 4).\n\
         namespace source {{\n"
    ));
    for (name, _) in &elements {
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

    let names: Vec<&str> = elements.iter().map(|(n, _)| n.as_str()).collect();
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
            .map(|(n, ty)| {
                format!(
                    "std::is_convertible<decltype(source::{n}(std::declval<const S&>())), {ty}>"
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

    // The encode throws only what a source's reads throw: the sink's writes
    // throw nothing, and the generated value's accessors are `noexcept`, so
    // its own `encode` is. A type that is no source is not asked, so the
    // encode's `static_assert` is what names its missing elements.
    let nothrow = if names.is_empty() {
        "true".to_string()
    } else {
        names
            .iter()
            .map(|n| format!("noexcept(source::{n}(std::declval<const S&>()))"))
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
    let read = |f: &CodecField| format!("source::{}(src)", l.codec_field_id(&f.id));
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
            Write::Fixed(f) => {
                let bits = f.fixed_bits().unwrap_or(8);
                let endian = f.endian.unwrap_or(plan.model.default_endian);
                let call = match bits {
                    b if b < 8 => {
                        let at = f.bit_offset.unwrap_or(0);
                        format!(
                            "w.write_u8(static_cast<std::uint8_t>((static_cast<std::uint32_t>({}) & 0x{:X}u) << {at}))",
                            read(f),
                            (1u64 << b) - 1
                        )
                    }
                    8 => format!("w.write_u8(static_cast<std::uint8_t>({}))", read(f)),
                    b => {
                        let side = if matches!(endian, Endian::Little) {
                            "le"
                        } else {
                            "be"
                        };
                        format!(
                            "w.write_u{b}_{side}(static_cast<std::uint{b}_t>({}))",
                            read(f)
                        )
                    }
                };
                before.push_str(&format!("        if (auto _e = {call}; _e) return _e;\n"));
            }
            Write::Tail { .. } | Write::Streamed { .. } | Write::Variant => {
                unreachable!("`unlowered` refuses a plan with a write the printer does not lower")
            }
        }
    }
    before.push_str("        return std::nullopt;\n    }\n};\n\n");

    let mut members = String::new();
    members.push_str(&format!("    {struct_name}() = default;\n"));
    if !elements.is_empty() {
        let params: Vec<String> = elements.iter().map(|(n, ty)| format!("{ty} {n}")).collect();
        let inits: Vec<String> = elements
            .iter()
            .map(|(n, _)| format!("{}({n})", member(n)))
            .collect();
        members.push_str(&format!(
            "    /// The value of each element it presents, in declaration order (ADR 0059 §5).\n\
             \x20   constexpr {struct_name}({}) noexcept\n\
             \x20       : {} {{}}\n",
            params.join(", "),
            inits.join(", ")
        ));
    }
    for (name, ty) in &elements {
        members.push_str(&format!(
            "    /// The `{name}` element.\n\
             \x20   [[nodiscard]] constexpr {ty} {name}() const noexcept {{ return {}; }}\n",
            member(name)
        ));
    }

    // A default-made value holds what its declared flag values give its
    // carrier (`<sce:flag value=..>`), which is what lets a variant's marked
    // default arm encode its own wire tag (RFC variant-default-uniformity);
    // every other element holds its zero.
    let mut private = String::new();
    if !elements.is_empty() {
        private.push_str("\nprivate:\n");
        for (e, (name, ty)) in plan.elements.iter().zip(&elements) {
            let init = if e.field.flags.iter().any(|fl| fl.value.is_some()) {
                cpp_flag_default_init(e.field)
            } else {
                "{}".to_string()
            };
            private.push_str(&format!("    {ty} {}{init};\n", member(name)));
        }
    }
    // Each text ends at its last line; the template places it on a line of
    // its own.
    let line = |s: String| s.trim_end_matches('\n').to_string();
    CppSource {
        before: line(before),
        members: line(members),
        private: line(private),
    }
}
