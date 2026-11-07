// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `<sce:lift>` (docs/adr/0087, section 2): what the build reads of a lift, said
//! once for the judge that refuses one ([`crate::forge::static_datamodel`]) and
//! the lowering that writes it ([`crate::forge::static_lowering`]).
//!
//! A lift reads a text a statechart holds as a record of the schema `R` its
//! `schema` names. When the text is a value of `R` the record replaces its
//! `location` and its `refusal` becomes absent; when it is not, its `refusal`
//! holds a record of the standard refusal document ([`REFUSAL_DOCUMENT`]): the
//! code, the strict RFC 6901 pointer to the first place refused, and the bound
//! that place broke. The statement raises nothing.
//!
//! What the build computes of `R` for it:
//!
//! - `depth(R)` ([`depth`]): a container opened deeper than this is refused
//!   (`tooDeep`) before it is read, so a lift's work is linear in its text and
//!   its stack bounded by the type;
//! - the longest pointer to a place `R` declares ([`longest_pointer`]): every
//!   refusal names such a place, so the refusal record's `path` holds every
//!   pointer whole when its bound is at least this long, and a schema that
//!   would not fit is refused for a lift (`validation/lift-path-bound`).
//!
//! The pointer of a `tooDeep` refusal names the container where `R` declares
//! the place it stands at, and otherwise the record that holds the name no
//! field of `R` declares: a container under a key a sender added is no place of
//! `R`, and naming it would make the pointer as long as the text rather than
//! the type. Every runtime decides it the same way, over the text's brackets
//! alone, before it reads the text as JSON (`lift` in each runtime's payload
//! reader).

use crate::forge::value_type::{SizeError, TypeScope, ValueType};

/// The document a lift keeps its refusal in, by the identity every reader of a
/// document names it with ([`crate::forge::stdlib::identity`]).
pub const REFUSAL_DOCUMENT: &str = "sce:std/lift/refusal.scxml";

/// The field of the refusal record that holds the place refused.
pub const REFUSAL_PATH_FIELD: &str = "path";

/// Whether the record the statechart imports as `alias` is the refusal record:
/// the standard document itself, by whatever alias and path it is imported.
pub fn is_refusal_record(alias: &str, scope: &TypeScope) -> bool {
    scope
        .records
        .get(alias)
        .is_some_and(|record| record.identity == REFUSAL_DOCUMENT)
}

/// Whether the aliases `a` and `b` of one statechart name one record document
/// (SCE_FORGE.md section 4.12: a record type is its document).
pub fn same_record(a: &str, b: &str, scope: &TypeScope) -> bool {
    if a == b {
        return true;
    }
    match (scope.records.get(a), scope.records.get(b)) {
        (Some(first), Some(second)) => first.identity == second.identity,
        _ => false,
    }
}

/// The bound of the `path` of the refusal record imported as `alias`, as this
/// build resolved it (`string[lift_path=256]`); `None` for a record with no
/// such text field.
pub fn path_bound(alias: &str, scope: &TypeScope) -> Option<u32> {
    let record = scope.records.get(alias)?;
    record
        .fields
        .iter()
        .find(|(id, _)| id == REFUSAL_PATH_FIELD)
        .and_then(|(_, ty)| match ty {
            ValueType::Text { max } => Some(max.max),
            _ => None,
        })
}

/// Whether the aliases `a` and `b` of one statechart name one union document
/// (docs/adr/0092, section 1.1: a union type is its document).
pub fn same_union(a: &str, b: &str, scope: &TypeScope) -> bool {
    if a == b {
        return scope.unions.contains_key(a);
    }
    match (scope.unions.get(a), scope.unions.get(b)) {
        (Some(first), Some(second)) => {
            !first.identity.is_empty() && first.identity == second.identity
        }
        _ => false,
    }
}

/// The type a lift whose `schema` is `alias` reads its text as: the union the
/// statechart imports as `alias` where it imports one (docs/adr/0092, section
/// 6.1 item 5: a whole union is put into a variable from a lift), and the
/// record of the event schema `alias` names otherwise.
pub fn lifted_type(alias: &str, scope: &TypeScope) -> ValueType {
    if scope.unions.contains_key(alias) {
        ValueType::Union {
            alias: alias.to_string(),
        }
    } else {
        ValueType::Record {
            alias: alias.to_string(),
        }
    }
}

/// `depth(R)` of the record or union the statechart imports as `alias`
/// ([`lifted_type`], [`crate::forge::value_type::container_depth`]).
pub fn depth(alias: &str, scope: &TypeScope) -> Result<u32, SizeError> {
    crate::forge::value_type::container_depth(&lifted_type(alias, scope), scope)
}

/// The longest pointer to a place the record or union the statechart imports
/// as `alias` declares, with its length in bytes ([`lifted_type`],
/// [`crate::forge::value_type::longest_pointer`]).
pub fn longest_pointer(alias: &str, scope: &TypeScope) -> Result<(String, u64), SizeError> {
    crate::forge::value_type::longest_pointer(&lifted_type(alias, scope), scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::value_type::{ScopedRecord, WrittenType};

    fn ty(text: &str) -> ValueType {
        WrittenType::read(text).unwrap().bounded().unwrap()
    }

    fn record(identity: &str, fields: &[(&str, &str)]) -> ScopedRecord {
        ScopedRecord {
            identity: identity.to_string(),
            name: identity.to_string(),
            fields: fields
                .iter()
                .map(|(id, written)| (id.to_string(), ty(written)))
                .collect(),
            scope: TypeScope::default(),
        }
    }

    fn scope() -> TypeScope {
        let mut scope = TypeScope::default();
        scope.records.insert(
            "LiftRefusal".into(),
            record(
                REFUSAL_DOCUMENT,
                &[("path", "string[256]"), ("bound", "optional<uint32>")],
            ),
        );
        scope.records.insert(
            "Copy".into(),
            record("/elsewhere/refusal.scxml", &[("path", "string[256]")]),
        );
        scope.records.insert(
            "Answer".into(),
            record(
                "/here/answer.scxml",
                &[("title", "string[16]"), ("count", "uint32")],
            ),
        );
        scope.records.insert(
            "Again".into(),
            record(
                "/here/answer.scxml",
                &[("title", "string[16]"), ("count", "uint32")],
            ),
        );
        scope
    }

    /// The refusal record is the standard document, told by its identity and
    /// not by its alias or its fields: a copy of it elsewhere is another record.
    #[test]
    fn the_refusal_record_is_the_standard_document_by_its_identity() {
        let scope = scope();
        assert!(is_refusal_record("LiftRefusal", &scope));
        assert!(!is_refusal_record("Copy", &scope));
        assert!(!is_refusal_record("Missing", &scope));
        assert_eq!(path_bound("LiftRefusal", &scope), Some(256));
        assert_eq!(path_bound("Answer", &scope), None);
    }

    /// Two imports of one document name one record.
    #[test]
    fn two_aliases_of_one_document_name_one_record() {
        let scope = scope();
        assert!(same_record("Answer", "Again", &scope));
        assert!(same_record("Answer", "Answer", &scope));
        assert!(!same_record("Answer", "LiftRefusal", &scope));
        assert!(!same_record("Answer", "Missing", &scope));
    }

    /// A lift whose schema names a union reads the union: one level deep, as
    /// a record is, its longest pointer the tag's or a case's field's, and two
    /// aliases of one union document name one union.
    #[test]
    fn a_union_alias_is_lifted_as_the_union() {
        use crate::forge::value_type::{ScopedCase, ScopedUnion, ScopedVariant};
        let mut scope = scope();
        let mut inner = TypeScope::default();
        inner.records.insert(
            "Wide".into(),
            record("/here/wide.scxml", &[("breadth", "uint8")]),
        );
        let union = ScopedUnion {
            identity: "/here/shape.scxml".into(),
            name: "shape".into(),
            tag: "kind".into(),
            tag_enum: "Kind".into(),
            cases: vec![
                ScopedCase {
                    variant: ScopedVariant {
                        name: "dot".into(),
                        value: 0,
                    },
                    record: None,
                },
                ScopedCase {
                    variant: ScopedVariant {
                        name: "wide".into(),
                        value: 1,
                    },
                    record: Some("Wide".into()),
                },
            ],
            scope: inner,
        };
        scope.unions.insert("Shape".into(), union.clone());
        scope.unions.insert("Form".into(), union);
        assert_eq!(
            lifted_type("Shape", &scope),
            ValueType::Union {
                alias: "Shape".into()
            }
        );
        assert_eq!(
            lifted_type("Answer", &scope),
            ValueType::Record {
                alias: "Answer".into()
            }
        );
        assert_eq!(depth("Shape", &scope), Ok(1));
        assert_eq!(
            longest_pointer("Shape", &scope),
            Ok(("/breadth".to_string(), 8))
        );
        assert!(same_union("Shape", "Form", &scope));
        assert!(same_union("Shape", "Shape", &scope));
        assert!(!same_union("Shape", "Answer", &scope));
        assert!(!same_union("Answer", "Answer", &scope));
    }

    /// A record of scalars is one level deep, and its longest pointer is its
    /// longest field's.
    #[test]
    fn a_record_of_scalars_is_one_level_deep() {
        let scope = scope();
        assert_eq!(depth("Answer", &scope), Ok(1));
        assert_eq!(
            longest_pointer("Answer", &scope),
            Ok(("/title".to_string(), 6))
        );
    }
}
