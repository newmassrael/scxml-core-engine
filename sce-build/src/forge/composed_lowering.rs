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
//! A backend whose types carry a bound (section 7.4) copies a value into a
//! place of a larger bound by widening it at each depth where the two bounds
//! differ, the fit proven before ([`copier`], section 5.1); a backend whose
//! types carry none holds two places whose types differ only in their bounds
//! in one type, and copies a value that fits as it is.
//!
//! Each backend's spelling is a module of its own beside this file
//! (`composed_lowering/<backend>.rs`), which states that backend's row of
//! section 7.1, the type it holds each constructor in, and how it copies a
//! value; its unit tests sit with it, over the schemas this file's tests
//! share (`composed_lowering/test_support.rs`). Nothing in this file names a
//! backend beyond the lines that declare each module and re-export what the
//! generator names of it; what each backend lowers is decided once, by its row of
//! [`crate::forge::event_schema_check::ComposedLowering::of`]
//! (ARCHITECTURE.md, "Composed Values: One Walk, One Module per Backend").

use std::collections::BTreeMap;

use crate::forge::model::{EventSchemaModel, SceType};
use crate::forge::value_type::{ScopedCase, ScopedUnion, TypeScope, ValueType};

mod c11;
mod cpp;
mod ecmascript;
mod go;
mod kotlin;
mod python;
mod rust;
mod swift;
#[cfg(test)]
mod test_support;

pub use c11::C11Composed;
pub use cpp::{CppCarrier, CppComposed};
pub use ecmascript::JsComposed;
pub use go::{GoCarrier, GoComposed};
pub use kotlin::{KotlinCarrier, KotlinComposed};
pub use python::{PythonCarrier, PythonComposed};
pub use rust::{RustCarrier, RustComposed};
pub use swift::{SwiftCarrier, SwiftComposed};

pub(crate) use c11::{c11_pairs, c11_path_len, C11_WIRE};
pub(crate) use cpp::{
    cpp_bounded_list, cpp_bytes_type, cpp_case_ident, cpp_case_read, cpp_raise_checks,
    cpp_text_type, cpp_union_member, cpp_unit_union_read, CPP_CASE_VALUE, CPP_FORGE,
    CPP_UNION_CASES, CPP_UNION_KIND, CPP_UNREACHED,
};
pub(crate) use go::{
    go_case_ident, go_case_member, go_case_read, go_raise_checks, go_union_member,
    go_unit_union_read, GO_UNION_KIND, GO_UNION_TAG,
};
pub(crate) use kotlin::{kotlin_init_block, kotlin_union_member, kotlin_unit_union_read};
pub(crate) use python::{python_post_init, python_union_member, PYTHON_PAYLOAD};
pub(crate) use rust::{
    rust_bounded_list, rust_bytes_type, rust_case_ident, rust_case_read, rust_text_type,
    rust_union_member, rust_union_tag_method, rust_unit_union_read, RUST_EMPTY_LIST, RUST_PORTABLE,
};
pub(crate) use swift::{
    swift_case_ident, swift_case_read, swift_raise_checks, swift_union_member,
    swift_unit_union_read, SWIFT_UNION_KIND,
};

/// Whether a value of `ty`, written in the scope `within` names, has no value to
/// start at: a union has none (docs/adr/0092, amendment of 2026-11-07), and a
/// record with a member that has none has none either. A list and a value that may
/// be absent start empty and absent, whatever they hold. What holds such a value
/// is built whole, by every member, wherever a backend gives a record a value at
/// its zero ([`ComposedSpelling::record_starts_at_zero`]).
pub fn holds_no_zero(
    ty: &ValueType,
    within: Option<&str>,
    records: &dyn RecordEnv,
) -> Result<bool, String> {
    Ok(match ty {
        ValueType::Union { .. } => true,
        ValueType::Record { alias } => {
            let held = records.record(within, alias)?;
            for field in &held.schema.fields {
                if holds_no_zero(&field.sce_type, Some(held.alias), records)? {
                    return Ok(true);
                }
            }
            false
        }
        ValueType::Scalar(_)
        | ValueType::Text { .. }
        | ValueType::Bytes { .. }
        | ValueType::List { .. }
        | ValueType::Optional { .. } => false,
    })
}

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
    /// The JSON of a value of the enum the statechart holds as `alias`: what
    /// the walk writes an enum by, [`Self::write_enum`] for a spelling whose
    /// value names its variant without its enum (a method of the value), and
    /// the enum's own function of its declared names for one that has no
    /// method to call (C11).
    fn write_enum_of(&self, _alias: &str, held: &Held) -> String {
        self.write_enum(held)
    }
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
    /// `sent`, what [`Self::send_param`] made of a list held at `held`, written
    /// only once the list is seen to hold at least `least` elements: the lower
    /// count of the field `field` reads it as, which a list filled by appends
    /// does not prove where the document is read (docs/adr/0086, section 15.6;
    /// [`crate::forge::value_type::fits_but_for_count`]). A list short of it
    /// fails the param as a checked operation that fails does, through the
    /// failure the target's `<send>` reads its params' failures from, so the
    /// event is refused whole with `error.execution`; a target that writes a
    /// param as statements (`StaticTarget::composed_param_writes`) refuses the
    /// wire before the pair is written, as its other checks do.
    /// `None` for a spelling that holds no such count, which the lowering
    /// refuses by name.
    fn send_counted(
        &self,
        _held: &Held,
        _field: &str,
        _least: u32,
        _sent: String,
    ) -> Option<String> {
        None
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
    /// A copy of the list `held` into a place the machine holds in `list_ty`,
    /// as [`Self::copy_list`] spells it: what the walk copies a list by,
    /// [`Self::copy_list`] for a spelling whose list's type is told by its
    /// element, and the constructor of the place's own type for one whose
    /// list's type is named by its bound too and that has no generic to name
    /// it by (C11).
    fn copy_list_into(&self, held: &Held, elem: &str, elem_ty: &str, _list_ty: &str) -> String {
        self.copy_list(held, elem, elem_ty)
    }
    /// A copy of the union value `held`, or of a union that may be absent, into
    /// a place of the same type (docs/adr/0092, section 6.1 item 5):
    /// [`Self::copy_whole`] of a value
    /// not copied as it is read, for a backend whose copy of such a value is
    /// one spelling whatever its type; a backend whose copy of a list is not
    /// the copy of a union spells it apart (Go).
    fn copy_union(&self, held: &Held) -> String {
        self.copy_whole(held, false)
    }

    /// The declaration that writes the composed record `alias` as a saved state
    /// does and reads it back (docs/adr/0092, section 6.1 item 7), for a backend
    /// whose record type carries that form itself (a Swift record has
    /// `toSaved` and `fromSaved`); `None` for a backend that writes it
    /// elsewhere. Asked only for a record a variable holds.
    fn saved_record_ext(
        &self,
        _alias: &str,
        _records: &dyn RecordEnv,
        _enums: &BTreeMap<String, MachineEnum>,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }
    /// The declaration that writes the union `def` as a saved state does
    /// (its tag beside the fields of its case) and reads it back by its tag,
    /// the case's record read by [`Self::saved_record_ext`]'s; `None` for a
    /// backend that writes it elsewhere. Asked only for a union a variable holds.
    fn saved_union_ext(
        &self,
        _def: &UnionDef,
        _key: &str,
        _records: &dyn RecordEnv,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }
    /// The read of the field `id`, of `ty`, of a composed record whose saved
    /// form the machine's template writes
    /// ([`crate::forge::static_lowering::StaticRecord`]), for a field whose
    /// type states what the type the machine holds it in does not: a list's
    /// lower count (docs/adr/0086, section 15.6), which a restore holds as a
    /// delivery does. `None` where the field's own type reads all `ty` states,
    /// and for a backend whose record carries its saved form itself
    /// ([`Self::saved_record_ext`]), whose reader holds each count.
    fn saved_field_read(&self, _ty: &ValueType, _id: &str) -> Option<String> {
        None
    }
    /// Whether a record of this backend starts every member at its zero (a Swift
    /// record is `SceDefault`, which a checked read of an element not there
    /// answers), so a record that holds a member with no zero ([`holds_no_zero`])
    /// cannot be declared. A backend that builds a record whole only (Rust) has
    /// no such record to refuse.
    fn record_starts_at_zero(&self) -> bool {
        false
    }
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
    /// How a value is written into the field `member` of `holder`, a value of
    /// `union`, where it may be in the cases `left` (docs/adr/0092, section
    /// 6.1 item 6): through the place of the field
    /// ([`Self::union_member_place`]) on a backend whose union is written in
    /// place, which is the default; a backend whose records are immutable
    /// values assigns the holder whole the next value of the union, the field
    /// replaced. `Err` as [`Self::union_member_place`]'s.
    fn union_member_write(
        &self,
        holder: &str,
        union: &ScopedUnion,
        left: &[&ScopedCase],
        member: &str,
    ) -> Result<UnionMemberWrite, String> {
        self.union_member_place(holder, union, left, member)
            .map(UnionMemberWrite::Place)
    }
}

/// How a field of a union value is written where its case is proven
/// (docs/adr/0092, section 6.1 item 6;
/// [`ComposedSpelling::union_member_write`]): the statement the target's
/// assignment makes of it.
pub enum UnionMemberWrite {
    /// The place of the field itself, which the value is assigned to.
    Place(String),
    /// The holder of the union, assigned whole the value `next` builds of the
    /// value written: the union in the case it is in, its record with that
    /// field replaced, as a backend whose records are immutable values writes
    /// a record's field.
    Holder { holder: String, next: UnionNext },
}

/// What builds the next value of a union from the value written into one of
/// its fields ([`UnionMemberWrite::Holder`]).
pub type UnionNext = Box<dyn Fn(&str) -> String>;

/// What a read of the member `member` of a value of `union` reads where the
/// value may be in the cases `left` (docs/adr/0092, section 6.1 item 2;
/// [`union_member_of`]), which each backend spells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnionMember<'u> {
    /// The tag, always readable.
    Tag,
    /// A field of the one case left, which declares it, read through that
    /// case's proven read.
    Case(&'u ScopedCase),
    /// A field every case left declares with one type, copied as it is read:
    /// on a machine's union through its one accessor over the cases
    /// ([`shared_plain_field`]), on a unit's by a test of the case over the
    /// cases left ([`unit_union_member_of`]).
    Shared,
}

/// The type the record of `case`, a case of `union`, declares `member` with,
/// if it declares it.
fn case_field<'u>(
    union: &'u ScopedUnion,
    case: &ScopedCase,
    member: &str,
) -> Option<&'u ValueType> {
    case.record
        .as_deref()
        .and_then(|record| union.scope.records.get(record))
        .and_then(|record| record.fields.iter().find(|(id, _)| id == member))
        .map(|(_, ty)| ty)
}

/// The cases `left`, each named in backquotes, for a sentence that says where
/// a read stands.
fn cases_named(left: &[&ScopedCase]) -> String {
    left.iter()
        .map(|case| format!("`{}`", case.variant.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What a read of `member` of a value of `union` that may be in the cases
/// `left` reads ([`UnionMember`]): its tag; the field of the one case left; or
/// a field the cases left share. `Err` for a field the cases left share whose
/// value is not copied as it is read, which no accessor answers yet, and for
/// one no case left declares, which the judge refused before
/// (`expression/union-read-unproven`).
pub fn union_member_of<'u>(
    union: &ScopedUnion,
    left: &[&'u ScopedCase],
    member: &str,
) -> Result<UnionMember<'u>, String> {
    if member == union.tag {
        return Ok(UnionMember::Tag);
    }
    let declares = |case: &ScopedCase| -> bool { case_field(union, case, member).is_some() };
    match left {
        [case] if declares(case) => Ok(UnionMember::Case(case)),
        _ if !left.is_empty()
            && left.iter().all(|case| declares(case))
            && shared_plain_field(union, member).is_some() =>
        {
            Ok(UnionMember::Shared)
        }
        _ => Err(format!(
            "`{member}` of `union:{}`, read where it may be in the cases {}, is a field those \
             cases share whose value is not copied as it is read, and a machine reads such a \
             field only where a test proves one case (docs/adr/0092, section 6.1)",
            union.name,
            cases_named(left)
        )),
    }
}

/// What a read of `member` of a value of `union` held in the type the union's
/// own unit declares reads, where the value may be in the cases `left`
/// ([`UnionMember`]; docs/adr/0092, section 6.1 item 2, and "Decided while
/// landing step 5" item 7): the read of an algorithm's parameter or local. Its
/// tag; the field of the one case left; or a field every case left declares
/// with one type, which the backend reads by a test of the case over the cases
/// left alone, so the unit needs no accessor of it and a case outside `left`
/// need not declare it alike, as a machine's one accessor needs
/// ([`union_member_of`]). A field is copied out as it is read, so `Err` for one
/// whose value is not ([`plain`]: a text, a byte string), for one no case left
/// declares, and for one the cases left declare with two types. One
/// classification for every backend that reads a unit's union.
pub fn unit_union_member_of<'u>(
    union: &ScopedUnion,
    left: &[&'u ScopedCase],
    member: &str,
) -> Result<UnionMember<'u>, String> {
    if member == union.tag {
        return Ok(UnionMember::Tag);
    }
    let refused = |why: &str| {
        format!(
            "`{member}` of `union:{}`, read where it may be in the cases {}, {why} \
             (docs/adr/0092, section 6.1)",
            union.name,
            cases_named(left)
        )
    };
    let mut types = left.iter().map(|case| case_field(union, case, member));
    let Some(Some(first)) = types.next() else {
        return Err(refused("is read where no case left declares it"));
    };
    if !types.all(|ty| ty == Some(first)) {
        return Err(refused("is not declared with one type by every case left"));
    }
    if !plain(first) {
        return Err(refused(
            "is a value an algorithm does not copy out of a union yet: a text or a byte string \
             of a case is read in a statechart",
        ));
    }
    Ok(match left {
        [case] => UnionMember::Case(case),
        _ => UnionMember::Shared,
    })
}

/// Whether a test of the case over the cases `left` of `union` needs an arm
/// for the others: it does where some case is not left, which the narrowing
/// proved the value is not in, and it must not where every case is left, since
/// a backend that holds its tests to every case warns of an arm none reaches.
pub(crate) fn unit_union_leaves_others(union: &ScopedUnion, left: &[&ScopedCase]) -> bool {
    left.len() < union.cases.len()
}

/// The sentence of the arm a test of the case over the cases `left` of `union`
/// takes for any other case, which the narrowing proved the value is not in
/// ([`unit_union_leaves_others`]): what the backend's statement that the arm is
/// never taken says.
pub(crate) fn unit_union_other_case(union: &ScopedUnion, left: &[&ScopedCase]) -> String {
    format!(
        "a value of {} read as one of its cases {} is in another case",
        union.name,
        left.iter()
            .map(|case| case.variant.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
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
        // A union, and one that may be absent, is copied as a union is.
        let union = match from {
            ValueType::Optional { elem } => &**elem,
            other => other,
        };
        return Ok(match union {
            ValueType::Union { .. } => spelling.copy_union(held),
            _ => spelling.copy_whole(held, plain(from)),
        });
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
        (ValueType::List { elem: from, .. }, ValueType::List { elem: to_elem, .. }) => Ok(spelling
            .copy_list_into(
                held,
                &part(from, to_elem, &element)?,
                &typed(to_elem)?,
                &typed(to)?,
            )),
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
        ValueType::Scalar(SceType::Enum(reference)) => {
            spelling.write_enum_of(&reference.alias, held)
        }
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

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

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

    /// A backend whose union is written in place writes a field through the
    /// place its spelling gives the field ([`ComposedSpelling::union_member_write`],
    /// by default).
    #[test]
    fn a_union_written_in_place_is_written_through_the_place_of_its_field() {
        let shapes = shapes();
        let union = &shapes.union;
        let disc: Vec<&ScopedCase> = union
            .cases
            .iter()
            .filter(|case| case.variant.name == "disc")
            .collect();
        let rust = RustComposed {
            machine: "M",
            carrier: RustCarrier::Json,
        };
        match rust.union_member_write("self.s", union, &disc, "radius") {
            Ok(UnionMemberWrite::Place(place)) => {
                assert_eq!(place, "self.s.proven_disc_mut().radius")
            }
            Ok(UnionMemberWrite::Holder { holder, .. }) => {
                panic!("a Rust union is written in place, not through {holder}")
            }
            Err(why) => panic!("{why}"),
        }
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
}
