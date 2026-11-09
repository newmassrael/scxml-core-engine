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
    /// Whether the spelling names a read of a union's tag, which is how a
    /// backend says it holds a union at all
    /// ([`ComposedSpelling::union_tag_read`]). A machine takes it from the
    /// backend's row ([`Self::new`]), so the spelling and the row cannot
    /// disagree; a test of the spellings that are written before the row says
    /// so sets it itself.
    pub unions: bool,
}

impl<'a> SwiftComposed<'a> {
    /// The spelling of the machine `machine`, as the backend's row of the one
    /// table of what each backend lowers says it holds
    /// ([`crate::forge::event_schema_check::ComposedLowering::of`]).
    pub fn new(machine: &'a str, carrier: SwiftCarrier) -> Self {
        Self {
            machine,
            carrier,
            unions: crate::forge::event_schema_check::ComposedLowering::of(
                crate::generator::Language::Swift,
            )
            .unions,
        }
    }
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

/// The name a Swift union reader binds the case the tag named to.
const SWIFT_CASE_BINDING: &str = "sceCase";

/// The identifier of the case `variant` of a machine's union: the variant as
/// the Swift declaration of an enum spells it, with no document to qualify it
/// ([`crate::forge::enum_naming::variant_ident`]), as the machine's own enum
/// spells the same variant.
fn swift_machine_case_ident(variant: &str) -> String {
    crate::forge::enum_naming::variant_ident(crate::generator::Language::Swift, "", variant)
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
pub(crate) fn swift_shared_read(field: &str) -> String {
    format!(
        "provenField{}",
        crate::filters::to_pascal_case(field.to_string())
    )
}

/// The Swift spelling of what follows a union's value to read `member` where it
/// may be in the cases `left` ([`ComposedSpelling::union_member`]): the tag's
/// property; the field of the one case left, read through that case's proven
/// read, a place as a record's field is; or the accessor of a field several
/// cases share ([`shared_plain_field`]). `Err` for a field the cases left share
/// whose value is not copied as it is read, which no accessor answers yet.
fn swift_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    if member == union.tag {
        return Ok(SWIFT_UNION_KIND.to_string());
    }
    let declares = |case: &ScopedCase| -> bool {
        case.record
            .as_deref()
            .and_then(|record| union.scope.records.get(record))
            .is_some_and(|record| record.fields.iter().any(|(id, _)| id == member))
    };
    match left {
        [case] if declares(case) => Ok(format!("{}.{member}", swift_case_read(&case.variant.name))),
        _ if !left.is_empty()
            && left.iter().all(|case| declares(case))
            && shared_plain_field(union, member).is_some() =>
        {
            Ok(swift_shared_read(member))
        }
        _ => Err(format!(
            "`{member}` of `union:{}`, read where it may be in the cases {}, is a field those \
             cases share whose value is not copied as it is read, and a machine reads such a \
             field only where a test proves one case (docs/adr/0092, section 6.1)",
            union.name,
            left.iter()
                .map(|case| format!("`{}`", case.variant.name))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

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
        // The case the value is in, whose record is checked field by field at the
        // path of the union's own (docs/adr/0092, section 7): the tag is part of
        // no path, and a case that holds nothing, or nothing to check, is left.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            let bound = format!("sceCase{depth}");
            let mut arms = Vec::new();
            for case in &held.union.cases {
                let Some(record) = &case.record else {
                    continue;
                };
                let record = ValueType::Record {
                    alias: record.clone(),
                };
                if let Some(check) =
                    swift_bound_check(&record, Some(&held.key), path, &bound, depth, records)?
                {
                    arms.push(format!(
                        "case .{}(let {bound}): {check}",
                        swift_machine_case_ident(&case.variant.name)
                    ));
                }
            }
            if arms.is_empty() {
                return Ok(None);
            }
            // Every other case holds nothing a bound is declared on; where there
            // is none, a `default` would be one the compiler warns is never run.
            if arms.len() < held.union.cases.len() {
                arms.push("default: break".to_string());
            }
            Some(format!("switch {value} {{ {} }}", arms.join("; ")))
        }
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

    // The machine's own sum, beside its records (docs/adr/0092, section 7, the
    // Swift row): named by the union's document, so one union is one type of the
    // machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        swift_machine_case_ident(variant)
    }
    // The tag is read by the one property every union of a Swift unit and of a
    // Swift machine answers it by, whatever key it is written under on the wire.
    // A machine that holds no union yet ([`SwiftComposed::unions`]) names none.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        self.unions.then(|| SWIFT_UNION_KIND.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        swift_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        swift_shared_read(field)
    }
    // An enum of the machine, a value compared whole as a record that holds a
    // composed value is. Every read the narrowing proves is a property of it,
    // whose other cases are a `preconditionFailure` (the one place they are
    // written, as a proven optional is read through the runtime's one `proven`,
    // docs/adr/0055, section 5), and each has its place beside it that is
    // written through: a getter and a setter that assigns the case whole.
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_swift(def.name.clone());
        let others = |covered: usize, variant: &str, indent: &str| {
            if covered < def.cases.len() {
                format!(
                    "{indent}default:\n{indent}    preconditionFailure(\"a value of {name} read as \
                     {} is in another case\")\n",
                    crate::filters::escape_swift(variant.to_string())
                )
            } else {
                String::new()
            }
        };
        let mut variants = String::new();
        let mut kind_arms = String::new();
        let mut proven = String::new();
        for case in &def.cases {
            let ident = &case.ident;
            kind_arms.push_str(&format!(
                "        case .{ident}:\n            return {}.{}\n",
                def.tag_ty, case.tag_ident
            ));
            match &case.record {
                Some(record) => {
                    variants.push_str(&format!("    case {ident}({record})\n"));
                    proven.push_str(&format!(
                        "\n    public var {read}: {record} {{\n        guard case .{ident}(let \
                         sceRecord) = self else {{\n            preconditionFailure(\"a value of \
                         {name} read as {variant} is in another case\")\n        }}\n        \
                         return sceRecord\n    }}\n\n    public var {place}: {record} {{\n        \
                         get {{ {read} }}\n        set {{ self = .{ident}(newValue) }}\n    }}\n",
                        read = case.proven,
                        place = self.union_place_of(&case.proven),
                        variant = crate::filters::escape_swift(case.variant.clone()),
                    ));
                }
                None => variants.push_str(&format!("    case {ident}\n")),
            }
        }
        let shared: String = def
            .shared
            .iter()
            .map(|field| {
                let reads: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "        case .{ident}(let sceRecord):\n            return \
                             sceRecord.{}\n",
                            field.field
                        )
                    })
                    .collect();
                let writes: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "            case .{ident}(var sceRecord):\n                \
                             sceRecord.{} = newValue\n                self = .{ident}(sceRecord)\n",
                            field.field
                        )
                    })
                    .collect();
                format!(
                    "\n    public var {read}: {ty} {{\n        switch self {{\n{reads}{}        \
                     }}\n    }}\n\n    public var {place}: {ty} {{\n        get {{ {read} }}\n        \
                     set {{\n            switch self {{\n{writes}{}            }}\n        }}\n    \
                     }}\n",
                    others(field.cases.len(), &field.field, "        "),
                    others(field.cases.len(), &field.field, "            "),
                    read = field.accessor,
                    place = self.union_place_of(&field.accessor),
                    ty = field.ty,
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092).\n\
             public enum {ty}: Equatable, Sendable {{\n{variants}\n    public var {tag}: {tag_ty} \
             {{\n        switch self {{\n{kind_arms}        }}\n    }}\n{proven}{shared}}}",
            ty = def.ty,
            tag = def.tag_method,
            tag_ty = def.tag_ty,
        )
    }
    // The case's constructor, holding the record or nothing.
    fn union_value(&self, ty: &str, case: &str, record: Option<&str>) -> String {
        match record {
            Some(record) => format!("{ty}.{case}({record})"),
            None => format!("{ty}.{case}"),
        }
    }
    // The tag first, by the enum's reader, then the record of the case it
    // names from the same object (docs/adr/0092, section 4), each case an arm
    // of a `switch` the compiler holds to every variant of the tag's enum.
    fn read_union(&self, _value: &str, read: &UnionRead<'_>) -> String {
        let arms: String = read
            .arms
            .iter()
            .map(|arm| {
                let case = format!("{}.{}", read.ty, arm.case_ident);
                let value = match &arm.record {
                    Some(record) => format!("{case}(try ({record})({SWIFT_READ_BINDING}))"),
                    None => case,
                };
                format!("case {}.{}: return {value}; ", read.tag_ty, arm.tag_ident)
            })
            .collect();
        format!(
            "{} try {SWIFT_READ_BINDING}.union(tag: {}, {}, {{ ({SWIFT_CASE_BINDING}: {}, \
             {SWIFT_READ_BINDING}: PayloadValue) throws -> {} in switch {SWIFT_CASE_BINDING} \
             {{ {arms}}} }}) }}",
            closure_head(&read.ty),
            literal(read.tag),
            read.tag_reader,
            read.tag_ty,
            read.ty
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092,
    // section 2.1), keys ascending as the runtime's `unionJSON` writes them.
    // The case's record is the reference its arm binds.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let tag = literal(tag);
        let arms: String = arms
            .iter()
            .map(|arm| {
                let variant = literal(&arm.variant);
                match &arm.fields {
                    Some(fields) => {
                        let members: Vec<String> = fields
                            .iter()
                            .map(|(id, json)| {
                                format!("PayloadPair.json({}, rawJSON: {json})", literal(id))
                            })
                            .collect();
                        format!(
                            "case .{}(let {SWIFT_ELEMENT_BINDING}): return unionJSON(tag: {tag}, \
                             variant: {variant}, [{}]); ",
                            arm.case_ident,
                            members.join(", ")
                        )
                    }
                    None => format!(
                        "case .{}: return unionJSON(tag: {tag}, variant: {variant}, []); ",
                        arm.case_ident
                    ),
                }
            })
            .collect();
        format!(
            "{{ () -> String in switch ({}) as {ty} {{ {arms}}} }}()",
            held.text
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        swift_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}Place")
    }
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
        swift_union_member(union, left, member)?;
        Ok(match left {
            [case] => format!(
                "{holder}.{}.{member}",
                self.union_place_of(&swift_case_read(&case.variant.name))
            ),
            _ => format!(
                "{holder}.{}",
                self.union_place_of(&swift_shared_read(member))
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::*;
    use super::*;

    const SWIFT: SwiftComposed<'static> = SwiftComposed {
        machine: "M",
        carrier: SwiftCarrier::Json,
        unions: true,
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
            unions: true,
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

    /// The tag's enum as a Swift machine declares it: a variant is the member
    /// of the enum's type its document's name spells in lowerCamelCase.
    fn swift_kinds() -> std::collections::BTreeMap<String, MachineEnum> {
        [(
            "Kind".to_string(),
            MachineEnum {
                alias: "Kind".to_string(),
                ty: "MKindEnum".to_string(),
                variants: vec![
                    ("dot".to_string(), "dot".to_string()),
                    ("circle".to_string(), "circle".to_string()),
                    ("disc".to_string(), "disc".to_string()),
                ],
            },
        )]
        .into()
    }

    /// A union is the machine's own sum at any depth, named by its document
    /// (docs/adr/0092, section 7), and reaches its tag's enum, then the record
    /// each case holds once, then itself.
    #[test]
    fn a_swift_machine_holds_a_union_in_its_own_sum_after_its_cases_records() {
        let shapes = shapes();
        let kinds = swift_kinds();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &SWIFT),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &SWIFT
            ),
            Ok("[MShapeUnion?]".to_string())
        );
        let mut reached = Reached::default();
        held_records(&ty("list<union:Shape>[2]"), None, &shapes, &mut reached).expect("reached");
        held_records(&ty("union:Shape"), None, &shapes, &mut reached).expect("reached");
        assert_eq!(reached.records, vec!["Radius".to_string()]);
        assert_eq!(reached.unions, vec![SHAPE_KEY.to_string()]);
    }

    /// The declaration: one case per variant of the tag, the tag read as the
    /// machine's enum, a proven read and its place per case that holds a record,
    /// whose other cases are a `preconditionFailure`, and one accessor for
    /// `radius`, which `circle` and `disc` share.
    #[test]
    fn a_swift_union_is_declared_with_its_tag_its_proven_reads_and_its_shared_fields() {
        let def = union_definition(SHAPE_KEY, &shapes(), &swift_kinds(), &SWIFT).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "provenFieldRadius".to_string(),
                ty: "UInt16".to_string(),
                cases: vec!["circle".to_string(), "disc".to_string()],
            }]
        );
        let text = SWIFT.union_def(&def);
        for fragment in [
            "public enum MShapeUnion: Equatable, Sendable {\n    case dot\n    case \
             circle(MRadiusRecord)\n    case disc(MRadiusRecord)\n",
            "    public var kind: MKindEnum {\n        switch self {\n        case .dot:\n            \
             return MKindEnum.dot\n        case .circle:\n            return MKindEnum.circle\n",
            "    public var provenCircle: MRadiusRecord {\n        guard case .circle(let \
             sceRecord) = self else {\n",
            // ... and the place each is written through (section 6.1 item 6).
            "    public var provenCirclePlace: MRadiusRecord {\n        get { provenCircle }\n        \
             set { self = .circle(newValue) }\n    }\n",
            "    public var provenFieldRadius: UInt16 {\n        switch self {\n        case \
             .circle(let sceRecord):\n            return sceRecord.radius\n        case \
             .disc(let sceRecord):\n            return sceRecord.radius\n        default:\n            \
             preconditionFailure(",
            "    public var provenFieldRadiusPlace: UInt16 {\n        get { provenFieldRadius }\n        \
             set {\n            switch self {\n            case .circle(var sceRecord):\n                \
             sceRecord.radius = newValue\n                self = .circle(sceRecord)\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
        assert!(!text.contains("provenDot"), "{text}");
        assert_eq!(text.matches("preconditionFailure(").count(), 4, "{text}");
    }

    /// A union whose every case holds the shared field leaves no case for a
    /// `default`, which Swift warns is never run.
    #[test]
    fn a_swift_accessor_over_every_case_has_no_default() {
        let def = UnionDef {
            ty: "MPairUnion".to_string(),
            name: "pair".to_string(),
            tag_method: "kind".to_string(),
            tag_ty: "MKindEnum".to_string(),
            cases: ["left", "right"]
                .iter()
                .map(|name| UnionCaseDef {
                    ident: name.to_string(),
                    variant: name.to_string(),
                    tag_ident: name.to_string(),
                    record: Some("MRadiusRecord".to_string()),
                    proven: swift_case_read(name),
                })
                .collect(),
            shared: vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "provenFieldRadius".to_string(),
                ty: "UInt16".to_string(),
                cases: vec!["left".to_string(), "right".to_string()],
            }],
        };
        let text = SWIFT.union_def(&def);
        assert!(text.contains("provenFieldRadiusPlace"), "{text}");
        assert!(!text.contains("default:"), "{text}");
    }

    /// A write of a union's field (docs/adr/0092, section 6.1 item 6): through
    /// the place of the one case left, or the place of a field two cases share;
    /// the tag is never written alone.
    #[test]
    fn a_swift_proven_write_is_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|case| case.variant.name == name)
                .expect("a case")
        };
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        assert_eq!(
            SWIFT.union_member_place("self.s", union, &[case("disc")], "radius"),
            Ok("self.s.provenDiscPlace.radius".into())
        );
        assert_eq!(
            SWIFT.union_member_place("self.s", union, &[case("circle"), case("disc")], "radius"),
            Ok("self.s.provenFieldRadiusPlace".into())
        );
        assert!(SWIFT
            .union_member_place("self.s", union, &all, "kind")
            .unwrap_err()
            .contains("never written alone"));
        assert!(SWIFT
            .union_member_place("self.s", union, &[case("dot")], "radius")
            .is_err());
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// property, a field of the one case left through that case's proven read,
    /// and a field two cases share through its accessor.
    #[test]
    fn a_swift_proven_read_is_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|case| case.variant.name == name)
                .expect("a case")
        };
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        assert_eq!(swift_union_member(union, &all, "kind"), Ok("kind".into()));
        assert_eq!(
            swift_union_member(union, &[case("disc")], "radius"),
            Ok("provenDisc.radius".into())
        );
        assert_eq!(
            swift_union_member(union, &[case("circle"), case("disc")], "radius"),
            Ok("provenFieldRadius".into())
        );
        assert!(swift_union_member(union, &[case("dot")], "radius").is_err());
    }

    /// The reader reads the tag by the enum's reader and then the record of
    /// the case it names from the same object, each case an arm of a `switch`
    /// over the tag's enum; the writer writes the tag among the case's fields.
    #[test]
    fn a_swift_union_is_read_tag_first_and_written_by_its_case() {
        let shapes = shapes();
        let kinds = swift_kinds();
        let read = reader(
            &ty("union:Shape"),
            None,
            &shapes,
            &kinds,
            &SWIFT,
            "sceValue",
        )
        .expect("read");
        assert!(
            read.starts_with(
                "{ (sceValue: PayloadValue) throws -> MShapeUnion in try \
                 sceValue.union(tag: \"kind\", { (sceValue: PayloadValue) throws -> MKindEnum \
                 in "
            ),
            "{read}"
        );
        assert!(
            read.contains(
                "{ (sceCase: MKindEnum, sceValue: PayloadValue) throws -> MShapeUnion in switch \
                 sceCase { case MKindEnum.dot: return MShapeUnion.dot; case MKindEnum.circle: \
                 return MShapeUnion.circle(try ("
            ),
            "{read}"
        );
        assert!(read.ends_with("} }) }"), "{read}");
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &SWIFT,
                &Held::place("x.one")
            ),
            Ok(
                "{ () -> String in switch (x.one) as MShapeUnion { case .dot: return \
                unionJSON(tag: \"kind\", variant: \"dot\", []); case .circle(let sceElement): \
                return unionJSON(tag: \"kind\", variant: \"circle\", [PayloadPair.json(\
                \"radius\", rawJSON: WireValue.integer(Int64(sceElement.radius)).json)]); case \
                .disc(let sceElement): return unionJSON(tag: \"kind\", variant: \"disc\", \
                [PayloadPair.json(\"radius\", rawJSON: \
                WireValue.integer(Int64(sceElement.radius)).json)]); } }()"
                    .to_string()
            )
        );
    }

    /// A raise seam holds a union a host built to the bound every case's
    /// record declares, at the path of the union's own place: the case a record
    /// is in is a `switch`, and a union none of whose cases holds a bound has no
    /// check.
    #[test]
    fn a_swift_raise_checks_each_case_of_a_union_where_a_record_holds_a_bound() {
        let shapes = shapes();
        assert_eq!(
            swift_bound_check(&ty("union:Shape"), None, "\"one\"", "one", 0, &shapes),
            Ok(None)
        );
    }

    /// The tag's property is the one name every union answers its tag by.
    #[test]
    fn a_swift_union_answers_its_tag_by_one_property() {
        assert_eq!(SWIFT.union_tag_read("kind"), Some("kind".to_string()));
        assert_eq!(SWIFT.union_tag_read("type"), Some("kind".to_string()));
    }
}
