// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Python row).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `list[T]` | `str` / `bytes` | `Optional[T]` | the machine's record class |
//!
//! No Python type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is, into a copy that shares no list with the place it was
//! taken from, since the variable it is copied into is written into in place.

use super::*;

/// The Python spellings of the walks, for the machine `machine` (section 7.1,
/// the Python row): a reader is the runtime's name of the value's type
/// (`sce_runtime.event_payload`'s `ListOf`, `RecordOf`, `Text`, ...), by which
/// its one reader of a value at a path reads it, and the writers write a value
/// into `carrier`.
///
/// Every name a writer or a copy binds is under the generator's prefix, which
/// no author's name is spelled as
/// ([`crate::forge::generator::python_local_spelling`]). A binding nested in
/// another of the same name is its own: the holder is read where the outer one
/// is bound, before the inner one binds its element.
pub struct PythonComposed<'a> {
    pub machine: &'a str,
    pub carrier: PythonCarrier,
}

/// What a Python writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonCarrier {
    /// The JSON text itself, by the runtime's `list_json`, `record_json`,
    /// `value_json` and `wide_json`: what the inject seam writes beside the
    /// typed carrier (section 3.1).
    Json,
    /// The plain value a `<send>`'s `ScriptValue` is made of
    /// ([`ComposedSpelling::send_param`]), which the send takes back out of it
    /// where it puts the param: a list as a `list` of its elements, a record as
    /// a `dict` of its fields, whose keys the runtime orders by their UTF-8
    /// bytes at every depth when it writes the event's JSON, an absent value as
    /// `None`, and every scalar as a `<send>`'s scalar param is
    /// (`StaticTarget::wire_value`; section 5.3; ARCHITECTURE.md, "JSON Object
    /// Key Order").
    Wire,
}

/// The alias a generated Python module imports the runtime module
/// `sce_runtime.event_payload` by, whose type names, writers and check the
/// Python spellings call.
pub(crate) const PYTHON_PAYLOAD: &str = "sce_event_payload";

/// The runtime's name of the type of a whole number of the width `ty`
/// declares, held to that width: `Whole`, or `WideOf` for a 64-bit one, which
/// crosses as the text of its digits. `None` for a type that is no whole
/// number.
fn python_whole(ty: &SceType) -> Option<String> {
    let (low, high) = ty.int_value_range()?;
    let name = if matches!(ty, SceType::Uint64 | SceType::Int64) {
        "WideOf"
    } else {
        "Whole"
    };
    Some(format!("{PYTHON_PAYLOAD}.{name}({low}, {high})"))
}

/// The runtime's name of the type of a list of at least `least` and at most
/// `max` elements of the type `elem` names.
fn python_list_of(elem: &str, least: u32, max: u32) -> String {
    if least == 0 {
        format!("{PYTHON_PAYLOAD}.ListOf({elem}, {max})")
    } else {
        format!("{PYTHON_PAYLOAD}.ListOf({elem}, {max}, least={least})")
    }
}

/// The runtime's name of the type a Python check holds a value of `ty` to
/// where it is made (docs/adr/0086, section 5.4): a text and a byte string to
/// their bounds, a list to its counts with each element checked inside it, a
/// present value as its type is, and a whole number to its width, which
/// Python's `int` holds none of. `None` for a type with nothing of its own to
/// hold there: a truth value, a real, an enum, and a record, whose own class
/// checks its fields where it is made. An element with nothing of its own to
/// hold is named `None` inside its list, which the runtime's check passes over,
/// so the list is held to its counts alone.
fn python_check_type(ty: &ValueType) -> Option<String> {
    match ty {
        ValueType::Text { max } => Some(format!("{PYTHON_PAYLOAD}.Text({})", max.max)),
        ValueType::Bytes { max } => Some(format!("{PYTHON_PAYLOAD}.Bytes({})", max.max)),
        ValueType::Scalar(scalar) => python_whole(scalar),
        ValueType::List { elem, max } => {
            let each = python_check_type(elem).unwrap_or_else(|| "None".to_string());
            Some(python_list_of(&each, max.min, max.max))
        }
        ValueType::Optional { elem } => {
            python_check_type(elem).map(|inner| format!("{PYTHON_PAYLOAD}.OptionalOf({inner})"))
        }
        ValueType::Record { .. } | ValueType::Union { .. } => None,
    }
}

/// The method a Python class takes for `checks`: a `__post_init__` that runs
/// each where a value is made, or nothing where there is none.
pub(crate) fn python_post_init(checks: &[String]) -> String {
    if checks.is_empty() {
        return String::new();
    }
    let body: String = checks
        .iter()
        .map(|check| format!("        {check}\n"))
        .collect();
    format!("\n    def __post_init__(self) -> None:\n{body}")
}

/// Why a Python walk meets no union: the one decision of what is lowered
/// refuses a union on Python before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const PYTHON_HOLDS_NO_UNION: &str =
    "a Python machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on Python before any walk reaches one";

impl ComposedSpelling for PythonComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::python_type(ty).to_string()
    }
    // No Python type has a parameter a bound could be (section 7.4): the
    // reader holds a value to its bound where it is read, and the class that
    // holds it where it is made.
    fn text_type(&self, _max: u32) -> String {
        "str".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "bytes".to_string()
    }
    fn optional_type(&self, elem: &str) -> String {
        format!("Optional[{elem}]")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("list[{elem}]")
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
    // Frozen and compared whole, as a record of entries is; each field named
    // as the schema names it, which is how a read names it. A value past a
    // bound is refused in its `__post_init__`, where it is made. Declared only
    // beside the payload that holds it (`declare_composed_payload`), whose
    // channel imports `dataclass` and the runtime module the checks call.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    {id}: {ty}\n"))
            .collect();
        format!(
            "@dataclass(frozen=True)\nclass {ty}:\n    \"\"\"SCE Accepted Subset §2.15: a \
             `record:{alias}` value that holds a composed value (docs/adr/0086).\"\"\"\n\n\
             {members}{}",
            python_post_init(checks)
        )
    }
    // One call of the runtime's `check_value` per field that holds a bound or
    // a width at any depth, by the type it is held to; a record a field holds
    // checks its own fields in its own `__post_init__`.
    fn field_checks(&self, fields: &[(&str, &ValueType)]) -> Vec<String> {
        fields
            .iter()
            .filter_map(|(id, ty)| {
                python_check_type(ty).map(|declared| {
                    format!(
                        "{PYTHON_PAYLOAD}.check_value({}, {declared}, self.{id})",
                        crate::filters::py_string_literal(id.to_string())
                    )
                })
            })
            .collect()
    }

    // A reader is the name of the type the runtime reads a value by; the value
    // is the runtime's to hand it, so `value` names nothing here.
    fn read_binding(&self) -> &'static str {
        "sce_value"
    }
    // The width is in the type's name, so a field widened in its schema cannot
    // keep being read at the old width.
    fn read_scalar(&self, ty: &SceType, _value: &str) -> String {
        match ty {
            SceType::Float32 => format!("{PYTHON_PAYLOAD}.Single"),
            SceType::Float64 => "float".to_string(),
            SceType::Bool => "bool".to_string(),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
            whole => python_whole(whole).unwrap_or_else(|| {
                unreachable!("every other scalar of a value type is a whole number")
            }),
        }
    }
    // The machine's enum, whose member the runtime reads by the declared name
    // the wire carries (`sce_name`); a name the enum does not declare is
    // refused with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, _value: &str) -> String {
        held.ty.clone()
    }
    fn read_text(&self, max: u32, _value: &str) -> String {
        format!("{PYTHON_PAYLOAD}.Text({max})")
    }
    fn read_bytes(&self, max: u32, _value: &str) -> String {
        format!("{PYTHON_PAYLOAD}.Bytes({max})")
    }
    fn read_optional(&self, _value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{PYTHON_PAYLOAD}.OptionalOf({elem})")
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, _value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        python_list_of(elem, least, max)
    }
    // The record class builds the value from the fields read, in the schema's
    // order, which is the order of its own fields.
    fn read_record(
        &self,
        _value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "None".to_string(),
            |json| crate::filters::py_string_literal(json.to_string()),
        );
        let members: Vec<String> = fields
            .iter()
            .map(|(id, read)| {
                format!(
                    "({}, {read})",
                    crate::filters::py_string_literal(id.clone())
                )
            })
            .collect();
        format!(
            "{PYTHON_PAYLOAD}.RecordOf([{}], {defaults}, {ty})",
            members.join(", ")
        )
    }

    fn write_binding(&self) -> &'static str {
        "sce_element"
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        let wide = matches!(ty, SceType::Uint64 | SceType::Int64);
        match self.carrier {
            // A 64-bit integer crosses as the text of its digits.
            PythonCarrier::Json if wide => format!("{PYTHON_PAYLOAD}.wide_json({})", held.text),
            // A number of every other width and a truth value as one JSON
            // value, a real as ECMAScript spells it.
            PythonCarrier::Json => format!("{PYTHON_PAYLOAD}.value_json({})", held.text),
            // As a scalar param is: a 64-bit integer as the text of its digits,
            // a real as a `float`, and an integer and a truth value as held.
            PythonCarrier::Wire if wide => format!("str({})", held.text),
            PythonCarrier::Wire => match ty {
                SceType::Float32 | SceType::Float64 => format!("float({})", held.text),
                _ => held.text.clone(),
            },
        }
    }
    // The machine's enum answers the name its document declares (`sce_name`).
    fn write_enum(&self, held: &Held) -> String {
        match self.carrier {
            PythonCarrier::Json => {
                format!("{PYTHON_PAYLOAD}.value_json(({}).sce_name)", held.text)
            }
            PythonCarrier::Wire => format!("({}).sce_name", held.text),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            PythonCarrier::Json => format!("{PYTHON_PAYLOAD}.value_json({})", held.text),
            PythonCarrier::Wire => held.text.clone(),
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text,
    // which `value_json` writes a `bytes` as.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            PythonCarrier::Json => format!("{PYTHON_PAYLOAD}.value_json({})", held.text),
            PythonCarrier::Wire => format!("({}).decode(\"latin-1\")", held.text),
        }
    }
    // `null` where the value is absent, its key always written; presence is
    // `is None`, never a truth test, so a present `0` is written as the value
    // it is (docs/adr/0055, section 4).
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            PythonCarrier::Json => format!(
                "{PYTHON_PAYLOAD}.value_json({PYTHON_PAYLOAD}.write_optional({}, lambda \
                 sce_element: {elem}))",
                held.text
            ),
            PythonCarrier::Wire => format!(
                "(lambda sce_element: None if sce_element is None else {elem})({})",
                held.text
            ),
        }
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            PythonCarrier::Json => format!(
                "{PYTHON_PAYLOAD}.list_json({}, lambda sce_element: {elem})",
                held.text
            ),
            PythonCarrier::Wire => format!("[{elem} for sce_element in {}]", held.text),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime orders the keys by their UTF-8 bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), the JSON writer and the
    // `ScriptValue` a send is written from alike.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, json)| format!("{}: {json}", crate::filters::py_string_literal(id.clone())))
            .collect();
        let dict = format!("{{{}}}", members.join(", "));
        match self.carrier {
            PythonCarrier::Json => format!("{PYTHON_PAYLOAD}.record_json({dict})"),
            PythonCarrier::Wire => dict,
        }
    }
    // The runtime's dynamic value, made of the plain value the writers wrote.
    fn send_param(&self, written: String) -> String {
        match self.carrier {
            PythonCarrier::Json => written,
            PythonCarrier::Wire => format!("_ScriptValue.of({written})"),
        }
    }

    fn copy_binding(&self) -> &'static str {
        "sce_copied"
    }
    // A list is written into in place (an append, a clear), so a value that
    // is or holds one is copied whole, sharing no list with the place it was
    // taken from; a number, a truth value and an enum are copied as read.
    fn copy_whole(&self, held: &Held, plain: bool) -> String {
        if plain {
            held.text.clone()
        } else {
            format!("sce_algorithm.whole_copy({})", held.text)
        }
    }
    // A text and a byte string are one Python type whatever their bound, so a
    // walk that copies by type never reaches these; the value, which nothing
    // writes into, is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_optional(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!(
            "(lambda sce_copied: None if sce_copied is None else {elem})({})",
            held.text
        )
    }
    // A `T` stands where an `Optional[T]` is expected as it is.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    fn copy_list(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!("[{elem} for sce_copied in {}]", held.text)
    }

    // A Python machine holds no union yet: the gate refuses one by name
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
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Python, "", variant)
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!(
            "proven_{}",
            crate::filters::to_snake_case(variant.to_string())
        )
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "proven_field_{}",
            crate::filters::to_snake_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{PYTHON_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{PYTHON_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{PYTHON_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{PYTHON_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(PYTHON_HOLDS_NO_UNION.to_string())
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}_place")
    }
    fn union_member_place(
        &self,
        _holder: &str,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(PYTHON_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const PYTHON: PythonComposed<'static> = PythonComposed {
        machine: "M",
        carrier: PythonCarrier::Json,
    };

    /// The Python row of the one table of what each backend lowers of a
    /// composed value: a payload read at any depth, a list taken whole and a
    /// composed value sent (docs/adr/0086, sections 5 and 6), and no union
    /// held yet (docs/adr/0092, section 7).
    #[test]
    fn the_python_row_lowers_every_composed_position_but_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Python),
            ComposedLowering {
                name: "python",
                reads: true,
                copies: true,
                unions: false,
                sends: true,
            }
        );
    }

    /// The Python row of section 7.1: no type carries a bound, a value that
    /// may be absent is `Optional` over its type, a record is the machine's
    /// frozen record class, at any depth; the class checks each field that
    /// holds a bound or a width where it is made.
    #[test]
    fn a_python_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &PYTHON).expect("a type");
        assert_eq!(spelled("uint16"), "int");
        assert_eq!(spelled("string[8]"), "str");
        assert_eq!(spelled("bytes[4]"), "bytes");
        assert_eq!(spelled("optional<int64>"), "Optional[int]");
        assert_eq!(spelled("list<list<uint8>[2]>[3]"), "list[list[int]]");
        assert_eq!(spelled("list<enum:Level>[2]"), "list[MLevelEnum]");
        assert_eq!(spelled("list<record:Item>[3]"), "list[MItemRecord]");
        assert_eq!(spelled("optional<list<uint8>[2]>"), "Optional[list[int]]");
        assert_eq!(spelled("list<optional<uint16>>[2]"), "list[Optional[int]]");
        let members = record_members("Item", &held, &enums, &PYTHON).expect("members");
        let checks = record_checks("Item", &held, &PYTHON).expect("checks");
        let def = PYTHON.record_def("MItemRecord", "Item", &members, &checks);
        let p = PYTHON_PAYLOAD;
        assert!(
            def.starts_with("@dataclass(frozen=True)\nclass MItemRecord:\n"),
            "{def}"
        );
        assert!(
            def.ends_with(&format!(
                "\n\n    id: int\n    level: MLevelEnum\n    due: Optional[int]\n    \
                 blob: bytes\n    tags: list[str]\n\n    def __post_init__(self) -> None:\n        \
                 {p}.check_value(\"id\", {p}.Whole(0, 4294967295), self.id)\n        \
                 {p}.check_value(\"due\", {p}.OptionalOf({p}.WideOf(-9223372036854775808, \
                 9223372036854775807)), self.due)\n        \
                 {p}.check_value(\"blob\", {p}.Bytes(4), self.blob)\n        \
                 {p}.check_value(\"tags\", {p}.ListOf({p}.Text(8), 4), self.tags)\n"
            )),
            "{def}"
        );
    }

    /// A Python record holds a host's value to every bound and width where it
    /// is made (docs/adr/0086, section 5.4): one call of the runtime's check
    /// per field that holds one at any depth, an element's or a present
    /// value's inside its holder's type; a lower count is held too; a truth
    /// value, a real, an enum and a record (which checks itself) are not
    /// checked here, and a list of them is held to its counts alone.
    #[test]
    fn a_python_record_checks_each_bound_and_width_where_it_is_made() {
        let p = PYTHON_PAYLOAD;
        let checks = |fields: &[(&str, &str)]| {
            let typed: Vec<(String, ValueType)> = fields
                .iter()
                .map(|(id, text)| (id.to_string(), ty(text)))
                .collect();
            let view: Vec<(&str, &ValueType)> = typed
                .iter()
                .map(|(id, written)| (id.as_str(), written))
                .collect();
            PYTHON.field_checks(&view)
        };
        assert_eq!(
            checks(&[
                ("on", "bool"),
                ("x", "float64"),
                ("level", "enum:Level"),
                ("item", "record:Item"),
                ("maybe", "optional<float32>"),
            ]),
            Vec::<String>::new()
        );
        assert_eq!(
            checks(&[("n", "uint8")]),
            vec![format!("{p}.check_value(\"n\", {p}.Whole(0, 255), self.n)")]
        );
        assert_eq!(
            checks(&[("grid", "list<list<uint8>[2]>[2..3]")]),
            vec![format!(
                "{p}.check_value(\"grid\", {p}.ListOf({p}.ListOf({p}.Whole(0, 255), 2), 3, \
                 least=2), self.grid)"
            )]
        );
        assert_eq!(
            checks(&[("items", "list<record:Item>[3]")]),
            vec![format!(
                "{p}.check_value(\"items\", {p}.ListOf(None, 3), self.items)"
            )]
        );
        assert_eq!(
            checks(&[("spare", "optional<list<string[4]>[2]>")]),
            vec![format!(
                "{p}.check_value(\"spare\", {p}.OptionalOf({p}.ListOf({p}.Text(4), 2)), \
                 self.spare)"
            )]
        );
        assert_eq!(python_post_init(&[]), "");
    }

    /// A Python reader is the runtime's name of each constructor's type, a
    /// bound and a lower count handed to the list that holds them, a record
    /// read with its schema's defaults into the machine's record class.
    #[test]
    fn a_python_reader_names_the_type_of_each_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &PYTHON, "v").expect("read");
        let p = PYTHON_PAYLOAD;
        assert_eq!(read("uint8"), format!("{p}.Whole(0, 255)"));
        assert_eq!(
            read("int64"),
            format!("{p}.WideOf(-9223372036854775808, 9223372036854775807)")
        );
        assert_eq!(read("bool"), "bool");
        assert_eq!(read("float32"), format!("{p}.Single"));
        assert_eq!(read("float64"), "float");
        assert_eq!(read("string[8]"), format!("{p}.Text(8)"));
        assert_eq!(read("bytes[4]"), format!("{p}.Bytes(4)"));
        assert_eq!(
            read("optional<uint16>"),
            format!("{p}.OptionalOf({p}.Whole(0, 65535))")
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            format!("{p}.ListOf({p}.ListOf({p}.Whole(0, 255), 2), 3)")
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            format!("{p}.ListOf({p}.ListOf({p}.Whole(0, 255), 2, least=1), 3, least=2)")
        );
        assert_eq!(read("enum:Level"), "MLevelEnum");
        assert_eq!(
            read("record:Item"),
            format!(
                "{p}.RecordOf([(\"id\", {p}.Whole(0, 4294967295)), (\"level\", MLevelEnum), \
                 (\"due\", {p}.OptionalOf({p}.WideOf(-9223372036854775808, \
                 9223372036854775807))), (\"blob\", {p}.Bytes(4)), (\"tags\", \
                 {p}.ListOf({p}.Text(8), 4))], None, MItemRecord)"
            )
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.ends_with(", \"{\\\"late\\\":[]}\", MBatchRecord)"),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor; an absent value is `null`, a 64-bit integer the text of
    /// its digits, an enum value its declared name.
    #[test]
    fn a_python_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &PYTHON, &at).expect("write");
        let p = PYTHON_PAYLOAD;
        assert_eq!(
            write("uint8", Held::place("x.n")),
            format!("{p}.value_json(x.n)")
        );
        assert_eq!(
            write("int64", Held::place("x.n")),
            format!("{p}.wide_json(x.n)")
        );
        assert_eq!(
            write("enum:Level", Held::place("x.level")),
            format!("{p}.value_json((x.level).sce_name)")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "{p}.list_json(x.maybe, lambda sce_element: {p}.value_json({p}.write_optional(\
                 sce_element, lambda sce_element: {p}.value_json(sce_element))))"
            )
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            format!(
                "{p}.list_json(x.items, lambda sce_element: {p}.record_json({{\"id\": \
                 {p}.value_json(sce_element.id), \"level\": \
                 {p}.value_json((sce_element.level).sce_name), \"due\": \
                 {p}.value_json({p}.write_optional(sce_element.due, lambda sce_element: \
                 {p}.wide_json(sce_element))), \"blob\": {p}.value_json(sce_element.blob), \
                 \"tags\": {p}.list_json(sce_element.tags, lambda sce_element: \
                 {p}.value_json(sce_element))}}))"
            )
        );
    }

    /// The value a Python `<send>` holds a composed param in: the runtime's
    /// `ScriptValue` made of a `list`, a `dict` and `None`, each scalar as a
    /// scalar param crosses (a 64-bit integer as its digits, a real as a
    /// `float`, a byte string as its Latin-1 text, an enum value as its
    /// declared name).
    #[test]
    fn a_python_send_writes_each_constructor_into_the_value_its_script_value_is_made_of() {
        let held = held();
        let wire = PythonComposed {
            machine: "M",
            carrier: PythonCarrier::Wire,
        };
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &wire, &at).expect("write");
        assert_eq!(write("uint8", Held::place("x.n")), "x.n");
        assert_eq!(write("bool", Held::place("x.b")), "x.b");
        assert_eq!(write("int64", Held::place("x.n")), "str(x.n)");
        assert_eq!(write("float32", Held::place("x.n")), "float(x.n)");
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            "[[sce_element for sce_element in sce_element] for sce_element in x.grid]"
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            "(lambda sce_element: None if sce_element is None else [sce_element for \
             sce_element in sce_element])(x.spare)"
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            "[{\"id\": sce_element.id, \"level\": (sce_element.level).sce_name, \"due\": \
             (lambda sce_element: None if sce_element is None else \
             str(sce_element))(sce_element.due), \"blob\": \
             (sce_element.blob).decode(\"latin-1\"), \"tags\": [sce_element for sce_element \
             in sce_element.tags]} for sce_element in x.items]"
        );
        assert_eq!(wire.send_param("[1]".to_string()), "_ScriptValue.of([1])");
        assert_eq!(PYTHON.send_param("x".to_string()), "x");
    }

    /// Two places whose types differ only in their bounds are one Python type,
    /// so a value that fits is copied whole, sharing no list with the place it
    /// was taken from; a number is copied as read, and a value of `T` stands
    /// where an `Optional[T]` is expected as it is.
    #[test]
    fn a_python_copy_of_a_value_that_fits_shares_no_list_with_it() {
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
                &PYTHON,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok("sce_algorithm.whole_copy(x)".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok("sce_algorithm.whole_copy(x)".to_string())
        );
        assert_eq!(copy("uint8", "uint8"), Ok("x".to_string()));
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok("[sce_copied for sce_copied in x]".to_string())
        );
    }

    /// A Python walk names no union: the one decision of what is lowered
    /// refuses one on Python first, and the walk that would declare one says
    /// why there is none.
    #[test]
    fn a_python_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &PYTHON).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(PYTHON
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
    }
}
