// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A composed value of a statechart (docs/adr/0086), lowered for one backend by
//! one walk of its type, one case per constructor:
//!
//! * the type the machine holds a value in (section 7.1, the machine column),
//!   [`machine_type`];
//! * the records it names, each declared once, an inner one before the outer
//!   one that names it, [`held_records`];
//! * the reader that lifts it out of the event's data, one runtime call per
//!   constructor, which refuses a value outside its type before it keeps any of
//!   it and names the path to the place (section 4), [`reader`];
//! * the writer that spells it as the JSON the wire carries (section 3.1),
//!   [`writer`].
//!
//! The walks are here, once; a backend supplies only the spelling of each case
//! ([`ComposedSpelling`]), so a composition of any depth is a composition of
//! the cases on every backend that spells them, and a backend that spells none
//! lowers none ([`crate::forge::event_schema_check::ComposedLowering`]).
//!
//! A record is held as the statechart holds its schema: by the alias the
//! statechart imports it under, its enums named as the statechart names them
//! ([`crate::forge::event_schema_check::name_schema_enums`]). A record a field
//! names in the scope of its own document is found by its document
//! ([`RecordEnv`]), so one record is one type of the machine whichever document
//! names it.
//!
//! | Backend | `list<T>[N]` | `string[N]` / `bytes[N]` | `optional<T>` | record |
//! |---|---|---|---|---|
//! | Rust (machine) | `BoundedList<T, N>` | `SceString<N>` / `SceBytes<N>` | `Option<T>` | the machine's record |
//!
//! A list, a text and a byte string carry their bound in their type (section
//! 7.4), so a value is copied into a place of a larger bound by widening it at
//! each depth where the two bounds differ, the fit proven before ([`copier`],
//! section 5.1) and asserted again by the widening's own const parameters.

use std::collections::BTreeMap;

use crate::forge::model::{EventSchemaModel, SceType};
use crate::forge::value_type::{TypeScope, ValueType};

/// An enum as a machine holds it: the name the statechart holds it by, the
/// type it is declared as, and each variant's declared name with the
/// identifier the type gives it, in the enum's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineEnum {
    pub alias: String,
    pub ty: String,
    pub variants: Vec<(String, String)>,
}

/// A record a machine holds: the alias the statechart imports its schema
/// under, and the schema as the statechart holds it.
#[derive(Debug, Clone, Copy)]
pub struct HeldRecord<'m> {
    pub alias: &'m str,
    pub schema: &'m EventSchemaModel,
}

/// Where the records a composed value names are found.
pub trait RecordEnv {
    /// The record `alias` names, written in the statechart's own scope when
    /// `within` is `None`, and in the document of the record the statechart
    /// holds as `within` otherwise. `Err` is the sentence of a record the
    /// machine does not hold.
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String>;
}

/// The records of a statechart: the schemas it imports, by alias, and the
/// scope each document names its records in. Held apart from the model, so a
/// walk that rewrites the model's states reads them.
#[derive(Debug, Clone)]
pub struct StatechartRecords {
    scope: TypeScope,
    records: BTreeMap<String, EventSchemaModel>,
}

impl StatechartRecords {
    /// The records `model` holds.
    pub fn of(model: &crate::model::SCXMLModel) -> Self {
        Self {
            scope: crate::forge::value_path::statechart_scope(model),
            records: model.imported_records.clone(),
        }
    }
}

impl RecordEnv for StatechartRecords {
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
        let unheld = || {
            format!("`record:{alias}` names a record whose document the statechart does not import")
        };
        // A record named inside another is named by that record's document,
        // and held by the statechart under the alias it imports the same
        // document by.
        let held: &str = match within {
            None => alias,
            Some(record) => {
                let identity = &self
                    .scope
                    .records
                    .get(record)
                    .and_then(|held| held.scope.records.get(alias))
                    .ok_or_else(unheld)?
                    .identity;
                self.scope
                    .records
                    .iter()
                    .find(|(_, held)| held.identity == *identity)
                    .map(|(alias, _)| alias.as_str())
                    .ok_or_else(unheld)?
            }
        };
        let (alias, schema) = self.records.get_key_value(held).ok_or_else(unheld)?;
        Ok(HeldRecord { alias, schema })
    }
}

/// The record whose document the aliases of a place's type are written in, as
/// the statechart holds it, for a place reached through the records `chain`
/// names, each alias in the scope of the one before it
/// ([`crate::forge::value_path::Place::scope`]); `None` for a place in the
/// statechart's own scope.
pub fn within_of(chain: &[String], records: &dyn RecordEnv) -> Result<Option<String>, String> {
    let mut within: Option<String> = None;
    for alias in chain {
        let held = records.record(within.as_deref(), alias)?.alias.to_string();
        within = Some(held);
    }
    Ok(within)
}

/// A value a writer spells: an expression that is the value (a place), or one
/// that is a reference to it (what a closure over the elements of a list is
/// handed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub text: String,
    pub by_reference: bool,
}

impl Held {
    /// The value at the place `text`.
    pub fn place(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            by_reference: false,
        }
    }

    /// The value `text` is a reference to.
    pub fn reference(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            by_reference: true,
        }
    }

    /// A reference to the value: `text` itself where it is one, and a borrow of
    /// the place otherwise.
    pub fn borrowed(&self) -> String {
        if self.by_reference {
            self.text.clone()
        } else {
            format!("&{}", self.text)
        }
    }
}

/// How one backend spells each case of the walks. Nothing here decides what a
/// value is or which cases there are: the walks do, once.
pub trait ComposedSpelling {
    /// The type a value of the scalar `ty` (a fixed-width number or `bool`, no
    /// enum) is held in.
    fn scalar_type(&self, ty: &SceType) -> String;
    /// The type a text of at most `max` bytes is held in.
    fn text_type(&self, max: u32) -> String;
    /// The type a byte string of at most `max` bytes is held in.
    fn bytes_type(&self, max: u32) -> String;
    /// The type a value that may be absent is held in, its present value held
    /// in `elem`.
    fn optional_type(&self, elem: &str) -> String;
    /// The type a list of at most `max` values held in `elem` is held in.
    fn list_type(&self, elem: &str, max: u32) -> String;
    /// The type a record of the schema the statechart imports as `alias` is
    /// held in.
    fn record_type(&self, alias: &str) -> String;
    /// The declaration of the record type `ty` of the schema imported as
    /// `alias`, one member per field, `(id, type)`, in the schema's order.
    fn record_def(&self, ty: &str, alias: &str, fields: &[(String, String)]) -> String;

    /// The name a reader binds the value of an element, a present value or a
    /// field to, which the reader of that part reads.
    fn read_binding(&self) -> &'static str;
    /// The reader of the scalar `ty` (no enum) at the value `value`.
    fn read_scalar(&self, ty: &SceType, value: &str) -> String;
    /// The reader of a value of the enum `held`, by the variant's declared
    /// name, a name the enum does not declare refused.
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String;
    /// The reader of a text of at most `max` bytes.
    fn read_text(&self, max: u32, value: &str) -> String;
    /// The reader of a byte string of at most `max` bytes.
    fn read_bytes(&self, max: u32, value: &str) -> String;
    /// The reader of a value that may be absent, `elem` the reader of its
    /// present value bound to [`Self::read_binding`].
    fn read_optional(&self, value: &str, elem: &str) -> String;
    /// The reader of a list of at least `least` and at most `max` elements,
    /// `elem` the reader of an element bound to [`Self::read_binding`]; one past
    /// the bound, or short of the lower count, is refused before any element is
    /// read (docs/adr/0086, section 15.5). `least` is 0 for a list that states
    /// no lower count.
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str) -> String;
    /// The reader of a record held in `ty`: `defaults` the JSON of the fields a
    /// later revision of its schema added, `fields` each field's id with the
    /// reader of its value bound to [`Self::read_binding`].
    fn read_record(
        &self,
        value: &str,
        defaults: Option<&str>,
        ty: &str,
        fields: &[(String, String)],
    ) -> String;

    /// The name a writer binds a reference to an element or a present value to.
    fn write_binding(&self) -> &'static str;
    /// The JSON of the scalar `ty` (no enum).
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String;
    /// The JSON of a value of an enum: the declared name of its variant.
    fn write_enum(&self, held: &Held) -> String;
    /// The JSON of a text.
    fn write_text(&self, held: &Held) -> String;
    /// The JSON of a byte string.
    fn write_bytes(&self, held: &Held) -> String;
    /// The JSON of a value that may be absent, `elem` the writer of its present
    /// value, a reference bound to [`Self::write_binding`].
    fn write_optional(&self, held: &Held, elem: &str) -> String;
    /// The JSON of a list, `elem` the writer of an element, a reference bound to
    /// [`Self::write_binding`].
    fn write_list(&self, held: &Held, elem: &str) -> String;
    /// The place of the field `field` of the record `held`.
    fn write_field(&self, held: &Held, field: &str) -> String;
    /// The JSON of a record: each field's id with the JSON of its value.
    fn write_record(&self, fields: &[(String, String)]) -> String;

    /// The name a copy binds a reference to an element or a present value to.
    fn copy_binding(&self) -> &'static str;
    /// A copy of `held`, a value of the very type of the place it is copied
    /// into; `plain` for a value the language copies as it reads it (a number,
    /// `bool`, an enum, an optional one of them).
    fn copy_whole(&self, held: &Held, plain: bool) -> String;
    /// A copy of the text `held` into a place of `max` bytes, at least its own.
    fn copy_text(&self, max: u32, held: &Held) -> String;
    /// A copy of the byte string `held` into a place of `max` bytes, at least
    /// its own.
    fn copy_bytes(&self, max: u32, held: &Held) -> String;
    /// A copy of the value that may be absent `held`, `elem` the copy of its
    /// present value, a reference bound to [`Self::copy_binding`].
    fn copy_optional(&self, held: &Held, elem: &str) -> String;
    /// `value`, a copy of a value of `T`, as the present value of a place of
    /// `optional<T>`.
    fn copy_present(&self, value: &str) -> String;
    /// A copy of the list `held`, `elem` the copy of an element, a reference
    /// bound to [`Self::copy_binding`].
    fn copy_list(&self, held: &Held, elem: &str) -> String;
}

/// Whether a value of `ty` is copied as it is read, which a copy says
/// ([`ComposedSpelling::copy_whole`]).
fn plain(ty: &ValueType) -> bool {
    match ty {
        ValueType::Scalar(_) => true,
        ValueType::Optional { elem } => matches!(**elem, ValueType::Scalar(_)),
        ValueType::Text { .. }
        | ValueType::Bytes { .. }
        | ValueType::Record { .. }
        | ValueType::Union { .. }
        | ValueType::List { .. } => false,
    }
}

/// The copy of `held`, a value of `from` written in the scope `from_within`
/// names, into a place of `to` written in the scope `to_within` names, `from`
/// fitting `to` (docs/adr/0086, section 5.1, proven where the document was
/// read): the value as it is where the machine holds the two in one type, and
/// otherwise each part copied by the case of its constructor, a text into the
/// larger bound of the place. A copy that cannot fail, since the fit was
/// proven. `Err` names a pair of types the walk has no copy of.
#[allow(clippy::too_many_arguments)]
pub fn copier(
    from: &ValueType,
    from_within: Option<&str>,
    to: &ValueType,
    to_within: Option<&str>,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
    held: &Held,
) -> Result<String, String> {
    let same = machine_type(from, from_within, records, enums, spelling)?
        == machine_type(to, to_within, records, enums, spelling)?;
    if same {
        return Ok(spelling.copy_whole(held, plain(from)));
    }
    let element = Held::reference(spelling.copy_binding());
    let part = |from: &ValueType, to: &ValueType, held: &Held| {
        copier(
            from,
            from_within,
            to,
            to_within,
            records,
            enums,
            spelling,
            held,
        )
    };
    match (from, to) {
        (ValueType::Text { .. }, ValueType::Text { max }) => Ok(spelling.copy_text(max.max, held)),
        (ValueType::Bytes { .. }, ValueType::Bytes { max }) => {
            Ok(spelling.copy_bytes(max.max, held))
        }
        (ValueType::Optional { elem: from }, ValueType::Optional { elem: to }) => {
            Ok(spelling.copy_optional(held, &part(from, to, &element)?))
        }
        (from, ValueType::Optional { elem: to }) => {
            Ok(spelling.copy_present(&part(from, to, held)?))
        }
        (ValueType::List { elem: from, .. }, ValueType::List { elem: to, .. }) => {
            Ok(spelling.copy_list(held, &part(from, to, &element)?))
        }
        (from, to) => Err(format!(
            "`{from}` copied into a place of `{to}` has no copy that changes its type yet"
        )),
    }
}

/// The enum the machine holds as `alias`, or the sentence of one it does not.
fn machine_enum<'e>(
    enums: &'e BTreeMap<String, MachineEnum>,
    alias: &str,
) -> Result<&'e MachineEnum, String> {
    enums
        .get(alias)
        .ok_or_else(|| format!("`enum:{alias}` names an enum the machine holds no type for"))
}

/// The type the machine holds a value of `ty` in, written in the scope
/// `within` names ([`RecordEnv::record`]).
pub fn machine_type(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
) -> Result<String, String> {
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(reference)) => {
            machine_enum(enums, &reference.alias)?.ty.clone()
        }
        ValueType::Scalar(scalar) => spelling.scalar_type(scalar),
        ValueType::Text { max } => spelling.text_type(max.max),
        ValueType::Bytes { max } => spelling.bytes_type(max.max),
        ValueType::Optional { elem } => {
            spelling.optional_type(&machine_type(elem, within, records, enums, spelling)?)
        }
        ValueType::List { elem, max } => spelling.list_type(
            &machine_type(elem, within, records, enums, spelling)?,
            max.max,
        ),
        ValueType::Record { alias } => spelling.record_type(records.record(within, alias)?.alias),
        // docs/adr/0092, landing order, step 4: the backend generates the
        // union's unit, and no machine holds one yet; every walk below refuses
        // it in the same words.
        ValueType::Union { alias } => {
            return Err(crate::forge::value_type::union_unlowered_in_machine(alias))
        }
    })
}

/// What a value of a type names from outside it, at any depth: the records,
/// each once, an inner one before the outer one that names it, and the enums.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reached {
    /// The statechart's alias of each record.
    pub records: Vec<String>,
    /// The statechart's name of each enum.
    pub enums: std::collections::BTreeSet<String>,
}

/// Add to `reached` what a value of `ty`, written in the scope `within` names,
/// names at any depth.
pub fn held_records(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
    reached: &mut Reached,
) -> Result<(), String> {
    match ty {
        ValueType::Scalar(SceType::Enum(reference)) => {
            reached.enums.insert(reference.alias.clone());
        }
        ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => {}
        ValueType::Optional { elem } | ValueType::List { elem, .. } => {
            held_records(elem, within, records, reached)?;
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            if reached.records.iter().any(|r| r == held.alias) {
                return Ok(());
            }
            for field in &held.schema.fields {
                held_records(&field.sce_type, Some(held.alias), records, reached)?;
            }
            // A record that holds itself is refused where its document is read
            // (docs/adr/0086, section 1.5), so the walk above ends; it is added
            // after its fields, so an inner record is declared first.
            if !reached.records.iter().any(|r| r == held.alias) {
                reached.records.push(held.alias.to_string());
            }
        }
        ValueType::Union { alias } => {
            return Err(crate::forge::value_type::union_unlowered_in_machine(alias))
        }
    }
    Ok(())
}

/// The members of the record the statechart holds as `alias`, each field's id
/// with the type the machine holds it in, in the schema's order.
pub fn record_members(
    alias: &str,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
) -> Result<Vec<(String, String)>, String> {
    let held = records.record(None, alias)?;
    held.schema
        .fields
        .iter()
        .map(|field| {
            Ok((
                field.id.clone(),
                machine_type(&field.sce_type, Some(held.alias), records, enums, spelling)?,
            ))
        })
        .collect()
}

/// The reader of a value of `ty` at `value`, written in the scope `within`
/// names: one call of the runtime per constructor, the reader of each part
/// inside the call of the part that holds it.
pub fn reader(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
    value: &str,
) -> Result<String, String> {
    let bound = spelling.read_binding();
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(reference)) => {
            spelling.read_enum(machine_enum(enums, &reference.alias)?, value)
        }
        ValueType::Scalar(scalar) => spelling.read_scalar(scalar, value),
        ValueType::Text { max } => spelling.read_text(max.max, value),
        ValueType::Bytes { max } => spelling.read_bytes(max.max, value),
        ValueType::Optional { elem } => spelling.read_optional(
            value,
            &reader(elem, within, records, enums, spelling, bound)?,
        ),
        ValueType::List { elem, max } => spelling.read_list(
            value,
            max.min,
            max.max,
            &reader(elem, within, records, enums, spelling, bound)?,
        ),
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let fields = field_readers(held, records, enums, spelling)?;
            spelling.read_record(
                value,
                crate::forge::schema_revision::composed_defaults_wire(held.schema).as_deref(),
                &spelling.record_type(held.alias),
                &fields,
            )
        }
        ValueType::Union { alias } => {
            return Err(crate::forge::value_type::union_unlowered_in_machine(alias))
        }
    })
}

/// Each field of the record `held` with the reader of its value bound to the
/// spelling's [`ComposedSpelling::read_binding`]: what a record's reader reads,
/// and what an event's whole payload, a record of its schema, is lifted by.
pub fn field_readers(
    held: HeldRecord<'_>,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
) -> Result<Vec<(String, String)>, String> {
    held.schema
        .fields
        .iter()
        .map(|field| {
            Ok((
                field.id.clone(),
                reader(
                    &field.sce_type,
                    Some(held.alias),
                    records,
                    enums,
                    spelling,
                    spelling.read_binding(),
                )?,
            ))
        })
        .collect()
}

/// The JSON a value of `ty` held at `held` is spelled as, written in the scope
/// `within` names: one call of the runtime's writers per constructor.
pub fn writer(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
    spelling: &dyn ComposedSpelling,
    held: &Held,
) -> Result<String, String> {
    let bound = Held::reference(spelling.write_binding());
    Ok(match ty {
        ValueType::Scalar(SceType::Enum(_)) => spelling.write_enum(held),
        ValueType::Scalar(scalar) => spelling.write_scalar(scalar, held),
        ValueType::Text { .. } => spelling.write_text(held),
        ValueType::Bytes { .. } => spelling.write_bytes(held),
        ValueType::Optional { elem } => {
            spelling.write_optional(held, &writer(elem, within, records, spelling, &bound)?)
        }
        ValueType::List { elem, .. } => {
            spelling.write_list(held, &writer(elem, within, records, spelling, &bound)?)
        }
        ValueType::Record { alias } => {
            let record = records.record(within, alias)?;
            spelling.write_record(&field_writers(record, records, spelling, held)?)
        }
        ValueType::Union { alias } => {
            return Err(crate::forge::value_type::union_unlowered_in_machine(alias))
        }
    })
}

/// Each field of the record `record`, held at `held`, with the JSON of its
/// value: what a record's writer writes, and what an event's whole payload is
/// written as.
pub fn field_writers(
    record: HeldRecord<'_>,
    records: &dyn RecordEnv,
    spelling: &dyn ComposedSpelling,
    held: &Held,
) -> Result<Vec<(String, String)>, String> {
    record
        .schema
        .fields
        .iter()
        .map(|field| {
            let place = Held::place(spelling.write_field(held, &field.id));
            Ok((
                field.id.clone(),
                writer(
                    &field.sce_type,
                    Some(record.alias),
                    records,
                    spelling,
                    &place,
                )?,
            ))
        })
        .collect()
}

/// The Rust spellings of the walks, for the machine `machine` (section 7.1,
/// the machine column): the runtime's `PayloadValue` reads each constructor,
/// and its `list_json`, `optional_json` and `record_json` write them.
pub struct RustComposed<'a> {
    pub machine: &'a str,
}

/// The runtime module the Rust readers and writers are in.
const RUST_PAYLOAD: &str = "::sce_rust_runtime::event_payload";

/// The crate of the bounded storage a Rust machine holds a list, a text and a
/// byte string in.
///
/// Named through the runtime every machine already depends on
/// (`sce_rust_runtime::portable`, the crate re-exported whole), so a crate that
/// holds a machine needs no dependency of its own on `sce-portable-bytes`. A
/// schema unit names the same types as `::sce_portable_bytes`, a crate a forge
/// unit depends on directly; the two paths are one type.
pub(crate) const RUST_PORTABLE: &str = "::sce_rust_runtime::portable";

/// The type a Rust machine holds a list of at most `max` values held in `elem`
/// in, every place alike (docs/adr/0086, section 7.4): a variable, a record's
/// field and a payload's, so two places of one list type are one Rust type.
pub(crate) fn rust_bounded_list(elem: &str, max: u32) -> String {
    format!("{RUST_PORTABLE}::BoundedList<{elem}, {max}>")
}

/// The type a Rust machine holds a text of at most `max` bytes in, every place
/// alike (docs/adr/0086, section 7.4).
pub(crate) fn rust_text_type(max: u32) -> String {
    format!("{RUST_PORTABLE}::SceString<{max}>")
}

/// The type a Rust machine holds a byte string of at most `max` bytes in,
/// every place alike (docs/adr/0086, section 7.4).
pub(crate) fn rust_bytes_type(max: u32) -> String {
    format!("{RUST_PORTABLE}::SceBytes<{max}>")
}

/// The empty list a Rust machine starts a list variable at, its element and
/// bound taken from the place ([`rust_bounded_list`]).
pub(crate) const RUST_EMPTY_LIST: &str = "::sce_rust_runtime::portable::BoundedList::new()";

impl ComposedSpelling for RustComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::rust_type(ty).to_string()
    }
    // The portable bounded storage, `N` on the type (docs/adr/0086, section 7.1).
    fn text_type(&self, max: u32) -> String {
        rust_text_type(max)
    }
    fn bytes_type(&self, max: u32) -> String {
        rust_bytes_type(max)
    }
    fn optional_type(&self, elem: &str) -> String {
        format!("Option<{elem}>")
    }
    fn list_type(&self, elem: &str, max: u32) -> String {
        rust_bounded_list(elem, max)
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
    // Owned and compared whole, as a record of entries that owns a buffer is;
    // no default, since an enum a field holds has none.
    fn record_def(&self, ty: &str, alias: &str, fields: &[(String, String)]) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    pub {id}: {ty},\n"))
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed \
             value (docs/adr/0086).\n#[derive(Clone, Debug, PartialEq)]\n\
             #[allow(non_snake_case)]\npub struct {ty} {{\n{members}}}"
        )
    }

    fn read_binding(&self) -> &'static str {
        "v"
    }
    fn read_scalar(&self, ty: &SceType, value: &str) -> String {
        match ty {
            SceType::Uint64 => format!("{value}.unsigned64()"),
            SceType::Int64 => format!("{value}.signed64()"),
            SceType::Uint8 | SceType::Uint16 | SceType::Uint32 => format!(
                "{value}.unsigned::<{}>()",
                crate::forge::generator::rust_type(ty)
            ),
            SceType::Int8 | SceType::Int16 | SceType::Int32 => format!(
                "{value}.signed::<{}>()",
                crate::forge::generator::rust_type(ty)
            ),
            SceType::Float32 => format!("{value}.float32()"),
            SceType::Float64 => format!("{value}.float64()"),
            SceType::Bool => format!("{value}.truth()"),
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        }
    }
    fn read_enum(&self, held: &MachineEnum, value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "\"{}\" => Ok({}::{ident}), ",
                    crate::filters::escape_rust(declared.clone()),
                    held.ty
                )
            })
            .collect();
        format!(
            "{value}.name().and_then(|name| match name {{ {arms}other => \
             Err({value}.unknown_variant(other, \"{}\")) }})",
            crate::filters::escape_rust(held.alias.clone())
        )
    }
    // The runtime's reader answers the portable `SceString<N>` itself, the one
    // text type (docs/adr/0086, section 7.4).
    fn read_text(&self, max: u32, value: &str) -> String {
        format!("{value}.text::<{max}>()")
    }
    // The runtime's reader answers the portable `SceBytes<N>` itself.
    fn read_bytes(&self, max: u32, value: &str) -> String {
        format!("{value}.bytes::<{max}>()")
    }
    fn read_optional(&self, value: &str, elem: &str) -> String {
        format!("{value}.optional(|v| {elem})")
    }
    // The runtime's reader answers the `BoundedList<T, N>` the machine holds;
    // a lower count is held by the reader too, since no Rust type holds it
    // (docs/adr/0086, section 15.6).
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str) -> String {
        if least == 0 {
            format!("{value}.list::<_, {max}>(|v| {elem})")
        } else {
            format!("{value}.list_within::<_, {max}>({least}, |v| {elem})")
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
            || "None".to_string(),
            |json| {
                format!(
                    "Some(\"{}\")",
                    crate::filters::escape_rust(json.to_string())
                )
            },
        );
        let members: Vec<String> = fields
            .iter()
            .map(|(id, read)| format!("{id}: r.field(\"{id}\").and_then(|v| {read})?"))
            .collect();
        format!(
            "{value}.record({defaults}, |r| Ok({ty} {{ {} }}))",
            members.join(", ")
        )
    }

    fn write_binding(&self) -> &'static str {
        "h"
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        match ty {
            // A 64-bit integer crosses as the text of its digits.
            SceType::Uint64 | SceType::Int64 => {
                format!("{RUST_PAYLOAD}::quote(&{}.to_string())", held.text)
            }
            // A number other than a 64-bit integer, and a truth value, as
            // written.
            _ => format!("{}.to_string()", held.text),
        }
    }
    // The machine's enum answers the name its document declares
    // (`StaticTarget::enum_def`'s `sce_name`).
    fn write_enum(&self, held: &Held) -> String {
        format!("{RUST_PAYLOAD}::quote({}.sce_name())", held.text)
    }
    fn write_text(&self, held: &Held) -> String {
        format!("{RUST_PAYLOAD}::quote({})", held.borrowed())
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        format!(
            "{RUST_PAYLOAD}::quote(&{RUST_PAYLOAD}::bytes_as_payload_text({}))",
            held.borrowed()
        )
    }
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        format!(
            "{RUST_PAYLOAD}::optional_json({}.as_ref(), |h| {elem})",
            held.text
        )
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        format!("{RUST_PAYLOAD}::list_json({}, |h| {elem})", held.borrowed())
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, json)| format!("(\"{id}\", {json})"))
            .collect();
        format!("{RUST_PAYLOAD}::record_json(vec![{}])", members.join(", "))
    }

    fn copy_binding(&self) -> &'static str {
        "v"
    }
    fn copy_whole(&self, held: &Held, plain: bool) -> String {
        match (plain, held.by_reference) {
            (true, true) => format!("*{}", held.text),
            (true, false) => held.text.clone(),
            (false, _) => format!("{}.clone()", held.text),
        }
    }
    // The fit was proven, and the widening asserts it again by its const
    // parameters, so a copy the build did not prove does not compile.
    fn copy_text(&self, max: u32, held: &Held) -> String {
        format!(
            "{RUST_PORTABLE}::SceString::<{max}>::widened({})",
            held.borrowed()
        )
    }
    fn copy_bytes(&self, max: u32, held: &Held) -> String {
        format!(
            "{RUST_PORTABLE}::SceBytes::<{max}>::widened({})",
            held.borrowed()
        )
    }
    fn copy_optional(&self, held: &Held, elem: &str) -> String {
        format!("{}.as_ref().map(|v| {elem})", held.text)
    }
    fn copy_present(&self, value: &str) -> String {
        format!("Some({value})")
    }
    // Each element copied into the place's element type, the list's own bound
    // widened into the place's by type; the place's type names both.
    fn copy_list(&self, held: &Held, elem: &str) -> String {
        format!(
            "{RUST_PORTABLE}::BoundedList::widened_map({}, |v| {elem})",
            held.borrowed()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An event schema read from `text` as the parser reads it.
    fn schema(text: &str) -> EventSchemaModel {
        match crate::forge::parser::parse_forge_with_imports(
            text,
            crate::DocumentLabel::symmetric("probe"),
        )
        .expect("the schema parses")
        .expect("a forge document")
        .document
        {
            crate::forge::model::ForgeDocument::EventSchema(schema) => schema,
            _ => panic!("an event schema"),
        }
    }

    const ITEM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="item" sce:event-name="item.sent">
  <sce:import kind="enum" src="level.scxml" as="Level"/>
  <datamodel>
    <data id="id" sce:type="uint32" sce:direction="in"/>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
    <data id="due" sce:type="optional&lt;int64&gt;" sce:direction="in"/>
    <data id="blob" sce:type="bytes[4]" sce:direction="in"/>
    <data id="tags" sce:type="list&lt;string[8]&gt;[4]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

    const BATCH: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="batch" sce:event-name="batch.sent" sce:revision="2">
  <sce:import kind="event-schema" src="item.scxml" as="Item"/>
  <datamodel>
    <data id="items" sce:type="list&lt;record:Item&gt;[3]" sce:direction="in"/>
    <data id="grid" sce:type="list&lt;list&lt;uint8&gt;[2]&gt;[2]" sce:direction="in"/>
    <data id="late" sce:type="list&lt;uint8&gt;[2]" sce:direction="in" sce:since="2"/>
  </datamodel>
</scxml>
"#;

    /// A statechart that imports `batch` as `Batch` and `item` as `Item`, and
    /// whose `batch` names `item` as `Item` too.
    struct TwoRecords {
        batch: EventSchemaModel,
        item: EventSchemaModel,
    }

    impl RecordEnv for TwoRecords {
        fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
            match (within, alias) {
                (None | Some("Batch"), "Item") => Ok(HeldRecord {
                    alias: "Item",
                    schema: &self.item,
                }),
                (None, "Batch") => Ok(HeldRecord {
                    alias: "Batch",
                    schema: &self.batch,
                }),
                _ => Err(format!("no record `{alias}`")),
            }
        }
    }

    fn held() -> TwoRecords {
        TwoRecords {
            batch: schema(BATCH),
            item: schema(ITEM),
        }
    }

    fn enums() -> BTreeMap<String, MachineEnum> {
        [(
            "Level".to_string(),
            MachineEnum {
                alias: "Level".to_string(),
                ty: "MLevelEnum".to_string(),
                variants: vec![
                    ("low".to_string(), "Low".to_string()),
                    ("high".to_string(), "High".to_string()),
                ],
            },
        )]
        .into()
    }

    const RUST: RustComposed<'static> = RustComposed { machine: "M" };

    fn ty(text: &str) -> ValueType {
        crate::forge::value_type::WrittenType::read(text)
            .expect("a type")
            .bounded()
            .expect("every bound stated")
    }

    #[test]
    fn the_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &RUST).expect("a type");
        assert_eq!(spelled("uint16"), "u16");
        assert_eq!(
            spelled("string[8]"),
            "::sce_rust_runtime::portable::SceString<8>"
        );
        assert_eq!(
            spelled("bytes[4]"),
            "::sce_rust_runtime::portable::SceBytes<4>"
        );
        assert_eq!(spelled("optional<int64>"), "Option<i64>");
        let list = "::sce_rust_runtime::portable::BoundedList";
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            format!("{list}<{list}<u8, 2>, 3>")
        );
        assert_eq!(
            spelled("list<enum:Level>[2]"),
            format!("{list}<MLevelEnum, 2>")
        );
        assert_eq!(
            spelled("list<record:Item>[3]"),
            format!("{list}<MItemRecord, 3>")
        );
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            format!("Option<{list}<u8, 2>>")
        );
        assert!(machine_type(&ty("enum:Other"), None, &held, &enums, &RUST).is_err());
    }

    /// Every walk refuses a union at any depth in the machine's own words
    /// (docs/adr/0092, landing order, step 4): the backend that spells the
    /// walks generates the union's unit, so no refusal says it lowers none.
    #[test]
    fn every_walk_refuses_a_union_as_a_machine_that_holds_none_yet() {
        let held = held();
        let enums = enums();
        let refused: Result<String, String> = Err(
            crate::forge::value_type::union_unlowered_in_machine("Shape"),
        );
        for text in [
            "union:Shape",
            "optional<union:Shape>",
            "list<union:Shape>[2]",
        ] {
            assert_eq!(
                machine_type(&ty(text), None, &held, &enums, &RUST),
                refused,
                "{text}"
            );
            assert_eq!(
                held_records(&ty(text), None, &held, &mut Reached::default()),
                refused.clone().map(|_: String| ()),
                "{text}"
            );
            assert_eq!(
                reader(&ty(text), None, &held, &enums, &RUST, "v"),
                refused,
                "{text}"
            );
            assert_eq!(
                writer(&ty(text), None, &held, &RUST, &Held::place("x")),
                refused,
                "{text}"
            );
        }
    }

    #[test]
    fn each_record_is_reached_once_an_inner_one_first() {
        let held = held();
        let mut reached = Reached::default();
        held_records(&ty("record:Batch"), None, &held, &mut reached).expect("reached");
        held_records(&ty("list<record:Item>[2]"), None, &held, &mut reached).expect("reached");
        assert_eq!(
            reached.records,
            vec!["Item".to_string(), "Batch".to_string()]
        );
        assert_eq!(reached.enums.into_iter().collect::<Vec<_>>(), vec!["Level"]);
    }

    #[test]
    fn a_record_is_declared_with_each_field_in_its_machine_type() {
        let held = held();
        let members = record_members("Item", &held, &enums(), &RUST).expect("members");
        assert_eq!(
            members,
            vec![
                ("id".to_string(), "u32".to_string()),
                ("level".to_string(), "MLevelEnum".to_string()),
                ("due".to_string(), "Option<i64>".to_string()),
                (
                    "blob".to_string(),
                    "::sce_rust_runtime::portable::SceBytes<4>".to_string()
                ),
                (
                    "tags".to_string(),
                    "::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, 4>"
                        .to_string()
                ),
            ]
        );
        let def = RUST.record_def("MItemRecord", "Item", &members);
        assert!(
            def.contains("pub struct MItemRecord {\n    pub id: u32,\n"),
            "{def}"
        );
        assert!(
            def.contains(
                "    pub tags: ::sce_rust_runtime::portable::BoundedList<::sce_rust_runtime::portable::SceString<8>, \
                 4>,\n}"
            ),
            "{def}"
        );
    }

    #[test]
    fn a_reader_is_one_runtime_call_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &RUST, "v").expect("read");
        assert_eq!(read("uint8"), "v.unsigned::<u8>()");
        assert_eq!(read("int32"), "v.signed::<i32>()");
        assert_eq!(read("uint64"), "v.unsigned64()");
        assert_eq!(read("int64"), "v.signed64()");
        assert_eq!(read("float32"), "v.float32()");
        assert_eq!(read("bool"), "v.truth()");
        assert_eq!(read("string[8]"), "v.text::<8>()");
        assert_eq!(read("bytes[4]"), "v.bytes::<4>()");
        assert_eq!(read("optional<int64>"), "v.optional(|v| v.signed64())");
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            "v.list::<_, 3>(|v| v.list::<_, 2>(|v| v.unsigned::<u8>()))"
        );
        // A lower count is handed to the reader, at any depth (docs/adr/0086,
        // section 15.5); the machine's type is the one its bound gives.
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            "v.list_within::<_, 3>(2, |v| v.list_within::<_, 2>(1, |v| v.unsigned::<u8>()))"
        );
        assert_eq!(
            machine_type(&ty("list<uint8>[2..3]"), None, &held, &enums, &RUST),
            machine_type(&ty("list<uint8>[3]"), None, &held, &enums, &RUST)
        );
        assert_eq!(
            read("enum:Level"),
            "v.name().and_then(|name| match name { \"low\" => Ok(MLevelEnum::Low), \"high\" => \
             Ok(MLevelEnum::High), other => Err(v.unknown_variant(other, \"Level\")) })"
        );
    }

    #[test]
    fn a_record_is_read_field_by_field_with_its_defaults() {
        let held = held();
        let enums = enums();
        let item = reader(&ty("record:Item"), None, &held, &enums, &RUST, "v").expect("read");
        assert!(
            item.starts_with("v.record(None, |r| Ok(MItemRecord { id: r.field(\"id\").and_then(|v| v.unsigned::<u32>())?, "),
            "{item}"
        );
        assert!(
            item.ends_with(
                "tags: r.field(\"tags\").and_then(|v| v.list::<_, 4>(|v| v.text::<8>()))? }))"
            ),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = reader(&ty("record:Batch"), None, &held, &enums, &RUST, "v").expect("read");
        assert!(
            batch.starts_with("v.record(Some(\"{\\\"late\\\":[]}\"), |r| Ok(MBatchRecord {"),
            "{batch}"
        );
    }

    #[test]
    fn a_value_that_fits_is_copied_by_the_case_of_each_constructor() {
        let held = held();
        let enums = enums();
        let copy = |from: &str, to: &str, at: Held| {
            copier(&ty(from), None, &ty(to), None, &held, &enums, &RUST, &at)
        };
        // One type in the machine: the value as it is.
        assert_eq!(
            copy("list<uint8>[4]", "list<uint8>[4]", Held::place("x")),
            Ok("x.clone()".to_string())
        );
        // A list of a smaller bound is widened by type, each element copied.
        assert_eq!(
            copy("list<uint8>[2]", "list<uint8>[4]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::BoundedList::widened_map(&x, |v| *v)".to_string())
        );
        assert_eq!(
            copy(
                "list<record:Item>[3]",
                "list<record:Item>[3]",
                Held::place("x")
            ),
            Ok("x.clone()".to_string())
        );
        assert_eq!(
            copy("optional<int64>", "optional<int64>", Held::reference("v")),
            Ok("*v".to_string())
        );
        // A text of a smaller bound, at depth 1 and 2.
        assert_eq!(
            copy("string[4]", "string[8]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::SceString::<8>::widened(&x)".to_string())
        );
        assert_eq!(
            copy("list<string[4]>[2]", "list<string[8]>[3]", Held::place("x")),
            Ok(
                "::sce_rust_runtime::portable::BoundedList::widened_map(&x, |v| \
                 ::sce_rust_runtime::portable::SceString::<8>::widened(v))"
                    .to_string()
            )
        );
        assert_eq!(
            copy("string[4]", "optional<string[8]>", Held::place("x")),
            Ok("Some(::sce_rust_runtime::portable::SceString::<8>::widened(&x))".to_string())
        );
        // A byte string of a smaller bound is widened by type too.
        assert_eq!(
            copy("bytes[2]", "bytes[4]", Held::place("x")),
            Ok("::sce_rust_runtime::portable::SceBytes::<4>::widened(&x)".to_string())
        );
        // A pair of types the walk has no copy of is refused.
        assert!(copy("record:Item", "list<record:Item>[3]", Held::place("x")).is_err());
    }

    #[test]
    fn a_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &RUST, &at).expect("write");
        let p = "::sce_rust_runtime::event_payload";
        assert_eq!(write("uint8", Held::place("x.n")), "x.n.to_string()");
        assert_eq!(
            write("int64", Held::place("x.n")),
            format!("{p}::quote(&x.n.to_string())")
        );
        assert_eq!(
            write("string[8]", Held::place("x.s")),
            format!("{p}::quote(&x.s)")
        );
        assert_eq!(
            write("string[8]", Held::reference("h")),
            format!("{p}::quote(h)")
        );
        assert_eq!(
            write("enum:Level", Held::reference("h")),
            format!("{p}::quote(h.sce_name())")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "{p}::list_json(&x.maybe, |h| {p}::optional_json(h.as_ref(), |h| h.to_string()))"
            )
        );
        assert_eq!(
            write("list<record:Item>[3]", Held::place("x.items")),
            format!(
                "{p}::list_json(&x.items, |h| {p}::record_json(vec![(\"id\", h.id.to_string()), \
                 (\"level\", {p}::quote(h.level.sce_name())), (\"due\", {p}::optional_json(h.due.as_ref(), \
                 |h| {p}::quote(&h.to_string()))), (\"blob\", {p}::quote(&{p}::bytes_as_payload_text(&h.blob))), \
                 (\"tags\", {p}::list_json(&h.tags, |h| {p}::quote(h)))]))"
            )
        );
    }
}
