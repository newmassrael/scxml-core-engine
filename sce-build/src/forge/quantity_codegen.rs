// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// NL→IR Mapping Roadmap Item 4 — per-language codegen helpers for the
// `raw ↔ physical` accessor pair.
//
// Each codec / transform field carrying a `Quantity { scale, offset,
// unit }` annotation emits two accessor methods alongside the raw
// member:
//
//   * `<id>_phys()`     — returns the physical reading, computed as
//                          `physical = raw * scale + offset`. The
//                          return type is `f64` (Rust / Go / C / C11),
//                          `Double` (Kotlin), or plain `float` semantics
//                          (Python).
//   * `set_<id>_phys()` — accepts a physical reading and stores the
//                          inverse `raw = (physical - offset) / scale`
//                          back into the raw member, truncated to the
//                          field's base type.
//
// Templates receive a `quantity_accessor: { ... }` payload per field
// when the field is unit-annotated; the template then conditionally
// renders the accessor pair. Fields without quantity stay byte-identical
// to the pre-Item-4 emission.
//
// **Precision contract**: `Rational::to_f64` rounds to the nearest
// double. ARXML COMPU-METHOD blocks in real automotive use cases carry
// scale factors that are exact in IEEE-754 (`1`, `0.5`, `1/256`, `1/100`,
// etc.); the v1 emission embeds the rounded `f64` literal. Authors
// needing exact rational round-tripping at non-power-of-2 denominators
// should reach for a custom helper — out of scope until a real consumer
// surfaces.

use crate::filters;
use crate::forge::model::SceType;
use crate::forge::quantity::Quantity;
use crate::generator::Language;
use serde_json::Value;

/// Build the per-field `quantity_accessor` payload that the template
/// consumes. Returns `None` when the field has no quantity annotation
/// (preserving the legacy byte-identical emission).
///
/// `raw_member_ref` is the language-specific way to read the raw
/// member from inside an accessor method on `self` / `this` / `p`:
/// `self.<id>` for Rust, `this-><id>` for C++ / C11, `<id>` for Kotlin
/// (since Kotlin members are bare inside class scope), `p.<Id>` for
/// Go, `self.<id>` for Python. The caller passes the right form for
/// the language; this helper inserts it verbatim into the conversion
/// expression.
///
/// `raw_assign_lhs` is the assignment lvalue for the setter (`self.<id>`,
/// `this-><id>`, `<id>`, `p.<Id>`, `self.<id>`). Symmetric to the
/// reader.
///
/// `base_type` is the field's `SceType`. Used by Rust / C / C11 / Go
/// to emit the back-cast (`as i8` / `(int8_t)…` / `int8(…)`).
///
/// `field_id` is the field's source identifier (un-case-converted). The
/// template will derive the accessor / setter names from it using the
/// language's own `to_snake_case` / `to_camel_case` filter where
/// applicable; we only emit the conversion expressions and the
/// physical-type token here.
pub(crate) fn build_accessor_payload(
    quantity: Quantity,
    base_type: &SceType,
    raw_member_ref: &str,
    raw_assign_lhs: &str,
    field_id: &str,
    lang: Language,
) -> Value {
    let scale_lit = float_lit(quantity.scale.to_f64(), lang);
    let offset_lit = float_lit(quantity.offset.to_f64(), lang);
    let phys_type = phys_type_token(lang);

    let raw_to_phys = raw_to_phys_expr(raw_member_ref, &scale_lit, &offset_lit, lang);
    let phys_to_raw = phys_to_raw_expr(base_type, &scale_lit, &offset_lit, lang);

    // Per-language accessor symbol names — pre-rendered server-side
    // so the template emits `{{ field.quantity_accessor.getter_name }}`
    // without an extra case-conversion filter at render time.
    let (getter_name, setter_name) = accessor_names(field_id, lang);

    let mut obj = serde_json::Map::new();
    obj.insert(
        "unit".into(),
        Value::String(quantity.unit.as_str().to_owned()),
    );
    obj.insert(
        "scale_repr".into(),
        Value::String(quantity.scale.to_string()),
    );
    obj.insert(
        "offset_repr".into(),
        Value::String(quantity.offset.to_string()),
    );
    obj.insert("scale_literal".into(), Value::String(scale_lit));
    obj.insert("offset_literal".into(), Value::String(offset_lit));
    obj.insert("phys_type".into(), Value::String(phys_type.to_owned()));
    obj.insert("getter_name".into(), Value::String(getter_name));
    obj.insert("setter_name".into(), Value::String(setter_name));
    obj.insert("raw_to_phys_expr".into(), Value::String(raw_to_phys));
    obj.insert("phys_to_raw_expr".into(), Value::String(phys_to_raw));
    obj.insert(
        "raw_assign_lhs".into(),
        Value::String(raw_assign_lhs.to_owned()),
    );
    obj.insert(
        "field_id_for_phys".into(),
        Value::String(field_id.to_owned()),
    );
    Value::Object(obj)
}

/// Per-language `(getter, setter)` symbol names. The base form is
/// `<snake>_phys` / `set_<snake>_phys`; Kotlin and Go switch to their
/// idiomatic case conventions.
fn accessor_names(field_id: &str, lang: Language) -> (String, String) {
    let snake = filters::to_snake_case(field_id.to_owned());
    let pascal = filters::to_pascal_case(field_id.to_owned());
    let camel = pascal_to_camel(&pascal);
    match lang {
        Language::Rust | Language::Cpp | Language::C11 | Language::Python => {
            (format!("{snake}_phys"), format!("set_{snake}_phys"))
        }
        Language::Kotlin => (format!("{camel}Phys"), format!("set{pascal}Phys")),
        Language::Go => (format!("{pascal}Phys"), format!("Set{pascal}Phys")),
    }
}

fn pascal_to_camel(pascal: &str) -> String {
    let mut chars = pascal.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Physical-reading return type per language. C11 and C++ both use
/// IEEE-754 double; Rust uses `f64`; Go uses `float64`; Kotlin uses
/// `Double`; Python's lazy duck typing surfaces `float` via the docstring
/// + return annotation.
fn phys_type_token(lang: Language) -> &'static str {
    match lang {
        Language::Rust => "f64",
        Language::Cpp | Language::C11 => "double",
        Language::Kotlin => "Double",
        Language::Go => "float64",
        Language::Python => "float",
    }
}

/// Emit a floating-point literal with the language's preferred form.
/// Integer-valued doubles get an explicit `.0` suffix in Rust / Go to
/// avoid type-inference ambiguity; the other languages render the
/// `f64::to_string` natively (already includes the decimal point when
/// nontrivial).
fn float_lit(value: f64, lang: Language) -> String {
    // Rust / Go expect `<int>.0` for integer-valued floats so the
    // emitted token unambiguously typechecks as `f64` / `float64`. C /
    // C++ / Kotlin / Python parse a bare `<int>` as a float when the
    // surrounding context demands one, so the bare form is fine —
    // matches each language's literal grammar.
    let s = format!("{value:?}"); // `{:?}` always emits the decimal point form for f64
    let _ = lang; // currently uniform; reserved for future per-language tuning
    s
}

fn raw_to_phys_expr(raw_member: &str, scale_lit: &str, offset_lit: &str, lang: Language) -> String {
    match lang {
        // Rust: cast the integer member to f64 before scaling so the
        // multiplication uses double-precision arithmetic; the offset
        // adds in f64 directly.
        Language::Rust => format!("{raw_member} as f64 * {scale_lit} + {offset_lit}"),
        // C / C11 / C++: explicit cast then arithmetic.
        Language::C11 | Language::Cpp => {
            format!("(double){raw_member} * {scale_lit} + {offset_lit}")
        }
        // Kotlin: `.toDouble()` is the canonical promote.
        Language::Kotlin => format!("{raw_member}.toDouble() * {scale_lit} + {offset_lit}"),
        // Go: explicit conversion using `float64(...)`.
        Language::Go => format!("float64({raw_member}) * {scale_lit} + {offset_lit}"),
        // Python: `float(...)` to be defensive even though the
        // arithmetic already promotes.
        Language::Python => format!("float({raw_member}) * {scale_lit} + {offset_lit}"),
    }
}

fn phys_to_raw_expr(
    base_type: &SceType,
    scale_lit: &str,
    offset_lit: &str,
    lang: Language,
) -> String {
    // Local-binding name agreed across all language emitters; the
    // setter signature names its physical argument `value`.
    let arg = "value";
    let inverse = format!("({arg} - {offset_lit}) / {scale_lit}");
    match lang {
        Language::Rust => {
            let cast = rust_base_cast(base_type);
            format!("({inverse}) as {cast}")
        }
        Language::Cpp => {
            let cast = c_base_cast(base_type);
            format!("static_cast<{cast}>({inverse})")
        }
        Language::C11 => {
            let cast = c_base_cast(base_type);
            format!("({cast})({inverse})")
        }
        Language::Kotlin => {
            // Kotlin numerics dispatch through extension functions:
            // .toUByte() / .toByte() / .toUShort() / ... — emitted via
            // `kotlin_base_cast`.
            let cast = kotlin_base_cast(base_type);
            format!("({inverse}).{cast}")
        }
        Language::Go => {
            let cast = go_base_cast(base_type);
            format!("{cast}({inverse})")
        }
        Language::Python => {
            // Python integer truncation is the natural inverse of
            // floor when the math is already aligned; we use `int(...)`
            // for integer base types and `float(...)` for float bases.
            if base_type.is_unsigned() || base_type.is_signed() {
                format!("int({inverse})")
            } else {
                format!("float({inverse})")
            }
        }
    }
}

fn rust_base_cast(t: &SceType) -> &'static str {
    match t {
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
        SceType::Bool | SceType::String | SceType::Bytes => "f64", /* unreachable for quantity */
        // NL→IR Item C1 Path A: Enum-typed fields cannot carry a
        // physical-quantity annotation — the variant value is a
        // discrete wire byte, not a measured numeric. Unreachable
        // for any quantity codegen path; placeholder matches the
        // existing non-numeric fallback.
        SceType::Enum(_) => "f64",
    }
}

fn c_base_cast(t: &SceType) -> &'static str {
    match t {
        SceType::Uint8 => "uint8_t",
        SceType::Uint16 => "uint16_t",
        SceType::Uint32 => "uint32_t",
        SceType::Uint64 => "uint64_t",
        SceType::Int8 => "int8_t",
        SceType::Int16 => "int16_t",
        SceType::Int32 => "int32_t",
        SceType::Int64 => "int64_t",
        SceType::Float32 => "float",
        SceType::Float64 => "double",
        SceType::Bool | SceType::String | SceType::Bytes => "double",
        // NL→IR Item C1 Path A: see `rust_base_cast` — Enum is
        // never quantity-annotated.
        SceType::Enum(_) => "double",
    }
}

fn kotlin_base_cast(t: &SceType) -> &'static str {
    match t {
        SceType::Uint8 => "toUByte()",
        SceType::Uint16 => "toUShort()",
        SceType::Uint32 => "toUInt()",
        SceType::Uint64 => "toULong()",
        SceType::Int8 => "toByte()",
        SceType::Int16 => "toShort()",
        SceType::Int32 => "toInt()",
        SceType::Int64 => "toLong()",
        SceType::Float32 => "toFloat()",
        SceType::Float64 => "toDouble()",
        SceType::Bool | SceType::String | SceType::Bytes => "toDouble()",
        // NL→IR Item C1 Path A: see `rust_base_cast`.
        SceType::Enum(_) => "toDouble()",
    }
}

fn go_base_cast(t: &SceType) -> &'static str {
    match t {
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
        SceType::Bool | SceType::String | SceType::Bytes => "float64",
        // NL→IR Item C1 Path A: see `rust_base_cast`.
        SceType::Enum(_) => "float64",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::quantity::{Rational, UnitTag};

    fn celsius() -> Quantity {
        Quantity {
            scale: Rational::parse("0.5").unwrap(),
            offset: Rational::from_int(-40),
            unit: UnitTag::intern("celsius-codegen-test"),
        }
    }

    #[test]
    fn rust_raw_to_phys_uses_f64_cast() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Int8,
            "self.raw_temp",
            "self.raw_temp",
            "raw_temp",
            Language::Rust,
        );
        let raw_to_phys = payload
            .as_object()
            .and_then(|o| o.get("raw_to_phys_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(raw_to_phys, "self.raw_temp as f64 * 0.5 + -40.0");
    }

    #[test]
    fn rust_phys_to_raw_truncates_to_base() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Int8,
            "self.raw_temp",
            "self.raw_temp",
            "raw_temp",
            Language::Rust,
        );
        let phys_to_raw = payload
            .as_object()
            .and_then(|o| o.get("phys_to_raw_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(phys_to_raw, "((value - -40.0) / 0.5) as i8");
    }

    #[test]
    fn c_cast_picks_stdint_token() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Uint16,
            "self->raw",
            "self->raw",
            "raw",
            Language::C11,
        );
        let phys_to_raw = payload
            .as_object()
            .and_then(|o| o.get("phys_to_raw_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(phys_to_raw, "(uint16_t)((value - -40.0) / 0.5)");
    }

    #[test]
    fn kotlin_emits_to_unsigned_conversion() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Uint8,
            "rawTemp",
            "rawTemp",
            "raw_temp",
            Language::Kotlin,
        );
        let phys_to_raw = payload
            .as_object()
            .and_then(|o| o.get("phys_to_raw_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(phys_to_raw, "((value - -40.0) / 0.5).toUByte()");
    }

    #[test]
    fn go_round_trip_naming() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Int16,
            "p.RawTemp",
            "p.RawTemp",
            "raw_temp",
            Language::Go,
        );
        let raw_to_phys = payload
            .as_object()
            .and_then(|o| o.get("raw_to_phys_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(raw_to_phys, "float64(p.RawTemp) * 0.5 + -40.0");
    }

    #[test]
    fn python_keeps_int_truncation_for_integer_base() {
        let payload = build_accessor_payload(
            celsius(),
            &SceType::Int8,
            "self.raw_temp",
            "self.raw_temp",
            "raw_temp",
            Language::Python,
        );
        let phys_to_raw = payload
            .as_object()
            .and_then(|o| o.get("phys_to_raw_expr"))
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(phys_to_raw, "int((value - -40.0) / 0.5)");
    }
}
