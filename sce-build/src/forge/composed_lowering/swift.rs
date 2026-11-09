// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Swift spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Swift row), and the checks a Swift raise
//! seam runs over a host's value before it encodes it (section 5.4).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|
//! | `[T]` | `String` / `[UInt8]` | `T?` | the machine's struct |
//!
//! No Swift type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is: an `Array`, an `Optional` and a struct are values, so
//! the assignment copies them at every depth and the place copied into shares
//! nothing with the place copied from.

use super::*;

/// The Swift spellings of the walks, for the machine `machine` (section 7.1,
/// the Swift row): the runtime's `PayloadValue` (`SceRuntime/EventPayload.swift`)
/// reads each constructor, and its typed writers (`listJSON`, `recordJSON`,
/// `optionalJSON`, `WireValue.json`) write each as its JSON, which `carrier`
/// says where it goes.
///
/// Every reader is a closure literal of the value it reads that names the
/// type it answers (`CLOSURE_HEAD`), handed to the runtime's combinator of
/// the part that holds it (`list`, `optional`, `record`) and called where a
/// record reads a field, so the type checker is told each part's type rather
/// than asked to infer it through the nesting. A writer and a copy walk a list
/// and a value that may be absent by a closure the runtime's writer or the
/// standard library's `map` calls, its result type named. A parameter of a
/// nested closure hides its holder's of the same name: the holder's value was
/// handed in before the inner one is bound, and each part reads its own.
pub struct SwiftComposed<'a> {
    pub machine: &'a str,
    pub carrier: SwiftCarrier,
}

/// Where a Swift writer's JSON goes: the writers spell every composed value as
/// its JSON text, one walk of its type, and the carrier says what holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwiftCarrier {
    /// The JSON text itself: what the inject seam writes as the event's `data`
    /// beside the typed carrier (section 3.1).
    Json,
    /// The `WireValue` a `<send>` holds each param in
    /// (`StaticTarget::wire_value`) until the runtime writes them as one JSON
    /// object: a composed value is `WireValue.composed` of the JSON the writers
    /// wrote, whose keys `recordJSON` orders by their bytes at every depth,
    /// carried as it is (section 5.3; ARCHITECTURE.md, "JSON Object Key
    /// Order").
    Wire,
}

/// The name a Swift reader binds the value it reads to: the parameter of
/// every reader's closure.
const SWIFT_READ_BINDING: &str = "sceValue";

/// The name a Swift record reader binds the fields of the record to.
const SWIFT_FIELDS_BINDING: &str = "sceFields";

/// The name a Swift writer binds an element or a present value to.
const SWIFT_ELEMENT_BINDING: &str = "sceElement";

/// The name a Swift copy binds an element or a present value to.
const SWIFT_COPY_BINDING: &str = "sceCopied";

/// The computed property of a union's Swift enum that answers the value of the
/// tag's enum its case is for (docs/adr/0092, section 7, the Swift row).
pub(crate) const SWIFT_UNION_KIND: &str = "kind";

/// The identifier of the case `variant` of a union's Swift enum: the variant as
/// the Swift declaration of the tag's enum, the document `tag_document`, spells
/// it ([`crate::forge::enum_naming::variant_ident`]), so the case and the value
/// of the tag it is for are one name.
pub(crate) fn swift_case_ident(tag_document: &str, variant: &str) -> String {
    crate::forge::enum_naming::variant_ident(
        crate::generator::Language::Swift,
        tag_document,
        variant,
    )
}

/// The read of the record of the case `variant` where a test proved it, on a
/// union's Swift enum (the unit's and the machine's alike).
pub(crate) fn swift_case_read(variant: &str) -> String {
    format!(
        "proven{}",
        crate::filters::to_pascal_case(variant.to_string())
    )
}

/// Why a Swift walk meets no union: the one decision of what is lowered
/// refuses a union on Swift before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const SWIFT_HOLDS_NO_UNION: &str =
    "a Swift machine holds no union yet (docs/adr/0092, section 7): the one decision of what \
     is lowered refuses a union on Swift before any walk reaches one";

/// The head of a Swift reader that answers a value of `ty`: a closure of the
/// value it reads, refusing a value outside its type with the runtime's
/// `PayloadRefusal`.
fn closure_head(ty: &str) -> String {
    format!("{{ ({SWIFT_READ_BINDING}: PayloadValue) throws -> {ty} in")
}

/// A Swift string literal of `text`.
fn literal(text: &str) -> String {
    format!("\"{}\"", crate::filters::escape_swift(text.to_string()))
}

/// The check of a Swift value of `ty`, written in the scope `within` names, at
/// `value`, named `path` (an expression of a `String`): the statements that
/// throw the runtime's `BoundExceeded`, naming the path to the place, for a
/// value past a bound its type declares at any depth (docs/adr/0086, section
/// 5.4). One call of the runtime's `checkText`, `checkBytes`, `checkList` or
/// `checkOptional` per constructor that holds a bound, the check of an element
/// or a present value inside the closure the call hands it to, its parameters
/// named by their `depth`; a record is checked field by field where it stands,
/// at the path of each field below its own, the statements apart by `; `.
/// `None` for a type that holds no bound at any depth: a number, an enum, an
/// optional one of them and a record of those.
///
/// A Swift struct has no constructor that could refuse a value, so a Swift
/// raise seam runs this check over the payload a host hands it before it
/// encodes it, a record a list holds included.
fn swift_bound_check(
    ty: &ValueType,
    within: Option<&str>,
    path: &str,
    value: &str,
    depth: usize,
    records: &dyn RecordEnv,
) -> Result<Option<String>, String> {
    let (inner_path, inner_value) = (format!("scePath{depth}"), format!("sceValue{depth}"));
    // The closure the runtime hands a part to, at the part's own path.
    let part = |inner: &str| format!("{{ {inner_path}, {inner_value} in {inner} }}");
    Ok(match ty {
        ValueType::Text { max } => Some(format!(
            "try checkText({path}, {value}, bound: {})",
            max.max
        )),
        ValueType::Bytes { max } => Some(format!(
            "try checkBytes({path}, {value}, bound: {})",
            max.max
        )),
        // The count is held whatever the elements are; an element with
        // nothing of its own to check is handed to a closure that checks
        // nothing.
        ValueType::List { elem, max } => {
            let each = match swift_bound_check(
                elem,
                within,
                &inner_path,
                &inner_value,
                depth + 1,
                records,
            )? {
                Some(inner) => part(&inner),
                None => "{ _, _ in }".to_string(),
            };
            let least = if max.min == 0 {
                String::new()
            } else {
                format!("least: {}, ", max.min)
            };
            Some(format!(
                "try checkList({path}, {value}, {least}bound: {}, {each})",
                max.max
            ))
        }
        // An absent value holds nothing to check; a present one is checked at
        // the place's own path, which the runtime hands the closure.
        ValueType::Optional { elem } => {
            let Some(inner) =
                swift_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?
            else {
                return Ok(None);
            };
            Some(format!(
                "try checkOptional({path}, {value}, {})",
                part(&inner)
            ))
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let mut checks = Vec::new();
            for field in &held.schema.fields {
                let below = format!(
                    "{path} + \"/{}\"",
                    crate::filters::escape_swift(field.id.clone())
                );
                if let Some(check) = swift_bound_check(
                    &field.sce_type,
                    Some(held.alias),
                    &below,
                    &format!("{value}.{}", field.id),
                    depth,
                    records,
                )? {
                    checks.push(check);
                }
            }
            (!checks.is_empty()).then(|| checks.join("; "))
        }
        ValueType::Scalar(_) => None,
        ValueType::Union { .. } => return Err(SWIFT_HOLDS_NO_UNION.to_string()),
    })
}

/// The checks a Swift raise seam runs over the fields of the payload a host
/// hands it, each a parameter of the seam named by the field's id, before it
/// encodes any of it ([`swift_bound_check`]): the statements of each field
/// whose type holds a bound at any depth, in the schema's order, the path of
/// each the field's id.
pub(crate) fn swift_raise_checks(
    held: HeldRecord<'_>,
    records: &dyn RecordEnv,
) -> Result<Vec<String>, String> {
    let mut checks = Vec::new();
    for field in &held.schema.fields {
        if let Some(check) = swift_bound_check(
            &field.sce_type,
            Some(held.alias),
            &literal(&field.id),
            &field.id,
            0,
            records,
        )? {
            checks.push(check);
        }
    }
    Ok(checks)
}

impl SwiftComposed<'_> {
    /// The value a member of the type `ty` holds before anything is written
    /// to it, `ty` one of the types this spelling holds a value in
    /// ([`machine_type`]): `nil` for a value that may be absent (a type that
    /// ends in `?`, which is its outermost constructor), the empty list for a
    /// list or a byte string (one that opens with `[`), the scalar's own zero
    /// ([`crate::forge::generator::swift_zero_value`]), a record of the
    /// machine at its zero (`<Machine><Alias>Record()`, which is
    /// `SceDefault`), and the first case of the machine's enum, the one other
    /// type a value is held in. What a record's member and a payload's member
    /// start at, and what a checked read of an element that is not there
    /// answers for a record that holds one (`Checked.atDefault`).
    pub(crate) fn zero_of(&self, ty: &str) -> String {
        if ty.ends_with('?') {
            return "nil".to_string();
        }
        if ty.starts_with('[') {
            return "[]".to_string();
        }
        let scalars = [
            SceType::Uint8,
            SceType::Uint16,
            SceType::Uint32,
            SceType::Uint64,
            SceType::Int8,
            SceType::Int16,
            SceType::Int32,
            SceType::Int64,
            SceType::Float32,
            SceType::Float64,
            SceType::Bool,
            SceType::String,
        ];
        if let Some(scalar) = scalars
            .iter()
            .find(|scalar| crate::forge::generator::swift_type(scalar) == ty)
        {
            return crate::forge::generator::swift_zero_value(scalar).to_string();
        }
        if ty.starts_with(self.machine) && ty.ends_with("Record") {
            return format!("{ty}()");
        }
        format!("{ty}.allCases[0]")
    }
}

impl ComposedSpelling for SwiftComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::swift_type(ty).to_string()
    }
    // The Swift row of section 7.1: no type has a bound as its parameter (section
    // 7.4, last paragraph), so the reader holds a value to its bound where it is
    // read, and a raise seam where a host hands it one.
    fn text_type(&self, _max: u32) -> String {
        "String".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "[UInt8]".to_string()
    }
    // The one Swift spelling of a value that may be absent (docs/adr/0055,
    // section 5).
    fn optional_type(&self, elem: &str) -> String {
        format!("{elem}?")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("[{elem}]")
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
    // schema names it, which is how a read names it, and built whole by the
    // initializer of every field, which a host calls with the labels in that
    // order. Every member starts at its zero ([`SwiftComposed::zero_of`]), so
    // the record is `SceDefault`: what a checked read of an element of a list
    // of it answers where the index is outside the list. A Swift struct has no
    // constructor that could refuse a value, so the raise seam that takes one
    // holds it to every bound its type declares before it sends it
    // (docs/adr/0086, section 5.4), and no check is declared beside it.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    public var {id}: {ty} = {}\n", self.zero_of(ty)))
            .collect();
        // A record of no fields is built by the one initializer of none.
        let whole = if fields.is_empty() {
            String::new()
        } else {
            let params: Vec<String> = fields
                .iter()
                .map(|(id, ty)| format!("{id}: {ty}"))
                .collect();
            let assigns: String = fields
                .iter()
                .map(|(id, _)| format!("        self.{id} = {id}\n"))
                .collect();
            format!(
                "\n    public init({}) {{\n{assigns}    }}\n",
                params.join(", ")
            )
        };
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed\n\
             /// value (docs/adr/0086), built whole from its fields in the schema's order; a\n\
             /// raise seam holds it to every bound its type declares before it sends it.\n\
             public struct {ty}: Equatable, Sendable, SceDefault {{\n{members}\n\
             \x20   public init() {{}}\n{whole}}}"
        )
    }

    fn read_binding(&self) -> &'static str {
        SWIFT_READ_BINDING
    }
    // The width is the closure's own result type, which the runtime's generic
    // reader is called at, so a field widened in its schema cannot keep being
    // read at the old width.
    fn read_scalar(&self, ty: &SceType, _value: &str) -> String {
        let read = match ty {
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 => "unsigned",
            SceType::Int8 | SceType::Int16 | SceType::Int32 => "signed",
            SceType::Uint64 => "unsigned64",
            SceType::Int64 => "signed64",
            SceType::Float32 => "float32",
            SceType::Float64 => "float64",
            SceType::Bool => "truth",
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        };
        format!(
            "{} try {SWIFT_READ_BINDING}.{read}() }}",
            closure_head(&self.scalar_type(ty))
        )
    }
    // The variant whose declared name the wire carries, by the machine enum's
    // own `init?(declaredName:)`; a name the enum does not declare is refused
    // with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, _value: &str) -> String {
        format!(
            "{} let sceName = try {SWIFT_READ_BINDING}.name(); guard let sceVariant = \
             {}(declaredName: sceName) else {{ throw {SWIFT_READ_BINDING}.unknownVariant(sceName, \
             alias: {}) }}; return sceVariant }}",
            closure_head(&held.ty),
            held.ty,
            literal(&held.alias)
        )
    }
    fn read_text(&self, max: u32, _value: &str) -> String {
        format!(
            "{} try {SWIFT_READ_BINDING}.text(bound: {max}) }}",
            closure_head("String")
        )
    }
    fn read_bytes(&self, max: u32, _value: &str) -> String {
        format!(
            "{} try {SWIFT_READ_BINDING}.bytes(bound: {max}) }}",
            closure_head("[UInt8]")
        )
    }
    fn read_optional(&self, _value: &str, elem: &str, elem_ty: &str) -> String {
        format!(
            "{} try {SWIFT_READ_BINDING}.optional({elem}) }}",
            closure_head(&self.optional_type(elem_ty))
        )
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, _value: &str, least: u32, max: u32, elem: &str, elem_ty: &str) -> String {
        let least = if least == 0 {
            String::new()
        } else {
            format!("least: {least}, ")
        };
        format!(
            "{} try {SWIFT_READ_BINDING}.list({least}bound: {max}, {elem}) }}",
            closure_head(&self.list_type(elem_ty, max))
        )
    }
    // The record built whole by the initializer of every field, each field
    // read by its own reader out of the fields of the object, in the schema's
    // order, the first refusal thrown as it is; a field a later revision added
    // is read out of the defaults where the value lacks it.
    fn read_record(
        &self,
        _value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults =
            defaults.map_or_else(String::new, |json| format!("defaults: {}, ", literal(json)));
        let args: Vec<String> = fields
            .iter()
            .map(|(id, read)| {
                format!(
                    "{id}: ({read})({SWIFT_FIELDS_BINDING}.field({}))",
                    literal(id)
                )
            })
            .collect();
        format!(
            "{} try {SWIFT_READ_BINDING}.record({defaults}{{ ({SWIFT_FIELDS_BINDING}: \
             PayloadFields) throws -> {ty} in try {ty}({}) }}) }}",
            closure_head(ty),
            args.join(", ")
        )
    }

    fn write_binding(&self) -> &'static str {
        SWIFT_ELEMENT_BINDING
    }
    // The JSON a scalar param of the same type crosses as (`WireValue.json`): a
    // narrow integer as a number and a real as a double, both exactly, and a
    // 64-bit integer as the text of its digits.
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        let value = &held.text;
        let wire = match ty {
            SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32 => format!("integer(Int64({value}))"),
            SceType::Uint64 => format!("unsigned64({value})"),
            SceType::Int64 => format!("signed64({value})"),
            SceType::Float32 => format!("real(Double({value}))"),
            SceType::Float64 => format!("real({value})"),
            SceType::Bool => format!("truth({value})"),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        };
        format!("WireValue.{wire}.json")
    }
    // The machine's enum's `declaredName` answers the name its document
    // declares.
    fn write_enum(&self, held: &Held) -> String {
        format!("WireValue.text({}.declaredName).json", held.text)
    }
    fn write_text(&self, held: &Held) -> String {
        format!("WireValue.text({}).json", held.text)
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        format!("WireValue.bytes({}).json", held.text)
    }
    // The present value where there is one, and `null` where there is none,
    // as an absent scalar param is.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        format!(
            "optionalJSON({}, write: {{ ({SWIFT_ELEMENT_BINDING}) -> String in {elem} }})",
            held.text
        )
    }
    // Each element written in the list's order.
    fn write_list(&self, held: &Held, elem: &str) -> String {
        format!(
            "listJSON({}, {{ ({SWIFT_ELEMENT_BINDING}) -> String in {elem} }})",
            held.text
        )
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime writes the keys ascending by their bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), whatever order they are given
    // in.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| format!("PayloadPair.json({}, rawJSON: {value})", literal(id)))
            .collect();
        format!("recordJSON([{}])", members.join(", "))
    }
    // A `<send>` holds the JSON the writers wrote as the one case of its
    // `WireValue` that carries written JSON as it is.
    fn send_param(&self, written: String) -> String {
        match self.carrier {
            SwiftCarrier::Json => written,
            SwiftCarrier::Wire => format!("WireValue.composed({written})"),
        }
    }

    fn copy_binding(&self) -> &'static str {
        SWIFT_COPY_BINDING
    }
    // Swift copies by value: an assignment of an `Array`, an `Optional` or a
    // struct copies every element at every depth, so the variable it is copied
    // into, which is appended to and emptied in place, shares nothing with the
    // place the value was taken from.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        held.text.clone()
    }
    // A text and a byte string are one Swift type whatever their bound, so a
    // walk that copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    // The present value mapped into the place's type, absent where it is
    // absent.
    fn copy_optional(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "({}).map({{ ({SWIFT_COPY_BINDING}) -> {elem_ty} in {elem} }})",
            held.text
        )
    }
    // A `T` is lifted into a `T?` wherever one is expected.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "({}).map({{ ({SWIFT_COPY_BINDING}) -> {elem_ty} in {elem} }})",
            held.text
        )
    }

    // A Swift machine holds no union yet: the gate refuses one by name before a
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
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Swift, "", variant)
    }
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        None
    }
    fn union_case_read(&self, variant: &str) -> String {
        swift_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        format!(
            "provenField{}",
            crate::filters::to_pascal_case(field.to_string())
        )
    }
    fn union_def(&self, _def: &UnionDef) -> String {
        unreachable!("{SWIFT_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{SWIFT_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{SWIFT_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{SWIFT_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(SWIFT_HOLDS_NO_UNION.to_string())
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
        Err(SWIFT_HOLDS_NO_UNION.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const SWIFT: SwiftComposed<'static> = SwiftComposed {
        machine: "M",
        carrier: SwiftCarrier::Json,
    };

    /// The Swift row of the one table of what each backend lowers of a
    /// composed value: a payload read at any depth, a list taken whole and a
    /// composed value sent (docs/adr/0086, sections 5 and 6), and no union held
    /// yet (docs/adr/0092, section 7).
    #[test]
    fn the_swift_row_lowers_every_composed_position_but_a_union() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Swift),
            ComposedLowering {
                name: "swift",
                reads: true,
                copies: true,
                unions: false,
                sends: true,
            }
        );
    }

    /// The head of a Swift reader that answers `ty`, as the tests below spell
    /// what they expect of one.
    fn head(ty: &str) -> String {
        format!("{{ (sceValue: PayloadValue) throws -> {ty} in")
    }

    /// The Swift row of section 7.1: no type carries a bound, a value that may
    /// be absent is `T?` over its type, a record is the machine's struct, at
    /// any depth; every member of a record starts at its zero, and no check is
    /// declared beside it, since a Swift struct has no constructor that could
    /// refuse a value.
    #[test]
    fn a_swift_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &SWIFT).expect("a type");
        assert_eq!(spelled("uint16"), "UInt16");
        assert_eq!(spelled("string[8]"), "String");
        assert_eq!(spelled("bytes[4]"), "[UInt8]");
        assert_eq!(spelled("optional<int64>"), "Int64?");
        assert_eq!(spelled("list<list<uint8>[2]>[3]"), "[[UInt8]]");
        assert_eq!(spelled("list<enum:Level>[2]"), "[MLevelEnum]");
        assert_eq!(spelled("list<record:Item>[3]"), "[MItemRecord]");
        assert_eq!(spelled("optional<list<uint8>[2]>"), "[UInt8]?");
        assert_eq!(spelled("list<optional<uint16>>[2]"), "[UInt16?]");
        let members = record_members("Item", &held, &enums, &SWIFT).expect("members");
        let checks = record_checks("Item", &held, &SWIFT).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = SWIFT.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.ends_with(
                "public struct MItemRecord: Equatable, Sendable, SceDefault {\n    \
                 public var id: UInt32 = 0\n    \
                 public var level: MLevelEnum = MLevelEnum.allCases[0]\n    \
                 public var due: Int64? = nil\n    \
                 public var blob: [UInt8] = []\n    \
                 public var tags: [String] = []\n\n    \
                 public init() {}\n\n    \
                 public init(id: UInt32, level: MLevelEnum, due: Int64?, blob: [UInt8], tags: \
                 [String]) {\n        \
                 self.id = id\n        \
                 self.level = level\n        \
                 self.due = due\n        \
                 self.blob = blob\n        \
                 self.tags = tags\n    \
                 }\n}"
            ),
            "{def}"
        );
    }

    /// The zero of each type the spelling holds a value in, read off the
    /// outermost constructor: absent, the empty list, the scalar's zero, a
    /// record of the machine at its zero and the first case of an enum.
    #[test]
    fn a_swift_member_starts_at_the_zero_of_its_type() {
        assert_eq!(SWIFT.zero_of("[UInt8]?"), "nil");
        assert_eq!(SWIFT.zero_of("[UInt16?]"), "[]");
        assert_eq!(SWIFT.zero_of("UInt32"), "0");
        assert_eq!(SWIFT.zero_of("Double"), "0.0");
        assert_eq!(SWIFT.zero_of("Bool"), "false");
        assert_eq!(SWIFT.zero_of("String"), "\"\"");
        assert_eq!(SWIFT.zero_of("MItemRecord"), "MItemRecord()");
        assert_eq!(SWIFT.zero_of("MLevelEnum"), "MLevelEnum.allCases[0]");
    }

    /// One closure of the value per constructor, naming the type it answers,
    /// each calling the runtime's reader of its constructor, the reader of a
    /// part handed to the combinator of its holder; a bound and a lower count
    /// handed to the list that holds them, a record built whole from its
    /// fields with its defaults.
    #[test]
    fn a_swift_reader_is_one_closure_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &SWIFT, "v").expect("read");
        let byte = format!("{} try sceValue.unsigned() }}", head("UInt8"));
        assert_eq!(read("uint8"), byte);
        assert_eq!(
            read("int16"),
            format!("{} try sceValue.signed() }}", head("Int16"))
        );
        let wide = format!("{} try sceValue.signed64() }}", head("Int64"));
        assert_eq!(read("int64"), wide);
        assert_eq!(
            read("uint64"),
            format!("{} try sceValue.unsigned64() }}", head("UInt64"))
        );
        assert_eq!(
            read("bool"),
            format!("{} try sceValue.truth() }}", head("Bool"))
        );
        assert_eq!(
            read("float32"),
            format!("{} try sceValue.float32() }}", head("Float"))
        );
        assert_eq!(
            read("string[8]"),
            format!("{} try sceValue.text(bound: 8) }}", head("String"))
        );
        assert_eq!(
            read("bytes[4]"),
            format!("{} try sceValue.bytes(bound: 4) }}", head("[UInt8]"))
        );
        assert_eq!(
            read("optional<int64>"),
            format!("{} try sceValue.optional({wide}) }}", head("Int64?"))
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            format!(
                "{} try sceValue.list(bound: 3, {} try sceValue.list(bound: 2, {byte}) }}) }}",
                head("[[UInt8]]"),
                head("[UInt8]")
            )
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            format!(
                "{} try sceValue.list(least: 2, bound: 3, {} try sceValue.list(least: 1, bound: \
                 2, {byte}) }}) }}",
                head("[[UInt8]]"),
                head("[UInt8]")
            )
        );
        assert_eq!(
            read("enum:Level"),
            format!(
                "{} let sceName = try sceValue.name(); guard let sceVariant = \
                 MLevelEnum(declaredName: sceName) else {{ throw \
                 sceValue.unknownVariant(sceName, alias: \"Level\") }}; return sceVariant }}",
                head("MLevelEnum")
            )
        );
        let item = read("record:Item");
        let id = format!("{} try sceValue.unsigned() }}", head("UInt32"));
        assert!(
            item.starts_with(&format!(
                "{} try sceValue.record({{ (sceFields: PayloadFields) throws -> MItemRecord in \
                 try MItemRecord(id: ({id})(sceFields.field(\"id\")), ",
                head("MItemRecord")
            )),
            "{item}"
        );
        assert!(
            item.ends_with(&format!(
                "tags: ({} try sceValue.list(bound: 4, {} try sceValue.text(bound: 8) }}) \
                 }})(sceFields.field(\"tags\"))) }}) }}",
                head("[String]"),
                head("String")
            )),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.starts_with(&format!(
                "{} try sceValue.record(defaults: \"{{\\\"late\\\":[]}}\", ",
                head("MBatchRecord")
            )),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor: a scalar as the `WireValue` of its type writes it, a list
    /// by `listJSON`, a value that may be absent by `optionalJSON`, a record
    /// by its field ids, which `recordJSON` orders.
    #[test]
    fn a_swift_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &SWIFT, &at).expect("write");
        assert_eq!(
            write("uint8", Held::place("x.n")),
            "WireValue.integer(Int64(x.n)).json"
        );
        assert_eq!(
            write("int64", Held::place("x.n")),
            "WireValue.signed64(x.n).json"
        );
        assert_eq!(
            write("float32", Held::place("x.n")),
            "WireValue.real(Double(x.n)).json"
        );
        assert_eq!(
            write("bytes[4]", Held::place("x.b")),
            "WireValue.bytes(x.b).json"
        );
        assert_eq!(
            write("enum:Level", Held::reference("sceElement")),
            "WireValue.text(sceElement.declaredName).json"
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            "listJSON(x.maybe, { (sceElement) -> String in optionalJSON(sceElement, write: { \
             (sceElement) -> String in WireValue.integer(Int64(sceElement)).json }) })"
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "listJSON(x.items, { (sceElement) -> String in recordJSON([PayloadPair.json(\"id\", \
             rawJSON: WireValue.integer(Int64(sceElement.id)).json), ",
            "PayloadPair.json(\"level\", rawJSON: WireValue.text(sceElement.level.declaredName)\
             .json), ",
            "PayloadPair.json(\"due\", rawJSON: optionalJSON(sceElement.due, write: { \
             (sceElement) -> String in WireValue.signed64(sceElement).json })), ",
            "PayloadPair.json(\"blob\", rawJSON: WireValue.bytes(sceElement.blob).json), ",
            "PayloadPair.json(\"tags\", rawJSON: listJSON(sceElement.tags, { (sceElement) -> \
             String in WireValue.text(sceElement).json }))]) })",
        ] {
            assert!(items.contains(part), "expected `{part}` in:\n{items}");
        }
        // The inject seam writes the JSON itself.
        assert_eq!(SWIFT.send_param("x".to_string()), "x");
    }

    /// The value a Swift `<send>` holds a composed param in: the one
    /// `WireValue` that carries the JSON the writers wrote as it is, every
    /// scalar inside it written as a scalar param of its type is.
    #[test]
    fn a_swift_send_holds_the_json_its_writers_wrote_as_one_wire_value() {
        let held = held();
        let wire = SwiftComposed {
            machine: "M",
            carrier: SwiftCarrier::Wire,
        };
        let written = writer(
            &ty("list<list<uint8>[2]>[2]"),
            None,
            &held,
            &wire,
            &Held::place("x.grid"),
        )
        .expect("write");
        assert_eq!(
            wire.send_param(written),
            "WireValue.composed(listJSON(x.grid, { (sceElement) -> String in \
             listJSON(sceElement, { (sceElement) -> String in \
             WireValue.integer(Int64(sceElement)).json }) }))"
        );
    }

    /// Two places whose types differ only in their bounds are one Swift type,
    /// and a Swift assignment copies an `Array` by value at every depth, so a
    /// value that fits is copied as it is; where the types differ, each part is
    /// copied into the place's type by a `map` whose closure names it.
    #[test]
    fn a_swift_copy_of_a_value_that_fits_is_the_value() {
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
                &SWIFT,
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
            copy("list<record:Item>[2]", "list<record:Item>[3]"),
            Ok("x".to_string())
        );
        assert_eq!(
            copy("list<uint8>[2]", "list<optional<uint8>>[2]"),
            Ok("(x).map({ (sceCopied) -> UInt8? in sceCopied })".to_string())
        );
    }

    /// A Swift raise seam holds a host's value to every bound its type
    /// declares (docs/adr/0086, section 5.4): one check per constructor that
    /// holds a bound, an element's or a present value's inside the closure the
    /// runtime hands it to, each parameter named by its depth; a lower count is
    /// held too; a record a list holds is checked field by field at the path
    /// below the element; a number, an enum and an optional one of them are
    /// not checked.
    #[test]
    fn a_swift_raise_seam_checks_each_bound_before_it_encodes() {
        let held = held();
        let check = |text: &str, id: &str| {
            swift_bound_check(&ty(text), None, &literal(id), id, 0, &held).expect("a check")
        };
        assert_eq!(check("uint8", "n"), None);
        assert_eq!(check("enum:Level", "level"), None);
        assert_eq!(check("optional<int64>", "due"), None);
        assert_eq!(
            check("list<list<uint8>[2]>[2..3]", "grid"),
            Some(
                "try checkList(\"grid\", grid, least: 2, bound: 3, { scePath0, sceValue0 in try \
                 checkList(scePath0, sceValue0, bound: 2, { _, _ in }) })"
                    .to_string()
            )
        );
        assert_eq!(
            check("optional<list<string[4]>[2]>", "spare"),
            Some(
                "try checkOptional(\"spare\", spare, { scePath0, sceValue0 in try \
                 checkList(scePath0, sceValue0, bound: 2, { scePath1, sceValue1 in try \
                 checkText(scePath1, sceValue1, bound: 4) }) })"
                    .to_string()
            )
        );
        let batch = held.record(None, "Batch").expect("Batch");
        assert_eq!(
            swift_raise_checks(batch, &held).expect("checks"),
            vec![
                "try checkList(\"items\", items, bound: 3, { scePath0, sceValue0 in try \
                 checkBytes(scePath0 + \"/blob\", sceValue0.blob, bound: 4); try \
                 checkList(scePath0 + \"/tags\", sceValue0.tags, bound: 4, { scePath1, \
                 sceValue1 in try checkText(scePath1, sceValue1, bound: 8) }) })"
                    .to_string(),
                "try checkList(\"grid\", grid, bound: 2, { scePath0, sceValue0 in try \
                 checkList(scePath0, sceValue0, bound: 2, { _, _ in }) })"
                    .to_string(),
                "try checkList(\"late\", late, bound: 2, { _, _ in })".to_string(),
            ]
        );
    }

    /// A Swift walk names no union: the one decision of what is lowered
    /// refuses one on Swift first, and the walk that would declare or check one
    /// says why there is none.
    #[test]
    fn a_swift_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &SWIFT).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(SWIFT
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
        assert!(
            swift_bound_check(&ty("union:Shape"), None, "path", "one", 0, &shapes)
                .unwrap_err()
                .contains("holds no union")
        );
    }
}
