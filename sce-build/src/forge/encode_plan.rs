// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The encode plan of a codec (docs/adr/0081 §1): what every backend's encode
//! of the codec shares, decided once, with none of any backend's spelling.
//!
//! The plan is built from the model, its roles ([`codec_roles`]) and its
//! totality ([`codec_totality`]). A backend is a printer of the plan
//! (docs/adr/0081 §2): it spells what the plan holds and decides none of it.
//! This module therefore names no type of a target language, and imports no
//! generator context that spells one.
//!
//! The plan holds, so far, the interface: the elements a value presents, each
//! with its abstract type, the refinement its type carries (docs/adr/0065 §6 R1,
//! R2, R8), and whether it may be absent (docs/adr/0058 §1).

use crate::forge::codec_roles::{self, BindSite, CodecRoles, GateGroup, InputValue};
use crate::forge::codec_totality::{self, Totality};
use crate::forge::generator::ImportContext;
use crate::forge::model::{CodecField, CodecModel, SceType};

/// The encode plan of one positional codec.
pub struct EncodePlan<'m> {
    /// The codec the plan is of.
    pub model: &'m CodecModel,
    /// What each element of the codec is: presented, or derived and from what.
    pub roles: CodecRoles,
    /// Where the encode can refuse a value, the one statement of it.
    pub totality: Totality,
    /// The elements a value presents by an accessor of its own, in declaration
    /// order. A field a gate group holds is presented by its group
    /// ([`EncodePlan::groups`]).
    pub elements: Vec<Element<'m>>,
}

/// One element a value presents by an accessor of its own.
pub struct Element<'m> {
    /// The field it is.
    pub field: &'m CodecField,
    /// What it holds where it is present.
    pub value: Value<'m>,
    /// Whether it may be absent.
    pub presence: Presence,
}

/// What an element holds where it is present: its abstract type, with the
/// refinement the type carries.
pub enum Value<'m> {
    /// An integer, a flag set or an enum, by its declared type.
    Scalar(&'m SceType),
    /// Bytes, held to their bound (docs/adr/0077 §3, docs/adr/0065 §6 R1).
    Bytes(Bound),
    /// Text, held to its bound (docs/adr/0077 §3, docs/adr/0065 §6 R1).
    Text(Bound),
    /// The body of the codec imported as `codec`, embedded in place.
    Body {
        /// The import alias of the codec.
        codec: &'m str,
    },
    /// A list or a chain of entries of the codec imported as `codec`, held to
    /// its bound (docs/adr/0065 §6 R2, R7) and to its law (§6 R8).
    List {
        /// The import alias of the entries' codec.
        codec: &'m str,
        /// How many entries it holds.
        bound: Bound,
        /// The law its entries follow; empty where they say no input the list
        /// hands them.
        law: Law,
    },
}

/// Whether an element may be absent (docs/adr/0058 §1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Presence {
    /// It is always there.
    Always,
    /// A gate decides it (`sce:present-if`).
    Gated,
    /// A length or count of gated content, stated by the value only while all
    /// the content it measures is absent and derived the rest of the time.
    Stated,
}

impl Presence {
    /// Whether an accessor of the element answers `None` at times.
    pub fn optional(self) -> bool {
        !matches!(self, Presence::Always)
    }
}

/// How many bytes, characters or entries an element holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bound {
    /// The fewest.
    pub least: u64,
    /// The most, or `None` where the protocol states none (docs/adr/0077 §7).
    pub most: Option<u64>,
}

/// The law the entries of a list follow (docs/adr/0065 §6 R8): an automaton for
/// each input the list hands its entries and they say, run together.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Law {
    /// One automaton for each input, in the order the law holds them.
    pub inputs: Vec<LawInput>,
}

impl Law {
    /// Whether the list has no law: its entries say no input it hands them.
    pub fn is_none(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// The automaton over one input of a list's law.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LawInput {
    /// The input of the entries' codec.
    pub input: String,
    /// How the entries say it.
    pub automaton: Automaton,
}

/// How the entries of a list say one input (docs/adr/0065 §6 R8).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Automaton {
    /// Every entry says the one value the list hands all of them, its start.
    Uniform,
    /// Each entry says whether the entry before it is of the kind `id`.
    AfterKey {
        /// The identifier of the kind.
        id: u64,
    },
}

impl<'m> EncodePlan<'m> {
    /// The plan of the positional codec `model`, given what its imports are.
    pub fn of(model: &'m CodecModel, imports: &[ImportContext]) -> Self {
        let roles = codec_roles::classify(model, imports);
        let totality = codec_totality::analyse(model, &mut codec_totality::Resolved(imports));
        let elements = model
            .fields
            .iter()
            .filter(|field| presented_alone(model, &roles, field))
            .map(|field| Element {
                field,
                value: value(model, &roles, field),
                presence: presence(model, &roles, field),
            })
            .collect();
        EncodePlan {
            model,
            roles,
            totality,
            elements,
        }
    }

    /// The gate groups of the codec (docs/adr/0065 §6 R4): each holds the
    /// fields its gates decide, and a value presents them together.
    pub fn groups(&self) -> &[GateGroup] {
        &self.roles.groups
    }

    /// What `field` holds where it is present, whether a value presents it by
    /// an accessor of its own or as a member of a group.
    pub fn value<'f>(&self, field: &'f CodecField) -> Value<'f> {
        value(self.model, &self.roles, field)
    }

    /// Whether `field` may be absent.
    pub fn presence(&self, field: &CodecField) -> Presence {
        presence(self.model, &self.roles, field)
    }

    /// Whether a value presents `field` by an accessor of its own.
    pub fn presents_alone(&self, field: &CodecField) -> bool {
        presented_alone(self.model, &self.roles, field)
    }
}

/// Whether a value presents `field` by an accessor of its own: it is presented
/// or stated, and no group holds it.
fn presented_alone(model: &CodecModel, roles: &CodecRoles, field: &CodecField) -> bool {
    (codec_totality::presented(roles, field) || codec_totality::stated(model, roles, field))
        && roles.group_of(&field.id).is_none()
}

fn presence(model: &CodecModel, roles: &CodecRoles, field: &CodecField) -> Presence {
    if field.present_if.is_some() {
        Presence::Gated
    } else if codec_totality::stated(model, roles, field) {
        Presence::Stated
    } else {
        Presence::Always
    }
}

fn value<'f>(model: &CodecModel, roles: &CodecRoles, field: &'f CodecField) -> Value<'f> {
    if let Some(codec) = codec_totality::child_alias(field) {
        return if field.is_embed() {
            Value::Body { codec }
        } else {
            Value::List {
                codec,
                bound: list_bound(field, &model.fields),
                law: list_law(roles, &field.id),
            }
        };
    }
    match &field.sce_type {
        SceType::Bytes => Value::Bytes(byte_bound(field, &model.fields)),
        SceType::String => Value::Text(byte_bound(field, &model.fields)),
        other => Value::Scalar(other),
    }
}

/// The bound of the `bytes` or `string` field `field` ([`codec_totality::byte_bound`]).
///
/// # Panics
///
/// Where `field` holds no length: a codec's bytes and text are a tail or a
/// length-ref field, which have bounds.
pub fn byte_bound(field: &CodecField, fields: &[CodecField]) -> Bound {
    let bound = codec_totality::byte_bound(field, fields)
        .expect("a codec's byte string or text is a tail or length-ref field, which has bounds");
    Bound {
        least: bound.min,
        most: bound.max,
    }
}

/// The bound of the list `field` ([`codec_totality::list_bound`],
/// [`codec_totality::list_least`]).
pub fn list_bound(field: &CodecField, fields: &[CodecField]) -> Bound {
    Bound {
        least: codec_totality::list_least(field),
        most: codec_totality::list_bound(field, fields),
    }
}

/// The law the entries of the list `field` follow (docs/adr/0065 §6 R8), read
/// off the roles: an input bound at the list and checked against a carrier or
/// a parent's input is [`Automaton::Uniform`], one checked against the entry
/// before is [`Automaton::AfterKey`].
pub fn list_law(roles: &CodecRoles, field: &str) -> Law {
    Law {
        inputs: roles
            .list_laws
            .iter()
            .filter(|law| matches!(&law.binder.site, BindSite::List { field: f } if f == field))
            .map(|law| LawInput {
                input: law.binder.input.clone(),
                automaton: match &law.against {
                    InputValue::PreviousEntry { id } => Automaton::AfterKey { id: *id },
                    InputValue::Carrier { .. } | InputValue::Input { .. } => Automaton::Uniform,
                },
            })
            .collect(),
    }
}
