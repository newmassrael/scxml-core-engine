// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Python spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Python row).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record | `union:U` |
//! |---|---|---|---|---|
//! | `list[T]` | `str` / `bytes` | `Optional[T]` | the machine's record class | the machine's union class |
//!
//! No Python type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is, into a copy that shares no list with the place it was
//! taken from, since the variable it is copied into is written into in place.
//!
//! A union (docs/adr/0092, section 7, the Python row) is a class of the
//! machine and one frozen dataclass per case derived from it, named by the
//! union's class and the variant in Pascal case, since Python spells no
//! nesting a case is named by (section 7 item 1): a case that holds a record
//! holds it as `value`, an empty case holds nothing, and each answers its tag
//! by a `kind` its class holds. Its values are immutable, as a record's are,
//! so a field of a case is written by assigning the holder the next value
//! whole, as Kotlin's is.

use super::*;

/// The attribute a case of a Python union answers the value of its tag's enum
/// by, whatever key the tag is written under (docs/adr/0092, section 7, the
/// Python row: a `kind` class attribute of each case class), on a machine's
/// union as on a unit's.
pub(crate) const PYTHON_UNION_KIND: &str = "kind";

/// The attribute a case of a Python union holds its record in, on a machine's
/// union as on a unit's.
pub(crate) const PYTHON_CASE_VALUE: &str = "value";

/// The identifier of the case `variant` of a Python union: the variant in
/// Pascal case, which follows the union's class in the name of the case's own
/// class (docs/adr/0092, section 7 item 1), on a machine's union as on a
/// unit's.
pub(crate) fn python_case_ident(variant: &str) -> String {
    crate::filters::to_pascal_case(variant.to_string())
}

/// The read of the record of the case `variant` where a test proved it, on a
/// machine's union as on a unit's: a method of the union's class, written once.
pub(crate) fn python_case_read(variant: &str) -> String {
    format!(
        "proven_{}",
        crate::filters::to_snake_case(variant.to_string())
    )
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
fn python_shared_read(field: &str) -> String {
    format!(
        "proven_field_{}",
        crate::filters::to_snake_case(field.to_string())
    )
}

/// The Python spelling of what follows a union's value to read `member` where
/// it may be in the cases `left` ([`ComposedSpelling::union_member`]), as
/// [`union_member_of`] reads it: the tag's attribute; the field of the one case
/// left, through that case's proven read; or the accessor of a field several
/// cases share. `Err` as [`union_member_of`]'s.
pub(crate) fn python_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    Ok(match union_member_of(union, left, member)? {
        UnionMember::Tag => PYTHON_UNION_KIND.to_string(),
        UnionMember::Case(case) => format!("{}().{member}", python_case_read(&case.variant.name)),
        UnionMember::Shared => format!("{}()", python_shared_read(member)),
    })
}

/// The Python read of `member` of the union value `holder`, a parameter or a
/// local of an algorithm spelled as Python names it, whose type is the class
/// the union's own unit declares (docs/adr/0092, section 7, the Python row, and
/// Landing order step 5), where the value may be in the cases `left`, as
/// [`unit_union_member_of`] classifies it: the tag by `kind`, which every case
/// of the unit answers whatever key the tag is written under; the field of the
/// one case left, through that case's proven read; or the field the cases left
/// each declare, read through `value`, which every case that holds a record
/// holds it in. Python reads an attribute whatever class holds it, so the read
/// of a field the narrowing proved each case left declares is that attribute,
/// and the unit needs no accessor of it and no case analysis, as Rust's and
/// Kotlin's units need none. A case's record is the payload class its event
/// schema's unit declares, whose fields are named as the schema writes them
/// ([`crate::forge::generator::event_schema_field_ident`]). `Err` as
/// [`unit_union_member_of`]'s.
pub(crate) fn python_unit_union_read(
    holder: &str,
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    let field = crate::forge::generator::event_schema_field_ident(
        member,
        crate::generator::Language::Python,
    );
    Ok(match unit_union_member_of(union, left, member)? {
        UnionMember::Tag => format!("{holder}.{PYTHON_UNION_KIND}"),
        UnionMember::Case(case) => {
            format!(
                "{holder}.{}().{field}",
                python_case_read(&case.variant.name)
            )
        }
        UnionMember::Shared => format!("{holder}.{PYTHON_CASE_VALUE}.{field}"),
    })
}

/// A Python string literal of `text`.
fn python_literal(text: &str) -> String {
    crate::filters::py_string_literal(text.to_string())
}

/// The Python function value that writes a value of `ty` as a saved state does
/// (`sce_saved.saved_write_*`, a function of the value): a number, a truth
/// value, a text and a byte string by the runtime's writer of its type, an
/// enum, a record and a union by their own `to_saved`, a value that may be
/// absent as `None` or as its present value, and a list element by element.
/// `within` is the scope the type is written in, `depth` names the parameter of
/// each nested `lambda`, so none hides its holder's. `Err` for a byte string
/// inside a list, which a saved state would write as an array where it writes
/// a byte string as its text.
fn python_saved_writer(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &PythonComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let element = format!("sce_saved{depth}");
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!(
                "{}.to_saved",
                machine_type(ty, within, records, enums, spelling)?
            )
        }
        ValueType::Scalar(scalar) => format!("sce_saved.saved_write_{}", scalar.as_attr()),
        ValueType::Text { .. } => "sce_saved.saved_write_string".to_string(),
        ValueType::Bytes { .. } => "sce_saved.saved_write_bytes".to_string(),
        ValueType::Optional { elem } => format!(
            "lambda {element}: sce_saved.saved_write_optional({element}, {})",
            python_saved_writer(elem, within, names, spelling, depth + 1)?
        ),
        ValueType::List { elem, .. } => {
            if matches!(**elem, ValueType::Bytes { .. }) {
                return Err(
                    "a byte string inside a list, which a saved state has no text for".to_string(),
                );
            }
            format!(
                "lambda {element}: sce_saved.saved_write_list({element}, {})",
                python_saved_writer(elem, within, names, spelling, depth + 1)?
            )
        }
    })
}

/// The Python function value that reads a saved value back as a value of `ty`,
/// as [`python_saved_writer`] writes it (a function of the value and the name a
/// refusal gives it): a text and a byte string held to the bound the type
/// states, a list to its bound and to the lower count its type states, an
/// enum, a record and a union by their own `from_saved`, a value that may be
/// absent read absent from `null`. Each refusal is the runtime's
/// `StateRefusal`. `within` and `depth` as [`python_saved_writer`]'s.
fn python_saved_reader(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &PythonComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let (saved, what) = (format!("sce_saved{depth}"), format!("sce_what{depth}"));
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!(
                "{}.from_saved",
                machine_type(ty, within, records, enums, spelling)?
            )
        }
        ValueType::Scalar(scalar) => format!("sce_saved.saved_read_{}", scalar.as_attr()),
        ValueType::Text { max } => format!("sce_saved.saved_bounded_string({})", max.max),
        ValueType::Bytes { max } => format!("sce_saved.saved_bounded_bytes({})", max.max),
        ValueType::Optional { elem } => format!(
            "lambda {saved}, {what}: sce_saved.saved_read_optional({saved}, {what}, {})",
            python_saved_reader(elem, within, names, spelling, depth + 1)?
        ),
        ValueType::List { elem, max } => format!(
            "lambda {saved}, {what}: sce_saved.saved_read_list_within({saved}, {what}, {}, {}, \
             {})",
            max.min,
            max.max,
            python_saved_reader(elem, within, names, spelling, depth + 1)?
        ),
    })
}

/// The lines that give the class `ty` its saved form: `to_saved` and the
/// class method `from_saved`, defined as the functions `writes` and `reads` are
/// the bodies of, and set on the class once it is declared, as a Kotlin
/// extension is written beside its type. `about` says what the reading
/// refuses.
fn python_saved_methods(ty: &str, writes: &str, about: &str, reads: &str) -> String {
    format!(
        "def _{ty}_to_saved(self):\n    \"\"\"This value as a saved state writes it{about}.\"\"\"\n\
         {writes}\n\n\ndef _{ty}_from_saved(cls, value, what):\n    \"\"\"The value a saved \
         state holds, refused unless it is one.\"\"\"\n{reads}\n\n\n{ty}.to_saved = \
         _{ty}_to_saved\n{ty}.from_saved = classmethod(_{ty}_from_saved)"
    )
}

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
    // The count held first, by the `AlgorithmFailure` a `<send>` catches of a
    // param, then the list written by the `lambda` the runtime calls only for
    // a list that holds it.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "sce_algorithm.holds_at_least(len({}), {least}, lambda: {sent})",
            held.text
        ))
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

    // The record's saved form, a `dict` of its fields keyed by their ids in the
    // schema's order, each written and read as its type says
    // ([`python_saved_writer`], [`python_saved_reader`]): the names a record of
    // entries' saved form has (`to_saved`, `from_saved`), so the machine's
    // saving reads one spelling for either. Set on the class beside it, as a
    // Kotlin extension is written, since the class is declared before the
    // machine knows a variable holds it.
    fn saved_record_ext(
        &self,
        alias: &str,
        records: &dyn RecordEnv,
        enums: &BTreeMap<String, MachineEnum>,
    ) -> Result<Option<String>, String> {
        let held = records.record(None, alias)?;
        let ty = self.record_type(alias);
        let mut writes = String::new();
        let mut reads = String::new();
        for field in &held.schema.fields {
            let key = python_literal(&field.id);
            let refused =
                |why: String| format!("the field `{}` of `{}`: {why}", field.id, held.schema.name);
            // The fields of the record name its types in its own document's scope.
            let within = Some(held.alias);
            let write = python_saved_writer(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            let read = python_saved_reader(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            writes.push_str(&format!("        {key}: ({write})(self.{}),\n", field.id));
            reads.push_str(&format!(
                "        {}=({read})(sce_saved.saved_field(value, what, {key}), what + {}),\n",
                field.id,
                python_literal(&format!(".{}", field.id))
            ));
        }
        Ok(Some(python_saved_methods(
            &ty,
            &format!("    return {{\n{writes}    }}"),
            "",
            &format!("    return cls(\n{reads}    )"),
        )))
    }
    // The union's saved form is one object, its tag beside the fields of its
    // case (docs/adr/0092, section 6.1 item 7): written by the runtime's
    // `saved_write_union` over the record's own `to_saved`, and read by the
    // runtime's `saved_read_union`, its tag first, which names the record the
    // rest of the object is read as and refuses a name no case has.
    fn saved_union_ext(
        &self,
        def: &UnionDef,
        key: &str,
        records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        let tag = python_literal(&records.union_of_key(key)?.union.tag);
        let ty = &def.ty;
        let mut writes = String::new();
        let mut reads = String::new();
        for case in &def.cases {
            let (class, variant) = (format!("{ty}{}", case.ident), python_literal(&case.variant));
            let (written, built) = match &case.record {
                Some(record) => (
                    format!("self.{PYTHON_CASE_VALUE}.to_saved()"),
                    format!("{class}({record}.from_saved(value, what))"),
                ),
                None => ("None".to_string(), format!("{class}()")),
            };
            writes.push_str(&format!(
                "    if isinstance(self, {class}):\n        return \
                 sce_saved.saved_write_union({tag}, {variant}, {written})\n"
            ));
            reads.push_str(&format!("        {variant}: lambda: {built},\n"));
        }
        let name = python_literal(&def.name);
        Ok(Some(python_saved_methods(
            ty,
            &format!(
                "{writes}    raise AssertionError({})",
                python_literal(&format!("a value of {} is in no case of its tag", def.name))
            ),
            ": its tag beside the fields of its case",
            &format!(
                "    return sce_saved.saved_read_union(value, what, {tag}, {name}, {{\n{reads}    }})"
            ),
        )))
    }

    // The machine's own class, beside its records (docs/adr/0092, section 7,
    // the Python row): named by the union's document, so one union is one type
    // of the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    // The variant in Pascal case, which follows the union's class in the name of
    // the case's class (section 7 item 1).
    fn union_case_ident(&self, variant: &str) -> String {
        python_case_ident(variant)
    }
    // The tag is read by the one attribute every case of a Python unit and of a
    // Python machine answers it by, whatever key it is written under.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        Some(PYTHON_UNION_KIND.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        python_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        python_shared_read(field)
    }
    // A class of the machine and a frozen dataclass per case derived from it,
    // in the tag enum's order, compared whole as a record is. Every read the
    // narrowing proves is a method of the union's class, which tests the case
    // the value is in and raises `AssertionError` in any other (the one place
    // that is written, as a proven optional is read through the runtime's one
    // `proven`, docs/adr/0055, section 5); a field several cases share has
    // beside its read the method that answers this value with that field
    // replaced, which a write assigns the holder ([`Self::union_member_write`]).
    // A case reads its tag from its class, so its tag and its case cannot
    // disagree.
    fn union_def(&self, def: &UnionDef) -> String {
        let ty = &def.ty;
        let raised = |read: &str| {
            format!(
                "raise AssertionError({})",
                python_literal(&format!(
                    "a value of {} read as {read} is in another case",
                    def.name
                ))
            )
        };
        let mut reads = String::new();
        let mut cases = String::new();
        for case in &def.cases {
            let class = format!("{ty}{}", case.ident);
            let kind = format!(
                "    {PYTHON_UNION_KIND} = {}.{}\n",
                def.tag_ty, case.tag_ident
            );
            match &case.record {
                Some(record) => {
                    cases.push_str(&format!(
                        "\n\n@sce_dataclasses.dataclass(frozen=True)\nclass {class}({ty}):\n    \
                         \"\"\"The case `{variant}`, holding its record.\"\"\"\n\n    \
                         {PYTHON_CASE_VALUE}: {record}\n{kind}",
                        variant = case.variant,
                    ));
                    reads.push_str(&format!(
                        "\n    def {read}(self) -> {record}:\n        \"\"\"The record of the \
                         case `{variant}`, read where a test of the tag proved the value is in \
                         it.\"\"\"\n        if isinstance(self, {class}):\n            return \
                         self.{PYTHON_CASE_VALUE}\n        {raised}\n",
                        read = case.proven,
                        variant = case.variant,
                        raised = raised(&case.variant),
                    ));
                }
                None => cases.push_str(&format!(
                    "\n\n@sce_dataclasses.dataclass(frozen=True)\nclass {class}({ty}):\n    \
                     \"\"\"The case `{variant}`, its tag alone.\"\"\"\n\n{kind}",
                    variant = case.variant,
                )),
            }
        }
        for field in &def.shared {
            let classes: Vec<String> = field
                .cases
                .iter()
                .map(|ident| format!("{ty}{ident}"))
                .collect();
            let held = format!("({},)", classes.join(", "));
            reads.push_str(&format!(
                "\n    def {read}(self) -> {fty}:\n        \"\"\"The field `{id}` the cases that \
                 declare it share, read where a test of the tag proved the value is in one of \
                 them.\"\"\"\n        if isinstance(self, {held}):\n            return \
                 self.{PYTHON_CASE_VALUE}.{id}\n        {raised}\n\n    def {place}(self, \
                 sce_value: {fty}) -> {ty}:\n        \"\"\"This value with the field `{id}` of its \
                 case replaced by `sce_value`, where a test of the tag proved the value is in a \
                 case that declares it.\"\"\"\n        if isinstance(self, {held}):\n            \
                 return type(self)(sce_dataclasses.replace(self.{PYTHON_CASE_VALUE}, \
                 {id}=sce_value))\n        {raised}\n",
                read = field.accessor,
                place = self.union_place_of(&field.accessor),
                fty = field.ty,
                id = field.field,
                raised = raised(&field.field),
            ));
        }
        format!(
            "class {ty}:\n    \"\"\"SCE Accepted Subset §2.15: a `union:{name}` value \
             (docs/adr/0092): exactly one of the cases below, each a frozen dataclass that holds \
             its case's record as `{PYTHON_CASE_VALUE}` and answers its tag by \
             `{PYTHON_UNION_KIND}`.\"\"\"\n{reads}{cases}",
            name = def.name,
        )
    }
    // The case's class, handed the record or nothing.
    fn union_value(&self, ty: &str, _tag: &str, case: &str, record: Option<&str>) -> String {
        format!("{ty}{case}({})", record.unwrap_or_default())
    }
    // The runtime's name of the union's type, which its one reader of a value
    // reads the tag of first, by the machine's enum, and then the record of the
    // case the tag names from the same object, each case built by its class
    // (docs/adr/0092, section 4).
    fn read_union(&self, _value: &str, read: &UnionRead<'_>) -> String {
        let cases: Vec<String> = read
            .arms
            .iter()
            .map(|arm| {
                format!(
                    "{PYTHON_PAYLOAD}.CaseOf({}.{}, {}, {}{})",
                    read.tag_ty,
                    arm.tag_ident,
                    arm.record.as_deref().unwrap_or("None"),
                    read.ty,
                    arm.case_ident
                )
            })
            .collect();
        format!(
            "{PYTHON_PAYLOAD}.UnionOf({}, {}, [{}])",
            python_literal(read.tag),
            read.tag_reader,
            cases.join(", ")
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092, section
    // 2.1): the runtime's `union_json`, which orders the keys as `record_json`
    // does, or the `dict` a `<send>` holds a record in with the tag beside its
    // fields, which the runtime orders. The case is told by its class, a `dict`
    // of one writer per case, every case a key, so a value in none is never
    // written; the record of a case is bound in its writer, as a record's
    // writer reads it.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let tag = python_literal(tag);
        let arms: Vec<String> = arms
            .iter()
            .map(|arm| {
                let variant = python_literal(&arm.variant);
                let members: Vec<String> = arm
                    .fields
                    .iter()
                    .flatten()
                    .map(|(id, value)| format!("{}: {value}", python_literal(id)))
                    .collect();
                let written = match self.carrier {
                    PythonCarrier::Json => format!(
                        "{PYTHON_PAYLOAD}.union_json({tag}, {variant}, {{{}}})",
                        members.join(", ")
                    ),
                    PythonCarrier::Wire => {
                        let mut pairs = vec![format!("{tag}: {variant}")];
                        pairs.extend(members);
                        format!("{{{}}}", pairs.join(", "))
                    }
                };
                let body = match &arm.fields {
                    Some(_) => format!(
                        "(lambda {}: {written})(sce_union.{PYTHON_CASE_VALUE})",
                        self.write_binding()
                    ),
                    None => written,
                };
                format!("{ty}{}: lambda sce_union: {body}", arm.case_ident)
            })
            .collect();
        format!(
            "(lambda sce_union: {{{}}}[type(sce_union)](sce_union))({})",
            arms.join(", "),
            held.text
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        python_union_member(union, left, member)
    }
    // A field several cases share is written through the method that answers
    // the union with that field replaced ([`Self::union_def`]); a case's record
    // is rebuilt by the case's own class, so its name is kept free only.
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}_with")
    }
    // A Python union is an immutable value, so no field of it is a place a
    // value can be assigned to: a write assigns the holder the next value
    // whole ([`Self::union_member_write`]).
    fn union_member_place(
        &self,
        _holder: &str,
        union: &ScopedUnion,
        _left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        Err(format!(
            "`{member}` of `union:{}` is a field of an immutable Python value, which has no place \
             of its own: its holder is assigned the next value whole",
            union.name
        ))
    }
    // The holder is assigned the union in the case it is in, its record with
    // the field replaced, as a record variable's field is written
    // (`sce_dataclasses.replace`, `StaticTarget::assign_field`); a field
    // several cases share through the method that replaces it in whichever of
    // them the value is in.
    fn union_member_write(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<UnionMemberWrite, String> {
        let next: UnionNext = match union_member_of(union, left, member)? {
            UnionMember::Tag => {
                return Err(format!(
                    "the tag `{member}` of `union:{}` is never written alone: the case changes by \
                     a whole assignment (docs/adr/0092, section 6.1 item 6)",
                    union.name
                ))
            }
            UnionMember::Case(case) => {
                let class = format!(
                    "{}{}",
                    self.union_type(&union.name),
                    self.union_case_ident(&case.variant.name)
                );
                let record = format!("{holder}.{}()", python_case_read(&case.variant.name));
                let field = member.to_string();
                Box::new(move |value: &str| {
                    format!("{class}(sce_dataclasses.replace({record}, {field}=({value})))")
                })
            }
            UnionMember::Shared => {
                let replace = format!(
                    "{holder}.{}",
                    self.union_place_of(&python_shared_read(member))
                );
                Box::new(move |value: &str| format!("{replace}({value})"))
            }
        };
        Ok(UnionMemberWrite::Holder {
            holder: holder.to_string(),
            next,
        })
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
    /// composed value sent (docs/adr/0086, sections 5 and 6), and a union held,
    /// read by its tag, lifted and saved (docs/adr/0092, section 7); no
    /// variable of a record that holds a composition yet.
    #[test]
    fn the_python_row_lowers_every_composed_position_but_a_composed_record_variable() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Python),
            ComposedLowering {
                name: "python",
                reads: true,
                copies: true,
                unions: true,
                records: false,
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
        // A list sent where its field asks a lower count it does not prove is
        // held to the count before it is written (docs/adr/0086, section 15.6).
        assert_eq!(
            wire.send_counted(
                &Held::place("self.v_taking"),
                "ops",
                1,
                "_ScriptValue.of([])".to_string()
            ),
            Some(
                "sce_algorithm.holds_at_least(len(self.v_taking), 1, lambda: _ScriptValue.of([]))"
                    .to_string()
            )
        );
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

    /// The case of the shape union named `name`.
    fn case<'s>(shapes: &'s Shapes, name: &str) -> &'s ScopedCase {
        shapes
            .union
            .cases
            .iter()
            .find(|case| case.variant.name == name)
            .expect("a case")
    }

    /// A union is the machine's own class at any depth, named by its document
    /// (docs/adr/0092, section 7, the Python row); its tag is read by `kind`
    /// whatever key it is written under, and a case is built by its own class.
    #[test]
    fn a_python_machine_holds_a_union_in_its_own_class() {
        let shapes = shapes();
        let kinds = kinds();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &PYTHON),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &PYTHON
            ),
            Ok("list[Optional[MShapeUnion]]".to_string())
        );
        assert_eq!(PYTHON.union_tag_read("type"), Some("kind".to_string()));
        assert_eq!(
            PYTHON.union_value("MShapeUnion", "kind", "Dot", None),
            "MShapeUnionDot()"
        );
        assert_eq!(
            PYTHON.union_value("MShapeUnion", "kind", "Circle", Some("r")),
            "MShapeUnionCircle(r)"
        );
    }

    /// The declaration: the union's class, its proven read per case that holds
    /// a record and its accessor of `radius`, which `circle` and `disc` share,
    /// each testing the case and raising in any other, beside the method that
    /// replaces the shared field; and one frozen dataclass per case, derived
    /// from it, holding the record as `value` and its tag as `kind`.
    #[test]
    fn a_python_union_is_declared_as_a_class_and_one_dataclass_per_case() {
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &PYTHON).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "proven_field_radius".to_string(),
                ty: "int".to_string(),
                cases: vec!["Circle".to_string(), "Disc".to_string()],
            }]
        );
        let text = PYTHON.union_def(&def);
        let raised = |read: &str| {
            format!("raise AssertionError(\"a value of shape read as {read} is in another case\")")
        };
        for fragment in [
            "class MShapeUnion:\n    \"\"\"SCE Accepted Subset §2.15: a `union:shape` value"
                .to_string(),
            format!(
                "    def proven_circle(self) -> MRadiusRecord:\n        \"\"\"The record of the \
                 case `circle`, read where a test of the tag proved the value is in it.\"\"\"\n        \
                 if isinstance(self, MShapeUnionCircle):\n            return self.value\n        \
                 {}\n",
                raised("circle")
            ),
            format!(
                "    def proven_field_radius(self) -> int:\n        \"\"\"The field `radius` the \
                 cases that declare it share, read where a test of the tag proved the value is \
                 in one of them.\"\"\"\n        if isinstance(self, (MShapeUnionCircle, \
                 MShapeUnionDisc,)):\n            return self.value.radius\n        {}\n",
                raised("radius")
            ),
            "    def proven_field_radius_with(self, sce_value: int) -> MShapeUnion:\n".to_string(),
            "            return type(self)(sce_dataclasses.replace(self.value, \
             radius=sce_value))\n"
                .to_string(),
            "\n\n@sce_dataclasses.dataclass(frozen=True)\nclass MShapeUnionDot(MShapeUnion):\n    \
             \"\"\"The case `dot`, its tag alone.\"\"\"\n\n    kind = MKindEnum.Dot\n"
                .to_string(),
            "\n\n@sce_dataclasses.dataclass(frozen=True)\nclass MShapeUnionCircle(MShapeUnion):\n    \
             \"\"\"The case `circle`, holding its record.\"\"\"\n\n    value: MRadiusRecord\n    \
             kind = MKindEnum.Circle\n"
                .to_string(),
        ] {
            assert!(text.contains(&fragment), "expected `{fragment}` in:\n{text}");
        }
        // An empty case has nothing to read.
        assert!(!text.contains("proven_dot"), "{text}");
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// attribute, a field of the one case left through that case's proven
    /// read, and a field two cases share through its accessor; a write assigns
    /// the holder the next value whole, the case's record with the field
    /// replaced, and the tag is never written alone.
    #[test]
    fn a_python_proven_read_and_write_are_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        assert_eq!(python_union_member(union, &all, "kind"), Ok("kind".into()));
        assert_eq!(
            python_union_member(union, &[case(&shapes, "disc")], "radius"),
            Ok("proven_disc().radius".into())
        );
        assert_eq!(
            PYTHON.union_member(union, &both, "radius"),
            Ok("proven_field_radius()".into())
        );
        assert!(python_union_member(union, &[case(&shapes, "dot")], "radius").is_err());
        assert!(PYTHON
            .union_member_place("self.v_s", union, &both, "radius")
            .is_err());
        let written = |left: &[&ScopedCase]| match PYTHON
            .union_member_write("self.v_s", union, left, "radius")
        {
            Ok(UnionMemberWrite::Holder { holder, next }) => (holder, next("7")),
            Ok(UnionMemberWrite::Place(place)) => {
                panic!("a Python union has no place of a field, not {place}")
            }
            Err(why) => panic!("{why}"),
        };
        assert_eq!(
            written(&[case(&shapes, "disc")]),
            (
                "self.v_s".to_string(),
                "MShapeUnionDisc(sce_dataclasses.replace(self.v_s.proven_disc(), radius=(7)))"
                    .to_string()
            )
        );
        assert_eq!(
            written(&both),
            (
                "self.v_s".to_string(),
                "self.v_s.proven_field_radius_with(7)".to_string()
            )
        );
        let Err(tag) = PYTHON.union_member_write("self.v_s", union, &all, "kind") else {
            panic!("the tag is never written alone");
        };
        assert!(tag.contains("never written alone"), "{tag}");
    }

    /// An algorithm's union is its unit's class (docs/adr/0092, Landing order
    /// step 5): the tag by `kind`, the field of the one case left by that
    /// case's proven read, a field several cases declare through the `value`
    /// each holds its record in, and no read where no case left declares the
    /// field.
    #[test]
    fn an_algorithms_union_is_read_on_its_units_class() {
        let shapes = shapes();
        let union = &shapes.union;
        let read = |left: &[&ScopedCase], member: &str| {
            python_unit_union_read("shape", union, left, member)
        };
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        assert_eq!(read(&[], "kind"), Ok("shape.kind".to_string()));
        assert_eq!(
            read(&[case(&shapes, "circle")], "radius"),
            Ok("shape.proven_circle().radius".to_string())
        );
        assert_eq!(read(&both, "radius"), Ok("shape.value.radius".to_string()));
        assert!(read(&[], "radius").is_err());
        assert!(read(&[case(&shapes, "dot"), case(&shapes, "circle")], "radius").is_err());
    }

    /// The reader is the runtime's name of the union's type, its tag read by
    /// the machine's enum and each case built by its class from the record its
    /// arm reads; the writer tells the case by its class and writes the tag
    /// among the case's fields, as the JSON text and as the `dict` a `<send>`
    /// holds; a union is copied whole, sharing no list with its source.
    #[test]
    fn a_python_union_is_read_tag_first_and_written_by_its_case() {
        let shapes = shapes();
        let kinds = kinds();
        let p = PYTHON_PAYLOAD;
        assert_eq!(
            reader(&ty("union:Shape"), None, &shapes, &kinds, &PYTHON, "v"),
            Ok(format!(
                "{p}.UnionOf(\"kind\", MKindEnum, [{p}.CaseOf(MKindEnum.Dot, None, \
                 MShapeUnionDot), {p}.CaseOf(MKindEnum.Circle, {p}.RecordOf([(\"radius\", \
                 {p}.Whole(0, 65535))], None, MRadiusRecord), MShapeUnionCircle), \
                 {p}.CaseOf(MKindEnum.Disc, {p}.RecordOf([(\"radius\", {p}.Whole(0, 65535))], \
                 None, MRadiusRecord), MShapeUnionDisc)])"
            ))
        );
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &PYTHON,
                &Held::place("x.s")
            ),
            Ok(format!(
                "(lambda sce_union: {{MShapeUnionDot: lambda sce_union: {p}.union_json(\"kind\", \
                 \"dot\", {{}}), MShapeUnionCircle: lambda sce_union: (lambda sce_element: \
                 {p}.union_json(\"kind\", \"circle\", {{\"radius\": \
                 {p}.value_json(sce_element.radius)}}))(sce_union.value), MShapeUnionDisc: \
                 lambda sce_union: (lambda sce_element: {p}.union_json(\"kind\", \"disc\", \
                 {{\"radius\": {p}.value_json(sce_element.radius)}}))(sce_union.value)}}\
                 [type(sce_union)](sce_union))(x.s)"
            ))
        );
        let wire = PythonComposed {
            machine: "M",
            carrier: PythonCarrier::Wire,
        };
        let sent = writer(
            &ty("union:Shape"),
            None,
            &shapes,
            &wire,
            &Held::place("x.s"),
        )
        .expect("write");
        for fragment in [
            "MShapeUnionDot: lambda sce_union: {\"kind\": \"dot\"}",
            "MShapeUnionCircle: lambda sce_union: (lambda sce_element: {\"kind\": \"circle\", \
             \"radius\": sce_element.radius})(sce_union.value)",
        ] {
            assert!(sent.contains(fragment), "expected `{fragment}` in:\n{sent}");
        }
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &shapes,
                &kinds,
                &PYTHON,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("union:Shape", "union:Shape"),
            Ok("sce_algorithm.whole_copy(x)".to_string())
        );
        assert_eq!(
            copy("optional<union:Shape>", "optional<union:Shape>"),
            Ok("sce_algorithm.whole_copy(x)".to_string())
        );
    }

    /// A union is saved as its tag beside the fields of its case, written by
    /// the runtime's `saved_write_union` over the record's own `to_saved`, and
    /// read back by the runtime's `saved_read_union`, tag first (docs/adr/0092,
    /// section 6.1 item 7); both are set on the union's class.
    #[test]
    fn a_python_union_is_saved_by_its_tag_beside_its_case() {
        let shapes = shapes();
        let def = union_definition(SHAPE_KEY, &shapes, &kinds(), &PYTHON).expect("declared");
        let text = PYTHON
            .saved_union_ext(&def, SHAPE_KEY, &shapes)
            .expect("spelled")
            .expect("a Python union carries its saved form");
        for fragment in [
            "def _MShapeUnion_to_saved(self):\n",
            "    if isinstance(self, MShapeUnionDot):\n        return \
             sce_saved.saved_write_union(\"kind\", \"dot\", None)\n",
            "    if isinstance(self, MShapeUnionCircle):\n        return \
             sce_saved.saved_write_union(\"kind\", \"circle\", self.value.to_saved())\n",
            "    raise AssertionError(\"a value of shape is in no case of its tag\")",
            "def _MShapeUnion_from_saved(cls, value, what):\n",
            "    return sce_saved.saved_read_union(value, what, \"kind\", \"shape\", {\n        \
             \"dot\": lambda: MShapeUnionDot(),\n        \"circle\": lambda: \
             MShapeUnionCircle(MRadiusRecord.from_saved(value, what)),\n",
            "MShapeUnion.to_saved = _MShapeUnion_to_saved\nMShapeUnion.from_saved = \
             classmethod(_MShapeUnion_from_saved)",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }

    /// A composed record's saved form: each field written by the runtime's
    /// writer of its type and read back held to every bound and count its type
    /// states, a list's lower count included (docs/adr/0086, section 15.6),
    /// both set on the record's class.
    #[test]
    fn a_python_composed_record_is_saved_field_by_field_by_its_types() {
        let held = held();
        let enums = enums();
        let text = PYTHON
            .saved_record_ext("Item", &held, &enums)
            .expect("spelled")
            .expect("a Python composed record carries its saved form");
        for fragment in [
            "def _MItemRecord_to_saved(self):\n    \"\"\"This value as a saved state writes \
             it.\"\"\"\n    return {\n        \"id\": (sce_saved.saved_write_uint32)(self.id),\n",
            "        \"level\": (MLevelEnum.to_saved)(self.level),\n",
            "        \"due\": (lambda sce_saved0: sce_saved.saved_write_optional(sce_saved0, \
             sce_saved.saved_write_int64))(self.due),\n",
            "        \"tags\": (lambda sce_saved0: sce_saved.saved_write_list(sce_saved0, \
             sce_saved.saved_write_string))(self.tags),\n",
            "        id=(sce_saved.saved_read_uint32)(sce_saved.saved_field(value, what, \"id\"), \
             what + \".id\"),\n",
            "        blob=(sce_saved.saved_bounded_bytes(4))(sce_saved.saved_field(value, what, \
             \"blob\"), what + \".blob\"),\n",
            "        tags=(lambda sce_saved0, sce_what0: \
             sce_saved.saved_read_list_within(sce_saved0, sce_what0, 0, 4, \
             sce_saved.saved_bounded_string(8)))(sce_saved.saved_field(value, what, \"tags\"), \
             what + \".tags\"),\n",
            "MItemRecord.to_saved = _MItemRecord_to_saved\nMItemRecord.from_saved = \
             classmethod(_MItemRecord_from_saved)",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }
}
