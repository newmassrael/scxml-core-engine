// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Go spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Go row), and the checks a Go raise seam
//! runs over a host's value before it encodes it (section 5.4).

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

/// Why a Go walk meets no union: the one decision of what is lowered refuses a
/// union on Go before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const GO_HOLDS_NO_UNION: &str = "a Go machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on Go before any walk reaches one";

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
        ValueType::Union { .. } => return Err(GO_HOLDS_NO_UNION.to_string()),
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

    fn copy_binding(&self) -> &'static str {
        GO_ELEMENT_BINDING
    }
    // A number, a truth value, an enum, an optional one of them, a text and a
    // record are copied by value, and so is an element a copy walks: the
    // machine replaces a byte string, a record's field and an element and
    // never writes into one. The list a place holds is copied into a slice of
    // its own, since the variable it is copied into is appended to and
    // emptied in place: on a machine that holds no union, a place is copied
    // whole only into a list variable from a list (`whole_list_copy`), so a
    // place that is not copied as it is read is a list.
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

    // A Go machine holds no union yet: the gate refuses one by name before a
    // walk reaches it, so each spelling below either names what it would be or
    // says why there is none.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        crate::filters::to_pascal_case(variant.to_string())
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        format!(
            "Proven{}",
            crate::filters::to_pascal_case(variant.to_string())
        )
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "ProvenField{}",
            crate::filters::to_pascal_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{GO_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{GO_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{GO_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{GO_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(GO_HOLDS_NO_UNION.to_string())
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
        Err(GO_HOLDS_NO_UNION.to_string())
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

    /// A Go walk names no union: the one decision of what is lowered refuses
    /// one on Go first, and the walk that would declare or check one says why
    /// there is none.
    #[test]
    fn a_go_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &GO).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(GO
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
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
        .unwrap_err()
        .contains("holds no union"));
    }
}
