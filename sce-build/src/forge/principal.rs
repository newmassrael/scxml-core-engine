// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The person behind an instance, as a `datamodel="sce-static"` statechart reads it
//! (docs/adr/0116, D3 and D4).
//!
//! * **A document declares what it reads, in a place no sender writes.**
//!   `<scxml sce:principal="Person">` names an imported record schema, and the document
//!   reads `_x.principal`, an `optional<record:Person>` of the event being processed: the
//!   platform's variable, beside `_x.limits` (`_x` is the system variable a session
//!   reserves for what a platform adds, which no document declares). It is present on an
//!   event an instance of an open machine sent while its incarnation acts for a person,
//!   and absent on every other: an internal event, a timer, a completion. The record's
//!   fields are taken by name from a fixed list ([`FIELDS`]); a field not declared is not
//!   given.
//! * **A machine that must have a person says it in its guard.** `sce:principal="required"`
//!   on a `<transition>` is that guard, `_x.principal !== null`, joined before the
//!   transition's own condition ([`guard_with_required`]), so an event without one does
//!   not select it.
//! * **Nothing passes on by itself.** `<send sce:on-behalf="_x.principal">` quotes the
//!   person to the receiver, which reads a quoted person, `_x.quoted`, and never
//!   `_x.principal`. A deployment lists which machines a machine takes a quote from, and
//!   until one does every quote is refused: where it is written, by the build
//!   ([`refuse_quote`]), and `_x.quoted` is absent on every event (Open question 2 of
//!   the record).
//!
//! The Rust host core is the one that holds a person (D10); every other backend refuses a
//! document that declares one by name, where it is lowered
//! (`crate::forge::static_lowering::refuse_principal`).

use crate::forge::error::{ForgeError, GenerateError, Located, ValidationError};
use crate::forge::model::{EntryType, ForgeKind, SceType};
use crate::model::{Datamodel, SCXMLModel};

/// The system variable a platform's values are read under, which a session reserves for what
/// a platform adds and no document declares.
pub const SYSTEM_VARIABLE: &str = "_x";

/// The person the event being processed was said for.
pub const PRINCIPAL: &str = "_x.principal";

/// The person a machine quoted to this one, with the event it sent.
pub const QUOTED: &str = "_x.quoted";

/// The present value of [`PRINCIPAL`], the closed record its fields are read under.
pub const PRINCIPAL_VALUE: &str = "_x.principal.value";

/// The present value of [`QUOTED`].
pub const QUOTED_VALUE: &str = "_x.quoted.value";

/// The one value `sce:principal` takes on a `<transition>`.
pub const REQUIRED: &str = "required";

/// The fields a document's record of a person takes, by name, with the type each is of and
/// its spelling: who proved the person (OpenID Connect's `iss`), who they are to that
/// issuer (`sub`, unique only within its issuer, so the two are one key), when they proved
/// it, in milliseconds since the Unix epoch, how, as the port that judged the proof names
/// it, and the instance that acts for them. No display name, address or group is among
/// them: those are the application's data, and facts of a stream (D5).
pub static FIELDS: [(&str, SceType, &str); 5] = [
    ("issuer", SceType::String, "string"),
    ("subject", SceType::String, "string"),
    ("proved_ms", SceType::Uint64, "uint64"),
    ("method", SceType::String, "string"),
    ("instance", SceType::String, "string"),
];

/// Refuse `model`, a document that reads the person an instance acts for, on the backend
/// `backend`, whose core holds no person (D10): the one wording every such refusal takes,
/// where a backend's generation begins and where a lowering with no place for a person is
/// asked for one. `Ok` for a document that reads none.
pub fn refuse_backend(model: &SCXMLModel, backend: &str) -> Result<(), GenerateError> {
    let Some(alias) = model.principal.as_deref() else {
        return Ok(());
    };
    Err(GenerateError::unsupported(format!(
        "`sce:principal=\"{alias}\"` in '{}' has no {backend} lowering: its machine holds no \
         person an instance acts for, which the Rust host core alone holds (docs/adr/0116, \
         D10); generate it with `--lang rust`",
        model.name
    )))
}

/// The type `_x.principal` and `_x.quoted` are of: an optional record of the schema the
/// document imports as `alias`.
pub fn value_type(alias: &str) -> Option<crate::forge::model::AlgorithmValueType> {
    crate::forge::value_type::WrittenType::read(&format!("optional<record:{alias}>")).ok()
}

/// The place `_x.principal` and `_x.quoted` name, typed by walking [`value_type`].
pub fn place(alias: &str) -> Option<crate::forge::value_path::Place> {
    Some(crate::forge::value_path::Place::of(
        value_type(alias)?.bounded().ok()?,
    ))
}

/// The alias `<scxml sce:principal>` names on `root`, or `None` for a document that
/// reads no person. Refused where it is written under a data model other than
/// `sce-static`, whose record of a person is a type the build holds it to, and when it
/// names nothing.
pub fn declared(
    root: &roxmltree::Node,
    model: &SCXMLModel,
    diag_label: &str,
) -> Result<Option<String>, Located<ForgeError>> {
    let Some(value) = crate::sce_attr::read(root, "principal") else {
        return Ok(None);
    };
    let refuse = |reason: &str| {
        Err(crate::forge::parser::located(
            root,
            diag_label,
            ValidationError::PrincipalRecord {
                element: "<scxml>".to_string(),
                value: value.to_string(),
                reason: reason.to_string(),
            },
        ))
    };
    if model.datamodel != Datamodel::SceStatic {
        return refuse(
            "a person is read under `datamodel=\"sce-static\"`, where the record a document \
             reads them as is a type the build holds it to",
        );
    }
    let alias = value.trim();
    if alias.is_empty() {
        return refuse("it names no record: name the imported event schema the person is read as");
    }
    Ok(Some(alias.to_string()))
}

/// Refuse the record `model` reads a person as (`<scxml sce:principal>`) unless it is an
/// imported event schema whose every field is one of [`FIELDS`], of that field's type; a
/// text states its bound, as every text does, and a person whose text is longer refuses the
/// delivery where it is taken in (`StatePolicy::admit_principal` of the Rust runtime). A
/// schema the build did not read (a document read with no directory beside it) is judged
/// where it is generated, which reads it.
pub fn check_declaration(model: &SCXMLModel, diag_label: &str) -> Result<(), Located<ForgeError>> {
    let Some(alias) = model.principal.as_deref() else {
        return Ok(());
    };
    let refuse = |reason: String| {
        Located::new(
            ValidationError::PrincipalRecord {
                element: "<scxml>".to_string(),
                value: alias.to_string(),
                reason,
            }
            .into(),
            diag_label,
            None,
            None,
        )
    };
    let imported = model
        .forge_imports
        .iter()
        .any(|import| import.alias == alias && import.kind == ForgeKind::EventSchema);
    if !imported {
        return Err(refuse(format!(
            "it names no imported event schema: import the record the person is read as \
             (`<sce:import kind=\"event-schema\" as=\"{alias}\"/>`)"
        )));
    }
    let Some(schema) = model.imported_records.get(alias) else {
        return Ok(());
    };
    if schema.fields.is_empty() {
        return Err(refuse(
            "the record declares no field, and a record a person is read as names at least one \
             of issuer, subject, proved_ms, method and instance"
                .to_string(),
        ));
    }
    let Ok(schema) = schema.entry_schema() else {
        return Err(refuse(
            "a field of the record is a list, a record or an optional value, and each field \
             of a person is one of issuer, subject, proved_ms, method and instance"
                .to_string(),
        ));
    };
    for field in &schema.fields {
        let Some((_, ty, spelling)) = FIELDS.iter().find(|(name, _, _)| *name == field.id) else {
            return Err(refuse(format!(
                "the record declares `{}`, which is none of the fields a person is read with: \
                 issuer, subject, proved_ms, method and instance",
                field.id
            )));
        };
        if !matches!(&field.sce_type, EntryType::Scalar(declared) if declared == ty) {
            return Err(refuse(format!(
                "the record declares `{}` as `{}`, and it is a `{spelling}`",
                field.id,
                field.sce_type.as_attr()
            )));
        }
    }
    Ok(())
}

/// The guard of a `<transition>` written `node`, whose own condition is `cond`: with
/// `sce:principal="required"` the condition `_x.principal !== null`, joined before `cond`
/// so that `cond` reads the person where it is proven present (D3). Refused where it is
/// written with another value, and on a document that declares no person.
pub fn guard_with_required(
    node: &roxmltree::Node,
    model: &SCXMLModel,
    source_name: &str,
    cond: String,
) -> Result<String, Located<ForgeError>> {
    let Some(value) = crate::sce_attr::read(node, "principal") else {
        return Ok(cond);
    };
    if value != REQUIRED {
        return Err(crate::forge::parser::located(
            node,
            source_name,
            ValidationError::AttributeRuleViolated {
                element: "transition".to_string(),
                attr: "sce:principal".to_string(),
                value: value.to_string(),
                rule: "\"required\", a transition taken only on an event said for a person"
                    .to_string(),
            },
        ));
    }
    if model.principal.is_none() {
        return Err(crate::forge::parser::located(
            node,
            source_name,
            ValidationError::PrincipalRecord {
                element: "<transition>".to_string(),
                value: value.to_string(),
                reason: "the document declares no person it reads (`<scxml sce:principal>`), \
                         so no event is said for one"
                    .to_string(),
            },
        ));
    }
    let present = format!("{PRINCIPAL} !== null");
    Ok(if cond.trim().is_empty() {
        present
    } else {
        format!("{present} && ({cond})")
    })
}

/// Refuse `<send sce:on-behalf>` written `node` (D4): a quote is refused until a
/// deployment lists the machines a machine takes one from, and the build is where it is
/// refused, before anything is sent to be dropped at the receiver's edge. The reason names
/// what the attribute is held to first: it quotes `_x.principal`, the one authority a
/// machine holds, on a document that declares the person it reads.
pub fn refuse_quote(
    node: &roxmltree::Node,
    model: &SCXMLModel,
    source_name: &str,
) -> Result<(), Located<ForgeError>> {
    let Some(value) = crate::sce_attr::read(node, "on-behalf") else {
        return Ok(());
    };
    let reason = if value.trim() != PRINCIPAL {
        format!(
            "it names `{}`, and the one authority a machine holds is the person the event it \
             is processing came for, `{PRINCIPAL}`",
            value.trim()
        )
    } else if model.principal.is_none() {
        "the document declares no person it reads (`<scxml sce:principal>`), so it holds none \
         to quote"
            .to_string()
    } else {
        "no deployment lists this machine among those its receiver takes a quote from, and \
         every quote is refused until one does"
            .to_string()
    };
    Err(crate::forge::parser::located(
        node,
        source_name,
        ValidationError::QuoteRefused {
            value: value.to_string(),
            reason,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fixed list is the one D3 names, each field once, and the type `_x.principal` is
    /// read as is an optional record of the declared schema. Red if a field is added to or
    /// dropped from the list, or the type is spelled as anything but an optional record.
    #[test]
    fn a_person_is_an_optional_record_of_five_fields_at_most() {
        let names: Vec<&str> = FIELDS.iter().map(|(name, _, _)| *name).collect();
        assert_eq!(
            names,
            ["issuer", "subject", "proved_ms", "method", "instance"]
        );
        let ty = value_type("Person").expect("a type of the grammar");
        assert_eq!(
            ty.optional_elem()
                .and_then(crate::forge::model::AlgorithmValueType::record_alias),
            Some("Person")
        );
    }
}
