// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Kotlin spelling of the walks over a composed value ([`super`])
//! (docs/adr/0086, section 7.1, the Kotlin row).
//!
//! | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record | `union:U` |
//! |---|---|---|---|---|
//! | `List<T>` | `String` / `ByteArray` | `T?` | the machine's data class | the machine's sealed interface |
//!
//! No Kotlin type carries a bound (section 7.4, last paragraph), so two places
//! whose types differ only in their bounds are one type, and a value that fits
//! is copied as it is.
//!
//! A union (docs/adr/0092, section 7, the Kotlin row) is a sealed interface
//! of one nested class per case, named as the machine's enum names the
//! variant: a `data class` holding the case's record as `value`, or a `data
//! object` for an empty case. Its values are immutable, as a record's are, so
//! a field of a case is written by assigning the holder the next value whole.

use super::*;

/// The property of a union's Kotlin sealed interface that answers the value of
/// the tag's enum its case is for, whatever key the tag is written under
/// (docs/adr/0092, section 7, the Kotlin row; the unit's interface names it so
/// too).
const KOTLIN_UNION_KIND: &str = "kind";

/// The read of the record of the case `variant` where a test proved it, on a
/// machine's union: a function of the interface, as the unit's is
/// (`forge/kotlin/union.kt.jinja2`).
fn kotlin_case_read(variant: &str) -> String {
    format!(
        "proven{}",
        crate::filters::to_pascal_case(variant.to_string())
    )
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
fn kotlin_shared_read(field: &str) -> String {
    format!(
        "provenField{}",
        crate::filters::to_pascal_case(field.to_string())
    )
}

/// The Kotlin spelling of what follows a union's value to read `member` where
/// it may be in the cases `left` ([`ComposedSpelling::union_member`]), as
/// [`union_member_of`] reads it: the tag's property; the field of the one case
/// left, through that case's proven read; or the accessor of a field several
/// cases share. `Err` as [`union_member_of`]'s.
pub(crate) fn kotlin_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    Ok(match union_member_of(union, left, member)? {
        UnionMember::Tag => KOTLIN_UNION_KIND.to_string(),
        UnionMember::Case(case) => format!("{}().{member}", kotlin_case_read(&case.variant.name)),
        UnionMember::Shared => format!("{}()", kotlin_shared_read(member)),
    })
}

/// The Kotlin read of `member` of the union value `holder`, a parameter or a
/// local of an algorithm spelled as Kotlin names it, whose type is the union's
/// own unit, the sealed interface `ty` (docs/adr/0092, section 7, the Kotlin
/// row, and Landing order step 5), where the value may be in the cases `left`,
/// as [`unit_union_member_of`] classifies it: the tag's property, which the
/// unit's interface answers under one name whatever key the tag is written
/// under; the field of the one case left, through that case's proven read; or
/// the field the cases left each declare, read by a `when` over them that takes
/// each case's record apart, whose `else` the narrowing proved is never taken.
/// The `when` tests `holder` itself, a parameter or a local the body never
/// writes, so each arm reads the case it tested. A case is the class the unit
/// nests in the interface ([`crate::forge::generator::kotlin_case_ident`]), and
/// its record is the payload class its event schema's unit declares, whose
/// fields are named as the schema writes them
/// ([`crate::forge::generator::event_schema_field_ident`]). `Err` as
/// [`unit_union_member_of`]'s.
pub(crate) fn kotlin_unit_union_read(
    holder: &str,
    ty: &str,
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    let field = crate::forge::generator::event_schema_field_ident(
        member,
        crate::generator::Language::Kotlin,
    );
    Ok(match unit_union_member_of(union, left, member)? {
        UnionMember::Tag => format!("{holder}.{KOTLIN_UNION_KIND}"),
        UnionMember::Case(case) => {
            format!(
                "{holder}.{}().{field}",
                kotlin_case_read(&case.variant.name)
            )
        }
        UnionMember::Shared => {
            let arms: Vec<String> = left
                .iter()
                .map(|case| {
                    format!(
                        "is {ty}.{} -> {holder}.value.{field}",
                        crate::forge::generator::kotlin_case_ident(&case.variant.name)
                    )
                })
                .collect();
            // A `when` over every case of a sealed interface is exhaustive, and
            // an `else` beside it is one Kotlin warns is redundant.
            let rest = if unit_union_leaves_others(union, left) {
                format!(
                    "; else -> throw IllegalStateException(\"{}\")",
                    crate::filters::escape_kotlin(unit_union_other_case(union, left))
                )
            } else {
                String::new()
            };
            format!("(when ({holder}) {{ {}{rest} }})", arms.join("; "))
        }
    })
}

/// A Kotlin string literal of `text`.
fn kotlin_literal(text: &str) -> String {
    format!("\"{}\"", crate::filters::escape_kotlin(text.to_string()))
}

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

/// The `init` a Kotlin data class runs `checks` in where a value is made,
/// indented as a member, or `None` where there is no check.
fn kotlin_init(checks: &[String]) -> Option<String> {
    if checks.is_empty() {
        return None;
    }
    let body: String = checks
        .iter()
        .map(|check| format!("        {check}\n"))
        .collect();
    Some(format!("    init {{\n{body}    }}\n"))
}

/// The body a Kotlin data class takes for `checks`: an `init` that runs each
/// where a value is made, or nothing where there is none.
pub(crate) fn kotlin_init_block(checks: &[String]) -> String {
    kotlin_init(checks).map_or_else(String::new, |init| format!(" {{\n{init}}}"))
}

/// The Kotlin expression that writes the value of `ty` held at `value` as a
/// saved state does (`com.sce.runtime.SavedValues`): a number, a truth value,
/// a text and a byte string by `of`, an enum, a record and a union by their own
/// `toSaved`, a value that may be absent as `null` or as its present value,
/// and a list element by element. `depth` names the parameter of each nested
/// lambda, so none hides its holder's. `Err` for a byte string inside a list,
/// which a saved state would write as an array where it writes a byte string
/// as its text.
fn kotlin_saved_write(ty: &ValueType, value: &str, depth: usize) -> Result<String, String> {
    let element = format!("sceSaved{depth}");
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_))
        | ValueType::Record { .. }
        | ValueType::Union { .. } => {
            format!("{value}.toSaved()")
        }
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => {
            format!("SavedValues.of({value})")
        }
        ValueType::Optional { elem } => format!(
            "SavedValues.optional({value}) {{ {element} -> {} }}",
            kotlin_saved_write(elem, &element, depth + 1)?
        ),
        ValueType::List { elem, .. } => {
            if matches!(**elem, ValueType::Bytes { .. }) {
                return Err(
                    "a byte string inside a list, which a saved state has no text for".to_string(),
                );
            }
            format!(
                "SavedValues.list({value}) {{ {element} -> {} }}",
                kotlin_saved_write(elem, &element, depth + 1)?
            )
        }
    })
}

/// What a read of a saved value holds still down the whole walk of one type:
/// the spelling that names the machine's types, the scope the type is written
/// in (`within`, the record whose field it is), the records a statechart holds
/// and the enums it declares.
struct KotlinSavedReader<'a, 'm> {
    spelling: &'a KotlinComposed<'m>,
    within: Option<&'a str>,
    records: &'a dyn RecordEnv,
    enums: &'a BTreeMap<String, MachineEnum>,
}

impl KotlinSavedReader<'_, '_> {
    /// The Kotlin expression that reads the saved value `saved` (an
    /// expression), named `what` in a refusal (an expression of a `String`),
    /// as a value of `ty`, as [`kotlin_saved_write`] writes it: a text and a
    /// byte string held to the bound the machine keeps them to, a list to its
    /// bound and to the lower count its type states, an enum, a record and a
    /// union by their own `fromSaved`. Each refusal is the runtime's `StateRefusal`,
    /// before the record's own `init` could meet a value past a bound. `depth`
    /// names the parameters of each nested lambda, so none hides its holder's.
    fn read(
        &self,
        ty: &ValueType,
        saved: &str,
        what: &str,
        depth: usize,
    ) -> Result<String, String> {
        let (inner_saved, inner_what) = (format!("sceSaved{depth}"), format!("sceWhat{depth}"));
        Ok(match ty {
            ValueType::Scalar(SceType::Enum(reference)) => format!(
                "{}.fromSaved({saved}, {what})",
                machine_enum(self.enums, &reference.alias)?.ty
            ),
            ValueType::Scalar(scalar) => {
                format!("SavedValues.{}({saved}, {what})", scalar.as_attr())
            }
            ValueType::Text { max } => format!("SavedValues.string({saved}, {what}, {})", max.max),
            ValueType::Bytes { max } => format!("SavedValues.bytes({saved}, {what}, {})", max.max),
            ValueType::Optional { elem } => format!(
                "SavedValues.optional({saved}, {what}) {{ {inner_saved}, {inner_what} -> {} }}",
                self.read(elem, &inner_saved, &inner_what, depth + 1)?
            ),
            ValueType::List { elem, max } => {
                let each = self.read(elem, &inner_saved, &inner_what, depth + 1)?;
                if max.min == 0 {
                    format!(
                        "SavedValues.list({saved}, {what}, {}) {{ {inner_saved}, {inner_what} -> \
                         {each} }}",
                        max.max
                    )
                } else {
                    format!(
                        "SavedValues.listWithin({saved}, {what}, {}, {}) {{ {inner_saved}, \
                         {inner_what} -> {each} }}",
                        max.min, max.max
                    )
                }
            }
            ValueType::Record { alias } => {
                let held = self.records.record(self.within, alias)?;
                format!(
                    "{}.fromSaved({saved}, {what})",
                    self.spelling.record_type(held.alias)
                )
            }
            ValueType::Union { alias } => {
                let held = self.records.union(self.within, alias)?;
                format!(
                    "{}.fromSaved({saved}, {what})",
                    self.spelling.union_type(&held.union.name)
                )
            }
        })
    }
}

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
    // value past a bound is refused in its `init`, where it is made. Its
    // companion is what the saved form written beside the type extends with
    // `fromSaved` ([`Self::saved_record_ext`]): declared on every such record,
    // since a record a payload declares first may be held by a variable too,
    // and Kotlin gives a class declared without one no companion.
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
        let init = kotlin_init(checks).map_or_else(String::new, |init| format!("{init}\n"));
        format!(
            "/** SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed \
             value (docs/adr/0086). */\ndata class {ty}({}) {{\n{init}    /** What a value \
             is read back from a saved state through (`fromSaved`). */\n    companion object\n}}",
            params.join(", "),
        )
    }
    // The record's saved form, an object of its fields keyed by their ids in
    // the schema's order, each written and read as its type says
    // ([`kotlin_saved_write`], [`KotlinSavedReader`]): what a variable that
    // holds the record writes it by (`SavedValues`). Extensions, as a record of
    // entries' members are named (`toSaved`, `fromSaved` on its companion), so
    // the machine's saving reads one spelling for either.
    fn saved_record_ext(
        &self,
        alias: &str,
        records: &dyn RecordEnv,
        enums: &BTreeMap<String, MachineEnum>,
    ) -> Result<Option<String>, String> {
        let held = records.record(None, alias)?;
        let ty = self.record_type(alias);
        // The fields of the record name its types in its own document's scope.
        let reader = KotlinSavedReader {
            spelling: self,
            within: Some(held.alias),
            records,
            enums,
        };
        let mut writes = Vec::with_capacity(held.schema.fields.len());
        let mut reads = Vec::with_capacity(held.schema.fields.len());
        for field in &held.schema.fields {
            let key = crate::filters::escape_kotlin(field.id.clone());
            let refused =
                |why: String| format!("the field `{}` of `{}`: {why}", field.id, held.schema.name);
            let written = kotlin_saved_write(&field.sce_type, &format!("this.{}", field.id), 0)
                .map_err(refused)?;
            writes.push(format!("\"{key}\" to {written}"));
            let read = reader
                .read(
                    &field.sce_type,
                    &format!("SavedValues.field(value, what, \"{key}\")"),
                    &format!("\"$what.{key}\""),
                    0,
                )
                .map_err(refused)?;
            reads.push(format!("{} = {read}", field.id));
        }
        Ok(Some(format!(
            "/** This value as a saved state writes it. */\nfun {ty}.toSaved(): Any = \
             linkedMapOf<String, Any?>({})\n\n/** The value a saved state holds, refused unless \
             it is one. */\nfun {ty}.Companion.fromSaved(value: Any?, what: String): {ty} = \
             {ty}({})",
            writes.join(", "),
            reads.join(", "),
        )))
    }
    // The union's saved form is one object, its tag beside the fields of its
    // case (docs/adr/0092, section 6.1 item 7): written by `SavedValues.union`
    // over the record's own `toSaved`, and read by its tag first, which names
    // the record the rest of the object is read as, a name no case has refused
    // by the union's. Extensions, as a composed record's saved form is
    // ([`Self::saved_record_ext`]), the read on the companion the interface
    // declares ([`Self::union_def`]).
    fn saved_union_ext(
        &self,
        def: &UnionDef,
        key: &str,
        records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        let tag = kotlin_literal(&records.union_of_key(key)?.union.tag);
        let name = crate::filters::escape_kotlin(def.name.clone());
        let ty = &def.ty;
        let mut writes = String::new();
        let mut reads = String::new();
        for case in &def.cases {
            let (ident, variant) = (&case.ident, kotlin_literal(&case.variant));
            match &case.record {
                Some(record) => {
                    writes.push_str(&format!(
                        "    is {ty}.{ident} -> SavedValues.union({tag}, {variant}, \
                         this.value.toSaved())\n"
                    ));
                    reads.push_str(&format!(
                        "        {variant} -> {ty}.{ident}({record}.fromSaved(value, what))\n"
                    ));
                }
                None => {
                    writes.push_str(&format!(
                        "    {ty}.{ident} -> SavedValues.union({tag}, {variant}, null)\n"
                    ));
                    reads.push_str(&format!("        {variant} -> {ty}.{ident}\n"));
                }
            }
        }
        Ok(Some(format!(
            "/** This value as a saved state writes it: its tag beside the fields of its case. \
             */\nfun {ty}.toSaved(): Any = when (this) {{\n{writes}}}\n\n/** The value a saved \
             state holds, read by its tag first, refused unless it is one of the cases of \
             `{name}`. */\nfun {ty}.Companion.fromSaved(value: Any?, what: String): {ty} =\n    \
             when (val sceTag = SavedValues.unionTag(value, what, {tag})) {{\n{reads}        else \
             -> throw StateRefusal(\"'$what' ($sceTag) is not a case of {name}\")\n    }}"
        )))
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

    // The machine's own sum, beside its records (docs/adr/0092, section 7, the
    // Kotlin row): named by the union's document, so one union is one type of
    // the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    // The variant as the machine's enum class names its constant (section 7
    // item 1), so a case is spelled by no name the enum's spelling has not
    // already proven safe, and none is the companion's.
    fn union_case_ident(&self, variant: &str) -> String {
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Kotlin, "", variant)
    }
    // The tag is read by the one property every union of a Kotlin unit and of
    // a Kotlin machine answers it by, whatever key it is written under.
    fn union_tag_read(&self, _tag: &str) -> Option<String> {
        Some(KOTLIN_UNION_KIND.to_string())
    }
    fn union_case_read(&self, variant: &str) -> String {
        kotlin_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        kotlin_shared_read(field)
    }
    // A sealed interface of the machine, its values compared whole as a record
    // that holds a composed value is. Every read the narrowing proves is a
    // function of it, whose other cases throw (the one place they are written,
    // as a proven optional is read through the runtime's one `proven`,
    // docs/adr/0055, section 5); a field several cases share has beside its
    // read the function that answers this value with that field replaced,
    // which a write assigns the holder ([`Self::union_member_write`]). Its
    // companion is what the saved form written beside the type extends with
    // `fromSaved` ([`Self::saved_union_ext`]), declared on every union as a
    // record's is.
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_kotlin(def.name.clone());
        let ty = &def.ty;
        let thrown = |read: &str| {
            format!(
                "throw IllegalStateException(\"a value of {name} read as {} is in another case\")",
                crate::filters::escape_kotlin(read.to_string())
            )
        };
        let mut cases = String::new();
        let mut proven = String::new();
        for case in &def.cases {
            let ident = &case.ident;
            let kind = format!(
                "        override val {tag}: {tag_ty}\n            get() = {tag_ty}.{}\n",
                case.tag_ident,
                tag = def.tag_method,
                tag_ty = def.tag_ty,
            );
            match &case.record {
                Some(record) => {
                    cases.push_str(&format!(
                        "\n    /** The case `{variant}`, holding its record. */\n    data class \
                         {ident}(val value: {record}) : {ty} {{\n{kind}    }}\n",
                        variant = crate::filters::escape_kotlin(case.variant.clone()),
                    ));
                    proven.push_str(&format!(
                        "\n    /** The record of the case `{variant}`, read where a test of the \
                         tag proved the value is in it. */\n    fun {read}(): {record} =\n        \
                         (this as? {ident})?.value ?: {thrown}\n",
                        variant = crate::filters::escape_kotlin(case.variant.clone()),
                        read = case.proven,
                        thrown = thrown(&case.variant),
                    ));
                }
                None => cases.push_str(&format!(
                    "\n    /** The case `{variant}`, its tag alone. */\n    data object {ident} : \
                     {ty} {{\n{kind}    }}\n",
                    variant = crate::filters::escape_kotlin(case.variant.clone()),
                )),
            }
        }
        // A `when` over every case is exhaustive, and an `else` beside it is one
        // Kotlin warns is redundant.
        let otherwise = |covered: usize, field: &str| {
            if covered < def.cases.len() {
                format!("        else -> {}\n", thrown(field))
            } else {
                String::new()
            }
        };
        let shared: String = def
            .shared
            .iter()
            .map(|field| {
                let reads: String = field
                    .cases
                    .iter()
                    .map(|ident| format!("        is {ident} -> this.value.{}\n", field.field))
                    .collect();
                let writes: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "        is {ident} -> {ident}(this.value.copy({} = sceValue))\n",
                            field.field
                        )
                    })
                    .collect();
                format!(
                    "\n    /** The field `{id}` the cases that declare it share, read where a \
                     test of the tag proved the value is in one of them. */\n    fun {read}(): \
                     {fty} = when (this) {{\n{reads}{else_read}    }}\n\n    /** This value with \
                     the field `{id}` of its case replaced by [sceValue], where a test of the tag \
                     proved the value is in a case that declares it. */\n    fun {place}(sceValue: \
                     {fty}): {ty} = when (this) {{\n{writes}{else_write}    }}\n",
                    id = crate::filters::escape_kotlin(field.field.clone()),
                    read = field.accessor,
                    place = self.union_place_of(&field.accessor),
                    fty = field.ty,
                    else_read = otherwise(field.cases.len(), &field.field),
                    else_write = otherwise(field.cases.len(), &field.field),
                )
            })
            .collect();
        format!(
            "/** SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092). */\nsealed \
             interface {ty} {{\n    /** The case this value is in, as its tag names it. */\n    \
             val {tag}: {tag_ty}\n{cases}{proven}{shared}\n    /** What a value is read back \
             from a saved state through (`fromSaved`). */\n    companion object\n}}",
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
    // The tag first, by the enum's reader, in the runtime's `Value.union`,
    // which refuses a value that is no object before it; then the record of
    // the case the tag names, read from the same value, each case an arm of a
    // `when` the compiler holds to every variant of the tag's enum
    // (docs/adr/0092, section 4). The case is handed back out of the runtime's
    // call rather than read inside it, so the only named parameter is in a
    // lambda that holds no other: a union inside a case's record declares no
    // name that hides its holder's.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String {
        let arms: Vec<String> = read
            .arms
            .iter()
            .map(|arm| {
                let case = format!("{}.{}", read.ty, arm.case_ident);
                let built = match &arm.record {
                    Some(record) => format!("{case}({value}.let {{ {record} }})"),
                    None => case,
                };
                format!("{}.{} -> {built}", read.tag_ty, arm.tag_ident)
            })
            .collect();
        format!(
            "when ({value}.union({}, {{ {} }}) {{ sceCase, _ -> sceCase }}) {{ {} }}",
            kotlin_literal(read.tag),
            read.tag_reader,
            arms.join("; ")
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092,
    // section 2.1): the runtime's `unionJson`, which orders the keys as
    // `recordJson` does, or the `Map` a `<send>` holds a record in with the tag
    // beside its fields, which the engine orders. The case's record is bound to
    // `it` in its arm, as a record's writer reads it.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        let tag = kotlin_literal(tag);
        let arms: Vec<String> = arms
            .iter()
            .map(|arm| {
                let variant = kotlin_literal(&arm.variant);
                let case = format!("{ty}.{}", arm.case_ident);
                match &arm.fields {
                    Some(fields) => {
                        let members: Vec<String> = fields
                            .iter()
                            .map(|(id, json)| format!("\"{id}\" to {json}"))
                            .collect();
                        let written = match self.carrier {
                            KotlinCarrier::Json => format!(
                                "{KOTLIN_PAYLOAD}.unionJson({tag}, {variant}, mapOf<String, \
                                 Any?>({}))",
                                members.join(", ")
                            ),
                            KotlinCarrier::Wire => format!(
                                "mapOf<String, Any?>({tag} to {variant}, {})",
                                members.join(", ")
                            ),
                        };
                        format!("is {case} -> it.value.let {{ {written} }}")
                    }
                    None => {
                        let written = match self.carrier {
                            KotlinCarrier::Json => {
                                format!("{KOTLIN_PAYLOAD}.unionJson({tag}, {variant}, emptyMap())")
                            }
                            KotlinCarrier::Wire => {
                                format!("mapOf<String, Any?>({tag} to {variant})")
                            }
                        };
                        format!("{case} -> {written}")
                    }
                }
            })
            .collect();
        format!(
            "({}).let {{ when (it) {{ {} }} }}",
            held.text,
            arms.join("; ")
        )
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        kotlin_union_member(union, left, member)
    }
    // A field several cases share is written through the function that answers
    // the union with that field replaced ([`Self::union_def`]); a case's record
    // is rebuilt by the case's own constructor, so its name is kept free only.
    fn union_place_of(&self, read: &str) -> String {
        format!("{read}With")
    }
    // A Kotlin union is an immutable value, so no field of it is a place a
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
            "`{member}` of `union:{}` is a field of an immutable Kotlin value, which has no place \
             of its own: its holder is assigned the next value whole",
            union.name
        ))
    }
    // The holder is assigned the union in the case it is in, its record with
    // the field replaced, as a record's field is written (`copy`), and as an
    // optional record's present field is (`StaticTarget::assign_present_field`);
    // a field several cases share through the function that replaces it in
    // whichever of them the value is in.
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
                let built = format!(
                    "{}.{}",
                    self.union_type(&union.name),
                    self.union_case_ident(&case.variant.name)
                );
                let record = format!("{holder}.{}()", kotlin_case_read(&case.variant.name));
                let field = member.to_string();
                Box::new(move |value: &str| format!("{built}({record}.copy({field} = {value}))"))
            }
            UnionMember::Shared => {
                let replace = format!(
                    "{holder}.{}",
                    self.union_place_of(&kotlin_shared_read(member))
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

    const KOTLIN: KotlinComposed<'static> = KotlinComposed {
        machine: "M",
        carrier: KotlinCarrier::Json,
    };

    /// The Kotlin row of the one table of what each backend lowers of a
    /// composed value: a payload read at any depth, a list taken whole and a
    /// composed value sent (docs/adr/0086, sections 5 and 6), a variable of a
    /// record that holds a composition, which a lift writes whole and a saved
    /// state keeps by its type (docs/adr/0087, "Landing order" item 8), and a
    /// union held, read by its tag, lifted and saved (docs/adr/0092, section 7).
    #[test]
    fn the_kotlin_row_lowers_every_composed_position() {
        use crate::forge::event_schema_check::ComposedLowering;
        assert_eq!(
            ComposedLowering::of(crate::generator::Language::Kotlin),
            ComposedLowering {
                name: "kotlin",
                reads: true,
                copies: true,
                unions: true,
                records: true,
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
                 com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 8) }\n    }\n\n    \
                 /** What a value is read back from a saved state through (`fromSaved`). */\n    \
                 companion object\n}"
            ),
            "{def}"
        );
    }

    /// A Kotlin record that holds a composed value is saved as an object of
    /// its fields keyed by their ids, each by its type, and read back by its
    /// type (docs/adr/0087, "Landing order" item 8): a text and a byte string
    /// held to their bounds, a list to its bound and to the lower count it
    /// states, an enum and a nested record by their own `fromSaved`, a value
    /// that may be absent read absent from `null`. The read is an extension of
    /// the companion every such record declares.
    #[test]
    fn a_kotlin_record_that_holds_a_composed_value_is_saved_by_its_type() {
        let held = held();
        let enums = enums();
        let ext = KOTLIN
            .saved_record_ext("Item", &held, &enums)
            .expect("Item has a saved form")
            .expect("a Kotlin record carries its saved form");
        assert!(
            ext.contains(
                "fun MItemRecord.toSaved(): Any = linkedMapOf<String, Any?>(\"id\" to \
                 SavedValues.of(this.id), \"level\" to this.level.toSaved(), \"due\" to \
                 SavedValues.optional(this.due) { sceSaved0 -> SavedValues.of(sceSaved0) }, \
                 \"blob\" to SavedValues.of(this.blob), \"tags\" to SavedValues.list(this.tags) \
                 { sceSaved0 -> SavedValues.of(sceSaved0) })"
            ),
            "{ext}"
        );
        assert!(
            ext.contains(
                "fun MItemRecord.Companion.fromSaved(value: Any?, what: String): MItemRecord = \
                 MItemRecord(id = SavedValues.uint32(SavedValues.field(value, what, \"id\"), \
                 \"$what.id\"), level = MLevelEnum.fromSaved(SavedValues.field(value, what, \
                 \"level\"), \"$what.level\"), due = SavedValues.optional(SavedValues.field(value, \
                 what, \"due\"), \"$what.due\") { sceSaved0, sceWhat0 -> \
                 SavedValues.int64(sceSaved0, sceWhat0) }, blob = \
                 SavedValues.bytes(SavedValues.field(value, what, \"blob\"), \"$what.blob\", 4), \
                 tags = SavedValues.list(SavedValues.field(value, what, \"tags\"), \"$what.tags\", \
                 4) { sceSaved0, sceWhat0 -> SavedValues.string(sceSaved0, sceWhat0, 8) })"
            ),
            "{ext}"
        );
    }

    /// A list that states a lower count is read back from a saved state held
    /// to it as well as to its bound, so a restore refuses a list its record's
    /// `init` would; a record a list holds is read by its own `fromSaved`.
    #[test]
    fn a_kotlin_saved_list_is_held_to_its_lower_count() {
        let held = held();
        let enums = enums();
        let reader = KotlinSavedReader {
            spelling: &KOTLIN,
            within: None,
            records: &held,
            enums: &enums,
        };
        let read = |text: &str| reader.read(&ty(text), "s", "w", 0).expect("a read");
        assert_eq!(
            read("list<record:Item>[1..3]"),
            "SavedValues.listWithin(s, w, 1, 3) { sceSaved0, sceWhat0 -> \
             MItemRecord.fromSaved(sceSaved0, sceWhat0) }"
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "SavedValues.list(s, w, 3) { sceSaved0, sceWhat0 -> SavedValues.list(sceSaved0, \
             sceWhat0, 2) { sceSaved1, sceWhat1 -> SavedValues.uint8(sceSaved1, sceWhat1) } }"
        );
        assert!(kotlin_saved_write(&ty("list<bytes[4]>[2]"), "x", 0).is_err());
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

    /// The tag's enum as a Kotlin machine declares it: an enum class whose
    /// constant is the variant in upper snake case, unqualified
    /// (`StaticTarget::enum_variant`).
    fn kotlin_kinds() -> BTreeMap<String, MachineEnum> {
        [(
            "Kind".to_string(),
            MachineEnum {
                alias: "Kind".to_string(),
                ty: "MKindEnum".to_string(),
                variants: vec![
                    ("dot".to_string(), "DOT".to_string()),
                    ("circle".to_string(), "CIRCLE".to_string()),
                    ("disc".to_string(), "DISC".to_string()),
                ],
            },
        )]
        .into()
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

    /// A union is the machine's own sealed interface at any depth, named by its
    /// document (docs/adr/0092, section 7, the Kotlin row), and reaches its
    /// tag's enum, then the record each case holds once, then itself.
    #[test]
    fn a_kotlin_machine_holds_a_union_in_its_own_sealed_interface() {
        let shapes = shapes();
        let kinds = kotlin_kinds();
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &KOTLIN),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &KOTLIN
            ),
            Ok("List<MShapeUnion?>".to_string())
        );
        let mut reached = Reached::default();
        held_records(&ty("list<union:Shape>[2]"), None, &shapes, &mut reached).expect("reached");
        held_records(&ty("union:Shape"), None, &shapes, &mut reached).expect("reached");
        assert_eq!(reached.records, vec!["Radius".to_string()]);
        assert_eq!(reached.unions, vec![SHAPE_KEY.to_string()]);
        assert_eq!(KOTLIN.union_tag_read("type"), Some("kind".to_string()));
    }

    /// The declaration: one nested class per variant of the tag, named as the
    /// machine's enum names the constant, the tag a property each case answers
    /// with the machine's enum, a proven read per case that holds a record
    /// whose other cases throw, one accessor for `radius`, which `circle` and
    /// `disc` share, beside the function that replaces it, and the companion
    /// the saved form extends.
    #[test]
    fn a_kotlin_union_is_declared_with_its_tag_its_proven_reads_and_its_shared_fields() {
        let def =
            union_definition(SHAPE_KEY, &shapes(), &kotlin_kinds(), &KOTLIN).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "provenFieldRadius".to_string(),
                ty: "UShort".to_string(),
                cases: vec!["CIRCLE".to_string(), "DISC".to_string()],
            }]
        );
        let text = KOTLIN.union_def(&def);
        let thrown = |read: &str| {
            format!(
                "throw IllegalStateException(\"a value of shape read as {read} is in another case\")"
            )
        };
        for fragment in [
            "sealed interface MShapeUnion {\n    /** The case this value is in, as its tag names \
             it. */\n    val kind: MKindEnum\n"
                .to_string(),
            "    data object DOT : MShapeUnion {\n        override val kind: MKindEnum\n            \
             get() = MKindEnum.DOT\n    }\n"
                .to_string(),
            "    data class CIRCLE(val value: MRadiusRecord) : MShapeUnion {\n        override val \
             kind: MKindEnum\n            get() = MKindEnum.CIRCLE\n    }\n"
                .to_string(),
            format!(
                "    fun provenCircle(): MRadiusRecord =\n        (this as? CIRCLE)?.value ?: {}\n",
                thrown("circle")
            ),
            format!(
                "    fun provenFieldRadius(): UShort = when (this) {{\n        is CIRCLE -> \
                 this.value.radius\n        is DISC -> this.value.radius\n        else -> {}\n    \
                 }}\n",
                thrown("radius")
            ),
            // ... and the function a write of it assigns the holder (section 6.1 item 6).
            format!(
                "    fun provenFieldRadiusWith(sceValue: UShort): MShapeUnion = when (this) {{\n        \
                 is CIRCLE -> CIRCLE(this.value.copy(radius = sceValue))\n        is DISC -> \
                 DISC(this.value.copy(radius = sceValue))\n        else -> {}\n    }}\n",
                thrown("radius")
            ),
        ] {
            assert!(text.contains(&fragment), "expected `{fragment}` in:\n{text}");
        }
        assert!(text.ends_with("    companion object\n}"), "{text}");
        assert!(!text.contains("provenDot"), "{text}");
        assert_eq!(
            text.matches("throw IllegalStateException(").count(),
            4,
            "{text}"
        );
    }

    /// A union whose every case holds the shared field leaves no case for an
    /// `else`, which Kotlin warns is redundant, and a machine is compiled with
    /// every warning an error.
    #[test]
    fn a_kotlin_accessor_over_every_case_has_no_else() {
        let def = UnionDef {
            ty: "MPairUnion".to_string(),
            name: "pair".to_string(),
            tag_method: "kind".to_string(),
            tag_ty: "MKindEnum".to_string(),
            cases: ["LEFT", "RIGHT"]
                .iter()
                .map(|ident| UnionCaseDef {
                    ident: ident.to_string(),
                    variant: ident.to_lowercase(),
                    tag_ident: ident.to_string(),
                    record: Some("MRadiusRecord".to_string()),
                    proven: kotlin_case_read(&ident.to_lowercase()),
                })
                .collect(),
            shared: vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "provenFieldRadius".to_string(),
                ty: "UShort".to_string(),
                cases: vec!["LEFT".to_string(), "RIGHT".to_string()],
            }],
        };
        let text = KOTLIN.union_def(&def);
        assert!(text.contains("provenFieldRadiusWith"), "{text}");
        assert!(!text.contains("else ->"), "{text}");
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// property, a field of the one case left through that case's proven
    /// read, and a field two cases share through its accessor.
    #[test]
    fn a_kotlin_proven_read_is_spelled_by_the_cases_left() {
        let shapes = shapes();
        let union = &shapes.union;
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        assert_eq!(kotlin_union_member(union, &all, "kind"), Ok("kind".into()));
        assert_eq!(
            kotlin_union_member(union, &[case(&shapes, "disc")], "radius"),
            Ok("provenDisc().radius".into())
        );
        assert_eq!(
            KOTLIN.union_member(
                union,
                &[case(&shapes, "circle"), case(&shapes, "disc")],
                "radius"
            ),
            Ok("provenFieldRadius()".into())
        );
        assert!(kotlin_union_member(union, &[case(&shapes, "dot")], "radius").is_err());
    }

    /// A write of a union's field (docs/adr/0092, section 6.1 item 6) assigns
    /// the holder the next value whole, a Kotlin union being immutable: the one
    /// case left rebuilt with its record copied, the field replaced, or a field
    /// two cases share replaced by its function; the tag is never written
    /// alone, and no field is a place of its own.
    #[test]
    fn a_kotlin_proven_write_assigns_the_holder_the_next_value() {
        let shapes = shapes();
        let union = &shapes.union;
        let written = |left: &[&ScopedCase]| match KOTLIN
            .union_member_write("vS", union, left, "radius")
        {
            Ok(UnionMemberWrite::Holder { holder, next }) => (holder, next("7")),
            Ok(UnionMemberWrite::Place(place)) => panic!("a Kotlin union has no place: {place}"),
            Err(why) => panic!("{why}"),
        };
        assert_eq!(
            written(&[case(&shapes, "disc")]),
            (
                "vS".to_string(),
                "MShapeUnion.DISC(vS.provenDisc().copy(radius = 7))".to_string()
            )
        );
        assert_eq!(
            written(&[case(&shapes, "circle"), case(&shapes, "disc")]),
            ("vS".to_string(), "vS.provenFieldRadiusWith(7)".to_string())
        );
        let all: Vec<&ScopedCase> = union.cases.iter().collect();
        let Err(tag) = KOTLIN.union_member_write("vS", union, &all, "kind") else {
            panic!("the tag is never written alone");
        };
        assert!(tag.contains("never written alone"), "{tag}");
        assert!(KOTLIN
            .union_member_write("vS", union, &[case(&shapes, "dot")], "radius")
            .is_err());
        assert!(KOTLIN
            .union_member_place("vS", union, &[case(&shapes, "disc")], "radius")
            .is_err());
    }

    /// The reader reads the tag by the enum's reader inside the runtime's
    /// `Value.union`, which answers it back, and then the record of the case it
    /// names from the same value, each case an arm of a `when` over the tag's
    /// enum; the writer writes the tag among the case's fields, as the JSON
    /// text and as the `Map` a `<send>` holds.
    #[test]
    fn a_kotlin_union_is_read_tag_first_and_written_by_its_case() {
        let shapes = shapes();
        let kinds = kotlin_kinds();
        let radius = "v.let { it.record(null) { MRadiusRecord(radius = it.field(\"radius\").let { \
                      it.uint16() }) } }";
        assert_eq!(
            reader(&ty("union:Shape"), None, &shapes, &kinds, &KOTLIN, "v"),
            Ok(format!(
                "when (v.union(\"kind\", {{ when (val sceName = it.name()) {{ \"dot\" -> \
                 MKindEnum.DOT; \"circle\" -> MKindEnum.CIRCLE; \"disc\" -> MKindEnum.DISC; else \
                 -> throw it.unknownVariant(sceName, \"Kind\") }} }}) {{ sceCase, _ -> sceCase }}) \
                 {{ MKindEnum.DOT -> MShapeUnion.DOT; MKindEnum.CIRCLE -> MShapeUnion.CIRCLE({radius}); \
                 MKindEnum.DISC -> MShapeUnion.DISC({radius}) }}"
            ))
        );
        let p = "com.sce.runtime.EventPayload";
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &KOTLIN,
                &Held::place("x.one")
            ),
            Ok(format!(
                "(x.one).let {{ when (it) {{ MShapeUnion.DOT -> {p}.unionJson(\"kind\", \"dot\", \
                 emptyMap()); is MShapeUnion.CIRCLE -> it.value.let {{ {p}.unionJson(\"kind\", \
                 \"circle\", mapOf<String, Any?>(\"radius\" to {p}.valueJson(it.radius))) }}; is \
                 MShapeUnion.DISC -> it.value.let {{ {p}.unionJson(\"kind\", \"disc\", \
                 mapOf<String, Any?>(\"radius\" to {p}.valueJson(it.radius))) }} }} }}"
            ))
        );
        let wire = KotlinComposed {
            machine: "M",
            carrier: KotlinCarrier::Wire,
        };
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &wire,
                &Held::place("x.one")
            ),
            Ok(
                "(x.one).let { when (it) { MShapeUnion.DOT -> mapOf<String, Any?>(\"kind\" to \
                 \"dot\"); is MShapeUnion.CIRCLE -> it.value.let { mapOf<String, Any?>(\"kind\" to \
                 \"circle\", \"radius\" to (it.radius).toLong()) }; is MShapeUnion.DISC -> \
                 it.value.let { mapOf<String, Any?>(\"kind\" to \"disc\", \"radius\" to \
                 (it.radius).toLong()) } } }"
                    .to_string()
            )
        );
        assert_eq!(
            KOTLIN.union_value("MShapeUnion", "CIRCLE", Some("r")),
            "MShapeUnion.CIRCLE(r)"
        );
        assert_eq!(
            KOTLIN.union_value("MShapeUnion", "DOT", None),
            "MShapeUnion.DOT"
        );
    }

    /// A union is saved as one object, its tag beside the fields of its case
    /// (docs/adr/0092, section 6.1 item 7), and read back by its tag first, the
    /// record of the case by its own `fromSaved`, a name no case has refused;
    /// a union a record or a variable holds is written and read by its own.
    #[test]
    fn a_kotlin_union_is_saved_as_its_tag_beside_its_case() {
        let shapes = shapes();
        let def = union_definition(SHAPE_KEY, &shapes, &kotlin_kinds(), &KOTLIN).expect("declared");
        let ext = KOTLIN
            .saved_union_ext(&def, SHAPE_KEY, &shapes)
            .expect("Shape has a saved form")
            .expect("a Kotlin union carries its saved form");
        for fragment in [
            "fun MShapeUnion.toSaved(): Any = when (this) {\n    MShapeUnion.DOT -> \
             SavedValues.union(\"kind\", \"dot\", null)\n    is MShapeUnion.CIRCLE -> \
             SavedValues.union(\"kind\", \"circle\", this.value.toSaved())\n",
            "fun MShapeUnion.Companion.fromSaved(value: Any?, what: String): MShapeUnion =\n    \
             when (val sceTag = SavedValues.unionTag(value, what, \"kind\")) {\n        \"dot\" -> \
             MShapeUnion.DOT\n        \"circle\" -> MShapeUnion.CIRCLE(MRadiusRecord.fromSaved(value, \
             what))\n",
            "        else -> throw StateRefusal(\"'$what' ($sceTag) is not a case of shape\")\n    }",
        ] {
            assert!(ext.contains(fragment), "expected `{fragment}` in:\n{ext}");
        }
        let reader = KotlinSavedReader {
            spelling: &KOTLIN,
            within: None,
            records: &shapes,
            enums: &BTreeMap::new(),
        };
        assert_eq!(
            reader.read(&ty("optional<union:Shape>"), "s", "w", 0),
            Ok("SavedValues.optional(s, w) { sceSaved0, sceWhat0 -> \
                 MShapeUnion.fromSaved(sceSaved0, sceWhat0) }"
                .to_string())
        );
        assert_eq!(
            kotlin_saved_write(&ty("list<union:Shape>[2]"), "x", 0),
            Ok("SavedValues.list(x) { sceSaved0 -> sceSaved0.toSaved() }".to_string())
        );
    }

    /// An algorithm's union is its unit's sealed interface (docs/adr/0092,
    /// Landing order step 5): the tag by the property the interface answers it
    /// by, the field of the one case left by that case's proven read, a field
    /// several cases declare by a `when` over them whose `else` is never taken
    /// and is written only where a case is not left, and no read where no case
    /// left declares the field.
    #[test]
    fn an_algorithms_union_is_read_on_its_units_sealed_interface() {
        let shapes = shapes();
        let union = &shapes.union;
        let case = |name: &str| {
            union
                .cases
                .iter()
                .find(|c| c.variant.name == name)
                .expect("a case")
        };
        let read = |left: &[&ScopedCase], member: &str| {
            kotlin_unit_union_read("shape", "Shape", union, left, member)
        };
        assert_eq!(read(&[], "kind"), Ok("shape.kind".to_string()));
        assert_eq!(
            read(&[case("circle")], "radius"),
            Ok("shape.provenCircle().radius".to_string())
        );
        assert_eq!(
            read(&[case("circle"), case("disc")], "radius"),
            Ok(
                "(when (shape) { is Shape.Circle -> shape.value.radius; is Shape.Disc -> \
                 shape.value.radius; else -> throw IllegalStateException(\"a value of shape \
                 read as one of its cases circle, disc is in another case\") })"
                    .to_string()
            )
        );
        // Where every case of the union is left, each is an arm and there is
        // no other.
        let mut both = union.clone();
        both.cases.retain(|c| c.variant.name != "dot");
        let every: Vec<&ScopedCase> = both.cases.iter().collect();
        assert_eq!(
            kotlin_unit_union_read("shape", "Shape", &both, &every, "radius"),
            Ok(
                "(when (shape) { is Shape.Circle -> shape.value.radius; is Shape.Disc -> \
                 shape.value.radius })"
                    .to_string()
            )
        );
        assert!(read(&[], "radius").is_err());
        assert!(read(&[case("dot"), case("circle")], "radius").is_err());
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
