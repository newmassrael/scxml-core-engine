// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The type of a place a statechart reads, by walking the one value type
//! (docs/adr/0086, section 6.2).
//!
//! A place is a name that holds a value (a variable, the payload of the event a
//! transition is on, the item of a `<foreach>`) and the steps from it, each a
//! field of a record, an element of a list or the present value of an optional
//! value. Its type is found by walking [`ValueType`] from the name one step at a
//! time ([`walk`]), so every rule of docs/SCE_ACCEPTED_SUBSET.md section 2.15
//! that asks the type of a place asks it once, the same way at every depth.
//!
//! A place is written as an expression reads it, without its indices:
//! `_event.data.items[].tags[]` is the place of `_event.data.items[i].tags[j]`
//! whatever `i` and `j` are ([`ELEMENT`]), the spelling a lowering names the
//! field of an element by (`days[].dayOfMonth`).
//!
//! A union (docs/adr/0092, section 6.1) is stepped into by its tag, a value of
//! the tag's enum wherever the union is, and by a field of its cases: the field's
//! type where every case left that declares it gives it one type, the cases left
//! being those the tests that hold where the path is read leave its subject in
//! ([`resolve_narrowed`]), or every case where none is known. Whether the read
//! stands where its case is proven is the judge's to say
//! ([`union_read_refusal`]); the walk types it.

use crate::forge::narrowing::Facts;
use crate::forge::value_type::{ScopedCase, ScopedRecord, ScopedUnion, TypeScope, ValueType};

/// What a place writes for one element of a list, whatever its index.
pub const ELEMENT: &str = "[]";

/// A value a place holds: its type, and the records whose fields were read on
/// the way to it, by the alias each was named by. The record aliases `ty` names
/// are read in the scope of the document of the last of those records, or in
/// the statechart's own where there is none ([`scope_at`]): a record's fields
/// name records and enums by the aliases its own document imports them by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub ty: ValueType,
    pub scope: Vec<String>,
}

impl Place {
    /// A value of `ty` named in the statechart's own scope: a variable, the
    /// payload.
    pub fn of(ty: ValueType) -> Self {
        Self {
            ty,
            scope: Vec::new(),
        }
    }

    /// Whether the value is composed: a list, a record, a union, or an optional
    /// value of one. Such a value is read through its fields, its elements and
    /// its length, and is never an operand.
    pub fn is_composed(&self) -> bool {
        match &self.ty {
            ValueType::List { .. } | ValueType::Record { .. } | ValueType::Union { .. } => true,
            ValueType::Optional { elem } => matches!(
                **elem,
                ValueType::List { .. } | ValueType::Record { .. } | ValueType::Union { .. }
            ),
            ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => false,
        }
    }

    /// The bound of the list the place holds, the most elements a walk over it
    /// meets; `None` for a value that is no list.
    pub fn list_bound(&self) -> Option<u32> {
        match &self.ty {
            ValueType::List { max, .. } => Some(max.max),
            _ => None,
        }
    }

    /// The place of one element of the list this place holds; `None` for a
    /// value that is no list.
    pub fn element(&self) -> Option<Place> {
        match &self.ty {
            ValueType::List { elem, .. } => Some(Place {
                ty: (**elem).clone(),
                scope: self.scope.clone(),
            }),
            _ => None,
        }
    }
}

/// One step from a value to a part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step<'p> {
    /// A field of a record, or `value`, the present value of an optional one
    /// (docs/adr/0055).
    Field(&'p str),
    /// An element of a list.
    Element,
}

/// The steps `rest` writes after the name of a place: each `.<name>` a field
/// and each [`ELEMENT`] an element. `None` for a text that is no such run of
/// steps.
pub fn steps(rest: &str) -> Option<Vec<Step<'_>>> {
    let mut out = Vec::new();
    let mut rest = rest;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(ELEMENT) {
            out.push(Step::Element);
            rest = after;
            continue;
        }
        let after = rest.strip_prefix('.')?;
        let end = after.find(['.', '[']).unwrap_or(after.len());
        let name = &after[..end];
        if name.is_empty() {
            return None;
        }
        out.push(Step::Field(name));
        rest = &after[end..];
    }
    Some(out)
}

/// The scope the records and unions of `chain` lead to from `base`, the
/// statechart's own: the scope of the document of the last of them.
pub fn scope_at<'s>(base: &'s TypeScope, chain: &[String]) -> Option<&'s TypeScope> {
    chain.iter().try_fold(base, |scope, alias| {
        scope
            .records
            .get(alias)
            .map(|r| &r.scope)
            .or_else(|| scope.unions.get(alias).map(|u| &u.scope))
    })
}

/// The record `alias` names where the place `at` reads its aliases.
fn record_at<'s>(base: Option<&'s TypeScope>, at: &Place, alias: &str) -> Option<&'s ScopedRecord> {
    scope_at(base?, &at.scope)?.records.get(alias)
}

/// The union `alias` names where the place `at` reads its aliases.
fn union_at<'s>(base: Option<&'s TypeScope>, at: &Place, alias: &str) -> Option<&'s ScopedUnion> {
    scope_at(base?, &at.scope)?.unions.get(alias)
}

/// The union the place `at` holds, with the alias it is named by; `None` for a
/// value that is no union, or whose union the scope does not resolve.
pub fn union_of<'s>(base: Option<&'s TypeScope>, at: &Place) -> Option<(&'s ScopedUnion, String)> {
    let ValueType::Union { alias } = &at.ty else {
        return None;
    };
    Some((union_at(base, at, alias)?, alias.clone()))
}

/// The alias the statechart's own scope `base` names the enum of `union`'s tag
/// by, the enum a test of the tag writes a variant of (`ShapeKind.circle`):
/// the first of its enums of the same variants, numbers and underlying type, as
/// two enums are one type across documents (docs/adr/0086, section 5.1).
/// `None` where the statechart imports no such enum, so writes no test of the
/// tag.
pub fn tag_enum_alias(base: &TypeScope, union: &ScopedUnion) -> Option<String> {
    let tag = union.scope.enums.get(&union.tag_enum)?;
    base.enums
        .iter()
        .find(|(_, held)| *held == tag)
        .map(|(alias, _)| alias.clone())
}

/// The case of `union` the tag's `expr` names, where it is a variant literal of
/// the enum the statechart's scope `base` names the tag's enum by
/// ([`tag_enum_alias`]): `ShapeKind.circle` names the case `circle`. `None` for
/// any other expression, which builds no case the build knows (docs/adr/0092,
/// section 6.1 item 5).
pub fn case_of_tag_literal<'u>(
    base: &TypeScope,
    union: &'u ScopedUnion,
    expr: &str,
) -> Option<&'u ScopedCase> {
    let alias = tag_enum_alias(base, union)?;
    let (written_alias, variant) = expr.trim().split_once('.')?;
    if written_alias.trim() != alias {
        return None;
    }
    union
        .cases
        .iter()
        .find(|case| case.variant.name == variant.trim())
}

/// The alias the statechart's scope `base` imports the record of `case` of
/// `union` by: the first of its records of the same document. `None` for an
/// empty case, and for a record the statechart does not import.
pub fn case_record_alias(
    base: &TypeScope,
    union: &ScopedUnion,
    case: &ScopedCase,
) -> Option<String> {
    let record = union.scope.records.get(case.record.as_deref()?)?;
    base.records
        .iter()
        .find(|(_, held)| held.identity == record.identity)
        .map(|(alias, _)| alias.clone())
}

/// The cases of `union` the subject `subject` may be in where `facts` hold
/// (docs/adr/0092, section 6.1): those no test of its tag rules out, in the
/// enum's order. Every case where the subject is no subject (an element read
/// by its index), where no fact is known, or where the statechart names no
/// enum of the tag's.
pub fn cases_left<'u>(
    base: Option<&TypeScope>,
    union: &'u ScopedUnion,
    subject: Option<&str>,
    facts: Option<&Facts>,
) -> Vec<&'u ScopedCase> {
    let fact = subject
        .zip(facts)
        .and_then(|(subject, facts)| facts.cases_of(&format!("{subject}.{}", union.tag)));
    let alias = base.and_then(|base| tag_enum_alias(base, union));
    union
        .cases
        .iter()
        .filter(|case| match (fact, alias.as_deref()) {
            (Some(fact), Some(alias)) => fact.admits(alias, &case.variant.name),
            _ => true,
        })
        .collect()
}

/// The type the field `field` of a case's record has in each of `cases` that
/// declares it, with the alias of the record of the first, where they give it
/// one type; `None` where no case declares it, or two give it two types.
fn case_field<'u>(
    union: &'u ScopedUnion,
    cases: &[&'u ScopedCase],
    field: &str,
) -> Option<(&'u str, &'u ValueType)> {
    let mut found: Option<(&'u str, &'u ValueType)> = None;
    for case in cases {
        let Some(record) = case.record.as_deref() else {
            continue;
        };
        let declared = union
            .scope
            .records
            .get(record)
            .and_then(|r| r.fields.iter().find(|(id, _)| id == field));
        if let Some((_, ty)) = declared {
            match found {
                None => found = Some((record, ty)),
                Some((_, held)) if held == ty => {}
                Some(_) => return None,
            }
        }
    }
    found
}

/// Why the read of `field` of the union `union` (named `alias`, read through the
/// subject `subject`, or through an element read by its index where `None`)
/// does not stand where `facts` hold: the cases left that do not declare it, or
/// declare it with another type than the rest (docs/adr/0092, section 6.1 item
/// 2). `None` where the read stands: every case left declares it with one type.
/// A field no case declares is no business of this rule.
pub fn union_read_refusal(
    base: Option<&TypeScope>,
    union: &ScopedUnion,
    alias: &str,
    subject: Option<&str>,
    field: &str,
    facts: Option<&Facts>,
) -> Option<UnionReadRefusal> {
    let declares = |case: &ScopedCase| -> bool {
        case.record.as_deref().is_some_and(|record| {
            union
                .scope
                .records
                .get(record)
                .is_some_and(|r| r.fields.iter().any(|(id, _)| id == field))
        })
    };
    let declaring: Vec<String> = union
        .cases
        .iter()
        .filter(|case| declares(case))
        .map(|case| case.variant.name.clone())
        .collect();
    if declaring.is_empty() {
        return None;
    }
    let left = cases_left(base, union, subject, facts);
    let stands =
        left.iter().all(|case| declares(case)) && case_field(union, &left, field).is_some();
    if stands {
        return None;
    }
    let enum_alias = base
        .and_then(|base| tag_enum_alias(base, union))
        .unwrap_or_else(|| union.tag_enum.clone());
    Some(UnionReadRefusal {
        union: alias.to_string(),
        field: field.to_string(),
        declaring,
        test: match subject {
            Some(subject) => format!("{subject}.{} === {enum_alias}.<case>", union.tag),
            None => format!(
                "a <foreach> over the list, whose item is a subject, and \
                 `<item>.{} === {enum_alias}.<case>`",
                union.tag
            ),
        },
    })
}

/// A read of a case's field the build has not proven ([`union_read_refusal`]):
/// the union, the field, the cases that declare it, in the enum's order, and the
/// test that proves one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionReadRefusal {
    pub union: String,
    pub field: String,
    pub declaring: Vec<String>,
    pub test: String,
}

/// The place `steps` lead to from `root`, each record's fields read from the
/// scope `base` and the records on the way lead to; `None` where a step asks a
/// value for a part its type does not have: a field a record does not declare,
/// an element of a value that is no list, a member of a value that is no record
/// or union.
///
/// The present value of an optional value is its `value`; a record whose field
/// is named `value` is read by that field, since the walk follows the type.
/// A union's field is typed over every case ([`resolve_narrowed`] reads one
/// over the cases a test leaves).
pub fn walk(base: Option<&TypeScope>, root: &Place, steps: &[Step<'_>]) -> Option<Place> {
    walk_narrowed(base, root, None, steps, None)
}

/// [`walk`] from a root named `root_name` (the subject a test of a union it
/// reaches names, while no element is read by its index), each union's field
/// typed over the cases `facts` leave it in ([`cases_left`]).
fn walk_narrowed(
    base: Option<&TypeScope>,
    root: &Place,
    root_name: Option<&str>,
    steps: &[Step<'_>],
    facts: Option<&Facts>,
) -> Option<Place> {
    let mut place = root.clone();
    let mut subject = root_name.map(str::to_string);
    for step in steps {
        place = match (*step, &place.ty) {
            (Step::Element, _) => place.element()?,
            (Step::Field(name), ValueType::Optional { elem })
                if name == crate::forge::expr::OPTIONAL_VALUE =>
            {
                Place {
                    ty: (**elem).clone(),
                    scope: place.scope.clone(),
                }
            }
            (Step::Field(name), ValueType::Record { alias }) => {
                let record = record_at(base, &place, alias)?;
                let (_, ty) = record.fields.iter().find(|(id, _)| id == name)?;
                let mut scope = place.scope.clone();
                scope.push(alias.clone());
                Place {
                    ty: ty.clone(),
                    scope,
                }
            }
            // docs/adr/0092, section 6.1: the tag is a value of its enum, read
            // in the union's own scope; a field of a case is read in its
            // record's.
            (Step::Field(name), ValueType::Union { alias }) => {
                let union = union_at(base, &place, alias)?;
                let mut scope = place.scope.clone();
                scope.push(alias.clone());
                if name == union.tag {
                    Place {
                        ty: ValueType::Scalar(crate::forge::model::SceType::Enum(
                            crate::forge::model::EnumRef {
                                alias: union.tag_enum.clone(),
                            },
                        )),
                        scope,
                    }
                } else {
                    let left = cases_left(base, union, subject.as_deref(), facts);
                    let (record, ty) = case_field(union, &left, name).or_else(|| {
                        case_field(union, &union.cases.iter().collect::<Vec<_>>(), name)
                    })?;
                    scope.push(record.to_string());
                    Place {
                        ty: ty.clone(),
                        scope,
                    }
                }
            }
            (Step::Field(_), _) => return None,
        };
        subject = match (*step, subject) {
            (Step::Field(name), Some(held)) => Some(format!("{held}.{name}")),
            (Step::Field(_), None) | (Step::Element, _) => None,
        };
    }
    Some(place)
}

/// The place `path` writes, from the longest name at its start that `root_of`
/// holds a value for: `_event.data` before `_event`. `None` for a path that
/// starts at no such name, or whose steps the walk does not take.
pub fn resolve<'r>(
    base: Option<&TypeScope>,
    root_of: impl Fn(&str) -> Option<&'r Place>,
    path: &str,
) -> Option<Place> {
    resolve_narrowed(base, root_of, path, None)
}

/// [`resolve`], each union the path reads through a subject typed over the
/// cases `facts` leave it in (docs/adr/0092, section 6.1): where a test proves
/// one case, a field that case declares has that case's type whatever another
/// case gives it.
pub fn resolve_narrowed<'r>(
    base: Option<&TypeScope>,
    root_of: impl Fn(&str) -> Option<&'r Place>,
    path: &str,
    facts: Option<&Facts>,
) -> Option<Place> {
    let path = path.trim();
    let ends: Vec<usize> = path
        .match_indices(['.', '['])
        .map(|(at, _)| at)
        .chain(std::iter::once(path.len()))
        .collect();
    let (root, at) = ends
        .into_iter()
        .rev()
        .find_map(|at| root_of(&path[..at]).map(|root| (root, at)))?;
    walk_narrowed(base, root, Some(&path[..at]), &steps(&path[at..])?, facts)
}

/// Whether the place `at` holds a composed value whose parts are places of
/// their own (docs/adr/0086, section 1.1): a list, a union, a value that may be
/// absent of one of those or of a record, or a record a field of which is one
/// of those or a record. A record of entries alone holds none: its fields are
/// the entries a schema has always held. A record the scope does not resolve
/// holds none that can be named.
pub fn holds_composition(base: Option<&TypeScope>, at: &Place) -> bool {
    fn composed(ty: &ValueType) -> bool {
        match ty {
            ValueType::List { .. } | ValueType::Record { .. } | ValueType::Union { .. } => true,
            ValueType::Optional { elem } => composed(elem),
            ValueType::Scalar(_) | ValueType::Text { .. } | ValueType::Bytes { .. } => false,
        }
    }
    match &at.ty {
        ValueType::Record { alias } => record_at(base, at, alias)
            .is_some_and(|record| record.fields.iter().any(|(_, ty)| composed(ty))),
        other => composed(other),
    }
}

/// The fields of the record the place `at` holds, by id; `None` for a value
/// that is no record, or whose record the scope does not resolve.
pub fn record_fields(base: Option<&TypeScope>, at: &Place) -> Option<Vec<String>> {
    let ValueType::Record { alias } = &at.ty else {
        return None;
    };
    Some(
        record_at(base, at, alias)?
            .fields
            .iter()
            .map(|(id, _)| id.clone())
            .collect(),
    )
}

/// The identity of the record document the place `at` holds a record of
/// ([`ScopedRecord::identity`]); `None` for a value that is no record, or
/// whose record the scope does not resolve.
pub fn record_identity<'s>(base: Option<&'s TypeScope>, at: &Place) -> Option<&'s str> {
    let ValueType::Record { alias } = &at.ty else {
        return None;
    };
    Some(record_at(base, at, alias)?.identity.as_str())
}

/// The scope a `sce-static` statechart's types are read in: each record and
/// enum its imports name, every record with the scope of its own document
/// ([`crate::model::SCXMLModel::imported_type_scope`]). A record the model
/// holds and that scope does not (a model built with no files to read) is
/// read from the model's own schema, with no scope of its own beneath it.
pub fn statechart_scope(model: &crate::model::SCXMLModel) -> TypeScope {
    let mut scope = model.imported_type_scope.clone();
    for (alias, schema) in &model.imported_records {
        scope
            .records
            .entry(alias.clone())
            .or_insert_with(|| ScopedRecord {
                identity: schema.name.clone(),
                name: schema.name.clone(),
                fields: schema
                    .fields
                    .iter()
                    .map(|field| (field.id.clone(), field.sce_type.clone()))
                    .collect(),
                scope: TypeScope::default(),
            });
    }
    scope
}

/// The place the variable `var` names: its declared type, every bound
/// stated; `None` for a variable of no type, or one whose type leaves a bound
/// out.
pub fn variable_place(var: &crate::model::Variable) -> Option<Place> {
    let ty = var.value_type.clone()?.bounded().ok()?;
    Some(Place::of(ty))
}

/// The place `_event.data` names on a transition taken on an event whose
/// schema is named `schema`: a record of that schema, by the alias the
/// statechart imports it under; `None` when no import names it.
pub fn payload_place(model: &crate::model::SCXMLModel, schema: &str) -> Option<Place> {
    model
        .imported_records
        .iter()
        .find(|(_, held)| held.name == schema)
        .map(|(alias, _)| {
            Place::of(ValueType::Record {
                alias: alias.clone(),
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::SceType;
    use crate::forge::value_type::{Bound, WrittenType};
    use std::collections::HashMap;

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text)
            .expect("a type")
            .bounded()
            .expect("every bound stated")
    }

    /// The scope of a statechart that imports `event_batch.scxml` as `Batch`,
    /// whose `items` are records of `Item` as `event_batch.scxml` names it,
    /// each of which holds `tags`, and whose `grid` is a list of lists.
    fn scope() -> TypeScope {
        let item = ScopedRecord {
            identity: "event_batch_item".into(),
            name: "event_batch_item".into(),
            fields: vec![
                ("id".into(), ty("uint32")),
                ("due".into(), ty("optional<int64>")),
                ("tags".into(), ty("list<string[8]>[4]")),
            ],
            scope: TypeScope::default(),
        };
        let mut batch_scope = TypeScope::default();
        batch_scope.records.insert("Item".into(), item);
        let batch = ScopedRecord {
            identity: "event_batch".into(),
            name: "event_batch".into(),
            fields: vec![
                ("items".into(), ty("list<record:Item>[3]")),
                ("grid".into(), ty("list<list<uint8>[2]>[2]")),
                ("spare".into(), ty("optional<list<uint8>[2]>")),
                ("note".into(), ty("string[4]")),
            ],
            scope: batch_scope,
        };
        let mut scope = TypeScope::default();
        scope.records.insert("Batch".into(), batch);
        scope
    }

    fn roots() -> HashMap<&'static str, Place> {
        HashMap::from([
            ("_event.data", Place::of(ty("record:Batch"))),
            ("kept", Place::of(ty("list<record:Batch>[2]"))),
        ])
    }

    fn typed(path: &str) -> Option<ValueType> {
        let scope = scope();
        let roots = roots();
        resolve(Some(&scope), |name| roots.get(name), path).map(|place| place.ty)
    }

    /// One step of each kind, at depth 1: a field of the payload, an element
    /// of a list variable.
    #[test]
    fn a_place_one_step_deep_is_typed_by_its_step() {
        assert_eq!(typed("_event.data.note"), Some(ty("string[4]")));
        assert_eq!(typed("_event.data.items"), Some(ty("list<record:Item>[3]")));
        assert_eq!(typed("kept[]"), Some(ty("record:Batch")));
    }

    /// Two steps: the element of a list field, the field of a record element,
    /// the present value of an optional field.
    #[test]
    fn a_place_two_steps_deep_is_typed_through_the_first() {
        assert_eq!(typed("_event.data.items[]"), Some(ty("record:Item")));
        assert_eq!(typed("_event.data.grid[]"), Some(ty("list<uint8>[2]")));
        assert_eq!(typed("_event.data.spare.value"), Some(ty("list<uint8>[2]")));
        assert_eq!(typed("kept[].note"), Some(ty("string[4]")));
    }

    /// Three steps and more: a field of a record element read in the scope of
    /// the record's own document (`Item` is `event_batch.scxml`'s alias, not
    /// the statechart's), an element of an element, a list in a list element.
    #[test]
    fn a_place_three_steps_deep_reads_each_record_in_its_own_scope() {
        assert_eq!(
            typed("_event.data.items[].tags"),
            Some(ty("list<string[8]>[4]"))
        );
        assert_eq!(typed("_event.data.items[].tags[]"), Some(ty("string[8]")));
        assert_eq!(typed("_event.data.grid[][]"), Some(ty("uint8")));
        assert_eq!(typed("_event.data.items[].due.value"), Some(ty("int64")));
        assert_eq!(typed("kept[].items[].id"), Some(ty("uint32")));
        let scope = scope();
        let roots = roots();
        let deep = resolve(
            Some(&scope),
            |name| roots.get(name),
            "kept[].items[].tags[]",
        )
        .expect("a place");
        assert_eq!(deep.scope, vec!["Batch".to_string(), "Item".to_string()]);
    }

    /// A step the type does not have is no place: a field a record does not
    /// declare, an element of what is no list, a member of a scalar, the
    /// present value of what is not optional.
    #[test]
    fn a_step_the_type_does_not_have_is_no_place() {
        for path in [
            "_event.data.nope",
            "_event.data.note[]",
            "_event.data.items.id",
            "_event.data.items[].id.x",
            "_event.data.items[].tags.value",
            "_event.data.",
            "_event.data..note",
            "other.note",
        ] {
            assert_eq!(typed(path), None, "{path}");
        }
        // A record whose own scope is not read has no fields to step into.
        let roots = roots();
        assert_eq!(
            resolve(None, |name| roots.get(name), "_event.data.note"),
            None
        );
    }

    /// The longest name at the start of a path that holds a value is the one
    /// read: `_event.data` and not `_event`, which holds none.
    #[test]
    fn the_longest_name_that_holds_a_value_starts_the_walk() {
        let roots = HashMap::from([
            ("_event", Place::of(ValueType::Scalar(SceType::Uint8))),
            ("_event.data", Place::of(ty("record:Batch"))),
        ]);
        let scope = scope();
        assert_eq!(
            resolve(Some(&scope), |name| roots.get(name), "_event.data.note").map(|p| p.ty),
            Some(ty("string[4]"))
        );
    }

    #[test]
    fn a_list_a_record_and_an_optional_one_of_either_are_composed() {
        for (text, composed) in [
            ("list<uint8>[2]", true),
            ("record:Batch", true),
            ("optional<record:Batch>", true),
            ("optional<list<uint8>[2]>", true),
            ("optional<int64>", false),
            ("string[4]", false),
            ("uint8", false),
        ] {
            assert_eq!(Place::of(ty(text)).is_composed(), composed, "{text}");
        }
        assert_eq!(Place::of(ty("list<uint8>[3]")).list_bound(), Some(3));
        assert_eq!(
            Place::of(ValueType::List {
                elem: Box::new(ty("uint8")),
                max: Bound::named("rows", 5),
            })
            .list_bound(),
            Some(5)
        );
        assert_eq!(Place::of(ty("uint8")).list_bound(), None);
    }
}
