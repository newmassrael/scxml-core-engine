// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Go row), and the checks a Go raise seam
//! runs over a host's value before it encodes it (section 5.4).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record | `union:U` |
//! |---|---|---|---|---|
//! | `[]T` | `string` / `[]byte` | `optional.Of[T]` | the machine's struct | the machine's value struct |
//!
//! No Go type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is: the list a place holds into a slice of its own, since
//! the variable it is copied into is appended to in place.
//!
//! A union (docs/adr/0092, section 7, the Go row, and its item 4) is a value
//! struct of the machine, not an interface: an unexported tag and one
//! unexported member per case that holds a record, built only by the
//! constructor of its case (`<Union><Case>(r R)`, `<Union><Case>()` for an
//! empty case), its tag answered by `Kind()` and the record of a case a test
//! proved by `Proven<Case>()`. A field of a case is written in place, through
//! the pointer an unexported accessor answers where the narrowing proved the
//! case, as a Rust union is.

use super::*;

/// The Go spellings of the walks, for the machine `machine` (section 7.1, the
/// Go row): the runtime's `PayloadValue` reads each constructor, and its
/// writers write them into `carrier`.
///
/// Go spells the type of every parameter and result of a function literal, so
/// a reader and a copy name the type the walk hands them for the part they
/// read or copy. A writer and a copy walk a list by a `range` loop and a value
/// that may be absent by its `Get`, which name no type. A binding of a nested
/// literal, loop or `if` shadows its holder's of the same name, which Go
/// allows: the holder's is read where the inner one is declared, and each part
/// reads its own.
pub struct GoComposed<'a> {
    pub machine: &'a str,
    pub carrier: GoCarrier,
}

/// What a Go writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoCarrier {
    /// The JSON text itself, by the runtime's `ListJSON`, `RecordJSON` and
    /// `ValueJSON`: what the inject seam writes as the event's `Data` beside
    /// the typed carrier (section 3.1).
    Json,
    /// The value a `<send>` holds each param in (`StaticTarget::wire_value`)
    /// until the runtime's `ScriptValueToJSON` writes them as one JSON object: a
    /// list as an `[]any` of its elements, a record as a `map[string]any` of its
    /// fields, whose keys it writes ascending by their bytes at every depth, an
    /// absent value as `nil`, and every scalar as a `<send>`'s scalar param is
    /// (section 5.3; ARCHITECTURE.md, "JSON Object Key Order").
    Wire,
}

/// The name a Go reader binds the value of an element, a present value or a
/// field to: the parameter of the function literal that reads it.
const GO_READ_BINDING: &str = "sceValue";

/// The name a Go writer and a copy bind an element or a present value to.
const GO_ELEMENT_BINDING: &str = "sceElement";

/// The method a Go union answers the value of its tag's enum by, whatever key
/// the tag is written under (docs/adr/0092, section 7, the Go row: `Kind() E`),
/// on a machine's union as on a unit's.
pub(crate) const GO_UNION_KIND: &str = "Kind";

/// The member of a Go union that holds its tag, on a machine's union as on a
/// unit's.
pub(crate) const GO_UNION_TAG: &str = "tag";

/// The name a Go writer binds the union it writes to, in the function literal
/// that tells its cases apart.
const GO_UNION_BINDING: &str = "sceUnion";

/// The identifier of the case `variant` of a Go union: the variant in Pascal
/// case, which follows the union's type in the name of the case's constructor
/// (docs/adr/0092, section 7 item 1: Go has no nesting), on a machine's union
/// as on a unit's.
pub(crate) fn go_case_ident(variant: &str) -> String {
    crate::filters::to_pascal_case(variant.to_string())
}

/// The member of a Go union that holds the record of the case whose identifier
/// is `case` ([`go_case_ident`]): unexported, and never the tag's member, since
/// every one begins `case`; on a machine's union as on a unit's.
pub(crate) fn go_case_member(case: &str) -> String {
    format!("case{case}")
}

/// The read of the record of the case `variant` where a test proved it, on a
/// machine's union as on a unit's.
pub(crate) fn go_case_read(variant: &str) -> String {
    format!(
        "Proven{}",
        crate::filters::to_pascal_case(variant.to_string())
    )
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
fn go_shared_read(field: &str) -> String {
    format!(
        "ProvenField{}",
        crate::filters::to_pascal_case(field.to_string())
    )
}

/// The unexported accessor that answers a pointer to what the read `read`
/// answers (a case's record, a shared field), which a write of a field goes
/// through (docs/adr/0092, section 6.1 item 6): unexported, so a host that
/// holds a union builds a new one by a constructor rather than writing into a
/// case.
fn go_place_of(read: &str) -> String {
    let mut chars = read.chars();
    let first: String = chars
        .next()
        .map(|c| c.to_lowercase().collect())
        .unwrap_or_default();
    format!("{first}{}Place", chars.as_str())
}

/// A Go string literal of `text`.
fn go_literal(text: &str) -> String {
    format!("\"{}\"", crate::filters::escape_go(text.to_string()))
}

/// The Go spelling of what follows a union's value to read `member` where it
/// may be in the cases `left` ([`ComposedSpelling::union_member`]), as
/// [`union_member_of`] reads it: the tag's method; the field of the one case
/// left, through that case's proven read; or the accessor of a field several
/// cases share. `Err` as [`union_member_of`]'s.
pub(crate) fn go_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    Ok(match union_member_of(union, left, member)? {
        UnionMember::Tag => format!("{GO_UNION_KIND}()"),
        UnionMember::Case(case) => format!("{}().{member}", go_case_read(&case.variant.name)),
        UnionMember::Shared => format!("{}()", go_shared_read(member)),
    })
}

/// The Go function value that writes a value of `ty` as a saved state does
/// (`sce.SavedWrite*`, a `func(T) any`): a number, a truth value, a text and a
/// byte string by the runtime's writer of its type, an enum, a record and a
/// union by their own `ToSaved` (a method expression of the type), a value that
/// may be absent as `nil` or as its present value, and a list element by
/// element. `within` is the scope the type is written in, `depth` names the
/// parameter of each nested literal, so none hides its holder's. `Err` for a
/// byte string inside a list, which a saved state would write as an array
/// where it writes a byte string as its text.
fn go_saved_writer(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &GoComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let element = format!("sceSaved{depth}");
    let typed = |ty: &ValueType| machine_type(ty, within, records, enums, spelling);
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!("{}.ToSaved", typed(ty)?)
        }
        ValueType::Scalar(scalar) => format!(
            "sce.SavedWrite{}",
            crate::filters::to_pascal_case(scalar.as_attr())
        ),
        ValueType::Text { .. } => "sce.SavedWriteString".to_string(),
        ValueType::Bytes { .. } => "sce.SavedWriteBytes".to_string(),
        ValueType::Optional { elem } => format!(
            "func({element} {}) any {{ return optional.Wire({element}, {}) }}",
            typed(ty)?,
            go_saved_writer(elem, within, names, spelling, depth + 1)?
        ),
        ValueType::List { elem, .. } => {
            if matches!(**elem, ValueType::Bytes { .. }) {
                return Err(
                    "a byte string inside a list, which a saved state has no text for".to_string(),
                );
            }
            format!(
                "func({element} {}) any {{ return sce.SavedWriteList({element}, {}) }}",
                typed(ty)?,
                go_saved_writer(elem, within, names, spelling, depth + 1)?
            )
        }
    })
}

/// The Go function value that reads a saved value back as a value of `ty`, as
/// [`go_saved_writer`] writes it (a `func(any, string) (T, error)`, the value
/// and the name a refusal gives it): a text and a byte string held to the
/// bound the type states, a list to its bound and to the lower count its type
/// states, an enum, a record and a union by their own `<Type>FromSaved`, a
/// value that may be absent read absent from `null`. Each refusal is the
/// runtime's `StateRefusal`. `within` and `depth` as [`go_saved_writer`]'s.
fn go_saved_reader(
    ty: &ValueType,
    within: Option<&str>,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &GoComposed<'_>,
    depth: usize,
) -> Result<String, String> {
    let (records, enums) = names;
    let (saved, what) = (format!("sceSaved{depth}"), format!("sceWhat{depth}"));
    let typed = |ty: &ValueType| machine_type(ty, within, records, enums, spelling);
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!("{}FromSaved", typed(ty)?)
        }
        ValueType::Scalar(scalar) => format!(
            "sce.SavedRead{}",
            crate::filters::to_pascal_case(scalar.as_attr())
        ),
        ValueType::Text { max } => format!("sce.SavedBoundedString({})", max.max),
        ValueType::Bytes { max } => format!("sce.SavedBoundedBytes({})", max.max),
        ValueType::Optional { elem } => {
            let present = typed(elem)?;
            format!(
                "func({saved} any, {what} string) ({}, error) {{ return \
                 sce.SavedReadOptional({saved}, {what}, {}, optional.None[{present}], \
                 optional.Some[{present}]) }}",
                typed(ty)?,
                go_saved_reader(elem, within, names, spelling, depth + 1)?
            )
        }
        ValueType::List { elem, max } => format!(
            "func({saved} any, {what} string) ({}, error) {{ return \
             sce.SavedReadListWithin({saved}, {what}, {}, {}, {}) }}",
            typed(ty)?,
            max.min,
            max.max,
            go_saved_reader(elem, within, names, spelling, depth + 1)?
        ),
    })
}

/// The check of a Go value of `ty`, written in the scope `within` names, at
/// `value`, named `path` (an expression of a Go `string`): an expression of
/// type `error` that is the runtime's refusal, naming the path to the place,
/// where the value is past a bound its type declares at any depth, and `nil`
/// where it is not (docs/adr/0086, section 5.4). One call of the runtime's
/// `CheckText`, `CheckBytes`, `CheckList` or `CheckListWithin` per constructor
/// that holds a bound, and of `optional.Check` for a value that may be absent,
/// the check of an element or a present value inside the function literal of
/// the call that holds it, its bindings named by their `depth`; a record is
/// checked field by field where it stands, at the path of each field below
/// its own. `None` for a type that holds no bound at any depth: a number, an
/// enum, an optional one of them and a record of those.
///
/// A Go struct has no constructor that could refuse a value, so a Go raise
/// seam runs this check over the payload a host hands it before it encodes it,
/// a record a list holds included.
fn go_bound_check(
    ty: &ValueType,
    within: Option<&str>,
    path: &str,
    value: &str,
    depth: usize,
    names: (&dyn RecordEnv, &BTreeMap<String, MachineEnum>),
    spelling: &GoComposed<'_>,
) -> Result<Option<String>, String> {
    let (records, enums) = names;
    let (inner_path, inner_value) = (format!("scePath{depth}"), format!("sceValue{depth}"));
    Ok(match ty {
        ValueType::Text { max } => Some(format!("sce.CheckText({path}, {value}, {})", max.max)),
        ValueType::Bytes { max } => Some(format!("sce.CheckBytes({path}, {value}, {})", max.max)),
        // The count is held whatever the elements are; an element with
        // nothing of its own to check is handed to a literal that checks
        // nothing.
        ValueType::List { elem, max } => {
            let elem_ty = machine_type(elem, within, records, enums, spelling)?;
            let each = match go_bound_check(
                elem,
                within,
                &inner_path,
                &inner_value,
                depth + 1,
                names,
                spelling,
            )? {
                Some(inner) => format!(
                    "func({inner_path} string, {inner_value} {elem_ty}) error {{ return \
                     {inner} }}"
                ),
                None => format!("func(string, {elem_ty}) error {{ return nil }}"),
            };
            Some(if max.min == 0 {
                format!("sce.CheckList({path}, {value}, {}, {each})", max.max)
            } else {
                format!(
                    "sce.CheckListWithin({path}, {value}, {}, {}, {each})",
                    max.min, max.max
                )
            })
        }
        // An absent value holds nothing to check; a present one is checked at
        // the place's own path.
        ValueType::Optional { elem } => {
            let Some(inner) =
                go_bound_check(elem, within, path, &inner_value, depth + 1, names, spelling)?
            else {
                return Ok(None);
            };
            let elem_ty = machine_type(elem, within, records, enums, spelling)?;
            Some(format!(
                "optional.Check({value}, func({inner_value} {elem_ty}) error {{ return {inner} }})"
            ))
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let mut checks = Vec::new();
            for field in &held.schema.fields {
                let below = format!(
                    "{path} + \"/{}\"",
                    crate::filters::escape_go(field.id.clone())
                );
                if let Some(check) = go_bound_check(
                    &field.sce_type,
                    Some(held.alias),
                    &below,
                    &format!("{value}.{}", field.id),
                    depth,
                    names,
                    spelling,
                )? {
                    checks.push(check);
                }
            }
            match checks.as_slice() {
                [] => None,
                [one] => Some(one.clone()),
                many => {
                    let each: String = many
                        .iter()
                        .map(|check| format!("if err := {check}; err != nil {{ return err }}; "))
                        .collect();
                    Some(format!("func() error {{ {each}return nil }}()"))
                }
            }
        }
        ValueType::Scalar(_) => None,
        // The record of the case the value is in, checked as a record is, at
        // the union's own path: a field of a case stands at the union's place
        // (docs/adr/0092, section 2.1). A case whose record holds no bound, and
        // an empty case, check nothing.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            let tag = machine_enum(enums, held.tag_enum()?)?;
            let mut arms = String::new();
            for case in &held.union.cases {
                let Some(record) = &case.record else {
                    continue;
                };
                let variant = case.variant.name.as_str();
                let Some((_, tag_ident)) = tag.variants.iter().find(|(name, _)| name == variant)
                else {
                    return Err(format!(
                        "`union:{alias}`: the machine's `{}` has no variant `{variant}`",
                        tag.ty
                    ));
                };
                if let Some(check) = go_bound_check(
                    &ValueType::Record {
                        alias: record.clone(),
                    },
                    Some(held.key.as_str()),
                    path,
                    &format!("{value}.{}()", go_case_read(variant)),
                    depth,
                    names,
                    spelling,
                )? {
                    arms.push_str(&format!("case {tag_ident}: return {check}; "));
                }
            }
            (!arms.is_empty()).then(|| {
                format!(
                    "func() error {{ switch {value}.{GO_UNION_KIND}() {{ {arms}}}; return nil }}()"
                )
            })
        }
    })
}

/// The checks a Go raise seam runs over the fields of the payload a host hands
/// it, each a parameter of the seam named by the field's id, before it encodes
/// any of it ([`go_bound_check`]): one expression of type `error` per field
/// whose type holds a bound at any depth, in the schema's order, the path of
/// each the field's id.
pub(crate) fn go_raise_checks(
    held: HeldRecord<'_>,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &GoComposed<'_>,
) -> Result<Vec<String>, String> {
    let mut checks = Vec::new();
    for field in &held.schema.fields {
        let path = format!("\"{}\"", crate::filters::escape_go(field.id.clone()));
        if let Some(check) = go_bound_check(
            &field.sce_type,
            Some(held.alias),
            &path,
            &field.id,
            0,
            (records, enums),
            spelling,
        )? {
            checks.push(check);
        }
    }
    Ok(checks)
}

impl ComposedSpelling for GoComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::go_type(ty).to_string()
    }
    // No Go type has a constant parameter a bound could be (section 7.4): the
    // reader holds a value to its bound where it is read, and a raise seam
    // where a host hands it one.
    fn text_type(&self, _max: u32) -> String {
        "string".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "[]byte".to_string()
    }
    // The forge runtime's optional (docs/adr/0055, section 5), the one Go
    // spelling of a value that may be absent.
    fn optional_type(&self, elem: &str) -> String {
        format!("optional.Of[{elem}]")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("[]{elem}")
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
    // A struct of the schema's fields in the schema's order, each named as the
    // schema names it, which is how a read names it. Its fields are unexported,
    // as a record of entries' are, so a host builds one by its constructor to
    // hand a raise seam, which holds every bound of it before it encodes it
    // (docs/adr/0086, section 5.4): a Go struct has no constructor that could
    // refuse a value where it is made, so no check is declared beside it.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("\t{id} {ty}\n"))
            .collect();
        let params: Vec<String> = fields.iter().map(|(id, ty)| format!("{id} {ty}")).collect();
        let values: Vec<String> = fields.iter().map(|(id, _)| format!("{id}: {id}")).collect();
        format!(
            "// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed\n\
             // value (docs/adr/0086).\n\
             type {ty} struct {{\n{members}}}\n\n\
             // New{ty} is a `record:{alias}` value of the fields given: what a host builds\n\
             // to hand a raise seam, which holds it to every bound its type declares before\n\
             // it sends it (docs/adr/0086, section 5.4).\n\
             func New{ty}({}) {ty} {{\n\treturn {ty}{{{}}}\n}}",
            params.join(", "),
            values.join(", ")
        )
    }

    fn read_binding(&self) -> &'static str {
        GO_READ_BINDING
    }
    // The width is in the reader's type argument, so a field widened in its
    // schema cannot keep being read at the old width.
    fn read_scalar(&self, ty: &SceType, value: &str) -> String {
        let go = crate::forge::generator::go_type;
        match ty {
            SceType::Uint64 => format!("sce.ValueUnsigned64({value})"),
            SceType::Int64 => format!("sce.ValueSigned64({value})"),
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 => {
                format!("sce.ValueUnsigned[{}]({value})", go(ty))
            }
            SceType::Int8 | SceType::Int16 | SceType::Int32 => {
                format!("sce.ValueSigned[{}]({value})", go(ty))
            }
            SceType::Float32 | SceType::Float64 => {
                format!("sce.ValueFloat[{}]({value})", go(ty))
            }
            SceType::Bool => format!("{value}.Bool()"),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        }
    }
    // The variant whose declared name the wire carries; a name the enum does
    // not declare is refused with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "case \"{}\": return {ident}, nil; ",
                    crate::filters::escape_go(declared.clone())
                )
            })
            .collect();
        format!(
            "func() ({}, error) {{ sceName, err := {value}.Name(); if err != nil {{ return 0, err \
             }}; switch sceName {{ {arms}}}; return 0, {value}.UnknownVariant(sceName, \"{}\") }}()",
            held.ty,
            crate::filters::escape_go(held.alias.clone())
        )
    }
    fn read_text(&self, max: u32, value: &str) -> String {
        format!("{value}.Text({max})")
    }
    fn read_bytes(&self, max: u32, value: &str) -> String {
        format!("{value}.Bytes({max})")
    }
    fn read_optional(&self, value: &str, elem: &str, elem_ty: &str) -> String {
        format!(
            "optional.Read({value}, func({GO_READ_BINDING} sce.PayloadValue) ({elem_ty}, error) \
             {{ return {elem} }})"
        )
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, elem_ty: &str) -> String {
        let each = format!(
            "func({GO_READ_BINDING} sce.PayloadValue) ({elem_ty}, error) {{ return {elem} }}"
        );
        if least == 0 {
            format!("sce.ReadList({value}, {max}, {each})")
        } else {
            format!("sce.ReadListWithin({value}, {least}, {max}, {each})")
        }
    }
    // Each field read into the struct in the schema's order, the first
    // refusal returned as it is; a field a later revision added is read out
    // of the defaults where the value lacks it.
    fn read_record(
        &self,
        value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults
            .map(|json| crate::filters::escape_go(json.to_string()))
            .unwrap_or_default();
        let members: String = fields
            .iter()
            .map(|(id, read)| {
                format!(
                    "{{ {GO_READ_BINDING}, err := sceRecord.Field(\"{}\"); if err != nil {{ \
                     return {ty}{{}}, err }}; if sceOut.{id}, err = {read}; err != nil {{ return \
                     {ty}{{}}, err }} }}; ",
                    crate::filters::escape_go(id.clone())
                )
            })
            .collect();
        format!(
            "func() ({ty}, error) {{ sceRecord, err := {value}.Record(\"{defaults}\"); if err != \
             nil {{ return {ty}{{}}, err }}; var sceOut {ty}; {members}return sceOut, nil }}()"
        )
    }

    fn write_binding(&self) -> &'static str {
        GO_ELEMENT_BINDING
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        match self.carrier {
            // The one spelling of a payload field on the wire the inject seam
            // of a schema of entries writes too: a 64-bit integer as the text
            // of its digits.
            GoCarrier::Json => format!(
                "sce.ValueJSON({})",
                crate::forge::generator::go_payload_wire_value(ty, &held.text)
            ),
            // As a scalar param is (`StaticTarget::wire_value`): an integer of
            // at most 32 bits widened to `int64` and a `float32` to `float64`,
            // both exactly, and a 64-bit integer as the text of its digits.
            GoCarrier::Wire => match ty {
                SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32 => format!("int64({})", held.text),
                SceType::Uint64 => format!("sce.Uint64PayloadText({})", held.text),
                SceType::Int64 => format!("sce.Int64PayloadText({})", held.text),
                SceType::Float32 => format!("float64({})", held.text),
                SceType::Float64 | SceType::Bool => held.text.clone(),
                SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                    "a text, a byte string and an enum are cases of their own in the walk, \
                     never a scalar of a value type"
                ),
            },
        }
    }
    // The machine's enum's `String` answers the name its document declares.
    fn write_enum(&self, held: &Held) -> String {
        match self.carrier {
            GoCarrier::Json => format!("sce.ValueJSON({}.String())", held.text),
            GoCarrier::Wire => format!("{}.String()", held.text),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            GoCarrier::Json => format!("sce.ValueJSON({})", held.text),
            GoCarrier::Wire => held.text.clone(),
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            GoCarrier::Json => format!("sce.ValueJSON(sce.BytesAsPayloadText({}))", held.text),
            GoCarrier::Wire => format!("sce.BytesAsPayloadText({})", held.text),
        }
    }
    // The present value where there is one, and `null` where there is none,
    // as an absent scalar param is.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        let (result, absent) = match self.carrier {
            GoCarrier::Json => ("sce.RawJSON", "sce.ValueJSON(nil)"),
            GoCarrier::Wire => ("any", "nil"),
        };
        format!(
            "func() {result} {{ if {GO_ELEMENT_BINDING}, sceIs := ({}).Get(); sceIs {{ return \
             {elem} }}; return {absent} }}()",
            held.text
        )
    }
    // Each element written in the list's order.
    fn write_list(&self, held: &Held, elem: &str) -> String {
        // The literal's result, what each element is written as, and what the
        // elements are answered as.
        let (result, item, written) = match self.carrier {
            GoCarrier::Json => (
                "sce.RawJSON",
                "sce.RawJSON",
                "sce.ListJSON(sceItems, func(sceItem sce.RawJSON) sce.RawJSON { return sceItem })",
            ),
            GoCarrier::Wire => ("[]any", "any", "sceItems"),
        };
        format!(
            "func() {result} {{ sceItems := make([]{item}, 0, len({list})); for _, \
             {GO_ELEMENT_BINDING} := range {list} {{ sceItems = append(sceItems, {elem}) }}; \
             return {written} }}()",
            list = held.text
        )
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime writes the keys ascending by their bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), the JSON writer and
    // `ScriptValueToJSON` alike.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        match self.carrier {
            GoCarrier::Json => {
                let members: Vec<String> = fields
                    .iter()
                    .map(|(id, json)| {
                        format!(
                            "sce.Pair(\"{}\", {json})",
                            crate::filters::escape_go(id.clone())
                        )
                    })
                    .collect();
                format!("sce.RecordJSON({})", members.join(", "))
            }
            GoCarrier::Wire => {
                let members: Vec<String> = fields
                    .iter()
                    .map(|(id, value)| {
                        format!("\"{}\": {value}", crate::filters::escape_go(id.clone()))
                    })
                    .collect();
                format!("map[string]any{{{}}}", members.join(", "))
            }
        }
    }
    // The count held first, by the `sceFailure` a `<send>` declares for a param
    // that can fail and reads after it, then the list written.
    fn send_counted(&self, held: &Held, _field: &str, least: u32, sent: String) -> Option<String> {
        Some(format!(
            "func() any {{ if len({}) < {least} {{ sceFailure.Fail(scealgorithm.Precondition) }}; \
             return {sent} }}()",
            held.text
        ))
    }

    fn copy_binding(&self) -> &'static str {
        GO_ELEMENT_BINDING
    }
    // A number, a truth value, an enum, an optional one of them, a text and a
    // record are copied by value, and so is an element a copy walks: the
    // machine replaces a byte string, a record's field and an element and
    // never writes into one. The list a place holds is copied into a slice of
    // its own, since the variable it is copied into is appended to and
    // emptied in place: a place is copied whole into a list variable from a
    // list (`whole_list_copy`) and into a union variable from a union, which
    // the walk copies by [`Self::copy_union`], so a place that is not copied
    // as it is read here is a list.
    fn copy_whole(&self, held: &Held, plain: bool) -> String {
        if plain || held.by_reference {
            held.text.clone()
        } else {
            format!("append({0}[:0:0], {0}...)", held.text)
        }
    }
    // A text and a byte string are one Go type whatever their bound, so a walk
    // that copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_optional(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "func() optional.Of[{elem_ty}] {{ if {GO_ELEMENT_BINDING}, sceIs := ({}).Get(); sceIs \
             {{ return optional.Some[{elem_ty}]({elem}) }}; return optional.None[{elem_ty}]() }}()",
            held.text
        )
    }
    fn copy_present(&self, value: &str) -> String {
        format!("optional.Some({value})")
    }
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "func() []{elem_ty} {{ sceItems := make([]{elem_ty}, 0, len({list})); for _, \
             {GO_ELEMENT_BINDING} := range {list} {{ sceItems = append(sceItems, {elem}) }}; return \
             sceItems }}()",
            list = held.text
        )
    }

    // A union is a value struct, copied as a record is: the machine changes a
    // case by a whole assignment and writes a field of one through the place
    // a proven read answers, never into a list a case holds, so the copy shares
    // nothing the machine writes into.
    fn copy_union(&self, held: &Held) -> String {
        held.text.clone()
    }

    // The record's saved form, an object of its fields keyed by their ids in
    // the schema's order, each written and read as its type says
    // ([`go_saved_writer`], [`go_saved_reader`]): the names a record of entries'
    // saved form has (`ToSaved`, `<Type>FromSaved`), so the machine's saving
    // reads one spelling for either.
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
            let key = go_literal(&field.id);
            let refused =
                |why: String| format!("the field `{}` of `{}`: {why}", field.id, held.schema.name);
            // The fields of the record name its types in its own document's scope.
            let within = Some(held.alias);
            let write = go_saved_writer(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            let read = go_saved_reader(&field.sce_type, within, (records, enums), self, 0)
                .map_err(refused)?;
            writes.push_str(&format!("\tsaved.Set({key}, ({write})(r.{}))\n", field.id));
            reads.push_str(&format!(
                "\t{{\n\t\tsaved, err := sce.SavedField(value, what, {key})\n\t\tif err != nil \
                 {{\n\t\t\treturn record, err\n\t\t}}\n\t\tif record.{}, err = ({read})(saved, \
                 what + \".{}\"); err != nil {{\n\t\t\treturn record, err\n\t\t}}\n\t}}\n",
                field.id,
                crate::filters::escape_go(field.id.clone())
            ));
        }
        Ok(Some(format!(
            "// ToSaved is this value as a saved state writes it.\nfunc (r {ty}) ToSaved() any \
             {{\n\tsaved := sce.NewJSONObject()\n{writes}\treturn saved\n}}\n\n// {ty}FromSaved \
             is the value a saved state holds, refused unless it is one.\nfunc {ty}FromSaved(value \
             any, what string) ({ty}, error) {{\n\tvar record {ty}\n{reads}\treturn record, nil\n}}"
        )))
    }
    // The union's saved form is one object, its tag beside the fields of its
    // case (docs/adr/0092, section 6.1 item 7): written by the runtime's
    // `SavedWriteUnion` over the record's own `ToSaved`, and read by its tag
    // first, which names the record the rest of the object is read as.
    fn saved_union_ext(
        &self,
        def: &UnionDef,
        key: &str,
        records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        let tag = go_literal(&records.union_of_key(key)?.union.tag);
        let ty = &def.ty;
        let mut writes = String::new();
        let mut reads = String::new();
        for case in &def.cases {
            let variant = go_literal(&case.variant);
            let built = format!("{ty}{}", case.ident);
            match &case.record {
                Some(record) => {
                    writes.push_str(&format!(
                        "\tcase {}:\n\t\treturn sce.SavedWriteUnion({tag}, {variant}, \
                         u.{}.ToSaved())\n",
                        case.tag_ident,
                        go_case_member(&case.ident)
                    ));
                    reads.push_str(&format!(
                        "\tcase {variant}:\n\t\trecord, err := {record}FromSaved(value, what)\n\t\t\
                         if err != nil {{\n\t\t\treturn {ty}{{}}, err\n\t\t}}\n\t\treturn \
                         {built}(record), nil\n"
                    ));
                }
                None => {
                    writes.push_str(&format!(
                        "\tcase {}:\n\t\treturn sce.SavedWriteUnion({tag}, {variant}, nil)\n",
                        case.tag_ident
                    ));
                    reads.push_str(&format!("\tcase {variant}:\n\t\treturn {built}(), nil\n"));
                }
            }
        }
        let name = crate::filters::escape_go(def.name.clone());
        Ok(Some(format!(
            "// ToSaved is this value as a saved state writes it: its tag beside the fields of \
             its case.\nfunc (u {ty}) ToSaved() any {{\n\tswitch u.{GO_UNION_TAG} {{\n{writes}\t}}\n\t\
             panic(\"a value of {name} is in no case of its tag\")\n}}\n\n// {ty}FromSaved is the \
             value a saved state holds, read by its tag first, refused unless it is one of the \
             cases of {name}.\nfunc {ty}FromSaved(value any, what string) ({ty}, error) {{\n\t\
             tag, err := sce.SavedReadUnionTag(value, what, {tag})\n\tif err != nil {{\n\t\treturn \
             {ty}{{}}, err\n\t}}\n\tswitch tag {{\n{reads}\t}}\n\treturn {ty}{{}}, \
             sce.Refuse(fmt.Sprintf(\"'%s' (%s) is not a case of {name}\", what, tag))\n}}"
        )))
    }

    // The machine's own value struct, beside its records (docs/adr/0092,
    // section 7, the Go row): named by the union's document, so one union is
    // one type of the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    // The variant in Pascal case, which follows the union's type in the name of
    // the case's constructor (section 7 item 1: Go has no nesting).
    fn union_case_ident(&self, variant: &str) -> String {
        go_case_ident(variant)
    }
    // The tag is read by the one method every union of a Go unit and of a Go
    // machine answers it by, whatever key it is written under.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        Some(GO_UNION_KIND.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        go_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        go_shared_read(field)
    }
    // A struct of the tag and one member per case that holds a record, built
    // only by the constructor of a case. Every read the narrowing proves is a
    // method of it whose other cases panic, the one place they are written (as
    // a proven optional is read through the runtime's one `Proven`,
    // docs/adr/0055, section 5); beside each is the unexported accessor of the
    // pointer a write goes through ([`Self::union_member_place`]).
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_go(def.name.clone());
        let ty = &def.ty;
        let other = |read: &str| {
            format!(
                "panic(\"a value of {name} read as {} is in another case\")",
                crate::filters::escape_go(read.to_string())
            )
        };
        let mut members = format!("\t{GO_UNION_TAG} {}\n", def.tag_ty);
        let mut built = String::new();
        let mut reads = String::new();
        for case in &def.cases {
            let (ident, tag) = (&case.ident, &case.tag_ident);
            let member = go_case_member(ident);
            let variant = &case.variant;
            match &case.record {
                Some(record) => {
                    members.push_str(&format!("\t{member} {record}\n"));
                    built.push_str(&format!(
                        "\n\n// {ty}{ident} is the case `{variant}`, holding its record.\nfunc \
                         {ty}{ident}(r {record}) {ty} {{\n\treturn {ty}{{{GO_UNION_TAG}: {tag}, \
                         {member}: r}}\n}}"
                    ));
                    let (read, place) = (&case.proven, go_place_of(&case.proven));
                    reads.push_str(&format!(
                        "\n\n// {read} is the record of the case `{variant}`, read where a test of \
                         the tag\n// proved the value is in it.\nfunc (u {ty}) {read}() {record} \
                         {{\n\tif u.{GO_UNION_TAG} != {tag} {{\n\t\t{panic}\n\t}}\n\treturn \
                         u.{member}\n}}\n\n// {place} is the record of the case `{variant}`, \
                         written where a test of the\n// tag proved the value is in it.\nfunc (u \
                         *{ty}) {place}() *{record} {{\n\tif u.{GO_UNION_TAG} != {tag} {{\n\t\t\
                         {panic}\n\t}}\n\treturn &u.{member}\n}}",
                        panic = other(variant),
                    ));
                }
                None => built.push_str(&format!(
                    "\n\n// {ty}{ident} is the case `{variant}`, its tag alone.\nfunc {ty}{ident}() \
                     {ty} {{\n\treturn {ty}{{{GO_UNION_TAG}: {tag}}}\n}}"
                )),
            }
        }
        for field in &def.shared {
            let arms = |read: bool| -> String {
                field
                    .cases
                    .iter()
                    .filter_map(|ident| def.cases.iter().find(|case| case.ident == *ident))
                    .map(|case| {
                        format!(
                            "\tcase {}:\n\t\treturn {}u.{}.{}\n",
                            case.tag_ident,
                            if read { "" } else { "&" },
                            go_case_member(&case.ident),
                            field.field
                        )
                    })
                    .collect()
            };
            let (id, accessor, fty) = (&field.field, &field.accessor, &field.ty);
            let place = go_place_of(accessor);
            reads.push_str(&format!(
                "\n\n// {accessor} is the field `{id}` the cases that declare it share, read where a\n\
                 // test of the tag proved the value is in one of them.\nfunc (u {ty}) \
                 {accessor}() {fty} {{\n\tswitch u.{GO_UNION_TAG} {{\n{}\t}}\n\t{panic}\n}}\n\n// \
                 {place} is the field `{id}` the cases that declare it share, written where\n// a \
                 test of the tag proved the value is in one of them.\nfunc (u *{ty}) {place}() \
                 *{fty} {{\n\tswitch u.{GO_UNION_TAG} {{\n{}\t}}\n\t{panic}\n}}",
                arms(true),
                arms(false),
                panic = other(id),
            ));
        }
        format!(
            "// SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092): exactly one\n\
             // case, told apart by its tag, built only by the constructor of its case, and\n\
             // holding a member for the record of each case (section 7 item 4).\ntype {ty} struct \
             {{\n{members}}}\n\n// {GO_UNION_KIND} is the case this value is in, as its tag names \
             it.\nfunc (u {ty}) {GO_UNION_KIND}() {} {{\n\treturn u.{GO_UNION_TAG}\n}}{built}{reads}",
            def.tag_ty
        )
    }
    // The case's constructor, handed the record or nothing.
    fn union_value(&self, ty: &str, case: &str, record: Option<&str>) -> String {
        format!("{ty}{case}({})", record.unwrap_or_default())
    }
    // The tag first, by the enum's reader, in the runtime's `ReadUnion`, which
    // refuses a value that is no object before it; then the record of the case
    // the tag names, read from the same value by the case's arm of a `switch`
    // over the tag's enum (docs/adr/0092, section 4). The tag's reader answers
    // only a variant the enum declares, so the `switch` ends in no case left.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String {
        let ty = &read.ty;
        let arms: String = read
            .arms
            .iter()
            .map(|arm| {
                let built = format!("{ty}{}", arm.case_ident);
                match &arm.record {
                    Some(record) => format!(
                        "case {}: sceCase, err := {record}; if err != nil {{ return {ty}{{}}, err \
                         }}; return {built}(sceCase), nil; ",
                        arm.tag_ident
                    ),
                    None => format!("case {}: return {built}(), nil; ", arm.tag_ident),
                }
            })
            .collect();
        format!(
            "sce.ReadUnion({value}, {}, func({GO_READ_BINDING} sce.PayloadValue) ({tag_ty}, error) \
             {{ return {} }}, func(sceTag {tag_ty}, {GO_READ_BINDING} sce.PayloadValue) ({ty}, \
             error) {{ switch sceTag {{ {arms}}}; panic(\"the tag's reader answered no variant of \
             its enum\") }})",
            go_literal(read.tag),
            read.tag_reader,
            tag_ty = read.tag_ty,
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092, section
    // 2.1): the runtime's `UnionJSON`, which orders the keys as `RecordJSON`
    // does, or the `map[string]any` a `<send>` holds a record in with the tag
    // beside its fields, which `ScriptValueToJSON` orders. The case is told by
    // the declared name of the tag's variant, which the machine's enum answers
    // (`String`) and the tag is written as; its record is bound in its arm, as a
    // record's writer reads it.
    fn write_union(&self, held: &Held, _ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let tag = go_literal(tag);
        let result = match self.carrier {
            GoCarrier::Json => "sce.RawJSON",
            GoCarrier::Wire => "any",
        };
        let arms: String = arms
            .iter()
            .map(|arm| {
                let variant = go_literal(&arm.variant);
                let written = match (self.carrier, &arm.fields) {
                    (GoCarrier::Json, fields) => {
                        let pairs: String = fields
                            .iter()
                            .flatten()
                            .map(|(id, json)| format!(", sce.Pair({}, {json})", go_literal(id)))
                            .collect();
                        format!("sce.UnionJSON({tag}, {variant}{pairs})")
                    }
                    (GoCarrier::Wire, fields) => {
                        let members: String = fields
                            .iter()
                            .flatten()
                            .map(|(id, value)| format!(", {}: {value}", go_literal(id)))
                            .collect();
                        format!("map[string]any{{{tag}: {variant}{members}}}")
                    }
                };
                match &arm.fields {
                    Some(_) => format!(
                        "case {variant}: {GO_ELEMENT_BINDING} := {GO_UNION_BINDING}.{}(); return \
                         {written}; ",
                        go_case_read(&arm.variant)
                    ),
                    None => format!("case {variant}: return {written}; "),
                }
            })
            .collect();
        format!(
            "func() {result} {{ {GO_UNION_BINDING} := {}; switch \
             {GO_UNION_BINDING}.{GO_UNION_KIND}().String() {{ {arms}}}; panic(\"a union's tag \
             names no case of it\") }}()",
            held.text
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        go_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        go_place_of(read)
    }
    // A field is written in place, through the pointer the accessor beside its
    // proven read answers: the field of the one case left, or the field the
    // cases left share. The holder is a variable of the machine, which Go takes
    // the address of for the accessor's pointer receiver.
    fn union_member_place(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        if member == union.tag {
            return Err(format!(
                "the tag `{member}` of `union:{}` is never written alone: the case changes by a \
                 whole assignment (docs/adr/0092, section 6.1 item 6)",
                union.name
            ));
        }
        // The read stands exactly where the write does: one case left that
        // declares the field, or several that share it.
        go_union_member(union, left, member)?;
        Ok(match left {
            [case] => format!(
                "{holder}.{}().{member}",
                go_place_of(&go_case_read(&case.variant.name))
            ),
            _ => format!("*{holder}.{}()", go_place_of(&go_shared_read(member))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const GO: GoComposed<'static> = GoComposed {
        machine: "M",
        carrier: GoCarrier::Json,
    };

    /// The Go row of the one table of what each backend lowers of a composed
    /// value: a payload read at any depth, a list taken whole and a composed
    /// value sent (docs/adr/0086, sections 5 and 6), and a union held, read by
    /// its tag, lifted and saved (docs/adr/0092, section 7); no variable of a
    /// record that holds a composition yet.
    #[test]
    fn the_go_row_lowers_every_composed_position_but_a_composed_record_variable() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Go),
            ComposedLowering {
                name: "go",
                reads: true,
                copies: true,
                unions: true,
                records: false,
                sends: true,
            }
        );
    }

    /// The Go row of section 7.1: no type carries a bound, a value that may be
    /// absent is the forge runtime's `optional.Of`, a record is the machine's
    /// struct with a constructor a host builds one by, at any depth; no check
    /// is declared beside a record, since a Go struct has no constructor that
    /// could refuse a value.
    #[test]
    fn a_go_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &GO).expect("a type");
        assert_eq!(spelled("uint16"), "uint16");
        assert_eq!(spelled("string[8]"), "string");
        assert_eq!(spelled("bytes[4]"), "[]byte");
        assert_eq!(spelled("optional<int64>"), "optional.Of[int64]");
        assert_eq!(spelled("list<list<uint8>[2]>[3]"), "[][]uint8");
        assert_eq!(spelled("list<enum:Level>[2]"), "[]MLevelEnum");
        assert_eq!(spelled("list<record:Item>[3]"), "[]MItemRecord");
        assert_eq!(spelled("optional<list<uint8>[2]>"), "optional.Of[[]uint8]");
        assert_eq!(
            spelled("list<optional<uint16>>[2]"),
            "[]optional.Of[uint16]"
        );
        let members = record_members("Item", &held, &enums, &GO).expect("members");
        let checks = record_checks("Item", &held, &GO).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = GO.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.contains(
                "type MItemRecord struct {\n\tid uint32\n\tlevel MLevelEnum\n\tdue \
                 optional.Of[int64]\n\tblob []byte\n\ttags []string\n}\n"
            ),
            "{def}"
        );
        assert!(
            def.ends_with(
                "func NewMItemRecord(id uint32, level MLevelEnum, due optional.Of[int64], blob \
                 []byte, tags []string) MItemRecord {\n\treturn MItemRecord{id: id, level: \
                 level, due: due, blob: blob, tags: tags}\n}"
            ),
            "{def}"
        );
    }

    /// One call of the runtime per constructor, each element and present value
    /// read by a function literal that names the type the machine holds it in,
    /// a bound and a lower count handed to the reader that holds them, a record
    /// read field by field into its struct with its defaults.
    #[test]
    fn a_go_reader_is_one_runtime_call_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &GO, "v").expect("read");
        assert_eq!(read("uint8"), "sce.ValueUnsigned[uint8](v)");
        assert_eq!(read("int16"), "sce.ValueSigned[int16](v)");
        assert_eq!(read("uint64"), "sce.ValueUnsigned64(v)");
        assert_eq!(read("int64"), "sce.ValueSigned64(v)");
        assert_eq!(read("bool"), "v.Bool()");
        assert_eq!(read("float32"), "sce.ValueFloat[float32](v)");
        assert_eq!(read("string[8]"), "v.Text(8)");
        assert_eq!(read("bytes[4]"), "v.Bytes(4)");
        assert_eq!(
            read("optional<int64>"),
            "optional.Read(v, func(sceValue sce.PayloadValue) (int64, error) { return \
             sce.ValueSigned64(sceValue) })"
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "sce.ReadList(v, 3, func(sceValue sce.PayloadValue) ([]uint8, error) { return \
             sce.ReadList(sceValue, 2, func(sceValue sce.PayloadValue) (uint8, error) { return \
             sce.ValueUnsigned[uint8](sceValue) }) })"
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            "sce.ReadListWithin(v, 2, 3, func(sceValue sce.PayloadValue) ([]uint8, error) { \
             return sce.ReadListWithin(sceValue, 1, 2, func(sceValue sce.PayloadValue) (uint8, \
             error) { return sce.ValueUnsigned[uint8](sceValue) }) })"
        );
        assert_eq!(
            read("list<optional<uint16>>[2]"),
            "sce.ReadList(v, 2, func(sceValue sce.PayloadValue) (optional.Of[uint16], error) { \
             return optional.Read(sceValue, func(sceValue sce.PayloadValue) (uint16, error) { \
             return sce.ValueUnsigned[uint16](sceValue) }) })"
        );
        assert_eq!(
            read("enum:Level"),
            "func() (MLevelEnum, error) { sceName, err := v.Name(); if err != nil { return 0, \
             err }; switch sceName { case \"low\": return Low, nil; case \"high\": return High, \
             nil; }; return 0, v.UnknownVariant(sceName, \"Level\") }()"
        );
        let item = read("record:Item");
        assert!(
            item.starts_with(
                "func() (MItemRecord, error) { sceRecord, err := v.Record(\"\"); if err != nil { \
                 return MItemRecord{}, err }; var sceOut MItemRecord; { sceValue, err := \
                 sceRecord.Field(\"id\"); if err != nil { return MItemRecord{}, err }; if \
                 sceOut.id, err = sce.ValueUnsigned[uint32](sceValue); err != nil { return \
                 MItemRecord{}, err } }; "
            ),
            "{item}"
        );
        assert!(
            item.ends_with(
                "if sceOut.tags, err = sce.ReadList(sceValue, 4, func(sceValue sce.PayloadValue) \
                 (string, error) { return sceValue.Text(8) }); err != nil { return MItemRecord{}, \
                 err } }; return sceOut, nil }()"
            ),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.starts_with(
                "func() (MBatchRecord, error) { sceRecord, err := v.Record(\"{\\\"late\\\":[]}\"); "
            ),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor: a list by a `range` loop over it, a value that may be
    /// absent by its `Get` and `null` where it is absent, a record by its
    /// field ids, which the runtime orders.
    #[test]
    fn a_go_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write = |text: &str, at: Held| writer(&ty(text), None, &held, &GO, &at).expect("write");
        assert_eq!(write("uint8", Held::place("x.n")), "sce.ValueJSON(x.n)");
        assert_eq!(
            write("int64", Held::place("x.n")),
            "sce.ValueJSON(sce.Int64PayloadText(x.n))"
        );
        assert_eq!(
            write("bytes[4]", Held::place("x.b")),
            "sce.ValueJSON(sce.BytesAsPayloadText(x.b))"
        );
        assert_eq!(
            write("enum:Level", Held::reference("sceElement")),
            "sce.ValueJSON(sceElement.String())"
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            "func() sce.RawJSON { sceItems := make([]sce.RawJSON, 0, len(x.maybe)); for _, \
             sceElement := range x.maybe { sceItems = append(sceItems, func() sce.RawJSON { if \
             sceElement, sceIs := (sceElement).Get(); sceIs { return sce.ValueJSON(sceElement) }; \
             return sce.ValueJSON(nil) }()) }; return sce.ListJSON(sceItems, func(sceItem \
             sce.RawJSON) sce.RawJSON { return sceItem }) }()"
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "for _, sceElement := range x.items { sceItems = append(sceItems, sce.RecordJSON(",
            "sce.Pair(\"id\", sce.ValueJSON(sceElement.id)), ",
            "sce.Pair(\"level\", sce.ValueJSON(sceElement.level.String())), ",
            "sce.Pair(\"due\", func() sce.RawJSON { if sceElement, sceIs := \
             (sceElement.due).Get(); sceIs { return sce.ValueJSON(sce.Int64PayloadText(sceElement)) \
             }; return sce.ValueJSON(nil) }()), ",
            "sce.Pair(\"blob\", sce.ValueJSON(sce.BytesAsPayloadText(sceElement.blob))), ",
            "sce.Pair(\"tags\", func() sce.RawJSON { sceItems := make([]sce.RawJSON, 0, \
             len(sceElement.tags)); for _, sceElement := range sceElement.tags { sceItems = \
             append(sceItems, sce.ValueJSON(sceElement)) }; ",
        ] {
            assert!(items.contains(part), "expected `{part}` in:\n{items}");
        }
    }

    /// The value a Go `<send>` holds a composed param in: an `[]any`, a
    /// `map[string]any` whose keys the runtime orders, `nil` where a value is
    /// absent, and each scalar as a scalar param is (a narrow integer widened
    /// to `int64`, a 64-bit one as its digits, a byte string as its Latin-1
    /// text).
    #[test]
    fn a_go_send_writes_each_constructor_into_the_value_it_holds_params_in() {
        let held = held();
        let wire = GoComposed {
            machine: "M",
            carrier: GoCarrier::Wire,
        };
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &wire, &at).expect("write");
        assert_eq!(write("uint8", Held::place("x.n")), "int64(x.n)");
        assert_eq!(
            write("int64", Held::place("x.n")),
            "sce.Int64PayloadText(x.n)"
        );
        assert_eq!(
            write("uint64", Held::place("x.n")),
            "sce.Uint64PayloadText(x.n)"
        );
        assert_eq!(write("float32", Held::place("x.n")), "float64(x.n)");
        assert_eq!(write("float64", Held::place("x.n")), "x.n");
        let inner = "func() []any { sceItems := make([]any, 0, len(sceElement)); for _, sceElement \
                     := range sceElement { sceItems = append(sceItems, int64(sceElement)) }; return \
                     sceItems }()";
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            format!(
                "func() []any {{ sceItems := make([]any, 0, len(x.grid)); for _, sceElement := \
                 range x.grid {{ sceItems = append(sceItems, {inner}) }}; return sceItems }}()"
            )
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            format!(
                "func() any {{ if sceElement, sceIs := (x.spare).Get(); sceIs {{ return {inner} \
                 }}; return nil }}()"
            )
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        assert!(
            items.contains(
                "sceItems = append(sceItems, map[string]any{\"id\": int64(sceElement.id), \
                 \"level\": sceElement.level.String(), \"due\": func() any { if sceElement, sceIs \
                 := (sceElement.due).Get(); sceIs { return sce.Int64PayloadText(sceElement) }; \
                 return nil }(), \"blob\": sce.BytesAsPayloadText(sceElement.blob), \"tags\": \
                 func() []any { sceItems := make([]any, 0, len(sceElement.tags)); for _, \
                 sceElement := range sceElement.tags { sceItems = append(sceItems, sceElement) }; \
                 return sceItems }()})"
            ),
            "{items}"
        );
    }

    /// Two places whose types differ only in their bounds are one Go type, so
    /// the list a place holds is copied into a slice of its own, which the
    /// variable it is copied into appends to and empties in place; where the
    /// types differ, each part is copied into the place's type by a function
    /// literal that names it.
    #[test]
    fn a_go_copy_of_a_list_is_a_slice_of_its_own() {
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
                &GO,
                &Held::place("x"),
            )
        };
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]"),
            Ok("append(x[:0:0], x...)".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]"),
            Ok("append(x[:0:0], x...)".to_string())
        );
        let widened = "func() []optional.Of[uint8] { sceItems := make([]optional.Of[uint8], 0, \
                       len(sceElement)); for _, sceElement := range sceElement { sceItems = \
                       append(sceItems, optional.Some(sceElement)) }; return sceItems }()";
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok(widened
                .replace("len(sceElement)", "len(x)")
                .replace("range sceElement", "range x"))
        );
        assert_eq!(
            copy(
                "optional<list<uint8>[2]>",
                "optional<list<optional<uint8>>[2]>"
            ),
            Ok(format!(
                "func() optional.Of[[]optional.Of[uint8]] {{ if sceElement, sceIs := (x).Get(); \
                 sceIs {{ return optional.Some[[]optional.Of[uint8]]({widened}) }}; return \
                 optional.None[[]optional.Of[uint8]]() }}()"
            ))
        );
    }

    /// A Go raise seam holds a host's value to every bound its type declares
    /// (docs/adr/0086, section 5.4): one check per constructor that holds a
    /// bound, an element's or a present value's inside the function literal of
    /// its holder's, which names the type the machine holds it in, each
    /// binding named by its depth; a lower count is held too; a record a list
    /// holds is checked field by field at the path below the element; a number,
    /// an enum and an optional one of them are not checked.
    #[test]
    fn a_go_raise_seam_checks_each_bound_before_it_encodes() {
        let held = held();
        let enums = enums();
        let check = |text: &str, id: &str| {
            go_bound_check(
                &ty(text),
                None,
                &format!("\"{id}\""),
                id,
                0,
                (&held, &enums),
                &GO,
            )
            .expect("a check")
        };
        assert_eq!(check("uint8", "n"), None);
        assert_eq!(check("enum:Level", "level"), None);
        assert_eq!(check("optional<int64>", "due"), None);
        assert_eq!(
            check("list<list<uint8>[2]>[2..3]", "grid"),
            Some(
                "sce.CheckListWithin(\"grid\", grid, 2, 3, func(scePath0 string, sceValue0 \
                 []uint8) error { return sce.CheckList(scePath0, sceValue0, 2, func(string, \
                 uint8) error { return nil }) })"
                    .to_string()
            )
        );
        assert_eq!(
            check("optional<list<string[4]>[2]>", "spare"),
            Some(
                "optional.Check(spare, func(sceValue0 []string) error { return \
                 sce.CheckList(\"spare\", sceValue0, 2, func(scePath1 string, sceValue1 string) \
                 error { return sce.CheckText(scePath1, sceValue1, 4) }) })"
                    .to_string()
            )
        );
        let batch = held.record(None, "Batch").expect("Batch");
        assert_eq!(
            go_raise_checks(batch, &held, &enums, &GO).expect("checks"),
            vec![
                "sce.CheckList(\"items\", items, 3, func(scePath0 string, sceValue0 MItemRecord) \
                 error { return func() error { if err := sce.CheckBytes(scePath0 + \"/blob\", \
                 sceValue0.blob, 4); err != nil { return err }; if err := \
                 sce.CheckList(scePath0 + \"/tags\", sceValue0.tags, 4, func(scePath1 string, \
                 sceValue1 string) error { return sce.CheckText(scePath1, sceValue1, 8) }); err != \
                 nil { return err }; return nil }() })"
                    .to_string(),
                "sce.CheckList(\"grid\", grid, 2, func(scePath0 string, sceValue0 []uint8) error { \
                 return sce.CheckList(scePath0, sceValue0, 2, func(string, uint8) error { return \
                 nil }) })"
                    .to_string(),
                "sce.CheckList(\"late\", late, 2, func(string, uint8) error { return nil })"
                    .to_string(),
            ]
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

    /// A union is the machine's own value struct at any depth, named by its
    /// document (docs/adr/0092, section 7, the Go row), and its tag is read by
    /// `Kind` whatever key it is written under.
    #[test]
    fn a_go_machine_holds_a_union_in_its_own_value_struct() {
        let shapes = shapes();
        let kinds = kinds();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &GO),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &GO
            ),
            Ok("[]optional.Of[MShapeUnion]".to_string())
        );
        assert_eq!(GO.union_tag_read("type"), Some("Kind".to_string()));
        assert_eq!(
            GO.union_value("MShapeUnion", "Dot", None),
            "MShapeUnionDot()"
        );
        assert_eq!(
            GO.union_value("MShapeUnion", "Circle", Some("r")),
            "MShapeUnionCircle(r)"
        );
    }

    /// The declaration: an unexported tag and one unexported member per case
    /// that holds a record, a constructor per case, `Kind`, a proven read per
    /// case that holds a record whose other cases panic, beside the
    /// unexported accessor of the pointer a write goes through, and one
    /// accessor for `radius`, which `circle` and `disc` share.
    #[test]
    fn a_go_union_is_declared_with_its_constructors_and_its_proven_reads() {
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &GO).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "Kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "ProvenFieldRadius".to_string(),
                ty: "uint16".to_string(),
                cases: vec!["Circle".to_string(), "Disc".to_string()],
            }]
        );
        let text = GO.union_def(&def);
        let panic =
            |read: &str| format!("panic(\"a value of shape read as {read} is in another case\")");
        for fragment in [
            "type MShapeUnion struct {\n\ttag MKindEnum\n\tcaseCircle MRadiusRecord\n\tcaseDisc \
             MRadiusRecord\n}\n"
                .to_string(),
            "func (u MShapeUnion) Kind() MKindEnum {\n\treturn u.tag\n}".to_string(),
            "func MShapeUnionDot() MShapeUnion {\n\treturn MShapeUnion{tag: Dot}\n}".to_string(),
            "func MShapeUnionCircle(r MRadiusRecord) MShapeUnion {\n\treturn MShapeUnion{tag: \
             Circle, caseCircle: r}\n}"
                .to_string(),
            format!(
                "func (u MShapeUnion) ProvenCircle() MRadiusRecord {{\n\tif u.tag != Circle {{\n\t\t\
                 {}\n\t}}\n\treturn u.caseCircle\n}}",
                panic("circle")
            ),
            format!(
                "func (u *MShapeUnion) provenCirclePlace() *MRadiusRecord {{\n\tif u.tag != Circle \
                 {{\n\t\t{}\n\t}}\n\treturn &u.caseCircle\n}}",
                panic("circle")
            ),
            format!(
                "func (u MShapeUnion) ProvenFieldRadius() uint16 {{\n\tswitch u.tag {{\n\tcase \
                 Circle:\n\t\treturn u.caseCircle.radius\n\tcase Disc:\n\t\treturn \
                 u.caseDisc.radius\n\t}}\n\t{}\n}}",
                panic("radius")
            ),
            format!(
                "func (u *MShapeUnion) provenFieldRadiusPlace() *uint16 {{\n\tswitch u.tag {{\n\t\
                 case Circle:\n\t\treturn &u.caseCircle.radius\n\tcase Disc:\n\t\treturn \
                 &u.caseDisc.radius\n\t}}\n\t{}\n}}",
                panic("radius")
            ),
        ] {
            assert!(text.contains(&fragment), "expected `{fragment}` in:\n{text}");
        }
        // An empty case has no member and nothing to read.
        assert!(!text.contains("caseDot"), "{text}");
        assert!(!text.contains("ProvenDot"), "{text}");
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// method, a field of the one case left through that case's proven read,
    /// and a field two cases share through its accessor; a write goes through
    /// the pointer the unexported accessor beside each answers, and the tag is
    /// never written alone.
    #[test]
    fn a_go_proven_read_and_write_are_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let both = [case(&shapes, "circle"), case(&shapes, "disc")];
        assert_eq!(go_union_member(union, &all, "kind"), Ok("Kind()".into()));
        assert_eq!(
            go_union_member(union, &[case(&shapes, "disc")], "radius"),
            Ok("ProvenDisc().radius".into())
        );
        assert_eq!(
            GO.union_member(union, &both, "radius"),
            Ok("ProvenFieldRadius()".into())
        );
        assert!(go_union_member(union, &[case(&shapes, "dot")], "radius").is_err());
        assert_eq!(
            GO.union_member_place("p.vS", union, &[case(&shapes, "disc")], "radius"),
            Ok("p.vS.provenDiscPlace().radius".into())
        );
        assert_eq!(
            GO.union_member_place("p.vS", union, &both, "radius"),
            Ok("*p.vS.provenFieldRadiusPlace()".into())
        );
        let tag = GO
            .union_member_place("p.vS", union, &all, "kind")
            .expect_err("the tag is never written alone");
        assert!(tag.contains("never written alone"), "{tag}");
    }

    /// The reader reads the tag by the enum's reader inside the runtime's
    /// `ReadUnion`, and then the record of the case it names from the same
    /// value, each case an arm of a `switch` over the tag's enum; the writer
    /// tells the case by its tag's declared name and writes the tag among the
    /// case's fields, as the JSON text and as the value a `<send>` holds; a
    /// union is copied by value, never as a list is.
    #[test]
    fn a_go_union_is_read_tag_first_written_by_its_case_and_copied_by_value() {
        let shapes = shapes();
        let kinds = kinds();
        let read = reader(&ty("union:Shape"), None, &shapes, &kinds, &GO, "v").expect("read");
        for fragment in [
            "sce.ReadUnion(v, \"kind\", func(sceValue sce.PayloadValue) (MKindEnum, error) { \
             return func() (MKindEnum, error) { sceName, err := sceValue.Name();",
            "func(sceTag MKindEnum, sceValue sce.PayloadValue) (MShapeUnion, error) { switch \
             sceTag { case Dot: return MShapeUnionDot(), nil; case Circle: sceCase, err := ",
            "return MShapeUnionCircle(sceCase), nil; case Disc: ",
            "}; panic(\"the tag's reader answered no variant of its enum\") })",
        ] {
            assert!(read.contains(fragment), "expected `{fragment}` in:\n{read}");
        }
        let json =
            writer(&ty("union:Shape"), None, &shapes, &GO, &Held::place("x.s")).expect("write");
        assert_eq!(
            json,
            "func() sce.RawJSON { sceUnion := x.s; switch sceUnion.Kind().String() { case \"dot\": \
             return sce.UnionJSON(\"kind\", \"dot\"); case \"circle\": sceElement := \
             sceUnion.ProvenCircle(); return sce.UnionJSON(\"kind\", \"circle\", \
             sce.Pair(\"radius\", sce.ValueJSON(sceElement.radius))); case \"disc\": sceElement := \
             sceUnion.ProvenDisc(); return sce.UnionJSON(\"kind\", \"disc\", sce.Pair(\"radius\", \
             sce.ValueJSON(sceElement.radius))); }; panic(\"a union's tag names no case of it\") }()"
        );
        let wire = GoComposed {
            machine: "M",
            carrier: GoCarrier::Wire,
        };
        let sent = writer(
            &ty("union:Shape"),
            None,
            &shapes,
            &wire,
            &Held::place("x.s"),
        )
        .expect("write");
        assert!(
            sent.contains(
                "case \"circle\": sceElement := sceUnion.ProvenCircle(); return \
                 map[string]any{\"kind\": \"circle\", \"radius\": int64(sceElement.radius)}; "
            ),
            "{sent}"
        );
        let copy = |from: &str, to: &str| {
            copier(
                &ty(from),
                None,
                &ty(to),
                None,
                &shapes,
                &kinds,
                &GO,
                &Held::place("x"),
            )
        };
        assert_eq!(copy("union:Shape", "union:Shape"), Ok("x".to_string()));
        assert_eq!(
            copy("optional<union:Shape>", "optional<union:Shape>"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("union:Shape", "optional<union:Shape>"),
            Ok("optional.Some(x)".to_string())
        );
    }

    /// A union is saved as its tag beside the fields of its case, written by
    /// the runtime's `SavedWriteUnion` over the record's own `ToSaved`, and read
    /// back by its tag first (docs/adr/0092, section 6.1 item 7); a tag no case
    /// names is refused naming the union.
    #[test]
    fn a_go_union_is_saved_by_its_tag_beside_its_case() {
        let shapes = shapes();
        let def = union_definition(SHAPE_KEY, &shapes, &kinds(), &GO).expect("declared");
        let text = GO
            .saved_union_ext(&def, SHAPE_KEY, &shapes)
            .expect("spelled")
            .expect("a Go union carries its saved form");
        for fragment in [
            "func (u MShapeUnion) ToSaved() any {\n\tswitch u.tag {\n\tcase Dot:\n\t\treturn \
             sce.SavedWriteUnion(\"kind\", \"dot\", nil)\n\tcase Circle:\n\t\treturn \
             sce.SavedWriteUnion(\"kind\", \"circle\", u.caseCircle.ToSaved())\n",
            "func MShapeUnionFromSaved(value any, what string) (MShapeUnion, error) {\n\ttag, err \
             := sce.SavedReadUnionTag(value, what, \"kind\")\n",
            "\tcase \"dot\":\n\t\treturn MShapeUnionDot(), nil\n",
            "\tcase \"circle\":\n\t\trecord, err := MRadiusRecordFromSaved(value, what)\n\t\tif err \
             != nil {\n\t\t\treturn MShapeUnion{}, err\n\t\t}\n\t\treturn \
             MShapeUnionCircle(record), nil\n",
            "sce.Refuse(fmt.Sprintf(\"'%s' (%s) is not a case of shape\", what, tag))",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }

    /// A composed record's saved form: each field written by the runtime's
    /// writer of its type and read back held to every bound and count its type
    /// states, a list's lower count included (docs/adr/0086, section 15.6).
    #[test]
    fn a_go_composed_record_is_saved_field_by_field_by_its_types() {
        let held = held();
        let enums = enums();
        let text = GO
            .saved_record_ext("Item", &held, &enums)
            .expect("spelled")
            .expect("a Go composed record carries its saved form");
        for fragment in [
            "func (r MItemRecord) ToSaved() any {\n\tsaved := sce.NewJSONObject()\n",
            "\tsaved.Set(\"id\", (sce.SavedWriteUint32)(r.id))\n",
            "\tsaved.Set(\"level\", (MLevelEnum.ToSaved)(r.level))\n",
            "\tsaved.Set(\"due\", (func(sceSaved0 optional.Of[int64]) any { return \
             optional.Wire(sceSaved0, sce.SavedWriteInt64) })(r.due))\n",
            "\tsaved.Set(\"tags\", (func(sceSaved0 []string) any { return \
             sce.SavedWriteList(sceSaved0, sce.SavedWriteString) })(r.tags))\n",
            "func MItemRecordFromSaved(value any, what string) (MItemRecord, error) {\n\tvar record \
             MItemRecord\n",
            "\t\tif record.blob, err = (sce.SavedBoundedBytes(4))(saved, what + \".blob\"); err != \
             nil {\n",
            "\t\tif record.tags, err = (func(sceSaved0 any, sceWhat0 string) ([]string, error) { \
             return sce.SavedReadListWithin(sceSaved0, sceWhat0, 0, 4, \
             sce.SavedBoundedString(8)) })(saved, what + \".tags\"); err != nil {\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
    }

    /// A union a raise seam is handed is checked by the record of the case it
    /// is in, at the union's own path; one whose cases hold no bound checks
    /// nothing.
    #[test]
    fn a_go_raise_seam_checks_a_union_by_its_case() {
        let shapes = shapes();
        let kinds = kinds();
        assert_eq!(
            go_bound_check(
                &ty("union:Shape"),
                None,
                "\"one\"",
                "one",
                0,
                (&shapes, &kinds),
                &GO
            ),
            Ok(None)
        );
        let none = BTreeMap::new();
        assert!(go_bound_check(
            &ty("union:Shape"),
            None,
            "\"one\"",
            "one",
            0,
            (&shapes, &none),
            &GO
        )
        .is_err());
    }

    /// A list sent where its field asks a lower count it does not prove is
    /// held to the count before it is written (docs/adr/0086, section 15.6).
    #[test]
    fn a_list_sent_short_of_its_fields_count_fails_before_it_is_written() {
        assert_eq!(
            GO.send_counted(&Held::place("p.Taking"), "ops", 1, "W".to_string()),
            Some(
                "func() any { if len(p.Taking) < 1 { sceFailure.Fail(scealgorithm.Precondition) }; \
                 return W }()"
                    .to_string()
            )
        );
    }
}
