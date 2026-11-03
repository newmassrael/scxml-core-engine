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
//! The plan holds, so far:
//!
//! - the interface: the elements a value presents, each with its abstract type,
//!   the refinement its type carries (docs/adr/0065 §6 R1, R2, R8), and whether
//!   it may be absent (docs/adr/0058 §1);
//! - the arms of the codec's variant;
//! - the derivations: each derived element and how the encode computes it
//!   ([`Derive`]), a number by what it measures and how it goes into the
//!   element that holds it.

use crate::forge::codec_roles::{
    self, BindSite, CodecRoles, Derivation, ElementId, GateGroup, InputValue,
};
use crate::forge::codec_totality::{self, RefusalKind, Totality};
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
    /// The arms of the codec's variant: those it names, in order, then the
    /// default. None where it has no variant.
    pub arms: Vec<Arm<'m>>,
    /// Each derived element and how the encode computes it, in the order of
    /// the roles, each derivation of an element apart.
    pub derivations: Vec<Derived<'m>>,
}

/// A derived element and one derivation of it.
pub struct Derived<'m> {
    /// The element.
    pub element: ElementId,
    /// How the encode computes it.
    pub by: Derive<'m>,
}

/// How the encode computes a derived element.
pub enum Derive<'m> {
    /// A number: a length, a count or an encoded length, or a flag bound to an
    /// input the codecs the value holds say (docs/adr/0065 §6 R4a, R8).
    Number(Number<'m>),
    /// The tag of the arm of the codec's variant the value holds; a default
    /// arm presents the tag it keeps.
    VariantTag,
    /// The bits of a carrier the arm of an embedded variant fills.
    ParentTag(ParentTag<'m>),
    /// A flag set exactly when the fields it gates are present.
    Presence(PresenceFlag<'m>),
    /// A flag of a disjunction that decides a group (docs/adr/0065 §6 R4c):
    /// what the arm of the group the value holds gives it.
    ArmFlag {
        /// The group, by its place among [`EncodePlan::groups`].
        group: usize,
        /// The flag, by its place among the group's gates.
        gate: usize,
    },
    /// The flag a chain's entry is closed by, which the chain sets on each
    /// entry as it writes it, and which is no value of this codec's own.
    ChainContinuation,
}

/// One arm of the codec's variant.
pub struct Arm<'m> {
    /// The import alias of the arm's codec.
    pub codec: &'m str,
    /// The tag the arm is chosen by, or `None` for the default arm, which
    /// keeps the tag no arm names.
    pub tag: Option<u64>,
    /// The build feature the arm is compiled under (docs/adr/0072).
    pub feature: Option<&'m str>,
}

/// The bits of the carrier `carrier` that the arms of embedded variants fill,
/// each body's arm masked to the bits: the terms are joined bit by bit.
pub struct ParentTag<'m> {
    /// The carrier.
    pub carrier: &'m CodecField,
    /// One for each embedded body whose arm fills the bits.
    pub terms: Vec<TagTerm<'m>>,
}

/// The bits the arm of the embedded body `body` fills: the tag of each arm,
/// masked to the bits, and nothing where a gated body is absent.
pub struct TagTerm<'m> {
    /// The embedded body.
    pub body: &'m CodecField,
    /// The import alias of its codec.
    pub codec: String,
    /// The bits the tag is masked to.
    pub mask: u64,
    /// Each arm of the body's variant and its masked tag.
    pub arms: Vec<TagArm>,
    /// Whether the body's variant has a default arm, which fills the bits with
    /// the tag it keeps, masked.
    pub catch_all: bool,
}

/// One arm of an embedded variant and the bits it fills.
pub struct TagArm {
    /// The import alias the embedded codec names the arm's body by.
    pub codec: String,
    /// The arm's tag, masked to the bits.
    pub tag: u64,
    /// The build feature the arm is compiled under (docs/adr/0072).
    pub feature: Option<String>,
}

/// A flag set exactly when the fields it gates are present (or absent, for a
/// field gated on the flag's clear state). Fields that disagree have no frame,
/// and the encode refuses them with [`RefusalKind::InconsistentSource`], unless
/// one group holds them all and they cannot disagree.
pub struct PresenceFlag<'m> {
    /// Each gated field, and whether it is present when the flag is clear.
    pub of: Vec<(&'m CodecField, bool)>,
    /// Whether one group holds every field, so the first says the flag.
    pub together: bool,
}

/// A derived number (docs/adr/0058 §1): what it measures, and how it goes into
/// the element that holds it.
///
/// The number is what is measured. Every measure that is there says the same
/// number, or no frame says them all, and the encode refuses the value with
/// [`RefusalKind::InconsistentSource`]. A measure always there fixes the number;
/// where every measure may be absent, the number is the first one there, and
/// [`NumberInto`] says what the element holds when none is.
pub struct Number<'m> {
    /// What it measures, in the order the declarations name it.
    pub measures: Vec<Measure<'m>>,
    /// Where the number goes.
    pub into: NumberInto<'m>,
}

impl Number<'_> {
    /// Whether some measure is always there, which then fixes the number.
    pub fn fixed(&self) -> bool {
        self.measures.iter().any(|m| !m.gated)
    }

    /// Whether every measure may be absent, and one is there at times: the
    /// number is then the first one there, and there may be none.
    pub fn free(&self) -> bool {
        !self.fixed() && !self.measures.is_empty()
    }
}

/// One thing a derived number measures.
pub struct Measure<'m> {
    /// What it is.
    pub of: Measured<'m>,
    /// Whether it may be absent, and the number is then not its.
    pub gated: bool,
    /// The place of the declaration that names it among those of the
    /// derivation, which a printer may name what it computes by.
    pub at: usize,
}

/// What a derived number measures.
pub enum Measured<'m> {
    /// How many bytes the bytes or text `field` holds, less what its
    /// `sce:length-arith` adds (`offset`).
    Bytes {
        /// The bytes or text.
        field: &'m CodecField,
        /// How the wire's number differs from the count of bytes.
        offset: Offset,
    },
    /// How many entries the list `list` holds.
    Entries {
        /// The list.
        list: &'m CodecField,
    },
    /// How many bytes the embedded body `body` encodes to, the codec imported
    /// as `codec`. A body that is absent encodes to none, so the measure is
    /// always there.
    Encoded {
        /// The embedded body.
        body: &'m CodecField,
        /// The import alias of its codec.
        codec: &'m str,
        /// Whether the body's encode can refuse a value, whose refusal the
        /// count then passes on.
        refuses: bool,
    },
    /// The input `input` the embedded body `body` says, of the codec imported
    /// as `codec` (docs/adr/0065 §6 R4a): 0 or 1.
    Says {
        /// The embedded body.
        body: &'m CodecField,
        /// The import alias of its codec.
        codec: String,
        /// The input.
        input: String,
    },
    /// The value of the input `input` the list `list` hands every entry and
    /// holds them to by its law: the list's start (docs/adr/0065 §6 R8).
    Start {
        /// The list.
        list: &'m CodecField,
        /// The input.
        input: String,
    },
    /// The input the arm of the variant the value holds says, where it holds
    /// one of `arms` (docs/adr/0065 §6 R4a). The measure is always there when
    /// every arm says it.
    ArmSays {
        /// Each arm that says it, in the order the declarations name them.
        arms: Vec<ArmInput>,
    },
}

/// An arm of the codec's variant and the input of its codec it says.
pub struct ArmInput {
    /// The arm, by its place among [`EncodePlan::arms`].
    pub arm: usize,
    /// The import alias of the arm's codec.
    pub codec: String,
    /// The input.
    pub input: String,
}

/// How the wire's length differs from the count of bytes it measures: the wire
/// holds `count - arith` (`sce:length-arith`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Offset {
    /// The wire holds the count.
    None,
    /// The wire holds the count less `by`. Where the content's type holds at
    /// least `by` bytes (`held`, docs/adr/0077 §3) that is never negative;
    /// otherwise a count below `by` is refused with
    /// [`RefusalKind::LengthOutOfRange`] at the content.
    Less {
        /// What is taken off.
        by: u32,
        /// Whether the content's type holds at least `by` bytes.
        held: bool,
    },
    /// The wire holds the count plus `by`.
    More {
        /// What is added.
        by: u32,
    },
}

/// Where a derived number goes.
pub enum NumberInto<'m> {
    /// Into the field `field`.
    Field {
        /// The field.
        field: &'m CodecField,
        /// How the number becomes a value of the field's type.
        narrow: Narrow,
        /// What the field holds where nothing it measures is there.
        absent: Absent,
    },
    /// Into a flag of a carrier, which keeps the bits the value presents where
    /// the number is free ([`Number::free`]) and nothing it measures is there.
    Flag,
}

/// How a derived number becomes a value of the type of the field it goes into
/// (docs/adr/0065 §5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Narrow {
    /// The number may not fit: one that does not is refused with
    /// [`RefusalKind::LengthOutOfRange`] at the field.
    Checked,
    /// The field is 64 bits wide, which holds every number.
    Widened,
    /// The number's bound fits the field, so nothing is checked.
    Cast,
}

/// What a field that holds a derived number holds where nothing it measures is
/// there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Absent {
    /// Something it measures is always there ([`Number::fixed`]), or it
    /// measures nothing and holds zero.
    Never,
    /// Zero: the field is gated beside its content and written only where
    /// there is content.
    Zero,
    /// What the group that holds the field holds on the side where its content
    /// is not (docs/adr/0065 §6 R4b).
    Group,
    /// What the value states (docs/adr/0058 §1, [`Presence::Stated`]). A value
    /// that states it where there is content must state the derived number.
    Stated,
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
        let arms = arms(model);
        let derivations = derivations(model, &roles, &totality, &arms, imports);
        EncodePlan {
            model,
            roles,
            totality,
            elements,
            arms,
            derivations,
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

/// Each derived element of `model` and how the encode computes it, in the order
/// of the roles.
fn derivations<'m>(
    model: &'m CodecModel,
    roles: &CodecRoles,
    totality: &Totality,
    arms: &[Arm<'m>],
    imports: &[ImportContext],
) -> Vec<Derived<'m>> {
    let field = |id: &str| model.fields.iter().find(|f| f.id == id);
    let mut out = Vec::new();
    for (element, by) in roles.derived() {
        for derivation in by {
            let measured = |measures: Vec<Measure<'m>>| {
                Derive::Number(number(model, roles, totality, element, measures))
            };
            let by = match derivation {
                Derivation::LengthOf { contents } => measured(
                    contents
                        .iter()
                        .enumerate()
                        .filter_map(|(at, content)| {
                            let field = field(&content.field)?;
                            Some(Measure {
                                of: Measured::Bytes {
                                    field,
                                    offset: offset(field, &model.fields, content.arith),
                                },
                                gated: field.present_if.is_some(),
                                at,
                            })
                        })
                        .collect(),
                ),
                // A list held with another is of its count already
                // (docs/adr/0065 §6 R5).
                Derivation::CountOf { lists } => measured(
                    lists
                        .iter()
                        .enumerate()
                        .filter(|(_, list)| !roles.counted_with_another(list))
                        .filter_map(|(at, list)| Some((at, field(list)?)))
                        .map(|(at, list)| Measure {
                            of: Measured::Entries { list },
                            gated: list.present_if.is_some(),
                            at,
                        })
                        .collect(),
                ),
                Derivation::EncodedLengthOf { bodies } => measured(
                    bodies
                        .iter()
                        .enumerate()
                        .filter_map(|(at, body)| Some((at, field(body)?)))
                        .map(|(at, body)| {
                            let codec = codec_totality::child_alias(body)
                                .expect("a body whose length is derived holds a codec");
                            Measure {
                                of: Measured::Encoded {
                                    body,
                                    codec,
                                    refuses: imports.iter().any(|i| {
                                        i.alias == codec
                                            && i.kind == "codec"
                                            && i.codec_source_can_refuse
                                    }),
                                },
                                gated: false,
                                at,
                            }
                        })
                        .collect(),
                ),
                Derivation::BoundInput { binders } => measured(bound_input(model, binders, arms)),
                Derivation::VariantTag { .. } => Derive::VariantTag,
                Derivation::ParentTag { .. } => match parent_tag(model, imports, element) {
                    Some(tag) => Derive::ParentTag(tag),
                    // A carrier the codec does not hold has no bits to fill.
                    None => continue,
                },
                Derivation::PresenceOf { gated } => Derive::Presence(PresenceFlag {
                    of: gated
                        .iter()
                        .filter_map(|g| Some((field(&g.field)?, g.when_clear)))
                        .collect(),
                    // Fields one group holds are present together
                    // (docs/adr/0065 §6 R4b).
                    together: gated
                        .first()
                        .and_then(|g| roles.group_of(&g.field))
                        .is_some_and(|group| gated.iter().all(|g| group.contains(&g.field))),
                }),
                Derivation::ArmOf { gates } => arm_flag(roles, element, gates),
                Derivation::ChainContinuation { .. } => Derive::ChainContinuation,
            };
            out.push(Derived {
                element: element.clone(),
                by,
            });
        }
    }
    out
}

/// What a flag bound to an input of the codecs the value holds measures
/// (docs/adr/0065 §6 R4a, R8): the input each embedded body says, the start of
/// each list that hands it to its entries, and the input the arm the value
/// holds says, where one that binds it is held.
fn bound_input<'m>(
    model: &'m CodecModel,
    binders: &[codec_roles::Binder],
    arms: &[Arm<'m>],
) -> Vec<Measure<'m>> {
    let site = |id: &str| {
        model
            .fields
            .iter()
            .find(|f| f.id == id)
            .expect("a bind site is a field of the codec")
    };
    let mut measures = Vec::new();
    let mut said = Vec::new();
    for (at, binder) in binders.iter().enumerate() {
        match &binder.site {
            BindSite::Embed { field } => {
                let body = site(field);
                measures.push(Measure {
                    of: Measured::Says {
                        body,
                        codec: binder.alias.clone(),
                        input: binder.input.clone(),
                    },
                    gated: body.present_if.is_some(),
                    at,
                });
            }
            BindSite::Arm => {
                for (arm, _) in arms
                    .iter()
                    .enumerate()
                    .filter(|(_, arm)| arm.codec == binder.alias)
                {
                    said.push(ArmInput {
                        arm,
                        codec: binder.alias.clone(),
                        input: binder.input.clone(),
                    });
                }
            }
            BindSite::List { field } => {
                let list = site(field);
                measures.push(Measure {
                    of: Measured::Start {
                        list,
                        input: binder.input.clone(),
                    },
                    gated: list.present_if.is_some(),
                    at,
                });
            }
        }
    }
    // The arms are one measure, there when the value holds one that says it:
    // always, where every arm does.
    if !said.is_empty() {
        let every = said.len() == arms.len();
        measures.push(Measure {
            of: Measured::ArmSays { arms: said },
            gated: !every,
            at: binders.len(),
        });
    }
    measures
}

/// The bits of the carrier flag `element` that the arms of embedded variants
/// fill, or `None` where the codec holds no such carrier.
fn parent_tag<'m>(
    model: &'m CodecModel,
    imports: &[ImportContext],
    element: &ElementId,
) -> Option<ParentTag<'m>> {
    let ElementId::Flag { field, flag } = element else {
        unreachable!("a parent tag fills a flag of a carrier");
    };
    let carrier = model.fields.iter().find(|f| &f.id == field)?;
    let terms = crate::forge::generator::collect_parent_tag_derivations(model, imports)
        .into_iter()
        .filter(|d| &d.carrier_field_id == field && &d.carrier_flag == flag)
        .filter_map(|d| {
            let import = imports.iter().find(|i| i.alias == d.embed_alias)?;
            let body = model.fields.iter().find(|f| f.id == d.embed_field_id)?;
            let mask: u64 = (1u64 << d.width) - 1;
            Some(TagTerm {
                body,
                mask,
                arms: d
                    .arms
                    .iter()
                    .map(|arm| TagArm {
                        codec: arm.body_alias.clone(),
                        tag: arm.value & mask,
                        feature: arm.feature.clone(),
                    })
                    .collect(),
                // A catch-all arm presents the tag it could not name, and that
                // is what the carrier holds.
                catch_all: import.codec_variant_has_catch_all,
                codec: d.embed_alias,
            })
        })
        .collect();
    Some(ParentTag { carrier, terms })
}

/// The flag `element` of the disjunction `gates` that decides a group
/// (docs/adr/0065 §6 R4c).
fn arm_flag<'m>(
    roles: &CodecRoles,
    element: &ElementId,
    gates: &[codec_roles::GateVar],
) -> Derive<'m> {
    let ElementId::Flag { field, flag } = element else {
        unreachable!("a disjunction's gate is a flag of a carrier");
    };
    let group = roles
        .groups
        .iter()
        .position(|g| g.gates == gates)
        .expect("a flag derived from an arm has its group");
    let gate = roles.groups[group]
        .gates
        .iter()
        .position(|g| {
            matches!(g, codec_roles::GateVar::Flag { carrier, flag: f } if carrier == field && f == flag)
        })
        .expect("the flag is one of the group's gates");
    Derive::ArmFlag { group, gate }
}

/// The arms of the variant of `model`: those it names, in order, then the
/// default.
fn arms(model: &CodecModel) -> Vec<Arm<'_>> {
    let Some(variant) = &model.variant else {
        return Vec::new();
    };
    let mut arms: Vec<Arm<'_>> = variant
        .arms
        .iter()
        .map(|arm| Arm {
            codec: &arm.body_alias,
            tag: Some(arm.value),
            feature: arm.feature.as_deref(),
        })
        .collect();
    if let Some(default) = &variant.default_arm {
        arms.push(Arm {
            codec: &default.body_alias,
            tag: None,
            feature: None,
        });
    }
    arms
}

/// The derived number of `element` that `measures` say.
fn number<'m>(
    model: &'m CodecModel,
    roles: &CodecRoles,
    totality: &Totality,
    element: &ElementId,
    measures: Vec<Measure<'m>>,
) -> Number<'m> {
    let mut number = Number {
        measures,
        into: NumberInto::Flag,
    };
    number.into = match element {
        ElementId::Field(name) => {
            let field = model
                .fields
                .iter()
                .find(|f| &f.id == name)
                .expect("a derived field is a field of the codec");
            let narrow = if totality.refuses(RefusalKind::LengthOutOfRange, &element.to_string()) {
                Narrow::Checked
            } else if field.sce_type == SceType::Uint64 {
                Narrow::Widened
            } else {
                Narrow::Cast
            };
            let absent = if !number.free() {
                Absent::Never
            } else if !codec_totality::stated(model, roles, field) {
                Absent::Zero
            } else if roles.group_of(&field.id).is_some() {
                Absent::Group
            } else {
                Absent::Stated
            };
            NumberInto::Field {
                field,
                narrow,
                absent,
            }
        }
        ElementId::Flag { .. } => NumberInto::Flag,
        ElementId::Variant => unreachable!("a length or a count is not the variant"),
    };
    number
}

/// How the wire's length of the content `field` differs from its count of
/// bytes, for an `sce:length-arith` of `arith`.
fn offset(field: &CodecField, fields: &[CodecField], arith: i32) -> Offset {
    match arith {
        0 => Offset::None,
        arith if arith > 0 => Offset::Less {
            by: arith.unsigned_abs(),
            held: codec_totality::held_to_offset(field, fields, arith),
        },
        arith => Offset::More {
            by: arith.unsigned_abs(),
        },
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
