// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which element of a codec a value presents, and which the encoding derives
//! (docs/adr/0058 §1).
//!
//! An element of a codec is either **presented** — information the value carries
//! that nothing else on the wire determines — or **derived** — determined by what
//! is presented. The rule is by declaration and not by appearance: an element is
//! derived only where the codec declares the derivation, and the declarations are
//! the ones a decoder reads to walk a frame:
//!
//! - a length field named by `sce:length-field` is the length of the bytes or
//!   string that points at it ([`Derivation::LengthOf`]);
//! - a count field named by `<sce:repeat count=>` is the length of its list
//!   ([`Derivation::CountOf`]);
//! - a field named by `sce:length-from` on an embedded body is that body's
//!   encoded length ([`Derivation::EncodedLengthOf`]);
//! - the tag of an own-field `<sce:variant>` is the active arm's
//!   ([`Derivation::VariantTag`]);
//! - the bit-range of a carrier named by `<sce:variant-dispatch flag=>` on an
//!   import is filled from the embedded variant's active arm
//!   ([`Derivation::ParentTag`]);
//! - the flag a `<sce:tlv-chain terminate-on="entry-flag">` ends on is set on
//!   every entry but the last ([`Derivation::ChainContinuation`]);
//! - the flag a field is gated on (`sce:present-if="carrier.flag"`) is set
//!   exactly when the field is present ([`Derivation::PresenceOf`]). A gate of
//!   any other form (a disjunction, a flag-input, a chain's membership) names no
//!   one flag to set, so it is a [`Check`] the measure holds a source to.
//!
//! Everything else is presented, a header flag bit among it: a flag that says
//! something the content does not (a header bit that marks an interest as a
//! subscription to tokens or not) is a thing a value can say.
//!
//! This is the one place the classification is written. The generator's
//! decoder reads the same declarations, and the encoder that derives from them
//! (ADR 0058, step 2) reads this module and not the model a second time.
//! [`CodecRoles::table`] renders it as the table a consumer reads to see which
//! of the values it sets by hand would move.

use std::collections::BTreeMap;
use std::fmt;

use crate::forge::generator::{collect_parent_tag_derivations, ImportContext};
use crate::forge::model::{
    BitSize, CodecEncoding, CodecModel, CountRef, PresentIfPredicate, PresentIfScope,
    TlvTerminateStrategy,
};

/// One element of a codec, as the table names it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementId {
    /// A field of the codec.
    Field(String),
    /// A named bit-range of a field (`field.flag`).
    Flag { field: String, flag: String },
    /// The arm of the codec's `<sce:variant>`, whole.
    Variant,
}

impl ElementId {
    /// The element a reference names: `field` or the dotted `field.flag`.
    fn of_reference(reference: &str) -> ElementId {
        match reference.split_once('.') {
            Some((field, flag)) => ElementId::Flag {
                field: field.trim().to_string(),
                flag: flag.trim().to_string(),
            },
            None => ElementId::Field(reference.trim().to_string()),
        }
    }
}

impl fmt::Display for ElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElementId::Field(field) => f.write_str(field),
            ElementId::Flag { field, flag } => write!(f, "{field}.{flag}"),
            ElementId::Variant => f.write_str("<variant>"),
        }
    }
}

/// The bytes or string that a length element is the length of, and what the
/// declaration adds to it (`sce:length-arith`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measured {
    pub field: String,
    pub arith: i32,
}

/// How the encoding determines a derived element.
///
/// Several declarations of one kind may name one element: a count shared by two
/// lists, a length shared by two fields. They are one derivation with several
/// sources, and the sources must then agree in size — a check the encoder's
/// measure makes, not a second derivation ([`Derivation::sources`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Derivation {
    /// The length of `contents`, bytes or string fields that name this element
    /// in `sce:length-field`, each less its `sce:length-arith`.
    LengthOf { contents: Vec<Measured> },
    /// The number of entries of `lists`, `<sce:repeat count=>` fields.
    CountOf { lists: Vec<String> },
    /// The encoded length of `bodies`, embedded codecs that name this element
    /// in `sce:length-from`.
    EncodedLengthOf { bodies: Vec<String> },
    /// The tag of the active arm. `default_arm_presents` is true where the
    /// codec has a `<sce:default>` arm: that arm keeps the tag it could not
    /// name, so the arm presents it.
    VariantTag { default_arm_presents: bool },
    /// The bit-range filled from the active arm of the variants embedded as
    /// `embeds`, which the import declares with `<sce:variant-dispatch>`.
    ParentTag { embeds: Vec<String> },
    /// The flag of each entry of `chains`, set on every entry but the last: the
    /// flag the chain ends on (`terminate-on="entry-flag"`).
    ChainContinuation { chains: Vec<String> },
    /// The flag that gates `gated`: set exactly when each of them is present
    /// (clear exactly when it is, for a field gated on `!flag`). A flag and the
    /// presence it announces are one fact, kept by the value in the presence and
    /// by the encoding in the flag. Only a gate of the one-flag form is
    /// derived; any other is a [`Check`].
    PresenceOf { gated: Vec<Gated> },
}

/// A field a flag gates, and whether the gate is `!flag`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gated {
    pub field: String,
    /// The field is present when the flag is clear.
    pub when_clear: bool,
}

/// A relation between presented elements that the encoder's measure holds a
/// source to, because the codec cannot derive one side of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    /// The gated field.
    pub field: String,
    /// The gate as the codec states it (`a.X || b.Y`, `ext.has(0x1)`, a bare
    /// flag-input name): the field is present exactly when it holds.
    pub gate: String,
}

impl Derivation {
    /// How many fields this element is derived from. More than one is a
    /// derivation the sources must agree on.
    pub fn sources(&self) -> usize {
        match self {
            Derivation::LengthOf { contents } => contents.len(),
            Derivation::CountOf { lists } => lists.len(),
            Derivation::EncodedLengthOf { bodies } => bodies.len(),
            Derivation::ParentTag { embeds } => embeds.len(),
            Derivation::ChainContinuation { chains } => chains.len(),
            Derivation::PresenceOf { gated } => gated.len(),
            Derivation::VariantTag { .. } => 1,
        }
    }

    /// Whether two derivations are of one kind (and so one derivation).
    fn same_kind(&self, other: &Derivation) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Fold `other`, of the same kind, into this.
    fn absorb(&mut self, other: Derivation) {
        match (self, other) {
            (Derivation::LengthOf { contents }, Derivation::LengthOf { contents: more }) => {
                contents.extend(more)
            }
            (Derivation::CountOf { lists }, Derivation::CountOf { lists: more }) => {
                lists.extend(more)
            }
            (
                Derivation::EncodedLengthOf { bodies },
                Derivation::EncodedLengthOf { bodies: more },
            ) => bodies.extend(more),
            (Derivation::ParentTag { embeds }, Derivation::ParentTag { embeds: more }) => {
                embeds.extend(more)
            }
            (
                Derivation::ChainContinuation { chains },
                Derivation::ChainContinuation { chains: more },
            ) => chains.extend(more),
            (Derivation::PresenceOf { gated }, Derivation::PresenceOf { gated: more }) => {
                gated.extend(more)
            }
            // One tag per codec: a second is the same declaration read twice.
            (Derivation::VariantTag { .. }, Derivation::VariantTag { .. }) => {}
            _ => unreachable!("absorb is called for derivations of one kind"),
        }
    }
}

impl fmt::Display for Derivation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = |names: &[String]| names.join(", ");
        match self {
            Derivation::LengthOf { contents } => {
                let each: Vec<String> = contents
                    .iter()
                    .map(|m| match m.arith {
                        0 => m.field.clone(),
                        arith => format!("{} {arith:+}", m.field),
                    })
                    .collect();
                write!(f, "length-of({})", each.join(", "))
            }
            Derivation::CountOf { lists } => write!(f, "count-of({})", names(lists)),
            Derivation::EncodedLengthOf { bodies } => {
                write!(f, "encoded-length-of({})", names(bodies))
            }
            Derivation::VariantTag {
                default_arm_presents: true,
            } => f.write_str("variant-tag (the default arm presents its own)"),
            Derivation::VariantTag { .. } => f.write_str("variant-tag"),
            Derivation::ParentTag { embeds } => write!(f, "parent-tag({})", names(embeds)),
            Derivation::ChainContinuation { chains } => {
                write!(f, "chain-continuation({})", names(chains))
            }
            Derivation::PresenceOf { gated } => {
                let each: Vec<String> = gated
                    .iter()
                    .map(|g| match g.when_clear {
                        false => format!("presence-of({})", g.field),
                        true => format!("absence-of({})", g.field),
                    })
                    .collect();
                f.write_str(&each.join(", "))
            }
        }?;
        if self.sources() > 1 {
            f.write_str(" [the sources agree in size]")?;
        }
        Ok(())
    }
}

/// What an element is to a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    /// The value says it.
    Presented,
    /// The encoding determines it. More than one derivation is an element
    /// declarations of different kinds claim, which no encoder can serve:
    /// [`CodecRoles::ambiguous`].
    Derived(Vec<Derivation>),
}

/// One row of the table: an element and its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: ElementId,
    pub role: Role,
}

/// A codec's elements, each with its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecRoles {
    pub codec: String,
    /// How the codec lays its fields on the wire: only a positional codec has
    /// anything to derive.
    pub encoding: CodecEncoding,
    pub rows: Vec<Row>,
    /// The gates the codec cannot derive a flag from (a disjunction, a
    /// flag-input, a chain's membership): the measure holds a source to them.
    pub checks: Vec<Check>,
    /// A derivation that names an element the codec does not have. A codec the
    /// parser accepted has none; a non-empty list is a defect of the model.
    pub unresolved: Vec<String>,
}

impl CodecRoles {
    /// The role of an element, `None` when the codec has no such element.
    pub fn role_of(&self, id: &ElementId) -> Option<&Role> {
        self.rows.iter().find(|r| &r.id == id).map(|r| &r.role)
    }

    /// The derived elements, in table order.
    pub fn derived(&self) -> impl Iterator<Item = (&ElementId, &[Derivation])> {
        self.rows.iter().filter_map(|row| match &row.role {
            Role::Derived(by) => Some((&row.id, by.as_slice())),
            Role::Presented => None,
        })
    }

    /// The elements that declarations of different kinds derive: one no encoder
    /// can serve, since it cannot take its value from both.
    pub fn ambiguous(&self) -> Vec<&ElementId> {
        self.derived()
            .filter(|(_, by)| by.len() > 1)
            .map(|(id, _)| id)
            .collect()
    }

    /// The elements derived from more than one field, each of which the
    /// encoder's measure holds to one size (a count shared by two lists).
    pub fn shared(&self) -> Vec<&ElementId> {
        self.derived()
            .filter(|(_, by)| by.iter().any(|d| d.sources() > 1))
            .map(|(id, _)| id)
            .collect()
    }

    /// The table a consumer reads: one line per element, presented or derived,
    /// in declaration order.
    pub fn table(&self) -> String {
        let width = self
            .rows
            .iter()
            .map(|r| r.id.to_string().len())
            .max()
            .unwrap_or(0);
        let mut out = format!("codec {}\n", self.codec);
        if self.rows.is_empty() {
            out.push_str("  (no elements)\n");
        }
        for row in &self.rows {
            let id = row.id.to_string();
            match &row.role {
                Role::Presented => out.push_str(&format!("  {id:<width$}  presented\n")),
                Role::Derived(by) => {
                    let by: Vec<String> = by.iter().map(Derivation::to_string).collect();
                    out.push_str(&format!("  {id:<width$}  derived  {}\n", by.join(" AND ")));
                }
            }
        }
        for check in &self.checks {
            out.push_str(&format!(
                "  check  `{}` is present exactly when {}\n",
                check.field, check.gate
            ));
        }
        for reference in &self.unresolved {
            out.push_str(&format!("  UNRESOLVED  {reference}\n"));
        }
        out
    }
}

/// The gate `predicate` states, as the codec spells it.
fn describe_gate(predicate: &PresentIfPredicate) -> String {
    let clause = match predicate.scope {
        PresentIfScope::Local => format!("{}.{}", predicate.field_id, predicate.flag_name),
        PresentIfScope::Input => predicate.flag_name.clone(),
        PresentIfScope::Chain => {
            // As the author wrote the identifier, else as a number.
            let entry = if predicate.entry_id_text.is_empty() {
                predicate.entry_id.unwrap_or_default().to_string()
            } else {
                predicate.entry_id_text.clone()
            };
            format!("{}.has({entry})", predicate.field_id)
        }
    };
    let clause = if predicate.negate {
        format!("!{clause}")
    } else {
        clause
    };
    match &predicate.or_with {
        Some(rest) => format!("{clause} || {}", describe_gate(rest)),
        None => clause,
    }
}

/// Classify the elements of `model`. `imports` is the codec's resolved import
/// list ([`ImportContext`]), which the parent-tag carriers are read from; a codec
/// that imports nothing takes `&[]`.
pub fn classify(model: &CodecModel, imports: &[ImportContext]) -> CodecRoles {
    let mut rows: Vec<Row> = Vec::new();
    let mut checks: Vec<Check> = Vec::new();
    let mut derivations: BTreeMap<ElementId, Vec<Derivation>> = BTreeMap::new();
    // Declarations of one kind that name one element are one derivation with
    // several sources; those of different kinds stay apart, and are ambiguous.
    let mut derive = |id: ElementId, by: Derivation| {
        let all = derivations.entry(id).or_default();
        match all.iter_mut().find(|known| known.same_kind(&by)) {
            Some(known) => known.absorb(by),
            None => all.push(by),
        }
    };

    match model.encoding {
        CodecEncoding::Positional => {}
        // The heads, keys, folding and escapes of these encodings are computed by
        // the encoding rules and appear in no value: every entry is presented.
        CodecEncoding::Cbor => {
            return flat(model, model.cbor_entries.iter().map(|e| e.id.as_str()));
        }
        CodecEncoding::ContentLine => {
            let entries = model.content_line.iter().flat_map(|c| c.entries.iter());
            return flat(model, entries.map(|e| e.id.as_str()));
        }
    }

    for field in &model.fields {
        rows.push(Row {
            id: ElementId::Field(field.id.clone()),
            role: Role::Presented,
        });
        for flag in &field.flags {
            rows.push(Row {
                id: ElementId::Flag {
                    field: field.id.clone(),
                    flag: flag.name.clone(),
                },
                role: Role::Presented,
            });
        }
        // A field gated on one flag of a carrier of this codec is present
        // exactly when the flag is set: the flag is the presence announced.
        // Any other gate (a disjunction, a flag-input the caller supplies, a
        // chain's membership) names no one flag this codec could set, so it is
        // a relation the measure checks.
        if let Some(gate) = &field.present_if {
            if gate.scope == PresentIfScope::Local && gate.or_with.is_none() {
                derive(
                    ElementId::Flag {
                        field: gate.field_id.clone(),
                        flag: gate.flag_name.clone(),
                    },
                    Derivation::PresenceOf {
                        gated: vec![Gated {
                            field: field.id.clone(),
                            when_clear: gate.negate,
                        }],
                    },
                );
            } else {
                checks.push(Check {
                    field: field.id.clone(),
                    gate: describe_gate(gate),
                });
            }
        }
        match &field.bit_size {
            BitSize::LengthRef => {
                if let Some(length) = &field.length_field {
                    derive(
                        ElementId::of_reference(length),
                        Derivation::LengthOf {
                            contents: vec![Measured {
                                field: field.id.clone(),
                                arith: field.length_arith.unwrap_or(0),
                            }],
                        },
                    );
                }
            }
            BitSize::Repeat {
                count_ref: CountRef::LengthField(count),
            } => derive(
                ElementId::of_reference(count),
                Derivation::CountOf {
                    lists: vec![field.id.clone()],
                },
            ),
            BitSize::Embed => {
                if let Some(length) = &field.embed_length_from {
                    derive(
                        ElementId::of_reference(length),
                        Derivation::EncodedLengthOf {
                            bodies: vec![field.id.clone()],
                        },
                    );
                }
            }
            BitSize::TlvChain {
                terminate_on: TlvTerminateStrategy::EntryFlag { flag_name },
                ..
            } => {
                // The flag is the entry codec's, set by the chain: it is an
                // element of this codec only as the chain's.
                let id = ElementId::Flag {
                    field: field.id.clone(),
                    flag: flag_name.clone(),
                };
                rows.push(Row {
                    id: id.clone(),
                    role: Role::Presented,
                });
                derive(
                    id,
                    Derivation::ChainContinuation {
                        chains: vec![field.id.clone()],
                    },
                );
            }
            _ => {}
        }
    }

    if let Some(variant) = &model.variant {
        rows.push(Row {
            id: ElementId::Variant,
            role: Role::Presented,
        });
        // A peek-byte variant has no tag of its own: the byte stays on the cursor
        // for the arm's body to read as its own header. A caller-tag variant has
        // none either; the caller supplies it.
        if variant.peek_byte.is_none() {
            if let Some(tag) = &variant.tag_field {
                let id = match &variant.tag_flag {
                    Some(flag) => ElementId::Flag {
                        field: tag.clone(),
                        flag: flag.clone(),
                    },
                    None => ElementId::Field(tag.clone()),
                };
                derive(
                    id,
                    Derivation::VariantTag {
                        default_arm_presents: variant.default_arm.is_some(),
                    },
                );
            }
        }
    }

    for parent in collect_parent_tag_derivations(model, imports) {
        derive(
            ElementId::Flag {
                field: parent.carrier_field_id.clone(),
                flag: parent.carrier_flag.clone(),
            },
            Derivation::ParentTag {
                embeds: vec![parent.embed_field_id.clone()],
            },
        );
    }

    let mut unresolved = Vec::new();
    for (id, by) in derivations {
        match rows.iter_mut().find(|row| row.id == id) {
            Some(row) => row.role = Role::Derived(by),
            None => unresolved.push(format!(
                "{} names `{id}`, which the codec does not have",
                by.iter()
                    .map(Derivation::to_string)
                    .collect::<Vec<_>>()
                    .join(" AND ")
            )),
        }
    }
    CodecRoles {
        codec: model.name.clone(),
        encoding: model.encoding,
        rows,
        checks,
        unresolved,
    }
}

/// The roles of a codec whose every entry is presented.
fn flat<'a>(model: &CodecModel, ids: impl Iterator<Item = &'a str>) -> CodecRoles {
    CodecRoles {
        codec: model.name.clone(),
        encoding: model.encoding,
        rows: ids
            .map(|id| Row {
                id: ElementId::Field(id.to_string()),
                role: Role::Presented,
            })
            .collect(),
        checks: Vec::new(),
        unresolved: Vec::new(),
    }
}
