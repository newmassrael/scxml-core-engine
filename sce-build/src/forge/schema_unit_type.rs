// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The type a schema unit declares a field of a composed type in, on each
//! backend (docs/adr/0086, section 7.1, the schema-unit column).
//!
//! A field that is one entry (a number, `bool`, an enum, a bounded text or byte
//! string, or an optional one) is spelled where the unit has always spelled it;
//! a list, a record, and an optional over either are spelled here, by one walk
//! of the type, one case per constructor, so a composition of any depth is a
//! composition of the cases:
//!
//! | Backend | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|---|
//! | Rust | `BoundedList<T, N>` | `SceString<N>` / `SceBytes<N>` | `Option<T>` | its payload struct |
//! | Kotlin | `List<T>` | `String` / `ByteArray` | `T?` | its data class |
//! | C++ | `OwnedList<T, N>` | `OwnedText<N>` / `OwnedBytes<N>` | `std::optional<T>` | its struct |
//! | Go | `[]T` | `string` / `[]byte` | `optional.Of[T]` | its struct |
//! | Python | `list[T]` | `str` / `bytes` | `Optional[T]` | its record class |
//! | Swift | `[T]` | `String` / `[UInt8]` | `T?` | its struct |
//! | C11 | `{ uint32_t len; T items[N]; }` | `sce_static_string_<N>_t` / `{ uint32_t len; uint8_t data[N]; }` | `{ bool present; T value; }` | its struct |
//!
//! A union (docs/adr/0092, section 7) is spelled on a backend that lowers one
//! ([`crate::forge::codegen_matrix::UNION_BACKENDS`]) as the sum type its own
//! unit declares, named as the import names the union's unit: on Rust the enum
//! through its sibling module (`union_shape::UnionShape`), on Kotlin the sealed
//! interface, on Go the value struct through the package its import binds
//! (`sce_union_shape.UnionShape`), on C++ the struct through its namespace
//! (`SCE::Generated::UnionShape::UnionShape`), on Python the class through the
//! module its import binds (`sce_union_shape.UnionShape`), on C11 the typedef
//! its header declares (`union_shape_t`), on Swift the enum bare in the module.
//! Every other backend refuses it by name.
//!
//! Rust's text, byte string and list inside a composition are the bounded types
//! of `sce-portable-bytes`, whose every constructor refuses a value past its
//! bound on every profile (sections 5.4 and 7.4), the types a machine holds
//! the same values in; C++'s are the forge runtime's owned `Bounded` forms
//! (`sce/forge/bounded.h`), which hold the same bounds for the same reason.
//! C11 lays every composition out
//! statically: each distinct one is one named type, its name mangled from its
//! structure (`sce_list_3_list_4_string_8_t`), declared once under a guard
//! named by it, so the same composition is one type across the units of a
//! build (section 7.2).

use std::collections::BTreeSet;

use crate::forge::model::SceType;
use crate::forge::value_type::ValueType;
use crate::generator::Language;

/// What a unit's type names from outside it, as the unit's language spells it.
pub(crate) trait UnitNames {
    /// The type a value of the scalar `ty` (a number, `bool`, or a value of an
    /// enum the unit imports) is held in.
    fn scalar(&self, ty: &SceType) -> String;
    /// The payload type of the record the import `alias` names, or `None` when
    /// no event-schema import goes by that alias.
    fn record(&self, alias: &str) -> Option<String>;
    /// The sum type the unit of the union the import `alias` names declares,
    /// or `None` when no union import goes by that alias.
    fn union(&self, alias: &str) -> Option<String>;
    /// The name the enum the import `alias` names is mangled by in a C11
    /// composition's type name: its document, so one enum is one name whatever
    /// alias a unit imports it under.
    fn enum_mangle(&self, alias: &str) -> String;
}

/// The C11 declarations a unit makes for the compositions its fields hold:
/// the bounds of the texts it holds (declared by the buffer every header of
/// that bound shares), and each composition's type, once, an inner one before
/// the outer one that names it.
#[derive(Debug, Default)]
pub(crate) struct C11Declarations {
    /// The bound of each text a composition holds.
    pub(crate) texts: BTreeSet<u32>,
    /// Each composition's declaration, under its guard, in the order they are
    /// written.
    pub(crate) defs: Vec<String>,
    declared: BTreeSet<String>,
}

impl C11Declarations {
    fn declare(&mut self, mangled: &str, body: String) -> String {
        let name = format!("sce_{mangled}_t");
        if self.declared.insert(mangled.to_string()) {
            let guard = format!("SCE_{}_DECLARED", mangled.to_uppercase());
            self.defs.push(format!(
                "#ifndef {guard}\n#define {guard}\ntypedef struct {{\n{body}}} {name};\n#endif"
            ));
        }
        name
    }
}

/// The type a unit of `lang` holds a value of `ty` in, at any depth, and on C11
/// the declarations it names gathered in `c11`. Refused, with the sentence, when
/// the type names a record no import of the unit resolves.
pub(crate) fn unit_type(
    lang: Language,
    ty: &ValueType,
    names: &dyn UnitNames,
    c11: &mut C11Declarations,
) -> Result<String, String> {
    let record = |alias: &str| {
        names
            .record(alias)
            .ok_or_else(|| format!("`record:{alias}` names no event schema the document imports"))
    };
    // Every backend lowers a union (docs/adr/0092, section 7,
    // `codegen_matrix::UNION_BACKENDS`) and names the sum type of its unit.
    let union_type = |alias: &str| {
        names
            .union(alias)
            .ok_or_else(|| format!("`union:{alias}` names no union the document imports"))
    };
    let inner = |elem: &ValueType, c11: &mut C11Declarations| unit_type(lang, elem, names, c11);
    Ok(match lang {
        Language::Rust => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { max } => format!("::sce_portable_bytes::SceString<{}>", max.max),
            ValueType::Bytes { max } => format!("::sce_portable_bytes::SceBytes<{}>", max.max),
            ValueType::Optional { elem } => format!("Option<{}>", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, max } => format!(
                "::sce_portable_bytes::BoundedList<{}, {}>",
                inner(elem, c11)?,
                max.max
            ),
        },
        Language::Kotlin => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { .. } => "String".to_string(),
            ValueType::Bytes { .. } => "ByteArray".to_string(),
            ValueType::Optional { elem } => format!("{}?", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, .. } => format!("List<{}>", inner(elem, c11)?),
        },
        Language::Cpp => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { max } => crate::forge::composed_lowering::cpp_text_type(max.max),
            ValueType::Bytes { max } => crate::forge::composed_lowering::cpp_bytes_type(max.max),
            ValueType::Optional { elem } => format!("std::optional<{}>", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, max } => {
                crate::forge::composed_lowering::cpp_bounded_list(&inner(elem, c11)?, max.max)
            }
        },
        Language::Go => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { .. } => "string".to_string(),
            ValueType::Bytes { .. } => "[]byte".to_string(),
            ValueType::Optional { elem } => format!("optional.Of[{}]", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, .. } => format!("[]{}", inner(elem, c11)?),
        },
        Language::Python => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { .. } => "str".to_string(),
            ValueType::Bytes { .. } => "bytes".to_string(),
            ValueType::Optional { elem } => format!("Optional[{}]", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, .. } => format!("list[{}]", inner(elem, c11)?),
        },
        Language::Swift => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { .. } => "String".to_string(),
            ValueType::Bytes { .. } => "[UInt8]".to_string(),
            ValueType::Optional { elem } => format!("{}?", inner(elem, c11)?),
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, .. } => format!("[{}]", inner(elem, c11)?),
        },
        Language::C11 => match ty {
            ValueType::Scalar(t) => names.scalar(t),
            ValueType::Text { max } => {
                c11.texts.insert(max.max);
                crate::forge::static_lowering::c_string_storage_type(max.max)
            }
            ValueType::Bytes { max } => c11.declare(
                &c11_mangle(ty, names),
                format!("    uint32_t len;\n    uint8_t data[{}];\n", max.max),
            ),
            ValueType::Optional { elem } => {
                let value = inner(elem, c11)?;
                c11.declare(
                    &c11_mangle(ty, names),
                    format!("    bool present;\n    {value} value;\n"),
                )
            }
            ValueType::Record { alias } => record(alias)?,
            ValueType::Union { alias } => union_type(alias)?,
            ValueType::List { elem, max } => {
                let item = inner(elem, c11)?;
                c11.declare(
                    &c11_mangle(ty, names),
                    format!("    uint32_t len;\n    {item} items[{}];\n", max.max),
                )
            }
        },
    })
}

/// The name a C11 composition's type is mangled from, by its structure: one
/// word per constructor and its bound, outermost first, as
/// `list_3_list_4_string_8`. An optional is `opt_`, which no type
/// `sce/forge/optional.h` declares begins with, so a composition's optional
/// and the runtime's optional scalar never share a name.
fn c11_mangle(ty: &ValueType, names: &dyn UnitNames) -> String {
    match ty {
        ValueType::Scalar(SceType::Enum(reference)) => {
            format!("enum_{}", names.enum_mangle(&reference.alias))
        }
        ValueType::Scalar(t) => t.as_attr(),
        ValueType::Text { max } => format!("string_{}", max.max),
        ValueType::Bytes { max } => format!("bytes_{}", max.max),
        ValueType::Optional { elem } => format!("opt_{}", c11_mangle(elem, names)),
        ValueType::Record { alias } => format!(
            "record_{}",
            names
                .record(alias)
                .unwrap_or_else(|| alias.to_string())
                .trim_end_matches("_t")
        ),
        // By the type its unit declares, as a record is, so one union is one
        // name whatever alias a unit imports it under (docs/adr/0092, section 7).
        ValueType::Union { alias } => format!(
            "union_{}",
            names
                .union(alias)
                .unwrap_or_else(|| alias.to_string())
                .trim_end_matches("_t")
        ),
        ValueType::List { elem, max } => format!("list_{}_{}", max.max, c11_mangle(elem, names)),
    }
}

/// The value a Swift unit's field of the composed type `ty` holds until it is
/// written: an empty list, nothing, or a record at its own zero. A union has
/// none (docs/adr/0092, amendment of 2026-11-07: no case is the zero of the
/// others), so a field that holds one directly answers the empty text, and its
/// unit is built whole by its memberwise initialiser and is no `SceDefault`.
pub(crate) fn swift_zero(ty: &ValueType, names: &dyn UnitNames) -> Result<String, String> {
    Ok(match ty {
        ValueType::List { .. } => "[]".to_string(),
        ValueType::Optional { .. } => "nil".to_string(),
        ValueType::Record { alias } => format!(
            "{}()",
            names.record(alias).ok_or_else(|| {
                format!("`record:{alias}` names no event schema the document imports")
            })?
        ),
        ValueType::Union { .. } => String::new(),
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => {
            return Err(format!(
                "`{ty}` is one entry, which the unit spells where it spells every entry"
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::WrittenType;

    struct Names;

    impl UnitNames for Names {
        fn scalar(&self, ty: &SceType) -> String {
            match ty {
                SceType::Enum(r) => format!("E{}", r.alias),
                other => other.as_attr(),
            }
        }
        fn record(&self, alias: &str) -> Option<String> {
            (alias == "Item").then(|| "ItemPayload_t".to_string())
        }
        fn union(&self, alias: &str) -> Option<String> {
            (alias == "Shape").then(|| "UnionShape".to_string())
        }
        fn enum_mangle(&self, alias: &str) -> String {
            format!("doc_{}", alias.to_lowercase())
        }
    }

    fn read(text: &str) -> ValueType {
        WrittenType::read(text)
            .expect("a type")
            .bounded()
            .expect("every bound stated")
    }

    fn spelled(lang: Language, text: &str) -> String {
        unit_type(lang, &read(text), &Names, &mut C11Declarations::default()).expect("spelled")
    }

    #[test]
    fn each_backend_spells_a_composition_by_its_constructors() {
        let ty = "list<optional<list<string[8]>[4]>>[3]";
        for (lang, want) in [
            (
                Language::Rust,
                "::sce_portable_bytes::BoundedList<Option<::sce_portable_bytes::BoundedList<\
                 ::sce_portable_bytes::SceString<8>, 4>>, 3>",
            ),
            (Language::Kotlin, "List<List<String>?>"),
            (
                Language::Cpp,
                "::SCE::Forge::OwnedList<std::optional<::SCE::Forge::OwnedList<\
                 ::SCE::Forge::OwnedText<8>, 4>>, 3>",
            ),
            (Language::Go, "[]optional.Of[[]string]"),
            (Language::Python, "list[Optional[list[str]]]"),
            (Language::Swift, "[[String]?]"),
            (Language::C11, "sce_list_3_opt_list_4_string_8_t"),
        ] {
            assert_eq!(spelled(lang, ty), want, "{lang:?}");
        }
    }

    #[test]
    fn a_record_and_an_enum_are_named_by_the_unit() {
        assert_eq!(
            spelled(Language::Kotlin, "list<record:Item>[2]"),
            "List<ItemPayload_t>"
        );
        assert_eq!(spelled(Language::Swift, "list<enum:Level>[2]"), "[ELevel]");
        assert!(unit_type(
            Language::Rust,
            &read("record:Other"),
            &Names,
            &mut C11Declarations::default()
        )
        .is_err());
    }

    /// Kotlin names a union by the sealed interface its unit declares, Rust by
    /// its enum and Go by its value struct, at any depth; a union no import
    /// names is refused, and a backend that does not lower a union refuses it
    /// by name (docs/adr/0092, section 7).
    #[test]
    fn kotlin_names_a_union_by_its_units_sum_and_the_rest_refuse_it() {
        assert_eq!(spelled(Language::Kotlin, "union:Shape"), "UnionShape");
        assert_eq!(
            spelled(Language::Kotlin, "list<optional<union:Shape>>[3]"),
            "List<UnionShape?>"
        );
        let unnamed = unit_type(
            Language::Kotlin,
            &read("union:Other"),
            &Names,
            &mut C11Declarations::default(),
        )
        .expect_err("no import names the union");
        assert!(unnamed.contains("`union:Other`"), "{unnamed}");
        // Rust names the same sum inside its bounded list and its optional.
        assert_eq!(spelled(Language::Rust, "union:Shape"), "UnionShape");
        assert_eq!(
            spelled(Language::Rust, "list<optional<union:Shape>>[3]"),
            "::sce_portable_bytes::BoundedList<Option<UnionShape>, 3>"
        );
        // Go names its value struct inside its slice and the runtime's
        // optional (docs/adr/0092, section 7 item 4).
        assert_eq!(spelled(Language::Go, "union:Shape"), "UnionShape");
        assert_eq!(
            spelled(Language::Go, "list<optional<union:Shape>>[3]"),
            "[]optional.Of[UnionShape]"
        );
        let unnamed = unit_type(
            Language::Go,
            &read("union:Other"),
            &Names,
            &mut C11Declarations::default(),
        )
        .expect_err("no import names the union");
        assert!(unnamed.contains("`union:Other`"), "{unnamed}");
        // C++ names its struct inside its bounded list and `std::optional`
        // (docs/adr/0092, section 7, the C++ row).
        assert_eq!(spelled(Language::Cpp, "union:Shape"), "UnionShape");
        assert_eq!(
            spelled(Language::Cpp, "list<optional<union:Shape>>[3]"),
            "::SCE::Forge::OwnedList<std::optional<UnionShape>, 3>"
        );
        // Python names its class inside `list` and `Optional` (docs/adr/0092,
        // section 7, the Python row).
        assert_eq!(spelled(Language::Python, "union:Shape"), "UnionShape");
        assert_eq!(
            spelled(Language::Python, "list<optional<union:Shape>>[3]"),
            "list[Optional[UnionShape]]"
        );
        // C11 names the typedef its unit declares, and a composition over it by
        // that type, inside the struct of each composition.
        assert_eq!(spelled(Language::C11, "union:Shape"), "UnionShape");
        let mut c11 = C11Declarations::default();
        assert_eq!(
            unit_type(
                Language::C11,
                &read("list<optional<union:Shape>>[3]"),
                &Names,
                &mut c11
            ),
            Ok("sce_list_3_opt_union_UnionShape_t".to_string())
        );
        assert!(
            c11.defs
                .iter()
                .any(|def| def.contains("    bool present;\n    UnionShape value;\n")),
            "{:#?}",
            c11.defs
        );
        for lang in Language::ALL.iter().copied() {
            if crate::forge::codegen_matrix::UNION_BACKENDS.contains(&lang) {
                continue;
            }
            assert_eq!(
                unit_type(
                    lang,
                    &read("union:Shape"),
                    &Names,
                    &mut C11Declarations::default()
                ),
                Err(crate::forge::value_type::union_unlowered("Shape")),
                "{lang:?}"
            );
        }
    }

    #[test]
    fn c11_declares_each_composition_once_inner_first() {
        let mut c11 = C11Declarations::default();
        let outer = unit_type(
            Language::C11,
            &read("list<list<bytes[4]>[2]>[3]"),
            &Names,
            &mut c11,
        )
        .expect("spelled");
        assert_eq!(outer, "sce_list_3_list_2_bytes_4_t");
        let again = unit_type(Language::C11, &read("list<bytes[4]>[2]"), &Names, &mut c11)
            .expect("spelled");
        assert_eq!(again, "sce_list_2_bytes_4_t");
        assert_eq!(c11.defs.len(), 3, "{:#?}", c11.defs);
        assert!(c11.defs[0].contains("uint8_t data[4];") && c11.defs[0].contains("sce_bytes_4_t"));
        assert!(c11.defs[1].contains("sce_bytes_4_t items[2];"));
        assert!(c11.defs[2].contains("sce_list_2_bytes_4_t items[3];"));
        assert!(c11.defs[2].starts_with("#ifndef SCE_LIST_3_LIST_2_BYTES_4_DECLARED"));
        // A text is held in the buffer every header of its bound shares.
        let text = unit_type(
            Language::C11,
            &read("optional<string[8]>"),
            &Names,
            &mut c11,
        )
        .expect("spelled");
        assert_eq!(text, "sce_opt_string_8_t");
        assert!(c11.texts.contains(&8));
        // An enum is mangled by its document, a record by its payload type.
        assert_eq!(
            unit_type(
                Language::C11,
                &read("list<enum:Level>[2]"),
                &Names,
                &mut c11
            )
            .expect("spelled"),
            "sce_list_2_enum_doc_level_t"
        );
        assert_eq!(
            unit_type(
                Language::C11,
                &read("optional<record:Item>"),
                &Names,
                &mut c11
            )
            .expect("spelled"),
            "sce_opt_record_ItemPayload_t"
        );
    }

    #[test]
    fn a_swift_field_starts_empty_absent_or_at_its_records_zero() {
        assert_eq!(swift_zero(&read("list<uint8>[2]"), &Names), Ok("[]".into()));
        assert_eq!(
            swift_zero(&read("optional<list<uint8>[2]>"), &Names),
            Ok("nil".into())
        );
        assert_eq!(
            swift_zero(&read("record:Item"), &Names),
            Ok("ItemPayload_t()".into())
        );
    }
}
