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
//! | Kotlin | `List<T>` | `String` / `ByteArray` | `T?` | the machine's data class |
//! | Go | `[]T` | `string` / `[]byte` | `optional.Of[T]` | the machine's struct |
//! | Python | `list[T]` | `str` / `bytes` | `Optional[T]` | the machine's record class |
//! | C++ | `std::vector<T>` | `std::string` / `std::vector<uint8_t>` | `std::optional<T>` | the machine's struct |
//!
//! On Rust a list, a text and a byte string carry their bound in their type
//! (section 7.4), so a value is copied into a place of a larger bound by
//! widening it at each depth where the two bounds differ, the fit proven before
//! ([`copier`], section 5.1) and asserted again by the widening's own const
//! parameters. Kotlin's, Go's, Python's and C++'s types carry no bound
//! (section 7.4, last paragraph), so two places whose types differ only in
//! their bounds are one type, and a value that fits is copied as it is: on Go
//! the list a place holds into a slice of its own, since the variable it is
//! copied into is appended to in place, on Python into a copy that shares no
//! list with it, for the same reason, and on C++ by the assignment, which
//! copies a list by value at every depth.

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
    /// `alias`, one member per field, `(id, type)`, in the schema's order, and
    /// `checks`, what a value of it runs over its fields where it is made
    /// ([`Self::field_checks`]).
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        checks: &[String],
    ) -> String;
    /// The statements that refuse, where a record is made, a value of one of
    /// `fields` (each its id with its type, in the schema's order) past a
    /// bound its type declares at any depth (docs/adr/0086, section 5.4): a
    /// host's value is refused where it is made, so a value that exists is one
    /// of its type. None for a backend whose types hold every bound themselves
    /// (Rust, section 7.4), which a value past it is no value of.
    fn field_checks(&self, _fields: &[(&str, &ValueType)]) -> Vec<String> {
        Vec::new()
    }

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
    /// present value bound to [`Self::read_binding`], and `elem_ty` the type the
    /// machine holds that present value in, which a backend whose function
    /// literals name their types (Go) spells the reader's literal with.
    fn read_optional(&self, value: &str, elem: &str, elem_ty: &str) -> String;
    /// The reader of a list of at least `least` and at most `max` elements,
    /// `elem` the reader of an element bound to [`Self::read_binding`], and
    /// `elem_ty` the type the machine holds an element in; one past the bound,
    /// or short of the lower count, is refused before any element is read
    /// (docs/adr/0086, section 15.5). `least` is 0 for a list that states no
    /// lower count.
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, elem_ty: &str) -> String;
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
    /// The value a `<send>` holds one of its params in, `written` being what
    /// this spelling's writers wrote the param's composed value as
    /// (docs/adr/0086, section 5.3): `written` itself where the writers write
    /// that value (Rust's `ScriptValue`, Kotlin's `List` and `Map`), and the
    /// holder made of it where they write the plain value the holder is made
    /// from (Python's `ScriptValue`, made of a `list` or a `dict`).
    fn send_param(&self, written: String) -> String {
        written
    }

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
    /// present value, a reference bound to [`Self::copy_binding`], into a place
    /// whose present value the machine holds in `elem_ty`.
    fn copy_optional(&self, held: &Held, elem: &str, elem_ty: &str) -> String;
    /// `value`, a copy of a value of `T`, as the present value of a place of
    /// `optional<T>`.
    fn copy_present(&self, value: &str) -> String;
    /// A copy of the list `held`, `elem` the copy of an element, a reference
    /// bound to [`Self::copy_binding`], into a place whose element the machine
    /// holds in `elem_ty`.
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String;

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
    // The type the place copied into holds a part in, which a backend whose
    // function literals name their types spells the copy of the part with.
    let typed = |to: &ValueType| machine_type(to, to_within, records, enums, spelling);
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
            Ok(spelling.copy_optional(held, &part(from, to, &element)?, &typed(to)?))
        }
        (from, ValueType::Optional { elem: to }) => {
            Ok(spelling.copy_present(&part(from, to, held)?))
        }
        (ValueType::List { elem: from, .. }, ValueType::List { elem: to, .. }) => {
            Ok(spelling.copy_list(held, &part(from, to, &element)?, &typed(to)?))
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

/// The first union a value of `ty`, written in the scope `within` names,
/// holds at any depth, through the lists, optional values and records that
/// hold it, as it is written (`union:Shape`); `None` for a type that holds
/// none. A record the statechart does not hold is passed over: the walk that
/// reads it refuses it in its own words. A lowering that holds no union asks
/// this before a walk could meet one (docs/adr/0092, section 7).
pub fn union_reached(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
) -> Option<ValueType> {
    fn walk<'r>(
        ty: &ValueType,
        within: Option<&str>,
        records: &'r dyn RecordEnv,
        seen: &mut std::collections::BTreeSet<&'r str>,
    ) -> Option<ValueType> {
        match ty {
            ValueType::Union { .. } => Some(ty.clone()),
            ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => None,
            ValueType::Optional { elem } | ValueType::List { elem, .. } => {
                walk(elem, within, records, seen)
            }
            ValueType::Record { alias } => {
                let held = records.record(within, alias).ok()?;
                // Each record is walked once, so the walk ends whatever names
                // what.
                if !seen.insert(held.alias) {
                    return None;
                }
                held.schema
                    .fields
                    .iter()
                    .find_map(|field| walk(&field.sce_type, Some(held.alias), records, seen))
            }
        }
    }
    walk(ty, within, records, &mut std::collections::BTreeSet::new())
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

/// What a value of the record the statechart holds as `alias` runs over its
/// fields where it is made ([`ComposedSpelling::field_checks`]), in the
/// schema's order: what holds a host's value to its type (docs/adr/0086,
/// section 5.4).
pub fn record_checks(
    alias: &str,
    records: &dyn RecordEnv,
    spelling: &dyn ComposedSpelling,
) -> Result<Vec<String>, String> {
    let held = records.record(None, alias)?;
    let fields: Vec<(&str, &ValueType)> = held
        .schema
        .fields
        .iter()
        .map(|field| (field.id.as_str(), &field.sce_type))
        .collect();
    Ok(spelling.field_checks(&fields))
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
    // The type the machine holds a part in, which a backend whose function
    // literals name their types spells the reader of the part with.
    let typed = |part: &ValueType| machine_type(part, within, records, enums, spelling);
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
            &typed(elem)?,
        ),
        ValueType::List { elem, max } => spelling.read_list(
            value,
            max.min,
            max.max,
            &reader(elem, within, records, enums, spelling, bound)?,
            &typed(elem)?,
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
    // no default, since an enum a field holds has none. Every bound is held by
    // a member's type (section 7.4), so there is nothing to check.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
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
    // A Rust closure's types are inferred, so the element's type is not
    // spelled.
    fn read_optional(&self, value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{value}.optional(|v| {elem})")
    }
    // The runtime's reader answers the `BoundedList<T, N>` the machine holds;
    // a lower count is held by the reader too, since no Rust type holds it
    // (docs/adr/0086, section 15.6).
    fn read_list(&self, value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
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
    fn copy_optional(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
        format!("{}.as_ref().map(|v| {elem})", held.text)
    }
    fn copy_present(&self, value: &str) -> String {
        format!("Some({value})")
    }
    // Each element copied into the place's element type, the list's own bound
    // widened into the place's by type; the place's type names both.
    fn copy_list(&self, held: &Held, elem: &str, _elem_ty: &str) -> String {
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

/// The C++ spellings of the walks, for the machine `machine` (section 7.1, the
/// C++ row): the runtime's `EventPayloadValue` (`common/EventPayloadLift.h`)
/// reads each constructor, and its writers write them into `carrier`.
///
/// A C++ reader fills a place and answers its refusal, so a reader here is a
/// lambda of the value and the place (`CPP_READER`), which the reader of
/// the part that holds it hands to the runtime's combinator (`readList`,
/// `readOptional`, `readRecord`); the place's type is deduced, so no reader
/// names one. A writer and a copy walk a list by a range `for` and a value
/// that may be absent by its `has_value`, each in a lambda the value is handed
/// to as its argument. A binding of a nested lambda is its own parameter or
/// local, which hides its holder's of the same name: the holder's value was
/// handed in before the inner one is bound, and each part reads its own.
pub struct CppComposed<'a> {
    pub machine: &'a str,
    pub carrier: CppCarrier,
}

/// What a C++ writer spells a composed value into: one walk of the value's
/// type, written as the carrier the place that takes it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CppCarrier {
    /// The JSON text itself, by the runtime's `EventPayloadFields::json`,
    /// `list` and `record`: what the inject seam writes as the event's `data`
    /// beside the typed carrier (section 3.1).
    Json,
    /// The `ScriptValue` a `<send>` holds each param in
    /// (`StaticTarget::wire_value`) until `EventDataHelper::buildEventDataJson`
    /// writes them as one JSON object: a list as a `ScriptArray` of its
    /// elements, a record as a `ScriptObject` of its fields, whose keys the
    /// writer orders by their bytes at every depth, an absent value as
    /// `ScriptNull`, and every scalar as a `<send>`'s scalar param is (section
    /// 5.3; ARCHITECTURE.md, "JSON Object Key Order").
    Wire,
}

/// The namespace of the C++ runtime's payload reader, writers and checks
/// (`sce/include/common/EventPayloadLift.h`).
const CPP_COMMON: &str = "::SCE::Common";

/// The name a C++ reader binds the value it reads to: the first parameter of
/// [`CPP_READER`].
const CPP_READ_BINDING: &str = "sceValue";

/// The head of every C++ reader: a lambda of the value it reads and the place
/// it fills, whose type is deduced where the runtime's combinator hands it
/// one, answering the refusal of a value outside its type, empty where it
/// filled the place.
const CPP_READER: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) \
                          -> ::SCE::Common::EventPayloadRefusal";

/// The name a C++ writer binds an element or a present value to.
const CPP_ELEMENT_BINDING: &str = "sceElement";

/// The name a C++ copy binds an element or a present value to.
const CPP_COPY_BINDING: &str = "sceCopied";

/// Why a C++ walk meets no union: the one decision of what is lowered refuses
/// a union on C++ before any walk reaches one
/// ([`crate::forge::event_schema_check::ComposedLowering::unions`]).
const CPP_HOLDS_NO_UNION: &str = "a C++ machine holds no union yet (docs/adr/0092, section 7): \
     the one decision of what is lowered refuses a union on C++ before any walk reaches one";

/// The check of a C++ value of `ty`, written in the scope `within` names, at
/// `value`, named `path` (an expression of a `std::string`): an expression of
/// type `std::string` that is the refusal, naming the path to the place, of a
/// value past a bound its type declares at any depth, and empty where there
/// is none (docs/adr/0086, section 5.4). One call of the runtime's
/// `EventPayloadBounds::text`, `bytes`, `list` or `listWithin` per
/// constructor that holds a bound, and of `optional` for a value that may be
/// absent, the check of an element or a present value inside the lambda the
/// call hands it to, its parameters named by their `depth`; a record is
/// checked field by field where it stands, at the path of each field below
/// its own. `None` for a type that holds no bound at any depth: a number, an
/// enum, an optional one of them and a record of those.
///
/// A C++ struct has no constructor that could refuse a value, so a C++ raise
/// seam runs this check over the payload a host hands it before it encodes
/// it, a record a list holds included.
fn cpp_bound_check(
    ty: &ValueType,
    within: Option<&str>,
    path: &str,
    value: &str,
    depth: usize,
    records: &dyn RecordEnv,
) -> Result<Option<String>, String> {
    let bounds = format!("{CPP_COMMON}::EventPayloadBounds");
    let (inner_path, inner_value) = (format!("scePath{depth}"), format!("sceValue{depth}"));
    // The lambda the runtime hands a part to, at the part's own path.
    let part = |inner: &str| {
        format!(
            "[&](const ::std::string &{inner_path}, const auto &{inner_value}) -> ::std::string \
             {{ return {inner}; }}"
        )
    };
    Ok(match ty {
        ValueType::Text { max } => Some(format!("{bounds}::text({path}, {value}, {}u)", max.max)),
        ValueType::Bytes { max } => Some(format!("{bounds}::bytes({path}, {value}, {}u)", max.max)),
        // The count is held whatever the elements are; an element with
        // nothing of its own to check is handed to a lambda that checks
        // nothing.
        ValueType::List { elem, max } => {
            let each =
                match cpp_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?
                {
                    Some(inner) => part(&inner),
                    None => {
                        "[](const ::std::string &, const auto &) -> ::std::string { return {}; }"
                            .to_string()
                    }
                };
            Some(if max.min == 0 {
                format!("{bounds}::list({path}, {value}, {}u, {each})", max.max)
            } else {
                format!(
                    "{bounds}::listWithin({path}, {value}, {}u, {}u, {each})",
                    max.min, max.max
                )
            })
        }
        // An absent value holds nothing to check; a present one is checked at
        // the place's own path, which the runtime hands the lambda.
        ValueType::Optional { elem } => {
            let Some(inner) =
                cpp_bound_check(elem, within, &inner_path, &inner_value, depth + 1, records)?
            else {
                return Ok(None);
            };
            Some(format!(
                "{bounds}::optional({path}, {value}, {})",
                part(&inner)
            ))
        }
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            let mut checks = Vec::new();
            for field in &held.schema.fields {
                let below = format!(
                    "{path} + \"/{}\"",
                    crate::filters::escape_cpp(field.id.clone())
                );
                if let Some(check) = cpp_bound_check(
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
            match checks.as_slice() {
                [] => None,
                [one] => Some(one.clone()),
                many => {
                    let each: String = many
                        .iter()
                        .map(|check| {
                            format!(
                                "sceRefused = {check}; if (!sceRefused.empty()) {{ return \
                                 sceRefused; }} "
                            )
                        })
                        .collect();
                    Some(format!(
                        "[&]() -> ::std::string {{ ::std::string sceRefused; {each}return \
                         sceRefused; }}()"
                    ))
                }
            }
        }
        ValueType::Scalar(_) => None,
        ValueType::Union { .. } => return Err(CPP_HOLDS_NO_UNION.to_string()),
    })
}

/// The checks a C++ raise seam runs over the fields of the payload a host
/// hands it, each a parameter of the seam named by the field's id, before it
/// encodes any of it ([`cpp_bound_check`]): one expression of type
/// `std::string` per field whose type holds a bound at any depth, in the
/// schema's order, the path of each the field's id.
pub(crate) fn cpp_raise_checks(
    held: HeldRecord<'_>,
    records: &dyn RecordEnv,
) -> Result<Vec<String>, String> {
    let mut checks = Vec::new();
    for field in &held.schema.fields {
        let path = format!(
            "::std::string(\"{}\")",
            crate::filters::escape_cpp(field.id.clone())
        );
        if let Some(check) = cpp_bound_check(
            &field.sce_type,
            Some(held.alias),
            &path,
            &field.id,
            0,
            records,
        )? {
            checks.push(check);
        }
    }
    Ok(checks)
}

impl CppComposed<'_> {
    /// The runtime's JSON of the scalar `value`, at the spelling its type
    /// deserves (`EventPayloadFields::json`).
    fn json(value: &str) -> String {
        format!("{CPP_COMMON}::EventPayloadFields::json({value})")
    }
}

impl ComposedSpelling for CppComposed<'_> {
    fn scalar_type(&self, ty: &SceType) -> String {
        crate::forge::generator::cpp_type(ty).to_string()
    }
    // The C++ row of section 7.1: no type has a bound as its parameter yet
    // (section 7.4, last paragraph), so the reader holds a value to its bound
    // where it is read, and a raise seam where a host hands it one.
    fn text_type(&self, _max: u32) -> String {
        "std::string".to_string()
    }
    fn bytes_type(&self, _max: u32) -> String {
        "std::vector<uint8_t>".to_string()
    }
    // The one C++ spelling of a value that may be absent (docs/adr/0055,
    // section 5).
    fn optional_type(&self, elem: &str) -> String {
        format!("std::optional<{elem}>")
    }
    fn list_type(&self, elem: &str, _max: u32) -> String {
        format!("std::vector<{elem}>")
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
    // Plain data by the record rule: a struct of the schema's fields in the
    // schema's order, each named as the schema names it, which is how a read
    // names it, and which a host builds by position in that order
    // (docs/adr/0059, section 5). A C++ struct has no constructor that could
    // refuse a value, so the raise seam that takes one holds it to every bound
    // its type declares before it sends it (docs/adr/0086, section 5.4), and
    // no check is declared beside it.
    fn record_def(
        &self,
        ty: &str,
        alias: &str,
        fields: &[(String, String)],
        _checks: &[String],
    ) -> String {
        let members: String = fields
            .iter()
            .map(|(id, ty)| format!("    {ty} {id};\n"))
            .collect();
        format!(
            "/// SCE Accepted Subset §2.15: a `record:{alias}` value that holds a composed\n\
             /// value (docs/adr/0086), built by position in the schema's order; a raise\n\
             /// seam holds it to every bound its type declares before it sends it.\n\
             struct {ty} {{\n{members}}};"
        )
    }

    fn read_binding(&self) -> &'static str {
        CPP_READ_BINDING
    }
    // The width is the place's own type, which the runtime's reader is
    // instantiated at, so a field widened in its schema cannot keep being read
    // at the old width.
    fn read_scalar(&self, ty: &SceType, _value: &str) -> String {
        let read = match ty {
            SceType::Uint64 | SceType::Int64 => "readWide",
            SceType::Float32 | SceType::Float64 => "readFloating",
            SceType::Bool => "readBool",
            SceType::Uint8
            | SceType::Uint16
            | SceType::Uint32
            | SceType::Int8
            | SceType::Int16
            | SceType::Int32 => "readInteger",
            SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                "a text, a byte string and an enum are cases of their own in the walk, never \
                 a scalar of a value type"
            ),
        };
        format!("{CPP_READER} {{ return sceValue.{read}(sceInto); }}")
    }
    // The variant whose declared name the wire carries; a name the enum does
    // not declare is refused with the code and pointer of the refusal.
    fn read_enum(&self, held: &MachineEnum, _value: &str) -> String {
        let arms: String = held
            .variants
            .iter()
            .map(|(declared, ident)| {
                format!(
                    "if (sceName == \"{}\") {{ sceInto = {}::{ident}; return {{}}; }} ",
                    crate::filters::escape_cpp(declared.clone()),
                    held.ty
                )
            })
            .collect();
        format!(
            "{CPP_READER} {{ ::std::string sceName; {CPP_COMMON}::EventPayloadRefusal sceNamed = \
             sceValue.readName(sceName); if (!sceNamed.empty()) {{ return sceNamed; }} {arms}return \
             sceValue.unknownVariant(sceName, \"{}\"); }}",
            crate::filters::escape_cpp(held.alias.clone())
        )
    }
    fn read_text(&self, max: u32, _value: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readText({max}u, sceInto); }}")
    }
    fn read_bytes(&self, max: u32, _value: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readBytes({max}u, sceInto); }}")
    }
    fn read_optional(&self, _value: &str, elem: &str, _elem_ty: &str) -> String {
        format!("{CPP_READER} {{ return sceValue.readOptional(sceInto, {elem}); }}")
    }
    // The bound, and a lower count where the type states one, are held before
    // any element is read (docs/adr/0086, section 15.5).
    fn read_list(&self, _value: &str, least: u32, max: u32, elem: &str, _elem_ty: &str) -> String {
        if least == 0 {
            format!("{CPP_READER} {{ return sceValue.readList({max}u, sceInto, {elem}); }}")
        } else {
            format!(
                "{CPP_READER} {{ return sceValue.readListWithin({least}u, {max}u, sceInto, {elem}); \
                 }}"
            )
        }
    }
    // Each field read into the struct in the schema's order by its own reader,
    // the first refusal returned as it is; a field a later revision added is
    // read out of the defaults where the value lacks it.
    fn read_record(
        &self,
        _value: &str,
        defaults: Option<&str>,
        _ty: &str,
        fields: &[(String, String)],
    ) -> String {
        let defaults = defaults.map_or_else(
            || "nullptr".to_string(),
            |json| format!("\"{}\"", crate::filters::escape_cpp(json.to_string())),
        );
        // A record of no fields fills nothing of its place.
        if fields.is_empty() {
            return format!(
                "{CPP_READER} {{ (void)sceInto; return sceValue.readRecord({defaults}, [](const \
                 {CPP_COMMON}::EventPayloadFields &) -> {CPP_COMMON}::EventPayloadRefusal {{ \
                 return {{}}; }}); }}"
            );
        }
        let members: String = fields
            .iter()
            .map(|(id, read)| {
                format!(
                    "sceRefused = sceFields.value(\"{}\", sceField); if (!sceRefused.empty()) {{ \
                     return sceRefused; }} sceRefused = ({read})(sceField, sceInto.{id}); if \
                     (!sceRefused.empty()) {{ return sceRefused; }} ",
                    crate::filters::escape_cpp(id.clone())
                )
            })
            .collect();
        format!(
            "{CPP_READER} {{ return sceValue.readRecord({defaults}, [&](const \
             {CPP_COMMON}::EventPayloadFields &sceFields) -> {CPP_COMMON}::EventPayloadRefusal {{ \
             {CPP_COMMON}::EventPayloadValue sceField; {CPP_COMMON}::EventPayloadRefusal \
             sceRefused; {members}return {{}}; }}); }}"
        )
    }

    fn write_binding(&self) -> &'static str {
        CPP_ELEMENT_BINDING
    }
    fn write_scalar(&self, ty: &SceType, held: &Held) -> String {
        let value = &held.text;
        let wide = matches!(ty, SceType::Uint64 | SceType::Int64);
        match self.carrier {
            // The one spelling of a payload field on the wire the inject seam
            // of a schema of entries writes too: a 64-bit integer as the text
            // of its digits.
            CppCarrier::Json if wide => Self::json(&format!("::std::to_string({value})")),
            CppCarrier::Json => Self::json(value),
            // As a scalar param is (`StaticTarget::wire_value`): an integer of
            // at most 32 bits widened to `int64_t` and a real to `double`,
            // both exactly, and a 64-bit integer as the text of its digits.
            CppCarrier::Wire => match ty {
                SceType::Uint8
                | SceType::Uint16
                | SceType::Uint32
                | SceType::Int8
                | SceType::Int16
                | SceType::Int32 => format!(
                    "ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>({value}))"
                ),
                SceType::Uint64 | SceType::Int64 => {
                    format!("ScriptValue(std::in_place_type<std::string>, std::to_string({value}))")
                }
                SceType::Float32 | SceType::Float64 => {
                    format!("ScriptValue(std::in_place_type<double>, static_cast<double>({value}))")
                }
                SceType::Bool => format!("ScriptValue(std::in_place_type<bool>, {value})"),
                SceType::String | SceType::Bytes | SceType::Enum(_) => unreachable!(
                    "a text, a byte string and an enum are cases of their own in the walk, \
                     never a scalar of a value type"
                ),
            },
        }
    }
    // The machine's enum's `sceLogName` answers the name its document
    // declares.
    fn write_enum(&self, held: &Held) -> String {
        let name = format!("::std::string(sceLogName({}))", held.text);
        match self.carrier {
            CppCarrier::Json => Self::json(&name),
            CppCarrier::Wire => format!("ScriptValue(std::in_place_type<std::string>, {name})"),
        }
    }
    fn write_text(&self, held: &Held) -> String {
        match self.carrier {
            CppCarrier::Json => Self::json(&held.text),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::string>, {})",
                held.text
            ),
        }
    }
    // JSON has no byte string: it crosses as its byte-exact Latin-1 text,
    // which `EventPayloadFields::json` writes a `std::vector<uint8_t>` as.
    fn write_bytes(&self, held: &Held) -> String {
        match self.carrier {
            CppCarrier::Json => Self::json(&held.text),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::string>, ::SCE::Latin1Bytes::textOf({}))",
                held.text
            ),
        }
    }
    // The present value where there is one, and `null` where there is none,
    // as an absent scalar param is. The value is the argument of the lambda
    // that writes it, read once, so no binding inside it names a place
    // outside.
    fn write_optional(&self, held: &Held, elem: &str) -> String {
        let (result, absent) = match self.carrier {
            CppCarrier::Json => ("::std::string", "\"null\""),
            CppCarrier::Wire => ("ScriptValue", "ScriptValue(ScriptNull{})"),
        };
        format!(
            "[&](const auto &sceOptional) -> {result} {{ if (!sceOptional.has_value()) {{ return \
             {absent}; }} const auto &{CPP_ELEMENT_BINDING} = *sceOptional; return {elem}; }}({})",
            held.text
        )
    }
    // Each element written in the list's order.
    fn write_list(&self, held: &Held, elem: &str) -> String {
        match self.carrier {
            CppCarrier::Json => format!(
                "{CPP_COMMON}::EventPayloadFields::list({}, [&](const auto &{CPP_ELEMENT_BINDING}) \
                 -> ::std::string {{ return {elem}; }})",
                held.text
            ),
            CppCarrier::Wire => format!(
                "[&](const auto &sceList) -> ScriptValue {{ auto sceItems = \
                 ::std::make_shared<ScriptArray>(); for (const auto &{CPP_ELEMENT_BINDING} : \
                 sceList) {{ sceItems->elements.push_back({elem}); }} return \
                 ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                 ::std::move(sceItems)); }}({})",
                held.text
            ),
        }
    }
    fn write_field(&self, held: &Held, field: &str) -> String {
        format!("{}.{field}", held.text)
    }
    // The runtime writes the keys ascending by their bytes at every depth
    // (ARCHITECTURE.md, "JSON Object Key Order"), `EventPayloadFields::record`
    // and the writer of a send's typed params alike.
    fn write_record(&self, fields: &[(String, String)]) -> String {
        let members: Vec<String> = fields
            .iter()
            .map(|(id, value)| {
                format!(
                    "{{\"{}\", {value}}}",
                    crate::filters::escape_cpp(id.clone())
                )
            })
            .collect();
        let members = members.join(", ");
        match self.carrier {
            CppCarrier::Json => format!("{CPP_COMMON}::EventPayloadFields::record({{{members}}})"),
            CppCarrier::Wire => format!(
                "ScriptValue(std::in_place_type<std::shared_ptr<ScriptObject>>, \
                 ::std::make_shared<ScriptObject>(ScriptObject{{{members}}}))"
            ),
        }
    }

    fn copy_binding(&self) -> &'static str {
        CPP_COPY_BINDING
    }
    // C++ copies by value: an assignment of a list, a record or an optional
    // one copies every element at every depth, so the variable it is copied
    // into, which is appended to and emptied in place, shares nothing with
    // the place the value was taken from.
    fn copy_whole(&self, held: &Held, _plain: bool) -> String {
        held.text.clone()
    }
    // A text and a byte string are one C++ type whatever their bound, so a
    // walk that copies by type never reaches these; the value is the copy.
    fn copy_text(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    fn copy_bytes(&self, _max: u32, held: &Held) -> String {
        held.text.clone()
    }
    // The value is the argument of the lambda that copies it, as a writer's
    // is.
    fn copy_optional(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "[&](const auto &sceOptional) -> std::optional<{elem_ty}> {{ if \
             (!sceOptional.has_value()) {{ return std::nullopt; }} const auto &{CPP_COPY_BINDING} \
             = *sceOptional; return {elem}; }}({})",
            held.text
        )
    }
    // A `T` converts to a `std::optional<T>` where one is expected.
    fn copy_present(&self, value: &str) -> String {
        value.to_string()
    }
    fn copy_list(&self, held: &Held, elem: &str, elem_ty: &str) -> String {
        format!(
            "[&](const auto &sceList) -> std::vector<{elem_ty}> {{ std::vector<{elem_ty}> \
             sceItems; sceItems.reserve(sceList.size()); for (const auto &{CPP_COPY_BINDING} : \
             sceList) {{ sceItems.push_back({elem}); }} return sceItems; }}({})",
            held.text
        )
    }

    // A C++ machine holds no union yet: the gate refuses one by name before a
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
        crate::forge::enum_naming::variant_ident(crate::generator::Language::Cpp, "", variant)
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
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn union_value(&self, _ty: &str, _case: &str, _record: Option<&str>) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn read_union(&self, _value: &str, _read: &UnionRead<'_>) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn write_union(&self, _held: &Held, _ty: &str, _tag: &str, _arms: &[UnionWriteArm]) -> String {
        unreachable!("{CPP_HOLDS_NO_UNION}")
    }
    fn union_member(
        &self,
        _union: &ScopedUnion,
        _left: &[&ScopedCase],
        _member: &str,
    ) -> Result<String, String> {
        Err(CPP_HOLDS_NO_UNION.to_string())
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
        Err(CPP_HOLDS_NO_UNION.to_string())
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
                name: "shape_kind".to_string(),
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
        // Every bound is held by a member's type, so a Rust record checks
        // nothing where it is made.
        let checks = record_checks("Item", &held, &RUST).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = RUST.record_def("MItemRecord", "Item", &members, &checks);
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

    const KOTLIN: KotlinComposed<'static> = KotlinComposed {
        machine: "M",
        carrier: KotlinCarrier::Json,
    };

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
        // A Rust record holds every bound in a member's type.
        let held = held();
        assert!(record_checks("Batch", &held, &RUST)
            .expect("checks")
            .is_empty());
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

    /// The first union a type holds at any depth, through a list, an optional
    /// value and a record, and none where it holds none.
    #[test]
    fn a_union_is_found_at_any_depth_or_not_at_all() {
        let shapes = shapes();
        assert_eq!(
            union_reached(&ty("list<optional<union:Shape>>[3]"), None, &shapes),
            Some(ty("union:Shape"))
        );
        assert_eq!(union_reached(&ty("list<uint8>[3]"), None, &shapes), None);
        assert_eq!(union_reached(&ty("record:Radius"), None, &shapes), None);
        let held = held();
        assert_eq!(union_reached(&ty("record:Batch"), None, &held), None);
    }

    const PYTHON: PythonComposed<'static> = PythonComposed {
        machine: "M",
        carrier: PythonCarrier::Json,
    };

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

    const CPP: CppComposed<'static> = CppComposed {
        machine: "M",
        carrier: CppCarrier::Json,
    };

    /// The head of every C++ reader, as the tests below spell what they
    /// expect of one.
    const R: &str = "[&](const ::SCE::Common::EventPayloadValue &sceValue, auto &sceInto) -> \
                     ::SCE::Common::EventPayloadRefusal";

    /// The C++ row of section 7.1: no type carries a bound, a value that may
    /// be absent is `std::optional` over its type, a record is the machine's
    /// struct, at any depth; no check is declared beside a record, since a C++
    /// struct has no constructor that could refuse a value.
    #[test]
    fn a_cpp_machine_holds_each_constructor_in_its_own_type_at_any_depth() {
        let held = held();
        let enums = enums();
        let spelled =
            |text: &str| machine_type(&ty(text), None, &held, &enums, &CPP).expect("a type");
        assert_eq!(spelled("uint16"), "uint16_t");
        assert_eq!(spelled("string[8]"), "std::string");
        assert_eq!(spelled("bytes[4]"), "std::vector<uint8_t>");
        assert_eq!(spelled("optional<int64>"), "std::optional<int64_t>");
        assert_eq!(
            spelled("list<list<uint8>[2]>[3]"),
            "std::vector<std::vector<uint8_t>>"
        );
        assert_eq!(spelled("list<enum:Level>[2]"), "std::vector<MLevelEnum>");
        assert_eq!(spelled("list<record:Item>[3]"), "std::vector<MItemRecord>");
        assert_eq!(
            spelled("optional<list<uint8>[2]>"),
            "std::optional<std::vector<uint8_t>>"
        );
        assert_eq!(
            spelled("list<optional<uint16>>[2]"),
            "std::vector<std::optional<uint16_t>>"
        );
        let members = record_members("Item", &held, &enums, &CPP).expect("members");
        let checks = record_checks("Item", &held, &CPP).expect("checks");
        assert!(checks.is_empty(), "{checks:?}");
        let def = CPP.record_def("MItemRecord", "Item", &members, &checks);
        assert!(
            def.ends_with(
                "struct MItemRecord {\n    uint32_t id;\n    MLevelEnum level;\n    \
                 std::optional<int64_t> due;\n    std::vector<uint8_t> blob;\n    \
                 std::vector<std::string> tags;\n};"
            ),
            "{def}"
        );
    }

    /// One lambda of the value and the place per constructor, each calling
    /// the runtime's reader of its constructor, the reader of a part handed to
    /// the combinator of its holder; a bound and a lower count handed to the
    /// list that holds them, a record read field by field into its struct with
    /// its defaults.
    #[test]
    fn a_cpp_reader_is_one_lambda_per_constructor() {
        let held = held();
        let enums = enums();
        let read = |text: &str| reader(&ty(text), None, &held, &enums, &CPP, "v").expect("read");
        let whole = format!("{R} {{ return sceValue.readInteger(sceInto); }}");
        assert_eq!(read("uint8"), whole);
        assert_eq!(read("int16"), whole);
        let wide = format!("{R} {{ return sceValue.readWide(sceInto); }}");
        assert_eq!(read("uint64"), wide);
        assert_eq!(read("int64"), wide);
        assert_eq!(
            read("bool"),
            format!("{R} {{ return sceValue.readBool(sceInto); }}")
        );
        assert_eq!(
            read("float32"),
            format!("{R} {{ return sceValue.readFloating(sceInto); }}")
        );
        assert_eq!(
            read("string[8]"),
            format!("{R} {{ return sceValue.readText(8u, sceInto); }}")
        );
        assert_eq!(
            read("bytes[4]"),
            format!("{R} {{ return sceValue.readBytes(4u, sceInto); }}")
        );
        assert_eq!(
            read("optional<int64>"),
            format!("{R} {{ return sceValue.readOptional(sceInto, {wide}); }}")
        );
        assert_eq!(
            read("list<list<uint8>[2]>[3]"),
            format!(
                "{R} {{ return sceValue.readList(3u, sceInto, {R} {{ return \
                 sceValue.readList(2u, sceInto, {whole}); }}); }}"
            )
        );
        assert_eq!(
            read("list<list<uint8>[1..2]>[2..3]"),
            format!(
                "{R} {{ return sceValue.readListWithin(2u, 3u, sceInto, {R} {{ return \
                 sceValue.readListWithin(1u, 2u, sceInto, {whole}); }}); }}"
            )
        );
        assert_eq!(
            read("enum:Level"),
            format!(
                "{R} {{ ::std::string sceName; ::SCE::Common::EventPayloadRefusal sceNamed = \
                 sceValue.readName(sceName); if (!sceNamed.empty()) {{ return sceNamed; }} if \
                 (sceName == \"low\") {{ sceInto = MLevelEnum::Low; return {{}}; }} if (sceName \
                 == \"high\") {{ sceInto = MLevelEnum::High; return {{}}; }} return \
                 sceValue.unknownVariant(sceName, \"Level\"); }}"
            )
        );
        let item = read("record:Item");
        assert!(
            item.starts_with(&format!(
                "{R} {{ return sceValue.readRecord(nullptr, [&](const \
                 ::SCE::Common::EventPayloadFields &sceFields) -> \
                 ::SCE::Common::EventPayloadRefusal {{ ::SCE::Common::EventPayloadValue \
                 sceField; ::SCE::Common::EventPayloadRefusal sceRefused; sceRefused = \
                 sceFields.value(\"id\", sceField); if (!sceRefused.empty()) {{ return \
                 sceRefused; }} sceRefused = ({whole})(sceField, sceInto.id); if \
                 (!sceRefused.empty()) {{ return sceRefused; }} "
            )),
            "{item}"
        );
        assert!(
            item.ends_with(&format!(
                "sceRefused = ({R} {{ return sceValue.readList(4u, sceInto, {R} {{ return \
                 sceValue.readText(8u, sceInto); }}); }})(sceField, sceInto.tags); if \
                 (!sceRefused.empty()) {{ return sceRefused; }} return {{}}; }}); }}"
            )),
            "{item}"
        );
        // A list the second revision added reads empty where a delivery lacks it.
        let batch = read("record:Batch");
        assert!(
            batch.starts_with(&format!(
                "{R} {{ return sceValue.readRecord(\"{{\\\"late\\\":[]}}\", "
            )),
            "{batch}"
        );
    }

    /// The JSON a value is spelled as, one call of the runtime's writers per
    /// constructor: a list by `EventPayloadFields::list`, a value that may be
    /// absent handed to a lambda that writes `null` where it is absent, a
    /// record by its field ids, which the runtime orders.
    #[test]
    fn a_cpp_writer_is_one_runtime_call_per_constructor() {
        let held = held();
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &CPP, &at).expect("write");
        let j = "::SCE::Common::EventPayloadFields::json";
        assert_eq!(write("uint8", Held::place("x.n")), format!("{j}(x.n)"));
        assert_eq!(
            write("int64", Held::place("x.n")),
            format!("{j}(::std::to_string(x.n))")
        );
        assert_eq!(write("bytes[4]", Held::place("x.b")), format!("{j}(x.b)"));
        assert_eq!(
            write("enum:Level", Held::reference("sceElement")),
            format!("{j}(::std::string(sceLogName(sceElement)))")
        );
        assert_eq!(
            write("list<optional<uint16>>[2]", Held::place("x.maybe")),
            format!(
                "::SCE::Common::EventPayloadFields::list(x.maybe, [&](const auto &sceElement) -> \
                 ::std::string {{ return [&](const auto &sceOptional) -> ::std::string {{ if \
                 (!sceOptional.has_value()) {{ return \"null\"; }} const auto &sceElement = \
                 *sceOptional; return {j}(sceElement); }}(sceElement); }})"
            )
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "::SCE::Common::EventPayloadFields::list(x.items, [&](const auto &sceElement) -> \
             ::std::string { return ::SCE::Common::EventPayloadFields::record({{\"id\", "
                .to_string(),
            format!("{{\"id\", {j}(sceElement.id)}}, "),
            format!("{{\"level\", {j}(::std::string(sceLogName(sceElement.level)))}}, "),
            format!(
                "{{\"due\", [&](const auto &sceOptional) -> ::std::string {{ if \
                 (!sceOptional.has_value()) {{ return \"null\"; }} const auto &sceElement = \
                 *sceOptional; return {j}(::std::to_string(sceElement)); }}(sceElement.due)}}, "
            ),
            format!("{{\"blob\", {j}(sceElement.blob)}}, "),
            format!(
                "{{\"tags\", ::SCE::Common::EventPayloadFields::list(sceElement.tags, [&](const \
                 auto &sceElement) -> ::std::string {{ return {j}(sceElement); }})}}}}); }})"
            ),
        ] {
            assert!(items.contains(&part), "expected `{part}` in:\n{items}");
        }
    }

    /// The value a C++ `<send>` holds a composed param in: a `ScriptValue`
    /// holding a `ScriptArray`, a `ScriptObject` whose keys the writer orders,
    /// `ScriptNull` where a value is absent, and each scalar as a scalar param
    /// is (a narrow integer widened to `int64_t`, a 64-bit one as its digits, a
    /// real as a `double`, a byte string as its Latin-1 text, an enum value as
    /// its declared name).
    #[test]
    fn a_cpp_send_writes_each_constructor_into_the_value_it_holds_params_in() {
        let held = held();
        let wire = CppComposed {
            machine: "M",
            carrier: CppCarrier::Wire,
        };
        let write =
            |text: &str, at: Held| writer(&ty(text), None, &held, &wire, &at).expect("write");
        assert_eq!(
            write("uint8", Held::place("x.n")),
            "ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>(x.n))"
        );
        assert_eq!(
            write("int64", Held::place("x.n")),
            "ScriptValue(std::in_place_type<std::string>, std::to_string(x.n))"
        );
        assert_eq!(
            write("float32", Held::place("x.n")),
            "ScriptValue(std::in_place_type<double>, static_cast<double>(x.n))"
        );
        assert_eq!(
            write("bool", Held::place("x.b")),
            "ScriptValue(std::in_place_type<bool>, x.b)"
        );
        let inner = "[&](const auto &sceList) -> ScriptValue { auto sceItems = \
                     ::std::make_shared<ScriptArray>(); for (const auto &sceElement : sceList) { \
                     sceItems->elements.push_back(ScriptValue(std::in_place_type<int64_t>, \
                     static_cast<int64_t>(sceElement))); } return \
                     ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                     ::std::move(sceItems)); }(sceElement)";
        assert_eq!(
            write("list<list<uint8>[2]>[2]", Held::place("x.grid")),
            format!(
                "[&](const auto &sceList) -> ScriptValue {{ auto sceItems = \
                 ::std::make_shared<ScriptArray>(); for (const auto &sceElement : sceList) {{ \
                 sceItems->elements.push_back({inner}); }} return \
                 ScriptValue(std::in_place_type<std::shared_ptr<ScriptArray>>, \
                 ::std::move(sceItems)); }}(x.grid)"
            )
        );
        assert_eq!(
            write("optional<list<uint8>[2]>", Held::place("x.spare")),
            format!(
                "[&](const auto &sceOptional) -> ScriptValue {{ if (!sceOptional.has_value()) {{ \
                 return ScriptValue(ScriptNull{{}}); }} const auto &sceElement = *sceOptional; \
                 return {inner}; }}(x.spare)"
            )
        );
        let items = write("list<record:Item>[3]", Held::place("x.items"));
        for part in [
            "sceItems->elements.push_back(ScriptValue(std::in_place_type<std::shared_ptr<\
             ScriptObject>>, ::std::make_shared<ScriptObject>(ScriptObject{{\"id\", \
             ScriptValue(std::in_place_type<int64_t>, static_cast<int64_t>(sceElement.id))}, ",
            "{\"level\", ScriptValue(std::in_place_type<std::string>, \
             ::std::string(sceLogName(sceElement.level)))}, ",
            "{\"blob\", ScriptValue(std::in_place_type<std::string>, \
             ::SCE::Latin1Bytes::textOf(sceElement.blob))}, ",
        ] {
            assert!(items.contains(part), "expected `{part}` in:\n{items}");
        }
        // The writers write the `ScriptValue` a send holds its params in.
        assert_eq!(wire.send_param("x".to_string()), "x");
    }

    /// Two places whose types differ only in their bounds are one C++ type,
    /// and a C++ assignment copies a list by value at every depth, so a value
    /// that fits is copied as it is; where the types differ, each part is
    /// copied into the place's type by a lambda the value is handed to.
    #[test]
    fn a_cpp_copy_of_a_value_that_fits_is_the_value() {
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
                &CPP,
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
            Ok(
                "[&](const auto &sceList) -> std::vector<std::optional<uint8_t>> { \
                std::vector<std::optional<uint8_t>> sceItems; \
                sceItems.reserve(sceList.size()); for (const auto &sceCopied : sceList) { \
                sceItems.push_back(sceCopied); } return sceItems; }(x)"
                    .to_string()
            )
        );
    }

    /// A C++ raise seam holds a host's value to every bound its type declares
    /// (docs/adr/0086, section 5.4): one check per constructor that holds a
    /// bound, an element's or a present value's inside the lambda the runtime
    /// hands it to, each parameter named by its depth; a lower count is held
    /// too; a record a list holds is checked field by field at the path below
    /// the element; a number, an enum and an optional one of them are not
    /// checked.
    #[test]
    fn a_cpp_raise_seam_checks_each_bound_before_it_encodes() {
        let held = held();
        let check = |text: &str, id: &str| {
            cpp_bound_check(
                &ty(text),
                None,
                &format!("::std::string(\"{id}\")"),
                id,
                0,
                &held,
            )
            .expect("a check")
        };
        let b = "::SCE::Common::EventPayloadBounds";
        let nothing = "[](const ::std::string &, const auto &) -> ::std::string { return {}; }";
        assert_eq!(check("uint8", "n"), None);
        assert_eq!(check("enum:Level", "level"), None);
        assert_eq!(check("optional<int64>", "due"), None);
        assert_eq!(
            check("list<list<uint8>[2]>[2..3]", "grid"),
            Some(format!(
                "{b}::listWithin(::std::string(\"grid\"), grid, 2u, 3u, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, {nothing}); }})"
            ))
        );
        assert_eq!(
            check("optional<list<string[4]>[2]>", "spare"),
            Some(format!(
                "{b}::optional(::std::string(\"spare\"), spare, [&](const ::std::string \
                 &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                 {b}::list(scePath0, sceValue0, 2u, [&](const ::std::string &scePath1, const \
                 auto &sceValue1) -> ::std::string {{ return {b}::text(scePath1, sceValue1, \
                 4u); }}); }})"
            ))
        );
        let batch = held.record(None, "Batch").expect("Batch");
        assert_eq!(
            cpp_raise_checks(batch, &held).expect("checks"),
            vec![
                format!(
                    "{b}::list(::std::string(\"items\"), items, 3u, [&](const ::std::string \
                     &scePath0, const auto &sceValue0) -> ::std::string {{ return [&]() -> \
                     ::std::string {{ ::std::string sceRefused; sceRefused = \
                     {b}::bytes(scePath0 + \"/blob\", sceValue0.blob, 4u); if \
                     (!sceRefused.empty()) {{ return sceRefused; }} sceRefused = \
                     {b}::list(scePath0 + \"/tags\", sceValue0.tags, 4u, [&](const \
                     ::std::string &scePath1, const auto &sceValue1) -> ::std::string {{ return \
                     {b}::text(scePath1, sceValue1, 8u); }}); if (!sceRefused.empty()) {{ return \
                     sceRefused; }} return sceRefused; }}(); }})"
                ),
                format!(
                    "{b}::list(::std::string(\"grid\"), grid, 2u, [&](const ::std::string \
                     &scePath0, const auto &sceValue0) -> ::std::string {{ return \
                     {b}::list(scePath0, sceValue0, 2u, {nothing}); }})"
                ),
                format!("{b}::list(::std::string(\"late\"), late, 2u, {nothing})"),
            ]
        );
    }

    /// A C++ walk names no union: the one decision of what is lowered refuses
    /// one on C++ first, and the walk that would declare or check one says why
    /// there is none.
    #[test]
    fn a_cpp_walk_reads_no_union() {
        let shapes = shapes();
        assert!(union_definition(SHAPE_KEY, &shapes, &kinds(), &CPP).is_err());
        let all: Vec<&ScopedCase> = shapes.union.cases.iter().collect();
        assert!(CPP
            .union_member(&shapes.union, &all, "kind")
            .unwrap_err()
            .contains("holds no union"));
        assert!(
            cpp_bound_check(&ty("union:Shape"), None, "path", "one", 0, &shapes)
                .unwrap_err()
                .contains("holds no union")
        );
    }
}
