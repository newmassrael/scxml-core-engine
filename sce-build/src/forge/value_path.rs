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

use crate::forge::value_type::{ScopedRecord, TypeScope, ValueType};

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

/// The scope the records of `chain` lead to from `base`, the statechart's own:
/// the scope of the document of the last record of the chain.
pub fn scope_at<'s>(base: &'s TypeScope, chain: &[String]) -> Option<&'s TypeScope> {
    chain.iter().try_fold(base, |scope, alias| {
        scope.records.get(alias).map(|r| &r.scope)
    })
}

/// The record `alias` names where the place `at` reads its aliases.
fn record_at<'s>(base: Option<&'s TypeScope>, at: &Place, alias: &str) -> Option<&'s ScopedRecord> {
    scope_at(base?, &at.scope)?.records.get(alias)
}

/// The place `steps` lead to from `root`, each record's fields read from the
/// scope `base` and the records on the way lead to; `None` where a step asks a
/// value for a part its type does not have: a field a record does not declare,
/// an element of a value that is no list, a member of a value that is no record.
///
/// The present value of an optional value is its `value`; a record whose field
/// is named `value` is read by that field, since the walk follows the type.
pub fn walk(base: Option<&TypeScope>, root: &Place, steps: &[Step<'_>]) -> Option<Place> {
    let mut place = root.clone();
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
            (Step::Field(_), _) => return None,
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
    walk(base, root, &steps(&path[at..])?)
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
