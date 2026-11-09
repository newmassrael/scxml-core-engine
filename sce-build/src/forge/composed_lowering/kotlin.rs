// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Kotlin row).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `List<T>` | `String` / `ByteArray` | `T?` | the machine's data class |
//!
//! No Kotlin type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is.

use super::*;

/// The Kotlin spellings of the walks, for the machine `machine` (section 7.1,
/// the Kotlin row): the runtime's `EventPayload.Value` reads each constructor,
/// and its writers write them into `carrier`.
///
/// Every binding is Kotlin's implicit `it`, so a lambda nested in another
/// declares no parameter that could shadow its holder's: a generated machine is
/// compiled with every warning an error.
pub struct KotlinComposed<'a> {
    pub machine: &'a str,
    pub carrier: KotlinCarrier,
}

/// What a Kotlin writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KotlinCarrier {
    /// The JSON text itself, by the runtime's `EventPayload.listJson`,
    /// `recordJson` and `valueJson`: what the inject seam writes beside the
    /// typed carrier (section 3.1).
    Json,
    /// The value a `<send>` holds each param in (`StaticTarget::wire_value`)
    /// until the engine's `valueToJson` writes them as one JSON object: a list
    /// as a `List` of its elements, a record as a `Map` of its fields, whose
    /// keys the engine orders by their UTF-8 bytes at every depth, an absent
    /// value as `null`, and every scalar as a `<send>`'s scalar param is
    /// (section 5.3; ARCHITECTURE.md, "JSON Object Key Order").
    Wire,
}

/// The runtime object the Kotlin readers and writers are members of.
const KOTLIN_PAYLOAD: &str = "com.sce.runtime.EventPayload";

/// The runtime object whose checks a Kotlin record runs where it is made.
const KOTLIN_BOUNDS: &str = "com.sce.runtime.ValueBounds";

/// The check of a Kotlin value of `ty` at `value`, named `path` (an expression
/// of a text), that throws the runtime's `BoundExceeded`, naming the path to
/// the place, where the value is past a bound its type declares at any depth
/// (docs/adr/0086, section 5.4): one call of `ValueBounds` per constructor,
/// the check of an element or a present value inside the call of the list or
/// optional value that holds it, each binding names of its own `depth`, so no
/// lambda shadows another's. `None` for a type that holds no bound of its own:
/// a number, an enum, a record (whose own `init` checks its fields) and an
/// optional value of one of those.
fn kotlin_bound_check(ty: &ValueType, path: &str, value: &str, depth: usize) -> Option<String> {
    let (inner_path, inner_value) = (format!("scePath{depth}"), format!("sceValue{depth}"));
    match ty {
        ValueType::Text { max } => Some(format!(
            "{KOTLIN_BOUNDS}.text({path}, {value}, {})",
            max.max
        )),
        ValueType::Bytes { max } => Some(format!(
            "{KOTLIN_BOUNDS}.bytes({path}, {value}, {})",
            max.max
        )),
        // The count is held whatever the elements are; an element with
        // nothing of its own to check binds nothing.
        ValueType::List { elem, max } => {
            let each = match kotlin_bound_check(elem, &inner_path, &inner_value, depth + 1) {
                Some(inner) => format!("{inner_path}, {inner_value} -> {inner}"),
                None => "_, _ ->".to_string(),
            };
            Some(if max.min == 0 {
                format!(
                    "{KOTLIN_BOUNDS}.list({path}, {value}, {}) {{ {each} }}",
                    max.max
                )
            } else {
                format!(
                    "{KOTLIN_BOUNDS}.listWithin({path}, {value}, {}, {}) {{ {each} }}",
                    max.min, max.max
                )
            })
        }
        ValueType::Optional { elem } => {
            kotlin_bound_check(elem, &inner_path, &inner_value, depth + 1).map(|inner| {
                format!(
                    "{KOTLIN_BOUNDS}.optional({path}, {value}) {{ {inner_path}, {inner_value} -> \
                     {inner} }}"
                )
            })
        }
        ValueType::Scalar(_) | ValueType::Record { .. } | ValueType::Union { .. } => None,
    }
}

/// The body a Kotlin data class takes for `checks`: an `init` that runs each
/// where a value is made, or nothing where there is none.
pub(crate) fn kotlin_init_block(checks: &[String]) -> String {
    if checks.is_empty() {
        return String::new();
    }
    let body: String = checks
        .iter()
        .map(|check| format!("        {check}\n"))
        .collect();
    format!(" {{\n    init {{\n{body}    }}\n}}")
}

/// Why a Kotlin walk meets no union: the one decision of what is lowered
/// refuses a union on Kotlin before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const KOTLIN_HOLDS_NO_UNION: &str =
    "a Kotlin machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on Kotlin before any walk reaches one";

impl ComposedSpelling for KotlinComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::kotlin_type(ty).to_string()
    }
    // No Kotlin type has a constant parameter a bound could be (section 7.4):
    // the reader holds a value to its bound where it is read.
    fn text_type(&self, _max: u32) -> String {
        "String".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "ByteArray".to_string()
    }
    fn optional_type(&self, elem: &str) -> String {
        format!("{elem}?")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("List<{elem}>")
    }
    // The name a statechart's record variable of the same schema is held in
    // (`StaticTarget::record_type`), so a record a payload holds is the type a
    // variable holds.
    fn record_type(&self, alias: &str) -> String {
        format!(
            "{}{}Record",
            self.machine,
            crate::filters::to_pascal_case(alias.to_string())
        )
    }
    // Immutable and compared whole, as a record of entries is; each member
    // named as the schema names its field, which is how a read names it. A
    // value past a bound is refused in its `init`, where it is made.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        checks: &[String],
    ) -> String {
        let params: Vec<String> = fields
            .iter()
            .map(|(id, ty)| format!("val {id}: {ty}"))
            .collect();
        format!(
            "/** SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed \
             value (docs/adr/0086). */\ndata class {ty}({}){}",
            params.join(", "),
            kotlin_init_block(checks)
        )
    }
    // One check of the runtime's `ValueBounds` per constructor that holds a
    // bound, recursively through each list and optional value; a record a
    // field holds checks its own fields in its own `init`.
    fn field_checks(&self, fields: &[(&str, &ValueType)]) -> Vec<String> {
        fields
            .iter()
            .filter_map(|(id, ty)| {
                kotlin_bound_check(
                    ty,
                    &format!("\"{}\"", crate::filters::escape_kotlin(id.to_string())),
                    id,
                    0,
                )
            })
            .collect()
    }

    fn read_binding(&self) -> &'static str {
        "it"
    }
    // The width is in the reader's name, so a field widened in its schema
    // cannot keep being read at the old width.
    fn read_scalar(&self, ty: &SceType, value: &str) -> String {
        let reader = match ty {
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
            SceType::Bool => "boolean",
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        };
        format!("{value}.{reader}()")
    }
    // The variant whose declared name the wire carries; a name the enum does
    // not declare is refused with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "\"{}\" -> {}.{ident}; ",
                    crate::filters::escape_kotlin(declared.clone()),
                    held.ty
                )
            })
            .collect();
        format!(
            "when (val sceName = {value}.name()) {{ {arms}else -> throw \
             {value}.unknownVariant(sceName, \"{}\") }}",
            crate::filters::escape_kotlin(held.alias.clone())
        )
    }
    fn read_text(&self, max: u32, value: &str) -> String {
        format!("{value}.string({max})")
    }
    fn read_bytes(&self, max: u32, value: &str) -> String {
        format!("{value}.bytes({max})")
    }
    // A Kotlin lambda's types are inferred, so the element's type is not
    // spelled.
    fn read_optional(&self, value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{value}.optional {{ {elem} }}")
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        if least == 0 {
            format!("{value}.list({max}) {{ {elem} }}")
        } else {
            format!("{value}.listWithin({least}, {max}) {{ {elem} }}")
        }
    }
    fn read_record(
        &self,
        value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "null".to_string(),
            |json| format!("\"{}\"", crate::filters::escape_kotlin(json.to_string())),
        );
        let members: Vec<String> = fields
            .iter()
            .map(|(id, read)| format!("{id} = it.field(\"{id}\").let {{ {read} }}"))
            .collect();
        format!(
            "{value}.record({defaults}) {{ {ty}({}) }}",
            members.join(", ")
        )
    }

    fn write_binding(&self) -> &'static str {
        "it"
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        match self.carrier {
            // The runtime spells a number of every width and a truth value as
            // one JSON value, a 64-bit integer as the text of its digits.
            KotlinCarrier::Json => format!("{KOTLIN_PAYLOAD}.valueJson({})", held.text),
            // As a scalar param is: an integer of at most 32 bits widened to a
            // `Long` and a `Float` to a `Double`, both exactly, and a 64-bit
            // integer as the text of its digits.
            KotlinCarrier::Wire => match ty {
                SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32 => format!("({}).toLong()", held.text),
                SceType::Uint64 | SceType::Int64 => format!("({}).toString()", held.text),
                SceType::Float32 => format!("({}).toDouble()", held.text),
                SceType::Float64 | SceType::Bool => held.text.clone(),
                SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                    "a text, a byte string and an enum are cases of their own in the walk, \
                     never a scalar of a value type"
                ),
            },
        }
    }
    // The machine's enum declares the name its document gives the variant.
    fn write_enum(&self, held: &Held) -> String {
        match self.carrier {
            KotlinCarrier::Json => {
                format!("{KOTLIN_PAYLOAD}.valueJson({}.declaredName)", held.text)
            }
            KotlinCarrier::Wire => format!("{}.declaredName", held.text),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            KotlinCarrier::Json => format!("{KOTLIN_PAYLOAD}.valueJson({})", held.text),
            KotlinCarrier::Wire => held.text.clone(),
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            KotlinCarrier::Json => format!("{KOTLIN_PAYLOAD}.valueJson({})", held.text),
            KotlinCarrier::Wire => format!("{KOTLIN_PAYLOAD}.bytesAsText({})", held.text),
        }
    }
    // Parenthesised whole, so the elvis binds inside the member it is written
    // in: an infix `to` binds tighter than `?:`. `null` where the value is
    // absent, as an absent scalar param is.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            KotlinCarrier::Json => format!(
                "(({})?.let {{ {elem} }} ?: {KOTLIN_PAYLOAD}.valueJson(null))",
                held.text
            ),
            KotlinCarrier::Wire => format!("(({})?.let {{ {elem} }})", held.text),
        }
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            KotlinCarrier::Json => {
                format!("{KOTLIN_PAYLOAD}.listJson({}) {{ {elem} }}", held.text)
            }
            KotlinCarrier::Wire => format!("({}).map {{ {elem} }}", held.text),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime orders the keys by their UTF-8 bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), the JSON writer and the
    // engine's alike.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, json)| format!("\"{id}\" to {json}"))
            .collect();
        let map = format!("mapOf<String, Any?>({})", members.join(", "));
        match self.carrier {
            KotlinCarrier::Json => format!("{KOTLIN_PAYLOAD}.recordJson({map})"),
            KotlinCarrier::Wire => map,
        }
    }
    // The count held first, by the `AlgorithmFailure` a `<send>` catches of a
    // param that can fail, then the list written.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "run {{ if (({}).size < {least}) throw com.sce.forge.runtime.AlgorithmFailure(\
             com.sce.forge.runtime.AlgorithmError.Precondition); {sent} }}",
            held.text
        ))
    }

    fn copy_binding(&self) -> &'static str {
        "it"
    }
    // Every value the machine holds is immutable, a byte string included (the
    // machine never writes into one), so a copy is the value itself.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        held.text.clone()
    }
    // A text and a byte string are one Kotlin type whatever their bound, so a
    // walk that copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_optional(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!("({})?.let {{ {elem} }}", held.text)
    }
    // A `T` stands where a `T?` is expected as it is.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    fn copy_list(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!("({}).map {{ {elem} }}", held.text)
    }

    // A Kotlin machine holds no union yet: the gate refuses one by name
    // before a walk reaches it, so each spelling below either names what it
    // would be or says why there is none.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Kotlin, "", variant)
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!(
            "proven{}",
            crate::filters::to_pascal_case(variant.to_string())
        )
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "provenField{}",
            crate::filters::to_pascal_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{KOTLIN_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{KOTLIN_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{KOTLIN_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{KOTLIN_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(KOTLIN_HOLDS_NO_UNION.to_string())
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}Place")
    }
    fn union_member_place(
        &self,
        _holder: &str,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(KOTLIN_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const KOTLIN: KotlinComposed<'static> = KotlinComposed {
        machine: "M",
        carrier: KotlinCarrier::Json,
    };

    /// The Kotlin row of the one table of what each backend lowers of a
    /// composed value: a payload read at any depth, a list taken whole and a
    /// composed value sent (docs/adr/0086, sections 5 and 6), and no union
    /// held yet (docs/adr/0092, section 7).
    #[test]
    fn the_kotlin_row_lowers_every_composed_position_but_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Kotlin),
            ComposedLowering {
                name: "kotlin",
                reads: true,
                copies: true,
                unions: false,
                sends: true,
            }
        );
    }

    /// The value a Kotlin `<send>` holds a composed param in: a `List`, a `Map`
    /// whose keys the engine orders, `null` where a value is absent, and each
    /// scalar as a scalar param is (a narrow integer widened to `Long`, a
    /// 64-bit one as its digits, a byte string as its Latin-1 text).
    #[test]
    fn a_kotlin_send_writes_each_constructor_into_the_value_it_holds_params_in() {
        let held = held();
        let wire = KotlinComposed {
            machine: "M",
            carrier: KotlinCarrier::Wire,
        };
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &wire, &at).expect("write");
        let p = "com.sce.runtime.EventPayload";
        assert_eq!(write("uint8", Held::place("x.n")), "(x.n).toLong()");
        assert_eq!(write("int64", Held::place("x.n")), "(x.n).toString()");
        assert_eq!(write("float32", Held::place("x.n")), "(x.n).toDouble()");
        assert_eq!(write("float64", Held::place("x.n")), "x.n");
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            "(x.grid).map { (it).map { (it).toLong() } }"
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            "((x.spare)?.let { (it).map { (it).toLong() } })"
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            format!(
                "(x.items).map {{ mapOf<String, Any?>(\"id\" to (it.id).toLong(), \"level\" to \
                 it.level.declaredName, \"due\" to ((it.due)?.let {{ (it).toString() }}), \
                 \"blob\" to {p}.bytesAsText(it.blob), \"tags\" to (it.tags).map {{ it }}) }}"
            )
        );
    }

    /// The Kotlin row of section 7.1: no type carries a bound, a value that
    /// may be absent is the type over `?`, a record is the machine's data
    /// class, at any depth.
    #[test]
    fn a_kotlin_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &KOTLIN).expect("a type");
        assert_eq!(spelled("uint16"), "UShort");
        assert_eq!(spelled("string[8]"), "String");
        assert_eq!(spelled("bytes[4]"), "ByteArray");
        assert_eq!(spelled("optional<int64>"), "Long?");
        assert_eq!(spelled("list<list<uint8>[2]>[3]"), "List<List<UByte>>");
        assert_eq!(spelled("list<enum:Level>[2]"), "List<MLevelEnum>");
        assert_eq!(spelled("list<record:Item>[3]"), "List<MItemRecord>");
        assert_eq!(spelled("optional<list<uint8>[2]>"), "List<UByte>?");
        assert_eq!(spelled("list<optional<uint16>>[2]"), "List<UShort?>");
        let members = record_members("Item", &held, &enums, &KOTLIN).expect("members");
        let checks = record_checks("Item", &held, &KOTLIN).expect("checks");
        let def = KOTLIN.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.ends_with(
                "data class MItemRecord(val id: UInt, val level: MLevelEnum, val due: Long?, \
                 val blob: ByteArray, val tags: List<String>) {\n    init {\n        \
                 com.sce.runtime.ValueBounds.bytes(\"blob\", blob, 4)\n        \
                 com.sce.runtime.ValueBounds.list(\"tags\", tags, 4) { scePath0, sceValue0 -> \
                 com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 8) }\n    }\n}"
            ),
            "{def}"
        );
    }

    /// A Kotlin record holds a host's value to every bound where it is made
    /// (docs/adr/0086, section 5.4): one check per constructor that holds a
    /// bound, an element's or a present value's inside its holder's, each
    /// binding names of its own depth; a lower count is held too; a number, an
    /// enum and a record (which checks itself) are not checked here.
    #[test]
    fn a_kotlin_record_checks_each_bound_where_it_is_made() {
        let b = "com.sce.runtime.ValueBounds";
        let checks = |fields: &[(&str, &str)]| {
            let typed: Vec<(String, ValueType)> = fields
                .iter()
                .map(|(id, text)| (id.to_string(), ty(text)))
                .collect();
            let view: Vec<(&str, &ValueType)> = typed
                .iter()
                .map(|(id, written)| (id.as_str(), written))
                .collect();
            KOTLIN.field_checks(&view)
        };
        assert_eq!(
            checks(&[
                ("n", "uint8"),
                ("level", "enum:Level"),
                ("item", "record:Item"),
                ("due", "optional<int64>"),
            ]),
            Vec::<String>::new()
        );
        assert_eq!(
            checks(&[("grid", "list<list<uint8>[2]>[2..3]")]),
            vec![format!(
                "{b}.listWithin(\"grid\", grid, 2, 3) {{ scePath0, sceValue0 -> \
                 {b}.list(scePath0, sceValue0, 2) {{ _, _ -> }} }}"
            )]
        );
        assert_eq!(
            checks(&[("spare", "optional<list<string[4]>[2]>")]),
            vec![format!(
                "{b}.optional(\"spare\", spare) {{ scePath0, sceValue0 -> \
                 {b}.list(scePath0, sceValue0, 2) {{ scePath1, sceValue1 -> \
                 {b}.text(scePath1, sceValue1, 4) }} }}"
            )]
        );
        assert_eq!(
            checks(&[("items", "list<record:Item>[3]")]),
            vec![format!("{b}.list(\"items\", items, 3) {{ _, _ -> }}")]
        );
    }

    /// One call of the runtime's `EventPayload.Value` per constructor, every
    /// lambda reading its own `it`, a bound and a lower count handed to the
    /// reader that holds them.
    #[test]
    fn a_kotlin_reader_is_one_runtime_call_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &KOTLIN, "v").expect("read");
        assert_eq!(read("uint8"), "v.uint8()");
        assert_eq!(read("int64"), "v.int64()");
        assert_eq!(read("bool"), "v.boolean()");
        assert_eq!(read("float32"), "v.float32()");
        assert_eq!(read("string[8]"), "v.string(8)");
        assert_eq!(read("bytes[4]"), "v.bytes(4)");
        assert_eq!(read("optional<int64>"), "v.optional { it.int64() }");
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "v.list(3) { it.list(2) { it.uint8() } }"
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            "v.listWithin(2, 3) { it.listWithin(1, 2) { it.uint8() } }"
        );
        assert_eq!(
            read("enum:Level"),
            "when (val sceName = v.name()) { \"low\" -> MLevelEnum.Low; \"high\" -> \
             MLevelEnum.High; else -> throw v.unknownVariant(sceName, \"Level\") }"
        );
        let item = read("record:Item");
        assert!(
            item.starts_with(
                "v.record(null) { MItemRecord(id = it.field(\"id\").let { it.uint32() }, "
            ),
            "{item}"
        );
        assert!(
            item.ends_with("tags = it.field(\"tags\").let { it.list(4) { it.string(8) } }) }"),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.starts_with("v.record(\"{\\\"late\\\":[]}\") { MBatchRecord("),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor; an absent value is `null`, parenthesised whole so an infix
    /// `to` that holds it does not take the elvis as its own.
    #[test]
    fn a_kotlin_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &KOTLIN, &at).expect("write");
        let p = "com.sce.runtime.EventPayload";
        assert_eq!(
            write("uint8", Held::place("x.n")),
            format!("{p}.valueJson(x.n)")
        );
        assert_eq!(
            write("enum:Level", Held::reference("it")),
            format!("{p}.valueJson(it.declaredName)")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "{p}.listJson(x.maybe) {{ ((it)?.let {{ {p}.valueJson(it) }} ?: \
                 {p}.valueJson(null)) }}"
            )
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            format!(
                "{p}.listJson(x.items) {{ {p}.recordJson(mapOf<String, Any?>(\"id\" to \
                 {p}.valueJson(it.id), \"level\" to {p}.valueJson(it.level.declaredName), \
                 \"due\" to ((it.due)?.let {{ {p}.valueJson(it) }} ?: {p}.valueJson(null)), \
                 \"blob\" to {p}.valueJson(it.blob), \"tags\" to {p}.listJson(it.tags) {{ \
                 {p}.valueJson(it) }})) }}"
            )
        );
    }

    /// Two places whose types differ only in their bounds are one Kotlin type,
    /// so a value that fits is copied as it is; a value of `T` stands where a
    /// `T?` is expected as it is too.
    #[test]
    fn a_kotlin_copy_of_a_value_that_fits_is_the_value() {
        let held = held();
        let enums = enums();
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &held,
                &enums,
                &KOTLIN,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("string[4]", "optional<string[8]>"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok("(x).map { it }".to_string())
        );
    }

    /// A Kotlin walk names no union: the one decision of what is lowered
    /// refuses one on Kotlin first, and the walk that would declare one says
    /// why there is none.
    #[test]
    fn a_kotlin_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &KOTLIN).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(KOTLIN
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
    }

    /// A list sent where its field asks a lower count it does not prove is
    /// held to the count before it is written (docs/adr/0086, section 15.6).
    #[test]
    fn a_list_sent_short_of_its_fields_count_fails_before_it_is_written() {
        assert_eq!(
            KOTLIN.send_counted(&Held::place("vTaking"), "ops", 1, "W".to_string()),
            Some(
                "run { if ((vTaking).size < 1) throw com.sce.forge.runtime.AlgorithmFailure(\
                 com.sce.forge.runtime.AlgorithmError.Precondition); W }"
                    .to_string()
            )
        );
    }
}
