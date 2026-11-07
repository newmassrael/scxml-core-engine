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
//! * the writer that spells it as the JSON the wire carries (section 3.1), or
//!   as the value a `<send>`'s params carry it in until they are written as
//!   that JSON (section 5.3), [`writer`].
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
use crate::forge::value_type::{ScopedCase, ScopedUnion, TypeScope, ValueType};

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

/// A union a machine holds (docs/adr/0092): the key the walks name it by (a
/// `within` its cases' records are read in, [`union_key`]), the union as the
/// type grammar reads it, and the alias the statechart imports its tag's enum
/// by, `None` where it imports no enum of the tag's.
#[derive(Debug, Clone)]
pub struct HeldUnion<'m> {
    pub key: String,
    pub union: &'m ScopedUnion,
    pub tag_enum: Option<String>,
}

impl HeldUnion<'_> {
    /// The alias of the enum of the tag as the machine holds it, or the
    /// sentence of a statechart that imports no enum of the tag's: the reader
    /// of a value reads its tag by that enum, and the tag's read answers one.
    pub fn tag_enum(&self) -> Result<&str, String> {
        self.tag_enum.as_deref().ok_or_else(|| {
            format!(
                "`union:{}` has its tag `{}` of an enum the statechart does not import; import \
                 the enum so the machine holds a type for it",
                self.union.name, self.union.tag
            )
        })
    }
}

/// What the key of a union opens with, so a `within` that names a union is
/// told from one that names a record by its alias.
const UNION_KEY: &str = "union:";

/// The key the walks name `union` by: its document's identity, or its name for
/// a union read from no file.
pub fn union_key(union: &ScopedUnion) -> String {
    let identity = if union.identity.is_empty() {
        &union.name
    } else {
        &union.identity
    };
    format!("{UNION_KEY}{identity}")
}

/// Where the records and unions a composed value names are found.
pub trait RecordEnv {
    /// The record `alias` names, written in the statechart's own scope when
    /// `within` is `None`, and in the document of the record or union the
    /// statechart holds as `within` otherwise. `Err` is the sentence of a
    /// record the machine does not hold.
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String>;
    /// The union `alias` names, written in the scope `within` names as for
    /// [`Self::record`]. `Err` is the sentence of a union the machine cannot
    /// read.
    fn union(&self, within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String>;
    /// The union the walks name by `key` ([`union_key`]).
    fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String>;
}

/// The records of a statechart: the schemas it imports, by alias, the scope
/// each document names its records in, and every union a type it reads names
/// at any depth, by key. Held apart from the model, so a walk that rewrites
/// the model's states reads them.
#[derive(Debug, Clone)]
pub struct StatechartRecords {
    scope: TypeScope,
    records: BTreeMap<String, EventSchemaModel>,
    unions: BTreeMap<String, ScopedUnion>,
}

impl StatechartRecords {
    /// The records `model` holds.
    pub fn of(model: &crate::model::SCXMLModel) -> Self {
        let scope = crate::forge::value_path::statechart_scope(model);
        let mut unions = BTreeMap::new();
        let mut seen = std::collections::BTreeSet::new();
        gather_unions(&scope, &mut unions, &mut seen);
        Self {
            scope,
            records: model.imported_records.clone(),
            unions,
        }
    }

    /// The scope the record or union `within` names its types in.
    fn scope_within(&self, within: &str) -> Option<&TypeScope> {
        if within.starts_with(UNION_KEY) {
            self.unions.get(within).map(|union| &union.scope)
        } else {
            self.scope.records.get(within).map(|record| &record.scope)
        }
    }
}

/// Every union `scope` names at any depth, through its records' and its
/// unions' own scopes, by key; `seen` holds each record already walked, so
/// one document is walked once.
fn gather_unions(
    scope: &TypeScope,
    unions: &mut BTreeMap<String, ScopedUnion>,
    seen: &mut std::collections::BTreeSet<String>,
) {
    for union in scope.unions.values() {
        let key = union_key(union);
        if let std::collections::btree_map::Entry::Vacant(slot) = unions.entry(key) {
            slot.insert(union.clone());
            gather_unions(&union.scope, unions, seen);
        }
    }
    for record in scope.records.values() {
        if seen.insert(record.identity.clone()) {
            gather_unions(&record.scope, unions, seen);
        }
    }
}

impl RecordEnv for StatechartRecords {
    fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
        let unheld = || {
            format!("`record:{alias}` names a record whose document the statechart does not import")
        };
        // A record named inside another, or inside a union, is named by that
        // document, and held by the statechart under the alias it imports the
        // same document by.
        let held: &str = match within {
            None => alias,
            Some(within) => {
                let identity = &self
                    .scope_within(within)
                    .and_then(|scope| scope.records.get(alias))
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

    fn union(&self, within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String> {
        let scope = match within {
            None => Some(&self.scope),
            Some(within) => self.scope_within(within),
        };
        let union = scope
            .and_then(|scope| scope.unions.get(alias))
            .ok_or_else(|| format!("`union:{alias}` names a union the statechart cannot read"))?;
        self.union_of_key(&union_key(union))
    }

    fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String> {
        let union = self
            .unions
            .get(key)
            .ok_or_else(|| format!("`{key}` names a union the statechart cannot read"))?;
        Ok(HeldUnion {
            key: key.to_string(),
            union,
            tag_enum: crate::forge::value_path::tag_enum_alias(&self.scope, union),
        })
    }
}

/// The record or union whose document the aliases of a place's type are
/// written in, as the statechart holds it, for a place reached through the
/// records and unions `chain` names, each alias in the scope of the one before
/// it ([`crate::forge::value_path::Place::scope`]); `None` for a place in the
/// statechart's own scope. A union is named by its key ([`union_key`]).
pub fn within_of(chain: &[String], records: &dyn RecordEnv) -> Result<Option<String>, String> {
    let mut within: Option<String> = None;
    for alias in chain {
        let held = match records.record(within.as_deref(), alias) {
            Ok(record) => record.alias.to_string(),
            Err(why) => records
                .union(within.as_deref(), alias)
                .map(|union| union.key)
                .map_err(|_| why)?,
        };
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

    /// The type a value of the union whose document is named `name` is held
    /// in (docs/adr/0092, section 7).
    fn union_type(&self, name: &str) -> String;
    /// The identifier of the case of the variant `variant` in a union's type.
    fn union_case_ident(&self, variant: &str) -> String;
    /// The read of a union's tag under the key `tag`, `None` for a key this
    /// backend names no read by.
    fn union_tag_read(&self, tag: &str) -> Option<String>;
    /// The read of the record of the case of `variant` where a test proved it.
    fn union_case_read(&self, variant: &str) -> String;
    /// The read of the field `field` several cases share ([`SharedFieldDef`]).
    fn union_shared_read(&self, field: &str) -> String;
    /// The declaration of a union's type, its cases and its reads.
    fn union_def(&self, def: &UnionDef) -> String;
    /// A value of the union held in `ty`, in the case `case` (its identifier,
    /// [`Self::union_case_ident`]) holding the record value `record`, or none
    /// for an empty case (docs/adr/0092, section 7 item 3: built only whole).
    fn union_value(&self, ty: &str, case: &str, record: Option<&str>) -> String;
    /// The reader of a value of a union at `value`, as `read` says it: its tag
    /// first, and then the case the tag names, by its arm.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String;
    /// The JSON of a value of a union held in `ty`: its tag under the key
    /// `tag` beside the fields of its case, by the arm of the case it is in,
    /// the case's record a reference bound to [`Self::write_binding`].
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String;
    /// What follows a value of `union` to read its member `member` where it
    /// may be in the cases `left` (docs/adr/0092, section 6.1): its tag, or a
    /// field the cases left each declare. `Err` names a read this backend
    /// does not spell.
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String>;
    /// The accessor of the place a read of a union (a case's record, a shared
    /// field) is written through, beside the read named `read`
    /// (docs/adr/0092, section 6.1 item 6).
    fn union_place_of(&self, read: &str) -> String;
    /// The place of the field `member` of `holder`, a value of `union`, where
    /// it may be in the cases `left` (docs/adr/0092, section 6.1 item 6): the
    /// field of the one case left, or a field the cases left share. `Err` for
    /// the tag, which is never written alone, and for a field this backend has
    /// no place of.
    fn union_member_place(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String>;
}

/// One case of a union's declaration ([`UnionDef`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionCaseDef {
    /// The case's identifier in the union's type.
    pub ident: String,
    /// The variant's declared name.
    pub variant: String,
    /// The identifier of the variant in the tag's enum type ([`UnionDef::tag_ty`]).
    pub tag_ident: String,
    /// The type of the record the case holds, `None` for an empty case.
    pub record: Option<String>,
    /// The read of the case's record where a test proved it.
    pub proven: String,
}

/// A field two or more cases of a union declare with one type a value is
/// copied out as, read by one accessor over every case that declares it
/// (docs/adr/0092, section 6.1 item 2, open question 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedFieldDef {
    pub field: String,
    pub accessor: String,
    pub ty: String,
    /// The identifier of each case that declares it.
    pub cases: Vec<String>,
}

/// A union's declaration in a machine (docs/adr/0092, section 7): its type,
/// its document's name, the read of its tag and the enum type it answers, each
/// case in the tag enum's order, and each field shared by several cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionDef {
    pub ty: String,
    pub name: String,
    pub tag_method: String,
    pub tag_ty: String,
    pub cases: Vec<UnionCaseDef>,
    pub shared: Vec<SharedFieldDef>,
}

/// What a union's reader reads ([`ComposedSpelling::read_union`]): the key of
/// its tag, the reader of the tag bound to [`ComposedSpelling::read_binding`],
/// the enum type it answers, the union's type, and one arm per case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionRead<'a> {
    pub tag: &'a str,
    pub tag_reader: String,
    pub tag_ty: &'a str,
    pub ty: String,
    pub arms: Vec<UnionReadArm>,
}

/// One arm of a union's reader: the identifier of the variant in the tag's
/// enum type, of the case in the union's type, and the reader of the case's
/// record bound to [`ComposedSpelling::read_binding`] (`None` for an empty
/// case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionReadArm {
    pub tag_ident: String,
    pub case_ident: String,
    pub record: Option<String>,
}

/// One arm of a union's writer: the identifier of the case in the union's
/// type, the variant's declared name its tag is written as, and each field of
/// its record with the JSON of its value (`None` for an empty case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionWriteArm {
    pub case_ident: String,
    pub variant: String,
    pub fields: Option<Vec<(String, String)>>,
}

/// The identifier of the case `variant` of a union's type: the variant as an
/// enum's Rust declaration spells it ([`crate::forge::enum_naming::variant_ident`]).
pub(crate) fn rust_case_ident(variant: &str) -> String {
    crate::forge::enum_naming::variant_ident(crate::generator::Language::Rust, "", variant)
}

/// The read of the record of the case `variant` where a test proved it, on a
/// union's Rust type (the unit's and the machine's alike).
pub(crate) fn rust_case_read(variant: &str) -> String {
    format!(
        "proven_{}",
        crate::filters::to_snake_case(variant.to_string())
    )
}

/// The read of a field several cases share ([`SharedFieldDef`]) on a machine's
/// union.
pub(crate) fn rust_shared_read(field: &str) -> String {
    format!(
        "proven_field_{}",
        crate::filters::to_snake_case(field.to_string())
    )
}

/// The methods every value of a generated Rust union answers through its
/// derives (`Clone`, `PartialEq`, `Debug`): an inherent method of one of these
/// names would hide the trait's at every call written with a dot, so the tag's
/// accessor is never one of them.
const RUST_UNION_DERIVED_METHODS: &[&str] = &["clone", "clone_from", "eq", "ne", "fmt"];

/// The Rust method the tag's key `tag` is read by on a generated union, the
/// unit's and the machine's alike: the key in snake case, as a payload's field
/// is named, escaped with `r#` where Rust reserves it. `None` for a key that
/// is no Rust identifier, one `r#` cannot escape (`self`, `Self`, `super`,
/// `crate`), and one that would hide a derived method
/// ([`RUST_UNION_DERIVED_METHODS`]).
pub(crate) fn rust_union_tag_method(tag: &str) -> Option<String> {
    let snake = crate::filters::to_snake_case(tag.to_string());
    let mut chars = snake.chars();
    let identifier = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && snake != "_";
    if !identifier
        || ["self", "Self", "super", "crate"].contains(&snake.as_str())
        || RUST_UNION_DERIVED_METHODS.contains(&snake.as_str())
    {
        return None;
    }
    Some(if crate::filters::RUST_KEYWORDS.contains(&snake.as_str()) {
        format!("r#{snake}")
    } else {
        snake
    })
}

/// The type `field` has in every case of `union` that declares it, where two
/// or more cases declare it, all with one type, and a value of it is copied as
/// it is read ([`plain`]): the fields a machine's union reads by one accessor
/// over the cases ([`SharedFieldDef`]). `None` for every other field.
pub fn shared_plain_field<'u>(union: &'u ScopedUnion, field: &str) -> Option<&'u ValueType> {
    let mut found: Option<&ValueType> = None;
    let mut declaring = 0usize;
    for case in &union.cases {
        let declared = case
            .record
            .as_deref()
            .and_then(|record| union.scope.records.get(record))
            .and_then(|record| record.fields.iter().find(|(id, _)| id == field));
        if let Some((_, ty)) = declared {
            match found {
                Some(held) if held != ty => return None,
                _ => found = Some(ty),
            }
            declaring += 1;
        }
    }
    found.filter(|ty| declaring >= 2 && plain(ty))
}

/// The Rust spelling of what follows a union's value to read `member` where
/// it may be in the cases `left` ([`ComposedSpelling::union_member`]): the
/// tag's method; the field of the one case left, read through that case's
/// proven read, a place as a record's field is; or the accessor of a field
/// several cases share ([`shared_plain_field`]). `Err` for a field the cases
/// left share whose value is not copied as it is read, which no accessor
/// answers yet.
pub(crate) fn rust_union_member(
    union: &ScopedUnion,
    left: &[&ScopedCase],
    member: &str,
) -> Result<String, String> {
    if member == union.tag {
        return rust_union_tag_method(member)
            .map(|method| format!("{method}()"))
            .ok_or_else(|| {
                format!("the tag's key `{member}` names no Rust method its value can be read by")
            });
    }
    let declares = |case: &ScopedCase| -> bool {
        case.record
            .as_deref()
            .and_then(|record| union.scope.records.get(record))
            .is_some_and(|record| record.fields.iter().any(|(id, _)| id == member))
    };
    match left {
        [case] if declares(case) => {
            Ok(format!("{}().{member}", rust_case_read(&case.variant.name)))
        }
        _ if !left.is_empty()
            && left.iter().all(|case| declares(case))
            && shared_plain_field(union, member).is_some() =>
        {
            Ok(format!("{}()", rust_shared_read(member)))
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

/// The Rust method a read of a machine's union (`proven_<case>`,
/// `proven_field_<f>`) is written through: the read's name with `_mut`, a
/// mutable reference to the same place.
pub(crate) fn rust_union_place_of(read: &str) -> String {
    format!("{read}_mut")
}

/// The Rust place of the field `member` of the union value `holder` where it
/// may be in the cases `left` ([`ComposedSpelling::union_member_place`]): the
/// field of the one case left, through that case's mutable proven read, or
/// the place a field several cases share is written through
/// ([`shared_plain_field`]). `Err` for the tag, which is never written alone
/// (docs/adr/0092, section 6.1 item 6), and for a field the cases left share
/// whose value is not copied as it is read, which no accessor answers yet.
pub(crate) fn rust_union_member_place(
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
    rust_union_member(union, left, member)?;
    Ok(match left {
        [case] => format!(
            "{holder}.{}().{member}",
            rust_union_place_of(&rust_case_read(&case.variant.name))
        ),
        _ => format!(
            "*{holder}.{}()",
            rust_union_place_of(&rust_shared_read(member))
        ),
    })
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
        // docs/adr/0092, section 7: the machine's own sum, named by the
        // union's document, whichever alias names it.
        ValueType::Union { alias } => {
            spelling.union_type(&records.union(within, alias)?.union.name)
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
    /// The key of each union ([`union_key`]), each once, after the records
    /// its cases hold.
    pub unions: Vec<String>,
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
        // docs/adr/0092, section 7: the union, its tag's enum, and the record
        // each case holds, which the machine's sum holds as the machine's
        // records; a union that holds itself is refused where its document is
        // read (section 1.1), so the walk ends.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            if reached.unions.contains(&held.key) {
                return Ok(());
            }
            reached.enums.insert(held.tag_enum()?.to_string());
            for case in &held.union.cases {
                if let Some(record) = &case.record {
                    held_records(
                        &ValueType::Record {
                            alias: record.clone(),
                        },
                        Some(&held.key),
                        records,
                        reached,
                    )?;
                }
            }
            if !reached.unions.contains(&held.key) {
                reached.unions.push(held.key);
            }
        }
    }
    Ok(())
}

/// The declaration of the union the walks name by `key` ([`union_key`]) in
/// the machine (docs/adr/0092, section 7): each case in its tag enum's order,
/// holding the machine's record of its case or nothing, the read of the tag
/// as the machine's enum, a proven read per case that holds a record, and an
/// accessor per field several cases share ([`shared_plain_field`]). `Err`
/// names a union the machine has no declaration of: a tag of an enum the
/// statechart does not import, a key the backend names no read by, a record
/// the statechart does not import, or two reads of one name.
pub fn union_definition(
    key: &str,
    records: &dyn RecordEnv,
    enums: &BTreeMap<String, MachineEnum>,
    spelling: &dyn ComposedSpelling,
) -> Result<UnionDef, String> {
    let held = records.union_of_key(key)?;
    let union = held.union;
    let tag = machine_enum(enums, held.tag_enum()?)?;
    let tag_method = spelling.union_tag_read(&union.tag).ok_or_else(|| {
        format!(
            "`union:{}`: the tag's key `{}` names no method its value can be read by",
            union.name, union.tag
        )
    })?;
    let mut methods = std::collections::BTreeSet::new();
    methods.insert(tag_method.clone());
    let mut cases = Vec::with_capacity(union.cases.len());
    let mut case_records: BTreeMap<String, &str> = BTreeMap::new();
    for case in &union.cases {
        let variant = case.variant.name.as_str();
        let (_, tag_ident) = tag
            .variants
            .iter()
            .find(|(declared, _)| declared == variant)
            .ok_or_else(|| {
                format!(
                    "`union:{}`: the machine's `{}` has no variant `{variant}`",
                    union.name, tag.ty
                )
            })?;
        let record = match &case.record {
            None => None,
            Some(record) => {
                let held_record = records.record(Some(key), record)?;
                case_records.insert(variant.to_string(), held_record.alias);
                Some(spelling.record_type(held_record.alias))
            }
        };
        let proven = spelling.union_case_read(variant);
        if record.is_some() {
            for method in [proven.clone(), spelling.union_place_of(&proven)] {
                if !methods.insert(method.clone()) {
                    return Err(format!(
                        "`union:{}`: the case `{variant}` is read by `{method}`, which another \
                         read of the union is named too",
                        union.name
                    ));
                }
            }
        }
        cases.push(UnionCaseDef {
            ident: spelling.union_case_ident(variant),
            variant: variant.to_string(),
            tag_ident: tag_ident.clone(),
            record,
            proven,
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut shared = Vec::new();
    for case in &union.cases {
        let Some(record) = case
            .record
            .as_deref()
            .and_then(|r| union.scope.records.get(r))
        else {
            continue;
        };
        for (field, ty) in &record.fields {
            if !seen.insert(field.clone()) || shared_plain_field(union, field).is_none() {
                continue;
            }
            let accessor = spelling.union_shared_read(field);
            for method in [accessor.clone(), spelling.union_place_of(&accessor)] {
                if !methods.insert(method.clone()) {
                    return Err(format!(
                        "`union:{}`: the field `{field}` several cases share is read by \
                         `{method}`, which another read of the union is named too",
                        union.name
                    ));
                }
            }
            let within = case_records.get(&case.variant.name).copied();
            shared.push(SharedFieldDef {
                field: field.clone(),
                accessor,
                ty: machine_type(ty, within, records, enums, spelling)?,
                cases: union
                    .cases
                    .iter()
                    .filter(|other| {
                        other
                            .record
                            .as_deref()
                            .and_then(|r| union.scope.records.get(r))
                            .is_some_and(|r| r.fields.iter().any(|(id, _)| id == field))
                    })
                    .map(|other| spelling.union_case_ident(&other.variant.name))
                    .collect(),
            });
        }
    }
    Ok(UnionDef {
        ty: spelling.union_type(&union.name),
        name: union.name.clone(),
        tag_method,
        tag_ty: tag.ty.clone(),
        cases,
        shared,
    })
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
        // docs/adr/0092, section 4: the tag first, by its enum's reader, and
        // then the record of the case it names, from the same object.
        ValueType::Union { alias } => {
            let held = records.union(within, alias)?;
            let tag = machine_enum(enums, held.tag_enum()?)?;
            let ty = spelling.union_type(&held.union.name);
            let mut arms = Vec::with_capacity(held.union.cases.len());
            for case in &held.union.cases {
                let variant = case.variant.name.as_str();
                let (_, tag_ident) = tag
                    .variants
                    .iter()
                    .find(|(declared, _)| declared == variant)
                    .ok_or_else(|| {
                        format!(
                            "`union:{}`: the machine's `{}` has no variant `{variant}`",
                            held.union.name, tag.ty
                        )
                    })?;
                let record = match &case.record {
                    None => None,
                    Some(record) => Some(reader(
                        &ValueType::Record {
                            alias: record.clone(),
                        },
                        Some(&held.key),
                        records,
                        enums,
                        spelling,
                        bound,
                    )?),
                };
                arms.push(UnionReadArm {
                    tag_ident: tag_ident.clone(),
                    case_ident: spelling.union_case_ident(variant),
                    record,
                });
            }
            spelling.read_union(
                value,
                &UnionRead {
                    tag: &held.union.tag,
                    tag_reader: spelling.read_enum(tag, bound),
                    tag_ty: &tag.ty,
                    ty,
                    arms,
                },
            )
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
        // docs/adr/0092, section 2.1: the tag beside the fields of the case
        // the value is in, each field of the case's record read through the
        // reference its arm binds.
        ValueType::Union { alias } => {
            let union = records.union(within, alias)?;
            let ty = spelling.union_type(&union.union.name);
            let mut arms = Vec::with_capacity(union.union.cases.len());
            for case in &union.union.cases {
                let fields = match &case.record {
                    None => None,
                    Some(record) => Some(field_writers(
                        records.record(Some(&union.key), record)?,
                        records,
                        spelling,
                        &bound,
                    )?),
                };
                arms.push(UnionWriteArm {
                    case_ident: spelling.union_case_ident(&case.variant.name),
                    variant: case.variant.name.clone(),
                    fields,
                });
            }
            spelling.write_union(held, &ty, &union.union.tag, &arms)
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
/// and its writers write them into `carrier`.
pub struct RustComposed<'a> {
    pub machine: &'a str,
    pub carrier: RustCarrier,
}

/// What a Rust writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RustCarrier {
    /// The JSON text itself, by the runtime's `list_json`, `optional_json`,
    /// `record_json` and `union_json`: what the inject seam hands a lift
    /// (section 3.1).
    Json,
    /// The runtime's `ScriptValue`, a list as its `Array` and a record as its
    /// `Object`, every scalar as a `<send>`'s scalar param is
    /// (`StaticTarget::wire_value`): what a `<send>`'s params are held in until
    /// the runtime writes them as one JSON object, a record's keys ascending by
    /// their UTF-8 bytes at every depth (section 5.3; ARCHITECTURE.md, "JSON
    /// Object Key Order").
    ScriptValue,
}

/// The runtime module the Rust readers and writers are in.
const RUST_PAYLOAD: &str = "::sce_rust_runtime::event_payload";

/// The runtime's dynamic value, which a Rust `<send>` holds its params in.
const RUST_SCRIPT_VALUE: &str = "::sce_rust_runtime::ScriptValue";

/// `members`, each a key and the `ScriptValue` its value is, as one
/// `ScriptValue::Object`: the runtime orders its keys when it writes it.
fn rust_script_object(members: &[(String, String)]) -> String {
    let members: Vec<String> = members
        .iter()
        .map(|(id, value)| format!("(\"{id}\".to_string(), {value})"))
        .collect();
    format!(
        "{RUST_SCRIPT_VALUE}::Object(vec![{}].into_iter().collect())",
        members.join(", ")
    )
}

/// A union value held at `held`, of the machine's sum `ty`, as one
/// `ScriptValue::Object`: the tag `tag`, its value the case's name, beside the
/// fields of the case's record (docs/adr/0092, section 2.1), as the runtime's
/// `union_json` writes the same value.
fn rust_script_union(held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
    let tag = crate::filters::escape_rust(tag.to_string());
    let arms: String = arms
        .iter()
        .map(|arm| {
            let named = (
                tag.clone(),
                format!(
                    "{RUST_SCRIPT_VALUE}::String(\"{}\".to_string())",
                    crate::filters::escape_rust(arm.variant.clone())
                ),
            );
            match &arm.fields {
                Some(fields) => {
                    let mut members = fields.clone();
                    members.push(named);
                    format!(
                        "{ty}::{}(h) => {}, ",
                        arm.case_ident,
                        rust_script_object(&members)
                    )
                }
                None => format!(
                    "{ty}::{} => {}, ",
                    arm.case_ident,
                    rust_script_object(&[named])
                ),
            }
        })
        .collect();
    format!("match {} {{ {arms}}}", held.borrowed())
}

/// The value `held` is, where it is a reference to the value: a copy of a
/// number or a truth value read through it.
fn rust_copied(held: &Held) -> String {
    if held.by_reference {
        format!("*{}", held.text)
    } else {
        held.text.clone()
    }
}

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
        match (self.carrier, ty) {
            // A 64-bit integer crosses as the text of its digits.
            (RustCarrier::Json, SceType::Uint64 | SceType::Int64) => {
                format!("{RUST_PAYLOAD}::quote(&{}.to_string())", held.text)
            }
            // A number other than a 64-bit integer, and a truth value, as
            // written.
            (RustCarrier::Json, _) => format!("{}.to_string()", held.text),
            // As a scalar param is: a narrow integer widened to `i64` and a
            // `float` to `f64`, both exactly.
            (RustCarrier::ScriptValue, SceType::Uint64 | SceType::Int64) => {
                format!("{RUST_SCRIPT_VALUE}::String({}.to_string())", held.text)
            }
            (RustCarrier::ScriptValue, SceType::Bool) => {
                format!("{RUST_SCRIPT_VALUE}::Bool({})", rust_copied(held))
            }
            (RustCarrier::ScriptValue, SceType::Float32) => {
                format!(
                    "{RUST_SCRIPT_VALUE}::Double(f64::from({}))",
                    rust_copied(held)
                )
            }
            (RustCarrier::ScriptValue, SceType::Float64) => {
                format!("{RUST_SCRIPT_VALUE}::Double({})", rust_copied(held))
            }
            (RustCarrier::ScriptValue, _) => {
                format!("{RUST_SCRIPT_VALUE}::Int(i64::from({}))", rust_copied(held))
            }
        }
    }
    // The machine's enum answers the name its document declares
    // (`StaticTarget::enum_def`'s `sce_name`).
    fn write_enum(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!("{RUST_PAYLOAD}::quote({}.sce_name())", held.text),
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::String({}.sce_name().to_string())",
                held.text
            ),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!("{RUST_PAYLOAD}::quote({})", held.borrowed()),
            RustCarrier::ScriptValue => {
                format!("{RUST_SCRIPT_VALUE}::String({}.to_string())", held.text)
            }
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            RustCarrier::Json => format!(
                "{RUST_PAYLOAD}::quote(&{RUST_PAYLOAD}::bytes_as_payload_text({}))",
                held.borrowed()
            ),
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::String({RUST_PAYLOAD}::bytes_as_payload_text({}))",
                held.borrowed()
            ),
        }
    }
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            RustCarrier::Json => format!(
                "{RUST_PAYLOAD}::optional_json({}.as_ref(), |h| {elem})",
                held.text
            ),
            // `null` where the value is absent, as an absent scalar param is.
            RustCarrier::ScriptValue => format!(
                "{}.as_ref().map_or({RUST_SCRIPT_VALUE}::Null, |h| {elem})",
                held.text
            ),
        }
    }
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            RustCarrier::Json => {
                format!("{RUST_PAYLOAD}::list_json({}, |h| {elem})", held.borrowed())
            }
            RustCarrier::ScriptValue => format!(
                "{RUST_SCRIPT_VALUE}::Array({}.iter().map(|h| {elem}).collect())",
                held.text
            ),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    fn write_record(&self, fields: &[(String, String)]) -> String {
        match self.carrier {
            RustCarrier::Json => {
                let members: Vec<String> = fields
                    .iter()
                    .map(|(id, json)| format!("(\"{id}\", {json})"))
                    .collect();
                format!("{RUST_PAYLOAD}::record_json(vec![{}])", members.join(", "))
            }
            RustCarrier::ScriptValue => rust_script_object(fields),
        }
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

    // The machine's own sum, beside its records (docs/adr/0092, section 7, the
    // Rust row): named by the union's document, so one union is one type of
    // the machine whichever alias names it.
    fn union_type(&self, name: &str) -> String {
        format!(
            "{}{}Union",
            self.machine,
            crate::filters::to_pascal_case(name.to_string())
        )
    }
    fn union_case_ident(&self, variant: &str) -> String {
        rust_case_ident(variant)
    }
    fn union_tag_read(&self, tag: &str) -> Option<String> {
        rust_union_tag_method(tag)
    }
    fn union_case_read(&self, variant: &str) -> String {
        rust_case_read(variant)
    }
    fn union_shared_read(&self, field: &str) -> String {
        rust_shared_read(field)
    }
    // Owned and compared whole, as a record that holds a composed value is;
    // every read the narrowing proves is a method here, whose other arms are
    // `unreachable!()`, the one place they are written (as a proven optional
    // is read through the runtime's one `proven`, docs/adr/0055, section 5).
    fn union_def(&self, def: &UnionDef) -> String {
        let name = crate::filters::escape_rust(def.name.clone());
        let others = |covered: usize, variant: &str| {
            if covered < def.cases.len() {
                format!(
                    "            _ => unreachable!(\"a value of {{}} read as {{}} is in another \
                     case\", \"{name}\", \"{}\"),\n",
                    crate::filters::escape_rust(variant.to_string())
                )
            } else {
                String::new()
            }
        };
        let mut variants = String::new();
        let mut tag_arms = String::new();
        let mut proven = String::new();
        for case in &def.cases {
            let ident = &case.ident;
            match &case.record {
                Some(record) => {
                    variants.push_str(&format!("    {ident}({record}),\n"));
                    tag_arms.push_str(&format!(
                        "            Self::{ident}(_) => {}::{},\n",
                        def.tag_ty, case.tag_ident
                    ));
                    proven.push_str(&format!(
                        "\n    pub fn {}(&self) -> &{record} {{\n        match self {{\n            \
                         Self::{ident}(record) => record,\n{}        }}\n    }}\n",
                        case.proven,
                        others(1, &case.variant)
                    ));
                    // The place a field of the case is written through where
                    // the case is proven (docs/adr/0092, section 6.1 item 6).
                    proven.push_str(&format!(
                        "\n    pub fn {}(&mut self) -> &mut {record} {{\n        match self {{\n            \
                         Self::{ident}(record) => record,\n{}        }}\n    }}\n",
                        rust_union_place_of(&case.proven),
                        others(1, &case.variant)
                    ));
                }
                None => {
                    variants.push_str(&format!("    {ident},\n"));
                    tag_arms.push_str(&format!(
                        "            Self::{ident} => {}::{},\n",
                        def.tag_ty, case.tag_ident
                    ));
                }
            }
        }
        let shared: String = def
            .shared
            .iter()
            .map(|field| {
                let arms: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!("            Self::{ident}(record) => record.{},\n", field.field)
                    })
                    .collect();
                let places: String = field
                    .cases
                    .iter()
                    .map(|ident| {
                        format!(
                            "            Self::{ident}(record) => &mut record.{},\n",
                            field.field
                        )
                    })
                    .collect();
                format!(
                    "\n    pub fn {}(&self) -> {} {{\n        match self {{\n{arms}{}        }}\n    \
                     }}\n\n    pub fn {}(&mut self) -> &mut {} {{\n        match self \
                     {{\n{places}{}        }}\n    }}\n",
                    field.accessor,
                    field.ty,
                    others(field.cases.len(), &field.field),
                    rust_union_place_of(&field.accessor),
                    field.ty,
                    others(field.cases.len(), &field.field)
                )
            })
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `union:{name}` value (docs/adr/0092).\n\
             #[derive(Clone, Debug, PartialEq)]\npub enum {ty} {{\n{variants}}}\n\n\
             #[allow(dead_code, non_snake_case)]\nimpl {ty} {{\n    pub fn {tag}(&self) -> {tag_ty} \
             {{\n        match self {{\n{tag_arms}        }}\n    }}\n{proven}{shared}}}",
            ty = def.ty,
            tag = def.tag_method,
            tag_ty = def.tag_ty,
        )
    }
    // The case's constructor, holding the record or nothing.
    fn union_value(&self, ty: &str, case: &str, record: Option<&str>) -> String {
        match record {
            Some(record) => format!("{ty}::{case}({record})"),
            None => format!("{ty}::{case}"),
        }
    }
    // The tag first, by the enum's reader, then the record of the case it
    // names from the same object (docs/adr/0092, section 4), each case a
    // match arm the compiler holds to every variant of the tag's enum.
    fn read_union(&self, value: &str, read: &UnionRead<'_>) -> String {
        let arms: String = read
            .arms
            .iter()
            .map(|arm| {
                let case = format!("{}::{}", read.ty, arm.case_ident);
                let value = match &arm.record {
                    Some(record) => format!("{record}.map({case})"),
                    None => format!("Ok({case})"),
                };
                format!("{}::{} => {value}, ", read.tag_ty, arm.tag_ident)
            })
            .collect();
        format!(
            "{value}.union(\"{}\", |v| {}, |sce_case, v| match sce_case {{ {arms}}})",
            crate::filters::escape_rust(read.tag.to_string()),
            read.tag_reader
        )
    }
    // One object, the tag among the fields of the case (docs/adr/0092,
    // section 2.1), keys ascending as the runtime's `union_json` writes them.
    fn write_union(&self, held: &Held, ty: &str, tag: &str, arms: &[UnionWriteArm]) -> String {
        if self.carrier == RustCarrier::ScriptValue {
            return rust_script_union(held, ty, tag, arms);
        }
        let tag = crate::filters::escape_rust(tag.to_string());
        let arms: String = arms
            .iter()
            .map(|arm| {
                let variant = crate::filters::escape_rust(arm.variant.clone());
                match &arm.fields {
                    Some(fields) => {
                        let members: Vec<String> = fields
                            .iter()
                            .map(|(id, json)| format!("(\"{id}\", {json})"))
                            .collect();
                        format!(
                            "{ty}::{}(h) => {RUST_PAYLOAD}::union_json(\"{tag}\", \"{variant}\", \
                             vec![{}]), ",
                            arm.case_ident,
                            members.join(", ")
                        )
                    }
                    None => format!(
                        "{ty}::{} => {RUST_PAYLOAD}::union_json(\"{tag}\", \"{variant}\", \
                         Vec::new()), ",
                        arm.case_ident
                    ),
                }
            })
            .collect();
        format!("match {} {{ {arms}}}", held.borrowed())
    }
    fn union_member(
        &self,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        rust_union_member(union, left, member)
    }
    fn union_place_of(&self, read: &str) -> String {
        rust_union_place_of(read)
    }
    fn union_member_place(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<String, String> {
        rust_union_member_place(holder, union, left, member)
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
        fn union(&self, _within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String> {
            Err(format!("no union `{alias}`"))
        }
        fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String> {
            Err(format!("no union `{key}`"))
        }
    }

    const RADIUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="event-schema" name="shape_radius" sce:event-name="shape.radius">
  <datamodel>
    <data id="radius" sce:type="uint16" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

    /// A statechart that imports `shape_radius` as `Radius` and the tag's enum
    /// as `Kind`, and reads a `shape`: a dot of nothing, and a circle and a
    /// disc that both hold `Radius`.
    struct Shapes {
        radius: EventSchemaModel,
        union: ScopedUnion,
    }

    const SHAPE_KEY: &str = "union:shape";

    impl RecordEnv for Shapes {
        fn record(&self, within: Option<&str>, alias: &str) -> Result<HeldRecord<'_>, String> {
            match (within, alias) {
                (None | Some(SHAPE_KEY), "Radius") => Ok(HeldRecord {
                    alias: "Radius",
                    schema: &self.radius,
                }),
                _ => Err(format!("no record `{alias}`")),
            }
        }
        fn union(&self, within: Option<&str>, alias: &str) -> Result<HeldUnion<'_>, String> {
            match (within, alias) {
                (None, "Shape") => self.union_of_key(SHAPE_KEY),
                _ => Err(format!("no union `{alias}`")),
            }
        }
        fn union_of_key(&self, key: &str) -> Result<HeldUnion<'_>, String> {
            (key == SHAPE_KEY)
                .then(|| HeldUnion {
                    key: key.to_string(),
                    union: &self.union,
                    tag_enum: Some("Kind".to_string()),
                })
                .ok_or_else(|| format!("no union `{key}`"))
        }
    }

    fn shapes() -> Shapes {
        use crate::forge::value_type::{ScopedEnum, ScopedRecord, ScopedVariant};
        let variants: Vec<ScopedVariant> = ["dot", "circle", "disc"]
            .iter()
            .zip(0..)
            .map(|(name, value)| ScopedVariant {
                name: name.to_string(),
                value,
            })
            .collect();
        let mut scope = TypeScope::default();
        scope.enums.insert(
            "ShapeKind".to_string(),
            ScopedEnum {
                variants: variants.clone(),
                underlying: SceType::Uint8,
                strict: true,
            },
        );
        scope.records.insert(
            "Radius".to_string(),
            ScopedRecord {
                identity: "shape_radius".to_string(),
                name: "shape_radius".to_string(),
                fields: vec![("radius".to_string(), ty("uint16"))],
                scope: TypeScope::default(),
            },
        );
        let cases = variants
            .into_iter()
            .map(|variant| ScopedCase {
                record: (variant.name != "dot").then(|| "Radius".to_string()),
                variant,
            })
            .collect();
        Shapes {
            radius: schema(RADIUS),
            union: ScopedUnion {
                identity: "shape".to_string(),
                name: "shape".to_string(),
                tag: "kind".to_string(),
                tag_enum: "ShapeKind".to_string(),
                cases,
                scope,
            },
        }
    }

    fn kinds() -> BTreeMap<String, MachineEnum> {
        [(
            "Kind".to_string(),
            MachineEnum {
                alias: "Kind".to_string(),
                ty: "MKindEnum".to_string(),
                variants: vec![
                    ("dot".to_string(), "Dot".to_string()),
                    ("circle".to_string(), "Circle".to_string()),
                    ("disc".to_string(), "Disc".to_string()),
                ],
            },
        )]
        .into()
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

    const RUST: RustComposed<'static> = RustComposed {
        machine: "M",
        carrier: RustCarrier::Json,
    };

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

    /// A union is the machine's own sum at any depth, named by its document
    /// (docs/adr/0092, section 7), and reaches its tag's enum, then the record
    /// each case holds once, then itself.
    #[test]
    fn the_machine_holds_a_union_in_its_own_sum_after_its_cases_records() {
        let shapes = shapes();
        let kinds = kinds();
        let list = "::sce_rust_runtime::portable::BoundedList";
        assert_eq!(
            machine_type(&ty("union:Shape"), None, &shapes, &kinds, &RUST),
            Ok("MShapeUnion".to_string())
        );
        assert_eq!(
            machine_type(
                &ty("list<optional<union:Shape>>[3]"),
                None,
                &shapes,
                &kinds,
                &RUST
            ),
            Ok(format!("{list}<Option<MShapeUnion>, 3>"))
        );
        let mut reached = Reached::default();
        held_records(&ty("list<union:Shape>[2]"), None, &shapes, &mut reached).expect("reached");
        held_records(&ty("union:Shape"), None, &shapes, &mut reached).expect("reached");
        assert_eq!(reached.records, vec!["Radius".to_string()]);
        assert_eq!(reached.unions, vec![SHAPE_KEY.to_string()]);
        assert_eq!(reached.enums.into_iter().collect::<Vec<_>>(), vec!["Kind"]);
    }

    /// The declaration: one variant per case, the tag read as the machine's
    /// enum, a proven read per case that holds a record whose other arms are
    /// `unreachable!()`, and one accessor for `radius`, which `circle` and
    /// `disc` share.
    #[test]
    fn a_union_is_declared_with_its_tag_its_proven_reads_and_its_shared_fields() {
        let def = union_definition(SHAPE_KEY, &shapes(), &kinds(), &RUST).expect("declared");
        assert_eq!(def.ty, "MShapeUnion");
        assert_eq!(def.tag_method, "kind");
        assert_eq!(
            def.shared,
            vec![SharedFieldDef {
                field: "radius".to_string(),
                accessor: "proven_field_radius".to_string(),
                ty: "u16".to_string(),
                cases: vec!["Circle".to_string(), "Disc".to_string()],
            }]
        );
        let text = RUST.union_def(&def);
        for fragment in [
            "#[derive(Clone, Debug, PartialEq)]\npub enum MShapeUnion {\n    Dot,\n    \
             Circle(MRadiusRecord),\n    Disc(MRadiusRecord),\n}",
            "    pub fn kind(&self) -> MKindEnum {\n",
            "            Self::Dot => MKindEnum::Dot,\n",
            "            Self::Circle(_) => MKindEnum::Circle,\n",
            "    pub fn proven_circle(&self) -> &MRadiusRecord {\n",
            "            Self::Circle(record) => record,\n",
            "    pub fn proven_field_radius(&self) -> u16 {\n",
            "            Self::Disc(record) => record.radius,\n",
            // ... and the place each is written through (section 6.1 item 6).
            "    pub fn proven_circle_mut(&mut self) -> &mut MRadiusRecord {\n",
            "    pub fn proven_field_radius_mut(&mut self) -> &mut u16 {\n",
            "            Self::Disc(record) => &mut record.radius,\n",
        ] {
            assert!(text.contains(fragment), "expected `{fragment}` in:\n{text}");
        }
        assert!(!text.contains("proven_dot"), "{text}");
        assert_eq!(text.matches("unreachable!(").count(), 6, "{text}");
    }

    /// A write of a union's field (docs/adr/0092, section 6.1 item 6): through
    /// the mutable proven read of the one case left, or the place of a field
    /// two cases share; the tag is never written alone.
    #[test]
    fn a_proven_write_is_spelled_by_the_cases_left() {
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
            rust_union_member_place("self.s", union, &[case("disc")], "radius"),
            Ok("self.s.proven_disc_mut().radius".into())
        );
        assert_eq!(
            rust_union_member_place("self.s", union, &[case("circle"), case("disc")], "radius"),
            Ok("*self.s.proven_field_radius_mut()".into())
        );
        assert!(rust_union_member_place("self.s", union, &all, "kind")
            .unwrap_err()
            .contains("never written alone"));
        assert!(rust_union_member_place("self.s", union, &[case("dot")], "radius").is_err());
    }

    /// The reader reads the tag by the enum's reader and then the record of
    /// the case it names; the writer writes the tag among the case's fields.
    #[test]
    fn a_union_is_read_tag_first_and_written_by_its_case() {
        let shapes = shapes();
        let read = reader(&ty("union:Shape"), None, &shapes, &kinds(), &RUST, "v").expect("read");
        assert!(
            read.starts_with(
                "v.union(\"kind\", |v| v.name().and_then(|name| match name { \"dot\" \
                              => Ok(MKindEnum::Dot), "
            ),
            "{read}"
        );
        assert!(
            read.contains(
                "|sce_case, v| match sce_case { MKindEnum::Dot => Ok(MShapeUnion::Dot), \
                 MKindEnum::Circle => v.record(None, |r| Ok(MRadiusRecord { radius: \
                 r.field(\"radius\").and_then(|v| v.unsigned::<u16>())? })).map(MShapeUnion::Circle), "
            ),
            "{read}"
        );
        let p = "::sce_rust_runtime::event_payload";
        assert_eq!(
            writer(
                &ty("union:Shape"),
                None,
                &shapes,
                &RUST,
                &Held::place("x.one")
            ),
            Ok(format!(
                "match &x.one {{ MShapeUnion::Dot => {p}::union_json(\"kind\", \"dot\", \
                 Vec::new()), MShapeUnion::Circle(h) => {p}::union_json(\"kind\", \"circle\", \
                 vec![(\"radius\", h.radius.to_string())]), MShapeUnion::Disc(h) => \
                 {p}::union_json(\"kind\", \"disc\", vec![(\"radius\", h.radius.to_string())]), }}"
            ))
        );
    }

    /// A read of a union's member (docs/adr/0092, section 6.1): the tag by its
    /// method, a field of the one case left through that case's proven read,
    /// and a field two cases share through its accessor.
    #[test]
    fn a_proven_read_is_spelled_by_the_cases_left() {
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
        assert_eq!(rust_union_member(union, &all, "kind"), Ok("kind()".into()));
        assert_eq!(
            rust_union_member(union, &[case("disc")], "radius"),
            Ok("proven_disc().radius".into())
        );
        assert_eq!(
            rust_union_member(union, &[case("circle"), case("disc")], "radius"),
            Ok("proven_field_radius()".into())
        );
        assert!(rust_union_member(union, &[case("dot")], "radius").is_err());
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
