// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A union document (docs/adr/0092, section 1.1): the rules of its cases that
//! need the documents it imports, and the refusal a backend that does not
//! lower a union gives one.
//!
//! The parser holds a union to every rule the document states alone: one tag,
//! a value of an imported enum; each case naming a variant once and, where it
//! holds a record, an imported event schema; at least one case, and not every
//! case empty ([`crate::forge::parser`]). What is left needs the imported
//! documents, and is asked here, where the imports are read: the tag's enum is
//! closed, its variants and the cases are one to one, and no case's record
//! declares the key the tag is written under.

use std::path::Path;

use crate::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use crate::forge::import_source::{enum_model, parse_quietly};
use crate::forge::model::{ForgeDocument, ForgeKind, ParsedForge, UnionCase, UnionModel};

/// Why a backend does not generate a union, as every refusal of one says it:
/// the type is the language's, and its lowering on a backend is a step of the
/// landing, not a rule (docs/adr/0092, section 7, and its landing order). The
/// backends that lower one are
/// [`crate::forge::codegen_matrix::UNION_BACKENDS`].
pub const NOT_LOWERED_YET: &str = "a union is a type of the language, and this backend does \
                                   not lower a union yet (docs/adr/0092, section 7)";

/// Why a statechart's machine does not hold a union yet, on the backend whose
/// walks over a composed value its machine spells
/// ([`crate::forge::composed_lowering`]): that backend generates the union's
/// own unit and a schema unit's field of one, and the machine's read, write,
/// send and variable of one are the next step of the landing (docs/adr/0092,
/// landing order, step 4). Said apart from [`NOT_LOWERED_YET`], which a backend
/// that generates no union at all gives.
pub const MACHINE_NOT_LOWERED_YET: &str =
    "a union is a type of the language, and a statechart's machine does not hold, read, \
     write or send one yet (docs/adr/0092, landing order, step 4)";

/// The refusal of a union document on a backend that does not lower one: its
/// document is read and checked, and generating it is refused by name
/// (`generate/unsupported-feature`), as a composition a backend does not lower
/// yet is (docs/adr/0086, section 7.3).
pub fn document_unlowered(m: &UnionModel) -> GenerateError {
    GenerateError::unsupported_at(
        format!("union '{}': {NOT_LOWERED_YET}", m.name),
        m.source_location.clone(),
    )
}

/// Refuse a union document whose tag's enum is open
/// (`validation/attribute-rule-violated`), whose cases and the variants of its
/// tag's enum are not one to one (`validation/union-cases`, naming each case
/// that names no variant and each variant that has no case), or a case of
/// which holds a record that declares the tag's key
/// (`validation/union-tag-collision`). A document of any other kind has none of
/// these rules.
///
/// An import that cannot be read is left out, as [`parse_quietly`] leaves it:
/// the import pass reports it in its own words.
pub fn check(
    parsed: &ParsedForge,
    base_dir: &Path,
    document: &str,
) -> Result<(), Located<ForgeError>> {
    let ForgeDocument::Union(union) = &parsed.document else {
        return Ok(());
    };
    let Some(tag_enum) = enum_model(parsed, base_dir, &union.tag_enum) else {
        return Ok(());
    };
    if !tag_enum.strict_variants {
        return Err(Located::new(
            ValidationError::AttributeRuleViolated {
                element: format!("the tag <data id=\"{}\">", union.tag),
                attr: "sce:type".into(),
                value: format!("enum:{}", union.tag_enum),
                rule: format!(
                    "a value of a closed enum: `{}` is open (sce:strict-variants=\"false\"), \
                     and a number no variant names would be a value of the union with no case",
                    union.tag_enum
                ),
            }
            .into(),
            document,
            union.tag_line,
            None,
        ));
    }

    // One case per variant, and one variant per case.
    let stray: Vec<&UnionCase> = union
        .cases
        .iter()
        .filter(|case| !tag_enum.variants.iter().any(|v| v.name == case.variant))
        .collect();
    let missing: Vec<&str> = tag_enum
        .variants
        .iter()
        .map(|v| v.name.as_str())
        .filter(|name| union.case(name).is_none())
        .collect();
    let quoted = |names: &[&str]| {
        names
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut parts = Vec::new();
    if !stray.is_empty() {
        let names: Vec<&str> = stray.iter().map(|case| case.variant.as_str()).collect();
        parts.push(format!(
            "cases naming no variant of the enum `{}`: {}",
            union.tag_enum,
            quoted(&names)
        ));
    }
    if !missing.is_empty() {
        parts.push(format!(
            "variants of the enum `{}` with no case: {}",
            union.tag_enum,
            quoted(&missing)
        ));
    }
    if !parts.is_empty() {
        // Placed on the first case that names no variant, which is written in
        // the document, with that variant as `actual`. A variant with no case
        // is written only in the enum, so the refusal of one alone stands on
        // the tag and has no `actual`: no line of this document holds the
        // name, and `detail` names every missing variant.
        let (variant, line) = match stray.first() {
            Some(case) => (case.variant.clone(), case.source_line),
            None => (String::new(), union.tag_line),
        };
        return Err(Located::new(
            ValidationError::UnionCases {
                document: union.name.clone(),
                variant,
                detail: format!(
                    "{}; the cases and the variants of the tag's enum are one to one",
                    parts.join("; ")
                ),
            }
            .into(),
            document,
            line,
            None,
        ));
    }

    // The tag stands beside a case's fields in one object.
    for case in &union.cases {
        let Some(alias) = &case.record else {
            continue;
        };
        let Some(import) = parsed
            .imports
            .iter()
            .find(|i| i.alias == *alias && i.kind == ForgeKind::EventSchema)
        else {
            continue;
        };
        let Some(ForgeDocument::EventSchema(schema)) =
            parse_quietly(base_dir, import).map(|p| p.document)
        else {
            continue;
        };
        if let Some(field) = schema.fields.iter().find(|f| f.id == union.tag) {
            // Placed on the tag, whose `id` is the colliding key and so the
            // line that holds `actual`; the case and its record are named in
            // the message.
            return Err(Located::new(
                ValidationError::UnionTagCollision {
                    document: union.name.clone(),
                    case: case.variant.clone(),
                    record: alias.clone(),
                    field: field.id.clone(),
                }
                .into(),
                document,
                union.tag_line,
                None,
            ));
        }
    }
    Ok(())
}
